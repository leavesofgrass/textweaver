//! The client against the test broker, on 127.0.0.1 only: plain and TLS,
//! with certificates made at test time.

mod support;

use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rumqttc::mqttbytes::v4::ConnectReturnCode;
use support::{Harness, Opts, TestBroker, pki, write_ca};
use textweaver_mqtt::{MqttConfig, MqttEvent, Payload, Secret, Tls};

fn plain(port: u16, topics: &[&str]) -> MqttConfig {
    let mut c = MqttConfig::new("127.0.0.1", port, Tls::Off);
    c.min_backoff = Duration::from_millis(100);
    c.max_backoff = Duration::from_millis(400);
    c.connect_timeout = Duration::from_secs(5);
    c.topics = topics.iter().map(|t| t.to_string()).collect();
    c
}

fn text(ev: &MqttEvent, body: &str) -> bool {
    matches!(ev, MqttEvent::Message { payload: Payload::Text(t), .. } if t == body)
}

fn count(events: &[MqttEvent], pred: impl Fn(&MqttEvent) -> bool) -> usize {
    events.iter().filter(|e| pred(e)).count()
}

fn reason(events: &[MqttEvent]) -> (String, bool) {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            MqttEvent::Disconnected { reason, retrying } => Some((reason.clone(), *retrying)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no Disconnected event in {events:#?}"))
}

#[test]
fn plain_login_subscribe_retained_and_qos1() {
    let broker = TestBroker::start(Opts {
        login: Some(("reader".into(), "s3cret".into())),
        ..Opts::default()
    });
    broker.publish(
        "net/log",
        br#"[{"call":"N0CALL","name":"Ada Example"}]"#,
        true,
    );
    let mut cfg = plain(broker.port, &["net/log", "net/chat/+"]);
    cfg.username = Some("reader".into());
    cfg.password = Some(Secret::new("s3cret"));
    let mut h = Harness::start(cfg);

    let seen = h.wait_for(5, |e| {
        matches!(e, MqttEvent::Message { retained: true, .. })
    });
    assert_eq!(seen[0], MqttEvent::Connecting { attempt: 1 });
    assert!(seen.contains(&MqttEvent::Connected {
        encrypted: false,
        tailscale: false
    }));
    let MqttEvent::Message { topic, payload, .. } = seen.last().unwrap() else {
        unreachable!()
    };
    assert_eq!(topic, "net/log");
    assert_eq!(
        *payload,
        Payload::Json(serde_json::json!([{"call": "N0CALL", "name": "Ada Example"}]))
    );

    broker.publish("net/log", br#"{"call":"N1CALL"}"#, false);
    broker.publish("net/chat/main", b"hello", false);
    broker.publish("other/topic", b"not for us", false);
    let seen = h.wait_for(5, |e| text(e, "hello"));
    assert!(!seen.iter().any(|e| text(e, "not for us")));
    assert!(seen.iter().any(|e| matches!(
        e,
        MqttEvent::Message { payload: Payload::Json(v), retained: false, .. } if v["call"] == "N1CALL"
    )));

    // QoS 1 messages (the retained list and N1CALL; the chat topic is
    // QoS 1 too) were acknowledged, and nothing was published.
    let t = Instant::now();
    while broker.stats().get(&broker.stats().pubacks) < 3 && t.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(broker.stats().get(&broker.stats().pubacks), 3);
    assert_eq!(broker.stats().get(&broker.stats().client_publishes), 0);
    let ids = broker.stats().client_ids.lock().unwrap().clone();
    assert_eq!(ids, vec![h.source().client_id().to_string()]);
    assert!(ids[0].starts_with("tw-") && ids[0].len() == 19);
    assert!(h.wakes.load(Ordering::SeqCst) >= 4);
    h.stop();
}

#[test]
fn wrong_password_is_refused_in_words_and_not_retried() {
    let broker = TestBroker::start(Opts {
        login: Some(("reader".into(), "s3cret".into())),
        ..Opts::default()
    });
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.username = Some("reader".into());
    cfg.password = Some(Secret::new("wrong"));
    let mut h = Harness::start(cfg);
    let seen = h.wait_for(5, |e| matches!(e, MqttEvent::Disconnected { .. }));
    assert_eq!(
        reason(&seen),
        (
            "Not authorized: the broker refused the user name or password.".to_string(),
            false
        )
    );
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(
        broker.stats().get(&broker.stats().refused_logins),
        1,
        "no retry"
    );
    h.stop();
}

#[test]
fn mosquitto_code_5_says_check_the_password() {
    let broker = TestBroker::start(Opts {
        login: Some(("reader".into(), "s3cret".into())),
        refuse_code: Some(ConnectReturnCode::NotAuthorized),
        ..Opts::default()
    });
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.username = Some("reader".into());
    cfg.password = Some(Secret::new("wrong"));
    let mut h = Harness::start(cfg);
    let seen = h.wait_for(5, |e| matches!(e, MqttEvent::Disconnected { .. }));
    assert_eq!(
        reason(&seen),
        (
            "Not authorized: the broker refused the login. Check the user name and password."
                .to_string(),
            false
        )
    );
    h.stop();
}

#[test]
fn unreachable_is_reported_once_and_stops_quickly() {
    // A port with nothing listening: bind, read the port, close.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut h = Harness::start(plain(port, &["net/log"]));
    let seen = h.wait_for(10, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, retrying) = reason(&seen);
    assert!(
        why.starts_with(&format!("Unreachable: cannot connect to 127.0.0.1:{port}")),
        "{why}"
    );
    assert!(retrying);
    // Several retries at 100 to 400 ms, but the status is said once.
    let more = h.collect(1500);
    assert!(
        !more.iter().any(|e| matches!(
            e,
            MqttEvent::Disconnected { .. } | MqttEvent::Connecting { .. }
        )),
        "{more:#?}"
    );
    assert!(h.stop() < Duration::from_secs(2));
}

#[test]
fn status_is_reported_once_per_change_not_per_attempt() {
    // A listener that accepts and closes at once: every attempt fails the
    // same way, after the TCP connection is made, so attempts can be
    // counted.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let accepts = Arc::new(AtomicU64::new(0));
    let a = accepts.clone();
    listener.set_nonblocking(true).unwrap();
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let d = done.clone();
    let server = std::thread::spawn(move || {
        while !d.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((s, _)) => {
                    a.fetch_add(1, Ordering::SeqCst);
                    drop(s);
                }
                Err(_) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    });
    let mut cfg = plain(port, &["net/log"]);
    cfg.max_backoff = Duration::from_millis(200);
    let mut h = Harness::start(cfg);
    let events = h.collect(2000);
    done.store(true, Ordering::SeqCst);
    server.join().unwrap();
    assert!(accepts.load(Ordering::SeqCst) >= 4, "several attempts");
    assert_eq!(
        count(&events, |e| matches!(e, MqttEvent::Connecting { .. })),
        1
    );
    assert_eq!(
        count(&events, |e| matches!(e, MqttEvent::Disconnected { .. })),
        1,
        "{events:#?}"
    );
    let (why, retrying) = reason(&events);
    assert!(
        why.starts_with("Connection lost while logging in to 127.0.0.1:"),
        "{why}"
    );
    assert!(why.ends_with("If this port expects TLS, turn TLS on for this source."));
    assert!(retrying);
    h.stop();
}

/// After a connection that worked, a link dropped while logging in again
/// is a broker restart, not a TLS port: the TLS hint is left out.
#[test]
fn the_tls_hint_is_only_for_a_first_connection() {
    use std::io::{Read, Write};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let d = done.clone();
    let server = std::thread::spawn(move || {
        let mut first = true;
        while !d.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut s, _)) => {
                    if first {
                        // Log the client in (CONNACK, accepted), then
                        // close, as a broker going down would.
                        first = false;
                        s.set_nonblocking(false).unwrap();
                        s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                        let mut buf = [0u8; 256];
                        let _ = s.read(&mut buf);
                        let _ = s.write_all(&[0x20, 0x02, 0x00, 0x00]);
                        std::thread::sleep(Duration::from_millis(300));
                    }
                    drop(s);
                }
                Err(_) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    });
    let mut h = Harness::start(plain(port, &["net/log"]));
    h.wait_for(5, |e| matches!(e, MqttEvent::Connected { .. }));
    let seen = h.wait_for(
        5,
        |e| matches!(e, MqttEvent::Disconnected { reason, .. } if reason.contains("logging in")),
    );
    done.store(true, Ordering::SeqCst);
    server.join().unwrap();
    let (why, retrying) = reason(&seen);
    assert!(retrying);
    assert!(!why.contains("expects TLS"), "{why}");
    h.stop();
}

#[test]
fn keepalive_pings_and_stays_connected() {
    let broker = TestBroker::start(Opts::default());
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.keep_alive = Duration::from_secs(1);
    let mut h = Harness::start(cfg);
    h.wait_for(5, |e| matches!(e, MqttEvent::Connected { .. }));
    std::thread::sleep(Duration::from_millis(3500));
    assert!(broker.stats().get(&broker.stats().pingreqs) >= 2);
    broker.publish("net/log", b"still here", false);
    let seen = h.wait_for(5, |e| text(e, "still here"));
    assert!(
        !seen
            .iter()
            .any(|e| matches!(e, MqttEvent::Disconnected { .. }))
    );
    assert_eq!(broker.stats().keep_alives.lock().unwrap()[0], 1);
    h.stop();
}

#[test]
fn reconnects_and_resubscribes_on_every_connack() {
    let broker = TestBroker::start(Opts::default());
    broker.publish("net/log", b"retained list", true);
    let mut h = Harness::start(plain(broker.port, &["net/log"]));
    h.wait_for(5, |e| text(e, "retained list"));

    broker.kick_all();
    let seen = h.wait_for(5, |e| text(e, "retained list"));
    let (why, retrying) = reason(&seen);
    assert!(why.starts_with("Connection lost to 127.0.0.1:"), "{why}");
    assert!(retrying);
    assert_eq!(
        count(&seen, |e| matches!(e, MqttEvent::Disconnected { .. })),
        1
    );
    assert_eq!(
        count(&seen, |e| matches!(e, MqttEvent::Connected { .. })),
        1
    );
    assert_eq!(
        count(&seen, |e| matches!(e, MqttEvent::Connecting { .. })),
        0
    );
    assert_eq!(broker.stats().get(&broker.stats().subscribes), 2);
    assert_eq!(broker.stats().get(&broker.stats().connects), 2);
    broker.publish("net/log", b"after", false);
    h.wait_for(5, |e| text(e, "after"));
    h.stop();
}

#[test]
fn oversize_message_is_named_acknowledged_and_skipped() {
    let broker = TestBroker::start(Opts::default());
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.limits.max_payload = 64 * 1024;
    let mut h = Harness::start(cfg);
    h.wait_for(5, |e| matches!(e, MqttEvent::Connected { .. }));
    // Let the SUBACK arrive before publishing.
    std::thread::sleep(Duration::from_millis(200));
    broker.publish("net/log", &vec![b'x'; 200 * 1024], false);
    broker.publish("net/log", b"small", false);
    let seen = h.wait_for(5, |e| text(e, "small"));
    assert!(seen.contains(&MqttEvent::Message {
        topic: "net/log".into(),
        payload: Payload::TooLarge { len: 200 * 1024 },
        retained: false,
    }));
    assert!(
        !seen
            .iter()
            .any(|e| matches!(e, MqttEvent::Disconnected { .. }))
    );
    let t = Instant::now();
    while broker.stats().get(&broker.stats().pubacks) < 2 && t.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(broker.stats().get(&broker.stats().pubacks), 2);
    h.stop();
}

#[test]
fn json_caps_and_sanitizing_end_to_end() {
    let broker = TestBroker::start(Opts::default());
    let mut cfg = plain(broker.port, &["net/#"]);
    cfg.limits.max_depth = 3;
    cfg.limits.max_fields = 10;
    let mut h = Harness::start(cfg);
    h.wait_for(5, |e| matches!(e, MqttEvent::Connected { .. }));
    std::thread::sleep(Duration::from_millis(200));
    broker.publish("net/a", b"[[[[1]]]]", false);
    broker.publish("net/b", b"[1,2,3,4,5,6,7,8,9,10,11]", false);
    broker.publish("net/c", &[0xff, 0x00, 0xfe], false);
    broker.publish(
        "net/d\u{1b}[2J\u{202e}",
        br#"{"call\u001b[31m":"N2CALL","name":"Hostile\u001b]0;title\u0007 Example\u202e","notes":"line one\nline two"}"#,
        false,
    );
    // Raw control characters make it invalid JSON: cleaned text.
    broker.publish(
        "net/e",
        "{\"a\":\"plain\u{1b}[2J text\u{0}\"}".as_bytes(),
        false,
    );
    let seen = h.wait_for(
        5,
        |e| matches!(e, MqttEvent::Message { topic, .. } if topic == "net/e"),
    );
    let msgs: Vec<(String, Payload)> = seen
        .into_iter()
        .filter_map(|e| match e {
            MqttEvent::Message { topic, payload, .. } => Some((topic, payload)),
            _ => None,
        })
        .collect();
    assert_eq!(msgs[0], ("net/a".into(), Payload::TooLarge { len: 9 }));
    assert_eq!(msgs[1], ("net/b".into(), Payload::TooLarge { len: 25 }));
    assert_eq!(msgs[2], ("net/c".into(), Payload::Binary { len: 3 }));
    assert_eq!(msgs[3].0, "net/d");
    assert_eq!(
        msgs[3].1,
        Payload::Json(serde_json::json!({
            "call": "N2CALL",
            "name": "Hostile Example",
            "notes": "line one line two"
        }))
    );
    assert_eq!(
        msgs[4],
        (
            "net/e".into(),
            Payload::Text(r#"{"a":"plain text"}"#.into())
        )
    );
    h.stop();
}

#[test]
fn quiet_topic_is_reported_once_until_a_message_arrives() {
    let broker = TestBroker::start(Opts::default());
    let mut cfg = plain(broker.port, &["net/busy", "net/quiet/#"]);
    cfg.quiet_after = Duration::from_secs(1);
    let mut h = Harness::start(cfg);
    h.wait_for(5, |e| matches!(e, MqttEvent::Connected { .. }));
    let mut events = Vec::new();
    let t = Instant::now();
    while t.elapsed() < Duration::from_millis(2600) {
        broker.publish("net/busy", b"tick", false);
        events.extend(h.collect(200));
    }
    let quiet: Vec<&MqttEvent> = events
        .iter()
        .filter(|e| matches!(e, MqttEvent::Quiet { .. }))
        .collect();
    assert_eq!(
        quiet,
        vec![&MqttEvent::Quiet {
            topic: "net/quiet/#".into(),
            seconds: 1
        }],
        "only the quiet topic, once"
    );
    // A message resets it; a second quiet spell is reported again.
    broker.publish("net/quiet/x", b"hello", false);
    h.wait_for(5, |e| text(e, "hello"));
    let seen = h.wait_for(
        5,
        |e| matches!(e, MqttEvent::Quiet { topic, .. } if topic == "net/quiet/#"),
    );
    assert!(
        !seen
            .iter()
            .any(|e| matches!(e, MqttEvent::Disconnected { .. }))
    );
    h.stop();
}

#[test]
fn refused_topics_are_named() {
    let broker = TestBroker::start(Opts {
        refuse_prefix: Some("secret/".into()),
        ..Opts::default()
    });
    let mut h = Harness::start(plain(broker.port, &["secret/log", "net/log"]));
    h.wait_for(
        5,
        |e| matches!(e, MqttEvent::TopicRefused { topic } if topic == "secret/log"),
    );
    broker.publish("net/log", b"granted", false);
    h.wait_for(5, |e| text(e, "granted"));
    h.stop();

    // Every topic refused: the client stops and says so.
    let mut h = Harness::start(plain(broker.port, &["secret/log"]));
    let seen = h.wait_for(5, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, retrying) = reason(&seen);
    assert!(
        why.starts_with("Not authorized: the broker refused every topic"),
        "{why}"
    );
    assert!(!retrying);
    h.stop();
}

#[test]
fn slow_reader_gets_a_count_not_a_blocked_connection() {
    let broker = TestBroker::start(Opts::default());
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.queue = 16;
    cfg.keep_alive = Duration::from_secs(1);
    let mut h = Harness::start(cfg);
    h.wait_for(5, |e| matches!(e, MqttEvent::Connected { .. }));
    std::thread::sleep(Duration::from_millis(200));
    for i in 0..100 {
        broker.publish("net/log", format!("m{i}").as_bytes(), false);
    }
    // Do not read for a while: the client must keep the connection alive.
    std::thread::sleep(Duration::from_millis(2500));
    assert!(
        broker.stats().get(&broker.stats().pingreqs) >= 1,
        "keepalive kept running while the queue was full"
    );
    broker.publish("net/log", b"last", false);
    let seen = h.wait_for(5, |e| text(e, "last"));
    let skipped: u64 = seen
        .iter()
        .filter_map(|e| match e {
            MqttEvent::Skipped { count } => Some(*count),
            _ => None,
        })
        .sum();
    let got = count(&seen, |e| matches!(e, MqttEvent::Message { .. })) as u64;
    assert!(skipped > 0);
    assert_eq!(got + skipped, 101, "every message is delivered or counted");
    h.stop();
}

#[test]
fn settings_errors_stop_before_connecting() {
    let cases: [(MqttConfig, &str); 4] = [
        (plain(1883, &[]), "Settings error: no topic is set."),
        (
            plain(1883, &["a/#/b"]),
            "Settings error: # may only be a whole last level of a topic: a/#/b.",
        ),
        (
            {
                let mut c = plain(1883, &["a"]);
                c.tls = Tls::CaFile("no-such-ca.pem".into());
                c
            },
            "Settings error: cannot read the CA file no-such-ca.pem",
        ),
        (
            {
                let mut c = plain(1883, &["a"]);
                c.password = Some(Secret::new("x"));
                c
            },
            "Settings error: a password needs a user name.",
        ),
    ];
    for (cfg, want) in cases {
        let mut h = Harness::start(cfg);
        let seen = h.wait_for(5, |e| matches!(e, MqttEvent::Disconnected { .. }));
        assert_eq!(seen.len(), 1, "{seen:#?}");
        let (why, retrying) = reason(&seen);
        assert!(why.starts_with(want), "{why}");
        assert!(!retrying);
        h.stop();
    }
}

#[test]
fn tls_with_a_ca_file_by_name_and_by_ip() {
    let p = pki(&["localhost", "127.0.0.1"]);
    let ca = write_ca("ca-good.pem", &p.ca_pem);
    let broker = TestBroker::start(Opts {
        tls: Some(p.server.clone()),
        login: Some(("reader".into(), "s3cret".into())),
        ..Opts::default()
    });
    broker.publish("net/log", b"over tls", true);
    for host in ["localhost", "127.0.0.1"] {
        let mut cfg = plain(broker.port, &["net/log"]);
        cfg.host = host.into();
        cfg.tls = Tls::CaFile(ca.clone());
        cfg.username = Some("reader".into());
        cfg.password = Some(Secret::new("s3cret"));
        let t = Instant::now();
        let mut h = Harness::start(cfg);
        let seen = h.wait_for(5, |e| text(e, "over tls"));
        eprintln!(
            "MEASURE TLS connect to first retained message ({host}): {} ms",
            t.elapsed().as_millis()
        );
        assert!(seen.contains(&MqttEvent::Connected {
            encrypted: true,
            tailscale: false
        }));
        h.stop();
    }
}

#[test]
fn tls_failures_are_named_and_not_retried() {
    let p = pki(&["localhost"]);
    let broker = TestBroker::start(Opts {
        tls: Some(p.server.clone()),
        ..Opts::default()
    });

    // The bundled roots do not trust the test CA.
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.host = "localhost".into();
    cfg.tls = Tls::System;
    let mut h = Harness::start(cfg);
    let seen = h.wait_for(5, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, retrying) = reason(&seen);
    assert!(
        why.starts_with("TLS failure: the broker's certificate is not trusted."),
        "{why}"
    );
    assert!(!retrying);
    h.stop();

    // Right CA, wrong name: the certificate has no 127.0.0.1 entry.
    let ca = write_ca("ca-name.pem", &p.ca_pem);
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.tls = Tls::CaFile(ca);
    let mut h = Harness::start(cfg);
    let seen = h.wait_for(5, |e| matches!(e, MqttEvent::Disconnected { .. }));
    assert_eq!(
        reason(&seen).0,
        "TLS failure: the broker's certificate is for a different host name."
    );
    h.stop();

    // A different CA altogether.
    let other = pki(&["localhost"]);
    let ca = write_ca("ca-other.pem", &other.ca_pem);
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.host = "localhost".into();
    cfg.tls = Tls::CaFile(ca);
    let mut h = Harness::start(cfg);
    let seen = h.wait_for(5, |e| matches!(e, MqttEvent::Disconnected { .. }));
    assert!(reason(&seen).0.starts_with("TLS failure:"));
    h.stop();
}

#[test]
fn tls_client_against_a_plain_port_says_tls() {
    let broker = TestBroker::start(Opts::default());
    let p = pki(&["localhost"]);
    let ca = write_ca("ca-plainport.pem", &p.ca_pem);
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.host = "localhost".into();
    cfg.tls = Tls::CaFile(ca);
    let mut h = Harness::start(cfg);
    let seen = h.wait_for(15, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, _) = reason(&seen);
    assert!(why.starts_with("TLS failure:"), "{why}");
    h.stop();
}

#[test]
fn plain_client_against_a_tls_port_is_named() {
    let p = pki(&["localhost"]);
    let broker = TestBroker::start(Opts {
        tls: Some(p.server.clone()),
        ..Opts::default()
    });
    let mut cfg = plain(broker.port, &["net/log"]);
    cfg.connect_timeout = Duration::from_secs(2);
    let mut h = Harness::start(cfg);
    let seen = h.wait_for(10, |e| matches!(e, MqttEvent::Disconnected { .. }));
    let (why, retrying) = reason(&seen);
    assert!(why.starts_with("TLS needed: 127.0.0.1:"), "{why}");
    assert!(!retrying);
    h.stop();
}
