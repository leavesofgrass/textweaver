//! A real Whisper program, when one is installed. Ignored unless
//! `TEXTWEAVER_WHISPER_REAL=1`; run with
//! `TEXTWEAVER_WHISPER_REAL=1 cargo test -p textweaver-dictation --test real_whisper -- --ignored --nocapture`.
//!
//! The speech comes from `espeak-ng -w` (written to a file, never played).
//! The model is `tiny` unless `TEXTWEAVER_WHISPER_MODEL` says otherwise;
//! whisper.cpp also needs its `ggml-<model>.bin` (see
//! `TEXTWEAVER_WHISPER_MODELS`).

use std::process::Command;
use std::time::Instant;

use textweaver_dictation::{
    Dictation, DictationEvent, DictationInput, WhisperConfig, WhisperDictation, WhisperModel,
    apply_spoken_commands, detect,
};

#[test]
#[ignore = "needs a Whisper program; set TEXTWEAVER_WHISPER_REAL=1"]
fn transcribes_synthesized_speech() {
    if std::env::var("TEXTWEAVER_WHISPER_REAL").as_deref() != Ok("1") {
        eprintln!("TEXTWEAVER_WHISPER_REAL is not 1; skipping");
        return;
    }
    let engines = detect();
    eprintln!("Whisper programs found: {engines:?}");
    let Some(found) = engines.into_iter().next() else {
        panic!("TEXTWEAVER_WHISPER_REAL=1 but no Whisper program was found");
    };
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("speech.wav");
    let status = Command::new("espeak-ng")
        .arg("-w")
        .arg(&wav)
        .arg("The quick brown fox jumps over the lazy dog. New line. Reading is fun period")
        .status()
        .expect("espeak-ng runs");
    assert!(status.success());

    let model = std::env::var("TEXTWEAVER_WHISPER_MODEL")
        .ok()
        .and_then(|m| WhisperModel::parse(&m))
        .unwrap_or(WhisperModel::Tiny);
    let mut config = WhisperConfig::new(found.engine, found.program.clone(), model);
    config.language = Some("en".into());
    let mut d = WhisperDictation::new(config);
    let started = Instant::now();
    d.start(DictationInput::File(wav)).unwrap();
    let events = d.wait();
    eprintln!(
        "{} ({}) with {model} took {:.1} s",
        d.name(),
        found.program.display(),
        started.elapsed().as_secs_f64()
    );
    for e in &events {
        eprintln!("  {e:?}");
    }
    let Some(DictationEvent::Final(t)) = events.last() else {
        panic!("no transcript: {events:?}");
    };
    let text = t.text().to_lowercase();
    eprintln!("transcript: {}", t.text());
    eprintln!(
        "with spoken commands: {:?}",
        apply_spoken_commands(&t.text())
    );
    for word in ["quick", "brown", "fox", "lazy", "dog"] {
        assert!(text.contains(word), "{word} missing from {text:?}");
    }
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DictationEvent::Partial(_)))
    );
}
