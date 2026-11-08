//! The shared period (`None` = all entries) as alumina's [`Period`], and its conversions
//! to the core filter and to the string saved in `settings.toml`.

use corundum_framework_gui::datetime::{self, Period};

/// The core filter for `period`: its first day at 00:00:00 through its last day at 23:59:59.
pub fn to_filter(period: &Period) -> sapphire_journal_core::period::Period {
    sapphire_journal_core::period::Period::Range(
        period.first_day().and_hms_opt(0, 0, 0).unwrap(),
        period.last_day().and_hms_opt(23, 59, 59).unwrap(),
    )
}

/// `period` moved by `by` of its own unit (days, weeks, months or years); `None` for a range,
/// which has no unit to step by, or when the result is out of range.
pub fn shift(period: &Period, by: i32) -> Option<Period> {
    match period {
        Period::Day(d) => d
            .checked_add_signed(chrono::Duration::days(i64::from(by)))
            .map(Period::Day),
        Period::Week { start } => start
            .checked_add_signed(chrono::Duration::days(7 * i64::from(by)))
            .map(|start| Period::Week { start }),
        Period::Month { year, month } => {
            let first = chrono::NaiveDate::from_ymd_opt(*year, *month, 1)?;
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
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
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
        let start = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let sapphire_journal_core::period::Period::Range(a, b) = to_filter(&Period::Week { start })
        else {
            panic!("not a range");
        };
        assert_eq!(a.to_string(), "2026-11-02 00:00:00");
        assert_eq!(b.to_string(), "2026-11-08 23:59:59");
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
