//! The desktop's own page in the framework's Settings: what the window opens
//! on. It writes `settings.toml` as soon as a choice changes.

use std::cell::RefCell;
use std::rc::Rc;

use sapphire_framework::gui::{SettingsSection, egui};

use crate::settings::{Settings, StartupPeriod};

/// The startup period choices, in the order the combo lists them.
const STARTUP_OPTIONS: &[(StartupPeriod, &str)] = &[
    (StartupPeriod::Last, "Last period shown"),
    (StartupPeriod::Today, "Today"),
    (StartupPeriod::ThisWeek, "This week"),
    (StartupPeriod::ThisMonth, "This month"),
];

fn startup_label(period: StartupPeriod) -> &'static str {
    STARTUP_OPTIONS
        .iter()
        .find(|(p, _)| *p == period)
        .map_or("", |(_, label)| label)
}

/// The desktop's page in Settings, sharing the app's settings.
pub struct DesktopSettingsPage {
    settings: Rc<RefCell<Settings>>,
    error_msg: Option<String>,
}

impl DesktopSettingsPage {
    pub fn new(settings: Rc<RefCell<Settings>>) -> Self {
        Self {
            settings,
            error_msg: None,
        }
    }
}

impl SettingsSection for DesktopSettingsPage {
    fn label(&self, _ctx: &egui::Context) -> String {
        "Desktop".to_string()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Startup period");
        ui.add_space(4.0);

        let mut chosen = self.settings.borrow().startup_period;
        egui::ComboBox::from_id_salt("settings_startup_period")
            .selected_text(startup_label(chosen))
            .show_ui(ui, |ui| {
                for (period, label) in STARTUP_OPTIONS {
                    ui.selectable_value(&mut chosen, *period, *label);
                }
            });

        // The borrow is short: nothing here draws while it is held.
        let mut settings = self.settings.borrow_mut();
        if settings.startup_period != chosen {
            settings.startup_period = chosen;
            self.error_msg = settings
                .save()
                .err()
                .map(|e| format!("Failed to save settings: {e}"));
        }
        drop(settings);

        if let Some(msg) = &self.error_msg {
            ui.add_space(8.0);
            ui.colored_label(egui::Color32::LIGHT_RED, msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_startup_period_has_one_distinct_label() {
        for p in [
            StartupPeriod::Last,
            StartupPeriod::Today,
            StartupPeriod::ThisWeek,
            StartupPeriod::ThisMonth,
        ] {
            assert_eq!(STARTUP_OPTIONS.iter().filter(|(q, _)| *q == p).count(), 1);
        }
        assert_eq!(STARTUP_OPTIONS.len(), 4);
        assert_eq!(startup_label(StartupPeriod::ThisWeek), "This week");
    }
}
