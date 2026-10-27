//! The broker on 127.0.0.1 only, with textweaver's own receiver and
//! publisher and with raw packets. Never a real broker on the network, and
//! never audio. Key files go to this test's folder in the target directory.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::BytesMut;
use rumqttc::mqttbytes::QoS;
use rumqttc::mqttbytes::v4::{Connect, Packet, Publish, Subscribe, SubscribeReasonCode};
use textweaver_broker::{Account, Accounts, Broker, BrokerConfig, BrokerEvent, BrokerLimits, Role};
use textweaver_mqtt::{
    JoinCode, MqttConfig, MqttEvent, MqttSource, Payload, Pin, Publisher, Secret, Tls,
};

const READER_PW: &str = "reader-example-password";
const LOGGER_PW: &str = "logger-example-password";

/// A folder of this run's own under the target directory, so a key made
/// by an earlier run is never reused.
fn data_dir(name: &str) -> PathBuf {
    static RUN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let run = RUN.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("run-{}-{nanos}", std::process::id())
    });
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("broker")
        .join(run)
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn accounts() -> Accounts {
    let mut a = Accounts::default();
    a.add(
        Account::with_iterations(
            "netcontrol",
            Role::Logger,
            vec!["netlog/#".into()],
            vec!["netlog/#".into()],
            LOGGER_PW,
            1_000,
        )
        .unwrap(),
        false,
    )
    .unwrap();
    a.add(
        Account::with_iterations(
            "ada",
            Role::Reader,
            vec!["netlog/#".into()],
            vec![],
            READER_PW,
            1_000,
        )
        .unwrap(),
        false,
    )
    .unwrap();
    a
}

type Events = Arc<Mutex<Vec<BrokerEvent>>>;

fn start(name: &str, tls: bool, join: bool) -> (Broker, Events) {
    start_with(name, tls, join, BrokerLimits::default())
}

fn start_with(name: &str, tls: bool, join: bool, limits: BrokerLimits) -> (Broker, Events) {
    let mut cfg = BrokerConfig::new(data_dir(name));
    cfg.listen = vec![SocketAddr::from(([127, 0, 0, 1], 0))];
    cfg.tls = tls;
    cfg.join_code = join;
    cfg.join_topics = vec!["netlog/#".into()];
    cfg.accounts = accounts();
    cfg.limits = limits;
    let events: Events = Arc::default();
    let seen = events.clone();
    let broker = Broker::start(cfg, move |e| seen.lock().unwrap().push(e)).unwrap();
    (broker, events)
}

fn port(b: &Broker) -> u16 {
    b.local_addrs()[0].port()
}

fn config(port: u16, tls: Tls, user: &str, password: &str) -> MqttConfig {
    let mut c = MqttConfig::new("127.0.0.1", port, tls);
    c.username = Some(user.into());
    c.password = Some(Secret::new(password));
    c.connect_timeout = Duration::from_secs(5);
    c.min_backoff = Duration::from_millis(100);
    c.max_backoff = Duration::from_millis(400);
    c.topics = vec!["netlog/#".into()];
    c
}

/// A receiver and the events it has reported so far.
struct Reader {
    source: Option<MqttSource>,
    seen: Vec<MqttEvent>,
}

impl Reader {
    fn start(cfg: MqttConfig) -> Self {
        Reader {
            source: Some(MqttSource::start(cfg, Box::new(|| {}))),
            seen: Vec::new(),
        }
    }

    /// Waits up to `secs` for an event that `pred` accepts.
    fn wait(&mut self, secs: u64, pred: impl Fn(&MqttEvent) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(secs);
        loop {
            let new = self.source.as_ref().unwrap().poll();
            let hit = new.iter().any(&pred);
            self.seen.extend(new);
            if hit {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn stop(mut self) {
        self.source.take().unwrap().stop();
    }
}

fn text_is(e: &MqttEvent, body: &str) -> bool {
    matches!(e, MqttEvent::Message { payload: Payload::Text(t), .. } if t == body)
}

fn json_has(e: &MqttEvent, key: &str, value: &str) -> bool {
    matches!(e, MqttEvent::Message { payload: Payload::Json(j), .. } if j[key] == value)
}

fn down(e: &MqttEvent) -> Option<&str> {
    match e {
        MqttEvent::Disconnected { reason, .. } => Some(reason),
        _ => None,
    }
}

fn wait_event(events: &Events, secs: u64, pred: impl Fn(&BrokerEvent) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if events.lock().unwrap().iter().any(&pred) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn a_logger_publishes_and_a_reader_reads_live_and_retained() {
    let (broker, events) = start("plain", false, false);
    let p = port(&broker);
    assert!(matches!(
        events.lock().unwrap()[0],
        BrokerEvent::Listening { tls: false, .. }
    ));

    // Retained before anyone reads: a late subscriber gets it.
    let mut logger = Publisher::connect(&config(p, Tls::Off, "netcontrol", LOGGER_PW)).unwrap();
    logger
        .publish("netlog/checkins", br#"{"callsign":"N0CALL"}"#, true)
        .unwrap();

    let mut reader = Reader::start(config(p, Tls::Off, "ada", READER_PW));
    assert!(reader.wait(5, |e| json_has(e, "callsign", "N0CALL")));
    assert!(
        reader
            .seen
            .iter()
            .any(|e| matches!(e, MqttEvent::Message { retained: true, .. }))
    );

    // Live, not retained.
    logger
        .publish("netlog/checkins", b"live entry", false)
        .unwrap();
    assert!(reader.wait(5, |e| text_is(e, "live entry")));
    logger.disconnect();

    // From inside the program, as the app will.
    broker.publish("netlog/doc", b"from the app", true).unwrap();
    assert!(reader.wait(5, |e| text_is(e, "from the app")));
    assert!(broker.publish("netlog/+", b"x", false).is_err());

    assert!(wait_event(&events, 5, |e| matches!(
        e,
        BrokerEvent::Connected { user, .. } if user == "ada"
    )));
    reader.stop();
    broker.stop();
    assert_eq!(events.lock().unwrap().last(), Some(&BrokerEvent::Stopped));
}

#[test]
fn a_reader_that_publishes_is_refused_and_dropped() {
    let (broker, events) = start("reader-publish", false, false);
    let p = port(&broker);
    let mut watcher = Reader::start(config(p, Tls::Off, "netcontrol", LOGGER_PW));
    assert!(watcher.wait(5, |e| matches!(e, MqttEvent::Connected { .. })));

    let mut reader = Publisher::connect(&config(p, Tls::Off, "ada", READER_PW)).unwrap();
    let err = reader
        .publish("netlog/checkins", b"fake entry", false)
        .unwrap_err();
    assert!(
        err.text()
            .starts_with("Refused: the broker closed the connection"),
        "{err}"
    );
    assert!(wait_event(&events, 5, |e| matches!(
        e,
        BrokerEvent::PublishRefused { user, topic } if user == "ada" && topic == "netlog/checkins"
    )));
    // Nobody received the fake entry.
    assert!(!watcher.wait(1, |e| text_is(e, "fake entry")));

    // A logger outside its topics is refused the same way.
    let mut logger = Publisher::connect(&config(p, Tls::Off, "netcontrol", LOGGER_PW)).unwrap();
    assert!(logger.publish("other/topic", b"x", false).is_err());
    watcher.stop();
    broker.stop();
}

#[test]
fn subscriptions_outside_the_read_topics_are_refused() {
    let (broker, _) = start("subscribe", false, false);
    let mut cfg = config(port(&broker), Tls::Off, "ada", READER_PW);
    cfg.topics = vec!["netlog/#".into(), "#".into(), "private/x".into()];
    let mut reader = Reader::start(cfg);
    reader.wait(
        5,
        |e| matches!(e, MqttEvent::TopicRefused { topic } if topic == "private/x"),
    );
    reader.wait(1, |_| false);
    let refused: Vec<&str> = reader
        .seen
        .iter()
        .filter_map(|e| match e {
            MqttEvent::TopicRefused { topic } => Some(topic.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(refused, ["#", "private/x"]);
    assert!(
        !reader
            .seen
            .iter()
            .any(|e| matches!(e, MqttEvent::Disconnected { .. }))
    );
    reader.stop();
    broker.stop();
}

#[test]
fn wrong_passwords_are_refused_then_the_address_is_locked_out() {
    let limits = BrokerLimits {
        max_fails: 5,
        ..BrokerLimits::default()
    };
    let (broker, events) = start_with("lockout", false, false, limits);
    let p = port(&broker);
    for _ in 0..5 {
        let err = Publisher::connect(&config(p, Tls::Off, "ada", "wrong"))
            .err()
            .unwrap();
        assert!(err.text().starts_with("Not authorized:"), "{err}");
    }
    assert!(wait_event(&events, 5, |e| matches!(
        e,
        BrokerEvent::LockedOut { minutes: 5, .. }
    )));
    assert!(events.lock().unwrap().iter().any(|e| {
        e.to_string()
            .starts_with("Login refused: wrong user name or password for ada")
    }));
    // Even the right password is turned away while locked out.
    let err = Publisher::connect(&config(p, Tls::Off, "ada", READER_PW))
        .err()
        .unwrap();
    assert!(!err.text().starts_with("Not authorized"), "{err}");
    // An unknown name is refused like a wrong password.
    let (broker2, _) = start("unknown-user", false, false);
    let err = Publisher::connect(&config(port(&broker2), Tls::Off, "nobody", "x"))
        .err()
        .unwrap();
    assert!(err.text().starts_with("Not authorized:"), "{err}");
    broker2.stop();
    broker.stop();
}

#[test]
fn pinned_tls_accepts_the_right_pin_and_keeps_it_across_restarts() {
    let (broker, events) = start("pinned", true, false);
    let p = port(&broker);
    let pin = broker.pin().unwrap().clone();
    assert!(!pin.is_short());
    assert!(broker.made_new_identity());
    assert!(matches!(
        events.lock().unwrap()[0],
        BrokerEvent::Listening { tls: true, .. }
    ));

    let mut logger = Publisher::connect(&config(
        p,
        Tls::Pinned(pin.clone()),
        "netcontrol",
        LOGGER_PW,
    ))
    .unwrap();
    logger.publish("netlog/pinned", b"over tls", true).unwrap();
    logger.disconnect();

    let mut reader = Reader::start(config(p, Tls::Pinned(pin.clone()), "ada", READER_PW));
    assert!(reader.wait(5, |e| text_is(e, "over tls")));
    assert!(reader.seen.contains(&MqttEvent::Connected {
        encrypted: true,
        tailscale: false
    }));
    reader.stop();

    let wrong = Pin::of_spki(b"not the broker's key");
    let mut reader = Reader::start(config(p, Tls::Pinned(wrong), "ada", READER_PW));
    assert!(reader.wait(5, |e| down(e).is_some()));
    let why = reader.seen.iter().find_map(down).unwrap().to_string();
    assert!(
        why.starts_with("TLS failure: the broker's key does not match the pin."),
        "{why}"
    );
    reader.stop();

    // A plain client on the TLS port is named, not hung.
    let err = Publisher::connect(&config(p, Tls::Off, "ada", READER_PW))
        .err()
        .unwrap();
    assert!(!err.text().is_empty());
    broker.stop();

    let (again, _) = start("pinned", true, false);
    assert_eq!(again.pin(), Some(&pin));
    assert!(!again.made_new_identity());
    again.stop();
}

#[test]
fn a_join_code_reads_but_needs_its_secret_and_never_publishes() {
    let (broker, _) = start("join", true, true);
    let p = port(&broker);
    let code = broker.join_code().unwrap().clone();
    let pin = broker.pin().unwrap().clone();
    assert_eq!(code.pin(), pin.short());
    broker
        .publish("netlog/doc", b"for the class", true)
        .unwrap();

    let joined = JoinCode::parse(&code.grouped().to_lowercase()).unwrap();
    let mut reader = Reader::start(config(
        p,
        Tls::Pinned(joined.pin()),
        textweaver_mqtt::JOIN_USER,
        joined.password().expose(),
    ));
    assert!(reader.wait(5, |e| text_is(e, "for the class")));
    reader.stop();

    // An outsider who read the pin from the certificate, but not the
    // secret, derives a password from a guessed code and is refused.
    let guess = JoinCode::parse(&format!("{}0000", pin.short().as_str())).unwrap();
    assert_ne!(guess, code);
    let err = Publisher::connect(&config(
        p,
        Tls::Pinned(guess.pin()),
        textweaver_mqtt::JOIN_USER,
        guess.password().expose(),
    ))
    .err()
    .unwrap();
    assert!(err.text().starts_with("Not authorized:"), "{err}");

    // The code may only read.
    let mut joiner = Publisher::connect(&config(
        p,
        Tls::Pinned(code.pin()),
        textweaver_mqtt::JOIN_USER,
        code.password().expose(),
    ))
    .unwrap();
    assert!(joiner.publish("netlog/doc", b"fake", true).is_err());

    // Accounts still work beside the code, with the session's pin.
    let mut logger =
        Publisher::connect(&config(p, Tls::Pinned(pin), "netcontrol", LOGGER_PW)).unwrap();
    logger.publish("netlog/checkins", b"{}", false).unwrap();
    broker.stop();
}

#[test]
fn stop_frees_the_port_and_ends_every_connection() {
    let (broker, _) = start("stop", false, false);
    let p = port(&broker);
    let mut reader = Reader::start(config(p, Tls::Off, "ada", READER_PW));
    assert!(reader.wait(5, |e| matches!(e, MqttEvent::Connected { .. })));
    let t = Instant::now();
    broker.stop();
    assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
    // The port is free at once.
    let again = TcpListener::bind(("127.0.0.1", p)).expect("the port is free");
    drop(again);
    assert!(reader.wait(5, |e| down(e).is_some()));
    reader.stop();
}

fn raw(port: u16) -> TcpStream {
    let s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s
}

fn write(s: &mut TcpStream, p: Packet) {
    let mut b = BytesMut::new();
    p.write(&mut b, usize::MAX).unwrap();
    s.write_all(&b).unwrap();
}

/// Reads one packet, or `None` when the broker closed the connection.
fn read(s: &mut TcpStream) -> Option<Packet> {
    let mut buf = BytesMut::new();
    let mut chunk = [0u8; 4096];
    loop {
        match Packet::read(&mut buf, 1 << 20) {
            Ok(p) => return Some(p),
            Err(rumqttc::mqttbytes::Error::InsufficientBytes(_)) => match s.read(&mut chunk) {
                Ok(0) | Err(_) => return None,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            },
            Err(e) => panic!("broker sent a bad packet: {e}"),
        }
    }
}

fn login(port: u16, user: &str, pw: &str) -> TcpStream {
    let mut s = raw(port);
    let mut c = Connect::new("raw-test");
    c.set_login(user, pw);
    write(&mut s, Packet::Connect(c));
    match read(&mut s) {
        Some(Packet::ConnAck(a)) => {
            assert_eq!(a.code, rumqttc::mqttbytes::v4::ConnectReturnCode::Success)
        }
        other => panic!("no CONNACK: {other:?}"),
    }
    s
}

#[test]
fn hostile_and_odd_packets_are_handled() {
    let (broker, _) = start("hostile", false, false);
    let p = port(&broker);

    // Garbage instead of CONNECT: closed, nothing sent.
    let mut s = raw(p);
    s.write_all(&[0xff, 0xff, 0xff, 0xff, 0xff]).unwrap();
    assert!(read(&mut s).is_none());

    // MQTT 5: the "unacceptable protocol version" code.
    let mut s = raw(p);
    let mut c = Connect::new("v5");
    c.protocol = rumqttc::mqttbytes::Protocol::V5;
    write(&mut s, Packet::Connect(c));
    match read(&mut s) {
        Some(Packet::ConnAck(a)) => assert_eq!(
            a.code,
            rumqttc::mqttbytes::v4::ConnectReturnCode::RefusedProtocolVersion
        ),
        other => panic!("{other:?}"),
    }

    // No login at all.
    let mut s = raw(p);
    write(&mut s, Packet::Connect(Connect::new("anon")));
    match read(&mut s) {
        Some(Packet::ConnAck(a)) => assert_eq!(
            a.code,
            rumqttc::mqttbytes::v4::ConnectReturnCode::NotAuthorized
        ),
        other => panic!("{other:?}"),
    }

    // A logger: QoS 2 is taken and delivered at QoS 1; a wildcard topic
    // drops the client.
    let mut s = login(p, "netcontrol", LOGGER_PW);
    write(
        &mut s,
        Packet::Subscribe(Subscribe::new("netlog/#", QoS::ExactlyOnce)),
    );
    match read(&mut s) {
        Some(Packet::SubAck(a)) => {
            assert_eq!(
                a.return_codes,
                vec![SubscribeReasonCode::Success(QoS::AtLeastOnce)]
            );
        }
        other => panic!("{other:?}"),
    }
    let mut q2 = Publish::new("netlog/q2", QoS::ExactlyOnce, b"two".to_vec());
    q2.pkid = 9;
    write(&mut s, Packet::Publish(q2));
    let mut got_rec = false;
    let mut got_msg = false;
    for _ in 0..2 {
        match read(&mut s) {
            Some(Packet::PubRec(r)) => got_rec = r.pkid == 9,
            Some(Packet::Publish(m)) => {
                got_msg = m.qos == QoS::AtLeastOnce && m.payload.as_ref() == b"two"
            }
            other => panic!("{other:?}"),
        }
    }
    assert!(got_rec && got_msg);
    let wild = Publish::new("netlog/+", QoS::AtMostOnce, b"x".to_vec());
    write(&mut s, Packet::Publish(wild));
    assert!(read(&mut s).is_none() || read(&mut s).is_none());

    // Over the size cap: dropped from the header alone.
    let mut s = login(p, "netcontrol", LOGGER_PW);
    let mut header = vec![0x30];
    let mut len = 300 * 1024usize;
    loop {
        let mut b = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            b |= 0x80;
        }
        header.push(b);
        if len == 0 {
            break;
        }
    }
    s.write_all(&header).unwrap();
    assert!(read(&mut s).is_none());

    // The broker is still serving.
    let mut s = login(p, "ada", READER_PW);
    write(&mut s, Packet::PingReq);
    assert!(matches!(read(&mut s), Some(Packet::PingResp)));
    broker.stop();
}

#[test]
fn a_full_broker_turns_connections_away_in_words() {
    let limits = BrokerLimits {
        max_clients: 1,
        ..BrokerLimits::default()
    };
    let (broker, events) = start_with("full", false, false, limits);
    let p = port(&broker);
    let _first = login(p, "ada", READER_PW);
    let mut second = raw(p);
    assert!(read(&mut second).is_none());
    assert!(wait_event(&events, 5, |e| e.to_string()
        == "Refused a connection: the broker has 1 clients, its limit."));
    broker.stop();
}

#[test]
fn the_fuzz_seeds_do_not_panic() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/b5/broker");
    let mut n = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        textweaver_broker::fuzz_connection(&std::fs::read(&path).unwrap());
        n += 1;
    }
    assert!(n >= 4, "seeds in {}", dir.display());
    textweaver_broker::fuzz_connection(&[]);
    textweaver_broker::fuzz_connection(&[0x10, 0xff, 0xff, 0xff, 0x7f]);
}

#[test]
fn the_broker_certificate_works_as_a_ca_file_on_this_computer() {
    let (broker, _) = start("ca-file", true, false);
    let cert = data_dir("ca-file").join(textweaver_broker::CERT_FILE);
    for host in ["localhost", "127.0.0.1"] {
        let mut cfg = config(port(&broker), Tls::CaFile(cert.clone()), "ada", READER_PW);
        cfg.host = host.into();
        let p = Publisher::connect(&cfg).unwrap_or_else(|e| panic!("{host}: {e}"));
        p.disconnect();
    }
    broker.stop();
}

#[test]
fn the_example_accounts_file_reads() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples/live/broker.toml");
    let a = Accounts::load(&path).unwrap();
    let logger = a.get("netcontrol").unwrap();
    assert_eq!(logger.role, Role::Logger);
    assert!(logger.verify("example-logger-password"));
    let reader = a.get("ada").unwrap();
    assert_eq!(reader.role, Role::Reader);
    assert!(reader.verify("example-reader-password"));
    assert!(!reader.verify("example-logger-password"));
}
