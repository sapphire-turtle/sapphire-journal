use anyhow::Result;
use clap::Parser as _;
use sapphire_journal::cli::{Cli, Command, FrameworkCommand};
use sapphire_journal::{commands, root, serve};
use sapphire_journal_core::AppKind;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // A bare invocation *is* `serve`: what the service manager starts.
        None | Some(Command::Framework(FrameworkCommand::Serve)) => {
            sapphire_journal_core::init_app_context(AppKind::Server);
            runtime()?.block_on(serve::run())
        }
        // `mcp` among them: its stdout is the MCP stream, and the
        // framework's console log goes to stderr.
        Some(Command::Framework(command)) => {
            sapphire_journal_core::init_app_context(AppKind::Cli);
            let code =
                runtime()?.block_on(command.dispatch(serve::build_oneshot(), serve::VERSION))?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        Some(Command::Init { path }) => {
            sapphire_journal_core::init_app_context(AppKind::Cli);
            commands::init::run(path)
        }
        Some(Command::Entry { action }) => {
            sapphire_journal_core::init_app_context(AppKind::Cli);
            commands::entry::run(&root::resolve(cli.journal_dir.as_deref())?, action)
        }
        Some(Command::Cache { action }) => {
            sapphire_journal_core::init_app_context(AppKind::Cli);
            commands::cache::run(&root::resolve(cli.journal_dir.as_deref())?, action)
        }
        Some(Command::Config { action }) => {
            sapphire_journal_core::init_app_context(AppKind::Cli);
            let journal = match cli.journal_dir {
                Some(dir) => Some(dir),
                None => root::selected().ok().flatten(),
            };
            commands::config::run(journal.as_deref(), action)
        }
    }
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}
