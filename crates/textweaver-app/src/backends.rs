//! The speech backends this build offers: the speech crate's built-ins plus
//! the engines that live in their own crates (ADR-0007: ETI-Eloquence
//! through ECI). Every frontend selects from this one registry, so
//! `tw backends`, `tw speak`, and the reader agree.

use textweaver_eci::EciConfig;
use textweaver_speech::{BackendRegistry, SpeechBackend};

/// The registry of every backend compiled into this build.
///
/// `eci` ranks above the built-in engines, so an installed ETI-Eloquence is
/// the automatic choice; it is available only when discovery finds a library
/// the user installed (`textweaver_eci::discovery`).
pub fn speech_registry() -> BackendRegistry {
    let mut registry = BackendRegistry::with_builtins();
    registry.register(
        textweaver_eci::backend_info(),
        || textweaver_eci::backend_info().available,
        || {
            textweaver_eci::EciBackend::new(EciConfig::default())
                .map(|b| Box::new(b) as Box<dyn SpeechBackend>)
        },
    );
    registry
}

#[cfg(test)]
mod tests {
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
}
