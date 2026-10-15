// SPDX-License-Identifier: GPL-3.0-or-later

//! sapphire-journal's desktop app: the journal, and the framework's `SyncPanel`.
//!
//! A pure client: it talks to the service-managed `sapphire-journal serve` and
//! `sapphire-bridge` over IPC and never starts either itself, nor opens the
//! journal. The Journal tab calls the server's `sapphire-journal.*` methods, so
//! it shows the journal the server has selected; the Sync tab picks that
//! journal and manages the workgroup. Closing the window stops nothing.

use std::sync::Arc;

use eframe::egui;
use sapphire_framework::gui::SyncPanel;
use sapphire_framework::gui::client::{AppIdentity, ClientConfig, FrameworkClient};
use sapphire_journal_mcp::JournalClient;

mod api;
mod app;
mod error;
mod fonts;
mod gpu;
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
    gpu::init_logging();
    gpu::install_panic_hook();
    // The desktop's own data directory (its UI settings); never the journal's.
    sapphire_journal_core::init_app_context(sapphire_journal_core::AppKind::Desktop);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a tokio runtime");
    let config = ClientConfig::new(APP).expect("the runtime directory and this executable's path");
    let journal_client = JournalClient::new(
        APP.app_name,
        "desktop",
        APP.version,
        config.endpoints.app.clone(),
    );

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Sapphire Journal"),
        wgpu_options: gpu::wgpu_options(),
        ..Default::default()
    };

    eframe::run_native(
        "Sapphire Journal",
        options,
        Box::new(move |cc| {
            gpu::on_render_state(cc.wgpu_render_state.as_ref());
            fonts::install(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let ctx = cc.egui_ctx.clone();
            let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(move || ctx.request_repaint());
            let framework = FrameworkClient::spawn(runtime.handle(), config, Arc::clone(&repaint));
            let (api, replies) = Api::new(runtime.handle().clone(), journal_client, repaint);
            Ok(Box::new(App::new(
                runtime,
                api,
                replies,
                SyncPanel::new(framework),
            )))
        }),
    )
}
