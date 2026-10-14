# sapphire-journal-desktop

Cross-platform desktop GUI for [sapphire-journal](https://github.com/fluo10/sapphire-journal) — a Markdown-based task and note manager. Built with [egui](https://github.com/emilk/egui) / eframe.

The desktop app is the primary human-facing UI: an editor, tree view, calendar and settings panel for working with entries. It is a client of the `sapphire-journal` server, which it reaches over IPC and never starts itself: the **Journal** tab shows the journal the server serves, and the **Sync** tab (sapphire-framework's sync panel) chooses that journal — open a folder, create one, or bring one from the workgroup — and manages sync. Run the server with `sapphire-journal service install` (or `sapphire-journal serve`).

## Installation

The desktop crate is `publish = false`, so prebuilt binaries from the [Releases](https://github.com/fluo10/sapphire-journal/releases) page are the supported install path.

### Install script (Linux / macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/fluo10/sapphire-journal/main/desktop/install.sh | sh
```

### Install script (Windows)

```powershell
irm https://raw.githubusercontent.com/fluo10/sapphire-journal/main/desktop/install.ps1 | iex
```

Both scripts download the latest `desktop-v*` release asset for your platform into `~/.local/bin/` (Windows: `%USERPROFILE%\.local\bin\`) and add the directory to `PATH` if needed.

### From source

```sh
cargo build --release --package sapphire-journal-desktop
# binary at target/release/sapphire-journal-desktop
```

On Linux, building requires the GTK/WebKit dev packages:

```sh
sudo apt-get install protobuf-compiler libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libxdo-dev
```

### Uninstall

Remove the installed binary:

```sh
# Linux / macOS (install script location)
rm ~/.local/bin/sapphire-journal-desktop

# Windows (PowerShell, install script location)
Remove-Item "$HOME\.local\bin\sapphire-journal-desktop.exe"
```

The app's own preferences live in `settings.toml` under the app's data directory (`~/.local/share/sapphire-journal/` on Linux). Journals are wherever you keep them; the server remembers which one it serves.

### Journals from earlier versions

Earlier versions kept a registry of journals (`journals.toml`) and their files under `~/.local/share/sapphire-journal/journals/<uuid>/`. Those directories are untouched: open one from the **Sync** tab (or `sapphire-journal workspace select <dir>`) to serve it.

## MCP

AI agents reach the journal through the server, not the desktop app: `sapphire-journal mcp` on stdio, or `http://127.0.0.1:3172/mcp` once `[mcp] http = true` is set in the server's `server.toml`.

## License

GPL-3.0-or-later, with an additional [App Store / marketplace compatibility exception](LICENSE.App-Store-Exception) that permits distribution through official application marketplaces (Apple App Store, Google Play, Microsoft Store, etc.) whose standard terms would otherwise conflict with GPL §6/§7.

See [LICENSE](LICENSE) for the full GPL-3.0 text.
