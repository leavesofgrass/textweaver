//! A component: what it is, what needs it, and its pinned files.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::error::ComponentError;
use crate::manifest::Listing;
use crate::pin::{FilePin, hash_file, is_plain_name};

/// An optional component: a model, font, or voice textweaver can use but
/// does not ship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    /// Short id, a plain name (`whisper-base.en`, `ocr-ocrs`, `lexend`).
    /// A mirror keeps the component's files under it.
    pub id: Cow<'static, str>,
    /// What it is, for a person ("Whisper base.en, English dictation").
    pub title: Cow<'static, str>,
    /// The license, as said ("MIT", "CC BY-SA 4.0", "MIT, unconfirmed").
    pub license: Cow<'static, str>,
    /// Who made it and where it comes from (for the notices).
    pub credit: Cow<'static, str>,
    /// The features that need it, as keys the app names in words
    /// (`dictation`, `ocr`, `reading-font`, `voice`).
    pub features: Cow<'static, [Cow<'static, str>]>,
    /// Where it is installed, under the data folder: plain names joined
    /// with `/` (`whisper/rten/base.en`).
    pub folder: Cow<'static, str>,
    /// Its files.
    pub files: Cow<'static, [FilePin]>,
    /// A license file written beside the files when it is installed: its
    /// name and text (Lexend's `OFL.txt`).
    pub notice: Option<(Cow<'static, str>, Cow<'static, str>)>,
    /// What a source's `components.toml` said beyond the pins (version,
    /// platform, action); `None` for a built-in component.
    pub listing: Option<Listing>,
}

/// Whether a component is installed, from the files' sizes (quick; the
/// hashes are checked by [`Component::verify_in`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Every file is there with its pinned size.
    Installed,
    /// None of its files are there.
    NotInstalled,
    /// Some files are missing (their names).
    Partial(Vec<String>),
    /// A file has the wrong size (its name): cut short or altered.
    Damaged(String),
}

/// One file, checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileState {
    /// Size and hash match.
    Good,
    /// Not there.
    Missing,
    /// There, with the wrong size (bytes found).
    WrongSize(u64),
    /// The right size, the wrong hash.
    WrongHash,
}

impl Component {
    /// Bytes to download.
    pub fn size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    /// The size for a person: "79.3 MB".
    pub fn size_text(&self) -> String {
        crate::size_text(self.size())
    }

    /// The folder holding its files in a source or a mirror: on GitHub, the
    /// release's tag. `<id>-<version>` when its list gives a version
    /// (`ffmpeg-9.0.2`), else the id (`whisper-base.en`).
    pub fn release(&self) -> String {
        match &self.listing {
            Some(l) if !l.version.is_empty() => format!("{}-{}", self.id, l.version),
            _ => self.id.to_string(),
        }
    }

    /// The folder it is installed in under `data_dir`.
    pub fn dir_in(&self, data_dir: &Path) -> PathBuf {
        self.folder
            .split('/')
            .filter(|s| !s.is_empty())
            .fold(data_dir.to_owned(), |p, s| p.join(s))
    }

    /// The pinned file named `name`.
    pub fn file(&self, name: &str) -> Option<&FilePin> {
        self.files.iter().find(|f| f.name == name)
    }

    /// Checks that every name in it is plain: the id, each folder part,
    /// each file name, and the notice's name; and that each hash is well
    /// formed and each size more than zero. A mirror's manifest is
    /// checked with this before any of it is used.
    pub fn check_names(&self) -> Result<(), ComponentError> {
        let bad = |s: &str| Err(ComponentError::BadName(s.to_owned()));
        if !is_plain_name(&self.id) {
            return bad(&self.id);
        }
        if self.folder.is_empty() || !self.folder.split('/').all(is_plain_name) {
            return bad(&self.folder);
        }
        if self.files.is_empty() {
            return Err(ComponentError::Manifest(format!(
                "{} has no files",
                self.id
            )));
        }
        for f in self.files.iter() {
            if !is_plain_name(&f.name) || f.name.ends_with(".part") {
                return bad(&f.name);
            }
            if f.size == 0 || !f.check.is_well_formed() {
                return Err(ComponentError::Manifest(format!(
                    "{} in {} has no size or a malformed hash",
                    f.name, self.id
                )));
            }
        }
        if let Some((name, _)) = &self.notice
            && !is_plain_name(name)
        {
            return bad(name);
        }
        Ok(())
    }

    /// Whether it is installed in `dir`, from the files' sizes.
    pub fn status_in(&self, dir: &Path) -> Status {
        let mut missing = Vec::new();
        for f in self.files.iter() {
            match std::fs::metadata(dir.join(f.name.as_ref())) {
                Ok(m) if m.is_file() && m.len() == f.size => {}
                Ok(_) => return Status::Damaged(f.name.to_string()),
                Err(_) => missing.push(f.name.to_string()),
            }
        }
        if missing.is_empty() {
            Status::Installed
        } else if missing.len() == self.files.len() {
            Status::NotInstalled
        } else {
            Status::Partial(missing)
        }
    }

    /// Checks each file in `dir` by size and hash.
    pub fn verify_in(&self, dir: &Path) -> Vec<(String, FileState)> {
        self.files
            .iter()
            .map(|f| {
                let path = dir.join(f.name.as_ref());
                let state = match std::fs::metadata(&path) {
                    Err(_) => FileState::Missing,
                    Ok(m) if !m.is_file() => FileState::Missing,
                    Ok(m) if m.len() != f.size => FileState::WrongSize(m.len()),
                    Ok(_) => match hash_file(&f.check, &path) {
                        Ok(h) if h == f.check.expected() => FileState::Good,
                        _ => FileState::WrongHash,
                    },
                };
                (f.name.to_string(), state)
            })
            .collect()
    }

    /// Removes the component's own files from `dir` (its pinned files,
    /// their `.part` files, and its notice), then `dir` itself when that
    /// leaves it empty. Nothing else is touched: a file the component does
    /// not name stays, and so does the folder holding it. Returns how many
    /// files were removed.
    pub fn remove_in(&self, dir: &Path) -> Result<usize, ComponentError> {
        let mut names: Vec<String> = Vec::new();
        for f in self.files.iter() {
            names.push(f.name.to_string());
            names.push(format!("{}.part", f.name));
        }
        if let Some((n, _)) = &self.notice {
            names.push(n.to_string());
        }
        let mut removed = 0;
        for name in names {
            let path = dir.join(&name);
            if path.is_file() {
                std::fs::remove_file(&path).map_err(|e| ComponentError::io(&path, e))?;
                removed += 1;
            }
        }
        if std::fs::read_dir(dir).is_ok_and(|mut d| d.next().is_none()) {
            std::fs::remove_dir(dir).map_err(|e| ComponentError::io(dir, e))?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;
    use crate::pin::Check;

    pub(crate) fn sample() -> Component {
        Component {
            id: "sample".into(),
            title: "a sample".into(),
            license: "CC0".into(),
            credit: "".into(),
            features: Cow::Borrowed(&[Cow::Borrowed("test")]),
            folder: "things/sample".into(),
            files: vec![FilePin {
                name: "a.bin".into(),
                url: "https://example.invalid/a.bin".into(),
                size: 3,
                check: Check::Sha256(
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
                ),
            }]
            .into(),
            notice: None,
            listing: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pin::Check;
    use tests_support::sample;

    #[test]
    fn status_verify_and_remove() {
        let tmp = tempfile::tempdir().unwrap();
        let c = sample();
        let dir = c.dir_in(tmp.path());
        assert_eq!(dir, tmp.path().join("things").join("sample"));
        assert_eq!(c.status_in(&dir), Status::NotInstalled);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.bin"), b"ab").unwrap();
        assert_eq!(c.status_in(&dir), Status::Damaged("a.bin".into()));
        assert_eq!(c.verify_in(&dir)[0].1, FileState::WrongSize(2));
        std::fs::write(dir.join("a.bin"), b"abd").unwrap();
        assert_eq!(c.status_in(&dir), Status::Installed);
        assert_eq!(c.verify_in(&dir)[0].1, FileState::WrongHash);
        std::fs::write(dir.join("a.bin"), b"abc").unwrap();
        assert_eq!(c.verify_in(&dir)[0].1, FileState::Good);
        // A file the component does not name keeps the folder.
        std::fs::write(dir.join("mine.txt"), b"keep").unwrap();
        assert_eq!(c.remove_in(&dir).unwrap(), 1);
        assert!(dir.join("mine.txt").is_file());
        std::fs::remove_file(dir.join("mine.txt")).unwrap();
        std::fs::write(dir.join("a.bin"), b"abc").unwrap();
        assert_eq!(c.remove_in(&dir).unwrap(), 1);
        assert!(!dir.exists());
    }

    #[test]
    fn names_are_checked() {
        let mut c = sample();
        assert!(c.check_names().is_ok());
        c.folder = "things/../up".into();
        assert!(c.check_names().is_err());
        c = sample();
        c.id = "a b".into();
        assert!(c.check_names().is_err());
        c = sample();
        c.files.to_mut()[0].name = "x.part".into();
        assert!(c.check_names().is_err());
        c = sample();
        c.files.to_mut()[0].check = Check::Sha256("00".into());
        assert!(c.check_names().is_err());
    }
}
