//! [`HostProcess`] against real child processes: this test binary runs
//! itself as a small host (`--as-host <mode>`), so spawning, replies,
//! crashes, hangs, bad frames, and shutdown are exercised without any
//! speech engine. `harness = false`: the host modes need a `main` that
//! writes nothing but frames to stdout.

use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use textweaver_enginehost::protocol::{
    self, EndStatus, FrameReader, FrameWriter, ProtocolError, ReadyHeader, encode_audio,
    encode_end, tag,
};
use textweaver_enginehost::serve::{Incoming, RequestReader};
use textweaver_enginehost::{HostMsg, HostProcess, Message};

/// The toy host's requests: Speak with a token and a sample count.
#[derive(Debug, PartialEq, Eq)]
struct Speak {
    token: u64,
    samples: u32,
}

impl Message for Speak {
    fn encode(&self) -> Vec<u8> {
        let mut e = FrameWriter::new(tag::SPEAK);
        e.u64(self.token);
        e.u32(self.samples);
        e.finish()
    }
    fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (t, mut r) = FrameReader::open(body)?;
        if t != tag::SPEAK {
            return Err(ProtocolError::BadTag(t));
        }
        let s = Speak {
            token: r.u64()?,
            samples: r.u32()?,
        };
        r.finish()?;
        Ok(s)
    }
}

/// The toy host's replies.
#[derive(Debug, PartialEq, Eq)]
enum Reply {
    Ready(ReadyHeader),
    Audio(u64, Vec<i16>),
    End(u64, EndStatus, u64),
}

impl Message for Reply {
    fn encode(&self) -> Vec<u8> {
        match self {
            Reply::Ready(h) => {
                let mut e = FrameWriter::new(tag::READY);
                h.write(&mut e);
                e.finish()
            }
            Reply::Audio(t, s) => encode_audio(*t, s),
            Reply::End(t, st, n) => encode_end(*t, *st, *n),
        }
    }
    fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
        let (t, mut r) = FrameReader::open(body)?;
        let m = match t {
            tag::READY => Reply::Ready(ReadyHeader {
                protocol: r.u16()?,
                sample_rate: r.u32()?,
            }),
            tag::AUDIO => Reply::Audio(r.u64()?, r.samples()?),
            tag::END => Reply::End(r.u64()?, EndStatus::from_byte(r.u8()?)?, r.u64()?),
            other => return Err(ProtocolError::BadTag(other)),
        };
        r.finish()?;
        Ok(m)
    }
}

// ---------------------------------------------------------------- host side

fn run_host(mode: &str) -> ExitCode {
    let mut out = std::io::stdout().lock();
    let ready = Reply::Ready(ReadyHeader {
        protocol: if mode == "old" {
            protocol::PROTOCOL_VERSION + 1
        } else {
            protocol::PROTOCOL_VERSION
        },
        sample_rate: 8000,
    });
    if mode == "garbage" {
        let _ = protocol::write_frame(&mut out, &[1, 0, 0, 0, 0x7f]);
        std::thread::sleep(Duration::from_secs(5));
        return ExitCode::SUCCESS;
    }
    if protocol::write_frame(&mut out, &ready.encode()).is_err() {
        return ExitCode::FAILURE;
    }
    match mode {
        "crash" => std::process::exit(3),
        "hang" => {
            // Never reads stdin, never answers, ignores Quit.
            std::thread::sleep(Duration::from_secs(60));
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let Ok(reader) = RequestReader::<Speak>::spawn(std::io::stdin(), "toy-host-reader") else {
        return ExitCode::FAILURE;
    };
    while let Some(item) = reader.next() {
        let Incoming::Request(s, _) = item else {
            continue;
        };
        let samples = vec![7i16; s.samples as usize];
        let frames = [
            Reply::Audio(s.token, samples).encode(),
            Reply::End(s.token, EndStatus::Done, u64::from(s.samples)).encode(),
        ];
        for f in frames {
            if protocol::write_frame(&mut out, &f).is_err() {
                return ExitCode::FAILURE;
            }
        }
    }
    let _ = out.flush();
    ExitCode::SUCCESS
}

// ------------------------------------------------------------- backend side

fn me() -> PathBuf {
    std::env::current_exe().expect("test binary path")
}

fn host(mode: &str) -> HostProcess<Reply> {
    HostProcess::spawn(&me(), ["--as-host", mode], "toy").expect("toy host starts")
}

fn next(h: &mut HostProcess<Reply>) -> HostMsg<Reply> {
    h.recv_timeout(Duration::from_secs(10))
        .expect("the host answers in time")
}

fn ready(h: &mut HostProcess<Reply>) -> ReadyHeader {
    match next(h) {
        HostMsg::Reply(Reply::Ready(r)) => r,
        other => panic!("expected Ready, got {other:?}"),
    }
}

fn speaks_and_shuts_down_cleanly() {
    let mut h = host("echo");
    let r = ready(&mut h);
    protocol::check_version(r.protocol).unwrap();
    assert_eq!(r.sample_rate, 8000);
    for token in 1..=3u64 {
        h.send(&Speak { token, samples: 5 }).unwrap();
        h.touch();
        match next(&mut h) {
            HostMsg::Reply(Reply::Audio(t, s)) => assert_eq!((t, s), (token, vec![7; 5])),
            other => panic!("expected Audio, got {other:?}"),
        }
        match next(&mut h) {
            HostMsg::Reply(Reply::End(t, EndStatus::Done, 5)) => assert_eq!(t, token),
            other => panic!("expected End, got {other:?}"),
        }
    }
    assert!(h.try_recv().is_none());
    assert!(!h.stalled(false, Duration::ZERO));
    let t0 = Instant::now();
    h.shutdown();
    assert!(t0.elapsed() < Duration::from_millis(450), "Quit ends it");
    assert!(
        h.send(&Speak {
            token: 9,
            samples: 1
        })
        .is_err()
    );
}

fn a_crash_is_reported_as_closed() {
    let mut h = host("crash");
    ready(&mut h);
    match next(&mut h) {
        HostMsg::Closed(why) => assert_eq!(why, "host exited"),
        other => panic!("expected Closed, got {other:?}"),
    }
}

fn a_bad_frame_closes_the_host() {
    let mut h = host("garbage");
    match next(&mut h) {
        HostMsg::Closed(why) => assert!(why.starts_with("bad frame from host"), "{why}"),
        other => panic!("expected Closed, got {other:?}"),
    }
    let t0 = Instant::now();
    h.kill();
    assert!(t0.elapsed() < Duration::from_secs(2));
}

fn a_hung_host_is_noticed_and_killed() {
    let mut h = host("hang");
    ready(&mut h);
    let _ = h.send(&Speak {
        token: 1,
        samples: 1,
    });
    h.touch();
    assert!(h.recv_timeout(Duration::from_millis(300)).is_none());
    assert!(h.idle_for() >= Duration::from_millis(250));
    assert!(h.stalled(true, Duration::from_millis(200)));
    assert!(!h.stalled(false, Duration::from_millis(200)));
    assert!(!h.stalled(true, Duration::from_secs(60)));
    // Quit is ignored: shutdown waits its grace period, then kills.
    let t0 = Instant::now();
    h.shutdown();
    let took = t0.elapsed();
    assert!(
        took >= Duration::from_millis(400) && took < Duration::from_secs(5),
        "{took:?}"
    );
}

fn a_version_mismatch_is_refused() {
    let mut h = host("old");
    let r = ready(&mut h);
    let e = protocol::check_version(r.protocol).unwrap_err();
    assert!(e.contains("cargo xtask hosts"), "{e}");
}

fn a_missing_executable_is_an_error() {
    let e = HostProcess::<Reply>::spawn(&me().with_file_name("no-such-host.exe"), ["x"], "missing")
        .unwrap_err();
    assert!(e.starts_with("cannot start"), "{e}");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--as-host") {
        return run_host(args.get(i + 1).map_or("echo", String::as_str));
    }
    // `cargo test -- --list` and filters: run everything, name each test.
    if args.iter().any(|a| a == "--list") {
        return ExitCode::SUCCESS;
    }
    let tests: [(&str, fn()); 6] = [
        (
            "speaks_and_shuts_down_cleanly",
            speaks_and_shuts_down_cleanly,
        ),
        (
            "a_crash_is_reported_as_closed",
            a_crash_is_reported_as_closed,
        ),
        ("a_bad_frame_closes_the_host", a_bad_frame_closes_the_host),
        (
            "a_hung_host_is_noticed_and_killed",
            a_hung_host_is_noticed_and_killed,
        ),
        (
            "a_version_mismatch_is_refused",
            a_version_mismatch_is_refused,
        ),
        (
            "a_missing_executable_is_an_error",
            a_missing_executable_is_an_error,
        ),
    ];
    let filter: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();
    println!("\nrunning {} tests", tests.len());
    let mut failed = 0;
    let mut passed = 0;
    for (name, f) in tests {
        if !filter.is_empty() && !filter.iter().any(|p| name.contains(p.as_str())) {
            continue;
        }
        let ok = catch_unwind(AssertUnwindSafe(f)).is_ok();
        println!("test {name} ... {}", if ok { "ok" } else { "FAILED" });
        if ok {
            passed += 1;
        } else {
            failed += 1;
        }
    }
    println!(
        "\ntest result: {}. {passed} passed; {failed} failed\n",
        if failed == 0 { "ok" } else { "FAILED" }
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
