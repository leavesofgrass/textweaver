//! JSON-RPC session tests (ADR-0015): a scripted session against
//! [`Server`] with a recording speech backend, and the stdio transport with
//! both framings.

use std::io::Cursor;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use textweaver_app::rpc::{self, Server, codes};
use textweaver_app::store::Paths;
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig};

const TEXT: &str = "First sentence here. Second sentence there.\n\nA new paragraph.";

fn server(home: &std::path::Path) -> Server {
    let (speech, _log) = recording_service().unwrap();
    let (announcer, queue) = rpc::announcer();
    let app = App::new(AppConfig {
        speech,
        announcer,
        paths: Some(Paths::under(home)),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    Server::new(app, queue)
}

struct Session {
    server: Server,
    next_id: u64,
    notes: Vec<Value>,
}

impl Session {
    /// Sends a request; returns its response, keeping notifications.
    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let msg = json!({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params});
        let out = self.server.handle(&msg.to_string());
        let mut response = Value::Null;
        for m in out {
            if m.get("id") == Some(&json!(self.next_id)) {
                response = m;
            } else {
                self.notes.push(m);
            }
        }
        assert_eq!(response["jsonrpc"], "2.0", "{response}");
        response
    }

    fn result(&mut self, method: &str, params: Value) -> Value {
        let r = self.call(method, params);
        assert!(r.get("error").is_none(), "{method}: {r}");
        r["result"].clone()
    }

    fn error_code(&mut self, method: &str, params: Value) -> i64 {
        let r = self.call(method, params);
        r["error"]["code"].as_i64().unwrap_or_else(|| panic!("{r}"))
    }

    /// Polls until reading stops, collecting notifications.
    fn wait_stopped(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            self.notes.extend(self.server.poll());
            let stopped = self
                .notes
                .iter()
                .any(|n| n["method"] == "playback" && n["params"]["state"] == "stopped");
            if stopped {
                return;
            }
            assert!(Instant::now() < deadline, "reading never stopped");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn notifications(&self, method: &str) -> Vec<Value> {
        self.notes
            .iter()
            .filter(|n| n["method"] == method)
            .map(|n| n["params"].clone())
            .collect()
    }
}

fn char_of(needle: &str) -> u64 {
    TEXT[..TEXT.find(needle).unwrap()].chars().count() as u64
}

#[test]
fn scripted_json_rpc_session() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.txt");
    std::fs::write(&file, TEXT).unwrap();
    let mut s = Session {
        server: server(&dir.path().join("home")),
        next_id: 0,
        notes: Vec::new(),
    };

    let init = s.result("initialize", json!({}));
    assert_eq!(init["server"], "textweaver");
    assert_eq!(init["protocol"], rpc::PROTOCOL_VERSION);
    assert!(
        init["methods"]
            .as_array()
            .unwrap()
            .contains(&json!("navigate"))
    );

    // Before a document: status works, position is an error.
    assert_eq!(s.result("status", json!({}))["document"], Value::Null);
    assert_eq!(s.error_code("position", json!({})), codes::NO_DOCUMENT);

    // Open.
    let opened = s.result("open", json!({"path": file.display().to_string()}));
    assert_eq!(opened["document"]["title"], "doc.txt");
    assert_eq!(opened["document"]["lines"], 3);
    assert_eq!(opened["position"]["char"], 0);
    let said: Vec<Value> = s.notifications("announcement");
    assert!(
        said.iter().any(|a| a["text"] == "Opened doc.txt."),
        "{said:?}"
    );
    assert_eq!(
        s.error_code("open", json!({"path": "no/such/file.txt"})),
        codes::OPEN_FAILED
    );

    // Navigate by action and by go-to target.
    let pos = s.result("navigate", json!({"action": "next_sentence"}));
    assert_eq!(pos["char"], char_of("Second"));
    assert_eq!(pos["word"]["text"], "Second");
    let pos = s.result("navigate", json!({"goto": "3"}));
    assert_eq!(pos["line"], 3);
    assert_eq!(
        s.error_code("navigate", json!({"action": "bold"})),
        codes::INVALID_PARAMS
    );

    // Search moves to the first match at or after the cursor, wrapping.
    let found = s.result("search", json!({"pattern": "sentence"}));
    assert_eq!(found["matches"].as_array().unwrap().len(), 2);
    assert_eq!(found["current"], 0);
    let pos = s.result("position", json!({}));
    assert_eq!(pos["char"], char_of("sentence"));

    // Read the sentence at the cursor: playback and position notifications
    // follow, each position inside that sentence.
    s.notes.clear();
    let r = s.result("read", json!({"what": "sentence"}));
    assert_eq!(r["playback"], "reading");
    s.wait_stopped();
    let states: Vec<String> = s
        .notifications("playback")
        .iter()
        .map(|p| p["state"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(states.first().map(String::as_str), Some("reading"));
    assert_eq!(states.last().map(String::as_str), Some("stopped"));
    let positions = s.notifications("position");
    assert!(!positions.is_empty());
    let end = char_of("Second");
    for p in &positions {
        assert!(p["end"].as_u64().unwrap() <= end, "{p}");
        assert_eq!(p["line"], 1);
    }

    // Text by range.
    let t = s.result("text", json!({"start": 0, "end": 5}));
    assert_eq!(t["text"], "First");

    // Any action by id; its list comes back as an effect.
    let r = s.result("action", json!({"id": "add_bookmark"}));
    assert!(r["status"].as_str().unwrap().starts_with("Bookmark mark1"));
    let r = s.result("action", json!({"id": "list_bookmarks"}));
    assert_eq!(r["effects"][0]["type"], "list");
    let r = s.result("choose", json!({"index": 0}));
    assert!(r["status"].as_str().unwrap().contains("mark1"), "{r}");
    // A prompt round trip: go to, answered by the client.
    let r = s.result("action", json!({"id": "go_to"}));
    assert_eq!(r["effects"][0]["type"], "prompt");
    assert_eq!(r["effects"][0]["params"]["purpose"], "go_to");
    s.result("answer", json!({"text": "2"}));
    assert_eq!(s.result("position", json!({}))["line"], 2);

    // Errors.
    assert_eq!(s.error_code("fly", json!({})), codes::METHOD_NOT_FOUND);
    assert_eq!(s.error_code("action", json!({})), codes::INVALID_PARAMS);
    let bad = s.server.handle("{not json");
    assert_eq!(bad[0]["error"]["code"], codes::PARSE_ERROR);
    let bad = s.server.handle(r#"{"id": 1, "method": "status"}"#);
    assert_eq!(bad[0]["error"]["code"], codes::INVALID_REQUEST);
    // A notification gets no response.
    let none = s.server.handle(r#"{"jsonrpc": "2.0", "method": "status"}"#);
    assert!(none.iter().all(|m| m.get("id").is_none()));

    // Shutdown saves; afterwards only exit is accepted.
    assert_eq!(s.result("shutdown", json!({})), Value::Null);
    assert_eq!(s.error_code("status", json!({})), codes::SHUT_DOWN);
    s.server.handle(r#"{"jsonrpc": "2.0", "method": "exit"}"#);
    assert!(s.server.is_done());
    let state_dir = Paths::under(&dir.path().join("home")).state_dir();
    assert_eq!(std::fs::read_dir(state_dir).unwrap().count(), 1);
}

#[test]
fn actions_that_ask_first_take_a_confirm_param() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.txt");
    std::fs::write(&file, TEXT).unwrap();
    let mut s = Session {
        server: server(&dir.path().join("home")),
        next_id: 0,
        notes: Vec::new(),
    };
    s.result("open", json!({"path": file.display().to_string()}));
    assert_eq!(s.result("status", json!({}))["pending"], Value::Null);

    // Without `confirm`, quit only asks; the result and status say what.
    let r = s.result("action", json!({"id": "quit"}));
    assert_eq!(r["pending"]["action"], "quit");
    assert_eq!(r["pending"]["question"], "Quit textweaver? y or n");
    assert_eq!(r["status"], "Quit textweaver? y or n");
    assert!(!s.server.is_done());
    assert_eq!(
        s.result("status", json!({}))["pending"]["question"],
        "Quit textweaver? y or n"
    );
    // `cancel` answers no.
    let r = s.result("cancel", json!({}));
    assert_eq!(r["status"], "Cancelled.");
    assert_eq!(s.result("status", json!({}))["pending"], Value::Null);
    // `confirm: false` answers no at once.
    let r = s.result("action", json!({"id": "quit", "confirm": false}));
    assert_eq!(r["pending"], Value::Null);
    assert!(!s.server.is_done());
    assert_eq!(
        s.error_code("action", json!({"id": "quit", "confirm": "yes"})),
        codes::INVALID_PARAMS
    );

    // Deleting a note: asked, then confirmed.
    s.result("action", json!({"id": "add_note"}));
    s.result("answer", json!({"text": "a note"}));
    let r = s.result("action", json!({"id": "delete_note"}));
    assert_eq!(r["pending"]["action"], "delete_note");
    let r = s.result("action", json!({"id": "delete_note", "confirm": true}));
    assert_eq!(r["pending"], Value::Null);
    assert!(
        r["status"].as_str().unwrap().starts_with("Note deleted"),
        "{r}"
    );

    // `confirm: true` quits in one call.
    let r = s.result("action", json!({"id": "quit", "confirm": true}));
    assert!(
        r["effects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["type"] == "quit"),
        "{r}"
    );
    assert!(s.server.is_done());
}

#[test]
fn stdio_transport_handles_both_framings() {
    let dir = tempfile::tempdir().unwrap();
    let body = r#"{"jsonrpc":"2.0","id":2,"method":"status"}"#;
    let input = format!(
        "{}\nContent-Length: {}\r\n\r\n{}{}\n",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        body.len(),
        body,
        r#"{"jsonrpc":"2.0","method":"exit"}"#,
    );
    let mut out = Vec::new();
    rpc::serve(
        server(&dir.path().join("home")),
        Cursor::new(input.into_bytes()),
        &mut out,
    )
    .unwrap();
    let out = String::from_utf8(out).unwrap();
    // The first answer is a plain line; the second is framed like its
    // request.
    let first_line = out.lines().next().unwrap();
    let first: Value = serde_json::from_str(first_line).unwrap();
    assert_eq!(first["id"], 1);
    assert_eq!(first["result"]["server"], "textweaver");
    let framed = out.find("Content-Length: ").expect("framed reply");
    let rest = &out[framed..];
    let (header, payload) = rest.split_once("\r\n\r\n").unwrap();
    let len: usize = header["Content-Length: ".len()..].parse().unwrap();
    let second: Value = serde_json::from_str(&payload[..len]).unwrap();
    assert_eq!(second["id"], 2);
    assert_eq!(second["result"]["playback"], "stopped");
}

#[test]
fn closing_input_ends_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let mut out = Vec::new();
    rpc::serve(
        server(&dir.path().join("home")),
        Cursor::new(b"".to_vec()),
        &mut out,
    )
    .unwrap();
    assert!(out.is_empty());
}

/// Wave 3: lists, prompts, and settings through JSON-RPC, on the app's own
/// list and prompt models and its settings schema.
#[test]
fn lists_prompts_and_settings_over_json_rpc() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.txt");
    std::fs::write(&file, TEXT).unwrap();
    let home = dir.path().join("home");
    let mut s = Session {
        server: server(&home),
        next_id: 0,
        notes: Vec::new(),
    };
    let init = s.result("initialize", json!({}));
    for m in [
        "list_state",
        "list_key",
        "prompt_key",
        "settings_schema",
        "set_setting",
    ] {
        assert!(
            init["methods"].as_array().unwrap().contains(&json!(m)),
            "{m}"
        );
    }
    s.result("open", json!({"path": file.display().to_string()}));
    assert_eq!(s.result("list_state", json!({})), Value::Null);

    // Two bookmarks, then their list: the app keeps the focus.
    s.result("action", json!({"id": "add_bookmark"}));
    s.result("navigate", json!({"action": "next_paragraph"}));
    s.result("action", json!({"id": "add_bookmark"}));
    s.result("action", json!({"id": "list_bookmarks"}));
    let list = s.result("list_state", json!({}));
    assert_eq!(list["items"].as_array().unwrap().len(), 2, "{list}");
    assert_eq!(list["selected"], 0);
    let r = s.result("list_key", json!({"key": "down"}));
    assert_eq!(r["list"]["selected"], 1);
    assert!(r["status"].as_str().unwrap().starts_with("2 of 2, "), "{r}");
    let r = s.result("list_key", json!({"key": "down"}));
    assert_eq!(r["status"], "End of list.");
    let r = s.result("list_key", json!({"key": "escape"}));
    assert_eq!(r["list"], Value::Null);

    // The find prompt: typed, recalled, answered.
    s.result("action", json!({"id": "find"}));
    let r = s.result("prompt_key", json!({"text": "new"}));
    assert_eq!(r["prompt"]["text"], "new");
    assert_eq!(r["prompt"]["purpose"], "find");
    s.result("prompt_key", json!({"key": "enter"}));
    assert_eq!(s.result("prompt_state", json!({})), Value::Null);
    assert_eq!(s.result("position", json!({}))["char"], char_of("new"));
    s.result("action", json!({"id": "find"}));
    let r = s.result("prompt_key", json!({"key": "up"}));
    assert_eq!(r["prompt"]["text"], "new", "the earlier answer comes back");
    s.result("prompt_key", json!({"key": "escape"}));

    // Settings: the schema, a change, and a refusal.
    let schema = s.result("settings_schema", json!({}));
    let rate = schema
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["path"] == "speech.rate")
        .unwrap()
        .clone();
    assert_eq!(rate["kind"], "number");
    assert_eq!(rate["max"], 900.0);
    let r = s.result("set_setting", json!({"path": "speech.rate", "value": 320}));
    assert_eq!(r["value"], 320);
    assert_eq!(r["spoken"], "320 words per minute");
    assert_eq!(s.result("status", json!({}))["rate"], 320);
    let saved = textweaver_app::store::SettingsStore::new(Paths::under(&home))
        .load()
        .0;
    assert_eq!(saved.speech.rate.wpm(), 320, "saved through the writer");
    assert_eq!(
        s.error_code("set_setting", json!({"path": "speech.nothing", "value": 1})),
        codes::INVALID_PARAMS
    );
    assert_eq!(
        s.error_code(
            "set_setting",
            json!({"path": "speech.rate", "value": "fast"})
        ),
        codes::INVALID_PARAMS
    );
    let r = s.result("get_setting", json!({"path": "highlight.granularity"}));
    assert_eq!(r["spoken"], "the word");
}
