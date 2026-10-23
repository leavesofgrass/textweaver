//! CW (Morse code) in the reader: what changes while CW is the speech
//! engine (`[speech] backend = "cw"`).
//!
//! - **Structure as prosigns.** The narration says no structure in words
//!   (no "heading level 2" sent letter by letter); instead, with `[speech.cw]
//!   prosigns` on, BT follows each heading and each paragraph, so a heading
//!   has BT before it (its paragraph break) and after it, and AR ends a
//!   reading that reaches the end of the document ([`prosign_structure`]).
//! - **Rate keys** change the character speed by one word per minute, and
//!   the effective (Farnsworth) speed with it in proportion; the reading
//!   restarts from the last word, so the new speed is heard at once.
//! - **The status line** shows the characters as they are sent, newest
//!   last, in at most 40 cells, so a screen reader puts them on a Braille
//!   display. It is shown, not said: nothing extra is announced. When a
//!   reading ends, a count of characters Morse code does not have, which
//!   were skipped, is shown once.

use textweaver_a11y::{Announcer, Priority};
use textweaver_core::Utterance;
use textweaver_cw::sending;
use textweaver_engines::cw::{BACKEND_ID, CwOptions, set_options};
use textweaver_lexicon::args;
use textweaver_speech::{Earcon, PauseAt, PauseKind};
use textweaver_store::{CW_MAX_WPM, CW_MIN_WPM};

use crate::app::App;
use crate::playback::Playback;

/// The cells of a Braille display line the status line fits in.
const STATUS_CELLS: usize = 40;

/// What CW has shown on the status line.
#[derive(Debug, Default)]
pub(crate) struct CwState {
    /// The sending record's sequence last shown.
    seen: u64,
    /// The skipped characters already reported.
    skipped_said: u32,
}

/// `utterances` with structure sent as prosigns: `<BT>` after each one
/// that ends a heading or a paragraph (`pauses`, from
/// [`crate::playback::structural_pauses`]), and `<AR>` at the end when
/// the reading reaches the end of the document (`ends_document`).
pub fn prosign_structure(
    utterances: Vec<Utterance>,
    pauses: &[PauseAt],
    ends_document: bool,
) -> Vec<Utterance> {
    let mut out = Vec::with_capacity(utterances.len() + pauses.len() + 1);
    for u in utterances {
        let end = u.source_range().map(|r| r.end);
        out.push(u);
        let breaks = end.is_some_and(|end| {
            pauses.iter().any(|p| {
                p.after == end && matches!(p.kind, PauseKind::Heading | PauseKind::Paragraph)
            })
        });
        if breaks {
            out.push(Utterance::announcement("<BT>"));
        }
    }
    if ends_document && !out.is_empty() {
        out.push(Utterance::announcement("<AR>"));
    }
    out
}

/// The last `cells` characters of `tail`, starting at a word where it can.
fn newest(tail: &str, cells: usize) -> &str {
    let n = tail.chars().count();
    if n <= cells {
        return tail;
    }
    let cut = tail
        .char_indices()
        .nth(n - cells)
        .map_or(tail.len(), |(i, _)| i);
    let rest = &tail[cut..];
    // Drop a partial word at the start when a whole one follows.
    match rest.find(' ') {
        Some(i) if i + 1 < rest.len() && !tail[..cut].ends_with(' ') => &rest[i + 1..],
        _ => rest,
    }
}

impl App {
    /// True while CW is the speech engine.
    pub(crate) fn cw_engine(&self) -> bool {
        self.speech.backend_id() == BACKEND_ID
    }

    /// True when readings send structure as prosigns.
    pub(crate) fn cw_prosigns(&self) -> bool {
        self.cw_engine() && self.settings.speech.cw.prosigns
    }

    /// Puts `[speech.cw]` into effect: the engine reads it at its next
    /// utterance, and a reading in progress restarts from the last word.
    pub(crate) fn apply_cw_settings(&mut self) {
        set_options(CwOptions::from_settings(&self.settings));
        if self.cw_engine() {
            self.speech.respeak();
        }
    }

    /// A reading starts (not a continuation of one): the status line
    /// starts afresh.
    pub(crate) fn cw_reading_starts(&mut self) {
        if self.cw_engine() && self.playback != Playback::Reading {
            sending().clear();
            self.cw.skipped_said = 0;
        }
    }

    /// The rate keys with CW: the character speed by one word per minute,
    /// the effective speed in proportion.
    pub(crate) fn cw_change_rate(&mut self, delta: i32) {
        let cw = &self.settings.speech.cw;
        let (old, eff) = (cw.wpm, cw.effective_wpm);
        let new = if delta > 0 {
            old.saturating_add(1).min(CW_MAX_WPM)
        } else {
            old.saturating_sub(1).max(CW_MIN_WPM)
        };
        if new == old {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg(if delta > 0 {
                "voice-fastest-rate"
            } else {
                "voice-slowest-rate"
            });
            self.tell(&msg);
            return;
        }
        let scaled = (f64::from(eff) * f64::from(new) / f64::from(old)).round();
        // Between 5 and 50 words per minute: exact as u16.
        let effective = (scaled as u16).clamp(CW_MIN_WPM, new);
        self.settings.speech.cw.wpm = new;
        self.settings.speech.cw.effective_wpm = effective;
        self.settings_dirty = true;
        self.apply_cw_settings();
        let msg = self.msg_args("cw-speed", &args!["wpm" => new, "effective" => effective]);
        self.tell(&msg);
    }

    /// Shows what CW is sending on the status line (called from
    /// [`App::tick`]); after a reading, the count of characters skipped.
    /// True when the status line changed.
    pub(crate) fn cw_tick(&mut self) -> bool {
        if !self.cw_engine() {
            return false;
        }
        let snap = sending().snapshot();
        if self.playback != Playback::Reading {
            if snap.skipped > self.cw.skipped_said {
                let n = snap.skipped - self.cw.skipped_said;
                self.cw.skipped_said = snap.skipped;
                let msg = self.msg_args("cw-skipped", &args!["count" => n]);
                self.show(&msg);
                return true;
            }
            return false;
        }
        if !self.settings.speech.cw.status_line || snap.seq == self.cw.seen {
            return false;
        }
        self.cw.seen = snap.seq;
        if snap.tail.is_empty() {
            return false;
        }
        let label = self.msg_args("cw-sending", &args!["text" => ""]);
        let room = STATUS_CELLS.saturating_sub(label.trim_end().chars().count() + 1);
        let line = self.msg_args(
            "cw-sending",
            &args!["text" => newest(&snap.tail, room.max(8))],
        );
        let shown = self.screen_text(&line);
        self.status.announce(&shown, Priority::Polite);
        true
    }

    /// True while the status line follows CW characters, which change
    /// faster than the idle tick.
    pub(crate) fn cw_status_active(&self) -> bool {
        self.playback == Playback::Reading
            && self.settings.speech.cw.status_line
            && self.cw_engine()
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::{CharPos, CharRange};

    use super::*;

    fn text(s: &str, at: usize) -> Utterance {
        Utterance::literal(s, CharPos(at))
    }

    #[test]
    fn headings_and_paragraphs_end_with_bt_and_the_document_with_ar() {
        let u = vec![text("Title", 0), text("One.", 7), text("Two.", 12)];
        let pauses = [
            PauseAt {
                after: CharPos(5),
                kind: PauseKind::Heading,
            },
            PauseAt {
                after: CharPos(11),
                kind: PauseKind::ListItem,
            },
        ];
        let out = prosign_structure(u, &pauses, true);
        let texts: Vec<&str> = out.iter().map(|u| u.text.as_str()).collect();
        assert_eq!(texts, ["Title", "<BT>", "One.", "Two.", "<AR>"]);
        assert_eq!(out[1].source_range(), None);
        assert_eq!(
            out[0].source_range(),
            Some(CharRange::new(CharPos(0), CharPos(5)))
        );
        let none = prosign_structure(Vec::new(), &pauses, true);
        assert!(none.is_empty());
    }

    #[test]
    fn the_newest_characters_fit_the_line() {
        assert_eq!(newest("CQ DE", 30), "CQ DE");
        assert_eq!(newest("CQ CQ DE TEST", 8), "DE TEST");
        assert_eq!(newest("ABCDEFGHIJ", 4), "GHIJ");
    }
}
