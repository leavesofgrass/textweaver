//! Voices installed in the data folder.
//!
//! Each voice has a folder named by its key, holding the model, its
//! settings, and its `MODEL_CARD`:
//!
//! ```text
//! <data>/piper/voices/en_US-joe-medium/en_US-joe-medium.onnx
//! <data>/piper/voices/en_US-joe-medium/en_US-joe-medium.onnx.json
//! <data>/piper/voices/en_US-joe-medium/MODEL_CARD
//! ```
//!
//! A model and its `.onnx.json` copied straight into the voices folder
//! also count, for voices from elsewhere.

use std::path::{Path, PathBuf};

use textweaver_speech::Voice;

use crate::PiperError;
use crate::catalog::{Licence, display_name};

/// One installed voice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledVoice {
    /// `en_US-joe-medium`.
    pub key: String,
    /// The model.
    pub onnx: PathBuf,
    /// Its settings.
    pub json: PathBuf,
    /// The licence from its `MODEL_CARD` (unknown when it has none).
    pub licence: Licence,
}

impl InstalledVoice {
    /// The key's parts: language code, name, quality (`en_US`, `joe`,
    /// `medium`). A key not in that shape is all name.
    pub fn parts(&self) -> (&str, &str, &str) {
        let mut it = self.key.splitn(3, '-');
        match (it.next(), it.next(), it.next()) {
            (Some(l), Some(n), Some(q)) if l.contains('_') => (l, n, q),
            _ => ("", self.key.as_str(), ""),
        }
    }

    /// The voice as the speech service lists it: id is the key, name is
    /// "Joe (medium)", tagged "Piper" and with its quality.
    pub fn to_voice(&self) -> Voice {
        let (lang, name, quality) = self.parts();
        let mut tags = vec!["Piper".to_owned()];
        if !quality.is_empty() {
            tags.push(quality.replace('_', " "));
        }
        Voice {
            id: self.key.clone(),
            name: if quality.is_empty() {
                display_name(name)
            } else {
                format!("{} ({})", display_name(name), quality.replace('_', " "))
            },
            languages: if lang.is_empty() {
                Vec::new()
            } else {
                vec![lang.replace('_', "-")]
            },
            gender: None,
            tags,
        }
    }
}

/// The voices folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceStore {
    dir: PathBuf,
}

impl VoiceStore {
    /// The store in `dir` (`<data>/piper/voices`). The folder need not
    /// exist yet.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        VoiceStore { dir: dir.into() }
    }

    /// The folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The folder a voice is downloaded into.
    pub fn voice_dir(&self, key: &str) -> PathBuf {
        self.dir.join(key)
    }

    /// Every installed voice, sorted by key. A missing folder has none.
    pub fn installed(&self) -> Vec<InstalledVoice> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out: Vec<InstalledVoice> = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                if let Some(key) = path.file_name().and_then(|n| n.to_str())
                    && let Some(v) = voice_in(&path, key)
                {
                    out.push(v);
                }
            } else if let Some(key) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".onnx"))
                && let Some(v) = voice_in(&self.dir, key)
            {
                out.push(v);
            }
        }
        out.sort_by(|a, b| a.key.cmp(&b.key));
        out.dedup_by(|a, b| a.key == b.key);
        out
    }

    /// The installed voice `key`.
    pub fn get(&self, key: &str) -> Option<InstalledVoice> {
        voice_in(&self.voice_dir(key), key).or_else(|| voice_in(&self.dir, key))
    }

    /// Removes voice `key` (its folder, or its two files).
    pub fn remove(&self, key: &str) -> Result<(), PiperError> {
        if key.is_empty() || key.contains(['/', '\\']) || key.contains("..") {
            return Err(PiperError::NotInstalled(key.to_owned()));
        }
        let folder = self.voice_dir(key);
        if folder.is_dir() {
            return std::fs::remove_dir_all(&folder).map_err(|e| PiperError::io(&folder, e));
        }
        let Some(v) = voice_in(&self.dir, key) else {
            return Err(PiperError::NotInstalled(key.to_owned()));
        };
        for p in [&v.onnx, &v.json] {
            std::fs::remove_file(p).map_err(|e| PiperError::io(p, e))?;
        }
        Ok(())
    }
}

/// The voice `key` in `dir`, when both its files are there.
fn voice_in(dir: &Path, key: &str) -> Option<InstalledVoice> {
    let onnx = dir.join(format!("{key}.onnx"));
    let json = dir.join(format!("{key}.onnx.json"));
    if !onnx.is_file() || !json.is_file() {
        return None;
    }
    let card = dir.join("MODEL_CARD");
    let licence = std::fs::read_to_string(&card)
        .ok()
        .filter(|_| dir.file_name().and_then(|n| n.to_str()) == Some(key))
        .map_or_else(|| Licence::classify(""), |c| Licence::from_model_card(&c));
    Some(InstalledVoice {
        key: key.to_owned(),
        onnx,
        json,
        licence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::LicenceKind;

    fn install(dir: &Path, key: &str, card: Option<&str>) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{key}.onnx")), b"model").unwrap();
        std::fs::write(dir.join(format!("{key}.onnx.json")), b"{}").unwrap();
        if let Some(c) = card {
            std::fs::write(dir.join("MODEL_CARD"), c).unwrap();
        }
    }

    #[test]
    fn lists_gets_and_removes_voices() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VoiceStore::new(tmp.path().join("voices"));
        assert!(store.installed().is_empty());
        install(
            &store.voice_dir("en_US-joe-medium"),
            "en_US-joe-medium",
            Some("* License: CC0\n"),
        );
        install(store.dir(), "my-voice", None);
        // Half a voice is not a voice.
        std::fs::write(store.dir().join("broken.onnx"), b"x").unwrap();

        let list = store.installed();
        let keys: Vec<&str> = list.iter().map(|v| v.key.as_str()).collect();
        assert_eq!(keys, vec!["en_US-joe-medium", "my-voice"]);
        assert_eq!(list[0].licence.kind, LicenceKind::PublicDomain);
        assert_eq!(list[1].licence.kind, LicenceKind::Unknown);

        let joe = store.get("en_US-joe-medium").unwrap();
        assert_eq!(joe.parts(), ("en_US", "joe", "medium"));
        let v = joe.to_voice();
        assert_eq!(v.id, "en_US-joe-medium");
        assert_eq!(v.name, "Joe (medium)");
        assert_eq!(v.languages, vec!["en-US"]);
        assert_eq!(v.tags, vec!["Piper", "medium"]);
        let mine = store.get("my-voice").unwrap().to_voice();
        assert_eq!(mine.name, "My-voice");
        assert!(mine.languages.is_empty());

        store.remove("en_US-joe-medium").unwrap();
        store.remove("my-voice").unwrap();
        assert!(store.installed().is_empty());
        assert!(matches!(
            store.remove("my-voice"),
            Err(PiperError::NotInstalled(_))
        ));
        assert!(store.remove("../x").is_err());
    }
}
