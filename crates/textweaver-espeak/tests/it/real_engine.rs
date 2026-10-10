//! The helper against the installed libespeak-ng, when there is one
//! (skipped otherwise), with the virtual output: nothing is played.
//!
//! The helper and the in-process backend drive the same library in the
//! same retrieval mode, so their words must agree exactly: the same byte
//! ranges at the same milliseconds. That is what keeps the highlight exact
//! whichever path speaks.

use std::path::PathBuf;

use textweaver_espeak::{AudioOutput, EspeakHostBackend, EspeakHostConfig, discovery};
use textweaver_speech::backends::espeak::{EspeakBackend, EspeakOutput};
use textweaver_speech::{RawEvent, SpeechBackend};

use crate::fake_host::{Rec, pump, utt};

/// The helper with the real engine, or `None` (and a note) when no
/// libespeak-ng this test can run is installed.
fn helper() -> Option<EspeakHostBackend> {
    let lib = discovery::choose_library(&discovery::library_candidates())?;
    if !discovery::host_name(lib.arch, discovery::Arch::current()).eq(discovery::HOST_NAME) {
        eprintln!("skipped: {} needs another host", lib.path.display());
        return None;
    }
    let config = EspeakHostConfig {
        host: Some(PathBuf::from(env!("CARGO_BIN_EXE_textweaver-espeak-host"))),
        output: AudioOutput::Null { speed: 4.0 },
        ..EspeakHostConfig::default()
    };
    match EspeakHostBackend::new(config) {
        Ok(b) => Some(b),
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

const TEXT: &str = "The quick brown fox, caf\u{e9} na\u{ef}ve, jumps over 12 lazy dogs.";

#[test]
fn the_helper_speaks_with_words_in_order() {
    let Some(mut b) = helper() else { return };
    assert_eq!(b.engine(), Some("espeak-ng"));
    assert!(!b.voices().unwrap().is_empty());
    let mut rec = Rec::default();
    let u = utt(TEXT, 1);
    b.speak(&u, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, u.id));
    assert_eq!(rec.of(u.id).first(), Some(&RawEvent::Started));
    assert_eq!(rec.of(u.id).last(), Some(&RawEvent::Finished));
    let words = rec.words(u.id);
    assert!(words.len() >= 10, "{words:?}");
    assert!(
        words
            .windows(2)
            .all(|w| w[0].1 <= w[1].1 && w[0].0.end <= w[1].0.start)
    );
    assert_eq!(
        &TEXT[words[5].0.start as usize..words[5].0.end as usize],
        "na\u{ef}ve"
    );
}

#[test]
fn helper_and_in_process_words_agree_exactly() {
    let Some(mut helper) = helper() else { return };
    let Ok(mut local) = EspeakBackend::new(EspeakOutput::Virtual) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let u = utt(TEXT, 1);
    let a = helper
        .synthesize_utterance(&u, &dir.path().join("helper.wav"))
        .unwrap();
    let b = local
        .synthesize_utterance(&u, &dir.path().join("local.wav"))
        .unwrap();
    assert_eq!(a.words, b.words);
    let size = |n: &str| std::fs::metadata(dir.path().join(n)).unwrap().len();
    assert_eq!(size("helper.wav"), size("local.wav"));
}
