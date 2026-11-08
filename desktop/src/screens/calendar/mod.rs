//! The calendar view: a timeline for a day or a week, a grid for a month, an
//! agenda for anything else.

pub mod agenda;
pub mod chip;
pub mod items;
pub mod month;
pub mod timeline;

use corundum_framework_gui::datetime::Period;
use sapphire_framework::gui::egui;

use crate::app::HomeState;

/// How the calendar lays out the shared period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    Timeline,
    Month,
    Agenda,
}

fn layout_for(period: Option<&Period>) -> Layout {
    match period {
        Some(Period::Day(_) | Period::Week { .. }) => Layout::Timeline,
        Some(Period::Month { .. }) => Layout::Month,
        _ => Layout::Agenda,
    }
}

/// Draw the calendar for `home.period`.
pub fn show(home: &mut HomeState, ui: &mut egui::Ui) {
    let period = home.period;
    match layout_for(period.as_ref()) {
        Layout::Timeline => timeline::show(home, period, ui),
        Layout::Month => month::show(home, period, ui),
        Layout::Agenda => agenda::show(home, period, ui),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Weekday};

    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    #[test]
    fn layout_follows_the_period() {
        assert_eq!(layout_for(Some(&Period::Day(d(8)))), Layout::Timeline);
        assert_eq!(
            layout_for(Some(&Period::week_of(d(8), Weekday::Mon))),
            Layout::Timeline
        );
        assert_eq!(layout_for(Some(&Period::month_of(d(8)))), Layout::Month);
        assert_eq!(layout_for(Some(&Period::Year(2026))), Layout::Agenda);
        let range = Period::Range {
            start: d(1),
            end: d(9),
        };
        assert_eq!(layout_for(Some(&range)), Layout::Agenda);
        assert_eq!(layout_for(None), Layout::Agenda);
    }
}
