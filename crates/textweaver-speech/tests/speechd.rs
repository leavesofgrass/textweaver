//! The speech-dispatcher backend against a fake SSIP server (every OS, over
//! TCP) and, when `TEXTWEAVER_SPEECHD=1`, against a real speech-dispatcher
//! with its espeak-ng module started by the test on a private socket with
//! ALSA's null device (the dev container; nothing is played aloud).
#![cfg(feature = "speechd")]

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use textweaver_speech::backends::speechd::{SpeechdAddress, SpeechdBackend};
use textweaver_speech::core::{CharPos, CharRange, Rate, Utterance, UtteranceId};
use textweaver_speech::{
    EventSink, NormalizeConfig, RawEvent, ServiceConfig, SpeechBackend, SpeechService,
    SpeechStatus, VoiceParams,
};

/// A fake speech-dispatcher: answers every command, and for each message
/// either plays it at once (BEGIN, every mark, END) or holds it until
/// CANCEL (then CANCELED, followed by a late END that must be ignored).
struct FakeServer {
    port: u16,
    lines: Arc<Mutex<Vec<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl FakeServer {
    fn start(hold: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&lines);
        let thread = std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            serve(stream, &log, hold);
        });
        FakeServer {
            port,
            lines,
            thread: Some(thread),
        }
    }

    fn address(&self) -> SpeechdAddress {
        SpeechdAddress::Inet {
            host: "127.0.0.1".into(),
            port: self.port,
        }
    }

    fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn serve(stream: TcpStream, log: &Mutex<Vec<String>>, hold: bool) {
    let mut out = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);
    let mut next_msg = 1u64;
    let mut held: Vec<u64> = Vec::new();
    let mut send = |s: &str| {
        let _ = out.write_all(s.replace('\n', "\r\n").as_bytes());
    };
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let cmd = line.trim_end().to_owned();
        log.lock().unwrap().push(cmd.clone());
        match cmd.as_str() {
            "LIST SYNTHESIS_VOICES" => send(
                "249-English (America)\ten-US\tnone\n249-German+Adam\tde\tAdam\n249 OK VOICE LIST SENT\n",
            ),
            "SPEAK" => {
                send("230 OK RECEIVING DATA\n");
                let mut data = String::new();
                loop {
                    line.clear();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let l = line.trim_end().to_owned();
                    if l == "." {
                        break;
                    }
                    log.lock().unwrap().push(l.clone());
                    data.push_str(&l);
                }
                let id = next_msg;
                next_msg += 1;
                send(&format!("225-{id}\n225 OK MESSAGE QUEUED\n"));
                let marks = data.matches("<mark name=\"").count();
                if hold {
                    send(&format!("701-{id}\n701-1\n701 BEGIN\n"));
                    held.push(id);
                } else {
                    send(&format!("701-{id}\n701-1\n701 BEGIN\n"));
                    for m in 0..marks {
                        send(&format!("700-{id}\n700-1\n700-{m}\n700 INDEX MARK\n"));
                    }
                    send(&format!("702-{id}\n702-1\n702 END\n"));
                }
            }
            "CANCEL SELF" => {
                send("213 OK CANCELED\n");
                for id in held.drain(..) {
                    send(&format!("703-{id}\n703-1\n703 CANCELED\n"));
                    // A late notification for a cancelled message.
                    send(&format!("702-{id}\n702-1\n702 END\n"));
                }
            }
            "PAUSE SELF" => send("211 OK PAUSED\n"),
            "RESUME SELF" => send("212 OK RESUMED\n"),
            "QUIT" => {
                send("231 HAPPY HACKING\n");
                return;
            }
            c if c.starts_with("SET SELF SYNTHESIS_VOICE Refused") => {
                send("409 ERR NO SUCH VOICE\n");
            }
            c if c.starts_with("SET ") => send("200 OK SET\n"),
            _ => send("300 ERR UNKNOWN COMMAND\n"),
        }
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

fn poll_until(b: &mut SpeechdBackend, sink: &mut Collect, done: impl Fn(&Collect) -> bool) {
    for _ in 0..500 {
        b.poll(sink);
        if done(sink) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out; events so far: {:?}", sink.0);
}

#[test]
fn backend_speaks_with_a_mark_before_every_word() {
    let server = FakeServer::start(false);
    let mut b = SpeechdBackend::connect(&server.address()).unwrap();
    let voices = b.voices().unwrap();
    assert_eq!(voices.len(), 2);
    assert_eq!(voices[1].id, "German+Adam");
    assert_eq!(voices[1].languages, ["de"]);
    assert_eq!(voices[1].tags, ["Adam"]);
    b.set_params(&VoiceParams {
        voice: Some("German+Adam".into()),
        rate: Rate::Wpm(265),
        ..VoiceParams::default()
    })
    .unwrap();
    assert_eq!(b.effective_wpm(), 264);
    let mut u = Utterance::literal("Hello, brave world.", CharPos(0));
    u.id = UtteranceId {
        generation: 1,
        chunk: 0,
    };
    let mut sink = Collect::default();
    b.speak(&u, &mut sink).unwrap();
    poll_until(&mut b, &mut sink, |s| {
        s.0.last().is_some_and(|(_, e)| *e == RawEvent::Finished)
    });
    let events: Vec<RawEvent> = sink.0.iter().map(|(_, e)| e.clone()).collect();
    let word = |a, b| RawEvent::Word {
        byte_range: a..b,
        audio_ms: None,
    };
    assert_eq!(
        events,
        [
            RawEvent::Started,
            word(0, 5),
            word(7, 12),
            word(13, 18),
            RawEvent::Finished
        ]
    );
    drop(b);
    let lines = server.lines();
    let expect = [
        "SET SELF CLIENT_NAME user:textweaver:main",
        "SET SELF SSML_MODE on",
        "SET SELF NOTIFICATION ALL on",
        "LIST SYNTHESIS_VOICES",
        "SET SELF LANGUAGE de",
        "SET SELF SYNTHESIS_VOICE German+Adam",
        "SET SELF RATE 34",
        "SET SELF PITCH 0",
        "SET SELF VOLUME 100",
        "SPEAK",
        "<speak><mark name=\"0\"/>Hello, <mark name=\"1\"/>brave <mark name=\"2\"/>world.</speak>",
        "QUIT",
    ];
    assert_eq!(lines, expect);
}

#[test]
fn stop_cancels_and_late_events_are_ignored() {
    let server = FakeServer::start(true);
    let mut b = SpeechdBackend::connect(&server.address()).unwrap();
    let mut sink = Collect::default();
    let u = Utterance::literal("One two.", CharPos(0));
    b.speak(&u, &mut sink).unwrap();
    poll_until(&mut b, &mut sink, |s| !s.0.is_empty());
    b.pause().unwrap();
    b.resume().unwrap();
    b.stop();
    std::thread::sleep(Duration::from_millis(100));
    b.poll(&mut sink);
    assert_eq!(sink.0.len(), 1, "only Started: {:?}", sink.0);
    drop(b);
    let lines = server.lines();
    assert!(lines.ends_with(&[
        "PAUSE SELF".to_owned(),
        "RESUME SELF".to_owned(),
        "CANCEL SELF".to_owned(),
        "QUIT".to_owned()
    ]));
}

#[test]
fn a_refused_command_is_an_error() {
    let server = FakeServer::start(false);
    let mut b = SpeechdBackend::connect(&server.address()).unwrap();
    // Voices are passed through by name; the server decides.
    let err = b
        .set_params(&VoiceParams {
            voice: Some("Refused Voice".into()),
            ..VoiceParams::default()
        })
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("refused \"SET SELF SYNTHESIS_VOICE Refused Voice\": 409"),
        "{err}"
    );
    // The session goes on: the next command gets its own reply.
    b.pause().unwrap();
    drop(b);
    // Nothing listening: a clear error, not a hang.
    let closed = SpeechdAddress::Inet {
        host: "127.0.0.1".into(),
        port: server.port,
    };
    drop(server);
    let e = SpeechdBackend::connect(&closed).unwrap_err();
    assert!(e.to_string().contains("speech-dispatcher"), "{e}");
}

fn service(addr: SpeechdAddress) -> SpeechService {
    SpeechService::spawn(
        Box::new(move || {
            SpeechdBackend::connect(&addr).map(|b| Box::new(b) as Box<dyn SpeechBackend>)
        }),
        ServiceConfig {
            normalize: NormalizeConfig::none(),
            ..ServiceConfig::default()
        },
    )
    .unwrap()
}

fn read_positions(s: &SpeechService, utterances: Vec<Utterance>) -> Vec<CharRange> {
    let g = s.read(utterances);
    let mut words = Vec::new();
    loop {
        match s.statuses().recv_timeout(Duration::from_secs(20)) {
            Ok(SpeechStatus::Position {
                generation,
                source_range: Some(r),
                ..
            }) if generation == g => words.push(r),
            Ok(SpeechStatus::Finished { generation }) if generation == g => break,
            Ok(SpeechStatus::BackendError(e)) => panic!("{e}"),
            Ok(_) => {}
            Err(e) => panic!("no Finished: {e}; words {words:?}"),
        }
    }
    words
}

#[test]
fn the_service_reads_with_word_positions() {
    let server = FakeServer::start(false);
    let s = service(server.address());
    let words = read_positions(
        &s,
        vec![
            Utterance::literal("One two.", CharPos(0)),
            Utterance::literal("Three.", CharPos(9)),
        ],
    );
    assert_eq!(
        words,
        [
            CharRange::new(0, 3),
            CharRange::new(4, 7),
            CharRange::new(9, 14)
        ]
    );
    s.shutdown();
}

/// A real speech-dispatcher with its espeak-ng module, on a private socket
/// with ALSA's null device, so nothing is heard.
#[test]
#[ignore = "needs speech-dispatcher with sd_espeak-ng; run with TEXTWEAVER_SPEECHD=1"]
fn real_speech_dispatcher_reports_every_word() {
    if std::env::var("TEXTWEAVER_SPEECHD").as_deref() != Ok("1") {
        eprintln!("TEXTWEAVER_SPEECHD is not 1; skipping");
        return;
    }
    let Some(server) = RealServer::start() else {
        return;
    };
    let s = service(SpeechdAddress::Unix(server.socket.clone()));
    let words = read_positions(
        &s,
        vec![Utterance::literal("Hello brave new world.", CharPos(0))],
    );
    assert_eq!(
        words,
        [
            CharRange::new(0, 5),
            CharRange::new(6, 11),
            CharRange::new(12, 15),
            CharRange::new(16, 21)
        ]
    );
    // Stop right after starting a longer reading. ALSA's null device does
    // not pace audio, so the reading may already be over; either way,
    // nothing about it may follow `Stopped`.
    let g = s.read(vec![Utterance::literal(
        "one two three four five six seven eight nine ten eleven twelve.",
        CharPos(0),
    )]);
    s.stop();
    let mut after_stop = Vec::new();
    let mut stopped = false;
    while let Ok(st) = s.statuses().recv_timeout(Duration::from_secs(2)) {
        if st == (SpeechStatus::Stopped { generation: g }) {
            stopped = true;
        } else if stopped && st.generation() == Some(g) {
            after_stop.push(st);
        }
    }
    assert!(stopped);
    assert!(after_stop.is_empty(), "{after_stop:?}");
    s.shutdown();
    eprintln!(
        "real speech-dispatcher: {} words reported in order",
        words.len()
    );
}

struct RealServer {
    socket: std::path::PathBuf,
    child: std::process::Child,
    _dir: tempfile::TempDir,
}

impl RealServer {
    fn start() -> Option<Self> {
        let module = std::path::Path::new("/usr/lib/speech-dispatcher-modules/sd_espeak-ng");
        if !module.exists() {
            eprintln!(
                "speech-dispatcher's espeak-ng module is not installed \
                 (apt-get install speech-dispatcher-espeak-ng); skipping"
            );
            return None;
        }
        let dir = tempfile::tempdir().unwrap();
        let conf = dir.path().join("conf");
        let status = std::process::Command::new("cp")
            .arg("-r")
            .arg("/etc/speech-dispatcher")
            .arg(&conf)
            .status()
            .unwrap();
        assert!(status.success());
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(conf.join("speechd.conf"))
            .unwrap();
        writeln!(
            f,
            "AudioOutputMethod \"alsa\"\nAudioALSADevice \"null\"\n\
             AddModule \"espeak-ng\" \"sd_espeak-ng\" \"espeak-ng.conf\"\n\
             DefaultModule espeak-ng"
        )
        .unwrap();
        let socket = dir.path().join("speechd.sock");
        let child = std::process::Command::new("speech-dispatcher")
            .args(["-s", "-t", "30", "-c", "unix_socket", "-S"])
            .arg(&socket)
            .arg("-C")
            .arg(&conf)
            .arg("-L")
            .arg(dir.path())
            .spawn()
            .unwrap();
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(socket.exists(), "speech-dispatcher did not start");
        Some(RealServer {
            socket,
            child,
            _dir: dir,
        })
    }
}

impl Drop for RealServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
