//! What a client says to the sapphire-journal app server, and how the server
//! answers: the journal's `sapphire-journal.*` methods, their parameters and
//! their results.
//!
//! Kept apart from the rest of the journal so that its version moves only when
//! this API does, and a client that only talks to the server links neither the
//! journal's files nor its cache through it. Its types are plain serde data:
//! the transport is the framework's IPC, the server side
//! `sapphire-journal-mcp`.
//!
//! The framework's own methods every app server serves — `workspace.*`,
//! `sync.*`, `server.info` — are [`server`]'s.

#![warn(missing_docs)]
// The types moved here from `sapphire-journal-core` with their fields as they
// were; documenting every field is left for later.
#![allow(missing_docs)]

pub use grain_id::GrainId;
pub use sapphire_server_api as server;

mod entry;
pub use entry::{
    Entry, EntryHeader, EventMeta, EventMetaView, Frontmatter, FrontmatterView, TaskMeta,
    TaskMetaView,
};

mod entry_ref;
pub use entry_ref::EntryRef;

mod flag;
pub use flag::{EntryFlag, MatchFlag};

mod list;
pub use list::{EntryListItem, EntryTreeNode};

mod params;
pub use params::{
    ConfigSetParams, DuplicateTitlePolicy, EmptyParams, EntryCheckParams, EntryFixParams,
    EntryGetParams, EntryListParams, EntryModifyParams, EntryNewParams, EntryPrepareParams,
    EntryRemoveParams, EntryResolveParams, EntrySaveParams, EntryShowParams, EntryTreeParams,
    JournalSettings,
};

mod update;
pub use update::UpdateOption;

/// The version of the journal's API: the methods below and their types,
/// reported in the IPC handshake as the app's API (`app_api`).
///
/// It is this crate's major version, parsed at compile time, so the two cannot
/// drift: a breaking change to a method, a parameter or a result here is a
/// major release of this crate, and that release is the new API version. A
/// client built against another version is refused at connect time rather
/// than mid-call.
pub const API_VERSION: u32 = parse_major(env!("CARGO_PKG_VERSION_MAJOR"));

/// `CARGO_PKG_VERSION_MAJOR` as a number. Cargo guarantees it is decimal digits.
const fn parse_major(s: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut n = 0u32;
    while i < bytes.len() {
        n = n * 10 + (bytes[i] - b'0') as u32;
        i += 1;
    }
    n
}

/// List entries matching a filter.
pub const ENTRY_LIST: &str = "sapphire-journal.entry_list";
/// The entries matching a filter, as a tree.
pub const ENTRY_TREE: &str = "sapphire-journal.entry_tree";
/// Show one entry.
pub const ENTRY_SHOW: &str = "sapphire-journal.entry_show";
/// Create an entry.
pub const ENTRY_NEW: &str = "sapphire-journal.entry_new";
/// Change an entry's fields.
pub const ENTRY_MODIFY: &str = "sapphire-journal.entry_modify";
/// Check an entry's frontmatter.
pub const ENTRY_CHECK: &str = "sapphire-journal.entry_check";
/// Fix an entry's frontmatter and file name.
pub const ENTRY_FIX: &str = "sapphire-journal.entry_fix";
/// Remove an entry.
pub const ENTRY_REMOVE: &str = "sapphire-journal.entry_remove";
/// Describe the entry cache.
pub const CACHE_INFO: &str = "sapphire-journal.cache_info";
/// Bring the entry cache up to date with the files.
pub const CACHE_SYNC: &str = "sapphire-journal.cache_sync";
/// Rebuild the entry cache from scratch.
pub const CACHE_REBUILD: &str = "sapphire-journal.cache_rebuild";

// For the CLI and the desktop app; no MCP tool.

/// One whole entry, as JSON: an [`Entry`].
pub const ENTRY_GET: &str = "sapphire-journal.entry_get";
/// The path an [`EntryRef`] names.
pub const ENTRY_RESOLVE: &str = "sapphire-journal.entry_resolve";
/// A new entry's file, created for a client to fill in.
pub const ENTRY_PREPARE: &str = "sapphire-journal.entry_prepare";
/// Write back an entry a client changed.
pub const ENTRY_SAVE: &str = "sapphire-journal.entry_save";
/// Delete the entry cache.
pub const CACHE_CLEAN: &str = "sapphire-journal.cache_clean";
/// The journal's settings, as text.
pub const CONFIG_SHOW: &str = "sapphire-journal.config_show";
/// The journal's settings, as JSON: [`JournalSettings`].
pub const CONFIG_GET: &str = "sapphire-journal.config_get";
/// Change the journal's settings; answers with the new [`JournalSettings`].
pub const CONFIG_SET: &str = "sapphire-journal.config_set";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_version_is_the_major() {
        assert_eq!(API_VERSION, 3, "the crate's major is the API version");
    }

    /// An entry's unknown fields survive the trip through JSON.
    #[test]
    fn entry_round_trips_through_json() {
        let json = r#"{
            "path": "/j/a.md",
            "frontmatter": {
                "id": "abc1234",
                "title": "A",
                "created_at": "2026-10-15T09:00",
                "updated_at": "2026-10-15T09:00",
                "task": { "status": "open", "priority": 2 },
                "mood": "good"
            },
            "body": "hi"
        }"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.frontmatter.extra["mood"], "good");
        assert_eq!(
            entry.frontmatter.task.as_ref().unwrap().extra["priority"],
            2
        );
        let back: Entry = serde_json::from_str(&serde_json::to_string(&entry).unwrap()).unwrap();
        assert_eq!(back.frontmatter.extra["mood"], "good");
    }
}
