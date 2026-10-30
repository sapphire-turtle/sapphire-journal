//! sapphire-journal's app server: what `sapphire-journal serve` runs, in either
//! edition. A library, not a binary of its own: the server is set up with the
//! same `sapphire-journal` command that runs it (`sapphire-journal-cli`), so
//! there is no separate server command to tell from the client's.
//!
//! The framework's [`AppServer`] owns the IPC socket, the current workspace
//! (`workspace init|select|show|move|remove|map`), its search index, signals
//! and the log file. This module adds what is the journal's own:
//!
//! - **sync**, through the host's `sapphire-bridge`. A bridge that is not
//!   running is not fatal: the server serves its workspace without sync and
//!   picks the bridge up once it starts, or again after it restarts.
//! - **the journal's methods** (`sapphire-journal.*`), which its MCP tools
//!   call: the framework serves those tools on stdio (`sapphire-journal mcp`)
//!   and, when `[http.mcp] enabled` in `config.toml` turns it on, at `/mcp`
//!   on the server's HTTP listener.
//! - **the periodic re-scan** of the entry cache, at the user config's
//!   `sync_interval_minutes`, so entries synced in from other devices show up
//!   in `entry_list` without a write here first.
//! - **keeping the search index current** with the journal's own writes. The
//!   framework re-indexes after a sync session, or when asked
//!   (`workspace.reindex`); a write through the journal's methods, or one
//!   found by the re-scan, asks for an incremental re-index, so
//!   `workspace_search` finds it with sync off or no bridge at all.

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use sapphire_framework::backend::WorkspaceBackend as _;
use sapphire_framework::ipc::ManagedBy;
use sapphire_framework::server::{
    AppFiles, AppServer, AppSettings, BridgeConnector, FieldError, HttpConfig, ReleaseSource,
    SERVER_CONFIG_FILE, ServerConfig, SettingDef, Settings, Staged, StatusRow, SyncRuntime,
    WorkspaceHost, serve_restarting,
};
use sapphire_journal_core::{JOURNAL_CTX, JournalState, user_config::UserConfig};
use serde::Deserialize as _;

/// This binary's version, reported in every handshake.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where the HTTP listener binds unless `config.toml`'s `[http] listen` says
/// otherwise: loopback, on the port `sapphire-journal-server` used.
pub const DEFAULT_HTTP_ADDR: &str = "127.0.0.1:3172";

/// Which edition runs the server: the release file `update` installs is the
/// edition's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edition {
    /// `sapphire-journal-headless`, the command line alone.
    Headless,
    /// `sapphire-journal-desktop`, the desktop app.
    Desktop,
}

impl Edition {
    /// Where the edition's releases are published: what `update` installs
    /// from, and where the server looks once a day for a newer one. Both are
    /// in the shared `v{version}` release (release-binaries.yml), as
    /// `<crate>-<os>-<arch>`; only the headless edition is on crates.io.
    pub fn releases(self) -> ReleaseSource {
        let source = ReleaseSource::github("sapphire-turtle", "sapphire-journal");
        match self {
            Edition::Headless => source
                .asset("sapphire-journal-headless")
                .cargo_crate("sapphire-journal-headless"),
            Edition::Desktop => source.asset("sapphire-journal-desktop"),
        }
    }
}

/// The server the framework's one-shot verbs dispatch against: the journal's
/// MCP tools and API version, nothing mounted.
///
/// `service` and `update` read only the service spec and the release source
/// from it; the other verbs talk to a *running* server over IPC themselves.
/// `mcp` serves the journal's tools on stdio, dialling that server. Opening a
/// bridge here would make `mcp` on a host without one fail with a bridge
/// error instead of the framework's "no server is running".
pub fn build_oneshot(edition: Edition) -> AppServer {
    base().releases(edition.releases())
}

/// The journal's MCP tools and API version, on the framework's server.
fn base() -> AppServer {
    AppServer::new(&JOURNAL_CTX, VERSION)
        .app_api(sapphire_journal_api::API_VERSION)
        .mcp(sapphire_journal_mcp::service())
}

/// The journal's server, without sync: the framework's server with the
/// journal's methods, its MCP tools, and its HTTP port.
///
/// [`run`] adds the release source, the bridge and the re-scan; a test runs
/// this as it is, so it never looks for a newer release.
pub fn build() -> (AppServer, Arc<CurrentJournal>) {
    let server = base().http_listen(default_http_addr());
    let journal = Arc::new(CurrentJournal::new(Arc::clone(server.host())));
    let source = Arc::clone(&journal);
    let written = Arc::clone(&journal);
    let server = server.extend(move |router| {
        sapphire_journal_mcp::register(
            router,
            Arc::new(move || source.get()),
            // Methods run on the blocking pool, inside the server's runtime.
            Some(Arc::new(move |_paths: &[PathBuf]| {
                let journal = Arc::clone(&written);
                tokio::runtime::Handle::current().spawn(async move { journal.reindex().await });
            })),
        )
    });
    (server, journal)
}

fn default_http_addr() -> SocketAddr {
    DEFAULT_HTTP_ADDR
        .parse()
        .expect("the default address parses")
}

/// The journal as an owner of the app server's settings: no keys of its own,
/// and one check of the framework's `[http]` table, [`check_exposure`]. The
/// framework runs it when the server starts, so a refused configuration
/// stops the start, and `sapphire-journal config list` warns about it.
#[derive(Clone)]
pub struct JournalOwner;

impl AppSettings for JournalOwner {
    fn defs(&self) -> Vec<SettingDef> {
        Vec::new()
    }

    fn validate(&self, files: &AppFiles, staged: &Staged) -> Result<(), Vec<FieldError>> {
        // A file that does not read, or an `[http]` the framework does not
        // take, is the framework's to report.
        let Ok(Some(text)) = staged.read(files.device()) else {
            return Ok(());
        };
        let Ok(table) = text.parse::<toml::Table>() else {
            return Ok(());
        };
        let Some(Ok(http)) = table.get("http").cloned().map(HttpConfig::deserialize) else {
            return Ok(());
        };
        if !http.mcp.enabled {
            return Ok(());
        }
        let addr = http.listen.unwrap_or_else(default_http_addr);
        check_exposure(addr, &http.allowed_hosts).map_err(|err| {
            vec![FieldError {
                key: Some("http.allowed_hosts".to_owned()),
                file: Some(files.device().to_path_buf()),
                message: err.to_string(),
            }]
        })
    }
}

/// Refuse a configuration that would serve 403 to every `/mcp` client.
///
/// rmcp's `Host` allowlist is loopback-only unless widened. Binding beyond
/// loopback without naming a host is not dangerous — it is useless, and it
/// presents as a client bug. Failing here turns that into an obvious
/// configuration error.
pub fn check_exposure(addr: SocketAddr, allowed_hosts: &[String]) -> anyhow::Result<()> {
    if addr.ip().is_loopback() {
        return Ok(());
    }
    if allowed_hosts.iter().any(|h| !h.trim().is_empty()) {
        return Ok(());
    }
    anyhow::bail!(
        "binding to {addr} without allowed_hosts would refuse every request: \
         rmcp's Host allowlist stays loopback-only, so a client reaching this \
         server by any other name gets 403. List each name clients use in \
         [http] allowed_hosts of {SERVER_CONFIG_FILE}, or bind to loopback."
    )
}

/// The journal the server's current workspace holds, opened once per root.
///
/// Calls share one [`JournalState`] while the workspace stays the same; a
/// `workspace select` to another root makes the next call open that one.
pub struct CurrentJournal {
    host: Arc<WorkspaceHost>,
    open: Mutex<Option<(PathBuf, Arc<Mutex<JournalState>>)>>,
}

impl CurrentJournal {
    pub fn new(host: Arc<WorkspaceHost>) -> CurrentJournal {
        CurrentJournal {
            host,
            open: Mutex::new(None),
        }
    }

    /// The current workspace's journal, or why there is none.
    pub fn get(&self) -> io::Result<Arc<Mutex<JournalState>>> {
        let root = self.host.root().ok_or_else(|| {
            io::Error::other(
                "this server has no workspace selected; run \
                 `sapphire-journal workspace select <dir>`",
            )
        })?;
        let mut open = self.open.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((open_root, state)) = open.as_ref()
            && *open_root == root
        {
            return Ok(Arc::clone(state));
        }
        let state = sapphire_journal_core::state::open_state(&root).map_err(|e| {
            io::Error::other(format!(
                "{} is not a journal yet ({e}); run `sapphire-journal init {}`",
                root.display(),
                root.display()
            ))
        })?;
        let state = Arc::new(Mutex::new(state));
        *open = Some((root, Arc::clone(&state)));
        Ok(state)
    }

    /// Bring the framework's search index up to date with the workspace's
    /// files, incrementally. Best-effort: a failure is logged, and the next
    /// write or re-scan tries again.
    pub async fn reindex(&self) {
        let backend = match self.host.backend().await {
            Ok(backend) => backend,
            Err(err) => {
                tracing::warn!("re-indexing the journal for search failed: {err}");
                return;
            }
        };
        if let Err(err) = backend.sync().await {
            tracing::warn!("re-indexing the journal for search failed: {err}");
        }
    }
}

/// Re-scan the current journal's entry cache, and re-index it for search,
/// every `interval`.
///
/// `journal` is the running server's, when one runs. A pass with no server
/// running, no workspace selected, or one that is not a journal, does
/// nothing; a failed pass is logged and the next one tries again.
pub fn spawn_rescan(
    journal: Arc<Mutex<Weak<CurrentJournal>>>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let Some(journal) = journal.lock().unwrap_or_else(|e| e.into_inner()).upgrade() else {
                continue;
            };
            let current = Arc::clone(&journal);
            let pass = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let Ok(state) = current.get() else {
                    return Ok(());
                };
                // Recover a poisoned lock: a panic here would end this task
                // silently, and no later pass would ever run.
                let state = state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                Ok(state.sync()?)
            })
            .await;
            match pass {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    tracing::warn!("periodic re-scan of the entry cache failed: {err:#}")
                }
                Err(err) => tracing::warn!("periodic re-scan of the entry cache panicked: {err}"),
            }
            if journal.host.root().is_some() {
                journal.reindex().await;
            }
        }
    })
}

/// Run the server as `edition` until SIGTERM or SIGINT, in place again on
/// `server.restart`.
pub async fn run(edition: Edition) -> anyhow::Result<()> {
    // Resolved once, up front: a server's `build` cannot fail.
    let exe = std::env::current_exe()?;
    // The re-scan outlives the rounds: each round's server points it at its
    // journal. Weak, so a stopped server's workspace is not held open.
    let current = Arc::new(Mutex::new(Weak::new()));
    let rescan = UserConfig::load()?
        .sync_interval()
        .map(|interval| spawn_rescan(Arc::clone(&current), interval));
    let result = serve_restarting(&JOURNAL_CTX, JournalOwner, |settings: &Settings| {
        build_serving(settings, &exe, edition, &current)
    })
    .await;
    if let Some(rescan) = rescan {
        rescan.abort();
    }
    Ok(result?)
}

/// One round of [`run`]: [`build`], with `edition`'s release source, sync and
/// the status row, as `settings` say.
fn build_serving(
    settings: &Settings,
    exe: &std::path::Path,
    edition: Edition,
    current: &Mutex<Weak<CurrentJournal>>,
) -> AppServer {
    // The framework checked the settings before handing them over, so they
    // read; the defaults are only a fallback.
    let config = ServerConfig::from_settings(settings).unwrap_or_else(|err| {
        tracing::warn!("reading the server's settings failed: {err}");
        ServerConfig::default()
    });

    // Sync through the host's bridge. The framework connects, and connects
    // again whenever the bridge restarts; a bridge that is not running yet
    // stops nothing but sync. Any other failure — a bridge of another
    // version, say — stops the start, in `run`.
    let (server, journal) = build();
    *current.lock().unwrap_or_else(|e| e.into_inner()) = Arc::downgrade(&journal);
    let runtime = SyncRuntime::connecting(
        &JOURNAL_CTX,
        BridgeConnector::standard(JOURNAL_CTX.app_name, VERSION),
        exe.to_path_buf(),
        ManagedBy::Service,
    );
    let server = server.releases(edition.releases()).sync(Arc::new(runtime));

    let mcp_row = if config.http.mcp.enabled {
        let http_addr = config.http.listen.unwrap_or_else(default_http_addr);
        format!("http://{http_addr}/mcp")
    } else {
        "off".to_owned()
    };
    server.status_rows(Arc::new(move || {
        vec![StatusRow {
            name: "mcp".to_owned(),
            value: mcp_row.clone(),
        }]
    }))
}
