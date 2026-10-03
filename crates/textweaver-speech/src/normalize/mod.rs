//! Text normalization with offset maps (ADR-0005).
//!
//! Each [`Transform`] turns its input into spoken text and an
//! [`OffsetMap`] from its output bytes to its input chars. A [`Pipeline`]
//! applies transforms to one utterance at a time (never across chunks) and
//! composes the maps so the result still points into the document. Every
//! transform builds its output with
//! [`SpokenBuilder`](textweaver_core::SpokenBuilder) (through the rule
//! engine in `rewrite`), so highlighting stays exact after any expansion.
//!
//! The chain, in order ([`Pipeline::for_settings`]):
//!
//! | # | Transform | Setting | Star |
//! |---|---|---|---|
//! | 1 | [`Math`] (`textweaver-math`, ADR-0018; MathCAT, ADR-0029) | `math`, `math_verbosity`, `asciimath_delimiter`, `math_engine` | step 4 |
//! | 2 | [`MarkdownResidue`] (and table narration) | `markdown` (off by default) | `_strip_markdown_for_tts`, load time |
//! | 3 | [`Pronunciations`] | `use_pronunciations` + lexicon | step 1 |
//! | 4 | [`MedicalLexicon`] (drug names, clinical terms, dosing) | `medical_lexicon.enabled` (off by default) | new |
//! | 5 | [`CommunityLexicon`] (IBMTTS community dictionaries) | `community_lexicon.enabled` (off by default) | new |
//! | 6 | [`Abbreviations`] (error-prone ones spelled first) | `abbreviations` | step 2 |
//! | 7 | [`Numbers`] (identifiers as digits, then ranges, units, dates, times, currency, percent, ordinals, decimals, years) | `numbers` | step 3 |
//! | 8 | [`SplitCaps`] (Tall Man drug names kept whole) | service `split_caps` | new |
//! | 9 | [`Punctuation`] | service punctuation level | new |
//!
//! Math runs first, where Star ran it last: `Numbers` turns `$2` into
//! currency words, which destroyed `$2x$`, and Markdown residue removal or a
//! lexicon could rewrite parts of a formula. Math speech leaves digits
//! literal, so `Numbers` still reads them afterwards, and every later step
//! sees spoken words, never LaTeX.
//!
//! **Engines that normalize natively.** A backend with
//! [`Caps::NATIVE_NORMALIZATION`](crate::Caps::NATIVE_NORMALIZATION) (such
//! as ETI-Eloquence, which reads numbers, dates, times, currency, and
//! abbreviations itself) gets the pipeline without the built-in
//! abbreviations (user abbreviations and the error-prone list still
//! apply), with only the identifier guard of numbers
//! ([`Numbers::identifiers_only`]), and without the community lexicon
//! (Eloquence loads those dictionaries itself). Markdown residue, the
//! user's pronunciation lexicon, the medical lexicon, math, split caps,
//! and punctuation verbosity still apply.
//!
//! **Allocation.** A transform that leaves an utterance unchanged allocates
//! nothing ([`Transform::apply_changed`] returns `None`, and its rules
//! search without building capture groups), and the pipeline then neither
//! copies the text nor composes a map. Most utterances pass most steps
//! unchanged, so this is what keeps normalization cheap.
//!
//! Star's expected strings from `tests/test_ttstext.py` are ported as tests
//! in this module, each also checking the offset map's invariants; the
//! deliberate differences are listed in each transform's module docs.

mod abbreviations;
pub mod community;
mod markdown;
mod math;
pub mod medical;
mod numbers;
mod punctuation;
pub(crate) mod rewrite;
mod ssml;
pub mod words;

use std::collections::BTreeMap;

pub use abbreviations::{
    Abbreviations, BUILTIN as BUILTIN_ABBREVIATIONS, ERROR_PRONE as ERROR_PRONE_ABBREVIATIONS,
    Pronunciations, apply_pronunciations, expand_abbreviations,
};
pub use community::{CommunityLexicon, CommunityLexiconConfig};
pub use markdown::{MarkdownResidue, TableMode, strip_markdown, tables_to_narration};
pub use math::{Math, MathEngine, mathcat_available, normalize_math};
pub use medical::{MedicalLexicon, MedicalLexiconConfig};
pub use numbers::{Numbers, normalize_numbers};
pub use punctuation::{Punctuation, SplitCaps, char_name};
use serde::{Deserialize, Serialize};
pub use ssml::{text_to_dectalk, text_to_ssml};
use textweaver_core::{CharPos, OffsetMap, PunctuationLevel, Utterance, Verbosity};

/// One normalization step.
pub trait Transform: Send + Sync {
    /// Stable name, used in settings toggles and diagnostics.
    fn name(&self) -> &'static str;
    /// Returns the transformed text and a map from its bytes to `input` chars
    /// (positions counted from 0 within `input`).
    fn apply(&self, input: &str) -> (String, OffsetMap);

    /// Like [`apply`](Self::apply), but `None` when the text is unchanged,
    /// so a [`Pipeline`] neither copies the text nor composes an identity
    /// map. The transforms in this module return `None` without
    /// allocating; the default calls `apply` and returns `None` when it
    /// gave the identity.
    fn apply_changed(&self, input: &str) -> Option<(String, OffsetMap)> {
        let (out, map) = self.apply(input);
        (!is_identity(input, &out, &map)).then_some((out, map))
    }
}

/// True when `(out, map)` is the identity of `input`: the same text in one
/// literal span from char 0 (or no span, for empty text).
fn is_identity(input: &str, out: &str, map: &OffsetMap) -> bool {
    if out != input {
        return false;
    }
    match map.spans() {
        [] => input.is_empty(),
        [s] => {
            s.kind == textweaver_core::SpanKind::Literal
                && s.source.start == CharPos::ZERO
                && s.spoken.start == 0
                && s.spoken.end as usize == out.len()
        }
        _ => false,
    }
}

/// The identity output for `input`.
pub(crate) fn identity(input: &str) -> (String, OffsetMap) {
    (input.to_owned(), OffsetMap::identity(input, CharPos::ZERO))
}

/// Normalization settings (mirrors `[normalization]` in `settings.toml`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NormalizeConfig {
    /// Strip Markdown syntax left in plain text (headings, emphasis, links,
    /// list markers, tables). Off by default: the Markdown loader already
    /// produces clean text.
    pub markdown: bool,
    /// With `markdown`: drop code blocks instead of reading them.
    pub skip_code: bool,
    /// With `markdown`: how pipe tables are read.
    pub table_mode: TableMode,
    /// Apply the pronunciation lexicon.
    pub use_pronunciations: bool,
    /// Pronunciation lexicon, `term -> spoken form` (whole terms,
    /// case-insensitive).
    pub pronunciations: BTreeMap<String, String>,
    /// Expand built-in abbreviations.
    pub abbreviations: bool,
    /// User abbreviations, `abbrev -> expansion` (case-sensitive; applied
    /// with `abbreviations`, also for engines that normalize natively).
    pub abbrev_expansions: BTreeMap<String, String>,
    /// Numbers, dates, times, and currency to words.
    pub numbers: bool,
    /// Speak math notation (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`, and ASCIIMath
    /// when `asciimath_delimiter` is set).
    pub math: bool,
    /// How explicit spoken math is: low ("a over b"), normal (ClearSpeak
    /// style, the default), or high (with end markers such as "end
    /// fraction").
    pub math_verbosity: Verbosity,
    /// The character around ASCIIMath (usually a backtick). `None`, the
    /// default, reads no ASCIIMath, because in Markdown a backtick is code.
    pub asciimath_delimiter: Option<char>,
    /// Which engine speaks math: textweaver's own (the default) or MathCAT
    /// (ADR-0029). MathCAT needs a build with the `mathcat` feature;
    /// without it, math is read by textweaver's own engine.
    pub math_engine: MathEngine,
    /// The document's language (a tag such as `fr` or `en-GB`), for
    /// MathCAT's rules. `None` is English. textweaver's own math speech
    /// is English only.
    pub math_language: Option<String>,
    /// The IBMTTS community pronunciation dictionaries as a lexicon, for
    /// engines that do not normalize natively (off by default; see
    /// [`community`]).
    pub community_lexicon: CommunityLexiconConfig,
    /// The medical lexicon: drug names, clinical terms and safe dosing
    /// abbreviations, with a per-user overlay (off by default; see
    /// [`medical`]).
    pub medical_lexicon: MedicalLexiconConfig,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        NormalizeConfig {
            markdown: false,
            skip_code: true,
            table_mode: TableMode::default(),
            use_pronunciations: true,
            pronunciations: BTreeMap::new(),
            abbreviations: true,
            abbrev_expansions: BTreeMap::new(),
            numbers: true,
            math: true,
            math_verbosity: Verbosity::Normal,
            asciimath_delimiter: None,
            math_engine: MathEngine::Builtin,
            math_language: None,
            community_lexicon: CommunityLexiconConfig::default(),
            medical_lexicon: MedicalLexiconConfig::default(),
        }
    }
}

impl NormalizeConfig {
    /// Everything off: spoken text equals source text (punctuation and split
    /// caps are separate service settings).
    pub fn none() -> Self {
        NormalizeConfig {
            markdown: false,
            skip_code: false,
            table_mode: TableMode::default(),
            use_pronunciations: false,
            pronunciations: BTreeMap::new(),
            abbreviations: false,
            abbrev_expansions: BTreeMap::new(),
            numbers: false,
            math: false,
            math_verbosity: Verbosity::Normal,
            asciimath_delimiter: None,
            math_engine: MathEngine::Builtin,
            math_language: None,
            community_lexicon: CommunityLexiconConfig::default(),
            medical_lexicon: MedicalLexiconConfig::default(),
        }
    }
}

/// An ordered chain of transforms.
#[derive(Default)]
pub struct Pipeline {
    transforms: Vec<Box<dyn Transform>>,
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.names()).finish()
    }
}

impl Pipeline {
    /// An empty pipeline (spoken text equals source text).
    pub fn new() -> Self {
        Self::default()
    }

    /// The standard chain for these settings (see the module docs).
    /// `native` is true for engines with
    /// [`Caps::NATIVE_NORMALIZATION`](crate::Caps::NATIVE_NORMALIZATION).
    pub fn for_settings(
        config: &NormalizeConfig,
        punctuation: PunctuationLevel,
        split_caps: bool,
        native: bool,
    ) -> Self {
        let mut p = Pipeline::new();
        if config.math {
            let math = Math::from_config(config).with_punctuation(punctuation);
            math.warm_up();
            p.push(Box::new(math));
        }
        if config.markdown {
            p.push(Box::new(MarkdownResidue::new(
                config.skip_code,
                config.table_mode,
            )));
        }
        if config.use_pronunciations && !config.pronunciations.is_empty() {
            p.push(Box::new(Pronunciations::new(&config.pronunciations)));
        }
        // After the user's own entries, so the student always wins.
        if let Some(l) = MedicalLexicon::from_config(&config.medical_lexicon) {
            p.push(Box::new(l));
        }
        if !native && let Some(l) = CommunityLexicon::from_config(&config.community_lexicon) {
            p.push(Box::new(l));
        }
        if config.abbreviations && !native {
            p.push(Box::new(Abbreviations::new(&config.abbrev_expansions)));
        } else if config.abbreviations {
            // The user's entries and the error-prone list: a native engine
            // must not guess "QD" or "MS" either.
            p.push(Box::new(Abbreviations::custom_only(
                &config.abbrev_expansions,
            )));
        }
        if config.numbers && !native {
            p.push(Box::new(Numbers::default()));
        } else if config.numbers {
            // A native engine reads numbers itself, but would still read
            // "PMID 31769816" as an amount.
            p.push(Box::new(Numbers::identifiers_only()));
        }
        if split_caps {
            p.push(Box::new(SplitCaps));
        }
        p.push(Box::new(Punctuation::new(punctuation)));
        p
    }

    /// Appends a transform.
    pub fn push(&mut self, t: Box<dyn Transform>) -> &mut Self {
        self.transforms.push(t);
        self
    }

    /// Names of the transforms, in order.
    pub fn names(&self) -> Vec<&'static str> {
        self.transforms.iter().map(|t| t.name()).collect()
    }

    /// Normalizes a string on its own, returning the text and the map from
    /// it to `text`'s chars.
    pub fn apply_text(&self, text: &str) -> (String, OffsetMap) {
        let mut acc: Option<(String, OffsetMap)> = None;
        for t in &self.transforms {
            let current = acc.as_ref().map_or(text, |a| a.0.as_str());
            let Some((out, map)) = t.apply_changed(current) else {
                continue;
            };
            acc = Some(match acc {
                // The first change maps straight to `text`.
                None => (out, map),
                Some((mid, first)) => (out, OffsetMap::compose(&first, &mid, &map)),
            });
        }
        acc.unwrap_or_else(|| identity(text))
    }

    /// Normalizes one utterance, composing each transform's map onto the
    /// utterance's map.
    pub fn apply(&self, mut u: Utterance) -> Utterance {
        for t in &self.transforms {
            // Unchanged text costs nothing: no copy, no composed map.
            let Some((text, map)) = t.apply_changed(&u.text) else {
                continue;
            };
            u.offset_map = if u.offset_map.is_empty() {
                // Announcements carry no source mapping; keep it that way.
                OffsetMap::default()
            } else {
                OffsetMap::compose(&u.offset_map, &u.text, &map)
            };
            u.text = text;
        }
        u
    }
}

/// The identity transform, useful in tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct Identity;

impl Transform for Identity {
    fn name(&self) -> &'static str {
        "identity"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        identity(input)
    }
}

/// Star's speak-time pipeline `_preprocess_tts_text(text, settings)`
/// (no punctuation step), with math moved first: math, lexicon,
/// abbreviations, numbers.
pub fn preprocess(text: &str, config: &NormalizeConfig) -> String {
    let mut p = Pipeline::new();
    if config.math {
        p.push(Box::new(Math::from_config(config)));
    }
    if config.use_pronunciations && !config.pronunciations.is_empty() {
        p.push(Box::new(Pronunciations::new(&config.pronunciations)));
    }
    if config.abbreviations {
        p.push(Box::new(Abbreviations::new(&config.abbrev_expansions)));
    }
    if config.numbers {
        p.push(Box::new(Numbers::default()));
    }
    p.apply_text(text).0
}

#[cfg(test)]
mod tests;
