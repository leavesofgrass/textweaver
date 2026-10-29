//! Publish a check-in (beta 5, phase 5b): one entry, typed in a form and
//! published as a JSON object, so every reader's log view adds it.
//!
//! The form is the shared prompt model, as the edit details form
//! ([`crate::details_form`]) and the live source form are: each field is
//! a labeled line ("Callsign, 1 of 3") holding its value, and moving to
//! it says the label and the value. Tab and Shift+Tab move between the
//! fields, keeping what was typed; Enter sends; Escape cancels and says
//! so. An entry with every field empty is said, and the form opens again.
//!
//! The fields come from the live source's field mapping
//! (`[[live.sources.<id>.fields]]`), in its order, labeled as it labels
//! them; a source with none, or no source, gets the net log's fields:
//! callsign, name and notes. A net log is one example of a live
//! document, not a special case: the entry is a plain JSON object whose
//! keys are the fields' paths (a dotted path makes nested objects), in
//! the form's order, empty fields left out.
//!
//! Where it goes: into the broker the app runs, when one is running;
//! else to the open live source's broker, signed in as that source; else
//! through a live source chosen from a list. The topic is the source's
//! first plain topic (one without `+` or `#`), else `[live.broker]
//! checkin_topic`. A reader's account cannot publish: the broker refuses,
//! and that is said in words.

// Without the mqtt feature nothing opens the form: the commands that do
// are hidden, so its helpers go unused.
#![cfg_attr(not(feature = "mqtt"), allow(dead_code))]

use textweaver_a11y::Priority;
use textweaver_lexicon::args;
use textweaver_store::field_path_segments;

use crate::app::App;
use crate::command::{Effect, PromptPurpose};

/// Where a check-in or a shared document is published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SendTarget {
    /// The broker the app runs (Start your broker): no network, no login.
    Broker,
    /// The broker of live source `id`, signed in as that source.
    Source(String),
}

/// One field of the check-in form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckInField {
    /// Its path in the entry (`callsign`, `station.grid`).
    pub(crate) path: String,
    /// Its label.
    pub(crate) label: String,
}

/// The check-in form, while it is open.
#[derive(Clone, Debug)]
pub(crate) struct CheckInForm {
    pub(crate) target: SendTarget,
    pub(crate) topic: String,
    pub(crate) fields: Vec<CheckInField>,
    pub(crate) values: Vec<String>,
    pub(crate) focus: usize,
}

/// The net log's fields, offered when the source maps none: their paths
/// and the message ids of their labels.
const DEFAULT_FIELDS: [(&str, &str); 3] = [
    ("callsign", "live-field-callsign"),
    ("name", "live-field-name"),
    ("notes", "live-field-notes"),
];

/// True for a topic a message can be published to: no wildcard.
pub(crate) fn is_plain_topic(topic: &str) -> bool {
    let t = topic.trim();
    !t.is_empty() && !t.contains(['+', '#'])
}

/// The entry for `fields` and `values`: a JSON object in the form's
/// order, empty values left out, a dotted path or a JSON Pointer making
/// nested objects; `None` when every value is empty.
pub(crate) fn entry_json(fields: &[CheckInField], values: &[String]) -> Option<String> {
    let mut root = serde_json::Map::new();
    let mut any = false;
    for (f, v) in fields.iter().zip(values) {
        let v = v.trim();
        if v.is_empty() {
            continue;
        }
        let keys = field_path_segments(&f.path).unwrap_or_else(|| vec![f.path.clone()]);
        any |= place(&mut root, &keys, v);
    }
    if !any {
        return None;
    }
    // serde_json keeps keys sorted; write them in the form's order.
    Some(ordered(&serde_json::Value::Object(root), fields))
}

/// Puts `value` at `keys` in `obj`, making objects on the way; false when
/// a key on the way already holds a value that is not an object.
fn place(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    keys: &[String],
    value: &str,
) -> bool {
    match keys {
        [] => false,
        [last] => {
            obj.insert(last.clone(), serde_json::Value::String(value.to_owned()));
            true
        }
        [first, rest @ ..] => {
            let next = obj
                .entry(first.clone())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            match next.as_object_mut() {
                Some(inner) => place(inner, rest, value),
                None => false,
            }
        }
    }
}

/// `value` as JSON text, an object's keys in the order the fields first
/// name them.
fn ordered(value: &serde_json::Value, fields: &[CheckInField]) -> String {
    let order: Vec<String> = fields
        .iter()
        .flat_map(|f| field_path_segments(&f.path).unwrap_or_else(|| vec![f.path.clone()]))
        .collect();
    write_ordered(value, &order)
}

fn write_ordered(value: &serde_json::Value, order: &[String]) -> String {
    let Some(obj) = value.as_object() else {
        return value.to_string();
    };
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort_by_key(|k| order.iter().position(|o| o == *k).unwrap_or(usize::MAX));
    let parts: Vec<String> = keys
        .into_iter()
        .map(|k| {
            format!(
                "{}:{}",
                serde_json::Value::String(k.clone()),
                write_ordered(&obj[k], order)
            )
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

impl App {
    /// The fields a check-in to source `id` offers: its field mapping, or
    /// the net log's callsign, name and notes.
    pub(crate) fn live_checkin_fields(&self, id: Option<&str>) -> Vec<CheckInField> {
        let mapped: Vec<CheckInField> = id
            .and_then(|id| self.settings.live.sources.get(id))
            .map(|s| {
                let mut fields = s.fields.clone();
                fields.sort_by_key(|f| f.order.unwrap_or(u32::MAX));
                fields
                    .iter()
                    .filter(|f| field_path_segments(&f.path).is_some())
                    .map(|f| CheckInField {
                        path: f.path.clone(),
                        label: if f.label.trim().is_empty() {
                            f.path.trim_start_matches('/').to_owned()
                        } else {
                            f.label.trim().to_owned()
                        },
                    })
                    .collect()
            })
            .unwrap_or_default();
        if !mapped.is_empty() {
            return mapped;
        }
        DEFAULT_FIELDS
            .iter()
            .map(|(path, label)| CheckInField {
                path: (*path).to_owned(),
                label: self.msg(label),
            })
            .collect()
    }

    /// The topic a check-in to source `id` goes on: its first plain topic,
    /// else `[live.broker] checkin_topic`.
    pub(crate) fn live_checkin_topic(&self, id: Option<&str>) -> String {
        id.and_then(|id| self.settings.live.sources.get(id))
            .and_then(|s| s.topics.iter().find(|t| is_plain_topic(t)))
            .map_or_else(
                || self.settings.live.broker.checkin_topic().to_owned(),
                |t| t.trim().to_owned(),
            )
    }

    /// Opens the check-in form for `target`, its fields and topic from
    /// source `source` (the target's, or the open one's).
    pub(crate) fn open_checkin_form(
        &mut self,
        target: SendTarget,
        source: Option<String>,
    ) -> Vec<Effect> {
        let fields = self.live_checkin_fields(source.as_deref());
        let topic = self.live_checkin_topic(source.as_deref());
        let values = vec![String::new(); fields.len()];
        let to = match &target {
            SendTarget::Broker => self.msg("live-checkin-to-broker"),
            SendTarget::Source(id) => self.live_display_name(id),
        };
        self.live_wire.checkin = Some(CheckInForm {
            target,
            topic: topic.clone(),
            fields,
            values,
            focus: 0,
        });
        let intro = self.msg_args("live-checkin-intro", &args!["to" => to, "topic" => topic]);
        self.tell(&intro);
        self.checkin_field_prompt(false)
    }

    /// The prompt for the focused field, its label and value said:
    /// queued after the introduction when `interrupt` is false, at once
    /// when moving between fields.
    fn checkin_field_prompt(&mut self, interrupt: bool) -> Vec<Effect> {
        let Some(form) = &self.live_wire.checkin else {
            return vec![Effect::Redraw];
        };
        let field = form.fields[form.focus].label.clone();
        let value = form.values[form.focus].clone();
        let (n, total) = (form.focus + 1, form.fields.len());
        let label = self.msg_args(
            "details-prompt-label",
            &args!["label" => field.as_str(), "n" => n, "total" => total],
        );
        let shown = if value.trim().is_empty() {
            self.msg("nav-blank")
        } else {
            value.clone()
        };
        let said = self.msg_args("details-field", &args!["label" => field, "value" => shown]);
        if interrupt {
            self.announce(&said, Priority::Assertive);
        } else {
            self.announce_queued(&said, Priority::Polite);
        }
        if !self.mode.is_prompt() {
            self.return_mode = self.mode;
        }
        self.mode = crate::app::Mode::Prompt;
        self.prompt_purpose = PromptPurpose::LiveCheckIn;
        self.pending_prompt_text = Some(value);
        vec![Effect::Prompt {
            label,
            purpose: PromptPurpose::LiveCheckIn,
        }]
    }

    /// Tab (`step` 1) or Shift+Tab (`step` -1) in the check-in form:
    /// keeps what the field holds and moves to the next or previous field,
    /// wrapping.
    pub(crate) fn checkin_move(&mut self, text: String, step: isize) -> Vec<Effect> {
        let Some(form) = self.live_wire.checkin.as_mut() else {
            return vec![Effect::Redraw];
        };
        form.values[form.focus] = text;
        let n = form.fields.len() as isize;
        form.focus = (form.focus as isize + step).rem_euclid(n) as usize;
        self.checkin_field_prompt(true)
    }

    /// Enter in the check-in form: sends the entry, or, with every field
    /// empty, says so and opens the form again.
    pub(crate) fn answer_live_checkin(&mut self, text: &str) -> Vec<Effect> {
        let Some(mut form) = self.live_wire.checkin.take() else {
            return vec![Effect::Redraw];
        };
        form.values[form.focus] = text.to_owned();
        let Some(entry) = entry_json(&form.fields, &form.values) else {
            let m = self.msg("live-checkin-empty");
            self.error(&m);
            form.focus = 0;
            self.live_wire.checkin = Some(form);
            return self.checkin_field_prompt(false);
        };
        self.leave_prompt();
        // What is said once it is sent: the entry's first value, meaning
        // first ("N0CALL").
        let first = form
            .values
            .iter()
            .map(|v| v.trim())
            .find(|v| !v.is_empty())
            .unwrap_or_default()
            .to_owned();
        self.live_send(
            form.target,
            &form.topic,
            entry.into_bytes(),
            false,
            super::wire::Sent::CheckIn { first },
        )
    }

    /// Escape in the check-in form: nothing is sent, and the owner hears
    /// so.
    pub(crate) fn cancel_live_checkin(&mut self) -> Vec<Effect> {
        if self.live_wire.checkin.take().is_none() {
            return vec![Effect::Redraw];
        }
        self.leave_prompt();
        let m = self.msg("live-checkin-cancelled");
        self.tell(&m);
        vec![Effect::Redraw]
    }

    /// True while a field of the check-in form is the open prompt.
    pub(crate) fn prompt_is_checkin(&self) -> bool {
        self.live_wire.checkin.is_some()
            && self.mode.is_prompt()
            && self.prompt_purpose == PromptPurpose::LiveCheckIn
    }
}

#[cfg(test)]
mod tests {
    use textweaver_a11y::{LogAnnouncer, Shared};
    use textweaver_store::{LiveField, LiveSourceSettings};

    use super::*;
    use crate::AppConfig;
    use crate::command::Command;
    use crate::list_model::PromptKey;

    #[test]
    fn the_form_offers_the_sources_fields_and_escape_sends_nothing() {
        let said = Shared::new(LogAnnouncer::default());
        let mut app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            ..AppConfig::for_tests()
        });
        let field = |path: &str, label: &str, order: Option<u32>| LiveField {
            path: path.into(),
            label: label.into(),
            order,
            ..LiveField::default()
        };
        app.settings.live.sources.insert(
            "club".into(),
            LiveSourceSettings {
                name: "Club net".into(),
                host: "127.0.0.1".into(),
                topics: vec!["club/#".into(), "club/in".into()],
                fields: vec![
                    field("station.grid", "Grid", Some(2)),
                    field("op", "", Some(1)),
                    field("a..b", "Broken", None),
                ],
                ..LiveSourceSettings::default()
            },
        );
        // No broker and no source open: the sources, to choose one.
        app.dispatch(Command::Action(textweaver_keymap::ActionId::SendCheckIn));
        assert_eq!(app.list_model().unwrap().items, ["Club net"]);
        app.dispatch(Command::ListKey(crate::list_model::ListKey::Enter));
        let texts =
            || -> Vec<String> { said.lock().texts().iter().map(|s| s.to_string()).collect() };
        assert!(
            texts().iter().any(|t| t == "Check-in to Club net, on club/in. Tab moves between fields, Enter publishes, Escape cancels."),
            "the first plain topic: {:?}",
            texts()
        );
        let label = |app: &App| app.prompt_model().unwrap().label.clone();
        assert_eq!(
            label(&app),
            "op, 1 of 2",
            "mapped, in order, a broken path left out"
        );
        app.dispatch(Command::PromptKey(PromptKey::Char('x')));
        app.dispatch(Command::PromptKey(PromptKey::Tab));
        assert_eq!(label(&app), "Grid, 2 of 2");
        assert_eq!(texts().last().unwrap(), "Grid: blank");
        app.dispatch(Command::PromptKey(PromptKey::BackTab));
        assert_eq!(
            app.prompt_model().unwrap().text(),
            "x",
            "Tab keeps the value"
        );
        // Every field empty: said, and the form opens again.
        app.dispatch(Command::PromptKey(PromptKey::Backspace));
        app.dispatch(Command::PromptKey(PromptKey::Enter));
        assert!(
            texts()
                .iter()
                .any(|t| t.contains("Fill in at least one field."))
        );
        assert_eq!(label(&app), "op, 1 of 2");
        app.dispatch(Command::PromptKey(PromptKey::Escape));
        assert!(app.prompt_model().is_none());
        assert_eq!(texts().last().unwrap(), "Canceled. No check-in published.");
        // No source: the net log's fields and the check-in topic.
        assert_eq!(
            app.live_checkin_fields(None)
                .iter()
                .map(|f| f.label.as_str())
                .collect::<Vec<_>>(),
            ["Callsign", "Name", "Notes"]
        );
        assert_eq!(app.live_checkin_topic(None), "netlog/checkins");
    }

    fn f(path: &str) -> CheckInField {
        CheckInField {
            path: path.into(),
            label: path.into(),
        }
    }

    #[test]
    fn the_entry_keeps_the_forms_order_and_leaves_out_empty_fields() {
        let fields = [f("callsign"), f("name"), f("notes")];
        let values = [
            "N0CALL".to_owned(),
            " Ada Example ".to_owned(),
            String::new(),
        ];
        assert_eq!(
            entry_json(&fields, &values).as_deref(),
            Some(r#"{"callsign":"N0CALL","name":"Ada Example"}"#)
        );
        assert_eq!(
            entry_json(&fields, &[String::new(), " ".into(), String::new()]),
            None
        );
        let nested = [f("zeta"), f("station.grid"), f("/station/power")];
        let values = ["1".to_owned(), "CN85".to_owned(), "5".to_owned()];
        assert_eq!(
            entry_json(&nested, &values).as_deref(),
            Some(r#"{"zeta":"1","station":{"grid":"CN85","power":"5"}}"#)
        );
        assert!(is_plain_topic("netlog/checkins"));
        assert!(!is_plain_topic("netlog/#"));
        assert!(!is_plain_topic(" "));
    }
}
