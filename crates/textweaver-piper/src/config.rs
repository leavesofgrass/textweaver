//! A Piper voice's configuration: the `.onnx.json` file beside the model.

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

use crate::PiperError;

/// The parts of a voice's `.onnx.json` that synthesis needs.
///
/// See <https://github.com/OHF-Voice/piper1-gpl/blob/main/docs/VOICES.md>.
#[derive(Clone, Debug, Deserialize)]
pub struct VoiceConfig {
    /// Output audio.
    pub audio: AudioConfig,
    /// The eSpeak NG voice that phonemizes text for this model.
    #[serde(default)]
    pub espeak: EspeakConfig,
    /// The model's default noise and length scales.
    #[serde(default)]
    pub inference: InferenceConfig,
    /// `espeak` (IPA from eSpeak NG) or `text` (the text's own
    /// characters). Other phonemizers (Japanese, Chinese, ...) need the
    /// `piper` program.
    #[serde(default = "default_phoneme_type")]
    pub phoneme_type: String,
    /// Each phoneme (one Unicode character) to its model input ids.
    pub phoneme_id_map: HashMap<String, Vec<i64>>,
    /// Speakers in a multi-speaker model; 1 otherwise.
    #[serde(default = "one")]
    pub num_speakers: u32,
    /// Speaker names to ids, for multi-speaker models.
    #[serde(default)]
    pub speaker_id_map: BTreeMap<String, i64>,
    /// The voice's language, when the file says.
    #[serde(default)]
    pub language: Option<LanguageConfig>,
    /// The dataset the voice was trained on (usually its name).
    #[serde(default)]
    pub dataset: Option<String>,
}

fn default_phoneme_type() -> String {
    "espeak".to_owned()
}

fn one() -> u32 {
    1
}

/// Output audio settings.
#[derive(Clone, Debug, Deserialize)]
pub struct AudioConfig {
    /// Samples per second (16000 or 22050).
    pub sample_rate: u32,
    /// `x_low`, `low`, `medium`, or `high`.
    #[serde(default)]
    pub quality: Option<String>,
}

/// The eSpeak NG voice for phonemes.
#[derive(Clone, Debug, Deserialize)]
pub struct EspeakConfig {
    /// eSpeak NG voice name (`en-us`).
    pub voice: String,
}

impl Default for EspeakConfig {
    fn default() -> Self {
        EspeakConfig {
            voice: "en-us".to_owned(),
        }
    }
}

/// Inference scales.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct InferenceConfig {
    /// Variation in the voice.
    #[serde(default = "noise_scale")]
    pub noise_scale: f32,
    /// Phoneme length: above 1 is slower, below 1 faster.
    #[serde(default = "length_scale")]
    pub length_scale: f32,
    /// Variation in phoneme length.
    #[serde(default = "noise_w")]
    pub noise_w: f32,
}

fn noise_scale() -> f32 {
    0.667
}
fn length_scale() -> f32 {
    1.0
}
fn noise_w() -> f32 {
    0.8
}

impl Default for InferenceConfig {
    fn default() -> Self {
        InferenceConfig {
            noise_scale: noise_scale(),
            length_scale: length_scale(),
            noise_w: noise_w(),
        }
    }
}

/// The voice's language.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct LanguageConfig {
    /// `en_US`.
    #[serde(default)]
    pub code: String,
    /// `English`.
    #[serde(default)]
    pub name_english: String,
    /// `United States`.
    #[serde(default)]
    pub country_english: String,
}

/// Phoneme ids that start and end every input, and separate phonemes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecialIds {
    /// `^`: beginning of the input.
    pub bos: Vec<i64>,
    /// `$`: end of the input.
    pub eos: Vec<i64>,
    /// `_`: padding after every phoneme.
    pub pad: Vec<i64>,
}

impl VoiceConfig {
    /// Parses a voice's `.onnx.json`.
    pub fn from_json(json: &str) -> Result<Self, PiperError> {
        let config: VoiceConfig = serde_json::from_str(json)
            .map_err(|e| PiperError::Config(format!("the voice's settings are not valid: {e}")))?;
        if config.audio.sample_rate == 0 {
            return Err(PiperError::Config("the voice's sample rate is 0".into()));
        }
        config.special_ids()?;
        Ok(config)
    }

    /// Reads and parses `path`.
    pub fn from_file(path: &std::path::Path) -> Result<Self, PiperError> {
        let json = std::fs::read_to_string(path).map_err(|e| PiperError::io(path, e))?;
        Self::from_json(&json)
    }

    /// The ids of `phoneme`, if the model knows it.
    pub fn ids(&self, phoneme: char) -> Option<&[i64]> {
        let mut buf = [0u8; 4];
        self.phoneme_id_map
            .get(phoneme.encode_utf8(&mut buf) as &str)
            .map(Vec::as_slice)
    }

    /// The start, end, and padding ids.
    pub fn special_ids(&self) -> Result<SpecialIds, PiperError> {
        let get = |c: char| {
            self.ids(c)
                .map(<[i64]>::to_vec)
                .ok_or_else(|| PiperError::Config(format!("the voice's phoneme map has no {c:?}")))
        };
        Ok(SpecialIds {
            bos: get('^')?,
            eos: get('$')?,
            pad: self
                .ids('_')
                .map(<[i64]>::to_vec)
                .unwrap_or_else(|| vec![0]),
        })
    }

    /// True when text is phonemized with eSpeak NG (most voices).
    pub fn uses_espeak(&self) -> bool {
        self.phoneme_type == "espeak"
    }

    /// A BCP 47 tag for the voice's language (`en-US`), from `language` or
    /// else the eSpeak voice.
    pub fn bcp47(&self) -> String {
        match &self.language {
            Some(l) if !l.code.is_empty() => l.code.replace('_', "-"),
            _ => {
                let v = &self.espeak.voice;
                match v.split_once('-') {
                    Some((lang, region)) => format!("{lang}-{}", region.to_ascii_uppercase()),
                    None => v.clone(),
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small voice configuration in the real files' shape.
    pub(crate) const JSON: &str = r#"{
        "audio": {"sample_rate": 22050, "quality": "medium"},
        "espeak": {"voice": "en-us"},
        "inference": {"noise_scale": 0.667, "length_scale": 1, "noise_w": 0.8},
        "phoneme_type": "espeak",
        "phoneme_map": {},
        "phoneme_id_map": {
            "_": [0], "^": [1], "$": [2], " ": [3], "!": [4], ",": [8],
            ".": [10], "?": [13], "a": [14], "b": [15], "d": [17], "e": [18],
            "h": [20], "i": [21], "l": [24], "n": [26], "o": [27], "s": [31],
            "t": [32], "w": [35], "z": [38], "ð": [41], "ə": [59], "ɪ": [74],
            "ˈ": [120], "ː": [122]
        },
        "num_symbols": 256,
        "num_speakers": 1,
        "speaker_id_map": {},
        "piper_version": "1.0.0",
        "language": {"code": "en_US", "family": "en", "region": "US",
                     "name_native": "English", "name_english": "English",
                     "country_english": "United States"},
        "dataset": "joe"
    }"#;

    #[test]
    fn parses_a_voice_config() {
        let c = VoiceConfig::from_json(JSON).unwrap();
        assert_eq!(c.audio.sample_rate, 22050);
        assert_eq!(c.espeak.voice, "en-us");
        assert_eq!(c.ids('ð'), Some(&[41][..]));
        assert_eq!(c.ids('Q'), None);
        assert_eq!(
            c.special_ids().unwrap(),
            SpecialIds {
                bos: vec![1],
                eos: vec![2],
                pad: vec![0]
            }
        );
        assert!(c.uses_espeak());
        assert_eq!(c.bcp47(), "en-US");
        assert_eq!(c.dataset.as_deref(), Some("joe"));
    }

    #[test]
    fn missing_start_or_end_ids_are_refused() {
        let bad = JSON.replace(r#""^": [1], "#, "");
        assert!(matches!(
            VoiceConfig::from_json(&bad),
            Err(PiperError::Config(_))
        ));
        assert!(VoiceConfig::from_json("{").is_err());
    }

    #[test]
    fn bcp47_falls_back_to_the_espeak_voice() {
        let mut c = VoiceConfig::from_json(JSON).unwrap();
        c.language = None;
        assert_eq!(c.bcp47(), "en-US");
        c.espeak.voice = "de".into();
        assert_eq!(c.bcp47(), "de");
    }
}
