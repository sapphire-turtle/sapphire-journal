//! `sapphire-journal`: the journal's one binary.
//!
//! Run bare (or as `serve`), it is the app server: the one process that reads
//! and writes the journal's files and its entry cache. Every other command is
//! a client of it over IPC — the journal's own verbs (`entry`, `cache`,
//! `config`) call its `sapphire-journal.*` methods, the framework's verbs
//! (`status`, `service`, `workspace`, `mcp`, …) its common ones. The modules
//! are a library so the integration tests can drive them without a process.

pub mod cli;
pub mod client;
pub mod commands;
pub mod serve;
