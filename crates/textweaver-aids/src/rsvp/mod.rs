//! RSVP: rapid serial visual presentation, one word at a time.
//!
//! star showed the word being spoken in a floating panel, fed by speech
//! word events (`gui/mixin_playback.py:408-423`, `tui/mixin_rsvp.py`). It
//! had no timing of its own, no recognition point, and no way to read
//! silently. textweaver keeps star's speech-driven mode ([`Pacing::External`])
//! and adds a silent, timed mode ([`Pacing::Timer`]) with:
//!
//! - the optimal recognition point of each word ([`optimal_recognition_point`]),
//!   which a frontend shows at a fixed column;
//! - words-per-minute timing with longer pauses after clauses, sentences,
//!   and paragraphs, and for long words;
//! - the previous and next words as context, each switchable, as in star;
//! - star's nine screen positions ([`RsvpPosition`]);
//! - pause, resume, and seeking by word, sentence, and paragraph.
//!
//! [`Rsvp`] is a pure state machine. It never reads a clock: every method
//! that depends on time takes `now`, milliseconds on any monotonic clock
//! the caller chooses. A frontend calls [`Rsvp::tick`] when
//! [`Rsvp::deadline`] passes (for example as its input poll timeout) and
//! redraws when the word changes. Tests pass literal numbers.
//!
//! Timing never skips a word: when ticks arrive late, the next word is
//! shown at the next tick, and the rhythm catches up only by the lateness
//! of one word.
//!
//! [`flash`] checks the schedule against WCAG 2.3.1 (three flashes a
//! second) and gives the cap a frontend with very large words would use.

pub mod flash;
mod layout;
mod track;

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange};

pub use layout::RsvpPosition;
pub use layout::{Area, Segment, SegmentRole, TuiBox, TuiBoxOptions, tui_box};
pub use track::{WordTrack, optimal_recognition_point};

/// Milliseconds on the caller's monotonic clock.
pub type Millis = u64;

/// Slowest supported rate, in words per minute.
pub const MIN_WPM: u32 = 50;
/// Fastest supported rate, in words per minute.
pub const MAX_WPM: u32 = 1500;

/// What moves the RSVP word forward.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pacing {
    /// The engine's own words-per-minute timing (silent reading).
    #[default]
    Timer,
    /// Speech word events, through [`Rsvp::follow`] (star's behaviour).
    /// [`Rsvp::tick`] never advances.
    External,
}

/// RSVP settings. Serializable, so the app can keep them in its settings
/// file; missing keys take the defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RsvpSettings {
    /// Words per minute, [`MIN_WPM`] to [`MAX_WPM`].
    pub wpm: u32,
    /// Timer or speech-driven.
    pub pacing: Pacing,
    /// Extra time after a word followed by a comma, semicolon, colon, dash,
    /// bracket, or closing quote, in percent of one word's time.
    pub clause_pause: u32,
    /// Extra time at the end of a sentence, in percent.
    pub sentence_pause: u32,
    /// Extra time at the end of a paragraph, in percent (used instead of
    /// the sentence pause there).
    pub paragraph_pause: u32,
    /// Words longer than this many letters get extra time.
    pub long_word_len: u32,
    /// Extra time per letter beyond [`RsvpSettings::long_word_len`], in percent.
    pub long_word_step: u32,
    /// Most extra time a long word gets, in percent.
    pub long_word_max: u32,
    /// Show the previous word (star: `qt_rsvp_show_prev`).
    pub show_previous: bool,
    /// Show the next word (star: `qt_rsvp_show_next`).
    pub show_next: bool,
    /// Where the word appears (star: `qt_rsvp_position`).
    pub position: RsvpPosition,
    /// Size of the word in the GUI, in points (star: `qt_rsvp_font_size`).
    /// star labelled this in points but drew pixels, 25 % too small at
    /// 96 DPI; use [`RsvpSettings::font_px`] to convert.
    pub font_size_pt: u16,
    /// With speech pacing, show this many words ahead of (positive) or
    /// behind (negative) the spoken word (star: `highlight_lead_words`).
    pub lead_words: i32,
}

impl Default for RsvpSettings {
    fn default() -> Self {
        RsvpSettings {
            wpm: 300,
            pacing: Pacing::Timer,
            clause_pause: 50,
            sentence_pause: 100,
            paragraph_pause: 150,
            long_word_len: 8,
            long_word_step: 10,
            long_word_max: 80,
            show_previous: true,
            show_next: true,
            position: RsvpPosition::TopCenter,
            font_size_pt: 48,
            lead_words: 0,
        }
    }
}

impl RsvpSettings {
    /// The word size in points, clamped to star's range, 12 to 200.
    pub fn clamped_font_pt(&self) -> u16 {
        self.font_size_pt.clamp(12, 200)
    }

    /// The word size in pixels at `dpi` dots per inch (96 on most screens).
    pub fn font_px(&self, dpi: f32) -> f32 {
        f32::from(self.clamped_font_pt()) * dpi / 72.0
    }

    /// The rate clamped to [`MIN_WPM`]..=[`MAX_WPM`].
    pub fn clamped_wpm(&self) -> u32 {
        self.wpm.clamp(MIN_WPM, MAX_WPM)
    }
}

/// Whether the session is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayState {
    /// Stopped on a word; nothing advances.
    Paused,
    /// Advancing; `since` is when the current word was first shown.
    Playing {
        /// Clock time the current word appeared.
        since: Millis,
    },
    /// The last word's time ran out. Playing again starts from the top.
    Finished,
}

/// What a call changed, for the frontend to announce.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RsvpEvent {
    /// Playback started or resumed.
    Playing,
    /// Playback paused.
    Paused,
    /// The last word was shown and its time ran out.
    Finished,
    /// The current word changed by a seek.
    Moved,
    /// A backward seek was already at the first word.
    AtStart,
    /// A forward seek was already at the last word.
    AtEnd,
    /// The rate changed.
    RateChanged,
    /// There are no words.
    Empty,
}

impl RsvpEvent {
    /// A short message to speak or show, or `None` for events the word
    /// display itself conveys.
    pub fn message(self) -> Option<&'static str> {
        match self {
            RsvpEvent::Playing => Some("RSVP playing"),
            RsvpEvent::Paused => Some("RSVP paused"),
            RsvpEvent::Finished => Some("End of text"),
            RsvpEvent::AtStart => Some("Start of text"),
            RsvpEvent::AtEnd => Some("End of text"),
            RsvpEvent::Empty => Some("No words to show"),
            RsvpEvent::Moved | RsvpEvent::RateChanged => None,
        }
    }
}

/// The result of [`Rsvp::tick`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tick {
    /// True when a new word is showing; redraw.
    pub advanced: bool,
    /// [`RsvpEvent::Finished`] when the text ran out on this tick.
    pub event: Option<RsvpEvent>,
    /// When to tick next, if playing.
    pub next_deadline: Option<Millis>,
}

/// Everything a frontend needs to draw the current word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RsvpFrame<'a> {
    /// Index of the word, from 0.
    pub index: usize,
    /// Number of words.
    pub total: usize,
    /// The word with its attached punctuation.
    pub text: &'a str,
    /// `text` before the pivot.
    pub before: &'a str,
    /// The pivot grapheme (the optimal recognition point).
    pub pivot: &'a str,
    /// `text` after the pivot.
    pub after: &'a str,
    /// The word unit in the document, for the view's highlight and caret.
    pub word: CharRange,
    /// The word with its punctuation, in the document.
    pub display: CharRange,
    /// The previous word, when shown.
    pub previous: Option<&'a str>,
    /// The next word, when shown.
    pub next: Option<&'a str>,
    /// True for the last word of a sentence.
    pub sentence_end: bool,
    /// True for the last word of a paragraph.
    pub paragraph_end: bool,
    /// How long this word shows at the current rate, in milliseconds.
    pub duration: Millis,
}

/// An RSVP session over a [`WordTrack`].
#[derive(Clone, Debug)]
pub struct Rsvp {
    track: WordTrack,
    settings: RsvpSettings,
    index: usize,
    state: PlayState,
}

impl Rsvp {
    /// A paused session at the first word.
    pub fn new(track: WordTrack, settings: RsvpSettings) -> Self {
        Rsvp {
            track,
            settings,
            index: 0,
            state: PlayState::Paused,
        }
    }

    /// The words.
    pub fn track(&self) -> &WordTrack {
        &self.track
    }

    /// The settings.
    pub fn settings(&self) -> &RsvpSettings {
        &self.settings
    }

    /// Replaces the settings. The current word keeps showing; its time is
    /// measured at the new rate from when it appeared.
    pub fn set_settings(&mut self, settings: RsvpSettings) {
        self.settings = settings;
    }

    /// Replaces the words (after an edit), keeping the position at the
    /// first word at or after the current word's start.
    pub fn set_track(&mut self, track: WordTrack, now: Millis) {
        let pos = self.current_pos();
        self.track = track;
        let i = self.track.word_at_or_after(pos).unwrap_or(0);
        self.move_to(i, now);
    }

    /// Index of the current word.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Number of words.
    pub fn len(&self) -> usize {
        self.track.len()
    }

    /// True when there are no words.
    pub fn is_empty(&self) -> bool {
        self.track.is_empty()
    }

    /// Playing, paused, or finished.
    pub fn state(&self) -> PlayState {
        self.state
    }

    /// True while playing.
    pub fn is_playing(&self) -> bool {
        matches!(self.state, PlayState::Playing { .. })
    }

    /// The canonical start of the current word (for saving the reading
    /// position), or 0 when empty.
    pub fn current_pos(&self) -> CharPos {
        self.track
            .word_range(self.index)
            .map_or(CharPos::ZERO, |r| r.start)
    }

    /// How long word `i` shows at the current rate, in milliseconds.
    pub fn duration(&self, i: usize) -> Millis {
        let Some(w) = self.track.get(i) else {
            return 0;
        };
        let s = &self.settings;
        let mut extra = 0u64;
        if w.paragraph_end() {
            extra += u64::from(s.paragraph_pause);
        } else if w.sentence_end() {
            extra += u64::from(s.sentence_pause);
        } else if w.clause_end() {
            extra += u64::from(s.clause_pause);
        }
        if w.graphemes > s.long_word_len {
            let over = u64::from(w.graphemes - s.long_word_len);
            extra += (over * u64::from(s.long_word_step)).min(u64::from(s.long_word_max));
        }
        let wpm = u64::from(s.clamped_wpm());
        // 60,000 ms per minute, scaled by (100 + extra) percent, rounded.
        (60_000 * (100 + extra) + wpm * 50) / (wpm * 100)
    }

    /// Estimated time to read from the current word to the end, in
    /// milliseconds.
    pub fn remaining(&self) -> Millis {
        (self.index..self.len()).map(|i| self.duration(i)).sum()
    }

    /// When the current word's time runs out, while playing on the timer.
    pub fn deadline(&self) -> Option<Millis> {
        match (self.state, self.settings.pacing) {
            (PlayState::Playing { since }, Pacing::Timer) => {
                Some(since.saturating_add(self.duration(self.index)))
            }
            _ => None,
        }
    }

    /// Starts or resumes. From the finished state, starts again at the
    /// first word. The current word shows for its full time.
    pub fn play(&mut self, now: Millis) -> RsvpEvent {
        if self.is_empty() {
            return RsvpEvent::Empty;
        }
        if self.state == PlayState::Finished {
            self.index = 0;
        }
        self.state = PlayState::Playing { since: now };
        RsvpEvent::Playing
    }

    /// Pauses on the current word. `None` when not playing.
    pub fn pause(&mut self) -> Option<RsvpEvent> {
        if self.is_playing() {
            self.state = PlayState::Paused;
            Some(RsvpEvent::Paused)
        } else {
            None
        }
    }

    /// Plays when paused or finished, pauses when playing.
    pub fn toggle(&mut self, now: Millis) -> RsvpEvent {
        match self.pause() {
            Some(e) => e,
            None => self.play(now),
        }
    }

    /// Advances the timer. Shows at most one new word per call, so a late
    /// tick never skips words.
    pub fn tick(&mut self, now: Millis) -> Tick {
        let idle = Tick {
            advanced: false,
            event: None,
            next_deadline: None,
        };
        let Some(deadline) = self.deadline() else {
            return idle;
        };
        if now < deadline {
            return Tick {
                next_deadline: Some(deadline),
                ..idle
            };
        }
        if self.index + 1 >= self.len() {
            self.state = PlayState::Finished;
            return Tick {
                event: Some(RsvpEvent::Finished),
                ..idle
            };
        }
        self.index += 1;
        let late = now - deadline;
        // Keep the rhythm when slightly late; restart it when very late
        // (the app was suspended, or a long pause in ticking).
        let since = if late < self.duration(self.index) {
            deadline
        } else {
            now
        };
        self.state = PlayState::Playing { since };
        Tick {
            advanced: true,
            event: None,
            next_deadline: self.deadline(),
        }
    }

    fn move_to(&mut self, i: usize, now: Millis) {
        self.index = i.min(self.len().saturating_sub(1));
        match self.state {
            PlayState::Playing { .. } => self.state = PlayState::Playing { since: now },
            PlayState::Finished => self.state = PlayState::Paused,
            PlayState::Paused => {}
        }
    }

    fn seek(&mut self, target: Option<usize>, forward: bool, now: Millis) -> RsvpEvent {
        if self.is_empty() {
            return RsvpEvent::Empty;
        }
        match target {
            Some(i) if i != self.index || self.state == PlayState::Finished => {
                self.move_to(i, now);
                RsvpEvent::Moved
            }
            _ if forward => RsvpEvent::AtEnd,
            _ => RsvpEvent::AtStart,
        }
    }

    /// Goes to word `i` (clamped to the last word).
    pub fn seek_word(&mut self, i: usize, now: Millis) -> RsvpEvent {
        let forward = i >= self.index;
        let target = i.min(self.len().saturating_sub(1));
        self.seek(Some(target), forward, now)
    }

    /// Goes to the word containing `pos`, or the first word after it.
    pub fn seek_pos(&mut self, pos: CharPos, now: Millis) -> RsvpEvent {
        let target = self.track.word_at_or_after(pos);
        let forward = target.is_some_and(|t| t >= self.index);
        self.seek(target, forward, now)
    }

    /// The next word.
    pub fn next_word(&mut self, now: Millis) -> RsvpEvent {
        let t = (self.index + 1 < self.len()).then_some(self.index + 1);
        self.seek(t, true, now)
    }

    /// The previous word.
    pub fn previous_word(&mut self, now: Millis) -> RsvpEvent {
        let t = self.index.checked_sub(1);
        self.seek(t, false, now)
    }

    /// The first word of the next sentence.
    pub fn next_sentence(&mut self, now: Millis) -> RsvpEvent {
        let t = self
            .track
            .get(self.index)
            .and_then(|w| self.track.sentence_start(w.sentence + 1));
        self.seek(t, true, now)
    }

    /// star's rule: more than three words into the current sentence, go to
    /// its start; otherwise to the start of the previous sentence.
    pub fn previous_sentence(&mut self, now: Millis) -> RsvpEvent {
        let t = self.track.get(self.index).and_then(|w| {
            let start = self.track.sentence_start(w.sentence)?;
            if self.index - start > 3 {
                Some(start)
            } else {
                w.sentence
                    .checked_sub(1)
                    .and_then(|s| self.track.sentence_start(s))
                    .or((start < self.index).then_some(start))
            }
        });
        self.seek(t, false, now)
    }

    /// The first word of the next paragraph.
    pub fn next_paragraph(&mut self, now: Millis) -> RsvpEvent {
        let t = self
            .track
            .get(self.index)
            .and_then(|w| self.track.paragraph_start(w.paragraph + 1));
        self.seek(t, true, now)
    }

    /// The start of this paragraph when more than three words in,
    /// otherwise the start of the previous paragraph (the sentence rule,
    /// applied to paragraphs).
    pub fn previous_paragraph(&mut self, now: Millis) -> RsvpEvent {
        let t = self.track.get(self.index).and_then(|w| {
            let start = self.track.paragraph_start(w.paragraph)?;
            if self.index - start > 3 {
                Some(start)
            } else {
                w.paragraph
                    .checked_sub(1)
                    .and_then(|p| self.track.paragraph_start(p))
                    .or((start < self.index).then_some(start))
            }
        });
        self.seek(t, false, now)
    }

    /// Speech pacing: shows the word being spoken at `pos` (a speech
    /// position's start), shifted by [`RsvpSettings::lead_words`]. Returns
    /// true when the word changed. Works in either pacing mode, so a
    /// frontend can also call it when the reader moves the caret.
    pub fn follow(&mut self, pos: CharPos) -> bool {
        let Some(i) = self.track.word_at_or_after(pos) else {
            return false;
        };
        let lead = self.settings.lead_words;
        let last = self.len() - 1;
        let i = if lead >= 0 {
            i.saturating_add(lead.unsigned_abs() as usize).min(last)
        } else {
            i.saturating_sub(lead.unsigned_abs() as usize)
        };
        let changed = i != self.index;
        self.index = i;
        changed
    }

    /// Sets the rate (clamped). Returns [`RsvpEvent::RateChanged`].
    pub fn set_wpm(&mut self, wpm: u32) -> RsvpEvent {
        self.settings.wpm = wpm.clamp(MIN_WPM, MAX_WPM);
        RsvpEvent::RateChanged
    }

    /// Raises the rate by `step` words per minute (clamped).
    pub fn faster(&mut self, step: u32) -> RsvpEvent {
        self.set_wpm(self.settings.clamped_wpm().saturating_add(step))
    }

    /// Lowers the rate by `step` words per minute (clamped).
    pub fn slower(&mut self, step: u32) -> RsvpEvent {
        self.set_wpm(self.settings.clamped_wpm().saturating_sub(step))
    }

    /// Reading progress in whole percent, rounded down (star's rule).
    pub fn percent(&self) -> u32 {
        if self.is_empty() {
            return 0;
        }
        (self.index * 100 / self.len()) as u32
    }

    /// The current word, ready to draw. `None` when there are no words.
    pub fn frame(&self) -> Option<RsvpFrame<'_>> {
        let i = self.index;
        let w = self.track.get(i)?;
        let (before, pivot, after) = self.track.split(i)?;
        let previous = if self.settings.show_previous && i > 0 {
            self.track.text(i - 1)
        } else {
            None
        };
        let next = if self.settings.show_next {
            self.track.text(i + 1)
        } else {
            None
        };
        Some(RsvpFrame {
            index: i,
            total: self.len(),
            text: self.track.text(i)?,
            before,
            pivot,
            after,
            word: w.word,
            display: w.display,
            previous,
            next,
            sentence_end: w.sentence_end(),
            paragraph_end: w.paragraph_end(),
            duration: self.duration(i),
        })
    }

    /// A status line to show and speak: "Word 12 of 300, 3 percent.
    /// Sentence 2 of 20. 300 words per minute. Paused."
    pub fn status(&self) -> String {
        if self.is_empty() {
            return "No words to show.".to_owned();
        }
        let sentence = self.track.get(self.index).map_or(0, |w| w.sentence) + 1;
        let state = match (self.state, self.settings.pacing) {
            (PlayState::Playing { .. }, _) => "Playing",
            (PlayState::Paused, _) => "Paused",
            (PlayState::Finished, _) => "Finished",
        };
        let rate = match self.settings.pacing {
            Pacing::Timer => format!("{} words per minute", self.settings.clamped_wpm()),
            Pacing::External => "following speech".to_owned(),
        };
        format!(
            "Word {} of {}, {} percent. Sentence {} of {}. {}. {}.",
            self.index + 1,
            self.len(),
            self.percent(),
            sentence,
            self.track.sentence_count(),
            rate,
            state,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "One two three four five. Six seven, eight.\n\nNine ten eleven twelve.";

    fn rsvp() -> Rsvp {
        Rsvp::new(WordTrack::from_text(TEXT), RsvpSettings::default())
    }

    fn word(r: &Rsvp) -> &str {
        r.frame().unwrap().text
    }

    #[test]
    fn durations_follow_the_rate_and_pauses() {
        let r = rsvp();
        // 300 wpm: 200 ms a word.
        assert_eq!(r.duration(0), 200);
        assert_eq!(r.duration(4), 400); // "five." ends a sentence: +100 %
        assert_eq!(r.duration(6), 300); // "seven," clause: +50 %
        assert_eq!(r.duration(7), 500); // "eight." ends a paragraph: +150 %
        let long = Rsvp::new(
            WordTrack::from_text("extraordinarily"),
            RsvpSettings {
                paragraph_pause: 0,
                ..RsvpSettings::default()
            },
        );
        // 15 letters, 7 over 8: +70 %.
        assert_eq!(long.duration(0), 340);
        let fast = Rsvp::new(
            WordTrack::from_text("a b"),
            RsvpSettings {
                wpm: 100_000,
                ..RsvpSettings::default()
            },
        );
        assert_eq!(fast.duration(0), 40); // clamped to 1500 wpm
        assert_eq!(r.duration(999), 0);
        assert_eq!(
            rsvp().remaining(),
            (0..12).map(|i| r.duration(i)).sum::<u64>()
        );
    }

    #[test]
    fn timer_steps_through_words() {
        let mut r = rsvp();
        assert_eq!(r.tick(0).next_deadline, None, "paused");
        assert_eq!(r.play(1000), RsvpEvent::Playing);
        assert_eq!(r.deadline(), Some(1200));
        assert!(!r.tick(1199).advanced);
        let t = r.tick(1200);
        assert!(t.advanced);
        assert_eq!(word(&r), "two");
        assert_eq!(t.next_deadline, Some(1400));
        // A slightly late tick keeps the rhythm.
        assert!(r.tick(1450).advanced);
        assert_eq!(r.deadline(), Some(1600));
        // A very late tick restarts it, and still shows only one new word.
        assert!(r.tick(9000).advanced);
        assert_eq!(word(&r), "four");
        assert_eq!(r.deadline(), Some(9200));
    }

    #[test]
    fn pause_resume_and_finish() {
        let mut r = rsvp();
        r.play(0);
        r.tick(200);
        assert_eq!(r.pause(), Some(RsvpEvent::Paused));
        assert_eq!(r.pause(), None);
        assert!(!r.tick(10_000).advanced);
        assert_eq!(word(&r), "two");
        assert_eq!(r.toggle(20_000), RsvpEvent::Playing);
        assert_eq!(r.deadline(), Some(20_200), "resumed word shows in full");
        r.seek_word(11, 20_000);
        assert_eq!(r.deadline(), Some(20_500));
        let t = r.tick(20_500);
        assert_eq!(t.event, Some(RsvpEvent::Finished));
        assert_eq!(r.state(), PlayState::Finished);
        assert_eq!(word(&r), "twelve.");
        assert_eq!(r.play(30_000), RsvpEvent::Playing);
        assert_eq!(r.index(), 0, "playing after the end starts over");
    }

    #[test]
    fn seeking_by_word_sentence_and_paragraph() {
        let mut r = rsvp();
        assert_eq!(r.previous_word(0), RsvpEvent::AtStart);
        assert_eq!(r.next_sentence(0), RsvpEvent::Moved);
        assert_eq!(word(&r), "Six");
        assert_eq!(r.next_paragraph(0), RsvpEvent::Moved);
        assert_eq!(word(&r), "Nine");
        assert_eq!(r.next_paragraph(0), RsvpEvent::AtEnd);
        assert_eq!(r.next_sentence(0), RsvpEvent::AtEnd);
        r.seek_word(11, 0);
        assert_eq!(r.next_word(0), RsvpEvent::AtEnd);
        // "twelve." is three words into its sentence: previous sentence.
        assert_eq!(r.previous_sentence(0), RsvpEvent::Moved);
        assert_eq!(word(&r), "Six");
        // Four words in: rewind to this sentence's start.
        r.seek_word(4, 0);
        assert_eq!(r.previous_sentence(0), RsvpEvent::Moved);
        assert_eq!(word(&r), "One");
        assert_eq!(r.previous_sentence(0), RsvpEvent::AtStart);
        r.seek_word(9, 0);
        assert_eq!(r.previous_paragraph(0), RsvpEvent::Moved);
        assert_eq!(word(&r), "One");
        r.seek_word(100, 0);
        assert_eq!(r.index(), 11);
        assert_eq!(r.seek_pos(CharPos(26), 0), RsvpEvent::Moved);
        assert_eq!(word(&r), "Six");
        assert_eq!(r.current_pos(), CharPos(25));
    }

    #[test]
    fn seeking_while_playing_restarts_the_word_time() {
        let mut r = rsvp();
        r.play(0);
        r.next_word(150);
        assert_eq!(r.deadline(), Some(350));
    }

    #[test]
    fn speech_pacing_follows_positions() {
        let mut r = Rsvp::new(
            WordTrack::from_text(TEXT),
            RsvpSettings {
                pacing: Pacing::External,
                ..RsvpSettings::default()
            },
        );
        r.play(0);
        assert_eq!(r.deadline(), None);
        assert!(!r.tick(99_999).advanced);
        assert!(r.follow(CharPos(4)));
        assert_eq!(word(&r), "two");
        assert!(!r.follow(CharPos(5)));
        r.set_settings(RsvpSettings {
            lead_words: 1,
            pacing: Pacing::External,
            ..RsvpSettings::default()
        });
        assert!(r.follow(CharPos(4)));
        assert_eq!(word(&r), "three");
        r.set_settings(RsvpSettings {
            lead_words: -5,
            pacing: Pacing::External,
            ..RsvpSettings::default()
        });
        r.follow(CharPos(4));
        assert_eq!(r.index(), 0);
        assert!(r.status().contains("following speech"));
    }

    #[test]
    fn frames_and_context() {
        let mut r = rsvp();
        r.seek_word(6, 0);
        let f = r.frame().unwrap();
        assert_eq!(f.text, "seven,");
        assert_eq!((f.before, f.pivot, f.after), ("s", "e", "ven,"));
        assert_eq!(f.previous, Some("Six"));
        assert_eq!(f.next, Some("eight."));
        assert_eq!(f.word, CharRange::new(29, 34));
        assert_eq!(f.display, CharRange::new(29, 35));
        let mut s = r.settings().clone();
        s.show_previous = false;
        s.show_next = false;
        r.set_settings(s);
        let f = r.frame().unwrap();
        assert_eq!((f.previous, f.next), (None, None));
        r.seek_word(0, 0);
        assert_eq!(r.frame().unwrap().previous, None);
    }

    #[test]
    fn rate_changes_and_status() {
        let mut r = rsvp();
        assert_eq!(r.faster(50), RsvpEvent::RateChanged);
        assert_eq!(r.settings().wpm, 350);
        r.slower(10_000);
        assert_eq!(r.settings().wpm, MIN_WPM);
        r.set_wpm(300);
        r.seek_word(5, 0);
        assert_eq!(
            r.status(),
            "Word 6 of 12, 41 percent. Sentence 2 of 3. 300 words per minute. Paused."
        );
        assert_eq!(RsvpEvent::Paused.message(), Some("RSVP paused"));
        assert_eq!(RsvpEvent::Moved.message(), None);
    }

    #[test]
    fn empty_sessions() {
        let mut r = Rsvp::new(WordTrack::default(), RsvpSettings::default());
        assert_eq!(r.play(0), RsvpEvent::Empty);
        assert_eq!(r.next_word(0), RsvpEvent::Empty);
        assert!(r.frame().is_none());
        assert!(!r.follow(CharPos(3)));
        assert_eq!(r.percent(), 0);
        assert_eq!(r.status(), "No words to show.");
        assert_eq!(r.current_pos(), CharPos::ZERO);
    }

    #[test]
    fn settings_round_trip_and_font_size() {
        let s = RsvpSettings::default();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<RsvpSettings>(&json).unwrap(), s);
        let partial: RsvpSettings = serde_json::from_str(r#"{"wpm": 450}"#).unwrap();
        assert_eq!(partial.wpm, 450);
        assert_eq!(partial.position, RsvpPosition::TopCenter);
        assert_eq!(s.font_px(96.0), 64.0);
    }

    #[test]
    fn set_track_keeps_the_position() {
        let mut r = rsvp();
        r.seek_word(6, 0);
        r.set_track(WordTrack::from_text(TEXT), 0);
        assert_eq!(r.index(), 6);
    }
}
