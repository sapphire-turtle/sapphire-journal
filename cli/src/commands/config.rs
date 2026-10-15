use anyhow::Result;
use clap::Subcommand;
use sapphire_journal_api::{self as api, EmptyParams};

use crate::client::Server;

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show the current effective configuration
    ///
    /// Displays the user-level config (~/.config/sapphire-journal/config.toml)
    /// and the journal-level config (.sapphire-journal/config.toml) of the
    /// journal the server serves.
    Show,
}

pub fn run(server: &Server, cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Show => println!("{}", server.call(api::CONFIG_SHOW, EmptyParams {})?),
    }
    Ok(())
}
