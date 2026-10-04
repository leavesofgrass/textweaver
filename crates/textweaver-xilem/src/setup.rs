//! Building the app for the GUI: paths, settings, the GUI keymap, speech.
//!
//! Ported from the wxDragon spike (`textweaver-gui`), which mirrors
//! `textweaver-tui`'s `setup.rs` (the TUI crate is not a dependency of the
//! GUI, so it does not pull in ratatui). The differences from the terminal
//! reader are the keymap frontend, the announcer (the live region, supplied
//! by the caller), and the silent `paced` backend for automated runs.
//!
//! The speech configuration, the engines' options, the background start,
//! and restarting speech are the terminal reader's (W6a5).

use std::path::PathBuf;

use std::sync::Arc;

use textweaver_app::a11y::Announcer;
use textweaver_app::core::Rate;
use textweaver_app::keymap::{Frontend, Keymap, Platform};
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::speech::{RecordingBackend, RecordingMode, ServiceConfig, SpeechService};
use textweaver_app::store::{Paths, Settings, SettingsStore};
use textweaver_app::{App, AppConfig};

/// The id of the GUI's silent, paced backend: the speech crate's recording
/// backend in timed mode, which emits word events on the audio clock at the
/// configured rate and plays nothing. Automated runs (the UI Automation
/// check, CI) use it to watch the caret follow speech without audio.
pub const PACED_BACKEND: &str = "paced";

/// Startup options (from the command line).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Run silently: the null speech backend.
    pub no_speech: bool,
    /// Speech backend id; overrides the settings. `paced` selects the
    /// silent paced backend.
    pub backend: Option<String>,
    /// Keep all state under this directory (like `TEXTWEAVER_HOME`).
    pub home: Option<PathBuf>,
    /// Speak textweaver's messages with its voice as well as sending them to
    /// the screen reader, for this run: the `[gui] speak_messages` switch,
    /// not saved.
    pub self_voicing: bool,
    /// Voice id or name for this run (see `tw voices`); not saved.
    pub voice: Option<String>,
}

/// The speech configuration the settings describe: the engines crate's,
/// shared by every frontend, so the window reads with the same settings as
/// the terminal (the community lexicon, math, DECtalk's options).
pub fn service_config(settings: &Settings) -> ServiceConfig {
    textweaver_app::engines::service_config(settings)
}

/// The `[gui] announce` setting ([`AnnounceMode`](crate::widgets::AnnounceMode)):
/// the live region (the default, ADR-0028) or UI Automation notifications.
/// The store checks the value; a bad one is reported with the other
/// settings warnings and reads as the live region.
pub fn announce_setting(settings: &Settings) -> crate::widgets::AnnounceMode {
    use crate::widgets::AnnounceMode;
    use textweaver_app::store::GuiAnnounce;
    match settings.gui.announce {
        GuiAnnounce::Live => AnnounceMode::Live,
        GuiAnnounce::Uia => AnnounceMode::Uia,
    }
}

/// Milliseconds per word at `rate`, for the paced backend.
fn ms_per_word(rate: Rate) -> u32 {
    let Rate::Wpm(wpm) = rate;
    60_000 / u32::from(wpm.max(1))
}

/// The interface catalog for messages made before the app exists: the
/// settings' language, with the state folder's own catalogs.
fn startup_catalog(settings: &Settings, opts: &Options) -> Arc<Catalog> {
    let paths = match &opts.home {
        Some(home) => Some(Paths::under(home)),
        None => Paths::platform().ok(),
    };
    let dir = paths.as_ref().map(Paths::locales_dir);
    Catalog::for_language(&settings.interface.language, dir.as_deref()).0
}

/// Starts the speech service: the paced backend when asked for, else the
/// requested backend if available (with the engines' options from
/// `[speech.eci]`, `[speech.sapi]`, `[speech.apple]`), else the automatic
/// choice, else silence. Returns the service, the backend id, and messages
/// for the user, in the interface language.
pub fn start_speech(settings: &Settings, opts: &Options) -> (SpeechService, String, Vec<String>) {
    let mut messages = Vec::new();
    if opts.no_speech {
        return (SpeechService::null(), "silent".into(), messages);
    }
    let wanted = opts
        .backend
        .clone()
        .unwrap_or_else(|| settings.speech.backend.clone());
    if wanted == PACED_BACKEND {
        let (backend, _handle) = RecordingBackend::with(
            RecordingMode::Timed {
                ms_per_word: ms_per_word(settings.speech.rate),
            },
            RecordingBackend::DEFAULT_CAPS,
        );
        return match SpeechService::spawn(backend.into_factory(), service_config(settings)) {
            Ok(service) => (service, PACED_BACKEND.into(), messages),
            Err(e) => {
                let c = startup_catalog(settings, opts);
                messages.push(c.fmt("tui-setup-speech-failed", &args!["error" => e.to_string()]));
                (SpeechService::null(), "silent".into(), messages)
            }
        };
    }
    let preference = (wanted != "auto" && !wanted.is_empty()).then_some(wanted.as_str());
    let registry = textweaver_app::engines::speech_registry_for(settings);
    let info = registry.select(preference).backend;
    if let Some(p) = preference
        && info.id != p
    {
        let c = startup_catalog(settings, opts);
        messages.push(c.fmt(
            "tui-setup-backend-unavailable",
            &args!["wanted" => p, "backend" => info.id],
        ));
    }
    let spawned = registry
        .factory(info.id)
        .ok_or_else(|| {
            startup_catalog(settings, opts)
                .fmt("tui-setup-backend-not-built", &args!["backend" => info.id])
        })
        .and_then(|factory| {
            SpeechService::spawn(factory, service_config(settings)).map_err(|e| e.to_string())
        });
    match spawned {
        Ok(service) => (service, info.id.to_owned(), messages),
        Err(e) => {
            let c = startup_catalog(settings, opts);
            messages.push(c.fmt("tui-setup-speech-failed", &args!["error" => e]));
            (SpeechService::null(), "silent".into(), messages)
        }
    }
}

/// True when the state folder `opts` chooses has no settings, no keys, and
/// no state yet: the first run, as the terminal reader decides it.
pub fn is_first_run(opts: &Options) -> bool {
    let paths = match &opts.home {
        Some(home) => Paths::under(home),
        None => match Paths::platform() {
            Ok(p) => p,
            Err(_) => return false,
        },
    };
    let state_empty = std::fs::read_dir(paths.state_dir())
        .map(|mut d| d.next().is_none())
        .unwrap_or(true);
    !paths.settings_file().exists() && !paths.keymap_file().exists() && state_empty
}

/// The welcome said once, on the first run: the five keys that get a new
/// user reading (open, play and pause, stop, the command palette, help),
/// the same words as the terminal reader's.
pub fn welcome_text(c: &Catalog, keymap: &Keymap) -> String {
    textweaver_app::welcome_text(c, keymap)
}

/// Builds the app: persistence paths, settings, the GUI keymap with the
/// user's overrides, and speech. Announcements go to `announcer` (the live
/// region). Returns messages to announce once the window is up.
///
/// As in the terminal reader, a real engine starts on a helper thread: the
/// window is usable at once, silent until the engine is ready, and
/// `App::tick` swaps it in. The same starter restarts speech in place
/// (Restart Speech, and once by itself when the speech thread dies).
/// `--no-speech` and the paced backend start at once.
pub fn build_app(opts: &Options, announcer: Box<dyn Announcer>) -> (App, Vec<String>) {
    build_app_following(opts, announcer, false)
}

/// True when the system's color scheme can choose the theme at startup:
/// `os_theme` (no `--theme` for this run), `[display] follow_os_theme` on,
/// and no theme picked by the user (`theme_explicit`). Otherwise the probe
/// is not run at all, as in the terminal reader.
pub fn follows_os_theme(settings: &Settings, os_theme: bool) -> bool {
    os_theme && settings.display.follow_os_theme && !settings.display.theme_explicit
}

/// [`build_app`], and with `os_theme` the theme the system's light, dark
/// or high-contrast setting picks ([`follows_os_theme`]). Reading that
/// setting starts small processes on Windows (35 to 90 ms measured for the
/// terminal reader), so it runs on a helper thread while the keys, speech
/// and app are built, and is joined at the end; a probe that fails or
/// panics leaves the saved theme (performance QW2, W8c-w).
pub fn build_app_following(
    opts: &Options,
    announcer: Box<dyn Announcer>,
    os_theme: bool,
) -> (App, Vec<String>) {
    let mut messages = Vec::new();
    let platform = Platform::current();
    let paths = match &opts.home {
        Some(home) => Some(Paths::under(home)),
        None => match Paths::platform() {
            Ok(p) => Some(p),
            Err(e) => {
                let c = startup_catalog(&Settings::default(), opts);
                messages.push(c.fmt("tui-setup-cannot-save", &args!["error" => e.to_string()]));
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
                let c = startup_catalog(&settings, opts);
                messages.push(c.fmt("tui-setup-keymap-ignored", &args!["error" => e.to_string()]));
                Default::default()
            });
            // The keyboard preset, then the overrides, as the terminal
            // reader builds it (W8a: the preset was left out here).
            let (keymap, warnings) =
                textweaver_app::startup_keymap(&settings, platform, Frontend::Gui, &overrides);
            messages.extend(warnings);
            (settings, keymap)
        }
        None => {
            let settings = Settings::default();
            let (keymap, _) = textweaver_app::startup_keymap(
                &settings,
                platform,
                Frontend::Gui,
                &Default::default(),
            );
            (settings, keymap)
        }
    };
    let os_scheme = follows_os_theme(&settings, os_theme)
        .then(|| {
            std::thread::Builder::new()
                .name("os-theme".into())
                .spawn(textweaver_app::theme::os::probe)
                .ok()
        })
        .flatten();
    if let Some(voice) = &opts.voice {
        settings.speech.voice = Some(voice.clone());
    }
    let wanted = opts
        .backend
        .clone()
        .unwrap_or_else(|| settings.speech.backend.clone());
    let in_background = !opts.no_speech && wanted != PACED_BACKEND && wanted != "null";
    let (speech, backend_name, speech_messages) = if in_background {
        (SpeechService::null(), "starting".to_owned(), Vec::new())
    } else {
        start_speech(&settings, opts)
    };
    messages.extend(speech_messages);
    let mut app = App::new(AppConfig {
        settings,
        keymap,
        speech,
        paths,
        announcer,
        self_voicing: false,
        backend_name,
    });
    // The window's two modes, "textweaver reads aloud" and "my screen
    // reader reads", with the switch that speaks messages ([gui]
    // speak_messages, or --self-voicing for this run). Before speech
    // starts in the background, which keeps the choice across a restart.
    app.use_window_modes(opts.self_voicing);
    // The voice manager's filters and fetch row are buttons in the window
    // (crate::voices), so its list holds only voices.
    app.set_voice_controls_in_list(false);
    let run = opts.clone();
    let starter: textweaver_app::SpeechStarter =
        std::sync::Arc::new(move |s| start_speech(s, &run));
    if in_background {
        app.start_speech_in_background(starter);
    } else {
        app.set_speech_starter(starter);
    }
    // The probe gives up after 500 ms; a panic in it keeps the saved theme.
    if let Some(scheme) = os_scheme.and_then(|probe| probe.join().ok()) {
        let _ = app.apply_startup_theme(scheme);
    }
    (app, messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_app::a11y::LogAnnouncer;

    #[test]
    fn paced_rate_is_one_word_per_beat() {
        assert_eq!(ms_per_word(Rate::Wpm(240)), 250);
        assert_eq!(ms_per_word(Rate::Wpm(0)), 60_000);
    }

    #[test]
    fn no_speech_is_silent_and_uses_home() {
        let dir = std::env::temp_dir().join(format!("tw-gui-setup-{}", std::process::id()));
        let opts = Options {
            no_speech: true,
            home: Some(dir.clone()),
            ..Options::default()
        };
        let (app, messages) = build_app(&opts, Box::new(LogAnnouncer::default()));
        assert!(messages.is_empty(), "{messages:?}");
        assert_eq!(app.backend_name(), "silent");
        assert_eq!(app.paths(), Some(&Paths::under(&dir)));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The system's color scheme is read only when it can change the theme:
    /// no `--theme`, following the system, and no theme the user picked
    /// (W8c-w, as the terminal reader decides it).
    #[test]
    fn the_os_theme_is_probed_only_when_it_can_apply() {
        let mut s = Settings::default();
        s.display.follow_os_theme = true;
        s.display.theme_explicit = false;
        assert!(follows_os_theme(&s, true));
        assert!(!follows_os_theme(&s, false), "--theme skips the probe");
        s.display.theme_explicit = true;
        assert!(!follows_os_theme(&s, true), "a picked theme skips it");
        s.display.theme_explicit = false;
        s.display.follow_os_theme = false;
        assert!(!follows_os_theme(&s, true), "not following skips it");
    }

    /// A theme the user picked stays, with the probe asked for: it is not
    /// run, and the saved theme is the one built.
    #[test]
    fn a_picked_theme_survives_the_startup_probe() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(Paths::under(dir.path()));
        let mut s = Settings::default();
        s.display.theme = "solarized-light".into();
        s.display.follow_os_theme = true;
        s.display.theme_explicit = true;
        store.save(&s).unwrap();
        let opts = Options {
            no_speech: true,
            home: Some(dir.path().to_path_buf()),
            ..Options::default()
        };
        let (app, _) = build_app_following(&opts, Box::new(LogAnnouncer::default()), true);
        assert_eq!(app.settings().display.theme, "solarized-light");
    }

    /// The keyboard preset from the settings is in effect from the start
    /// (W8a), as in the terminal reader, not only after an import or sync.
    #[test]
    fn the_keyboard_preset_applies_at_startup() {
        use textweaver_app::keymap::Preset;
        use textweaver_app::store::KeymapPreset;
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(Paths::under(dir.path()));
        let mut s = Settings::default();
        s.keyboard.preset = KeymapPreset::Classic;
        store.save(&s).unwrap();
        let opts = Options {
            no_speech: true,
            home: Some(dir.path().to_owned()),
            ..Options::default()
        };
        let (app, messages) = build_app(&opts, Box::new(LogAnnouncer::default()));
        assert!(messages.is_empty(), "{messages:?}");
        assert_eq!(app.keymap().preset(), Preset::Classic);
        let classic = Keymap::with_preset(Platform::current(), Frontend::Gui, Preset::Classic);
        assert_eq!(app.keymap().bindings(), classic.bindings());
        // Without a settings file, the default preset.
        let empty = tempfile::tempdir().unwrap();
        let opts = Options {
            no_speech: true,
            home: Some(empty.path().to_owned()),
            ..Options::default()
        };
        let (app, _) = build_app(&opts, Box::new(LogAnnouncer::default()));
        assert_eq!(app.keymap().preset(), Preset::Default);
    }

    #[test]
    fn announce_setting_reads_the_gui_table() {
        use crate::widgets::AnnounceMode;
        let read = |toml_text: &str| {
            let table: toml::Table = toml_text.parse().expect("toml");
            let (settings, warnings) = Settings::from_table(table);
            (announce_setting(&settings), warnings.len())
        };
        assert_eq!(read(""), (AnnounceMode::Live, 0));
        assert_eq!(read("[gui]\n"), (AnnounceMode::Live, 0));
        assert_eq!(read("[gui]\nannounce = \"uia\"\n"), (AnnounceMode::Uia, 0));
        assert_eq!(
            read("[gui]\nannounce = \"live\"\n"),
            (AnnounceMode::Live, 0)
        );
        // A bad value warns (with the other settings) and keeps the default.
        assert_eq!(
            read("[gui]\nannounce = \"loud\"\n"),
            (AnnounceMode::Live, 1)
        );
        assert_eq!(read("[gui]\nannounce = 3\n"), (AnnounceMode::Live, 1));
    }

    #[test]
    fn paced_backend_starts() {
        let opts = Options {
            backend: Some(PACED_BACKEND.into()),
            ..Options::default()
        };
        let (_speech, id, messages) = start_speech(&Settings::default(), &opts);
        assert_eq!(id, PACED_BACKEND);
        assert!(messages.is_empty(), "{messages:?}");
    }
}
