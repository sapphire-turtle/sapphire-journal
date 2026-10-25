//! The journal's page in the framework's Settings: the journal's
//! `.sapphire-journal/config.toml` and the host's
//! `sapphire-journal/config.toml`, read and written by the server
//! (`config_get` / `config_set`).

use std::cell::RefCell;
use std::rc::Rc;

use sapphire_framework::gui::{SettingsSection, egui};

use sapphire_journal_api::{ConfigSetParams, JournalSettings};
use sapphire_journal_core::journal::DuplicateTitlePolicy;
use sapphire_journal_core::ops::UpdateOption;

use crate::api::Api;

const DUP_TITLE_OPTIONS: &[&str] = &["allow", "warn", "error"];

/// The page's form; empty until the server's settings arrive.
pub struct SettingsPanelState {
    /// The settings as the server last reported them.
    current: Option<JournalSettings>,

    // ── Per-journal ────────────────────────────────────────────────────────
    duplicate_title: String,
    entries_dir: String,

    // ── Host ───────────────────────────────────────────────────────────────
    sync_interval_minutes: String,

    /// A save is on its way.
    saving: bool,
    pub error_msg: Option<String>,
    info_msg: Option<String>,
}

impl SettingsPanelState {
    /// An empty form; the page asks the server for the settings when it is
    /// first shown.
    pub fn open() -> Self {
        Self {
            current: None,
            duplicate_title: String::new(),
            entries_dir: String::new(),
            sync_interval_minutes: String::new(),
            saving: false,
            error_msg: None,
            info_msg: None,
        }
    }

    /// What the server reported, as it comes every few seconds: the form
    /// follows it (another journal selected, a change from the CLI) unless
    /// it holds an edit not saved yet.
    pub fn refreshed(&mut self, settings: JournalSettings) {
        if !self.edited() {
            self.loaded(settings);
        }
    }

    /// Whether the form differs from the settings it was filled from.
    fn edited(&self) -> bool {
        self.current.as_ref().is_some_and(|current| {
            let mut fresh = Self::open();
            fresh.loaded(current.clone());
            (
                &self.duplicate_title,
                &self.entries_dir,
                &self.sync_interval_minutes,
            ) != (
                &fresh.duplicate_title,
                &fresh.entries_dir,
                &fresh.sync_interval_minutes,
            )
        })
    }

    /// Fill the form from what the server reported.
    fn loaded(&mut self, settings: JournalSettings) {
        self.duplicate_title = policy_name(settings.duplicate_title).to_string();
        self.entries_dir = settings.entries_dir.clone().unwrap_or_default();
        self.sync_interval_minutes = settings
            .sync_interval_minutes
            .map(|n| n.to_string())
            .unwrap_or_else(|| "10".to_string());
        self.current = Some(settings);
    }

    /// The server's answer to a save.
    pub fn saved(&mut self, result: Result<JournalSettings, String>) {
        self.saving = false;
        match result {
            Ok(settings) => {
                let entries_dir_changed = self
                    .current
                    .as_ref()
                    .is_some_and(|c| c.entries_dir != settings.entries_dir);
                self.loaded(settings);
                self.info_msg = Some(
                    if entries_dir_changed {
                        "Saved. Refresh to see entries from the new directory."
                    } else {
                        "Saved."
                    }
                    .to_string(),
                );
            }
            Err(e) => self.error_msg = Some(format!("Failed to save: {e}")),
        }
    }

    /// The change the form asks for, or why it cannot be sent.
    fn params(&self) -> Result<ConfigSetParams, String> {
        let sync_interval_minutes =
            self.sync_interval_minutes
                .trim()
                .parse::<u32>()
                .map_err(|_| {
                    "Sync interval must be a non-negative integer (0 to disable).".to_string()
                })?;
        let duplicate_title = match self.duplicate_title.as_str() {
            "allow" => DuplicateTitlePolicy::Allow,
            "error" => DuplicateTitlePolicy::Error,
            _ => DuplicateTitlePolicy::Warn,
        };
        let entries_dir = match self.entries_dir.trim() {
            "" => UpdateOption::Clear,
            dir => UpdateOption::Set(dir.to_string()),
        };
        Ok(ConfigSetParams {
            sync_interval_minutes: Some(sync_interval_minutes),
            duplicate_title: Some(duplicate_title),
            entries_dir,
        })
    }
}

fn policy_name(policy: DuplicateTitlePolicy) -> &'static str {
    match policy {
        DuplicateTitlePolicy::Allow => "allow",
        DuplicateTitlePolicy::Warn => "warn",
        DuplicateTitlePolicy::Error => "error",
    }
}

/// The journal's page in Settings. The form is shared with the app, which
/// hands it the server's answers.
pub struct JournalSettingsPage {
    state: Rc<RefCell<SettingsPanelState>>,
    api: Api,
    /// The settings have been asked for once.
    requested: bool,
}

impl JournalSettingsPage {
    pub fn new(state: Rc<RefCell<SettingsPanelState>>, api: Api) -> Self {
        Self {
            state,
            api,
            requested: false,
        }
    }
}

impl SettingsSection for JournalSettingsPage {
    fn label(&self, _ctx: &egui::Context) -> String {
        "Journal".to_string()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if !std::mem::replace(&mut self.requested, true) {
            self.api.settings();
        }
        let mut state = self.state.borrow_mut();
        if state.current.is_none() {
            if let Some(msg) = state.error_msg.clone() {
                ui.colored_label(egui::Color32::LIGHT_RED, msg);
                if ui.button("Retry").clicked() {
                    state.error_msg = None;
                    self.api.settings();
                }
            } else {
                ui.spinner();
            }
            return;
        }

        ui.heading("This journal");
        ui.add_space(4.0);

        ui.label("Duplicate title policy");
        egui::ComboBox::from_id_salt("settings_dup_title")
            .selected_text(&state.duplicate_title)
            .show_ui(ui, |ui| {
                for v in DUP_TITLE_OPTIONS {
                    ui.selectable_value(&mut state.duplicate_title, (*v).to_string(), *v);
                }
            });
        ui.add_space(6.0);

        ui.label("Entries directory (relative to journal root; blank = root)");
        ui.add(
            egui::TextEdit::singleline(&mut state.entries_dir)
                .hint_text("entries")
                .desired_width(320.0),
        );

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        ui.heading("This computer");
        ui.weak("Takes effect when the sapphire-journal server next starts.");
        ui.add_space(4.0);

        ui.label("Re-scan interval (minutes; 0 to disable)");
        ui.add(egui::TextEdit::singleline(&mut state.sync_interval_minutes).desired_width(80.0));

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        ui.heading("MCP");
        ui.weak(
            "AI agents reach the journal through the sapphire-journal server: \
             `sapphire-journal mcp` on stdio, or http://127.0.0.1:3172/mcp once \
             `[http.mcp] enabled = true` is set in the server's config.toml.",
        );

        if let Some(msg) = state.error_msg.clone() {
            ui.add_space(8.0);
            ui.colored_label(egui::Color32::LIGHT_RED, msg);
        }
        if let Some(msg) = state.info_msg.clone() {
            ui.add_space(8.0);
            ui.colored_label(egui::Color32::LIGHT_GREEN, msg);
        }

        ui.add_space(8.0);
        if ui
            .add_enabled(!state.saving && state.edited(), egui::Button::new("Save"))
            .clicked()
        {
            state.error_msg = None;
            state.info_msg = None;
            match state.params() {
                Ok(params) => {
                    state.saving = true;
                    self.api.set_settings(params);
                }
                Err(e) => state.error_msg = Some(e),
            }
        }
    }
}
