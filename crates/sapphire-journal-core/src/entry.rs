//! An entry, as the journal's files hold it.
//!
//! The types are `sapphire-journal-api`'s, with the unknown frontmatter fields
//! held in the frontmatter's own value rather than JSON.

use chrono::Duration;

use crate::labels::{STALE_AFTER_DAYS_DEFAULT, entry_flags};

pub use sapphire_journal_api::{EntryHeader, EventMetaView, FrontmatterView, TaskMetaView};

/// The value of a frontmatter field the journal does not know.
pub type Value = sapphire_frontmatter::Value;

/// Frontmatter metadata stored at the top of each .md file.
pub type Frontmatter = sapphire_journal_api::Frontmatter<Value>;
/// Task-specific metadata.
pub type TaskMeta = sapphire_journal_api::TaskMeta<Value>;
/// Event-specific metadata.
pub type EventMeta = sapphire_journal_api::EventMeta<Value>;
/// A single entry — one Markdown file in the journal.
pub type Entry = sapphire_journal_api::Entry<Value>;

/// The metadata-only view of `entry`, with its flags computed now.
pub fn header(entry: Entry) -> EntryHeader {
    let fm = FrontmatterView::from(entry.frontmatter);
    // 表示用フラグ。stale 閾値はリスト/ツリー絞り込み時（`EntryFilter`）に
    // 設定値を反映するため、ここでは既定の 30 日を使う。
    let flags = entry_flags(
        fm.task.as_ref(),
        fm.event.as_ref(),
        fm.hidden == Some(true),
        fm.created_at,
        fm.updated_at,
        Duration::days(STALE_AFTER_DAYS_DEFAULT as i64),
    );
    EntryHeader {
        path: entry.path.to_string_lossy().into_owned(),
        frontmatter: fm,
        flags,
    }
}
