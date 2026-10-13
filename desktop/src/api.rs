//! The desktop's requests to the journal's methods on the running app server.
//!
//! The desktop never opens the journal itself: the app server is the one
//! process that reads and writes its files and its entry cache. A request runs
//! on the tokio runtime and its answer comes back as a [`Reply`] through a
//! channel the window drains every frame, so drawing never blocks on the
//! server.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use grain_id::GrainId;
use sapphire_journal_core::entry::{Entry, EntryHeader};
use sapphire_journal_core::entry_ref::EntryRef;
use sapphire_journal_core::ops::{EntryListItem, UpdateOption};
use sapphire_journal_mcp::JournalClient;
use sapphire_journal_mcp::methods::{
    ConfigSetParams, EmptyParams, EntryGetParams, EntryListParams, EntryModifyParams,
    EntryPrepareParams, EntryRemoveParams, EntrySaveParams, JournalSettings, names,
};

/// What came back from the server.
pub enum Reply {
    /// Every entry, hidden and stale ones included: the sidebar filters itself.
    Entries(Result<Vec<EntryHeader>, String>),
    /// One entry, whole, for the editor.
    Loaded(PathBuf, Result<Box<Entry>, String>),
    /// A new, empty entry was created at this path.
    Prepared(Result<PathBuf, String>),
    /// The editor's entry was written; its path now (a new title renames it).
    Saved(PathBuf, Result<PathBuf, String>),
    Removed(PathBuf, Result<(), String>),
    /// An entry's parent changed; its path now.
    Reparented(PathBuf, Result<PathBuf, String>),
    /// The entry cache was re-scanned.
    Refreshed(Result<(), String>),
    /// The settings, read or just changed.
    Settings(Result<JournalSettings, String>),
    SettingsSaved(Result<JournalSettings, String>),
}

/// A handle for sending requests; cheap to clone into each part of the UI.
#[derive(Clone)]
pub struct Api {
    runtime: tokio::runtime::Handle,
    client: Arc<JournalClient>,
    replies: mpsc::Sender<Reply>,
    repaint: Arc<dyn Fn() + Send + Sync>,
}

impl Api {
    pub fn new(
        runtime: tokio::runtime::Handle,
        client: JournalClient,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> (Api, mpsc::Receiver<Reply>) {
        let (tx, rx) = mpsc::channel();
        let api = Api {
            runtime,
            client: Arc::new(client),
            replies: tx,
            repaint,
        };
        (api, rx)
    }

    fn spawn(&self, call: impl Future<Output = Reply> + Send + 'static) {
        let tx = self.replies.clone();
        let repaint = Arc::clone(&self.repaint);
        self.runtime.spawn(async move {
            // The receiver lives as long as the window; a reply after it
            // closed has no one to tell.
            let _ = tx.send(call.await);
            repaint();
        });
    }

    pub fn list(&self) {
        let client = Arc::clone(&self.client);
        self.spawn(async move {
            let params = EntryListParams {
                include_stale: Some(true),
                include_hidden: Some(true),
                ..Default::default()
            };
            let items: Result<Vec<EntryListItem>, String> =
                client.read(names::ENTRY_LIST, params).await;
            Reply::Entries(items.map(|items| items.into_iter().map(|i| i.entry).collect()))
        });
    }

    pub fn get(&self, path: PathBuf) {
        let client = Arc::clone(&self.client);
        self.spawn(async move {
            let entry = EntryRef::Path(path.clone());
            let result = client.read(names::ENTRY_GET, EntryGetParams { entry }).await;
            Reply::Loaded(path, result.map(Box::new))
        });
    }

    pub fn prepare(&self, parent: Option<GrainId>) {
        let client = Arc::clone(&self.client);
        self.spawn(async move {
            let params = EntryPrepareParams {
                parent: parent.map(EntryRef::Id),
            };
            let result = client.call(names::ENTRY_PREPARE, params).await;
            Reply::Prepared(result.map(PathBuf::from))
        });
    }

    pub fn save(&self, entry: Entry) {
        let client = Arc::clone(&self.client);
        self.spawn(async move {
            let path = entry.path.clone();
            let result = client.call(names::ENTRY_SAVE, EntrySaveParams { entry }).await;
            Reply::Saved(path, result.map(PathBuf::from))
        });
    }

    pub fn remove(&self, path: PathBuf) {
        let client = Arc::clone(&self.client);
        self.spawn(async move {
            let entry = EntryRef::Path(path.clone());
            let result = client.call(names::ENTRY_REMOVE, EntryRemoveParams { entry }).await;
            Reply::Removed(path, result.map(|_| ()))
        });
    }

    pub fn reparent(&self, path: PathBuf, parent: Option<GrainId>) {
        let client = Arc::clone(&self.client);
        self.spawn(async move {
            let mut params = EntryModifyParams::of(EntryRef::Path(path.clone()));
            params.parent = match parent {
                Some(id) => UpdateOption::Set(EntryRef::Id(id)),
                None => UpdateOption::Clear,
            };
            // "updated: <path>" or "updated and renamed: <path>".
            let result = client.call(names::ENTRY_MODIFY, params).await.map(|text| {
                text.strip_prefix("updated and renamed: ")
                    .or_else(|| text.strip_prefix("updated: "))
                    .map(PathBuf::from)
                    .unwrap_or_else(|| path.clone())
            });
            Reply::Reparented(path, result)
        });
    }

    pub fn refresh(&self) {
        let client = Arc::clone(&self.client);
        self.spawn(async move {
            let result = client.call(names::CACHE_SYNC, EmptyParams {}).await;
            Reply::Refreshed(result.map(|_| ()))
        });
    }

    pub fn settings(&self) {
        let client = Arc::clone(&self.client);
        self.spawn(async move { Reply::Settings(client.read(names::CONFIG_GET, EmptyParams {}).await) });
    }

    pub fn set_settings(&self, params: ConfigSetParams) {
        let client = Arc::clone(&self.client);
        self.spawn(async move { Reply::SettingsSaved(client.read(names::CONFIG_SET, params).await) });
    }
}
