# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.14.1](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-core-v0.14.0...sapphire-journal-core-v0.14.1) - 2026-10-29

### Other

- update Cargo.toml dependencies

## [0.14.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-core-v0.13.0...sapphire-journal-core-v0.14.0) - 2026-10-25

### Other

- [**breaking**] follow sapphire-framework 0.22

## [0.13.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-core-v0.12.0...sapphire-journal-core-v0.13.0) - 2026-10-15

### Added

- *(mcp,core)* server methods for the CLI and the desktop app
- [**breaking**] follow sapphire-framework 0.17 ([#233](https://github.com/sapphire-turtle/sapphire-journal/pull/233)) as one sapphire-journal binary
- *(core,cli,mcp)* add stale/hidden filter with opt-in restoration (issue #288)

### Fixed

- *(core,deny)* read frontmatter with the framework, not serde_yaml
- adapt cache change-detection to framework FileStamp API
- *(core)* update include_str docs/config path after crate move

### Other

- install with the shared script; sapphire-turtle metadata
- pin the toolchain to 1.99.0; apply its fmt and clippy
- [**breaking**] split the journal's IPC API into sapphire-journal-api
- Merge branch 'main' into chore/framework-0.20
- *(core)* split and render entries with the framework's frontmatter
- *(deps)* sapphire-framework 0.20
- *(core)* centralize stale threshold in STALE_AFTER_DAYS_DEFAULT and default EntryFilter to 30
- *(core)* add unit tests for stale/hidden filter gate
- *(deps)* switch sapphire-framework from git/branch to crates.io version
- adopt repo-layout and language conventions; add CONTRIBUTING and README.ja

## [0.12.0](https://github.com/fluo10/sapphire-journal/compare/core-v0.11.1...core-v0.12.0) - 2026-05-24

### Added

- *(sync)* enable periodic sync by default and surface settings to VS Code
- *(deps)* upgrade sapphire-workspace to 0.9.0

### Fixed

- *(core)* [**breaking**] initialise AppContext at startup, bump sapphire-workspace to 0.11

### Other

- *(deps)* bump sapphire-workspace to 0.12.1
- *(deps)* bump sapphire-workspace to 0.12.0
- scrub leftover Archelon identifiers in active code
- extract shared frontend helpers from mcp to core
- adopt release-plz for per-crate release cycles
- *(deps)* bump grain-id from 0.14 to 0.15 (closes #188)
- Merge pull request #189 from fluo10/dependabot/cargo/sapphire-workspace-0.10.1
- Merge pull request #185 from fluo10/docs/config-examples
