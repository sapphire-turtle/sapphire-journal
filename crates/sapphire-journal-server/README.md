# sapphire-journal-server

The app server of [sapphire-journal](https://github.com/sapphire-turtle/sapphire-journal):
the one process that reads and writes the journal's files and its entry cache, syncs it
through the host's `sapphire-bridge`, and answers the journal's methods and MCP tools.

It is a library, run as `sapphire-journal serve` by either edition
([`sapphire-journal-headless`](../../headless/README.md) or the desktop app).

## License

MIT OR Apache-2.0.
