//! Morphy: reducing an inflected word to its base forms, as WordNet's own
//! `morphy` and NLTK's port of it do.
//!
//! For each part of speech, a word's base forms are, in order:
//!
//! 1. the word's entries in that part of speech's exception list
//!    (`geese` to `goose`, `ran` to `run`), with the word itself, when the
//!    word is on the list;
//! 2. otherwise the word itself and every form the detachment rules give
//!    (`running` to `runn` and `run`, `boxes` to `box`), keeping only
//!    forms WordNet has in that part of speech.
//!
//! Like NLTK, this does not split collocations (`running away`); such a
//! phrase is found only as it is written.

use crate::model::Pos;

/// Suffix rules, `(ending, replacement)`, from WordNet's `morph.c`.
pub fn rules(pos: Pos) -> &'static [(&'static str, &'static str)] {
    match pos {
        Pos::Noun => &[
            ("s", ""),
            ("ses", "s"),
            ("ves", "f"),
            ("xes", "x"),
            ("zes", "z"),
            ("ches", "ch"),
            ("shes", "sh"),
            ("men", "man"),
            ("ies", "y"),
        ],
        Pos::Verb => &[
            ("s", ""),
            ("ies", "y"),
            ("es", "e"),
            ("es", ""),
            ("ed", "e"),
            ("ed", ""),
            ("ing", "e"),
            ("ing", ""),
        ],
        Pos::Adjective => &[("er", ""), ("est", ""), ("er", "e"), ("est", "e")],
        Pos::Adverb => &[],
    }
}

/// The base forms of `word` in `pos`. `exceptions` gives the word's entries
/// in the exception list for `pos` (empty when it has none), and `known`
/// says whether WordNet has a form in `pos`.
pub fn base_forms(
    word: &str,
    pos: Pos,
    exceptions: &[String],
    known: impl Fn(&str, Pos) -> bool,
) -> Vec<String> {
    let mut candidates: Vec<String> = vec![word.to_owned()];
    if exceptions.is_empty() {
        for (ending, replacement) in rules(pos) {
            if let Some(stem) = word.strip_suffix(ending) {
                let form = format!("{stem}{replacement}");
                if !form.is_empty() {
                    candidates.push(form);
                }
            }
        }
    } else {
        candidates.extend(exceptions.iter().cloned());
    }
    let mut out: Vec<String> = Vec::new();
    for c in candidates {
        if !out.contains(&c) && known(&c, pos) {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known<'a>(words: &'a [(&'a str, Pos)]) -> impl Fn(&str, Pos) -> bool + 'a {
        move |w, p| words.iter().any(|(x, q)| *x == w && *q == p)
    }

    #[test]
    fn detachment_rules() {
        let k = known(&[
            ("run", Pos::Verb),
            ("running", Pos::Noun),
            ("box", Pos::Noun),
            ("box", Pos::Verb),
            ("church", Pos::Noun),
            ("fly", Pos::Noun),
            ("fly", Pos::Verb),
            ("hope", Pos::Verb),
            ("big", Pos::Adjective),
            ("wide", Pos::Adjective),
            ("man", Pos::Noun),
        ]);
        // Doubled consonants come from the exception list, as in WordNet.
        assert!(base_forms("running", Pos::Verb, &[], &k).is_empty());
        assert_eq!(
            base_forms("running", Pos::Verb, &["run".into()], &k),
            vec!["run"]
        );
        assert_eq!(base_forms("running", Pos::Noun, &[], &k), vec!["running"]);
        assert_eq!(base_forms("boxes", Pos::Noun, &[], &k), vec!["box"]);
        assert_eq!(base_forms("boxes", Pos::Verb, &[], &k), vec!["box"]);
        assert_eq!(base_forms("churches", Pos::Noun, &[], &k), vec!["church"]);
        assert_eq!(base_forms("flies", Pos::Noun, &[], &k), vec!["fly"]);
        assert_eq!(base_forms("flies", Pos::Verb, &[], &k), vec!["fly"]);
        assert_eq!(base_forms("hoped", Pos::Verb, &[], &k), vec!["hope"]);
        assert_eq!(base_forms("hoping", Pos::Verb, &[], &k), vec!["hope"]);
        assert_eq!(base_forms("wider", Pos::Adjective, &[], &k), vec!["wide"]);
        assert_eq!(base_forms("men", Pos::Noun, &[], &k), vec!["man"]);
        assert!(base_forms("quickly", Pos::Adverb, &[], &k).is_empty());
        // No empty stems: "s" alone is not reduced to "".
        assert!(base_forms("s", Pos::Noun, &[], &k).is_empty());
    }

    #[test]
    fn exceptions_replace_the_rules() {
        let k = known(&[("goose", Pos::Noun), ("run", Pos::Verb), ("ran", Pos::Noun)]);
        assert_eq!(
            base_forms("geese", Pos::Noun, &["goose".into()], &k),
            vec!["goose"]
        );
        // The word itself stays first when it is known too.
        assert_eq!(
            base_forms("ran", Pos::Noun, &["run".into()], &k),
            vec!["ran"]
        );
        assert_eq!(
            base_forms("ran", Pos::Verb, &["run".into()], &k),
            vec!["run"]
        );
    }
}
