# sapphire-journal-core

Core library for [sapphire-journal](https://github.com/sapphire-turtle/sapphire-journal) — a Markdown-based task and note manager.

## What this crate provides

- **Data model** — `Entry`, `Frontmatter`, `TaskMeta`, `EventMeta`
- **Parser / serializer** — reads and writes `.md` files with YAML frontmatter (`parse_entry`, `read_entry`, `render_entry`, `write_entry`)
- **Journal root** — `Journal::from_root()` opens a directory containing `.sapphire-journal/`, and `Journal::init()` creates one
- **Entry cache** — a machine-local SQLite cache of entry metadata and tags (`cache`, `JournalState`)
- **Filename helpers** — `slugify`, `entry_filename`, `new_entry_path` for the `{year}/{caretta-id}_{slug}.md` layout

## License

MIT OR Apache-2.0
