# sapphire-journal

Unified binary for [sapphire-journal](https://github.com/fluo10/sapphire-journal) — a Markdown-based task and note manager.
One binary is the CLI for humans, the app server that serves, indexes and syncs a journal,
and the MCP server for AI agents.

## Installation

### Install script (Linux / macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/fluo10/sapphire-journal/main/install.sh | sh
```

### Install script (Windows)

```powershell
irm https://raw.githubusercontent.com/fluo10/sapphire-journal/main/install.ps1 | iex
```

### From crates.io

```bash
cargo install sapphire-journal
```

### From source

```bash
cargo install --path .
```

### Uninstall

The CLI is a single binary and stores no global state outside your journal directories. Removing the binary is enough — your journals (`.sapphire-journal/` inside each project) are untouched.

```sh
# Linux / macOS (install script location)
rm ~/.local/bin/sapphire-journal

# Windows (PowerShell, install script location)
Remove-Item "$HOME\.local\bin\sapphire-journal.exe"

# cargo-installed
cargo uninstall sapphire-journal-cli
```

---

## CLI usage

### Run the server and choose its journal

```bash
sapphire-journal service install    # run the server as an OS service
# or: sapphire-journal serve        # run it in this terminal
sapphire-journal status

sapphire-journal workspace init /path/to/journal     # make an existing directory a journal and serve it
sapphire-journal workspace select /path/to/journal   # serve a journal that already exists
```

The server is the only process that reads and writes the journal and its cache, and it
serves one journal at a time. Every command below asks the running server and works on
the journal it serves; with no server running, they report that and exit 1. They do not
look at the current directory.

### Entry commands

#### Create a new entry

```bash
sapphire-journal entry new --title <TITLE> [--body "body text"] [OPTIONS]
```

Options:
- `--slug SLUG` — override the filename slug
- `--tags tag1,tag2` — set tags
- `--task-due DATETIME` — set task due date
- `--task-status STATUS` — set task status (`open` | `in_progress` | `done` | `cancelled` | `archived`)
- `--event-start DATETIME`, `--event-end DATETIME`

The filename is auto-generated as `{year}/{grain-id}_{slug}.md`.

#### Create and edit in $EDITOR

```bash
sapphire-journal entry edit --new
```

Opens `$EDITOR` (`$VISUAL` → `$EDITOR` → `vi`) with a pre-filled frontmatter template. On save, the filename is adjusted to match the title.

#### List entries

```bash
sapphire-journal entry list [PATH] [OPTIONS]
```

Timestamp filters (OR'd across fields):

```bash
sapphire-journal entry list [PERIOD]      # positional PERIOD applies to all timestamp fields
--task-due PERIOD             # filter by task due date
--event-span PERIOD           # filter by event span overlap (in-progress events included)
--created-at PERIOD           # filter by created_at
--updated-at PERIOD           # filter by updated_at
--all-periods                 # match entries regardless of period
```

PERIOD formats: `today` | `yesterday` | `this_week` | `last_week` | `this_month` | `last_month` | `YYYY-MM-DD` | `YYYY-MM-DD,YYYY-MM-DD` | `YYYY-MM-DDTHH:MM,YYYY-MM-DDTHH:MM`

Task state filters:

```bash
--task-overdue                # tasks whose due date is past and not closed
--task-in-progress            # tasks currently in_progress
--task-unstarted              # tasks with status open and no started_at
--active                      # composite: overdue OR in-progress
```

Other filters (AND'd on top):

```bash
--task-status open,in_progress   # comma-separated status values
--tags work,urgent               # entry must have ALL specified tags
```

Sort:

```bash
--sort-by FIELD    # id | title | task_status | created_at | updated_at | task_due | event_start | event_end
--sort-order asc   # or desc (default: asc)
```

Output:

```bash
--json   # output all matching entries as JSON (metadata + body)
```

#### Display entries as a tree

```bash
sapphire-journal entry tree [PATH] [OPTIONS]
```

Displays entries in a parent-child hierarchy based on `parent_id` in frontmatter.
Supports the same filter and sort options as `entry list`.

```bash
--json   # output the tree as JSON (nested children arrays)
```

#### Show an entry

```bash
sapphire-journal entry show <file-or-id>
```

#### Edit an entry

```bash
sapphire-journal entry edit <file-or-id>
```

Opens the entry in `$EDITOR`.

#### Update frontmatter fields

```bash
sapphire-journal entry modify <file-or-id> --title "New title"
sapphire-journal entry modify <file-or-id> --tags work,backend
sapphire-journal entry modify <file-or-id> --tags          # clear all tags
sapphire-journal entry modify <file-or-id> --task-status done
```

When `--task-status` is set to `done`, `cancelled`, or `archived`, `closed_at` is set automatically.

#### Check and fix filename

```bash
sapphire-journal entry check <file-or-id>   # report any filename/frontmatter mismatches
sapphire-journal entry fix <file-or-id>     # rename file to match frontmatter
```

#### Remove an entry

```bash
sapphire-journal entry remove <file-or-id>
```

#### Search

```bash
sapphire-journal entry search <query>              # full-text (trigram; substring / CJK friendly)
sapphire-journal entry search --semantic <query>   # vector similarity
```

Search asks the running server, which indexes the journal it serves. Semantic search
uses the embedding model of the host's `sapphire-bridge` (`sapphire-journal embedding show`
to see or change it); without one it falls back to full text.

### Cache commands

```bash
sapphire-journal cache info       # show the entry cache's status and statistics
sapphire-journal cache sync       # incrementally update the entry cache
sapphire-journal cache rebuild    # drop and rebuild the entry cache from scratch
sapphire-journal cache clean      # remove cache files of older versions
```

### Framework commands

`serve`, `status`, `service`, `workspace`, `workgroup`, `device`, `embedding`,
`external-device` and `mcp` come from [sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework);
see `sapphire-journal <command> --help`.

### DATETIME format

`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`.

For deadline/end timestamps (`--task-due`, `--event-end`), date-only input is interpreted as `23:59`.
For start/close timestamps (`--event-start`, `--task-closed-at`), date-only input is interpreted as `00:00`.

### Journal configuration

`.sapphire-journal/config.toml`:

```toml
[journal]
timezone = "Asia/Tokyo"   # IANA timezone name
# week_start is deprecated — ISO weeks (Monday start) are always used
```

---

## MCP server

AI agents (Claude, etc.) read and write journal entries over MCP. The tools run on the
app server, so it must be running (`sapphire-journal serve`, or the OS service).

- **stdio**: register `sapphire-journal mcp` with the agent. It talks to the running
  server and never starts one.
- **HTTP**: set `[mcp] http = true` in the server's `server.toml` (in the app's config
  directory); the server then answers at `http://127.0.0.1:3172/mcp` (`[http] listen`
  changes the address). Every request needs an external device's token:
  `sapphire-journal external-device add <name>`.

### Example: Claude Desktop

```json
{
  "mcpServers": {
    "sapphire-journal": {
      "command": "sapphire-journal",
      "args": ["mcp"]
    }
  }
}
```

### Available tools

| Tool | Description |
|---|---|
| `entry_list` | List entries as JSON with filtering and sorting |
| `entry_tree` | List entries as a nested JSON tree (parent-child hierarchy) |
| `entry_show` | Show the contents of an entry by ID prefix or file path |
| `entry_new` | Create a new journal entry |
| `entry_modify` | Update frontmatter fields of an existing entry |
| `entry_check` | Validate an entry's frontmatter and filename |
| `entry_fix` | Rename an entry file to match its frontmatter |
| `entry_remove` | Delete an entry file |
| `cache_info` | Show the entry cache's status and statistics |
| `cache_sync` | Incrementally update the entry cache |
| `cache_rebuild` | Drop and rebuild the entry cache from scratch |

Beside them, the framework's read-only tools: `status`, `workspace_show`,
`workspace_search` (the journal's search: full-text, and semantic when the host's bridge
embeds), `workspace_read_file`, `workspace_list_dir`, `workgroup_show` and `device_list`.

### `entry_list` parameters

Timestamp filters are **OR'd** across fields; `task_status` and `tags` are **AND'd** on top.

| Parameter | Description |
|---|---|
| `period` | Shorthand: applies to all timestamp fields |
| `task_due` | Filter by task due date |
| `event_span` | Filter by event span overlap: matches entries whose event [start, end] overlaps the period |
| `created_at` | Filter by created_at |
| `updated_at` | Filter by updated_at |
| `task_overdue` | Include tasks whose due date is past and not closed |
| `task_in_progress` | Include tasks currently in_progress |
| `task_unstarted` | Include tasks with status open and no started_at |
| `active` | Composite: overdue OR in-progress |
| `task_status` | Array of statuses to include, e.g. `["open", "in_progress"]` |
| `tags` | Array of tags; entry must have ALL specified tags |
| `sort_by` | Field to sort by: `id` \| `title` \| `task_status` \| `created_at` \| `updated_at` \| `task_due` \| `event_start` \| `event_end` |
| `sort_order` | `"asc"` (default) or `"desc"` |

PERIOD format: `today` \| `this_week` \| `this_month` \| `yesterday` \| `last_week` \| `last_month` \| `YYYY-MM-DD` \| `YYYY-MM-DD,YYYY-MM-DD` \| `YYYY-MM-DDTHH:MM,YYYY-MM-DDTHH:MM`

## License

MIT OR Apache-2.0
