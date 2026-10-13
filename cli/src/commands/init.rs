use anyhow::{Context, Result};
use sapphire_journal_core::journal::Journal;
use std::path::{Path, PathBuf};

pub fn run(path: Option<PathBuf>) -> Result<()> {
    let target = path.as_deref().unwrap_or(Path::new("."));
    if target.join(".sapphire-journal").exists() {
        anyhow::bail!("journal already initialized at {}", target.canonicalize()?.display());
    }
    Journal::init(target)
        .with_context(|| format!("failed to initialize a journal at {}", target.display()))?;

    let root = target.canonicalize()?;
    println!("initialized sapphire-journal in {}", root.display());
    println!("To serve it: sapphire-journal workspace select {}", root.display());
    Ok(())
}
