//! Starting speech with fallback, for every frontend (the owner's rule,
//! October 2026): the chosen engine, else the next available one in
//! priority order (SAPI 5's own voices on Windows, eSpeak NG, and so on),
//! and silence only after every available engine has failed. The user
//! hears one sentence naming the engine that failed and the one speaking
//! now. The terminal reader, the textweaver app, Restart Speech, and the
//! recovery after an engine crashes all start speech through
//! [`start_speech_service`].

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_speech::{BackendRegistry, ServiceConfig, SpeechService, StartFailure};

/// The id a frontend reports when speech is silent.
pub const SILENT: &str = "silent";

/// The names of the engines that failed, as one list for a sentence.
fn failed_names(failed: &[StartFailure]) -> String {
    failed
        .iter()
        .map(|f| f.backend.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The sentences for an engine choice: `wanted` was asked for and is not
/// available (so `now` speaks instead), and `failed` were tried and did
/// not start before `now` (an engine name, or `None` for silence).
pub fn start_messages(
    c: &Catalog,
    wanted_unavailable: Option<&str>,
    failed: &[StartFailure],
    now: Option<&str>,
) -> Vec<String> {
    let mut messages = Vec::new();
    if failed.is_empty() {
        if let (Some(wanted), Some(now)) = (wanted_unavailable, now) {
            messages.push(c.fmt(
                "tui-setup-backend-unavailable",
                &args!["wanted" => wanted, "backend" => now],
            ));
        }
        return messages;
    }
    let names = failed_names(failed);
    messages.push(match now {
        Some(now) => c.fmt(
            "speech-engine-fallback",
            &args!["failed" => names, "engine" => now],
        ),
        None => c.fmt("speech-engine-fallback-silent", &args!["failed" => names]),
    });
    messages
}

/// Starts a speech service from `registry`: `preference` (an engine id;
/// `None`, `""` or `"auto"` chooses automatically) when it is available,
/// else the other available engines in priority order, then silence.
/// Returns the service, the engine's id (`"silent"` for silence), and the
/// sentences for the user, in `c`'s language.
pub fn start_speech_service(
    registry: &BackendRegistry,
    preference: Option<&str>,
    config: &ServiceConfig,
    c: &Catalog,
) -> (SpeechService, String, Vec<String>) {
    let preference = preference
        .map(str::trim)
        .filter(|p| !p.is_empty() && !p.eq_ignore_ascii_case("auto"));
    let order = registry.start_order(preference);
    let wanted_unavailable = preference.filter(|p| !order.iter().any(|b| b.id == *p));
    let started = registry.start_first(preference, |factory, _| {
        SpeechService::spawn(factory, config.clone()).map_err(|e| e.to_string())
    });
    match started {
        Ok(s) if s.silent() => {
            // Silence chosen with no engine failing (nothing installed, or
            // `null` asked for) is named; after failures, the sentence
            // says textweaver stays silent.
            let now = s.failed.is_empty().then_some(s.backend.name);
            let messages = start_messages(c, wanted_unavailable, &s.failed, now);
            let id = if s.failed.is_empty() {
                s.backend.id.to_owned()
            } else {
                SILENT.to_owned()
            };
            (s.value, id, messages)
        }
        Ok(s) => {
            let messages = start_messages(c, wanted_unavailable, &s.failed, Some(s.backend.name));
            (s.value, s.backend.id.to_owned(), messages)
        }
        Err(failed) => {
            let error = failed
                .last()
                .map_or_else(|| "no engine".to_owned(), |f| f.error.clone());
            let mut messages = start_messages(c, None, &failed, None);
            if messages.is_empty() {
                messages.push(c.fmt("tui-setup-speech-failed", &args!["error" => error]));
            }
            (SpeechService::null(), SILENT.to_owned(), messages)
        }
    }
}

#[cfg(test)]
mod tests {
    use textweaver_speech::{BackendInfo, Caps, NullBackend, SpeechBackend, SpeechError};

    use super::*;

    fn fakes(engines: &[(&'static str, &'static str, i32, bool)]) -> BackendRegistry {
        let mut r = BackendRegistry::test_doubles();
        for &(id, name, priority, starts) in engines {
            r.register(
                BackendInfo {
                    id,
                    name,
                    priority,
                    opt_in: false,
                    available: true,
                    caps: Caps::empty(),
                },
                || true,
                move || {
                    if starts {
                        Ok(Box::new(NullBackend::default()) as Box<dyn SpeechBackend>)
                    } else {
                        Err(SpeechError::Unavailable(
                            id,
                            "the host did not start in time".into(),
                        ))
                    }
                },
            );
        }
        r
    }

    fn catalog() -> std::sync::Arc<Catalog> {
        Catalog::for_language("en", None).0
    }

    #[test]
    fn a_failed_engine_falls_back_and_says_so_in_one_sentence() {
        let r = fakes(&[
            ("eci", "Eloquence (OpenEVV, direct)", 1000, false),
            ("sapi", "Windows SAPI 5 voices", 500, true),
            ("espeak", "eSpeak NG", 50, true),
        ]);
        let (service, id, messages) =
            start_speech_service(&r, None, &ServiceConfig::default(), &catalog());
        assert_eq!(id, "sapi");
        assert_eq!(messages.len(), 1, "{messages:?}");
        let m = &messages[0];
        assert!(
            m.starts_with("Eloquence (OpenEVV, direct) could not start")
                && m.contains("Windows SAPI 5 voices"),
            "{m}"
        );
        service.shutdown();
    }

    #[test]
    fn silence_only_when_every_engine_failed() {
        let r = fakes(&[
            ("eci", "Eloquence", 1000, false),
            ("sapi", "Windows SAPI 5 voices", 500, false),
        ]);
        let (service, id, messages) =
            start_speech_service(&r, Some("eci"), &ServiceConfig::default(), &catalog());
        assert_eq!(id, SILENT);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(
            messages[0].starts_with("Eloquence, Windows SAPI 5 voices could not start"),
            "{}",
            messages[0]
        );
        service.shutdown();
    }

    #[test]
    fn nothing_is_said_when_the_first_engine_starts() {
        let r = fakes(&[("eci", "Eloquence", 1000, true)]);
        let (service, id, messages) =
            start_speech_service(&r, Some("auto"), &ServiceConfig::default(), &catalog());
        assert_eq!(id, "eci");
        assert!(messages.is_empty(), "{messages:?}");
        service.shutdown();
    }

    #[test]
    fn an_unavailable_choice_still_says_which_engine_speaks() {
        let r = fakes(&[("espeak", "eSpeak NG", 50, true)]);
        let (service, id, messages) =
            start_speech_service(&r, Some("eci"), &ServiceConfig::default(), &catalog());
        assert_eq!(id, "espeak");
        assert_eq!(
            messages,
            ["Speech engine eci is not available; using eSpeak NG."]
        );
        service.shutdown();
    }
}
