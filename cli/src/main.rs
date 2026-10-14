use anyhow::Result;
use clap::Parser as _;
use sapphire_journal::cli::{Cli, Command, FrameworkCommand};
use sapphire_journal::client::Server;
use sapphire_journal::{commands, serve};
use sapphire_journal_core::AppKind;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // What the service manager starts.
        Command::Framework(FrameworkCommand::Serve) => {
            sapphire_journal_core::init_app_context(AppKind::Server);
            runtime()?.block_on(serve::run())
        }
        // `mcp` among them: its stdout is the MCP stream, and the
        // framework's console log goes to stderr.
        Command::Framework(command) => {
            sapphire_journal_core::init_app_context(AppKind::Cli);
            let code =
                runtime()?.block_on(command.dispatch(serve::build_oneshot(), serve::VERSION))?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        // The journal's own verbs: requests to the running server.
        Command::Entry { action } => commands::entry::run(&Server::new()?, action),
        Command::Cache { action } => commands::cache::run(&Server::new()?, action),
        Command::Config { action } => commands::config::run(&Server::new()?, action),
    }
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}
