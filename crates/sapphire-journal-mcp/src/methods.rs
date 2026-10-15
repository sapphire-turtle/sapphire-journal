//! The journal's methods on its app server: what the MCP tools call.
//!
//! Every tool reaches the journal the way the framework's own tools reach the
//! workspace — over the app server's IPC, as `sapphire-journal.<tool>` — so one
//! implementation serves the `mcp` subcommand (stdio, dialling the running
//! server) and `/mcp` on the server's HTTP listener alike. [`register`] adds
//! the methods to the server's router; [`crate::tools`] is the other end.
//!
//! The same methods serve the CLI and the desktop app: the app server is the
//! only process that reads or writes the journal's files and its entry cache,
//! and every other process asks it. Some methods exist for those clients only
//! (`entry_get`, `entry_save`, `config_*`, …) and have no MCP tool.
//!
//! [`JournalMethods`] is the journal side of one call: an
//! `Arc<Mutex<JournalState>>`, and a function per method returning text — the
//! message an agent or a person reads, or JSON for a client to parse.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sapphire_framework::ipc::{RequestCtx, Router, RpcError};
use sapphire_journal_core::{
    cache,
    entry::Value,
    journal::Journal,
    ops::{self, EntryFields, EntryListItem, UpdateOption},
    parser::{read_entry, write_entry},
    text_input::{fields as core_fields, filter as core_filter},
    user_config::UserConfig,
    JournalState,
};
use sapphire_journal_api::{
    self as api, ConfigSetParams, EmptyParams, EntryCheckParams, EntryFixParams, EntryGetParams,
    EntryListParams, EntryModifyParams, EntryNewParams, EntryPrepareParams, EntryRemoveParams,
    EntryResolveParams, EntrySaveParams, EntryShowParams, EntryTreeParams, JournalSettings,
};
use serde::de::DeserializeOwned;

/// Called after a method writes to the journal, on the blocking thread the
/// method ran on. The argument is the paths affected.
///
/// **One call is one batch.** A rename reports the old and new path in the
/// same call; notifying separately would let a syncing receiver briefly lose
/// the entry.
pub type WriteObserver = Arc<dyn Fn(&[PathBuf]) + Send + Sync>;

/// Where a call finds its journal: asked once per call.
///
/// The app server's journal is whatever workspace it has selected, which
/// `workspace select` can change underneath it; an error refuses the call.
pub type StateSource = Arc<dyn Fn() -> std::io::Result<Arc<Mutex<JournalState>>> + Send + Sync>;

/// Add the journal's methods to the app server's `router`, each resolving the
/// journal through `source` when it is called, and reporting the paths a
/// write touched to `on_write`.
pub fn register(router: Router, source: StateSource, on_write: Option<WriteObserver>) -> Router {
    let methods = Methods { source, on_write };
    let router = route(router, api::ENTRY_LIST, &methods, JournalMethods::entry_list);
    let router = route(router, api::ENTRY_TREE, &methods, JournalMethods::entry_tree);
    let router = route(router, api::ENTRY_SHOW, &methods, JournalMethods::entry_show);
    let router = route(router, api::ENTRY_NEW, &methods, JournalMethods::entry_new);
    let router = route(router, api::ENTRY_MODIFY, &methods, JournalMethods::entry_modify);
    let router = route(router, api::ENTRY_CHECK, &methods, JournalMethods::entry_check);
    let router = route(router, api::ENTRY_FIX, &methods, JournalMethods::entry_fix);
    let router = route(router, api::ENTRY_REMOVE, &methods, JournalMethods::entry_remove);
    let router = route(router, api::CACHE_INFO, &methods, JournalMethods::cache_info);
    let router = route(router, api::CACHE_SYNC, &methods, JournalMethods::cache_sync);
    let router = route(router, api::CACHE_REBUILD, &methods, JournalMethods::cache_rebuild);
    let router = route(router, api::ENTRY_GET, &methods, JournalMethods::entry_get);
    let router = route(router, api::ENTRY_RESOLVE, &methods, JournalMethods::entry_resolve);
    let router = route(router, api::ENTRY_PREPARE, &methods, JournalMethods::entry_prepare);
    let router = route(router, api::ENTRY_SAVE, &methods, JournalMethods::entry_save);
    let router = route(router, api::CACHE_CLEAN, &methods, JournalMethods::cache_clean);
    let router = route(router, api::CONFIG_SHOW, &methods, JournalMethods::config_show);
    let router = route(router, api::CONFIG_GET, &methods, JournalMethods::config_get);
    route(router, api::CONFIG_SET, &methods, JournalMethods::config_set)
}

/// What every registered method shares.
#[derive(Clone)]
struct Methods {
    source: StateSource,
    on_write: Option<WriteObserver>,
}

impl Methods {
    fn resolve(&self) -> std::io::Result<JournalMethods> {
        let methods = JournalMethods::from_shared((self.source)()?);
        Ok(match &self.on_write {
            Some(observer) => methods.with_write_observer(Arc::clone(observer)),
            None => methods,
        })
    }
}

/// Register one method: parse its parameters, then run `method` against the
/// current journal on the blocking pool — the journal is files, SQLite and a
/// std mutex.
///
/// The method's text is the result; its error text is the RPC error's message,
/// which the tool hands to the agent as it is.
fn route<P>(
    router: Router,
    name: &str,
    methods: &Methods,
    method: fn(&JournalMethods, P) -> Result<String, String>,
) -> Router
where
    P: DeserializeOwned + Send + 'static,
{
    let methods = methods.clone();
    router.method(name, move |ctx: RequestCtx| {
        let methods = methods.clone();
        async move {
            let params: P = serde_json::from_value(ctx.params)
                .map_err(|e| RpcError::invalid_params(e.to_string()))?;
            let text = tokio::task::spawn_blocking(move || {
                let journal = methods.resolve().map_err(|e| e.to_string())?;
                method(&journal, params)
            })
            .await
            .map_err(|e| RpcError::internal(format!("the journal method panicked: {e}")))?
            .map_err(RpcError::internal)?;
            Ok(serde_json::Value::String(text))
        }
    })
}

#[derive(Clone)]
pub struct JournalMethods {
    /// Cached journal, shared across calls.
    state: Arc<Mutex<JournalState>>,
    write_observer: Option<WriteObserver>,
}

impl std::fmt::Debug for JournalMethods {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JournalMethods").finish_non_exhaustive()
    }
}

impl JournalMethods {
    pub fn new(state: JournalState) -> Self {
        Self::from_shared(Arc::new(Mutex::new(state)))
    }

    /// Build over an existing state handle: every call on the server shares
    /// one journal.
    pub fn from_shared(state: Arc<Mutex<JournalState>>) -> Self {
        Self {
            state,
            write_observer: None,
        }
    }

    pub fn shared_state(&self) -> Arc<Mutex<JournalState>> {
        Arc::clone(&self.state)
    }

    /// Set where post-write notifications go.
    pub fn with_write_observer(mut self, observer: WriteObserver) -> Self {
        self.write_observer = Some(observer);
        self
    }

    /// Notify the observer of the affected paths. A no-op when none is set.
    fn notify_write(&self, paths: &[PathBuf]) {
        if let Some(observer) = &self.write_observer {
            observer(paths);
        }
    }

    /// Take the journal lock, recovering it if an earlier panic poisoned it.
    ///
    /// `.lock().unwrap()` would turn one panic anywhere under this mutex into
    /// a permanently dead server: every later call would panic on the poison
    /// rather than on anything wrong with the journal. And `JournalState`'s
    /// invariants do not actually depend on the previous holder finishing —
    /// SQLite connections are opened per operation (`open_conn`), and
    /// `sync_cache` runs inside a transaction that rolls back if it is
    /// interrupted. So we say so once and carry on.
    fn lock_state(&self) -> std::sync::MutexGuard<'_, JournalState> {
        if self.state.is_poisoned() {
            tracing::warn!(
                "journal state mutex was poisoned by an earlier panic; recovering it rather \
                 than failing every call for the rest of the process"
            );
        }
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn with_state<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&JournalState) -> anyhow::Result<T>,
    {
        let guard = self.lock_state();
        f(&guard)
    }
}

// ── helpers for parameter parsing ─────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn parse_entry_fields(
    slug: Option<String>,
    tags: Option<String>,
    task_due: Option<&str>,
    task_status: Option<String>,
    task_started_at: Option<&str>,
    task_closed_at: Option<&str>,
    event_start: Option<&str>,
    event_end: Option<&str>,
    hidden: Option<bool>,
) -> anyhow::Result<EntryFields> {
    use core_fields::{parse_optional_datetime, parse_optional_datetime_end, parse_tags_csv};
    Ok(EntryFields {
        title: None,
        body: None,
        parent: UpdateOption::Unchanged,
        slug,
        tags: parse_tags_csv(tags.as_deref()),
        task_due: parse_optional_datetime_end(task_due)?,
        task_status,
        task_started_at: parse_optional_datetime(task_started_at)?,
        task_closed_at: parse_optional_datetime(task_closed_at)?,
        event_start: parse_optional_datetime(event_start)?,
        event_end: parse_optional_datetime_end(event_end)?,
        hidden,
    })
}

fn filter_inputs(p: &EntryListParams) -> core_filter::FilterInputs<'_> {
    core_filter::FilterInputs {
        period: p.period.as_deref(),
        active: p.active.unwrap_or(false),
        task_overdue: p.task_overdue.unwrap_or(false),
        task_in_progress: p.task_in_progress.unwrap_or(false),
        task_unstarted: p.task_unstarted.unwrap_or(false),
        event_span: p.event_span.unwrap_or(false),
        created_at: p.created_at.unwrap_or(false),
        updated_at: p.updated_at.unwrap_or(false),
        task_status: p.task_status.as_deref().unwrap_or(&[]),
        tags: p.tags.as_deref().unwrap_or(&[]),
        sort_by: p.sort_by.as_deref(),
        sort_order: p.sort_order.as_deref(),
        include_stale: p.include_stale.unwrap_or(false),
        include_hidden: p.include_hidden.unwrap_or(false),
    }
}

// ── method implementations ────────────────────────────────────────────────────

impl JournalMethods {
    pub fn entry_list(&self, p: EntryListParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let filter = core_filter::build_filter(filter_inputs(&p))?;
            let has_filter = filter.has_any_filter();
            let entries = self.with_state(|s| ops::list_entries(s, &filter).map_err(Into::into))?;
            let records: Vec<EntryListItem> = entries
                .iter()
                .map(|(entry, match_flags)| EntryListItem {
                    entry: entry.clone(),
                    match_flags: if has_filter { Some(match_flags.clone()) } else { None },
                })
                .collect();
            Ok(serde_json::to_string_pretty(&records)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn entry_tree(&self, p: EntryTreeParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let filter = core_filter::build_filter(filter_inputs(&p))?;
            let entries = self.with_state(|s| {
                let entries = ops::list_entries(s, &filter)?;
                // Keep a match's ancestors, flagged `parent_of_match`, so the
                // tree still shows where it sits.
                Ok(if filter.has_any_filter() {
                    ops::fill_ancestor_entries(entries, s)?
                } else {
                    entries
                })
            })?;
            let roots = ops::build_entry_tree(entries);
            Ok(serde_json::to_string_pretty(&roots)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn entry_show(&self, p: EntryShowParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let path = self.with_state(|s| {
                let conn = s.open_conn()?;
                ops::resolve_entry(&p.entry, &conn).map_err(Into::into)
            })?;
            let entry = read_entry(&path)?;
            let fm = &entry.frontmatter;

            let mut out = format!("# {}\n", entry.title());
            out.push_str(&format!("created:  {}\n", fm.created_at.format("%Y-%m-%dT%H:%M")));
            out.push_str(&format!("updated:  {}\n", fm.updated_at.format("%Y-%m-%dT%H:%M")));
            if !fm.tags.is_empty() {
                out.push_str(&format!("tags:     {}\n", fm.tags.join(", ")));
            }
            if let Some(task) = &fm.task {
                let status = task.status.as_str();
                match task.due {
                    Some(d) => out.push_str(&format!("task:     {status} (due {})\n", d.format("%Y-%m-%d"))),
                    None    => out.push_str(&format!("task:     {status}\n")),
                }
                if let Some(ca) = task.closed_at {
                    out.push_str(&format!("closed:   {}\n", ca.format("%Y-%m-%dT%H:%M")));
                }
            }
            if let Some(event) = &fm.event {
                out.push_str(&format!("event:    {} – {}\n", event.start.format("%Y-%m-%d"), event.end.format("%Y-%m-%d")));
            }
            out.push('\n');
            out.push_str(&entry.body);
            Ok(out)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn entry_new(&self, p: EntryNewParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let fields = parse_entry_fields(
                p.slug,
                p.tags,
                p.task_due.as_deref(),
                p.task_status,
                p.task_started_at.as_deref(),
                p.task_closed_at.as_deref(),
                p.event_start.as_deref(),
                p.event_end.as_deref(),
                p.hidden,
            )?;
            let fields = EntryFields {
                title: p.title,
                body: p.body,
                parent: match p.parent {
                    Some(r) => UpdateOption::Set(r),
                    None => UpdateOption::Unchanged,
                },
                ..fields
            };
            let dest = self.with_state(|s| {
                s.sync()?;
                let dest = ops::create_entry(s, fields)?;
                if let Ok(conn) = s.open_conn() {
                    let _ = cache::upsert_entry_from_path(&conn, &dest);
                }
                Ok(dest)
            })?;
            self.notify_write(std::slice::from_ref(&dest));
            Ok(format!("created: {}", dest.display()))
        })()
        .map_err(|e| e.to_string())
    }

    pub fn entry_modify(&self, p: EntryModifyParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            if p.title.is_none()
                && p.body.is_none()
                && p.slug.is_none()
                && p.tags.is_none()
                && p.task_due.is_none()
                && p.task_status.is_none()
                && p.task_started_at.is_none()
                && p.task_closed_at.is_none()
                && p.event_start.is_none()
                && p.event_end.is_none()
                && p.hidden.is_none()
                && p.parent.is_unchanged()
            {
                anyhow::bail!("nothing to update — specify at least one field");
            }

            let fields = parse_entry_fields(
                p.slug,
                p.tags,
                p.task_due.as_deref(),
                p.task_status,
                p.task_started_at.as_deref(),
                p.task_closed_at.as_deref(),
                p.event_start.as_deref(),
                p.event_end.as_deref(),
                p.hidden,
            )?;
            let fields = EntryFields {
                title: p.title,
                body: p.body,
                parent: p.parent,
                ..fields
            };
            let (msg, notify_paths) = self.with_state(|s| {
                s.sync()?;
                let conn = s.open_conn()?;
                let path = ops::resolve_entry(&p.entry, &conn)?;
                let renamed = ops::update_entry(&path, &conn, fields)?;
                let (msg, notify_paths) = match &renamed {
                    Some(new_path) => {
                        let _ = cache::upsert_entry_from_path(&conn, new_path);
                        (
                            format!("updated and renamed: {}", new_path.display()),
                            vec![path.clone(), new_path.clone()],
                        )
                    }
                    None => {
                        let _ = cache::upsert_entry_from_path(&conn, &path);
                        (format!("updated: {}", path.display()), vec![path.clone()])
                    }
                };
                Ok((msg, notify_paths))
            })?;
            self.notify_write(&notify_paths);
            Ok(msg)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn entry_check(&self, p: EntryCheckParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let path = self.with_state(|s| {
                let conn = s.open_conn()?;
                ops::resolve_entry(&p.entry, &conn).map_err(Into::into)
            })?;
            let issues = ops::check_entry(&path)?;
            if issues.is_empty() {
                Ok(format!("ok: {}", path.display()))
            } else {
                let lines: Vec<String> = issues
                    .iter()
                    .map(|i| format!("{}: {}", path.display(), i.as_str()))
                    .collect();
                Ok(lines.join("\n"))
            }
        })()
        .map_err(|e| e.to_string())
    }

    pub fn entry_fix(&self, p: EntryFixParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let (path, renamed) = self.with_state(|s| {
                let conn = s.open_conn()?;
                let path = ops::resolve_entry(&p.entry, &conn)?;
                let renamed = ops::fix_entry(&path)?;
                // The file may have been edited outside the server (`entry
                // edit` runs the user's editor on it): refresh its row either way.
                let _ = cache::upsert_entry_from_path(&conn, renamed.as_deref().unwrap_or(&path));
                Ok((path, renamed))
            })?;
            match renamed {
                Some(new_path) => {
                    self.notify_write(&[path.clone(), new_path.clone()]);
                    Ok(format!(
                        "renamed: {} → {}",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        new_path.file_name().unwrap_or_default().to_string_lossy(),
                    ))
                }
                None => {
                    self.notify_write(std::slice::from_ref(&path));
                    Ok(format!("ok: {} (already correct)", path.display()))
                }
            }
        })()
        .map_err(|e| e.to_string())
    }

    pub fn entry_remove(&self, p: EntryRemoveParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let path = self.with_state(|s| {
                s.sync()?;
                let conn = s.open_conn()?;
                let path = ops::resolve_entry(&p.entry, &conn)?;
                ops::remove_entry(&path)?;
                let _ = cache::remove_from_cache(&conn, &path);
                Ok(path)
            })?;
            self.notify_write(std::slice::from_ref(&path));
            Ok(format!("removed: {}", path.display()))
        })()
        .map_err(|e| e.to_string())
    }

    pub fn cache_info(&self, _: EmptyParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            self.with_state(|s| {
                let info = s.cache_info()?;
                Ok(format!(
                    "path: {}\nschema version: v{} (app: v{})\nfiles tracked: {}\nentries: {}\nunique tags: {}",
                    info.db_path.display(),
                    info.schema_version,
                    cache::SCHEMA_VERSION,
                    info.file_count,
                    info.entry_count,
                    info.unique_tag_count,
                ))
            })
        })()
        .map_err(|e| e.to_string())
    }

    pub fn cache_sync(&self, _: EmptyParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            self.with_state(|s| {
                s.sync()?;
                let info = s.cache_info()?;
                Ok(format!("synced: {} entries", info.entry_count))
            })
        })()
        .map_err(|e| e.to_string())
    }

    pub fn cache_rebuild(&self, _: EmptyParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let mut guard = self.lock_state();
            let journal = Journal::from_root(guard.journal.root.clone())?;
            let state = JournalState::rebuild(journal)?;
            state.sync()?;
            let info = state.cache_info()?;
            *guard = state;
            Ok(format!("rebuilt: {} entries indexed", info.entry_count))
        })()
        .map_err(|e| e.to_string())
    }
}

// ── methods for the CLI and the desktop app ───────────────────────────────────

impl JournalMethods {
    /// The whole entry — path, frontmatter and body — as JSON.
    pub fn entry_get(&self, p: EntryGetParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let path = self.with_state(|s| {
                let conn = s.open_conn()?;
                ops::resolve_entry(&p.entry, &conn).map_err(Into::into)
            })?;
            Ok(serde_json::to_string(&read_entry(&path)?)?)
        })()
        .map_err(|e| e.to_string())
    }

    /// The entry's path.
    pub fn entry_resolve(&self, p: EntryResolveParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let path = self.with_state(|s| {
                s.sync()?;
                let conn = s.open_conn()?;
                ops::resolve_entry(&p.entry, &conn).map_err(Into::into)
            })?;
            Ok(path.display().to_string())
        })()
        .map_err(|e| e.to_string())
    }

    /// Create an empty entry for an editor to fill in, and return its path.
    pub fn entry_prepare(&self, p: EntryPrepareParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let path = self.with_state(|s| {
                let parent = match &p.parent {
                    Some(parent) => {
                        s.sync()?;
                        let conn = s.open_conn()?;
                        ops::resolve_parent_id(&conn, Some(parent))?
                    }
                    None => None,
                };
                let path = ops::prepare_new_entry(&s.journal, parent)?;
                if let Ok(conn) = s.open_conn() {
                    let _ = cache::upsert_entry_from_path(&conn, &path);
                }
                Ok(path)
            })?;
            self.notify_write(std::slice::from_ref(&path));
            Ok(path.display().to_string())
        })()
        .map_err(|e| e.to_string())
    }

    /// Write a whole entry back, normalize it (`entry_fix`), and return its
    /// path, which the normalization may have changed.
    pub fn entry_save(&self, p: EntrySaveParams<Value>) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let mut entry = p.entry;
            let (old_path, final_path) = self.with_state(|s| {
                ensure_inside(&s.journal, &entry.path)?;
                let old_path = entry.path.clone();
                write_entry(&mut entry)?;
                let final_path = ops::fix_entry(&entry.path)?.unwrap_or_else(|| entry.path.clone());
                if let Ok(conn) = s.open_conn() {
                    let _ = cache::upsert_entry_from_path(&conn, &final_path);
                }
                Ok((old_path, final_path))
            })?;
            if old_path == final_path {
                self.notify_write(std::slice::from_ref(&final_path));
            } else {
                self.notify_write(&[old_path, final_path.clone()]);
            }
            Ok(final_path.display().to_string())
        })()
        .map_err(|e| e.to_string())
    }

    /// Remove the cache files older versions left behind.
    pub fn cache_clean(&self, _: EmptyParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let removed = self.with_state(|s| Ok(cache::clean_stale_files(&s.journal)?))?;
            if removed.is_empty() {
                return Ok("nothing to clean".to_owned());
            }
            Ok(removed
                .iter()
                .map(|(path, size)| format!("removed: {} ({})", path.display(), human_size(*size)))
                .collect::<Vec<_>>()
                .join("\n"))
        })()
        .map_err(|e| e.to_string())
    }

    /// The user config and the journal config, as their files hold them.
    pub fn config_show(&self, _: EmptyParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let journal_config = self.with_state(|s| Ok(s.journal.config_path()))?;
            let mut out = String::new();
            for (heading, path) in [("user config", UserConfig::path()), ("journal config", journal_config)] {
                out.push_str(&format!("## {heading}\npath: {}\n", path.display()));
                match std::fs::read_to_string(&path) {
                    Ok(raw) => out.push_str(raw.trim_end()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        out.push_str("(file not found — using defaults)")
                    }
                    Err(e) => return Err(e.into()),
                }
                out.push_str("\n\n");
            }
            Ok(out.trim_end().to_owned())
        })()
        .map_err(|e| e.to_string())
    }

    /// [`JournalSettings`] as JSON.
    pub fn config_get(&self, _: EmptyParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let settings = self.with_state(|s| settings_of(&s.journal))?;
            Ok(serde_json::to_string(&settings)?)
        })()
        .map_err(|e| e.to_string())
    }

    /// Change [`JournalSettings`]; returns them as they are now, as JSON. A
    /// new `sync_interval_minutes` applies when the server next starts.
    pub fn config_set(&self, p: ConfigSetParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let settings = self.with_state(|s| {
                if let Some(minutes) = p.sync_interval_minutes {
                    let mut user = UserConfig::load()?;
                    user.sync_interval_minutes = Some(minutes);
                    user.save()?;
                }
                if p.duplicate_title.is_some() || !p.entries_dir.is_unchanged() {
                    let mut config = s.journal.config()?;
                    if let Some(policy) = p.duplicate_title {
                        config.journal.duplicate_title = policy;
                    }
                    match p.entries_dir {
                        UpdateOption::Set(dir) => config.journal.entries_dir = Some(dir),
                        UpdateOption::Clear => config.journal.entries_dir = None,
                        UpdateOption::Unchanged => {}
                    }
                    s.journal.save_config(&config)?;
                }
                settings_of(&s.journal)
            })?;
            Ok(serde_json::to_string(&settings)?)
        })()
        .map_err(|e| e.to_string())
    }
}

fn settings_of(journal: &Journal) -> anyhow::Result<JournalSettings> {
    let config = journal.config()?;
    Ok(JournalSettings {
        root: journal.root.clone(),
        sync_interval_minutes: UserConfig::load()?.sync_interval_minutes,
        duplicate_title: config.journal.duplicate_title,
        entries_dir: config.journal.entries_dir,
    })
}

/// Refuse a path outside the journal, or one that is not an existing file: a
/// client hands `entry_save` the path `entry_get` gave it, and nothing else.
fn ensure_inside(journal: &Journal, path: &std::path::Path) -> anyhow::Result<()> {
    let root = journal.root.canonicalize()?;
    let file = path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    if !file.is_file() || !file.starts_with(&root) {
        anyhow::bail!("{} is not an entry of this journal", path.display());
    }
    Ok(())
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapphire_journal_api::EntryRef;

    /// A throwaway journal under a fresh `TempDir`, and methods over it. Hold
    /// onto the `TempDir` for the test's duration: dropping it removes the
    /// directory the open journal state points at.
    fn test_methods() -> (tempfile::TempDir, JournalMethods) {
        sapphire_journal_core::init_app_context(sapphire_journal_core::AppKind::Cli);
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let journal = Journal::init(dir.path()).expect("failed to init test journal");
        let state = JournalState::open(journal).expect("failed to open test journal");
        (dir, JournalMethods::new(state))
    }

    // Every observer test below calls an actual method, so the
    // create/update/fix/remove call sites are the things under test.

    fn new_params(title: &str) -> EntryNewParams {
        EntryNewParams {
            title: Some(title.to_owned()),
            ..Default::default()
        }
    }

    /// `entry_new`'s response is `"created: {path}"`; pull the path back out
    /// so the test can hand it to a follow-up `entry_modify`/`entry_remove`
    /// call and compare it against what the observer received.
    fn path_from_created_message(msg: &str) -> PathBuf {
        PathBuf::from(
            msg.strip_prefix("created: ")
                .unwrap_or_else(|| panic!("unexpected entry_new response: {msg}")),
        )
    }

    fn observer() -> (WriteObserver, Arc<Mutex<Vec<Vec<PathBuf>>>>) {
        let observed: Arc<Mutex<Vec<Vec<PathBuf>>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&observed);
        (Arc::new(move |paths: &[PathBuf]| sink.lock().unwrap().push(paths.to_vec())), observed)
    }

    /// The parameters survive the trip the tools send them on: serialized by
    /// the tool, deserialized by the method. `parent` in particular has three
    /// states, and "unchanged" is the absence of the field.
    #[test]
    fn modify_params_round_trip_all_three_parent_states() {
        let entry = EntryRef::Title("t".to_owned());
        let unchanged = serde_json::to_value(EntryModifyParams::of(entry.clone())).unwrap();
        assert!(unchanged.get("parent").is_none(), "{unchanged}");
        let back: EntryModifyParams = serde_json::from_value(unchanged).unwrap();
        assert!(back.parent.is_unchanged());

        let mut clear = EntryModifyParams::of(entry.clone());
        clear.parent = UpdateOption::Clear;
        let back: EntryModifyParams =
            serde_json::from_value(serde_json::to_value(clear).unwrap()).unwrap();
        assert!(matches!(back.parent, UpdateOption::Clear));

        let mut set = EntryModifyParams::of(entry);
        set.parent = UpdateOption::Set(EntryRef::Title("p".to_owned()));
        let back: EntryModifyParams =
            serde_json::from_value(serde_json::to_value(set).unwrap()).unwrap();
        assert!(matches!(back.parent, UpdateOption::Set(EntryRef::Title(t)) if t == "p"));
    }

    /// One panic under the journal lock must not end the server.
    ///
    /// The state mutex is shared by every call and by the server's periodic
    /// re-scan. With `.lock().unwrap()` a single panic anywhere under it
    /// poisons the mutex and every later call panics on the poison rather than
    /// on anything actually wrong, for the rest of the process's life.
    #[test]
    fn a_poisoned_journal_lock_does_not_end_the_server() {
        let (_dir, methods) = test_methods();
        let state = methods.shared_state();

        let poisoner = Arc::clone(&state);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.lock().unwrap();
            panic!("a call panicked while holding the journal lock");
        })
        .join();
        assert!(state.is_poisoned(), "precondition: the mutex should be poisoned");

        let msg = methods
            .entry_new(new_params("After The Panic"))
            .expect("entry_new failed after the mutex was poisoned");
        assert!(msg.starts_with("created: "), "{msg}");
    }

    #[test]
    fn entry_new_notifies_the_created_path_once() {
        let (_dir, methods) = test_methods();
        let (obs, observed) = observer();
        let methods = methods.with_write_observer(obs);

        let msg = methods.entry_new(new_params("Original Title")).expect("entry_new failed");
        let created = path_from_created_message(&msg);

        let calls = observed.lock().unwrap();
        assert_eq!(*calls, vec![vec![created]]);
    }

    #[test]
    fn entry_modify_rename_notifies_old_and_new_path_in_one_call() {
        let (_dir, methods) = test_methods();
        let (obs, observed) = observer();
        let methods = methods.with_write_observer(obs);

        let msg = methods.entry_new(new_params("Original Title")).expect("entry_new failed");
        let old_path = path_from_created_message(&msg);
        observed.lock().unwrap().clear(); // isolate the rename notification below

        let mut modify = EntryModifyParams::of(EntryRef::Path(old_path.clone()));
        modify.title = Some("Renamed Title".to_owned());
        let modify_msg = methods.entry_modify(modify).expect("entry_modify failed");
        assert!(
            modify_msg.starts_with("updated and renamed:"),
            "expected a rename, got: {modify_msg}"
        );

        let calls = observed.lock().unwrap();
        assert_eq!(calls.len(), 1, "rename must notify in exactly one call");
        assert_eq!(calls[0].len(), 2, "rename must carry both paths");
        assert_eq!(calls[0][0], old_path, "old path must come first");
        assert_ne!(calls[0][1], old_path, "second path must be the new one");
    }

    #[test]
    fn entry_fix_rename_notifies_old_and_new_path_in_one_call() {
        let (_dir, methods) = test_methods();
        let (obs, observed) = observer();
        let methods = methods.with_write_observer(obs);

        let msg = methods.entry_new(new_params("Original Title")).expect("entry_new failed");
        let canonical_path = path_from_created_message(&msg);
        observed.lock().unwrap().clear(); // isolate the fix notification below

        // Put the file into the state `entry_fix` is meant to correct: rename
        // it on disk (same dir, off-canonical name) so it no longer matches
        // the `{id}_{slug}.md` filename `fix_entry` computes from the
        // frontmatter it still carries.
        let mismatched_path = canonical_path.with_file_name("mismatched.md");
        std::fs::rename(&canonical_path, &mismatched_path)
            .expect("failed to rename fixture file on disk");

        let fix_msg = methods
            .entry_fix(EntryFixParams { entry: EntryRef::Path(mismatched_path.clone()) })
            .expect("entry_fix failed");
        assert!(fix_msg.starts_with("renamed:"), "expected a rename, got: {fix_msg}");

        let calls = observed.lock().unwrap();
        assert_eq!(calls.len(), 1, "rename must notify in exactly one call");
        assert_eq!(calls[0].len(), 2, "rename must carry both paths");
        assert_eq!(calls[0][0], mismatched_path, "old path must come first");
        assert_ne!(calls[0][1], mismatched_path, "second path must be the new one");
    }

    #[test]
    fn entry_modify_without_rename_notifies_a_single_path() {
        let (_dir, methods) = test_methods();
        let (obs, observed) = observer();
        let methods = methods.with_write_observer(obs);

        let msg = methods.entry_new(new_params("Stable Title")).expect("entry_new failed");
        let path = path_from_created_message(&msg);
        observed.lock().unwrap().clear(); // isolate the modify notification below

        let mut modify = EntryModifyParams::of(EntryRef::Path(path.clone()));
        modify.tags = Some("work".to_owned());
        let modify_msg = methods.entry_modify(modify).expect("entry_modify failed");
        assert!(modify_msg.starts_with("updated:"), "expected no rename, got: {modify_msg}");

        let calls = observed.lock().unwrap();
        assert_eq!(*calls, vec![vec![path]]);
    }

    #[test]
    fn entry_remove_notifies_the_removed_path() {
        let (_dir, methods) = test_methods();
        let (obs, observed) = observer();
        let methods = methods.with_write_observer(obs);

        let msg = methods.entry_new(new_params("Doomed Entry")).expect("entry_new failed");
        let path = path_from_created_message(&msg);
        observed.lock().unwrap().clear(); // isolate the remove notification below

        methods
            .entry_remove(EntryRemoveParams { entry: EntryRef::Path(path.clone()) })
            .expect("entry_remove failed");

        let calls = observed.lock().unwrap();
        assert_eq!(*calls, vec![vec![path]]);
    }

    #[test]
    fn cache_rebuild_keeps_the_entries() {
        let (_dir, methods) = test_methods();
        methods.entry_new(new_params("Kept")).expect("entry_new failed");
        let msg = methods.cache_rebuild(EmptyParams {}).expect("cache_rebuild failed");
        assert_eq!(msg, "rebuilt: 1 entries indexed");
    }
}
