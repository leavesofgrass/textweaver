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
//!
//! **Error-prone abbreviations are spelled, never expanded** (new in
//! textweaver). The abbreviations that ISMP and The Joint Commission tell
//! clinicians not to use, because they are misread and have caused harm,
//! are misheard in the same way. Before any expansion, a word on the
//! [`ERROR_PRONE`] list is spelled letter by letter ("QD" is "Q D", "q.o.d."
//! is "Q O D", "MgSO4" is "M G S O 4"), and neither the built-ins nor the
//! user's expansions can replace it. The one exception is "µg", spoken as
//! "micrograms", because the symbol itself is the hazard. The list also
//! applies to engines that expand abbreviations themselves (such as
//! ETI-Eloquence), so they cannot guess either.

use std::collections::{BTreeMap, HashMap, HashSet};

use regex::Captures;
use textweaver_core::OffsetMap;

use super::Transform;
use super::numbers::sentence_ends_at;
use super::rewrite::{Piece, Rule, char_after, char_before, then};

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

/// Error-prone abbreviations: spelled letter by letter, never expanded
/// (case-sensitive, as written).
///
/// Sources: The Joint Commission's official "Do Not Use" list (U, IU,
/// Q.D., QD, q.d., qd, Q.O.D., QOD, q.o.d., qod, MS, MSO4, MgSO4), and the
/// ISMP List of Error-Prone Abbreviations, Symbols, and Dose Designations
/// (Institute for Safe Medication Practices, updated 2024;
/// <https://home.ecri.org/blogs/ismp-resources/list-of-error-prone-abbreviations>):
/// cc, µg, the routes and frequencies SC, SQ, HS, hs, TIW, qhs, qn, q1d,
/// UD, BT, IJ, the ear and eye
/// abbreviations AD, AS, AU, OD, OS, OU (and their dotted lower-case
/// forms), and drug-name abbreviations such as TPA and HCTZ. ISMP's IN and
/// IT are left out: they are everyday words in text set in capitals. See
/// `docs/dev/research/health-sciences-use-cases.md`, section 3.
pub const ERROR_PRONE: &[&str] = &[
    // The Joint Commission "Do Not Use" list.
    "U", "IU", "QD", "Q.D.", "qd", "q.d.", "QOD", "Q.O.D.", "qod", "q.o.d.", "MS", "MSO4",
    "MgSO4", // ISMP: units, routes, frequencies.
    "cc", "µg", "μg", "SC", "SQ", "HS", "hs", "TIW", "qhs", "qn", "q1d", "UD", "BT", "IJ",
    // ISMP: ears and eyes.
    "AD", "AS", "AU", "OD", "OS", "OU", "a.d.", "a.s.", "a.u.", "o.d.", "o.s.", "o.u.",
    // ISMP: drug-name abbreviations.
    "APAP", "AZT", "CPZ", "DPT", "HCT", "HCTZ", "MTX", "PCA", "PTU", "T3", "TAC", "TNK", "TPA",
    "ZnSO4",
];

/// Error-prone words that are also everyday words in capitals ("SUCH AS"):
/// not spelled when a neighboring word is set in capitals too.
const PROSE_WORDS: [&str; 2] = ["AS", "AD"];

/// The spelled form: letters as capitals and digits as written, one per
/// word; "µg" is "micrograms".
fn spell(abbr: &str) -> String {
    if matches!(abbr, "µg" | "μg") {
        return "micrograms".into();
    }
    abbr.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The nearest word before `start` and after `end` (letters only).
fn neighbors(s: &str, start: usize, end: usize) -> [&str; 2] {
    let before = s[..start].trim_end_matches(|c: char| !c.is_alphabetic());
    let b_start = before
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_alphabetic())
        .map_or(0, |(i, c)| i + c.len_utf8());
    let after = s[end..].trim_start_matches(|c: char| !c.is_alphabetic());
    let a_end = after
        .char_indices()
        .find(|(_, c)| !c.is_alphabetic())
        .map_or(after.len(), |(i, _)| i);
    [&before[b_start..], &after[..a_end]]
}

/// The rule that spells error-prone abbreviations.
fn error_prone_rule() -> Rule {
    let set: HashSet<&'static str> = ERROR_PRONE.iter().copied().collect();
    Rule::with(
        // A word start (the micro sign also right after a number, "25µg").
        r"((?:\b[A-Za-z]|[µμ])[A-Za-z0-9]*(?:\.[A-Za-z0-9]+)*)(\.?)",
        move |c: &Captures<'_>, s: &str| {
            let m = c.get(0)?;
            let core = c.get(1)?;
            let first = core.as_str().chars().next()?;
            let before = char_before(s, m.start());
            let micro = matches!(first, 'µ' | 'μ');
            if before.is_some_and(|b| {
                b == '.' || b == '_' || b.is_alphabetic() || (b.is_ascii_digit() && !micro)
            }) {
                return None;
            }
            // Not part of a longer word: "cc'd" and "QDs" stay as written.
            if char_after(s, m.end())
                .is_some_and(|a| a.is_alphanumeric() || matches!(a, '\'' | '\u{2019}'))
            {
                return None;
            }
            let with_dot = m.as_str();
            let (abbr, keep_dot) =
                if c.get(2).is_some_and(|d| !d.is_empty()) && set.contains(with_dot) {
                    (with_dot, false)
                } else if set.contains(core.as_str()) {
                    (core.as_str(), true)
                } else {
                    return None;
                };
            if PROSE_WORDS.contains(&abbr) {
                let shouting = neighbors(s, m.start(), core.end()).iter().any(|w| {
                    w.chars().count() >= 2 && w.chars().all(char::is_uppercase) && !set.contains(w)
                });
                if shouting {
                    return None;
                }
            }
            let mut spoken = spell(abbr);
            if spoken == abbr {
                return None;
            }
            if before.is_some_and(|b| b.is_ascii_digit()) {
                // "25µg" is "25 micrograms".
                spoken.insert(0, ' ');
            }
            if !keep_dot && sentence_ends_at(s, m.end()) {
                spoken.push('.');
            }
            let mut pieces = vec![Piece::Text(spoken)];
            if keep_dot {
                pieces.push(Piece::Keep(2));
            }
            Some(pieces)
        },
    )
}

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
    /// Spells error-prone abbreviations; runs before any expansion.
    error_prone: Rule,
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
    /// themselves). Error-prone abbreviations are still spelled, so with no
    /// entries this is the [`ERROR_PRONE`] list alone.
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
            return Abbreviations {
                error_prone: error_prone_rule(),
                rule: None,
            };
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
        Abbreviations {
            error_prone: error_prone_rule(),
            rule,
        }
    }
}

impl Transform for Abbreviations {
    fn name(&self) -> &'static str {
        "abbreviations"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        let expand = |text: &str| self.rule.as_ref().and_then(|r| r.apply(text));
        match self.error_prone.apply(input) {
            None => expand(input).unwrap_or_else(|| super::identity(input)),
            Some(acc) => {
                let step = expand(&acc.0);
                then(acc, step)
            }
        }
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
