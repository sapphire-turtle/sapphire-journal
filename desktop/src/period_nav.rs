//! The shared period (`None` = all entries) as alumina's [`Period`], and its conversions
//! to the core filter and to the string saved in `settings.toml`.

use corundum_framework_gui::datetime::{self, Period};
use jiff::ToSpan;

/// The core filter for `period`: its first day at 00:00:00 through its last day at 23:59:59.
pub fn to_filter(period: &Period) -> sapphire_journal_core::period::Period {
    let here = |d: jiff::civil::DateTime| {
        sapphire_journal_api::time::from_local(d).unwrap_or_else(|_| jiff::Timestamp::now())
    };
    sapphire_journal_core::period::Period::Range(
        here(period.first_day().at(0, 0, 0, 0)),
        here(period.last_day().at(23, 59, 59, 0)),
    )
}

/// `period` moved by `by` of its own unit (days, weeks, months or years); `None` for a range,
/// which has no unit to step by, or when the result is out of range.
pub fn shift(period: &Period, by: i32) -> Option<Period> {
    match period {
        Period::Day(d) => d.checked_add((i64::from(by)).days()).ok().map(Period::Day),
        Period::Week { start } => start
            .checked_add((7 * i64::from(by)).days())
            .ok()
            .map(|start| Period::Week { start }),
        Period::Month { year, month } => {
            let first =
                jiff::civil::Date::new(i16::try_from(*year).ok()?, i8::try_from(*month).ok()?, 1)
                    .ok()?;
            Some(Period::month_of(datetime::calc::add_months(first, by)))
        }
        Period::Year(y) => y.checked_add(by).map(Period::Year),
        Period::Range { .. } => None,
    }
}

/// `period` as `Display` writes it, or `""` for all entries.
pub fn period_string(period: Option<&Period>) -> String {
    period.map(ToString::to_string).unwrap_or_default()
}

/// The inverse of [`period_string`]: `""` is `Some(None)` (all), anything unparsable `None`.
pub fn parse_period_string(s: &str) -> Option<Option<Period>> {
    let s = s.trim();
    if s.is_empty() {
        return Some(None);
    }
    s.parse().ok().map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::Date;

    fn d(y: i16, m: i8, day: i8) -> Date {
        Date::new(y, m, day).unwrap()
    }

    #[test]
    fn shift_day_across_leap_february() {
        let feb28 = Period::Day(d(2028, 2, 28));
        let feb29 = shift(&feb28, 1);
        assert_eq!(feb29, Some(Period::Day(d(2028, 2, 29))));
        assert_eq!(shift(&feb29.unwrap(), 1), Some(Period::Day(d(2028, 3, 1))));
    }

    #[test]
    fn shift_day() {
        assert_eq!(
            shift(&Period::Day(d(2026, 12, 31)), 1),
            Some(Period::Day(d(2027, 1, 1)))
        );
    }

    #[test]
    fn shift_iso_week_across_year() {
        let w53 = "2026-W53".parse::<Period>().unwrap();
        let w1 = "2027-W01".parse::<Period>().unwrap();
        assert_eq!(shift(&w53, 1), Some(w1));
        assert_eq!(shift(&w1, -1), Some(w53));
    }

    #[test]
    fn shift_month() {
        let dec = Period::Month {
            year: 2026,
            month: 12,
        };
        assert_eq!(
            shift(&dec, 1),
            Some(Period::Month {
                year: 2027,
                month: 1
            })
        );
    }

    #[test]
    fn shift_year() {
        assert_eq!(shift(&Period::Year(2026), -1), Some(Period::Year(2025)));
    }

    #[test]
    fn shift_range_is_none() {
        assert_eq!(
            shift(&Period::range(d(2026, 11, 1), d(2026, 11, 10)), 1),
            None
        );
    }

    #[test]
    fn filter_of_a_week() {
        let start = jiff::civil::date(2026, 11, 2);
        let sapphire_journal_core::period::Period::Range(a, b) = to_filter(&Period::Week { start })
        else {
            panic!("not a range");
        };
        let here = |d| sapphire_journal_api::time::from_local(d).unwrap();
        assert_eq!(a, here(start.at(0, 0, 0, 0)));
        assert_eq!(b, here(jiff::civil::date(2026, 11, 8).at(23, 59, 59, 0)));
    }

    #[test]
    fn period_string_round_trip() {
        for s in ["2026-W45", "2026-11", "2026", "2026-11-01..2026-11-10", ""] {
            let p = parse_period_string(s).expect(s);
            assert_eq!(period_string(p.as_ref()), s);
        }
    }

    #[test]
    fn bad_period_string_is_none() {
        assert!(parse_period_string("nonsense").is_none());
        assert!(parse_period_string("2026-13").is_none());
    }
}
