//! Math: delimited LaTeX and ASCIIMath spoken by `textweaver-math`
//! (ADR-0018), replacing Star's regular expressions
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
//! After the math regions, a few operator symbols that Star read in prose
//! are still spoken as words (`×` "times", `≤` "less than or equal to",
//! `→` "approaches"), because engines drop them at low punctuation levels.
//!
//! Differences from Star, all deliberate (ADR-0018): math must be delimited,
//! as it is in documents, so `x^2 and y^{3}` in prose stays as written while
//! `$x^2$ and $y^{3}$` is "x squared and y cubed"; `\alpha + \beta` is
//! "alpha plus beta" (Star left `+` to the engine); `\bar{x}` is "x bar"
//! (Star: "x-bar"). Star's bugs Q6 to Q8 (`snake_case`, trailing `x^2`,
//! global brace stripping, `$5 and $10`) cannot occur, since only
//! delimited regions are touched.

use textweaver_core::{OffsetMap, Verbosity};
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

/// The math transform.
pub struct Math {
    options: TextOptions,
    symbols: Vec<Rule>,
}

impl std::fmt::Debug for Math {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Math")
            .field("options", &self.options)
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
    }

    /// Math with every detection and speech option given.
    pub fn with_options(options: TextOptions) -> Self {
        Math {
            options,
            symbols: symbol_rules(),
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
        let mut acc = textweaver_math::speak_text(input, &self.options);
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
