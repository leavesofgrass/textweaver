//! Reading aloud: starting, pausing, resuming, stopping, and following the
//! speech service's status updates.

use textweaver_a11y::{Priority, Verbosity};
use textweaver_core::{CharPos, CharRange, MarkerKind, Unit};
use textweaver_speech::{Caps, ReadingGeneration, SayMode, SpeechStatus};
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

/// Which reading's statuses the app follows.
///
/// Every [`SpeechService::read`](textweaver_speech::SpeechService::read)
/// returns a [`ReadingGeneration`] that each `Position`, `Paused`,
/// `Stopped`, and `Finished` status about that reading carries. The app
/// keeps the generation of the reading it started last and drops every
/// status with another one, so a late word or "finished" from a reading
/// that was stopped, paused, or replaced can never move the highlight or
/// end a newer reading. (The wave 1 tracker had to guess from positions.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpeechTrack {
    /// The reading being followed.
    current: Option<ReadingGeneration>,
    /// A reading that was paused: only its `Paused` status is still wanted,
    /// for the service's resume point.
    paused: Option<ReadingGeneration>,
}

impl SpeechTrack {
    /// Follows a new reading.
    fn follow(&mut self, g: ReadingGeneration) {
        self.current = Some(g);
        self.paused = None;
    }

    /// Stops following anything.
    fn clear(&mut self) {
        *self = SpeechTrack::default();
    }

    /// The followed reading paused.
    fn pause(&mut self) {
        self.paused = self.current.take();
    }

    /// Whether a status of reading `g` belongs to the followed reading.
    fn is_current(&self, g: ReadingGeneration) -> bool {
        self.current == Some(g)
    }

    /// Whether a reading is being followed.
    fn active(&self) -> bool {
        self.current.is_some()
    }
}

/// How much text continuous reading plans at a time, in chars (about ten
/// minutes of speech). Planning the whole rest of a 10 MB document on the
/// UI thread took a quarter of a second on every Read, every jump while
/// reading, and every resume (docs/audit-2026-09.md, finding P1).
pub(crate) const READ_WINDOW: usize = 32_768;

/// The end of the reading window that starts at `start`: the end of the
/// sentence [`READ_WINDOW`] chars on, pushed past a table or code block it
/// falls in (they are narrated whole), or the end of the document.
pub(crate) fn window_end(doc: &textweaver_text::Document, start: CharPos) -> CharPos {
    let len = doc.len_chars();
    let target = start.0.saturating_add(READ_WINDOW);
    if target >= len {
        return doc.end();
    }
    let mut end = unit_at(doc, CharPos(target), Unit::Sentence)
        .map_or(CharPos(target), |r| r.end.max(CharPos(target)));
    let index = doc.marker_index();
    for kind in [MarkerKind::Table, MarkerKind::Code] {
        if let Some(m) = index.enclosing(kind, end) {
            end = end.max(m.range.end);
        }
    }
    end.clamp_to(len)
}

/// How documents are narrated with `settings`: code skipped or not, the
/// verbosity, and `[normalization] table_mode` (which was stored but never
/// used). Reading aloud and audio export use the same policy.
pub fn narration_policy(settings: &textweaver_store::Settings) -> NarrationPolicy {
    use textweaver_store::TableMode;
    use textweaver_text::narrate::TableNarration;
    NarrationPolicy {
        skip_code: settings.speech.skip_code,
        verbosity: settings.speech.verbosity,
        table_mode: match settings.normalization.table_mode {
            TableMode::Structured => TableNarration::Structured,
            TableMode::Flat => TableNarration::Flat,
            TableMode::Skip => TableNarration::Skip,
        },
        ..NarrationPolicy::default()
    }
}

/// How documents are loaded with `settings`: `[normalization]
/// footnote_mode` decides where footnotes are read (it was stored but never
/// used).
pub fn load_options(settings: &textweaver_store::Settings) -> textweaver_formats::LoadOptions {
    use textweaver_formats::FootnoteMode as Load;
    use textweaver_store::FootnoteMode;
    textweaver_formats::LoadOptions {
        footnotes: match settings.normalization.footnote_mode {
            FootnoteMode::Inline => Load::Inline,
            FootnoteMode::Deferred => Load::Deferred,
            FootnoteMode::Skip => Load::Skip,
        },
        ..textweaver_formats::LoadOptions::default()
    }
}

/// What a capability change means for the listener, or `None` when nothing
/// they would notice changed.
pub(crate) fn capability_message(old: Caps, new: Caps) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    let lost = |c: Caps| old.contains(c) && !new.contains(c);
    let gained = |c: Caps| !old.contains(c) && new.contains(c);
    if lost(Caps::WORD_EVENTS) {
        parts.push("This voice does not report words, so the word highlight is estimated.");
    } else if gained(Caps::WORD_EVENTS) {
        parts.push("This voice reports each word, so the highlight follows it exactly.");
    }
    if lost(Caps::PITCH) {
        parts.push("Pitch cannot be changed with this voice.");
    }
    if lost(Caps::VOLUME) {
        parts.push("Volume cannot be changed with this voice.");
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

impl App {
    /// Whether the app is reading, paused, or idle.
    pub fn playback(&self) -> Playback {
        self.playback
    }

    /// The speech backend's capabilities as last reported (they follow the
    /// selected voice).
    pub fn speech_capabilities(&self) -> Caps {
        self.speech_caps
    }

    pub(crate) fn narration_policy(&self) -> NarrationPolicy {
        narration_policy(&self.settings)
    }

    pub(crate) fn load_options(&self) -> textweaver_formats::LoadOptions {
        load_options(&self.settings)
    }

    /// Stops speech and forgets the reading state, without announcing.
    /// Statuses of the stopped reading still in the channel are dropped
    /// when they arrive (their generation is no longer followed).
    pub(crate) fn stop_speech(&mut self) {
        if self.playback != Playback::Idle || self.track.active() {
            self.speech.stop();
        }
        self.track.clear();
        self.playback = Playback::Idle;
        self.continue_from = None;
        self.planned_end = None;
        if let Some(s) = self.session.as_mut() {
            s.spoken = None;
            s.spoken_sentence = None;
        }
    }

    /// Where the text handed to the speech service for the current reading
    /// ends, while reading. Continuous reading is planned in windows of
    /// about ten minutes of speech, so this is usually well before the end
    /// of a long document; the next window is planned when this one ends.
    pub fn planned_reading_end(&self) -> Option<CharPos> {
        (self.playback == Playback::Reading)
            .then_some(self.planned_end)
            .flatten()
    }

    /// Reads `range` aloud. `kind` decides whether the cursor follows.
    /// Returns false when there was nothing to read.
    pub(crate) fn read_range(&mut self, range: CharRange, kind: ReadKind) -> bool {
        let policy = self.narration_policy();
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let range = range.clamp_to(s.doc.len_chars());
        let utterances = textweaver_text::plan(&s.doc, range, &policy);
        self.continue_from = None;
        if utterances.is_empty() {
            return false;
        }
        s.spoken = None;
        s.spoken_sentence = None;
        let generation = self.speech.read(utterances);
        self.track.follow(generation);
        self.playback = Playback::Reading;
        self.reading = kind;
        self.planned_end = Some(range.end);
        true
    }

    /// Reads the window starting at `start` continuously and notes where
    /// the next one starts. False when there was nothing left to read.
    fn read_window(&mut self, start: CharPos) -> bool {
        let mut from = start;
        loop {
            let Some(s) = self.session.as_ref() else {
                return false;
            };
            let doc_end = s.doc.end();
            let end = window_end(&s.doc, from);
            if self.read_range(CharRange::new(from, end), ReadKind::Continuous) {
                self.continue_from = (end < doc_end).then_some(end);
                return true;
            }
            // A window with nothing to say (blank, or skipped code only).
            if end >= doc_end {
                return false;
            }
            from = end;
        }
    }

    /// Reads continuously from `pos` to the end of the document, a window
    /// at a time.
    pub(crate) fn read_from(&mut self, pos: CharPos) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let start = text_util::word_start(&s.doc, pos);
        if self.read_window(start) {
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
            Playback::Reading
                if self.reading == ReadKind::InPlace && self.mode != Mode::SpeechCursor =>
            {
                // Saying one unit (the word, the sentence) is not a reading
                // to pause: Play means read on from the cursor (Agent K's
                // GUI finding: Play/Pause right after "read current word"
                // paused instead of playing).
                self.stop_speech();
                self.read_from_cursor();
            }
            Playback::Reading => {
                // Take in every position already reported, so the resume
                // point is the latest confirmed word. A `Finished` among
                // them is ignored: the user asked to pause.
                while let Some(status) = self.speech.try_status() {
                    match status {
                        SpeechStatus::Position {
                            generation,
                            source_range: Some(r),
                            ..
                        } if self.track.is_current(generation) => self.set_spoken(r),
                        s @ (SpeechStatus::Capabilities { .. } | SpeechStatus::BackendError(_)) => {
                            self.apply_status(s);
                        }
                        _ => {}
                    }
                }
                if self.playback != Playback::Reading {
                    // A backend error ended the reading meanwhile.
                    return;
                }
                let resume_at = self.reading_position();
                self.speech.pause();
                self.track.pause();
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

    /// Moves the cursor to `pos` quietly: no history entry, no
    /// announcement, no reading (a GUI caret click or a screen reader
    /// moving the caret). While paused, reading resumes from there. While
    /// reading, speech goes on (and the cursor follows it again when
    /// `cursor_follows_speech` is on).
    pub fn set_cursor(&mut self, pos: CharPos) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let pos = pos.clamp_to(s.doc.len_chars());
        s.cursor = pos;
        s.goal_column = None;
        if let Playback::Paused { .. } = self.playback {
            self.playback = Playback::Paused {
                resume_at: Some(pos),
            };
        }
        self.scroll_to_cursor();
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
        } else if let Some(s) = self.session.as_mut()
            && s.find.take().is_some()
        {
            s.selection = None;
            self.note("Search cleared.");
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
        } else {
            self.show_read_text(range, false);
        }
    }

    /// Puts text read in place on the status line: always for lines (the
    /// Speech Cursor), otherwise only without self-voicing, where a screen
    /// reader reads the status line and would hear nothing else
    /// (`--no-speech`).
    pub(crate) fn show_read_text(&mut self, range: CharRange, always: bool) {
        if !always && self.self_voicing {
            return;
        }
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let text = text_util::preview(&s.doc, range, 80);
        let text = if text.is_empty() {
            "blank".to_owned()
        } else {
            text
        };
        self.show(&text);
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
        } else {
            self.show_read_text(range, true);
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
                } else {
                    self.show_read_text(r, false);
                }
            }
            None => self.tell("No selection."),
        }
    }

    /// What the app does when its speech thread has died: reading stops,
    /// the error is shown on the status line (and through the announcer,
    /// which a screen reader or the JSON-RPC client hears; self-voicing
    /// cannot say it), and the silent service takes over so every other
    /// command keeps working. Frontends may call it; the app will once the
    /// speech service reports a dead thread.
    ///
    /// TODO(P1a speech thread death): Agent P1a is making a dead speech
    /// thread detectable in `textweaver-speech` (an `is_alive()` check or a
    /// fatal status). Its API was not on `main` when this was written, so
    /// nothing calls this yet: once it lands, check it in
    /// [`poll_speech`](Self::poll_speech) (or match the fatal status in
    /// `apply_status`) and call this, then offer a restart.
    pub fn speech_thread_died(&mut self, reason: &str) {
        self.track = SpeechTrack::default();
        self.playback = Playback::Idle;
        self.continue_from = None;
        self.planned_end = None;
        if let Some(s) = self.session.as_mut() {
            s.spoken = None;
            s.spoken_sentence = None;
        }
        self.speech = textweaver_speech::SpeechService::null();
        self.speech_caps = self.speech.capabilities();
        let voiced = std::mem::replace(&mut self.self_voicing, false);
        self.backend_name = "silent".into();
        let msg = if voiced {
            format!(
                "Speech stopped working ({reason}). textweaver is silent now; restart it to hear speech again."
            )
        } else {
            format!("Speech stopped working ({reason}).")
        };
        self.error(&msg);
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
                generation,
                source_range,
                ..
            } => {
                if !self.track.is_current(generation) || self.playback != Playback::Reading {
                    return false;
                }
                // `None` inside inserted speech ("heading level 2"): the
                // highlight stays where it was.
                let Some(r) = source_range else {
                    return false;
                };
                self.set_spoken(r);
                true
            }
            SpeechStatus::Paused {
                generation,
                resume_at,
            } => {
                // The service knows the last confirmed word; prefer it unless
                // the user has moved since pausing.
                if self.track.paused == Some(generation)
                    && let (Playback::Paused { resume_at: at }, Some(p)) =
                        (self.playback, resume_at)
                    && at == self.pause_origin
                {
                    self.playback = Playback::Paused { resume_at: Some(p) };
                    self.pause_origin = Some(p);
                }
                false
            }
            SpeechStatus::Finished { generation } => {
                if self.playback == Playback::Reading && self.track.is_current(generation) {
                    // The window is done: continuous reading goes on with
                    // the next one.
                    if self.reading == ReadKind::Continuous
                        && let Some(next) = self.continue_from.take()
                        && self.read_window(next)
                    {
                        return false;
                    }
                    self.track.clear();
                    self.planned_end = None;
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
            SpeechStatus::Stopped { .. } => false,
            SpeechStatus::Restarted { generation, reason } => {
                // The service restarted the engine and reads on from the
                // last word; the reading (and the highlight) go on.
                if self.track.is_current(generation) && self.playback == Playback::Reading {
                    self.say_at(
                        &format!("Speech restarted: {reason}. Reading on from the last word."),
                        Verbosity::Low,
                        Priority::Assertive,
                    );
                    return true;
                }
                false
            }
            SpeechStatus::Capabilities { caps } => {
                let old = std::mem::replace(&mut self.speech_caps, caps);
                if let Some(msg) = capability_message(old, caps) {
                    self.tell(&msg);
                    return true;
                }
                false
            }
            SpeechStatus::BackendError(e) => {
                self.track.clear();
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
        if self.rsvp.is_some() {
            self.rsvp_follow(r.start);
        }
        if self.spoken_log.len() < Self::SPOKEN_LOG_LIMIT {
            self.spoken_log.push(r);
        }
        self.scroll_to_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tracker_follows_one_reading() {
        let mut t = SpeechTrack::default();
        assert!(!t.active());
        t.follow(3);
        assert!(t.is_current(3) && !t.is_current(2) && !t.is_current(4));
        t.pause();
        assert!(!t.active() && t.paused == Some(3));
        t.follow(4);
        assert_eq!(t.paused, None);
        t.clear();
        assert!(!t.is_current(4));
    }

    #[test]
    fn capability_changes_read_well() {
        let full = Caps::WORD_EVENTS | Caps::PITCH | Caps::VOLUME;
        assert_eq!(
            capability_message(full, Caps::PITCH | Caps::VOLUME).as_deref(),
            Some("This voice does not report words, so the word highlight is estimated.")
        );
        assert_eq!(
            capability_message(Caps::VOLUME, full).as_deref(),
            Some("This voice reports each word, so the highlight follows it exactly.")
        );
        assert_eq!(
            capability_message(full, Caps::WORD_EVENTS).as_deref(),
            Some(
                "Pitch cannot be changed with this voice. Volume cannot be changed with this voice."
            )
        );
        assert_eq!(capability_message(full, full | Caps::TONES), None);
    }
}
