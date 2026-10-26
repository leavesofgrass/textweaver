//! The client against Debian 13's Mosquitto in Docker. Ignored by default:
//! run it with `crates/textweaver-mqtt/docker/run-mosquitto-test.sh`,
//! which builds the image `textweaver-mosquitto-test` from the local
//! `debian:trixie` first.
//!
//! The test starts its own container, publishes its ports on 127.0.0.1
//! only, restarts it once, and removes it (and only it) at the end. The
//! test CA, broker certificate and key are made at run time in the test's
//! own folder under the target directory.

mod support;

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use support::Harness;
use textweaver_mqtt::{MqttConfig, MqttEvent, Payload, Secret, Tls};

const IMAGE: &str = "textweaver-mosquitto-test";
const NAME: &str = "textweaver-mosquitto-test";
const PLAIN: u16 = 18830;
const TLS: u16 = 18883;

fn docker(args: &[&str]) -> (bool, String) {
    let out = Command::new("docker")
        .args(args)
        .env("MSYS_NO_PATHCONV", "1")
        .output()
        .expect("run docker");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

/// Removes this test's own container when the test ends, pass or fail,
/// after saving its log.
struct Container;

impl Drop for Container {
    fn drop(&mut self) {
        let (_, logs) = docker(&["logs", NAME]);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("mosquitto-log.txt");
        let _ = std::fs::write(&path, &logs);
        eprintln!("broker log saved to {}", path.display());
        let (ok, text) = docker(&["rm", "-f", NAME]);
        eprintln!(
            "docker rm -f {NAME}: {}",
            if ok { "done" } else { text.trim() }
        );
    }
}

fn wait_ready(since_starts: usize) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        let (_, logs) = docker(&["logs", NAME]);
        if logs.matches("running").count() > since_starts {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("Mosquitto did not start: {}", docker(&["logs", NAME]).1);
}

fn publish(topic: &str, payload: &str, retain: bool) {
    let mut args = vec![
        "exec",
        NAME,
        "mosquitto_pub",
        "-h",
        "127.0.0.1",
        "-p",
        "1883",
        "-u",
        "logger",
        "-P",
        "logger-test-pw",
        "-q",
        "1",
        "-t",
        topic,
        "-m",
        payload,
    ];
    if retain {
        args.push("-r");
    }
    let (ok, text) = docker(&args);
    assert!(ok, "mosquitto_pub failed: {text}");
}

fn reader(port: u16, tls: Tls, password: &str, topics: &[&str]) -> MqttConfig {
    let mut c = MqttConfig::new("127.0.0.1", port, tls);
    c.username = Some("reader".into());
    c.password = Some(Secret::new(password));
    c.min_backoff = Duration::from_millis(250);
    c.max_backoff = Duration::from_secs(2);
    c.connect_timeout = Duration::from_secs(5);
    c.keep_alive = Duration::from_secs(5);
    c.topics = topics.iter().map(|t| t.to_string()).collect();
    c
}

fn reason(seen: &[MqttEvent]) -> (String, bool) {
    seen.iter()
        .rev()
        .find_map(|e| match e {
            MqttEvent::Disconnected { reason, retrying } => Some((reason.clone(), *retrying)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no Disconnected event in {seen:#?}"))
}

fn retained(e: &MqttEvent) -> bool {
    matches!(e, MqttEvent::Message { retained: true, .. })
}

#[test]
#[ignore = "needs Docker and the textweaver-mosquitto-test image; run crates/textweaver-mqtt/docker/run-mosquitto-test.sh"]
fn mosquitto_in_docker() {
    let (exists, _) = docker(&["image", "inspect", IMAGE]);
    assert!(
        exists,
        "build the image first: crates/textweaver-mqtt/docker/run-mosquitto-test.sh"
    );
    let (_, ps) = docker(&[
        "ps",
        "-a",
        "--filter",
        &format!("name=^{NAME}$"),
        "--format",
        "{{.Names}}",
    ]);
    assert!(
        ps.trim().is_empty(),
        "a container named {NAME} is left from an earlier run; remove it first"
    );

    // Test CA and broker certificate for 127.0.0.1 and localhost, made now.
    let pki = support::pki(&["localhost", "127.0.0.1"]);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("mosquitto-certs");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("ca.crt"), &pki.ca_pem).unwrap();
    std::fs::write(dir.join("server.crt"), &pki.server_cert_pem).unwrap();
    std::fs::write(dir.join("server.key"), &pki.server_key_pem).unwrap();
    let ca = dir.join("ca.crt");

    let mount = format!("{}:/mosquitto/certs:ro", dir.display());
    let p1 = format!("127.0.0.1:{PLAIN}:1883");
    let p2 = format!("127.0.0.1:{TLS}:8883");
    let (ok, text) = docker(&[
        "run", "-d", "--name", NAME, "-p", &p1, "-p", &p2, "-v", &mount, IMAGE,
    ]);
    let _guard = Container;
    assert!(ok, "docker run failed: {text}");
    wait_ready(0);
    let (_, version) = docker(&["exec", NAME, "mosquitto", "-h"]);
    eprintln!("MEASURE broker: {}", version.lines().next().unwrap_or(""));

    // The logger's retained whole list, then single check-ins.
    publish(
        "netlog/log",
        r#"[{"id": 1, "call": "N0CALL", "name": "Ada Example", "notes": "Net control", "time": "2026-10-26T20:00:00Z"}]"#,
        true,
    );

    // 1. Wrong password: Mosquitto's code 5, in words, no retry.
    let mut h = Harness::start(reader(PLAIN, Tls::Off, "wrong", &["netlog/#"]));
    let seen = h.wait_for(10, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, retrying) = reason(&seen);
    eprintln!("MEASURE wrong password: {why}");
    assert_eq!(
        why,
        "Not authorized: the broker refused the login. Check the user name and password."
    );
    assert!(!retrying);
    h.stop();

    // 2. Plain, right password: retained list, then check-ins, QoS 1. A
    //    topic outside the ACL is asked for too: Mosquitto grants it and
    //    sends nothing, so the quiet hint names it.
    let mut cfg = reader(PLAIN, Tls::Off, "reader-test-pw", &["netlog/#", "other/#"]);
    cfg.quiet_after = Duration::from_secs(3);
    let t = Instant::now();
    let mut h = Harness::start(cfg);
    let seen = h.wait_for(10, retained);
    eprintln!(
        "MEASURE plain connect to retained list: {} ms",
        t.elapsed().as_millis()
    );
    assert!(seen.contains(&MqttEvent::Connected {
        encrypted: false,
        tailscale: false
    }));
    assert!(
        !seen
            .iter()
            .any(|e| matches!(e, MqttEvent::TopicRefused { .. }))
    );
    let checkins = [
        r#"{"id": 7, "call": "N0CALL", "name": "Ada Example", "location": "Portland, OR", "status": "mobile", "traffic": false, "notes": "Portable antenna", "time": "2026-10-26T20:10:00Z"}"#,
        r#"{"callsign": "N1CALL", "name": "Grace Example", "notes": "Mobile, portable antenna"}"#,
        r#"{"call": "N2CALL", "name": "Hostile\u001b[2J\u001b]0;title\u0007 Example\u202e", "notes": "line one\nline two"}"#,
    ];
    let t = Instant::now();
    for m in checkins {
        publish("netlog/checkins", m, false);
    }
    let mut got = Vec::new();
    h.wait_for(10, |e| {
        if let MqttEvent::Message {
            payload,
            retained: false,
            ..
        } = e
        {
            got.push(payload.clone());
        }
        got.len() == 3
    });
    eprintln!(
        "MEASURE three mosquitto_pub check-ins delivered in {} ms (includes docker exec)",
        t.elapsed().as_millis()
    );
    let Payload::Json(hostile) = &got[2] else {
        panic!("{:?}", got[2])
    };
    eprintln!(
        "MEASURE hostile entry after cleaning: name={} notes={}",
        hostile["name"], hostile["notes"]
    );
    assert_eq!(hostile["name"], "Hostile Example");
    assert_eq!(hostile["notes"], "line one line two");
    let seen = h.wait_for(10, |e| matches!(e, MqttEvent::Quiet { .. }));
    eprintln!("MEASURE ACL-denied topic: {:?}", seen.last().unwrap());
    assert_eq!(
        seen.last().unwrap(),
        &MqttEvent::Quiet {
            topic: "other/#".into(),
            seconds: 3
        }
    );

    // 3. Restart the broker: the client reconnects and resubscribes, and
    //    says so once per change.
    let t = Instant::now();
    let (ok, text) = docker(&["restart", NAME]);
    assert!(ok, "docker restart: {text}");
    let restart_ms = t.elapsed().as_millis();
    let seen = h.wait_for(30, retained);
    eprintln!(
        "MEASURE docker restart took {restart_ms} ms; retained list again after {} ms",
        t.elapsed().as_millis()
    );
    for e in &seen {
        if let MqttEvent::Disconnected { reason, retrying } = e {
            eprintln!("MEASURE during restart: {reason} (retrying: {retrying})");
        }
    }
    assert!(
        seen.iter()
            .any(|e| matches!(e, MqttEvent::Connected { .. }))
    );
    assert!(
        !seen
            .iter()
            .any(|e| matches!(e, MqttEvent::Connecting { .. }))
    );
    publish(
        "netlog/checkins",
        r#"{"call": "N3CALL", "name": "After Restart"}"#,
        false,
    );
    h.wait_for(10, |e| {
        matches!(e, MqttEvent::Message { payload: Payload::Json(v), .. } if v["call"] == "N3CALL")
    });
    h.stop();

    // 4. TLS with the CA file, by IP and by name ("localhost" tries ::1
    //    and 127.0.0.1 at once).
    for host in ["127.0.0.1", "localhost"] {
        let mut cfg = reader(
            TLS,
            Tls::CaFile(ca.clone()),
            "reader-test-pw",
            &["netlog/#"],
        );
        cfg.host = host.into();
        let t = Instant::now();
        let mut h = Harness::start(cfg);
        let seen = h.wait_for(10, retained);
        eprintln!(
            "MEASURE TLS ({host}) connect to retained list: {} ms",
            t.elapsed().as_millis()
        );
        assert!(seen.contains(&MqttEvent::Connected {
            encrypted: true,
            tailscale: false
        }));
        h.stop();
    }

    // 5. TLS with the bundled roots: not trusted.
    let mut h = Harness::start(reader(TLS, Tls::System, "reader-test-pw", &["netlog/#"]));
    let seen = h.wait_for(10, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, retrying) = reason(&seen);
    eprintln!("MEASURE TLS with bundled roots: {why}");
    assert!(why.starts_with("TLS failure: the broker's certificate is not trusted."));
    assert!(!retrying);
    h.stop();

    // 6. Plain client on the TLS port, and TLS client on the plain port.
    let mut h = Harness::start(reader(TLS, Tls::Off, "reader-test-pw", &["netlog/#"]));
    let seen = h.wait_for(15, |e| matches!(e, MqttEvent::Disconnected { .. }));
    eprintln!("MEASURE plain client on the TLS port: {:?}", reason(&seen));
    h.stop();
    let mut h = Harness::start(reader(
        PLAIN,
        Tls::CaFile(ca.clone()),
        "reader-test-pw",
        &["netlog/#"],
    ));
    let seen = h.wait_for(15, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, _) = reason(&seen);
    eprintln!("MEASURE TLS client on the plain port: {why}");
    assert!(why.starts_with("TLS failure:"));
    h.stop();

    // 7. Stopped container: unreachable in words, retrying.
    let (ok, _) = docker(&["stop", NAME]);
    assert!(ok);
    let mut h = Harness::start(reader(PLAIN, Tls::Off, "reader-test-pw", &["netlog/#"]));
    let seen = h.wait_for(15, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, retrying) = reason(&seen);
    eprintln!("MEASURE broker stopped: {why}");
    assert!(retrying);
    h.stop();
}
