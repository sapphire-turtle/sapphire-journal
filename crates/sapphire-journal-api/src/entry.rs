//! An entry, as the journal's files hold it and its methods carry it.

use grain_id::GrainId;
use chrono::NaiveDateTime;
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
    #[serde(default, with = "naive_datetime_serde")]
    pub created_at: NaiveDateTime,

    /// Timestamp of the last write. Updated automatically by `write_entry`.
    #[serde(default, with = "naive_datetime_serde")]
    pub updated_at: NaiveDateTime,

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
    #[serde(default, skip_serializing_if = "Option::is_none", with = "naive_datetime_serde::eod::opt")]
    pub due: Option<NaiveDateTime>,

    /// Task status. Conventional values: open | in_progress | done | cancelled | archived
    #[serde(default = "default_task_status")]
    pub status: String,

    /// Timestamp when the task was started (status → in_progress).
    /// Set automatically by `entry modify`; can be overridden manually.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "naive_datetime_serde::opt")]
    pub started_at: Option<NaiveDateTime>,

    /// Timestamp when the task was closed (status → done/cancelled/archived).
    /// Set automatically by `entry modify`; can be overridden manually.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "naive_datetime_serde::opt")]
    pub closed_at: Option<NaiveDateTime>,

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
    #[serde(with = "naive_datetime_serde")]
    pub start: NaiveDateTime,
    #[serde(with = "naive_datetime_serde::eod")]
    pub end: NaiveDateTime,

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
        return &self.frontmatter.title;
    }
}

/// Read-only view of [`TaskMeta`] used for cache output and JSON serialization.
///
/// Unlike [`TaskMeta`], this type has no `extra` field and can derive [`Serialize`] cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetaView {
    #[serde(default, skip_serializing_if = "Option::is_none", with = "naive_datetime_serde::eod::opt")]
    pub due: Option<NaiveDateTime>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "naive_datetime_serde::opt")]
    pub started_at: Option<NaiveDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "naive_datetime_serde::opt")]
    pub closed_at: Option<NaiveDateTime>,
}

impl<V> From<TaskMeta<V>> for TaskMetaView {
    fn from(t: TaskMeta<V>) -> Self {
        TaskMetaView { due: t.due, status: t.status, started_at: t.started_at, closed_at: t.closed_at }
    }
}

/// Read-only view of [`EventMeta`] used for cache output and JSON serialization.
///
/// Unlike [`EventMeta`], this type has no `extra` field and can derive [`Serialize`] cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetaView {
    #[serde(with = "naive_datetime_serde")]
    pub start: NaiveDateTime,
    #[serde(with = "naive_datetime_serde::eod")]
    pub end: NaiveDateTime,
}

impl<V> From<EventMeta<V>> for EventMetaView {
    fn from(e: EventMeta<V>) -> Self {
        EventMetaView { start: e.start, end: e.end }
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
    #[serde(with = "naive_datetime_serde")]
    pub created_at: NaiveDateTime,
    #[serde(with = "naive_datetime_serde")]
    pub updated_at: NaiveDateTime,
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

/// Custom serde module for `NaiveDateTime` using minute-precision format (`YYYY-MM-DDTHH:MM`).
///
/// Serializes to `%Y-%m-%dT%H:%M`. Deserializes from:
/// - `%Y-%m-%dT%H:%M`        (minute precision — preferred)
/// - `%Y-%m-%dT%H:%M:%S`     (second precision)
/// - `%Y-%m-%dT%H:%M:%S%.f`  (sub-second precision — for backward compat)
/// - `%Y-%m-%d`              (date only — `00:00` for start fields, `23:59` via `eod` sub-module)
mod naive_datetime_serde {
    use chrono::{NaiveDate, NaiveDateTime};
    use serde::{Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%dT%H:%M";

    /// Parse a datetime string, using `(h, m)` as the fallback time when only a date is given.
    pub(super) fn parse_with_fallback(s: &str, h: u32, m: u32) -> Result<NaiveDateTime, String> {
        for fmt in [FORMAT, "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M:%S%.f"] {
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
                return Ok(dt);
            }
        }
        if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return Ok(d.and_hms_opt(h, m, 0).unwrap());
        }
        Err(format!(
            "cannot parse `{s}` as a datetime; expected format YYYY-MM-DDTHH:MM"
        ))
    }

    pub fn serialize<S: Serializer>(dt: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&dt.format(FORMAT).to_string())
    }

    /// Deserializes with date-only → `00:00` (start-of-day).
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDateTime, D::Error> {
        let raw = String::deserialize(d)?;
        parse_with_fallback(&raw, 0, 0).map_err(serde::de::Error::custom)
    }

    /// For `Option<NaiveDateTime>` fields — date-only → `00:00`.
    pub mod opt {
        use chrono::NaiveDateTime;
        use serde::{Deserialize, Deserializer, Serializer};

        pub fn serialize<S: Serializer>(
            opt: &Option<NaiveDateTime>,
            s: S,
        ) -> Result<S::Ok, S::Error> {
            match opt {
                Some(dt) => super::serialize(dt, s),
                None => s.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            d: D,
        ) -> Result<Option<NaiveDateTime>, D::Error> {
            let raw = String::deserialize(d)?;
            super::parse_with_fallback(&raw, 0, 0).map(Some).map_err(serde::de::Error::custom)
        }
    }

    /// Variant where date-only → `23:59` (end-of-day). Used for due dates and event end times.
    pub mod eod {
        use chrono::NaiveDateTime;
        use serde::{Deserialize, Deserializer, Serializer};

        pub fn serialize<S: Serializer>(dt: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error> {
            super::serialize(dt, s)
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDateTime, D::Error> {
            let raw = String::deserialize(d)?;
            super::parse_with_fallback(&raw, 23, 59).map_err(serde::de::Error::custom)
        }

        pub mod opt {
            use chrono::NaiveDateTime;
            use serde::{Deserialize, Deserializer, Serializer};

            pub fn serialize<S: Serializer>(
                opt: &Option<NaiveDateTime>,
                s: S,
            ) -> Result<S::Ok, S::Error> {
                match opt {
                    Some(dt) => super::serialize(dt, s),
                    None => s.serialize_none(),
                }
            }

            pub fn deserialize<'de, D: Deserializer<'de>>(
                d: D,
            ) -> Result<Option<NaiveDateTime>, D::Error> {
                let raw = String::deserialize(d)?;
                super::super::parse_with_fallback(&raw, 23, 59)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}
