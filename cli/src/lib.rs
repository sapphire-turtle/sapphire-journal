//! `sapphire-journal`: the journal's one binary.
//!
//! The journal's own verbs (`init`, `entry`, `cache`, `config`) work on the
//! journal's files and its entry cache directly — `entry search` excepted,
//! which asks the server that indexes the journal. The framework's verbs
//! (`serve`, `status`, `service`, `workspace`, `mcp`, …) run the app server
//! or talk to a running one. The modules are a library so the integration
//! tests can drive them without a process.

pub mod cli;
pub mod commands;
pub mod root;
pub mod serve;
