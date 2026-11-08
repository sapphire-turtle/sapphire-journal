//! The agenda layout of the calendar (a stub until a later task draws it).

use corundum_framework_gui::datetime::Period;
use sapphire_framework::gui::egui;

use crate::app::HomeState;

pub fn show(_home: &mut HomeState, period: Option<Period>, ui: &mut egui::Ui) {
    ui.weak(format!(
        "Agenda ({})",
        crate::period_nav::period_string(period.as_ref())
    ));
}
