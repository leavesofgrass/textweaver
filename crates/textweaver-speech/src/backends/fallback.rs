//! Starting the first engine that works (the owner's rule, October 2026).
//!
//! textweaver always speaks one way or another if it can. When the chosen
//! engine cannot start (Eloquence's host does not answer, a SAPI voice is
//! broken), the next available engine is tried, in priority order, and
//! so on; silence (`null`) is the last and final fallback, after every
//! available engine on this machine has failed. Each failure is logged,
//! and the caller says one sentence naming the engine that failed and the
//! one speaking now ([`Started::failed`]).
//!
//! Every place speech starts uses this: the app's startup, the terminal
//! reader, `tw speak`, Restart Speech, a profile or setting that picks an
//! engine, and recovery after an engine crashes mid-reading.

use crate::backend::{BackendFactory, BackendInfo};

use super::{BackendRegistry, null_info};

/// One engine that was tried and did not start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartFailure {
    /// The engine.
    pub backend: BackendInfo,
    /// Why it did not start.
    pub error: String,
}

/// The engine that started, and the ones tried before it.
#[derive(Debug)]
pub struct Started<T> {
    /// What starting it produced (a speech service, a backend).
    pub value: T,
    /// The engine that started.
    pub backend: BackendInfo,
    /// The engines tried before it that failed, in the order tried.
    pub failed: Vec<StartFailure>,
}

impl<T> Started<T> {
    /// True when the engine speaking is silence (`null`) although engines
    /// were tried: every one failed.
    pub fn silent(&self) -> bool {
        self.backend.id == "null"
    }
}

impl BackendRegistry {
    /// The engines to try, in order, for `preferred` (an id, or `None`,
    /// `""` or `"auto"` for automatic selection): the preferred engine when
    /// it is available (even an opt-in one), then every other available
    /// engine that is not opt-in, highest priority first, then `null`.
    /// Probes each engine (once; see [`BackendRegistry::list`]).
    pub fn start_order(&self, preferred: Option<&str>) -> Vec<BackendInfo> {
        start_order_from(&self.list(), preferred)
    }

    /// Starts the first engine in [`start_order`](Self::start_order) for
    /// which `start` succeeds, logging each failure. `start` gets the
    /// engine's factory and description. Fails only when even `null` did
    /// not start, with every failure.
    pub fn start_first<T>(
        &self,
        preferred: Option<&str>,
        start: impl FnMut(BackendFactory, &BackendInfo) -> Result<T, String>,
    ) -> Result<Started<T>, Vec<StartFailure>> {
        start_first_in(self, &self.start_order(preferred), start)
    }
}

/// [`BackendRegistry::start_order`] over an already probed list.
pub fn start_order_from(list: &[BackendInfo], preferred: Option<&str>) -> Vec<BackendInfo> {
    let preferred = preferred
        .map(str::trim)
        .filter(|p| !p.is_empty() && !p.eq_ignore_ascii_case("auto"));
    let mut order: Vec<BackendInfo> = Vec::new();
    if let Some(b) = preferred.and_then(|p| list.iter().find(|b| b.id == p && b.available)) {
        order.push(b.clone());
    }
    let mut rest: Vec<&BackendInfo> = list
        .iter()
        .filter(|b| b.available && !b.opt_in && b.id != "null")
        .filter(|b| !order.iter().any(|o| o.id == b.id))
        .collect();
    rest.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(b.id)));
    order.extend(rest.into_iter().cloned());
    if !order.iter().any(|b| b.id == "null") {
        order.push(
            list.iter()
                .find(|b| b.id == "null")
                .cloned()
                .unwrap_or_else(null_info),
        );
    }
    order
}

fn start_first_in<T>(
    registry: &BackendRegistry,
    order: &[BackendInfo],
    mut start: impl FnMut(BackendFactory, &BackendInfo) -> Result<T, String>,
) -> Result<Started<T>, Vec<StartFailure>> {
    let mut failed = Vec::new();
    for backend in order {
        let Some(factory) = registry.factory(backend.id) else {
            continue;
        };
        match start(factory, backend) {
            Ok(value) => {
                if !failed.is_empty() {
                    log::warn!(
                        "speech: {} is speaking after {} engine(s) failed to start",
                        backend.id,
                        failed.len()
                    );
                }
                return Ok(Started {
                    value,
                    backend: backend.clone(),
                    failed,
                });
            }
            Err(error) => {
                log::warn!("speech: {} could not start: {error}", backend.id);
                failed.push(StartFailure {
                    backend: backend.clone(),
                    error,
                });
            }
        }
    }
    Err(failed)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::backend::{Caps, SpeechBackend, SpeechError};
    use crate::backends::NullBackend;

    /// A registry of fake engines: each id with its priority, and whether
    /// it starts. Nothing real is probed or started.
    fn fakes(engines: &[(&'static str, i32, bool)]) -> BackendRegistry {
        let mut r = BackendRegistry::test_doubles();
        for &(id, priority, starts) in engines {
            r.register(
                BackendInfo {
                    id,
                    name: id,
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

    /// Starts each engine by constructing it, recording the order tried.
    fn run(
        r: &BackendRegistry,
        preferred: Option<&str>,
    ) -> (Result<Started<()>, Vec<StartFailure>>, Vec<&'static str>) {
        let tried = Arc::new(Mutex::new(Vec::new()));
        let t = Arc::clone(&tried);
        let out = r.start_first(preferred, move |factory, info| {
            t.lock().unwrap().push(info.id);
            factory().map(|_| ()).map_err(|e| e.to_string())
        });
        let tried = tried.lock().unwrap().clone();
        (out, tried)
    }

    #[test]
    fn a_chain_of_failing_engines_ends_on_the_first_that_starts() {
        let r = fakes(&[
            ("eci", 1000, false),
            ("piper", 500, false),
            ("dectalk", 300, true),
            ("espeak", 50, true),
        ]);
        let (out, tried) = run(&r, None);
        let started = out.expect("an engine starts");
        assert_eq!(started.backend.id, "dectalk");
        assert_eq!(tried, ["eci", "piper", "dectalk"]);
        let failed: Vec<&str> = started.failed.iter().map(|f| f.backend.id).collect();
        assert_eq!(failed, ["eci", "piper"]);
        assert!(started.failed[0].error.contains("did not start in time"));
        assert!(!started.silent());
    }

    #[test]
    fn a_preferred_engine_is_tried_first_then_the_rest_by_priority() {
        let r = fakes(&[
            ("eci", 1000, true),
            ("piper", 500, false),
            ("espeak", 50, true),
        ]);
        let (out, tried) = run(&r, Some("piper"));
        assert_eq!(out.expect("starts").backend.id, "eci");
        assert_eq!(tried, ["piper", "eci"]);
        // Automatic selection and the preferred engine starting at once.
        let (out, tried) = run(&r, Some("auto"));
        assert_eq!(out.expect("starts").backend.id, "eci");
        assert_eq!(tried, ["eci"]);
    }

    #[test]
    fn silence_only_after_every_engine_failed() {
        let r = fakes(&[
            ("eci", 1000, false),
            ("piper", 500, false),
            ("espeak", 50, false),
        ]);
        let (out, tried) = run(&r, Some("eci"));
        let started = out.expect("silence always starts");
        assert_eq!(tried, ["eci", "piper", "espeak", "null"]);
        assert!(started.silent());
        assert_eq!(started.failed.len(), 3);
    }

    #[test]
    fn opt_in_engines_are_tried_only_when_chosen() {
        let r = fakes(&[("eci", 1000, false)]);
        let order: Vec<&str> = r.start_order(None).iter().map(|b| b.id).collect();
        assert_eq!(order, ["eci", "null"]);
        let order: Vec<&str> = r
            .start_order(Some("recording"))
            .iter()
            .map(|b| b.id)
            .collect();
        assert_eq!(order, ["recording", "eci", "null"]);
    }
}
