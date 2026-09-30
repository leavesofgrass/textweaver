//! Interface announcements (Wave 6, W6u; ADR-0043): every message the app
//! says has an [`Importance`], and `[accessibility]
//! interface_announcements` decides which get through
//! ([`textweaver_a11y::level`]).
//!
//! The routing functions here and in `app.rs` are the only places that
//! reach the status line, the announcer, and the voice with a message; a
//! test reads the sources to keep it so, so no announcement can escape the
//! setting. The familiar helpers keep their meaning: `tell` answers the
//! user, `error` reports a failure, `ask` asks, `note` confirms a routine
//! change; `say_result`, `say_dialog`, `say_progress`, `say_tip`, and
//! `say_hint` name the rest.
//!
//! **Staleness.** A message meant for a list or dialog can be held (while
//! the speech engine starts, or by a frontend that delivers later). Each
//! list, prompt, or menu shown or closed moves the dialog generation on
//! ([`App::dialog_generation`]); a held message whose generation has
//! passed is dropped, not said, as NVDA drops speech for a closed dialog.

use textweaver_a11y::{Importance, InterfaceLevel, Priority, Verbosity, lets_through};
use textweaver_lexicon::args;
use textweaver_store::InterfaceAnnouncements;

use crate::app::App;

/// The level a setting value means in `mode`.
pub fn interface_level(
    setting: InterfaceAnnouncements,
    mode: textweaver_a11y::AccessMode,
) -> InterfaceLevel {
    match setting {
        InterfaceAnnouncements::Auto => InterfaceLevel::for_mode(mode),
        InterfaceAnnouncements::Off => InterfaceLevel::Off,
        InterfaceAnnouncements::Minimal => InterfaceLevel::Minimal,
        InterfaceAnnouncements::Normal => InterfaceLevel::Normal,
        InterfaceAnnouncements::Full => InterfaceLevel::Full,
    }
}

impl App {
    /// The interface announcement level in effect: the setting, or for
    /// `auto` the one the accessibility mode implies.
    pub fn interface_level(&self) -> InterfaceLevel {
        interface_level(
            self.settings.accessibility.interface_announcements,
            self.access_mode,
        )
    }

    /// True when a message of `importance` is said now.
    pub fn interface_allows(&self, importance: Importance) -> bool {
        lets_through(self.interface_level(), importance)
    }

    /// The dialog generation: it moves on whenever a list, prompt, or menu
    /// is shown or closes. A frontend that delivers an announcement later
    /// passes the generation it was made in to
    /// [`announce_for`](Self::announce_for).
    pub fn dialog_generation(&self) -> u64 {
        self.dialog_generation
    }

    /// What list or prompt is shown: its title or label, or `None`.
    pub(crate) fn dialog_key(&self) -> Option<String> {
        self.list_model
            .as_ref()
            .map(|l| l.title.clone())
            .or_else(|| self.prompt_model.as_ref().map(|p| p.label.clone()))
    }

    /// After a command: when the list or prompt shown before it closed or
    /// gave way to another, the dialog generation moves on, and messages
    /// held for it are dropped.
    pub(crate) fn track_dialogs(&mut self, before: Option<String>) {
        if before.is_some() && before != self.dialog_key() {
            self.dialog_generation = self.dialog_generation.wrapping_add(1);
        }
    }

    /// The generation a message said now belongs to: `Some` while a list
    /// or prompt is shown (the message is about it), else `None`.
    pub(crate) fn dialog_tag(&self) -> Option<u64> {
        self.dialog_key().map(|_| self.dialog_generation)
    }

    /// Announces `text` as a frontend's own message of kind `importance`
    /// (a window opening: [`Importance::Dialog`]); the interface level
    /// decides whether it is said.
    pub fn announce_as(&mut self, text: &str, priority: Priority, importance: Importance) {
        self.say_kind(text, Verbosity::Low, priority, importance);
    }

    /// Announces `text`, made while dialog generation `generation` was
    /// current: dropped when a list, prompt, or menu has been shown or
    /// closed since, so a message for a closed dialog is never heard.
    /// Returns whether it was said.
    pub fn announce_for(
        &mut self,
        text: &str,
        priority: Priority,
        importance: Importance,
        generation: u64,
    ) -> bool {
        if generation != self.dialog_generation {
            return false;
        }
        self.announce_as(text, priority, importance);
        true
    }

    /// The outcome of a command the user ran: "Bionic reading on",
    /// "Saved". Heard from `minimal` up.
    pub(crate) fn say_result(&mut self, text: &str) {
        self.say_kind(text, Verbosity::Low, Priority::Polite, Importance::Result);
    }

    /// A list, dialog, or menu closing, or another change to the screen
    /// the user did not ask to hear about. Heard from `normal` up.
    pub(crate) fn say_dialog(&mut self, text: &str) {
        self.say_kind(text, Verbosity::Low, Priority::Polite, Importance::Dialog);
    }

    /// Progress of a long job before its result. Heard from `normal` up;
    /// callers keep to one message every ten seconds at most.
    #[allow(dead_code)]
    pub(crate) fn say_progress(&mut self, text: &str) {
        self.say_kind(text, Verbosity::Low, Priority::Polite, Importance::Progress);
    }

    /// A tip or a welcome. Heard from `normal` up.
    #[allow(dead_code)]
    pub(crate) fn say_tip(&mut self, text: &str) {
        self.say_kind(text, Verbosity::Low, Priority::Polite, Importance::Tip);
    }

    /// A hint about keys. Heard only at `full`.
    #[allow(dead_code)]
    pub(crate) fn say_hint(&mut self, text: &str) {
        self.say_kind(text, Verbosity::Low, Priority::Polite, Importance::Hint);
    }

    /// Ctrl+F9: the next interface announcement level, saved. The change
    /// is always said: it answers the key.
    pub(crate) fn cycle_interface_announcements(&mut self) {
        let next = self.interface_level().next();
        self.settings.accessibility.interface_announcements = match next {
            InterfaceLevel::Off => InterfaceAnnouncements::Off,
            InterfaceLevel::Minimal => InterfaceAnnouncements::Minimal,
            InterfaceLevel::Normal => InterfaceAnnouncements::Normal,
            InterfaceLevel::Full => InterfaceAnnouncements::Full,
        };
        self.settings_dirty = true;
        let level = self.msg(&format!("announce-level-{}", next.id()));
        let msg = self.msg_args("announce-level-changed", &args!["level" => level]);
        self.tell(&msg);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use textweaver_a11y::{AccessMode, Importance};
    use textweaver_keymap::ActionId;

    use crate::{AppConfig, Command};

    use super::*;

    fn app_at(level: InterfaceAnnouncements) -> App {
        let mut config = AppConfig::for_tests();
        config.settings.accessibility.interface_announcements = level;
        App::new(config)
    }

    #[test]
    fn auto_follows_the_mode() {
        use InterfaceAnnouncements as S;
        assert_eq!(
            interface_level(S::Auto, AccessMode::ScreenReader),
            InterfaceLevel::Minimal
        );
        assert_eq!(
            interface_level(S::Auto, AccessMode::Hybrid),
            InterfaceLevel::Minimal
        );
        assert_eq!(
            interface_level(S::Auto, AccessMode::SelfVoicing),
            InterfaceLevel::Normal
        );
        assert_eq!(
            interface_level(S::Full, AccessMode::ScreenReader),
            InterfaceLevel::Full
        );
    }

    /// At `off`, errors and answers still reach the status line; results,
    /// dialogs, and hints do not.
    #[test]
    fn off_keeps_errors_and_answers() {
        let mut app = app_at(InterfaceAnnouncements::Off);
        app.say_result("Bionic reading on.");
        assert_eq!(app.status_text(), "");
        app.say_dialog("Menus closed.");
        app.say_hint("Tab completes.");
        assert_eq!(app.status_text(), "");
        app.tell("Line 3, 12 percent.");
        assert_eq!(app.status_text(), "Line 3, 12 percent.");
        app.error("The file could not be read.");
        assert_eq!(app.status_text(), "The file could not be read.");
        app.announce_as("Opened.", Priority::Polite, Importance::Result);
        assert_eq!(app.status_text(), "The file could not be read.");
    }

    #[test]
    fn the_key_cycles_and_is_always_said() {
        let mut app = app_at(InterfaceAnnouncements::Full);
        app.dispatch(Command::Action(ActionId::CycleInterfaceAnnouncements));
        assert_eq!(app.interface_level(), InterfaceLevel::Off);
        assert_eq!(app.status_text(), "Interface announcements off.");
        app.dispatch(Command::Action(ActionId::CycleInterfaceAnnouncements));
        assert_eq!(app.interface_level(), InterfaceLevel::Minimal);
        assert_eq!(app.status_text(), "Interface announcements minimal.");
    }

    /// A message held for a list that has closed is dropped.
    #[test]
    fn stale_announcements_are_dropped() {
        let mut app = app_at(InterfaceAnnouncements::Full);
        app.dispatch(Command::Action(ActionId::Menu));
        let generation = app.dialog_generation();
        app.dispatch(Command::ListKey(crate::ListKey::Escape));
        assert!(!app.announce_for(
            "1 of 7, File",
            Priority::Polite,
            Importance::Answer,
            generation
        ));
        let now = app.dialog_generation();
        assert!(app.announce_for("Still here.", Priority::Polite, Importance::Answer, now));
        assert_eq!(app.status_text(), "Still here.");
    }

    /// No announcement escapes the setting: outside the routing functions,
    /// nothing in the app core writes to the status line, the announcer,
    /// or the voice with a message. The routing functions all take an
    /// [`Importance`] or give one.
    #[test]
    fn every_announcement_has_a_level() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let sinks = [
            ".status.announce(",
            ".announcer.announce(",
            ".voice_message(",
        ];
        // File, and the function each sink may be called in.
        let allowed: &[(&str, &str)] = &[
            ("app.rs", "fn say_kind"),
            ("app.rs", "fn announce_queued"),
            ("app.rs", "fn show"),
            // Document text read on the status line (screen-reader say
            // all): reading, not an interface announcement.
            ("access.rs", "fn show_screen_sentence"),
            // Messages held while the engine started, said once it is
            // ready, after the staleness check.
            ("restart.rs", "fn say_early_messages"),
            // Dictated words on the status line while speech is held
            // (they are said at the pause through announce_as): content
            // for the Braille display, like the screen say all.
            ("dictation.rs", "fn show_dictation_line"),
        ];
        let mut bad = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&path).unwrap();
            let mut current_fn = String::new();
            for (n, line) in text.lines().enumerate() {
                let t = line.trim_start();
                if let Some(i) = t.find("fn ")
                    && (t.starts_with("fn ") || t.starts_with("pub") || t.starts_with("async"))
                {
                    let rest = &t[i..];
                    current_fn = rest.split(['(', '<']).next().unwrap_or_default().to_owned();
                }
                if t.starts_with("//") {
                    continue;
                }
                for s in sinks {
                    // A sink named in a string (this test's own list) is
                    // not a call.
                    let called = t.match_indices(s).any(|(i, _)| !t[..i].ends_with('"'));
                    if called
                        && !allowed
                            .iter()
                            .any(|(f, func)| *f == name && current_fn == *func)
                    {
                        bad.push(format!("{name}:{}: {} in {current_fn}", n + 1, s));
                    }
                }
            }
        }
        assert!(
            bad.is_empty(),
            "announcements outside the routing: {bad:#?}"
        );
    }
}
