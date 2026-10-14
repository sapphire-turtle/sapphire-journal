//! The CLI's side of the journal's methods: a blocking wrapper over
//! [`JournalClient`], for commands that run one call after another.

use anyhow::Result;
use sapphire_journal_core::JOURNAL_CTX;
use sapphire_journal_mcp::JournalClient;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The running app server, as the CLI's commands reach it.
pub struct Server {
    runtime: tokio::runtime::Runtime,
    client: JournalClient,
}

impl Server {
    /// The server at its usual endpoint. Nothing is dialled until the first call.
    pub fn new() -> Result<Server> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let client = JournalClient::for_app(JOURNAL_CTX.app_name, "cli", crate::serve::VERSION)
            .map_err(anyhow::Error::msg)?;
        Ok(Server { runtime, client })
    }

    /// Call `method` and return the text it answers with.
    pub fn call(&self, method: &str, params: impl Serialize) -> Result<String> {
        self.runtime
            .block_on(self.client.call(method, params))
            .map_err(anyhow::Error::msg)
    }

    /// Call a read method and parse the JSON it answers with.
    pub fn read<R: DeserializeOwned>(&self, method: &str, params: impl Serialize) -> Result<R> {
        self.runtime
            .block_on(self.client.read(method, params))
            .map_err(anyhow::Error::msg)
    }
}
