//! Voice naming and default selection, shared by both backends.
//!
//! Apple's voice identifiers look like `com.apple.eloquence.en-US.Reed`,
//! `com.apple.voice.compact.en-US.Samantha`, or
//! `com.apple.speech.synthesis.voice.Alex`. The Eloquence voices exist in
//! several languages under the same name (Reed in English (US), English
//! (UK), German, ...), so their display names always carry the language and
//! the word "Eloquence", which is what users know them by.

use textweaver_speech::Voice;

/// Identifier prefix of Apple's bundled ETI-Eloquence voices.
pub const ELOQUENCE_PREFIX: &str = "com.apple.eloquence.";

/// True for Apple's ETI-Eloquence voices.
pub fn is_eloquence(id: &str) -> bool {
    id.starts_with(ELOQUENCE_PREFIX)
}

/// True when the voice normalizes numbers, abbreviations, and dates itself,
/// so the speech service should skip textweaver's overlapping normalization
/// (ADR-0007): Eloquence speaks "Dr." as "Doctor" and "9:30 a.m." as a time.
pub fn normalizes_natively(voice_id: Option<&str>) -> bool {
    voice_id.is_some_and(is_eloquence)
}

/// The language tag inside an identifier such as
/// `com.apple.eloquence.en-US.Reed`, if there is one.
fn language_in_id(id: &str) -> Option<&str> {
    id.split('.')
        .find(|part| part.len() >= 4 && part.as_bytes().get(2) == Some(&b'-'))
}

/// A readable English name for common language tags; the tag itself
/// otherwise.
pub fn language_name(tag: &str) -> String {
    let name = match tag.replace('_', "-").as_str() {
        "en-US" => "English, United States",
        "en-GB" => "English, United Kingdom",
        "en-AU" => "English, Australia",
        "en-IE" => "English, Ireland",
        "en-IN" => "English, India",
        "en-ZA" => "English, South Africa",
        "de-DE" => "German",
        "es-ES" => "Spanish, Spain",
        "es-MX" => "Spanish, Mexico",
        "fr-FR" => "French, France",
        "fr-CA" => "French, Canada",
        "it-IT" => "Italian",
        "pt-BR" => "Portuguese, Brazil",
        "fi-FI" => "Finnish",
        "ja-JP" => "Japanese",
        "ko-KR" => "Korean",
        "zh-CN" => "Chinese, China mainland",
        "zh-TW" => "Chinese, Taiwan",
        other => return other.to_string(),
    };
    name.to_string()
}

/// The display name for a voice: "Eloquence Reed (English, United States)"
/// for Eloquence, "Samantha (English, United States)" otherwise. `name` is
/// Apple's name for the voice, which for Eloquence may already carry the
/// language in parentheses ("Reed (English (US))"); that part is replaced.
pub fn display_name(id: &str, name: &str, language: &str) -> String {
    let base = name.split(" (").next().unwrap_or(name).trim();
    let base = if base.is_empty() { id } else { base };
    let tag = if language.is_empty() {
        language_in_id(id).unwrap_or("")
    } else {
        language
    };
    let lang = if tag.is_empty() {
        String::new()
    } else {
        format!(" ({})", language_name(tag))
    };
    if is_eloquence(id) {
        format!("Eloquence {base}{lang}")
    } else {
        format!("{base}{lang}")
    }
}

/// Builds a [`Voice`] from Apple's attributes.
pub fn voice(id: &str, name: &str, language: &str, gender: Option<&str>) -> Voice {
    let tag = if language.is_empty() {
        language_in_id(id).unwrap_or("").to_string()
    } else {
        language.replace('_', "-")
    };
    Voice {
        id: id.to_string(),
        name: display_name(id, name, &tag),
        languages: if tag.is_empty() {
            Vec::new()
        } else {
            vec![tag]
        },
        gender: gender.map(str::to_string),
    }
}

/// Sorts voices for presentation: Eloquence first, then by language and
/// name.
pub fn sort_voices(voices: &mut [Voice]) {
    voices.sort_by(|a, b| {
        let key = |v: &Voice| {
            (
                !is_eloquence(&v.id),
                v.languages.first().cloned().unwrap_or_default(),
                v.name.clone(),
            )
        };
        key(a).cmp(&key(b))
    });
}

/// The default voice among `ids`: Eloquence Reed (US English) when present
/// ([`crate::DEFAULT_VOICE`]), else any US English Eloquence voice, else
/// `None` (the system default voice).
pub fn default_voice<'a>(ids: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let mut fallback = None;
    for id in ids {
        if id == crate::DEFAULT_VOICE {
            return Some(id);
        }
        if fallback.is_none() && id.starts_with("com.apple.eloquence.en-US.") {
            fallback = Some(id);
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eloquence_voices_are_named_clearly() {
        let v = voice(
            "com.apple.eloquence.en-US.Reed",
            "Reed (English (US))",
            "en-US",
            Some("male"),
        );
        assert_eq!(v.name, "Eloquence Reed (English, United States)");
        assert_eq!(v.languages, ["en-US"]);
        assert_eq!(v.gender.as_deref(), Some("male"));
        let v = voice("com.apple.eloquence.de-DE.Shelley", "Shelley", "", None);
        assert_eq!(v.name, "Eloquence Shelley (German)");
        assert_eq!(v.languages, ["de-DE"]);
    }

    #[test]
    fn other_voices_keep_their_names() {
        let v = voice(
            "com.apple.voice.compact.en-US.Samantha",
            "Samantha",
            "en_US",
            None,
        );
        assert_eq!(v.name, "Samantha (English, United States)");
        assert_eq!(v.languages, ["en-US"]);
        let v = voice("com.apple.speech.synthesis.voice.Alex", "Alex", "", None);
        assert_eq!(v.name, "Alex");
        assert!(v.languages.is_empty());
        assert_eq!(language_name("xx-YY"), "xx-YY");
    }

    #[test]
    fn normalization_is_native_only_for_eloquence() {
        assert!(normalizes_natively(Some("com.apple.eloquence.en-US.Reed")));
        assert!(!normalizes_natively(Some(
            "com.apple.voice.compact.en-US.Samantha"
        )));
        assert!(!normalizes_natively(None));
    }

    #[test]
    fn default_prefers_reed_then_us_eloquence() {
        let ids = [
            "com.apple.voice.compact.en-US.Samantha",
            "com.apple.eloquence.en-US.Shelley",
            "com.apple.eloquence.en-US.Reed",
        ];
        assert_eq!(default_voice(ids), Some(crate::DEFAULT_VOICE));
        assert_eq!(
            default_voice(ids[..2].iter().copied()),
            Some("com.apple.eloquence.en-US.Shelley")
        );
        assert_eq!(default_voice(ids[..1].iter().copied()), None);
    }

    #[test]
    fn sorting_puts_eloquence_first() {
        let mut v = vec![
            voice(
                "com.apple.voice.compact.en-US.Samantha",
                "Samantha",
                "en-US",
                None,
            ),
            voice("com.apple.eloquence.fr-FR.Reed", "Reed", "fr-FR", None),
            voice("com.apple.eloquence.en-US.Reed", "Reed", "en-US", None),
        ];
        sort_voices(&mut v);
        assert_eq!(v[0].id, "com.apple.eloquence.en-US.Reed");
        assert_eq!(v[1].id, "com.apple.eloquence.fr-FR.Reed");
        assert_eq!(v[2].id, "com.apple.voice.compact.en-US.Samantha");
    }
}
