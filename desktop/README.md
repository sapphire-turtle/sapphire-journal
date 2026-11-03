# sapphire-journal-desktop

Cross-platform desktop GUI for [sapphire-journal](https://github.com/sapphire-turtle/sapphire-journal) — a Markdown-based task and note manager. Built with [egui](https://github.com/emilk/egui) / eframe.

The desktop app is the primary human-facing UI: the entries as a list (or tree) or a kanban of tasks, filtered by a period picked on a calendar, with an editing pane docked at the right or floating over the view. Its binary is also the CLI: run bare it opens the window, and with a subcommand (`serve`, `mcp`, `entry`, `status`, …) it does what `sapphire-journal` does. The window is a client of the `sapphire-journal` server, which it reaches over IPC and never starts itself: the views on the rail (List, Kanban) show the journal the server serves, and Settings at the rail's foot chooses that journal — open a folder, create one, or bring one from the workgroup — and manages sync. Run the server with `sapphire-journal service install` (or `sapphire-journal serve`).

## Installation

The desktop app is distributed as an installer for Windows, macOS and Linux on the [Releases](https://github.com/sapphire-turtle/sapphire-journal/releases) page (`sapphire-journal-desktop_<version>_x64-setup.exe`, `_aarch64.dmg`, `_x86_64.AppImage`, each with its minisign signature), which puts its binary on PATH as `sapphire-journal` and keeps it up to date. It is a superset of the CLI, so install it instead of the headless edition (`sapphire-journal-headless`), not beside it. The bare binaries (`sapphire-journal-desktop-<os>-<arch>`) can still be run directly. The release workflow builds the package with the `package` feature (`cargo build -p sapphire-journal-desktop --features package`), and packages it with cargo-packager.

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
# Linux / macOS
rm ~/.local/bin/sapphire-journal-desktop

# Windows (PowerShell)
Remove-Item "$HOME\.local\bin\sapphire-journal-desktop.exe"
```

The app's own preferences live in `settings.toml` under the app's data directory (`~/.local/share/sapphire-journal/` on Linux). Journals are wherever you keep them; the server remembers which one it serves.

### Journals from earlier versions

Earlier versions kept a registry of journals (`journals.toml`) and their files under `~/.local/share/sapphire-journal/journals/<uuid>/`. Those directories are untouched: open one from the **Sync** tab (or `sapphire-journal workspace select <dir>`) to serve it.

## MCP

AI agents reach the journal through the server, not the window: `sapphire-journal mcp` on stdio (the desktop app's binary runs it too), or `http://127.0.0.1:3172/mcp` once `[http.mcp] enabled = true` is set in the server's `config.toml` (`sapphire-journal config set http.mcp.enabled=true --restart`).

## License

GPL-3.0-or-later, with an additional [App Store / marketplace compatibility exception](LICENSE.App-Store-Exception) that permits distribution through official application marketplaces (Apple App Store, Google Play, Microsoft Store, etc.) whose standard terms would otherwise conflict with GPL §6/§7.

See [LICENSE](LICENSE) for the full GPL-3.0 text.
