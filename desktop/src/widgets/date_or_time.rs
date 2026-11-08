//! A date input that is a date picker for an all-day value and a date-and-time
//! picker for a timed one, and the "All day" switches that move between them.

use chrono::{NaiveDate, NaiveTime};
use corundum_framework_gui::datetime::{self, DatePickerButton, DateTimePickerButton};
use sapphire_framework::gui::egui;
use sapphire_journal_api::DateOrTime;

/// The time of day a due date or event start gets when "All day" is turned off.
const DEFAULT_START: (u32, u32) = (9, 0);
/// The time of day an event end gets when "All day" is turned off.
const DEFAULT_END: (u32, u32) = (10, 0);

fn at(date: NaiveDate, (h, m): (u32, u32)) -> DateOrTime {
    DateOrTime::DateTime(date.and_time(NaiveTime::from_hms_opt(h, m, 0).expect("valid time")))
}

/// A picker for `value`: a [`DatePickerButton`] for a date, a
/// [`DateTimePickerButton`] for a date and time.
pub fn date_or_time_input(
    ui: &mut egui::Ui,
    id_salt: &str,
    value: &mut DateOrTime,
) -> egui::Response {
    match value {
        DateOrTime::Date(date) => {
            let mut picked = Some(*date);
            let response = ui.add(
                DatePickerButton::new(id_salt, &mut picked).config(datetime::config(ui.ctx())),
            );
            if let Some(d) = picked {
                *date = d;
            }
            response
        }
        DateOrTime::DateTime(dt) => {
            ui.add(DateTimePickerButton::new(id_salt, dt).config(datetime::config(ui.ctx())))
        }
    }
}

/// Make both ends of an event all-day (dates) or timed (09:00 to 10:00).
pub fn set_event_all_day(start: &mut DateOrTime, end: &mut DateOrTime, all_day: bool) {
    if all_day {
        *start = DateOrTime::Date(start.date());
        *end = DateOrTime::Date(end.date());
    } else {
        if start.is_date() {
            *start = at(start.date(), DEFAULT_START);
        }
        if end.is_date() {
            *end = at(end.date(), DEFAULT_END);
        }
    }
}

/// Make a due date all-day (a date) or timed (09:00 on that day).
pub fn set_due_all_day(due: &mut DateOrTime, all_day: bool) {
    if all_day {
        *due = DateOrTime::Date(due.date());
    } else if due.is_date() {
        *due = at(due.date(), DEFAULT_START);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn t(day: u32, h: u32, m: u32) -> DateOrTime {
        DateOrTime::DateTime(d(day).and_hms_opt(h, m, 0).unwrap())
    }

    #[test]
    fn event_all_day_on() {
        let (mut s, mut e) = (t(8, 9, 0), t(8, 10, 30));
        set_event_all_day(&mut s, &mut e, true);
        assert_eq!(s, DateOrTime::Date(d(8)));
        assert_eq!(e, DateOrTime::Date(d(8)));
    }

    #[test]
    fn event_all_day_off() {
        let (mut s, mut e) = (DateOrTime::Date(d(8)), DateOrTime::Date(d(9)));
        set_event_all_day(&mut s, &mut e, false);
        assert_eq!(s, t(8, 9, 0));
        assert_eq!(e, t(9, 10, 0));
    }

    #[test]
    fn due_all_day_off() {
        let mut due = DateOrTime::Date(d(8));
        set_due_all_day(&mut due, false);
        assert_eq!(due, t(8, 9, 0));
    }

    #[test]
    fn due_all_day_on() {
        let mut due = t(8, 14, 30);
        set_due_all_day(&mut due, true);
        assert_eq!(due, DateOrTime::Date(d(8)));
    }
}
