//! The agenda layout of the calendar: one row per day.

use chrono::{Datelike, Duration, Local, NaiveDate};
use corundum_framework_gui::datetime::{self, Period};
use sapphire_framework::gui::egui;

use super::chip::{chip, chip_label, open_day};
use super::items::{CalendarItem, ItemKind};
use crate::app::HomeState;

pub const ROW_HEIGHT: f32 = 24.0;
/// How far either side of today "all" reaches.
const ALL_DAYS: i64 = 36500;
/// The width reserved for the date on the left of a row.
const LABEL_WIDTH: f32 = 120.0;
/// The width kept free for a `+n` after the chips.
const MORE_WIDTH: f32 = 36.0;
/// The most width one chip takes; longer titles are cut.
const CHIP_MAX_WIDTH: f32 = 240.0;

/// The days the agenda lists: `len` days from `first`.
pub struct AgendaRange {
    pub first: NaiveDate,
    pub len: usize,
}

/// The rows of `period`, or today ± 36500 days when it is `None`.
pub fn agenda_range(period: Option<&Period>, today: NaiveDate) -> AgendaRange {
    let (first, last) = match period {
        Some(p) => (p.first_day(), p.last_day()),
        None => (
            today - Duration::days(ALL_DAYS),
            today + Duration::days(ALL_DAYS),
        ),
    };
    AgendaRange {
        first,
        len: ((last - first).num_days().max(0) + 1) as usize,
    }
}

impl AgendaRange {
    pub fn date_at(&self, row: usize) -> NaiveDate {
        self.first + Duration::days(row as i64)
    }

    pub fn row_of(&self, day: NaiveDate) -> Option<usize> {
        let row = (day - self.first).num_days();
        (0..self.len as i64).contains(&row).then_some(row as usize)
    }
}

/// Draw the agenda of `period`: a year, a range, or all entries.
pub fn show(home: &mut HomeState, period: Option<Period>, ui: &mut egui::Ui) {
    let today = Local::now().date_naive();
    let range = agenda_range(period.as_ref(), today);
    let labels = datetime::labels(ui.ctx());

    ui.scope(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        // The rows are placed exactly `ROW_HEIGHT` apart.
        ui.spacing_mut().item_spacing.y = 0.0;

        let mut area = egui::ScrollArea::vertical()
            .id_salt("calendar_agenda")
            .auto_shrink([false, false]);
        // `calendar_scrolled` starts as `None`, which is also "all", so the
        // first "all" is told apart by a flag in the context.
        let flag = egui::Id::unique("calendar_agenda_scrolled");
        let seen = ui.data(|d| d.get_temp::<bool>(flag)).unwrap_or(false);
        if home.calendar_scrolled != period || (period.is_none() && !seen) {
            let row = match period {
                None => range.row_of(today).unwrap_or(0),
                Some(_) => 0,
            };
            area = area.vertical_scroll_offset(row as f32 * ROW_HEIGHT);
            home.calendar_scrolled = period;
            ui.data_mut(|d| d.insert_temp(flag, true));
        }

        area.show_rows(ui, ROW_HEIGHT, range.len, |ui, rows| {
            let first_row = rows.start;
            let per_row: Vec<Vec<CalendarItem>> = {
                let index = home.calendar.get(&home.entries, &home.filter_text);
                rows.clone()
                    .map(|r| index.on(range.date_at(r)).into_iter().cloned().collect())
                    .collect()
            };
            for (row, items) in rows.zip(per_row) {
                let day = range.date_at(row);
                let with_year = row == first_row || (day.month() == 1 && day.day() == 1);
                draw_row(home, ui, &labels, day, with_year, today, &items);
            }
        });
    });
}

/// `MM-DD (曜)`, with `YYYY-` in front when `with_year`.
fn day_label(day: NaiveDate, weekday: &str, with_year: bool) -> String {
    let date = if with_year {
        day.format("%Y-%m-%d")
    } else {
        day.format("%m-%d")
    };
    format!("{date} ({weekday})")
}

fn draw_row(
    home: &mut HomeState,
    ui: &mut egui::Ui,
    labels: &datetime::Labels,
    day: NaiveDate,
    with_year: bool,
    today: NaiveDate,
    items: &[CalendarItem],
) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let visuals = ui.visuals();
    let fill = if day == today {
        Some(visuals.selection.bg_fill.gamma_multiply(0.3))
    } else if day.weekday().number_from_monday() >= 6 {
        Some(visuals.faint_bg_color)
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, 0.0, fill);
    }

    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.set_clip_rect(rect.intersect(ui.clip_rect()));
    row.spacing_mut().item_spacing.x = 4.0;

    let weekday = &labels.weekdays_short[day.weekday().num_days_from_monday() as usize];
    let mut text = egui::RichText::new(day_label(day, weekday, with_year));
    if day == today {
        text = text.strong().color(row.visuals().strong_text_color());
    }
    let label = row
        .allocate_ui_with_layout(
            egui::vec2(LABEL_WIDTH, ROW_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| ui.add(egui::Button::new(text).frame(false)),
        )
        .inner;
    if label.clicked() {
        open_day(home, day);
    }

    let mut shown = 0;
    for (i, item) in items.iter().enumerate() {
        let label = chip_label(item);
        let width = chip_width(&row, item, &label);
        let remaining = row.available_width();
        // Keep room for `+n` unless this is the last chip.
        let reserve = if i + 1 < items.len() { MORE_WIDTH } else { 0.0 };
        // The first chip is always drawn, cut to fit if it must be.
        if width + reserve > remaining && i > 0 {
            break;
        }
        let selected = home.selected_path.as_ref() == Some(&item.path);
        let clicked = row
            .allocate_ui_with_layout(
                egui::vec2(width.min(remaining), ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| chip(ui, item, &label, selected),
            )
            .inner
            .clicked();
        if clicked {
            home.open(item.path.clone());
        }
        shown += 1;
    }
    if shown < items.len() {
        let more = egui::RichText::new(format!("+{}", items.len() - shown)).weak();
        if row.add(egui::Button::new(more).frame(false)).clicked() {
            open_day(home, day);
        }
    }
}

/// About how wide `chip` draws `label`: the text, the button's padding, and
/// the check icon of a task.
fn chip_width(ui: &egui::Ui, item: &CalendarItem, label: &str) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), font, egui::Color32::PLACEHOLDER)
            .size()
            .x
    });
    let spacing = ui.spacing();
    let icon = match (&item.kind, item.done) {
        (ItemKind::Due(_), false) => spacing.icon_width + spacing.icon_spacing,
        _ => 0.0,
    };
    (text + icon + spacing.button_padding.x * 2.0 + 2.0).min(CHIP_MAX_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn year_has_365_rows() {
        let today = d(2026, 11, 8);
        assert_eq!(agenda_range(Some(&Period::Year(2026)), today).len, 365);
        assert_eq!(agenda_range(Some(&Period::Year(2028)), today).len, 366);
    }

    #[test]
    fn range_ends() {
        let p = Period::Range {
            start: d(2026, 11, 1),
            end: d(2026, 11, 10),
        };
        let r = agenda_range(Some(&p), d(2026, 11, 8));
        assert_eq!(r.len, 10);
        assert_eq!(r.date_at(9), d(2026, 11, 10));
        assert_eq!(r.row_of(d(2026, 11, 11)), None);
        assert_eq!(r.row_of(d(2026, 10, 31)), None);
    }

    #[test]
    fn all_is_centred_on_today() {
        let today = d(2026, 11, 8);
        let r = agenda_range(None, today);
        assert_eq!(r.len, 73001);
        assert_eq!(r.row_of(today), Some(36500));
    }

    #[test]
    fn rows_and_dates_round_trip() {
        let r = agenda_range(Some(&Period::Year(2026)), d(2026, 11, 8));
        assert_eq!(r.date_at(0), d(2026, 1, 1));
        assert_eq!(r.row_of(d(2026, 12, 31)), Some(364));
        assert_eq!(r.date_at(r.row_of(d(2026, 11, 8)).unwrap()), d(2026, 11, 8));
    }

    #[test]
    fn label_has_year_when_asked() {
        assert_eq!(day_label(d(2027, 1, 1), "金", true), "2027-01-01 (金)");
        assert_eq!(day_label(d(2026, 11, 8), "日", false), "11-08 (日)");
    }
}
