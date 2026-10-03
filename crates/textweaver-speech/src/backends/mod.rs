//! Engine implementations, the backend registry, and backend selection.
//!
//! Selection follows Star (`star/tts/manager/_selection.py`, Part 2 section 3
//! of the Star parity reference) with its bug B11 fixed:
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
//! Built-in priorities (higher is tried first): `espeak` 50, `speechd` 45,
//! `omnivox` 40, `recording` 0 (opt-in, tests only), `null` lowest.
//!
//! Availability probes run at most once per registry entry (clones share
//! the answer), and entries registered with a cache key
//! ([`BackendRegistry::register_cached`], which the built-ins and the app's
//! engines use) at most once per process for that key: `tw backends` and
//! the reader no longer look for the same engine twice.

#[cfg(feature = "espeak-phonemes")]
pub mod espeak;
mod null;
#[cfg(feature = "omnivox")]
pub mod omnivox;
pub mod recording;
#[cfg(feature = "speechd")]
pub mod speechd;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub use null::NullBackend;
pub use recording::{Call, RecordingBackend, RecordingHandle, RecordingMode};
use serde::Serialize;

use crate::backend::{BackendFactory, BackendId, BackendInfo, SpeechBackend, SpeechError, Voice};

/// The program `name` on `PATH` ([`textweaver_core::process::find_program`]).
#[cfg_attr(not(any(feature = "omnivox", feature = "speechd")), allow(dead_code))]
pub(crate) fn on_path(name: &str) -> Option<std::path::PathBuf> {
    textweaver_core::process::find_program(name)
}

/// A constructor for a backend, callable any number of times.
pub type BackendConstructor =
    Arc<dyn Fn() -> Result<Box<dyn SpeechBackend>, SpeechError> + Send + Sync + 'static>;

/// An availability probe: true when the backend can start on this machine.
pub type AvailabilityProbe = Arc<dyn Fn() -> bool + Send + Sync + 'static>;

/// Probe answers kept for the life of the process, by backend id and the
/// cache key given at registration.
fn process_probes() -> &'static Mutex<HashMap<(BackendId, String), bool>> {
    static PROBES: OnceLock<Mutex<HashMap<(BackendId, String), bool>>> = OnceLock::new();
    PROBES.get_or_init(Mutex::default)
}

/// Forgets every probe answer kept for the process, so the next listing
/// looks again (after the user installs an engine, or before restarting
/// speech). Registries already built keep the answers they have.
pub fn forget_probes() {
    process_probes()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

/// Wraps `probe` so it runs at most once: per entry (clones share it), and
/// with a `key`, at most once per process for that id and key.
fn once(
    id: BackendId,
    key: Option<String>,
    probe: impl Fn() -> bool + Send + Sync + 'static,
) -> AvailabilityProbe {
    let answer: Arc<OnceLock<bool>> = Arc::new(OnceLock::new());
    Arc::new(move || {
        *answer.get_or_init(|| {
            let Some(key) = &key else {
                return probe();
            };
            let cache_key = (id, key.clone());
            if let Some(&known) = process_probes()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&cache_key)
            {
                return known;
            }
            // Probe outside the lock: a slow probe must not hold up others.
            let found = probe();
            process_probes()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(cache_key, found);
            found
        })
    })
}

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
        caps: NullBackend::CAPS,
    }
}

impl BackendRegistry {
    /// An empty registry. [`select`](Self::select) still falls back to
    /// `null`, and [`factory`](Self::factory) still builds it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Only the test doubles, `null` and `recording`, whatever engines this
    /// build has. Nothing is probed or started, so nothing is ever heard:
    /// tests use this, because automatic selection from
    /// [`with_builtins`](Self::with_builtins) reaches a real engine that
    /// plays audio (and, on Windows, crashed the test process).
    pub fn test_doubles() -> Self {
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
                caps: RecordingBackend::DEFAULT_CAPS,
            },
            || true,
            || Ok(Box::new(RecordingBackend::new().0) as Box<dyn SpeechBackend>),
        );
        r
    }

    /// The built-in backends of this build: `null`, `recording`, and the
    /// compiled-in engines (`espeak`, `omnivox`, `speechd`).
    pub fn with_builtins() -> Self {
        #[cfg_attr(
            not(any(feature = "espeak", feature = "omnivox", feature = "speechd")),
            allow(unused_mut)
        )]
        let mut r = Self::test_doubles();
        #[cfg(feature = "espeak")]
        r.register_cached(
            BackendInfo {
                id: "espeak",
                name: "eSpeak NG",
                priority: 50,
                opt_in: false,
                available: false,
                caps: espeak::CAPS,
            },
            "builtin",
            espeak::available,
            || {
                espeak::EspeakBackend::new(espeak::EspeakOutput::from_env())
                    .map(|b| Box::new(b) as Box<dyn SpeechBackend>)
            },
        );
        #[cfg(feature = "omnivox")]
        r.register_cached(
            BackendInfo {
                id: "omnivox",
                name: "Omnivox speech server",
                priority: 40,
                opt_in: false,
                available: false,
                caps: omnivox::CAPS,
            },
            "builtin",
            || omnivox::OmnivoxCommand::from_env().is_some(),
            || {
                let cmd = omnivox::OmnivoxCommand::from_env().ok_or_else(|| {
                    SpeechError::Unavailable("omnivox", "omnivox is not on PATH".into())
                })?;
                omnivox::OmnivoxBackend::spawn(&cmd).map(|b| Box::new(b) as Box<dyn SpeechBackend>)
            },
        );
        #[cfg(feature = "speechd")]
        r.register_cached(
            BackendInfo {
                id: "speechd",
                name: "Speech Dispatcher",
                priority: 45,
                opt_in: false,
                available: false,
                caps: speechd::CAPS,
            },
            "builtin",
            speechd::available,
            || {
                speechd::SpeechdBackend::connect_default()
                    .map(|b| Box::new(b) as Box<dyn SpeechBackend>)
            },
        );
        r
    }

    /// Adds a backend, replacing any entry with the same id. The probe
    /// runs at most once for this entry (clones of the registry share the
    /// answer).
    pub fn register(
        &mut self,
        info: BackendInfo,
        probe: impl Fn() -> bool + Send + Sync + 'static,
        constructor: impl Fn() -> Result<Box<dyn SpeechBackend>, SpeechError> + Send + Sync + 'static,
    ) {
        let probe = once(info.id, None, probe);
        self.insert(info, probe, Arc::new(constructor));
    }

    /// Like [`register`](Self::register), but the probe's answer is kept
    /// for the whole process under the backend's id and `key`: any registry
    /// built later with the same id and key reuses it. Use a key that names
    /// what the probe depends on (the engine's configuration), so a changed
    /// setting probes again. [`forget_probes`] clears the process cache.
    pub fn register_cached(
        &mut self,
        info: BackendInfo,
        key: impl Into<String>,
        probe: impl Fn() -> bool + Send + Sync + 'static,
        constructor: impl Fn() -> Result<Box<dyn SpeechBackend>, SpeechError> + Send + Sync + 'static,
    ) {
        let probe = once(info.id, Some(key.into()), probe);
        self.insert(info, probe, Arc::new(constructor));
    }

    fn insert(
        &mut self,
        info: BackendInfo,
        probe: AvailabilityProbe,
        constructor: BackendConstructor,
    ) {
        self.entries.retain(|e| e.info.id != info.id);
        self.entries.push(RegisteredBackend {
            info,
            probe,
            constructor,
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

    /// Every registered backend as described, highest priority first,
    /// without probing: `available` is the description's own value, and no
    /// engine is looked for or started. Tests that check what a registry
    /// holds use this.
    pub fn descriptions(&self) -> Vec<BackendInfo> {
        let mut out: Vec<BackendInfo> = self.entries.iter().map(|e| e.info.clone()).collect();
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
    if let Some(p) = requested.as_deref()
        && let Some(b) = list.iter().find(|b| b.id == p && b.available)
    {
        return Selection {
            backend: b.clone(),
            requested,
            fell_back: false,
        };
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

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn is_us_english(v: &Voice) -> bool {
    v.languages.iter().any(|l| {
        l.split(['-', '_'])
            .skip(1)
            .any(|sub| sub.eq_ignore_ascii_case("us"))
    })
}

/// Resolves what a user typed for a voice (`--voice`, settings, a prompt)
/// to a voice id: an exact id first, then an exact name (ignoring case),
/// then a name or id containing every word typed ("Zira" finds "Microsoft
/// Zira Desktop", "eloquence reed" finds Eloquence Reed), then any
/// substring. Among several matches a US English voice wins, then list
/// order. `None` when nothing matches.
pub fn resolve_voice(voices: &[Voice], query: &str) -> Option<String> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    if let Some(v) = voices.iter().find(|v| v.id == q) {
        return Some(v.id.clone());
    }
    let pick = |matches: Vec<&Voice>| {
        matches
            .iter()
            .find(|v| is_us_english(v))
            .or_else(|| matches.first())
            .map(|v| v.id.clone())
    };
    let exact: Vec<&Voice> = voices
        .iter()
        .filter(|v| v.name.eq_ignore_ascii_case(q))
        .collect();
    if !exact.is_empty() {
        return pick(exact);
    }
    let wanted = words(q);
    if !wanted.is_empty() {
        let by_words: Vec<&Voice> = voices
            .iter()
            .filter(|v| {
                let have = words(&format!("{} {}", v.name, v.id));
                wanted.iter().all(|w| have.contains(w))
            })
            .collect();
        if !by_words.is_empty() {
            return pick(by_words);
        }
    }
    let lower = q.to_lowercase();
    pick(
        voices
            .iter()
            .filter(|v| {
                format!("{} {}", v.name, v.id)
                    .to_lowercase()
                    .contains(&lower)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Caps;

    fn voice(id: &str, name: &str, lang: &str) -> Voice {
        Voice {
            id: id.into(),
            name: name.into(),
            languages: vec![lang.into()],
            ..Voice::default()
        }
    }

    #[test]
    fn voices_resolve_by_plain_name() {
        let vs = vec![
            voice(
                r"x64:HKLM\TTS_MS_EN-US_DAVID_11.0",
                "Microsoft David Desktop",
                "en-US",
            ),
            voice(
                r"x64:HKLM\TTS_MS_EN-US_ZIRA_11.0",
                "Microsoft Zira Desktop",
                "en-US",
            ),
            voice(
                r"x64:HKLM\MSTTS_V110_enUS_ZiraM",
                "Microsoft Zira (OneCore)",
                "en-US",
            ),
            voice(r"x86:VW Paul", "VW Paul (32-bit)", "en-US"),
            voice("eci:engb:reed", "Reed (UK English)", "en-GB"),
            voice("eci:enu:reed", "Reed", "en-US"),
        ];
        assert_eq!(
            resolve_voice(&vs, "Zira").as_deref(),
            Some(r"x64:HKLM\TTS_MS_EN-US_ZIRA_11.0")
        );
        assert_eq!(
            resolve_voice(&vs, "zira onecore").as_deref(),
            Some(r"x64:HKLM\MSTTS_V110_enUS_ZiraM")
        );
        assert_eq!(resolve_voice(&vs, "paul").as_deref(), Some(r"x86:VW Paul"));
        assert_eq!(resolve_voice(&vs, "Reed").as_deref(), Some("eci:enu:reed"));
        assert_eq!(
            resolve_voice(&vs, "eci:engb:reed").as_deref(),
            Some("eci:engb:reed")
        );
        assert_eq!(resolve_voice(&vs, "Hal"), None);
        assert_eq!(resolve_voice(&vs, "  "), None);
    }

    fn info(id: &'static str, priority: i32, available: bool, opt_in: bool) -> BackendInfo {
        BackendInfo {
            id,
            name: id,
            priority,
            opt_in,
            available,
            caps: Caps::empty(),
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
        let mut r = BackendRegistry::test_doubles();
        r.register(
            BackendInfo {
                id: "eci",
                name: "ETI-Eloquence",
                priority: 60,
                opt_in: false,
                available: false,
                caps: Caps::WORD_EVENTS | Caps::NATIVE_NORMALIZATION,
            },
            || true,
            || Ok(Box::new(NullBackend::default()) as Box<dyn SpeechBackend>),
        );
        assert_eq!(r.select(None).backend.id, "eci");
        assert_eq!(r.list()[0].id, "eci");
        assert!(r.list()[0].caps.contains(Caps::NATIVE_NORMALIZATION));
        assert!(r.factory("eci").is_some());
        assert!(r.factory("nope").is_none());
        // Re-registering replaces.
        r.register(
            BackendInfo {
                id: "eci",
                name: "ETI-Eloquence",
                ..BackendInfo::default()
            },
            || false,
            || Err(SpeechError::Unavailable("eci", "missing".into())),
        );
        assert_ne!(r.select(None).backend.id, "eci");
        assert_eq!(r.list().iter().filter(|b| b.id == "eci").count(), 1);
    }

    #[test]
    fn probes_run_once_per_entry_and_once_per_process_with_a_key() {
        use std::sync::atomic::{AtomicU32, Ordering};
        let info = |id: BackendId| BackendInfo {
            id,
            name: "probe test",
            priority: 70,
            ..BackendInfo::default()
        };
        let make = || Ok(Box::new(NullBackend::default()) as Box<dyn SpeechBackend>);
        // Per entry: list, get, and select all share one probe.
        let calls = Arc::new(AtomicU32::new(0));
        let mut r = BackendRegistry::new();
        let c = Arc::clone(&calls);
        r.register(
            info("probe-entry"),
            move || {
                c.fetch_add(1, Ordering::SeqCst);
                true
            },
            make,
        );
        let copy = r.clone();
        r.list();
        r.get("probe-entry");
        assert_eq!(r.select(None).backend.id, "probe-entry");
        copy.list();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // Per process with a key: a new registry with the same key reuses
        // the answer; another key probes again.
        let calls = Arc::new(AtomicU32::new(0));
        let build = |key: &str| {
            let mut r = BackendRegistry::new();
            let c = Arc::clone(&calls);
            r.register_cached(
                info("probe-process"),
                key,
                move || {
                    c.fetch_add(1, Ordering::SeqCst);
                    false
                },
                make,
            );
            r
        };
        for _ in 0..3 {
            let r = build("config A");
            assert!(!r.get("probe-process").unwrap().available);
            r.list();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        build("config B").list();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        forget_probes();
        build("config A").list();
        assert_eq!(calls.load(Ordering::SeqCst), 3);
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
    fn test_doubles_never_reach_an_engine() {
        let r = BackendRegistry::test_doubles();
        let ids: Vec<&str> = r.list().iter().map(|b| b.id).collect();
        assert_eq!(ids, ["recording", "null"]);
        let s = r.select(Some("no-such-engine"));
        assert_eq!(s.backend.id, "null");
        assert!(s.fell_back);
        assert!(r.factory("espeak").is_none());
    }

    #[test]
    fn registry_infos_carry_the_constructed_backends_caps() {
        infos_carry_the_constructed_caps(&BackendRegistry::test_doubles());
    }

    /// The same for this build's engines. Constructing an engine loads it,
    /// so this runs only when asked for: `TEXTWEAVER_ENGINES=1` and
    /// `-- --ignored`.
    #[test]
    #[ignore = "constructs this build's real engines; set TEXTWEAVER_ENGINES=1"]
    fn builtin_infos_carry_the_constructed_backends_caps() {
        if std::env::var_os("TEXTWEAVER_ENGINES").is_none() {
            return;
        }
        infos_carry_the_constructed_caps(&BackendRegistry::with_builtins());
    }

    /// The built-ins register the doubles and this build's engines, and
    /// describing them probes and starts nothing.
    #[test]
    fn builtins_register_the_doubles_and_this_builds_engines() {
        let ids: Vec<&str> = BackendRegistry::with_builtins()
            .descriptions()
            .iter()
            .map(|b| b.id)
            .collect();
        assert!(ids.contains(&"null") && ids.contains(&"recording"));
        assert_eq!(ids.contains(&"espeak"), cfg!(feature = "espeak"));
        assert_eq!(ids.contains(&"omnivox"), cfg!(feature = "omnivox"));
        assert_eq!(ids.contains(&"speechd"), cfg!(feature = "speechd"));
    }

    /// No test outside this file builds a registry with real engines
    /// (`BackendRegistry::with_builtins()`, `speech_registry()`, or
    /// `speech_registry_for(`): automatic selection from those reaches an
    /// engine that plays audio. Tests use
    /// [`BackendRegistry::test_doubles`]. A file's test code is its
    /// `tests/` folders, and everything from its first `#[cfg(test)]`.
    #[test]
    fn no_test_outside_the_registry_reaches_a_real_engine() {
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut found = Vec::new();
        scan_tests(&crates, false, &mut found);
        assert!(
            found.is_empty(),
            "Tests must use BackendRegistry::test_doubles(), not a registry with real engines:\n{}",
            found.join("\n")
        );
    }

    fn scan_tests(dir: &std::path::Path, in_tests: bool, found: &mut Vec<String>) {
        const FORBIDDEN: [&str; 3] = [
            "BackendRegistry::with_builtins()",
            "speech_registry()",
            "speech_registry_for(",
        ];
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                scan_tests(&path, in_tests || name == "tests", found);
                continue;
            }
            if !name.ends_with(".rs") || path.ends_with("textweaver-speech/src/backends/mod.rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let mut test_code = in_tests;
            for (n, line) in text.lines().enumerate() {
                if line.trim_start().starts_with("#[cfg(test)]") {
                    test_code = true;
                }
                if test_code
                    && !line.trim_start().starts_with("//")
                    && FORBIDDEN.iter().any(|f| line.contains(f))
                {
                    found.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                }
            }
        }
    }

    fn infos_carry_the_constructed_caps(r: &BackendRegistry) {
        for info in r.list().into_iter().filter(|b| b.available) {
            let Some(make) = r.factory(info.id) else {
                continue;
            };
            let Ok(backend) = make() else {
                continue;
            };
            assert_eq!(backend.capabilities(), info.caps, "{}", info.id);
        }
    }

    #[test]
    fn preferred_voice_resolution() {
        let v = |id: &str, name: &str, lang: &str| Voice {
            id: id.into(),
            name: name.into(),
            languages: vec![lang.into()],
            ..Voice::default()
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
