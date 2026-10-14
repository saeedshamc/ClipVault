//! TOML configuration loading and defaults.

use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Default maximum size of a single clipboard item (20 MiB).
pub const DEFAULT_MAX_ITEM_BYTES: u64 = 20 * 1024 * 1024;

/// Default maximum number of retained items.
pub const DEFAULT_MAX_ITEMS: u64 = 10_000;

/// Default maximum total storage size (1 GiB).
pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;

/// Top-level ClipVault configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub storage: StorageConfig,
    pub limits: LimitsConfig,
    pub retention: RetentionConfig,
    pub watcher: WatcherConfig,
}

/// Database and blob directory settings (paths relative to the data dir unless absolute).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub database: String,
    pub blobs_dir: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database: "clipvault.db".into(),
            blobs_dir: "blobs".into(),
        }
    }
}

/// Soft limits enforced when capturing clipboard data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    pub max_item_bytes: u64,
    pub max_items: u64,
    pub max_total_bytes: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_item_bytes: DEFAULT_MAX_ITEM_BYTES,
            max_items: DEFAULT_MAX_ITEMS,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

/// Retention policy knobs (enforced in the retention milestone).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RetentionConfig {
    /// Delete unpinned items older than this many days. `0` disables age-based retention.
    pub max_age_days: u64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self { max_age_days: 90 }
    }
}

/// Watcher runtime toggles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WatcherConfig {
    /// When true, the daemon must not persist new clipboard captures.
    pub paused: bool,
    /// Linux: also watch PRIMARY selection (X11). Ignored on Windows.
    pub capture_primary: bool,
}

/// Resolved filesystem locations for a ClipVault profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub config_file: PathBuf,
    pub database_file: PathBuf,
    pub blobs_dir: PathBuf,
}

impl AppPaths {
    /// Resolve platform-standard directories via the `directories` crate.
    pub fn resolve(config: &Config) -> CoreResult<Self> {
        let dirs = ProjectDirs::from("dev", "ClipVault", "ClipVault").ok_or_else(|| {
            CoreError::Config("unable to determine project directories for this platform".into())
        })?;

        let config_dir = dirs.config_dir().to_path_buf();
        let data_dir = dirs.data_dir().to_path_buf();
        let state_dir = dirs
            .state_dir()
            .unwrap_or_else(|| dirs.data_local_dir())
            .to_path_buf();

        let database_file = resolve_under(&data_dir, &config.storage.database);
        let blobs_dir = resolve_under(&data_dir, &config.storage.blobs_dir);

        Ok(Self {
            config_file: config_dir.join("config.toml"),
            config_dir,
            data_dir,
            state_dir,
            database_file,
            blobs_dir,
        })
    }

    /// Create config/data/state/blobs directories if missing.
    pub fn ensure_directories(&self) -> CoreResult<()> {
        for path in [
            &self.config_dir,
            &self.data_dir,
            &self.state_dir,
            &self.blobs_dir,
        ] {
            fs::create_dir_all(path).map_err(|err| CoreError::Path {
                path: path.clone(),
                message: err.to_string(),
            })?;
        }
        Ok(())
    }
}

fn resolve_under(base: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

impl Config {
    /// Parse configuration from a TOML string.
    pub fn from_toml_str(raw: &str) -> CoreResult<Self> {
        let config: Config = toml::from_str(raw)?;
        config.validate()?;
        Ok(config)
    }

    /// Load from `path`, or return defaults if the file does not exist.
    pub fn load_or_default(path: &Path) -> CoreResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Self::from_toml_str(&raw)
    }

    /// Serialize and write TOML to `path`, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> CoreResult<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| CoreError::Path {
                path: parent.to_path_buf(),
                message: err.to_string(),
            })?;
        }
        let raw = toml::to_string_pretty(self).map_err(|err| CoreError::Config(err.to_string()))?;
        fs::write(path, raw)?;
        Ok(())
    }

    /// Reject nonsensical limit combinations.
    pub fn validate(&self) -> CoreResult<()> {
        if self.limits.max_item_bytes == 0 {
            return Err(CoreError::Config(
                "limits.max_item_bytes must be greater than zero".into(),
            ));
        }
        if self.limits.max_items == 0 {
            return Err(CoreError::Config(
                "limits.max_items must be greater than zero".into(),
            ));
        }
        if self.storage.database.trim().is_empty() {
            return Err(CoreError::Config(
                "storage.database must not be empty".into(),
            ));
        }
        if self.storage.blobs_dir.trim().is_empty() {
            return Err(CoreError::Config(
                "storage.blobs_dir must not be empty".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_toml_roundtrip() {
        let config = Config::default();
        let raw = toml::to_string_pretty(&config).expect("serialize");
        let parsed = Config::from_toml_str(&raw).expect("parse");
        assert_eq!(parsed, config);
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("missing.toml");
        let config = Config::load_or_default(&path).expect("load");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn save_and_load_preserves_values() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let mut config = Config::default();
        config.limits.max_items = 42;
        config.watcher.paused = true;
        config.save(&path).expect("save");
        let loaded = Config::load_or_default(&path).expect("load");
        assert_eq!(loaded.limits.max_items, 42);
        assert!(loaded.watcher.paused);
    }

    #[test]
    fn rejects_zero_max_item_bytes() {
        let raw = r#"
            [limits]
            max_item_bytes = 0
        "#;
        let err = Config::from_toml_str(raw).expect_err("should fail");
        assert!(matches!(err, CoreError::Config(_)));
    }

    #[test]
    fn resolve_paths_are_absolute() {
        let paths = AppPaths::resolve(&Config::default()).expect("resolve");
        assert!(paths.database_file.is_absolute());
        assert!(paths.blobs_dir.is_absolute());
        assert!(paths.config_file.ends_with("config.toml"));
    }
}
