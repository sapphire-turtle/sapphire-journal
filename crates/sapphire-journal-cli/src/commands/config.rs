use anyhow::Result;
use clap::Subcommand;
use sapphire_framework::server::AppConfigCommand;
use sapphire_journal_api::{self as api, EmptyParams};

use crate::client::Server;

/// `config`: the journal's `show` beside the framework's settings verbs,
/// which share the name.
#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show the current effective configuration
    ///
    /// Displays the user-level config (~/.config/sapphire-journal/config.toml)
    /// and the journal-level config (.sapphire-journal/config.toml) of the
    /// journal the server serves.
    Show,
    /// The framework's `list`, `get`, `set` and `unset`: the server's
    /// settings and the bridge's, by key.
    #[command(flatten)]
    Settings(AppConfigCommand),
}

/// Run `show`; the framework's verbs are dispatched with the framework's
/// others, in `main`.
pub fn run(server: &Server) -> Result<()> {
    println!("{}", server.call(api::CONFIG_SHOW, EmptyParams {})?);
    Ok(())
}
