//! Citation keys: generating them and checking Pandoc's key syntax.
//!
//! Generated keys are the first creator's family name, folded to lowercase
//! ASCII where a Latin letter has an obvious base, followed by the year:
//! `doe2020`, `muller2019`, `vangogh1888`. They read aloud naturally ("doe
//! twenty twenty") and match what Better BibTeX and most Pandoc users type.
//! A second work by the same author in the same year gets `a`, `b`, ... as
//! in author-date styles: `doe2020a`.

use crate::reference::Reference;

/// A key for a reference, before collisions are resolved: family name and
/// year, else the first title word and year, else `ref`.
pub fn base_key(r: &Reference) -> String {
    let year = r.year().map(|y| y.to_string()).unwrap_or_default();
    let family = r
        .creators()
        .first()
        .map(|n| fold(&n.family_display()))
        .filter(|s| !s.is_empty());
    let stem = family.or_else(|| {
        r.title.as_deref().and_then(|t| {
            crate::text::plain_title(t)
                .split_whitespace()
                .map(fold)
                .find(|w| w.len() > 3 && !STOP_WORDS.contains(&w.as_str()))
        })
    });
    let key = format!("{}{}", stem.unwrap_or_else(|| "ref".to_owned()), year);
    // Pandoc keys must start with a letter, digit, or underscore; fold()
    // guarantees that, but a bare year still reads better with a stem.
    if key.chars().all(|c| c.is_ascii_digit()) {
        format!("ref{key}")
    } else {
        key
    }
}

const STOP_WORDS: &[&str] = &[
    "the", "and", "from", "with", "into", "about", "what", "when", "where", "which",
];

/// Returns `base` if `taken` says it is free, else `base` + `a`, `b`, ...
/// `z`, then `base-2`, `base-3`, ...
pub fn unique_key(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_owned();
    }
    for suffix in 'a'..='z' {
        let k = format!("{base}{suffix}");
        if !taken(&k) {
            return k;
        }
    }
    let mut n = 2usize;
    loop {
        let k = format!("{base}-{n}");
        if !taken(&k) {
            return k;
        }
        n += 1;
    }
}

/// Whether `key` can be written as `@key` in Pandoc Markdown without
/// braces: it starts with a letter, digit, or `_`, and any internal
/// punctuation (`:.#$%&-+?<>~/`) is followed by a letter or digit.
pub fn is_valid_key(key: &str) -> bool {
    let chars: Vec<char> = key.chars().collect();
    let Some(&first) = chars.first() else {
        return false;
    };
    if !(first.is_alphanumeric() || first == '_') {
        return false;
    }
    let mut i = 1;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphanumeric() || c == '_' {
            i += 1;
        } else if is_internal_punct(c) {
            match chars.get(i + 1) {
                Some(n) if n.is_alphanumeric() || *n == '_' => i += 1,
                _ => return false,
            }
        } else {
            return false;
        }
    }
    true
}

/// Punctuation Pandoc allows inside a citation key.
pub(crate) fn is_internal_punct(c: char) -> bool {
    matches!(
        c,
        ':' | '.' | '#' | '$' | '%' | '&' | '-' | '+' | '?' | '<' | '>' | '~' | '/'
    )
}

/// Lowercases and folds Latin letters with diacritics to their base letter;
/// keeps other letters and digits; drops everything else.
pub(crate) fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        match c {
            'a'..='z' | '0'..='9' => out.push(c),
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => out.push('a'),
            'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => out.push('c'),
            'ď' | 'đ' | 'ð' => out.push('d'),
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => out.push('e'),
            'ĝ' | 'ğ' | 'ġ' | 'ģ' => out.push('g'),
            'ĥ' | 'ħ' => out.push('h'),
            'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => out.push('i'),
            'ĵ' => out.push('j'),
            'ķ' => out.push('k'),
            'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => out.push('l'),
            'ñ' | 'ń' | 'ņ' | 'ň' => out.push('n'),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => out.push('o'),
            'ŕ' | 'ŗ' | 'ř' => out.push('r'),
            'ś' | 'ŝ' | 'ş' | 'š' | 'ș' => out.push('s'),
            'ţ' | 'ť' | 'ŧ' | 'ț' => out.push('t'),
            'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => out.push('u'),
            'ŵ' => out.push('w'),
            'ý' | 'ÿ' | 'ŷ' => out.push('y'),
            'ź' | 'ż' | 'ž' => out.push('z'),
            'ß' => out.push_str("ss"),
            'æ' => out.push_str("ae"),
            'œ' => out.push_str("oe"),
            'þ' => out.push_str("th"),
            c if c.is_alphanumeric() => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{CslDate, Name};

    #[test]
    fn keys_are_family_and_year() {
        let mut r = Reference::new("", "book");
        r.author = vec![Name::new("Müller-Lüdenscheidt", "Jörg")];
        r.issued = Some(CslDate::year(2019));
        assert_eq!(base_key(&r), "mullerludenscheidt2019");
        r.author = vec![Name::parse("Vincent van Gogh")];
        assert_eq!(base_key(&r), "vangogh2019");
        r.author.clear();
        r.title = Some("The Origin of Species".into());
        assert_eq!(base_key(&r), "origin2019");
        r.title = None;
        assert_eq!(base_key(&r), "ref2019");
        r.issued = None;
        assert_eq!(base_key(&r), "ref");
    }

    #[test]
    fn collisions_get_letters() {
        let taken = ["doe2020", "doe2020a"];
        assert_eq!(unique_key("doe2020", |k| taken.contains(&k)), "doe2020b");
        assert_eq!(unique_key("new", |_| false), "new");
    }

    #[test]
    fn pandoc_key_syntax() {
        assert!(is_valid_key("doe2020"));
        assert!(is_valid_key("Doe:2020.a"));
        assert!(is_valid_key("_x"));
        assert!(!is_valid_key("doe2020."));
        assert!(!is_valid_key("-doe"));
        assert!(!is_valid_key("doe 2020"));
        assert!(!is_valid_key(""));
    }
}
