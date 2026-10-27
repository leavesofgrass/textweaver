//! Live sources' passwords (beta 5): asked once, masked, and kept only in
//! the system credential store, under the account `live:<id>`.
//!
//! A live source's user name is in `settings.toml`
//! (`[live.sources.<id>] username`); its password never is, nor in the
//! log, a message, or the status line. [`App::ask_live_password`] opens a
//! secret prompt ([`PromptPurpose::LivePassword`], one star per
//! character, no history), [`App::live_password`] gives the password to
//! sign in with (looked up once a session), and
//! [`App::forget_live_password`] removes it. A build without the
//! credential store refuses to ask, in words, rather than take a password
//! it could only keep in plain text.
//!
//! Commands and keys for these come with the live source's connection;
//! this module is the part they call.

use std::collections::BTreeMap;

use textweaver_components::credentials::{self, Password};
use textweaver_lexicon::args;

use crate::app::App;
use crate::command::{Effect, PromptPurpose};

/// The password prompt's source, and the passwords known this session.
#[derive(Debug, Default)]
pub(crate) struct LivePasswords {
    /// The id of the source the open password prompt is for.
    asking: Option<String>,
    /// Passwords by source id, looked up or typed this session; `None`
    /// when the store had none.
    known: BTreeMap<String, Option<Password>>,
}

impl App {
    /// The name of live source `id` as shown and said: its `name`, or the
    /// id itself.
    fn live_source_name(&self, id: &str) -> String {
        self.settings
            .live
            .sources
            .get(id)
            .map_or(id, |s| s.display_name(id))
            .to_owned()
    }

    /// The source a [`PromptPurpose::LivePassword`] prompt names, for its
    /// label.
    pub(crate) fn live_password_prompt_name(&self, purpose: PromptPurpose) -> Option<String> {
        if purpose != PromptPurpose::LivePassword {
            return None;
        }
        let id = self.live_passwords.asking.as_deref()?;
        Some(self.live_source_name(id))
    }

    /// The password to sign in to live source `id` with: the one typed
    /// this session, else the one in the system credential store (looked
    /// up once a session), else none.
    pub fn live_password(&mut self, id: &str) -> Option<Password> {
        if !self.live_passwords.known.contains_key(id) {
            let found = credentials::stored_password(id);
            self.live_passwords.known.insert(id.to_owned(), found);
        }
        self.live_passwords.known.get(id).cloned().flatten()
    }

    /// Asks for live source `id`'s password, masked. Refused in words
    /// when there is no such source, or when this build has no credential
    /// store to keep it in.
    pub fn ask_live_password(&mut self, id: &str) -> Vec<Effect> {
        self.entry(|app| app.ask_live_password_inner(id))
    }

    fn ask_live_password_inner(&mut self, id: &str) -> Vec<Effect> {
        if !self.settings.live.sources.contains_key(id) {
            let msg = self.msg_args("live-source-unknown", &args!["source" => id]);
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        let name = self.live_source_name(id);
        if !credentials::has_store() {
            let msg = self.msg_args("live-password-no-store", &args!["source" => name.as_str()]);
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        self.live_passwords.asking = Some(id.to_owned());
        self.prompt(PromptPurpose::LivePassword)
    }

    /// The answer to the password prompt: kept in the system credential
    /// store, and used this session even when the store refuses it; empty
    /// cancels. Nothing said here holds the password.
    pub(crate) fn answer_live_password(&mut self, text: &str) -> Vec<Effect> {
        let Some(id) = self.live_passwords.asking.take() else {
            return vec![Effect::Redraw];
        };
        if text.is_empty() {
            let msg = self.msg("common-cancelled");
            self.note(&msg);
            return vec![Effect::Redraw];
        }
        let Some(password) = Password::new(text) else {
            let msg = self.msg("live-password-invalid");
            self.error(&msg);
            return self.ask_live_password_inner(&id);
        };
        self.keep_live_password(&id, password);
        vec![Effect::Redraw]
    }

    /// Keeps `password` for live source `id`: in the system credential
    /// store, and this session even when the store refuses it, saying
    /// which. Nothing said holds the password.
    pub(crate) fn keep_live_password(&mut self, id: &str, password: Password) {
        let name = self.live_source_name(id);
        match credentials::store_password(id, &password) {
            Ok(()) => {
                let msg = self.msg_args("live-password-kept", &args!["source" => name.as_str()]);
                self.note(&msg);
            }
            Err(reason) => {
                let msg = self.msg_args(
                    "live-password-not-kept",
                    &args!["source" => name.as_str(), "reason" => reason],
                );
                self.error(&msg);
            }
        }
        self.live_passwords
            .known
            .insert(id.to_owned(), Some(password));
    }

    /// Forgets live source `id`'s password: removes it from the system
    /// credential store and from this session, so it is asked for again.
    pub fn forget_live_password(&mut self, id: &str) -> Vec<Effect> {
        self.entry(|app| app.forget_live_password_inner(id))
    }

    fn forget_live_password_inner(&mut self, id: &str) -> Vec<Effect> {
        let name = self.live_source_name(id);
        self.live_passwords.known.remove(id);
        let key = match credentials::forget_password(id) {
            Ok(true) => "live-password-forgotten",
            Ok(false) => "live-password-none",
            Err(reason) => {
                let msg = self.msg_args(
                    "live-password-not-forgotten",
                    &args!["source" => name.as_str(), "reason" => reason],
                );
                self.error(&msg);
                return vec![Effect::Redraw];
            }
        };
        let msg = self.msg_args(key, &args!["source" => name.as_str()]);
        self.note(&msg);
        vec![Effect::Redraw]
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use textweaver_a11y::{Announcer, Priority};
    use textweaver_store::{LiveSourceSettings, Paths};

    use super::*;
    use crate::AppConfig;
    use crate::command::Command;
    use crate::list_model::PromptKey;

    const PASSWORD: &str = "example only, not a real password";

    /// What the app announced.
    #[derive(Clone, Default)]
    struct Said(Arc<Mutex<Vec<String>>>);

    impl Said {
        fn all(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
        fn any(&self, needle: &str) -> bool {
            self.all().iter().any(|s| s.contains(needle))
        }
        fn clear(&self) {
            self.0.lock().unwrap().clear();
        }
    }

    impl Announcer for Said {
        fn announce(&mut self, text: &str, _priority: Priority) {
            self.0.lock().unwrap().push(text.to_owned());
        }
    }

    /// The app with the live source `id`, named "Practice log", and the
    /// memory credential store (never the system's).
    fn app_with_source(id: &str) -> (App, Said, tempfile::TempDir) {
        textweaver_components::fake::memory_credentials();
        let tmp = tempfile::tempdir().unwrap();
        let said = Said::default();
        let mut app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            paths: Some(Paths::under(tmp.path())),
            ..AppConfig::for_tests()
        });
        let source = LiveSourceSettings {
            name: "Practice log".into(),
            host: "broker.example".into(),
            username: "reader".into(),
            topics: vec!["log/checkins".into()],
            ..LiveSourceSettings::default()
        };
        let _ = app.update_settings(|s| {
            s.live.sources.insert(id.to_owned(), source);
        });
        (app, said, tmp)
    }

    #[test]
    fn a_password_is_asked_masked_kept_and_forgotten() {
        let (mut app, said, tmp) = app_with_source("pw-app-a");
        assert_eq!(app.live_password("pw-app-a"), None);
        app.ask_live_password("pw-app-a");
        let model = app.prompt_model().unwrap().clone();
        assert_eq!(model.purpose, PromptPurpose::LivePassword);
        assert!(model.purpose.is_secret());
        assert_eq!(
            model.label, "Password for Practice log, Enter cancels",
            "the label names the source"
        );
        said.clear();
        for c in PASSWORD.chars() {
            app.dispatch(Command::PromptKey(PromptKey::Char(c)));
        }
        assert_eq!(
            app.prompt_model().unwrap().shown_text(),
            "*".repeat(PASSWORD.chars().count())
        );
        assert!(said.all().is_empty(), "nothing echoed: {:?}", said.all());
        app.dispatch(Command::PromptKey(PromptKey::Enter));
        assert!(app.prompt_history(PromptPurpose::LivePassword).is_empty());
        assert!(
            said.any("Password kept for Practice log, in the system credential store."),
            "{:?}",
            said.all()
        );
        assert_eq!(
            app.live_password("pw-app-a").map(|p| p.reveal().to_owned()),
            Some(PASSWORD.to_owned())
        );
        assert_eq!(
            credentials::stored_password("pw-app-a").map(|p| p.reveal().to_owned()),
            Some(PASSWORD.to_owned())
        );

        // Never in settings.toml, nor in anything said.
        app.save_settings().unwrap();
        let _ = app.wait_for_writes();
        let settings = std::fs::read_to_string(Paths::under(tmp.path()).settings_file()).unwrap();
        assert!(settings.contains("reader"), "{settings}");
        assert!(!settings.contains("not a real password"), "{settings}");
        assert!(!said.all().join(" ").contains("not a real password"));

        app.forget_live_password("pw-app-a");
        assert!(said.any("Password forgotten for Practice log."));
        assert_eq!(app.live_password("pw-app-a"), None);
        app.forget_live_password("pw-app-a");
        assert!(said.any("No password kept for Practice log."));
    }

    #[test]
    fn empty_cancels_a_bad_password_asks_again_and_an_unknown_source_is_said() {
        let (mut app, said, _tmp) = app_with_source("pw-app-b");
        app.ask_live_password("pw-app-b");
        app.dispatch(Command::PromptKey(PromptKey::Enter));
        assert!(app.prompt_model().is_none());
        assert_eq!(credentials::stored_password("pw-app-b"), None);

        app.ask_live_password("pw-app-b");
        app.leave_prompt();
        let _ = app.entry(|a| a.answer_live_password("tab\there"));
        assert!(said.any("Not a password: up to 1024 characters, with no line breaks or tabs."));
        let purpose = app.prompt_model().map(|p| p.purpose);
        assert_eq!(purpose, Some(PromptPurpose::LivePassword), "asked again");
        assert_eq!(credentials::stored_password("pw-app-b"), None);
        app.leave_prompt();

        app.ask_live_password("nowhere");
        assert!(said.any("No live source named nowhere."));
    }
}
