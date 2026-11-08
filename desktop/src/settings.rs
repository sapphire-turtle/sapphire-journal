//! Persisted UI preferences: the desktop's own, not the journal's.
//!
//! Lives at `<data dir>/sapphire-journal/settings.toml`. Which journal is
//! open is the server's to remember (`workspace select`), not this file's.

use std::path::{Path, PathBuf};

use chrono::{NaiveDate, Weekday};
use corundum_framework_gui::datetime::Period;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::period_nav::parse_period_string;

/// On-disk user preferences for the GUI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// How the journal screen lays out the entries.
    #[serde(default)]
    pub view: MainView,
    /// How the list view lays out the entries.
    #[serde(default)]
    pub list_layout: ListLayout,
    /// Which period the window opens on.
    #[serde(default)]
    pub startup_period: StartupPeriod,
    /// The period the window last showed, as `Period` `Display` writes it (`""` for all).
    #[serde(default)]
    pub last_period: Option<String>,
    /// Where [`save`](Self::save) writes: the file [`load`](Self::load) read.
    #[serde(skip)]
    path: PathBuf,
}

/// The journal screen's view of the entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainView {
    /// The entries as a tree or a flat list.
    #[default]
    List,
    /// Tasks in one column per status.
    Kanban,
}

/// How the list view lays out entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListLayout {
    /// Hierarchical view following `parent_id` relationships.
    #[default]
    Tree,
    /// Flat list, ignoring hierarchy.
    Flat,
}

/// Which period the window opens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupPeriod {
    /// The one shown when the window last closed.
    #[default]
    Last,
    Today,
    ThisWeek,
    ThisMonth,
}

impl Settings {
    /// The period the window opens on (`None` = all entries). A missing or unparsable
    /// `last_period` falls back to this week. Weeks start on Monday.
    pub fn initial_period(&self, today: NaiveDate) -> Option<Period> {
        let this_week = || Some(Period::week_of(today, Weekday::Mon));
        match self.startup_period {
            StartupPeriod::Today => Some(Period::Day(today)),
            StartupPeriod::ThisWeek => this_week(),
            StartupPeriod::ThisMonth => Some(Period::month_of(today)),
            StartupPeriod::Last => self
                .last_period
                .as_deref()
                .and_then(parse_period_string)
                .unwrap_or_else(this_week),
        }
    }

    /// `settings.toml` in the desktop's data directory (`app::run`'s
    /// `AppSetup::data_dir`, `<data dir>/sapphire-journal`).
    fn path(data_dir: &Path) -> PathBuf {
        data_dir.join("settings.toml")
    }

    /// Load the settings from `data_dir`: the defaults if the file does not
    /// exist or cannot be read. Either way they are saved back there.
    pub fn load(data_dir: &Path) -> Self {
        let path = Self::path(data_dir);
        let mut settings = Self::read(&path).unwrap_or_else(|e| {
            tracing::warn!(path = %path.display(), "using the default UI settings: {e}");
            Self::default()
        });
        settings.path = path;
        settings
    }

    fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    /// Persist the settings to where they were loaded from.
    pub fn save(&self) -> Result<()> {
        let path = &self.path;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sun() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()
    }

    fn with(startup: StartupPeriod, last: Option<&str>) -> Settings {
        Settings {
            startup_period: startup,
            last_period: last.map(str::to_owned),
            ..Settings::default()
        }
    }

    fn this_week() -> Option<Period> {
        Some(Period::Week {
            start: NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
        })
    }

    #[test]
    fn initial_today() {
        assert_eq!(
            with(StartupPeriod::Today, None).initial_period(sun()),
            Some(Period::Day(sun()))
        );
    }

    #[test]
    fn initial_this_week() {
        assert_eq!(
            with(StartupPeriod::ThisWeek, None).initial_period(sun()),
            this_week()
        );
    }

    #[test]
    fn initial_this_month() {
        assert_eq!(
            with(StartupPeriod::ThisMonth, None).initial_period(sun()),
            Some(Period::Month {
                year: 2026,
                month: 11
            })
        );
    }

    #[test]
    fn last_restores() {
        assert_eq!(
            with(StartupPeriod::Last, Some("2026-11")).initial_period(sun()),
            Some(Period::Month {
                year: 2026,
                month: 11
            })
        );
    }

    #[test]
    fn last_all() {
        assert_eq!(
            with(StartupPeriod::Last, Some("")).initial_period(sun()),
            None
        );
    }

    #[test]
    fn last_missing_is_this_week() {
        assert_eq!(
            with(StartupPeriod::Last, None).initial_period(sun()),
            this_week()
        );
    }

    #[test]
    fn last_garbage_is_this_week() {
        assert_eq!(
            with(StartupPeriod::Last, Some("garbage")).initial_period(sun()),
            this_week()
        );
    }

    #[test]
    fn old_file_loads() {
        let s: Settings = toml::from_str("view = \"list\"\nlist_layout = \"tree\"\n").unwrap();
        assert_eq!(s.startup_period, StartupPeriod::Last);
        assert_eq!(s.last_period, None);
    }

    #[test]
    fn a_file_with_the_old_pane_placement_still_loads() {
        let settings: Settings = toml::from_str(
            r#"
view = "kanban"
list_layout = "flat"

[pane]
placement = "floating"
width = 420.0
"#,
        )
        .unwrap();
        assert_eq!(settings.view, MainView::Kanban);
        assert_eq!(settings.list_layout, ListLayout::Flat);
        assert!(!toml::to_string(&settings).unwrap().contains("pane"));
    }
}
