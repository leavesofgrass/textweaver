//! The speech backends this build offers: the speech crate's built-ins plus
//! the engines that live in their own crates: ETI-Eloquence through ECI
//! (ADR-0007), Apple's system voices on macOS (ADR-0008), and SAPI5 voices
//! on Windows (ADR-0009). Every frontend selects from this one registry, so
//! `tw backends`, `tw speak`, and the reader agree.
//!
//! The engines' options come from the settings (`[speech.eci]`,
//! `[speech.sapi]`, `[speech.apple]`, [`speech_registry_for`]), and so does
//! the speech service's configuration ([`service_config`]): voice, rate,
//! pitch, volume, pacing, punctuation, normalization, and the community
//! lexicon (`[normalization.community_lexicon]`).

use std::path::PathBuf;

use textweaver_eci::EciConfig;
use textweaver_eci::dictionaries::Dictionaries;
use textweaver_speech::normalize::CommunityLexiconConfig;
use textweaver_speech::pacing::PacingConfig;
use textweaver_speech::{
    BackendRegistry, NormalizeConfig, ServiceConfig, SpeechBackend, TableMode, VoiceParams,
};
use textweaver_store::{EciDictionaries, Settings, TableMode as StoreTableMode};

/// Where Code Factory's "Eloquence for Windows" installs its engine. It is
/// used only when `[speech.eci] code_factory = true` (off by default: an
/// installed copy is not necessarily licensed for other programs,
/// ADR-0007).
pub const CODE_FACTORY_LIBRARY: &str =
    r"C:\Program Files (x86)\Code Factory\Eloquence for Windows\eci.dll";

/// The ECI backend's options from `[speech.eci]`. The environment
/// variables (`TEXTWEAVER_ECI_LIBRARY`, `TEXTWEAVER_ECI_DICTIONARIES`,
/// `TEXTWEAVER_ECI_CODE_FACTORY`) still win for one run: discovery consults
/// them first.
pub fn eci_config(settings: &Settings) -> EciConfig {
    let eci = &settings.speech.eci;
    let library = eci.library.clone().or_else(|| {
        (eci.code_factory && cfg!(windows)).then(|| PathBuf::from(CODE_FACTORY_LIBRARY))
    });
    EciConfig {
        library,
        dictionaries: match &eci.dictionaries {
            EciDictionaries::On => Dictionaries::Auto,
            EciDictionaries::Off => Dictionaries::Off,
            EciDictionaries::Path(p) => Dictionaries::Dir(p.clone()),
        },
        ..EciConfig::default()
    }
}

/// The SAPI backend's options from `[speech.sapi]`.
pub fn sapi_config(settings: &Settings) -> textweaver_sapi::SapiConfig {
    textweaver_sapi::SapiConfig {
        onecore: settings.speech.sapi.onecore,
        ..textweaver_sapi::SapiConfig::default()
    }
}

/// The registry of every backend compiled into this build, with default
/// engine options.
///
/// Automatic selection order: `eci` (1000), then `sapi` (500, Windows),
/// then `nsspeech` (80) and `avspeech` (70) on macOS, then the built-in
/// engines. An installed ETI-Eloquence is the automatic
/// choice; it is available only when discovery finds a library the user
/// installed (`textweaver_eci::discovery`).
pub fn speech_registry() -> BackendRegistry {
    speech_registry_for(&Settings::default())
}

/// The registry with the engines configured from `settings`: the ECI
/// library, dictionaries, and Code Factory switch (`[speech.eci]`), SAPI's
/// OneCore voices (`[speech.sapi]`), and which Apple engine is tried first
/// on macOS (`[speech.apple] backend`; the other stays available below it).
pub fn speech_registry_for(settings: &Settings) -> BackendRegistry {
    let mut registry = BackendRegistry::with_builtins();
    let eci = eci_config(settings);
    let probe_config = eci.clone();
    // Each engine is looked for at most once per process and configuration
    // (the registry caches the probe); describing it probes nothing.
    registry.register_cached(
        textweaver_eci::backend_description(),
        format!("{eci:?}"),
        move || textweaver_eci::probe(&probe_config),
        move || {
            textweaver_eci::EciBackend::new(eci.clone())
                .map(|b| Box::new(b) as Box<dyn SpeechBackend>)
        },
    );
    let preferred_apple = apple_preference(settings);
    let top_apple = textweaver_apple::backends()
        .iter()
        .map(|b| b.priority)
        .max()
        .unwrap_or(0);
    for mut info in textweaver_apple::backends() {
        let id = info.id;
        if preferred_apple == Some(id) {
            info.priority = top_apple + 1;
        }
        registry.register(
            info,
            textweaver_apple::available,
            move || match textweaver_apple::factory(id) {
                Some(make) => make(),
                None => Err(textweaver_speech::SpeechError::Unavailable(
                    id,
                    "Apple speech is only available on macOS".into(),
                )),
            },
        );
    }
    #[cfg(windows)]
    {
        let sapi = sapi_config(settings);
        let probe = sapi.clone();
        registry.register_cached(
            textweaver_sapi::backend_description(),
            format!("{sapi:?}"),
            move || textweaver_sapi::probe(&probe),
            move || (textweaver_sapi::factory(sapi.clone()))(),
        );
    }
    // DECtalk (ADR-0021): only a copy the user installed, below SAPI.
    let dectalk = dectalk_config(settings);
    let probe = dectalk.clone();
    registry.register_cached(
        textweaver_dectalk::backend_description(),
        format!("{dectalk:?}"),
        move || textweaver_dectalk::probe(&probe),
        move || (textweaver_dectalk::factory(dectalk.clone()))(),
    );
    // Piper neural voices (ADR-0023), in-process on RTen: available once a
    // voice is installed, below DECtalk and above the built-in engines.
    let piper = piper_config(settings);
    let probe = piper.clone();
    registry.register_cached(
        textweaver_piper::backend_description(),
        format!("{piper:?}"),
        move || textweaver_piper::probe(&probe),
        textweaver_piper::factory(piper),
    );
    registry
}

/// The Piper backend's options: `[speech.piper]` `voices` (the voices
/// folder, default `<data>/piper/voices`), `voice` (the voice to start
/// with), and `phonemizer` (`auto`, `library`, or `rust`), kept as an
/// unknown section until the store gains a typed one.
/// `TEXTWEAVER_PIPER_VOICES` names the voices folder for one run.
pub fn piper_config(settings: &Settings) -> textweaver_piper::PiperConfig {
    let section = settings.speech.extra.get("piper");
    let get = |key: &str| {
        section
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    };
    let data_dir = textweaver_store::Paths::platform()
        .map(|p| p.data_dir)
        .unwrap_or_else(|_| std::env::temp_dir().join("textweaver"));
    let mut config = textweaver_piper::PiperConfig::in_data_dir(&data_dir);
    if let Some(dir) = std::env::var_os("TEXTWEAVER_PIPER_VOICES").filter(|v| !v.is_empty()) {
        config.voices_dir = PathBuf::from(dir);
    } else if let Some(dir) = get("voices") {
        config.voices_dir = PathBuf::from(dir);
    }
    config.default_voice = get("voice").map(str::to_owned);
    config.phonemizer = match get("phonemizer") {
        Some("library") => textweaver_piper::PhonemizerChoice::Library,
        Some("rust") => textweaver_piper::PhonemizerChoice::Rust,
        _ => textweaver_piper::PhonemizerChoice::Auto,
    };
    config
}

/// The DECtalk backend's options: `library` from `[speech.dectalk]` (kept
/// as an unknown section until the store gains a typed one).
pub fn dectalk_config(settings: &Settings) -> textweaver_dectalk::DectalkConfig {
    let library = settings
        .speech
        .extra
        .get("dectalk")
        .and_then(|t| t.get("library"))
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from);
    textweaver_dectalk::DectalkConfig {
        library,
        ..textweaver_dectalk::DectalkConfig::default()
    }
}

/// Whether `settings` pin an Apple engine (`[speech.apple] backend`).
pub fn apple_preference(settings: &Settings) -> Option<&'static str> {
    settings.speech.apple.backend.backend_id()
}

/// The speech service configuration the settings describe.
pub fn service_config(settings: &Settings) -> ServiceConfig {
    let sp = &settings.speech;
    let norm = &settings.normalization;
    let lexicon = &norm.community_lexicon;
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
            math_verbosity: norm.math_verbosity,
            asciimath_delimiter: norm.asciimath_delimiter,
            community_lexicon: CommunityLexiconConfig {
                enabled: lexicon.enabled,
                dir: lexicon.dir.clone(),
                language: lexicon.language.clone(),
            },
            ..NormalizeConfig::default()
        },
        caps: sp.caps,
        prefer_voice: sp.prefer_voice.clone().filter(|p| !p.is_empty()),
        ..ServiceConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use textweaver_store::AppleBackend;

    use super::*;

    #[test]
    fn eloquence_is_registered_above_the_builtins() {
        let list = speech_registry().list();
        let eci = list.iter().find(|b| b.id == "eci").expect("eci registered");
        assert!(
            list.iter()
                .filter(|b| b.id != "eci")
                .all(|b| b.priority < eci.priority)
        );
    }

    #[test]
    fn engine_settings_map_onto_engine_options() {
        // Defaults are the engines' own defaults.
        let s = Settings::default();
        assert_eq!(eci_config(&s), EciConfig::default());
        assert!(sapi_config(&s).onecore);
        assert_eq!(apple_preference(&s), None);

        let mut s = Settings::default();
        s.speech.eci.library = Some(PathBuf::from("/opt/eci/libibmeci.so"));
        s.speech.eci.dictionaries = EciDictionaries::Path(PathBuf::from("/dicts"));
        s.speech.sapi.onecore = false;
        s.speech.apple.backend = AppleBackend::AvSpeech;
        let eci = eci_config(&s);
        assert_eq!(eci.library, Some(PathBuf::from("/opt/eci/libibmeci.so")));
        assert_eq!(eci.dictionaries, Dictionaries::Dir(PathBuf::from("/dicts")));
        assert!(!sapi_config(&s).onecore);
        assert_eq!(apple_preference(&s), Some("avspeech"));

        // Dictionaries off; Code Factory only names its library (never
        // loaded here) on Windows, and an explicit library wins.
        s.speech.eci.dictionaries = EciDictionaries::Off;
        s.speech.eci.library = None;
        s.speech.eci.code_factory = true;
        let eci = eci_config(&s);
        assert_eq!(eci.dictionaries, Dictionaries::Off);
        assert_eq!(
            eci.library,
            cfg!(windows).then(|| PathBuf::from(CODE_FACTORY_LIBRARY))
        );
        s.speech.eci.library = Some(PathBuf::from("mine.dll"));
        assert_eq!(eci_config(&s).library, Some(PathBuf::from("mine.dll")));
    }

    #[test]
    fn the_preferred_apple_engine_is_tried_first() {
        let mut s = Settings::default();
        s.speech.apple.backend = AppleBackend::AvSpeech;
        let list = speech_registry_for(&s).list();
        let prio = |id: &str| list.iter().find(|b| b.id == id).map(|b| b.priority);
        if textweaver_apple::available() {
            assert!(prio("avspeech") > prio("nsspeech"));
        } else {
            assert_eq!(prio("avspeech"), None, "Apple speech is macOS only");
        }
    }

    #[test]
    fn service_config_maps_normalization_and_the_community_lexicon() {
        let mut s = Settings::default();
        s.normalization.numbers = false;
        s.normalization.table_mode = StoreTableMode::Flat;
        s.normalization.community_lexicon.enabled = true;
        s.normalization.community_lexicon.language = "DEU".into();
        s.normalization.community_lexicon.dir = Some(PathBuf::from("/dicts"));
        s.speech.latency_offset_ms = 80;
        let c = service_config(&s);
        assert!(!c.normalize.numbers);
        assert_eq!(c.normalize.table_mode, TableMode::Flat);
        assert!(c.normalize.community_lexicon.enabled);
        assert_eq!(c.normalize.community_lexicon.language, "DEU");
        assert_eq!(
            c.normalize.community_lexicon.dir,
            Some(PathBuf::from("/dicts"))
        );
        assert_eq!(
            c.pacing.latency_offset,
            std::time::Duration::from_millis(80)
        );
        assert_eq!(c.prefer_voice.as_deref(), Some("eloquence"));
        // Off by default, as in the speech crate.
        let d = service_config(&Settings::default());
        assert_eq!(
            d.normalize.community_lexicon,
            CommunityLexiconConfig::default()
        );
    }

    #[test]
    fn dectalk_is_registered_below_sapi_and_eloquence() {
        let list = speech_registry().list();
        let dectalk = list
            .iter()
            .find(|b| b.id == "dectalk")
            .expect("dectalk registered");
        assert_eq!(dectalk.priority, 300);
        for above in ["eci", "sapi"] {
            if let Some(b) = list.iter().find(|b| b.id == above) {
                assert!(b.priority > dectalk.priority, "{above}");
            }
        }
    }

    #[test]
    fn service_config_maps_math_speech() {
        let d = service_config(&Settings::default());
        let speech_defaults = NormalizeConfig::default();
        assert_eq!(d.normalize.math_verbosity, speech_defaults.math_verbosity);
        assert_eq!(d.normalize.asciimath_delimiter, None);
        let mut s = Settings::default();
        s.normalization.math_verbosity = textweaver_core::Verbosity::Low;
        s.normalization.asciimath_delimiter = Some('`');
        let c = service_config(&s);
        assert_eq!(c.normalize.math_verbosity, textweaver_core::Verbosity::Low);
        assert_eq!(c.normalize.asciimath_delimiter, Some('`'));
    }
}
