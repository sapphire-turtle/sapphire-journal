//! The shared period (`None` = all entries) as alumina's [`Period`], and its conversions
//! to the core filter and to the string saved in `settings.toml`.

use corundum_framework_gui::datetime::Period;

/// The core filter for `period`: its first day at 00:00:00 through its last day at 23:59:59.
pub fn to_filter(period: &Period) -> sapphire_journal_core::period::Period {
    sapphire_journal_core::period::Period::Range(
        period.first_day().and_hms_opt(0, 0, 0).unwrap(),
        period.last_day().and_hms_opt(23, 59, 59).unwrap(),
    )
}

/// `period` as `Display` writes it, or `""` for all entries.
#[allow(dead_code)] // saved by the startup setting (Task 7)
pub fn period_string(period: Option<&Period>) -> String {
    period.map(ToString::to_string).unwrap_or_default()
}

/// The inverse of [`period_string`]: `""` is `Some(None)` (all), anything unparsable `None`.
#[allow(dead_code)] // read by the startup setting (Task 7)
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
