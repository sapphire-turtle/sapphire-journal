use clap::{ArgMatches, FromArgMatches, Parser, Subcommand};

pub use sapphire_framework::server::FrameworkCommand;

use crate::commands;

/// The command line of the one binary this app ships.
///
/// The journal's own verbs and the framework's sit side by side at the top
/// level. A bare invocation prints the help; the service manager starts the
/// binary with `serve`.
#[derive(Parser)]
#[command(
    name = "sapphire-journal",
    arg_required_else_help = true,
    about = "Markdown-based task and note manager",
    version
)]
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
    /// Show current configuration (user-level and journal-level), or list,
    /// get, set and unset the server's and the bridge's settings by key
    Config {
        #[command(subcommand)]
        action: commands::config::ConfigCommand,
    },
    /// The framework's verbs: serve, status, service, update, workspace,
    /// workgroup, device, embedding, external-device, mcp. Its `config` is
    /// under the journal's.
    #[command(flatten)]
    Framework(FrameworkVerbs),
}

/// The framework's verbs but `config`, whose name the journal's own `config`
/// keeps: that one holds the journal's `show` beside the framework's `list`,
/// `get`, `set` and `unset` (`ConfigCommand::Settings`).
#[derive(Debug)]
pub struct FrameworkVerbs(pub FrameworkCommand);

/// The framework verb the journal's `Command::Config` stands in for.
const TAKEN_OVER: &str = "config";

impl FromArgMatches for FrameworkVerbs {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        FrameworkCommand::from_arg_matches(matches).map(FrameworkVerbs)
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        self.0.update_from_arg_matches(matches)
    }
}

impl Subcommand for FrameworkVerbs {
    fn augment_subcommands(cmd: clap::Command) -> clap::Command {
        let framework = FrameworkCommand::augment_subcommands(clap::Command::new("framework"));
        cmd.subcommands(
            framework
                .get_subcommands()
                .filter(|verb| verb.get_name() != TAKEN_OVER)
                .cloned(),
        )
    }

    fn augment_subcommands_for_update(cmd: clap::Command) -> clap::Command {
        let framework =
            FrameworkCommand::augment_subcommands_for_update(clap::Command::new("framework"));
        cmd.subcommands(
            framework
                .get_subcommands()
                .filter(|verb| verb.get_name() != TAKEN_OVER)
                .cloned(),
        )
    }

    fn has_subcommand(name: &str) -> bool {
        name != TAKEN_OVER && FrameworkCommand::has_subcommand(name)
    }
}
