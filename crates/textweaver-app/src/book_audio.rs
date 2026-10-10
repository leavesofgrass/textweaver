//! A DAISY talking book's recorded narration, played in place of speech
//! (`[reading] book_audio`, "auto" by default).
//!
//! The first continuous reading of a DAISY book finds its recording
//! ([`textweaver_formats::book_audio()`]): each phrase of the text with its
//! clips. Continuous reading is then planned as one utterance per phrase
//! with audio, and speech for the text between them, and the speech
//! service is told which utterances play the recording
//! ([`textweaver_speech::recorded`]). The highlight, pause, stop and the
//! reading windows work as for speech; a window ends at a phrase's end, so
//! no phrase is heard twice. Reading a word or a sentence on its own uses
//! speech. A book with no text reads its headings, each holding the
//! recording up to the next one.

use std::sync::Arc;

use textweaver_core::{CharPos, CharRange, Utterance, UtteranceId};
use textweaver_formats::BookAudio;
use textweaver_speech::{PauseAt, PlayerFactory, RecordedClip, RecordedPar, RecordedPlan};
use textweaver_store::BookAudio as Setting;
use textweaver_text::{Document, InlineSpeech, NarrationPolicy};

use crate::app::App;
use crate::playback::plan_with_written_pauses;

/// A document's recording, found once per text revision.
#[derive(Debug)]
pub(crate) struct Found {
    revision: u64,
    audio: Option<Arc<BookAudio>>,
}

/// The utterances for `range` of `doc`: one per phrase of `audio` (its
/// text, without the blank lines around it), speech planned as usual for
/// the text between, and the phrases that play the recording.
pub(crate) fn plan(
    doc: &Document,
    range: CharRange,
    policy: &NarrationPolicy,
    inline: &[InlineSpeech],
    audio: &BookAudio,
) -> (Vec<Utterance>, Vec<PauseAt>, Vec<RecordedPar>) {
    let range = range.clamp_to(doc.len_chars());
    let mut out = Vec::new();
    let mut pauses = Vec::new();
    let mut pars = Vec::new();
    let mut from = range.start;
    let mut speak = |from: CharPos, to: CharPos, out: &mut Vec<Utterance>| {
        if to > from {
            let (u, w) = plan_with_written_pauses(doc, CharRange::new(from, to), policy, inline);
            out.extend(u);
            pauses.extend(w);
        }
    };
    let first = audio.pars.partition_point(|p| p.range.end <= range.start);
    for p in &audio.pars[first..] {
        if p.range.start >= range.end {
            break;
        }
        let start = p.range.start.max(from);
        speak(from, start, &mut out);
        let end = p.range.end.min(range.end);
        let text = doc.slice(CharRange::new(start, end));
        let lead = text.chars().take_while(|c| c.is_whitespace()).count();
        let trimmed = text.trim();
        // shortcut: a phrase whose text is blank (an image with no
        // description) is not played; give it a stand-in text if books
        // turn out to have many.
        if !trimmed.is_empty() {
            let at = CharPos(start.0 + lead);
            out.push(Utterance::literal(trimmed, at));
            pars.push(RecordedPar {
                range: CharRange::new(at, end),
                clips: p
                    .clips
                    .iter()
                    .map(|c| RecordedClip {
                        file: c.file.clone(),
                        begin: c.begin,
                        end: c.end,
                    })
                    .collect(),
            });
        }
        from = end;
    }
    speak(from, range.end, &mut out);
    for (i, u) in out.iter_mut().enumerate() {
        u.id = UtteranceId {
            generation: 0,
            chunk: u32::try_from(i).unwrap_or(u32::MAX),
        };
    }
    // The recording carries its own pauses.
    pauses.retain(|p| {
        !pars
            .iter()
            .any(|r| r.range.start < p.after && p.after <= r.range.end)
    });
    (out, pauses, pars)
}

impl App {
    /// The open document's recording, when it is a DAISY book with one
    /// and `[reading] book_audio` is "auto". Found on the first reading
    /// (a second walk of the book) and kept until the text changes.
    pub(crate) fn book_audio(&mut self) -> Option<Arc<BookAudio>> {
        if self.settings.reading.book_audio == Setting::Speech {
            return None;
        }
        let s = self.session.as_ref()?;
        if s.doc.meta.format != "daisy" {
            return None;
        }
        if let Some(found) = &self.book_audio
            && found.revision == s.revision
        {
            return found.audio.clone();
        }
        let path = s.doc.meta.path.clone()?;
        let revision = s.revision;
        let audio = match textweaver_formats::book_audio(&path, &self.load_options()) {
            Ok(a) => a.map(Arc::new),
            Err(e) => {
                log::warn!("cannot read the book's audio: {e}");
                None
            }
        };
        self.book_audio = Some(Found {
            revision,
            audio: audio.clone(),
        });
        audio
    }

    /// Where a reading window ending at `end` should end: after the
    /// phrase `end` falls in, so no phrase is split between windows.
    pub(crate) fn window_end_at_phrase(&self, end: CharPos) -> CharPos {
        let Some(audio) = self.book_audio.as_ref().and_then(|f| f.audio.as_ref()) else {
            return end;
        };
        let i = audio.pars.partition_point(|p| p.range.end <= end);
        match audio.pars.get(i) {
            Some(p) if p.range.start < end => p.range.end,
            _ => end,
        }
    }

    /// Tells the speech service which utterances of the reading about to
    /// start play the recording (none for an ordinary reading).
    pub(crate) fn set_recorded(&mut self, pars: Vec<RecordedPar>) {
        self.recorded_reading = !pars.is_empty();
        if pars.is_empty() {
            if std::mem::take(&mut self.recorded_set) {
                self.speech.set_recorded(None);
            }
            return;
        }
        let player = self.clip_player();
        self.speech
            .set_recorded(Some(Arc::new(RecordedPlan { pars, player })));
        self.recorded_set = true;
    }

    /// The player for recorded audio: the one given to
    /// [`set_clip_player`](Self::set_clip_player), else the output device.
    fn clip_player(&mut self) -> PlayerFactory {
        self.clip_player
            .get_or_insert_with(|| {
                textweaver_engines::recorded_player(Arc::new(|p: &std::path::Path| {
                    textweaver_formats::archive::read_path(p)
                }))
            })
            .clone()
    }

    /// Sets the player for books' recorded audio. Tests pass
    /// `textweaver_engines::silent_recorded_player`, which plays nothing.
    pub fn set_clip_player(&mut self, player: PlayerFactory) {
        self.clip_player = Some(player);
    }

    /// True while a reading plays a book's recording.
    pub(crate) fn reading_recorded(&self) -> bool {
        self.recorded_reading && self.playback != crate::Playback::Idle
    }

    /// The Toggle Book Audio command: the recorded narration or speech. A
    /// continuous reading goes on from where it is, the new way.
    pub(crate) fn toggle_book_audio(&mut self) {
        let next = match self.settings.reading.book_audio {
            Setting::Auto => Setting::Speech,
            Setting::Speech => Setting::Auto,
        };
        self.settings.reading.book_audio = next;
        self.settings_dirty = true;
        let msg = self.msg(match next {
            Setting::Auto => "book-audio-on",
            Setting::Speech => "book-audio-off",
        });
        let continuous = self.playback == crate::Playback::Reading
            && self.reading == crate::playback::ReadKind::Continuous
            && self.route(textweaver_a11y::Channel::Reading).speak;
        match (continuous, self.reading_position()) {
            (true, Some(pos)) => {
                self.stop_speech();
                self.show(&msg);
                let _ = self.book_audio();
                let Some(s) = self.session.as_ref() else {
                    return;
                };
                let start = crate::text_util::word_start(&s.doc, pos);
                let (end, doc_end) = (crate::playback::window_end(&s.doc, start), s.doc.end());
                let end = self.window_end_at_phrase(end);
                let kind = crate::playback::ReadKind::Continuous;
                if self.read_range_led(CharRange::new(start, end), kind, Some(msg.as_str())) {
                    self.continue_from = (end < doc_end).then_some(end);
                } else {
                    self.tell(&msg);
                }
            }
            _ => self.tell(&msg),
        }
    }
}

#[cfg(test)]
mod tests;
