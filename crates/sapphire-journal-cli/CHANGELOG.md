# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.17.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-cli-v0.16.0...sapphire-journal-cli-v0.17.0) - 2026-11-09

### Added

- *(cli,mcp)* [**breaking**] a date without a time is stored as all day
- *(core)* [**breaking**] due and event dates keep a date-only value as all day

## [0.16.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-cli-v0.15.1...sapphire-journal-cli-v0.16.0) - 2026-11-06

### Changed

- sapphire-framework 0.28 (sapphire-turtle/sapphire-framework#402).

## [0.15.1](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-cli-v0.15.0...sapphire-journal-cli-v0.15.1) - 2026-11-04

### Other

- update Cargo.toml dependencies

## [0.15.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-cli-v0.12.0...sapphire-journal-cli-v0.15.0) - 2026-11-03

### Added

- **Breaking:** one desktop binary that runs the CLI too; headless, cli and server crates

### Changed

- **Breaking:** sapphire-framework 0.26

### Fixed

- name each edition's releases, so update installs them
