//! A frontmatter value that is either a date (all day) or a date and time.

use std::{borrow::Cow, fmt, str::FromStr};

use chrono::{NaiveDate, NaiveDateTime};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const DATE_FORMAT: &str = "%Y-%m-%d";
const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M";

/// A date (meaning all day) or a date and time, kept as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateOrTime {
    Date(NaiveDate),
    DateTime(NaiveDateTime),
}

impl DateOrTime {
    /// The first moment covered: `00:00:00` for a date, the time itself otherwise.
    pub fn start_bound(&self) -> NaiveDateTime {
        match self {
            Self::Date(d) => d.and_hms_opt(0, 0, 0).unwrap(),
            Self::DateTime(dt) => *dt,
        }
    }

    /// The last moment covered: `23:59:59` for a date, the time itself otherwise.
    pub fn end_bound(&self) -> NaiveDateTime {
        match self {
            Self::Date(d) => d.and_hms_opt(23, 59, 59).unwrap(),
            Self::DateTime(dt) => *dt,
        }
    }

    pub fn is_date(&self) -> bool {
        matches!(self, Self::Date(_))
    }

    pub fn date(&self) -> NaiveDate {
        match self {
            Self::Date(d) => *d,
            Self::DateTime(dt) => dt.date(),
        }
    }
}

impl FromStr for DateOrTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for fmt in [DATETIME_FORMAT, "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M:%S%.f"] {
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
                return Ok(Self::DateTime(dt));
            }
        }
        NaiveDate::parse_from_str(s, DATE_FORMAT)
            .map(Self::Date)
            .map_err(|_| {
                format!(
                    "cannot parse `{s}` as a date or datetime; expected YYYY-MM-DD or YYYY-MM-DDTHH:MM"
                )
            })
    }
}

impl fmt::Display for DateOrTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Date(d) => write!(f, "{}", d.format(DATE_FORMAT)),
            Self::DateTime(dt) => write!(f, "{}", dt.format(DATETIME_FORMAT)),
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
            "description": "YYYY-MM-DD (all day) or YYYY-MM-DDTHH:MM"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }
    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
    }

    #[test]
    fn date_round_trips() {
        let v: DateOrTime = "2026-11-08".parse().unwrap();
        assert_eq!(v, DateOrTime::Date(d("2026-11-08")));
        assert_eq!(v.to_string(), "2026-11-08");
    }

    #[test]
    fn midnight_keeps_its_time() {
        let v: DateOrTime = "2026-11-08T00:00".parse().unwrap();
        assert!(matches!(v, DateOrTime::DateTime(_)));
        assert!(!v.is_date());
        assert_eq!(v.to_string(), "2026-11-08T00:00");
    }

    #[test]
    fn legacy_formats_read() {
        for s in ["2026-11-08T09:30:15", "2026-11-08T09:30:15.250"] {
            let v: DateOrTime = s.parse().unwrap();
            assert!(matches!(v, DateOrTime::DateTime(_)), "{s}");
            assert_eq!(v.to_string(), "2026-11-08T09:30");
        }
    }

    #[test]
    fn bounds_of_a_date() {
        let v = DateOrTime::Date(d("2026-11-08"));
        assert_eq!(v.start_bound(), dt("2026-11-08T00:00:00"));
        assert_eq!(v.end_bound(), dt("2026-11-08T23:59:59"));
        assert_eq!(v.date(), d("2026-11-08"));
    }

    #[test]
    fn bounds_of_a_datetime_are_itself() {
        let v = DateOrTime::DateTime(dt("2026-11-08T09:30:00"));
        assert_eq!(v.start_bound(), dt("2026-11-08T09:30:00"));
        assert_eq!(v.end_bound(), dt("2026-11-08T09:30:00"));
        assert_eq!(v.date(), d("2026-11-08"));
    }

    #[test]
    fn serde_round_trip() {
        for s in ["2026-11-08", "2026-11-08T10:00"] {
            let json = serde_json::to_string(&s).unwrap();
            let v: DateOrTime = serde_json::from_str(&json).unwrap();
            assert_eq!(serde_json::to_string(&v).unwrap(), json);
        }
    }

    #[test]
    fn rejects_garbage() {
        let err = "11/08".parse::<DateOrTime>().unwrap_err();
        assert!(err.contains("YYYY-MM-DD"), "{err}");
    }
}
