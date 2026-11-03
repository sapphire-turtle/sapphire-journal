// SPDX-License-Identifier: GPL-3.0-or-later

//! sapphire-journal's desktop app: the journal on the framework's shell,
//! with its Settings — and, given a subcommand, the CLI.
//!
//! This one binary is a superset of the CLI's: bare, it opens the window;
//! with `serve`, `mcp`, `entry`, `status`, … it runs the CLI's own code
//! (`sapphire_journal_cli::run`), so the service manager and MCP clients can
//! start it as they start `sapphire-journal`.
//!
//! The window is a pure client: it talks to the service-managed
//! `sapphire-journal serve` and `sapphire-bridge` over IPC and never starts
//! either itself, nor opens the journal. The Journal screen calls the server's `sapphire-journal.*`
//! methods, so it shows the journal the server has selected; Settings picks
//! that journal, manages its API keys and opens the Sapphire Bridge window for
//! the workgroup. Closing the window stops nothing.
//!
//! With the `package` feature (the release workflow's build) it is a cargo-packager package
//! (src/package.rs): it updates itself, and its first start asks to keep the app server
//! running.

// In the package, a GUI binary on Windows: no console window behind the window or the
// service. The command line goes through the console shim (src/bin/console.rs).
#![cfg_attr(all(windows, feature = "package"), windows_subsystem = "windows")]

use clap::Parser;
use corundum_framework_platform::launcher::LauncherSpec;
use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::{Settings, eframe};
use sapphire_framework::server::FrameworkCommand;
use sapphire_journal_cli::Edition;
use sapphire_journal_cli::cli::{Command, FrameworkVerbs};
use sapphire_journal_mcp::JournalClient;

mod api;
mod app;
mod error;
mod icons;
#[cfg(feature = "package")]
mod package;
mod screens;
mod settings;
mod widgets;

use api::Api;
use app::Window;

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-journal",
    version: env!("CARGO_PKG_VERSION"),
};

/// Sapphire Journal in the OS's app menu, and the window's id and icon. As the package's
/// (`[package.metadata.packager]`): `org.sapphire-turtle.sapphire-journal` is its identifier
/// and `Office` its Linux category (`Productivity`).
const LAUNCHER: LauncherSpec = LauncherSpec {
    id: "sapphire-journal",
    name: "Sapphire Journal",
    description: "Tasks, events and notes in Markdown files",
    category: "Office",
    icon_svg: include_bytes!("../assets/app-icon.svg"),
    bundle_prefix: "org.sapphire-turtle",
};

/// The CLI's command line, with the subcommand optional: none opens the window.
#[derive(Parser)]
#[command(
    name = "sapphire-journal",
    about = "Markdown-based task and note manager: the desktop app, or the CLI's commands",
    version
)]
struct DesktopCli {
    #[command(subcommand)]
    command: Option<Command>,
}

/// Whether this start goes through the package's start: every start but `serve`, the
/// package's service (the start starts the services an update stopped, and that is this
/// process). The installer's hooks (`--corundum-package <hook>`) pass arguments clap rejects,
/// so this is decided before clap may exit on them.
#[cfg_attr(not(feature = "package"), allow(dead_code))]
fn runs_package_start(parsed: &Result<DesktopCli, clap::Error>) -> bool {
    !matches!(
        parsed,
        Ok(DesktopCli {
            command: Some(Command::Framework(FrameworkVerbs(FrameworkCommand::Serve))),
        })
    )
}

fn main() -> anyhow::Result<()> {
    let parsed = DesktopCli::try_parse();
    // The package's start: an install hook runs and exits here. The window's console first,
    // so the hooks log; a command installs its own.
    #[cfg(feature = "package")]
    if runs_package_start(&parsed) {
        if !matches!(parsed, Ok(DesktopCli { command: Some(_) })) {
            sapphire_framework::gui::app::install_logging(APP);
        }
        let _ = sapphire_framework::gui::package::runtime::start(&package::PACKAGE);
    }
    match parsed.unwrap_or_else(|e| e.exit()).command {
        // In the package, the package's updater replaces it, not the desktop release file.
        #[cfg(feature = "package")]
        Some(Command::Framework(FrameworkVerbs(FrameworkCommand::Update(args))))
            if corundum_framework_platform::package::current().is_some() =>
        {
            std::process::exit(package_update(&args))
        }
        Some(command) => sapphire_journal_cli::run(command, Edition::Desktop),
        None => Ok(window()?),
    }
}

/// `update` in the package: `sapphire-bridge` first (unless `--no-bridge`), as the package's
/// own update ends this process, then the package through its updater. The exit code.
#[cfg(feature = "package")]
fn package_update(args: &sapphire_framework::server::AppUpdateArgs) -> i32 {
    let bridge = if args.no_bridge {
        0
    } else {
        match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime
                .block_on(sapphire_framework::server::update_bridge(&args.update))
                .unwrap_or_else(|err| {
                    eprintln!("sapphire-bridge: {err}");
                    1
                }),
            Err(err) => {
                eprintln!("sapphire-journal: {err}");
                1
            }
        }
    };
    let own = match package::APP_PACKAGE.updater() {
        Some(updater) => package::APP_PACKAGE.update(
            &updater,
            args.update.check,
            &mut std::io::stdout(),
            std::time::Duration::from_secs(5 * 60),
        ),
        None => 1,
    };
    own.max(bridge)
}

/// What the window opens with: the package's icon and id, and in the package, the package.
fn app_options() -> AppOptions {
    let options = AppOptions::new(APP, "Sapphire Journal")
        .inner_size([900.0, 600.0])
        .min_inner_size([600.0, 400.0])
        .launcher(LAUNCHER);
    #[cfg(feature = "package")]
    let options = options.package(package::APP_PACKAGE);
    options
}

fn window() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(app_options(), |setup: AppSetup<'_>| {
        egui_extras::install_image_loaders(&setup.cc.egui_ctx);
        let journal_client =
            JournalClient::new(APP.app_name, "desktop", APP.version, setup.endpoints.app);
        let (api, replies) = Api::new(setup.runtime, journal_client, setup.repaint);
        let settings = Settings::new(setup.framework);
        // About's service row and updates, in the package.
        #[cfg(feature = "package")]
        let settings = settings.with_package_about(setup.package_about);
        Box::new(Window::new(api, replies, settings, setup.data_dir))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The installer's hooks start the binary with arguments clap does not know; the package's
    // start must see them before clap exits on them.
    #[test]
    fn package_start_runs_before_clap_rejects() {
        let parsed = |args: &[&str]| DesktopCli::try_parse_from(args);
        assert!(runs_package_start(&parsed(&[
            "x",
            "--veloapp-install",
            "1.0.0"
        ])));
        assert!(runs_package_start(&parsed(&["x", "--version"])));
        assert!(runs_package_start(&parsed(&["x"])));
        assert!(runs_package_start(&parsed(&["x", "status"])));
        assert!(!runs_package_start(&parsed(&["x", "serve"])));
    }

    // The window shows the package's icon even outside a package, and goes by the
    // package's bundle id, so the OS groups it with the installer's entry.
    #[test]
    fn the_window_has_sapphire_journals_icon_and_id() {
        assert_eq!(LAUNCHER.id, "sapphire-journal");
        assert_eq!(LAUNCHER.name, "Sapphire Journal");
        assert_eq!(LAUNCHER.category, "Office");
        assert_eq!(LAUNCHER.bundle_prefix, "org.sapphire-turtle");
        assert_eq!(
            LAUNCHER.icon_svg,
            include_bytes!("../assets/app-icon.svg").as_slice()
        );
        assert!(corundum_framework_platform::icon::render_rgba(LAUNCHER.icon_svg, 64).is_ok());
    }
}
