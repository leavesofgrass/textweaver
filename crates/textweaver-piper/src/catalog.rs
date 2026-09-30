//! The Piper voice catalogue: `voices.json` from the
//! [piper-voices](https://huggingface.co/rhasspy/piper-voices) repository
//! (177 voices in 53 languages when this was written), and each voice's
//! licence from its `MODEL_CARD`.
//!
//! **Licences differ by voice.** Each voice inherits its training data's
//! licence: `joe` is CC0, `libritts_r` CC BY 4.0, `kristin`, `norman`, and
//! `cori` public domain, but `lessac` (Blizzard 2013 terms), `ryan`, and
//! `hfc_female` are non-commercial. The app shows the licence from the
//! `MODEL_CARD` before a download, and textweaver never bundles a voice
//! that [`Licence::may_bundle`] refuses.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::PiperError;

/// Where the voices live.
pub const REPOSITORY: &str = "rhasspy/piper-voices";

/// The catalogue file's URL.
pub const CATALOG_URL: &str =
    "https://huggingface.co/rhasspy/piper-voices/resolve/main/voices.json";

/// The URL of `path` in the voice repository.
pub fn file_url(path: &str) -> String {
    format!("https://huggingface.co/{REPOSITORY}/resolve/main/{path}")
}

/// One file of a voice, as `voices.json` lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct CatalogFile {
    /// Size in bytes.
    pub size_bytes: u64,
    /// MD5 of the file (the catalogue's own check; downloads are checked
    /// against Hugging Face's SHA-256 instead).
    #[serde(default)]
    pub md5_digest: String,
}

/// A voice's language.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct CatalogLanguage {
    /// `en_US`.
    pub code: String,
    /// `en`.
    #[serde(default)]
    pub family: String,
    /// `English`.
    #[serde(default)]
    pub name_english: String,
    /// `United States`.
    #[serde(default)]
    pub country_english: String,
}

/// One voice in the catalogue.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CatalogVoice {
    /// `en_US-joe-medium`.
    pub key: String,
    /// `joe`.
    pub name: String,
    /// The language.
    pub language: CatalogLanguage,
    /// `x_low`, `low`, `medium`, or `high`.
    pub quality: String,
    /// Speakers in the model.
    #[serde(default = "one")]
    pub num_speakers: u32,
    /// Repository paths of the voice's files.
    pub files: BTreeMap<String, CatalogFile>,
}

fn one() -> u32 {
    1
}

impl CatalogVoice {
    /// Total download size in bytes.
    pub fn size_bytes(&self) -> u64 {
        self.files.values().map(|f| f.size_bytes).sum()
    }

    /// The repository path of the file whose name ends with `suffix`.
    pub fn file_ending(&self, suffix: &str) -> Option<&str> {
        self.files
            .keys()
            .find(|p| p.ends_with(suffix))
            .map(String::as_str)
    }

    /// The model file's repository path.
    pub fn onnx_path(&self) -> Option<&str> {
        self.file_ending(".onnx")
    }

    /// The `MODEL_CARD`'s repository path.
    pub fn model_card_path(&self) -> Option<&str> {
        self.file_ending("MODEL_CARD")
    }

    /// A BCP 47 tag: `en-US`.
    pub fn bcp47(&self) -> String {
        self.language.code.replace('_', "-")
    }

    /// A name to read aloud: "Joe, English (United States), medium
    /// quality, 63 MB".
    pub fn describe(&self) -> String {
        let mut s = format!(
            "{}, {}",
            display_name(&self.name),
            language_name(&self.language)
        );
        s.push_str(&format!(", {} quality", self.quality.replace('_', " ")));
        if self.num_speakers > 1 {
            s.push_str(&format!(", {} speakers", self.num_speakers));
        }
        s.push_str(&format!(", {}", megabytes(self.size_bytes())));
        s
    }
}

/// "Joe", "Hfc female": a voice's name as a person would say it.
pub fn display_name(name: &str) -> String {
    let spaced = name.replace('_', " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// "English (United States)".
pub fn language_name(l: &CatalogLanguage) -> String {
    match (l.name_english.as_str(), l.country_english.as_str()) {
        ("", _) => l.code.clone(),
        (n, "") => n.to_owned(),
        (n, c) => format!("{n} ({c})"),
    }
}

/// "63 MB", "4 KB".
pub fn megabytes(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{} MB", (bytes + 500_000) / 1_000_000)
    } else {
        format!("{} KB", (bytes + 500) / 1000)
    }
}

/// The whole catalogue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    /// Voices, sorted by key.
    pub voices: Vec<CatalogVoice>,
}

impl Catalog {
    /// Parses `voices.json`.
    pub fn from_json(json: &str) -> Result<Self, PiperError> {
        let map: BTreeMap<String, CatalogVoice> = serde_json::from_str(json)
            .map_err(|e| PiperError::Config(format!("the voice list is not valid: {e}")))?;
        Ok(Catalog {
            voices: map.into_values().collect(),
        })
    }

    /// The voice with key `key`.
    pub fn get(&self, key: &str) -> Option<&CatalogVoice> {
        self.voices.iter().find(|v| v.key == key)
    }

    /// Voices whose language code or family starts with `language`
    /// (`en`, `en_GB`, `en-GB`), ignoring case.
    pub fn for_language<'a>(&'a self, language: &'a str) -> impl Iterator<Item = &'a CatalogVoice> {
        let want = language.replace('-', "_").to_ascii_lowercase();
        self.voices
            .iter()
            .filter(move |v| v.language.code.to_ascii_lowercase().starts_with(&want))
    }

    /// The distinct language codes, sorted.
    pub fn languages(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .voices
            .iter()
            .map(|v| v.language.code.as_str())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// How a voice may be used, from its licence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenceKind {
    /// Public domain or CC0.
    PublicDomain,
    /// Free to use with credit (CC BY, MIT, Apache).
    Attribution,
    /// Free to use with credit, and derivatives shared alike (CC BY-SA).
    ShareAlike,
    /// Not for commercial use (CC BY-NC, the Blizzard 2013 terms).
    NonCommercial,
    /// Not recognized: read it before use.
    Unknown,
}

/// A voice's licence, from its `MODEL_CARD`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Licence {
    /// The licence as the card states it (`CC BY 4.0`, or a URL).
    pub text: String,
    /// What it allows.
    pub kind: LicenceKind,
}

impl Licence {
    /// Reads the `License:` line of a `MODEL_CARD`. A card without one
    /// gives an unknown licence.
    pub fn from_model_card(card: &str) -> Licence {
        let text = card
            .lines()
            .map(|l| l.trim().trim_start_matches(['*', '-']).trim())
            .find_map(|l| {
                let (key, value) = l.split_once(':')?;
                let key = key.trim().to_ascii_lowercase();
                (key == "license" || key == "licence").then(|| value.trim().to_owned())
            })
            .unwrap_or_default();
        Licence::classify(&text)
    }

    /// Classifies a licence statement.
    pub fn classify(text: &str) -> Licence {
        let t = text.to_ascii_lowercase();
        let compact: String = t.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        let has_word = |w: &str| {
            t.split(|c: char| !c.is_ascii_alphanumeric())
                .any(|x| x == w)
        };
        let kind = if t.is_empty() {
            LicenceKind::Unknown
        } else if compact.contains("bync")
            || t.contains("non-commercial")
            || t.contains("noncommercial")
            || t.contains("blizzard")
        {
            LicenceKind::NonCommercial
        } else if t.contains("public domain") || compact.contains("cc0") || t.contains("zero/1.0") {
            LicenceKind::PublicDomain
        } else if compact.contains("bysa") {
            LicenceKind::ShareAlike
        } else if compact.contains("ccby")
            || t.contains("licenses/by/")
            || has_word("mit")
            || has_word("apache")
        {
            LicenceKind::Attribution
        } else {
            LicenceKind::Unknown
        };
        Licence {
            text: text.to_owned(),
            kind,
        }
    }

    /// True when textweaver may ship the voice with itself: never a
    /// non-commercial voice, nor one whose licence is not recognized.
    pub fn may_bundle(&self) -> bool {
        matches!(
            self.kind,
            LicenceKind::PublicDomain | LicenceKind::Attribution | LicenceKind::ShareAlike
        )
    }

    /// A sentence to read before downloading: "License: CC BY 4.0. Free to
    /// use; credit the voice's authors."
    pub fn describe(&self) -> String {
        let what = match self.kind {
            LicenceKind::PublicDomain => "Free to use for anything.",
            LicenceKind::Attribution => "Free to use; credit the voice's authors.",
            LicenceKind::ShareAlike => {
                "Free to use; credit the voice's authors and share changes alike."
            }
            LicenceKind::NonCommercial => "Personal and non-commercial use only.",
            LicenceKind::Unknown => "Read the licence before using this voice.",
        };
        if self.text.is_empty() {
            format!("License not stated. {what}")
        } else {
            format!("License: {}. {what}", self.text)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const VOICES: &str = r#"{
      "en_US-joe-medium": {
        "key": "en_US-joe-medium", "name": "joe",
        "language": {"code": "en_US", "family": "en", "region": "US",
          "name_native": "English", "name_english": "English",
          "country_english": "United States"},
        "quality": "medium", "num_speakers": 1, "speaker_id_map": {},
        "files": {
          "en/en_US/joe/medium/en_US-joe-medium.onnx": {"size_bytes": 63201294, "md5_digest": "74fd6a4dc39e0aa9dce145d7f5acd4f6"},
          "en/en_US/joe/medium/en_US-joe-medium.onnx.json": {"size_bytes": 4794, "md5_digest": "811036b9c1451545f9495fdc1baa0754"},
          "en/en_US/joe/medium/MODEL_CARD": {"size_bytes": 281, "md5_digest": "7dc55becaf84cb3b713281fac70491a1"}
        },
        "aliases": []
      },
      "de_DE-thorsten-medium": {
        "key": "de_DE-thorsten-medium", "name": "thorsten",
        "language": {"code": "de_DE", "family": "de", "name_english": "German",
          "country_english": "Germany"},
        "quality": "medium",
        "files": {"de/de_DE/thorsten/medium/de_DE-thorsten-medium.onnx": {"size_bytes": 63511038}}
      },
      "en_GB-vctk-medium": {
        "key": "en_GB-vctk-medium", "name": "vctk",
        "language": {"code": "en_GB", "family": "en", "name_english": "English",
          "country_english": "Great Britain"},
        "quality": "x_low", "num_speakers": 109,
        "files": {"a.onnx": {"size_bytes": 900}}
      }
    }"#;

    #[test]
    fn parses_the_catalogue() {
        let c = Catalog::from_json(VOICES).unwrap();
        assert_eq!(c.voices.len(), 3);
        let joe = c.get("en_US-joe-medium").unwrap();
        assert_eq!(joe.size_bytes(), 63_206_369);
        assert_eq!(
            joe.onnx_path(),
            Some("en/en_US/joe/medium/en_US-joe-medium.onnx")
        );
        assert_eq!(
            joe.model_card_path(),
            Some("en/en_US/joe/medium/MODEL_CARD")
        );
        assert_eq!(joe.bcp47(), "en-US");
        assert_eq!(
            joe.describe(),
            "Joe, English (United States), medium quality, 63 MB"
        );
        let vctk = c.get("en_GB-vctk-medium").unwrap();
        assert_eq!(
            vctk.describe(),
            "Vctk, English (Great Britain), x low quality, 109 speakers, 1 KB"
        );
        assert_eq!(c.languages(), vec!["de_DE", "en_GB", "en_US"]);
        assert_eq!(c.for_language("en").count(), 2);
        assert_eq!(c.for_language("en-gb").count(), 1);
        assert!(Catalog::from_json("[]").is_err());
        assert_eq!(
            file_url("en/x/MODEL_CARD"),
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/x/MODEL_CARD"
        );
    }

    #[test]
    fn licences_from_model_cards() {
        let card = "# Model card for joe (medium)\n\n* Language: en_US\n\n## Dataset\n\n* URL: https://example.org\n* License: CC0\n";
        let l = Licence::from_model_card(card);
        assert_eq!(l.text, "CC0");
        assert_eq!(l.kind, LicenceKind::PublicDomain);
        assert!(l.may_bundle());
        assert_eq!(l.describe(), "License: CC0. Free to use for anything.");

        let cases = [
            ("CC BY 4.0", LicenceKind::Attribution),
            ("public domain", LicenceKind::PublicDomain),
            ("CC BY-NC-SA 4.0", LicenceKind::NonCommercial),
            (
                "https://creativecommons.org/licenses/by-nc-sa/4.0/deed.en",
                LicenceKind::NonCommercial,
            ),
            (
                "https://www.cstr.ed.ac.uk/projects/blizzard/2013/lessac_blizzard2013/license.html",
                LicenceKind::NonCommercial,
            ),
            ("CC BY-SA 4.0", LicenceKind::ShareAlike),
            ("MIT", LicenceKind::Attribution),
            ("see the dataset", LicenceKind::Unknown),
        ];
        for (text, kind) in cases {
            let l = Licence::classify(text);
            assert_eq!(l.kind, kind, "{text}");
            assert_eq!(
                l.may_bundle(),
                !matches!(kind, LicenceKind::NonCommercial | LicenceKind::Unknown),
                "{text}"
            );
        }
        let none = Licence::from_model_card("# no licence here");
        assert_eq!(none.kind, LicenceKind::Unknown);
        assert!(!none.may_bundle());
        assert!(none.describe().starts_with("License not stated."));
    }

    #[test]
    fn names_read_well() {
        assert_eq!(display_name("hfc_female"), "Hfc female");
        assert_eq!(display_name(""), "");
        assert_eq!(megabytes(4794), "5 KB");
        assert_eq!(megabytes(137_000_000), "137 MB");
    }
}
