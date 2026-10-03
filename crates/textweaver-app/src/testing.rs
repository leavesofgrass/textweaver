//! Test support: a recording speech backend.
//!
//! [`RecordingBackend`] implements the public `SpeechBackend` trait, records
//! every utterance it is asked to speak, and emits `Started`, one `Word`
//! event per word (byte ranges into the spoken text, no audio clock), and
//! `Finished`, synchronously from `speak`. It plays silence between
//! utterances ([`Caps::SILENCE`]), so structural pauses never hold a test's
//! reading back; it records each pause instead. It stands in for Agent B's
//! `recording` backend until that is merged, and uses only the trait, so it
//! keeps working with B's speech service.

use std::sync::{Arc, Mutex, MutexGuard};

use textweaver_core::{CharRange, Utterance, UtteranceId, UtteranceKind};
use textweaver_speech::{
    Caps, EventSink, RawEvent, ServiceConfig, SpeechBackend, SpeechError, SpeechService, Voice,
    VoiceParams,
};

/// What a [`RecordingBackend`] was asked to do, shared with the test.
#[derive(Clone, Debug, Default)]
pub struct SpeechLog(Arc<Mutex<LogInner>>);

#[derive(Debug, Default)]
struct LogInner {
    utterances: Vec<Utterance>,
    stops: usize,
    params: Vec<VoiceParams>,
    /// Structural pauses: the utterance each follows, and its length.
    silences: Vec<(UtteranceId, u32)>,
}

impl SpeechLog {
    fn lock(&self) -> MutexGuard<'_, LogInner> {
        // A panic while holding the lock only happens in a failing test.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Every utterance spoken, oldest first.
    pub fn utterances(&self) -> Vec<Utterance> {
        self.lock().utterances.clone()
    }

    /// Document text utterances (not announcements or characters).
    pub fn text_utterances(&self) -> Vec<Utterance> {
        self.lock()
            .utterances
            .iter()
            .filter(|u| u.kind == UtteranceKind::Text)
            .cloned()
            .collect()
    }

    /// Source ranges of the document text utterances, in order.
    pub fn spoken_ranges(&self) -> Vec<CharRange> {
        self.text_utterances()
            .iter()
            .filter_map(Utterance::source_range)
            .collect()
    }

    /// Texts of every utterance (including announcements), in order.
    pub fn texts(&self) -> Vec<String> {
        self.lock()
            .utterances
            .iter()
            .map(|u| u.text.clone())
            .collect()
    }

    /// The structural pauses asked for, oldest first: the text of the
    /// utterance each follows, and its length in ms.
    pub fn silences(&self) -> Vec<(String, u32)> {
        let l = self.lock();
        l.silences
            .iter()
            .filter_map(|(id, ms)| {
                let u = l.utterances.iter().rev().find(|u| u.id == *id)?;
                Some((u.text.clone(), *ms))
            })
            .collect()
    }

    /// How many times speech was stopped.
    pub fn stops(&self) -> usize {
        self.lock().stops
    }

    /// The last voice parameters applied.
    pub fn params(&self) -> Option<VoiceParams> {
        self.lock().params.last().cloned()
    }

    /// Every set of voice parameters applied, oldest first.
    pub fn all_params(&self) -> Vec<VoiceParams> {
        self.lock().params.clone()
    }

    /// Forgets everything recorded so far.
    pub fn clear(&self) {
        let mut l = self.lock();
        l.utterances.clear();
        l.stops = 0;
        l.silences.clear();
    }
}

/// A backend that records calls and reports every word at once.
#[derive(Debug)]
pub struct RecordingBackend {
    log: SpeechLog,
    params: VoiceParams,
    id: &'static str,
}

impl RecordingBackend {
    /// A backend writing to `log`.
    pub fn new(log: SpeechLog) -> Self {
        RecordingBackend {
            log,
            params: VoiceParams::default(),
            id: "test-recording",
        }
    }

    /// The backend under another id (`test-recording` by default), to
    /// stand in for a second engine.
    pub fn with_id(mut self, id: &'static str) -> Self {
        self.id = id;
        self
    }
}

/// Byte ranges of the words (alphanumeric runs, with `'` and `-` inside).
fn word_bytes(text: &str) -> Vec<std::ops::Range<u32>> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut last_end = 0;
    for (i, c) in text.char_indices() {
        let inner = c.is_alphanumeric() || (start.is_some() && matches!(c, '\'' | '-'));
        match (inner, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, last_end));
                start = None;
            }
            _ => {}
        }
        if c.is_alphanumeric() {
            last_end = i + c.len_utf8();
        }
    }
    if let Some(s) = start {
        out.push((s, last_end));
    }
    out.into_iter()
        .filter_map(|(a, b)| Some(u32::try_from(a).ok()?..u32::try_from(b).ok()?))
        .collect()
}

impl SpeechBackend for RecordingBackend {
    fn id(&self) -> &'static str {
        self.id
    }

    fn capabilities(&self) -> Caps {
        Caps::WORD_EVENTS
            | Caps::PAUSE
            | Caps::PITCH
            | Caps::VOLUME
            | Caps::LIVE_RATE
            | Caps::SILENCE
    }

    fn silence_after(&mut self, id: UtteranceId, ms: u32) {
        self.log.lock().silences.push((id, ms));
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(vec![
            Voice {
                id: "test".into(),
                name: "Test voice".into(),
                ..Voice::default()
            },
            Voice {
                id: "second".into(),
                name: "Second voice".into(),
                ..Voice::default()
            },
        ])
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.params = params.clone();
        self.log.lock().params.push(params.clone());
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        self.params.rate.wpm()
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        self.log.lock().utterances.push(utterance.clone());
        sink.emit(utterance.id, RawEvent::Started);
        for byte_range in word_bytes(&utterance.text) {
            sink.emit(
                utterance.id,
                RawEvent::Word {
                    byte_range,
                    audio_ms: None,
                },
            );
        }
        sink.emit(utterance.id, RawEvent::Finished);
        Ok(())
    }

    fn stop(&mut self) {
        self.log.lock().stops += 1;
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        Ok(())
    }
}

/// A speech service running a [`RecordingBackend`], and its log.
pub fn recording_service() -> Result<(SpeechService, SpeechLog), SpeechError> {
    recording_service_as("test-recording")
}

/// [`recording_service`] with the backend under the id `id`.
pub fn recording_service_as(id: &'static str) -> Result<(SpeechService, SpeechLog), SpeechError> {
    let log = SpeechLog::default();
    let backend_log = log.clone();
    let service = SpeechService::spawn(
        Box::new(move || Ok(Box::new(RecordingBackend::new(backend_log).with_id(id)) as _)),
        ServiceConfig::default(),
    )?;
    Ok((service, log))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_alphanumeric_runs() {
        let t = "Hi, it's co-op. é!";
        let words: Vec<&str> = word_bytes(t)
            .into_iter()
            .map(|r| &t[r.start as usize..r.end as usize])
            .collect();
        assert_eq!(words, vec!["Hi", "it's", "co-op", "é"]);
    }
}
