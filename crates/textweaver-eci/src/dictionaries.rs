//! Community pronunciation dictionaries for the engine (ADR-0007).
//!
//! textweaver loads the IBMTTS community dictionaries
//! (`third_party/ibmtts-dictionaries/`, CC0) into ECI: for each language,
//! one ECI dictionary with up to three volumes, from files named
//! `<LANG>main.dic`, `<LANG>root.dic`, and `<LANG>abbr.dic` (matched
//! case-insensitively; `<LANG>` is the engine's three-letter code, so
//! `ENUmain.dic` serves American English and `DEUmain.dic` German). The
//! files are Windows-1252 with CRLF line ends; the engine reads them itself,
//! unchanged.
//!
//! The directory is chosen by [`find_dir`]: the backend option
//! ([`Dictionaries`]), else `TEXTWEAVER_ECI_DICTIONARIES` (`off` turns them
//! off), else an `ibmtts-dictionaries` folder beside the host executable
//! (where `cargo xtask eci-host` copies them), else the repository's
//! `third_party/ibmtts-dictionaries` when running from a checkout.

use std::path::{Path, PathBuf};

use crate::language;

/// Environment variable naming the dictionary directory (or `off`).
pub const DICTIONARIES_ENV: &str = "TEXTWEAVER_ECI_DICTIONARIES";

/// The folder name beside the host executable.
pub const DIR_NAME: &str = "ibmtts-dictionaries";

/// ECI dictionary volumes (`ECIDictVolume`). `eciMainDictExt` (3) is not
/// used: the community dictionaries ship main, root, and abbreviation files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Volume {
    /// Whole words to respellings or phonemes (`eciMainDict`).
    Main = 0,
    /// Word roots, applied inside inflected forms (`eciRootDict`).
    Root = 1,
    /// Abbreviations (`eciAbbvDict`).
    Abbreviation = 2,
}

impl Volume {
    /// All volumes, in load order.
    pub const ALL: [Volume; 3] = [Volume::Main, Volume::Root, Volume::Abbreviation];

    /// The file-name suffix after the language code.
    pub fn suffix(self) -> &'static str {
        match self {
            Volume::Main => "main.dic",
            Volume::Root => "root.dic",
            Volume::Abbreviation => "abbr.dic",
        }
    }

    /// The volume with ECI number `n`.
    pub fn from_number(n: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|v| *v as u8 == n)
    }
}

/// `ECIDictError` values, for messages.
pub fn status_text(status: i32) -> &'static str {
    match status {
        0 => "loaded",
        1 => "file not found",
        2 => "out of memory",
        3 => "internal error",
        4 => "no entry",
        5 => "lookup key error",
        6 => "access error",
        7 => "invalid volume",
        -1 => "not supported by this ECI library",
        _ => "unknown error",
    }
}

/// Which dictionaries to load.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Dictionaries {
    /// Search the usual places ([`find_dir`]).
    #[default]
    Auto,
    /// Load none.
    Off,
    /// Load from this directory.
    Dir(PathBuf),
}

/// The dictionary files for `dialect` in `dir`, by volume.
pub fn files_for(dir: &Path, dialect: u32) -> Vec<(Volume, PathBuf)> {
    let Some(d) = language::dialect_by_code(dialect) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let names: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
        .collect();
    Volume::ALL
        .into_iter()
        .filter_map(|v| {
            let want = format!("{}{}", d.short, v.suffix());
            names
                .iter()
                .find(|(n, _)| *n == want)
                .map(|(_, p)| (v, p.clone()))
        })
        .collect()
}

fn looks_like_dir(p: &Path) -> bool {
    p.is_dir()
        && std::fs::read_dir(p).is_ok_and(|mut it| {
            it.any(|e| {
                e.is_ok_and(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .ends_with(".dic")
                })
            })
        })
}

/// The dictionary directory to use, if any (see the module docs), as an
/// absolute path without `..` (ECI libraries limit and resolve paths their
/// own way).
pub fn find_dir(setting: &Dictionaries, host: Option<&Path>) -> Option<PathBuf> {
    find_dir_raw(setting, host).map(|p| clean(&p))
}

/// `p` made absolute and free of `.` and `..`; unchanged if it cannot be
/// resolved. On Windows the verbatim `\\?\` prefix is dropped again, since
/// ECI takes a plain ANSI path.
pub fn clean(p: &Path) -> PathBuf {
    let Ok(c) = std::fs::canonicalize(p) else {
        return p.to_path_buf();
    };
    let s = c.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => c,
    }
}

fn find_dir_raw(setting: &Dictionaries, host: Option<&Path>) -> Option<PathBuf> {
    match setting {
        Dictionaries::Off => return None,
        Dictionaries::Dir(d) => return Some(d.clone()),
        Dictionaries::Auto => {}
    }
    if let Some(v) = std::env::var_os(DICTIONARIES_ENV).filter(|v| !v.is_empty()) {
        let s = v.to_string_lossy().to_ascii_lowercase();
        if matches!(s.as_str(), "off" | "none" | "0" | "false") {
            return None;
        }
        return Some(PathBuf::from(v));
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = host.and_then(Path::parent) {
        candidates.push(dir.join(DIR_NAME));
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from));
    for start in host
        .and_then(Path::parent)
        .map(PathBuf::from)
        .into_iter()
        .chain(exe_dir)
    {
        for d in start.ancestors() {
            candidates.push(d.join("third_party").join(DIR_NAME));
        }
    }
    // The checkout this crate was built from (development builds).
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("third_party")
            .join(DIR_NAME),
    );
    candidates.into_iter().find(|p| looks_like_dir(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_files_case_insensitively_per_language() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "ENUmain.dic",
            "ENURoot.dic",
            "enuabbr.dic",
            "DEUmain.dic",
            "x.txt",
        ] {
            std::fs::write(dir.path().join(name), b"a\tb\r\n").unwrap();
        }
        let enu = files_for(dir.path(), 0x0001_0000);
        let vols: Vec<Volume> = enu.iter().map(|(v, _)| *v).collect();
        assert_eq!(vols, Volume::ALL);
        assert!(enu[1].1.ends_with("ENURoot.dic"));
        let deu = files_for(dir.path(), 0x0004_0000);
        assert_eq!(deu.len(), 1);
        assert_eq!(deu[0].0, Volume::Main);
        assert!(
            files_for(dir.path(), 0x0001_0001).is_empty(),
            "no ENG files"
        );
        assert!(files_for(Path::new("does/not/exist"), 0x0001_0000).is_empty());
    }

    #[test]
    fn explicit_settings_win_and_the_checkout_copy_is_found() {
        assert_eq!(find_dir(&Dictionaries::Off, None), None);
        assert_eq!(
            find_dir(&Dictionaries::Dir("/x".into()), None),
            Some(PathBuf::from("/x"))
        );
        if std::env::var_os(DICTIONARIES_ENV).is_none() {
            let found = find_dir(&Dictionaries::Auto, None).expect("repository copy");
            assert!(!files_for(&found, 0x0001_0000).is_empty());
        }
    }

    #[test]
    fn volume_numbers_match_eci() {
        assert_eq!(Volume::Main as u8, 0);
        assert_eq!(Volume::Root as u8, 1);
        assert_eq!(Volume::Abbreviation as u8, 2);
        assert_eq!(Volume::from_number(2), Some(Volume::Abbreviation));
        assert_eq!(Volume::from_number(3), None);
        assert_eq!(status_text(1), "file not found");
    }
}
