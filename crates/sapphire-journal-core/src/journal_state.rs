//! In-memory session state: an open journal and its entry cache.
//!
//! [`JournalState`] is the single object that frontends (CLI, MCP, GUI) hold
//! while a workspace is active.  SQLite connections are intentionally **not**
//! stored here: rusqlite recommends one connection per thread, so callers open
//! a connection with [`JournalState::open_conn`] for each operation and drop
//! it when done.
//!
//! Full-text and semantic search are not part of this state: the framework's
//! app server indexes the workspace, and embeds it through the host's
//! `sapphire-bridge`.

use crate::{cache, error::Result, journal::Journal};

/// An open journal paired with its entry cache.
pub struct JournalState {
    pub journal: Journal,
}

impl JournalState {
    /// Open the cache for `journal`, creating it if it does not yet exist.
    pub fn open(journal: Journal) -> Result<Self> {
        drop(cache::open_cache(&journal)?);
        Ok(Self { journal })
    }

    /// Drop and recreate the cache from scratch.
    pub fn rebuild(journal: Journal) -> Result<Self> {
        drop(cache::rebuild_cache(&journal)?);
        Ok(Self { journal })
    }

    /// Open a fresh SQLite connection to the cache database.
    pub fn open_conn(&self) -> Result<rusqlite::Connection> {
        cache::open_cache(&self.journal)
    }

    /// Incrementally sync the cache with the current on-disk journal state.
    pub fn sync(&self) -> Result<()> {
        let conn = self.open_conn()?;
        cache::sync_cache(&self.journal, &conn)
    }

    /// Return cache statistics (path, schema version, entry count, etc.).
    pub fn cache_info(&self) -> Result<cache::CacheInfo> {
        let conn = self.open_conn()?;
        cache::cache_info(&self.journal, &conn)
    }
}
