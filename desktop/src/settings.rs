//! Persisted UI preferences: the desktop's own, not the journal's.
//!
//! Lives at `<data dir>/sapphire-journal/settings.toml`. Which journal is
//! open is the server's to remember (`workspace select`), not this file's.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// On-disk user preferences for the GUI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// How the journal screen lays out the entries.
    #[serde(default)]
    pub view: MainView,
    /// How the list view lays out the entries.
    #[serde(default)]
    pub list_layout: ListLayout,
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

impl Settings {
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
