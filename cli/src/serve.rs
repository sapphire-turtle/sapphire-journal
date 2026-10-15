//! The app server: what `sapphire-journal serve` (and a bare `sapphire-journal`)
//! runs.
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
//!   and, when `server.toml` turns it on, at `/mcp` on the server's HTTP
//!   listener.
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
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sapphire_framework::backend::WorkspaceBackend as _;
use sapphire_framework::server::{
    AppServer, SERVER_CONFIG_FILE, ServerConfig, StatusRow, WorkspaceHost,
};
use sapphire_journal_core::{JOURNAL_CTX, JournalState, user_config::UserConfig};

/// This binary's version, reported in every handshake.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where the HTTP listener binds unless `server.toml`'s `[http] listen` says
/// otherwise: loopback, on the port `sapphire-journal-server` used.
pub const DEFAULT_HTTP_ADDR: &str = "127.0.0.1:3172";

/// The server the framework's one-shot verbs dispatch against: the journal's
/// MCP tools and API version, nothing mounted.
///
/// `status`, `service`, `workspace`, `workgroup`, `device`, `embedding` and
/// `external-device` read only the app name and the service spec from it and
/// talk to a *running* server (or the bridge) over IPC themselves; `mcp`
/// serves the journal's tools on stdio, dialling that server. Opening a
/// bridge here would make `status` on a host without one fail with a bridge
/// error instead of the framework's "no server is running".
pub fn build_oneshot() -> AppServer {
    AppServer::new(&JOURNAL_CTX, VERSION)
        .app_api(sapphire_journal_api::API_VERSION)
        .mcp(sapphire_journal_mcp::service())
}

/// The journal's server, without sync: the framework's server with the
/// journal's methods, its MCP tools, and its HTTP port.
///
/// [`run`] adds the bridge and the re-scan; a test runs this as it is.
pub fn build() -> (AppServer, Arc<CurrentJournal>) {
    let server = build_oneshot().http_listen(default_http_addr());
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
/// A pass with no workspace selected, or one that is not a journal, does
/// nothing; a failed pass is logged and the next one tries again.
pub fn spawn_rescan(journal: Arc<CurrentJournal>, interval: Duration) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let current = Arc::clone(&journal);
            let pass = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let Ok(state) = current.get() else { return Ok(()) };
                // Recover a poisoned lock: a panic here would end this task
                // silently, and no later pass would ever run.
                let state = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                Ok(state.sync()?)
            })
            .await;
            match pass {
                Ok(Ok(())) => {}
                Ok(Err(err)) => tracing::warn!("periodic re-scan of the entry cache failed: {err:#}"),
                Err(err) => tracing::warn!("periodic re-scan of the entry cache panicked: {err}"),
            }
            if journal.host.root().is_some() {
                journal.reindex().await;
            }
        }
    })
}

/// Run the server until SIGTERM or SIGINT.
pub async fn run() -> anyhow::Result<()> {
    // The framework reads `server.toml` again when it starts; this read is
    // for what the journal adds on top: the exposure check and the status row.
    let config = ServerConfig::load_for_app(&JOURNAL_CTX)?;
    let http_addr = config.http.listen.unwrap_or_else(default_http_addr);
    if config.mcp.http {
        check_exposure(http_addr, &config.http.allowed_hosts)?;
    }

    // Sync through the host's bridge. The framework connects, and connects
    // again whenever the bridge restarts; a bridge that is not running yet
    // stops nothing but sync. Any other failure — a bridge of another
    // version, say — stops the start, in `run`.
    let (server, journal) = build();
    let server = server.sync_via_bridge()?;

    let mcp_row = if config.mcp.http {
        format!("http://{http_addr}/mcp")
    } else {
        "off".to_owned()
    };
    let server = server.status_rows(Arc::new(move || {
        vec![StatusRow {
            name: "mcp".to_owned(),
            value: mcp_row.clone(),
        }]
    }));

    let rescan = UserConfig::load()?
        .sync_interval()
        .map(|interval| spawn_rescan(journal, interval));
    let result = server.run().await;
    if let Some(rescan) = rescan {
        rescan.abort();
    }
    Ok(result?)
}
