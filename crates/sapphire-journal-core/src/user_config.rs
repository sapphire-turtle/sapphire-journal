use std::path::PathBuf;
use std::time::Duration;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Default cadence (minutes) for periodic re-indexing of changed files.
///
/// Long-running frontends (the app server, the desktop app) re-scan the journal's
/// entry cache at this interval. Set `sync_interval_minutes = 0` to disable.
const DEFAULT_SYNC_INTERVAL_MINUTES: u32 = 10;

fn default_sync_interval_minutes() -> Option<u32> {
    Some(DEFAULT_SYNC_INTERVAL_MINUTES)
}

/// Contents of `$XDG_CONFIG_HOME/sapphire-journal/config.toml`.
///
/// This is a user-level (host-level) config for machine-specific settings. It
/// is intentionally separate from the per-journal `.sapphire-journal/config.toml`
/// so that the same journal can be shared across machines.
///
/// Embedding is not configured here: the host's `sapphire-bridge` embeds for
/// every app (`sapphire-journal embedding …`). A `[cache.retrieve]` section left
/// over from an older version is kept as it is and ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// How often to run periodic sync, in minutes.
    ///
    /// Defaults to 10 minutes. Set to `0` to disable.
    #[serde(default = "default_sync_interval_minutes", skip_serializing_if = "Option::is_none")]
    pub sync_interval_minutes: Option<u32>,

    /// Unknown fields preserved for round-trip TOML compatibility.
    #[serde(flatten)]
    pub extra: IndexMap<String, toml::Value>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            sync_interval_minutes: default_sync_interval_minutes(),
            extra: IndexMap::new(),
        }
    }
}

impl UserConfig {
    /// Periodic sync cadence as a [`Duration`]. Returns `None` when disabled
    /// (`sync_interval_minutes = 0`).
    pub fn sync_interval(&self) -> Option<Duration> {
        self.sync_interval_minutes
            .filter(|&m| m > 0)
            .map(|m| Duration::from_secs(u64::from(m) * 60))
    }

    /// Canonical path to the user config file.
    ///
    /// Resolves to `$XDG_CONFIG_HOME/sapphire-journal/config.toml`.
    pub fn path() -> PathBuf {
        xdg_config_home().join("sapphire-journal").join("config.toml")
    }

    /// Load the user config from disk, then apply environment variable overrides.
    ///
    /// Returns the default config if the file does not exist.
    ///
    /// `SAPPHIRE_JOURNAL_SYNC_INTERVAL_MINUTES` overrides `sync_interval_minutes`.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        let mut config = if !path.exists() {
            UserConfig::default()
        } else {
            let contents = std::fs::read_to_string(&path)?;
            toml::from_str(&contents).map_err(|e| Error::InvalidConfig(e.to_string()))?
        };
        config.apply_env_overrides();
        Ok(config)
    }

    /// Write the config to [`UserConfig::path`], creating its directory.
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| Error::InvalidConfig(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("SAPPHIRE_JOURNAL_SYNC_INTERVAL_MINUTES") {
            if let Ok(n) = v.parse::<u32>() {
                self.sync_interval_minutes = Some(n);
            }
        }
    }
}

fn xdg_config_home() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config");
    }
    std::env::temp_dir()
}
