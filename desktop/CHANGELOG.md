# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The period at the top of the window is picked with a calendar popup, and ◀ ▶ beside it step it back and forward by a day, week, month or year.
- Settings → Desktop sets the period the window opens on: the last one shown (the default), today, this week or this month.
- A "New entry" button on the rail opens a new entry in the side pane from any view.
- Due, start and end in the side pane have an "All day" switch.

### Changed

- **Breaking:** an event's start and end and a task's due take either a date, meaning all day, or a date and time. `--task-due 2026-11-08` and `--event-start 2026-11-08` (and the MCP tools' `task_due`, `event_start`, `event_end`) are now kept as the date alone, written `due: "2026-11-08"`, not turned into `T23:59` or `T00:00`. A date-only due is overdue from the day after. JSON output gives such a value as `"2026-11-08"`. Entries already written with times are kept as they are. The cache is rebuilt once on first start.

## [0.16.0](https://github.com/sapphire-turtle/sapphire-journal/compare/v0.15.1...v0.16.0) - 2026-11-06

### Added

- On macOS, the app bundle added to the app menu has the mark on the brand's tile as its icon, as the package's icns, rather than the bare mark on the system's plate.

### Changed

- **Breaking:** sapphire-framework 0.28, corundum-framework 0.11: `update` and the window's update panel install only a release file signed with sapphire-turtle's key, and refuse one without a valid signature (sapphire-turtle/sapphire-framework#402).
- Each release file — the bare binaries for servers, the install script and `cargo binstall` — comes with its `<file>.sig`, made with sapphire-turtle's minisign key, which `update` checks (sapphire-turtle/sapphire-framework#402).

### Fixed

- On Windows, `sapphire-journal-desktop.exe` carries the app icon, in the brand colour, so Explorer and the taskbar show Sapphire Journal's icon rather than a generic one.

## [0.15.1](https://github.com/sapphire-turtle/sapphire-journal/compare/v0.15.0...v0.15.1) - 2026-11-04

### Fixed

- On Windows, keeping the app server running (the first start's offer, or `service install`) no longer fails with "access is denied": the service is now a logon task for the installing user, which needs no administrator (sapphire-framework 0.27, corundum 0.9, sapphire-turtle/corundum-framework#105).
- On Windows, applying an update from the app's window installs it and starts the app again; before, the app only closed and was not updated (sapphire-framework 0.27.1, corundum 0.9.1, sapphire-turtle/corundum-framework#109). An app at v0.15.0 or earlier still hits this when it applies this update: install v0.15.1 by hand once, from its `_x64-setup.exe` ([#354](https://github.com/sapphire-turtle/sapphire-journal/pull/354))

## [0.15.0](https://github.com/sapphire-turtle/sapphire-journal/compare/desktop-v0.3.0...v0.15.0) - 2026-11-03

### Added

- **Breaking:** Sapphire Journal is distributed as an installer per OS, made with cargo-packager ([#353](https://github.com/sapphire-turtle/sapphire-journal/pull/353)): `sapphire-journal-desktop_<version>_x64-setup.exe`, `_aarch64.dmg` and `_x86_64.AppImage`. The installed app updates itself from the latest release, checking each update's signature first.
- **Breaking:** one desktop binary that runs the CLI too; headless, cli and server crates
- edit the open entry in the shell's side pane
- the app icon

### Changed

- [**breaking**] follow sapphire-framework 0.24: Settings has an API keys page (keys kept in the journal, each with a profile; existing keys become read-only) and links to the Sapphire Bridge window for the workgroup, devices and embedding.
- **Breaking:** sapphire-framework 0.26: `workspace init` and `select` without a directory use the app's default workspace, `<data dir>/workspaces/default`, not the current directory; pass `.` for the old behaviour.

### Fixed

- name each edition's releases, so update installs them

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
