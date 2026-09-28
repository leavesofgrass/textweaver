//! Sharing the work with a screen reader (`[accessibility]`,
//! `textweaver_a11y::mode`, `docs/screen-readers.md`).
//!
//! Every announcement, echo, and copy of read text asks [`App::route`]
//! whether to speak it, show it on the status line, or both, according to
//! the accessibility mode:
//!
//! - **self-voicing**: what textweaver always did.
//! - **screen-reader**: textweaver never speaks on its own. Messages, caret
//!   moves, and text read in place go to the status line, with math
//!   written out in words and tables narrated as textweaver would say them.
//!   Continuous reading moves a sentence at a time, putting each sentence
//!   on the status line at textweaver's rate (the screen reader reads it),
//!   or reads with textweaver's voice when `say_all = "voice"`.
//! - **hybrid**: textweaver reads documents aloud (continuous reading and
//!   reading a unit, with its math, table, citation, and structure
//!   narration); messages, typing echo, and caret moves are left to the
//!   screen reader, through the status line and the cursor.
//!
//! `quiet_screen` keeps the text being read aloud off the status line, and
//! the TUI freezes the title line's position while reading continuously.
//! On a run with a screen reader and no mode chosen, [`App::offer_hybrid`]
//! asks once whether to use hybrid mode.

use std::time::{Duration, Instant};

use textweaver_a11y::detect::Detected;
use textweaver_a11y::{
    AccessMode, Announcer, Channel, CursorPlacement, Priority, Route, RouteContext,
};
use textweaver_core::{CharPos, CharRange, Direction, Unit};
use textweaver_speech::normalize::{Math, Transform};
use textweaver_text::units::unit_at;
use textweaver_text::{NavOptions, navigate};

use crate::app::App;
use crate::command::{Confirm, Effect};
use crate::playback::{Playback, ReadKind};

/// The routing mode for the `[accessibility] mode` setting.
pub fn access_mode_from_setting(m: textweaver_store::AccessMode) -> AccessMode {
    match m {
        textweaver_store::AccessMode::SelfVoicing => AccessMode::SelfVoicing,
        textweaver_store::AccessMode::ScreenReader => AccessMode::ScreenReader,
        textweaver_store::AccessMode::Hybrid => AccessMode::Hybrid,
    }
}

/// The `[accessibility] mode` setting for a routing mode.
pub fn access_mode_setting(m: AccessMode) -> textweaver_store::AccessMode {
    match m {
        AccessMode::SelfVoicing => textweaver_store::AccessMode::SelfVoicing,
        AccessMode::ScreenReader => textweaver_store::AccessMode::ScreenReader,
        AccessMode::Hybrid => textweaver_store::AccessMode::Hybrid,
    }
}

/// The keymap preset for the `[keyboard] preset` setting.
pub fn keymap_preset(p: textweaver_store::KeymapPreset) -> textweaver_keymap::Preset {
    match p {
        textweaver_store::KeymapPreset::Default => textweaver_keymap::Preset::Default,
        textweaver_store::KeymapPreset::Classic => textweaver_keymap::Preset::Classic,
    }
}

/// The digit row for the `[keyboard] digit_row` setting.
pub fn digit_row(d: textweaver_store::DigitRow) -> textweaver_keymap::digits::DigitRow {
    match d {
        textweaver_store::DigitRow::Auto => textweaver_keymap::digits::DigitRow::Auto,
        textweaver_store::DigitRow::Azerty => textweaver_keymap::digits::DigitRow::Azerty,
    }
}

/// Continuous reading in screen-reader mode without textweaver's voice: the
/// sentence on the status line and when the next one is due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScreenSayAll {
    /// The sentence shown.
    pub(crate) sentence: CharRange,
    /// When to move to the next one.
    pub(crate) next_at: Instant,
}

/// The most characters of document text put on the status line at once.
pub const STATUS_TEXT_LIMIT: usize = 600;

/// A pause after each sentence of the screen say-all, so the screen reader
/// finishes before the next one appears.
pub const SENTENCE_GAP: Duration = Duration::from_millis(600);

/// How long `text` takes to say at `wpm` words per minute, plus
/// [`SENTENCE_GAP`]; at least one second.
pub fn sentence_duration(text: &str, wpm: u16) -> Duration {
    let words = u64::try_from(text.split_whitespace().count()).unwrap_or(u64::MAX);
    let wpm = u64::from(wpm.max(1));
    let speaking = Duration::from_millis(words.saturating_mul(60_000) / wpm);
    (speaking + SENTENCE_GAP).max(Duration::from_secs(1))
}

impl App {
    /// The accessibility mode in effect (the setting, or `--mode` and
    /// `--no-speech` for this run).
    pub fn access_mode(&self) -> AccessMode {
        self.access_mode
    }

    /// Uses `mode` for this run without saving it (`--mode`,
    /// `--no-speech`). The mode-cycling command saves.
    pub fn set_access_mode_for_run(&mut self, mode: AccessMode) {
        self.access_mode = mode;
    }

    /// Where the terminal's cursor waits (`[accessibility] cursor`).
    pub fn cursor_placement(&self) -> CursorPlacement {
        match self.settings.accessibility.cursor {
            textweaver_store::CursorPlacement::Follow => CursorPlacement::Follow,
            textweaver_store::CursorPlacement::Status => CursorPlacement::Status,
        }
    }

    /// True while the screen should keep still: `[accessibility]
    /// quiet_screen` is on and continuous reading is going on. The TUI then
    /// stops updating the title line's position.
    pub fn quiet_screen_active(&self) -> bool {
        self.settings.accessibility.quiet_screen
            && self.playback == Playback::Reading
            && self.reading == ReadKind::Continuous
    }

    /// True when the speech engine makes sound (not the silent backend).
    pub(crate) fn has_voice(&self) -> bool {
        self.speech.backend_id() != "null"
    }

    /// Where output of `channel` goes now.
    pub(crate) fn route(&self, channel: Channel) -> Route {
        let voice = match self.access_mode {
            // Self-voicing keeps its old meaning: the GUI reads aloud but
            // leaves its messages to the screen reader.
            AccessMode::SelfVoicing => self.self_voicing,
            _ => self.has_voice(),
        };
        let ctx = RouteContext {
            voice,
            quiet_screen: self.settings.accessibility.quiet_screen
                && self.playback == Playback::Reading,
        };
        textweaver_a11y::route(self.access_mode, channel, ctx)
    }

    /// Text for the status line: with a screen reader reading it, math is
    /// written out in words, as textweaver would say it (`$x^2$` becomes "x
    /// squared"); otherwise unchanged.
    pub(crate) fn screen_text(&self, text: &str) -> String {
        let norm = &self.settings.normalization;
        let has_math = text.contains('$')
            || text.contains("\\(")
            || text.contains("\\[")
            || norm.asciimath_delimiter.is_some_and(|d| text.contains(d));
        if !self.access_mode.uses_screen_reader() || !norm.math || !has_math {
            return text.to_owned();
        }
        let config = textweaver_engines::service_config(&self.settings).normalize;
        Math::from_config(&config).apply(text).0
    }

    /// Shows `text` on the status line when `channel` goes there now.
    pub(crate) fn show_as(&mut self, channel: Channel, text: &str) {
        if self.route(channel).status {
            self.show(text);
        }
    }

    /// Document text in `range` as textweaver would say it, for the status
    /// line: table and structure narration from the reading plan, math in
    /// words; at most [`STATUS_TEXT_LIMIT`] characters.
    pub(crate) fn narrated(&mut self, range: CharRange) -> String {
        let citations = self.citation_speech_in(range);
        let Some(s) = self.session.as_ref() else {
            return String::new();
        };
        let policy = self.narration_policy();
        let range = range.clamp_to(s.doc.len_chars());
        let joined = textweaver_text::plan_with(&s.doc, range, &policy, &citations)
            .into_iter()
            .map(|u| u.text.trim().to_owned())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let text = self.screen_text(&joined);
        if text.chars().count() <= STATUS_TEXT_LIMIT {
            return text;
        }
        let cut: String = text.chars().take(STATUS_TEXT_LIMIT).collect();
        let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(a, _)| a);
        format!("{cut}…")
    }

    /// Next accessibility mode (`cycle_access_mode`), saved. The change is
    /// announced the way the old mode announced things, so whoever was
    /// listening hears it.
    pub(crate) fn cycle_access_mode(&mut self) {
        let new = self.access_mode.next();
        self.tell(&format!("{} mode. {}", new.name(), new.description()));
        if self.playback != Playback::Idle {
            self.stop_speech();
        }
        self.access_mode = new;
        self.settings.accessibility.mode = access_mode_setting(new);
        self.settings_dirty = true;
    }

    // ---- The first-run question ----

    /// True when the first-run question about hybrid mode has not been
    /// asked yet and the mode was never chosen: then it is worth checking
    /// for a screen reader ([`textweaver_a11y::detect::detect`]).
    pub fn hybrid_offer_due(&self) -> bool {
        let a = &self.settings.accessibility;
        !a.hybrid_offered
            && a.mode == textweaver_store::AccessMode::SelfVoicing
            && self.access_mode == AccessMode::SelfVoicing
            && self.pending_hybrid.is_none()
    }

    /// Asks once, when a screen reader was found and the mode was never
    /// chosen, whether to use hybrid mode. Returns true when it asked; the
    /// frontend then sends key presses as [`Confirm`] answers
    /// ([`App::confirmation_pending`]).
    pub fn offer_hybrid(&mut self, found: &Detected) -> bool {
        if !self.hybrid_offer_due() {
            return false;
        }
        self.pending_hybrid = Some(hybrid_question(found));
        let q = self.pending_hybrid.clone().unwrap_or_default();
        self.ask(&q);
        true
    }

    /// The question waiting for a yes or no, if any: the first-run question
    /// or an action's confirmation ("Quit textweaver? y or n"). The TUI keeps
    /// it on the status line.
    pub fn pending_question(&self) -> Option<String> {
        self.pending_hybrid.clone().or_else(|| {
            self.pending_confirm
                .and_then(textweaver_keymap::ActionId::confirmation_prompt)
                .map(str::to_owned)
        })
    }

    /// The answer to the first-run question.
    pub(crate) fn confirm_hybrid(&mut self, answer: Confirm) -> Vec<Effect> {
        let key = self.keys(textweaver_keymap::ActionId::CycleAccessMode);
        match answer {
            Confirm::Yes => {
                self.pending_hybrid = None;
                self.access_mode = AccessMode::Hybrid;
                self.settings.accessibility.mode = textweaver_store::AccessMode::Hybrid;
                self.settings.accessibility.hybrid_offered = true;
                self.settings_dirty = true;
                self.tell(&format!(
                    "Hybrid mode. {} {key} changes the mode.",
                    AccessMode::Hybrid.description()
                ));
            }
            Confirm::No => {
                self.pending_hybrid = None;
                self.settings.accessibility.hybrid_offered = true;
                self.settings_dirty = true;
                self.tell(&format!(
                    "Staying in self-voicing mode. {key} changes the mode."
                ));
            }
            Confirm::Repeat => {
                let q = self.pending_hybrid.clone().unwrap_or_default();
                self.ask(&q);
            }
        }
        vec![Effect::Redraw]
    }

    // ---- Continuous reading on the status line ----

    /// True when continuous reading should move through the text on the
    /// status line instead of speaking: screen-reader mode with `say_all =
    /// "screen"`, or a screen reader mode with no voice to read with.
    pub(crate) fn screen_say_all_wanted(&self) -> bool {
        match self.access_mode {
            AccessMode::SelfVoicing => false,
            AccessMode::Hybrid => !self.has_voice(),
            AccessMode::ScreenReader => {
                self.settings.accessibility.say_all == textweaver_store::SayAll::Screen
                    || !self.has_voice()
            }
        }
    }

    /// Starts the screen say-all at the sentence holding `pos` (or the next
    /// one). False when there is nothing left to read.
    pub(crate) fn start_screen_say_all(&mut self, pos: CharPos, now: Instant) -> bool {
        let Some(s) = self.session.as_ref() else {
            return false;
        };
        let sentence = unit_at(&s.doc, pos, Unit::Sentence)
            .filter(|r| r.end > pos || r.start == pos)
            .or_else(|| {
                navigate(
                    &s.doc,
                    pos,
                    Unit::Sentence,
                    Direction::Forward,
                    NavOptions::default(),
                )
                .map(|t| t.range)
            });
        let Some(sentence) = sentence else {
            return false;
        };
        self.playback = Playback::Reading;
        self.reading = ReadKind::Continuous;
        self.show_screen_sentence(sentence, now);
        true
    }

    /// Puts `sentence` on the status line, moves the cursor and highlight to
    /// it, and schedules the next one.
    fn show_screen_sentence(&mut self, sentence: CharRange, now: Instant) {
        let text = self.narrated(sentence);
        let text = if text.is_empty() {
            "blank".to_owned()
        } else {
            text
        };
        if let Some(s) = self.session.as_mut() {
            s.cursor = sentence.start;
            s.spoken = Some(sentence);
            s.spoken_sentence = Some(sentence);
        }
        self.status.announce(&text, Priority::Polite);
        self.announcer.announce(&text, Priority::Polite);
        let wpm = self.settings.speech.rate.wpm();
        self.screen_say_all = Some(ScreenSayAll {
            sentence,
            next_at: now + sentence_duration(&text, wpm),
        });
        self.scroll_to_focus();
    }

    /// Moves the screen say-all on when the next sentence is due; true when
    /// the display changed. Called from [`App::tick`].
    pub(crate) fn screen_say_all_tick(&mut self, now: Instant) -> bool {
        let Some(sa) = self.screen_say_all else {
            return false;
        };
        if now < sa.next_at {
            return false;
        }
        let next = self.session.as_ref().and_then(|s| {
            navigate(
                &s.doc,
                sa.sentence.start,
                Unit::Sentence,
                Direction::Forward,
                NavOptions::default(),
            )
            .map(|t| t.range)
            .filter(|r| r.start > sa.sentence.start)
        });
        match next {
            Some(r) => self.show_screen_sentence(r, now),
            None => {
                self.screen_say_all = None;
                self.playback = Playback::Idle;
                if let Some(s) = self.session.as_mut() {
                    s.spoken = None;
                    s.spoken_sentence = None;
                }
                self.tell("End of document.");
            }
        }
        true
    }

    /// Pauses the screen say-all if it is running; true when it was.
    pub(crate) fn pause_screen_say_all(&mut self) -> bool {
        let Some(sa) = self.screen_say_all.take() else {
            return false;
        };
        self.playback = Playback::Paused {
            resume_at: Some(sa.sentence.start),
        };
        self.pause_origin = Some(sa.sentence.start);
        if let Some(s) = self.session.as_mut() {
            s.spoken = None;
            s.spoken_sentence = None;
        }
        self.note("Paused.");
        true
    }

    /// True while continuous reading goes through the status line.
    pub fn screen_say_all_running(&self) -> bool {
        self.screen_say_all.is_some()
    }
}

/// The first-run question, worded to be read aloud.
fn hybrid_question(found: &Detected) -> String {
    format!(
        "{} is running. Use hybrid mode, where textweaver reads documents aloud and your screen reader speaks messages and typing? y or n",
        found.spoken_name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_time_follows_the_rate() {
        let ten = "one two three four five six seven eight nine ten";
        assert_eq!(
            sentence_duration(ten, 300),
            Duration::from_secs(2) + SENTENCE_GAP
        );
        assert_eq!(sentence_duration("Hi.", 900), Duration::from_secs(1));
        assert_eq!(sentence_duration("", 0), Duration::from_secs(1));
    }

    #[test]
    fn the_question_names_the_screen_reader() {
        let q = hybrid_question(&Detected {
            name: Some("NVDA".into()),
        });
        assert!(q.starts_with("NVDA is running. Use hybrid mode"), "{q}");
        assert!(q.ends_with("y or n"));
    }
}
