//! A frontmatter value that is either a date (all day) or a moment.

use std::{borrow::Cow, fmt, str::FromStr};

use jiff::Timestamp;
use jiff::civil::Date;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::time;

/// A date (meaning all day, wherever the reader is) or a moment.
///
/// Written as `2026-11-08` or `2026-11-08T10:00+09:00`. A time written
/// without an offset (`2026-11-08T10:00`, as entries were before the move to
/// jiff) is read in this host's time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateOrTime {
    Date(Date),
    Time(Timestamp),
}

impl DateOrTime {
    /// The first moment covered: the date's midnight here, the moment itself
    /// otherwise.
    pub fn start_bound(&self) -> Timestamp {
        match self {
            Self::Date(d) => local_or_now(d.at(0, 0, 0, 0)),
            Self::Time(t) => *t,
        }
    }

    /// The last moment covered: the date's `23:59:59` here, the moment itself
    /// otherwise.
    pub fn end_bound(&self) -> Timestamp {
        match self {
            Self::Date(d) => local_or_now(d.at(23, 59, 59, 0)),
            Self::Time(t) => *t,
        }
    }

    pub fn is_date(&self) -> bool {
        matches!(self, Self::Date(_))
    }

    /// The date, or the moment's date in this host's time zone.
    pub fn date(&self) -> Date {
        match self {
            Self::Date(d) => *d,
            Self::Time(t) => time::local_date(*t),
        }
    }
}

/// A wall-clock time here. Every civil date's midnight and 23:59:59 is a
/// time somewhere, so this only fails for years jiff cannot represent.
fn local_or_now(datetime: jiff::civil::DateTime) -> Timestamp {
    time::from_local(datetime).unwrap_or_else(|_| Timestamp::now())
}

impl FromStr for DateOrTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if let Some(date) = time::parse_date(s) {
            return Ok(Self::Date(date));
        }
        time::parse(s).map(Self::Time).map_err(|_| {
            format!(
                "cannot parse `{s}` as a date or datetime; expected YYYY-MM-DD or YYYY-MM-DDTHH:MM"
            )
        })
    }
}

impl fmt::Display for DateOrTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Date(d) => write!(f, "{}", d.strftime("%Y-%m-%d")),
            Self::Time(t) => f.write_str(&time::format(*t)),
        }
    }
}

impl Serialize for DateOrTime {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for DateOrTime {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for DateOrTime {
    fn schema_name() -> Cow<'static, str> {
        "DateOrTime".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "description": "YYYY-MM-DD (all day) or YYYY-MM-DDTHH:MM, with an offset (+09:00) or in the server's time zone"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }
    fn local(s: &str) -> Timestamp {
        time::from_local(s.parse().unwrap()).unwrap()
    }

    #[test]
    fn date_round_trips() {
        let v: DateOrTime = "2026-11-08".parse().unwrap();
        assert_eq!(v, DateOrTime::Date(d("2026-11-08")));
        assert_eq!(v.to_string(), "2026-11-08");
    }

    #[test]
    fn a_time_round_trips_with_its_offset() {
        let v: DateOrTime = "2026-11-08T00:00+09:00".parse().unwrap();
        assert_eq!(v, DateOrTime::Time("2026-11-07T15:00Z".parse().unwrap()));
        assert!(!v.is_date());
        assert_eq!(v.to_string().parse::<DateOrTime>().unwrap(), v);
    }

    #[test]
    fn midnight_keeps_its_time() {
        let v: DateOrTime = "2026-11-08T00:00".parse().unwrap();
        assert_eq!(v, DateOrTime::Time(local("2026-11-08T00:00")));
        assert!(!v.is_date());
    }

    #[test]
    fn legacy_formats_read_in_the_local_time_zone() {
        for s in [
            "2026-11-08T09:30",
            "2026-11-08T09:30:15",
            "2026-11-08T09:30:15.250",
        ] {
            let v: DateOrTime = s.parse().unwrap();
            assert_eq!(v, DateOrTime::Time(local(s)), "{s}");
            assert_eq!(v.date(), d("2026-11-08"));
        }
    }

    #[test]
    fn bounds_of_a_date() {
        let v = DateOrTime::Date(d("2026-11-08"));
        assert_eq!(v.start_bound(), local("2026-11-08T00:00:00"));
        assert_eq!(v.end_bound(), local("2026-11-08T23:59:59"));
        assert_eq!(v.date(), d("2026-11-08"));
    }

    #[test]
    fn bounds_of_a_time_are_itself() {
        let t = local("2026-11-08T09:30:00");
        let v = DateOrTime::Time(t);
        assert_eq!(v.start_bound(), t);
        assert_eq!(v.end_bound(), t);
        assert_eq!(v.date(), d("2026-11-08"));
    }

    #[test]
    fn serde_round_trip() {
        for s in ["2026-11-08", "2026-11-08T10:00+09:00"] {
            let json = serde_json::to_string(&s).unwrap();
            let v: DateOrTime = serde_json::from_str(&json).unwrap();
            let back: DateOrTime =
                serde_json::from_str(&serde_json::to_string(&v).unwrap()).unwrap();
            assert_eq!(back, v);
        }
    }

    #[test]
    fn rejects_garbage() {
        let err = "11/08".parse::<DateOrTime>().unwrap_err();
        assert!(err.contains("YYYY-MM-DD"), "{err}");
    }
}
