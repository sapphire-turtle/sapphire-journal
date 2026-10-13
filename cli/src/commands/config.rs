use anyhow::Result;
use sapphire_journal_core::{journal::Journal, user_config::UserConfig};
use clap::Subcommand;
use std::path::Path;

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show the current effective configuration
    ///
    /// Displays the user-level config (~/.config/sapphire-journal/config.toml) and
    /// the journal-level config (.sapphire-journal/config.toml) of the journal
    /// `--journal-dir` names, or else of the one the server serves.
    Show,
}

/// `root` is the journal whose config to show beside the user's, if there is one.
pub fn run(root: Option<&Path>, cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Show => show(root),
    }
}

fn show(root: Option<&Path>) -> Result<()> {
    // ── User config ──────────────────────────────────────────────────────────
    let user_path = UserConfig::path();
    println!("## user config");
    println!("path: {}", user_path.display());
    if user_path.exists() {
        let raw = std::fs::read_to_string(&user_path)?;
        println!("{}", raw.trim_end());
    } else {
        println!("(file not found — using defaults)");
        println!();
        println!("# To customise, create the file above with settings such as:");
        println!("#");
        println!("# sync_interval_minutes = 10   # how often the server re-scans the entry cache; 0 = never");
        println!("#");
        println!("# Embedding is configured on the host's sapphire-bridge, not here:");
        println!("# see `sapphire-journal embedding show`.");
    }

    println!();

    // ── Journal config ───────────────────────────────────────────────────────
    println!("## journal config");
    let journal = root.and_then(|dir| Journal::from_root(dir.to_path_buf()).ok());
    match journal {
        Some(ref j) => {
            let journal_config_path = j.journal_dir().join("config.toml");
            println!("path: {}", journal_config_path.display());
            if journal_config_path.exists() {
                let raw = std::fs::read_to_string(&journal_config_path)?;
                println!("{}", raw.trim_end());
            } else {
                println!("(file not found — using defaults)");
            }
        }
        None => {
            println!("(no journal: pass --journal-dir, or select one with `sapphire-journal workspace select`)");
        }
    }

    Ok(())
}
