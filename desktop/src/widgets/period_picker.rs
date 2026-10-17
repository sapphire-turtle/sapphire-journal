//! The period the views show, as a label that opens a calendar.
//!
//! The label reads the period as it is written (`today`, `2026-08-19`,
//! `2026-W23`, …: the aliases the CLI's `--period` takes). The calendar in
//! its popup picks one year, month, ISO week or day; the field above the
//! calendar takes any other alias.

use chrono::{Datelike as _, NaiveDate};
use sapphire_framework::gui::egui;
use sapphire_journal_core::period::{Period, parse_period};

use crate::icons;

use super::month_grid::{DayBadge, MonthGrid, Selection, selection_from_period};

/// Shows the label for `period`; clicking it opens the calendar, which
/// writes the picked period back into `period`. `cursor` is the month the
/// calendar shows; `badge` marks the days entries fall on.
pub fn period_picker(
    ui: &mut egui::Ui,
    period: &mut String,
    cursor: &mut NaiveDate,
    badge: DayBadge<'_>,
) -> egui::Response {
    let tint = ui.visuals().text_color();
    let calendar = egui::Image::new(icons::calendar())
        .fit_to_exact_size(egui::vec2(14.0, 14.0))
        .tint(tint);
    let label = if period.trim().is_empty() {
        "All entries".to_string()
    } else {
        period.trim().to_string()
    };
    let button = ui
        .add(egui::Button::image_and_text(calendar, label))
        .on_hover_text(period_summary(period));
    // Open on the month the period starts in.
    if button.clicked()
        && let Ok(Period::Range(start, _)) = parse_period(period.trim())
    {
        *cursor = start.date();
    }

    let popup_id = button.id.with("period_picker");
    egui::Popup::from_toggle_button_response(&button)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            // As wide as the calendar, which the field would otherwise outgrow.
            ui.set_width(250.0);
            ui.add(
                egui::TextEdit::singleline(period)
                    .hint_text("today, 2026-W23, 2026-08 …")
                    .desired_width(f32::INFINITY),
            );
            let parsed = parse_period(period.trim());
            if !period.trim().is_empty()
                && let Err(e) = &parsed
            {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            ui.add_space(4.0);

            let selection = parsed
                .map(|p| selection_from_period(&p))
                .unwrap_or(Selection::None);
            let resp = MonthGrid::new("period_picker", cursor)
                .selection(selection)
                .show_week_column(true)
                .day_badge(badge)
                .show(ui);

            let picked = if let Some(d) = resp.day_clicked {
                Some(d.format("%Y-%m-%d").to_string())
            } else if let Some(monday) = resp.week_clicked {
                let week = monday.iso_week();
                Some(format!("{}-W{:02}", week.year(), week.week()))
            } else if let Some((y, m)) = resp.month_clicked {
                Some(format!("{y}-{m:02}"))
            } else if let Some(y) = resp.year_clicked {
                Some(y.to_string())
            } else if resp.today_clicked {
                Some("today".to_string())
            } else if resp.clear_clicked {
                Some(String::new())
            } else {
                None
            };
            if let Some(picked) = picked {
                *period = picked;
                egui::Popup::close_id(ui.ctx(), popup_id);
            }
        });

    button
}

/// The dates a period covers, for the label's hover text.  Best-effort —
/// "All entries" when no period is set, the text itself when it can't be
/// parsed.
fn period_summary(period: &str) -> String {
    let s = period.trim();
    if s.is_empty() {
        return "All entries".to_string();
    }
    match parse_period(s) {
        Ok(Period::Range(start, end)) => {
            let sd = start.date();
            let ed = end.date();
            if sd == ed {
                sd.format("%Y-%m-%d").to_string()
            } else {
                format!("{} – {}", sd.format("%Y-%m-%d"), ed.format("%Y-%m-%d"))
            }
        }
        Ok(Period::None) => "Unset field".to_string(),
        Err(_) => s.to_string(),
    }
}
