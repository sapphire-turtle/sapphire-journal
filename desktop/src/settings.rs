//! Persisted UI preferences: the desktop's own, not the journal's.
//!
//! Lives at `<data dir>/sapphire-journal/settings.toml`. Which journal is
//! open is the server's to remember (`workspace select`), not this file's.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// On-disk user preferences for the GUI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Right metadata sidebar (Obsidian-style) preferences.
    #[serde(default)]
    pub right_sidebar: RightSidebarSettings,
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
    /// `<data dir>/sapphire-journal/settings.toml`
    pub fn path() -> PathBuf {
        sapphire_journal_core::JOURNAL_CTX
            .data_dir()
            .join("settings.toml")
    }

    /// Load the settings from disk, returning defaults if the file does not exist.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)?;
        let settings: Self = toml::from_str(&contents)?;
        Ok(settings)
    }

    /// Persist the settings to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}
