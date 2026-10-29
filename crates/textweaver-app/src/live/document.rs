//! The document view (beta 5, phase 5b): a live source whose messages
//! are live document messages shows the shared document itself, and each
//! new revision replaces it in place.
//!
//! A live document message is one JSON object, retained on its topic:
//! `{"type": "textweaver.document", "rev": N, "title": "...", "format":
//! "markdown", "text": "..."}` (`tw share`, or Publish this document live
//! in the app). Its text keeps its line breaks: the MQTT client cleans it
//! with its own, larger cap (`textweaver_mqtt::clean_document_text`).
//!
//! A new revision goes through the same path as a log's update
//! ([`App::live_apply`]): the open document changes by the smallest
//! edits, so the cursor, the selection, bookmarks and the screen reader's
//! place stay on the same text, and lines that did not change keep their
//! nodes in the textweaver app. The revision is said once, briefly and
//! politely ("Agenda updated, revision 3."); the text itself is never
//! read out again. An older revision than the one shown is left out.
//! While updates are paused, the newest revision waits and is shown on
//! resuming.

use serde_json::Value;
use textweaver_core::{CharPos, Edit};
use textweaver_lexicon::args;
use textweaver_text::Document;

use super::load_markdown;
use crate::app::App;

/// The `type` of a live document message.
pub const DOCUMENT_TYPE: &str = "textweaver.document";

/// A live document message as the app shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedDoc {
    /// Its revision (0 when the message gives none).
    pub rev: u64,
    /// Its title; empty keeps the source's name.
    pub title: String,
    /// Markdown (true) or plain text.
    pub markdown: bool,
    /// The document's text, with its line breaks.
    pub text: String,
}

impl SharedDoc {
    /// The document a message carries, or `None` when it is not a live
    /// document message: an object whose `type` is [`DOCUMENT_TYPE`] and
    /// whose `text` is a string.
    pub fn of(value: &Value) -> Option<SharedDoc> {
        let obj = value.as_object()?;
        if obj.get("type").and_then(Value::as_str) != Some(DOCUMENT_TYPE) {
            return None;
        }
        Some(SharedDoc {
            rev: obj.get("rev").and_then(Value::as_u64).unwrap_or(0),
            title: obj
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned(),
            markdown: obj.get("format").and_then(Value::as_str) == Some("markdown"),
            text: obj.get("text")?.as_str()?.to_owned(),
        })
    }

    /// The document to show: Markdown loaded as any Markdown file is, or
    /// plain text line for line. Either ends with a line break.
    pub fn document(&self) -> Document {
        if self.markdown {
            return load_markdown(&self.text);
        }
        let mut doc = Document::from_plain_text(&self.text);
        let len = doc.len_chars();
        if len == 0 || doc.text().char(len - 1) != '\n' {
            let _ = doc.apply(&Edit::insert(CharPos(len), "\n"));
        }
        doc
    }
}

impl App {
    /// True when the open live document shows a shared document.
    pub fn live_shows_document(&self) -> bool {
        self.live_hub.as_ref().is_some_and(|h| h.document.is_some())
    }

    /// The revision of the shared document shown, if one is.
    pub fn live_document_revision(&self) -> Option<u64> {
        self.live_hub.as_ref().and_then(|h| h.document)
    }

    /// A live document message arrived: shown now, or kept for resuming
    /// while updates are paused (only the newest waits). True when the
    /// document changed.
    pub(crate) fn live_document_arrived(&mut self, doc: SharedDoc) -> bool {
        let Some(h) = self.live_hub.as_mut() else {
            return false;
        };
        if h.paused {
            if h.pending_document.as_ref().is_none_or(|p| doc.rev >= p.rev) {
                h.pending_document = Some(doc);
            }
            return false;
        }
        self.live_document_show(doc, false)
    }

    /// Shows `doc` in place of what is shown, keeping the reader's place,
    /// and says its revision briefly (not when `quiet`, as on resuming,
    /// which says its own words). True when the text changed.
    pub(crate) fn live_document_show(&mut self, doc: SharedDoc, quiet: bool) -> bool {
        let Some(h) = self.live_hub.as_mut() else {
            return false;
        };
        let first = h.document.is_none();
        if let Some(shown) = h.document
            && doc.rev != 0
            && doc.rev < shown
        {
            return false;
        }
        h.document = Some(doc.rev);
        let title = if doc.title.is_empty() {
            None
        } else {
            Some(doc.title.clone())
        };
        let changed = self.live_replace(doc.document(), None).is_some();
        if let Some(t) = title
            && let Some(s) = self.session.as_mut()
        {
            s.title = t;
        }
        let name = self
            .session
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_default();
        if !quiet && (first || changed) {
            let key = if first {
                "live-document-shown"
            } else {
                "live-document-updated"
            };
            let m = self.msg_args(key, &args!["name" => name, "rev" => doc.rev]);
            // Polite and brief; at every verbosity, so a Braille reader
            // knows the text changed.
            self.tell(&m);
        }
        changed
    }

    /// Log entries arrived on a source showing a shared document: they
    /// are left out, and that is said once.
    pub(crate) fn live_document_entries_left_out(&mut self) {
        let Some(h) = self.live_hub.as_mut() else {
            return;
        };
        if h.document_entries_said {
            return;
        }
        h.document_entries_said = true;
        let m = self.msg("live-document-entries-left-out");
        self.tell(&m);
    }

    /// The answer to a log's command (view switch, jump to newest, Save
    /// log) on a shared document, which has no entries.
    pub(crate) fn live_document_one_view(&mut self) -> Vec<crate::command::Effect> {
        let m = self.msg("live-document-one-view");
        self.tell(&m);
        vec![crate::command::Effect::Redraw]
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use textweaver_a11y::{LogAnnouncer, Shared};

    use super::*;
    use crate::AppConfig;
    use crate::command::Command;
    use crate::live::{LiveOptions, ScriptHandle, ScriptedSource};
    use textweaver_keymap::ActionId;

    fn message(rev: u64, text: &str) -> String {
        serde_json::json!({
            "type": DOCUMENT_TYPE, "rev": rev, "title": "Agenda",
            "format": "markdown", "text": text
        })
        .to_string()
    }

    fn open() -> (App, ScriptHandle, Shared<LogAnnouncer>) {
        let said = Shared::new(LogAnnouncer::default());
        let mut app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            ..AppConfig::for_tests()
        });
        let (source, feed) = ScriptedSource::new();
        let _ = app.open_live_source(
            Box::new(source),
            LiveOptions {
                id: "club".into(),
                title: "Club".into(),
                ..LiveOptions::default()
            },
        );
        (app, feed, said)
    }

    fn texts(said: &Shared<LogAnnouncer>) -> Vec<String> {
        said.lock().texts().iter().map(|s| s.to_string()).collect()
    }

    fn text(app: &App) -> String {
        app.session().unwrap().doc.text().to_string()
    }

    #[test]
    fn a_shared_document_replaces_in_place_and_keeps_the_readers_place() {
        let (mut app, feed, said) = open();
        feed.push_json(
            "docs/agenda",
            &message(1, "# Agenda\n\n- Check-ins\n- Traffic\n"),
        );
        app.tick(Instant::now());
        assert!(app.live_shows_document());
        assert_eq!(app.session().unwrap().title, "Agenda");
        assert!(
            text(&app).contains("Check-ins\nTraffic\n"),
            "{:?}",
            text(&app)
        );
        assert!(
            texts(&said)
                .iter()
                .any(|t| t == "Agenda, revision 1, published live.")
        );
        assert_eq!(app.live_position().unwrap(), ["Revision 1"]);
        // The reader on Traffic.
        let t = text(&app);
        let at = t[..t.find("Traffic").unwrap()].chars().count();
        app.session.as_mut().unwrap().cursor = textweaver_core::CharPos(at);
        let heard = texts(&said).len();
        feed.push_json(
            "docs/agenda",
            &message(2, "# Agenda\n\nOpening.\n\n- Check-ins\n- Traffic\n"),
        );
        app.tick(Instant::now());
        let t = text(&app);
        let cursor = app.session().unwrap().cursor.0;
        assert_eq!(
            t.chars().skip(cursor).take(7).collect::<String>(),
            "Traffic"
        );
        assert_eq!(texts(&said)[heard..], ["Agenda updated, revision 2."]);
        // An older revision is left out; the same one again says nothing.
        let heard = texts(&said).len();
        feed.push_json("docs/agenda", &message(1, "# Old\n"));
        feed.push_json(
            "docs/agenda",
            &message(2, "# Agenda\n\nOpening.\n\n- Check-ins\n- Traffic\n"),
        );
        app.tick(Instant::now());
        assert_eq!(text(&app), t);
        assert_eq!(texts(&said).len(), heard, "{:?}", &texts(&said)[heard..]);
        // A log entry on the same source is left out, said once.
        feed.push_json("docs/agenda", r#"{"callsign":"N0CALL"}"#);
        feed.push_json("docs/agenda", r#"{"callsign":"N1CALL"}"#);
        app.tick(Instant::now());
        assert_eq!(text(&app), t);
        let left = texts(&said)
            .iter()
            .filter(|m| m.starts_with("Log entries on this source are left out"))
            .count();
        assert_eq!(left, 1);
        // No log commands on a document, and only its buttons.
        app.dispatch(Command::Action(ActionId::LiveSwitchView));
        assert!(
            texts(&said)
                .last()
                .unwrap()
                .starts_with("This is a published document")
        );
        assert_eq!(
            app.live_buttons(),
            if cfg!(feature = "mqtt") {
                vec![ActionId::LiveTogglePause, ActionId::CloseLiveSource]
            } else {
                Vec::new()
            }
        );
    }

    #[test]
    fn paused_the_newest_revision_waits_and_shows_on_resuming() {
        let (mut app, feed, said) = open();
        feed.push_json("d", &message(1, "one\n"));
        app.tick(Instant::now());
        app.live_set_paused(true);
        feed.push_json("d", &message(3, "three\n"));
        feed.push_json("d", &message(2, "two\n"));
        app.tick(Instant::now());
        assert!(text(&app).contains("one"), "{:?}", text(&app));
        assert_eq!(
            app.live_position().unwrap(),
            ["Revision 1", "updates paused"]
        );
        app.live_set_paused(false);
        assert!(text(&app).contains("three"), "{:?}", text(&app));
        assert_eq!(app.live_document_revision(), Some(3));
        assert_eq!(
            texts(&said).last().unwrap(),
            "Updates resumed, 1 update applied."
        );
    }

    #[test]
    fn only_a_typed_message_is_a_document() {
        let v = serde_json::json!({
            "type": DOCUMENT_TYPE, "rev": 2, "title": " Agenda ",
            "format": "markdown", "text": "# Agenda\n\n- one\n- two"
        });
        let d = SharedDoc::of(&v).unwrap();
        assert_eq!((d.rev, d.title.as_str(), d.markdown), (2, "Agenda", true));
        let doc = d.document();
        assert!(doc.text().to_string().contains("one\n"), "{}", doc.text());
        assert!(SharedDoc::of(&serde_json::json!({"text": "x"})).is_none());
        assert!(SharedDoc::of(&serde_json::json!({"type": DOCUMENT_TYPE})).is_none());
        let plain = SharedDoc::of(&serde_json::json!({
            "type": DOCUMENT_TYPE, "format": "text", "text": "line one\nline two"
        }))
        .unwrap();
        assert_eq!(plain.rev, 0);
        assert_eq!(plain.document().text().to_string(), "line one\nline two\n");
    }
}
