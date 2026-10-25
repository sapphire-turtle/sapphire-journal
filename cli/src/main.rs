use anyhow::Result;
use clap::Parser as _;
use sapphire_framework::server::Settings;
use sapphire_journal::cli::{Cli, Command, FrameworkCommand, FrameworkVerbs};
use sapphire_journal::client::Server;
use sapphire_journal::commands::config::ConfigCommand;
use sapphire_journal::{commands, serve};
use sapphire_journal_core::{AppKind, JOURNAL_CTX};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // What the service manager starts.
        Command::Framework(FrameworkVerbs(FrameworkCommand::Serve)) => {
            sapphire_journal_core::init_app_context(AppKind::Server);
            runtime()?.block_on(serve::run())
        }
        // `mcp` among them: its stdout is the MCP stream, and the
        // framework's console log goes to stderr.
        Command::Framework(FrameworkVerbs(command)) => dispatch(command),
        Command::Config {
            action: ConfigCommand::Settings(command),
        } => dispatch(FrameworkCommand::Config(command)),
        // The journal's own verbs: requests to the running server.
        Command::Entry { action } => commands::entry::run(&Server::new()?, action),
        Command::Cache { action } => commands::cache::run(&Server::new()?, action),
        Command::Config {
            action: ConfigCommand::Show,
        } => commands::config::run(&Server::new()?),
    }
}

/// Run one of the framework's verbs other than `serve`, exiting with its code.
fn dispatch(command: FrameworkCommand) -> Result<()> {
    sapphire_journal_core::init_app_context(AppKind::Cli);
    let build = |_: &Settings| serve::build_oneshot();
    let code = runtime()?.block_on(command.dispatch(
        &JOURNAL_CTX,
        serve::JournalOwner,
        build,
        serve::VERSION,
    ))?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}
