//! `tw --engine-host <engine>` is host mode: `tw` becomes a speech
//! engine's host, checked before any other argument, and speaks the
//! engine's protocol instead of starting the reader. No audio is played.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use textweaver_enginehost::{HostMsg, HostProcess};
use textweaver_espeak::protocol::Reply;

#[test]
fn tw_in_host_mode_runs_the_engine_host() {
    let tw = Path::new(env!("CARGO_BIN_EXE_tw"));
    let mut host = HostProcess::<Reply>::spawn_engine(tw, "espeak", ["--engine", "fake"])
        .expect("tw starts in host mode");
    match host.recv_timeout(Duration::from_secs(20)) {
        Some(HostMsg::Reply(Reply::Ready { .. })) => {}
        other => panic!("tw did not start as a host: {other:?}"),
    }
}

#[test]
fn host_mode_without_an_engine_fails_in_words() {
    let out = Command::new(env!("CARGO_BIN_EXE_tw"))
        .arg("--engine-host")
        .output()
        .expect("tw runs");
    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "host mode printed to stdout");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("needs an engine name"), "{err}");
}
