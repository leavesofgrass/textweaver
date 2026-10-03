//! Pauses at the ends of headings, paragraphs and list items.
//!
//! Listeners at high rates lose a document's structure first: a heading
//! runs into the paragraph under it, one list item into the next. A short
//! silence at those boundaries keeps the structure audible without any
//! words, and without any engine's help:
//!
//! - Engines whose audio textweaver plays (SAPI, Eloquence, DECtalk, Piper:
//!   [`Caps::SILENCE`](crate::Caps::SILENCE)) get the silence as zero
//!   samples in the playback client's feed, after the utterance that ends
//!   the block ([`SpeechBackend::silence_after`](crate::SpeechBackend::silence_after)).
//!   The next utterance's audio is synthesized ahead as usual, so a pause
//!   costs no synthesis time, and its words are timed from its own first
//!   sample, so the offset map and the highlight are untouched.
//! - Engines that play their own audio (Apple, Speech Dispatcher, eSpeak
//!   NG) get a timed gap instead: the queue holds the next utterance back
//!   until the one before it has finished and the pause has passed on the
//!   playback clock ([`ReadingQueue`](crate::queue::ReadingQueue)).
//!
//! Stop, Pause and skipping cut a pause at once: the silence is audio in the
//! feed, which a stop clears and a pause holds, and the gap is queue state,
//! which every stop, restart and skip drops.
//!
//! The caller, who has the document, says where blocks end
//! ([`PauseAt`], computed by `textweaver_text::narrate::block_ends`); the
//! lengths come from `[speech]` settings ([`PauseConfig`]). A pause is keyed
//! by where its utterance ends in the document, which a resume that trims
//! the utterance's start does not change.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, Rate, Utterance, UtteranceKind};

/// Which block ends after an utterance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseKind {
    /// A heading.
    Heading,
    /// A paragraph (or another block separated by a blank line).
    Paragraph,
    /// A list item.
    ListItem,
}

/// A pause after the utterance that ends at `after` in the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PauseAt {
    /// Where the utterance ends in the document (its source range's end).
    pub after: CharPos,
    /// Which block ends there.
    pub kind: PauseKind,
}

/// The longest pause, in milliseconds. Longer settings are clamped to it,
/// so a pause can never look like a stalled engine.
pub const MAX_PAUSE_MS: u32 = 3000;

/// The rate at which pauses have their full length: Star's and textweaver's
/// default rate. Faster rates shorten them in proportion.
pub const FULL_LENGTH_WPM: u16 = 265;

/// The shortest a pause gets at a high rate, as a fraction of its length.
pub const MIN_RATE_SCALE: f32 = 0.25;

/// Pause lengths by structure, in milliseconds at the default rate
/// (`[speech] pause_heading_ms`, `pause_paragraph_ms`,
/// `pause_list_item_ms`). Zero turns that pause off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseConfig {
    /// After a heading.
    pub heading_ms: u32,
    /// After a paragraph.
    pub paragraph_ms: u32,
    /// After a list item.
    pub list_item_ms: u32,
}

impl Default for PauseConfig {
    fn default() -> Self {
        PauseConfig {
            heading_ms: 400,
            paragraph_ms: 300,
            list_item_ms: 150,
        }
    }
}

impl PauseConfig {
    /// No pauses at all.
    pub const OFF: PauseConfig = PauseConfig {
        heading_ms: 0,
        paragraph_ms: 0,
        list_item_ms: 0,
    };

    /// True when every pause is off.
    pub fn is_off(&self) -> bool {
        self.heading_ms == 0 && self.paragraph_ms == 0 && self.list_item_ms == 0
    }

    /// The configured length for `kind`, in ms, clamped to
    /// [`MAX_PAUSE_MS`].
    pub fn ms(&self, kind: PauseKind) -> u32 {
        let ms = match kind {
            PauseKind::Heading => self.heading_ms,
            PauseKind::Paragraph => self.paragraph_ms,
            PauseKind::ListItem => self.list_item_ms,
        };
        ms.min(MAX_PAUSE_MS)
    }

    /// How long the pause after `kind` lasts at `rate`: the configured
    /// length up to [`FULL_LENGTH_WPM`], shorter in proportion above it,
    /// and never below [`MIN_RATE_SCALE`] of it. Zero stays zero.
    pub fn length(&self, kind: PauseKind, rate: Rate) -> Duration {
        let ms = self.ms(kind);
        if ms == 0 {
            return Duration::ZERO;
        }
        let wpm = f32::from(rate.wpm().max(1));
        let scale = (f32::from(FULL_LENGTH_WPM) / wpm).clamp(MIN_RATE_SCALE, 1.0);
        // At most 3000 ms times at most 1: well inside f32's exact range.
        Duration::from_millis((ms as f32 * scale).round() as u64)
    }
}

/// Where the current reading's blocks end, by document position.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PausePlan {
    after: HashMap<CharPos, PauseKind>,
}

impl PausePlan {
    /// A plan from the caller's pauses (a later one at the same position
    /// wins).
    pub fn new(pauses: impl IntoIterator<Item = PauseAt>) -> Self {
        PausePlan {
            after: pauses.into_iter().map(|p| (p.after, p.kind)).collect(),
        }
    }

    /// True when the plan has no pauses.
    pub fn is_empty(&self) -> bool {
        self.after.is_empty()
    }

    /// The block that ends after `u`, for document text only.
    pub fn kind_after(&self, u: &Utterance) -> Option<PauseKind> {
        if self.after.is_empty() || u.kind != UtteranceKind::Text {
            return None;
        }
        let end = u.source_range()?.end;
        self.after.get(&end).copied()
    }

    /// The pause after `u` at `rate` with lengths `config`; zero when there
    /// is none.
    pub fn length_after(&self, u: &Utterance, config: &PauseConfig, rate: Rate) -> Duration {
        self.kind_after(u)
            .map_or(Duration::ZERO, |k| config.length(k, rate))
    }

    /// Normalization moved where an utterance ends from `from` to `to`
    /// (rare: it keeps the source covered): the pause moves with it.
    pub fn rekey(&mut self, from: CharPos, to: CharPos) {
        if from == to {
            return;
        }
        if let Some(kind) = self.after.remove(&from) {
            self.after.insert(to, kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_follow_the_settings_and_zero_is_off() {
        let c = PauseConfig::default();
        assert_eq!(
            c.length(PauseKind::Heading, Rate::default()),
            Duration::from_millis(400)
        );
        assert_eq!(
            c.length(PauseKind::Paragraph, Rate::default()),
            Duration::from_millis(300)
        );
        assert_eq!(
            c.length(PauseKind::ListItem, Rate::default()),
            Duration::from_millis(150)
        );
        let off = PauseConfig {
            paragraph_ms: 0,
            ..c
        };
        assert_eq!(
            off.length(PauseKind::Paragraph, Rate::default()),
            Duration::ZERO
        );
        assert!(PauseConfig::OFF.is_off());
        assert!(!c.is_off());
    }

    #[test]
    fn high_rates_shorten_pauses_down_to_a_quarter() {
        let c = PauseConfig::default();
        // Slower than the default: full length, never longer.
        assert_eq!(
            c.length(PauseKind::Paragraph, Rate::Wpm(150)),
            Duration::from_millis(300)
        );
        // Twice the default rate: half.
        assert_eq!(
            c.length(PauseKind::Paragraph, Rate::Wpm(530)),
            Duration::from_millis(150)
        );
        // Far beyond: a quarter at least.
        assert_eq!(
            c.length(PauseKind::Heading, Rate::Wpm(5000)),
            Duration::from_millis(100)
        );
    }

    #[test]
    fn long_settings_are_clamped() {
        let c = PauseConfig {
            heading_ms: 60_000,
            ..PauseConfig::default()
        };
        assert_eq!(c.ms(PauseKind::Heading), MAX_PAUSE_MS);
    }

    #[test]
    fn the_plan_finds_text_by_where_it_ends() {
        let mut plan = PausePlan::new([PauseAt {
            after: CharPos(10),
            kind: PauseKind::Heading,
        }]);
        let u = Utterance::literal("Chapter 1.", CharPos(0));
        assert_eq!(plan.kind_after(&u), Some(PauseKind::Heading));
        // A resume trims the start; the end, and so the pause, stay.
        let trimmed = Utterance::literal("1.", CharPos(8));
        assert_eq!(plan.kind_after(&trimmed), Some(PauseKind::Heading));
        // Announcements never pause.
        assert_eq!(
            plan.kind_after(&Utterance::announcement("Chapter 1.")),
            None
        );
        plan.rekey(CharPos(10), CharPos(9));
        assert_eq!(plan.kind_after(&u), None);
        assert_eq!(
            plan.kind_after(&Utterance::literal("Chapter 1", CharPos(0))),
            Some(PauseKind::Heading)
        );
    }
}
