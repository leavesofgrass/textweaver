use std::path::{Path, PathBuf};

use crate::StoreError;

/// Where textweaver keeps its files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// Configuration: `settings.toml`, `keymap.toml`.
    pub config_dir: PathBuf,
    /// Data: `state/`, `recovery/`, `recent.json`, `library.json`.
    pub data_dir: PathBuf,
    /// Disposable caches (parsed documents).
    pub cache_dir: PathBuf,
}

impl Paths {
    /// The platform directories (`%APPDATA%\leavesofgrass\textweaver`,
    /// `~/Library/Application Support/org.leavesofgrass.textweaver`,
    /// `$XDG_CONFIG_HOME/textweaver`, ...). `TEXTWEAVER_HOME`, when set,
    /// puts everything under one directory instead.
    pub fn platform() -> Result<Self, StoreError> {
        if let Some(home) = std::env::var_os("TEXTWEAVER_HOME") {
            return Ok(Paths::under(Path::new(&home)));
        }
        let dirs = directories::ProjectDirs::from("org", "leavesofgrass", "textweaver")
            .ok_or(StoreError::NoConfigDir)?;
        Ok(Paths {
            config_dir: dirs.config_dir().to_owned(),
            data_dir: dirs.data_dir().to_owned(),
            cache_dir: dirs.cache_dir().to_owned(),
        })
    }

    /// Everything under `root` (tests, portable installs).
    pub fn under(root: &Path) -> Self {
        Paths {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
        }
    }

    /// `settings.toml`.
    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.toml")
    }

    /// `keymap.toml`.
    pub fn keymap_file(&self) -> PathBuf {
        self.config_dir.join("keymap.toml")
    }

    /// Per-document state directory.
    pub fn state_dir(&self) -> PathBuf {
        self.data_dir.join("state")
    }

    /// Autosave recovery snapshots.
    pub fn recovery_dir(&self) -> PathBuf {
        self.data_dir.join("recovery")
    }

    /// `recent.json`.
    pub fn recent_file(&self) -> PathBuf {
        self.data_dir.join("recent.json")
    }

    /// `library.json`, the bookshelf.
    pub fn library_file(&self) -> PathBuf {
        self.data_dir.join("library.json")
    }

    /// The library search cache.
    pub fn fulltext_file(&self) -> PathBuf {
        self.cache_dir.join("fulltext.json")
    }
}
