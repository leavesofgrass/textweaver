//! Math: delimited LaTeX and ASCIIMath spoken by `textweaver-math`
//! (ADR-0018), replacing star's regular expressions
//! (`star/ttstext/mathspeech.py`).
//!
//! The transform calls [`textweaver_math::speak_text`], which finds math
//! regions (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`, and, when a delimiter is set,
//! ASCIIMath such as `` `x^2` ``), parses each into a math tree, and replaces
//! it with ClearSpeak-style English at the chosen verbosity. Everything
//! outside a region stays literal, so prose is never treated as math.
//! Spoken words map back to the part of the formula they came from, and the
//! delimiters are elided spans, so the highlight follows the formula while
//! it is read.
//!
//! The transform runs early in the pipeline, before abbreviations and
//! numbers: `Numbers` reads `$2` as currency, which would destroy `$2x$`,
//! and math speech leaves digits literal for `Numbers` to read afterwards.
//!
//! After the math regions, a few operator symbols that star read in prose
//! are still spoken as words (`×` "times", `≤` "less than or equal to",
//! `→` "approaches"), because engines drop them at low punctuation levels.
//!
//! Differences from star, all deliberate (ADR-0018): math must be delimited,
//! as it is in documents, so `x^2 and y^{3}` in prose stays as written while
//! `$x^2$ and $y^{3}$` is "x squared and y cubed"; `\alpha + \beta` is
//! "alpha plus beta" (star left `+` to the engine); `\bar{x}` is "x bar"
//! (star: "x-bar"). star's bugs Q6 to Q8 (`snake_case`, trailing `x^2`,
//! global brace stripping, `$5 and $10`) cannot occur, since only
//! delimited regions are touched.
//!
//! **MathCAT** (ADR-0029). With the `mathcat` feature and `math_engine`
//! set to MathCAT, each formula is spoken by MathCAT in ClearSpeak or
//! SimpleSpeak, at the same verbosity, in the document's language. Its
//! words replace the whole formula as one expanded span, so the highlight
//! covers the formula while it is read. A formula MathCAT cannot speak is
//! read by `textweaver-math` as above. Without the feature, the setting
//! changes nothing.

use serde::{Deserialize, Serialize};
use textweaver_core::{OffsetMap, PunctuationLevel, Verbosity};
use textweaver_math::{DetectOptions, SpeechOptions, TextOptions};

use super::rewrite::{Rule, then};
use super::{NormalizeConfig, Transform};

/// Operator symbols spoken in prose, outside math regions.
const SYMBOLS: [(&str, &str); 9] = [
    ("×", "times"),
    ("÷", "divided by"),
    ("≤", "less than or equal to"),
    ("≥", "greater than or equal to"),
    ("≈", "approximately equal to"),
    ("≠", "not equal to"),
    ("∞", "infinity"),
    ("→", "approaches"),
    ("←", "from"),
];

fn symbol_rules() -> Vec<Rule> {
    SYMBOLS
        .iter()
        .map(|(symbol, spoken)| Rule::new(&regex::escape(symbol), &format!(" {spoken} ")).padded())
        .collect()
}

/// Which engine speaks math (`[reading] math_engine`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MathEngine {
    /// textweaver's own ClearSpeak-style speech (`textweaver-math`).
    #[default]
    #[serde(rename = "builtin")]
    Builtin,
    /// MathCAT in ClearSpeak.
    #[serde(rename = "mathcat")]
    MathCatClearSpeak,
    /// MathCAT in SimpleSpeak.
    #[serde(rename = "mathcat_simplespeak")]
    MathCatSimpleSpeak,
}

/// Whether this build can speak math with MathCAT (the `mathcat`
/// feature).
pub fn mathcat_available() -> bool {
    cfg!(feature = "mathcat")
}

/// The math transform.
pub struct Math {
    options: TextOptions,
    symbols: Vec<Rule>,
    engine: MathEngine,
    language: Option<String>,
    /// Keep MathCAT's pause commas (off when punctuation is read aloud).
    pauses: bool,
}

impl std::fmt::Debug for Math {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Math")
            .field("options", &self.options)
            .field("engine", &self.engine)
            .field("language", &self.language)
            .finish_non_exhaustive()
    }
}

impl Default for Math {
    /// Normal verbosity; `$…$`, `$$…$$`, `\(…\)`, and `\[…\]`; no ASCIIMath.
    fn default() -> Self {
        Math::new(Verbosity::Normal, None)
    }
}

impl Math {
    /// Math spoken at `verbosity`, with ASCIIMath recognized between
    /// `asciimath` delimiters (usually a backtick) when one is given.
    pub fn new(verbosity: Verbosity, asciimath: Option<char>) -> Self {
        Math::with_options(TextOptions {
            detect: DetectOptions {
                asciimath,
                ..DetectOptions::default()
            },
            speech: SpeechOptions::new(verbosity),
        })
    }

    /// Math as the normalization settings describe it (`math_verbosity`,
    /// `asciimath_delimiter`).
    pub fn from_config(config: &NormalizeConfig) -> Self {
        Math::new(config.math_verbosity, config.asciimath_delimiter)
            .with_engine(config.math_engine, config.math_language.clone())
    }

    /// The same math, spoken by `engine` in `language` (a document
    /// language tag; `None` is English). MathCAT needs the `mathcat`
    /// feature; without it this changes nothing.
    pub fn with_engine(mut self, engine: MathEngine, language: Option<String>) -> Self {
        self.engine = engine;
        self.language = language;
        self
    }

    /// The same math for a punctuation level: at `All`, where every mark
    /// is read by name, MathCAT's pause commas and semicolons are left out.
    pub fn with_punctuation(mut self, level: PunctuationLevel) -> Self {
        self.pauses = level != PunctuationLevel::All;
        self
    }

    /// The engine in use: MathCAT only when asked for and built in.
    pub fn engine(&self) -> MathEngine {
        if mathcat_available() {
            self.engine
        } else {
            MathEngine::Builtin
        }
    }

    /// Starts MathCAT and loads its rules in the background when it is the
    /// engine, so the first formula does not wait. Otherwise nothing.
    pub fn warm_up(&self) {
        #[cfg(feature = "mathcat")]
        if self.engine() != MathEngine::Builtin {
            textweaver_mathcat::warm_up();
        }
    }

    #[cfg(feature = "mathcat")]
    fn mathcat_options(&self) -> Option<textweaver_mathcat::Options> {
        let style = match self.engine {
            MathEngine::Builtin => return None,
            MathEngine::MathCatClearSpeak => textweaver_mathcat::Style::ClearSpeak,
            MathEngine::MathCatSimpleSpeak => textweaver_mathcat::Style::SimpleSpeak,
        };
        Some(textweaver_mathcat::Options {
            style,
            verbosity: self.options.speech.verbosity,
            language: textweaver_mathcat::language_for(self.language.as_deref()),
            pauses: self.pauses,
        })
    }

    /// The math regions of `input` spoken by the engine in use.
    fn speak(&self, input: &str) -> (String, OffsetMap) {
        #[cfg(feature = "mathcat")]
        if let Some(o) = self.mathcat_options() {
            return textweaver_mathcat::speak_text(input, &self.options, &o);
        }
        textweaver_math::speak_text(input, &self.options)
    }

    /// Math with every detection and speech option given.
    pub fn with_options(options: TextOptions) -> Self {
        Math {
            options,
            symbols: symbol_rules(),
            engine: MathEngine::Builtin,
            language: None,
            pauses: true,
        }
    }

    /// The options in use.
    pub fn options(&self) -> &TextOptions {
        &self.options
    }
}

impl Transform for Math {
    fn name(&self) -> &'static str {
        "math"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        let mut acc = self.speak(input);
        if acc.0.contains(|c: char| !c.is_ascii()) {
            for r in &self.symbols {
                let step = r.apply(&acc.0);
                acc = then(acc, step);
            }
        }
        acc
    }
}

/// The math transform with default settings, on a string.
pub fn normalize_math(text: &str) -> String {
    Math::default().apply(text).0
}
