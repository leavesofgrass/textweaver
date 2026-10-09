//! Reading aloud: starting, pausing, resuming, stopping, and following the
//! speech service's status updates.

use textweaver_a11y::{AccessMode, Channel, Priority, Verbosity};
use textweaver_core::{CharPos, CharRange, MarkerKind, Unit};
use textweaver_speech::{Caps, ReadingGeneration, SayMode, SpeechStatus};
use textweaver_text::narrate::NarrationPolicy;
use textweaver_text::units::unit_at;

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::{App, Mode};
use crate::command::Effect;
use crate::text_util;

/// Whether the app is reading aloud.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
/// reading, and every resume (the September 2026 audit, finding P1).
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
        ocr: ocr_options(&settings.reading),
        revisions: revision_mode(settings),
        name_skipped_commands: settings.speech.verbosity >= Verbosity::High,
        keep_pause_markup: !settings.speech.markup_pauses,
        brf_code: match settings.braille.brf_code {
            textweaver_store::BrfCode::Ueb => textweaver_formats::BrfCode::Ueb,
            textweaver_store::BrfCode::Ebae => textweaver_formats::BrfCode::Ebae,
        },
        ..textweaver_formats::LoadOptions::default()
    }
}

/// Where a planned reading's headings, paragraphs and list items end, as
/// the speech service's structural pauses (`[speech] pause_heading_ms` and
/// the others say how long each is).
pub fn structural_pauses(
    doc: &textweaver_text::Document,
    utterances: &[textweaver_core::Utterance],
) -> Vec<textweaver_speech::PauseAt> {
    use textweaver_speech::{PauseAt, PauseKind};
    use textweaver_text::BlockEnd;
    textweaver_text::block_ends(doc, utterances)
        .into_iter()
        .map(|(after, block)| PauseAt {
            after,
            kind: match block {
                BlockEnd::Heading => PauseKind::Heading,
                BlockEnd::Paragraph => PauseKind::Paragraph,
                BlockEnd::ListItem => PauseKind::ListItem,
            },
        })
        .collect()
}

/// [`textweaver_text::plan_with`], cut at every pause written as markup in
/// the document (`<break time="500ms"/>`, recorded by the loader), with a
/// pause of the written length after the utterance that ends at each
/// break. A document with no written pauses is planned in one piece, as
/// before. The pauses come after any structural pause at the same place
/// in the list, so the written length wins.
pub fn plan_with_written_pauses(
    doc: &textweaver_text::Document,
    range: textweaver_core::CharRange,
    policy: &textweaver_text::NarrationPolicy,
    inline: &[textweaver_text::InlineSpeech],
) -> (
    Vec<textweaver_core::Utterance>,
    Vec<textweaver_speech::PauseAt>,
) {
    use textweaver_core::{CharRange, UtteranceId, UtteranceKind};
    use textweaver_speech::{PauseAt, PauseKind};
    let range = range.clamp_to(doc.len_chars());
    let breaks: Vec<_> = textweaver_formats::pause_markup::written_pauses(&doc.meta)
        .into_iter()
        .filter(|p| p.at > range.start && p.at <= range.end)
        .collect();
    if breaks.is_empty() {
        return (
            textweaver_text::plan_with(doc, range, policy, inline),
            Vec::new(),
        );
    }
    let mut out = Vec::new();
    let mut pauses = Vec::new();
    let mut from = range.start;
    let mut cuts = breaks
        .iter()
        .map(|b| (b.at, Some(b.ms)))
        .collect::<Vec<_>>();
    cuts.push((range.end, None));
    for (to, ms) in cuts {
        if to > from {
            let part = textweaver_text::plan_with(doc, CharRange::new(from, to), policy, inline);
            out.extend(part);
            from = to;
        }
        // The pause follows the last text read before the break.
        let Some(ms) = ms else { continue };
        let end = out
            .iter()
            .rev()
            .filter(|u| u.kind == UtteranceKind::Text)
            .find_map(|u| u.source_range())
            .map(|r| r.end);
        if let Some(after) = end {
            pauses.push(PauseAt {
                after,
                kind: PauseKind::Written { ms },
            });
        }
    }
    for (i, u) in out.iter_mut().enumerate() {
        u.id = UtteranceId {
            generation: 0,
            chunk: u32::try_from(i).unwrap_or(u32::MAX),
        };
    }
    (out, pauses)
}

/// `[reading] revisions`: how tracked changes in Word, OpenDocument, and
/// RTF files are read. `auto` (the default) says each change in place
/// ("deleted by Ada Example: ...") at high verbosity and reads the final
/// text otherwise; `marked` always says them; `final` never does. Read
/// A document already open keeps the way it was loaded until it is opened
/// again.
fn revision_mode(settings: &textweaver_store::Settings) -> textweaver_formats::RevisionMode {
    use textweaver_formats::RevisionMode;
    use textweaver_store::RevisionReading;
    match settings.reading.revisions {
        RevisionReading::Marked => RevisionMode::Marked,
        RevisionReading::Final => RevisionMode::Final,
        RevisionReading::Auto if settings.speech.verbosity >= Verbosity::High => {
            RevisionMode::Marked
        }
        RevisionReading::Auto => RevisionMode::Final,
    }
}

/// OCR from `[reading]`: `ocr`, `ocr_lang`, and `ocr_engine` (ADR-0026).
fn ocr_options(reading: &textweaver_store::ReadingSettings) -> textweaver_formats::OcrOptions {
    use textweaver_formats::OcrEngineChoice as Choice;
    use textweaver_store::OcrEngine;
    textweaver_formats::OcrOptions {
        enabled: reading.ocr,
        lang: reading.ocr_lang.trim().to_owned(),
        engine: match reading.ocr_engine {
            OcrEngine::Auto => Choice::Auto,
            OcrEngine::Ocrs => Choice::Ocrs,
            OcrEngine::Tesseract => Choice::Tesseract,
            OcrEngine::Paddle => Choice::Paddle,
        },
    }
}

/// What a capability change means for the listener, or `None` when nothing
/// they would notice changed.
pub(crate) fn capability_message(c: &Catalog, old: Caps, new: Caps) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let lost = |x: Caps| old.contains(x) && !new.contains(x);
    let gained = |x: Caps| !old.contains(x) && new.contains(x);
    if lost(Caps::WORD_EVENTS) {
        parts.push(c.tr("playback-caps-no-words"));
    } else if gained(Caps::WORD_EVENTS) {
        parts.push(c.tr("playback-caps-words"));
    }
    if lost(Caps::PITCH) {
        parts.push(c.tr("playback-caps-no-pitch"));
    }
    if lost(Caps::VOLUME) {
        parts.push(c.tr("playback-caps-no-volume"));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

impl App {
    /// Whether the app is reading, paused, or idle.
    pub fn playback(&self) -> Playback {
        self.playback
    }

    /// True once anything has been read aloud in this run (continuously or
    /// in place).
    pub fn has_read(&self) -> bool {
        self.has_read
    }

    /// The reading state in one word, as the title line shows it:
    /// "Reading", "Paused", "Ready" before anything has been read in this
    /// run, and "Stopped" after. A screen reader reading the title line at
    /// startup hears "Ready", not "Stopped" (usability pass, item 2).
    pub fn reading_state(&self) -> &'static str {
        match self.playback {
            Playback::Reading => "Reading",
            Playback::Paused { .. } => "Paused",
            Playback::Idle if self.has_read => "Stopped",
            Playback::Idle => "Ready",
        }
    }

    /// [`reading_state`](Self::reading_state) in the interface's language.
    pub fn reading_state_text(&self) -> String {
        self.msg(match self.playback {
            Playback::Reading => "state-reading",
            Playback::Paused { .. } => "state-paused",
            Playback::Idle if self.has_read => "state-stopped",
            Playback::Idle => "state-ready",
        })
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
        self.screen_say_all = None;
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
        self.read_range_led(range, kind, None)
    }

    /// [`read_range`](Self::read_range), saying `lead` first (structure the
    /// narration does not say, such as "list item").
    pub(crate) fn read_range_led(
        &mut self,
        range: CharRange,
        kind: ReadKind,
        lead: Option<&str>,
    ) -> bool {
        let mut policy = self.narration_policy();
        // A reading pass skims continuous reading only: reading the
        // current sentence or paragraph still says it.
        let mut pass_lead = None;
        if kind == ReadKind::Continuous {
            policy.pass = self.reading_pass;
            if std::mem::take(&mut self.pass_lead_pending) {
                pass_lead = self.reading_pass_lead();
            }
        }
        let citations = self.citation_speech_in(range);
        let sp = &self.settings.speech;
        let pauses_on =
            sp.pause_heading_ms > 0 || sp.pause_paragraph_ms > 0 || sp.pause_list_item_ms > 0;
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let range = range.clamp_to(s.doc.len_chars());
        // Cut where the document has pauses written as markup.
        let (mut utterances, written) =
            plan_with_written_pauses(&s.doc, range, &policy, &citations);
        // Where headings, paragraphs and list items end, for the speech
        // service's structural pauses; a written pause at the same place
        // comes later, so it wins.
        let mut pauses = if pauses_on {
            structural_pauses(&s.doc, &utterances)
        } else {
            Vec::new()
        };
        pauses.extend(written);
        if let Some(lead) = lead.filter(|_| !utterances.is_empty()) {
            utterances.insert(0, textweaver_core::Utterance::announcement(lead));
        }
        if let Some(lead) = pass_lead.filter(|_| !utterances.is_empty()) {
            utterances.insert(0, textweaver_core::Utterance::announcement(&lead));
        }
        self.continue_from = None;
        if utterances.is_empty() {
            return false;
        }
        // Read in place without a voice for it (screen-reader mode, or
        // hybrid without an engine): the caller shows the text instead.
        if kind == ReadKind::InPlace && !self.route(Channel::Reading).speak {
            return true;
        }
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        s.spoken = None;
        s.spoken_sentence = None;
        let generation = self.speech.read_with_pauses(utterances, pauses);
        self.track.follow(generation);
        self.playback = Playback::Reading;
        self.has_read = true;
        self.reading = kind;
        self.planned_end = Some(range.end);
        true
    }

    /// Reads utterances planned elsewhere (from another document, such as
    /// the rendered text while editing) in place: the cursor does not
    /// follow, and Stop and Pause work as for any reading. Returns the
    /// reading's generation, or `None` when there is nothing to read.
    #[cfg_attr(not(feature = "publish"), allow(dead_code))]
    pub(crate) fn read_planned(
        &mut self,
        utterances: Vec<textweaver_core::Utterance>,
    ) -> Option<ReadingGeneration> {
        if utterances.is_empty() {
            return None;
        }
        self.stop_speech();
        let generation = self.speech.read(utterances);
        self.track.follow(generation);
        self.playback = Playback::Reading;
        self.has_read = true;
        self.reading = ReadKind::InPlace;
        self.continue_from = None;
        self.planned_end = None;
        Some(generation)
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
        // Reading was asked for, even in screen-reader mode or when the
        // rest is blank: the title line says "Stopped" from now on.
        self.has_read = true;
        self.pass_lead_pending = true;
        if self.screen_say_all_wanted() {
            // Screen-reader mode: a sentence at a time on the status line.
            if !self.start_screen_say_all(start, std::time::Instant::now()) {
                let msg = self.msg("nav-end-of-document-stop");
                self.tell(&msg);
            }
            return;
        }
        if self.read_window(start) {
            // The voice starting is the feedback; with a screen reader, or
            // a quiet screen, nothing is added to the status line.
            if self.access_mode == AccessMode::SelfVoicing && !self.quiet_screen() {
                let rate = self.settings.speech.rate.wpm();
                let msg = self.msg_args("playback-reading-at", &args!["rate" => rate]);
                self.show(&msg);
            }
        } else {
            let msg = self.msg("nav-end-of-document-stop");
            self.tell(&msg);
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
        if self.pause_screen_say_all() {
            return;
        }
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
                let msg = self.msg("playback-paused");
                self.note(&msg);
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
            let msg = self.msg("playback-stopped-speech-cursor-off");
            self.note(&msg);
            return;
        }
        if was_active {
            let msg = self.msg("playback-stopped");
            self.note(&msg);
        } else if let Some(s) = self.session.as_mut()
            && s.find.take().is_some()
        {
            s.selection = None;
            let msg = self.msg("playback-search-cleared");
            self.note(&msg);
        } else if self.mode == Mode::Edit {
            // Escape is Stop everywhere; in an editor people expect it to
            // leave, so say how to (usability pass, item 5).
            let finish = self.key(textweaver_keymap::ActionId::ToggleEditMode);
            let msg = self.msg_args("playback-still-editing", &args!["key" => finish]);
            self.tell(&msg);
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
                let route = self.route(Channel::Caret);
                if route.speak {
                    self.speech.speak_char(c, Some(pos));
                }
                if route.status {
                    let name = text_util::char_name_text(self.cat(), c);
                    self.show(&name);
                }
            }
            Some(c) => {
                let name = text_util::char_name_text(self.cat(), c);
                self.speak_content(Channel::Caret, &name);
            }
            None => {
                let text = self.msg("playback-end-of-document-content");
                self.speak_content(Channel::Caret, &text);
            }
        }
    }

    /// Says text that is document content rather than an announcement
    /// ("blank", a character name, the word a caret move reached): spoken
    /// and shown as `channel` goes in the accessibility mode (in
    /// self-voicing mode always spoken, whatever the voicing).
    pub(crate) fn speak_content(&mut self, channel: Channel, text: &str) {
        self.stop_speech();
        let route = self.route(channel);
        if route.speak {
            self.speech.say(text, SayMode::Interrupt);
        }
        if route.status {
            // A whole long line (a caret move onto a paragraph) is cut
            // for the status line as text read in place is; the voice
            // says all of it.
            self.show(&crate::access::status_cut(text));
        }
    }

    pub(crate) fn read_current_unit(&mut self, unit: Unit) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let Some(range) = unit_at(&s.doc, s.cursor, unit) else {
            let what = crate::words::unit_name(self.cat(), unit);
            let msg = self.msg_args(
                "playback-no-unit-here",
                &args!["what" => what, "unit" => crate::words::unit_key(unit)],
            );
            self.tell(&msg);
            return;
        };
        self.stop_speech();
        if !self.read_range(range, ReadKind::InPlace) {
            let blank = self.msg("nav-blank");
            self.speak_content(Channel::Caret, &blank);
        } else {
            self.show_read_text(range, false);
        }
    }

    /// Puts text read in place on the status line when the accessibility
    /// mode sends it there: in self-voicing mode always for lines (the
    /// Speech Cursor) and otherwise only without a voice; with a screen
    /// reader whenever textweaver does not read it aloud, narrated as
    /// textweaver would say it (tables, math in words).
    pub(crate) fn show_read_text(&mut self, range: CharRange, always: bool) {
        let channel = if always {
            Channel::Line
        } else {
            Channel::Reading
        };
        if !self.route(channel).status {
            return;
        }
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let text = if self.access_mode.uses_screen_reader() {
            self.narrated(range)
        } else {
            text_util::preview(&s.doc, range, 80)
        };
        let text = if text.is_empty() {
            self.msg("nav-blank")
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

    /// Reads one line in place, saying "blank" for an empty line, and its
    /// structure first ("list item", "heading level 2", "row 3"). The
    /// narration already says headings and table rows, so only the rest
    /// is added to what is spoken; the status line shows all of it.
    pub(crate) fn read_line_range(&mut self, range: CharRange) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let blank = text_util::is_blank(&s.doc, range);
        let structure = if self.settings.speech.verbosity >= textweaver_a11y::Verbosity::Normal {
            crate::app::App::line_structure(
                self.cat(),
                &s.doc,
                text_util::line_of(&s.doc, range.start),
            )
        } else {
            None
        };
        // The first item of a list is introduced by the narration ("list
        // with 3 items").
        let first_item = s
            .doc
            .marker_index()
            .enclosing(MarkerKind::List, range.start)
            .is_some_and(|l| l.range.start == range.start);
        let narrated = matches!(
            crate::app::App::line_kind(&s.doc, text_util::line_of(&s.doc, range.start)),
            Some(MarkerKind::Heading | MarkerKind::TableRow)
        );
        let lead = structure.as_deref().filter(|_| !narrated && !first_item);
        let lead = lead.map(str::to_owned);
        self.stop_speech();
        if blank || !self.read_range_led(range, ReadKind::InPlace, lead.as_deref()) {
            let blank = self.msg("nav-blank");
            self.speak_content(Channel::Line, &blank);
        } else {
            self.show_read_text(range, true);
            if let Some(kind) = structure.filter(|_| self.route(Channel::Line).status) {
                let shown = format!("{kind}, {}", self.status_text());
                self.show(&shown);
            }
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
                    let blank = self.msg("nav-blank");
                    self.speak_content(Channel::Caret, &blank);
                } else {
                    self.show_read_text(r, false);
                }
            }
            None => {
                let msg = self.msg("playback-no-selection");
                self.tell(&msg);
            }
        }
    }

    /// What the app does when its speech thread has died: reading stops,
    /// the error is shown on the status line (and through the announcer,
    /// which a screen reader or the JSON-RPC client hears; self-voicing
    /// cannot say it), and the silent service takes over so every other
    /// command keeps working. [`poll_speech`](Self::poll_speech) calls it
    /// when the speech service reports its thread dead
    /// (`SpeechService::poll_status` returns `ServiceStopped`).
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
        // Restarted automatically once, when the frontend said how
        // (crate::restart); the new service is swapped in on a tick.
        let next = self.restart_after_death(voiced);
        let msg = self.msg_args(
            "playback-speech-died",
            &args!["reason" => reason, "next" => next],
        );
        let msg = msg.trim_end().to_owned();
        self.error(&msg);
    }

    /// Waits until the speech thread has handled every command sent before
    /// this call (a round trip to it), so every status those commands
    /// produced is already waiting for [`poll_speech`](Self::poll_speech).
    /// Tests use it instead of sleeping.
    pub fn wait_for_speech_thread(&self) {
        let _ = self.speech.sync();
    }

    /// Drains speech status updates and applies them (highlight, cursor).
    pub fn poll_speech(&mut self) -> Vec<Effect> {
        let mut changed = false;
        loop {
            match self.speech.poll_status() {
                Ok(Some(status)) => {
                    // A dead thread's last error says why it died;
                    // `speech_thread_died` reports that below.
                    if !self.speech.is_alive() && matches!(status, SpeechStatus::BackendError(_)) {
                        continue;
                    }
                    changed |= self.apply_status(status);
                }
                Ok(None) => break,
                Err(_) => {
                    let reason = self
                        .speech
                        .failure()
                        .unwrap_or_else(|| "the speech thread ended".to_owned());
                    self.speech_thread_died(&reason);
                    changed = true;
                    break;
                }
            }
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
        if !self.route(Channel::Echo).speak || self.playback == Playback::Reading {
            return;
        }
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if !c.is_whitespace() => self.speech.speak_char(c, None),
            (Some(c), None) => {
                let name = text_util::char_name_text(self.cat(), c);
                self.speech.say(name, SayMode::Interrupt);
            }
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
                // Listening to the rendered text: its positions are in the
                // rendered document; the highlight follows in the source.
                let r = self.map_listened(generation, r);
                self.set_spoken(r);
                self.note_signal(r);
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
                    let msg = self.msg("playback-done-reading");
                    self.say_at(&msg, Verbosity::High, Priority::Polite);
                    return true;
                }
                false
            }
            SpeechStatus::Stopped { .. } => false,
            SpeechStatus::Restarted { generation, reason } => {
                // The service restarted the engine and reads on from the
                // last word; the reading (and the highlight) go on.
                if self.track.is_current(generation) && self.playback == Playback::Reading {
                    let msg =
                        self.msg_args("playback-speech-restarted", &args!["reason" => reason]);
                    self.say_at(&msg, Verbosity::Low, Priority::Assertive);
                    return true;
                }
                false
            }
            SpeechStatus::Capabilities { caps } => {
                let old = std::mem::replace(&mut self.speech_caps, caps);
                if let Some(msg) = capability_message(self.cat(), old, caps) {
                    self.tell(&msg);
                    return true;
                }
                false
            }
            SpeechStatus::BackendError(e) => {
                self.track.clear();
                self.playback = Playback::Idle;
                let msg = self.msg_args("playback-speech-error", &args!["error" => e.to_string()]);
                self.error(&msg);
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
    #[test]
    fn revisions_follow_the_setting_and_verbosity() {
        use textweaver_formats::RevisionMode;
        let mut settings = textweaver_store::Settings::default();
        assert_eq!(
            super::load_options(&settings).revisions,
            RevisionMode::Final
        );
        settings.speech.verbosity = textweaver_a11y::Verbosity::High;
        assert_eq!(
            super::load_options(&settings).revisions,
            RevisionMode::Marked
        );
        settings.reading.revisions = textweaver_store::RevisionReading::Final;
        assert_eq!(
            super::load_options(&settings).revisions,
            RevisionMode::Final
        );
        settings.speech.verbosity = textweaver_a11y::Verbosity::Low;
        settings.reading.revisions = textweaver_store::RevisionReading::Marked;
        assert_eq!(
            super::load_options(&settings).revisions,
            RevisionMode::Marked
        );
    }

    #[test]
    fn ocr_settings_come_from_the_reading_section() {
        let mut settings = textweaver_store::Settings::default();
        assert_eq!(
            super::load_options(&settings).ocr,
            textweaver_formats::OcrOptions::default()
        );
        settings.reading.ocr = false;
        settings.reading.ocr_lang = " fra+eng ".into();
        settings.reading.ocr_engine = textweaver_store::OcrEngine::Tesseract;
        let o = super::load_options(&settings).ocr;
        assert!(!o.enabled);
        assert_eq!(o.lang, "fra+eng");
        assert_eq!(o.engine, textweaver_formats::OcrEngineChoice::Tesseract);
    }

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
        let en = Catalog::english();
        let full = Caps::WORD_EVENTS | Caps::PITCH | Caps::VOLUME;
        assert_eq!(
            capability_message(&en, full, Caps::PITCH | Caps::VOLUME).as_deref(),
            Some("This voice does not report words, so the word highlight is estimated.")
        );
        assert_eq!(
            capability_message(&en, Caps::VOLUME, full).as_deref(),
            Some("This voice reports each word, so the highlight follows it exactly.")
        );
        assert_eq!(
            capability_message(&en, full, Caps::WORD_EVENTS).as_deref(),
            Some(
                "Pitch cannot be changed with this voice. Volume cannot be changed with this voice."
            )
        );
        assert_eq!(capability_message(&en, full, full | Caps::TONES), None);
    }
}
