//! Live sources' commands (beta 5, N6): open a configured source, set one
//! up, close it, and the commands that work on the open live log.
//!
//! - **Open live source** lists `[live.sources]` by name; Enter opens one
//!   ([`App::open_configured_live_source`]). Opening says the source's
//!   problems in words, if it has any
//!   ([`LiveSourceSettings::problems`]); asks for the password when the
//!   source has a user name and none is kept, and connects once it is
//!   answered; then connects and shows the log.
//! - **Set up a live source** asks one thing at a time: the name, the
//!   broker's host, the encryption (a list), the certificate authority
//!   file when one is chosen, the port (the default for the encryption
//!   filled in), the topic, the user name, and the view (a list). It
//!   writes `[live.sources.<id>]` through the settings update path, then
//!   offers to open the source.
//! - **Close live source** stops receiving; the log stays open, read
//!   only, and is saved.
//! - **Switch list and table**, **Jump to newest**, **Pause or resume
//!   updates**, **Say unread**, **Save live log now**: the open log's
//!   commands (crate::live::hub).
//! - **Speak new entries** turns `[live] speak_new_entries` on or off.
//! - **Forget live source password** removes the open source's password
//!   (or a chosen source's) from the system credential store.
//!
//! Without the `mqtt` feature, the commands that open, set up, or close a
//! source, and Forget, are hidden from the menus and the palette
//! ([`crate::menu::PENDING`]); run by a key, they say in words that live
//! sources are not in this build.

use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_store::{LiveSourceSettings, LiveTls, LiveView, live_source_id_ok};

use super::announce::EntryThrottle;
use crate::app::{App, ListKind};
use crate::command::{Effect, PromptPurpose};

/// The commands that need the MQTT client: hidden without the `mqtt`
/// feature.
pub const NEEDS_MQTT: &[ActionId] = &[
    ActionId::OpenLiveSource,
    ActionId::SetUpLiveSource,
    ActionId::CloseLiveSource,
    ActionId::ForgetLiveSourcePassword,
];

/// Every live source command.
pub const COMMANDS: &[ActionId] = &[
    ActionId::OpenLiveSource,
    ActionId::SetUpLiveSource,
    ActionId::CloseLiveSource,
    ActionId::LiveSwitchView,
    ActionId::LiveJumpNewest,
    ActionId::LiveTogglePause,
    ActionId::LiveToggleSpeak,
    ActionId::LiveSayUnread,
    ActionId::SaveLiveLog,
    ActionId::ForgetLiveSourcePassword,
];

/// Connects a live source in place of the MQTT client: tests give one so
/// nothing reaches a network.
#[cfg(feature = "mqtt")]
pub type Connector = Box<
    dyn FnMut(
            textweaver_mqtt::MqttConfig,
            Box<dyn Fn() + Send + Sync>,
        ) -> Box<dyn super::LiveSource>
        + Send,
>;

/// The live commands' state.
#[derive(Default)]
pub(crate) struct LiveWire {
    /// New entries and field changes waiting to be said
    /// (crate::live::announce).
    pub(crate) throttle: EntryThrottle,
    /// The source the open password prompt was asked for by Open: once
    /// answered, it connects.
    open_after_password: Option<String>,
    /// Set up a live source, while it runs.
    setup: Option<Setup>,
    /// Stands in for the MQTT client (tests).
    #[cfg(feature = "mqtt")]
    connector: Option<Connector>,
}

impl std::fmt::Debug for LiveWire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveWire")
            .field("throttle", &self.throttle)
            .field("open_after_password", &self.open_after_password)
            .field("setup", &self.setup)
            .finish_non_exhaustive()
    }
}

/// What a live list is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LiveList {
    /// Open live source: the source ids, by row.
    Open(Vec<String>),
    /// Forget a password: the source ids, by row.
    Forget(Vec<String>),
    /// Set up: the encryption, in the order of [`TLS_CHOICES`].
    SetupTls,
    /// Set up: the view, in the order of [`VIEW_CHOICES`].
    SetupView,
    /// Set up is done: open source `id` now, or not.
    SetupOffer(String),
}

/// The encryption choices, in list order.
const TLS_CHOICES: [LiveTls; 3] = [LiveTls::Off, LiveTls::System, LiveTls::CaFile];

/// The view choices, in list order.
const VIEW_CHOICES: [LiveView; 2] = [LiveView::Log, LiveView::Values];

/// The step Set up a live source is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Name,
    Host,
    CaFile,
    Port,
    Topic,
    Username,
}

/// Set up a live source's answers so far.
#[derive(Clone, Debug)]
struct Setup {
    step: Step,
    source: LiveSourceSettings,
}

impl App {
    /// Runs a live source command.
    pub(crate) fn live_action(&mut self, a: ActionId) -> Vec<Effect> {
        match a {
            ActionId::OpenLiveSource
            | ActionId::SetUpLiveSource
            | ActionId::CloseLiveSource
            | ActionId::ForgetLiveSourcePassword => self.run_registered(a),
            ActionId::LiveSwitchView => self.live_toggle_view(),
            ActionId::LiveJumpNewest => self.live_jump_newest(),
            ActionId::LiveTogglePause => self.live_toggle_pause(),
            ActionId::LiveToggleSpeak => self.live_toggle_speak(),
            ActionId::LiveSayUnread => self.live_say_unread(),
            ActionId::SaveLiveLog => self.live_save_log(),
            _ => vec![Effect::Redraw],
        }
    }

    /// Says that live sources are not in this build (a lean build without
    /// the `mqtt` feature).
    pub(crate) fn live_not_in_build(&mut self) -> Vec<Effect> {
        let m = self.msg("live-not-in-build");
        self.error(&m);
        vec![Effect::Redraw]
    }

    /// Speak new entries: turns `[live] speak_new_entries` on or off, and
    /// says which.
    pub fn live_toggle_speak(&mut self) -> Vec<Effect> {
        let on = !self.settings.live.speak_new_entries;
        self.settings.live.speak_new_entries = on;
        self.settings_dirty = true;
        let m = self.msg(if on {
            "live-speak-on"
        } else {
            "live-speak-off"
        });
        self.tell(&m);
        vec![Effect::Redraw]
    }

    /// Say unread: how many entries are unread, and the newest one whole.
    pub fn live_say_unread(&mut self) -> Vec<Effect> {
        let Some(unread) = self.live_unread() else {
            let m = self.msg("live-not-open");
            self.tell(&m);
            return vec![Effect::Redraw];
        };
        let count = self.msg_args("live-unread", &args!["count" => unread]);
        let m = match self.live_newest_line() {
            Some(line) => self.msg_args(
                "live-unread-newest",
                &args!["count" => count, "line" => line],
            ),
            None => self.msg("live-newest-none"),
        };
        self.tell(&m);
        vec![Effect::Redraw]
    }

    /// The live list's answer.
    pub(crate) fn choose_live(&mut self, list: LiveList, n: usize) -> Vec<Effect> {
        match list {
            LiveList::Open(ids) => match ids.get(n) {
                Some(id) => self.open_configured_live_source(id),
                None => vec![Effect::Redraw],
            },
            LiveList::Forget(ids) => match ids.get(n) {
                Some(id) => self.forget_live_password(id),
                None => vec![Effect::Redraw],
            },
            LiveList::SetupTls => self.live_setup_tls(n),
            LiveList::SetupView => self.live_setup_view(n),
            LiveList::SetupOffer(id) => {
                if n == 0 {
                    self.open_configured_live_source(&id)
                } else {
                    let m = self.msg("live-setup-not-now");
                    self.note(&m);
                    vec![Effect::Redraw]
                }
            }
        }
    }

    /// The configured sources' ids, in id order.
    fn live_source_ids(&self) -> Vec<String> {
        self.settings.live.sources.keys().cloned().collect()
    }

    /// Open live source: the configured sources as a list; Enter opens
    /// one. With none set up, says how to add one.
    pub fn live_open_list(&mut self) -> Vec<Effect> {
        let ids = self.live_source_ids();
        if ids.is_empty() {
            let m = self.msg("live-none-set-up");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        let items = ids
            .iter()
            .map(|id| {
                let s = &self.settings.live.sources[id];
                let topics = s.topics.join(", ");
                self.msg_args(
                    "live-source-item",
                    &args![
                        "name" => s.display_name(id),
                        "host" => s.host.trim(),
                        "topic" => topics,
                    ],
                )
            })
            .collect();
        let title = self.msg("live-open-title");
        self.list = Some(ListKind::Live(LiveList::Open(ids)));
        vec![Effect::ShowList { title, items }]
    }

    /// Forget live source password: the open source's, or, with none
    /// open, a chosen source's (from those with a user name).
    pub fn live_forget_command(&mut self) -> Vec<Effect> {
        let open = self
            .session
            .as_ref()
            .and_then(|s| s.live.as_ref())
            .map(|t| t.source.clone())
            .filter(|id| self.settings.live.sources.contains_key(id));
        if let Some(id) = open {
            return self.forget_live_password(&id);
        }
        let ids: Vec<String> = self
            .settings
            .live
            .sources
            .iter()
            .filter(|(_, s)| !s.username.trim().is_empty())
            .map(|(id, _)| id.clone())
            .collect();
        if ids.is_empty() {
            let m = self.msg("live-no-passwords");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        let items = ids
            .iter()
            .map(|id| self.settings.live.sources[id].display_name(id).to_owned())
            .collect();
        let title = self.msg("live-forget-title");
        self.list = Some(ListKind::Live(LiveList::Forget(ids)));
        vec![Effect::ShowList { title, items }]
    }

    /// Close live source: stops receiving; the log stays open, read only.
    pub fn live_close_command(&mut self) -> Vec<Effect> {
        if !self.live_receiving() {
            let m = self.msg("live-not-receiving");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        self.live_close()
    }

    /// Opens configured live source `id`: says its problems if it has
    /// any; asks for its password when it has a user name and none is
    /// kept, and connects after the answer; else connects and shows the
    /// log, empty until the first message.
    pub fn open_configured_live_source(&mut self, id: &str) -> Vec<Effect> {
        let Some(source) = self.settings.live.sources.get(id).cloned() else {
            let m = self.msg_args("live-source-unknown", &args!["source" => id]);
            self.error(&m);
            return vec![Effect::Redraw];
        };
        if !cfg!(feature = "mqtt") {
            return self.live_not_in_build();
        }
        let problems = source.problems(id);
        if !problems.is_empty() {
            let m = self.msg_args("live-cannot-open", &args!["problems" => problems.join(" ")]);
            self.error(&m);
            return vec![Effect::Redraw];
        }
        let password = if source.username.trim().is_empty() {
            None
        } else {
            match self.live_password(id) {
                Some(p) => Some(p),
                None => {
                    let effects = self.ask_live_password(id);
                    if self.prompt_is(PromptPurpose::LivePassword) {
                        self.live_wire.open_after_password = Some(id.to_owned());
                    }
                    return effects;
                }
            }
        };
        self.live_connect(id, &source, password)
    }

    /// The password prompt's answer: kept as Forget and the store expect
    /// (crate::source_passwords); when Open asked for it, the source
    /// connects now, or, cancelled, is not opened.
    pub(crate) fn answer_live_password_then_open(&mut self, text: &str) -> Vec<Effect> {
        let pending = self.live_wire.open_after_password.take();
        let mut effects = self.answer_live_password(text);
        let Some(id) = pending else {
            return effects;
        };
        if self.prompt_is(PromptPurpose::LivePassword) {
            // Asked again (not a password): still pending.
            self.live_wire.open_after_password = Some(id);
            return effects;
        }
        if text.is_empty() {
            let name = self.live_display_name(&id);
            let m = self.msg_args("live-not-opened", &args!["name" => name]);
            self.note(&m);
            return effects;
        }
        effects.extend(self.open_configured_live_source(&id));
        effects
    }

    /// True while the prompt for `purpose` is open.
    fn prompt_is(&self, purpose: PromptPurpose) -> bool {
        self.mode.is_prompt() && self.prompt_purpose == purpose
    }

    fn live_display_name(&self, id: &str) -> String {
        self.settings
            .live
            .sources
            .get(id)
            .map_or(id, |s| s.display_name(id))
            .to_owned()
    }

    #[cfg(feature = "mqtt")]
    fn live_connect(
        &mut self,
        id: &str,
        source: &LiveSourceSettings,
        password: Option<textweaver_components::credentials::Password>,
    ) -> Vec<Effect> {
        let config = super::mqtt::mqtt_config(&self.settings.live, source, password.as_ref());
        let wake = self.live_waker();
        let live: Box<dyn super::LiveSource> = match self.live_wire.connector.as_mut() {
            Some(connect) => connect(config, wake),
            None => Box::new(super::mqtt::MqttLiveSource::start(config, wake)),
        };
        let id_field = source.id_field.trim();
        let options = super::LiveOptions {
            id: id.to_owned(),
            title: source.display_name(id).to_owned(),
            id_field: (!id_field.is_empty()).then(|| id_field.to_owned()),
            view: super::LogView::List,
        };
        self.live_wire.throttle.clear();
        self.open_live_source(live, options)
    }

    #[cfg(not(feature = "mqtt"))]
    fn live_connect(
        &mut self,
        _id: &str,
        _source: &LiveSourceSettings,
        _password: Option<textweaver_components::credentials::Password>,
    ) -> Vec<Effect> {
        self.live_not_in_build()
    }

    /// Connects live sources through `connect` instead of the MQTT client,
    /// so a test reaches no network.
    #[cfg(feature = "mqtt")]
    #[doc(hidden)]
    pub fn set_live_connector(&mut self, connect: Connector) {
        self.live_wire.connector = Some(connect);
    }

    // ----- Set up a live source -------------------------------------------

    /// Set up a live source: asks for its name first.
    pub fn live_setup(&mut self) -> Vec<Effect> {
        self.live_wire.setup = Some(Setup {
            step: Step::Name,
            source: LiveSourceSettings::default(),
        });
        self.live_setup_ask(Step::Name)
    }

    /// The label of the Set up prompt now open.
    pub(crate) fn live_setup_label(&self, purpose: PromptPurpose) -> Option<String> {
        if purpose != PromptPurpose::LiveSetup {
            return None;
        }
        let setup = self.live_wire.setup.as_ref()?;
        Some(match setup.step {
            Step::Name => self.msg("live-setup-name"),
            Step::Host => self.msg("live-setup-host"),
            Step::CaFile => self.msg("live-setup-ca-file"),
            Step::Port => self.msg("live-setup-port"),
            Step::Topic => self.msg("live-setup-topic"),
            Step::Username => self.msg("live-setup-username"),
        })
    }

    fn live_setup_ask(&mut self, step: Step) -> Vec<Effect> {
        let Some(setup) = self.live_wire.setup.as_mut() else {
            return vec![Effect::Redraw];
        };
        setup.step = step;
        if step == Step::Port {
            self.pending_prompt_text = Some(setup.source.port_or_default().to_string());
        }
        self.prompt(PromptPurpose::LiveSetup)
    }

    /// An answer in Set up a live source.
    pub(crate) fn answer_live_setup(&mut self, text: &str) -> Vec<Effect> {
        let Some(step) = self.live_wire.setup.as_ref().map(|s| s.step) else {
            return vec![Effect::Redraw];
        };
        let text = text.trim();
        match step {
            Step::Name => {
                if text.is_empty() {
                    self.live_wire.setup = None;
                    let m = self.msg("common-cancelled");
                    self.note(&m);
                    return vec![Effect::Redraw];
                }
                self.live_setup_source().name = text.to_owned();
                self.live_setup_ask(Step::Host)
            }
            Step::Host => {
                if text.is_empty() || text.chars().any(char::is_whitespace) {
                    return self.live_setup_again("live-setup-host-needed", step);
                }
                self.live_setup_source().host = text.to_owned();
                self.live_setup_tls_list()
            }
            Step::CaFile => {
                if text.is_empty() {
                    return self.live_setup_again("live-setup-ca-file-needed", step);
                }
                if !std::path::Path::new(text).is_file() {
                    let m = self.msg_args("live-setup-ca-file-missing", &args!["path" => text]);
                    self.error(&m);
                    return self.live_setup_ask(step);
                }
                self.live_setup_source().ca_file = text.to_owned();
                self.live_setup_ask(Step::Port)
            }
            Step::Port => {
                let tls = self
                    .live_wire
                    .setup
                    .as_ref()
                    .map_or(LiveTls::Off, |s| s.source.tls);
                let default = LiveSourceSettings {
                    tls,
                    ..LiveSourceSettings::default()
                }
                .port_or_default();
                let port = if text.is_empty() {
                    Some(default)
                } else {
                    text.parse::<u16>().ok().filter(|p| *p > 0)
                };
                let Some(port) = port else {
                    return self.live_setup_again("live-setup-port-bad", step);
                };
                self.live_setup_source().port = (port != default).then_some(port);
                self.live_setup_ask(Step::Topic)
            }
            Step::Topic => {
                if text.is_empty() {
                    return self.live_setup_again("live-setup-topic-needed", step);
                }
                self.live_setup_source().topics = vec![text.to_owned()];
                self.live_setup_ask(Step::Username)
            }
            Step::Username => {
                self.live_setup_source().username = text.to_owned();
                self.live_setup_view_list()
            }
        }
    }

    fn live_setup_source(&mut self) -> &mut LiveSourceSettings {
        &mut self
            .live_wire
            .setup
            .get_or_insert_with(|| Setup {
                step: Step::Name,
                source: LiveSourceSettings::default(),
            })
            .source
    }

    /// Says `key` as an error and asks `step` again.
    fn live_setup_again(&mut self, key: &str, step: Step) -> Vec<Effect> {
        let m = self.msg(key);
        self.error(&m);
        self.live_setup_ask(step)
    }

    fn live_setup_tls_list(&mut self) -> Vec<Effect> {
        let items = TLS_CHOICES
            .iter()
            .map(|t| self.msg(&format!("live-setup-tls-{}", t.as_str().replace('_', "-"))))
            .collect();
        let title = self.msg("live-setup-tls-title");
        self.list = Some(ListKind::Live(LiveList::SetupTls));
        vec![Effect::ShowList { title, items }]
    }

    fn live_setup_tls(&mut self, n: usize) -> Vec<Effect> {
        let Some(&tls) = TLS_CHOICES.get(n) else {
            return vec![Effect::Redraw];
        };
        if self.live_wire.setup.is_none() {
            return vec![Effect::Redraw];
        }
        self.live_setup_source().tls = tls;
        if tls == LiveTls::CaFile {
            self.live_setup_ask(Step::CaFile)
        } else {
            self.live_setup_ask(Step::Port)
        }
    }

    fn live_setup_view_list(&mut self) -> Vec<Effect> {
        let items = VIEW_CHOICES
            .iter()
            .map(|v| self.msg(&format!("live-setup-view-{}", v.as_str())))
            .collect();
        let title = self.msg("live-setup-view-title");
        self.list = Some(ListKind::Live(LiveList::SetupView));
        vec![Effect::ShowList { title, items }]
    }

    /// The view is the last answer: the source is written to the settings
    /// and offered for opening.
    fn live_setup_view(&mut self, n: usize) -> Vec<Effect> {
        let Some(&view) = VIEW_CHOICES.get(n) else {
            return vec![Effect::Redraw];
        };
        let Some(mut setup) = self.live_wire.setup.take() else {
            return vec![Effect::Redraw];
        };
        setup.source.view = view;
        let id = self.live_new_source_id(&setup.source.name);
        let name = setup.source.display_name(&id).to_owned();
        let source = setup.source;
        if let Err(e) = self.update_settings(|s| {
            s.live.sources.insert(id.clone(), source);
        }) {
            let m = self.msg_args(
                "live-setup-not-saved",
                &args!["name" => name.as_str(), "error" => e.to_string()],
            );
            self.error(&m);
            return vec![Effect::Redraw];
        }
        let m = self.msg_args("live-setup-saved", &args!["name" => name.as_str()]);
        self.tell(&m);
        let items = vec![
            self.msg("live-setup-open-now"),
            self.msg("live-setup-later"),
        ];
        let title = self.msg_args("live-setup-offer-title", &args!["name" => name.as_str()]);
        self.list = Some(ListKind::Live(LiveList::SetupOffer(id)));
        vec![Effect::ShowList { title, items }]
    }

    /// A new source's id from its name: lowercase letters and digits, with
    /// hyphens between words, at most 48 characters; `source` when the
    /// name has none; a number added when the id is taken.
    fn live_new_source_id(&self, name: &str) -> String {
        let mut base = String::new();
        for c in name.chars().flat_map(char::to_lowercase) {
            if c.is_ascii_alphanumeric() {
                base.push(c);
            } else if !base.is_empty() && !base.ends_with('-') {
                base.push('-');
            }
        }
        let mut base: String = base.trim_end_matches('-').chars().take(48).collect();
        base = base.trim_end_matches('-').to_owned();
        if base.is_empty() {
            base = "source".into();
        }
        let taken = |id: &str| self.settings.live.sources.contains_key(id);
        let mut id = base.clone();
        let mut n = 2;
        while taken(&id) || !live_source_id_ok(&id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }
}

/// Gives the commands that need the MQTT client their handlers (with the
/// `mqtt` feature); without it they stay hidden ([`NEEDS_MQTT`]).
pub(crate) fn register(app: &mut App) {
    #[cfg(feature = "mqtt")]
    {
        app.register_handler(ActionId::OpenLiveSource, |app| app.live_open_list());
        app.register_handler(ActionId::SetUpLiveSource, |app| app.live_setup());
        app.register_handler(ActionId::CloseLiveSource, |app| app.live_close_command());
        app.register_handler(ActionId::ForgetLiveSourcePassword, |app| {
            app.live_forget_command()
        });
    }
    #[cfg(not(feature = "mqtt"))]
    let _ = app;
}

#[cfg(all(test, feature = "mqtt"))]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use textweaver_a11y::{AccessMode, LogAnnouncer, Priority, Shared};
    use textweaver_mqtt::{MqttConfig, MqttEvent, Payload, Secret};
    use textweaver_store::{InterfaceAnnouncements, LiveSourceSettings, Paths};

    use super::*;
    use crate::AppConfig;
    use crate::command::Command;
    use crate::list_model::PromptKey;
    use crate::live::BURST;
    use crate::live::mqtt::{MqttLiveSource, fake::Script};

    const PASSWORD: &str = "example only, not a real password";

    /// What the connector saw: each configuration, and the script that
    /// feeds the source it made.
    #[derive(Clone, Default)]
    struct Wire {
        configs: Arc<Mutex<Vec<MqttConfig>>>,
        script: Script,
        connects: Arc<AtomicUsize>,
    }

    impl Wire {
        fn connector(&self) -> Connector {
            let w = self.clone();
            Box::new(move |config, _wake| {
                w.connects.fetch_add(1, Ordering::SeqCst);
                w.configs.lock().unwrap().push(config);
                Box::new(MqttLiveSource::from_feed(Box::new(w.script.clone())))
            })
        }

        fn connects(&self) -> usize {
            self.connects.load(Ordering::SeqCst)
        }

        fn config(&self) -> MqttConfig {
            self.configs.lock().unwrap().last().cloned().unwrap()
        }
    }

    struct Rig {
        app: App,
        said: Shared<LogAnnouncer>,
        wire: Wire,
        _tmp: tempfile::TempDir,
    }

    impl Rig {
        fn texts(&self) -> Vec<String> {
            self.said
                .lock()
                .texts()
                .iter()
                .map(|s| s.to_string())
                .collect()
        }

        fn any(&self, needle: &str) -> bool {
            self.texts().iter().any(|t| t.contains(needle))
        }

        fn priority_of(&self, needle: &str) -> Option<Priority> {
            self.said
                .lock()
                .log
                .iter()
                .find(|(t, _)| t.contains(needle))
                .map(|(_, p)| *p)
        }

        fn clear(&self) {
            self.said.lock().log.clear();
        }

        fn type_answer(&mut self, text: &str) {
            for c in text.chars() {
                self.app.dispatch(Command::PromptKey(PromptKey::Char(c)));
            }
            self.app.dispatch(Command::PromptKey(PromptKey::Enter));
        }

        fn tick(&mut self, now: Instant) {
            self.app.tick(now);
        }

        fn label(&self) -> String {
            self.app
                .prompt_model()
                .map(|p| p.label.clone())
                .unwrap_or_default()
        }
    }

    /// The app with live sources `ids` (each with `edit`), the memory
    /// credential store, a recording announcer, and a connector that
    /// reaches no network.
    fn rig(sources: &[(&str, LiveSourceSettings)]) -> Rig {
        textweaver_components::fake::memory_credentials();
        let tmp = tempfile::tempdir().unwrap();
        let said = Shared::new(LogAnnouncer::default());
        let mut app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            paths: Some(Paths::under(tmp.path())),
            ..AppConfig::for_tests()
        });
        let list: Vec<(String, LiveSourceSettings)> = sources
            .iter()
            .map(|(id, s)| ((*id).to_owned(), s.clone()))
            .collect();
        let _ = app.update_settings(|s| {
            for (id, src) in list {
                s.live.sources.insert(id, src);
            }
        });
        let wire = Wire::default();
        app.set_live_connector(wire.connector());
        Rig {
            app,
            said,
            wire,
            _tmp: tmp,
        }
    }

    /// Closes the list or prompt shown, if any.
    fn escape(app: &mut App) {
        if app.prompt_model().is_some() {
            app.leave_prompt();
        }
        if app.list_model().is_some() {
            app.dispatch(Command::ListKey(crate::list_model::ListKey::Escape));
        }
    }

    fn practice(username: &str) -> LiveSourceSettings {
        LiveSourceSettings {
            name: "Practice log".into(),
            host: "logger.example.ts.net".into(),
            username: username.into(),
            topics: vec!["log/checkins".into()],
            ..LiveSourceSettings::default()
        }
    }

    fn checkin(call: &str, name: &str) -> MqttEvent {
        MqttEvent::Message {
            topic: "log/checkins".into(),
            payload: Payload::Json(serde_json::json!({"call": call, "name": name})),
            retained: false,
        }
    }

    #[test]
    fn a_source_with_problems_says_them_and_does_not_connect() {
        let mut r = rig(&[("broken", LiveSourceSettings::default())]);
        r.app.open_configured_live_source("broken");
        assert!(r.any("Cannot open the live source."), "{:?}", r.texts());
        assert!(r.any("has no host"), "{:?}", r.texts());
        assert!(r.any("has no topic"), "{:?}", r.texts());
        assert_eq!(r.wire.connects(), 0);
        r.app.open_configured_live_source("nowhere");
        assert!(r.any("No live source named nowhere."));
    }

    #[test]
    fn the_password_is_asked_and_the_answer_goes_on_to_connect() {
        let mut r = rig(&[("pw-wire-a", practice("reader"))]);
        r.app.open_configured_live_source("pw-wire-a");
        assert_eq!(r.wire.connects(), 0, "not before the password");
        assert_eq!(r.label(), "Password for Practice log, Enter cancels");
        r.type_answer(PASSWORD);
        assert_eq!(r.wire.connects(), 1, "{:?}", r.texts());
        let c = r.wire.config();
        assert_eq!(c.host, "logger.example.ts.net");
        assert_eq!(c.port, 1883);
        assert_eq!(c.username.as_deref(), Some("reader"));
        assert_eq!(c.password.as_ref().map(Secret::expose), Some(PASSWORD));
        assert!(r.app.live_receiving());
        assert_eq!(r.app.session().unwrap().title, "Practice log");
        assert!(!r.texts().join(" ").contains("not a real password"));

        // The password is kept: the next open connects at once.
        r.app.live_close();
        r.app.open_configured_live_source("pw-wire-a");
        assert_eq!(r.wire.connects(), 2);
        assert!(r.app.prompt_model().is_none());
    }

    #[test]
    fn cancelling_the_password_does_not_connect() {
        let mut r = rig(&[("pw-wire-b", practice("reader"))]);
        r.app.open_configured_live_source("pw-wire-b");
        r.type_answer("");
        assert_eq!(r.wire.connects(), 0);
        assert!(
            r.any("Practice log not opened: no password."),
            "{:?}",
            r.texts()
        );
        assert!(!r.app.live_receiving());
    }

    #[test]
    fn connection_changes_are_said_and_only_a_loss_interrupts() {
        let mut r = rig(&[("plain", practice(""))]);
        r.app.open_configured_live_source("plain");
        assert_eq!(r.wire.connects(), 1);
        let s = r.wire.script.clone();
        let t0 = Instant::now();
        for e in [
            MqttEvent::Connecting { attempt: 1 },
            MqttEvent::Connected {
                encrypted: false,
                tailscale: false,
            },
            MqttEvent::Connected {
                encrypted: false,
                tailscale: true,
            },
            MqttEvent::Connected {
                encrypted: true,
                tailscale: false,
            },
            MqttEvent::TopicRefused {
                topic: "log/secret".into(),
            },
            MqttEvent::Quiet {
                topic: "log/checkins".into(),
                seconds: 60,
            },
            MqttEvent::Skipped { count: 3 },
            MqttEvent::Disconnected {
                reason: "Connection lost to logger.example.ts.net:1883.".into(),
                retrying: true,
            },
            MqttEvent::Disconnected {
                reason: "Not authorized: check the user name and password.".into(),
                retrying: false,
            },
        ] {
            s.push(e);
        }
        r.tick(t0);
        let polite = Some(Priority::Polite);
        let assertive = Some(Priority::Assertive);
        assert_eq!(r.priority_of("The connection is not encrypted."), polite);
        assert_eq!(
            r.priority_of("over Tailscale, which encrypts the link"),
            polite
        );
        assert_eq!(
            r.priority_of("Connected to Practice log, encrypted."),
            polite
        );
        assert_eq!(
            r.priority_of(
                "The broker refused the topic log/secret; it may not let this account read it."
            ),
            polite
        );
        assert_eq!(r.priority_of("No messages yet on log/checkins."), polite);
        assert_eq!(r.priority_of("3 messages were left out"), polite);
        assert_eq!(
            r.priority_of("Disconnected from Practice log: Connection lost"),
            assertive
        );
        assert_eq!(
            r.priority_of("Cannot connect to Practice log: Not authorized"),
            assertive
        );
        // Closing stops the client.
        r.app.dispatch(Command::Action(ActionId::CloseLiveSource));
        assert!(s.stopped());
    }

    #[test]
    fn new_entries_reach_the_screen_reader_whole_and_bursts_become_one_digest() {
        let mut r = rig(&[("plain", practice(""))]);
        r.app.open_configured_live_source("plain");
        r.clear();
        let s = r.wire.script.clone();
        let t0 = Instant::now();
        s.push(checkin("N0CALL", "Ada Example"));
        r.tick(t0);
        assert_eq!(r.priority_of("N0CALL"), Some(Priority::Polite));
        assert!(
            r.texts().iter().any(|t| t.starts_with("N0CALL")),
            "{:?}",
            r.texts()
        );
        r.clear();

        // A burst within two seconds waits, then goes as one message.
        for (i, call) in ["N1CALL", "N2CALL", "N3CALL"].iter().enumerate() {
            s.push(checkin(call, "Ada Example"));
            r.tick(t0 + Duration::from_millis(300 * (i as u64 + 1)));
        }
        assert!(!r.any("CALL"), "held: {:?}", r.texts());
        let wait = r.app.tick_interval(t0 + Duration::from_millis(900));
        assert!(wait <= Duration::from_millis(1100), "{wait:?}");
        r.tick(t0 + BURST);
        let texts = r.texts();
        assert_eq!(texts.len(), 1, "{texts:?}");
        assert!(
            texts[0].starts_with("3 new entries. Newest: N3CALL"),
            "{texts:?}"
        );
    }

    #[test]
    fn new_entries_wait_while_textweaver_reads_aloud() {
        let mut r = rig(&[("plain", practice(""))]);
        r.app.open_configured_live_source("plain");
        r.clear();
        let s = r.wire.script.clone();
        let t0 = Instant::now();
        r.app.playback = crate::playback::Playback::Reading;
        s.push(checkin("N4CALL", "Ada Example"));
        s.push(checkin("N5CALL", "Ada Example"));
        r.tick(t0);
        r.tick(t0 + Duration::from_secs(30));
        assert!(!r.any("CALL"), "{:?}", r.texts());
        r.app.playback = crate::playback::Playback::Idle;
        r.tick(t0 + Duration::from_secs(31));
        assert!(r.any("2 new entries. Newest: N5CALL"), "{:?}", r.texts());
    }

    #[test]
    fn announcing_off_says_nothing_and_no_interface_level_silences_an_entry() {
        let mut r = rig(&[("plain", practice(""))]);
        r.app.open_configured_live_source("plain");
        let s = r.wire.script.clone();
        let t0 = Instant::now();
        r.app.settings.accessibility.interface_announcements = InterfaceAnnouncements::Off;
        r.clear();
        s.push(checkin("N6CALL", "Ada Example"));
        r.tick(t0);
        assert!(r.any("N6CALL"), "{:?}", r.texts());
        r.app.settings.live.announce_new_entries = false;
        r.clear();
        s.push(checkin("N7CALL", "Ada Example"));
        r.tick(t0 + Duration::from_secs(5));
        assert!(!r.any("N7CALL"), "{:?}", r.texts());
    }

    /// textweaver's own voice says an entry only with speak_new_entries,
    /// and never in screen-reader mode; the screen reader gets it in every
    /// mode.
    #[test]
    fn the_voice_follows_speak_new_entries_in_each_access_mode() {
        let cases = [
            (AccessMode::SelfVoicing, false, false),
            (AccessMode::SelfVoicing, true, true),
            (AccessMode::Hybrid, false, false),
            (AccessMode::Hybrid, true, true),
            (AccessMode::ScreenReader, false, false),
            (AccessMode::ScreenReader, true, false),
        ];
        for (n, (mode, speak, spoken)) in cases.into_iter().enumerate() {
            textweaver_components::fake::memory_credentials();
            let (speech, log) = crate::testing::recording_service().unwrap();
            let said = Shared::new(LogAnnouncer::default());
            let mut app = App::new(AppConfig {
                speech,
                self_voicing: true,
                backend_name: "test-recording".into(),
                announcer: Box::new(said.clone()),
                ..AppConfig::for_tests()
            });
            app.set_access_mode_for_run(mode);
            app.settings.live.speak_new_entries = speak;
            let source = practice("");
            app.settings.live.sources.insert("plain".into(), source);
            let wire = Wire::default();
            app.set_live_connector(wire.connector());
            app.open_configured_live_source("plain");
            let call = format!("K{n}TEST");
            wire.script.push(checkin(&call, "Ada Example"));
            app.tick(Instant::now());
            assert!(
                said.lock().texts().iter().any(|t| t.starts_with(&call)),
                "{mode:?}: the screen reader gets it"
            );
            // A sentinel said after it: once heard, the entry would have
            // been heard first.
            app.tell("Sentinel message.");
            let deadline = Instant::now() + Duration::from_secs(10);
            let heard = |log: &crate::testing::SpeechLog| {
                log.texts().iter().any(|t| t.contains("Sentinel"))
            };
            if mode != AccessMode::ScreenReader && mode != AccessMode::Hybrid {
                while !heard(&log) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(5));
                }
            } else {
                // Messages are not spoken in these modes: wait a little
                // for anything the entry might have said.
                while Instant::now() < deadline.min(Instant::now() + Duration::from_millis(300)) {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            let said_aloud = log.texts().iter().any(|t| t.contains(&call));
            if spoken {
                let deadline = Instant::now() + Duration::from_secs(10);
                while !log.texts().iter().any(|t| t.contains(&call)) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(5));
                }
                assert!(
                    log.texts().iter().any(|t| t.contains(&call)),
                    "{mode:?} with speak on: {:?}",
                    log.texts()
                );
            } else {
                assert!(!said_aloud, "{mode:?} speak {speak}: {:?}", log.texts());
            }
            app.live_close();
        }
    }

    #[test]
    fn set_up_asks_one_thing_at_a_time_writes_the_source_and_offers_to_open_it() {
        let mut r = rig(&[]);
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        assert_eq!(
            r.label(),
            "Live source name, for example Practice log; Enter cancels"
        );
        r.type_answer("Practice log");
        assert_eq!(r.label(), "Broker host name or address");
        r.type_answer("bad host");
        assert!(r.any("A host name or address is needed, with no spaces."));
        r.type_answer("logger.example.ts.net");
        // Encryption: a list; Off is the first item.
        r.app.dispatch(Command::Choose(0));
        assert_eq!(r.label(), "Port, Enter keeps the default");
        assert_eq!(r.app.prompt_model().unwrap().shown_text(), "1883");
        r.app.dispatch(Command::PromptKey(PromptKey::Enter));
        assert_eq!(r.label(), "Topic to read, for example log/checkins");
        r.type_answer("log/checkins");
        assert_eq!(r.label(), "User name, Enter for none");
        r.type_answer("");
        // View: a list; Log is first.
        r.app.dispatch(Command::Choose(0));
        assert!(r.any("Live source Practice log saved."), "{:?}", r.texts());
        let s = r.app.settings().live.sources.get("practice-log").cloned();
        let s = s.expect("written as practice-log");
        assert_eq!(s.name, "Practice log");
        assert_eq!(s.host, "logger.example.ts.net");
        assert_eq!(s.port, None, "the default port is not written");
        assert_eq!(s.topics, ["log/checkins"]);
        assert_eq!(s.username, "");
        assert_eq!(s.tls, LiveTls::Off);
        assert_eq!(s.view, LiveView::Log);
        // Offered: Open it now.
        r.app.dispatch(Command::Choose(0));
        assert_eq!(r.wire.connects(), 1);
        assert!(r.app.live_receiving());
    }

    #[test]
    fn set_up_with_a_ca_file_a_port_and_the_values_view_and_a_taken_id() {
        let mut r = rig(&[("practice-log", practice(""))]);
        let ca = r._tmp.path().join("ca.pem");
        std::fs::write(&ca, "not checked here").unwrap();
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        r.type_answer("Practice log");
        r.type_answer("logger.example.org");
        r.app.dispatch(Command::Choose(2));
        assert_eq!(r.label(), "Certificate authority file, PEM");
        r.type_answer(&r._tmp.path().join("missing.pem").display().to_string());
        assert!(r.any("No file at"), "{:?}", r.texts());
        r.type_answer(&ca.display().to_string());
        assert_eq!(r.app.prompt_model().unwrap().shown_text(), "8883");
        for _ in 0..4 {
            r.app.dispatch(Command::PromptKey(PromptKey::Backspace));
        }
        r.type_answer("70000");
        assert!(r.any("The port is a number from 1 to 65535."));
        for _ in 0..4 {
            r.app.dispatch(Command::PromptKey(PromptKey::Backspace));
        }
        r.type_answer("9001");
        r.type_answer("sensors/#");
        r.type_answer("reader");
        r.app.dispatch(Command::Choose(1));
        r.app.dispatch(Command::Choose(1));
        assert!(r.any("Not opened."), "{:?}", r.texts());
        let s = r.app.settings().live.sources["practice-log-2"].clone();
        assert_eq!(s.tls, LiveTls::CaFile);
        assert_eq!(s.ca_file, ca.display().to_string());
        assert_eq!(s.port, Some(9001));
        assert_eq!(s.username, "reader");
        assert_eq!(s.view, LiveView::Values);
        assert_eq!(r.wire.connects(), 0);
    }

    #[test]
    fn every_live_command_is_reachable_and_answers_in_words() {
        let mut r = rig(&[("plain", practice(""))]);
        let menu: Vec<ActionId> = r
            .app
            .menu_view(crate::menu::MenuId::Live)
            .items
            .into_iter()
            .filter_map(|i| match i.kind {
                crate::menu::MenuItemKind::Action(a) => Some(a),
                _ => None,
            })
            .collect();
        for &a in COMMANDS {
            assert!(r.app.is_available(a), "{a:?} available with mqtt");
            assert!(menu.contains(&a), "{a:?} in the Live sources menu");
            let bound = !r.app.keymap().chords_for(a).is_empty();
            assert!(
                bound || a.is_palette_command(),
                "{a:?} has a key or is in the palette"
            );
        }
        // Each answers in words, with no live log open and with one.
        for &a in COMMANDS {
            r.clear();
            r.app.dispatch(Command::Action(a));
            assert!(
                !r.texts().is_empty() || r.app.list_model().is_some(),
                "{a:?}"
            );
            escape(&mut r.app);
        }
        r.app.open_configured_live_source("plain");
        r.wire.script.push(checkin("N8CALL", "Ada Example"));
        r.tick(Instant::now());
        for &a in COMMANDS {
            r.clear();
            r.app.dispatch(Command::Action(a));
            // Save is said once the writer has written the log.
            let _ = r.app.wait_for_writes();
            assert!(
                !r.texts().is_empty() || r.app.list_model().is_some(),
                "{a:?}: {:?}",
                r.app.status_text()
            );
            escape(&mut r.app);
        }
    }

    #[test]
    fn say_unread_and_speak_new_entries() {
        let mut r = rig(&[("plain", practice(""))]);
        r.app.dispatch(Command::Action(ActionId::LiveSayUnread));
        assert!(r.any("No document is open."), "{:?}", r.texts());
        r.app.open_live("other", "Notes", "Not a live source.");
        r.app.live_say_unread();
        assert!(r.any("No live log is open."), "{:?}", r.texts());
        r.app.open_configured_live_source("plain");
        r.wire.script.push(checkin("N9CALL", "Ada Example"));
        r.tick(Instant::now());
        r.clear();
        r.app.dispatch(Command::Action(ActionId::LiveSayUnread));
        assert!(r.any("1 unread. Newest: N9CALL"), "{:?}", r.texts());
        assert!(!r.app.settings().live.speak_new_entries);
        r.app.dispatch(Command::Action(ActionId::LiveToggleSpeak));
        assert!(r.app.settings().live.speak_new_entries);
        assert!(r.any("Speak new entries on."));
        r.app.dispatch(Command::Action(ActionId::LiveToggleSpeak));
        assert!(r.any("Speak new entries off."));
    }
}
