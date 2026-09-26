//! Abbreviations and the pronunciation lexicon, ported from
//! `star/ttstext/abbreviations.py` (Part 2 sections 5.B.1 and 5.B.2).
//!
//! Star ran 55 case-sensitive `\b`-prefixed substitutions in list order,
//! then the user's. Here they are one pass (user entries first, then the
//! built-ins in Star's order, so the first alternative that matches at a
//! position wins, as Star's sequence did). Deliberate fixes:
//!
//! - Q4: `No.`/`no.`, `p.`, and `pp.` expand only before a number ("No. 5",
//!   "p. 12"), so "I said no." stays "no"; and "a.m."/"p.m." are left to the
//!   numbers transform. Word abbreviations that double as ordinary words at a
//!   sentence end (`ed.`, `min.`, `max.`, `est.`, `sec.`, `temp.`, and
//!   similar) are not expanded there unless a number precedes them ("It took
//!   5 min.").
//! - An abbreviation expanded at the very end of a line keeps its period
//!   ("... et cetera.").
//! - User expansions are inserted literally (Star used them as regex
//!   templates, so backslashes were interpreted).
//! - The lexicon is one pass as well, so a replacement is never rewritten by
//!   a shorter term (Star rewrote "congestive heart failure" when "heart" was
//!   also a term).

use std::collections::{BTreeMap, HashMap};

use regex::Captures;
use textweaver_core::OffsetMap;

use super::Transform;
use super::numbers::sentence_ends_at;
use super::rewrite::{Piece, Rule, char_after};

/// Star's built-in list, in source order (55 entries).
pub const BUILTIN: [(&str, &str); 55] = [
    ("et al.", "and others"),
    ("op. cit.", "op cit"),
    ("e.g.,", "for example,"),
    ("e.g.", "for example"),
    ("i.e.,", "that is,"),
    ("i.e.", "that is"),
    ("etc.", "et cetera"),
    ("cf.", "compare"),
    ("ibid.", "ibid"),
    ("n.d.", "no date"),
    ("ca.", "circa"),
    ("vs.", "versus"),
    ("approx.", "approximately"),
    ("Fig.", "Figure"),
    ("Figs.", "Figures"),
    ("Eq.", "Equation"),
    ("Eqs.", "Equations"),
    ("Sec.", "Section"),
    ("Chap.", "Chapter"),
    ("Ref.", "Reference"),
    ("Refs.", "References"),
    ("Vol.", "Volume"),
    ("vol.", "volume"),
    ("No.", "Number"),
    ("no.", "number"),
    ("pp.", "pages"),
    ("p.", "page"),
    ("ed.", "edition"),
    ("eds.", "editors"),
    ("Dept.", "Department"),
    ("dept.", "department"),
    ("Assoc.", "Association"),
    ("Univ.", "University"),
    ("univ.", "university"),
    ("Dr.", "Doctor"),
    ("Mr.", "Mister"),
    ("Mrs.", "Missus"),
    ("Prof.", "Professor"),
    ("Jr.", "Junior"),
    ("Sr.", "Senior"),
    ("Rev.", "Reverend"),
    ("Gen.", "General"),
    ("Gov.", "Governor"),
    ("hr.", "hour"),
    ("min.", "minutes"),
    ("sec.", "seconds"),
    ("wt.", "weight"),
    ("avg.", "average"),
    ("temp.", "temperature"),
    ("conc.", "concentration"),
    ("est.", "estimated"),
    ("max.", "maximum"),
    ("Inc.", "Incorporated"),
    ("Corp.", "Corporation"),
    ("Ltd.", "Limited"),
];

/// When an abbreviation may expand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Context {
    Always,
    /// Only when a number follows ("No. 5").
    BeforeNumber,
    /// Not at a sentence end, unless a number precedes ("5 min.").
    NotSentenceFinal,
}

fn context_of(abbr: &str) -> Context {
    match abbr {
        "No." | "no." | "p." | "pp." => Context::BeforeNumber,
        "ed." | "eds." | "min." | "max." | "est." | "sec." | "temp." | "hr." | "wt." | "avg."
        | "conc." | "dept." | "univ." | "vol." => Context::NotSentenceFinal,
        _ => Context::Always,
    }
}

fn number_follows(s: &str, end: usize) -> bool {
    s[end..]
        .trim_start_matches([' ', '\u{a0}'])
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
}

fn number_precedes(s: &str, start: usize) -> bool {
    s[..start]
        .trim_end_matches([' ', '\u{a0}'])
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_digit())
}

/// The abbreviation transform.
pub struct Abbreviations {
    rule: Option<Rule>,
}

impl std::fmt::Debug for Abbreviations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Abbreviations").finish_non_exhaustive()
    }
}

impl Abbreviations {
    /// Built-ins plus `custom` (case-sensitive, user entries win).
    pub fn new(custom: &BTreeMap<String, String>) -> Self {
        Self::build(custom, true)
    }

    /// Only the user's entries (for engines that expand abbreviations
    /// themselves).
    pub fn custom_only(custom: &BTreeMap<String, String>) -> Self {
        Self::build(custom, false)
    }

    fn build(custom: &BTreeMap<String, String>, builtins: bool) -> Self {
        let mut table: HashMap<String, (String, Context)> = HashMap::new();
        let mut alternatives: Vec<String> = Vec::new();
        let mut user: Vec<(&String, &String)> =
            custom.iter().filter(|(k, _)| !k.is_empty()).collect();
        user.sort_by(|a, b| {
            b.0.chars()
                .count()
                .cmp(&a.0.chars().count())
                .then(a.0.cmp(b.0))
        });
        for (k, v) in user {
            alternatives.push(regex::escape(k));
            table.insert(k.clone(), (v.clone(), Context::Always));
        }
        if builtins {
            for (k, v) in BUILTIN {
                alternatives.push(regex::escape(k));
                table
                    .entry(k.to_owned())
                    .or_insert_with(|| (v.to_owned(), context_of(k)));
            }
        }
        if alternatives.is_empty() {
            return Abbreviations { rule: None };
        }
        let pattern = format!(r"\b(?:{})", alternatives.join("|"));
        let rule = Rule::try_with(&pattern, move |c: &Captures<'_>, s: &str| {
            let m = c.get(0)?;
            let (exp, ctx) = table.get(m.as_str())?;
            let ok = match ctx {
                Context::Always => true,
                Context::BeforeNumber => number_follows(s, m.end()),
                Context::NotSentenceFinal => {
                    !sentence_ends_at(s, m.end()) || number_precedes(s, m.start())
                }
            };
            if !ok {
                return None;
            }
            let mut exp = exp.clone();
            let line_end = matches!(char_after(s, m.end()), None | Some('\n' | '\r'));
            if m.as_str().ends_with('.') && line_end && !exp.ends_with('.') {
                exp.push('.');
            }
            Some(vec![Piece::Text(exp)])
        });
        Abbreviations { rule }
    }
}

impl Transform for Abbreviations {
    fn name(&self) -> &'static str {
        "abbreviations"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        self.rule
            .as_ref()
            .and_then(|r| r.apply(input))
            .unwrap_or_else(|| super::identity(input))
    }
}

/// The pronunciation lexicon transform: whole terms, case-insensitive,
/// longest first, replaced literally.
pub struct Pronunciations {
    rule: Option<Rule>,
}

impl std::fmt::Debug for Pronunciations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pronunciations").finish_non_exhaustive()
    }
}

impl Pronunciations {
    /// A lexicon of `term -> spoken form`. Empty terms are skipped.
    pub fn new(lexicon: &BTreeMap<String, String>) -> Self {
        let mut terms: Vec<(&String, &String)> =
            lexicon.iter().filter(|(k, _)| !k.is_empty()).collect();
        terms.sort_by(|a, b| {
            b.0.chars()
                .count()
                .cmp(&a.0.chars().count())
                .then(a.0.cmp(b.0))
        });
        if terms.is_empty() {
            return Pronunciations { rule: None };
        }
        let mut table: HashMap<String, String> = HashMap::new();
        let alternatives: Vec<String> = terms
            .iter()
            .map(|(k, v)| {
                table
                    .entry(k.to_lowercase())
                    .or_insert_with(|| (*v).clone());
                let left = if k.chars().next().is_some_and(char::is_alphanumeric) {
                    r"\b"
                } else {
                    ""
                };
                let right = if k.chars().next_back().is_some_and(char::is_alphanumeric) {
                    r"\b"
                } else {
                    ""
                };
                format!("{left}{}{right}", regex::escape(k))
            })
            .collect();
        let pattern = format!("(?i:{})", alternatives.join("|"));
        let rule = Rule::try_with(&pattern, move |c: &Captures<'_>, _| {
            let m = c.get(0)?;
            let spoken = table.get(&m.as_str().to_lowercase())?;
            Some(vec![Piece::Text(spoken.clone())])
        });
        Pronunciations { rule }
    }
}

impl Transform for Pronunciations {
    fn name(&self) -> &'static str {
        "pronunciations"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        self.rule
            .as_ref()
            .and_then(|r| r.apply(input))
            .unwrap_or_else(|| super::identity(input))
    }
}

/// Star `_expand_abbreviations(text, custom)`.
pub fn expand_abbreviations(text: &str, custom: &BTreeMap<String, String>) -> String {
    Abbreviations::new(custom).apply(text).0
}

/// Star `_apply_pronunciations(text, lexicon)`.
pub fn apply_pronunciations(text: &str, lexicon: &BTreeMap<String, String>) -> String {
    Pronunciations::new(lexicon).apply(text).0
}
