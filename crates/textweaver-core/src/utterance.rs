use serde::{Deserialize, Serialize};

use crate::{CharPos, CharRange, OffsetMap};

/// Identifies one chunk of speech.
///
/// `generation` is bumped by the speech service before every stop or restart
/// (Star's rule); events from an older generation are dropped. `chunk` numbers
/// the utterances within one reading request, starting at 0.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct UtteranceId {
    /// Reading generation, assigned by the speech service.
    pub generation: u64,
    /// Chunk index within the reading request.
    pub chunk: u32,
}

/// What an utterance is for, which affects how the speech service treats it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UtteranceKind {
    /// Document text being read; drives the reading highlight.
    #[default]
    Text,
    /// A single character spoken on its own (scaled rate, caps indication).
    Character,
    /// A status announcement; never moves the reading position.
    Announcement,
}

/// One sentence-sized chunk of text handed to the speech service.
///
/// `text` is what the engine speaks, already normalized. `offset_map` maps
/// byte ranges of `text` back to char ranges of the source document, so the
/// highlight follows the source even when the spoken text differs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Utterance {
    /// Identity, stamped by the speech service.
    pub id: UtteranceId,
    /// The text to speak.
    pub text: String,
    /// What the utterance is for.
    pub kind: UtteranceKind,
    /// Spoken-bytes to source-chars mapping. Empty for announcements.
    pub offset_map: OffsetMap,
}

impl Utterance {
    /// A text utterance whose spoken text is identical to the source text
    /// starting at `source_start`.
    pub fn literal(text: impl Into<String>, source_start: CharPos) -> Self {
        let text = text.into();
        let offset_map = OffsetMap::identity(&text, source_start);
        Utterance {
            id: UtteranceId::default(),
            text,
            kind: UtteranceKind::Text,
            offset_map,
        }
    }

    /// A text utterance with an explicit offset map.
    pub fn with_map(text: impl Into<String>, offset_map: OffsetMap) -> Self {
        Utterance {
            id: UtteranceId::default(),
            text: text.into(),
            kind: UtteranceKind::Text,
            offset_map,
        }
    }

    /// An announcement: spoken, never mapped to the document.
    pub fn announcement(text: impl Into<String>) -> Self {
        Utterance {
            id: UtteranceId::default(),
            text: text.into(),
            kind: UtteranceKind::Announcement,
            offset_map: OffsetMap::default(),
        }
    }

    /// A single character spoken on its own, mapped to `at` when it comes from
    /// the document.
    pub fn character(text: impl Into<String>, at: Option<CharPos>) -> Self {
        let text = text.into();
        let offset_map = match at {
            Some(pos) => OffsetMap::identity(&text, pos),
            None => OffsetMap::default(),
        };
        Utterance {
            id: UtteranceId::default(),
            text,
            kind: UtteranceKind::Character,
            offset_map,
        }
    }

    /// The source range this utterance covers, if it maps to the document.
    pub fn source_range(&self) -> Option<CharRange> {
        self.offset_map.source_extent()
    }

    /// Source range for a byte range of the spoken text (see
    /// [`OffsetMap::to_source`]).
    pub fn source_for(&self, bytes: std::ops::Range<u32>) -> Option<CharRange> {
        self.offset_map.to_source(&self.text, bytes)
    }
}
