//! Building the app for the GUI: paths, settings, the GUI keymap, speech.
//!
//! Ported from the wxDragon spike (`textweaver-gui`), which mirrors
//! `textweaver-tui`'s `setup.rs` (the TUI crate is not a dependency of the
//! GUI, so it does not pull in ratatui). The differences from the terminal
//! reader are the keymap frontend, the announcer (the live region, supplied
//! by the caller), and the silent `paced` backend for automated runs.
//!
//! When Agent W3a's settings schema and non-blocking engine start land,
//! `start_speech` should use them (ADR-0023, "Waiting on W3a").

use std::path::PathBuf;
use std::time::Duration;

use textweaver_app::a11y::Announcer;
use textweaver_app::core::Rate;
use textweaver_app::keymap::{Frontend, Keymap, Platform};
use textweaver_app::speech::pacing::PacingConfig;
use textweaver_app::speech::{
    NormalizeConfig, RecordingBackend, RecordingMode, ServiceConfig, SpeechService, TableMode,
    VoiceParams,
};
use textweaver_app::store::{Paths, Settings, SettingsStore, TableMode as StoreTableMode};
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
    /// Speak announcements with the reading voice as well as sending them to
    /// the screen reader.
    pub self_voicing: bool,
    /// Voice id or name for this run (see `tw voices`); not saved.
    pub voice: Option<String>,
}

/// The speech configuration the settings describe (the same mapping as the
/// terminal UI's).
pub fn service_config(settings: &Settings) -> ServiceConfig {
    let sp = &settings.speech;
    let norm = &settings.normalization;
    ServiceConfig {
        params: VoiceParams {
            voice: sp.voice.clone(),
            rate: sp.rate,
            pitch: sp.pitch,
            volume: sp.volume,
        },
        pacing: PacingConfig {
            latency_offset: Duration::from_millis(u64::from(sp.latency_offset_ms)),
            highlight_speed: settings.highlight.speed,
            ..PacingConfig::default()
        },
        punctuation: sp.punctuation,
        split_caps: sp.split_caps,
        normalize: NormalizeConfig {
            skip_code: sp.skip_code,
            table_mode: match norm.table_mode {
                StoreTableMode::Structured => TableMode::Structured,
                StoreTableMode::Flat => TableMode::Flat,
                StoreTableMode::Skip => TableMode::Skip,
            },
            use_pronunciations: norm.use_pronunciations,
            pronunciations: norm.pronunciations.clone(),
            abbreviations: norm.abbreviations,
            abbrev_expansions: norm.abbrev_expansions.clone(),
            numbers: norm.numbers,
            math: norm.math,
            ..NormalizeConfig::default()
        },
        caps: sp.caps,
        prefer_voice: sp.prefer_voice.clone().filter(|p| !p.is_empty()),
        ..ServiceConfig::default()
    }
}

/// Milliseconds per word at `rate`, for the paced backend.
fn ms_per_word(rate: Rate) -> u32 {
    let Rate::Wpm(wpm) = rate;
    60_000 / u32::from(wpm.max(1))
}

/// Starts the speech service: the paced backend when asked for, else the
/// requested backend if available, else the automatic choice, else silence.
/// Returns the service, the backend id, and messages for the user.
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
                messages.push(format!("Speech could not start ({e}); running silently."));
                (SpeechService::null(), "silent".into(), messages)
            }
        };
    }
    let preference = (wanted != "auto" && !wanted.is_empty()).then_some(wanted.as_str());
    let registry = textweaver_app::speech_registry();
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

/// Builds the app: persistence paths, settings, the GUI keymap with the
/// user's overrides, and speech. Announcements go to `announcer` (the live
/// region). Returns messages to announce once the window is up.
pub fn build_app(opts: &Options, announcer: Box<dyn Announcer>) -> (App, Vec<String>) {
    let mut messages = Vec::new();
    let platform = Platform::current();
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
            let (keymap, warnings) = Keymap::with_overrides(platform, Frontend::Gui, &overrides);
            messages.extend(warnings);
            (settings, keymap)
        }
        None => (
            Settings::default(),
            Keymap::defaults(platform, Frontend::Gui),
        ),
    };
    if let Some(voice) = &opts.voice {
        settings.speech.voice = Some(voice.clone());
    }
    let (speech, backend_name, speech_messages) = start_speech(&settings, opts);
    messages.extend(speech_messages);
    let self_voicing = opts.self_voicing && backend_name != "silent";
    let app = App::new(AppConfig {
        settings,
        keymap,
        speech,
        paths,
        announcer,
        self_voicing,
        backend_name,
    });
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
