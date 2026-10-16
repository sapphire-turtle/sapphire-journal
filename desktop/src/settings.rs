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
    /// Right metadata sidebar (Obsidian-style) preferences.
    #[serde(default)]
    pub right_sidebar: RightSidebarSettings,
    /// Where [`save`](Self::save) writes: the file [`load`](Self::load) read.
    #[serde(skip)]
    path: PathBuf,
}

/// Persisted state of the right metadata sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightSidebarSettings {
    #[serde(default)]
    pub visible: bool,
    #[serde(default = "default_right_sidebar_width")]
    pub width: f32,
    #[serde(default)]
    pub active_tab: RightTab,
}

impl Default for RightSidebarSettings {
    fn default() -> Self {
        Self {
            visible: false,
            width: default_right_sidebar_width(),
            active_tab: RightTab::default(),
        }
    }
}

fn default_right_sidebar_width() -> f32 {
    280.0
}

/// Which tab is currently selected in the right metadata sidebar.
///
/// Only one tab exists today; the enum is laid out so future tabs (outline,
/// backlinks, etc.) can be added without breaking the on-disk format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RightTab {
    #[default]
    Metadata,
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
