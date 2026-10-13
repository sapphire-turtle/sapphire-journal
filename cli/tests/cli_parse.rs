//! The one binary's command line: the journal's verbs and the framework's,
//! flat at the top level (#300).

use clap::Parser as _;
use sapphire_journal::cli::{Cli, Command, FrameworkCommand};

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("sapphire-journal").chain(args.iter().copied()))
        .unwrap_or_else(|e| panic!("{args:?} should parse: {e}"))
}

#[test]
fn a_bare_invocation_is_the_server() {
    // `None` is what `main` runs as `serve`; the service manager starts the
    // binary with `serve`, which must land in the same place.
    assert!(parse(&[]).command.is_none());
    assert!(matches!(
        parse(&["serve"]).command,
        Some(Command::Framework(FrameworkCommand::Serve))
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
            matches!(parse(args).command, Some(Command::Framework(_))),
            "{args:?} should be a framework verb"
        );
    }
}

#[test]
fn the_journal_verbs_still_parse_beside_them() {
    assert!(matches!(parse(&["init", "/tmp/j"]).command, Some(Command::Init { .. })));
    assert!(matches!(
        parse(&["--journal-dir", "/tmp/j", "entry", "list", "today"]).command,
        Some(Command::Entry { .. })
    ));
    assert!(matches!(parse(&["cache", "info"]).command, Some(Command::Cache { .. })));
    assert!(matches!(parse(&["config", "show"]).command, Some(Command::Config { .. })));
}

#[test]
fn the_old_embedding_cache_command_is_gone() {
    let parsed = Cli::try_parse_from(["sapphire-journal", "cache", "embed"]);
    assert!(parsed.is_err(), "`cache embed` moved to the bridge (`embedding`)");
}
