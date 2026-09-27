//! OCR languages: the `ocr_lang` setting, a document's language tag, and
//! Tesseract's language codes.
//!
//! The setting takes Tesseract's codes (`eng`, `fra`, `deu+eng`) or
//! language tags (`fr`, `de-CH`); both are turned into Tesseract codes here.

/// Language tags (the part before any `-`) and Tesseract's codes for them.
const TAGS: &[(&str, &str)] = &[
    ("af", "afr"),
    ("ar", "ara"),
    ("be", "bel"),
    ("bg", "bul"),
    ("bn", "ben"),
    ("br", "bre"),
    ("bs", "bos"),
    ("ca", "cat"),
    ("cs", "ces"),
    ("cy", "cym"),
    ("da", "dan"),
    ("de", "deu"),
    ("el", "ell"),
    ("en", "eng"),
    ("eo", "epo"),
    ("es", "spa"),
    ("et", "est"),
    ("eu", "eus"),
    ("fa", "fas"),
    ("fi", "fin"),
    ("fo", "fao"),
    ("fr", "fra"),
    ("fy", "fry"),
    ("ga", "gle"),
    ("gd", "gla"),
    ("gl", "glg"),
    ("he", "heb"),
    ("hi", "hin"),
    ("hr", "hrv"),
    ("ht", "hat"),
    ("hu", "hun"),
    ("hy", "hye"),
    ("id", "ind"),
    ("is", "isl"),
    ("it", "ita"),
    ("ja", "jpn"),
    ("ka", "kat"),
    ("kk", "kaz"),
    ("ko", "kor"),
    ("la", "lat"),
    ("lb", "ltz"),
    ("lt", "lit"),
    ("lv", "lav"),
    ("mi", "mri"),
    ("mk", "mkd"),
    ("ms", "msa"),
    ("mt", "mlt"),
    ("nb", "nor"),
    ("nl", "nld"),
    ("nn", "nor"),
    ("no", "nor"),
    ("oc", "oci"),
    ("pl", "pol"),
    ("pt", "por"),
    ("ro", "ron"),
    ("ru", "rus"),
    ("sk", "slk"),
    ("sl", "slv"),
    ("sq", "sqi"),
    ("sr", "srp"),
    ("sv", "swe"),
    ("sw", "swa"),
    ("ta", "tam"),
    ("th", "tha"),
    ("tr", "tur"),
    ("uk", "ukr"),
    ("ur", "urd"),
    ("vi", "vie"),
    ("yi", "yid"),
    ("zh", "chi_sim"),
];

/// Tesseract codes of languages written in the Latin alphabet (PaddleOCR's
/// Latin model covers them).
const LATIN: &[&str] = &[
    "afr", "bos", "bre", "cat", "ces", "cym", "dan", "deu", "eng", "epo", "est", "eus", "fao",
    "fin", "fra", "fry", "gla", "gle", "glg", "hat", "hrv", "hun", "ind", "isl", "ita", "lat",
    "lav", "lit", "ltz", "mlt", "mri", "msa", "nld", "nor", "oci", "pol", "por", "ron", "slk",
    "slv", "spa", "sqi", "swa", "swe", "tur", "vie",
];

/// Tesseract's code for one language: a code as written (`eng`,
/// `chi_tra`), or a language tag (`fr-CA` → `fra`, `zh-Hant` →
/// `chi_tra`). `None` for an empty or unknown value.
fn code(one: &str) -> Option<String> {
    let s = one.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if lower.len() >= 3 && !lower.contains('-') {
        // Already a Tesseract code (`eng`, `chi_sim`, `script/Latin`).
        return Some(s.to_owned());
    }
    let mut parts = lower.split(['-', '_']);
    let primary = parts.next().unwrap_or("");
    if primary == "zh" {
        let traditional = lower.contains("hant")
            || lower.ends_with("-tw")
            || lower.ends_with("-hk")
            || lower.ends_with("-mo");
        return Some(if traditional { "chi_tra" } else { "chi_sim" }.to_owned());
    }
    TAGS.iter()
        .find(|(tag, _)| *tag == primary)
        .map(|(_, c)| (*c).to_owned())
}

/// Tesseract's language argument for a setting or tag: codes joined by
/// `+` (`"fr, en"` → `fra+eng`). `None` when nothing is recognized.
pub fn tesseract_codes(value: &str) -> Option<String> {
    let codes: Vec<String> = value.split(['+', ',', ';', ' ']).filter_map(code).collect();
    (!codes.is_empty()).then(|| codes.join("+"))
}

/// True when every language named is English (ocrs reads English only).
pub fn is_english(value: &str) -> bool {
    tesseract_codes(value).is_some_and(|c| c.split('+').all(|c| c == "eng"))
}

/// True when every language named is written in the Latin alphabet.
pub fn is_latin(value: &str) -> bool {
    tesseract_codes(value).is_some_and(|c| c.split('+').all(|c| LATIN.contains(&c)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_become_tesseract_codes() {
        assert_eq!(tesseract_codes("en-US").as_deref(), Some("eng"));
        assert_eq!(tesseract_codes("fr, en").as_deref(), Some("fra+eng"));
        assert_eq!(tesseract_codes("deu+eng").as_deref(), Some("deu+eng"));
        assert_eq!(tesseract_codes("zh-Hant-TW").as_deref(), Some("chi_tra"));
        assert_eq!(tesseract_codes("zh").as_deref(), Some("chi_sim"));
        assert_eq!(tesseract_codes("  ").as_deref(), None);
        assert_eq!(tesseract_codes("xx").as_deref(), None);
        assert!(is_english("en-GB"));
        assert!(is_english("eng"));
        assert!(!is_english("fr"));
        assert!(!is_english("eng+fra"));
        assert!(is_latin("fra+eng"));
        assert!(!is_latin("rus"));
    }
}
