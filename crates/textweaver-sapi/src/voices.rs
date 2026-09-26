//! Voice ids, architectures, and voice metadata.
//!
//! A SAPI voice is a registry token. 32-bit-only voices (VW Paul, Kate,
//! James; eSpeak) are registered under `WOW6432Node`, which only a 32-bit
//! process sees, so each voice is tagged with the host architecture that
//! can load it. The textweaver voice id is `"<arch>:<token id>"`, for
//! example `x64:HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices\Tokens\TTS_MS_EN-US_DAVID_11.0`.
//!
//! A voice registered for both architectures is listed once, as x64.

use std::fmt;

use textweaver_speech::Voice;

use crate::protocol::VoiceToken;

/// The host architecture a voice runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Arch {
    /// 64-bit host (`x86_64-pc-windows-msvc`).
    X64,
    /// 32-bit host (`i686-pc-windows-msvc`).
    X86,
}

impl Arch {
    /// Both architectures, preferred first.
    pub const ALL: [Arch; 2] = [Arch::X64, Arch::X86];

    /// `"x64"` or `"x86"`, as in voice ids and [`Reply::Ready`](crate::protocol::Reply::Ready).
    pub fn as_str(self) -> &'static str {
        match self {
            Arch::X64 => "x64",
            Arch::X86 => "x86",
        }
    }

    /// Parses [`as_str`](Self::as_str) output.
    pub fn parse(s: &str) -> Option<Arch> {
        match s {
            "x64" => Some(Arch::X64),
            "x86" => Some(Arch::X86),
            _ => None,
        }
    }

    /// The architecture of this build.
    pub fn native() -> Arch {
        if cfg!(target_pointer_width = "64") {
            Arch::X64
        } else {
            Arch::X86
        }
    }

    /// Index into per-architecture arrays (x64 = 0, x86 = 1).
    pub fn index(self) -> usize {
        match self {
            Arch::X64 => 0,
            Arch::X86 => 1,
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A parsed textweaver SAPI voice id.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VoiceId {
    /// The host that loads it.
    pub arch: Arch,
    /// The SAPI token id (a registry path).
    pub token_id: String,
}

impl VoiceId {
    /// Parses `"<arch>:<token id>"`. A bare token id (no architecture)
    /// means x64.
    pub fn parse(id: &str) -> Option<VoiceId> {
        let id = id.trim();
        if id.is_empty() {
            return None;
        }
        match id.split_once(':').and_then(|(a, t)| Some((Arch::parse(a)?, t))) {
            Some((arch, token)) if !token.is_empty() => Some(VoiceId {
                arch,
                token_id: token.to_owned(),
            }),
            Some(_) => None,
            None => Some(VoiceId {
                arch: Arch::X64,
                token_id: id.to_owned(),
            }),
        }
    }
}

impl fmt::Display for VoiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.arch, self.token_id)
    }
}

/// Voice families with their own rate calibration and behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// Microsoft desktop voices (David, Zira) and OneCore voices.
    Microsoft,
    /// eSpeak's SAPI5 wrapper.
    Espeak,
    /// ETI-Eloquence through Code Factory's SAPI5 voices: one word event per
    /// sentence, so no word timing (ADR-0007). Served properly by the `eci`
    /// backend.
    Eloquence,
    /// OpenEVV's SAPI5 Eloquence voices ("OpenEVV Eloquence", one token per
    /// preset such as Reed), which are said to report word positions. Listed
    /// and treated like any other SAPI voice, with word timing, and tagged
    /// [`TAG_OPENEVV`] so the app can prefer them for Eloquence Reed.
    OpenEvv,
    /// Anything else (VW voices, third-party engines).
    Other,
}

impl Family {
    /// Classifies a voice token by its name, vendor, and id.
    pub fn of(token: &VoiceToken) -> Family {
        let hay = format!("{} {} {}", token.name, token.vendor, token.token_id).to_lowercase();
        if hay.contains("openevv") {
            Family::OpenEvv
        } else if hay.contains("eloquence") {
            Family::Eloquence
        } else if hay.contains("espeak") {
            Family::Espeak
        } else if token.vendor.eq_ignore_ascii_case("microsoft")
            || token.name.starts_with("Microsoft ")
        {
            Family::Microsoft
        } else {
            Family::Other
        }
    }

    /// Whether the voice's word events can drive the highlight.
    pub fn has_word_timing(self) -> bool {
        self != Family::Eloquence
    }
}

/// Tag on OpenEVV voices ([`Family::OpenEvv`]).
pub const TAG_OPENEVV: &str = "OpenEVV";
/// Tag on every Eloquence voice (OpenEVV or Code Factory).
pub const TAG_ELOQUENCE: &str = "Eloquence";
/// Tag on voices whose word events cannot drive the highlight.
pub const TAG_NO_WORD_TIMING: &str = "no word timing";
/// Tag on OneCore voices.
pub const TAG_ONECORE: &str = "OneCore";
/// Tag on voices that run in the 32-bit host.
pub const TAG_32_BIT: &str = "32-bit";

/// A voice with the SAPI details the generic [`Voice`] has no room for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SapiVoice {
    /// The voice as the speech service lists it.
    pub voice: Voice,
    /// The host that loads it.
    pub arch: Arch,
    /// Its family (calibration, word timing).
    pub family: Family,
    /// `Attributes\Vendor` from the token.
    pub vendor: String,
    /// Metadata tags ([`TAG_OPENEVV`], [`TAG_ELOQUENCE`],
    /// [`TAG_NO_WORD_TIMING`], [`TAG_ONECORE`], [`TAG_32_BIT`]).
    pub tags: Vec<String>,
}

impl SapiVoice {
    /// True when the voice carries `tag`.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// Describes a token for listing: the [`Voice`] plus family, vendor, and
/// tags.
pub fn describe(token: &VoiceToken, arch: Arch) -> SapiVoice {
    let family = Family::of(token);
    let mut tags = Vec::new();
    match family {
        Family::OpenEvv => {
            tags.push(TAG_OPENEVV);
            tags.push(TAG_ELOQUENCE);
        }
        Family::Eloquence => {
            tags.push(TAG_ELOQUENCE);
            tags.push(TAG_NO_WORD_TIMING);
        }
        Family::Microsoft | Family::Espeak | Family::Other => {}
    }
    if is_onecore(&token.token_id) {
        tags.push(TAG_ONECORE);
    }
    if arch == Arch::X86 {
        tags.push(TAG_32_BIT);
    }
    SapiVoice {
        voice: to_voice(token, arch),
        arch,
        family,
        vendor: token.vendor.clone(),
        tags: tags.into_iter().map(str::to_owned).collect(),
    }
}

/// Is this token a OneCore voice (`Speech_OneCore` registry tree)?
pub fn is_onecore(token_id: &str) -> bool {
    token_id.to_ascii_lowercase().contains("\\speech_onecore\\")
}

/// Converts a token's `Language` attribute (hex LCIDs, `;`-separated) to
/// BCP 47 tags. Unknown LCIDs are skipped.
pub fn languages(lcids: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in lcids.split(';') {
        let Ok(lcid) = u32::from_str_radix(part.trim(), 16) else {
            continue;
        };
        if let Some(tag) = lcid_to_bcp47(lcid) {
            if !out.iter().any(|t| t == tag) {
                out.push(tag.to_owned());
            }
        }
    }
    out
}

/// BCP 47 tag for a Windows LCID (the languages SAPI voices commonly ship).
pub fn lcid_to_bcp47(lcid: u32) -> Option<&'static str> {
    Some(match lcid {
        0x0009 => "en",
        0x0409 => "en-US",
        0x0809 => "en-GB",
        0x0c09 => "en-AU",
        0x1009 => "en-CA",
        0x1409 => "en-NZ",
        0x1809 => "en-IE",
        0x1c09 => "en-ZA",
        0x4009 => "en-IN",
        0x000c => "fr",
        0x040c => "fr-FR",
        0x0c0c => "fr-CA",
        0x0007 => "de",
        0x0407 => "de-DE",
        0x0010 => "it",
        0x0410 => "it-IT",
        0x000a => "es",
        0x040a | 0x0c0a => "es-ES",
        0x080a => "es-MX",
        0x0016 => "pt",
        0x0416 => "pt-BR",
        0x0816 => "pt-PT",
        0x040b => "fi-FI",
        0x0413 => "nl-NL",
        0x041d => "sv-SE",
        0x0414 => "nb-NO",
        0x0406 => "da-DK",
        0x0415 => "pl-PL",
        0x0405 => "cs-CZ",
        0x040e => "hu-HU",
        0x0408 => "el-GR",
        0x041f => "tr-TR",
        0x0419 => "ru-RU",
        0x040d => "he-IL",
        0x0401 => "ar-SA",
        0x0439 => "hi-IN",
        0x0411 => "ja-JP",
        0x0412 => "ko-KR",
        0x0804 => "zh-CN",
        0x0404 => "zh-TW",
        0x0c04 => "zh-HK",
        _ => return None,
    })
}

/// The voice as the speech service lists it. The name says what a listener
/// needs to know: "(32-bit)" for voices that run in the x86 host, and a
/// note on Eloquence voices, which give no word timing through SAPI.
pub fn to_voice(token: &VoiceToken, arch: Arch) -> Voice {
    let mut name = if token.name.is_empty() {
        token
            .token_id
            .rsplit('\\')
            .next()
            .unwrap_or(&token.token_id)
            .to_owned()
    } else {
        token.name.clone()
    };
    if is_onecore(&token.token_id) {
        name.push_str(" (OneCore)");
    }
    if arch == Arch::X86 {
        name.push_str(" (32-bit)");
    }
    if !Family::of(token).has_word_timing() {
        name.push_str(" (no word highlighting)");
    }
    let gender = match token.gender.trim() {
        "" => None,
        g => Some(g.to_owned()),
    };
    Voice {
        id: VoiceId {
            arch,
            token_id: token.token_id.clone(),
        }
        .to_string(),
        name,
        languages: languages(&token.language),
        gender,
    }
}

/// Merges per-architecture token lists into one voice list: x64 voices
/// first, then x86 voices whose token id is not already listed.
pub fn merge(lists: &[(Arch, Vec<VoiceToken>)]) -> Vec<(Arch, VoiceToken)> {
    let mut out: Vec<(Arch, VoiceToken)> = Vec::new();
    let mut sorted: Vec<&(Arch, Vec<VoiceToken>)> = lists.iter().collect();
    sorted.sort_by_key(|(a, _)| *a);
    for (arch, tokens) in sorted {
        for t in tokens {
            if !out
                .iter()
                .any(|(_, o)| o.token_id.eq_ignore_ascii_case(&t.token_id))
            {
                out.push((*arch, t.clone()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(name: &str, id: &str) -> VoiceToken {
        VoiceToken {
            token_id: id.into(),
            name: name.into(),
            language: "409".into(),
            gender: "Male".into(),
            vendor: "Microsoft".into(),
        }
    }

    const DAVID: &str =
        r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices\Tokens\TTS_MS_EN-US_DAVID_11.0";

    #[test]
    fn voice_ids_round_trip() {
        let id = VoiceId::parse(&format!("x86:{DAVID}")).unwrap();
        assert_eq!(id.arch, Arch::X86);
        assert_eq!(id.token_id, DAVID);
        assert_eq!(VoiceId::parse(&id.to_string()), Some(id));
        assert_eq!(VoiceId::parse(DAVID).unwrap().arch, Arch::X64);
        assert_eq!(VoiceId::parse(""), None);
        assert_eq!(VoiceId::parse("x64:"), None);
    }

    #[test]
    fn families() {
        assert_eq!(
            Family::of(&token("Microsoft David Desktop", DAVID)),
            Family::Microsoft
        );
        let mut e = token("eSpeak-EN", "x");
        e.vendor = "http://espeak.sf.net".into();
        assert_eq!(Family::of(&e), Family::Espeak);
        let mut q = token("Eloquence US English", "x");
        q.vendor = "Code Factory".into();
        assert_eq!(Family::of(&q), Family::Eloquence);
        assert!(!Family::Eloquence.has_word_timing());
        let mut v = token("VW Paul", "x");
        v.vendor = "NeoSpeech".into();
        assert_eq!(Family::of(&v), Family::Other);
    }

    /// Synthetic attributes of an OpenEVV token (not installed here, and
    /// never loaded: this reads attributes only).
    fn openevv_reed() -> VoiceToken {
        VoiceToken {
            token_id: r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices\Tokens\OpenEVV_Reed"
                .into(),
            name: "OpenEVV Eloquence Reed".into(),
            language: "409".into(),
            gender: "Male".into(),
            vendor: "OpenEVV".into(),
        }
    }

    #[test]
    fn openevv_voices_are_tagged_and_keep_word_timing() {
        let d = describe(&openevv_reed(), Arch::X64);
        assert_eq!(d.family, Family::OpenEvv);
        assert!(d.family.has_word_timing());
        assert!(d.has_tag(TAG_OPENEVV));
        assert!(d.has_tag(TAG_ELOQUENCE));
        assert!(!d.has_tag(TAG_NO_WORD_TIMING));
        assert_eq!(d.vendor, "OpenEVV");
        // Listed like any other voice: name unchanged, no warning.
        assert_eq!(d.voice.name, "OpenEVV Eloquence Reed");
        assert_eq!(d.voice.id, format!("x64:{}", openevv_reed().token_id));
        // Recognized by vendor alone, or by name alone.
        let mut by_vendor = openevv_reed();
        by_vendor.name = "Reed".into();
        by_vendor.token_id = "t".into();
        assert_eq!(Family::of(&by_vendor), Family::OpenEvv);
        let mut by_name = openevv_reed();
        by_name.vendor = "Someone".into();
        by_name.token_id = "t".into();
        assert_eq!(Family::of(&by_name), Family::OpenEvv);
    }

    #[test]
    fn code_factory_eloquence_stays_flagged() {
        let cf = VoiceToken {
            token_id: r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices\Tokens\ETI-Eloquence-US".into(),
            name: "Eloquence US English".into(),
            language: "409".into(),
            gender: "Male".into(),
            vendor: "Code Factory".into(),
        };
        let d = describe(&cf, Arch::X86);
        assert_eq!(d.family, Family::Eloquence);
        assert!(d.has_tag(TAG_ELOQUENCE));
        assert!(d.has_tag(TAG_NO_WORD_TIMING));
        assert!(d.has_tag(TAG_32_BIT));
        assert!(!d.has_tag(TAG_OPENEVV));
        assert_eq!(
            d.voice.name,
            "Eloquence US English (32-bit) (no word highlighting)"
        );
    }

    #[test]
    fn languages_from_lcids() {
        assert_eq!(languages("409;9"), ["en-US", "en"]);
        assert_eq!(languages("809"), ["en-GB"]);
        assert_eq!(languages("zz;409;409"), ["en-US"]);
        assert!(languages("").is_empty());
    }

    #[test]
    fn voice_names_carry_what_a_listener_needs() {
        let v = to_voice(&token("Microsoft David Desktop", DAVID), Arch::X64);
        assert_eq!(v.name, "Microsoft David Desktop");
        assert_eq!(v.id, format!("x64:{DAVID}"));
        assert_eq!(v.languages, ["en-US"]);
        assert_eq!(v.gender.as_deref(), Some("Male"));
        let v = to_voice(&token("VW Paul", "t"), Arch::X86);
        assert_eq!(v.name, "VW Paul (32-bit)");
        let v = to_voice(&token("Eloquence US English", "t"), Arch::X64);
        assert_eq!(v.name, "Eloquence US English (no word highlighting)");
        let one = r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech_OneCore\Voices\Tokens\MSTTS_V110_enUS_MarkM";
        let v = to_voice(&token("Microsoft Mark", one), Arch::X64);
        assert_eq!(v.name, "Microsoft Mark (OneCore)");
    }

    #[test]
    fn merge_prefers_x64_and_drops_duplicates() {
        let lists = vec![
            (
                Arch::X86,
                vec![token("David", DAVID), token("VW Paul", "paul")],
            ),
            (Arch::X64, vec![token("David", DAVID)]),
        ];
        let m = merge(&lists);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].0, Arch::X64);
        assert_eq!(m[1], (Arch::X86, token("VW Paul", "paul")));
    }
}
