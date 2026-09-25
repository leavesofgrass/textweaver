//! Text normalization with offset maps (ADR-0005).
//!
//! Each [`Transform`] turns its input into spoken text and an
//! [`OffsetMap`] from its output bytes to its input chars. A [`Pipeline`]
//! applies transforms to one utterance at a time (never across chunks) and
//! composes the maps so the result still points into the document.
//!
//! Agent B ports Star's chain (`star/ttstext/`): Markdown residue,
//! abbreviations, numbers, dates, times, currency, math, punctuation
//! verbosity, split caps, with the expected strings from
//! `tests/test_ttstext.py`.

use textweaver_core::{CharPos, OffsetMap, Utterance};

/// One normalization step.
pub trait Transform: Send + Sync {
    /// Stable name, used in settings toggles and diagnostics.
    fn name(&self) -> &'static str;
    /// Returns the transformed text and a map from its bytes to `input` chars
    /// (positions counted from 0 within `input`).
    fn apply(&self, input: &str) -> (String, OffsetMap);
}

/// An ordered chain of transforms.
#[derive(Default)]
pub struct Pipeline {
    transforms: Vec<Box<dyn Transform>>,
}

impl Pipeline {
    /// An empty pipeline (spoken text equals source text).
    pub fn new() -> Self {
        Self::default()
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

    /// Normalizes one utterance, composing each transform's map onto the
    /// utterance's map.
    pub fn apply(&self, mut u: Utterance) -> Utterance {
        for t in &self.transforms {
            let (text, map) = t.apply(&u.text);
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
        (input.to_owned(), OffsetMap::identity(input, CharPos::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::{CharRange, SpokenBuilder};

    use super::*;

    /// Expands "Dr." to "Doctor" wherever it starts a token.
    struct Doctor;

    impl Transform for Doctor {
        fn name(&self) -> &'static str {
            "doctor"
        }
        fn apply(&self, input: &str) -> (String, OffsetMap) {
            let mut b = SpokenBuilder::new();
            let mut pos = 0;
            for (i, tok) in input.split(' ').enumerate() {
                if i > 0 {
                    b.push_literal(" ", CharPos(pos));
                    pos += 1;
                }
                let n = tok.chars().count();
                if tok == "Dr." {
                    b.push_expanded("Doctor", CharRange::new(pos, pos + n));
                } else {
                    b.push_literal(tok, CharPos(pos));
                }
                pos += n;
            }
            b.finish()
        }
    }

    #[test]
    fn pipeline_keeps_document_positions() {
        let mut p = Pipeline::new();
        p.push(Box::new(Identity)).push(Box::new(Doctor));
        let u = p.apply(Utterance::literal("Ask Dr. Lee", CharPos(100)));
        assert_eq!(u.text, "Ask Doctor Lee");
        u.offset_map.check_invariants(&u.text).unwrap();
        assert_eq!(u.source_for(4..10), Some(CharRange::new(104, 107)));
        assert_eq!(u.source_for(11..14), Some(CharRange::new(108, 111)));
    }
}
