# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

- Times are `jiff::Timestamp` and dates `jiff::civil::Date`, not chrono's naive types: chrono is unmaintained (RUSTSEC-2026-0543). `created_at`, `updated_at`, `started_at` and `closed_at` are `Timestamp`s; `DateOrTime` is `Date(Date)` (all day) or `Time(Timestamp)`, which was `DateTime(NaiveDateTime)`. `start_bound` and `end_bound` are `Timestamp`s, an all-day date's in this host's time zone. `time` has the helpers (`now`, `local`, `local_date`, `from_local`, `parse`, `format`).
- A time is written to the minute with this host's offset (`2026-11-08T10:00+09:00`). A time without one — every entry written before — is read in this host's time zone, and gains the offset when the entry is next written. **Journal releases before this one cannot read an entry this one has written: update every device at once.**
- `sapphire-framework-server-api` 7 (re-exported as `server`), with sapphire-framework 0.29: the IPC handshake's API version 7.

## [5.0.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-api-v4.0.0...sapphire-journal-api-v5.0.0) - 2026-11-09

### Added

- *(cli,mcp)* [**breaking**] a date without a time is stored as all day
- *(core)* [**breaking**] due and event dates keep a date-only value as all day
- *(api)* DateOrTime, a date (all day) or a date and time

## [4.0.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-api-v3.0.0...sapphire-journal-api-v4.0.0) - 2026-11-03

### Changed

- [**breaking**] `sapphire-framework-server-api` 6.0 (sapphire-framework 0.24), re-exported as `server`; `API_VERSION` 4


## [3.0.0](https://github.com/sapphire-turtle/sapphire-journal/compare/sapphire-journal-api-v2.0.0...sapphire-journal-api-v3.0.0) - 2026-10-25

### Other

- [**breaking**] follow sapphire-framework 0.22

### Changed

- [**breaking**] `sapphire-framework-server-api` 5.0 (sapphire-framework 0.22), re-exported as `server`; `API_VERSION` 3

## [2.0.0](https://github.com/sapphire-turtle/sapphire-journal/releases/tag/sapphire-journal-api-v2.0.0) - 2026-10-15

### Other

- install with the shared script; sapphire-turtle metadata
- pin the toolchain to 1.99.0; apply its fmt and clippy
- [**breaking**] split the journal's IPC API into sapphire-journal-api
