//! Definitions for difficult words (ADR-0037).
//!
//! With difficult words marked and the verbosity high, a word move onto a
//! difficult word says "difficult word"; with `[reading_aids]
//! difficult_definitions` on, it also says the word's first definition.
//! This crate does not own a dictionary: the caller passes any
//! [`Definitions`] (the app passes its define-word dictionary, the
//! glossary first, then WordNet), and [`difficult_definition`] decides
//! whether the word is difficult and shortens what the dictionary says, so
//! a word move stays a short message.

use textweaver_core::CharRange;
use textweaver_text::Document;

use crate::difficult::{DifficultOptions, WordList, difficult_range};

/// A source of definitions.
pub trait Definitions {
    /// The first (most common) definition of `word`, as the dictionary
    /// writes it, or `None` when it has none.
    fn first_definition(&self, word: &str) -> Option<String>;
}

impl<F: Fn(&str) -> Option<String>> Definitions for F {
    fn first_definition(&self, word: &str) -> Option<String> {
        self(word)
    }
}

/// The most characters of a definition said on a word move.
pub const DEFINITION_MAX_CHARS: usize = 100;

/// The first definition of the word at `word`, shortened
/// ([`short_definition`]), when the word is difficult by `list` and
/// `options` and `definitions` knows it; else `None`.
pub fn difficult_definition<L: WordList + ?Sized, D: Definitions + ?Sized>(
    doc: &Document,
    word: CharRange,
    list: &L,
    options: &DifficultOptions,
    definitions: &D,
) -> Option<String> {
    let difficult = difficult_range(doc, word, list, options)
        .iter()
        .any(|r| r.start == word.start);
    if !difficult {
        return None;
    }
    let text = definitions.first_definition(&doc.slice(word))?;
    let short = short_definition(&text, DEFINITION_MAX_CHARS);
    (!short.is_empty()).then_some(short)
}

/// `definition` made short enough to say after a word: its first clause
/// (up to a semicolon), on one line, without a final full stop, and at
/// most `max` characters, cut at a space. No symbol marks the cut: an
/// ellipsis is noise on a braille display.
pub fn short_definition(definition: &str, max: usize) -> String {
    let clause = definition.split(';').next().unwrap_or_default();
    let one_line = clause.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = one_line.trim_end_matches(['.', ',', ':']);
    if trimmed.chars().count() <= max {
        return trimmed.to_owned();
    }
    let cut: String = trimmed.chars().take(max).collect();
    match cut.rfind(' ') {
        Some(i) if i > 0 => cut[..i].trim_end_matches([',', ':']).to_owned(),
        _ => cut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::difficult::FrequencyList;

    fn list() -> FrequencyList {
        FrequencyList::parse("the\t7.7\nstudents\t5.1\npharmacology\t2.4\nread\t5.6\n").unwrap()
    }

    fn dictionary(word: &str) -> Option<String> {
        match word {
            "pharmacology" => Some(
                "the science or study of drugs: their preparation and properties and uses \
                 and effects; also the branch of medicine"
                    .into(),
            ),
            "students" => Some("a learner who is enrolled in an educational institution".into()),
            _ => None,
        }
    }

    #[test]
    fn a_difficult_word_gets_its_first_clause() {
        let doc = Document::from_plain_text("The students read pharmacology.");
        let word = CharRange::new(18, 30);
        assert_eq!(doc.slice(word), "pharmacology");
        let said = difficult_definition(
            &doc,
            word,
            &list(),
            &DifficultOptions::default(),
            &dictionary,
        );
        assert_eq!(
            said.as_deref(),
            Some(
                "the science or study of drugs: their preparation and properties and uses and effects"
            )
        );
    }

    #[test]
    fn a_common_or_unknown_word_gets_nothing() {
        let doc = Document::from_plain_text("The students read pharmacology.");
        let o = DifficultOptions::default();
        // Common: no definition even though the dictionary has one.
        let students = CharRange::new(4, 12);
        assert_eq!(
            difficult_definition(&doc, students, &list(), &o, &dictionary),
            None
        );
        // Difficult, but the dictionary does not know it.
        let none = |_: &str| None;
        let word = CharRange::new(18, 30);
        assert_eq!(difficult_definition(&doc, word, &list(), &o, &none), None);
    }

    #[test]
    fn short_definitions_cut_at_a_space_without_symbols() {
        assert_eq!(short_definition("a drug.", 100), "a drug");
        assert_eq!(short_definition("first; second", 100), "first");
        let long = "word ".repeat(40);
        let s = short_definition(&long, 23);
        assert_eq!(s, "word word word word");
        assert!(!s.contains('\u{2026}'));
        assert_eq!(short_definition("  ", 10), "");
        assert_eq!(short_definition("unbreakable", 4), "unbr");
    }
}
