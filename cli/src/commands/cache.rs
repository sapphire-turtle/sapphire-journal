use anyhow::{Context, Result};
use sapphire_journal_core::{cache, journal::Journal, JournalState};
use clap::Subcommand;
use std::path::Path;

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Show cache location, schema version, and entry count
    Info,

    /// Incrementally sync the cache with the current journal state
    ///
    /// This is the same sync that runs automatically before ID lookups.
    /// Useful to warm the cache explicitly or to verify it is up to date.
    Sync,

    /// Delete the current-version cache and rebuild it from scratch
    ///
    /// Only the current-version files are removed; older versions are
    /// left untouched. Use `cache clean` to remove stale versions.
    Rebuild,

    /// Remove stale cache files from previous versions
    ///
    /// Deletes old `cache.vN.db` files that were kept after a schema upgrade,
    /// and the `track_v1.redb` change-detection store older versions kept.
    /// The current version is never touched.
    Clean,
}

pub fn run(root: &Path, cmd: CacheCommand) -> Result<()> {
    let journal = Journal::from_root(root.to_path_buf())
        .context("not a sapphire-journal — run `sapphire-journal init` to initialize one")?;
    match cmd {
        CacheCommand::Info => info(&journal),
        CacheCommand::Sync => sync(&journal),
        CacheCommand::Rebuild => rebuild(&journal),
        CacheCommand::Clean => clean(&journal),
    }
}

fn info(journal: &Journal) -> Result<()> {
    let state = JournalState::open(journal.clone())?;
    let info = state.cache_info()?;
    println!("path:           {}", info.db_path.display());
    println!("schema version: v{} (app: v{})", info.schema_version, cache::SCHEMA_VERSION);
    println!("files tracked:  {}", info.file_count);
    println!("entries:        {}", info.entry_count);
    println!("unique tags:    {}", info.unique_tag_count);

    // Show stale SQLite versions if any.
    if let Ok(cache_dir) = journal.cache_dir() {
        let stale_dbs = find_stale_sqlite(&cache_dir);
        if !stale_dbs.is_empty() {
            let total = stale_dbs.iter().map(|(_, sz)| sz).sum::<u64>();
            let names: Vec<String> = stale_dbs
                .iter()
                .map(|(p, _)| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
                .collect();
            println!(
                "stale cache:    {} ({}) — run `sapphire-journal cache clean` to remove",
                names.join(", "),
                human_size(total)
            );
        }
    }

    Ok(())
}

fn sync(journal: &Journal) -> Result<()> {
    let state = JournalState::open(journal.clone())?;
    state.sync()?;
    let info = state.cache_info()?;
    println!("synced: {} entries", info.entry_count);
    Ok(())
}

fn rebuild(journal: &Journal) -> Result<()> {
    let state = JournalState::rebuild(journal.clone())?;
    state.sync()?;
    let info = state.cache_info()?;
    println!("rebuilt: {} entries indexed", info.entry_count);
    Ok(())
}

fn clean(journal: &Journal) -> Result<()> {
    let mut removed_any = false;

    // ── stale SQLite files ────────────────────────────────────────────────────
    if let Ok(cache_dir) = journal.cache_dir() {
        for (path, size) in find_stale_sqlite(&cache_dir) {
            // Remove the main file plus any WAL/SHM sidecars.
            let base = path.to_string_lossy();
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{base}{suffix}"));
            }
            println!("removed: {} ({})", path.display(), human_size(size));
            removed_any = true;
        }
        // The redb change-detection store versions before 0.13 kept; the
        // entry cache tracks its files itself now.
        let old_track = cache_dir.join(OLD_TRACK_FILE);
        if old_track.is_file() {
            let size = file_size(&old_track);
            std::fs::remove_file(&old_track)
                .with_context(|| format!("failed to remove {}", old_track.display()))?;
            println!("removed: {} ({})", old_track.display(), human_size(size));
            removed_any = true;
        }
    }

    if !removed_any {
        println!("nothing to clean");
    }
    Ok(())
}

// ── stale version discovery ───────────────────────────────────────────────────

/// The redb change-detection store older versions kept beside the entry cache.
const OLD_TRACK_FILE: &str = "track_v1.redb";

/// Return `(path, size)` for every `cache.vN.db` in `cache_dir` where N ≠ SCHEMA_VERSION.
fn find_stale_sqlite(cache_dir: &Path) -> Vec<(std::path::PathBuf, u64)> {
    let current = format!("cache_v{}.db", cache::SCHEMA_VERSION);
    let Ok(rd) = std::fs::read_dir(cache_dir) else { return Vec::new() };
    rd.filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let n = name.to_string_lossy();
            n.starts_with("cache_v") && n.ends_with(".db") && n != current.as_str()
        })
        .map(|e| {
            let p = e.path();
            let sz = file_size(&p);
            (p, sz)
        })
        .collect()
}

// ── size helpers ──────────────────────────────────────────────────────────────

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{bytes} B")
    }
}
