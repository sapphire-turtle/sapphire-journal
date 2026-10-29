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
//! that journal and manages the workgroup. Closing the window stops nothing.

use clap::Parser;
use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::eframe;
use sapphire_journal_cli::cli::Command;
use sapphire_journal_mcp::JournalClient;

mod api;
mod app;
mod error;
mod icons;
mod screens;
mod settings;
mod widgets;

use api::Api;
use app::Window;

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-journal",
    version: env!("CARGO_PKG_VERSION"),
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

fn main() -> anyhow::Result<()> {
    match DesktopCli::parse().command {
        Some(command) => sapphire_journal_cli::run(command),
        None => Ok(window()?),
    }
}

fn window() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(
        AppOptions::new(APP, "Sapphire Journal")
            .inner_size([900.0, 600.0])
            .min_inner_size([600.0, 400.0]),
        |setup: AppSetup<'_>| {
            egui_extras::install_image_loaders(&setup.cc.egui_ctx);
            let journal_client =
                JournalClient::new(APP.app_name, "desktop", APP.version, setup.endpoints.app);
            let (api, replies) = Api::new(setup.runtime, journal_client, setup.repaint);
            Box::new(Window::new(api, replies, setup.framework, setup.data_dir))
        },
    )
}
