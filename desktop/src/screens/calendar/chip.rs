//! The chip an item is drawn as, shared by the calendar layouts.

use chrono::NaiveDate;
use corundum_framework_gui::datetime::Period;
use sapphire_framework::gui::egui;

use super::items::{CalendarItem, ItemKind};
use crate::app::HomeState;
use crate::icons;

/// Draw `item` as a chip showing `label`. An event is filled with the
/// accent colour, a task is outlined, a done task is weak and struck
/// through; the selected item gets a selection frame. The hover text is the title and when it is.
pub fn chip(ui: &mut egui::Ui, item: &CalendarItem, label: &str, selected: bool) -> egui::Response {
    let visuals = ui.visuals();
    let text = egui::RichText::new(label);
    let frame = if selected {
        visuals.selection.stroke
    } else {
        egui::Stroke::NONE
    };
    let button = match (&item.kind, item.done) {
        (_, true) => egui::Button::new(text.weak().strikethrough())
            .fill(egui::Color32::TRANSPARENT)
            .stroke(frame),
        (ItemKind::Due(_), false) => {
            let stroke = if selected {
                frame
            } else {
                visuals.widgets.inactive.fg_stroke
            };
            egui::Button::image_and_text(icons::check(), text)
                .fill(egui::Color32::TRANSPARENT)
                .stroke(stroke)
        }
        (ItemKind::Event { .. }, false) => egui::Button::new(text)
            .fill(visuals.selection.bg_fill)
            .stroke(frame),
    };
    ui.add(button).on_hover_text(hover_text(item))
}

/// The text a chip shows: the title, led by the start time unless the item
/// lasts all day.
pub fn chip_label(item: &CalendarItem) -> String {
    if item.all_day {
        item.title.clone()
    } else {
        format!("{} {}", item.span().0.format("%H:%M"), item.title)
    }
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
