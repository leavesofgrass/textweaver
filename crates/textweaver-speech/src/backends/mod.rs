//! Engine implementations, the backend registry, and backend selection.
//!
//! Selection follows Star (`star/tts/manager/_selection.py`, Part 2 section 3
//! of `docs/star-parity.md`) with its bug B11 fixed:
//!
//! 1. An explicit preference (anything but `None`, `""`, or `"auto"`) wins
//!    when that backend is available.
//! 2. Otherwise, including when the preferred backend is unknown or
//!    unavailable, the highest-priority available backend that is not
//!    opt-in is chosen. (Star fell straight to silence here.)
//! 3. `null` is the final fallback.
//!
//! The [`BackendRegistry`] is extensible: crates outside this one (the
//! ETI-Eloquence backend in `textweaver-eci`) register their own entries with
//! [`BackendRegistry::register`], and the app selects from the combined
//! list. The free functions [`registry`], [`factory`], and [`select`] are
//! thin wrappers over [`BackendRegistry::with_builtins`].
//!
//! Built-in priorities (higher is tried first): `espeak` 50, `omnivox` 40,
//! `recording` 0 (opt-in, tests only), `null` lowest.

#[cfg(feature = "espeak")]
pub mod espeak;
mod null;
#[cfg(feature = "omnivox")]
pub mod omnivox;
pub mod recording;

use std::sync::Arc;

pub use null::NullBackend;
pub use recording::{Call, RecordingBackend, RecordingHandle, RecordingMode};
use serde::Serialize;

use crate::backend::{BackendFactory, BackendInfo, SpeechBackend, SpeechError, Voice};

/// A constructor for a backend, callable any number of times.
pub type BackendConstructor =
    Arc<dyn Fn() -> Result<Box<dyn SpeechBackend>, SpeechError> + Send + Sync + 'static>;

/// An availability probe: true when the backend can start on this machine.
pub type AvailabilityProbe = Arc<dyn Fn() -> bool + Send + Sync + 'static>;

/// One registered backend.
#[derive(Clone)]
pub struct RegisteredBackend {
    /// Description; `available` is ignored here and computed by `probe`.
    pub info: BackendInfo,
    /// Availability probe.
    pub probe: AvailabilityProbe,
    /// Constructor, run on the speech thread through a [`BackendFactory`].
    pub constructor: BackendConstructor,
}

impl std::fmt::Debug for RegisteredBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisteredBackend")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

/// The outcome of [`BackendRegistry::select`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Selection {
    /// The chosen backend.
    pub backend: BackendInfo,
    /// The explicit preference, if one was given (not `auto`).
    pub requested: Option<String>,
    /// True when an explicit preference could not be honored and automatic
    /// selection chose instead.
    pub fell_back: bool,
}

impl Selection {
    /// A sentence for the user when the preference was not honored, for
    /// example "Backend espeak is not available; using null."
    pub fn fallback_message(&self) -> Option<String> {
        if !self.fell_back {
            return None;
        }
        let req = self.requested.as_deref().unwrap_or("auto");
        Some(format!(
            "Backend {req} is not available; using {}.",
            self.backend.id
        ))
    }
}

/// Backends known to this build, with a way to construct each.
#[derive(Clone, Debug, Default)]
pub struct BackendRegistry {
    entries: Vec<RegisteredBackend>,
}

fn null_info() -> BackendInfo {
    BackendInfo {
        id: "null",
        name: "Silent (no audio)",
        priority: i32::MIN,
        opt_in: false,
        available: true,
    }
}

impl BackendRegistry {
    /// An empty registry. [`select`](Self::select) still falls back to
    /// `null`, and [`factory`](Self::factory) still builds it.
    pub fn new() -> Self {
        Self::default()
    }

    /// The built-in backends of this build: `null`, `recording`, and the
    /// compiled-in engines (`espeak`, `omnivox`).
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        r.register(
            null_info(),
            || true,
            || Ok(Box::new(NullBackend::default()) as Box<dyn SpeechBackend>),
        );
        r.register(
            BackendInfo {
                id: "recording",
                name: "Recording (test double)",
                priority: 0,
                opt_in: true,
                available: true,
            },
            || true,
            || Ok(Box::new(RecordingBackend::new().0) as Box<dyn SpeechBackend>),
        );
        #[cfg(feature = "espeak")]
        r.register(
            BackendInfo {
                id: "espeak",
                name: "eSpeak NG",
                priority: 50,
                opt_in: false,
                available: false,
            },
            espeak::available,
            || {
                espeak::EspeakBackend::new(espeak::EspeakOutput::from_env())
                    .map(|b| Box::new(b) as Box<dyn SpeechBackend>)
            },
        );
        #[cfg(feature = "omnivox")]
        r.register(
            BackendInfo {
                id: "omnivox",
                name: "Omnivox speech server",
                priority: 40,
                opt_in: false,
                available: false,
            },
            || omnivox::OmnivoxCommand::from_env().is_some(),
            || {
                let cmd = omnivox::OmnivoxCommand::from_env().ok_or_else(|| {
                    SpeechError::Unavailable("omnivox", "omnivox is not on PATH".into())
                })?;
                omnivox::OmnivoxBackend::spawn(&cmd).map(|b| Box::new(b) as Box<dyn SpeechBackend>)
            },
        );
        r
    }

    /// Adds a backend, replacing any entry with the same id.
    pub fn register(
        &mut self,
        info: BackendInfo,
        probe: impl Fn() -> bool + Send + Sync + 'static,
        constructor: impl Fn() -> Result<Box<dyn SpeechBackend>, SpeechError> + Send + Sync + 'static,
    ) {
        self.entries.retain(|e| e.info.id != info.id);
        self.entries.push(RegisteredBackend {
            info,
            probe: Arc::new(probe),
            constructor: Arc::new(constructor),
        });
    }

    /// Every registered backend with availability checked now, highest
    /// priority first.
    pub fn list(&self) -> Vec<BackendInfo> {
        let mut out: Vec<BackendInfo> = self
            .entries
            .iter()
            .map(|e| BackendInfo {
                available: (e.probe)(),
                ..e.info.clone()
            })
            .collect();
        out.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(b.id)));
        out
    }

    /// The entry for `id`, with availability checked now.
    pub fn get(&self, id: &str) -> Option<BackendInfo> {
        self.entries
            .iter()
            .find(|e| e.info.id == id)
            .map(|e| BackendInfo {
                available: (e.probe)(),
                ..e.info.clone()
            })
    }

    /// A factory for backend `id`, if registered (`null` always is).
    pub fn factory(&self, id: &str) -> Option<BackendFactory> {
        if let Some(e) = self.entries.iter().find(|e| e.info.id == id) {
            let make = Arc::clone(&e.constructor);
            return Some(Box::new(move || make()));
        }
        (id == "null").then(|| {
            Box::new(|| Ok(Box::new(NullBackend::default()) as Box<dyn SpeechBackend>))
                as BackendFactory
        })
    }

    /// Chooses a backend by Star's rules, with an unavailable explicit
    /// preference falling back to automatic selection (not to silence).
    pub fn select(&self, preferred: Option<&str>) -> Selection {
        select_from(&self.list(), preferred)
    }
}

/// Selection over an already probed list (pure; used by tests with fake
/// availability).
pub fn select_from(list: &[BackendInfo], preferred: Option<&str>) -> Selection {
    let requested = preferred
        .map(str::trim)
        .filter(|p| !p.is_empty() && !p.eq_ignore_ascii_case("auto"))
        .map(str::to_owned);
    if let Some(p) = requested.as_deref() {
        if let Some(b) = list.iter().find(|b| b.id == p && b.available) {
            return Selection {
                backend: b.clone(),
                requested,
                fell_back: false,
            };
        }
    }
    let auto = list
        .iter()
        .filter(|b| b.available && !b.opt_in)
        .max_by_key(|b| b.priority)
        .cloned()
        .unwrap_or_else(null_info);
    Selection {
        fell_back: requested.is_some(),
        backend: auto,
        requested,
    }
}

/// Every backend compiled into this build, with availability.
pub fn registry() -> Vec<BackendInfo> {
    BackendRegistry::with_builtins().list()
}

/// A factory for backend `id`, if compiled in.
pub fn factory(id: &str) -> Option<BackendFactory> {
    BackendRegistry::with_builtins().factory(id)
}

/// Picks a backend: `preferred` if available, else the highest-priority
/// available non-opt-in backend, else `null`.
pub fn select(preferred: Option<&str>) -> BackendInfo {
    BackendRegistry::with_builtins().select(preferred).backend
}

/// Resolves a preferred-voice substring (Star's `tts_prefer_voice`, default
/// "eloquence") against a voice list: voices whose lowercased
/// `name + " " + id` contains the preference; among them the first whose
/// language region is US, else the first match. Returns the voice id.
///
/// Star matched "us" as a substring of the language, so "rus" and "aus"
/// counted as US (quirk Q11); this checks the region subtag.
pub fn resolve_preferred_voice(voices: &[Voice], prefer: &str) -> Option<String> {
    let prefer = prefer.trim().to_lowercase();
    if prefer.is_empty() {
        return None;
    }
    let matches: Vec<&Voice> = voices
        .iter()
        .filter(|v| {
            format!("{} {}", v.name, v.id)
                .to_lowercase()
                .contains(&prefer)
        })
        .collect();
    let is_us = |v: &Voice| {
        v.languages.iter().any(|l| {
            l.split(['-', '_'])
                .skip(1)
                .any(|sub| sub.eq_ignore_ascii_case("us"))
        })
    };
    matches
        .iter()
        .find(|v| is_us(v))
        .or_else(|| matches.first())
        .map(|v| v.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(id: &'static str, priority: i32, available: bool, opt_in: bool) -> BackendInfo {
        BackendInfo {
            id,
            name: id,
            priority,
            opt_in,
            available,
        }
    }

    /// Star's selection tests (tests/test_tts.py:680-740) with Star's
    /// priorities translated to "higher first" (Star sorted ascending).
    fn star_list(avail: &[&str]) -> Vec<BackendInfo> {
        let a = |id: &str| avail.contains(&id);
        vec![
            info("applesay", 85, a("applesay"), false),
            info("pyttsx3", 80, a("pyttsx3"), false),
            info("dectalk", 75, a("dectalk"), false),
            info("qtspeech", 65, a("qtspeech"), false),
            info("festival", 40, a("festival"), false),
            info("piper", -10, a("piper"), true),
            null_info(),
        ]
    }

    #[test]
    fn star_selection_order() {
        let pick =
            |avail: &[&str], pref: Option<&str>| select_from(&star_list(avail), pref).backend.id;
        assert_eq!(pick(&[], None), "null");
        assert_eq!(pick(&["applesay", "pyttsx3"], None), "applesay");
        assert_eq!(pick(&["pyttsx3"], None), "pyttsx3");
        assert_eq!(pick(&["pyttsx3", "dectalk"], None), "pyttsx3");
        assert_eq!(pick(&["dectalk"], None), "dectalk");
        assert_eq!(pick(&["pyttsx3", "dectalk", "qtspeech"], None), "pyttsx3");
        assert_eq!(
            pick(&["pyttsx3", "dectalk", "qtspeech"], Some("qtspeech")),
            "qtspeech"
        );
        assert_eq!(pick(&["dectalk", "qtspeech"], None), "dectalk");
        assert_eq!(pick(&["qtspeech"], None), "qtspeech");
        assert_eq!(pick(&["festival"], Some("festival")), "festival");
        assert_eq!(pick(&["pyttsx3"], Some("null")), "null");
        assert_eq!(pick(&["pyttsx3"], Some("auto")), "pyttsx3");
    }

    #[test]
    fn opt_in_only_when_explicit() {
        let list = star_list(&["piper"]);
        assert_eq!(select_from(&list, None).backend.id, "null");
        assert_eq!(select_from(&list, Some("piper")).backend.id, "piper");
    }

    #[test]
    fn unavailable_preference_falls_back_to_auto_not_silence() {
        // Star bug B11: an unavailable explicit backend gave silence.
        let s = select_from(&star_list(&["pyttsx3"]), Some("piper"));
        assert_eq!(s.backend.id, "pyttsx3");
        assert!(s.fell_back);
        assert_eq!(
            s.fallback_message().unwrap(),
            "Backend piper is not available; using pyttsx3."
        );
        let s = select_from(&star_list(&["pyttsx3"]), Some("no-such-engine"));
        assert_eq!(s.backend.id, "pyttsx3");
    }

    #[test]
    fn registry_is_extensible() {
        let mut r = BackendRegistry::with_builtins();
        r.register(
            BackendInfo {
                id: "eci",
                name: "ETI-Eloquence",
                priority: 60,
                opt_in: false,
                available: false,
            },
            || true,
            || Ok(Box::new(NullBackend::default()) as Box<dyn SpeechBackend>),
        );
        assert_eq!(r.select(None).backend.id, "eci");
        assert_eq!(r.list()[0].id, "eci");
        assert!(r.factory("eci").is_some());
        assert!(r.factory("nope").is_none());
        // Re-registering replaces.
        r.register(
            BackendInfo {
                id: "eci",
                name: "ETI-Eloquence",
                priority: 60,
                opt_in: false,
                available: false,
            },
            || false,
            || Err(SpeechError::Unavailable("eci", "missing".into())),
        );
        assert_ne!(r.select(None).backend.id, "eci");
        assert_eq!(r.list().iter().filter(|b| b.id == "eci").count(), 1);
    }

    #[test]
    fn builtins_contain_null_and_recording() {
        let list = registry();
        assert!(list.iter().any(|b| b.id == "null" && b.available));
        assert!(list.iter().any(|b| b.id == "recording" && b.opt_in));
        assert!(factory("null").is_some());
        assert!(factory("recording").is_some());
        assert_ne!(select(None).id, "recording");
        assert_eq!(BackendRegistry::new().select(Some("x")).backend.id, "null");
        assert!(BackendRegistry::new().factory("null").is_some());
    }

    #[test]
    fn preferred_voice_resolution() {
        let v = |id: &str, name: &str, lang: &str| Voice {
            id: id.into(),
            name: name.into(),
            languages: vec![lang.into()],
            gender: None,
        };
        let voices = vec![
            v("eloq-gb", "Eloquence Reed", "en-GB"),
            v("eloq-ru", "Eloquence Russian", "ru-RU"),
            v("eloq-us", "Eloquence Reed", "en-US"),
            v("other", "Other", "en-US"),
        ];
        assert_eq!(
            resolve_preferred_voice(&voices, "eloquence").as_deref(),
            Some("eloq-us")
        );
        assert_eq!(
            resolve_preferred_voice(&voices[..2], "Eloquence").as_deref(),
            Some("eloq-gb"),
            "no US variant: first match; 'ru' is not 'us'"
        );
        assert_eq!(resolve_preferred_voice(&voices, ""), None);
        assert_eq!(resolve_preferred_voice(&voices, "dectalk"), None);
    }
}
