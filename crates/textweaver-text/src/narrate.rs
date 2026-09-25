//! Narration planning: which utterances read a range aloud (ADR-0005).
//!
//! `plan` splits a range into sentence-sized utterances with offset maps.
//! Structure is spoken as `Inserted` spans ("heading level 2", table
//! coordinates) according to the policy. Normalization (numbers,
//! abbreviations, punctuation) is not done here: the speech service applies
//! its transform chain per utterance and composes the maps.

use serde::{Deserialize, Serialize};
use textweaver_core::{CharRange, Unit, Utterance, UtteranceId, Verbosity};

use crate::Document;
use crate::units::segments;

/// How a range is turned into utterances.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrationPolicy {
    /// Upper bound on one utterance's length in chars; longer sentences are
    /// split at whitespace.
    pub max_chunk_chars: usize,
    /// Announce structure ("heading level 2") before marked ranges.
    pub announce_structure: bool,
    /// Skip text under `Code` markers (Star's `skip_code`).
    pub skip_code: bool,
    /// Verbosity of structure announcements.
    pub verbosity: Verbosity,
}

impl Default for NarrationPolicy {
    fn default() -> Self {
        NarrationPolicy {
            max_chunk_chars: 400,
            announce_structure: true,
            skip_code: false,
            verbosity: Verbosity::Normal,
        }
    }
}

/// Sentence-sized utterances covering `range`, numbered from chunk 0.
///
/// Phase 0: one literal utterance per sentence intersecting the range, no
/// structure announcements. Agent A adds headings, tables, skipping, and
/// long-sentence splitting.
pub fn plan(doc: &Document, range: CharRange, policy: &NarrationPolicy) -> Vec<Utterance> {
    let _ = policy;
    segments(doc, Unit::Sentence)
        .into_iter()
        .filter_map(|s| s.intersection(range))
        .enumerate()
        .map(|(i, r)| {
            let mut u = Utterance::literal(doc.slice(r), r.start);
            u.id = UtteranceId {
                generation: 0,
                chunk: u32::try_from(i).unwrap_or(u32::MAX),
            };
            u
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use textweaver_core::CharPos;

    use super::*;

    #[test]
    fn plans_one_utterance_per_sentence() {
        let d = Document::from_plain_text("One two. Three four.");
        let us = plan(
            &d,
            CharRange::new(0, d.len_chars()),
            &NarrationPolicy::default(),
        );
        assert_eq!(us.len(), 2);
        assert_eq!(us[1].text, "Three four.");
        assert_eq!(us[1].source_range(), Some(CharRange::new(9, 20)));
        assert_eq!(
            us[1].source_for(0..5),
            Some(CharRange::new(CharPos(9), CharPos(14)))
        );
    }
}
