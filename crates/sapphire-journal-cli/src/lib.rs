//! `sapphire-journal`: the journal's command line, server and MCP server.
//!
//! As `serve`, it is the app server: the one process that reads and writes the
//! journal's files and its entry cache. Every other command is a client of it
//! over IPC — the journal's own verbs (`entry`, `cache`, `config`) call its
//! `sapphire-journal.*` methods, the framework's verbs (`status`, `service`,
//! `workspace`, `mcp`, …) its common ones.
//!
//! The modules are a library so the integration tests can drive them without a
//! process, and so both editions run the same commands ([`run`]): the headless
//! one (`sapphire-journal-headless`) and the desktop app, which runs them when
//! given a subcommand. Both put `sapphire-journal` on PATH.

pub mod cli;
pub mod client;
pub mod commands;

use std::path::{Path, PathBuf};

use anyhow::Result;
use sapphire_framework::server::Settings;
use sapphire_journal_core::{AppKind, JOURNAL_CTX};

use crate::cli::{Command, FrameworkCommand, FrameworkVerbs};
use crate::client::Server;
use crate::commands::config::ConfigCommand;
use sapphire_journal_server as serve;

pub use sapphire_journal_server::Edition;

/// Run one command as `edition`, exiting the process with its code when a
/// framework verb fails with one.
pub fn run(command: Command, edition: Edition) -> Result<()> {
    match command {
        // What the service manager starts.
        Command::Framework(FrameworkVerbs(FrameworkCommand::Serve)) => {
            sapphire_journal_core::init_app_context(AppKind::Server);
            runtime()?.block_on(serve::run(edition))
        }
        // Warned first: `dispatch` exits when the status is not good.
        Command::Framework(FrameworkVerbs(FrameworkCommand::Status)) => {
            warn_if_shadowed();
            dispatch(FrameworkCommand::Status, edition)
        }
        // `mcp` among them: its stdout is the MCP stream, and the
        // framework's console log goes to stderr.
        Command::Framework(FrameworkVerbs(command)) => dispatch(command, edition),
        Command::Config {
            action: ConfigCommand::Settings(command),
        } => dispatch(FrameworkCommand::Config(command), edition),
        // The journal's own verbs: requests to the running server.
        Command::Entry { action } => commands::entry::run(&Server::new()?, action),
        Command::Cache { action } => commands::cache::run(&Server::new()?, action),
        Command::Config {
            action: ConfigCommand::Show,
        } => commands::config::run(&Server::new()?),
    }
}

/// Run one of the framework's verbs other than `serve`, exiting with its code.
fn dispatch(command: FrameworkCommand, edition: Edition) -> Result<()> {
    sapphire_journal_core::init_app_context(AppKind::Cli);
    let build = |_: &Settings| serve::build_oneshot(edition);
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

/// The command both editions put on PATH.
const COMMAND: &str = "sapphire-journal";

/// Warn when the `sapphire-journal` on PATH is another executable than this
/// one: with both the headless edition and the desktop app installed, the one a shell or an
/// MCP client runs may not be the one the service was registered with.
fn warn_if_shadowed() {
    let Ok(this) = std::env::current_exe().and_then(std::fs::canonicalize) else {
        return;
    };
    let Some(on_path) = std::env::var_os("PATH").and_then(|path| find_on_path(&path)) else {
        return;
    };
    if std::fs::canonicalize(&on_path).is_ok_and(|found| found != this) {
        eprintln!(
            "warning: `{COMMAND}` on PATH is {}, not this executable ({}); \
             keep only one of the headless edition and the desktop app installed",
            on_path.display(),
            this.display(),
        );
    }
}

/// The first `sapphire-journal` executable in `path`, a `PATH` value.
fn find_on_path(path: &std::ffi::OsStr) -> Option<PathBuf> {
    let file = format!("{COMMAND}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(path)
        .map(|dir| dir.join(&file))
        .find(|candidate| Path::is_file(candidate))
}
