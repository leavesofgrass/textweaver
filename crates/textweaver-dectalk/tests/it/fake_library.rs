//! The host's real FFI code against a stand-in DECtalk library
//! (`examples/fake_dectalk.rs`, built as a shared library by `cargo test`):
//! loading, start-up, speech-to-memory buffers and the callback, index
//! marks and their sample numbers, and the text DECtalk receives. No
//! DECtalk is involved.
//!
//! `TEXTWEAVER_DECTALK_TEST_LIBRARY` and `TEXTWEAVER_DECTALK_TEST_HOST`
//! run the same tests against other builds, for example the 32-bit host
//! with a 32-bit fake built with `--features fake-stdcall`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use textweaver_core::{CharPos, Rate, Utterance, UtteranceId};
use textweaver_dectalk::protocol::{self, EndStatus, Piece, Reply, Request};
use textweaver_dectalk::{AudioOutput, DectalkBackend, DectalkConfig};
use textweaver_speech::{EventSink, RawEvent, SpeechBackend, VoiceParams};

fn var(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn host() -> PathBuf {
    var("TEXTWEAVER_DECTALK_TEST_HOST")
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_textweaver-dectalk-host")))
}

/// The fake library cargo built next to this test (`target/<profile>/examples`).
fn library() -> Option<PathBuf> {
    if let Some(p) = var("TEXTWEAVER_DECTALK_TEST_LIBRARY") {
        return Some(p);
    }
    let name = format!(
        "{}fake_dectalk{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let exe = std::env::current_exe().ok()?;
    let path = exe.parent()?.parent()?.join("examples").join(name);
    if path.is_file() {
        Some(path)
    } else {
        eprintln!(
            "skipped: {} is not built (run `cargo test -p textweaver-dectalk`, which builds it)",
            path.display()
        );
        None
    }
}

/// Runs the host on `lib` with `env`, sends `requests` then `Quit`, and
/// returns every reply.
fn converse(lib: &Path, env: &[(&str, &str)], requests: &[Request]) -> Vec<Reply> {
    let mut cmd = Command::new(host());
    cmd.arg("--library")
        .arg(lib)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for k in [
        "FAKE_DECTALK_MARKS",
        "FAKE_DECTALK_MAX_BYTES",
        "FAKE_DECTALK_NO_EX",
        "FAKE_DECTALK_FAIL",
        "FAKE_DECTALK_LOG",
    ] {
        cmd.env_remove(k);
    }
    cmd.envs(env.iter().copied());
    let mut child = cmd.spawn().expect("host starts");
    let mut stdin = child.stdin.take().unwrap();
    // A host that cannot start exits at once, so writes may find the pipe
    // closed; its replies are still read below.
    for r in requests.iter().chain([&Request::Quit]) {
        if stdin.write_all(&r.encode()).is_err() {
            break;
        }
    }
    let _ = stdin.flush();
    let mut stdout = child.stdout.take().unwrap();
    let mut replies = Vec::new();
    while let Ok(Some(body)) = protocol::read_body(&mut stdout) {
        replies.push(Reply::decode(&body).unwrap());
    }
    drop(stdin);
    let _ = child.wait();
    replies
}

fn speak(token: u64, text: &str) -> Request {
    let ws = textweaver_dectalk::words::words(text);
    Request::Speak {
        token,
        pieces: textweaver_dectalk::words::pieces(text, &ws),
    }
}

/// `(audio samples, marks)` of utterance `token`.
fn utterance(replies: &[Reply], token: u64) -> (usize, Vec<(u32, u64)>) {
    let mut n = 0;
    let mut marks = Vec::new();
    for r in replies {
        match r {
            Reply::Audio { token: t, samples } if *t == token => n += samples.len(),
            Reply::Mark {
                token: t,
                index,
                sample,
            } if *t == token => marks.push((*index, *sample)),
            Reply::End {
                token: t,
                status,
                samples,
            } if *t == token => {
                assert_eq!(*status, EndStatus::Done, "{replies:?}");
                assert_eq!(*samples as usize, n);
            }
            _ => {}
        }
    }
    (n, marks)
}

const TEXT: &str = "Hello brave new world";
/// 200 samples per byte at 180 wpm: "Hello " is 6 bytes, and so on.
const MARKS: [(u32, u64); 4] = [(0, 0), (1, 1200), (2, 2400), (3, 3200)];

#[test]
fn the_host_starts_the_library_and_marks_every_word() {
    let Some(lib) = library() else { return };
    let replies = converse(&lib, &[], &[speak(1, TEXT), speak(2, TEXT)]);
    assert!(
        matches!(&replies[0], Reply::Ready { engine, sample_rate: 11_025, .. } if engine == "dectalk"),
        "{:?}",
        replies.first()
    );
    let (n, marks) = utterance(&replies, 1);
    assert_eq!(n, TEXT.len() * 200);
    assert_eq!(marks, MARKS);
    // The second utterance counts from its own start although DECtalk's
    // sample numbers keep rising.
    assert_eq!(utterance(&replies, 2), (n, MARKS.to_vec()));
}

#[test]
fn per_buffer_sample_numbers_across_many_buffers_come_out_the_same() {
    let Some(lib) = library() else { return };
    // 1,000 bytes a buffer: DECtalk hands each full buffer to the
    // callback, which copies it and gives it back.
    for marks_env in ["buffer", "stream"] {
        let replies = converse(
            &lib,
            &[
                ("FAKE_DECTALK_MARKS", marks_env),
                ("FAKE_DECTALK_MAX_BYTES", "1000"),
            ],
            &[speak(1, TEXT), speak(2, TEXT)],
        );
        let (n, marks) = utterance(&replies, 1);
        assert_eq!(n, TEXT.len() * 200, "{marks_env}");
        assert_eq!(marks, MARKS, "{marks_env}");
        assert_eq!(utterance(&replies, 2), (n, MARKS.to_vec()), "{marks_env}");
    }
}

#[test]
fn without_startup_ex_one_buffer_holds_the_sentence() {
    let Some(lib) = library() else { return };
    let replies = converse(&lib, &[("FAKE_DECTALK_NO_EX", "1")], &[speak(1, TEXT)]);
    assert_eq!(utterance(&replies, 1), (TEXT.len() * 200, MARKS.to_vec()));
}

#[test]
fn a_dectalk_that_does_not_start_says_so() {
    let Some(lib) = library() else { return };
    let replies = converse(&lib, &[("FAKE_DECTALK_FAIL", "1")], &[]);
    assert!(
        matches!(&replies[..], [Reply::Error { token: 0, message }] if message.contains("did not start")),
        "{replies:?}"
    );
}

#[test]
fn dectalk_receives_latin1_text_with_the_voice_and_marks_inline() {
    let Some(lib) = library() else { return };
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("spoken.txt");
    let replies = converse(
        &lib,
        &[("FAKE_DECTALK_LOG", log.to_str().unwrap())],
        &[
            Request::SetVoice {
                speaker: 4,
                rate: 250,
                pitch: 233,
            },
            Request::Speak {
                token: 1,
                pieces: vec![
                    Piece::Index(0),
                    Piece::Text("Café [:np] ".into()),
                    Piece::Index(1),
                    Piece::Text("“ok”".into()),
                ],
            },
        ],
    );
    assert_eq!(utterance(&replies, 1).1.len(), 2);
    let spoken = std::fs::read(&log).unwrap();
    assert_eq!(
        spoken,
        b"[:nb][:rate 250][:dv ap 233][:index mark 1][:index mark 2]Caf\xe9 (:np) \
          [:index mark 3]\"ok\"\n"
    );
}

#[derive(Default)]
struct Rec(Vec<(UtteranceId, RawEvent)>);

impl EventSink for Rec {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        self.0.push((id, event));
    }
    fn is_current(&self, _id: UtteranceId) -> bool {
        true
    }
}

#[test]
fn the_backend_finds_the_library_and_speaks_through_it() {
    let Some(lib) = library() else { return };
    let mut b = DectalkBackend::new(DectalkConfig {
        library: Some(lib.clone()),
        host: Some(host()),
        output: AudioOutput::Null { speed: 8.0 },
        ..DectalkConfig::default()
    })
    .expect("the fake DECtalk starts");
    assert_eq!(b.engine(), Some("dectalk"));
    let choice = b.library().expect("a library was chosen");
    if std::env::var_os("TEXTWEAVER_DECTALK_LIBRARY").is_none() {
        assert_eq!(choice.candidate.path, lib);
        assert!(
            choice.reason.contains("named in the settings"),
            "{}",
            choice.reason
        );
    }

    b.set_params(&VoiceParams {
        rate: Rate::Wpm(180),
        ..VoiceParams::default()
    })
    .unwrap();
    let s = b.synthesize(TEXT).unwrap();
    let words: Vec<(&str, u64)> = s
        .words
        .iter()
        .map(|(r, at)| (&TEXT[r.start as usize..r.end as usize], *at))
        .collect();
    assert_eq!(
        words,
        [
            ("Hello", 0),
            ("brave", 1200),
            ("new", 2400),
            ("world", 3200)
        ]
    );

    b.set_params(&VoiceParams {
        voice: Some("dectalk:harry".into()),
        rate: Rate::Wpm(360),
        ..VoiceParams::default()
    })
    .unwrap();
    let mut rec = Rec::default();
    let mut u = Utterance::literal(TEXT, CharPos(0));
    u.id = UtteranceId {
        generation: 1,
        chunk: 0,
    };
    b.speak(&u, &mut rec).unwrap();
    let t0 = std::time::Instant::now();
    while !rec.0.iter().any(|(_, e)| *e == RawEvent::Finished) {
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(10),
            "{:?}",
            rec.0
        );
        b.poll(&mut rec);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let ms: Vec<u32> = rec
        .0
        .iter()
        .filter_map(|(_, e)| match e {
            RawEvent::Word { audio_ms, .. } => *audio_ms,
            _ => None,
        })
        .collect();
    // Twice the rate: "brave" at 600 samples, 54 ms.
    assert_eq!(ms, [0, 54, 108, 145]);
}
