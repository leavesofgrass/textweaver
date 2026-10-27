//! textweaver's broker with Mosquitto's own clients, `mosquitto_sub` and
//! `mosquitto_pub`, from Debian 13 in Docker. Ignored by default: run it
//! with `crates/textweaver-mqtt/docker/run-mosquitto-test.sh`, which builds
//! the image `textweaver-mosquitto-test` first, or with
//! `cargo test -p textweaver-broker --test mosquitto_clients -- --ignored`.
//!
//! The broker runs in this test program. On Windows and macOS it listens on
//! 127.0.0.1 and the clients reach it as `host.docker.internal`, through
//! Docker Desktop; on Linux the clients share the host's network. Each
//! client runs in its own container with `--rm`, so none is left. Test
//! passwords only; nothing is published outside this computer.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_broker::{Account, Accounts, Broker, BrokerConfig, BrokerEvent, Role};

const IMAGE: &str = "textweaver-mosquitto-test";

fn accounts() -> Accounts {
    let mut a = Accounts::default();
    for (name, role, publish, pw) in [
        (
            "netcontrol",
            Role::Logger,
            vec!["netlog/#".to_string()],
            "logger-test-pw",
        ),
        ("ada", Role::Reader, vec![], "reader-test-pw"),
    ] {
        let account =
            Account::with_iterations(name, role, vec!["netlog/#".into()], publish, pw, 1_000)
                .unwrap();
        a.add(account, false).unwrap();
    }
    a
}

type Events = Arc<Mutex<Vec<BrokerEvent>>>;

fn start(tls: bool) -> (Broker, Events, PathBuf) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("mosquitto-clients")
        .join(format!("run-{}-{nanos}", if tls { "tls" } else { "plain" }));
    std::fs::create_dir_all(&dir).unwrap();
    let mut cfg = BrokerConfig::new(&dir);
    cfg.tls = tls;
    cfg.listen = vec![SocketAddr::from(([127, 0, 0, 1], 0))];
    cfg.accounts = accounts();
    let events: Events = Arc::default();
    let seen = events.clone();
    let broker = Broker::start(cfg, move |e| {
        eprintln!("broker: {e}");
        seen.lock().unwrap().push(e);
    })
    .unwrap();
    (broker, events, dir)
}

/// The host name the clients use, and the network arguments for them.
fn network() -> (&'static str, Vec<&'static str>) {
    if cfg!(target_os = "linux") {
        ("127.0.0.1", vec!["--network", "host"])
    } else {
        ("host.docker.internal", vec![])
    }
}

/// A `docker run --rm` of the test image with `script` as a shell command,
/// with `CERT` set to `cert` (written to /tmp/ca.pem before the script).
fn client(script: &str, cert: &str) -> Command {
    let (_, net) = network();
    let mut cmd = Command::new("docker");
    cmd.env("MSYS_NO_PATHCONV", "1")
        .args(["run", "--rm", "-i"])
        .args(net)
        .args(["-e", &format!("CERT={cert}")])
        .args([IMAGE, "sh", "-c"])
        .arg(format!("printf '%s\\n' \"$CERT\" > /tmp/ca.pem; {script}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

fn run(script: &str, cert: &str) -> Output {
    client(script, cert).output().expect("run docker")
}

fn text(o: &Output) -> String {
    let mut t = String::from_utf8_lossy(&o.stdout).into_owned();
    t.push_str(&String::from_utf8_lossy(&o.stderr));
    t
}

fn wait_for(events: &Events, secs: u64, pred: impl Fn(&BrokerEvent) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if events.lock().unwrap().iter().any(&pred) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn finish(child: Child) -> String {
    let out = child.wait_with_output().expect("wait for the subscriber");
    text(&out)
}

fn exercise(tls: bool) {
    let (broker, events, dir) = start(tls);
    let (host, _) = network();
    let port = broker.local_addrs()[0].port();
    let cert = if tls {
        std::fs::read_to_string(dir.join(textweaver_broker::CERT_FILE)).unwrap()
    } else {
        String::new()
    };
    // --insecure skips only the host name check: the certificate names
    // localhost, and the clients reach the broker by another name. The
    // chain is still checked against the broker's own certificate.
    let tls_args = if tls {
        "--cafile /tmp/ca.pem --insecure"
    } else {
        ""
    };
    let common = format!("-h {host} -p {port} {tls_args}");

    // A reader subscribes for the next two messages.
    let sub = client(
        &format!("mosquitto_sub {common} -u ada -P reader-test-pw -t 'netlog/#' -v -C 2 -W 20"),
        &cert,
    )
    .spawn()
    .expect("start mosquitto_sub");
    assert!(
        wait_for(
            &events,
            20,
            |e| matches!(e, BrokerEvent::Connected { user, .. } if user == "ada")
        ),
        "mosquitto_sub did not connect"
    );

    // The reader may not publish: refused, and nobody receives it.
    let out = run(
        &format!(
            "mosquitto_pub {common} -u ada -P reader-test-pw -t netlog/checkins -q 1 -m fake-entry"
        ),
        &cert,
    );
    eprintln!("reader's mosquitto_pub: {}", text(&out));
    assert!(wait_for(&events, 10, |e| matches!(
        e,
        BrokerEvent::PublishRefused { user, .. } if user == "ada"
    )));

    // A wrong password is refused.
    let out = run(
        &format!("mosquitto_pub {common} -u netcontrol -P wrong -t netlog/checkins -q 1 -m x"),
        &cert,
    );
    assert!(!out.status.success(), "a wrong password was let in");

    // The logger publishes twice, the second retained.
    for (msg, retain) in [("{\"callsign\":\"N0CALL\"}", ""), ("retained-entry", "-r")] {
        let out = run(
            &format!(
                "mosquitto_pub {common} -u netcontrol -P logger-test-pw -t netlog/checkins -q 1 {retain} -m '{msg}'"
            ),
            &cert,
        );
        assert!(out.status.success(), "mosquitto_pub: {}", text(&out));
    }
    let got = finish(sub);
    assert!(
        got.contains("netlog/checkins {\"callsign\":\"N0CALL\"}"),
        "{got}"
    );
    assert!(got.contains("retained-entry"), "{got}");
    assert!(!got.contains("fake-entry"), "{got}");

    // A late subscriber gets the retained message at once.
    let out = run(
        &format!("mosquitto_sub {common} -u ada -P reader-test-pw -t 'netlog/#' -C 1 -W 10"),
        &cert,
    );
    assert_eq!(text(&out).trim(), "retained-entry");
    broker.stop();
}

#[test]
#[ignore = "needs Docker and the textweaver-mosquitto-test image; run crates/textweaver-mqtt/docker/run-mosquitto-test.sh"]
fn mosquitto_clients_plain() {
    exercise(false);
}

#[test]
#[ignore = "needs Docker and the textweaver-mosquitto-test image; run crates/textweaver-mqtt/docker/run-mosquitto-test.sh"]
fn mosquitto_clients_tls() {
    exercise(true);
}
