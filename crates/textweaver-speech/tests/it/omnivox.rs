//! The Omnivox backend against a fake server process that records the
//! protocol lines it receives.

#![cfg(feature = "omnivox")]

use std::path::PathBuf;
use std::time::Duration;

use textweaver_speech::backends::omnivox::{OmnivoxBackend, OmnivoxCommand};
use textweaver_speech::core::{CharPos, Rate, Utterance};
use textweaver_speech::{
    Earcon, SayMode, ServiceConfig, SpeechBackend, SpeechService, SpeechStatus, VoiceParams,
};

fn fake() -> (OmnivoxCommand, PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let log = dir.path().join("protocol.log");
    let cmd = OmnivoxCommand {
        program: PathBuf::from(env!("CARGO_BIN_EXE_tw-fake-omnivox")),
        args: vec![log.to_string_lossy().into_owned()],
    };
    (cmd, log, dir)
}

fn lines(log: &PathBuf) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_backend_drives_the_server_process() {
    let (cmd, log, _dir) = fake();
    let mut b = OmnivoxBackend::spawn(&cmd).expect("fake server starts");
    b.set_params(&VoiceParams {
        rate: Rate::Wpm(320),
        ..VoiceParams::default()
    })
    .unwrap();
    b.tone(880.0, 40);
    b.stop();
    // Dropping closes stdin; the fake flushes each line and exits.
    drop(b);
    assert_eq!(lines(&log), ["tts_set_speech_rate 320", "t 880 40", "s"]);
}

#[test]
fn the_service_reads_through_the_server_with_timer_pacing() {
    let (cmd, log, _dir) = fake();
    let config = ServiceConfig {
        params: VoiceParams {
            rate: Rate::Wpm(900), // about 67 ms per word
            ..VoiceParams::default()
        },
        ..ServiceConfig::default()
    };
    let factory = Box::new(move || {
        OmnivoxBackend::spawn(&cmd).map(|b| Box::new(b) as Box<dyn SpeechBackend>)
    });
    let s = SpeechService::spawn(factory, config).expect("service starts");
    assert_eq!(s.backend_id(), "omnivox");
    s.read(vec![
        Utterance::literal("Hello {brave} world.", CharPos(0)),
        Utterance::literal("It is 2024.", CharPos(21)),
    ]);
    let mut positions = Vec::new();
    loop {
        match s.statuses().recv_timeout(Duration::from_secs(10)) {
            Ok(SpeechStatus::Position {
                source_range: Some(r),
                ..
            }) => positions.push(r),
            Ok(SpeechStatus::Finished { .. }) => break,
            Ok(other) => panic!("unexpected status {other:?}"),
            Err(e) => panic!("no Finished: {e}; positions so far {positions:?}"),
        }
    }
    // The timer walked the words of both sentences in order.
    assert!(positions.len() >= 4, "{positions:?}");
    assert!(positions.windows(2).all(|w| w[0].start <= w[1].start));
    assert_eq!(positions[0].start, CharPos(0));
    s.earcon(Earcon::Boundary);
    s.speak_char('Z', None);
    s.say("Done", SayMode::Queue);
    s.shutdown();
    let got = lines(&log);
    assert_eq!(got[0], "tts_set_speech_rate 900");
    assert!(
        got.contains(&"q {Hello (brave) world.}".to_owned()),
        "{got:?}"
    );
    assert!(
        got.contains(&"q {It is twenty twenty-four.}".to_owned()),
        "{got:?}"
    );
    assert!(got.contains(&"t 330 90".to_owned()), "{got:?}");
    assert!(got.contains(&"l {Z}".to_owned()) || got.contains(&"q {cap Z}".to_owned()));
    assert!(got.contains(&"q {Done}".to_owned()), "{got:?}");
}
