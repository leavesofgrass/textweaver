//! Building the app for the terminal: paths, settings, keymap, speech.

use std::path::PathBuf;

use textweaver_app::a11y::{Announcer, LogAnnouncer};
use textweaver_app::keymap::{Frontend, Keymap, Platform};
use textweaver_app::speech::{ServiceConfig, SpeechService};
use textweaver_app::store::{Paths, Settings, SettingsStore};
use textweaver_app::{App, AppConfig};

/// Startup options (from the command line).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Run silently: no self-voicing, the null speech backend.
    pub no_speech: bool,
    /// Speech backend id; overrides the settings.
    pub backend: Option<String>,
    /// Keep all state under this directory (like `TEXTWEAVER_HOME`).
    pub home: Option<PathBuf>,
    /// Theme name for this run (not saved).
    pub theme: Option<String>,
    /// Log level from `--log` (`off`, `error`, `warn`, `info`, `debug`,
    /// `trace`); `None` uses `TEXTWEAVER_LOG`, else warnings and errors.
    pub log: Option<String>,
}

/// Starts the log file (`textweaver.log` in the state directory) at the
/// level `--log` or `TEXTWEAVER_LOG` asks for. Returns a message for the
/// user when the level name is unknown.
pub fn start_log(opts: &Options) -> Option<String> {
    let env = std::env::var("TEXTWEAVER_LOG").ok();
    let (level, message) =
        match textweaver_app::logfile::choose_level(opts.log.as_deref(), env.as_deref()) {
            Ok(level) => (level, None),
            Err(msg) => (textweaver_app::logfile::DEFAULT_LEVEL, Some(msg)),
        };
    let paths = match &opts.home {
        Some(home) => Some(Paths::under(home)),
        None => Paths::platform().ok(),
    };
    if let Some(paths) = paths {
        textweaver_app::logfile::init(&paths, level);
    }
    message
}

/// The speech configuration the settings describe (the app's
/// [`textweaver_app::service_config`], shared by every frontend).
pub fn service_config(settings: &Settings) -> ServiceConfig {
    textweaver_app::service_config(settings)
}

/// Starts the speech service: the requested backend if available, else the
/// automatic choice (an unavailable explicit backend falls back to auto,
/// not straight to silence), else silence. Returns the service, the backend
/// id, and messages for the user.
pub fn start_speech(settings: &Settings, opts: &Options) -> (SpeechService, String, Vec<String>) {
    let mut messages = Vec::new();
    if opts.no_speech {
        return (SpeechService::null(), "silent".into(), messages);
    }
    let wanted = opts
        .backend
        .clone()
        .unwrap_or_else(|| settings.speech.backend.clone());
    let preference = (wanted != "auto" && !wanted.is_empty()).then_some(wanted.as_str());
    // Engine options from `[speech.eci]`, `[speech.sapi]`, `[speech.apple]`.
    let registry = textweaver_app::speech_registry_for(settings);
    let info = registry.select(preference).backend;
    if let Some(p) = preference
        && info.id != p
    {
        messages.push(format!(
            "Speech backend {p} is not available; using {}.",
            info.id
        ));
    }
    let spawned = registry
        .factory(info.id)
        .ok_or_else(|| format!("backend {} is not compiled in", info.id))
        .and_then(|factory| {
            SpeechService::spawn(factory, service_config(settings)).map_err(|e| e.to_string())
        });
    match spawned {
        Ok(service) => (service, info.id.to_owned(), messages),
        Err(e) => {
            messages.push(format!("Speech could not start ({e}); running silently."));
            (SpeechService::null(), "silent".into(), messages)
        }
    }
}

/// Builds the app from the options: persistence paths, settings, keymap
/// overrides, and speech. Returns messages to announce once it is running.
pub fn build_app(opts: &Options) -> (App, Vec<String>) {
    build_app_with(opts, Box::new(LogAnnouncer::default()))
}

/// [`build_app`] with a chosen announcer (the JSON-RPC server passes one
/// that forwards announcements to its client).
pub fn build_app_with(opts: &Options, announcer: Box<dyn Announcer>) -> (App, Vec<String>) {
    let mut messages = Vec::new();
    let paths = match &opts.home {
        Some(home) => Some(Paths::under(home)),
        None => match Paths::platform() {
            Ok(p) => Some(p),
            Err(e) => {
                messages.push(format!("Cannot save settings or positions: {e}."));
                None
            }
        },
    };
    let (mut settings, keymap) = match &paths {
        Some(p) => {
            let store = SettingsStore::new(p.clone());
            let (settings, msg) = store.load();
            messages.extend(msg);
            let overrides = store.load_keymap().unwrap_or_else(|e| {
                messages.push(format!("Keymap file ignored: {e}."));
                Default::default()
            });
            let (keymap, warnings) =
                Keymap::with_overrides(Platform::current(), Frontend::Terminal, &overrides);
            messages.extend(warnings);
            (settings, keymap)
        }
        None => (
            Settings::default(),
            Keymap::defaults(Platform::current(), Frontend::Terminal),
        ),
    };
    if let Some(t) = &opts.theme {
        settings.display.theme = t.clone();
    }
    let (speech, backend_name, speech_messages) = start_speech(&settings, opts);
    messages.extend(speech_messages);
    let self_voicing = !opts.no_speech && backend_name != "null" && backend_name != "silent";
    let mut app = App::new(AppConfig {
        settings,
        keymap,
        speech,
        paths,
        announcer,
        self_voicing,
        backend_name,
    });
    if opts.theme.is_none() {
        // Follow the system's light, dark, or high-contrast setting unless
        // the user picked a theme (display.follow_os_theme and
        // display.theme_explicit); the probe gives up after 500 ms.
        app.apply_startup_theme(textweaver_app::theme::os::probe());
    }
    (app, messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_speech_is_silent_and_uses_home() {
        let dir = tempfile::tempdir().unwrap();
        let opts = Options {
            no_speech: true,
            home: Some(dir.path().to_owned()),
            theme: Some("light".into()),
            ..Options::default()
        };
        let (app, messages) = build_app(&opts);
        assert!(messages.is_empty(), "{messages:?}");
        assert_eq!(app.backend_name(), "silent");
        assert_eq!(app.settings().display.theme, "light");
        assert_eq!(app.paths(), Some(&Paths::under(dir.path())));
    }

    /// `[keyboard] character_keys = false` is in effect from the first key
    /// press of a terminal session (App::new applies it).
    #[test]
    fn single_key_setting_applies_at_startup() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(Paths::under(dir.path()));
        let mut s = Settings::default();
        s.keyboard.character_keys = false;
        store.save(&s).unwrap();
        let opts = Options {
            no_speech: true,
            home: Some(dir.path().to_owned()),
            theme: Some("galaxy".into()),
            ..Options::default()
        };
        let (app, _) = build_app(&opts);
        assert!(!app.keymap().character_keys());
    }

    /// The only test in this binary that installs the global logger.
    #[test]
    fn the_log_goes_to_the_state_folder_and_a_bad_level_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let opts = Options {
            no_speech: true,
            home: Some(dir.path().to_owned()),
            log: Some("loud".into()),
            ..Options::default()
        };
        let msg = start_log(&opts).expect("an unknown level is reported");
        assert!(msg.contains("Unknown log level loud"), "{msg}");
        log::warn!("cannot save position: test");
        log::info!("below the default level");
        log::logger().flush();
        let path = textweaver_app::logfile::log_path(&Paths::under(dir.path()));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("cannot save position: test"), "{text}");
        assert!(!text.contains("below the default level"), "{text}");
    }

    #[test]
    fn unknown_backend_falls_back_to_auto() {
        let settings = Settings::default();
        let opts = Options {
            backend: Some("nonexistent".into()),
            ..Options::default()
        };
        let (_speech, id, messages) = start_speech(&settings, &opts);
        assert_ne!(id, "nonexistent");
        assert!(messages[0].contains("not available"));
    }
}
