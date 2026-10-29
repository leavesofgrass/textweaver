//! Set up or edit a live source in one form (beta 5, phase 4), and the
//! live source files (`.twlive`) that fill it.
//!
//! The form is the shared prompt model ([`crate::list_model`]), as the
//! edit details form is ([`crate::details_form`]): each field is a labeled
//! line ("Host, 2 of 7") holding its value, and moving to it says the
//! label and the value. Tab and Shift+Tab move between the fields,
//! keeping what was typed; Enter checks and saves every field; Escape
//! cancels and says so. A field that is wrong is said in words, and the
//! form opens again on it with every other value kept.
//!
//! The fields, in order: name, host, encryption, CA file (only when the
//! encryption is a CA file), pin (only with a pinned key), join code (only
//! when joining by code), port, topic, user name (not when joining by
//! code), and view. Encryption and view are choices: Up and Down Arrow
//! change them in place, and each choice is said with what it means. The
//! port follows the encryption (1883 off, 8883 with TLS) unless it was
//! changed. The password is not in the form: once the source is saved
//! with a user name, it is asked for in a masked prompt and kept in the
//! system credential store.
//!
//! **Joining by code** (textweaver's own broker, `tw broker --join-code`
//! or Start a broker): the code's 16 characters are typed in the form, in
//! any case, with or without separators. The source is saved as a pinned
//! key (the code's first 12 characters) with the user name `join`, and
//! the password derived from the whole code goes straight to the system
//! credential store: the code itself is kept nowhere. Editing such a
//! source shows the join code field empty; left empty, the kept password
//! stays. The field is secret ([`crate::PromptModel::is_secret`]):
//! masked, never echoed or recalled, its value said as a count of
//! characters, until the show key shows it.
//!
//! **Join links** (crate::live::broker's Copy the join link,
//! `textweaver_mqtt::JoinLink`): pasting a link into any field fills the
//! host, the port, the encryption (Join code), the code and the topic,
//! and the name with the host when it is empty, and says what it filled;
//! the field the link was pasted into keeps what it held, unless the link
//! fills it. The focus goes to the topic when the link names none.
//! Pasting into the join code field takes the code out of whatever came
//! with it: line breaks, a "Join code:" label, quotes, the "Show the join
//! code" list, or a link. A form filled from a link connects on Enter,
//! with no Open it now question. **Open a join link** opens the form
//! filled from the link on the clipboard, or empty for one to be pasted.
//! A link is never echoed, kept in the prompt history, or saved: only
//! its host, port, short pin and topic are, as for a typed join code.
//!
//! **Set up a live source** opens the form empty; **Edit a live source**
//! (or F2 in the Open live source list) opens it filled in, and keeps the
//! keys the form does not show (field mapping, `id_field`). Opening a
//! live source file (File, Open, the command line) reads it as untrusted
//! input ([`textweaver_store::read_live_source_file`]), says what was left
//! out, and opens the form filled from it; Enter saves it as a new source
//! whose id comes from its name, never replacing one already set up.
//! **Save live source as file** writes a source to a `.twlive` file to
//! share, without a password.

use std::path::{Path, PathBuf};

use textweaver_a11y::Priority;
use textweaver_lexicon::args;
use textweaver_store::{
    LIVE_SOURCE_FILE_EXTENSION, LIVE_SOURCE_FILE_MAX, LiveFileError, LiveFileNote,
    LiveSourceSettings, LiveTls, LiveView,
};

use super::wire::LiveList;
use crate::app::{App, ListKind};
use crate::command::{Effect, PromptPurpose};

/// One field of the form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormField {
    Name,
    Host,
    Tls,
    CaFile,
    Pin,
    JoinCode,
    Port,
    Topic,
    Username,
    View,
}

impl FormField {
    /// Every field, in the form's order.
    const ALL: [FormField; 10] = [
        FormField::Name,
        FormField::Host,
        FormField::Tls,
        FormField::CaFile,
        FormField::Pin,
        FormField::JoinCode,
        FormField::Port,
        FormField::Topic,
        FormField::Username,
        FormField::View,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|f| *f == self).unwrap_or(0)
    }

    /// The message id of its label.
    fn label_id(self) -> &'static str {
        match self {
            FormField::Name => "live-form-name",
            FormField::Host => "live-form-host",
            FormField::Tls => "live-form-tls",
            FormField::CaFile => "live-form-ca-file",
            FormField::Pin => "live-form-pin",
            FormField::JoinCode => "live-form-join-code",
            FormField::Port => "live-form-port",
            FormField::Topic => "live-form-topic",
            FormField::Username => "live-form-username",
            FormField::View => "live-form-view",
        }
    }

    /// A choice, changed with Up and Down Arrow.
    fn is_choice(self) -> bool {
        matches!(self, FormField::Tls | FormField::View)
    }
}

/// The user name a join code signs in with (`textweaver_mqtt::JOIN_USER`).
pub(crate) const JOIN_USER: &str = "join";

/// The encryption field's choices: the settings' encryptions, and joining
/// by code, which is saved as a pinned key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Protect {
    #[default]
    Off,
    System,
    CaFile,
    Pinned,
    Join,
}

impl Protect {
    /// The encryption it is saved as.
    fn tls(self) -> LiveTls {
        match self {
            Protect::Off => LiveTls::Off,
            Protect::System => LiveTls::System,
            Protect::CaFile => LiveTls::CaFile,
            Protect::Pinned | Protect::Join => LiveTls::Pinned,
        }
    }

    /// Its value in a settings file or a typed answer.
    fn as_str(self) -> &'static str {
        match self {
            Protect::Join => "join_code",
            other => other.tls().as_str(),
        }
    }

    /// The choice that shows `source`: a pinned key with the user name
    /// `join` and a 12-character pin is a source joined by code.
    fn of(source: &LiveSourceSettings) -> Protect {
        match source.tls {
            LiveTls::Off => Protect::Off,
            LiveTls::System => Protect::System,
            LiveTls::CaFile => Protect::CaFile,
            LiveTls::Pinned => {
                let pin_chars = source
                    .pin
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .count();
                if source.username.trim() == JOIN_USER && pin_chars == 12 {
                    Protect::Join
                } else {
                    Protect::Pinned
                }
            }
        }
    }
}

/// The encryption choices, in the order Down Arrow goes through them.
const TLS_CHOICES: [Protect; 5] = [
    Protect::Off,
    Protect::System,
    Protect::CaFile,
    Protect::Pinned,
    Protect::Join,
];

/// The view choices, in order.
const VIEW_CHOICES: [LiveView; 2] = [LiveView::Log, LiveView::Values];

/// The message id of an encryption choice's word, and of what it means.
fn tls_ids(tls: Protect) -> (&'static str, &'static str) {
    match tls {
        Protect::Off => ("live-form-tls-off", "live-form-tls-off-about"),
        Protect::System => ("live-form-tls-system", "live-form-tls-system-about"),
        Protect::CaFile => ("live-form-tls-ca-file", "live-form-tls-ca-file-about"),
        Protect::Pinned => ("live-form-tls-pinned", "live-form-tls-pinned-about"),
        Protect::Join => ("live-form-tls-join", "live-form-tls-join-about"),
    }
}

/// The message id of a view choice's word, and of what it means.
fn view_ids(view: LiveView) -> (&'static str, &'static str) {
    match view {
        LiveView::Log => ("live-form-view-log", "live-form-view-log-about"),
        LiveView::Values => ("live-form-view-values", "live-form-view-values-about"),
    }
}

/// The port for the choice `p` when none is set.
fn choice_port(p: Protect) -> u16 {
    default_port(p.tls())
}

/// The port for `tls` when none is set.
fn default_port(tls: LiveTls) -> u16 {
    LiveSourceSettings {
        tls,
        ..LiveSourceSettings::default()
    }
    .port_or_default()
}

/// Where the form came from, and so what Enter and Escape do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FormOrigin {
    /// Set up a live source: a new source.
    New,
    /// Edit a live source: source `id`; `back` is the Open live source
    /// list's row it was opened from with F2, to show again on Escape.
    Edit { id: String, back: Option<usize> },
    /// A live source file, named `file`: saved as a new source.
    File { file: String },
}

/// The form, while it is open.
#[derive(Clone, Debug)]
pub(crate) struct LiveForm {
    /// The source it started from: the keys the form does not show (field
    /// mapping, `id_field`, the rest) are kept from it.
    base: LiveSourceSettings,
    /// What each field holds now, in [`FormField::ALL`]'s order.
    values: [String; 10],
    /// The field being edited.
    focus: FormField,
    origin: FormOrigin,
    /// Filled from a join link: Enter saves and connects at once.
    from_link: bool,
}

impl LiveForm {
    fn value(&self, f: FormField) -> &str {
        &self.values[f.index()]
    }

    fn set(&mut self, f: FormField, text: String) {
        self.values[f.index()] = text;
    }
}

/// What a field's text means, once checked.
enum Checked {
    /// The source, and the password a join code gives, to keep.
    Ok(
        Box<LiveSourceSettings>,
        Option<textweaver_components::credentials::Password>,
    ),
    Bad(FormField, String),
}

impl App {
    /// The words of the encryption choice `tls`.
    fn tls_word(&self, tls: Protect) -> String {
        self.msg(tls_ids(tls).0)
    }

    fn view_word(&self, view: LiveView) -> String {
        self.msg(view_ids(view).0)
    }

    /// The encryption a field's text names: one of the choices' words, or
    /// its value in `settings.toml`, in any case.
    fn parse_tls(&self, text: &str) -> Option<Protect> {
        let t = text.trim().to_lowercase();
        TLS_CHOICES.into_iter().find(|c| {
            self.tls_word(*c).to_lowercase() == t
                || c.as_str() == t
                || c.as_str().replace('_', " ") == t
        })
    }

    fn parse_view(&self, text: &str) -> Option<LiveView> {
        let t = text.trim().to_lowercase();
        VIEW_CHOICES
            .into_iter()
            .find(|c| self.view_word(*c).to_lowercase() == t || c.as_str() == t)
    }

    /// The fields shown, in order: the CA file, the pin and the join code
    /// only with their encryption, and no user name when joining by code.
    fn live_form_fields(&self, form: &LiveForm) -> Vec<FormField> {
        let p = self.parse_tls(form.value(FormField::Tls));
        FormField::ALL
            .into_iter()
            .filter(|f| match f {
                FormField::CaFile => p == Some(Protect::CaFile),
                FormField::Pin => p == Some(Protect::Pinned),
                FormField::JoinCode => p == Some(Protect::Join),
                FormField::Username => p != Some(Protect::Join),
                _ => true,
            })
            .collect()
    }

    /// Opens the form on `source`, said with `intro` and then any `extra`
    /// message (what a file left out).
    fn open_live_form(
        &mut self,
        source: LiveSourceSettings,
        origin: FormOrigin,
        intro: &str,
        extra: Option<String>,
    ) -> Vec<Effect> {
        let port = source.port_or_default().to_string();
        let values = [
            source.name.clone(),
            source.host.clone(),
            self.tls_word(Protect::of(&source)),
            source.ca_file.clone(),
            source.pin.clone(),
            // A join code is never kept: empty keeps the password.
            String::new(),
            port,
            source.topics.join(", "),
            source.username.clone(),
            self.view_word(source.view),
        ];
        self.live_wire.offer_after_password = None;
        self.live_wire.form = Some(LiveForm {
            base: source,
            values,
            focus: FormField::Name,
            origin,
            from_link: false,
        });
        self.tell(intro);
        if let Some(extra) = extra {
            self.tell(&extra);
        }
        self.live_form_prompt(false)
    }

    /// Set up a live source: the form, empty.
    pub fn live_setup(&mut self) -> Vec<Effect> {
        let intro = self.msg("live-form-intro-new");
        self.open_live_form(LiveSourceSettings::default(), FormOrigin::New, &intro, None)
    }

    /// Edit a live source: the open source's form, or, with none open, a
    /// list of the sources to choose from.
    pub fn live_edit_command(&mut self) -> Vec<Effect> {
        if let Some(id) = self.open_configured_id() {
            return self.live_edit(&id, None);
        }
        self.live_choose_list(LiveList::Edit, "live-edit-title")
    }

    /// The form filled with source `id`'s settings; `back` is the Open
    /// live source list's row it came from.
    pub(crate) fn live_edit(&mut self, id: &str, back: Option<usize>) -> Vec<Effect> {
        let Some(source) = self.settings.live.sources.get(id).cloned() else {
            let m = self.msg_args("live-source-unknown", &args!["source" => id]);
            self.error(&m);
            return vec![Effect::Redraw];
        };
        let name = source.display_name(id).to_owned();
        let intro = self.msg_args("live-form-intro-edit", &args!["name" => name]);
        let origin = FormOrigin::Edit {
            id: id.to_owned(),
            back,
        };
        self.open_live_form(source, origin, &intro, None)
    }

    /// The configured source open in the reader, if any.
    pub(crate) fn open_configured_id(&self) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|s| s.live.as_ref())
            .map(|t| t.source.clone())
            .filter(|id| self.settings.live.sources.contains_key(id))
    }

    /// The configured sources as a list titled `title_id`, for `kind`;
    /// with none set up, says how to add one.
    fn live_choose_list(
        &mut self,
        kind: fn(Vec<String>) -> LiveList,
        title_id: &str,
    ) -> Vec<Effect> {
        let ids: Vec<String> = self.settings.live.sources.keys().cloned().collect();
        if ids.is_empty() {
            let m = self.msg("live-none-set-up");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        let items = ids
            .iter()
            .map(|id| self.settings.live.sources[id].display_name(id).to_owned())
            .collect();
        let title = self.msg(title_id);
        self.list = Some(ListKind::Live(kind(ids)));
        vec![Effect::ShowList { title, items }]
    }

    /// The prompt for the focused field, its label and value said: queued
    /// after what was said before when `interrupt` is false, at once when
    /// moving between fields.
    fn live_form_prompt(&mut self, interrupt: bool) -> Vec<Effect> {
        let Some(form) = &self.live_wire.form else {
            return vec![Effect::Redraw];
        };
        let fields = self.live_form_fields(form);
        let field = form.focus;
        let value = form.value(field).to_owned();
        let n = fields.iter().position(|f| *f == field).unwrap_or(0) + 1;
        let total = fields.len();
        let label_text = self.msg(field.label_id());
        let label_id = if field.is_choice() {
            "live-form-choice-label"
        } else {
            "details-prompt-label"
        };
        let label = self.msg_args(
            label_id,
            &args!["label" => label_text.as_str(), "n" => n, "total" => total],
        );
        // The join code is a secret field: masked like a password, its
        // value never said.
        let secret = field == FormField::JoinCode;
        let shown = if value.trim().is_empty() {
            self.msg("nav-blank")
        } else if secret {
            self.msg_args(
                "prompt-secret-hidden",
                &args!["count" => value.chars().count()],
            )
        } else {
            value.clone()
        };
        let said_id = if field.is_choice() {
            "live-form-choice-field"
        } else {
            "details-field"
        };
        let mut said = self.msg_args(said_id, &args!["label" => label_text, "value" => shown]);
        if secret {
            said = self.secret_prompt_said(&said);
        }
        if interrupt {
            self.announce(&said, Priority::Assertive);
        } else {
            self.announce_queued(&said, Priority::Polite);
        }
        if !self.mode.is_prompt() {
            self.return_mode = self.mode;
        }
        self.mode = crate::app::Mode::Prompt;
        self.prompt_purpose = PromptPurpose::LiveSetup;
        self.pending_prompt_text = Some(value);
        self.pending_prompt_secret = secret;
        vec![Effect::Prompt {
            label,
            purpose: PromptPurpose::LiveSetup,
        }]
    }

    /// Tab (`step` 1) or Shift+Tab (`step` -1) in the form: keeps what the
    /// field holds and moves to the next or previous field shown,
    /// wrapping.
    pub(crate) fn live_form_move(&mut self, text: String, step: isize) -> Vec<Effect> {
        let Some(mut form) = self.live_wire.form.take() else {
            return vec![Effect::Redraw];
        };
        let focus = form.focus;
        self.live_form_keep(&mut form, text);
        let fields = self.live_form_fields(&form);
        let at = fields.iter().position(|f| *f == focus).unwrap_or(0) as isize;
        form.focus = fields[(at + step).rem_euclid(fields.len() as isize) as usize];
        self.live_wire.form = Some(form);
        self.live_form_prompt(true)
    }

    /// The focused field's text becomes `text`.
    fn live_form_keep(&self, form: &mut LiveForm, text: String) {
        if form.focus == FormField::Tls {
            let before = form.value(FormField::Tls).to_owned();
            self.live_form_set_tls(form, &before, text);
        } else {
            form.set(form.focus, text);
        }
    }

    /// The encryption field's text becomes `text` (it was `before`): the
    /// port follows it when it still held the old encryption's default or
    /// nothing.
    fn live_form_set_tls(&self, form: &mut LiveForm, before: &str, text: String) {
        let old = self.parse_tls(before).unwrap_or_default();
        let new = self.parse_tls(&text);
        form.set(FormField::Tls, text);
        let Some(new) = new else {
            return;
        };
        let port = form.value(FormField::Port).trim().to_owned();
        if port.is_empty() || port == choice_port(old).to_string() {
            form.set(FormField::Port, choice_port(new).to_string());
        }
    }

    /// Up (`step` -1) or Down (`step` 1) in the form: on a choice, the
    /// previous or next choice, said with what it means; `None` on any
    /// other field, where the keys recall earlier answers as usual.
    pub(crate) fn live_form_cycle(&mut self, step: isize) -> Option<Vec<Effect>> {
        if !self.prompt_is_live_form() {
            return None;
        }
        let mut form = self.live_wire.form.take()?;
        let field = form.focus;
        if !field.is_choice() {
            self.live_wire.form = Some(form);
            return None;
        }
        let text = self
            .prompt_model
            .as_ref()
            .map_or_else(|| form.value(field).to_owned(), crate::PromptModel::text);
        let (word, about) = if field == FormField::Tls {
            let at = self
                .parse_tls(&text)
                .and_then(|t| TLS_CHOICES.iter().position(|c| *c == t));
            let n = TLS_CHOICES.len() as isize;
            let i = at.map_or(0, |i| (i as isize + step).rem_euclid(n) as usize);
            let tls = TLS_CHOICES[i];
            let word = self.tls_word(tls);
            // Joining by code fills the user name in; leaving it clears it.
            if tls == Protect::Join || form.value(FormField::Username).trim() == JOIN_USER {
                form.set(FormField::Username, String::new());
            }
            self.live_form_set_tls(&mut form, &text, word.clone());
            (word, self.msg(tls_ids(tls).1))
        } else {
            let at = self
                .parse_view(&text)
                .and_then(|v| VIEW_CHOICES.iter().position(|c| *c == v));
            let n = VIEW_CHOICES.len() as isize;
            let i = at.map_or(0, |i| (i as isize + step).rem_euclid(n) as usize);
            let view = VIEW_CHOICES[i];
            let word = self.view_word(view);
            form.set(FormField::View, word.clone());
            (word, self.msg(view_ids(view).1))
        };
        self.live_wire.form = Some(form);
        if let Some(model) = self.prompt_model.as_mut() {
            model.set_text(&word);
        }
        self.announce(&about, Priority::Assertive);
        Some(vec![Effect::Redraw])
    }

    /// True while a field of the form is the open prompt.
    pub(crate) fn prompt_is_live_form(&self) -> bool {
        self.live_wire.form.is_some()
            && self.mode.is_prompt()
            && self.prompt_purpose == PromptPurpose::LiveSetup
    }

    /// Checks every field; the first wrong one, in the form's order, with
    /// what is wrong in words.
    fn live_form_check(&self, form: &LiveForm) -> Checked {
        let mut s = form.base.clone();
        let text = |f: FormField| form.value(f).trim().to_owned();
        let name = text(FormField::Name);
        if name.is_empty() {
            return Checked::Bad(FormField::Name, self.msg("live-form-name-needed"));
        }
        s.name = name;
        let host = text(FormField::Host);
        if host.is_empty() || host.chars().any(char::is_whitespace) {
            return Checked::Bad(FormField::Host, self.msg("live-setup-host-needed"));
        }
        s.host = host;
        let Some(protect) = self.parse_tls(form.value(FormField::Tls)) else {
            let choices = TLS_CHOICES.map(|t| self.tls_word(t)).join(", ");
            let label = self.msg(FormField::Tls.label_id());
            let m = self.msg_args(
                "live-form-not-a-choice",
                &args!["label" => label, "choices" => choices],
            );
            return Checked::Bad(FormField::Tls, m);
        };
        let tls = protect.tls();
        s.tls = tls;
        let mut join_password = None;
        match protect {
            Protect::Pinned => match self.live_form_pin(&text(FormField::Pin)) {
                Ok(pin) => s.pin = pin,
                Err(why) => return Checked::Bad(FormField::Pin, why),
            },
            Protect::Join => {
                match self.live_form_join(form, &text(FormField::JoinCode)) {
                    Ok((pin, password)) => {
                        s.pin = pin;
                        join_password = password;
                    }
                    Err(why) => return Checked::Bad(FormField::JoinCode, why),
                }
                s.username = JOIN_USER.to_owned();
            }
            _ => s.pin.clear(),
        }
        if tls == LiveTls::CaFile {
            let ca = text(FormField::CaFile);
            if ca.is_empty() {
                return Checked::Bad(FormField::CaFile, self.msg("live-setup-ca-file-needed"));
            }
            if !Path::new(&ca).is_file() {
                let m = self.msg_args("live-setup-ca-file-missing", &args!["path" => ca.as_str()]);
                return Checked::Bad(FormField::CaFile, m);
            }
            s.ca_file = ca;
        }
        let default = choice_port(protect);
        let port = text(FormField::Port);
        let port = if port.is_empty() {
            Some(default)
        } else {
            port.parse::<u16>().ok().filter(|p| *p > 0)
        };
        let Some(port) = port else {
            return Checked::Bad(FormField::Port, self.msg("live-setup-port-bad"));
        };
        s.port = (port != default).then_some(port);
        let topics: Vec<String> = form
            .value(FormField::Topic)
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect();
        if topics.is_empty() {
            return Checked::Bad(FormField::Topic, self.msg("live-setup-topic-needed"));
        }
        s.topics = topics;
        if protect != Protect::Join {
            s.username = text(FormField::Username);
        }
        let Some(view) = self.parse_view(form.value(FormField::View)) else {
            let choices = VIEW_CHOICES.map(|v| self.view_word(v)).join(", ");
            let label = self.msg(FormField::View.label_id());
            let m = self.msg_args(
                "live-form-not-a-choice",
                &args!["label" => label, "choices" => choices],
            );
            return Checked::Bad(FormField::View, m);
        };
        s.view = view;
        Checked::Ok(Box::new(s), join_password)
    }

    /// A pinned key's pin as typed: checked, and kept in groups of four.
    fn live_form_pin(&self, typed: &str) -> Result<String, String> {
        if typed.is_empty() {
            return Err(self.msg("live-form-pin-needed"));
        }
        #[cfg(feature = "mqtt")]
        {
            textweaver_mqtt::Pin::parse(typed)
                .map(|p| p.grouped())
                .map_err(|_| self.msg("live-form-pin-bad"))
        }
        #[cfg(not(feature = "mqtt"))]
        {
            Ok(typed.to_owned())
        }
    }

    /// A join code as typed: the pin it carries, and the password it
    /// gives. Empty keeps an edited source's kept password (no new one).
    fn live_form_join(
        &self,
        form: &LiveForm,
        typed: &str,
    ) -> Result<(String, Option<textweaver_components::credentials::Password>), String> {
        if typed.is_empty() {
            let kept = matches!(form.origin, FormOrigin::Edit { .. })
                && Protect::of(&form.base) == Protect::Join;
            if kept {
                return Ok((form.base.pin.clone(), None));
            }
            return Err(self.msg("live-form-join-needed"));
        }
        #[cfg(feature = "mqtt")]
        {
            let code = textweaver_mqtt::JoinCode::from_pasted(typed)
                .map_err(|_| self.msg("live-form-join-bad"))?;
            let password =
                textweaver_components::credentials::Password::new(code.password().expose());
            Ok((code.pin().grouped(), password))
        }
        #[cfg(not(feature = "mqtt"))]
        {
            Err(self.msg("live-not-in-build"))
        }
    }

    /// Enter in the form: checks every field, then saves the source, asks
    /// for its password when it signs in with a user name, and offers to
    /// open it.
    pub(crate) fn answer_live_form(&mut self, text: &str) -> Vec<Effect> {
        let Some(mut form) = self.live_wire.form.take() else {
            return vec![Effect::Redraw];
        };
        self.live_form_keep(&mut form, text.to_owned());
        let (source, join_password) = match self.live_form_check(&form) {
            Checked::Ok(s, p) => (*s, p),
            Checked::Bad(field, why) => {
                self.error(&why);
                form.focus = field;
                self.live_wire.form = Some(form);
                return self.live_form_prompt(false);
            }
        };
        self.leave_prompt();
        let (id, was) = match &form.origin {
            FormOrigin::Edit { id, .. } => {
                let was = self.settings.live.sources.get(id).cloned();
                // A port written out that is the default is no change.
                let same = was.as_ref().is_some_and(|w| {
                    let mut w = w.clone();
                    if w.port == Some(default_port(w.tls)) {
                        w.port = None;
                    }
                    w == source
                });
                if same {
                    let name = source.display_name(id).to_owned();
                    let m = self.msg_args("live-form-unchanged", &args!["name" => name]);
                    self.tell(&m);
                    return self.live_form_closed(&form);
                }
                (id.clone(), was)
            }
            FormOrigin::New | FormOrigin::File { .. } => {
                (self.live_new_source_id(&source.name), None)
            }
        };
        let name = source.display_name(&id).to_owned();
        let same_name = was.is_none()
            && self
                .settings
                .live
                .sources
                .iter()
                .any(|(other, s)| s.display_name(other).eq_ignore_ascii_case(&name));
        let user_changed = was
            .as_ref()
            .is_none_or(|w| w.username.trim() != source.username);
        let signs_in = !source.username.is_empty();
        let kept = source.clone();
        if let Err(e) = self.update_settings(|s| {
            s.live.sources.insert(id.clone(), kept);
        }) {
            let m = self.msg_args(
                "live-setup-not-saved",
                &args!["name" => name.as_str(), "error" => e.to_string()],
            );
            self.error(&m);
            return vec![Effect::Redraw];
        }
        let saved_id = if same_name {
            "live-form-saved-beside"
        } else {
            "live-setup-saved"
        };
        let m = self.msg_args(saved_id, &args!["name" => name.as_str()]);
        self.tell(&m);
        if let Some(password) = join_password {
            // Joined by code: the derived password, never the code.
            self.keep_live_password(&id, password);
            if form.from_link {
                // A pasted link means "join": no question first.
                return self.open_configured_live_source(&id);
            }
            return self.live_offer_open(&id);
        }
        if signs_in && (user_changed || self.live_password(&id).is_none()) {
            let effects = self.ask_live_password(&id);
            if self.prompt_is(PromptPurpose::LivePassword) {
                self.live_wire.offer_after_password = Some(id);
                return effects;
            }
        }
        self.live_offer_open(&id)
    }

    /// Escape in the form: nothing is saved, and the owner hears so.
    pub(crate) fn cancel_live_form(&mut self) -> Vec<Effect> {
        let Some(form) = self.live_wire.form.take() else {
            return vec![Effect::Redraw];
        };
        let m = match &form.origin {
            FormOrigin::Edit { id, .. } => {
                let name = self.live_display_name(id);
                self.msg_args("live-form-cancelled-edit", &args!["name" => name])
            }
            FormOrigin::New | FormOrigin::File { .. } => self.msg("live-form-cancelled"),
        };
        self.leave_prompt();
        self.tell(&m);
        self.live_form_closed(&form)
    }

    /// The form closed without saving: the Open live source list it came
    /// from is shown again, on its row.
    fn live_form_closed(&mut self, form: &LiveForm) -> Vec<Effect> {
        self.leave_prompt();
        match form.origin {
            FormOrigin::Edit { back: Some(n), .. } => {
                let effects = self.live_open_list();
                if self.list_is_live_open() {
                    self.pending_list_focus = Some(n);
                }
                effects
            }
            _ => vec![Effect::Redraw],
        }
    }

    fn list_is_live_open(&self) -> bool {
        matches!(self.list, Some(ListKind::Live(LiveList::Open(_))))
    }

    /// The source is saved: open it now, or later.
    pub(crate) fn live_offer_open(&mut self, id: &str) -> Vec<Effect> {
        let name = self.live_display_name(id);
        let items = vec![
            self.msg("live-setup-open-now"),
            self.msg("live-setup-later"),
        ];
        let title = self.msg_args("live-setup-offer-title", &args!["name" => name.as_str()]);
        self.list = Some(ListKind::Live(LiveList::SetupOffer(id.to_owned())));
        vec![Effect::ShowList { title, items }]
    }

    // ----- Join links ---------------------------------------------------

    /// Text pasted into the form's focused field, which held `before` and
    /// holds `whole` now (`pasted` is what came in, line breaks and all):
    /// a join link fills the form, and the join code field keeps only the
    /// code. `None` when the paste is ordinary text, typed on as usual.
    pub(crate) fn live_form_pasted(
        &mut self,
        before: &str,
        whole: &str,
        pasted: &str,
    ) -> Option<Vec<Effect>> {
        #[cfg(feature = "mqtt")]
        {
            use textweaver_mqtt::{JoinCode, JoinLink};
            let focus = self.live_wire.form.as_ref()?.focus;
            let source = [pasted, whole].into_iter().find(|t| JoinLink::is_in(t));
            if let Some(found) = source.and_then(JoinLink::find) {
                return Some(match found {
                    Ok(link) => self.live_form_fill(&link, before),
                    Err(reason) => {
                        // The link is not kept in the field, and not said.
                        if let Some(form) = self.live_wire.form.as_mut() {
                            form.set(focus, before.to_owned());
                        }
                        // "Not a join link: the port is ...", not "Not a
                        // join link: Join link error: the port is ...".
                        let reason = ["Join link error: ", "Join code error: "]
                            .iter()
                            .find_map(|p| reason.strip_prefix(p))
                            .unwrap_or(&reason)
                            .to_owned();
                        let m = self.msg_args("live-join-link-bad", &args!["reason" => reason]);
                        self.error(&m);
                        self.live_form_prompt(false)
                    }
                });
            }
            if focus != FormField::JoinCode {
                return None;
            }
            let code = JoinCode::from_pasted(pasted)
                .or_else(|_| JoinCode::from_pasted(whole))
                .ok()?;
            if let Some(form) = self.live_wire.form.as_mut() {
                form.set(FormField::JoinCode, code.grouped());
            }
            let m = self.msg("live-join-code-pasted");
            self.tell(&m);
            Some(self.live_form_prompt(false))
        }
        #[cfg(not(feature = "mqtt"))]
        {
            let _ = (before, whole, pasted);
            None
        }
    }

    /// Fills the open form from `link`: host, port, Join code, the code,
    /// the topic it names, and the name (the host) when empty. The
    /// focused field gets back `before` unless the link fills it.
    #[cfg(feature = "mqtt")]
    fn live_form_fill(&mut self, link: &textweaver_mqtt::JoinLink, before: &str) -> Vec<Effect> {
        let join_word = self.tls_word(Protect::Join);
        let Some(mut form) = self.live_wire.form.take() else {
            return vec![Effect::Redraw];
        };
        let focus = form.focus;
        form.set(focus, before.to_owned());
        form.set(FormField::Tls, join_word);
        form.set(FormField::Host, link.host().to_owned());
        form.set(FormField::Port, link.port().to_string());
        form.set(FormField::JoinCode, link.code().grouped());
        form.set(FormField::Username, String::new());
        if let Some(topic) = link.topic() {
            form.set(FormField::Topic, topic.to_owned());
        }
        if form.value(FormField::Name).trim().is_empty() {
            form.set(FormField::Name, link.host().to_owned());
        }
        form.from_link = true;
        let fields = self.live_form_fields(&form);
        form.focus = if form.value(FormField::Topic).trim().is_empty() {
            FormField::Topic
        } else if fields.contains(&focus) {
            focus
        } else {
            FormField::JoinCode
        };
        let said = if link.topic().is_some() {
            "live-join-link-filled"
        } else {
            "live-join-link-filled-no-topic"
        };
        self.live_wire.form = Some(form);
        let m = self.msg(said);
        self.tell(&m);
        self.live_form_prompt(false)
    }

    /// Open a join link: the form, filled from the join link on the
    /// clipboard (the system's, or the text last copied in textweaver);
    /// with none there, empty, to paste one into.
    pub fn live_open_join_link(&mut self) -> Vec<Effect> {
        let clip = self
            .system_clipboard
            .as_mut()
            .and_then(|c| c.read().text)
            .filter(|t| !t.trim().is_empty())
            .or_else(|| self.authoring.copied.clone())
            .unwrap_or_default();
        #[cfg(feature = "mqtt")]
        if let Some(Ok(link)) = textweaver_mqtt::JoinLink::find(&clip) {
            let intro = self.msg("live-form-intro-link-clipboard");
            let _ =
                self.open_live_form(LiveSourceSettings::default(), FormOrigin::New, &intro, None);
            return self.live_form_fill(&link, "");
        }
        let _ = clip;
        let intro = self.msg("live-form-intro-link");
        let mut effects =
            self.open_live_form(LiveSourceSettings::default(), FormOrigin::New, &intro, None);
        if let Some(form) = self.live_wire.form.as_mut() {
            form.from_link = true;
        }
        effects.push(Effect::Redraw);
        effects
    }

    // ----- Live source files --------------------------------------------

    /// Opening `path`: a live source file opens the form, a JSON log
    /// saved from a live document opens in the log view. `None` for any
    /// other file.
    pub(crate) fn open_live_file(&mut self, path: &Path) -> Option<Vec<Effect>> {
        if let Some(effects) = self.open_live_source_file(path) {
            return Some(effects);
        }
        self.open_live_log(path)
    }

    /// Opening `path`: a live source file (`.twlive`) opens the form
    /// filled from it. `None` for any other file.
    pub(crate) fn open_live_source_file(&mut self, path: &Path) -> Option<Vec<Effect>> {
        let ext = path.extension()?.to_string_lossy().to_lowercase();
        if ext != LIVE_SOURCE_FILE_EXTENSION {
            return None;
        }
        if !cfg!(feature = "mqtt") {
            return Some(self.live_not_in_build());
        }
        let file = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let file = textweaver_store::live_source_file::clean_live_text(&file);
        let read = read_capped(path).and_then(|bytes| {
            textweaver_store::read_live_source_file(&bytes, path.parent())
                .map_err(ReadFailure::File)
        });
        let parsed = match read {
            Ok(p) => p,
            Err(failure) => {
                let reason = match failure {
                    ReadFailure::Io(e) => e.to_string(),
                    ReadFailure::File(e) => self.live_file_error(&e),
                };
                let m = self.msg_args(
                    "live-file-not-read",
                    &args!["file" => file.as_str(), "reason" => reason],
                );
                self.error(&m);
                return Some(vec![Effect::Redraw]);
            }
        };
        let left_out = self.live_file_notes(&parsed.notes);
        let intro = self.msg_args("live-form-intro-file", &args!["file" => file.as_str()]);
        Some(self.open_live_form(parsed.source, FormOrigin::File { file }, &intro, left_out))
    }

    /// Why a live source file was refused, in words.
    fn live_file_error(&self, e: &LiveFileError) -> String {
        match e {
            LiveFileError::TooLarge { max, .. } => {
                self.msg_args("live-file-too-large", &args!["max" => *max])
            }
            LiveFileError::NotText => self.msg("live-file-not-text"),
            LiveFileError::NotToml { line } => {
                self.msg_args("live-file-not-toml", &args!["line" => *line])
            }
        }
    }

    /// What a live source file left out, as one message, or `None`.
    fn live_file_notes(&self, notes: &[LiveFileNote]) -> Option<String> {
        if notes.is_empty() {
            return None;
        }
        // The password first: it matters most.
        let mut parts = Vec::new();
        if notes.contains(&LiveFileNote::Password) {
            parts.push(self.msg("live-file-note-password"));
        }
        for n in notes {
            match n {
                LiveFileNote::Password => {}
                LiveFileNote::JoinCode => parts.push(self.msg("live-file-note-join-code")),
                LiveFileNote::UnknownKey(key) => {
                    parts.push(
                        self.msg_args("live-file-note-unknown", &args!["key" => key.as_str()]),
                    );
                }
                LiveFileNote::BadValue(detail) => parts.push(detail.clone()),
                LiveFileNote::TooMany { key, max } => parts.push(self.msg_args(
                    "live-file-note-too-many",
                    &args!["key" => key.as_str(), "max" => *max],
                )),
            }
        }
        Some(self.msg_args("live-file-left-out", &args!["items" => parts.join("; ")]))
    }

    /// Save live source as file: the open source's, or, with none open, a
    /// chosen source's; then asks where.
    pub fn live_save_file_command(&mut self) -> Vec<Effect> {
        if let Some(id) = self.open_configured_id() {
            return self.live_save_file_prompt(&id);
        }
        self.live_choose_list(LiveList::SaveFile, "live-save-file-title")
    }

    /// Asks where to write source `id` as a file.
    pub(crate) fn live_save_file_prompt(&mut self, id: &str) -> Vec<Effect> {
        if !self.settings.live.sources.contains_key(id) {
            let m = self.msg_args("live-source-unknown", &args!["source" => id]);
            self.error(&m);
            return vec![Effect::Redraw];
        }
        self.live_wire.save_file = Some(id.to_owned());
        self.prompt(PromptPurpose::SaveLiveSourceFile)
    }

    /// The file name offered for the source being saved as a file.
    pub(crate) fn live_save_file_name(&self) -> Option<String> {
        let id = self.live_wire.save_file.as_deref()?;
        Some(format!("{id}.{LIVE_SOURCE_FILE_EXTENSION}"))
    }

    /// The answer to Save live source as file: the source is written there,
    /// with `.twlive` added when the name has no extension. Never with a
    /// password.
    pub(crate) fn answer_live_save_file(&mut self, text: &str) -> Vec<Effect> {
        let Some(id) = self.live_wire.save_file.take() else {
            return vec![Effect::Redraw];
        };
        let typed = text.trim().trim_matches('"');
        if typed.is_empty() {
            let m = self.msg("common-cancelled");
            self.note(&m);
            return vec![Effect::Redraw];
        }
        let mut path = PathBuf::from(typed);
        if path.extension().is_none() {
            path.set_extension(LIVE_SOURCE_FILE_EXTENSION);
        }
        let Some(source) = self.settings.live.sources.get(&id) else {
            return vec![Effect::Redraw];
        };
        let name = source.display_name(&id).to_owned();
        let body = textweaver_store::live_source_file_text(source, path.parent());
        match textweaver_store::atomic_write(&path, body.as_bytes()) {
            Ok(()) => {
                let m = self.msg_args(
                    "live-save-file-done",
                    &args!["name" => name, "path" => path.display().to_string()],
                );
                self.tell(&m);
            }
            Err(e) => {
                let m = self.msg_args(
                    "live-save-file-failed",
                    &args!["name" => name, "error" => e.to_string()],
                );
                self.error(&m);
            }
        }
        vec![Effect::Redraw]
    }
}

/// Why a live source file was not read.
enum ReadFailure {
    Io(std::io::Error),
    File(LiveFileError),
}

/// The bytes of `path`, refused past the size cap before they are read.
fn read_capped(path: &Path) -> Result<Vec<u8>, ReadFailure> {
    let len = std::fs::metadata(path).map_err(ReadFailure::Io)?.len();
    if len > LIVE_SOURCE_FILE_MAX as u64 {
        return Err(ReadFailure::File(LiveFileError::TooLarge {
            len: usize::try_from(len).unwrap_or(usize::MAX),
            max: LIVE_SOURCE_FILE_MAX,
        }));
    }
    std::fs::read(path).map_err(ReadFailure::Io)
}
