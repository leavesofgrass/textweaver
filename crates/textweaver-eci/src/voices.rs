//! Voices: ECI's eight presets in each installed language.
//!
//! A voice id is `"<language>:<preset>"`, for example `en-US:reed` or
//! `de-DE:shelley`. Parsing is forgiving: the language is any BCP 47 tag or
//! bare language ECI knows (`en`, `en_gb`), the preset is a name or its
//! number 1..=8, and either part may be given alone (`shelley` keeps the
//! current language; `de-DE` uses preset 1, Reed).
//!
//! Preset names are the ones Eloquence users know (NVDA, JAWS, and Code
//! Factory use them); the engine's own names ("Adult Male 1" and so on)
//! appear in the display name when they differ.

use textweaver_speech::Voice;

use crate::language::{self, Dialect};
use crate::protocol::PresetInfo;

/// ECI voice parameters (`ECIVoiceParam`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VoiceParam {
    /// 0 male, 1 female.
    Gender = 0,
    /// Head size, 0..=100.
    HeadSize = 1,
    /// Pitch baseline, 0..=100.
    PitchBaseline = 2,
    /// Pitch fluctuation (intonation), 0..=100.
    PitchFluctuation = 3,
    /// Roughness, 0..=100.
    Roughness = 4,
    /// Breathiness, 0..=100.
    Breathiness = 5,
    /// Speed, 0..=250.
    Speed = 6,
    /// Volume, 0..=100.
    Volume = 7,
}

/// Slugs for presets 1..=8.
pub const PRESET_SLUGS: [&str; 8] = [
    "reed", "shelley", "bobby", "rocko", "glen", "sandy", "grandma", "grandpa",
];

/// Display names for presets 1..=8.
pub const PRESET_NAMES: [&str; 8] = [
    "Reed", "Shelley", "Bobby", "Rocko", "Glen", "Sandy", "Grandma", "Grandpa",
];

/// A parsed voice id. `None` parts keep the current setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceSel {
    /// Dialect code.
    pub dialect: Option<u32>,
    /// Preset 1..=8.
    pub preset: Option<u8>,
}

fn parse_preset(s: &str) -> Option<u8> {
    let s = s.trim();
    if let Ok(n) = s.parse::<u8>() {
        return (1..=8).contains(&n).then_some(n);
    }
    PRESET_SLUGS
        .iter()
        .position(|p| p.eq_ignore_ascii_case(s))
        .and_then(|i| u8::try_from(i + 1).ok())
}

/// Parses a voice id (see the module docs). `None` for unknown ids.
pub fn parse_voice_id(id: &str) -> Option<VoiceSel> {
    let id = id.trim();
    let id = id.strip_prefix("eloquence:").unwrap_or(id);
    if id.is_empty() {
        return None;
    }
    match id.split_once(':') {
        Some((lang, preset)) => Some(VoiceSel {
            dialect: Some(language::dialect_by_tag(lang)?.code),
            preset: Some(parse_preset(preset)?),
        }),
        None => {
            if let Some(p) = parse_preset(id) {
                Some(VoiceSel {
                    dialect: None,
                    preset: Some(p),
                })
            } else {
                Some(VoiceSel {
                    dialect: Some(language::dialect_by_tag(id)?.code),
                    preset: Some(1),
                })
            }
        }
    }
}

/// The canonical id for `dialect` and `preset`.
pub fn voice_id(dialect: &Dialect, preset: u8) -> String {
    let slug = PRESET_SLUGS
        .get(usize::from(preset.saturating_sub(1)))
        .copied()
        .unwrap_or("reed");
    format!("{}:{slug}", dialect.tag)
}

/// The voice list for the given installed dialects and presets.
pub fn voice_list(dialects: &[u32], presets: &[PresetInfo]) -> Vec<Voice> {
    let mut out = Vec::new();
    for code in dialects {
        let Some(d) = language::dialect_by_code(*code).filter(|d| d.supported) else {
            continue;
        };
        for (i, name) in PRESET_NAMES.iter().enumerate() {
            let preset = u8::try_from(i + 1).unwrap_or(1);
            let info = presets.get(i);
            let engine_name = info.map(|p| p.name.as_str()).unwrap_or("");
            let display = if engine_name.is_empty() || engine_name.eq_ignore_ascii_case(name) {
                format!("Eloquence {name}, {}", d.name)
            } else {
                format!("Eloquence {name} ({engine_name}), {}", d.name)
            };
            let gender = info.map(|p| if p.params[0] == 1 { "female" } else { "male" });
            out.push(Voice {
                id: voice_id(d, preset),
                name: display,
                languages: vec![d.tag.to_string()],
                gender: gender.map(str::to_string),
            });
        }
    }
    out
}

/// Dialects listed in an `eci.ini` whose `.syn` file exists. Sections are
/// named `[major.minor]` after the dialect code (`[1.0]` is American
/// English), each with a `Path=` to its language file. Used when the engine
/// cannot enumerate its languages.
pub fn ini_dialects(ini: &str, exists: impl Fn(&str) -> bool) -> Vec<u32> {
    let mut out = Vec::new();
    let mut section: Option<u32> = None;
    for line in ini.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.split_once('.').and_then(|(a, b)| {
                let major: u32 = a.trim().parse().ok()?;
                let minor: u32 = b.trim().parse().ok()?;
                Some((major << 16) | minor)
            });
        } else if let (Some(code), Some(path)) = (section, line.strip_prefix("Path=")) {
            if exists(path.trim()) && !out.contains(&code) {
                out.push(code);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_voice_ids() {
        assert_eq!(
            parse_voice_id("en-US:reed"),
            Some(VoiceSel {
                dialect: Some(0x0001_0000),
                preset: Some(1)
            })
        );
        assert_eq!(
            parse_voice_id("de:Shelley"),
            Some(VoiceSel {
                dialect: Some(0x0004_0000),
                preset: Some(2)
            })
        );
        assert_eq!(
            parse_voice_id("en_gb:8"),
            Some(VoiceSel {
                dialect: Some(0x0001_0001),
                preset: Some(8)
            })
        );
        assert_eq!(
            parse_voice_id("grandma"),
            Some(VoiceSel {
                dialect: None,
                preset: Some(7)
            })
        );
        assert_eq!(
            parse_voice_id("fr-CA"),
            Some(VoiceSel {
                dialect: Some(0x0003_0001),
                preset: Some(1)
            })
        );
        assert_eq!(
            parse_voice_id("eloquence:en-US:glen").unwrap().preset,
            Some(5)
        );
        assert_eq!(parse_voice_id("en-US:9"), None);
        assert_eq!(parse_voice_id("klingon"), None);
        assert_eq!(parse_voice_id(""), None);
    }

    #[test]
    fn lists_supported_languages_times_presets() {
        let presets: Vec<PresetInfo> = (0..8)
            .map(|i| PresetInfo {
                name: if i == 0 {
                    "Adult Male 1".into()
                } else {
                    String::new()
                },
                params: [i32::from(i % 2 == 1), 50, 65, 30, 0, 0, 50, 90],
            })
            .collect();
        let v = voice_list(&[0x0001_0000, 0x0008_0000, 0x0004_0000], &presets);
        assert_eq!(v.len(), 16, "Japanese is not offered");
        assert_eq!(v[0].id, "en-US:reed");
        assert_eq!(
            v[0].name,
            "Eloquence Reed (Adult Male 1), English (United States)"
        );
        assert_eq!(v[1].name, "Eloquence Shelley, English (United States)");
        assert_eq!(v[1].gender.as_deref(), Some("female"));
        assert_eq!(v[8].id, "de-DE:reed");
        assert_eq!(v[8].languages, ["de-DE"]);
        for voice in &v {
            let sel = parse_voice_id(&voice.id).unwrap();
            assert!(sel.dialect.is_some() && sel.preset.is_some());
        }
    }

    #[test]
    fn reads_dialects_from_eci_ini() {
        let ini = "[1.0]\nPath=C:\\E\\enu.syn\nVoice1=0 50 65 30 0 0 50 92\n\
                   [8.0]\nPath=C:\\E\\jpn.syn\n[4.0]\nPath=C:\\E\\deu.syn\n[bad]\nPath=x\n";
        let got = ini_dialects(ini, |p| !p.ends_with("jpn.syn"));
        assert_eq!(got, [0x0001_0000, 0x0004_0000]);
    }
}
