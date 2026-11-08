//! The chip an item is drawn as, shared by the calendar layouts.

use chrono::NaiveDate;
use corundum_framework_gui::datetime::Period;
use sapphire_framework::gui::egui;

use super::items::{CalendarItem, ItemKind};
use crate::app::HomeState;
use crate::icons;

/// Draw `item` as a chip showing `label`. An event is filled with the
/// selection colour when `selected`, a task is outlined, a done task is weak
/// and struck through. The hover text is the title and when it is.
pub fn chip(ui: &mut egui::Ui, item: &CalendarItem, label: &str, selected: bool) -> egui::Response {
    let visuals = ui.visuals();
    let text = egui::RichText::new(label);
    let button = match (&item.kind, item.done) {
        (_, true) => egui::Button::new(text.weak().strikethrough()).frame(false),
        (ItemKind::Due(_), false) => {
            let stroke = if selected {
                visuals.selection.stroke
            } else {
                visuals.widgets.inactive.fg_stroke
            };
            egui::Button::image_and_text(icons::check(), text)
                .fill(egui::Color32::TRANSPARENT)
                .stroke(stroke)
        }
        (ItemKind::Event { .. }, false) => {
            let fill = if selected {
                visuals.selection.bg_fill
            } else {
                visuals.widgets.inactive.weak_bg_fill
            };
            egui::Button::new(text).fill(fill)
        }
    };
    ui.add(button).on_hover_text(hover_text(item))
}

fn hover_text(item: &CalendarItem) -> String {
    let (start, end) = item.span();
    let when = if item.all_day {
        let (first, last) = (item.first_day(), item.last_day());
        if first == last {
            first.to_string()
        } else {
            format!("{first} – {last}")
        }
    } else if matches!(item.kind, ItemKind::Due(_)) {
        start.format("%Y-%m-%d %H:%M").to_string()
    } else if start.date() == end.date() {
        format!("{}–{}", start.format("%Y-%m-%d %H:%M"), end.format("%H:%M"))
    } else {
        format!(
            "{}–{}",
            start.format("%Y-%m-%d %H:%M"),
            end.format("%Y-%m-%d %H:%M")
        )
    };
    format!("{}\n{when}", item.title)
}

/// Show `day` alone.
pub fn open_day(home: &mut HomeState, day: NaiveDate) {
    home.period = Some(Period::Day(day));
}
