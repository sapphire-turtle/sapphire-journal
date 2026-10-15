//! The parameters of the journal's methods, and the settings they read and change.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Entry, EntryRef, UpdateOption};

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

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryGetParams {
    pub entry: EntryRef,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryResolveParams {
    pub entry: EntryRef,
}

#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntryPrepareParams {
    /// Parent of the new entry, if any.
    pub parent: Option<EntryRef>,
}

/// A whole entry to write back, as `entry_get` returned it and the client
/// changed it. The file must already exist inside the journal.
///
/// `V` is [`Entry`]'s: the server reads the unknown fields straight into its
/// own frontmatter value.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(serialize = "V: Serialize", deserialize = "V: Deserialize<'de>"))]
pub struct EntrySaveParams<V = serde_json::Value> {
    pub entry: Entry<V>,
}

/// The settings a client may read and change: `sync_interval_minutes` from the
/// host's user config, the rest from the journal's `.sapphire-journal/config.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalSettings {
    /// The journal the server serves.
    pub root: PathBuf,
    pub sync_interval_minutes: Option<u32>,
    pub duplicate_title: DuplicateTitlePolicy,
    pub entries_dir: Option<String>,
}

/// A change to [`JournalSettings`]; an absent field stays as it is.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigSetParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_interval_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_title: Option<DuplicateTitlePolicy>,
    /// `null` clears it: entries live at the journal root.
    #[serde(default, skip_serializing_if = "UpdateOption::is_unchanged")]
    pub entries_dir: UpdateOption<String>,
}

/// Controls how duplicate entry titles are treated during cache sync.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateTitlePolicy {
    /// Duplicates are silently allowed.
    Allow,
    /// Print a warning to stderr for each duplicate title, but continue.
    #[default]
    Warn,
    /// Abort sync with a duplicate-title error on the first duplicate found.
    Error,
}
