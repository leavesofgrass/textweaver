//! `tw broker account`, `tw send` and `tw share`, against textweaver's own
//! broker running inside this test program on 127.0.0.1. Never a real
//! broker, never audio. The data folders are under the target directory.

use std::io::Write as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use textweaver_broker::{Account, Accounts, Broker, BrokerConfig, Role};
use textweaver_mqtt::{MqttConfig, MqttEvent, MqttSource, Payload, Secret, Tls};

/// A fresh data folder for one test, under the target directory.
fn home(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("live-send")
        .join(format!("{name}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `tw` with `args`, `input` on standard input, and `password` in
/// TEXTWEAVER_MQTT_PASSWORD when given.
fn tw(home: &Path, args: &[&str], input: &str, password: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tw"));
    cmd.args(args)
        .env("TEXTWEAVER_HOME", home)
        .env("TEXTWEAVER_ESPEAK_OUTPUT", "virtual")
        .env_remove("TEXTWEAVER_MQTT_PASSWORD")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(p) = password {
        cmd.env("TEXTWEAVER_MQTT_PASSWORD", p);
    }
    let mut child = cmd.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();
    drop(stdin);
    child.wait_with_output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn accounts_are_added_listed_and_removed_without_a_password_on_the_line() {
    let h = home("accounts");
    let out = tw(
        &h,
        &[
            "broker",
            "account",
            "add",
            "netcontrol",
            "--role",
            "logger",
            "--read",
            "netlog/#",
            "--publish",
            "netlog/#",
        ],
        "example-logger-password\n",
        None,
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "Added: netcontrol, logger, reads netlog/#, publishes netlog/#.\n"
    );
    let out = tw(
        &h,
        &["broker", "account", "add", "ada", "--role", "reader"],
        "example-reader-password\n",
        None,
    );
    assert!(out.status.success(), "{}", stderr(&out));

    // The file holds hashes, never the passwords.
    let file = std::fs::read_to_string(h.join("data").join("broker.toml")).unwrap();
    assert!(!file.contains("example-logger-password"));
    assert!(file.contains("pbkdf2-sha256$600000$"));
    let accounts = Accounts::parse(&file).unwrap();
    assert!(
        accounts
            .get("ada")
            .unwrap()
            .verify("example-reader-password")
    );

    let out = tw(&h, &["broker", "account", "list", "--json"], "", None);
    let rows: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(rows[1]["name"], "ada");
    assert_eq!(rows[1]["read"][0], "#");
    assert!(!stdout(&out).contains("pbkdf2"));

    // Mistakes are one line, before any password is read.
    let out = tw(
        &h,
        &["broker", "account", "add", "ada", "--role", "reader"],
        "x\n",
        None,
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stderr(&out),
        "Error: Account error: ada already exists. Give --replace to change it.\n"
    );
    let out = tw(
        &h,
        &[
            "broker",
            "account",
            "add",
            "bob",
            "--role",
            "reader",
            "--publish",
            "x",
        ],
        "x\n",
        None,
    );
    assert!(stderr(&out).starts_with("Error: A reader may not publish."));
    let out = tw(
        &h,
        &["broker", "account", "add", "bob", "--role", "logger"],
        "x\n",
        None,
    );
    assert!(stderr(&out).starts_with("Error: A logger needs at least one --publish topic"));

    let out = tw(&h, &["broker", "account", "remove", "ada"], "", None);
    assert_eq!(stdout(&out), "Removed: ada.\n");
    let out = tw(&h, &["broker", "account", "list"], "", None);
    assert_eq!(
        stdout(&out),
        "netcontrol, logger, reads netlog/#, publishes netlog/#.\n"
    );

    let out = tw(&h, &["broker", "pin", "--json"], "", None);
    let pin: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(pin["made"], true);
    assert_eq!(pin["pin"].as_str().unwrap().len(), 64);
}

fn start_broker() -> Broker {
    let mut accounts = Accounts::default();
    accounts
        .add(
            Account::with_iterations(
                "netcontrol",
                Role::Logger,
                vec!["netlog/#".into()],
                vec!["netlog/#".into()],
                "logger-pw",
                1_000,
            )
            .unwrap(),
            false,
        )
        .unwrap();
    accounts
        .add(
            Account::with_iterations(
                "ada",
                Role::Reader,
                vec!["netlog/#".into()],
                vec![],
                "reader-pw",
                1_000,
            )
            .unwrap(),
            false,
        )
        .unwrap();
    let mut cfg = BrokerConfig::new(home("broker-data"));
    cfg.tls = false;
    cfg.listen = vec![SocketAddr::from(([127, 0, 0, 1], 0))];
    cfg.accounts = accounts;
    Broker::start(cfg, |_| {}).unwrap()
}

/// Everything a reader receives within `secs`.
fn read_all(port: u16, secs: u64) -> Vec<MqttEvent> {
    let mut c = MqttConfig::new("127.0.0.1", port, Tls::Off);
    c.username = Some("ada".into());
    c.password = Some(Secret::new("reader-pw"));
    c.topics = vec!["netlog/#".into()];
    let source = MqttSource::start(c, Box::new(|| {}));
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        seen.extend(source.poll());
        if seen
            .iter()
            .filter(|e| matches!(e, MqttEvent::Message { .. }))
            .count()
            >= 2
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    source.stop();
    seen
}

#[test]
fn send_and_share_reach_a_reader() {
    let broker = start_broker();
    let port = broker.local_addrs()[0].port().to_string();
    let h = home("send");
    let conn = ["--plain", "--port", port.as_str(), "--user", "netcontrol"];

    let mut args = vec![
        "send",
        "--topic",
        "netlog/checkins",
        "--retain",
        "--field",
        "callsign=N0CALL",
        "--field",
        "name=Ada Example",
    ];
    args.extend(conn);
    let out = tw(&h, &args, "", Some("logger-pw"));
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!("Sent: one entry to netlog/checkins on 127.0.0.1:{port}, retained.\n")
    );

    let doc = h.join("agenda.md");
    std::fs::write(&doc, "# Agenda\n\n1. Check-ins\n").unwrap();
    let mut args = vec![
        "share",
        doc.to_str().unwrap(),
        "--topic",
        "netlog/agenda",
        "--once",
    ];
    args.extend(conn);
    let out = tw(&h, &args, "", Some("logger-pw"));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with("Shared: revision 1 of agenda.md,"),
        "{}",
        stdout(&out)
    );

    let seen = read_all(broker.local_addrs()[0].port(), 5);
    let json: Vec<&serde_json::Value> = seen
        .iter()
        .filter_map(|e| match e {
            MqttEvent::Message {
                payload: Payload::Json(j),
                retained: true,
                ..
            } => Some(j),
            _ => None,
        })
        .collect();
    assert!(
        json.iter()
            .any(|j| j["callsign"] == "N0CALL" && j["name"] == "Ada Example")
    );
    let d = json
        .iter()
        .find(|j| j["type"] == "textweaver.document")
        .expect("the document");
    assert_eq!(d["rev"], 1);
    assert_eq!(d["title"], "agenda");
    assert_eq!(d["format"], "markdown");
    // A document's text keeps its line breaks, still cleaned.
    assert_eq!(d["text"], "# Agenda\n\n1. Check-ins\n");

    // From standard input, and refusals in one line.
    let mut args = vec!["send", "--topic", "netlog/checkins"];
    args.extend(conn);
    let out = tw(&h, &args, r#"{"callsign": "N1TEST"}"#, Some("logger-pw"));
    assert!(out.status.success(), "{}", stderr(&out));
    let out = tw(&h, &args, "[1, 2]", Some("logger-pw"));
    assert!(stderr(&out).starts_with("Error: The entry must be one JSON object"));
    let out = tw(&h, &args, "{}", None);
    assert_eq!(
        stderr(&out),
        "Error: No password for netcontrol. Set TEXTWEAVER_MQTT_PASSWORD, or run this in a terminal to be asked.\n"
    );
    let reader = [
        "send",
        "--topic",
        "netlog/checkins",
        "--plain",
        "--port",
        &port,
        "--user",
        "ada",
    ];
    let out = tw(&h, &reader, "{}", Some("reader-pw"));
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).starts_with("Error: Not sent: Refused: the broker closed the connection"),
        "{}",
        stderr(&out)
    );
    let out = tw(
        &h,
        &["send", "--topic", "t", "--join-code", "ABCD-EFGH-JKUN-PQRS"],
        "{}",
        None,
    );
    assert_eq!(
        stderr(&out),
        "Error: Join code error: U in group 3 is not used in join codes. Check that character.\n"
    );
    broker.stop();
}
