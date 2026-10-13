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
///
/// Every command but `serve` works on the journal the running server serves;
/// `workspace init|select` chooses it.
pub struct Cli {
    /// Omit to run the server.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
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
