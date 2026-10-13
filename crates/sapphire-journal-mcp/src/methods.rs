//! The journal's methods on its app server: what the MCP tools call.
//!
//! Every tool reaches the journal the way the framework's own tools reach the
//! workspace — over the app server's IPC, as `sapphire-journal.<tool>` — so one
//! implementation serves the `mcp` subcommand (stdio, dialling the running
//! server) and `/mcp` on the server's HTTP listener alike. [`register`] adds
//! the methods to the server's router; [`crate::tools`] is the other end.
//!
//! [`JournalMethods`] is the journal side of one call: an
//! `Arc<Mutex<JournalState>>`, and a function per method returning the text the
//! agent reads.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sapphire_framework::ipc::{RequestCtx, Router, RpcError};
use sapphire_journal_core::{
    cache,
    entry_ref::EntryRef,
    journal::Journal,
    ops::{self, EntryFields, EntryListItem, UpdateOption},
    parser::read_entry,
    text_input::{fields as core_fields, filter as core_filter},
    JournalState,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

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

/// The journal's methods, `sapphire-journal.<tool>`.
pub mod names {
    pub const ENTRY_LIST: &str = "sapphire-journal.entry_list";
    pub const ENTRY_TREE: &str = "sapphire-journal.entry_tree";
    pub const ENTRY_SHOW: &str = "sapphire-journal.entry_show";
    pub const ENTRY_NEW: &str = "sapphire-journal.entry_new";
    pub const ENTRY_MODIFY: &str = "sapphire-journal.entry_modify";
    pub const ENTRY_CHECK: &str = "sapphire-journal.entry_check";
    pub const ENTRY_FIX: &str = "sapphire-journal.entry_fix";
    pub const ENTRY_REMOVE: &str = "sapphire-journal.entry_remove";
    pub const CACHE_INFO: &str = "sapphire-journal.cache_info";
    pub const CACHE_SYNC: &str = "sapphire-journal.cache_sync";
    pub const CACHE_REBUILD: &str = "sapphire-journal.cache_rebuild";
}

/// The version of the methods in [`names`], reported in the IPC handshake.
///
/// Bump it when a method's parameters or meaning change incompatibly: a
/// client built against another version is refused at connect time rather
/// than mid-call.
pub const APP_API: u32 = 1;

/// Add the journal's methods to the app server's `router`, each resolving the
/// journal through `source` when it is called, and reporting the paths a
/// write touched to `on_write`.
pub fn register(router: Router, source: StateSource, on_write: Option<WriteObserver>) -> Router {
    let methods = Methods { source, on_write };
    let router = route(router, names::ENTRY_LIST, &methods, JournalMethods::entry_list);
    let router = route(router, names::ENTRY_TREE, &methods, JournalMethods::entry_tree);
    let router = route(router, names::ENTRY_SHOW, &methods, JournalMethods::entry_show);
    let router = route(router, names::ENTRY_NEW, &methods, JournalMethods::entry_new);
    let router = route(router, names::ENTRY_MODIFY, &methods, JournalMethods::entry_modify);
    let router = route(router, names::ENTRY_CHECK, &methods, JournalMethods::entry_check);
    let router = route(router, names::ENTRY_FIX, &methods, JournalMethods::entry_fix);
    let router = route(router, names::ENTRY_REMOVE, &methods, JournalMethods::entry_remove);
    let router = route(router, names::CACHE_INFO, &methods, JournalMethods::cache_info);
    let router = route(router, names::CACHE_SYNC, &methods, JournalMethods::cache_sync);
    route(router, names::CACHE_REBUILD, &methods, JournalMethods::cache_rebuild)
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

// ── parameter structs ─────────────────────────────────────────────────────────

/// Explicit empty parameter object for zero-argument tools.
///
/// Not `serde_json::Value`: that renders as `{"title":"AnyValue"}` with no
/// top-level `type`, which strict MCP clients reject — Anthropic returns
/// `tools.<N>.custom.input_schema.type: Field required` (400).
#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EmptyParams {}

#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryListParams {
    /// Period to match against timestamp fields.
    /// When no field selectors are set, the period applies to all fields (OR).
    ///
    /// Accepted formats: today | yesterday | tomorrow |
    /// this_week | last_week | next_week |
    /// this_month | last_month | next_month | none |
    /// YYYY | YYYY-MM | YYYY-Www |
    /// YYYY-MM-DD | YYYY-MM-DD/YYYY-MM-DD | YYYY-MM-DDTHH:MM/YYYY-MM-DDTHH:MM
    pub period: Option<String>,

    /// Enable all selectors at once: task_overdue, task_in_progress, event_span,
    /// created_at, updated_at. Produces a Bullet Journal-style log view when combined
    /// with a period. Individual selectors can still be set on top.
    pub active: Option<bool>,

    /// Include incomplete tasks whose due date falls within (or before) the period.
    /// Without period: include tasks whose due date is in the past and closed_at is absent.
    pub task_overdue: Option<bool>,

    /// Include incomplete tasks that were started within (or before) the period.
    /// Without period: include all tasks that have started_at set and closed_at absent.
    pub task_in_progress: Option<bool>,

    /// Include tasks that have not been started yet (started_at and closed_at both absent).
    /// Period is not applied to this filter.
    pub task_unstarted: Option<bool>,

    /// Restrict period matching to event span (overlap semantics).
    /// Without period: include entries that have an event set.
    pub event_span: Option<bool>,

    /// Restrict period matching to created_at timestamp.
    pub created_at: Option<bool>,

    /// Restrict period matching to updated_at timestamp.
    pub updated_at: Option<bool>,

    /// AND filter: include only entries whose task status matches one of these values.
    /// Provide as an array, e.g. ["open", "in_progress"]
    pub task_status: Option<Vec<String>>,

    /// AND filter: include only entries that have ALL of these tags.
    /// Provide as an array, e.g. ["work", "urgent"]
    pub tags: Option<Vec<String>>,

    /// Field to sort results by.
    /// Accepted values: id | title | task_status | created_at | updated_at | task_due | event_start | event_end
    pub sort_by: Option<String>,

    /// Sort direction: "asc" (default) or "desc"
    pub sort_order: Option<String>,

    /// Include stale (long-neglected) tasks, which are excluded from results by
    /// default. Omit or set false to keep them hidden.
    pub include_stale: Option<bool>,

    /// Include explicitly hidden entries (`hidden: true`), which are excluded
    /// from results by default. Omit or set false to keep them hidden.
    pub include_hidden: Option<bool>,
}

/// Same filter parameters as [`EntryListParams`] but for the tree tool.
pub type EntryTreeParams = EntryListParams;

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryShowParams {
    pub entry: EntryRef,
}

#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryNewParams {
    /// Title of the entry — written into the frontmatter and used to generate the filename slug
    pub title: Option<String>,
    /// Body content (Markdown)
    pub body: Option<String>,
    /// Parent entry
    pub parent: Option<EntryRef>,
    /// Slug override in the frontmatter
    pub slug: Option<String>,
    /// Tags as comma-separated string (e.g. "work,project")
    pub tags: Option<String>,
    /// Task due date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM)
    pub task_due: Option<String>,
    /// Task status (open | in_progress | done | cancelled | archived)
    pub task_status: Option<String>,
    /// Task start date/time; set automatically when status → in_progress
    pub task_started_at: Option<String>,
    /// Task close date/time
    pub task_closed_at: Option<String>,
    /// Event start date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM)
    pub event_start: Option<String>,
    /// Event end date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM)
    pub event_end: Option<String>,
    /// Hide the entry from default list/tree results (true) or reveal it (false).
    pub hidden: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryModifyParams {
    pub entry: EntryRef,
    /// New title
    pub title: Option<String>,
    /// New body content (Markdown). Replaces the existing body.
    pub body: Option<String>,
    /// New parent entry. Omit to leave unchanged; pass null to remove the parent.
    #[serde(default, skip_serializing_if = "UpdateOption::is_unchanged")]
    pub parent: UpdateOption<EntryRef>,
    /// New slug override
    pub slug: Option<String>,
    /// New tags as comma-separated string. Pass empty string to clear all tags.
    pub tags: Option<String>,
    /// Task due date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM)
    pub task_due: Option<String>,
    /// Task status (open | in_progress | done | cancelled | archived)
    pub task_status: Option<String>,
    /// Task start date/time; set automatically when status → in_progress
    pub task_started_at: Option<String>,
    /// Task close date/time
    pub task_closed_at: Option<String>,
    /// Event start date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM)
    pub event_start: Option<String>,
    /// Event end date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM)
    pub event_end: Option<String>,
    /// Hide the entry from default list/tree results (true) or reveal it (false).
    /// Omit to leave the existing value unchanged.
    pub hidden: Option<bool>,
}

impl EntryModifyParams {
    /// A modification of `entry` that changes nothing yet.
    pub fn of(entry: EntryRef) -> Self {
        Self {
            entry,
            title: None,
            body: None,
            parent: UpdateOption::Unchanged,
            slug: None,
            tags: None,
            task_due: None,
            task_status: None,
            task_started_at: None,
            task_closed_at: None,
            event_start: None,
            event_end: None,
            hidden: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryCheckParams {
    pub entry: EntryRef,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryFixParams {
    pub entry: EntryRef,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryRemoveParams {
    pub entry: EntryRef,
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

impl<'a> From<&'a EntryListParams> for core_filter::FilterInputs<'a> {
    fn from(p: &'a EntryListParams) -> Self {
        Self {
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
}

// ── method implementations ────────────────────────────────────────────────────

impl JournalMethods {
    pub fn entry_list(&self, p: EntryListParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let filter = core_filter::build_filter(core_filter::FilterInputs::from(&p))?;
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
            let filter = core_filter::build_filter(core_filter::FilterInputs::from(&p))?;
            let entries = self.with_state(|s| ops::list_entries(s, &filter).map_err(Into::into))?;
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
            self.notify_write(&[dest.clone()]);
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
            let path = self.with_state(|s| {
                let conn = s.open_conn()?;
                ops::resolve_entry(&p.entry, &conn).map_err(Into::into)
            })?;
            match ops::fix_entry(&path)? {
                Some(new_path) => {
                    self.notify_write(&[path.clone(), new_path.clone()]);
                    Ok(format!(
                        "renamed: {} → {}",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        new_path.file_name().unwrap_or_default().to_string_lossy(),
                    ))
                }
                None => Ok(format!("ok: {} (already correct)", path.display())),
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
            self.notify_write(&[path.clone()]);
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

#[cfg(test)]
mod tests {
    use super::*;

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
