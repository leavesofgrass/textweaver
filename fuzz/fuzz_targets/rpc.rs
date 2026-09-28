//! Fuzz target: the JSON-RPC server (`textweaver-app`'s `rpc.rs`,
//! ADR-0015). Each line of the input is one message, as `tw serve --stdio`
//! reads them, handed to a server with a small document open in memory,
//! silent speech, and nothing saved. A message that is not JSON, not a
//! request, or asks for an unknown method gets an error, never a panic.
//! Every message the server sends is a JSON-RPC 2.0 object that encodes
//! to text which decodes to the same value, and every request with an id
//! gets exactly one response with that id.
//!
//! Only methods that cannot reach the file system are sent: a message
//! naming `open`, `action`, `answer`, `choose`, `list_key`, `prompt_key`,
//! `set_setting`, `shutdown`, or `exit` is skipped, because those can open,
//! write, or save files named in the input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use textweaver_app::rpc::{self, Server};
use textweaver_app::store::DocKey;
use textweaver_app::{App, AppConfig};
use textweaver_formats::{Registry, Source};

const DOC: &str =
    "# A heading\n\nFirst sentence here. Second sentence, with $x^2$.\n\n- one\n- two\n\nThe end.";

/// Methods that only read the document or change reading state.
const SAFE: [&str; 15] = [
    "initialize",
    "status",
    "position",
    "navigate",
    "read",
    "pause",
    "resume",
    "stop",
    "search",
    "text",
    "cancel",
    "list_state",
    "prompt_state",
    "settings_schema",
    "get_setting",
];

fn allowed(msg: &Value) -> bool {
    let one = |m: &Value| match m.get("method").and_then(Value::as_str) {
        Some(name) => SAFE.contains(&name) || !rpc::METHODS.contains(&name),
        None => true,
    };
    match msg {
        Value::Array(items) => items.iter().all(one),
        other => one(other),
    }
}

fn server() -> Server {
    let (announcer, queue) = rpc::announcer();
    let mut app = App::new(AppConfig {
        announcer,
        ..AppConfig::for_tests()
    });
    let source = Source::Bytes {
        data: DOC.as_bytes().to_vec(),
        hint: "md".into(),
    };
    let doc = Registry::with_builtins()
        .load(&source, &textweaver_fuzz::options())
        .expect("the fuzz document loads");
    app.open_document(doc, DocKey("fuzz".into()), "Fuzz".into());
    Server::new(app, queue)
}

fn check_reply(out: &[Value], request: &Value) {
    for m in out {
        assert_eq!(m.get("jsonrpc").and_then(Value::as_str), Some("2.0"), "{m}");
        let text = m.to_string();
        let again: Value = serde_json::from_str(&text).expect("a reply decodes");
        assert_eq!(&again, m, "a reply encodes and decodes the same");
    }
    // One response per request with an id, outside batches.
    if let Some(id) = request.as_object().and_then(|o| o.get("id")) {
        let n = out
            .iter()
            .filter(|m| m.get("method").is_none() && m.get("id") == Some(id))
            .count();
        assert_eq!(n, 1, "one response for id {id}: {out:?}");
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut server = server();
    for line in text.lines().take(32) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: Option<Value> = serde_json::from_str(line).ok();
        if let Some(msg) = &parsed
            && !allowed(msg)
        {
            continue;
        }
        let out = server.handle(line);
        check_reply(&out, parsed.as_ref().unwrap_or(&Value::Null));
        let _ = server.poll();
    }
});
