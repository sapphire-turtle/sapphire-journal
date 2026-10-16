// SPDX-License-Identifier: GPL-3.0-or-later

//! sapphire-journal's desktop app: the journal, and the framework's Settings.
//!
//! A pure client: it talks to the service-managed `sapphire-journal serve` and
//! `sapphire-bridge` over IPC and never starts either itself, nor opens the
//! journal. The Journal tab calls the server's `sapphire-journal.*` methods, so
//! it shows the journal the server has selected; the Sync tab picks that
//! journal and manages the workgroup. Closing the window stops nothing.

use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::eframe;
use sapphire_journal_mcp::JournalClient;

mod api;
mod app;
mod error;
mod icons;
mod screens;
mod settings;
mod widgets;

use api::Api;
use app::App;

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-journal",
    version: env!("CARGO_PKG_VERSION"),
};

fn main() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(
        AppOptions::new(APP, "Sapphire Journal")
            .inner_size([900.0, 600.0])
            .min_inner_size([600.0, 400.0]),
        |setup: AppSetup<'_>| {
            egui_extras::install_image_loaders(&setup.cc.egui_ctx);
            let journal_client =
                JournalClient::new(APP.app_name, "desktop", APP.version, setup.endpoints.app);
            let (api, replies) = Api::new(setup.runtime, journal_client, setup.repaint);
            Box::new(App::new(
                api,
                replies,
                sapphire_framework::gui::Settings::new(setup.framework),
                setup.data_dir,
            ))
        },
    )
}
