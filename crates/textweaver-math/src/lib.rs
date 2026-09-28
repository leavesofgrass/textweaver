//! Math for textweaver (ADR-0018): LaTeX and ASCIIMath parsed into one math
//! tree, written as MathML for HTML, spoken as natural English
//! (ClearSpeak-style wording at three verbosity levels) with offset maps
//! back to the source, navigable by term, fraction part, and script, and
//! found in plain text without mistaking prices for math.
//!
//! # Entry points
//!
//! - [`parse_latex`] and [`parse_asciimath`] (or [`parse`]): source to a
//!   [`Math`] tree. Parsing is total: it never fails or panics, and records
//!   what it recovered from in [`Math::diagnostics`].
//! - [`speak`]: a [`Math`] to [`Spoken`] text with an
//!   [`OffsetMap`] (ADR-0005).
//! - [`speak_text`]: the normalization transform for the speech pipeline:
//!   finds math in plain text and replaces each region with its spoken
//!   form, leaving everything else literal.
//! - [`to_mathml`], [`latex_to_mathml`], [`asciimath_to_mathml`]:
//!   presentation MathML with `alttext` for the HTML renderer.
//! - [`Navigator`]: moves through an expression part by part.
//! - [`find_math`]: math regions in plain text.
//!
//! ```
//! use textweaver_core::Verbosity;
//! use textweaver_math::{SpeechOptions, parse_latex, speak};
//!
//! let math = parse_latex(r"\frac{a}{b} + x^2");
//! let spoken = speak(&math, &SpeechOptions::new(Verbosity::Normal));
//! assert_eq!(spoken.text, "a over b plus x squared");
//! spoken.map.check_invariants(&spoken.text).unwrap();
//! ```

mod asciimath;
mod build;
mod detect;
mod error;
mod latex;
mod mathml;
mod nav;
mod speech;
mod symbols;
mod tree;
mod unicode;

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, OffsetMap, SpokenBuilder};

pub use asciimath::parse_asciimath;
pub use detect::{Delimiter, DetectOptions, MathRegion, find_math};
pub use error::MathError;
pub use latex::parse_latex;
pub use mathml::{AltText, MathMlOptions, to_mathml};
pub use nav::{NavStep, Navigator, Role};
pub use speech::{SpeechOptions, Spoken, speak, speak_node};
pub use tree::{
    AccentKind, Diagnostic, Enclosure, Math, Node, NodeKind, Notation, OpClass, TableKind, Variant,
};
pub use unicode::to_unicode;

/// Parses `src` in the given notation.
pub fn parse(src: &str, notation: Notation) -> Math {
    match notation {
        Notation::Latex => parse_latex(src),
        Notation::AsciiMath => parse_asciimath(src),
    }
}

/// LaTeX math to MathML, inline or display, with the source as `alttext`
/// and annotation. This is the call for a Markdown renderer's math events.
pub fn latex_to_mathml(src: &str, display: bool) -> String {
    to_mathml(&parse_latex(src), &MathMlOptions::new(display))
}

/// ASCIIMath to MathML, inline or display.
pub fn asciimath_to_mathml(src: &str, display: bool) -> String {
    to_mathml(&parse_asciimath(src), &MathMlOptions::new(display))
}

/// Options for [`speak_text`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextOptions {
    /// Which delimiters mark math.
    pub detect: DetectOptions,
    /// How math is spoken.
    pub speech: SpeechOptions,
}

/// Speaks the math in `text`, leaving everything else unchanged.
///
/// Each math region found by [`find_math`] is replaced with its spoken form;
/// its delimiters become elided spans; the text around it stays literal.
/// The returned map points from the output's bytes to `text`'s chars, as a
/// normalization transform's must (ADR-0005). Text without math comes back
/// unchanged with an identity map.
pub fn speak_text(text: &str, opts: &TextOptions) -> (String, OffsetMap) {
    speak_text_with(text, opts, &mut |_, _| None)
}

/// [`speak_text`] with another math speech engine asked first.
///
/// For each region, `speaker` gets the region and its parsed tree. When it
/// returns words, they replace the region's content as one expanded span
/// (ADR-0005 allows a span-level map: any word of it highlights the whole
/// formula). When it returns `None`, or only blanks, the region is spoken
/// by this crate, word by word, as [`speak_text`] does. MathCAT is such an
/// engine (ADR-0029).
pub fn speak_text_with(
    text: &str,
    opts: &TextOptions,
    speaker: &mut dyn FnMut(&MathRegion, &Math) -> Option<String>,
) -> (String, OffsetMap) {
    let chars: Vec<char> = text.chars().collect();
    let regions = detect::find_math_chars(&chars, &opts.detect);
    if regions.is_empty() {
        return (text.to_owned(), OffsetMap::identity(text, CharPos::ZERO));
    }
    let slice = |a: usize, b: usize| -> String { chars[a..b].iter().collect() };
    let mut b = SpokenBuilder::new();
    let mut at = 0usize;
    for r in regions {
        let (start, end) = (r.range.start.0, r.range.end.0);
        if start > at {
            b.push_literal(&slice(at, start), CharPos(at));
        }
        b.push_elided(CharRange::new(start, r.content.start));
        let math = parse(&slice(r.content.start.0, r.content.end.0), r.notation);
        // Keep the math a separate word when it touches a letter or digit.
        if start > 0 && chars[start - 1].is_alphanumeric() {
            b.push_inserted(" ", r.content.start);
        }
        let other = speaker(&r, &math)
            .map(|w| w.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|w| !w.is_empty());
        let spoke = match other {
            Some(words) => {
                b.push_expanded(&words, r.content);
                true
            }
            None => {
                speech::speak_node_into(&math, &math.root, &opts.speech, &mut b, r.content.start.0)
            }
        };
        if !spoke {
            b.push_elided(r.content);
        } else if chars.get(end).is_some_and(|c| c.is_alphanumeric()) {
            b.push_inserted(" ", r.content.end);
        }
        b.push_elided(CharRange::new(r.content.end, end));
        at = end;
    }
    if at < chars.len() {
        b.push_literal(&slice(at, chars.len()), CharPos(at));
    }
    b.finish()
}
