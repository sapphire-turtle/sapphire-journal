//! `sapphire-journal`, the headless edition: the command line of
//! [`sapphire_journal_cli`] and nothing else. The desktop app's binary runs the
//! same commands.

use anyhow::Result;
use clap::Parser as _;
use sapphire_journal_cli::Edition;
use sapphire_journal_cli::cli::Cli;

fn main() -> Result<()> {
    sapphire_journal_cli::run(Cli::parse().command, Edition::Headless)
}
