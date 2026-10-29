# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/sapphire-turtle/sapphire-journal/compare/desktop-v0.3.0...desktop-v0.3.1) - 2026-10-29

### Added

- *(desktop)* edit the open entry in the shell's side pane

### Other

- call the shared workflows from sapphire-turtle/.github@v1
- the filled app icon
- add the app icon and show it in the README

## [0.3.0](https://github.com/sapphire-turtle/sapphire-journal/compare/desktop-v0.2.0...desktop-v0.3.0) - 2026-10-25

### Added

- *(desktop)* remember the list's tree or flat layout
- *(desktop)* pick the list or the kanban on the rail
- let cargo binstall install the release binaries
- *(desktop)* list and kanban views with a shared entry pane
- *(desktop)* lay the window out with the framework's Shell

### Fixed

- *(desktop)* point to Settings and the status bar, not the Sync tab
- *(desktop)* use Metal on macOS instead of forcing Vulkan

### Other

- [**breaking**] follow sapphire-framework 0.22
- Merge pull request #332 from sapphire-turtle/feat/binstall
- *(desktop)* start through sapphire-framework's gui::app::run

### Changed

- sapphire-framework 0.22 and egui 0.37

## [0.2.0](https://github.com/sapphire-turtle/sapphire-journal/compare/desktop-v0.1.0...desktop-v0.2.0) - 2026-10-15

### Added

- *(mcp,core)* server methods for the CLI and the desktop app
- [**breaking**] follow sapphire-framework 0.17 ([#233](https://github.com/sapphire-turtle/sapphire-journal/pull/233)) as one sapphire-journal binary
- *(core,cli,mcp)* add stale/hidden filter with opt-in restoration (issue #288)

### Other

- install with the shared script; sapphire-turtle metadata
- run CI, cargo-deny and releases on the shared workflows
- pin the toolchain to 1.99.0; apply its fmt and clippy
- [**breaking**] split the journal's IPC API into sapphire-journal-api
- *(desktop)* [**breaking**] a client of the app server, with the framework's sync panel
- *(desktop)* rename egui show_inside to show (deprecated in egui 0.36)
- *(deps)* update rmcp to 3.2 and egui/eframe/egui_extras to 0.36
- adopt repo-layout and language conventions; add CONTRIBUTING and README.ja
