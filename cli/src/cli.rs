use clap::{Parser, Subcommand};

pub use sapphire_framework::server::FrameworkCommand;

use crate::commands;

/// The command line of the one binary this app ships.
///
/// The journal's own verbs and the framework's sit side by side at the top
/// level. A bare invocation prints the help; the service manager starts the
/// binary with `serve`.
#[derive(Parser)]
#[command(name = "sapphire-journal",
    arg_required_else_help = true, about = "Markdown-based task and note manager", version)]
///
/// Every command but `serve` works on the journal the running server serves;
/// `workspace init|select` chooses it.
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
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
