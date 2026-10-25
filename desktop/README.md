# sapphire-journal-desktop

Cross-platform desktop GUI for [sapphire-journal](https://github.com/sapphire-turtle/sapphire-journal) — a Markdown-based task and note manager. Built with [egui](https://github.com/emilk/egui) / eframe.

The desktop app is the primary human-facing UI: the entries as a list (or tree) or a kanban of tasks, filtered by a period picked on a calendar, with an editing pane docked at the right or floating over the view. It is a client of the `sapphire-journal` server, which it reaches over IPC and never starts itself: the views on the rail (List, Kanban) show the journal the server serves, and Settings at the rail's foot chooses that journal — open a folder, create one, or bring one from the workgroup — and manages sync. Run the server with `sapphire-journal service install` (or `sapphire-journal serve`).

## Installation

The desktop crate is `publish = false`, so prebuilt binaries from the [Releases](https://github.com/sapphire-turtle/sapphire-journal/releases) page are the supported install path.

### Install script

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- journal journal-desktop
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) journal journal-desktop
```

This installs the desktop app, the `sapphire-journal` server it is a client of, and
`sapphire-bridge`, which the server syncs through, into `~/.local/bin`. See the
[install script's README](https://github.com/sapphire-turtle/sapphire-framework/tree/main/install) for the options.

### From source

```sh
cargo build --release --package sapphire-journal-desktop
# binary at target/release/sapphire-journal-desktop
```

On Linux, building requires protoc and the Wayland/xkbcommon dev packages eframe's windowing uses:

```sh
sudo apt-get install protobuf-compiler libwayland-dev libxkbcommon-dev
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

AI agents reach the journal through the server, not the desktop app: `sapphire-journal mcp` on stdio, or `http://127.0.0.1:3172/mcp` once `[http.mcp] enabled = true` is set in the server's `config.toml` (`sapphire-journal config set http.mcp.enabled=true --restart`).

## License

GPL-3.0-or-later, with an additional [App Store / marketplace compatibility exception](LICENSE.App-Store-Exception) that permits distribution through official application marketplaces (Apple App Store, Google Play, Microsoft Store, etc.) whose standard terms would otherwise conflict with GPL §6/§7.

See [LICENSE](LICENSE) for the full GPL-3.0 text.
