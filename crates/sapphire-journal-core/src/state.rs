//! Frontend-neutral helpers for opening a journal.

use std::path::Path;

use crate::{JournalState, error::Result, journal::Journal};

/// Open a [`JournalState`] for the journal at `root`.
pub fn open_state(root: &Path) -> Result<JournalState> {
    JournalState::open(Journal::from_root(root.to_path_buf())?)
}
