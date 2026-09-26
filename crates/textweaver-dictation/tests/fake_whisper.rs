//! The Whisper backend against a fake Whisper program
//! (`src/bin/fake_whisper.rs`) in each engine's command-line style.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use textweaver_dictation::{
    BufferCapture, Dictation, DictationError, DictationEvent, DictationInput, DictationState, Pcm,
    Transcript, WhisperConfig, WhisperDictation, WhisperEngine, WhisperModel,
};

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tw-fake-whisper"))
}

struct Setup {
    dir: tempfile::TempDir,
}

impl Setup {
    fn new() -> Self {
        Setup {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// A script "audio" file.
    fn audio(&self, name: &str, script: &str) -> PathBuf {
        let p = self.path().join(name);
        std::fs::write(&p, script).unwrap();
        p
    }

    fn config(&self, engine: WhisperEngine) -> WhisperConfig {
        let mut c = WhisperConfig::new(engine, fake(), WhisperModel::Base);
        let models = self.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("ggml-base.bin"), "fake model").unwrap();
        c.model_dirs = vec![models];
        let work = self.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        c.work_dir = Some(work);
        c.env.push((
            "TW_FAKE_WHISPER_ARGS".into(),
            self.path().join("args.txt").into_os_string(),
        ));
        c
    }

    fn recorded_args(&self) -> Vec<String> {
        std::fs::read_to_string(self.path().join("args.txt"))
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Temporary session folders left behind.
    fn leftovers(&self) -> usize {
        std::fs::read_dir(self.path().join("work")).unwrap().count()
    }
}

/// Polls until a terminal event or the timeout.
fn run_to_end(d: &mut WhisperDictation) -> Vec<DictationEvent> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut events = Vec::new();
    loop {
        let batch = d.poll();
        let done = batch.iter().any(DictationEvent::is_terminal);
        events.extend(batch);
        if done {
            return events;
        }
        assert!(Instant::now() < deadline, "no final event: {events:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn final_transcript(events: &[DictationEvent]) -> Transcript {
    match events.last() {
        Some(DictationEvent::Final(t)) => t.clone(),
        other => panic!("expected Final, got {other:?} in {events:?}"),
    }
}

fn partial_texts(events: &[DictationEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            DictationEvent::Partial(s) => Some(s.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn whisper_cpp_file_transcription() {
    let s = Setup::new();
    let audio = s.audio(
        "lecture.wav",
        "Cells are the unit of life.\nMitochondria make energy.\n",
    );
    let mut d = WhisperDictation::new(s.config(WhisperEngine::Cpp));
    assert_eq!(d.name(), "whisper.cpp");
    d.start(DictationInput::File(audio.clone())).unwrap();
    assert_eq!(d.state(), DictationState::Transcribing);
    let events = run_to_end(&mut d);
    assert_eq!(events[0], DictationEvent::Transcribing);
    assert_eq!(
        partial_texts(&events),
        vec!["Cells are the unit of life.", "Mitochondria make energy."]
    );
    let t = final_transcript(&events);
    assert_eq!(
        t.text(),
        "Cells are the unit of life. Mitochondria make energy."
    );
    assert_eq!(t.segments[1].start_ms, 2000);
    assert_eq!(t.segments[1].end_ms, 4000);
    assert_eq!(
        t.text_with_timestamps(),
        "[00:00] Cells are the unit of life.\n[00:02] Mitochondria make energy."
    );
    assert_eq!(d.state(), DictationState::Idle);

    let args = s.recorded_args();
    let model = s.path().join("models").join("ggml-base.bin");
    assert_eq!(
        args[0..4],
        [
            "-m".to_owned(),
            model.to_string_lossy().into_owned(),
            "-f".to_owned(),
            audio.to_string_lossy().into_owned()
        ]
    );
    assert!(args.windows(2).any(|w| w == ["-l", "auto"]));
    assert!(args.contains(&"-oj".to_owned()));
    assert_eq!(s.leftovers(), 0, "the session folder is removed");
}

#[test]
fn openai_and_faster_file_transcription() {
    for (engine, name) in [
        (WhisperEngine::OpenAi, "OpenAI Whisper"),
        (WhisperEngine::Faster, "faster-whisper"),
    ] {
        let s = Setup::new();
        let audio = s.audio("talk.mp3", "Hello there.\nÇa va très bien.\n");
        let mut c = s.config(engine);
        c.model = WhisperModel::Small;
        c.language = Some("fr".into());
        let mut d = WhisperDictation::new(c);
        assert_eq!(d.name(), name);
        d.start(DictationInput::File(audio)).unwrap();
        let events = run_to_end(&mut d);
        assert_eq!(
            final_transcript(&events).text(),
            "Hello there. Ça va très bien."
        );
        let args = s.recorded_args();
        assert!(
            args.windows(2).any(|w| w == ["--model", "small"]),
            "{args:?}"
        );
        assert!(
            args.windows(2).any(|w| w == ["--language", "fr"]),
            "{args:?}"
        );
        assert_eq!(
            args.contains(&"--fp16".to_owned()),
            engine == WhisperEngine::OpenAi
        );
    }
}

#[test]
fn printed_segments_stand_in_when_no_json_is_written() {
    let s = Setup::new();
    let audio = s.audio("a.wav", "!nojson\nOne.\nTwo.\n");
    let mut d = WhisperDictation::new(s.config(WhisperEngine::Cpp));
    d.start(DictationInput::File(audio)).unwrap();
    assert_eq!(final_transcript(&run_to_end(&mut d)).text(), "One. Two.");
}

#[test]
fn a_failing_whisper_reports_its_error() {
    let s = Setup::new();
    let audio = s.audio("a.wav", "First.\n!fail error: out of memory\n");
    let mut d = WhisperDictation::new(s.config(WhisperEngine::Cpp));
    d.start(DictationInput::File(audio)).unwrap();
    let events = run_to_end(&mut d);
    match events.last() {
        Some(DictationEvent::Failed { message }) => {
            assert_eq!(message, "Whisper failed: error: out of memory");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(s.leftovers(), 0);
}

#[test]
fn cancel_kills_whisper_and_nothing_follows() {
    let s = Setup::new();
    let audio = s.audio("a.wav", "Before.\n!sleep 20000\nAfter.\n");
    let mut d = WhisperDictation::new(s.config(WhisperEngine::Cpp));
    d.start(DictationInput::File(audio)).unwrap();
    // Wait for the first segment so the process is surely running.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut seen = Vec::new();
    while !seen.iter().any(|e| matches!(e, DictationEvent::Partial(_))) {
        seen.extend(d.poll());
        assert!(Instant::now() < deadline, "{seen:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
    let started = Instant::now();
    d.cancel();
    assert_eq!(d.state(), DictationState::Idle);
    assert_eq!(d.poll(), vec![DictationEvent::Cancelled]);
    // The worker winds down quickly because the process was killed, and
    // its late events are dropped.
    while s.leftovers() > 0 {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the process was not killed"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(100));
    assert!(d.poll().is_empty());
    // A new session works after a cancelled one.
    let again = s.audio("b.wav", "Again.\n");
    d.start(DictationInput::File(again)).unwrap();
    assert_eq!(final_transcript(&run_to_end(&mut d)).text(), "Again.");
}

#[test]
fn capture_records_then_transcribes_on_stop() {
    let s = Setup::new();
    let mut d = WhisperDictation::new(s.config(WhisperEngine::OpenAi));
    let pcm = Pcm::new(16_000, vec![0; 16_000]);
    d.start(DictationInput::Capture(Box::new(BufferCapture::new(pcm))))
        .unwrap();
    assert_eq!(d.state(), DictationState::Recording);
    assert_eq!(d.poll(), vec![DictationEvent::Recording]);
    // Starting again while recording is refused.
    assert!(matches!(
        d.start(DictationInput::File(s.path().join("x.wav"))),
        Err(DictationError::Busy)
    ));
    d.stop().unwrap();
    assert_eq!(d.state(), DictationState::Transcribing);
    let events = run_to_end(&mut d);
    assert_eq!(events[0], DictationEvent::Transcribing);
    let t = final_transcript(&events);
    assert_eq!(t.text(), "Captured 16000 samples at 16000 hertz.");
    assert_eq!(
        DictationEvent::Final(t).announcement().as_deref(),
        Some("Transcribed 6 words")
    );
}

#[test]
fn an_empty_recording_says_so() {
    let s = Setup::new();
    let mut d = WhisperDictation::new(s.config(WhisperEngine::OpenAi));
    d.start(DictationInput::Capture(Box::new(BufferCapture::new(
        Pcm::default(),
    ))))
    .unwrap();
    d.stop().unwrap();
    let events = d.poll();
    assert_eq!(
        events.last(),
        Some(&DictationEvent::Failed {
            message: "No audio was recorded. Check your microphone.".into()
        })
    );
    assert_eq!(d.state(), DictationState::Idle);
}

#[test]
fn cancel_while_recording_discards() {
    let s = Setup::new();
    let mut d = WhisperDictation::new(s.config(WhisperEngine::OpenAi));
    d.start(DictationInput::Capture(Box::new(BufferCapture::new(
        Pcm::new(16_000, vec![1; 10]),
    ))))
    .unwrap();
    d.cancel();
    assert_eq!(
        d.poll(),
        vec![DictationEvent::Recording, DictationEvent::Cancelled]
    );
    assert_eq!(d.state(), DictationState::Idle);
    assert_eq!(s.leftovers(), 0);
}

#[test]
fn missing_model_or_file_fails_at_start() {
    let s = Setup::new();
    let mut c = s.config(WhisperEngine::Cpp);
    c.model = WhisperModel::LargeV3;
    let mut d = WhisperDictation::new(c);
    let audio = s.audio("a.wav", "x\n");
    let err = d.start(DictationInput::File(audio)).unwrap_err();
    assert!(matches!(err, DictationError::ModelNotFound { .. }));
    assert!(err.to_string().contains("ggml-large-v3.bin"), "{err}");
    // Before recording, so the student does not talk into the void.
    let err = d
        .start(DictationInput::Capture(Box::new(BufferCapture::new(
            Pcm::default(),
        ))))
        .unwrap_err();
    assert!(matches!(err, DictationError::ModelNotFound { .. }));
    assert_eq!(d.state(), DictationState::Idle);

    let mut d = WhisperDictation::new(s.config(WhisperEngine::OpenAi));
    let err = d
        .start(DictationInput::File(s.path().join("missing.wav")))
        .unwrap_err();
    assert!(matches!(err, DictationError::Io { .. }));
}

#[test]
fn whisper_cpp_needs_ffmpeg_for_other_formats() {
    let s = Setup::new();
    let mut c = s.config(WhisperEngine::Cpp);
    // An ffmpeg that does not exist.
    c.ffmpeg = Some(s.path().join("no-ffmpeg.exe"));
    let audio = s.audio("talk.ogg", "x\n");
    let mut d = WhisperDictation::new(c);
    d.start(DictationInput::File(audio)).unwrap();
    let events = run_to_end(&mut d);
    assert!(
        matches!(events.last(), Some(DictationEvent::Failed { message }) if message.starts_with("Could not start")),
        "{events:?}"
    );
}

#[test]
fn whisper_cpp_converts_other_formats_with_ffmpeg_when_present() {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let Some(ffmpeg) = textweaver_dictation::engine::find_on_path("ffmpeg", &path) else {
        eprintln!("ffmpeg not on PATH; skipping the conversion test");
        return;
    };
    let s = Setup::new();
    // A real one-second WAV, renamed so it must be converted: ffmpeg
    // resamples it to 16 kHz, and the fake reports the sample count.
    let wav = Pcm::new(8000, vec![0; 8000]).to_wav();
    let audio = s.path().join("clip.dat");
    std::fs::write(&audio, wav).unwrap();
    let mut c = s.config(WhisperEngine::Cpp);
    c.ffmpeg = Some(ffmpeg);
    let mut d = WhisperDictation::new(c);
    d.start(DictationInput::File(audio)).unwrap();
    let t = final_transcript(&run_to_end(&mut d));
    assert_eq!(t.text(), "Captured 16000 samples at 16000 hertz.");
}
