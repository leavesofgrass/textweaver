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

    /// The components folder, `components/` in the data folder: where
    /// optional components from a source or a mirror are installed, each
    /// in its own folder. Helper programs and libraries found here
    /// (ffmpeg, libespeak-ng, an ECI library) are used before the
    /// system's ([`find_in_components`]).
    pub fn components_dir(&self) -> PathBuf {
        self.data_dir.join(COMPONENTS_DIR)
    }

    /// Per-document state directory.
    pub fn state_dir(&self) -> PathBuf {
        self.data_dir.join("state")
    }

    /// Study cards, one file per document (`cards/<doc-key>.json`).
    pub fn cards_dir(&self) -> PathBuf {
        self.data_dir.join(crate::cards::CARDS_DIR)
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

/// The components folder's name in the data folder.
pub const COMPONENTS_DIR: &str = "components";

/// The deepest [`find_in_components`] looks: an unpacked archive keeps
/// its program a few folders down (`ffmpeg/<build>/bin/ffmpeg.exe`).
const FIND_DEPTH: usize = 5;

/// The most entries [`find_in_components`] looks at.
const FIND_ENTRIES: usize = 20_000;

/// The components folder of this computer's textweaver
/// ([`Paths::platform`], so `TEXTWEAVER_HOME` moves it too).
///
/// shortcut: a frontend started with other paths (tests, `--home`) still
/// searches the platform folder; pass the folder to the lookups' `_in`
/// forms when that matters.
pub fn components_dir() -> Option<PathBuf> {
    Paths::platform().ok().map(|p| p.components_dir())
}

/// The first file under `dir` named one of `names` (compared without
/// regard to case on Windows and macOS), searched breadth first in name
/// order, at most a few folders deep: a helper program or library a
/// component placed or unpacked there. `None` when `dir` is missing.
pub fn find_in_components(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    let fold = cfg!(any(windows, target_os = "macos"));
    let wanted = |n: &str| {
        names.iter().any(|w| {
            if fold {
                w.eq_ignore_ascii_case(n)
            } else {
                *w == n
            }
        })
    };
    let mut level = vec![dir.to_owned()];
    let mut seen = 0usize;
    for _ in 0..=FIND_DEPTH {
        let mut next = Vec::new();
        for d in level {
            let Ok(read) = std::fs::read_dir(&d) else {
                continue;
            };
            let mut entries: Vec<_> = read.filter_map(Result::ok).collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                seen += 1;
                if seen > FIND_ENTRIES {
                    return None;
                }
                let Ok(kind) = e.file_type() else { continue };
                if kind.is_dir() {
                    next.push(e.path());
                } else if kind.is_file() && wanted(&e.file_name().to_string_lossy()) {
                    return Some(e.path());
                }
            }
        }
        if next.is_empty() {
            break;
        }
        level = next;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_helper_is_found_in_the_components_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Paths::under(tmp.path()).components_dir();
        assert_eq!(find_in_components(&root, &["ffmpeg"]), None);
        let bin = root.join("ffmpeg").join("ffmpeg-9.0.2-win64").join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("ffprobe"), b"x").unwrap();
        std::fs::write(bin.join("ffmpeg"), b"x").unwrap();
        assert_eq!(
            find_in_components(&root, &["ffmpeg.exe", "ffmpeg"]),
            Some(bin.join("ffmpeg"))
        );
        // A shallower copy wins.
        std::fs::write(root.join("ffmpeg").join("ffmpeg"), b"x").unwrap();
        assert_eq!(
            find_in_components(&root, &["ffmpeg"]),
            Some(root.join("ffmpeg").join("ffmpeg"))
        );
        // Too deep is not looked at.
        let deep = (0..=FIND_DEPTH + 1).fold(root.join("deep"), |p, i| p.join(i.to_string()));
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("libtts.so"), b"x").unwrap();
        assert_eq!(find_in_components(&root, &["libtts.so"]), None);
    }

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
