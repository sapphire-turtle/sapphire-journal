//! Core library for **sapphire-journal** — a Markdown-based task and note manager that
//! keeps your data alive as plain text.
//!
//! This crate provides the data models, parsing utilities, and high-level
//! operations that are shared across the CLI and MCP frontends.
//!
//! # Modules
//!
//! - [`entry`] — Core data structures: [`entry::Entry`], [`entry::Frontmatter`],
//!   [`entry::TaskMeta`], and [`entry::EventMeta`].
//! - [`entry_ref`] — [`entry_ref::EntryRef`]: a canonical input type that
//!   accepts either a filesystem path, an exact GrainId, or a title.
//! - [`error`] — Library-wide [`error::Error`] and [`error::Result`] types.
//! - [`journal`] — The journal root, entry collection, and
//!   configuration loading.
//! - [`labels`] — Status label classification for entry types and freshness.
//! - [`ops`] — High-level entry operations (list, create, update, delete)
//!   shared across frontends.
//! - [`parser`] — Markdown + YAML frontmatter parsing and serialization.
//! - [`period`] — Time period types used to filter entries by timestamp ranges.

pub mod cache;
pub mod entry;
pub mod entry_ref;
pub mod error;
pub mod journal;
pub mod journal_state;
pub mod labels;
pub mod ops;
pub mod parser;
pub mod period;
pub mod state;
#[cfg(feature = "text-input")]
pub mod text_input;
pub mod user_config;

pub use journal_state::JournalState;
pub use sapphire_workspace::AppKind;

/// Shared application context for sapphire-journal.
///
/// Provides the app name (`"sapphire-journal"`) plus the cache, data and
/// config directories used by every [`Journal`](journal::Journal) instance.
/// Call [`init_app_context`] once at the top of `main`; forgetting it makes the
/// first call to [`Journal::cache_dir`](journal::Journal::cache_dir) panic.
pub static JOURNAL_CTX: sapphire_workspace::AppContext =
    sapphire_workspace::AppContext::new("sapphire-journal");

/// Point [`JOURNAL_CTX`] at this machine's cache, data and config directories.
///
/// The framework resolves them (`<platform root>/sapphire-journal/`, honouring
/// `SAPPHIRE_JOURNAL_{CACHE,DATA,CONFIG}_DIR`) and installs its console log
/// subscriber, which prints to stderr. Idempotent: first writer wins.
pub fn init_app_context(kind: AppKind) {
    JOURNAL_CTX.init(kind);
}
