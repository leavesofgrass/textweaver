//! The window's safety net (Wave 9, W9a-w): the log file, a save when the
//! program stops unexpectedly, and the sentence that says so.
//!
//! - [`start_log`] writes `textweaver.log` in the state folder, as the
//!   terminal reader does: warnings and errors by default, the level
//!   `TEXTWEAVER_LOG` names otherwise.
//! - [`install_panic_hook`] logs every panic. A window whose graphics
//!   cannot start says so in words ([`StartupMessages::graphics`]), in a
//!   message box when it was started from a shortcut.
//! - [`save_after_trouble`] writes the recovery copy of unsaved edits and
//!   saves the place and the settings, through the app's own
//!   `emergency_save`, the terminal reader's. The window calls it when it
//!   is dropped without having closed (a panic unwinding through the event
//!   loop) and when the session ends.
//! - [`StartupMessages::crashed`] is the sentence said after a panic.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use textweaver_app::App;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::{Paths, SettingsStore};

/// Set once the graphics failure has been reported, so it is said once.
static GRAPHICS_REPORTED: AtomicBool = AtomicBool::new(false);
/// Set once the window saved after trouble.
static SAVED_AFTER_TROUBLE: AtomicBool = AtomicBool::new(false);

/// The sentences the window may need when it cannot carry on, looked up in
/// the listener's language before the window starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupMessages {
    /// "The window could not start its graphics. The terminal reader,
    /// textweaver, needs none."
    pub graphics: String,
    /// "textweaver stopped after an internal error."
    pub crashed: String,
    /// "Your place was saved, and unsaved edits will be offered for
    /// recovery next time."
    pub crashed_saved: String,
}

impl StartupMessages {
    /// The messages from `catalog`.
    pub fn from_catalog(catalog: &Catalog) -> Self {
        StartupMessages {
            graphics: catalog.tr("gui-graphics-failed"),
            crashed: catalog.tr("gui-crashed"),
            crashed_saved: catalog.tr("gui-crashed-saved"),
        }
    }

    /// The sentence said after a panic stopped the window: what happened,
    /// then, when the save after it ran, that the place and edits are
    /// safe.
    pub fn after_crash(&self, saved: bool) -> String {
        if saved {
            format!("{} {}", self.crashed, self.crashed_saved)
        } else {
            self.crashed.clone()
        }
    }

    /// The messages in the language of the settings under `home` (or the
    /// platform's folder).
    pub fn load(home: Option<&Path>) -> Self {
        let paths = paths_for(home);
        let language = paths
            .as_ref()
            .map(|p| SettingsStore::new(p.clone()).load().0.interface.language)
            .unwrap_or_default();
        let dir = paths.as_ref().map(Paths::locales_dir);
        let (catalog, _) = Catalog::for_language(&language, dir.as_deref());
        Self::from_catalog(&catalog)
    }
}

/// The data folders under `home`, or the platform's.
fn paths_for(home: Option<&Path>) -> Option<Paths> {
    match home {
        Some(h) => Some(Paths::under(h)),
        None => Paths::platform().ok(),
    }
}

/// Starts `textweaver.log` in the state folder under `home` (or the
/// platform's), at the level `TEXTWEAVER_LOG` names, else warnings and
/// errors. Returns a message when the level's name is unknown.
pub fn start_log(home: Option<&Path>) -> Option<String> {
    let env = std::env::var("TEXTWEAVER_LOG").ok();
    let (level, message) = match textweaver_app::logfile::choose_level(None, env.as_deref()) {
        Ok(level) => (level, None),
        Err(msg) => (textweaver_app::logfile::DEFAULT_LEVEL, Some(msg)),
    };
    if let Some(paths) = paths_for(home) {
        textweaver_app::logfile::init(&paths, level);
    }
    message
}

/// True when a panic's text is the vendored window runner's "could not
/// start its graphics" failure.
pub fn is_graphics_failure(panic_text: &str) -> bool {
    panic_text.contains(masonry_winit::app::GRAPHICS_FAILURE)
}

/// Records that the window saved after trouble (see [`saved_after_trouble`]).
pub fn note_saved_after_trouble() {
    SAVED_AFTER_TROUBLE.store(true, Ordering::Release);
}

/// True once the window saved after trouble, so the crash sentence can
/// say the place and the edits are safe.
pub fn saved_after_trouble() -> bool {
    SAVED_AFTER_TROUBLE.load(Ordering::Acquire)
}

/// True once the graphics failure was reported.
pub fn graphics_failure_reported() -> bool {
    GRAPHICS_REPORTED.load(Ordering::Acquire)
}

/// Adds to the panic hook: every panic goes to `textweaver.log` and to the
/// `--log-file` log; the graphics failure is said in words at once, to the
/// terminal or in a message box ([`crate::console::report_error`]),
/// before the default hook prints the details.
pub fn install_panic_hook(messages: StartupMessages, background: bool) {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = info.to_string();
        log::error!("panic: {text}");
        if crate::log::to_file_active() {
            crate::log::line(&format!("panic: {text}"));
        }
        if is_graphics_failure(&text) && !GRAPHICS_REPORTED.swap(true, Ordering::AcqRel) {
            log::error!("{}", messages.graphics);
            crate::console::report_error(&messages.graphics, background);
        }
        crate::console::attach();
        default(info);
    }));
}

/// Saves what can be saved when the window stops without closing (a panic,
/// the session ending): the recovery copy of unsaved edits, the place, and
/// the settings, through [`App::emergency_save`]. A second panic while
/// saving is caught and logged. Returns true when the save ran through.
pub fn save_after_trouble(app: &mut App) -> bool {
    let saved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.emergency_save()));
    match saved {
        Ok(()) => {
            log::warn!("the window saved the place and any unsaved edits before stopping");
            true
        }
        Err(_) => {
            log::error!("saving after an internal error failed as well");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_app::Command;
    use textweaver_app::a11y::LogAnnouncer;
    use textweaver_app::keymap::ActionId;

    use crate::setup::{self, Options};

    /// Each catalog has both sentences, and the graphics one names the
    /// terminal reader.
    #[test]
    fn every_catalog_has_the_messages() {
        for lang in ["en", "de", "fr", "es", "pt", "ar"] {
            let (catalog, _) = Catalog::for_language(lang, None);
            let m = StartupMessages::from_catalog(&catalog);
            assert!(!m.graphics.contains("gui-graphics-failed"), "{lang}");
            assert!(!m.crashed.contains("gui-crashed"), "{lang}");
            assert!(!m.crashed_saved.contains("gui-crashed"), "{lang}");
            assert!(m.graphics.contains("textweaver"), "{lang}: {}", m.graphics);
            assert!(m.crashed.contains("textweaver"), "{lang}: {}", m.crashed);
        }
        let (en, _) = Catalog::for_language("en", None);
        let m = StartupMessages::from_catalog(&en);
        assert_eq!(
            m.graphics,
            "The window could not start its graphics. The terminal reader, textweaver, needs none."
        );
        assert_eq!(
            m.after_crash(false),
            "textweaver stopped after an internal error."
        );
        assert!(m.after_crash(true).starts_with("textweaver stopped"));
        assert!(
            m.after_crash(true)
                .ends_with("offered for recovery next time.")
        );
    }

    #[test]
    fn the_graphics_failure_is_recognized() {
        let text = format!(
            "panicked at event_loop_runner.rs:1216:6:\n{}: no compatible WGPU device found",
            masonry_winit::app::GRAPHICS_FAILURE
        );
        assert!(is_graphics_failure(&text));
        assert!(!is_graphics_failure("index out of bounds"));
    }

    /// A save after trouble leaves the recovery copy of unsaved edits,
    /// which the next start offers, and leaves the file as it was.
    #[test]
    fn a_save_after_trouble_leaves_a_recovery_copy() {
        let home = tempfile::tempdir().expect("temp dir");
        let opts = Options {
            no_speech: true,
            home: Some(home.path().to_path_buf()),
            ..Options::default()
        };
        let (mut app, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
        let file = home.path().join("ada-example-notes.md");
        std::fs::write(&file, "Original.\n").expect("write the notes");
        app.open(&file).expect("the notes open");
        let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
        let _ = app.dispatch(Command::Insert("Unsaved ".into()));
        assert!(save_after_trouble(&mut app));
        drop(app);
        assert_eq!(std::fs::read_to_string(&file).expect("read"), "Original.\n");
        let recovery = Paths::under(home.path()).recovery_dir();
        let copies = std::fs::read_dir(&recovery).map(|d| d.count()).unwrap_or(0);
        assert!(copies > 0, "no recovery copy in {}", recovery.display());
    }
}
