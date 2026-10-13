use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub use sapphire_framework::server::FrameworkCommand;

use crate::commands;

/// The command line of the one binary this app ships.
///
/// The journal's own verbs and the framework's sit side by side at the top
/// level. A bare invocation parses to `command: None`, which is the
/// framework's default verb, `serve` — what the service manager starts.
#[derive(Parser)]
#[command(name = "sapphire-journal", about = "Markdown-based task and note manager", version)]
pub struct Cli {
    /// Path to the journal root (the directory containing `.sapphire-journal/`).
    /// Defaults to the journal the server serves (`workspace select`).
    /// Can also be set via the SAPPHIRE_JOURNAL_DIR environment variable.
    ///
    /// Read by the journal's own verbs only: `serve` serves the workspace the
    /// server has selected, and `mcp` works on that one.
    #[arg(long, env = "SAPPHIRE_JOURNAL_DIR", global = true, value_name = "DIR")]
    pub journal_dir: Option<PathBuf>,

    /// Omit to run the server.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new journal in the given directory (defaults to current directory)
    Init {
        /// Directory to initialize (created if it does not exist)
        path: Option<PathBuf>,
    },
    /// Manage entries
    Entry {
        #[command(subcommand)]
        action: commands::entry::EntryCommand,
    },
    /// Manage the local entry cache
    Cache {
        #[command(subcommand)]
        action: commands::cache::CacheCommand,
    },
    /// Show current configuration (user-level and journal-level)
    Config {
        #[command(subcommand)]
        action: commands::config::ConfigCommand,
    },
    /// The framework's verbs: serve, status, service, workspace, workgroup,
    /// device, embedding, external-device, mcp.
    #[command(flatten)]
    Framework(FrameworkCommand),
}
