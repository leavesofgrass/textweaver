//! The combined host runs every engine, chosen by its argument: each
//! engine's fake starts and reports `Ready` over that engine's protocol,
//! in both the bare form (`textweaver-host eci ...`) and the host-mode
//! form textweaver's programs take (`--engine-host eci ...`), which
//! [`HostProcess::spawn_engine`] puts first. No audio is played.

use std::path::PathBuf;
use std::time::Duration;

use textweaver_enginehost::protocol::Message;
use textweaver_enginehost::{HostMsg, HostProcess};

fn host() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_textweaver-host"))
}

/// The first reply from `process`, which must come within ten seconds.
fn first<R: Message + Send + std::fmt::Debug + 'static>(mut process: HostProcess<R>) -> R {
    match process.recv_timeout(Duration::from_secs(10)) {
        Some(HostMsg::Reply(r)) => r,
        other => panic!("no reply from the host: {other:?}"),
    }
}

#[test]
fn each_engine_starts_through_host_mode() {
    let fake = ["--engine", "fake"];
    let r = first(
        HostProcess::<textweaver_eci::protocol::Reply>::spawn_engine(&host(), "eci", fake).unwrap(),
    );
    assert!(
        matches!(r, textweaver_eci::protocol::Reply::Ready { .. }),
        "{r:?}"
    );
    let r = first(
        HostProcess::<textweaver_espeak::protocol::Reply>::spawn_engine(&host(), "espeak", fake)
            .unwrap(),
    );
    assert!(
        matches!(r, textweaver_espeak::protocol::Reply::Ready { .. }),
        "{r:?}"
    );
    let r = first(
        HostProcess::<textweaver_dectalk::protocol::Reply>::spawn_engine(&host(), "dectalk", fake)
            .unwrap(),
    );
    assert!(
        matches!(r, textweaver_dectalk::protocol::Reply::Ready { .. }),
        "{r:?}"
    );
    let r = first(
        HostProcess::<textweaver_sapi::protocol::Reply>::spawn_engine(&host(), "sapi", fake)
            .unwrap(),
    );
    assert!(
        matches!(r, textweaver_sapi::protocol::Reply::Ready { .. }),
        "{r:?}"
    );
}

#[test]
fn the_bare_engine_argument_chooses_the_engine_too() {
    let r = first(
        HostProcess::<textweaver_espeak::protocol::Reply>::spawn(
            &host(),
            ["espeak", "--engine", "fake"],
            "espeak",
        )
        .unwrap(),
    );
    assert!(
        matches!(r, textweaver_espeak::protocol::Reply::Ready { .. }),
        "{r:?}"
    );
}

#[test]
fn an_unknown_engine_ends_the_host() {
    let r = HostProcess::<textweaver_eci::protocol::Reply>::spawn(&host(), ["votrax"], "eci")
        .unwrap()
        .recv_timeout(Duration::from_secs(10));
    assert!(matches!(r, Some(HostMsg::Closed(_))), "{r:?}");
}
