//! The silent backend: accepts everything, speaks nothing, finishes at once.

use textweaver_core::{Rate, Utterance};

use crate::backend::{
    BackendId, Caps, EventSink, RawEvent, SpeechBackend, SpeechError, Voice, VoiceParams,
};

/// Silent backend; the final fallback.
#[derive(Clone, Debug, Default)]
pub struct NullBackend {
    params: VoiceParams,
}

impl SpeechBackend for NullBackend {
    fn id(&self) -> BackendId {
        "null"
    }

    fn capabilities(&self) -> Caps {
        Caps::PAUSE | Caps::PITCH | Caps::VOLUME | Caps::LIVE_RATE
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(vec![Voice {
            id: "null".into(),
            name: "Silent".into(),
            ..Voice::default()
        }])
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.params = params.clone();
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        match self.params.rate {
            Rate::Wpm(w) => w,
        }
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        sink.emit(utterance.id, RawEvent::Started);
        sink.emit(utterance.id, RawEvent::Finished);
        Ok(())
    }

    fn stop(&mut self) {}

    fn pause(&mut self) -> Result<(), SpeechError> {
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        Ok(())
    }
}
