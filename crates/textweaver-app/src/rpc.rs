//! JSON-RPC 2.0 over a byte stream: how editors and other frontends drive
//! textweaver (`tw serve --stdio`, ADR-0015).
//!
//! Messages are JSON-RPC 2.0 objects, one per line (newline-delimited
//! JSON). A client may instead frame messages with LSP-style
//! `Content-Length` headers; the server answers in the framing of the last
//! message it received. Requests are answered in order. While speech is
//! active the server sends notifications on its own: `position` as the
//! spoken word moves, `playback` when reading starts, pauses, or stops,
//! and `announcement` for every message the app announces (what a screen
//! reader user would hear), so a client can voice or display them.
//!
//! [`Server`] is the transport-free core (tests drive it directly);
//! [`serve`] runs it over a reader and a writer.
//!
//! # Methods
//!
//! | Method | Params | Result |
//! |---|---|---|
//! | `initialize` | none | server name, version, protocol, methods, notifications |
//! | `open` | `{path}` | the document (`title`, `path`, `format`, `length`, `lines`) and `position` |
//! | `status` | none | mode, playback, document, position, rate, backend, status line |
//! | `position` | none | `{char, line, column, percent, word}` |
//! | `navigate` | `{action}` (a navigation action id, e.g. `next_sentence`) or `{goto}` (`"12"`, `"50%"`, `"end"`) | `position` |
//! | `read` | `{what}`: `cursor` (default), `document`, `sentence`, `paragraph`, `line`, `word`, `selection`; optional `from` (a char offset) | `{playback}` |
//! | `pause`, `resume`, `stop` | none | `{playback}` |
//! | `search` | `{pattern, regex?}` | `{matches: [{start, end, line}], current}`; moves to the first match at or after the cursor |
//! | `text` | `{start?, end?}` (char offsets) | `{text, start, end}` |
//! | `action` | `{id}`: any keymap action id or notes command name | `{status, effects}` |
//! | `answer` | `{text}`: answers the open prompt | `{status, effects}` |
//! | `choose` | `{index}`: picks from the shown list | `{status, effects}` |
//! | `cancel` | none | `{status, effects}` |
//! | `shutdown` | none | `null`; saves the position and settings |
//! | `exit` | none (a notification) | the server stops |
//!
//! Notifications from the server: `position` `{start, end, line}`,
//! `playback` `{state}`, `announcement` `{text, priority}`, `prompt`
//! `{label, purpose}`, `list` `{title, items}`, and `quit`.

use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use textweaver_a11y::{Announcer, Priority};
use textweaver_core::{CharPos, CharRange};
use textweaver_keymap::ActionId;

use crate::app::App;
use crate::command::{Command, Effect, NoteCommand, PromptPurpose};
use crate::playback::Playback;
use crate::text_util;

/// Protocol version reported by `initialize`.
pub const PROTOCOL_VERSION: u32 = 1;

/// Every request method.
pub const METHODS: [&str; 17] = [
    "initialize",
    "open",
    "status",
    "position",
    "navigate",
    "read",
    "pause",
    "resume",
    "stop",
    "search",
    "text",
    "action",
    "answer",
    "choose",
    "cancel",
    "shutdown",
    "exit",
];

/// Every notification the server sends.
pub const NOTIFICATIONS: [&str; 6] = [
    "position",
    "playback",
    "announcement",
    "prompt",
    "list",
    "quit",
];

/// Error codes (JSON-RPC's own, and the server's in -32000..-32099).
pub mod codes {
    /// The message is not JSON.
    pub const PARSE_ERROR: i64 = -32700;
    /// The JSON is not a request.
    pub const INVALID_REQUEST: i64 = -32600;
    /// No such method.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// Missing or wrong parameters.
    pub const INVALID_PARAMS: i64 = -32602;
    /// The method needs an open document.
    pub const NO_DOCUMENT: i64 = -32001;
    /// The document could not be opened.
    pub const OPEN_FAILED: i64 = -32002;
    /// A request arrived after `shutdown`.
    pub const SHUT_DOWN: i64 = -32003;
}

/// Announcements the app made, waiting to be sent as notifications.
#[derive(Clone, Debug, Default)]
pub struct AnnouncementQueue(Arc<Mutex<VecDeque<(String, Priority)>>>);

impl AnnouncementQueue {
    fn take(&self) -> Vec<(String, Priority)> {
        self.0
            .lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default()
    }
}

/// An announcer that queues everything for the server.
#[derive(Debug)]
struct QueueAnnouncer(AnnouncementQueue);

impl Announcer for QueueAnnouncer {
    fn announce(&mut self, text: &str, priority: Priority) {
        if let Ok(mut q) = (self.0).0.lock() {
            q.push_back((text.to_owned(), priority));
        }
    }
}

/// An announcer for the app a [`Server`] wraps, and the queue it fills.
/// Pass the announcer in `AppConfig::announcer` and the queue to
/// [`Server::new`].
pub fn announcer() -> (Box<dyn Announcer>, AnnouncementQueue) {
    let q = AnnouncementQueue::default();
    (Box::new(QueueAnnouncer(q.clone())), q)
}

/// A request that failed.
struct RpcError {
    code: i64,
    message: String,
}

impl RpcError {
    fn new(code: i64, message: impl Into<String>) -> Self {
        RpcError {
            code,
            message: message.into(),
        }
    }

    fn params(message: impl Into<String>) -> Self {
        RpcError::new(codes::INVALID_PARAMS, message)
    }
}

type RpcResult = Result<Value, RpcError>;

fn playback_name(p: Playback) -> &'static str {
    match p {
        Playback::Idle => "stopped",
        Playback::Reading => "reading",
        Playback::Paused { .. } => "paused",
    }
}

fn priority_name(p: Priority) -> &'static str {
    match p {
        Priority::Assertive => "assertive",
        _ => "polite",
    }
}

fn purpose_name(p: PromptPurpose) -> String {
    format!("{p:?}")
        .chars()
        .enumerate()
        .flat_map(|(i, c)| {
            let lower = c.to_ascii_lowercase();
            if c.is_ascii_uppercase() && i > 0 {
                vec!['_', lower]
            } else {
                vec![lower]
            }
        })
        .collect()
}

fn notification(method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "method": method, "params": params })
}

/// The server: an [`App`] plus what the client has been told.
pub struct Server {
    app: App,
    queue: AnnouncementQueue,
    sent_playback: Playback,
    sent_spoken: Option<CharRange>,
    shut_down: bool,
    exited: bool,
}

impl Server {
    /// Wraps `app`, whose announcer feeds `queue` (see [`announcer`]).
    pub fn new(app: App, queue: AnnouncementQueue) -> Self {
        Server {
            app,
            queue,
            sent_playback: Playback::Idle,
            sent_spoken: None,
            shut_down: false,
            exited: false,
        }
    }

    /// The app.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// The app, mutably.
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// True once the client sent `exit` (or the app quit).
    pub fn is_done(&self) -> bool {
        self.exited
    }

    /// Handles one incoming message (a JSON text). Returns the messages to
    /// send: notifications raised while handling it, then the response (none
    /// for a notification).
    pub fn handle(&mut self, message: &str) -> Vec<Value> {
        let parsed: Value = match serde_json::from_str(message) {
            Ok(v) => v,
            Err(e) => {
                return vec![error_response(
                    Value::Null,
                    RpcError::new(codes::PARSE_ERROR, format!("Parse error: {e}")),
                )];
            }
        };
        if let Value::Array(batch) = parsed {
            if batch.is_empty() {
                return vec![error_response(
                    Value::Null,
                    RpcError::new(codes::INVALID_REQUEST, "Empty batch"),
                )];
            }
            let mut out = Vec::new();
            for item in batch {
                out.extend(self.handle_value(item));
            }
            return out;
        }
        self.handle_value(parsed)
    }

    fn handle_value(&mut self, msg: Value) -> Vec<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).map(str::to_owned);
        let valid = msg.get("jsonrpc").and_then(Value::as_str) == Some("2.0");
        let Some(method) = method.filter(|_| valid) else {
            return vec![error_response(
                id.unwrap_or(Value::Null),
                RpcError::new(codes::INVALID_REQUEST, "Not a JSON-RPC 2.0 request"),
            )];
        };
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let result = self.call(&method, &params);
        let mut out = self.poll();
        if let Some(id) = id {
            out.push(match result {
                Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
                Err(e) => error_response(id, e),
            });
        }
        out
    }

    /// Applies speech status and housekeeping, and returns the notifications
    /// due: announcements, playback changes, and the spoken position.
    pub fn poll(&mut self) -> Vec<Value> {
        let mut out = Vec::new();
        self.changes(&mut out);
        // One status at a time, so every word spoken is reported even when
        // several arrived since the last poll.
        while self.app.poll_speech_step().is_some() {
            self.changes(&mut out);
        }
        self.app.tick(Instant::now());
        self.changes(&mut out);
        out
    }

    /// Notifications for what changed since the last call.
    fn changes(&mut self, out: &mut Vec<Value>) {
        for (text, p) in self.queue.take() {
            out.push(notification(
                "announcement",
                json!({ "text": text, "priority": priority_name(p) }),
            ));
        }
        let playback = self.app.playback();
        if playback_name(playback) != playback_name(self.sent_playback) {
            self.sent_playback = playback;
            out.push(notification(
                "playback",
                json!({ "state": playback_name(playback) }),
            ));
        }
        let spoken = self.app.session().and_then(|s| s.spoken);
        if spoken != self.sent_spoken {
            self.sent_spoken = spoken;
            if let (Some(r), Some(s)) = (spoken, self.app.session()) {
                out.push(notification(
                    "position",
                    json!({
                        "start": r.start.0,
                        "end": r.end.0,
                        "line": text_util::line_of(&s.doc, r.start) + 1,
                    }),
                ));
            }
        }
    }

    fn call(&mut self, method: &str, params: &Value) -> RpcResult {
        if self.shut_down && method != "exit" {
            return Err(RpcError::new(
                codes::SHUT_DOWN,
                "The server is shutting down; send exit",
            ));
        }
        match method {
            "initialize" => Ok(json!({
                "server": "textweaver",
                "version": env!("CARGO_PKG_VERSION"),
                "protocol": PROTOCOL_VERSION,
                "methods": METHODS,
                "notifications": NOTIFICATIONS,
            })),
            "open" => self.open(params),
            "status" => Ok(self.status()),
            "position" => {
                self.need_document()?;
                Ok(self.position())
            }
            "navigate" => self.navigate(params),
            "read" => self.read(params),
            "pause" => {
                self.need_document()?;
                if self.app.playback() == Playback::Reading {
                    self.dispatch(Command::Action(ActionId::PlayPause));
                }
                Ok(self.playback_result())
            }
            "resume" => {
                self.need_document()?;
                if matches!(self.app.playback(), Playback::Paused { .. }) {
                    self.dispatch(Command::Action(ActionId::PlayPause));
                }
                Ok(self.playback_result())
            }
            "stop" => {
                self.dispatch(Command::Action(ActionId::Stop));
                Ok(self.playback_result())
            }
            "search" => self.search(params),
            "text" => self.text(params),
            "action" => self.action(params),
            "answer" => {
                let text = str_param(params, "text")?.to_owned();
                Ok(self.run(Command::Answer(text)))
            }
            "choose" => {
                let index = params
                    .get("index")
                    .and_then(Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| RpcError::params("index must be a number"))?;
                Ok(self.run(Command::Choose(index)))
            }
            "cancel" => Ok(self.run(Command::Cancel)),
            "shutdown" => {
                self.app.shutdown();
                self.shut_down = true;
                Ok(Value::Null)
            }
            "exit" => {
                if !self.shut_down {
                    self.app.shutdown();
                }
                self.exited = true;
                Ok(Value::Null)
            }
            other => Err(RpcError::new(
                codes::METHOD_NOT_FOUND,
                format!("No method {other}"),
            )),
        }
    }

    fn need_document(&self) -> Result<(), RpcError> {
        if self.app.session().is_some() {
            Ok(())
        } else {
            Err(RpcError::new(
                codes::NO_DOCUMENT,
                "No document is open; call open first",
            ))
        }
    }

    /// Dispatches and turns effects into notifications queued for `poll`.
    fn dispatch(&mut self, cmd: Command) -> Vec<Value> {
        let effects = self.app.dispatch(cmd);
        self.effects(effects)
    }

    fn effects(&mut self, effects: Vec<Effect>) -> Vec<Value> {
        let mut out = Vec::new();
        for e in effects {
            match e {
                Effect::Redraw => {}
                Effect::Quit => {
                    self.exited = true;
                    out.push(notification("quit", json!({})));
                }
                Effect::Prompt { label, purpose } => out.push(notification(
                    "prompt",
                    json!({ "label": label, "purpose": purpose_name(purpose) }),
                )),
                Effect::ShowList { title, items } => out.push(notification(
                    "list",
                    json!({ "title": title, "items": items }),
                )),
            }
        }
        out
    }

    /// Runs a command; the result carries the status line and the effects
    /// (as the notifications they became).
    fn run(&mut self, cmd: Command) -> Value {
        let effects = self.dispatch(cmd);
        json!({
            "status": self.app.status_text(),
            "effects": effects.iter().map(|n| json!({
                "type": n["method"].clone(),
                "params": n["params"].clone(),
            })).collect::<Vec<_>>(),
        })
    }

    fn document(&self) -> Value {
        match self.app.session() {
            Some(s) => json!({
                "title": s.title,
                "path": s.doc.meta.path.as_ref().map(|p| p.display().to_string()),
                "format": s.doc.meta.format,
                "length": s.doc.len_chars(),
                "lines": text_util::line_count(&s.doc),
                "editing": self.app.is_editing(),
                "dirty": self.app.is_dirty(),
            }),
            None => Value::Null,
        }
    }

    fn position(&self) -> Value {
        let (Some(s), Some(pos)) = (self.app.session(), self.app.reading_position()) else {
            return Value::Null;
        };
        let line = text_util::line_of(&s.doc, pos);
        let col = pos.0 - text_util::line_range(&s.doc, line).start.0;
        let word = text_util::word_containing(&s.doc, pos)
            .map(|w| json!({ "start": w.start.0, "end": w.end.0, "text": s.doc.slice(w) }));
        json!({
            "char": pos.0,
            "line": line + 1,
            "column": col + 1,
            "percent": text_util::percent(&s.doc, pos),
            "word": word,
        })
    }

    fn playback_result(&self) -> Value {
        json!({ "playback": playback_name(self.app.playback()) })
    }

    fn status(&self) -> Value {
        let settings = self.app.settings();
        json!({
            "mode": self.app.mode().name(),
            "playback": playback_name(self.app.playback()),
            "document": self.document(),
            "position": self.position(),
            "rate": settings.speech.rate.wpm(),
            "backend": self.app.backend_name(),
            "status": self.app.status_text(),
        })
    }

    fn open(&mut self, params: &Value) -> RpcResult {
        let path = PathBuf::from(str_param(params, "path")?);
        if !path.is_file() {
            return Err(RpcError::new(
                codes::OPEN_FAILED,
                format!("{}: no such file", path.display()),
            ));
        }
        if self.app.is_editing() {
            // Ask the client through the usual list, like any frontend.
            let notes = self.dispatch(Command::Open(path));
            return Ok(json!({ "document": self.document(), "effects": notes }));
        }
        match self.app.open(&path) {
            Ok(effects) => {
                self.effects(effects);
                Ok(json!({ "document": self.document(), "position": self.position() }))
            }
            Err(e) => Err(RpcError::new(
                codes::OPEN_FAILED,
                format!("Could not open {}: {e}", path.display()),
            )),
        }
    }

    fn navigate(&mut self, params: &Value) -> RpcResult {
        self.need_document()?;
        if let Some(target) = params.get("goto").and_then(Value::as_str) {
            let t = crate::goto::parse_go_to(target)
                .ok_or_else(|| RpcError::params(format!("Not a go-to target: {target}")))?;
            self.dispatch(Command::GoTo(t));
            return Ok(self.position());
        }
        let id = str_param(params, "action")?;
        let action = ActionId::from_id(id)
            .filter(|a| {
                matches!(
                    a.category(),
                    textweaver_keymap::Category::Navigation
                        | textweaver_keymap::Category::SpeechCursor
                        | textweaver_keymap::Category::Bookmarks
                        | textweaver_keymap::Category::Search
                )
            })
            .ok_or_else(|| RpcError::params(format!("Not a navigation action: {id}")))?;
        self.dispatch(Command::Action(action));
        Ok(self.position())
    }

    fn read(&mut self, params: &Value) -> RpcResult {
        self.need_document()?;
        if let Some(from) = params.get("from") {
            let at = from
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| RpcError::params("from must be a char offset"))?;
            self.dispatch(Command::GoTo(textweaver_text::GoTo::Char(CharPos(at))));
        }
        let what = params
            .get("what")
            .and_then(Value::as_str)
            .unwrap_or("cursor");
        let action = match what {
            "cursor" => ActionId::ReadFromCursor,
            "document" => ActionId::ReadDocument,
            "sentence" => ActionId::ReadCurrentSentence,
            "paragraph" => ActionId::ReplayParagraph,
            "line" => ActionId::ReadCurrentLine,
            "word" => ActionId::ReadCurrentWord,
            "character" => ActionId::ReadCurrentCharacter,
            "selection" => ActionId::ReadSelection,
            other => return Err(RpcError::params(format!("Cannot read {other}"))),
        };
        self.dispatch(Command::Action(action));
        Ok(self.playback_result())
    }

    fn search(&mut self, params: &Value) -> RpcResult {
        self.need_document()?;
        let pattern = str_param(params, "pattern")?;
        let regex = params
            .get("regex")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let query = if regex {
            format!("/{pattern}/")
        } else {
            pattern.to_owned()
        };
        self.dispatch(Command::Find(query));
        let Some(s) = self.app.session() else {
            return Ok(Value::Null);
        };
        let (matches, current) = match &s.find {
            Some(f) => (
                f.hits
                    .iter()
                    .map(|h| {
                        json!({
                            "start": h.start.0,
                            "end": h.end.0,
                            "line": text_util::line_of(&s.doc, h.start) + 1,
                        })
                    })
                    .collect::<Vec<_>>(),
                f.current,
            ),
            None => (Vec::new(), None),
        };
        Ok(json!({ "matches": matches, "current": current }))
    }

    fn text(&self, params: &Value) -> RpcResult {
        self.need_document()?;
        let Some(s) = self.app.session() else {
            return Ok(Value::Null);
        };
        let len = s.doc.len_chars();
        let num = |k: &str, default: usize| -> Result<usize, RpcError> {
            match params.get(k) {
                None | Some(Value::Null) => Ok(default),
                Some(v) => v
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .map(|n| n.min(len))
                    .ok_or_else(|| RpcError::params(format!("{k} must be a char offset"))),
            }
        };
        let start = num("start", 0)?;
        let end = num("end", len)?.max(start);
        let r = CharRange::new(start, end);
        Ok(json!({ "text": s.doc.slice(r), "start": start, "end": end }))
    }

    fn action(&mut self, params: &Value) -> RpcResult {
        let id = str_param(params, "id")?;
        if let Some(c) = NoteCommand::from_name(id) {
            return Ok(self.run(Command::Notes(c)));
        }
        let action =
            ActionId::from_id(id).ok_or_else(|| RpcError::params(format!("No action {id}")))?;
        Ok(self.run(Command::Action(action)))
    }
}

fn str_param<'a>(params: &'a Value, key: &str) -> Result<&'a str, RpcError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::params(format!("{key} is required and must be a string")))
}

fn error_response(id: Value, e: RpcError) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": e.code, "message": e.message },
    })
}

/// How a message was framed on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Framing {
    Line,
    Header,
}

/// Reads messages from `reader`, one per line or `Content-Length` framed.
fn read_messages<R: BufRead>(mut reader: R, tx: &std::sync::mpsc::Sender<(Framing, String)>) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let header = trimmed
            .split_once(':')
            .filter(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
            .and_then(|(_, v)| v.trim().parse::<usize>().ok());
        let msg = match header {
            Some(len) => {
                // Skip other headers up to the blank line, then the body.
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => return,
                        Ok(_) if line.trim().is_empty() => break,
                        Ok(_) => {}
                    }
                }
                let mut body = vec![0u8; len];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                (Framing::Header, String::from_utf8_lossy(&body).into_owned())
            }
            None => (Framing::Line, trimmed.to_owned()),
        };
        if tx.send(msg).is_err() {
            return;
        }
    }
}

fn write_message<W: Write>(w: &mut W, framing: Framing, msg: &Value) -> std::io::Result<()> {
    let body = msg.to_string();
    match framing {
        Framing::Line => writeln!(w, "{body}")?,
        Framing::Header => write!(w, "Content-Length: {}\r\n\r\n{body}", body.len())?,
    }
    w.flush()
}

/// How often the server polls speech while waiting for input.
pub const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Serves `server` over `reader` and `writer` until the client sends `exit`
/// or closes its end. The position and settings are saved on the way out.
pub fn serve<R, W>(mut server: Server, reader: R, mut writer: W) -> std::io::Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    let (tx, rx) = channel();
    std::thread::spawn(move || read_messages(reader, &tx));
    let mut framing = Framing::Line;
    loop {
        let out = match rx.recv_timeout(POLL_INTERVAL) {
            Ok((f, msg)) => {
                framing = f;
                server.handle(&msg)
            }
            Err(RecvTimeoutError::Timeout) => server.poll(),
            Err(RecvTimeoutError::Disconnected) => {
                server.app_mut().shutdown();
                return Ok(());
            }
        };
        for m in &out {
            write_message(&mut writer, framing, m)?;
        }
        if server.is_done() {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purpose_names_are_snake_case() {
        assert_eq!(purpose_name(PromptPurpose::SaveAs), "save_as");
        assert_eq!(purpose_name(PromptPurpose::Find), "find");
        assert_eq!(
            purpose_name(PromptPurpose::CommandPalette),
            "command_palette"
        );
    }

    #[test]
    fn unit_names_cover_read_targets() {
        // `read` maps onto real actions.
        for id in ["read_from_cursor", "read_document", "read_current_word"] {
            assert!(ActionId::from_id(id).is_some());
        }
    }
}
