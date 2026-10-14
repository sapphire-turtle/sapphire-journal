use anyhow::Result;
use clap::Subcommand;
use sapphire_journal_mcp::methods::{EmptyParams, names};

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
        ConfigCommand::Show => println!("{}", server.call(names::CONFIG_SHOW, EmptyParams {})?),
    }
    Ok(())
}
