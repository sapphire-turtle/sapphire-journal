# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-mcp-v0.1.0...sapphire-journal-mcp-v0.2.0) - 2026-10-15

### Added

- *(mcp,core)* server methods for the CLI and the desktop app
- [**breaking**] follow sapphire-framework 0.17 ([#233](https://github.com/sapphire-turtle/sapphire-journal/pull/233)) as one sapphire-journal binary
- *(core,cli,mcp)* add stale/hidden filter with opt-in restoration (issue #288)

### Other

- pin the toolchain to 1.99.0; apply its fmt and clippy
- [**breaking**] split the journal's IPC API into sapphire-journal-api
- Merge branch 'main' into chore/framework-0.19
- *(deps)* sapphire-framework 0.19, rmcp 3.5; reconnect to the bridge on its own
- adopt repo-layout and language conventions; add CONTRIBUTING and README.ja

## [0.1.0](https://github.com/fluo10/sapphire-journal/releases/tag/mcp-v0.1.0) - 2026-05-24

### Added

- *(desktop)* expose journal to AI agents via in-process HTTP MCP server

### Other

- *(release-plz)* enable mcp and reset to 0.1.0 for initial release
- *(sapphire-journal-core)* release v0.12.0
- scrub leftover Archelon identifiers in active code
- *(mcp)* convert mcp crate to lib-only and expose via cli `mcp` subcommand
- *(mcp)* drop journal open/close modes for a static tool list
- extract shared frontend helpers from mcp to core
- adopt release-plz for per-crate release cycles
- extract MCP server into sapphire-journal-mcp crate
