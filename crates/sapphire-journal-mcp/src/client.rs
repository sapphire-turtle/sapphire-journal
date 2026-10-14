//! A client's link to the journal's methods on the running app server: what
//! the CLI and the desktop app call instead of opening the journal themselves.
//!
//! It is dialled on the first call and again after it drops — the server may
//! start after a window opens, or restart while it is open — and a call is
//! never retried, so a write is never applied twice.

use std::sync::Arc;

use sapphire_framework::backend::protocol::API_VERSION;
use sapphire_framework::ipc::{self, Client, ClientInfo, Endpoint};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::APP_API;

pub struct JournalClient {
    app: &'static str,
    kind: &'static str,
    version: &'static str,
    endpoint: Endpoint,
    held: tokio::sync::Mutex<Option<Arc<Client>>>,
}

impl JournalClient {
    /// A client of `app`'s server at `endpoint`. `kind` (`cli`, `desktop`) and
    /// `version` are what it reports in the handshake.
    pub fn new(
        app: &'static str,
        kind: &'static str,
        version: &'static str,
        endpoint: Endpoint,
    ) -> JournalClient {
        JournalClient {
            app,
            kind,
            version,
            endpoint,
            held: tokio::sync::Mutex::new(None),
        }
    }

    /// A client of the app's server at its usual endpoint.
    pub fn for_app(
        app: &'static str,
        kind: &'static str,
        version: &'static str,
    ) -> Result<JournalClient, String> {
        let endpoint = Endpoint::for_app(app).map_err(|e| e.to_string())?;
        Ok(JournalClient::new(app, kind, version, endpoint))
    }

    /// Call `method` and return its text: a confirmation for a write, JSON
    /// for a read. The error is a message to show as it is.
    pub async fn call(&self, method: &str, params: impl Serialize) -> Result<String, String> {
        let client = self.client().await?;
        match client.call::<_, String>(method, params).await {
            Ok(text) => Ok(text),
            Err(ipc::Error::Rpc(rpc)) => Err(rpc.message),
            Err(ipc::Error::Closed | ipc::Error::Io(_)) => {
                self.forget(&client).await;
                Err(format!(
                    "the connection to the {} server was lost; try again",
                    self.app
                ))
            }
            Err(err) => Err(err.to_string()),
        }
    }

    /// Call a read method and parse the JSON it answers with.
    pub async fn read<R: DeserializeOwned>(
        &self,
        method: &str,
        params: impl Serialize,
    ) -> Result<R, String> {
        let text = self.call(method, params).await?;
        serde_json::from_str(&text).map_err(|e| format!("unexpected answer to {method}: {e}"))
    }

    async fn client(&self) -> Result<Arc<Client>, String> {
        let mut held = self.held.lock().await;
        if let Some(client) = held.as_ref().filter(|c| !c.is_closed()) {
            return Ok(Arc::clone(client));
        }
        let info = ClientInfo {
            kind: self.kind.to_owned(),
            version: self.version.to_owned(),
            api: API_VERSION,
            app_api: Some(APP_API),
            pid: std::process::id(),
        };
        let (client, _) = ipc::connect_or_absent(&self.endpoint, self.app, info)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                format!(
                    "no {} server is running; start it with `{} serve` or install its service",
                    self.app, self.app
                )
            })?;
        let client = Arc::new(client);
        *held = Some(Arc::clone(&client));
        Ok(client)
    }

    async fn forget(&self, lost: &Arc<Client>) {
        let mut held = self.held.lock().await;
        if held.as_ref().is_some_and(|h| Arc::ptr_eq(h, lost)) {
            *held = None;
        }
    }
}
