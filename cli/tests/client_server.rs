//! The clients' side of the journal's methods against a running server: what
//! the CLI and the desktop app call instead of touching the journal
//! themselves, and that the answers parse into the types they show.
//!
//! One test: `JOURNAL_CTX`'s directories are first-writer-wins per process.

use std::time::Duration;

use sapphire_framework::backend::protocol as proto;
use sapphire_framework::ipc::{self, ClientInfo, Endpoint};
use sapphire_journal_core::entry::Entry;
use sapphire_journal_core::entry_ref::EntryRef;
use sapphire_journal_core::journal::DuplicateTitlePolicy;
use sapphire_journal_core::ops::{EntryListItem, EntryTreeNode, UpdateOption};
use sapphire_journal_core::{AppKind, JOURNAL_CTX};
use sapphire_journal_mcp::JournalClient;
use sapphire_journal_mcp::methods::{
    ConfigSetParams, EmptyParams, EntryGetParams, EntryListParams, EntryPrepareParams,
    EntrySaveParams, JournalSettings, names,
};

#[tokio::test(flavor = "multi_thread")]
async fn clients_read_and_write_the_journal_through_the_server() {
    let tmp = tempfile::tempdir().unwrap();
    for cat in ["CACHE", "DATA", "CONFIG"] {
        // SAFETY: the only test in this binary that touches the environment,
        // before the server starts.
        unsafe {
            std::env::set_var(
                format!("SAPPHIRE_JOURNAL_{cat}_DIR"),
                tmp.path().join(cat.to_lowercase()),
            )
        };
    }
    // SAFETY: as above. The user config (sync interval) lives here.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path().join("xdg")) };
    sapphire_journal_core::init_app_context(AppKind::Server);
    std::fs::create_dir_all(tmp.path().join("run")).unwrap();
    let endpoint = Endpoint::in_dir(JOURNAL_CTX.app_name, tmp.path().join("run"));

    let client = JournalClient::new(JOURNAL_CTX.app_name, "cli", "0.0.0", endpoint.clone());
    let err = client.call(names::CACHE_INFO, EmptyParams {}).await.unwrap_err();
    assert!(err.contains("no sapphire-journal server is running"), "got: {err}");

    let (server, _journal) = sapphire_journal::serve::build();
    let server = server.endpoint(endpoint.clone());
    let task = tokio::spawn(async move { server.run().await.unwrap() });
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !ipc::probe(&endpoint).await.unwrap() {
        assert!(tokio::time::Instant::now() < deadline, "the server never listened");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // The framework's half: make a journal the server's workspace.
    let info = ClientInfo {
        kind: "test".into(),
        version: "0.0.0".into(),
        api: proto::API_VERSION,
        app_api: None,
        pid: std::process::id(),
    };
    let (framework, _) = ipc::connect_or_absent(&endpoint, JOURNAL_CTX.app_name, info)
        .await
        .unwrap()
        .expect("the server is listening");
    let journal = tmp.path().join("journal");
    std::fs::create_dir_all(&journal).unwrap();
    let _: proto::WorkspaceInitResult = framework
        .call(proto::WORKSPACE_INIT, proto::WorkspaceInitParams { dir: journal.clone() })
        .await
        .unwrap();

    // A new entry, read back whole, changed and saved: the desktop's editor.
    let path = client
        .call(names::ENTRY_PREPARE, EntryPrepareParams::default())
        .await
        .expect("entry_prepare");
    let mut entry: Entry = client
        .read(names::ENTRY_GET, EntryGetParams { entry: EntryRef::Path(path.clone().into()) })
        .await
        .expect("entry_get parses");
    entry.frontmatter.title = "Written Through The Server".into();
    entry.frontmatter.tags = vec!["ipc".into()];
    entry.body = "hello\n".into();
    let saved = client
        .call(names::ENTRY_SAVE, EntrySaveParams { entry })
        .await
        .expect("entry_save");
    assert_ne!(saved, path, "the title renames the file");
    assert!(saved.ends_with("_written_through_the_server.md"), "got: {saved}");
    assert!(!std::path::Path::new(&path).exists(), "the old name is gone");

    // A child, so the tree and the list carry `parent_id`.
    let parent_id = saved.rsplit('/').next().unwrap().split('_').next().unwrap().to_owned();
    client
        .call(
            names::ENTRY_PREPARE,
            EntryPrepareParams { parent: Some(EntryRef::parse(&format!("@{parent_id}"))) },
        )
        .await
        .expect("entry_prepare with a parent");

    let all = EntryListParams {
        include_stale: Some(true),
        include_hidden: Some(true),
        ..Default::default()
    };
    let items: Vec<EntryListItem> = client.read(names::ENTRY_LIST, &all).await.expect("entry_list parses");
    assert_eq!(items.len(), 2);
    let saved_item = items
        .iter()
        .find(|i| i.entry.path == saved)
        .expect("the saved entry is listed");
    assert_eq!(saved_item.entry.frontmatter.tags, ["ipc"]);
    assert!(
        items.iter().any(|i| i.entry.frontmatter.parent_id.map(|p| p.to_string()) == Some(parent_id.clone())),
        "the child's parent_id comes through"
    );
    let roots: Vec<EntryTreeNode> = client.read(names::ENTRY_TREE, &all).await.expect("entry_tree parses");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].children.len(), 1);

    // Settings: the journal's config keeps the framework's `[workspace]` table.
    let settings: JournalSettings = client.read(names::CONFIG_GET, EmptyParams {}).await.expect("config_get");
    assert_eq!(settings.root.canonicalize().unwrap(), journal.canonicalize().unwrap());
    let settings: JournalSettings = client
        .read(
            names::CONFIG_SET,
            ConfigSetParams {
                duplicate_title: Some(DuplicateTitlePolicy::Allow),
                entries_dir: UpdateOption::Clear,
                sync_interval_minutes: Some(5),
            },
        )
        .await
        .expect("config_set");
    assert_eq!(settings.duplicate_title, DuplicateTitlePolicy::Allow);
    assert_eq!(settings.sync_interval_minutes, Some(5));
    let config = std::fs::read_to_string(journal.join(".sapphire-journal/config.toml")).unwrap();
    assert!(config.contains("[workspace"), "the framework's registry survives: {config}");
    assert!(config.contains("duplicate_title = \"allow\""), "{config}");

    let cleaned = client.call(names::CACHE_CLEAN, EmptyParams {}).await.expect("cache_clean");
    assert_eq!(cleaned, "nothing to clean");

    let _: serde_json::Value = framework
        .call(ipc::SHUTDOWN_METHOD, serde_json::json!({}))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("the server stopped")
        .unwrap();
}
