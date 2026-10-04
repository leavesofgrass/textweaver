//! The first run (W9b-f): one sequence of at most three skippable steps,
//! the same in the window and the terminal reader.
//!
//! 1. **The language**, only when the system's language is not one of the
//!    built-in ones: a list, the system's first, Escape keeps English.
//!    When it is built in, it is put into effect without a word, before
//!    the welcome, so the welcome is already in that language.
//! 2. **A screen reader**, inferred, never asked: when one is running and
//!    the mode was never chosen, textweaver starts in hybrid mode and says
//!    so in one sentence with the key that changes it
//!    ([`App::infer_hybrid`]).
//! 3. **The optional components**, a list with nothing chosen, shown once
//!    nothing else is open (`[components] chooser_shown`). Nothing is
//!    fetched unless something is chosen and Download is pressed.
//!
//! One dialog at a time: the components list waits for the language list
//! to close ([`App::tick`]). "Ask again about first-run choices" (Tools)
//! clears `hybrid_offered` and `chooser_shown`, so the screen reader step
//! and the components list come again at the next start.

use textweaver_a11y::detect::Detected;
use textweaver_lexicon::i18n;

use crate::app::App;
use crate::command::Effect;

/// The most steps the first run has, each skippable with Escape.
pub const FIRST_RUN_MAX_STEPS: usize = 3;

impl App {
    /// The first run's language step, before the welcome. `system` is the
    /// system's language tag (`None` to read it from the system). When it
    /// is one of the built-in languages, it is put into effect quietly and
    /// false is returned: no list. Otherwise true: the list is due.
    pub fn first_run_language(&mut self, system: Option<&str>) -> bool {
        let read;
        let system = match system {
            Some(s) => Some(s),
            None => {
                read = crate::words::system_language();
                read.as_deref()
            }
        };
        let Some(lang) = system.and_then(i18n::language) else {
            return true;
        };
        if self.settings.interface.language != lang.tag {
            self.settings.interface.language = lang.tag.to_owned();
            self.settings_dirty = true;
            self.apply_interface_language();
            // A note about the voice is for a change the user made.
            self.language_note = None;
        }
        false
    }

    /// The first run's steps after the welcome: the language list when
    /// [`App::first_run_language`] said it is due, hybrid mode inferred
    /// when `screen_reader` found one, and the components list offered for
    /// when nothing else is open. Returns the effects to show the list.
    pub fn first_run_steps(
        &mut self,
        language_due: bool,
        screen_reader: Option<&Detected>,
    ) -> Vec<Effect> {
        if let Some(found) = screen_reader {
            self.infer_hybrid(found);
        }
        let effects = if language_due {
            self.language_list()
        } else {
            Vec::new()
        };
        self.offer_components_on_first_run();
        effects
    }

    /// A later start: when "Ask again about first-run choices" cleared the
    /// marker and a screen reader runs, hybrid mode is inferred and said,
    /// as on the first run. True when it chose.
    pub fn startup_screen_reader_step(&mut self, screen_reader: Option<&Detected>) -> bool {
        screen_reader.is_some_and(|found| self.infer_hybrid(found))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppConfig;
    use crate::command::Command;
    use std::sync::{Arc, Mutex};
    use textweaver_a11y::{AccessMode, Announcer, Priority};
    use textweaver_store::Paths;

    #[derive(Clone, Default)]
    struct Said(Arc<Mutex<Vec<String>>>);

    impl Said {
        fn all(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }

    impl Announcer for Said {
        fn announce(&mut self, text: &str, _priority: Priority) {
            self.0.lock().unwrap().push(text.to_owned());
        }
    }

    fn first_app(home: &std::path::Path) -> (App, Said) {
        let said = Said::default();
        let app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            paths: Some(Paths::under(home)),
            ..AppConfig::for_tests()
        });
        (app, said)
    }

    /// Runs the first run and presses Escape at each dialog; returns how
    /// many dialogs came, one at a time.
    fn escape_each(app: &mut App, effects: Vec<Effect>) -> usize {
        let mut steps = 0;
        let mut shown = effects.iter().any(|e| matches!(e, Effect::ShowList { .. }));
        for _ in 0..10 {
            if !shown {
                let e = app.tick(std::time::Instant::now());
                shown = e.iter().any(|e| matches!(e, Effect::ShowList { .. }))
                    || app.list_model().is_some();
                if !shown {
                    break;
                }
            }
            steps += 1;
            app.dispatch(Command::Cancel);
            assert!(app.list_model().is_none(), "Escape closes step {steps}");
            shown = false;
        }
        steps
    }

    #[test]
    fn a_built_in_system_language_skips_the_list() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut app, said) = first_app(tmp.path());
        assert!(!app.first_run_language(Some("fr_CA.UTF-8")));
        assert_eq!(app.settings().interface.language, "fr");
        assert!(said.all().is_empty(), "nothing said: {:?}", said.all());
        let effects = app.first_run_steps(false, None);
        assert!(effects.is_empty());
        // Only the components list comes, then nothing.
        assert_eq!(escape_each(&mut app, effects), 1);
    }

    #[test]
    fn other_system_languages_get_the_list_then_the_components() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut app, _) = first_app(tmp.path());
        assert!(app.first_run_language(Some("ja-JP")));
        assert_eq!(app.settings().interface.language, "en");
        let effects = app.first_run_steps(true, None);
        let steps = escape_each(&mut app, effects);
        assert_eq!(steps, 2, "the language list, then the components list");
        assert!(steps <= FIRST_RUN_MAX_STEPS);
        assert!(app.settings().components.chooser_shown);
        // Escape kept the language.
        assert_eq!(app.settings().interface.language, "en");
    }

    #[test]
    fn a_screen_reader_is_inferred_and_said_not_asked() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut app, said) = first_app(tmp.path());
        let nvda = Detected {
            name: Some("NVDA".into()),
        };
        let effects = app.first_run_steps(false, Some(&nvda));
        assert!(!app.confirmation_pending(), "no question");
        assert_eq!(app.access_mode(), AccessMode::Hybrid);
        let a = &app.settings().accessibility;
        assert!(a.hybrid_offered);
        assert_eq!(a.mode, textweaver_store::AccessMode::Hybrid);
        let all = said.all().join("\n");
        assert!(
            all.contains("NVDA is running: textweaver reads documents aloud"),
            "{all}"
        );
        // The mode is not a dialog: still only the components list.
        assert!(escape_each(&mut app, effects) <= FIRST_RUN_MAX_STEPS);
        // Chosen once: a later start says nothing more.
        assert!(!app.startup_screen_reader_step(Some(&nvda)));
    }
}
