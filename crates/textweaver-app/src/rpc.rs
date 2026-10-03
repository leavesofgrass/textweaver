//! JSON-RPC 2.0 over a byte stream: how editors and other frontends drive
//! textweaver (`tw serve --stdio`, ADR-0015).
//!
//! Messages are JSON-RPC 2.0 objects, one per line (newline-delimited
//! JSON). A client may instead frame messages with LSP-style
//! `Content-Length` headers; the server answers in the framing of the last
//! message it received. A line or a message body over 16 MiB ends the
//! session, as if the client had closed. Requests are answered in order. While speech is
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
//! | `open` | `{path}`: a file, an archive member (`book.zip!chapter.pdf`), or a web address, as `tw open` takes | the document (`title`, `path`, `format`, `length`, `lines`) and `position` |
//! | `status` | none | mode, playback, document, position, rate, backend, status line, pending question |
//! | `position` | none | `{char, line, column, percent, word}` |
//! | `navigate` | `{action}` (a navigation action id, e.g. `next_sentence`) or `{goto}` (`"12"`, `"50%"`, `"end"`) | `position` |
//! | `read` | `{what}`: `cursor` (default), `document`, `sentence`, `paragraph`, `line`, `word`, `selection`; optional `from` (a char offset) | `{playback}` |
//! | `pause`, `resume`, `stop` | none | `{playback}` |
//! | `search` | `{pattern, regex?}` | `{matches: [{start, end, line}], current}`; moves to the first match at or after the cursor |
//! | `text` | `{start?, end?}` (char offsets) | `{text, start, end}` |
//! | `outline` | none | `{items: [{level, text, char, line}], pages}`; `pages` is true when the items are a paged document's pages (level 0) |
//! | `notes` | none | `{notes: [{id, start, end, line, anchor, text, tags, color}]}` |
//! | `highlights` | none | `{highlights: [{id, start, end, line, color, text}]}` |
//! | `info` | none | title, path, format, author, language, chars, words, lines, headings, notes, highlights, bookmarks, `reading_minutes` at `rate` |
//! | `insert` | `{text}`: typed at the caret in edit mode, as typing is (one undo step, echoed) | `{status, effects, position}`; refused outside edit mode (`NOT_EDITING`) |
//! | `action` | `{id}`: any keymap action id or notes command name; optional `confirm` (`true` answers yes, `false` no) for an action that asks first (quit, delete note) | `{status, effects, pending}`; `pending` is `{action, question}` while a question waits, else null |
//! | `answer` | `{text}`: answers the open prompt | `{status, effects}` |
//! | `choose` | `{index}`: picks from the shown list | `{status, effects}` |
//! | `cancel` | none | `{status, effects}`; also answers no to a pending question |
//! | `list_state` | none | the list shown: `{title, items, selected, filter}`, or null |
//! | `list_key` | `{key}`: `up`, `down`, `page_up`, `page_down`, `home`, `end`, `left`, `right`, `enter`, `escape`, `backspace`, `delete`, `rename`, `introduce`, `details`, the file browser's `choose_here`, `sort`, and `show_all`, or one character | `{status, effects, list}` |
//! | `prompt_state` | none | the prompt open: `{label, purpose, text, caret}`, or null |
//! | `prompt_key` | `{key}` as for `list_key` plus `tab`, `kill_to_start`, `kill_to_end`, `delete_word_back`; or `{text}` to set the whole text | `{status, effects, prompt}` |
//! | `settings_schema` | none | every setting: `path`, `section`, `label`, `help`, `kind`, `default`, and its range or choices |
//! | `get_setting` | `{path}` | `{path, value, spoken}` |
//! | `set_setting` | `{path, value}` (`null` for the default) | `{path, value, spoken}`; the change is in effect and saved |
//! | `shutdown` | none | `null`; saves the position and settings |
//! | `exit` | none (a notification) | the server stops |
//!
//! Notifications from the server: `position` `{start, end, line}`,
//! `playback` `{state}`, `announcement` `{text, priority}`, `prompt`
//! `{label, purpose}`, `list` `{title, items}`, and `quit`.

use std::collections::VecDeque;
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use textweaver_a11y::{Announcer, Priority};
use textweaver_core::{CharPos, CharRange};
use textweaver_keymap::ActionId;

use crate::app::App;
use crate::command::{Command, Confirm, Effect, NoteCommand, PromptPurpose};
use crate::playback::Playback;
use crate::text_util;

/// Protocol version reported by `initialize`.
pub const PROTOCOL_VERSION: u32 = 1;

/// Every request method.
pub const METHODS: [&str; 29] = [
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
    "outline",
    "notes",
    "highlights",
    "info",
    "insert",
    "action",
    "answer",
    "choose",
    "cancel",
    "list_state",
    "list_key",
    "prompt_state",
    "prompt_key",
    "settings_schema",
    "get_setting",
    "set_setting",
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
    /// The method needs edit mode (`insert`).
    pub const NOT_EDITING: i64 = -32004;
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

/// The protocol's name for a prompt purpose. Pinned by hand rather than
/// derived from the Rust name, so renaming a variant cannot change the
/// protocol (ADR-0015); a new variant fails to compile until it is named
/// here, and the test below fails until it is listed there too.
fn purpose_name(p: PromptPurpose) -> &'static str {
    match p {
        PromptPurpose::Find => "find",
        PromptPurpose::GoTo => "go_to",
        PromptPurpose::Open => "open",
        PromptPurpose::CommandPalette => "command_palette",
        PromptPurpose::SaveAs => "save_as",
        PromptPurpose::TableSize => "table_size",
        PromptPurpose::ImagePath => "image_path",
        PromptPurpose::ReplaceFind => "replace_find",
        PromptPurpose::ReplaceWith => "replace_with",
        PromptPurpose::NoteText => "note_text",
        PromptPurpose::EditNote => "edit_note",
        PromptPurpose::RenameBookmark => "rename_bookmark",
        PromptPurpose::ExportSettings => "export_settings",
        PromptPurpose::ImportSettings => "import_settings",
        PromptPurpose::CitationLocator => "citation_locator",
        PromptPurpose::ReferenceIdentifier => "reference_identifier",
        PromptPurpose::ImportReferences => "import_references",
        PromptPurpose::TemplateTitle => "template_title",
        PromptPurpose::DefineWord => "define_word",
        PromptPurpose::ProfileName => "profile_name",
        PromptPurpose::RenameProfile => "rename_profile",
        PromptPurpose::ImportProfiles => "import_profiles",
        PromptPurpose::ExportProfiles => "export_profiles",
        PromptPurpose::SettingValue => "setting_value",
        PromptPurpose::SyncComputerName => "sync_computer_name",
        PromptPurpose::DocumentDetails => "document_details",
    }
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
        let effects = self.app.tick(Instant::now());
        out.extend(self.effects(effects));
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
            "outline" => self.outline(),
            "notes" => self.notes(),
            "highlights" => self.highlights(),
            "info" => self.info(),
            "insert" => self.insert(params),
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
            "list_state" => Ok(self.list_state()),
            "list_key" => {
                let key = list_key(str_param(params, "key")?)?;
                let mut result = self.run(Command::ListKey(key));
                result["list"] = self.list_state();
                Ok(result)
            }
            "prompt_state" => Ok(self.prompt_state()),
            "prompt_key" => {
                let key = match params.get("text").and_then(Value::as_str) {
                    Some(text) => crate::list_model::PromptKey::SetText(text.to_owned()),
                    None => prompt_key(str_param(params, "key")?)?,
                };
                let mut result = self.run(Command::PromptKey(key));
                result["prompt"] = self.prompt_state();
                Ok(result)
            }
            "settings_schema" => Ok(self.app.settings_schema().to_json()),
            "get_setting" => {
                let path = str_param(params, "path")?;
                self.setting_result(path)
            }
            "set_setting" => {
                let path = str_param(params, "path")?.to_owned();
                let value = params.get("value").cloned().unwrap_or(Value::Null);
                self.app
                    .set_setting(&path, value)
                    .map_err(RpcError::params)?;
                let _ = self.app.save_settings();
                self.app.wait_for_writes();
                self.setting_result(&path)
            }
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
    /// A request's file work (a save, a bookmark) is finished before its
    /// response, so a client can rely on the disk when it gets the answer.
    fn dispatch(&mut self, cmd: Command) -> Vec<Value> {
        let mut effects = self.app.dispatch(cmd);
        effects.extend(self.app.wait_for_writes());
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
    /// `insert`: text at the caret in edit mode (dictation and other
    /// clients type through it), refused outside edit mode.
    fn insert(&mut self, params: &Value) -> RpcResult {
        self.need_document()?;
        let text = str_param(params, "text")?.to_owned();
        if !self.app.is_editing() {
            return Err(RpcError::new(
                codes::NOT_EDITING,
                "insert needs edit mode; run the toggle_edit_mode action first",
            ));
        }
        let mut result = self.run(Command::Insert(text));
        result["position"] = self.position();
        Ok(result)
    }

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
            "pending": self.pending(),
        })
    }

    /// The 1-based line of `pos` in the open document.
    fn line_of(&self, pos: CharPos) -> usize {
        self.app
            .session()
            .map_or(0, |s| text_util::line_of(&s.doc, pos) + 1)
    }

    /// `outline`: the headings, or a paged document's pages.
    fn outline(&self) -> RpcResult {
        self.need_document()?;
        let (items, pages) = self.app.outline_items();
        let items: Vec<Value> = items
            .iter()
            .map(|h| {
                json!({
                    "level": h.level,
                    "text": h.text,
                    "char": h.pos.0,
                    "line": self.line_of(h.pos),
                })
            })
            .collect();
        Ok(json!({ "items": items, "pages": pages }))
    }

    /// `notes`: the open document's notes, in document order.
    fn notes(&self) -> RpcResult {
        self.need_document()?;
        let Some(s) = self.app.session() else {
            return Ok(json!({ "notes": [] }));
        };
        let notes: Vec<Value> = s
            .notes
            .iter()
            .map(|n| {
                json!({
                    "id": n.id,
                    "start": n.range.start.0,
                    "end": n.range.end.0,
                    "line": self.line_of(n.range.start),
                    "anchor": n.anchor,
                    "text": n.note,
                    "tags": n.tags,
                    "color": n.color,
                })
            })
            .collect();
        Ok(json!({ "notes": notes }))
    }

    /// `highlights`: the open document's highlights, in document order.
    fn highlights(&self) -> RpcResult {
        self.need_document()?;
        let Some(s) = self.app.session() else {
            return Ok(json!({ "highlights": [] }));
        };
        let highlights: Vec<Value> = s
            .highlights
            .iter()
            .map(|h| {
                json!({
                    "id": h.id,
                    "start": h.range.start.0,
                    "end": h.range.end.0,
                    "line": self.line_of(h.range.start),
                    "color": h.color,
                    "text": h.text,
                })
            })
            .collect();
        Ok(json!({ "highlights": highlights }))
    }

    /// `info`: facts about the open document, as `tw info` gives them for
    /// a file.
    fn info(&self) -> RpcResult {
        self.need_document()?;
        let Some(s) = self.app.session() else {
            return Ok(Value::Null);
        };
        let doc = &s.doc;
        let words = textweaver_text::units::segments(doc, textweaver_core::Unit::Word).len();
        let wpm = self.app.settings().speech.rate.wpm().max(1);
        let index = doc.marker_index();
        Ok(json!({
            "title": s.title,
            "path": doc.meta.path.as_ref().map(|p| p.display().to_string()),
            "format": doc.meta.format,
            "author": doc.meta.author,
            "language": doc.meta.language,
            "chars": doc.len_chars(),
            "words": words,
            "lines": text_util::line_count(doc),
            "headings": index.count(textweaver_core::MarkerKind::Heading, None),
            "notes": s.notes.len(),
            "highlights": s.highlights.len(),
            "bookmarks": s.bookmarks.len(),
            "reading_minutes": words.div_ceil(wpm as usize),
            "rate": wpm,
        }))
    }

    fn open(&mut self, params: &Value) -> RpcResult {
        let path = PathBuf::from(str_param(params, "path")?);
        // What `tw open` accepts: a file, a member of an archive
        // (`book.zip!chapter.pdf`), or a web address. Only plain paths are
        // checked on disk.
        if crate::formats::Source::Path(path.clone()).url().is_none() {
            if !crate::formats::archive::exists(&path) {
                return Err(RpcError::new(
                    codes::OPEN_FAILED,
                    format!("{}: no such file", path.display()),
                ));
            }
            if path.is_dir() {
                return Err(RpcError::new(
                    codes::OPEN_FAILED,
                    format!("{} is a folder, not a document", path.display()),
                ));
            }
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
        let confirm = match params.get("confirm") {
            None | Some(Value::Null) => None,
            Some(Value::Bool(b)) => Some(*b),
            Some(_) => return Err(RpcError::params("confirm must be true or false")),
        };
        let mut result = self.run(Command::Action(action));
        if self.app.pending_confirmation() == Some(action)
            && let Some(yes) = confirm
        {
            // Answer the question the action just asked, as `y` or `n` would.
            let answer = if yes { Confirm::Yes } else { Confirm::No };
            let first = result["effects"].as_array().cloned().unwrap_or_default();
            result = self.run(Command::Confirm(answer));
            if let Some(effects) = result["effects"].as_array_mut() {
                let mut all = first;
                all.append(effects);
                *effects = all;
            }
        }
        result["pending"] = self.pending();
        Ok(result)
    }

    /// The list shown, or null.
    fn list_state(&self) -> Value {
        match self.app.list_model() {
            Some(l) => json!({
                "title": l.title,
                "items": l.items,
                "selected": l.selected,
                "filter": self.app.list_filter(),
            }),
            None => Value::Null,
        }
    }

    /// The prompt open, or null.
    fn prompt_state(&self) -> Value {
        match self.app.prompt_model() {
            Some(p) => json!({
                "label": p.label,
                "purpose": purpose_name(p.purpose),
                "text": p.text(),
                "caret": p.caret(),
            }),
            None => Value::Null,
        }
    }

    /// A setting's value, and how it reads aloud.
    fn setting_result(&self, path: &str) -> RpcResult {
        let schema = self.app.settings_schema();
        let setting = schema
            .get(path)
            .ok_or_else(|| RpcError::params(format!("There is no setting {path}.")))?;
        let value = self.app.setting_value(path).unwrap_or(Value::Null);
        Ok(json!({
            "path": path,
            "spoken": setting.describe(&value),
            "value": value,
        }))
    }

    /// The question waiting for a yes or no, as `{action, question}`, or
    /// null.
    fn pending(&self) -> Value {
        match self.app.pending_confirmation() {
            Some(a) => json!({
                "action": a.id(),
                "question": a.confirmation_prompt(),
            }),
            None => Value::Null,
        }
    }
}

/// A list key by name (`down`, `enter`, or one character).
fn list_key(name: &str) -> Result<crate::list_model::ListKey, RpcError> {
    use crate::list_model::ListKey as K;
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Ok(K::Char(c));
    }
    Ok(match name {
        "up" => K::Up,
        "down" => K::Down,
        "page_up" => K::PageUp,
        "page_down" => K::PageDown,
        "home" => K::Home,
        "end" => K::End,
        "left" => K::Left,
        "right" => K::Right,
        "enter" => K::Enter,
        "escape" => K::Escape,
        "backspace" => K::Backspace,
        "delete" => K::Delete,
        "rename" => K::Rename,
        "introduce" => K::Introduce,
        "details" => K::Details,
        "choose_here" => K::ChooseHere,
        "sort" => K::Sort,
        "show_all" => K::ShowAll,
        other => return Err(RpcError::params(format!("No list key {other}"))),
    })
}

/// A prompt key by name (`enter`, `tab`, or one character).
fn prompt_key(name: &str) -> Result<crate::list_model::PromptKey, RpcError> {
    use crate::list_model::PromptKey as K;
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Ok(K::Char(c));
    }
    Ok(match name {
        "up" => K::Up,
        "down" => K::Down,
        "left" => K::Left,
        "right" => K::Right,
        "home" => K::Home,
        "end" => K::End,
        "enter" => K::Enter,
        "escape" => K::Escape,
        "backspace" => K::Backspace,
        "delete" => K::Delete,
        "tab" => K::Tab,
        "shift_tab" | "back_tab" => K::BackTab,
        "kill_to_start" => K::KillToStart,
        "kill_to_end" => K::KillToEnd,
        "delete_word_back" => K::DeleteWordBack,
        other => return Err(RpcError::params(format!("No prompt key {other}"))),
    })
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

/// What wakes the serve loop: a message from the client, or the app's
/// waker.
enum Incoming {
    Message(Framing, String),
    Wake,
    /// The client closed its end (the waker keeps the channel open, so
    /// the reader says so itself).
    Closed,
}

/// The largest message the server reads, in bytes: a line, a header
/// line, or a `Content-Length` body. A client that sends more is broken or
/// hostile, and reading on would take all the memory it asks for, so the
/// server stops reading and shuts down as if the client had closed.
const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

/// Reads one line into `line`, at most [`MAX_MESSAGE_BYTES`] of it.
/// `None` at the end of the input, on a read error, or on a longer line.
fn read_line_capped<R: BufRead>(reader: &mut R, line: &mut String) -> Option<usize> {
    let limit = MAX_MESSAGE_BYTES as u64 + 1;
    match reader.by_ref().take(limit).read_line(line) {
        Ok(0) | Err(_) => None,
        Ok(n) if n > MAX_MESSAGE_BYTES => {
            log::warn!("JSON-RPC: a line longer than {MAX_MESSAGE_BYTES} bytes; closing");
            None
        }
        Ok(n) => Some(n),
    }
}

/// Reads messages from `reader`, one per line or `Content-Length` framed.
fn read_messages<R: BufRead>(mut reader: R, tx: &std::sync::mpsc::Sender<Incoming>) {
    let mut line = String::new();
    loop {
        line.clear();
        if read_line_capped(&mut reader, &mut line).is_none() {
            return;
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
                if len > MAX_MESSAGE_BYTES {
                    log::warn!(
                        "JSON-RPC: Content-Length {len} is over {MAX_MESSAGE_BYTES} bytes; closing"
                    );
                    return;
                }
                // Skip other headers up to the blank line, then the body.
                loop {
                    line.clear();
                    match read_line_capped(&mut reader, &mut line) {
                        None => return,
                        Some(_) if line.trim().is_empty() => break,
                        Some(_) => {}
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
        if tx.send(Incoming::Message(msg.0, msg.1)).is_err() {
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

/// How long the server waits for input at most before it polls again.
/// Since Wave 3 the app's waker ([`crate::wake`]) wakes the loop as soon
/// as a word is heard or background work finishes, so this is only a
/// backstop; [`App::tick_interval`] is shorter when the app's timers need
/// it.
pub const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Serves `server` over `reader` and `writer` until the client sends `exit`
/// or closes its end. The position and settings are saved on the way out.
pub fn serve<R, W>(mut server: Server, reader: R, mut writer: W) -> std::io::Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    let (tx, rx) = channel::<Incoming>();
    let wake_tx = Mutex::new(tx.clone());
    std::thread::spawn(move || {
        read_messages(reader, &tx);
        let _ = tx.send(Incoming::Closed);
    });
    // Speech statuses and finished background work wake the loop at once.
    server.app_mut().set_waker(Some(Arc::new(move || {
        if let Ok(t) = wake_tx.lock() {
            let _ = t.send(Incoming::Wake);
        }
    })));
    let mut framing = Framing::Line;
    loop {
        // Apple's AVSpeechSynthesizer delivers through the main run loop
        // (ADR-0008); a no-op on other platforms.
        crate::apple::pump_main_loop(Duration::ZERO);
        let wait = if cfg!(target_os = "macos") {
            // The main run loop needs pumping while speech is active.
            Duration::from_millis(20)
        } else {
            server
                .app()
                .tick_interval(Instant::now())
                .min(POLL_INTERVAL)
        };
        let out = match rx.recv_timeout(wait) {
            Ok(Incoming::Message(f, msg)) => {
                framing = f;
                server.handle(&msg)
            }
            Ok(Incoming::Wake) | Err(RecvTimeoutError::Timeout) => server.poll(),
            Ok(Incoming::Closed) | Err(RecvTimeoutError::Disconnected) => {
                server.app_mut().set_waker(None);
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

    /// Every prompt purpose and its protocol name. The protocol freezes at
    /// the final alpha: a change here is a protocol change.
    const PURPOSES: [(PromptPurpose, &str); 26] = [
        (PromptPurpose::Find, "find"),
        (PromptPurpose::GoTo, "go_to"),
        (PromptPurpose::Open, "open"),
        (PromptPurpose::CommandPalette, "command_palette"),
        (PromptPurpose::SaveAs, "save_as"),
        (PromptPurpose::TableSize, "table_size"),
        (PromptPurpose::ImagePath, "image_path"),
        (PromptPurpose::ReplaceFind, "replace_find"),
        (PromptPurpose::ReplaceWith, "replace_with"),
        (PromptPurpose::NoteText, "note_text"),
        (PromptPurpose::EditNote, "edit_note"),
        (PromptPurpose::RenameBookmark, "rename_bookmark"),
        (PromptPurpose::ExportSettings, "export_settings"),
        (PromptPurpose::ImportSettings, "import_settings"),
        (PromptPurpose::CitationLocator, "citation_locator"),
        (PromptPurpose::ReferenceIdentifier, "reference_identifier"),
        (PromptPurpose::ImportReferences, "import_references"),
        (PromptPurpose::TemplateTitle, "template_title"),
        (PromptPurpose::DefineWord, "define_word"),
        (PromptPurpose::ProfileName, "profile_name"),
        (PromptPurpose::RenameProfile, "rename_profile"),
        (PromptPurpose::ImportProfiles, "import_profiles"),
        (PromptPurpose::ExportProfiles, "export_profiles"),
        (PromptPurpose::SettingValue, "setting_value"),
        (PromptPurpose::SyncComputerName, "sync_computer_name"),
        (PromptPurpose::DocumentDetails, "document_details"),
    ];

    #[test]
    fn the_26_purpose_names_are_pinned() {
        let mut seen = std::collections::HashSet::new();
        for (p, name) in PURPOSES {
            assert_eq!(purpose_name(p), name, "{p:?}");
            assert!(seen.insert(name), "{name} named twice");
            // A new variant must be added to PURPOSES as well as the match.
            let variant = match p {
                PromptPurpose::Find
                | PromptPurpose::GoTo
                | PromptPurpose::Open
                | PromptPurpose::CommandPalette
                | PromptPurpose::SaveAs
                | PromptPurpose::TableSize
                | PromptPurpose::ImagePath
                | PromptPurpose::ReplaceFind
                | PromptPurpose::ReplaceWith
                | PromptPurpose::NoteText
                | PromptPurpose::EditNote
                | PromptPurpose::RenameBookmark
                | PromptPurpose::ExportSettings
                | PromptPurpose::ImportSettings
                | PromptPurpose::CitationLocator
                | PromptPurpose::ReferenceIdentifier
                | PromptPurpose::ImportReferences
                | PromptPurpose::TemplateTitle
                | PromptPurpose::DefineWord
                | PromptPurpose::ProfileName
                | PromptPurpose::RenameProfile
                | PromptPurpose::ImportProfiles
                | PromptPurpose::ExportProfiles
                | PromptPurpose::SettingValue
                | PromptPurpose::SyncComputerName
                | PromptPurpose::DocumentDetails => name,
            };
            assert!(!variant.is_empty());
        }
    }

    /// The JSON-RPC guide names every purpose, so a client author can
    /// handle each one.
    #[test]
    fn the_guide_lists_every_purpose() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/json-rpc.md");
        let guide = std::fs::read_to_string(path).unwrap();
        for (_, name) in PURPOSES {
            assert!(guide.contains(&format!("`{name}`")), "{name} missing");
        }
        assert!(guide.contains("26 purposes"));
    }

    /// The messages `read_messages` passes on for `input`, with their
    /// framing.
    fn messages(input: &[u8]) -> Vec<(Framing, String)> {
        let (tx, rx) = channel();
        read_messages(std::io::Cursor::new(input.to_vec()), &tx);
        drop(tx);
        rx.into_iter()
            .filter_map(|m| match m {
                Incoming::Message(f, text) => Some((f, text)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_huge_content_length_closes_without_allocating() {
        // Before the cap, this asked for a terabyte and aborted the process.
        let input =
            b"{\"jsonrpc\":\"2.0\",\"method\":\"exit\"}\nContent-Length: 1099511627776\r\n\r\n{}";
        let got = messages(input);
        assert_eq!(got.len(), 1, "only the message before the header: {got:?}");
        assert_eq!(got[0].0, Framing::Line);
        let max = format!("Content-Length: {}\r\n\r\n", usize::MAX);
        assert!(messages(max.as_bytes()).is_empty());
    }

    #[test]
    fn a_line_over_the_cap_closes() {
        let mut input = vec![b' '; MAX_MESSAGE_BYTES + 10];
        input.push(b'\n');
        input.extend_from_slice(b"{}\n");
        assert!(messages(&input).is_empty());
    }

    #[test]
    fn messages_under_the_cap_still_read() {
        let body = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"status\"}";
        let input = format!(
            "Content-Length: {}\r\nX-Other: 1\r\n\r\n{body}{body}\n",
            body.len()
        );
        let got = messages(input.as_bytes());
        assert_eq!(
            got,
            vec![
                (Framing::Header, body.to_owned()),
                (Framing::Line, body.to_owned())
            ]
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
