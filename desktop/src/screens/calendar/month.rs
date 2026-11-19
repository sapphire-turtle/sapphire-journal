//! The month layout of the calendar: a 7×6 grid of days.

use corundum_framework_gui::datetime::{self, Period};
use jiff::ToSpan;
use jiff::civil::Date;
use sapphire_framework::gui::egui;

use super::chip::{chip, chip_label, open_day};
use super::items::CalendarItem;
use crate::app::HomeState;

const COLUMNS: usize = 7;
const ROWS: usize = 6;
const CELL_PADDING: f32 = 2.0;

/// The first cell of the month grid: the Monday on or before the 1st.
pub fn grid_start(year: i32, month: u32) -> Date {
    let first = i16::try_from(year)
        .ok()
        .zip(i8::try_from(month).ok())
        .and_then(|(y, m)| Date::new(y, m, 1).ok())
        .unwrap_or(Date::MIN);
    let back = i64::from(first.weekday().to_monday_zero_offset());
    first.checked_sub(back.days()).unwrap_or(first)
}

/// Draw the month grid of `period`.
pub fn show(home: &mut HomeState, period: Option<Period>, ui: &mut egui::Ui) {
    let Some(Period::Month { year, month }) = period else {
        return;
    };
    let start = grid_start(year, month);
    let days: Vec<Date> = start.series(1.day()).take(COLUMNS * ROWS).collect();
    let per_day: Vec<Vec<CalendarItem>> = {
        let index = home.calendar.get(&home.entries, &home.filter_text);
        days.iter()
            .map(|&d| index.on(d).into_iter().cloned().collect())
            .collect()
    };
    let today = sapphire_journal_api::time::local_date(jiff::Timestamp::now());
    let labels = datetime::labels(ui.ctx());

    ui.scope(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let width = ui.available_width();
        let cell_width = (width / COLUMNS as f32).max(1.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for name in &labels.weekdays_short {
                ui.allocate_ui_with_layout(
                    egui::vec2(cell_width, 0.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.set_width(cell_width);
                        ui.weak(name);
                    },
                );
            }
        });

        let cell_height = (ui.available_height() / ROWS as f32).max(1.0);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(cell_width * COLUMNS as f32, cell_height * ROWS as f32),
            egui::Sense::hover(),
        );
        for (i, &day) in days.iter().enumerate() {
            let cell = egui::Rect::from_min_size(
                rect.min
                    + egui::vec2(
                        (i % COLUMNS) as f32 * cell_width,
                        (i / COLUMNS) as f32 * cell_height,
                    ),
                egui::vec2(cell_width, cell_height),
            );
            let in_month =
                i32::from(day.year()) == year && i64::from(day.month()) == i64::from(month);
            draw_cell(home, ui, cell, day, in_month, day == today, &per_day[i]);
        }
    });
}

fn draw_cell(
    home: &mut HomeState,
    ui: &mut egui::Ui,
    cell: egui::Rect,
    day: Date,
    in_month: bool,
    is_today: bool,
    items: &[CalendarItem],
) {
    let visuals = ui.visuals();
    let line = visuals.widgets.noninteractive.bg_stroke;
    let fill = if is_today {
        visuals.selection.bg_fill.gamma_multiply(0.3)
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter()
        .rect(cell, 0.0, fill, line, egui::StrokeKind::Inside);

    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(cell.shrink(CELL_PADDING))
            .layout(egui::Layout::top_down_justified(egui::Align::Min)),
    );
    inner.set_clip_rect(cell.shrink(1.0).intersect(ui.clip_rect()));
    inner.spacing_mut().item_spacing.y = 1.0;

    let mut number = egui::RichText::new(day.day().to_string());
    if is_today {
        number = number.strong().color(inner.visuals().strong_text_color());
    } else if !in_month {
        number = number.weak();
    }
    if inner.add(egui::Button::new(number).frame(false)).clicked() {
        open_day(home, day);
    }

    let row = inner.spacing().interact_size.y + inner.spacing().item_spacing.y;
    let fits = (inner.available_height() / row).floor().max(0.0) as usize;
    let shown = visible_count(items.len(), fits);
    for item in &items[..shown] {
        let label = chip_label(item);
        let selected = home.selected_path.as_ref() == Some(&item.path);
        if chip(&mut inner, item, &label, selected).clicked() {
            home.open(item.path.clone());
        }
    }
    if shown < items.len() {
        let more = egui::RichText::new(format!("+{}", items.len() - shown)).weak();
        if inner.add(egui::Button::new(more).frame(false)).clicked() {
            open_day(home, day);
        }
    }
}

/// How many of `total` items get a chip when `fits` lines are free: all of
/// them if they fit, otherwise one line is kept for the `+n`.
pub fn visible_count(total: usize, fits: usize) -> usize {
    if total <= fits {
        total
    } else {
        fits.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i16, m: i8, d: i8) -> Date {
        Date::new(y, m, d).unwrap()
    }

    #[test]
    fn grid_starts_on_the_first_when_monday() {
        assert_eq!(grid_start(2026, 6), d(2026, 6, 1));
    }

    #[test]
    fn grid_starts_before_when_sunday() {
        assert_eq!(grid_start(2026, 11), d(2026, 10, 26));
    }

    #[test]
    fn grid_covers_month_end() {
        assert!(grid_start(2026, 11) + 41.days() >= d(2026, 11, 30));
    }

    #[test]
    fn overflow_keeps_a_line_for_the_count() {
        assert_eq!(visible_count(2, 3), 2);
        assert_eq!(visible_count(3, 3), 3);
        assert_eq!(visible_count(4, 3), 2);
        assert_eq!(visible_count(1, 0), 0);
    }
}
