//! The one binary's command line: the journal's verbs and the framework's,
//! flat at the top level (#300).

use clap::Parser as _;
use sapphire_journal::cli::{Cli, Command, FrameworkCommand};

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("sapphire-journal").chain(args.iter().copied()))
        .unwrap_or_else(|e| panic!("{args:?} should parse: {e}"))
}

#[test]
fn a_bare_invocation_prints_the_help() {
    // The framework's recommendation: nothing starts on a bare invocation;
    // the service manager starts the binary with `serve`.
    let err = Cli::try_parse_from(["sapphire-journal"])
        .err()
        .expect("a bare invocation does not parse");
    assert_eq!(
        err.kind(),
        clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    );
    assert!(matches!(
        parse(&["serve"]).command,
        Command::Framework(FrameworkCommand::Serve)
    ));
}

#[test]
fn the_framework_verbs_sit_at_the_top_level() {
    for args in [
        &["status"][..],
        &["service", "status"],
        &["workspace", "show"],
        &["workspace", "select", "/tmp/journal"],
        &["workgroup", "list"],
        &["device", "list"],
        &["embedding", "show"],
        &["external-device", "list"],
        &["mcp"],
    ] {
        assert!(
            matches!(parse(args).command, Command::Framework(_)),
            "{args:?} should be a framework verb"
        );
    }
}

#[test]
fn the_journal_verbs_still_parse_beside_them() {
    assert!(matches!(
        parse(&["entry", "list", "today"]).command,
        Command::Entry { .. }
    ));
    assert!(matches!(
        parse(&["cache", "info"]).command,
        Command::Cache { .. }
    ));
    assert!(matches!(
        parse(&["config", "show"]).command,
        Command::Config { .. }
    ));
}

#[test]
fn the_journal_follows_the_server_not_a_directory() {
    // The server serves one journal; `workspace init|select` picks it.
    for args in [
        &["init", "/tmp/j"][..],
        &["--journal-dir", "/tmp/j", "entry", "list", "today"],
    ] {
        let parsed =
            Cli::try_parse_from(std::iter::once("sapphire-journal").chain(args.iter().copied()));
        assert!(parsed.is_err(), "{args:?} should no longer parse");
    }
    assert!(matches!(
        parse(&["workspace", "init", "/tmp/j"]).command,
        Command::Framework(_)
    ));
}

#[test]
fn the_old_embedding_cache_command_is_gone() {
    let parsed = Cli::try_parse_from(["sapphire-journal", "cache", "embed"]);
    assert!(
        parsed.is_err(),
        "`cache embed` moved to the bridge (`embedding`)"
    );
}
