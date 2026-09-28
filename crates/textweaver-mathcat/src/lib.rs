//! Math speech through MathCAT (ADR-0029): ClearSpeak and SimpleSpeak, the
//! wording NVDA and JAWS use, with `textweaver-math` as the fallback.
//!
//! MathCAT (DAISY; MIT) turns MathML into spoken words. textweaver keeps
//! its own math tree for everything else (parsing LaTeX and ASCIIMath,
//! MathML for the renderer, navigation, and the word-by-word offset maps of
//! ADR-0018), so this crate is a speech engine only:
//!
//! - [`speak_mathml`] speaks one MathML expression.
//! - [`speak_text`] is the normalization transform's body: it finds math in
//!   plain text as `textweaver_math::speak_text` does, writes each formula
//!   as MathML, and asks MathCAT for its words. Those words replace the
//!   whole formula as one expanded span, so the highlight covers the whole
//!   formula while any of it is spoken (a span-level map, which ADR-0005
//!   allows). A formula MathCAT cannot speak, or one the parser had to
//!   repair, is spoken by `textweaver-math` as before, word by word.
//!
//! # One thread
//!
//! MathCAT keeps its state (rules, preferences, the current expression) in
//! thread-local variables, so every call runs on one dedicated thread,
//! started on first use and named `textweaver-mathcat`. Callers wait for
//! the answer with a timeout. A panic inside MathCAT is caught on that
//! thread, logged once, and answered with [`Error::Crashed`]; the thread
//! keeps serving, and the caller falls back to textweaver's own speech.
//!
//! # Rules
//!
//! MathCAT's rules (about 9.4 MB of YAML, all languages and braille codes)
//! are embedded in the binary through MathCAT's `include-zip` feature,
//! zipped at build time from the crate's own `Rules` folder. Nothing is
//! downloaded, and no rule file is read from disk.
//!
//! ```no_run
//! use textweaver_mathcat::{Options, speak_mathml};
//!
//! let words = speak_mathml("<math><msup><mi>x</mi><mn>2</mn></msup></math>", &Options::default())?;
//! assert_eq!(words, "x squared");
//! # Ok::<(), textweaver_mathcat::Error>(())
//! ```

mod engine;

use std::fmt;

use textweaver_core::{OffsetMap, Verbosity};
use textweaver_math::{AltText, MathMlOptions, TextOptions, to_mathml};

/// MathCAT's version, as its crate reports it (`0.7.6-rc.3`).
pub fn version() -> String {
    libmathcat::get_version()
}

/// The MathCAT speech style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Style {
    /// ClearSpeak: the wording of the US ClearSpeak specification, used in
    /// classrooms and on tests. MathCAT's default.
    #[default]
    ClearSpeak,
    /// SimpleSpeak: shorter, with end words only where the structure would
    /// otherwise be ambiguous.
    SimpleSpeak,
}

impl Style {
    /// MathCAT's name for the style (the `SpeechStyle` preference).
    pub fn mathcat_name(self) -> &'static str {
        match self {
            Style::ClearSpeak => "ClearSpeak",
            Style::SimpleSpeak => "SimpleSpeak",
        }
    }
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mathcat_name())
    }
}

/// MathCAT's `Verbosity` preference for textweaver's three levels: low is
/// `Terse`, normal is `Medium`, high is `Verbose`.
pub fn mathcat_verbosity(v: Verbosity) -> &'static str {
    match v {
        Verbosity::Low => "Terse",
        Verbosity::Normal => "Medium",
        Verbosity::High => "Verbose",
    }
}

/// Languages MathCAT 0.7.6 has speech rules for (its `Rules/Languages`
/// folders, less the `zz` test language).
pub const LANGUAGES: [&str; 14] = [
    "de", "el", "en", "es", "fi", "fr", "hu", "id", "nb", "pl", "ru", "sv", "vi", "zh",
];

/// The MathCAT language for a document's language tag (`fr`, `fr-CA`,
/// `pt_BR`): its primary subtag when MathCAT has rules for it, else
/// English. `no` (Norwegian) is read as `nb` (Bokmål).
pub fn language_for(tag: Option<&str>) -> &'static str {
    let primary = tag
        .unwrap_or_default()
        .trim()
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let primary = if primary == "no" { "nb".into() } else { primary };
    LANGUAGES
        .iter()
        .find(|l| **l == primary)
        .copied()
        .unwrap_or("en")
}

/// How MathCAT speaks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Options {
    /// ClearSpeak or SimpleSpeak.
    pub style: Style,
    /// textweaver's math verbosity, mapped by [`mathcat_verbosity`].
    pub verbosity: Verbosity,
    /// A MathCAT language code (see [`language_for`]).
    pub language: &'static str,
    /// Keep the commas and semicolons MathCAT puts in for pauses. Off when
    /// every punctuation mark is spoken by name, so a formula is not read
    /// with "comma" and "semicolon" in it.
    pub pauses: bool,
}

impl Default for Options {
    /// ClearSpeak, normal verbosity, English, with pauses.
    fn default() -> Self {
        Options {
            style: Style::ClearSpeak,
            verbosity: Verbosity::Normal,
            language: "en",
            pauses: true,
        }
    }
}

/// Why MathCAT gave no words.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// MathCAT's rules could not be loaded.
    #[error("MathCAT's rules could not be loaded: {0}")]
    Rules(String),
    /// The MathML was refused: not well formed, too large, or nested too
    /// deeply.
    #[error("MathCAT could not read this math: {0}")]
    Rejected(String),
    /// MathCAT panicked on this expression. The thread survives.
    #[error("MathCAT stopped on an internal error")]
    Crashed,
    /// MathCAT did not answer in time.
    #[error("MathCAT did not answer in time")]
    Timeout,
    /// The MathCAT thread could not be started.
    #[error("the MathCAT thread is not running")]
    Unavailable,
}

/// The longest MathML accepted, in bytes. MathCAT's own limit is 1 MB; an
/// ordinary formula is well under a kilobyte.
pub const MAX_MATHML_BYTES: usize = 256 * 1024;

/// The deepest element nesting accepted. textweaver's own MathML stops at
/// 48 levels (ADR-0018); MathCAT recurses over the tree, so hostile input
/// is refused here, before it reaches MathCAT's parser.
pub const MAX_MATHML_DEPTH: usize = 128;

/// The most elements accepted. MathCAT's time grows with the square of a
/// row's length (measured on 0.7.6-rc.3 in a release build: 800 terms
/// took 0.74 seconds, 400 took 0.21); an ordinary formula has well under a
/// hundred elements. Longer ones are read by textweaver's own speech.
pub const MAX_MATHML_ELEMENTS: usize = 1000;

/// Speaks one MathML expression (a `<math>` element).
///
/// Runs on the MathCAT thread and waits for the answer. The words have
/// single spaces and no leading or trailing space.
pub fn speak_mathml(mathml: &str, options: &Options) -> Result<String, Error> {
    check_mathml(mathml)?;
    engine::speak(mathml, options)
}

/// Starts the MathCAT thread and loads its rules without waiting, so the
/// first formula read does not wait for them.
pub fn warm_up() {
    engine::warm_up();
}

/// Refuses MathML that is too large, too long, or nested too deeply, by a
/// cheap scan of its tags (comments, processing instructions, and
/// declarations are not counted as elements).
fn check_mathml(mathml: &str) -> Result<(), Error> {
    if mathml.len() > MAX_MATHML_BYTES {
        return Err(Error::Rejected(format!(
            "the MathML is {} bytes, more than the {MAX_MATHML_BYTES} allowed",
            mathml.len()
        )));
    }
    let bytes = mathml.as_bytes();
    let mut depth = 0usize;
    let mut elements = 0usize;
    let mut i = 0;
    while let Some(off) = bytes[i..].iter().position(|&b| b == b'<') {
        let at = i + off;
        let next = bytes.get(at + 1).copied();
        let end = bytes[at..]
            .iter()
            .position(|&b| b == b'>')
            .map_or(bytes.len(), |e| at + e);
        if !matches!(next, Some(b'/' | b'!' | b'?')) {
            elements += 1;
            if elements > MAX_MATHML_ELEMENTS {
                return Err(Error::Rejected(format!(
                    "the MathML has more than {MAX_MATHML_ELEMENTS} elements"
                )));
            }
        }
        match next {
            Some(b'/') => depth = depth.saturating_sub(1),
            Some(b'!' | b'?') => {}
            _ if end > at && bytes[end - 1] == b'/' => {}
            _ => {
                depth += 1;
                if depth > MAX_MATHML_DEPTH {
                    return Err(Error::Rejected(format!(
                        "the MathML is nested more than {MAX_MATHML_DEPTH} levels deep"
                    )));
                }
            }
        }
        i = (end + 1).min(bytes.len());
        if i >= bytes.len() {
            break;
        }
    }
    Ok(())
}

/// Speaks the math in `text` through MathCAT, leaving everything else
/// unchanged: the normalization transform's body when the math engine is
/// MathCAT.
///
/// Math is found by `text_options.detect` as `textweaver_math::speak_text`
/// finds it. Each formula MathCAT speaks becomes one expanded span over
/// the formula's content (its delimiters are elided); a formula MathCAT
/// refuses, or one the parser had to repair, is spoken by
/// `textweaver-math` at `text_options.speech`. The map points from the
/// output's bytes to `text`'s chars, as every transform's must (ADR-0005).
pub fn speak_text(text: &str, text_options: &TextOptions, options: &Options) -> (String, OffsetMap) {
    textweaver_math::speak_text_with(text, text_options, &mut |region, math| {
        if !math.diagnostics.is_empty() {
            return None;
        }
        let mathml = to_mathml(
            math,
            &MathMlOptions {
                display: region.display,
                alttext: AltText::None,
                annotation: false,
            },
        );
        match speak_mathml(&mathml, options) {
            Ok(words) => Some(words),
            Err(e) => {
                log::debug!("MathCAT gave no words for {:?}: {e}", math.source);
                None
            }
        }
    })
}

#[cfg(test)]
mod tests;
