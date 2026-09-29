//! Whisper models, transcript segments, and dictation events.

use std::fmt;

use serde::Serialize;

/// The Whisper model sizes offered, smallest first (Star's
/// `WHISPER_MODELS`, `star/settings.py:16`).
pub const WHISPER_MODELS: [&str; 6] = [
    "tiny",
    "base",
    "small",
    "medium",
    "large-v3",
    "large-v3-turbo",
];

/// A Whisper model size. Larger models are slower and more accurate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WhisperModel {
    /// `tiny`: fastest, least accurate.
    Tiny,
    /// `base`: Star's default.
    #[default]
    Base,
    /// `small`.
    Small,
    /// `medium`.
    Medium,
    /// `large-v3`: most accurate, slowest.
    LargeV3,
    /// `large-v3-turbo`: nearly as accurate as `large-v3`, much faster.
    LargeV3Turbo,
}

impl WhisperModel {
    /// Every model, in [`WHISPER_MODELS`] order.
    pub const ALL: [WhisperModel; 6] = [
        WhisperModel::Tiny,
        WhisperModel::Base,
        WhisperModel::Small,
        WhisperModel::Medium,
        WhisperModel::LargeV3,
        WhisperModel::LargeV3Turbo,
    ];

    /// The model's name, as Whisper's command lines take it: `large-v3`.
    pub fn as_str(self) -> &'static str {
        match self {
            WhisperModel::Tiny => "tiny",
            WhisperModel::Base => "base",
            WhisperModel::Small => "small",
            WhisperModel::Medium => "medium",
            WhisperModel::LargeV3 => "large-v3",
            WhisperModel::LargeV3Turbo => "large-v3-turbo",
        }
    }

    /// Parses a model name, ignoring case and surrounding space.
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        WhisperModel::ALL.into_iter().find(|m| m.as_str() == name)
    }

    /// whisper.cpp's file for this model: `ggml-base.bin`.
    pub fn ggml_file(self) -> String {
        format!("ggml-{}.bin", self.as_str())
    }
}

impl fmt::Display for WhisperModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One stretch of transcribed speech.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Segment {
    /// Start, in milliseconds from the start of the audio.
    pub start_ms: u64,
    /// End, in milliseconds.
    pub end_ms: u64,
    /// The words, trimmed.
    pub text: String,
}

/// A finished transcription.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Transcript {
    /// The segments in order.
    pub segments: Vec<Segment>,
}

impl Transcript {
    /// The whole text: segments joined with single spaces.
    pub fn text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// One line per segment, each starting with its time, as Star's
    /// `transcribe_timestamps` option wrote it: `[01:05] text`, or
    /// `[01:02:03] text` past an hour.
    pub fn text_with_timestamps(&self) -> String {
        self.segments
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .map(|s| format!("{} {}", format_timestamp(s.start_ms), s.text.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Number of words in the text.
    pub fn word_count(&self) -> usize {
        self.segments
            .iter()
            .map(|s| s.text.split_whitespace().count())
            .sum()
    }

    /// True when nothing was recognized.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|s| s.text.trim().is_empty())
    }
}

/// `[mm:ss]`, or `[hh:mm:ss]` from an hour on (Star's `_fmt_timestamp`).
pub fn format_timestamp(ms: u64) -> String {
    let s = ms / 1000;
    let (h, rem) = (s / 3600, s % 3600);
    let (m, sec) = (rem / 60, rem % 60);
    if h > 0 {
        format!("[{h:02}:{m:02}:{sec:02}]")
    } else {
        format!("[{m:02}:{sec:02}]")
    }
}

/// What a dictation session reports, in order, through `poll`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum DictationEvent {
    /// The microphone is open and recording.
    Recording,
    /// Whisper is working on the audio.
    Transcribing,
    /// A segment Whisper has finished (not yet final: a later segment may
    /// still arrive).
    Partial(Segment),
    /// Live dictation: words committed while the speaker is still talking
    /// (or at the pause that ends an utterance). Committed words are
    /// stable: never withdrawn or changed, and the utterance's text in
    /// the final transcript is exactly its committed words, in order.
    /// Spoken commands are not applied to them; they apply to the final
    /// text only.
    Committed {
        /// The newly committed words, joined with single spaces; never
        /// empty.
        text: String,
        /// Which utterance of the session they belong to, from 0.
        utterance: usize,
        /// True for the words committed at the utterance's pause (its
        /// last burst), false for words agreed on while it went on.
        at_pause: bool,
    },
    /// The session finished with this transcript.
    Final(Transcript),
    /// The session was cancelled; nothing more follows.
    Cancelled,
    /// The session failed; the message is written for the student.
    Failed {
        /// What went wrong and what to do about it.
        message: String,
    },
}

impl DictationEvent {
    /// What to announce for this event, if anything. Partial segments and
    /// committed words are not announced here (the app shows them, and
    /// decides when to speak them).
    pub fn announcement(&self) -> Option<String> {
        match self {
            DictationEvent::Recording => Some("Recording".to_owned()),
            DictationEvent::Transcribing => Some("Transcribing, this may take a while".to_owned()),
            DictationEvent::Partial(_) | DictationEvent::Committed { .. } => None,
            DictationEvent::Final(t) if t.is_empty() => {
                Some("Dictation produced no text".to_owned())
            }
            DictationEvent::Final(t) => Some(match t.word_count() {
                1 => "Transcribed 1 word".to_owned(),
                n => format!("Transcribed {n} words"),
            }),
            DictationEvent::Cancelled => Some("Dictation cancelled".to_owned()),
            DictationEvent::Failed { message } => Some(message.clone()),
        }
    }

    /// True for the events that end a session.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DictationEvent::Final(_) | DictationEvent::Cancelled | DictationEvent::Failed { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_names_match_star() {
        let names: Vec<&str> = WhisperModel::ALL.iter().map(|m| m.as_str()).collect();
        assert_eq!(names, WHISPER_MODELS);
        assert_eq!(
            WhisperModel::parse(" Large-V3 "),
            Some(WhisperModel::LargeV3)
        );
        assert_eq!(WhisperModel::parse("huge"), None);
        assert_eq!(WhisperModel::default(), WhisperModel::Base);
        assert_eq!(
            WhisperModel::LargeV3Turbo.ggml_file(),
            "ggml-large-v3-turbo.bin"
        );
    }

    #[test]
    fn timestamps_like_star() {
        assert_eq!(format_timestamp(0), "[00:00]");
        assert_eq!(format_timestamp(65_400), "[01:05]");
        assert_eq!(format_timestamp(3_723_000), "[01:02:03]");
    }

    #[test]
    fn transcript_text_forms() {
        let t = Transcript {
            segments: vec![
                Segment {
                    start_ms: 0,
                    end_ms: 2000,
                    text: " Hello there.".into(),
                },
                Segment {
                    start_ms: 2000,
                    end_ms: 3000,
                    text: " ".into(),
                },
                Segment {
                    start_ms: 61_000,
                    end_ms: 62_000,
                    text: "General Kenobi.".into(),
                },
            ],
        };
        assert_eq!(t.text(), "Hello there. General Kenobi.");
        assert_eq!(
            t.text_with_timestamps(),
            "[00:00] Hello there.\n[01:01] General Kenobi."
        );
        assert_eq!(t.word_count(), 4);
        assert!(!t.is_empty());
        assert!(Transcript::default().is_empty());
    }

    #[test]
    fn announcements_read_well() {
        assert_eq!(
            DictationEvent::Final(Transcript::default())
                .announcement()
                .as_deref(),
            Some("Dictation produced no text")
        );
        let one = Transcript {
            segments: vec![Segment {
                text: "Hi".into(),
                ..Segment::default()
            }],
        };
        assert_eq!(
            DictationEvent::Final(one).announcement().as_deref(),
            Some("Transcribed 1 word")
        );
        assert!(
            DictationEvent::Partial(Segment::default())
                .announcement()
                .is_none()
        );
        assert!(DictationEvent::Cancelled.is_terminal());
        assert!(!DictationEvent::Transcribing.is_terminal());
    }
}
