//! The timeline layout of the calendar: a column per day, all-day items in a
//! strip on top, timed ones as blocks on a 24-hour scale below.

use std::time::Duration;

use chrono::{Local, NaiveDate, NaiveDateTime, Timelike};
use corundum_framework_gui::datetime::Period;
use sapphire_framework::gui::egui;

use super::chip::{chip, chip_label, open_day};
use super::items::CalendarItem;
use crate::app::HomeState;

pub const HOUR_HEIGHT: f32 = 48.0;
/// The least height a block is drawn with; a due date is a block this tall.
pub const MIN_BLOCK: f32 = 20.0;
/// The width of the hour labels on the left.
const GUTTER: f32 = 40.0;
const DAY_MINUTES: u32 = 24 * 60;
/// Where the timeline starts scrolled to: 08:00.
const START_MINUTES: u32 = 8 * 60;

/// How far down the timeline `minutes` past midnight is.
pub fn y_of(minutes: u32) -> f32 {
    minutes as f32 * HOUR_HEIGHT / 60.0
}

/// The minutes (`0..=1440`) of `day` a timed item covers, clipped to that day.
/// `None` when the item is all-day or not on `day`; an event ending at 00:00
/// is not on the day it ends, so it leaves no sliver there.
pub fn segment_on(item: &CalendarItem, day: NaiveDate) -> Option<(u32, u32)> {
    if item.all_day || day < item.first_day() || day > item.last_day() {
        return None;
    }
    let (start, end) = item.span();
    let midnight = day.and_time(chrono::NaiveTime::MIN);
    let minutes = |t: NaiveDateTime| {
        let m = (t - midnight)
            .num_minutes()
            .clamp(0, i64::from(DAY_MINUTES));
        m as u32
    };
    Some((minutes(start), minutes(end)))
}

/// The column each segment goes in and how many columns its overlap group has,
/// in input order. Segments are placed by start in the first free column; a
/// segment counts as lasting at least as long as `MIN_BLOCK` is tall, as it is
/// drawn.
pub fn assign_columns(segments: &[(u32, u32)]) -> Vec<(usize, usize)> {
    let min_minutes = (MIN_BLOCK * 60.0 / HOUR_HEIGHT).ceil() as u32;
    let drawn_end = |(s, e): (u32, u32)| e.max(s + min_minutes);

    let mut order: Vec<usize> = (0..segments.len()).collect();
    order.sort_by_key(|&i| segments[i]);

    let mut placed = vec![(0, 0); segments.len()];
    let mut group: Vec<usize> = Vec::new();
    // The end of each column's last segment in the current group.
    let mut columns: Vec<u32> = Vec::new();
    let mut group_end = 0;
    for i in order {
        let (start, _) = segments[i];
        if start >= group_end {
            close_group(&mut placed, &group, columns.len());
            group.clear();
            columns.clear();
            group_end = 0;
        }
        let end = drawn_end(segments[i]);
        let column = match columns.iter().position(|&c| c <= start) {
            Some(c) => {
                columns[c] = end;
                c
            }
            None => {
                columns.push(end);
                columns.len() - 1
            }
        };
        placed[i].0 = column;
        group.push(i);
        group_end = group_end.max(end);
    }
    close_group(&mut placed, &group, columns.len());
    placed
}

fn close_group(placed: &mut [(usize, usize)], group: &[usize], columns: usize) {
    for &i in group {
        placed[i].1 = columns;
    }
}

/// The days the timeline shows: one for a day, seven for a week.
fn days_of(period: Option<Period>) -> Vec<NaiveDate> {
    match period {
        Some(Period::Day(day)) => vec![day],
        Some(Period::Week { start }) => start.iter_days().take(7).collect(),
        _ => Vec::new(),
    }
}

/// Draw the timeline of `period`, a day or a week.
pub fn show(home: &mut HomeState, period: Option<Period>, ui: &mut egui::Ui) {
    ui.scope(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        draw(home, period, ui);
    });
}

fn draw(home: &mut HomeState, period: Option<Period>, ui: &mut egui::Ui) {
    let days = days_of(period);
    if days.is_empty() {
        return;
    }
    let per_day: Vec<Vec<CalendarItem>> = {
        let index = home.calendar.get(&home.entries, &home.filter_text);
        days.iter()
            .map(|&d| index.on(d).into_iter().cloned().collect())
            .collect()
    };
    let now = Local::now().naive_local();
    let today = now.date();
    let column_width = ((ui.available_width() - GUTTER) / days.len() as f32).max(1.0);

    columns_row(ui, column_width, days.len(), |ui, i| {
        let day = days[i];
        let mut text = egui::RichText::new(day.format("%a %m-%d").to_string());
        if day == today {
            text = text.strong().color(ui.visuals().strong_text_color());
        }
        if ui.add(egui::Button::new(text).frame(false)).clicked() {
            open_day(home, day);
        }
    });

    columns_row(ui, column_width, days.len(), |ui, i| {
        for item in per_day[i].iter().filter(|item| item.all_day) {
            let selected = home.selected_path.as_ref() == Some(&item.path);
            if chip(ui, item, &item.title, selected).clicked() {
                home.open(item.path.clone());
            }
        }
    });
    ui.separator();

    let mut area = egui::ScrollArea::vertical()
        .id_salt("calendar_timeline")
        .auto_shrink([false, false]);
    if home.calendar_scrolled != period {
        area = area.vertical_scroll_offset(y_of(START_MINUTES));
        home.calendar_scrolled = period;
    }
    area.show(ui, |ui| {
        let size = egui::vec2(GUTTER + column_width * days.len() as f32, y_of(DAY_MINUTES));
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        draw_scale(ui, rect, days.len(), column_width);

        for (i, items) in per_day.iter().enumerate() {
            let left = rect.left() + GUTTER + i as f32 * column_width;
            draw_blocks(home, ui, items, days[i], left, rect.top(), column_width);
            if days[i] == today {
                let y = rect.top() + y_of(now.hour() * 60 + now.minute());
                let stroke = egui::Stroke::new(2.0, ui.visuals().error_fg_color);
                ui.painter().hline(left..=left + column_width, y, stroke);
                ui.ctx().request_repaint_after(Duration::from_secs(60));
            }
        }
    });
}

/// A row of `count` columns `width` wide, after the hour gutter.
fn columns_row(
    ui: &mut egui::Ui,
    width: f32,
    count: usize,
    mut add: impl FnMut(&mut egui::Ui, usize),
) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(GUTTER);
        for i in 0..count {
            ui.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                egui::Layout::top_down_justified(egui::Align::Min),
                |ui| {
                    ui.set_width(width);
                    add(ui, i);
                },
            );
        }
    });
}

/// The hour lines and labels, and a line between the days.
fn draw_scale(ui: &egui::Ui, rect: egui::Rect, days: usize, width: f32) {
    let painter = ui.painter();
    let visuals = ui.visuals();
    let line = visuals.widgets.noninteractive.bg_stroke;
    let font = egui::TextStyle::Small.resolve(ui.style());
    for hour in 0..24 {
        let y = rect.top() + y_of(hour * 60);
        painter.hline(rect.left() + GUTTER..=rect.right(), y, line);
        painter.text(
            egui::pos2(rect.left() + GUTTER - 4.0, y),
            egui::Align2::RIGHT_TOP,
            format!("{hour:02}:00"),
            font.clone(),
            visuals.weak_text_color(),
        );
    }
    for i in 0..=days {
        let x = rect.left() + GUTTER + i as f32 * width;
        painter.vline(x, rect.y_range(), line);
    }
}

/// The timed items of `day` as blocks in the column at `left`.
fn draw_blocks(
    home: &mut HomeState,
    ui: &mut egui::Ui,
    items: &[CalendarItem],
    day: NaiveDate,
    left: f32,
    top: f32,
    width: f32,
) {
    let timed: Vec<(&CalendarItem, (u32, u32))> = items
        .iter()
        .filter_map(|item| segment_on(item, day).map(|s| (item, s)))
        .collect();
    let segments: Vec<(u32, u32)> = timed.iter().map(|&(_, s)| s).collect();
    for ((item, (start, end)), (column, columns)) in
        timed.into_iter().zip(assign_columns(&segments))
    {
        let block_width = width / columns as f32;
        let height = (y_of(end) - y_of(start)).max(MIN_BLOCK);
        // A short item late in the day is lifted to stay inside it.
        let y = y_of(start).min(y_of(DAY_MINUTES) - height);
        let rect = egui::Rect::from_min_size(
            egui::pos2(left + column as f32 * block_width, top + y),
            egui::vec2(block_width, height),
        )
        .shrink(1.0);
        let label = chip_label(item);
        let selected = home.selected_path.as_ref() == Some(&item.path);
        let clicked = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::centered_and_justified(
                        egui::Direction::TopDown,
                    )),
                |ui| chip(ui, item, &label, selected),
            )
            .inner
            .clicked();
        if clicked {
            home.open(item.path.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDateTime;
    use sapphire_journal_api::DateOrTime;

    use super::*;
    use crate::screens::calendar::items::ItemKind;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    fn at(d: u32, h: u32, m: u32) -> NaiveDateTime {
        day(d).and_hms_opt(h, m, 0).unwrap()
    }

    fn item(kind: ItemKind, all_day: bool) -> CalendarItem {
        CalendarItem {
            path: PathBuf::from("/x.md"),
            title: "x".into(),
            kind,
            all_day,
            done: false,
        }
    }

    fn timed_event(start: NaiveDateTime, end: NaiveDateTime) -> CalendarItem {
        item(
            ItemKind::Event {
                start: DateOrTime::DateTime(start),
                end: DateOrTime::DateTime(end),
            },
            false,
        )
    }

    #[test]
    fn y_of_noon() {
        assert_eq!(y_of(720), 576.0);
    }

    #[test]
    fn segment_clipped_to_day() {
        let e = timed_event(at(7, 22, 0), at(8, 2, 0));
        assert_eq!(segment_on(&e, day(7)), Some((1320, 1440)));
        assert_eq!(segment_on(&e, day(8)), Some((0, 120)));
        assert_eq!(segment_on(&e, day(9)), None);
        assert_eq!(segment_on(&e, day(6)), None);
    }

    #[test]
    fn segment_midnight_end_none_next_day() {
        let e = timed_event(at(7, 22, 0), at(8, 0, 0));
        assert_eq!(segment_on(&e, day(7)), Some((1320, 1440)));
        assert_eq!(segment_on(&e, day(8)), None);
    }

    #[test]
    fn due_segment_is_a_point() {
        let due = item(ItemKind::Due(DateOrTime::DateTime(at(8, 15, 0))), false);
        assert_eq!(segment_on(&due, day(8)), Some((900, 900)));
    }

    #[test]
    fn midnight_due_sits_at_the_top() {
        let due = item(ItemKind::Due(DateOrTime::DateTime(at(8, 0, 0))), false);
        assert_eq!(segment_on(&due, day(8)), Some((0, 0)));
        assert_eq!(segment_on(&due, day(7)), None);
    }

    #[test]
    fn all_day_has_no_segment() {
        let e = item(
            ItemKind::Event {
                start: DateOrTime::Date(day(8)),
                end: DateOrTime::Date(day(8)),
            },
            true,
        );
        assert_eq!(segment_on(&e, day(8)), None);
        let due = item(ItemKind::Due(DateOrTime::Date(day(8))), true);
        assert_eq!(segment_on(&due, day(8)), None);
    }

    #[test]
    fn inverted_event_is_a_point() {
        let e = timed_event(at(9, 10, 0), at(7, 10, 0));
        assert_eq!(segment_on(&e, day(9)), Some((600, 600)));
        assert_eq!(segment_on(&e, day(8)), None);
    }

    #[test]
    fn columns_no_overlap() {
        assert_eq!(assign_columns(&[(0, 60), (60, 120)]), [(0, 1), (0, 1)]);
    }

    #[test]
    fn columns_two_overlap() {
        assert_eq!(assign_columns(&[(0, 120), (60, 180)]), [(0, 2), (1, 2)]);
    }

    #[test]
    fn columns_chain() {
        assert_eq!(
            assign_columns(&[(0, 120), (60, 180), (150, 240)]),
            [(0, 2), (1, 2), (0, 2)]
        );
    }

    #[test]
    fn columns_point_overlaps_a_block_at_its_time() {
        assert_eq!(assign_columns(&[(900, 900), (900, 960)]), [(0, 2), (1, 2)]);
        // 25 minutes after a point is free again.
        assert_eq!(assign_columns(&[(900, 900), (925, 960)]), [(0, 1), (0, 1)]);
    }

    #[test]
    fn columns_keep_input_order() {
        assert_eq!(assign_columns(&[(60, 180), (0, 120)]), [(1, 2), (0, 2)]);
    }

    #[test]
    fn columns_empty() {
        assert!(assign_columns(&[]).is_empty());
    }
}
