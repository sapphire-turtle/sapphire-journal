//! The one binary's command line: the journal's verbs and the framework's,
//! flat at the top level (#300).

use clap::Parser as _;
use sapphire_journal_cli::cli::{Cli, Command, FrameworkCommand, FrameworkVerbs};
use sapphire_journal_cli::commands::config::ConfigCommand;

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
        Command::Framework(FrameworkVerbs(FrameworkCommand::Serve))
    ));
}

#[test]
fn the_framework_verbs_sit_at_the_top_level() {
    for args in [
        &["status"][..],
        &["service", "status"],
        &["workspace", "show"],
        &["workspace", "select", "/tmp/journal"],
        &["api-key", "list"],
        // The hidden alias `api-key` keeps for this release.
        &["external-device", "list"],
        &["update", "--check"],
        &["mcp"],
    ] {
        assert!(
            matches!(parse(args).command, Command::Framework(_)),
            "{args:?} should be a framework verb"
        );
    }
}

#[test]
fn the_bridge_verbs_are_gone() {
    // sapphire-framework 0.24 moved them to `sapphire-bridge`.
    for args in [
        &["workgroup", "list"][..],
        &["device", "list"],
        &["embedding", "show"],
    ] {
        assert!(
            Cli::try_parse_from(std::iter::once("sapphire-journal").chain(args.iter().copied()))
                .is_err(),
            "{args:?} should no longer parse"
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
        Command::Config {
            action: ConfigCommand::Show
        }
    ));
}

#[test]
fn config_holds_the_journal_show_and_the_framework_settings() {
    for args in [
        &["config", "list"][..],
        &["config", "get", "http.mcp.enabled"],
        &["config", "set", "http.mcp.enabled=true"],
        &["config", "unset", "http.mcp"],
    ] {
        assert!(
            matches!(
                parse(args).command,
                Command::Config {
                    action: ConfigCommand::Settings(_)
                }
            ),
            "{args:?} should be the framework's settings verb"
        );
    }
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

fn modify_fields(args: &[&str]) -> sapphire_journal_cli::commands::entry::EntryFields {
    let mut all = vec!["entry", "modify", "X"];
    all.extend_from_slice(args);
    match parse(&all).command {
        Command::Entry {
            action: sapphire_journal_cli::commands::entry::EntryCommand::Modify { fields, .. },
        } => fields,
        _ => panic!("expected entry modify"),
    }
}

#[test]
fn due_date_only_parses_as_date() {
    let fields = modify_fields(&["--task-due", "2026-11-08"]);
    let want = chrono::NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
    assert_eq!(
        fields.task_due,
        Some(sapphire_journal_api::DateOrTime::Date(want))
    );
}

#[test]
fn event_start_with_time_parses_as_datetime() {
    let fields = modify_fields(&["--event-start", "2026-11-08T09:00"]);
    let want = chrono::NaiveDate::from_ymd_opt(2026, 11, 8)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    assert_eq!(
        fields.event_start,
        Some(sapphire_journal_api::DateOrTime::DateTime(want))
    );
}
