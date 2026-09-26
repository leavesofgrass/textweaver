//! Reading aloud: starting, pausing, resuming, stopping, and following the
//! speech service's status updates.

use textweaver_a11y::{Priority, Verbosity};
use textweaver_core::{CharPos, CharRange, Unit};
use textweaver_speech::{SayMode, SpeechStatus};
use textweaver_text::narrate::NarrationPolicy;
use textweaver_text::units::unit_at;

use crate::app::{App, Mode};
use crate::command::Effect;
use crate::text_util;

/// Whether the app is reading aloud.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Playback {
    /// Not reading.
    #[default]
    Idle,
    /// Reading; the highlight follows speech.
    Reading,
    /// Paused; reading resumes at `resume_at` (the last confirmed word, or
    /// wherever the cursor was moved while paused).
    Paused {
        /// Where reading resumes.
        resume_at: Option<CharPos>,
    },
}

/// What the current reading is: continuous reading moves the cursor along
/// with speech; reading a unit in place ("say the sentence") does not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ReadKind {
    #[default]
    Continuous,
    InPlace,
}

/// Tells the current reading's status from stale status still in the channel.
///
/// The service stamps utterances with a generation that grows with every
/// stop or restart, but the Phase 0 contract does not tell the app which
/// generation its own `read` got (see the contract change request in the
/// wave 1 report). So the app:
///
/// 1. drains the channel before every read or stop and treats every
///    generation seen so far as stale (`floor`);
/// 2. adopts a newer generation as its own only when that generation's
///    first located position touches the first utterance it asked for
///    (`head`); a stale reading started elsewhere is rejected whole;
/// 3. attributes `Finished` (which carries no generation) to the generation
///    of the latest position, since the speech thread emits in order.
///
/// A later generation that continues forward from the adopted one is
/// accepted too, so a service that restarts internally (for example to
/// apply a rate change) keeps the highlight moving.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SpeechTrack {
    floor: u64,
    max_seen: u64,
    head: Option<CharRange>,
    current: Option<u64>,
    rejected: Option<u64>,
    last: Option<(u64, Verdict)>,
    last_accepted: Option<CharPos>,
    active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Ours,
    Stale,
    Undecided,
}

fn touches(a: CharRange, b: CharRange) -> bool {
    a.start <= b.end && b.start <= a.end
}

impl SpeechTrack {
    fn restart(&mut self, head: Option<CharRange>, active: bool) {
        self.floor = self.max_seen + 1;
        self.head = head;
        self.current = None;
        self.rejected = None;
        self.last = None;
        self.last_accepted = None;
        self.active = active;
    }

    fn saw(&mut self, g: u64) {
        self.max_seen = self.max_seen.max(g);
    }

    /// Whether a position belongs to the current reading.
    fn position(&mut self, g: u64, range: Option<CharRange>) -> bool {
        self.saw(g);
        let verdict = self.judge(g, range);
        self.last = Some((g, verdict));
        if verdict == Verdict::Ours {
            if let Some(r) = range {
                self.last_accepted = Some(r.start);
            }
        }
        verdict == Verdict::Ours && range.is_some()
    }

    fn judge(&mut self, g: u64, range: Option<CharRange>) -> Verdict {
        if !self.active || g < self.floor || Some(g) == self.rejected {
            return Verdict::Stale;
        }
        match self.current {
            Some(c) if g == c => return Verdict::Ours,
            Some(c) if g < c => return Verdict::Stale,
            _ => {}
        }
        let Some(r) = range else {
            return Verdict::Undecided;
        };
        let ours = match (self.current, self.last_accepted) {
            // A newer generation continuing forward from ours.
            (Some(_), Some(at)) => r.start >= at,
            _ => self.head.is_none_or(|h| touches(h, r)),
        };
        if ours {
            self.current = Some(g);
            Verdict::Ours
        } else {
            self.rejected = Some(g);
            Verdict::Stale
        }
    }

    /// Whether `Finished` ends the current reading. When it does, the
    /// finished generation becomes stale, and a newer reading is judged
    /// against the head again.
    fn finished(&mut self) -> bool {
        let done = self.active
            && matches!(
                self.last,
                Some((g, Verdict::Ours | Verdict::Undecided)) if g >= self.floor
            );
        if let (true, Some((g, _))) = (done, self.last) {
            self.floor = g + 1;
            self.current = None;
            self.rejected = None;
            self.last_accepted = None;
        }
        done
    }
}

impl App {
    /// Whether the app is reading, paused, or idle.
    pub fn playback(&self) -> Playback {
        self.playback
    }

    pub(crate) fn narration_policy(&self) -> NarrationPolicy {
        NarrationPolicy {
            skip_code: self.settings.speech.skip_code,
            verbosity: self.settings.speech.verbosity,
            ..NarrationPolicy::default()
        }
    }

    /// Takes stale status out of the channel before a restart, keeping only
    /// what the tracker needs (generations) and reporting backend errors.
    fn drain_stale(&mut self) {
        while let Some(status) = self.speech.try_status() {
            match status {
                SpeechStatus::Position { utterance, .. } => self.track.saw(utterance.generation),
                SpeechStatus::BackendError(e) => self.error(&format!("Speech error: {e}")),
                _ => {}
            }
        }
    }

    /// Stops speech and forgets the reading state, without announcing.
    pub(crate) fn stop_speech(&mut self) {
        if self.playback != Playback::Idle || self.track.active {
            self.drain_stale();
            self.speech.stop();
            self.track.restart(None, false);
        }
        self.playback = Playback::Idle;
        if let Some(s) = self.session.as_mut() {
            s.spoken = None;
            s.spoken_sentence = None;
        }
    }

    /// Reads `range` aloud. `kind` decides whether the cursor follows.
    /// Returns false when there was nothing to read.
    pub(crate) fn read_range(&mut self, range: CharRange, kind: ReadKind) -> bool {
        let policy = self.narration_policy();
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let utterances = textweaver_text::plan(&s.doc, range, &policy);
        if utterances.is_empty() {
            return false;
        }
        s.spoken = None;
        s.spoken_sentence = None;
        let head = utterances.iter().find_map(|u| u.source_range());
        self.drain_stale();
        self.speech.read(utterances);
        self.track.restart(head, true);
        self.playback = Playback::Reading;
        self.reading = kind;
        true
    }

    /// Reads continuously from `pos` to the end of the document.
    pub(crate) fn read_from(&mut self, pos: CharPos) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let start = text_util::word_start(&s.doc, pos);
        let range = CharRange::new(start, s.doc.end());
        if self.read_range(range, ReadKind::Continuous) {
            let rate = self.settings.speech.rate.wpm();
            self.show(&format!("Reading at {rate} words per minute."));
        } else {
            self.tell("End of document.");
        }
    }

    /// Reads continuously from the cursor.
    pub(crate) fn read_from_cursor(&mut self) {
        if let Some(pos) = self.session.as_ref().map(|s| s.cursor) {
            self.read_from(pos);
        }
    }

    /// Restarts reading at `pos` when the app is reading; moves the resume
    /// point when paused. Used by every navigation.
    pub(crate) fn follow_jump(&mut self, pos: CharPos) {
        match self.playback {
            Playback::Reading => self.read_from(pos),
            Playback::Paused { .. } => {
                self.playback = Playback::Paused {
                    resume_at: Some(pos),
                }
            }
            Playback::Idle => {}
        }
    }

    pub(crate) fn play_pause(&mut self) {
        match self.playback {
            Playback::Reading => {
                let resume_at = self.reading_position();
                self.drain_stale();
                self.speech.pause();
                self.track.restart(None, false);
                self.playback = Playback::Paused { resume_at };
                self.pause_origin = resume_at;
                self.note("Paused.");
            }
            Playback::Paused { resume_at } => {
                // Resume by re-reading from the last confirmed word: may
                // repeat a word, never skips one (ADR-0003), and works the
                // same on every backend.
                let pos = resume_at
                    .or_else(|| self.session.as_ref().map(|s| s.cursor))
                    .unwrap_or_default();
                self.playback = Playback::Idle;
                self.read_from(pos);
            }
            Playback::Idle => {
                if self.mode == Mode::SpeechCursor {
                    self.speech_cursor_reread();
                } else {
                    self.read_from_cursor();
                }
            }
        }
    }

    pub(crate) fn stop_action(&mut self) {
        let was_active = self.playback != Playback::Idle;
        self.stop_speech();
        if self.mode == Mode::SpeechCursor {
            self.leave_speech_cursor();
            self.note("Stopped. Speech Cursor off.");
            return;
        }
        if was_active {
            self.note("Stopped.");
        } else if let Some(s) = self.session.as_mut() {
            if s.find.take().is_some() {
                s.selection = None;
                self.note("Search cleared.");
            }
        }
    }

    pub(crate) fn read_current_character(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let pos = s.cursor;
        match s.doc.char_at(pos) {
            Some(c) if !c.is_whitespace() => {
                self.stop_speech();
                self.speech.speak_char(c, Some(pos));
                self.show(&text_util::char_name(c));
            }
            Some(c) => self.speak_content(&text_util::char_name(c)),
            None => self.speak_content("end of document"),
        }
    }

    /// Speaks text that is document content rather than an announcement
    /// ("blank", a character name): always spoken, whatever the voicing.
    pub(crate) fn speak_content(&mut self, text: &str) {
        self.stop_speech();
        self.speech.say(text, SayMode::Interrupt);
        self.show(text);
    }

    pub(crate) fn read_current_unit(&mut self, unit: Unit) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let Some(range) = unit_at(&s.doc, s.cursor, unit) else {
            self.tell(&format!("No {} here.", unit.spoken_name()));
            return;
        };
        self.stop_speech();
        if !self.read_range(range, ReadKind::InPlace) {
            self.speak_content("blank");
        }
    }

    pub(crate) fn read_current_line(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let range = text_util::line_range(&s.doc, s.line());
        self.read_line_range(range);
    }

    /// Reads one line in place, saying "blank" for an empty line.
    pub(crate) fn read_line_range(&mut self, range: CharRange) {
        let blank = self
            .session
            .as_ref()
            .is_none_or(|s| text_util::is_blank(&s.doc, range));
        self.stop_speech();
        if blank || !self.read_range(range, ReadKind::InPlace) {
            self.speak_content("blank");
        }
    }

    pub(crate) fn read_selection(&mut self) {
        let sel = self
            .session
            .as_ref()
            .and_then(|s| s.selection)
            .filter(|r| !r.is_empty());
        match sel {
            Some(r) => {
                self.stop_speech();
                if !self.read_range(r, ReadKind::InPlace) {
                    self.speak_content("blank");
                }
            }
            None => self.tell("No selection."),
        }
    }

    /// Drains speech status updates and applies them (highlight, cursor).
    pub fn poll_speech(&mut self) -> Vec<Effect> {
        let mut changed = false;
        while let Some(status) = self.speech.try_status() {
            changed |= self.apply_status(status);
        }
        if changed {
            vec![Effect::Redraw]
        } else {
            Vec::new()
        }
    }

    /// Applies at most one waiting status update. Returns `None` when none
    /// was waiting, else whether the display changed. Frontends that want to
    /// draw every highlight step (and tests that check each one) call this
    /// instead of [`App::poll_speech`].
    pub fn poll_speech_step(&mut self) -> Option<bool> {
        let status = self.speech.try_status()?;
        Some(self.apply_status(status))
    }

    /// Echoes typed or deleted prompt text when self-voicing (a screen
    /// reader echoes it otherwise): one character spoken as a character,
    /// longer text as a word.
    pub fn echo(&mut self, text: &str) {
        if !self.self_voicing || self.playback == Playback::Reading {
            return;
        }
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if !c.is_whitespace() => self.speech.speak_char(c, None),
            (Some(c), None) => self.speech.say(text_util::char_name(c), SayMode::Interrupt),
            (Some(_), Some(_)) => self.speech.say(text, SayMode::Interrupt),
            (None, _) => {}
        }
    }

    fn apply_status(&mut self, status: SpeechStatus) -> bool {
        match status {
            SpeechStatus::Position {
                utterance,
                source_range,
            } => {
                if !self.track.position(utterance.generation, source_range) {
                    return false;
                }
                if self.playback == Playback::Idle {
                    // A newer reading of ours after an older one finished.
                    self.playback = Playback::Reading;
                }
                if self.playback != Playback::Reading {
                    return false;
                }
                let Some(r) = source_range else {
                    return false;
                };
                self.set_spoken(r);
                true
            }
            SpeechStatus::Paused { resume_at } => {
                // The service knows the last confirmed word; prefer it unless
                // the user has moved since pausing.
                if let (Playback::Paused { resume_at: at }, Some(p)) = (self.playback, resume_at) {
                    if at == self.pause_origin {
                        self.playback = Playback::Paused { resume_at: Some(p) };
                        self.pause_origin = Some(p);
                    }
                }
                false
            }
            SpeechStatus::Finished => {
                if self.playback == Playback::Reading && self.track.finished() {
                    self.playback = Playback::Idle;
                    if let Some(s) = self.session.as_mut() {
                        s.spoken = None;
                        s.spoken_sentence = None;
                    }
                    self.say_at("Done reading.", Verbosity::High, Priority::Polite);
                    return true;
                }
                false
            }
            SpeechStatus::Stopped => false,
            SpeechStatus::BackendError(e) => {
                self.playback = Playback::Idle;
                self.error(&format!("Speech error: {e}"));
                true
            }
        }
    }

    fn set_spoken(&mut self, r: CharRange) {
        let follow = self.settings.reading.cursor_follows_speech
            && self.reading == ReadKind::Continuous
            && self.mode != Mode::SpeechCursor;
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let r = r.clamp_to(s.doc.len_chars());
        s.spoken = Some(r);
        let in_sentence = s.spoken_sentence.is_some_and(|x| x.contains_range(r));
        if !in_sentence {
            s.spoken_sentence = unit_at(&s.doc, r.start, Unit::Sentence)
                .filter(|x| x.intersects(r) || x.start == r.start)
                .map(|x| x.cover(r));
        }
        if follow {
            s.cursor = r.start;
        }
        if self.spoken_log.len() < Self::SPOKEN_LOG_LIMIT {
            self.spoken_log.push(r);
        }
        self.scroll_to_focus();
    }
}
