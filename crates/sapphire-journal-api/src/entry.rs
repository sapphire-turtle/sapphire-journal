//! An entry, as the journal's files hold it and its methods carry it.

use jiff::Timestamp;

use crate::date_or_time::DateOrTime;
use grain_id::GrainId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::EntryFlag;

/// Frontmatter metadata stored at the top of each .md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter<V = serde_json::Value> {
    pub id: GrainId,

    /// Parent entry ID for hierarchical (bullet-journal nested) relationships.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<GrainId>,

    #[serde(default)]
    pub title: String,

    /// Optional slug override. If empty, the slug is derived from the title.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub slug: String,

    /// Timestamp when the entry was first created. Set automatically by `new`.
    #[serde(default, with = "crate::time::rfc3339")]
    pub created_at: Timestamp,

    /// Timestamp of the last write. Updated automatically by `write_entry`.
    #[serde(default, with = "crate::time::rfc3339")]
    pub updated_at: Timestamp,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Task metadata. Present only when this entry represents a task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskMeta<V>>,

    /// Event metadata. Present only when this entry represents a calendar event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<EventMeta<V>>,

    /// Explicitly hidden entry. `Some(true)` excludes the entry from default
    /// list/tree results (restorable via `--include-hidden`); absent or `false`
    /// is treated as normally visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,

    /// Unknown frontmatter fields preserved for round-trip compatibility: the
    /// journal's own frontmatter value on the server, JSON on the wire.
    #[serde(flatten)]
    pub extra: IndexMap<String, V>,
}

/// Task-specific metadata.
///
/// Conventional `status` values: `open`, `in_progress`, `done`, `cancelled`, `archived`.
/// Any custom string is also accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta<V = serde_json::Value> {
    /// Due date/time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<DateOrTime>,

    /// Task status. Conventional values: open | in_progress | done | cancelled | archived
    #[serde(default = "default_task_status")]
    pub status: String,

    /// Timestamp when the task was started (status → in_progress).
    /// Set automatically by `entry modify`; can be overridden manually.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::time::rfc3339::option"
    )]
    pub started_at: Option<Timestamp>,

    /// Timestamp when the task was closed (status → done/cancelled/archived).
    /// Set automatically by `entry modify`; can be overridden manually.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::time::rfc3339::option"
    )]
    pub closed_at: Option<Timestamp>,

    /// Unknown task fields preserved for round-trip compatibility.
    #[serde(flatten)]
    pub extra: IndexMap<String, V>,
}

fn default_task_status() -> String {
    "open".to_owned()
}

/// Event-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMeta<V = serde_json::Value> {
    pub start: DateOrTime,
    pub end: DateOrTime,

    /// Unknown event fields preserved for round-trip compatibility.
    #[serde(flatten)]
    pub extra: IndexMap<String, V>,
}

/// A single entry — one Markdown file in the journal.
/// Tasks and notes coexist freely in the body (bullet-journal style).
///
/// Serializable so the app server can hand a whole entry to its clients and
/// take one back (`entry_get` / `entry_save`). `V` is the value of the fields
/// the journal does not know: `serde_json::Value` for a client, the
/// frontmatter's own value in `sapphire-journal-core`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry<V = serde_json::Value> {
    /// Absolute path to the source .md file.
    pub path: PathBuf,

    /// Parsed frontmatter. Defaults to empty if the file has none.
    pub frontmatter: Frontmatter<V>,

    /// Raw Markdown body (everything after the frontmatter block).
    pub body: String,
}

impl<V> Entry<V> {
    /// Returns the GrainId from the frontmatter.
    pub fn id(&self) -> GrainId {
        self.frontmatter.id
    }

    /// Returns the title: frontmatter title (if non-empty) → file stem → "(untitled)".
    pub fn title(&self) -> &str {
        &self.frontmatter.title
    }
}

/// Read-only view of [`TaskMeta`] used for cache output and JSON serialization.
///
/// Unlike [`TaskMeta`], this type has no `extra` field and can derive [`Serialize`] cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetaView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<DateOrTime>,
    pub status: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::time::rfc3339::option"
    )]
    pub started_at: Option<Timestamp>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::time::rfc3339::option"
    )]
    pub closed_at: Option<Timestamp>,
}

impl<V> From<TaskMeta<V>> for TaskMetaView {
    fn from(t: TaskMeta<V>) -> Self {
        TaskMetaView {
            due: t.due,
            status: t.status,
            started_at: t.started_at,
            closed_at: t.closed_at,
        }
    }
}

/// Read-only view of [`EventMeta`] used for cache output and JSON serialization.
///
/// Unlike [`EventMeta`], this type has no `extra` field and can derive [`Serialize`] cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetaView {
    pub start: DateOrTime,
    pub end: DateOrTime,
}

impl<V> From<EventMeta<V>> for EventMetaView {
    fn from(e: EventMeta<V>) -> Self {
        EventMetaView {
            start: e.start,
            end: e.end,
        }
    }
}

/// Read-only view of [`Frontmatter`] used for cache output and JSON serialization.
///
/// Unlike [`Frontmatter`], this type has no `extra` field and can derive [`Serialize`] cleanly.
/// `parent_id` is serialized when set, so a client of the app server can build
/// the tree itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontmatterView {
    pub id: GrainId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<GrainId>,
    pub title: String,
    pub slug: String,
    #[serde(with = "crate::time::rfc3339")]
    pub created_at: Timestamp,
    #[serde(with = "crate::time::rfc3339")]
    pub updated_at: Timestamp,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskMetaView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<EventMetaView>,
    /// Explicitly hidden flag. `Some(true)` marks the entry as hidden;
    /// `None` (absent/false) is normally visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

impl<V> From<Frontmatter<V>> for FrontmatterView {
    fn from(fm: Frontmatter<V>) -> Self {
        FrontmatterView {
            id: fm.id,
            parent_id: fm.parent_id,
            title: fm.title,
            slug: fm.slug,
            created_at: fm.created_at,
            updated_at: fm.updated_at,
            tags: fm.tags,
            task: fm.task.map(TaskMetaView::from),
            event: fm.event.map(EventMetaView::from),
            hidden: fm.hidden,
        }
    }
}

/// Metadata-only view of an entry — path, frontmatter, and computed flags without the body.
///
/// Returned by list operations to avoid loading large bodies into memory
/// and to keep JSON output compact (e.g. for AI consumers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryHeader {
    pub path: String,
    #[serde(flatten)]
    pub frontmatter: FrontmatterView,
    /// Computed status flags (type + freshness). Set at construction time.
    pub flags: Vec<EntryFlag>,
}

impl EntryHeader {
    pub fn id(&self) -> GrainId {
        self.frontmatter.id
    }

    pub fn title(&self) -> &str {
        &self.frontmatter.title
    }
}
