//! MCP for sapphire-journal, on the framework's MCP (`sapphire_framework::mcp`).
//!
//! Two halves, joined by the app server's IPC:
//!
//! - [`methods`]: the journal's `sapphire-journal.*` methods, registered on the
//!   app server, working on the workspace it has selected.
//! - [`tools`]: the MCP tools an agent sees, which call those methods. The
//!   framework serves them on stdio (`sapphire-journal mcp`) and at `/mcp` on
//!   the server's HTTP listener.
//! - [`client`]: the CLI's and the desktop app's way to the same methods.

pub mod client;
pub mod methods;
pub mod tools;

pub use methods::{APP_API, JournalMethods, StateSource, WriteObserver, register};
pub use client::JournalClient;
pub use tools::{JournalTools, service};
