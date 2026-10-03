use std::path::{Path, PathBuf};

use crate::StoreError;

/// The medical lexicon overlay's file name in the configuration folder.
pub const MEDICAL_OVERLAY_FILE: &str = "medical-lexicon.toml";

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
    /// `$XDG_CONFIG_HOME/textweaver`, ...). `TEXTWEAVER_HOME`, when set
    /// and not empty, puts everything under one directory instead (an empty
    /// value used to put `config/`, `data/`, and `cache/` in the current
    /// directory).
    pub fn platform() -> Result<Self, StoreError> {
        if let Some(home) = home_override(std::env::var_os("TEXTWEAVER_HOME")) {
            return Ok(Paths::under(&home));
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
    ///
    /// An empty `root` would mean the current directory; callers read it
    /// from `TEXTWEAVER_HOME` through [`platform`](Self::platform), which
    /// ignores an empty value.
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

    /// The user themes folder, `themes/` in the configuration directory
    /// (ADR-0020).
    pub fn themes_dir(&self) -> PathBuf {
        self.config_dir.join("themes")
    }

    /// `recent.json`.
    pub fn recent_file(&self) -> PathBuf {
        self.data_dir.join("recent.json")
    }

    /// `library.json`, the bookshelf.
    pub fn library_file(&self) -> PathBuf {
        self.data_dir.join("library.json")
    }

    /// `profiles.toml`: named settings profiles.
    pub fn profiles_file(&self) -> PathBuf {
        self.config_dir.join("profiles.toml")
    }

    /// `stats.json`: reading statistics.
    pub fn stats_file(&self) -> PathBuf {
        self.data_dir.join("stats.json")
    }

    /// The interface translations folder, `locales/` in the configuration
    /// directory (`<language>.ftl` files).
    pub fn locales_dir(&self) -> PathBuf {
        self.config_dir.join("locales")
    }

    /// The glossary define word reads when `[lexicon] glossary` is unset:
    /// `glossary.txt`, else `glossary.json`, in the configuration directory,
    /// if either exists.
    pub fn default_glossary(&self) -> Option<PathBuf> {
        ["glossary.txt", "glossary.json"]
            .iter()
            .map(|n| self.config_dir.join(n))
            .find(|p| p.is_file())
    }

    /// The medical lexicon overlay read when
    /// `[normalization.medical_lexicon] overlay` is unset:
    /// `medical-lexicon.toml` in the configuration directory, if it exists.
    pub fn default_medical_overlay(&self) -> Option<PathBuf> {
        Some(self.config_dir.join(MEDICAL_OVERLAY_FILE)).filter(|p| p.is_file())
    }

    /// `sync-ids.json`: each document's sync id (ADR-0049), in the data
    /// folder ([`crate::sync_ids`]).
    pub fn sync_ids_file(&self) -> PathBuf {
        self.data_dir.join(crate::sync_ids::SYNC_IDS_FILE)
    }

    /// The library search cache.
    pub fn fulltext_file(&self) -> PathBuf {
        self.cache_dir.join("fulltext.json")
    }
}

/// The `TEXTWEAVER_HOME` directory, if the variable holds one: unset and
/// empty (or all-whitespace) values mean "use the platform directories".
fn home_override(value: Option<std::ffi::OsString>) -> Option<PathBuf> {
    value
        .filter(|v| !v.to_string_lossy().trim().is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_home_override_is_ignored() {
        assert_eq!(home_override(None), None);
        assert_eq!(home_override(Some("".into())), None);
        assert_eq!(home_override(Some("  ".into())), None);
        assert_eq!(
            home_override(Some("/tmp/tw".into())),
            Some(PathBuf::from("/tmp/tw"))
        );
    }
}
