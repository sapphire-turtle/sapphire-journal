//! A date input that is a date picker for an all-day value and a date-and-time
//! picker for a timed one, and the "All day" switches that move between them.

use corundum_framework_gui::datetime::{self, DatePickerButton, DateTimePickerButton};
use jiff::civil::{Date, Time};
use sapphire_framework::gui::egui;
use sapphire_journal_api::{DateOrTime, time};

/// The time of day a due date or event start gets when "All day" is turned off.
const DEFAULT_START: (i8, i8) = (9, 0);
/// The time of day an event end gets when "All day" is turned off.
const DEFAULT_END: (i8, i8) = (10, 0);

/// `date` at `h:m` in this host's time zone.
fn at(date: Date, (h, m): (i8, i8)) -> DateOrTime {
    let datetime = date.to_datetime(Time::new(h, m, 0, 0).expect("valid time"));
    DateOrTime::Time(time::from_local(datetime).expect("a recent day's time is a time here"))
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
        DateOrTime::Time(t) => {
            // The picker shows and edits wall-clock time here.
            let mut local = time::local(*t).datetime();
            let response = ui.add(
                DateTimePickerButton::new(id_salt, &mut local).config(datetime::config(ui.ctx())),
            );
            if local != time::local(*t).datetime()
                && let Ok(picked) = time::from_local(local)
            {
                *t = picked;
            }
            response
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

    fn d(day: i8) -> Date {
        jiff::civil::date(2026, 11, day)
    }

    fn t(day: i8, h: i8, m: i8) -> DateOrTime {
        at(d(day), (h, m))
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
