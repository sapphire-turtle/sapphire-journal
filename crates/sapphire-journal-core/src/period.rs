use jiff::civil::{Date, DateTime, ISOWeekDate, Weekday};
use jiff::{Timestamp, ToSpan};
use sapphire_journal_api::time;

/// A time period used for filtering entries.
#[derive(Debug, Clone)]
pub enum Period {
    /// Inclusive range [start, end].
    Range(Timestamp, Timestamp),
    /// Match entries where the field is absent (null / not set).
    None,
}

impl Period {
    /// Returns true if this period matches the given optional field value.
    ///
    /// - `Period::None` matches when `field` is `None`.
    /// - `Period::Range` matches when `field` is `Some(t)` and `start <= t <= end`.
    pub fn matches(&self, field: Option<Timestamp>) -> bool {
        match self {
            Period::None => field.is_none(),
            Period::Range(start, end) => field.is_some_and(|t| t >= *start && t <= *end),
        }
    }

    /// Like [`matches`] but for event overlap: true when the event [event_start, event_end]
    /// overlaps with this period. Either bound may be absent (treated as unbounded).
    pub fn overlaps_event(
        &self,
        event_start: Option<Timestamp>,
        event_end: Option<Timestamp>,
    ) -> bool {
        match self {
            Period::None => event_start.is_none() && event_end.is_none(),
            Period::Range(ps, pe) => {
                if event_start.is_none() && event_end.is_none() {
                    return false;
                }
                let after_period_end = event_start.is_some_and(|es| es > *pe);
                let before_period_start = event_end.is_some_and(|ee| ee < *ps);
                !after_period_end && !before_period_start
            }
        }
    }
}

const INVALID: &str = "is not a valid date/datetime — expected YYYY-MM-DD or YYYY-MM-DDTHH:MM";

/// Parse a moment from `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM[:SS]`, either with an
/// offset or in this host's time zone. Date-only input is that day's start
/// (00:00:00) here.
pub fn parse_datetime(s: &str) -> Result<Timestamp, String> {
    time::parse(s).map_err(|_| format!("`{s}` {INVALID}"))
}

/// Like [`parse_datetime`] but date-only input is the day's end (23:59:59) here.
pub fn parse_datetime_end(s: &str) -> Result<Timestamp, String> {
    if let Some(d) = time::parse_date(s) {
        return local(d.at(23, 59, 59, 0), s);
    }
    parse_datetime(s)
}

fn local(datetime: DateTime, s: &str) -> Result<Timestamp, String> {
    time::from_local(datetime).map_err(|_| format!("`{s}` {INVALID}"))
}

/// Return the ISO week range (Monday..Sunday) for the given date.
fn iso_week_range(d: Date) -> (Date, Date) {
    let monday = d - i64::from(d.weekday().to_monday_zero_offset()).days();
    (monday, monday + 6.days())
}

/// Return the first and last day of the month containing `d`.
fn month_range(year: i16, month: i8) -> (Date, Date) {
    let first = Date::new(year, month, 1).unwrap();
    (first, first.last_of_month())
}

/// Wrap a date range into a [`Period::Range`].
fn day_range(start: Date, end: Date) -> Period {
    let at = |d: DateTime| time::from_local(d).unwrap_or_else(|_| Timestamp::now());
    Period::Range(at(start.at(0, 0, 0, 0)), at(end.at(23, 59, 59, 0)))
}

/// Parse an ISO week string like `2026-W15` into (Monday, Sunday).
fn parse_iso_week(s: &str) -> Result<(Date, Date), String> {
    // Expected format: YYYY-Www
    let (year_s, w_s) = s
        .split_once("-W")
        .or_else(|| s.split_once("-w"))
        .ok_or_else(|| format!("`{s}` is not a valid ISO week — expected YYYY-Www"))?;
    let year: i16 = year_s
        .parse()
        .map_err(|_| format!("`{year_s}` is not a valid year"))?;
    let week: i8 = w_s
        .parse()
        .map_err(|_| format!("`{w_s}` is not a valid week number"))?;
    if week == 0 || week > 53 {
        return Err(format!("week number {week} is out of range (1–53)"));
    }
    let monday = ISOWeekDate::new(year, week, Weekday::Monday)
        .map_err(|_| format!("`{s}` does not correspond to a valid ISO week"))?
        .date();
    Ok((monday, monday + 6.days()))
}

/// Parse a period string into a [`Period`].
///
/// Accepted formats:
/// - `none`                                  → [`Period::None`] (field is absent)
/// - `today`                                 → today 00:00:00 .. 23:59:59
/// - `this_week`                             → ISO week (Mon–Sun) of the current week
/// - `this_month`                            → first .. last day of the current calendar month
/// - `YYYY`                                  → that year (Jan 1 .. Dec 31)
/// - `YYYY-MM`                               → that month (1st .. last day)
/// - `YYYY-Www`                              → ISO week (Mon .. Sun)
/// - `YYYY-MM-DD`                            → that day 00:00:00 .. 23:59:59
/// - `YYYY-MM-DD/YYYY-MM-DD`                → start 00:00:00 .. end 23:59:59
/// - `YYYY-MM-DDTHH:MM/YYYY-MM-DDTHH:MM`   → exact datetime range (start inclusive, end inclusive)
pub fn parse_period(s: &str) -> Result<Period, String> {
    if s == "none" {
        return Ok(Period::None);
    }

    let today = time::local_date(Timestamp::now());

    if s == "today" {
        return Ok(day_range(today, today));
    }

    if s == "yesterday" {
        let d = today - 1.day();
        return Ok(day_range(d, d));
    }

    if s == "tomorrow" {
        let d = today + 1.day();
        return Ok(day_range(d, d));
    }

    if s == "this_week" {
        let (mon, sun) = iso_week_range(today);
        return Ok(day_range(mon, sun));
    }

    if s == "last_week" {
        let (mon, sun) = iso_week_range(today - 7.days());
        return Ok(day_range(mon, sun));
    }

    if s == "next_week" {
        let (mon, sun) = iso_week_range(today + 7.days());
        return Ok(day_range(mon, sun));
    }

    if s == "this_month" {
        let (first, last) = month_range(today.year(), today.month());
        return Ok(day_range(first, last));
    }

    if s == "last_month" {
        let (year, month) = if today.month() == 1 {
            (today.year() - 1, 12i8)
        } else {
            (today.year(), today.month() - 1)
        };
        let (first, last) = month_range(year, month);
        return Ok(day_range(first, last));
    }

    if s == "next_month" {
        let (year, month) = if today.month() == 12 {
            (today.year() + 1, 1i8)
        } else {
            (today.year(), today.month() + 1)
        };
        let (first, last) = month_range(year, month);
        return Ok(day_range(first, last));
    }

    // ISO time interval: slash-separated range
    // (e.g. "2026-03-01/2026-03-07" or "2026-03-01T09:00/2026-03-01T17:00")
    if let Some((left, right)) = s.split_once('/') {
        let start = parse_datetime(left)
            .map_err(|_| format!("`{left}` is not a valid date/datetime in period `{s}`"))?;
        let end = parse_datetime_end(right)
            .map_err(|_| format!("`{right}` is not a valid date/datetime in period `{s}`"))?;
        return Ok(Period::Range(start, end));
    }

    // ISO week: YYYY-Www (e.g. "2026-W15")
    if s.contains("-W") || s.contains("-w") {
        let (mon, sun) = parse_iso_week(s)?;
        return Ok(day_range(mon, sun));
    }

    // Single date: YYYY-MM-DD
    if let Some(d) = time::parse_date(s) {
        return Ok(day_range(d, d));
    }

    // Year-month: YYYY-MM
    if let Some((year_s, month_s)) = s.split_once('-')
        && let (Ok(year), Ok(month)) = (year_s.parse::<i16>(), month_s.parse::<i8>())
        && (1..=12).contains(&month)
    {
        let (first, last) = month_range(year, month);
        return Ok(day_range(first, last));
    }

    // Year only: YYYY
    if let Ok(year) = s.parse::<i16>()
        && (1000..=9999).contains(&year)
    {
        let first = Date::new(year, 1, 1).unwrap();
        let last = Date::new(year, 12, 31).unwrap();
        return Ok(day_range(first, last));
    }

    Err(format!(
        "`{s}` is not a valid period — accepted: \
         none | today | yesterday | tomorrow | \
         this_week | last_week | next_week | \
         this_month | last_month | next_month | \
         YYYY | YYYY-MM | YYYY-Www | YYYY-MM-DD | \
         YYYY-MM-DD/YYYY-MM-DD | YYYY-MM-DDTHH:MM/YYYY-MM-DDTHH:MM"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> Timestamp {
        time::from_local(s.parse().unwrap()).unwrap()
    }

    #[test]
    fn parse_single_date() {
        let p = parse_period("2026-03-05").unwrap();
        let Period::Range(s, e) = p else { panic!() };
        assert_eq!(s, dt("2026-03-05T00:00:00"));
        assert_eq!(e, dt("2026-03-05T23:59:59"));
    }

    #[test]
    fn parse_date_range_slash() {
        let p = parse_period("2026-03-01/2026-03-07").unwrap();
        let Period::Range(s, e) = p else { panic!() };
        assert_eq!(s, dt("2026-03-01T00:00:00"));
        assert_eq!(e, dt("2026-03-07T23:59:59"));
    }

    #[test]
    fn parse_datetime_range_slash() {
        let p = parse_period("2026-03-01T09:00/2026-03-01T17:30").unwrap();
        let Period::Range(s, e) = p else { panic!() };
        assert_eq!(s, dt("2026-03-01T09:00:00"));
        assert_eq!(e, dt("2026-03-01T17:30:00"));
    }

    #[test]
    fn parse_year() {
        let p = parse_period("2026").unwrap();
        let Period::Range(s, e) = p else { panic!() };
        assert_eq!(s, dt("2026-01-01T00:00:00"));
        assert_eq!(e, dt("2026-12-31T23:59:59"));
    }

    #[test]
    fn parse_year_month() {
        let p = parse_period("2026-04").unwrap();
        let Period::Range(s, e) = p else { panic!() };
        assert_eq!(s, dt("2026-04-01T00:00:00"));
        assert_eq!(e, dt("2026-04-30T23:59:59"));
    }

    #[test]
    fn parse_iso_week_format() {
        // 2026-W15 = 2026-04-06 (Mon) to 2026-04-12 (Sun)
        let p = parse_period("2026-W15").unwrap();
        let Period::Range(s, e) = p else { panic!() };
        assert_eq!(s, dt("2026-04-06T00:00:00"));
        assert_eq!(e, dt("2026-04-12T23:59:59"));
    }

    #[test]
    fn parse_none() {
        let p = parse_period("none").unwrap();
        assert!(matches!(p, Period::None));
    }

    #[test]
    fn period_none_matches_absent() {
        assert!(Period::None.matches(None));
        assert!(!Period::None.matches(Some(dt("2026-03-05T10:00:00"))));
    }

    #[test]
    fn period_range_matches_inclusive() {
        let p = Period::Range(dt("2026-03-05T00:00:00"), dt("2026-03-05T23:59:59"));
        assert!(p.matches(Some(dt("2026-03-05T12:00:00"))));
        assert!(!p.matches(Some(dt("2026-03-06T00:00:00"))));
        assert!(!p.matches(None));
    }

    #[test]
    fn overlaps_event_no_event_never_matches() {
        let p = Period::Range(dt("2026-03-08T00:00:00"), dt("2026-03-08T23:59:59"));
        assert!(!p.overlaps_event(None, None));
    }

    #[test]
    fn overlaps_event_spanning_period() {
        let p = Period::Range(dt("2026-03-08T00:00:00"), dt("2026-03-08T23:59:59"));
        assert!(p.overlaps_event(
            Some(dt("2026-03-01T00:00:00")),
            Some(dt("2026-03-31T23:59:59")),
        ));
    }

    #[test]
    fn overlaps_event_outside_period() {
        let p = Period::Range(dt("2026-03-08T00:00:00"), dt("2026-03-08T23:59:59"));
        assert!(!p.overlaps_event(
            Some(dt("2026-03-10T00:00:00")),
            Some(dt("2026-03-20T23:59:59")),
        ));
    }
}
