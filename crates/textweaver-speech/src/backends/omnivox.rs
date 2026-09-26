//! Omnivox speech-server backend (feature `omnivox`): a subprocess driven
//! with the Emacspeak speech-server protocol on its stdin.
//!
//! The protocol is write-only (the server prints nothing back), so this
//! backend has no word events. It estimates each utterance's duration from
//! its word count and the rate (`words × 60 / wpm`), reports `Started` and
//! `Finished` on that schedule from `poll`, and the service's timer pacer
//! moves the highlight (ADR-0003: word-level highlighting with Omnivox is
//! not promised).
//!
//! Commands sent (one per line):
//!
//! | When | Line |
//! |---|---|
//! | rate changes | `tts_set_speech_rate {wpm}` (wpm, 80..=900) |
//! | speak text | `q {text}` then `d` |
//! | speak one character | `l {c}` |
//! | stop | `s` |
//! | tone | `t {hz} {ms}` |
//!
//! Text is sent on one line inside braces, so newlines become spaces and
//! braces and backslashes, which the Tcl-style protocol would interpret,
//! become parentheses and spaces. Pitch, volume, and voice have no standard
//! protocol command and are not supported.
//!
//! The server is found as `omnivox` on `PATH`, or at the path in
//! `TEXTWEAVER_OMNIVOX`, with extra arguments from
//! `TEXTWEAVER_OMNIVOX_ARGS` (split on whitespace).

use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use textweaver_core::{Utterance, UtteranceId, UtteranceKind};

use crate::backend::{
    BackendId, Caps, EventSink, RawEvent, SpeechBackend, SpeechError, Voice, VoiceParams,
};
use crate::pacing::{Clock, SystemClock, spoken_words};

/// Rate range sent to the server, in wpm.
pub const RATE_RANGE: std::ops::RangeInclusive<u16> = 80..=900;

/// What the Omnivox backend can do: tones only (the protocol is write-only,
/// so there are no word events and the timer paces the highlight).
pub const CAPS: Caps = Caps::TONES;

/// How to start the Omnivox server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmnivoxCommand {
    /// The executable.
    pub program: PathBuf,
    /// Arguments.
    pub args: Vec<String>,
}

impl OmnivoxCommand {
    /// `TEXTWEAVER_OMNIVOX` (and `TEXTWEAVER_OMNIVOX_ARGS`) if set, else
    /// `omnivox` found on `PATH`; `None` when there is no server.
    pub fn from_env() -> Option<Self> {
        let args: Vec<String> = std::env::var("TEXTWEAVER_OMNIVOX_ARGS")
            .map(|a| a.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        if let Some(p) = std::env::var_os("TEXTWEAVER_OMNIVOX") {
            let program = PathBuf::from(p);
            return program
                .is_file()
                .then_some(OmnivoxCommand { program, args });
        }
        find_on_path("omnivox").map(|program| OmnivoxCommand { program, args })
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exe = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(&path)
        .map(|dir| dir.join(&exe))
        .find(|p| p.is_file())
}

/// Makes `text` safe for one protocol line inside braces.
pub fn escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '{' => '(',
            '}' => ')',
            '\\' | '\n' | '\r' | '\t' => ' ',
            c => c,
        })
        .collect()
}

#[derive(Debug)]
struct Scheduled {
    id: UtteranceId,
    start: Duration,
    end: Duration,
    started: bool,
}

/// The Omnivox backend.
pub struct OmnivoxBackend {
    writer: Box<dyn Write + Send>,
    child: Option<Child>,
    clock: Box<dyn Clock>,
    params: VoiceParams,
    schedule: VecDeque<Scheduled>,
    cancelled: Vec<UtteranceId>,
    rate_sent: Option<u16>,
}

impl std::fmt::Debug for OmnivoxBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OmnivoxBackend")
            .field("params", &self.params)
            .field("schedule", &self.schedule)
            .finish_non_exhaustive()
    }
}

impl OmnivoxBackend {
    /// Starts the server and talks to it on its stdin.
    pub fn spawn(cmd: &OmnivoxCommand) -> Result<Self, SpeechError> {
        Self::spawn_program(&cmd.program, &cmd.args)
    }

    /// Starts `program` with `args` as the server.
    pub fn spawn_program(program: &Path, args: &[String]) -> Result<Self, SpeechError> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                SpeechError::Unavailable("omnivox", format!("{}: {e}", program.display()))
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SpeechError::Io("omnivox stdin unavailable".into()))?;
        let mut b = Self::with_writer(Box::new(stdin), Box::new(SystemClock::default()));
        b.child = Some(child);
        Ok(b)
    }

    /// A backend writing protocol lines to `writer` (tests use a buffer or
    /// a pipe) and estimating durations on `clock`.
    pub fn with_writer(writer: Box<dyn Write + Send>, clock: Box<dyn Clock>) -> Self {
        OmnivoxBackend {
            writer,
            child: None,
            clock,
            params: VoiceParams::default(),
            schedule: VecDeque::new(),
            cancelled: Vec::new(),
            rate_sent: None,
        }
    }

    fn send(&mut self, line: &str) -> Result<(), SpeechError> {
        let r = writeln!(self.writer, "{line}").and_then(|()| self.writer.flush());
        r.map_err(|e| SpeechError::Io(format!("omnivox: {e}")))
    }

    fn rate(&self) -> u16 {
        self.params
            .rate
            .wpm()
            .clamp(*RATE_RANGE.start(), *RATE_RANGE.end())
    }

    fn duration_of(&self, u: &Utterance) -> Duration {
        let words = spoken_words(&u.text).len().max(1);
        let secs = words as f64 * 60.0 / f64::from(self.rate());
        Duration::from_secs_f64(secs)
    }
}

impl SpeechBackend for OmnivoxBackend {
    fn id(&self) -> BackendId {
        "omnivox"
    }

    fn capabilities(&self) -> Caps {
        CAPS
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(vec![Voice {
            id: "default".into(),
            name: "Omnivox default voice".into(),
            tags: vec!["no word timing".into()],
            ..Voice::default()
        }])
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.params = params.clone();
        let rate = self.rate();
        if self.rate_sent != Some(rate) {
            self.send(&format!("tts_set_speech_rate {rate}"))?;
            self.rate_sent = Some(rate);
        }
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        self.rate()
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        let mut chars = utterance.text.chars();
        match (utterance.kind, chars.next(), chars.next()) {
            (UtteranceKind::Character, Some(c), None) => {
                self.send(&format!("l {{{}}}", escape(&c.to_string())))?;
            }
            _ => {
                self.send(&format!("q {{{}}}", escape(&utterance.text)))?;
                self.send("d")?;
            }
        }
        let now = self.clock.now();
        let start = self.schedule.back().map_or(now, |s| s.end.max(now));
        let end = start + self.duration_of(utterance);
        self.schedule.push_back(Scheduled {
            id: utterance.id,
            start,
            end,
            started: false,
        });
        self.poll(sink);
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        for id in self.cancelled.drain(..) {
            sink.emit(id, RawEvent::Cancelled);
        }
        let now = self.clock.now();
        while let Some(front) = self.schedule.front_mut() {
            if !front.started && front.start <= now {
                front.started = true;
                sink.emit(front.id, RawEvent::Started);
            }
            if front.end > now {
                break;
            }
            let id = front.id;
            self.schedule.pop_front();
            sink.emit(id, RawEvent::Finished);
        }
    }

    fn stop(&mut self) {
        if let Err(e) = self.send("s") {
            log::warn!("{e}");
        }
        let ids: Vec<UtteranceId> = self.schedule.drain(..).map(|s| s.id).collect();
        self.cancelled.extend(ids);
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        if let Err(e) = self.send(&format!("t {} {ms}", hz.round())) {
            log::warn!("{e}");
        }
    }
}

impl Drop for OmnivoxBackend {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        // Closing stdin ends the server; give it a moment, then insist.
        self.writer = Box::new(std::io::sink());
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(_) => break,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use textweaver_core::{CharPos, Rate};

    use super::*;
    use crate::pacing::FakeClock;

    /// A shared in-memory pipe.
    #[derive(Clone, Default)]
    struct Pipe(Arc<Mutex<Vec<u8>>>);

    impl Write for Pipe {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Pipe {
        fn lines(&self) -> Vec<String> {
            String::from_utf8_lossy(&self.0.lock().unwrap_or_else(|p| p.into_inner()))
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }

    #[derive(Default)]
    struct Collect(Vec<(UtteranceId, RawEvent)>);

    impl EventSink for Collect {
        fn emit(&mut self, id: UtteranceId, event: RawEvent) {
            self.0.push((id, event));
        }
        fn is_current(&self, _: UtteranceId) -> bool {
            true
        }
    }

    fn utt(text: &str, chunk: u32) -> Utterance {
        let mut u = Utterance::literal(text, CharPos(0));
        u.id = UtteranceId {
            generation: 1,
            chunk,
        };
        u
    }

    #[test]
    fn protocol_lines() {
        let pipe = Pipe::default();
        let clock = FakeClock::new();
        let mut b = OmnivoxBackend::with_writer(Box::new(pipe.clone()), Box::new(clock));
        let mut sink = Collect::default();
        b.set_params(&VoiceParams {
            rate: Rate::Wpm(300),
            ..VoiceParams::default()
        })
        .unwrap();
        b.speak(&utt("Hello {world}\nnext \\ line", 0), &mut sink)
            .unwrap();
        let mut c = Utterance::character("x", None);
        c.id.chunk = 1;
        b.speak(&c, &mut sink).unwrap();
        b.tone(440.4, 50);
        b.stop();
        assert_eq!(
            pipe.lines(),
            [
                "tts_set_speech_rate 300",
                "q {Hello (world) next   line}",
                "d",
                "l {x}",
                "t 440 50",
                "s"
            ]
        );
        assert_eq!(b.capabilities(), Caps::TONES);
        assert!(b.pause().is_err(), "pause is emulated by the service");
    }

    #[test]
    fn estimated_timing_and_cancel() {
        let clock = FakeClock::new();
        let mut b = OmnivoxBackend::with_writer(Box::new(Pipe::default()), Box::new(clock.clone()));
        b.set_params(&VoiceParams {
            rate: Rate::Wpm(600), // 100 ms per word
            ..VoiceParams::default()
        })
        .unwrap();
        let mut sink = Collect::default();
        b.speak(&utt("one two three", 0), &mut sink).unwrap();
        b.speak(&utt("four five", 1), &mut sink).unwrap();
        assert_eq!(sink.0, [(utt("", 0).id, RawEvent::Started)]);
        clock.advance(Duration::from_millis(300));
        b.poll(&mut sink);
        assert_eq!(
            sink.0[1..],
            [
                (utt("", 0).id, RawEvent::Finished),
                (utt("", 1).id, RawEvent::Started)
            ]
        );
        b.stop();
        b.poll(&mut sink);
        assert_eq!(sink.0.last(), Some(&(utt("", 1).id, RawEvent::Cancelled)));
        clock.advance(Duration::from_secs(5));
        let n = sink.0.len();
        b.poll(&mut sink);
        assert_eq!(sink.0.len(), n, "nothing after stop");
    }

    #[test]
    fn rate_is_clamped_to_the_server_range() {
        let mut b =
            OmnivoxBackend::with_writer(Box::new(Pipe::default()), Box::new(FakeClock::new()));
        b.set_params(&VoiceParams {
            rate: Rate::Wpm(50),
            ..VoiceParams::default()
        })
        .unwrap();
        assert_eq!(b.effective_wpm(), 80);
    }

    #[test]
    fn a_missing_server_is_unavailable() {
        let err =
            OmnivoxBackend::spawn_program(Path::new("/no/such/omnivox-server"), &[]).unwrap_err();
        assert!(matches!(err, SpeechError::Unavailable("omnivox", _)));
    }
}
