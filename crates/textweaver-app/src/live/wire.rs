//! Live sources' commands (beta 5, N6): open a configured source, set one
//! up, close it, and the commands that work on the open live log.
//!
//! - **Open live source** lists `[live.sources]` by name; Enter opens one
//!   ([`App::open_configured_live_source`]). Opening says the source's
//!   problems in words, if it has any
//!   ([`LiveSourceSettings::problems`]); asks for the password when the
//!   source has a user name and none is kept, and connects once it is
//!   answered; then connects and shows the log.
//! - **Set up a live source** and **Edit a live source** open one form
//!   (crate::live::form): the name, the broker's host, the encryption,
//!   the certificate authority file when one is chosen, the port, the
//!   topic, the user name, and the view. Enter writes
//!   `[live.sources.<id>]` through the settings update path, asks for the
//!   password when there is a user name, then offers to open the source.
//!   A live source file (`.twlive`) opens the same form, filled from it;
//!   **Save live source as file** writes one.
//! - **Close live source** stops receiving; the log stays open, read
//!   only, and is saved.
//! - **Switch list and table**, **Jump to newest**, **Pause or resume
//!   updates**, **Say unread**, **Save live log now**: the open log's
//!   commands (crate::live::hub).
//! - **Speak new entries** turns `[live] speak_new_entries` on or off.
//! - **Show changes of this field** and **Speak changes of this field**
//!   set the values view's field on the cursor's line, saved to the
//!   source.
//! - **Forget live source password** removes the open source's password
//!   (or a chosen source's) from the system credential store.
//! - **Start a broker**, **Stop the broker**, **Show the join code**,
//!   **Share this document live**, **Stop sharing** and **Send a
//!   check-in**: the sending side, with textweaver's own broker inside the
//!   app (crate::live::broker, crate::live::checkin).
//!
//! Without the `mqtt` feature, the commands that open, set up, edit,
//! save, or close a source, and Forget, are hidden from the menus and the palette
//! ([`crate::menu::PENDING`]); run by a key, they say in words that live
//! sources are not in this build.

use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_store::{LiveSourceSettings, live_source_id_ok};

use super::announce::EntryThrottle;
use crate::app::{App, ListKind};
use crate::command::{Effect, PromptPurpose};

/// The commands that need the MQTT client: hidden without the `mqtt`
/// feature.
pub const NEEDS_MQTT: &[ActionId] = &[
    ActionId::OpenLiveSource,
    ActionId::SetUpLiveSource,
    ActionId::EditLiveSource,
    ActionId::SaveLiveSourceFile,
    ActionId::CloseLiveSource,
    ActionId::ForgetLiveSourcePassword,
    ActionId::StartBroker,
    ActionId::StopBroker,
    ActionId::ShowJoinCode,
    ActionId::CopyJoinCode,
    ActionId::CopyJoinLink,
    ActionId::OpenJoinLink,
    ActionId::ShowJoinQr,
    ActionId::JoinFromQrImage,
    ActionId::ShareDocumentLive,
    ActionId::StopSharing,
    ActionId::SendCheckIn,
];

/// Every live source command.
pub const COMMANDS: &[ActionId] = &[
    ActionId::OpenLiveSource,
    ActionId::SetUpLiveSource,
    ActionId::EditLiveSource,
    ActionId::SaveLiveSourceFile,
    ActionId::CloseLiveSource,
    ActionId::LiveSwitchView,
    ActionId::LiveJumpNewest,
    ActionId::LiveTogglePause,
    ActionId::LiveToggleSpeak,
    ActionId::LiveToggleFieldShow,
    ActionId::LiveToggleFieldSpeak,
    ActionId::LiveSayUnread,
    ActionId::SaveLiveLog,
    ActionId::ForgetLiveSourcePassword,
    ActionId::StartBroker,
    ActionId::StopBroker,
    ActionId::ShowJoinCode,
    ActionId::CopyJoinCode,
    ActionId::CopyJoinLink,
    ActionId::OpenJoinLink,
    ActionId::ShowJoinQr,
    ActionId::JoinFromQrImage,
    ActionId::ShareDocumentLive,
    ActionId::StopSharing,
    ActionId::SendCheckIn,
];

/// What a publish was, for saying how it went.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "mqtt"), allow(dead_code))]
pub(crate) enum Sent {
    /// A check-in whose first value is `first`.
    CheckIn {
        /// The entry's first value, said meaning first.
        first: String,
    },
    /// Revision `rev` of the shared document `title`.
    Share {
        /// The document's title.
        title: String,
        /// Its revision.
        rev: u64,
    },
}

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
    /// The set-up and edit form, while it is open (crate::live::form).
    pub(crate) form: Option<super::form::LiveForm>,
    /// The source saved by the form whose password prompt is open: once
    /// answered, the source is offered for opening.
    pub(crate) offer_after_password: Option<String>,
    /// The source Save live source as file is asking a file for.
    pub(crate) save_file: Option<String>,
    /// The check-in form, while it is open (crate::live::checkin).
    pub(crate) checkin: Option<super::checkin::CheckInForm>,
    /// The broker the app runs (crate::live::broker).
    #[cfg(feature = "mqtt")]
    pub(crate) broker: Option<super::broker::AppBroker>,
    /// The document being shared live.
    #[cfg(feature = "mqtt")]
    pub(crate) share: Option<super::broker::Share>,
    /// Publishes waiting on helper threads.
    #[cfg(feature = "mqtt")]
    pub(crate) jobs: Vec<super::broker::SendJob>,
    /// Stands in for the MQTT client (tests).
    #[cfg(feature = "mqtt")]
    connector: Option<Connector>,
}

impl std::fmt::Debug for LiveWire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveWire")
            .field("throttle", &self.throttle)
            .field("open_after_password", &self.open_after_password)
            .field("form", &self.form)
            .field("offer_after_password", &self.offer_after_password)
            .field("save_file", &self.save_file)
            .field("checkin", &self.checkin)
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
    /// Edit a live source: the source ids, by row.
    Edit(Vec<String>),
    /// Save live source as file: the source ids, by row.
    SaveFile(Vec<String>),
    /// The form saved a source: open source `id` now, or not.
    SetupOffer(String),
    /// Show the join code: the code, then its groups, then (with a join
    /// code) the rows Copy the join code and Copy the join link, from this
    /// row on. Enter on a Copy row copies; on any other row, it closes.
    #[cfg(feature = "mqtt")]
    Code(Option<usize>),
    /// Share this document live through which source: the ids, by row.
    #[cfg(feature = "mqtt")]
    ShareThrough(Vec<String>),
    /// Send a check-in through which source: the ids, by row.
    #[cfg(feature = "mqtt")]
    CheckInThrough(Vec<String>),
    /// Show the join QR code (crate::live::qr): its rows and the code.
    #[cfg(feature = "mqtt")]
    Qr(super::qr::JoinQrView),
    /// The join QR code in block characters; Enter closes it.
    #[cfg(feature = "mqtt")]
    QrDrawing,
    /// The join code field's context menu (crate::live::form): its
    /// commands, by row. Escape goes back to the field.
    FieldMenu(Vec<super::form::FieldItem>),
}

impl App {
    /// Runs a live source command.
    pub(crate) fn live_action(&mut self, a: ActionId) -> Vec<Effect> {
        match a {
            ActionId::OpenLiveSource
            | ActionId::SetUpLiveSource
            | ActionId::EditLiveSource
            | ActionId::SaveLiveSourceFile
            | ActionId::CloseLiveSource
            | ActionId::ForgetLiveSourcePassword
            | ActionId::StartBroker
            | ActionId::StopBroker
            | ActionId::ShowJoinCode
            | ActionId::CopyJoinCode
            | ActionId::CopyJoinLink
            | ActionId::OpenJoinLink
            | ActionId::ShowJoinQr
            | ActionId::JoinFromQrImage
            | ActionId::ShareDocumentLive
            | ActionId::StopSharing
            | ActionId::SendCheckIn => self.run_registered(a),
            ActionId::LiveSwitchView => self.live_toggle_view(),
            ActionId::LiveJumpNewest => self.live_jump_newest(),
            ActionId::LiveTogglePause => self.live_toggle_pause(),
            ActionId::LiveToggleSpeak => self.live_toggle_speak(),
            ActionId::LiveToggleFieldShow => self.live_toggle_field_show(),
            ActionId::LiveToggleFieldSpeak => self.live_toggle_field_speak(),
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
            LiveList::Edit(ids) => match ids.get(n) {
                Some(id) => self.live_edit(id, None),
                None => vec![Effect::Redraw],
            },
            LiveList::SaveFile(ids) => match ids.get(n) {
                Some(id) => self.live_save_file_prompt(id),
                None => vec![Effect::Redraw],
            },
            LiveList::SetupOffer(id) => {
                if n == 0 {
                    self.open_configured_live_source(&id)
                } else {
                    let m = self.msg("live-setup-not-now");
                    self.note(&m);
                    vec![Effect::Redraw]
                }
            }
            #[cfg(feature = "mqtt")]
            LiveList::Code(copy_rows) => match copy_rows.map(|first| n.checked_sub(first)) {
                Some(Some(0)) => self.live_copy_join_code(),
                Some(Some(1)) => self.live_copy_join_link(),
                _ => vec![Effect::Redraw],
            },
            #[cfg(feature = "mqtt")]
            LiveList::ShareThrough(ids) => match ids.get(n) {
                Some(id) => self.live_share_start(super::checkin::SendTarget::Source(id.clone())),
                None => vec![Effect::Redraw],
            },
            #[cfg(feature = "mqtt")]
            LiveList::CheckInThrough(ids) => match ids.get(n) {
                Some(id) => self.open_checkin_form(
                    super::checkin::SendTarget::Source(id.clone()),
                    Some(id.clone()),
                ),
                None => vec![Effect::Redraw],
            },
            #[cfg(feature = "mqtt")]
            LiveList::Qr(view) => self.live_choose_qr(&view, n),
            #[cfg(feature = "mqtt")]
            LiveList::QrDrawing => vec![Effect::Redraw],
            LiveList::FieldMenu(items) => self.live_field_menu_chose(items.get(n).copied()),
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

    /// F2 on row `n` of the Open live source list: the form, filled in
    /// with that source; Escape shows the list again.
    pub(crate) fn live_edit_row(&mut self, n: usize) -> Vec<Effect> {
        let Some(ListKind::Live(LiveList::Open(ids))) = self.list.take() else {
            return vec![Effect::Redraw];
        };
        match ids.get(n) {
            Some(id) => self.live_edit(id, Some(n)),
            None => {
                self.list = Some(ListKind::Live(LiveList::Open(ids)));
                vec![Effect::Redraw]
            }
        }
    }

    /// Forget live source password: the open source's, or, with none
    /// open, a chosen source's (from those with a user name).
    pub fn live_forget_command(&mut self) -> Vec<Effect> {
        if let Some(id) = self.open_configured_id() {
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
        #[cfg(feature = "mqtt")]
        if super::mqtt::pin_problem(&source) {
            let name = source.display_name(id).to_owned();
            let m = self.msg_args("live-pin-not-read", &args!["name" => name]);
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
        let offer = self.live_wire.offer_after_password.take();
        let mut effects = self.answer_live_password(text);
        if let Some(id) = offer {
            if self.prompt_is(PromptPurpose::LivePassword) {
                // Asked again (not a password): still pending.
                self.live_wire.offer_after_password = Some(id);
                return effects;
            }
            // Saved with no password kept, it is asked for on opening.
            effects.extend(self.live_offer_open(&id));
            return effects;
        }
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
    pub(super) fn prompt_is(&self, purpose: PromptPurpose) -> bool {
        self.mode.is_prompt() && self.prompt_purpose == purpose
    }

    pub(super) fn live_display_name(&self, id: &str) -> String {
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

    /// A new source's id from its name: lowercase letters and digits, with
    /// hyphens between words, at most 48 characters; `source` when the
    /// name has none; a number added when the id is taken.
    pub(super) fn live_new_source_id(&self, name: &str) -> String {
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
        app.register_handler(ActionId::EditLiveSource, |app| app.live_edit_command());
        app.register_handler(ActionId::SaveLiveSourceFile, |app| {
            app.live_save_file_command()
        });
        app.register_handler(ActionId::CloseLiveSource, |app| app.live_close_command());
        app.register_handler(ActionId::ForgetLiveSourcePassword, |app| {
            app.live_forget_command()
        });
        app.register_handler(ActionId::StartBroker, |app| app.live_start_broker());
        app.register_handler(ActionId::StopBroker, |app| app.live_stop_broker());
        app.register_handler(ActionId::ShowJoinCode, |app| app.live_show_join_code());
        app.register_handler(ActionId::CopyJoinCode, |app| app.live_copy_join_code());
        app.register_handler(ActionId::CopyJoinLink, |app| app.live_copy_join_link());
        app.register_handler(ActionId::OpenJoinLink, |app| app.live_open_join_link());
        app.register_handler(ActionId::ShowJoinQr, |app| app.live_show_join_qr());
        app.register_handler(ActionId::JoinFromQrImage, |app| {
            app.live_join_from_qr_image()
        });
        app.register_handler(ActionId::ShareDocumentLive, |app| app.live_share_command());
        app.register_handler(ActionId::StopSharing, |app| app.live_stop_sharing());
        app.register_handler(ActionId::SendCheckIn, |app| app.live_checkin_command());
    }
    #[cfg(not(feature = "mqtt"))]
    let _ = app;
}

/// Without the `mqtt` feature there is no broker and nothing is sent.
#[cfg(not(feature = "mqtt"))]
impl App {
    /// The broker and the sharing in words: never, in this build.
    pub fn live_broker_words(&self) -> Option<String> {
        None
    }

    pub(crate) fn live_broker_tick(&mut self) -> Vec<Effect> {
        Vec::new()
    }

    pub(crate) fn live_broker_shutdown(&mut self) {}

    pub(crate) fn live_has_join_code(&self) -> bool {
        false
    }

    pub(crate) fn live_is_sharing(&self) -> bool {
        false
    }

    pub(crate) fn answer_save_join_qr(&mut self, _text: &str) -> Vec<Effect> {
        self.live_not_in_build()
    }

    pub(crate) fn answer_join_qr_image(&mut self, _text: &str) -> Vec<Effect> {
        self.live_not_in_build()
    }

    /// The join QR code view: never, in this build.
    pub fn live_join_qr(&self) -> Option<&super::qr::JoinQrView> {
        None
    }

    pub(crate) fn live_share_saved(&mut self, _path: &std::path::Path) {}

    pub(crate) fn live_send(
        &mut self,
        _target: super::checkin::SendTarget,
        _topic: &str,
        _payload: Vec<u8>,
        _retain: bool,
        _sent: Sent,
    ) -> Vec<Effect> {
        self.live_not_in_build()
    }
}

#[cfg(all(test, feature = "mqtt"))]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use textweaver_a11y::{AccessMode, LogAnnouncer, Priority, Shared};
    use textweaver_mqtt::{MqttConfig, MqttEvent, Payload, Secret};
    use textweaver_store::{InterfaceAnnouncements, LiveSourceSettings, LiveTls, LiveView, Paths};

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
            // Start a broker: loopback, a port the system picks.
            s.live.broker.listen = vec!["127.0.0.1:0".into()];
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
            key_order: Default::default(),
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
            texts[0].starts_with("N3CALL, Ada Example.") && texts[0].ends_with("3 new entries."),
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
        assert!(
            r.any("N5CALL, Ada Example. 2 new entries."),
            "{:?}",
            r.texts()
        );
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

    // ----- The live source form (crate::live::form) -----------------------

    impl Rig {
        /// Types `text` into the open field, key by key, as the terminal
        /// reader does.
        fn type_in(&mut self, text: &str) {
            for c in text.chars() {
                self.app.dispatch(Command::PromptKey(PromptKey::Char(c)));
            }
        }

        fn key(&mut self, k: PromptKey) {
            self.app.dispatch(Command::PromptKey(k));
        }

        /// The open field: its label and text.
        fn field(&self) -> (String, String) {
            let p = self.app.prompt_model().expect("the form is open");
            assert_eq!(p.purpose, PromptPurpose::LiveSetup);
            (p.label.clone(), p.text())
        }

        fn last(&self) -> String {
            self.texts().last().cloned().unwrap_or_default()
        }
    }

    /// The form's fields in order, each a labeled line said with its
    /// value; Tab and Shift+Tab keep what was typed; the choices change in
    /// place with Up and Down, said with what they mean; the CA file is a
    /// field only with that encryption; the port follows the encryption.
    #[test]
    fn the_form_has_its_fields_in_order_and_tab_keeps_every_value() {
        let mut r = rig(&[]);
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        assert!(
            r.any("Set up a live source. Tab moves between fields, Enter saves, Escape cancels."),
            "{:?}",
            r.texts()
        );
        assert_eq!(r.field(), ("Name, 1 of 7".into(), String::new()));
        assert!(r.any("Name: blank"), "{:?}", r.texts());
        r.type_in("Practice log");
        r.key(PromptKey::Tab);
        assert_eq!(r.field(), ("Broker host, 2 of 7".into(), String::new()));
        r.type_in("logger.example.ts.net");
        r.key(PromptKey::Tab);
        assert_eq!(
            r.field(),
            (
                "Encryption, 3 of 7, Up or Down Arrow changes".into(),
                "Off".into()
            )
        );
        assert_eq!(r.last(), "Encryption: Off. Up or Down Arrow changes it.");
        r.key(PromptKey::Down);
        assert_eq!(r.field().1, "Bundled roots");
        assert!(r.last().starts_with("Bundled roots: TLS"), "{}", r.last());
        r.key(PromptKey::Down);
        assert_eq!(r.field().1, "CA file");
        r.key(PromptKey::Tab);
        assert_eq!(r.field(), ("CA file, 4 of 8".into(), String::new()));
        r.key(PromptKey::Tab);
        assert_eq!(r.field(), ("Port, 5 of 8".into(), "8883".into()));
        // Back to the encryption: it kept CA file; Off brings 1883 back.
        r.key(PromptKey::BackTab);
        r.key(PromptKey::BackTab);
        assert_eq!(r.field().1, "CA file");
        r.key(PromptKey::Down);
        assert_eq!(r.field().1, "Pinned key");
        assert!(r.last().starts_with("Pinned key: TLS"), "{}", r.last());
        r.key(PromptKey::Down);
        assert_eq!(r.field().1, "Join code");
        r.key(PromptKey::Down);
        assert_eq!(r.field().1, "Off", "Down wraps");
        r.key(PromptKey::Tab);
        assert_eq!(r.field(), ("Port, 4 of 7".into(), "1883".into()));
        // A port changed by hand stays when the encryption changes.
        r.key(PromptKey::SetText("1884".into()));
        r.key(PromptKey::BackTab);
        r.key(PromptKey::Up);
        assert_eq!(r.field().1, "Join code", "Up wraps");
        r.key(PromptKey::Up);
        r.key(PromptKey::Up);
        assert_eq!(r.field().1, "CA file");
        r.key(PromptKey::Up);
        assert_eq!(r.field().1, "Bundled roots");
        r.key(PromptKey::Tab);
        assert_eq!(r.field().1, "1884");
        r.key(PromptKey::Tab);
        assert_eq!(r.field().0, "Topic, 5 of 7");
        r.type_in("log/checkins, log/#");
        r.key(PromptKey::Tab);
        assert_eq!(r.field().0, "User name, 6 of 7");
        r.key(PromptKey::Tab);
        assert_eq!(
            r.field(),
            (
                "View, 7 of 7, Up or Down Arrow changes".into(),
                "Log".into()
            )
        );
        r.key(PromptKey::Down);
        assert_eq!(r.last(), "Values: one line per field, updated in place.");
        // Tab from the last field wraps to the first, its value kept.
        r.key(PromptKey::Tab);
        assert_eq!(r.field(), ("Name, 1 of 7".into(), "Practice log".into()));
        assert_eq!(r.last(), "Name: Practice log");
        r.key(PromptKey::Enter);
        assert!(r.any("Live source Practice log saved."), "{:?}", r.texts());
        let s = r.app.settings().live.sources["practice-log"].clone();
        assert_eq!(s.host, "logger.example.ts.net");
        assert_eq!(s.tls, LiveTls::System);
        assert_eq!(s.port, Some(1884));
        assert_eq!(s.topics, ["log/checkins", "log/#"]);
        assert_eq!(s.username, "");
        assert_eq!(s.view, LiveView::Values);
        // No user name, so no password: offered for opening at once.
        let list = r.app.list_model().expect("the offer").clone();
        assert_eq!(list.title, "Open Practice log now?");
        r.app.dispatch(Command::Choose(0));
        assert_eq!(r.wire.connects(), 1);
    }

    /// Enter checks every field: the first wrong one is said in words and
    /// the form opens on it, every other value kept; a fixed field saves.
    #[test]
    fn a_bad_field_is_said_and_the_form_opens_again_on_it() {
        let mut r = rig(&[("practice-log", practice(""))]);
        let ca = r._tmp.path().join("ca.pem");
        std::fs::write(&ca, "not checked here").unwrap();
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        r.type_in("Practice log");
        r.key(PromptKey::Tab);
        r.type_in("bad host");
        r.key(PromptKey::Tab);
        r.key(PromptKey::Up);
        assert_eq!(r.field().1, "Join code", "Up wraps to the last choice");
        r.key(PromptKey::Up);
        r.key(PromptKey::Up);
        assert_eq!(r.field().1, "CA file");
        r.key(PromptKey::Tab);
        r.type_in(&r._tmp.path().join("missing.pem").display().to_string());
        r.key(PromptKey::Enter);
        assert!(
            r.any("A host name or address is needed, with no spaces."),
            "{:?}",
            r.texts()
        );
        assert_eq!(r.field(), ("Broker host, 2 of 8".into(), "bad host".into()));
        r.key(PromptKey::SetText("logger.example.org".into()));
        r.key(PromptKey::Enter);
        assert!(r.any("No file at"), "{:?}", r.texts());
        assert_eq!(r.field().0, "CA file, 4 of 8");
        r.key(PromptKey::SetText(ca.display().to_string()));
        r.key(PromptKey::Tab);
        r.key(PromptKey::SetText("70000".into()));
        r.key(PromptKey::Enter);
        assert!(r.any("The port is a number from 1 to 65535."));
        assert_eq!(r.field(), ("Port, 5 of 8".into(), "70000".into()));
        r.key(PromptKey::SetText(String::new()));
        r.key(PromptKey::Enter);
        assert!(r.any("A topic is needed."), "{:?}", r.texts());
        assert_eq!(r.field().0, "Topic, 6 of 8");
        r.type_in("sensors/#");
        r.key(PromptKey::BackTab);
        r.key(PromptKey::BackTab);
        r.key(PromptKey::BackTab);
        // A choice typed as something else is said with the choices.
        r.key(PromptKey::SetText("maybe".into()));
        r.key(PromptKey::Enter);
        assert!(
            r.any("Encryption is one of Off, Bundled roots, CA file, Pinned key, Join code."),
            "{:?}",
            r.texts()
        );
        r.key(PromptKey::SetText("ca file".into()));
        r.key(PromptKey::Enter);
        // The name is taken by another source: saved beside it.
        assert!(
            r.any("Live source Practice log saved, beside another source of the same name."),
            "{:?}",
            r.texts()
        );
        let s = r.app.settings().live.sources["practice-log-2"].clone();
        assert_eq!(s.tls, LiveTls::CaFile);
        assert_eq!(s.ca_file, ca.display().to_string());
        assert_eq!(s.port, None, "the default port is not written");
        assert_eq!(s.topics, ["sensors/#"]);
        assert_eq!(
            r.app.settings().live.sources["practice-log"],
            practice(""),
            "the other source is untouched"
        );
    }

    /// Escape cancels the whole form and says so; nothing is saved.
    #[test]
    fn escape_cancels_the_form_and_says_so() {
        let mut r = rig(&[]);
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        r.type_in("Practice log");
        r.key(PromptKey::Tab);
        r.type_in("logger.example");
        r.key(PromptKey::Escape);
        assert!(r.app.prompt_model().is_none());
        assert_eq!(r.last(), "Canceled. No live source saved.");
        assert!(r.app.settings().live.sources.is_empty());
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        assert_eq!(r.field().1, "", "a new form starts empty");
    }

    /// The join code field is masked like a password: stars, nothing
    /// typed said or kept, its value never said. The show key shows the
    /// code as typed and says so, keeping the text and the caret; it is
    /// hidden again on a second press, and each time the field opens.
    #[test]
    fn the_join_code_is_masked_and_the_show_key_shows_it() {
        let mut r = rig(&[]);
        r.app
            .set_prompt_show_key(Some(crate::list_model::show_key()));
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        // The show key does nothing in a field that is not secret.
        r.clear();
        r.key(PromptKey::ToggleShown);
        assert!(r.texts().is_empty(), "{:?}", r.texts());
        assert!(!r.app.prompt_model().unwrap().is_secret());
        r.type_in("Club");
        r.key(PromptKey::Tab);
        r.type_in("logger.example");
        r.key(PromptKey::Tab);
        for _ in 0..4 {
            r.key(PromptKey::Down);
        }
        assert_eq!(r.field().1, "Join code");
        r.key(PromptKey::Tab);
        assert!(r.field().0.starts_with("Join code, 4 of 7"));
        assert_eq!(r.last(), "Join code: blank. F3 shows it.");
        let p = r.app.prompt_model().unwrap();
        assert!(p.is_secret() && p.is_masked() && !p.is_shown());
        r.clear();
        r.type_in("abcd efgh");
        assert!(
            r.texts().is_empty(),
            "nothing typed is said: {:?}",
            r.texts()
        );
        let p = r.app.prompt_model().unwrap();
        assert_eq!(p.shown_text(), "*********");
        assert_eq!(p.text(), "abcd efgh");
        // Shown: the text as typed, said in words, text and caret kept.
        r.key(PromptKey::Left);
        r.key(PromptKey::ToggleShown);
        assert_eq!(r.last(), "Showing the join code");
        let p = r.app.prompt_model().unwrap();
        assert!(p.is_shown() && !p.is_masked());
        assert_eq!((p.shown_text().as_str(), p.caret()), ("abcd efgh", 8));
        r.key(PromptKey::Left);
        r.key(PromptKey::ToggleShown);
        assert_eq!(r.last(), "Hiding the join code");
        let p = r.app.prompt_model().unwrap();
        assert!(p.is_masked());
        assert_eq!((p.shown_text().as_str(), p.caret()), ("*********", 7));
        // No history to recall.
        r.key(PromptKey::Up);
        assert_eq!(r.field().1, "abcd efgh");
        // Away and back: hidden again, its value never said.
        r.key(PromptKey::ToggleShown);
        r.key(PromptKey::Tab);
        r.key(PromptKey::BackTab);
        assert_eq!(r.last(), "Join code: 9 characters, hidden. F3 shows it.");
        assert!(r.app.prompt_model().unwrap().is_masked());
        r.key(PromptKey::Escape);
        assert!(r.app.prompt_history(PromptPurpose::LiveSetup).is_empty());
        assert!(!r.texts().join(" ").contains("abcd"), "{:?}", r.texts());
    }

    /// A secret prompt (the password asked for after saving) is masked
    /// and names the show key; the key shows and hides the password.
    #[test]
    fn the_show_key_shows_a_password_and_hides_it_again() {
        let mut r = rig(&[]);
        r.app
            .set_prompt_show_key(Some(crate::list_model::show_key()));
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        r.type_in("Reader log");
        r.key(PromptKey::Tab);
        r.type_in("logger.example");
        for _ in 0..3 {
            r.key(PromptKey::Tab);
        }
        r.type_in("log/checkins");
        r.key(PromptKey::Tab);
        r.type_in("reader");
        r.key(PromptKey::Enter);
        assert_eq!(
            r.last(),
            "Password for Reader log, Enter cancels. F3 shows it."
        );
        r.type_in("pw");
        let p = r.app.prompt_model().unwrap();
        assert_eq!(p.purpose, PromptPurpose::LivePassword);
        assert_eq!(p.shown_text(), "**");
        r.key(PromptKey::ToggleShown);
        assert_eq!(r.last(), "Showing the password");
        assert_eq!(r.app.prompt_model().unwrap().shown_text(), "pw");
        r.key(PromptKey::ToggleShown);
        assert_eq!(r.last(), "Hiding the password");
        assert_eq!(r.app.prompt_model().unwrap().shown_text(), "**");
    }

    /// A pinned key: the pin field appears, a pin that is not one is said
    /// and the form opens on it, and a good one is kept in groups of four
    /// and written to a live source file; never a password.
    #[test]
    fn a_pinned_key_is_checked_kept_grouped_and_shared_in_the_file() {
        let mut r = rig(&[]);
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        r.type_in("Club log");
        r.key(PromptKey::Tab);
        r.type_in("100.101.1.2");
        r.key(PromptKey::Tab);
        for _ in 0..3 {
            r.key(PromptKey::Down);
        }
        assert_eq!(r.field().1, "Pinned key");
        r.key(PromptKey::Tab);
        assert_eq!(r.field(), ("Pin, 4 of 8".into(), String::new()));
        r.type_in("ABCD-EFGH-JKUN");
        r.key(PromptKey::Tab);
        assert_eq!(r.field().1, "8883", "a pinned key is TLS");
        r.key(PromptKey::Tab);
        r.type_in("netlog/#");
        r.key(PromptKey::Enter);
        assert!(r.any("Not a pin: it has 52 characters"), "{:?}", r.texts());
        assert_eq!(r.field().0, "Pin, 4 of 8", "the form opens on the pin");
        let pin = textweaver_mqtt::Pin::of_spki(b"example key").grouped();
        r.key(PromptKey::SetText(pin.to_lowercase().replace('-', " ")));
        r.key(PromptKey::Enter);
        let s = r.app.settings().live.sources["club-log"].clone();
        assert_eq!((s.tls, s.pin.as_str()), (LiveTls::Pinned, pin.as_str()));
        assert_eq!(s.port, None);
        let file = textweaver_store::live_source_file_text(&s, None);
        assert!(file.contains(&format!("pin = \"{pin}\"")), "{file}");
        assert!(file.contains("tls = \"pinned\""), "{file}");
        assert!(!file.to_lowercase().contains("password ="), "{file}");
        // A pin that does not read is refused before connecting.
        let mut bad = s.clone();
        bad.pin = "ABCD".into();
        r.app.settings.live.sources.insert("bad".into(), bad);
        r.app.open_configured_live_source("bad");
        assert!(r.any("its pin is not one"), "{:?}", r.texts());
        assert_eq!(r.wire.connects(), 0);
    }

    /// With a user name, the password is asked after saving, masked, and
    /// kept in the credential store only; then the source is offered.
    #[test]
    fn a_user_name_asks_for_the_password_after_saving_and_it_is_never_written() {
        let mut r = rig(&[]);
        r.app.dispatch(Command::Action(ActionId::SetUpLiveSource));
        r.type_in("Reader log");
        r.key(PromptKey::Tab);
        r.type_in("logger.example");
        for _ in 0..3 {
            r.key(PromptKey::Tab);
        }
        r.type_in("log/checkins");
        r.key(PromptKey::Tab);
        r.type_in("reader");
        r.key(PromptKey::Enter);
        assert!(r.any("Live source Reader log saved."), "{:?}", r.texts());
        let p = r.app.prompt_model().expect("the password prompt");
        assert_eq!(p.purpose, PromptPurpose::LivePassword);
        assert_eq!(p.label, "Password for Reader log, Enter cancels");
        r.type_answer(PASSWORD);
        assert!(r.any("Password kept for Reader log"), "{:?}", r.texts());
        assert_eq!(
            r.app.list_model().map(|l| l.title.clone()).as_deref(),
            Some("Open Reader log now?")
        );
        r.app.dispatch(Command::Choose(0));
        assert_eq!(r.wire.connects(), 1, "the kept password connects");
        let _ = r.app.wait_for_writes();
        let settings =
            std::fs::read_to_string(Paths::under(r._tmp.path()).settings_file()).unwrap();
        assert!(settings.contains("Reader log"), "{settings}");
        assert!(!settings.contains("not a real password"), "{settings}");
        assert!(!r.texts().join(" ").contains("not a real password"));
    }

    /// Edit a live source and F2 in the Open list open the form filled
    /// in; Escape from F2 shows the list again on its row; a saved edit
    /// keeps the keys the form does not show.
    #[test]
    fn edit_fills_the_form_and_keeps_what_the_form_does_not_show() {
        let mut mapped = practice("reader");
        mapped.id_field = "serial".into();
        mapped.fields = vec![textweaver_store::LiveField {
            path: "call".into(),
            label: "Callsign".into(),
            ..Default::default()
        }];
        let mut r = rig(&[("alpha", practice("")), ("beta", mapped.clone())]);
        r.app.dispatch(Command::Action(ActionId::OpenLiveSource));
        r.app
            .dispatch(Command::ListKey(crate::list_model::ListKey::Down));
        r.app
            .dispatch(Command::ListKey(crate::list_model::ListKey::Rename));
        assert!(r.any("Edit Practice log."), "{:?}", r.texts());
        assert_eq!(r.field(), ("Name, 1 of 7".into(), "Practice log".into()));
        r.key(PromptKey::Escape);
        assert!(r.any("Canceled. Practice log not changed."));
        let list = r.app.list_model().expect("the list again").clone();
        assert_eq!(list.selected, 1, "on the row it came from");
        // Edit from the command: a list, then the form.
        escape(&mut r.app);
        r.app.dispatch(Command::Action(ActionId::EditLiveSource));
        assert_eq!(
            r.app.list_model().map(|l| l.title.clone()).as_deref(),
            Some("Edit which live source?")
        );
        r.app.dispatch(Command::Choose(1));
        r.key(PromptKey::Tab);
        assert_eq!(r.field().1, "logger.example.ts.net");
        r.key(PromptKey::SetText("logger2.example.ts.net".into()));
        r.key(PromptKey::Enter);
        assert!(r.any("Live source Practice log saved."), "{:?}", r.texts());
        let s = r.app.settings().live.sources["beta"].clone();
        assert_eq!(s.host, "logger2.example.ts.net");
        assert_eq!(s.id_field, "serial", "kept");
        assert_eq!(s.fields, mapped.fields, "kept");
        assert_eq!(r.app.settings().live.sources.len(), 2, "no new source");
        // It signs in and has no password kept: asked; cancelled, the
        // source is still offered.
        assert_eq!(
            r.app.prompt_model().map(|p| p.purpose),
            Some(PromptPurpose::LivePassword)
        );
        r.type_answer("");
        assert!(r.app.list_model().is_some(), "the offer");
        escape(&mut r.app);
        // Unchanged: said so, nothing written.
        r.app.dispatch(Command::Action(ActionId::EditLiveSource));
        r.app.dispatch(Command::Choose(0));
        r.key(PromptKey::Enter);
        assert!(r.any("Practice log not changed."), "{:?}", r.texts());
    }

    fn write(dir: &std::path::Path, name: &str, text: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, text).unwrap();
        p
    }

    /// Opening a `.twlive` file opens the form filled from it, says what
    /// was left out (a password first), and Enter saves it as a new
    /// source beside one of the same name, never replacing it.
    #[test]
    fn a_live_source_file_opens_the_form_and_saves_a_new_source() {
        let mut r = rig(&[("practice-log", practice(""))]);
        let dir = r._tmp.path().join("shared");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("ca.pem"), "not checked here").unwrap();
        let file = write(
            &dir,
            "practice.twlive",
            "name = \"Practice log\"\nhost = \"logger.example.ts.net\"\ntls = \"ca_file\"\n\
             ca_file = \"ca.pem\"\ntopics = [\"log/checkins\"]\nview = \"values\"\n\
             password = \"example only, not a real password\"\ncolour = \"blue\"\n\
             [[fields]]\npath = \"call\"\nlabel = \"Callsign\"\n",
        );
        r.app.dispatch(Command::Open(file.clone()));
        assert!(
            r.any("Live source from practice.twlive. Check each field"),
            "{:?}",
            r.texts()
        );
        assert!(
            r.any("Left out of the file: a password, which textweaver never reads from a file; it is asked for after saving; the unknown key colour."),
            "{:?}",
            r.texts()
        );
        assert_eq!(r.field(), ("Name, 1 of 8".into(), "Practice log".into()));
        for _ in 0..3 {
            r.key(PromptKey::Tab);
        }
        assert_eq!(
            r.field(),
            (
                "CA file, 4 of 8".into(),
                dir.join("ca.pem").display().to_string()
            )
        );
        r.key(PromptKey::Enter);
        assert!(
            r.any("saved, beside another source of the same name"),
            "{:?}",
            r.texts()
        );
        let s = r.app.settings().live.sources["practice-log-2"].clone();
        assert_eq!(s.view, LiveView::Values);
        assert_eq!(s.fields.len(), 1);
        assert!(s.extra.is_empty());
        assert_eq!(r.app.settings().live.sources["practice-log"], practice(""));
        assert!(!format!("{:?}", r.app.settings().live).contains("not a real password"));
        assert!(!r.texts().join(" ").contains("not a real password"));
        // Escape on a file's form saves nothing.
        r.app
            .dispatch(Command::ListKey(crate::list_model::ListKey::Escape));
        r.app.dispatch(Command::Open(file));
        r.key(PromptKey::Escape);
        assert_eq!(r.app.settings().live.sources.len(), 2);
    }

    /// A hostile or broken file is refused in words and opens nothing.
    #[test]
    fn a_broken_live_source_file_is_refused_in_words() {
        let mut r = rig(&[]);
        let dir = r._tmp.path().to_path_buf();
        let cases = [
            (
                write(&dir, "broken.twlive", "host = = 3\n"),
                "it is not valid TOML at line 1.",
            ),
            (
                write(&dir, "huge.twlive", &"#".repeat(70_000)),
                "it is larger than 65536 bytes.",
            ),
        ];
        for (path, why) in cases {
            r.clear();
            r.app.dispatch(Command::Open(path));
            assert!(r.any(why), "{why}: {:?}", r.texts());
            assert!(r.app.prompt_model().is_none());
            assert!(r.app.session().is_none());
        }
        // An unusual extension case still opens the form.
        let p = write(&dir, "upper.TWLIVE", "host = \"192.0.2.1\"\n");
        r.app.dispatch(Command::Open(p));
        assert_eq!(r.field().0, "Name, 1 of 7");
    }

    /// Save live source as file writes the source with no password, and
    /// the file opens the form again.
    #[test]
    fn save_as_file_writes_the_source_without_a_password() {
        let mut r = rig(&[("practice", practice("reader"))]);
        r.app
            .dispatch(Command::Action(ActionId::SaveLiveSourceFile));
        assert_eq!(
            r.app.list_model().map(|l| l.title.clone()).as_deref(),
            Some("Save which live source as a file?")
        );
        r.app.dispatch(Command::Choose(0));
        let spec = r
            .app
            .path_prompt_spec(PromptPurpose::SaveLiveSourceFile)
            .expect("a save chooser");
        assert_eq!(spec.file_name.as_deref(), Some("practice.twlive"));
        assert_eq!(spec.extensions, ["twlive"]);
        let target = r._tmp.path().join("shared-practice");
        r.type_answer(&target.display().to_string());
        let written = target.with_extension("twlive");
        let text = std::fs::read_to_string(&written).unwrap();
        assert!(r.any("Practice log saved as"), "{:?}", r.texts());
        assert!(r.any("without a password"), "{:?}", r.texts());
        assert!(text.contains("username = \"reader\""), "{text}");
        assert!(!text.contains("password ="), "{text}");
        r.app.dispatch(Command::Open(written));
        assert_eq!(r.field().1, "Practice log");
    }

    #[test]
    fn every_live_command_is_reachable_and_answers_in_words() {
        let mut r = rig(&[("plain", practice(""))]);
        // Live documents, and its Reading and Sharing submenus.
        let menu: Vec<ActionId> = [
            crate::menu::MenuId::Live,
            crate::menu::MenuId::LiveReading,
            crate::menu::MenuId::LiveSharing,
        ]
        .into_iter()
        .flat_map(|m| r.app.menu_view(m).items)
        .filter_map(|i| match i.kind {
            crate::menu::MenuItemKind::Action(a) => Some(a),
            _ => None,
        })
        .collect();
        for &a in COMMANDS {
            assert!(r.app.is_available(a), "{a:?} available with mqtt");
            assert!(menu.contains(&a), "{a:?} in the Live documents menu");
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

    /// The values view's field changes go through the throttle, each
    /// field at most once each `[live] min_interval_seconds`.
    #[test]
    fn a_values_view_says_each_field_change_at_its_own_pace() {
        let source = LiveSourceSettings {
            view: LiveView::Values,
            ..practice("")
        };
        let mut r = rig(&[("sensor", source)]);
        r.app.settings.live.min_interval_seconds = 10;
        r.app.open_configured_live_source("sensor");
        assert!(r.app.live_shows_values());
        let s = r.wire.script.clone();
        let reading = |t: f64| MqttEvent::Message {
            topic: "log/checkins".into(),
            payload: Payload::Json(serde_json::json!({ "temp": t })),
            retained: false,
            key_order: Default::default(),
        };
        let t0 = Instant::now();
        s.push(reading(21.4));
        r.tick(t0);
        s.push(reading(21.5));
        r.tick(t0 + Duration::from_secs(1));
        r.clear();
        s.push(reading(21.6));
        r.tick(t0 + Duration::from_secs(3));
        assert!(!r.any("21.6"), "held: {:?}", r.texts());
        r.tick(t0 + Duration::from_secs(10));
        let said: Vec<String> = r
            .texts()
            .into_iter()
            .filter(|t| t.contains("21."))
            .collect();
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("21.6"), "the latest value only: {said:?}");
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
