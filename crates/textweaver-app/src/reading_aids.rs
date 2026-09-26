//! Reading aids (`textweaver-aids`, ADR-0022): RSVP, bionic reading, the
//! reading ruler and current line, terminal text spacing, and the reading
//! level. The aids crate computes; the app keeps the state and announces;
//! frontends draw.
//!
//! **RSVP** shows one word at a time. The word track covers a window of
//! the document around the cursor ([`RSVP_WINDOW`] chars) so it builds
//! quickly on any document; the next window is built when the word reaches
//! the end of this one. The clock is the app's: [`App::tick`] advances the
//! word (at most one word per tick, so a late tick never skips) and moves
//! the cursor to it; while speech reads, the word follows the spoken word
//! instead. While RSVP is showing, sentence, paragraph, and word keys move
//! the RSVP word, Play/Pause (when nothing is read aloud) starts and pauses
//! it, and Stop closes it.
//!
//! **Syllables** (`[reading_aids] syllables`, Alt+Shift+Z) draw long words
//! split with a middle dot (`read·a·bil·i·ty`). The aids crate builds the
//! display text with an offset map (ADR-0005); the app hands frontends the
//! places where a separator goes ([`App::syllable_breaks`]), always between
//! two chars of the document, so every highlight, the cursor, and a click
//! stay on the document's own positions, and speech reads the text as it
//! is.
//!
//! **Difficult words** (`[reading_aids] difficult_words`, Alt+Shift+J) are
//! words SCOWL ranks as rare. Frontends underline them (never colour
//! alone), and at high verbosity a word move onto one adds "difficult
//! word".

use std::time::{Duration, Instant};

use textweaver_aids::{
    DifficultOptions, Millis, Rsvp, RsvpEvent, RulerMode, RulerSettings, ScowlList, SplitText,
    TerminalSpacing, WordTrack, bionic_range, difficult_range, reading_level, split_range,
};
use textweaver_core::SpanKind;
use textweaver_core::{CharPos, CharRange};
use textweaver_keymap::ActionId;

use crate::app::{App, Mode};
use crate::playback::Playback;
use crate::text_util;

/// Chars of the document an RSVP word track covers at a time.
pub const RSVP_WINDOW: usize = 64 * 1024;

/// How far before the cursor an RSVP window starts, so going back a little
/// does not rebuild it.
const RSVP_BACK: usize = 4 * 1024;

/// Words per minute one `rsvp_faster` or `rsvp_slower` step changes.
pub const RSVP_STEP: u32 = 25;

/// RSVP while it is showing.
#[derive(Debug)]
pub(crate) struct RsvpState {
    pub(crate) rsvp: Rsvp,
    /// The origin of the RSVP clock.
    start: Instant,
}

impl RsvpState {
    fn now(&self, now: Instant) -> Millis {
        Millis::try_from(now.saturating_duration_since(self.start).as_millis())
            .unwrap_or(Millis::MAX)
    }
}

impl App {
    /// RSVP, while it is showing.
    pub fn rsvp(&self) -> Option<&Rsvp> {
        self.rsvp.as_ref().map(|s| &s.rsvp)
    }

    /// How long until the RSVP word changes, while it plays on its timer:
    /// frontends wait at most this long for input before calling
    /// [`App::tick`].
    pub fn rsvp_wait(&self, now: Instant) -> Option<Duration> {
        let s = self.rsvp.as_ref()?;
        let deadline = s.rsvp.deadline()?;
        Some(Duration::from_millis(deadline.saturating_sub(s.now(now))))
    }

    /// The window of the document an RSVP track starting near `pos` covers.
    fn rsvp_window(&self, pos: CharPos) -> Option<CharRange> {
        let s = self.session.as_ref()?;
        let len = s.doc.len_chars();
        let start = text_util::word_start(&s.doc, CharPos(pos.0.saturating_sub(RSVP_BACK)));
        let mut end = CharPos((start.0 + RSVP_WINDOW).min(len));
        if end.0 < len {
            // End between words, so no word is cut in two.
            end = text_util::word_start(&s.doc, end).max(CharPos(pos.0 + 1).clamp_to(len));
        }
        Some(CharRange::new(start, end))
    }

    /// Builds the RSVP track around `pos` and shows the word there.
    fn rsvp_build(&mut self, pos: CharPos, now: Instant) -> Option<RsvpEvent> {
        let window = self.rsvp_window(pos)?;
        let s = self.session.as_ref()?;
        let track = WordTrack::from_range(&s.doc, window);
        match self.rsvp.as_mut() {
            Some(st) => {
                let t = st.now(now);
                st.rsvp.set_track(track, t);
                Some(st.rsvp.seek_pos(pos, t))
            }
            None => {
                let mut rsvp = Rsvp::new(track, self.settings.reading_aids.rsvp.clone());
                let e = rsvp.seek_pos(pos, 0);
                self.rsvp = Some(RsvpState { rsvp, start: now });
                Some(e)
            }
        }
    }

    /// `rsvp_toggle`: shows RSVP at the cursor, or hides it.
    pub(crate) fn rsvp_toggle(&mut self, now: Instant) {
        if self.rsvp.take().is_some() {
            self.tell("RSVP off.");
            return;
        }
        if self.mode == Mode::Edit {
            self.tell("Leave edit mode to use RSVP.");
            return;
        }
        let Some(pos) = self.reading_position() else {
            return;
        };
        self.rsvp_build(pos, now);
        match self.rsvp.as_ref().map(|s| &s.rsvp) {
            Some(r) if !r.is_empty() => {
                let msg = format!("RSVP on. {}", r.status());
                self.sync_cursor_to_rsvp();
                self.tell(&msg);
            }
            _ => {
                self.rsvp = None;
                self.tell("No words to show.");
            }
        }
    }

    /// `rsvp_play_pause`: starts or pauses RSVP, showing it first if needed.
    pub(crate) fn rsvp_play_pause(&mut self, now: Instant) {
        if self.rsvp.is_none() {
            self.rsvp_toggle(now);
        }
        let Some(st) = self.rsvp.as_mut() else {
            return;
        };
        let t = st.now(now);
        let e = st.rsvp.toggle(t);
        self.rsvp_announce(e);
    }

    /// Faster or slower by [`RSVP_STEP`]; the rate is saved.
    pub(crate) fn rsvp_rate(&mut self, faster: bool) {
        let rate = &mut self.settings.reading_aids.rsvp;
        let old = rate.clamped_wpm();
        let new = if faster {
            old.saturating_add(RSVP_STEP)
        } else {
            old.saturating_sub(RSVP_STEP)
        }
        .clamp(
            textweaver_aids::rsvp::MIN_WPM,
            textweaver_aids::rsvp::MAX_WPM,
        );
        if new == old {
            self.tell(if faster {
                "Fastest RSVP rate."
            } else {
                "Slowest RSVP rate."
            });
            return;
        }
        rate.wpm = new;
        self.settings_dirty = true;
        if let Some(st) = self.rsvp.as_mut() {
            st.rsvp.set_wpm(new);
        }
        self.tell(&format!("RSVP {new} words per minute."));
    }

    /// Moves the RSVP word to the next of Star's nine places; saved.
    pub(crate) fn rsvp_position_next(&mut self) {
        let rs = &mut self.settings.reading_aids.rsvp;
        rs.position = rs.position.next();
        let label = rs.position.label();
        let settings = rs.clone();
        self.settings_dirty = true;
        if let Some(st) = self.rsvp.as_mut() {
            st.rsvp.set_settings(settings);
        }
        self.tell(&format!("RSVP at the {label}."));
    }

    /// Announces what an RSVP call changed (nothing for a plain move: the
    /// word itself is what changed).
    fn rsvp_announce(&mut self, e: RsvpEvent) {
        if let Some(m) = e.message() {
            self.tell(&format!("{m}."));
        }
    }

    /// Puts the cursor on the RSVP word, quietly.
    fn sync_cursor_to_rsvp(&mut self) {
        let Some(pos) = self.rsvp().and_then(Rsvp::frame).map(|f| f.word.start) else {
            return;
        };
        if let Some(s) = self.session.as_mut() {
            s.cursor = pos.clamp_to(s.doc.len_chars());
        }
        self.scroll_to_cursor();
    }

    /// While RSVP shows, navigation keys move its word. Returns false for
    /// actions RSVP does not take over.
    pub(crate) fn rsvp_action(&mut self, a: ActionId, now: Instant) -> bool {
        use ActionId as A;
        if self.rsvp.is_none() {
            return false;
        }
        let forward = matches!(a, A::NextSentence | A::NextParagraph | A::CaretNextWord);
        let Some(st) = self.rsvp.as_mut() else {
            return false;
        };
        let t = st.now(now);
        let e = match a {
            A::NextSentence => st.rsvp.next_sentence(t),
            A::PreviousSentence => st.rsvp.previous_sentence(t),
            A::NextParagraph => st.rsvp.next_paragraph(t),
            A::PreviousParagraph => st.rsvp.previous_paragraph(t),
            A::CaretNextWord => st.rsvp.next_word(t),
            A::CaretPreviousWord => st.rsvp.previous_word(t),
            A::PlayPause if self.playback == Playback::Idle => st.rsvp.toggle(t),
            A::Stop => {
                self.rsvp = None;
                self.stop_speech();
                self.tell("RSVP off.");
                return true;
            }
            _ => return false,
        };
        let range = st.rsvp.track().range();
        let e = match e {
            // The window ended: continue in the next (or previous) one.
            RsvpEvent::AtEnd if forward => self.rsvp_next_window(range, now).unwrap_or(e),
            RsvpEvent::AtStart if !forward && range.start > CharPos::ZERO => {
                self.rsvp_build(CharPos(range.start.0 - 1), now);
                RsvpEvent::Moved
            }
            e => e,
        };
        self.sync_cursor_to_rsvp();
        self.rsvp_announce(e);
        true
    }

    /// Continues after the window `range`, if the document goes on.
    fn rsvp_next_window(&mut self, range: CharRange, now: Instant) -> Option<RsvpEvent> {
        let len = self.session.as_ref()?.doc.len_chars();
        if range.end.0 >= len {
            return None;
        }
        let pos = self
            .session
            .as_ref()
            .map(|s| text_util::first_word_at_or_after(&s.doc, range.end))?;
        self.rsvp_build(pos, now)
    }

    /// Advances RSVP on its timer (called from [`App::tick`]). True when
    /// the word changed.
    pub(crate) fn rsvp_tick(&mut self, now: Instant) -> bool {
        let Some(st) = self.rsvp.as_mut() else {
            return false;
        };
        let t = st.now(now);
        let tick = st.rsvp.tick(t);
        if tick.advanced {
            self.sync_cursor_to_rsvp();
            return true;
        }
        if tick.event == Some(RsvpEvent::Finished) {
            let range = st.rsvp.track().range();
            if self.rsvp_next_window(range, now).is_some()
                && let Some(st) = self.rsvp.as_mut()
            {
                let t = st.now(now);
                st.rsvp.play(t);
                self.sync_cursor_to_rsvp();
            } else {
                self.rsvp_announce(RsvpEvent::Finished);
            }
            return true;
        }
        false
    }

    /// Speech moved to `pos`: the RSVP word follows it.
    pub(crate) fn rsvp_follow(&mut self, pos: CharPos) {
        let Some(st) = self.rsvp.as_mut() else {
            return;
        };
        let range = st.rsvp.track().range();
        if !range.contains(pos) {
            self.rsvp_build(pos, Instant::now());
        } else {
            st.rsvp.follow(pos);
        }
    }

    /// `bionic_toggle`: saved.
    pub(crate) fn bionic_toggle(&mut self) {
        let on = !self.settings.reading_aids.bionic;
        self.settings.reading_aids.bionic = on;
        self.settings_dirty = true;
        self.tell(if on {
            "Bionic reading on."
        } else {
            "Bionic reading off."
        });
    }

    /// The ranges to draw in bold for bionic reading within `range`, or
    /// none when it is off.
    pub fn bionic_ranges(&self, range: CharRange) -> Vec<CharRange> {
        match self.session.as_ref() {
            Some(s) if self.settings.reading_aids.bionic => {
                bionic_range(&s.doc, range, &self.settings.reading_aids.bionic_options)
            }
            _ => Vec::new(),
        }
    }

    /// `syllables_toggle`: syllables shown or hidden; saved.
    pub(crate) fn syllables_toggle(&mut self) {
        let on = !self.settings.reading_aids.syllables;
        self.settings.reading_aids.syllables = on;
        self.settings_dirty = true;
        self.tell(if on {
            "Syllables shown."
        } else {
            "Syllables hidden."
        });
    }

    /// The syllable display of `range` (text and offset map), or `None`
    /// when syllables are off.
    pub fn syllable_display(&self, range: CharRange) -> Option<SplitText> {
        let s = self.session.as_ref()?;
        let a = &self.settings.reading_aids;
        a.syllables
            .then(|| split_range(&s.doc, range, &a.syllable_options))
    }

    /// Where the syllable separator ([`syllable_separator`](Self::syllable_separator))
    /// is drawn in `range`: before the char at each position, in order.
    /// Empty when syllables are off.
    pub fn syllable_breaks(&self, range: CharRange) -> Vec<CharPos> {
        let Some(split) = self.syllable_display(range) else {
            return Vec::new();
        };
        split
            .map
            .spans()
            .iter()
            .filter(|sp| sp.kind == SpanKind::Inserted)
            .map(|sp| sp.source.start)
            .collect()
    }

    /// What is drawn between syllables (a middle dot by default).
    pub fn syllable_separator(&self) -> &str {
        &self.settings.reading_aids.syllable_options.separator
    }

    /// `difficult_words_toggle`: difficult words marked or not; saved.
    pub(crate) fn difficult_words_toggle(&mut self) {
        let on = !self.settings.reading_aids.difficult_words;
        self.settings.reading_aids.difficult_words = on;
        self.settings_dirty = true;
        let msg = match (on, ScowlList::builtin().is_some()) {
            (true, true) => "Difficult words underlined.",
            (true, false) => "Difficult words on, but the word list is missing from this build.",
            (false, _) => "Difficult words not marked.",
        };
        self.tell(msg);
    }

    /// The difficult words in `range`, to underline; none when the setting
    /// is off.
    pub fn difficult_ranges(&self, range: CharRange) -> Vec<CharRange> {
        let (Some(s), Some(list)) = (self.session.as_ref(), ScowlList::builtin()) else {
            return Vec::new();
        };
        if !self.settings.reading_aids.difficult_words {
            return Vec::new();
        }
        difficult_range(&s.doc, range, list, &DifficultOptions::default())
    }

    /// ", difficult word" for a word move onto `word` at high verbosity
    /// with difficult words marked.
    pub(crate) fn difficult_word_note(&self, word: CharRange) -> Option<&'static str> {
        let high = self.settings.speech.verbosity >= textweaver_a11y::Verbosity::High;
        (high && self.difficult_ranges(word).iter().any(|r| r.start == word.start))
            .then_some(", difficult word")
    }

    /// `ruler_cycle`: off, current line, ruler; saved.
    pub(crate) fn ruler_cycle(&mut self) {
        let r = &mut self.settings.reading_aids.ruler;
        r.mode = match r.mode {
            RulerMode::Off => RulerMode::CurrentLine,
            RulerMode::CurrentLine => RulerMode::Ruler,
            RulerMode::Ruler => RulerMode::Off,
        };
        let msg = match r.mode {
            RulerMode::Off => "Reading ruler off.",
            RulerMode::CurrentLine => "Current line marked.",
            RulerMode::Ruler => "Reading ruler on.",
        };
        self.settings_dirty = true;
        self.tell(msg);
    }

    /// The reading ruler settings.
    pub fn ruler(&self) -> RulerSettings {
        self.settings.reading_aids.ruler
    }

    /// The terminal's share of the text spacing settings: blank rows and
    /// extra spaces.
    pub fn terminal_spacing(&self) -> TerminalSpacing {
        self.settings.reading_aids.spacing.terminal()
    }

    /// `reading_level`: the Flesch-Kincaid grade and reading ease of the
    /// selection, else the whole document.
    pub(crate) fn say_reading_level(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let (range, what) = match s.selection.filter(|r| !r.is_empty()) {
            Some(r) => (r, "Selection"),
            None => (CharRange::new(CharPos::ZERO, s.doc.end()), "Document"),
        };
        let msg = match reading_level(&s.doc, range) {
            Some(level) => format!("{what}: {}", level.summary()),
            None => "Not enough text to measure the reading level.".to_owned(),
        };
        self.tell(&msg);
    }
}
