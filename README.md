# sapphire-journal

> Language: **English** | [日本語](README.ja.md)

A task, event and note management app built on [sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework) — file-based, local-first, made for human-agent collaboration.

## Concept

- **Markdown as source of truth** — all data lives in plain `.md` files you can read and edit with any tool
- **SQLite as cache** — fast querying and indexing on top of the Markdown files (planned)
- **Bullet-journal inspired** — tasks, events, and notes are equal peers, each living as its own entry; just as a bullet journal treats every bullet (task, event, or note) uniformly, sapphire-journal treats every entry the same way regardless of type
- **Text-editor / IDE compatible** — plain `.md` files with YAML frontmatter; readable and editable in any editor without special tooling
- **Human–AI collaborative editing** — designed to work alongside AI agents (Claude, etc.) that can read, create, and edit entries in the same journal via git or Syncthing sync

## Design decisions

### Entry IDs: caretta-id instead of sequential numbers

Each entry filename is prefixed with a [caretta-id](https://github.com/fluo10/caretta-id) — a 7-character BASE32 identifier with decisecond precision (e.g. `123abcd_my_note.md`).

Sequential IDs would collide when a human and an AI agent add entries at the same time in a shared journal synced via git or Syncthing.
caretta-id uses the current Unix time in deciseconds as its value, so two entries created more than 0.1 seconds apart are guaranteed to have different IDs — a collision-free guarantee without any central coordinator.

### File layout: `{year}/{id}_{slug}.md`

Entries are grouped into year directories (e.g. `2026/`) to prevent the journal root from filling up over time, while keeping the hierarchy shallow enough to stay navigable.
The slug derived from the entry title keeps filenames readable even without opening sapphire-journal.

## Data model

Each file is an **Entry** — the primary unit of data. An entry can contain free-form notes, task checkboxes (`- [ ]`), or both.

A note entry:

```markdown
---
id: '1a2b3c4'
title: Meeting notes
created_at: '2026-03-06T14:00:00'
tags: [work]
---

Discussion points from the team meeting.
```

A task entry adds a `task` block:

```markdown
---
id: '2b3c4d5'
title: Fix login bug
created_at: '2026-03-06T10:00:00'
tags: [work, backend]
task:
  status: open
  due: '2026-03-08T18:00:00'
---

Reproduction steps and notes here.
```

An event entry adds an `event` block:

```markdown
---
id: '3c4d5e6'
title: Team sync
created_at: '2026-03-06T09:00:00'
event:
  start: '2026-03-07T15:00:00'
  end: '2026-03-07T16:00:00'
---
```

## Installation

### Install script

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- journal
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) journal
```

This installs `sapphire-journal` together with `sapphire-bridge`, which it syncs
through, into `~/.local/bin`. Add `journal-desktop` for the desktop app. See the
[install script's README](https://github.com/sapphire-turtle/sapphire-framework/tree/main/install) for the options.

### cargo-binstall

```sh
cargo binstall sapphire-journal
# The desktop app is not on crates.io
cargo binstall --git https://github.com/sapphire-turtle/sapphire-journal sapphire-journal-desktop
```

### From source

```sh
cargo install sapphire-journal
```

## CLI usage

See [cli/README.md](cli/README.md) for the full command reference.

A **journal** is any directory tree that contains a `.sapphire-journal/` directory.
Use `sapphire-journal workspace init <dir>` to make one of an existing directory.

`sapphire-journal` is one binary: the journal's own commands (`entry`, `cache`, …)
sit beside [sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework)'s
(`serve`, `status`, `service`, `workspace`, `mcp`, …). Run bare (or as `serve`), it is the
app server: the one process that reads and writes the journal. It serves one journal,
indexes it for search, syncs it through the host's `sapphire-bridge`, and answers MCP.
Every other command — and the desktop app — is a client of it over IPC, working on the
journal it serves (`sapphire-journal workspace init|select <dir>` chooses it).

```sh
sapphire-journal service install        # or run `sapphire-journal serve` in a terminal
mkdir -p ~/journal
sapphire-journal workspace init ~/journal
sapphire-journal entry new --title "Hello"
```

AI agents connect with `sapphire-journal mcp` (stdio), or over HTTP at
`http://127.0.0.1:3172/mcp` once `[mcp] http = true` is set in the server's `server.toml`.
Semantic search uses the embedding model the host's `sapphire-bridge` runs
(`sapphire-journal embedding show`); without one, search is full-text.

## Project structure

```
sapphire-journal/
├── cli/                     # The sapphire-journal binary: CLI and app server
├── desktop/                 # Desktop GUI (egui)
└── crates/
    ├── sapphire-journal-core/   # Data model, Markdown parser/serializer, SQLite entry cache
    └── sapphire-journal-mcp/    # The journal's server methods and the MCP tools that call them
```

## Status

Early development — CLI and MCP server are functional for entry management including hierarchy and full-text search.

## License

This repository contains components under different licenses:

| Component | License |
|-----------|---------|
| `sapphire-journal-api` | MIT OR Apache-2.0 |
| `sapphire-journal-core` | MIT OR Apache-2.0 |
| `sapphire-journal` | MIT OR Apache-2.0 |
| `sapphire-journal-mcp` | MIT OR Apache-2.0 |
| `sapphire-journal-desktop` | GPL-3.0-or-later, with [App Store marketplace exception](desktop/LICENSE.App-Store-Exception) |

See the `LICENSE` (or `LICENSE-MIT` / `LICENSE-APACHE`) file in each component's directory for the full license text.
