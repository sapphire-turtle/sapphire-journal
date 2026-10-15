//! What `entry_list` and `entry_tree` answer with.

use crate::{EntryHeader, MatchFlag};

/// A flat list item pairing an [`EntryHeader`] with its [`MatchFlag`]s.
///
/// Serializes all entry fields at the top level, with `match_flags` omitted
/// when there is no active filter (i.e. when `match_flags` is `None`).
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntryListItem {
    #[serde(flatten)]
    pub entry: EntryHeader,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_flags: Option<Vec<MatchFlag>>,
}

/// A node in an entry hierarchy returned by `entry_tree`.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntryTreeNode {
    #[serde(flatten)]
    pub entry: EntryHeader,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub match_flags: Vec<MatchFlag>,
    pub children: Vec<EntryTreeNode>,
}
