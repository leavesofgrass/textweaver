//! Building the app for the terminal: paths, settings, keymap, speech.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use textweaver_app::a11y::{AccessMode, Announcer, RingAnnouncer};
use textweaver_app::keymap::{Frontend, Keymap, Platform};
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::speech::{BackendRegistry, ServiceConfig, SpeechService};
use textweaver_app::store::{Paths, Settings, SettingsStore};
use textweaver_app::{App, AppConfig};

/// Startup options (from the command line).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Run silently: no self-voicing, the null speech backend, and
    /// screen-reader mode for this run.
    pub no_speech: bool,
    /// Accessibility mode for this run (`--mode`), not saved; `None` uses
    /// `[accessibility] mode`, or screen-reader mode with `no_speech`.
    pub mode: Option<AccessMode>,
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

/// The interface's catalog for `settings` before the app exists (the app
/// builds the same one from the same language and locales folder), so
/// startup messages are in the listener's language.
fn startup_catalog(settings: &Settings, paths: Option<&Paths>) -> Arc<Catalog> {
    let dir = paths.map(Paths::locales_dir);
    Catalog::for_language(&settings.interface.language, dir.as_deref()).0
}

/// [`startup_catalog`] under the state folder `opts` chooses.
fn options_catalog(settings: &Settings, opts: &Options) -> Arc<Catalog> {
    let paths = match &opts.home {
        Some(home) => Some(Paths::under(home)),
        None => Paths::platform().ok(),
    };
    startup_catalog(settings, paths.as_ref())
}

/// The speech configuration the settings describe (the app's
/// [`textweaver_engines::service_config`], shared by every frontend).
pub fn service_config(settings: &Settings) -> ServiceConfig {
    textweaver_engines::service_config(settings)
}

/// Starts the speech service: the requested backend if available, else the
/// automatic choice (an unavailable explicit backend falls back to auto,
/// not straight to silence), else silence. Returns the service, the backend
/// id, and messages for the user.
pub fn start_speech(settings: &Settings, opts: &Options) -> (SpeechService, String, Vec<String>) {
    // Engine options from `[speech.eci]`, `[speech.sapi]`, `[speech.apple]`.
    let registry = textweaver_engines::speech_registry_for(settings);
    start_speech_with(&registry, settings, opts)
}

/// [`start_speech`] choosing from `registry` (tests pass
/// [`BackendRegistry::test_doubles`], so no real engine starts).
pub fn start_speech_with(
    registry: &BackendRegistry,
    settings: &Settings,
    opts: &Options,
) -> (SpeechService, String, Vec<String>) {
    let mut messages = Vec::new();
    if opts.no_speech {
        return (SpeechService::null(), "silent".into(), messages);
    }
    let wanted = opts
        .backend
        .clone()
        .unwrap_or_else(|| settings.speech.backend.clone());
    let preference = (wanted != "auto" && !wanted.is_empty()).then_some(wanted.as_str());
    let info = registry.select(preference).backend;
    if let Some(p) = preference
        && info.id != p
    {
        let c = options_catalog(settings, opts);
        messages.push(c.fmt(
            "tui-setup-backend-unavailable",
            &args!["wanted" => p, "backend" => info.id],
        ));
    }
    let spawned = registry
        .factory(info.id)
        .ok_or_else(|| {
            options_catalog(settings, opts)
                .fmt("tui-setup-backend-not-built", &args!["backend" => info.id])
        })
        .and_then(|factory| {
            SpeechService::spawn(factory, service_config(settings)).map_err(|e| e.to_string())
        });
    match spawned {
        Ok(service) => (service, info.id.to_owned(), messages),
        Err(e) => {
            let c = options_catalog(settings, opts);
            messages.push(c.fmt("tui-setup-speech-failed", &args!["error" => e]));
            (SpeechService::null(), "silent".into(), messages)
        }
    }
}

/// Builds the app from the options: persistence paths, settings, keymap
/// overrides, and speech. Returns messages to announce once it is running.
///
/// Announcements are kept in a [`RingAnnouncer`] of the latest
/// [`RingAnnouncer::DEFAULT_CAPACITY`]: a log of every one grew for as long
/// as the reader ran.
pub fn build_app(opts: &Options) -> (App, Vec<String>) {
    build_app_with(opts, Box::new(RingAnnouncer::default()))
}

/// [`build_app`] with a chosen announcer (the JSON-RPC server passes one
/// that forwards announcements to its client).
pub fn build_app_with(opts: &Options, announcer: Box<dyn Announcer>) -> (App, Vec<String>) {
    let started = Instant::now();
    let mut messages = Vec::new();
    // Said once the settings (and so the language) are known.
    let mut paths_error = None;
    let paths = match &opts.home {
        Some(home) => Some(Paths::under(home)),
        None => match Paths::platform() {
            Ok(p) => Some(p),
            Err(e) => {
                paths_error = Some(e.to_string());
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
                let c = startup_catalog(&settings, Some(p));
                messages.push(c.fmt("tui-setup-keymap-ignored", &args!["error" => e.to_string()]));
                Default::default()
            });
            let (keymap, warnings) = textweaver_app::startup_keymap(
                &settings,
                Platform::current(),
                Frontend::Terminal,
                &overrides,
            );
            messages.extend(warnings);
            (settings, keymap)
        }
        None => {
            let settings = Settings::default();
            if let Some(e) = paths_error {
                let c = startup_catalog(&settings, None);
                messages.push(c.fmt("tui-setup-cannot-save", &args!["error" => e]));
            }
            let (keymap, _) = textweaver_app::startup_keymap(
                &settings,
                Platform::current(),
                Frontend::Terminal,
                &Default::default(),
            );
            (settings, keymap)
        }
    };
    if let Some(t) = &opts.theme {
        settings.display.theme = t.clone();
    }
    log::debug!(
        "startup: settings and keys read in {} ms",
        started.elapsed().as_millis()
    );
    // Reading the system's light, dark, or high-contrast setting starts
    // two small processes on Windows (35 to 90 ms measured): only when it
    // can change the theme, and on a helper thread while the app is built.
    let os_scheme = follows_os_theme(&settings, opts)
        .then(|| {
            std::thread::Builder::new()
                .name("os-theme".into())
                .spawn(textweaver_app::theme::os::probe)
                .ok()
        })
        .flatten();
    // The engine starts on a helper thread (Wave 3): the reader is usable at
    // once, silent until the engine is ready, and `App::tick` swaps it in.
    // Messages about the engine (one that is not available) come then.
    // `--no-speech` and the silent backend start at once.
    let wanted = opts
        .backend
        .clone()
        .unwrap_or_else(|| settings.speech.backend.clone());
    let in_background = !opts.no_speech && wanted != "null";
    let (speech, backend_name, speech_messages) = if in_background {
        (SpeechService::null(), "starting".to_owned(), Vec::new())
    } else {
        start_speech(&settings, opts)
    };
    messages.extend(speech_messages);
    let self_voicing =
        in_background || (!opts.no_speech && backend_name != "null" && backend_name != "silent");
    let mut app = App::new(AppConfig {
        settings,
        keymap,
        speech,
        paths,
        announcer,
        self_voicing,
        backend_name,
    });
    if let Some(mode) = run_mode(opts) {
        app.set_access_mode_for_run(mode);
    }
    // Restarting speech in place (Restart Speech, and once automatically
    // after the speech thread dies) starts it as above, with the settings
    // current then.
    let run = opts.clone();
    let starter: textweaver_app::SpeechStarter =
        std::sync::Arc::new(move |s| start_speech(s, &run));
    if in_background {
        app.start_speech_in_background(starter);
    } else {
        app.set_speech_starter(starter);
    }
    // Follow the system's light, dark, or high-contrast setting unless the
    // user picked a theme (display.follow_os_theme and
    // display.theme_explicit); the probe gives up after 500 ms.
    if let Some(scheme) = os_scheme.and_then(|probe| probe.join().ok()) {
        app.apply_startup_theme(scheme);
    }
    log::debug!(
        "startup: reader built in {} ms",
        started.elapsed().as_millis()
    );
    (app, messages)
}

/// True when the system's color scheme can choose the theme at startup:
/// no `--theme` for this run, `[display] follow_os_theme` on, and no theme
/// picked by the user (`theme_explicit`). Otherwise the probe is skipped.
fn follows_os_theme(settings: &Settings, opts: &Options) -> bool {
    opts.theme.is_none() && settings.display.follow_os_theme && !settings.display.theme_explicit
}

/// True on the first run under these paths: no settings or keymap file,
/// and no saved document state. Quitting with a document open writes its
/// state, so the welcome is not heard again after that.
pub fn is_first_run(paths: &Paths) -> bool {
    let state_empty = std::fs::read_dir(paths.state_dir())
        .map(|mut d| d.next().is_none())
        .unwrap_or(true);
    !paths.settings_file().exists() && !paths.keymap_file().exists() && state_empty
}

/// The welcome said once, on the first run: the five keys that get a new
/// user reading, named from the keymap in effect. The keys are marked
/// ([`textweaver_app::named_key_in`]): the status line shows "Ctrl+Q", the
/// voice says "Control Q".
pub fn welcome_text(c: &Catalog, keymap: &Keymap) -> String {
    textweaver_app::welcome_text(c, keymap)
}

/// The welcome for this run, when it is the first one under the state
/// folder `opts` chooses ([`is_first_run`]); `None` otherwise, and when
/// nothing is persisted.
pub fn first_run_message(c: &Catalog, opts: &Options, keymap: &Keymap) -> Option<String> {
    let paths = match &opts.home {
        Some(home) => Paths::under(home),
        None => Paths::platform().ok()?,
    };
    is_first_run(&paths).then(|| welcome_text(c, keymap))
}

/// What the reader says when it starts without a document, naming the
/// keys from the keymap in effect (marked, as in [`welcome_text`]).
pub fn no_document_text(c: &Catalog, keymap: &Keymap) -> String {
    use textweaver_app::keymap::ActionId;
    let k = |a| textweaver_app::named_key_in(c, keymap, a);
    c.fmt(
        "tui-setup-no-document",
        &args![
            "open" => k(ActionId::Open),
            "new" => k(ActionId::NewDocument),
            "help" => k(ActionId::Help)
        ],
    )
}

/// The accessibility mode the command line asks for: `--mode`, else
/// screen-reader mode for `--no-speech` (textweaver has no voice).
pub fn run_mode(opts: &Options) -> Option<AccessMode> {
    opts.mode
        .or_else(|| opts.no_speech.then_some(AccessMode::ScreenReader))
}

/// On a run where the mode was never chosen, checks for a screen reader
/// and, when one is running, asks once whether to use hybrid mode
/// ([`App::offer_hybrid`]). Returns true when it asked.
pub fn offer_hybrid_if_screen_reader(app: &mut App, opts: &Options) -> bool {
    if run_mode(opts).is_some() || !app.hybrid_offer_due() {
        return false;
    }
    match textweaver_app::a11y::detect::detect() {
        Some(found) => app.offer_hybrid(&found),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_app::keymap::{ActionId, Layer, Preset};

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

    /// The system's color scheme is read only when it can choose the theme.
    #[test]
    fn the_os_theme_is_probed_only_when_it_can_apply() {
        let mut s = Settings::default();
        let none = Options::default();
        s.display.follow_os_theme = true;
        s.display.theme_explicit = false;
        assert!(follows_os_theme(&s, &none));
        let theme = Options {
            theme: Some("light".into()),
            ..Options::default()
        };
        assert!(!follows_os_theme(&s, &theme));
        s.display.theme_explicit = true;
        assert!(!follows_os_theme(&s, &none));
        s.display.theme_explicit = false;
        s.display.follow_os_theme = false;
        assert!(!follows_os_theme(&s, &none));
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

    /// `--no-speech` means screen-reader mode, `--mode` any mode; neither
    /// is saved, and the keyboard preset comes from the settings.
    #[test]
    fn modes_from_the_command_line_and_the_keyboard_preset() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(Paths::under(dir.path()));
        let mut s = Settings::default();
        s.keyboard.preset = textweaver_app::store::KeymapPreset::Classic;
        store.save(&s).unwrap();
        let opts = Options {
            no_speech: true,
            home: Some(dir.path().to_owned()),
            theme: Some("galaxy".into()),
            ..Options::default()
        };
        let (mut app, _) = build_app(&opts);
        assert_eq!(app.access_mode(), AccessMode::ScreenReader);
        assert_eq!(app.keymap().preset(), Preset::Classic);
        let k: textweaver_app::keymap::KeyChord = "k".parse().unwrap();
        assert_eq!(
            app.keymap().lookup(&k, Layer::Browse),
            Some(ActionId::ScrollUp)
        );
        // A mode given on the command line asks nothing at startup.
        assert!(!offer_hybrid_if_screen_reader(&mut app, &opts));
        app.shutdown();
        assert_eq!(
            store.load().0.accessibility.mode,
            textweaver_app::store::AccessMode::SelfVoicing,
            "a mode for this run is not saved"
        );
        let opts = Options {
            no_speech: false,
            backend: Some("null".into()),
            mode: Some(AccessMode::Hybrid),
            ..opts
        };
        let (app, _) = build_app(&opts);
        assert_eq!(app.access_mode(), AccessMode::Hybrid);
        assert_eq!(run_mode(&Options::default()), None);
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

    /// The welcome is said on the first run only: once a document's state
    /// or a settings file exists, it is not.
    #[test]
    fn the_welcome_is_for_the_first_run_only() {
        let dir = tempfile::tempdir().unwrap();
        let opts = Options {
            no_speech: true,
            home: Some(dir.path().to_owned()),
            ..Options::default()
        };
        let (app, _) = build_app(&opts);
        let welcome = first_run_message(&app.catalog(), &opts, app.keymap())
            .expect("a welcome on the first run");
        // Spoken by textweaver's voice, and written on the status line.
        let spoken = textweaver_app::spoken_text;
        let written = textweaver_app::written_text;
        assert_eq!(
            spoken(&welcome),
            "Welcome to textweaver. Control O opens a document. Space starts and pauses reading, and Escape stops. F2 lists every command. F1 opens the help."
        );
        assert_eq!(
            written(&welcome),
            "Welcome to textweaver. Ctrl+O opens a document. Space starts and pauses reading, and Escape stops. F2 lists every command. F1 opens the help."
        );
        let none = no_document_text(&app.catalog(), app.keymap());
        assert_eq!(
            spoken(&none),
            "No document is open. Press Control O to open one, Control N for a new one, or F1 for help."
        );
        assert_eq!(
            written(&none),
            "No document is open. Press Ctrl+O to open one, Ctrl+N for a new one, or F1 for help."
        );
        // A saved document state means a run happened before.
        let state = Paths::under(dir.path()).state_dir();
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join("essay.md-0123.json"), "{}").unwrap();
        assert_eq!(first_run_message(&app.catalog(), &opts, app.keymap()), None);
        // So does a settings file on its own.
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        assert!(is_first_run(&paths));
        SettingsStore::new(paths.clone())
            .save(&Settings::default())
            .unwrap();
        assert!(!is_first_run(&paths));
        // Nothing persisted: no welcome.
        assert_eq!(
            first_run_message(
                &app.catalog(),
                &Options {
                    home: Some(dir.path().to_owned()),
                    ..Options::default()
                },
                app.keymap()
            ),
            None
        );
    }

    #[test]
    fn unknown_backend_falls_back_to_auto() {
        let settings = Settings::default();
        let opts = Options {
            backend: Some("nonexistent".into()),
            ..Options::default()
        };
        let (_speech, id, messages) =
            start_speech_with(&BackendRegistry::test_doubles(), &settings, &opts);
        assert_eq!(id, "null");
        assert_eq!(
            messages,
            ["Speech engine nonexistent is not available; using null."]
        );
    }
}
