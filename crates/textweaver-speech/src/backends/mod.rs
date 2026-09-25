//! Engine implementations and backend selection.
//!
//! Selection follows Star: an explicit preference wins; otherwise available
//! backends are tried by descending priority, skipping opt-in ones; `null`
//! is the final fallback.

mod null;

pub use null::NullBackend;

use crate::backend::{BackendFactory, BackendInfo};

/// Every backend compiled into this build, with availability.
pub fn registry() -> Vec<BackendInfo> {
    vec![BackendInfo {
        id: "null",
        name: "Silent (no audio)",
        priority: i32::MIN,
        opt_in: false,
        available: true,
    }]
}

/// A factory for backend `id`, if compiled in.
pub fn factory(id: &str) -> Option<BackendFactory> {
    match id {
        "null" => Some(Box::new(|| Ok(Box::new(NullBackend::default()) as _))),
        _ => None,
    }
}

/// Picks a backend: `preferred` if available, else the highest-priority
/// available non-opt-in backend, else `null`.
pub fn select(preferred: Option<&str>) -> BackendInfo {
    let all = registry();
    if let Some(p) = preferred {
        if let Some(b) = all.iter().find(|b| b.id == p && b.available) {
            return b.clone();
        }
    }
    all.iter()
        .filter(|b| b.available && !b.opt_in)
        .max_by_key(|b| b.priority)
        .cloned()
        .unwrap_or(BackendInfo {
            id: "null",
            name: "Silent (no audio)",
            priority: i32::MIN,
            opt_in: false,
            available: true,
        })
}
