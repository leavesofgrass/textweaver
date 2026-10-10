//! A DAISY talking book's recorded narration (`[reading] book_audio`),
//! B1-r3: the recording plays where the book has it and speech reads the
//! rest. Tiny WAV clips are generated; the clip player is the silent one,
//! so nothing is played.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use textweaver_app::keymap::ActionId;
use textweaver_app::store::{BookAudio, Settings};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::{App, AppConfig, Command, Playback};

/// A WAV file of `seconds` of quiet 8 kHz mono audio.
fn wav(path: &Path, seconds: u32) {
    let n = 8_000 * seconds;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + n * 2).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&16_000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(n * 2).to_le_bytes());
    b.resize(b.len() + n as usize * 2, 0);
    std::fs::write(path, b).unwrap();
}

const NCC: &str = r#"<html><head><title>Talking Test</title>
<meta name="dc:title" content="Talking Test"/></head><body>
<h1 id="h1"><a href="a.smil#p1">Chapter One</a></h1></body></html>"#;

const SMIL: &str = r#"<smil><body><seq>
<par id="p1"><text src="text.html#t1"/><audio src="one.wav" clip-begin="npt=0.000s" clip-end="npt=0.500s"/></par>
<par id="p2"><text src="text.html#t2"/><audio src="one.wav" clip-begin="npt=0.500s" clip-end="npt=1.250s"/></par>
<par id="p3"><text src="text.html#t3"/></par>
<par id="p4"><text src="text.html#t4"/><audio src="one.wav" clip-begin="npt=1.250s" clip-end="npt=2.000s"/></par>
</seq></body></smil>"#;

const TEXT: &str = r#"<html><body><h1 id="t1">Chapter One</h1>
<p id="t2">The first words.</p><p id="t3">No audio here.</p><p id="t4">Last words.</p></body></html>"#;

/// A talking book opened in a self-voicing app with `settings`.
fn open_book(settings: Settings) -> (App, SpeechLog, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in [("ncc.html", NCC), ("a.smil", SMIL), ("text.html", TEXT)] {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    wav(&dir.path().join("one.wav"), 2);
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        settings,
        ..AppConfig::for_tests()
    });
    app.set_clip_player(textweaver_app::engines::silent_recorded_player(Arc::new(
        |p: &Path| std::fs::read(p),
    )));
    app.open(&dir.path().join("ncc.html")).unwrap();
    (app, log, dir)
}

fn wait_idle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while app.playback() != Playback::Idle && Instant::now() < deadline {
        app.poll_speech();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(app.playback(), Playback::Idle, "still reading");
}

#[test]
fn the_recording_plays_and_speech_reads_the_text_without_audio() {
    let (mut app, log, _dir) = open_book(Settings::default());
    log.clear();
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    wait_idle(&mut app);
    // Only the paragraph with no audio reached the speech engine.
    let texts = log.texts();
    assert!(
        texts.iter().any(|t| t.contains("No audio here.")),
        "{texts:?}"
    );
    for recorded in ["Chapter One", "The first words.", "Last words."] {
        assert!(
            !texts.iter().any(|t| t.contains(recorded)),
            "{recorded} was spoken: {texts:?}"
        );
    }
}

#[test]
fn speech_only_reads_every_word_and_the_toggle_switches_back() {
    let mut settings = Settings::default();
    settings.reading.book_audio = BookAudio::Speech;
    let (mut app, log, _dir) = open_book(settings);
    log.clear();
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    wait_idle(&mut app);
    assert!(log.texts().iter().any(|t| t.contains("Last words.")));
    app.dispatch(Command::Action(ActionId::ToggleBookAudio));
    assert_eq!(app.settings().reading.book_audio, BookAudio::Auto);
    assert!(
        app.status_text().starts_with("Book audio on."),
        "{}",
        app.status_text()
    );
}

#[test]
fn a_rate_change_during_the_recording_says_it_does_not_apply_once() {
    let (mut app, _log, _dir) = open_book(Settings::default());
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    app.dispatch(Command::Action(ActionId::RateUp));
    assert!(
        app.status_text()
            .contains("Rate changes do not apply to recorded audio."),
        "{}",
        app.status_text()
    );
    app.dispatch(Command::Action(ActionId::RateUp));
    assert!(!app.status_text().contains("recorded audio"));
    wait_idle(&mut app);
}
