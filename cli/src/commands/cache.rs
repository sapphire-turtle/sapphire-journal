use anyhow::Result;
use clap::Subcommand;
use sapphire_journal_api::{self as api, EmptyParams};

use crate::client::Server;

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Show cache location, schema version, and entry count
    Info,

    /// Incrementally sync the cache with the current journal state
    ///
    /// The server also does this on its own (`sync_interval_minutes`) and
    /// before every write; this runs it now.
    Sync,

    /// Delete the current-version cache and rebuild it from scratch
    Rebuild,

    /// Remove stale cache files from previous versions
    ///
    /// Deletes old `cache_vN.db` files that were kept after a schema upgrade,
    /// and the `track_v1.redb` change-detection store older versions kept.
    /// The current version is never touched.
    Clean,
}

/// Run a cache command on the server, which owns the cache.
pub fn run(server: &Server, cmd: CacheCommand) -> Result<()> {
    let method = match cmd {
        CacheCommand::Info => api::CACHE_INFO,
        CacheCommand::Sync => api::CACHE_SYNC,
        CacheCommand::Rebuild => api::CACHE_REBUILD,
        CacheCommand::Clean => api::CACHE_CLEAN,
    };
    println!("{}", server.call(method, EmptyParams {})?);
    Ok(())
}
