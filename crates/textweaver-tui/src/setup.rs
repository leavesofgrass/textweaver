//! Building the app for the terminal: paths, settings, keymap, speech.

use std::path::PathBuf;

use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::keymap::{Frontend, Keymap, Platform};
use textweaver_app::speech::pacing::PacingConfig;
use textweaver_app::speech::{
    NormalizeConfig, ServiceConfig, SpeechService, TableMode, VoiceParams,
};
use textweaver_app::store::{Paths, Settings, SettingsStore, TableMode as StoreTableMode};
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
}

/// The speech configuration the settings describe.
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
            latency_offset: std::time::Duration::from_millis(u64::from(sp.latency_offset_ms)),
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

/// Builds the app from the options: persistence paths, settings, keymap
/// overrides, and speech. Returns messages to announce once it is running.
pub fn build_app(opts: &Options) -> (App, Vec<String>) {
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
    let app = App::new(AppConfig {
        settings,
        keymap,
        speech,
        paths,
        announcer: Box::new(LogAnnouncer::default()),
        self_voicing,
        backend_name,
    });
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
