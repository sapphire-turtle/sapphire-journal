//! Which journal the journal's own verbs work on.
//!
//! Not the working directory: a server serves one workspace (`workspace
//! select`), and the CLI follows it rather than searching upward for a
//! `.sapphire-journal/` (#310). `--journal-dir` / `SAPPHIRE_JOURNAL_DIR` names
//! another one explicitly.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use sapphire_framework::server::SelectionFile;
use sapphire_journal_core::JOURNAL_CTX;

/// The journal at `explicit`, else the one the server serves.
///
/// The server's choice is read from its selection file, so this works whether
/// or not the server is running.
pub fn resolve(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        return Ok(dir.to_path_buf());
    }
    selected()?.context(
        "no journal selected: run `sapphire-journal workspace select <dir>`, \
         or pass --journal-dir",
    )
}

/// The journal the server serves, if one is selected.
pub fn selected() -> Result<Option<PathBuf>> {
    let selection = SelectionFile::for_app(&JOURNAL_CTX)
        .load(JOURNAL_CTX.app_name)
        .context("failed to read the server's workspace selection")?;
    Ok(selection.map(|s| s.root))
}
