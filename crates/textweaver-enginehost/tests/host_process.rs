//! [`HostProcess`] against real child processes: this test binary runs
//! itself as a small host (`--as-host <mode>`), so spawning, replies,
//! crashes, hangs, bad frames, shutdown, and hosts outliving their parent
//! are exercised without any speech engine. `harness = false`: the host
//! modes need a `main` that writes nothing but frames to stdout.
//!
//! No test depends on how fast the machine is: each waits for a signal
//! (a reply, the host's output closing, a process disappearing) with a
//! generous deadline.

use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use textweaver_enginehost::protocol::{
    self, EndStatus, FrameReader, FrameWriter, MAX_FRAME, ProtocolError, ReadyHeader, encode_audio,
    encode_end, tag,
};
use textweaver_enginehost::serve::{AtEnd, Incoming, RequestReader, log_line};
use textweaver_enginehost::{Ended, HostMsg, HostProcess, Message};

/// How long any test waits for something that should happen at once.
const DEADLINE: Duration = Duration::from_secs(10);

/// The toy `Pid` reply: a host started a host of its own.
const PID: u8 = 0x90;

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
    Pid(u32),
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
            Reply::Pid(pid) => {
                let mut e = FrameWriter::new(PID);
                e.u32(*pid);
                e.finish()
            }
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
            PID => Reply::Pid(r.u32()?),
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
    match mode {
        // A line that is not UTF-8, then more than any pipe buffer holds,
        // written as the hosts used to (`eprintln!` panics once nobody
        // reads the pipe).
        "stderr" => {
            let _ = std::io::stderr().write_all(b"bad \xff\xfe line\n");
            for i in 0..4000 {
                eprintln!("toy host: line {i} of a long start-up log");
            }
        }
        // Logs with `log_line`, which never fails, whatever stderr is.
        "chatty" => {
            for i in 0..4000 {
                log_line(&format!("toy host: line {i} of a long start-up log"));
            }
        }
        _ => {}
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
        "parent" => {
            // Starts a host of its own, says which, then waits to be killed.
            let Ok(child) = HostProcess::<Reply>::spawn(&me(), ["--as-host", "hang"], "grandchild")
            else {
                return ExitCode::FAILURE;
            };
            let _ = protocol::write_frame(&mut out, &Reply::Pid(child.id()).encode());
            std::thread::sleep(Duration::from_secs(60));
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let Ok(reader) =
        RequestReader::<Speak>::spawn_with(std::io::stdin(), "toy-host-reader", AtEnd::host())
    else {
        return ExitCode::FAILURE;
    };
    while let Some(item) = reader.next() {
        let Incoming::Request(s, _) = item else {
            continue;
        };
        if mode == "stuck" {
            // An engine stuck in synthesis that never checks the stop
            // epoch: it says it started, then never returns.
            let _ = protocol::write_frame(&mut out, &Reply::Audio(s.token, vec![1]).encode());
            std::thread::sleep(Duration::from_secs(60));
            return ExitCode::SUCCESS;
        }
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
    h.recv_timeout(DEADLINE).expect("the host answers in time")
}

fn closed(h: &mut HostProcess<Reply>) -> String {
    match next(h) {
        HostMsg::Closed(why) => why,
        other => panic!("expected Closed, got {other:?}"),
    }
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
    // Quit ends it: its output closes. (Before, this measured that
    // shutdown took under 450 ms, which failed on a busy machine.)
    h.send_frame(&protocol::encode_quit()).unwrap();
    assert_eq!(closed(&mut h), "host exited");
    assert_ne!(h.shutdown(), Ended::KilledAfterGrace);
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
    let why = closed(&mut h);
    assert!(why.starts_with("bad frame from host"), "{why}");
    // Before: shutdown sent Quit to the broken host and waited 500 ms
    // before killing it. Now a host that already failed is killed at once.
    assert_eq!(h.shutdown(), Ended::Killed);
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
    assert_eq!(h.shutdown(), Ended::KilledAfterGrace);
}

fn end_of_input_ends_a_stuck_host() {
    // Before: a host whose engine was stuck in synthesis lived on after
    // textweaver closed its input (textweaver exited or crashed).
    let mut h = host("stuck");
    ready(&mut h);
    h.send(&Speak {
        token: 1,
        samples: 1,
    })
    .unwrap();
    match next(&mut h) {
        HostMsg::Reply(Reply::Audio(1, _)) => {}
        other => panic!("expected the engine to start, got {other:?}"),
    }
    h.close_input();
    // It exits after its grace period (1.5 s), well within the deadline.
    assert_eq!(closed(&mut h), "host exited");
}

fn hosts_die_with_their_parent() {
    // Before: killing textweaver left its hosts running.
    let mut parent = host("parent");
    ready(&mut parent);
    let pid = match next(&mut parent) {
        HostMsg::Reply(Reply::Pid(pid)) => pid,
        other => panic!("expected the grandchild's pid, got {other:?}"),
    };
    assert!(process_alive(pid), "the grandchild runs");
    parent.kill();
    let deadline = Instant::now() + DEADLINE;
    while process_alive(pid) {
        assert!(Instant::now() < deadline, "host {pid} outlived its parent");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Whether process `pid` is running (a zombie is not).
fn process_alive(pid: u32) -> bool {
    if cfg!(windows) {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .expect("tasklist runs");
        String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\""))
    } else {
        match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            // The state follows the command name in parentheses.
            Ok(stat) => stat
                .rsplit_once(") ")
                .is_some_and(|(_, rest)| !rest.starts_with('Z') && !rest.starts_with('X')),
            Err(_) => false,
        }
    }
}

fn stderr_that_is_not_utf8_is_still_drained() {
    // Before: the stderr reader stopped at the first line that was not
    // UTF-8, so the host's next write to stderr failed and `eprintln!`
    // killed it (or the full pipe blocked it).
    let mut h = host("stderr");
    ready(&mut h);
    h.send(&Speak {
        token: 4,
        samples: 2,
    })
    .unwrap();
    match next(&mut h) {
        HostMsg::Reply(Reply::Audio(4, s)) => assert_eq!(s, [7, 7]),
        other => panic!("expected Audio, got {other:?}"),
    }
}

fn a_host_logs_safely_when_stderr_is_closed() {
    // `log_line` ignores a closed stderr where `eprintln!` would panic.
    let mut child = Command::new(me())
        .args(["--as-host", "chatty"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stderr.take());
    let mut stdout = child.stdout.take().unwrap();
    let body = protocol::read_body(&mut stdout)
        .unwrap()
        .expect("Ready, not a dead host");
    assert!(matches!(Reply::decode(&body), Ok(Reply::Ready(_))));
    drop(child.stdin.take());
    let _ = child.kill();
    let _ = child.wait();
}

fn an_utterance_over_the_frame_limit_is_refused_and_the_host_lives() {
    // Before: the frame was sent, the host's reader failed on its length
    // and the host exited.
    let mut h = host("echo");
    ready(&mut h);
    let mut big = FrameWriter::new(tag::SPEAK);
    big.u64(1);
    big.reserve(MAX_FRAME);
    for _ in 0..MAX_FRAME / 4 {
        big.u32(0);
    }
    let e = h.send_frame(&big.finish()).unwrap_err();
    assert_eq!(
        e,
        "this text is too long to speak in one piece (17 MB; the limit is 16 MB)"
    );
    h.send(&Speak {
        token: 2,
        samples: 3,
    })
    .unwrap();
    match next(&mut h) {
        HostMsg::Reply(Reply::Audio(2, s)) => assert_eq!(s, [7, 7, 7]),
        other => panic!("expected Audio, got {other:?}"),
    }
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
    let tests: [(&str, fn()); 11] = [
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
        (
            "end_of_input_ends_a_stuck_host",
            end_of_input_ends_a_stuck_host,
        ),
        ("hosts_die_with_their_parent", hosts_die_with_their_parent),
        (
            "stderr_that_is_not_utf8_is_still_drained",
            stderr_that_is_not_utf8_is_still_drained,
        ),
        (
            "a_host_logs_safely_when_stderr_is_closed",
            a_host_logs_safely_when_stderr_is_closed,
        ),
        (
            "an_utterance_over_the_frame_limit_is_refused_and_the_host_lives",
            an_utterance_over_the_frame_limit_is_refused_and_the_host_lives,
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
