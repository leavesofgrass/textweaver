//! Announcements to the user (ADR-0006).
//!
//! Every state change is announced through an [`Announcer`]. Frontends pick
//! the implementation:
//!
//! - [`SpeechAnnouncer`]: self-voicing, through the speech service (wired by
//!   the app with a closure, so this crate does not depend on speech);
//! - [`StatusLineAnnouncer`]: the terminal's status line, which terminal
//!   screen readers read as it changes;
//! - [`LogAnnouncer`]: records announcements for tests;
//! - wave 3: a live-region announcer for the GUI (feature `live-region`).
//!
//! [`Verbosity`] filters what is said.
//!
//! Owner: Agent C.

use serde::{Deserialize, Serialize};
pub use textweaver_core::Verbosity;

/// How urgently an announcement should be delivered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// Wait for current speech to finish.
    #[default]
    Polite,
    /// Interrupt current announcements.
    Assertive,
}

/// Something that tells the user about a state change.
pub trait Announcer {
    /// Announces `text`.
    fn announce(&mut self, text: &str, priority: Priority);

    /// Announces `text` only when the user's verbosity is at least `min`.
    fn announce_at(&mut self, text: &str, priority: Priority, min: Verbosity, current: Verbosity) {
        if current >= min {
            self.announce(text, priority);
        }
    }
}

/// Records every announcement; for tests and the scripted TUI test.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogAnnouncer {
    /// Everything announced, oldest first.
    pub log: Vec<(String, Priority)>,
}

impl Announcer for LogAnnouncer {
    fn announce(&mut self, text: &str, priority: Priority) {
        self.log.push((text.to_owned(), priority));
    }
}

/// Speaks announcements through a callback (the app passes one that calls
/// `SpeechService::say(text, SayMode::Announce)`).
pub struct SpeechAnnouncer<F: FnMut(&str, Priority)> {
    speak: F,
}

impl<F: FnMut(&str, Priority)> SpeechAnnouncer<F> {
    /// An announcer that calls `speak`.
    pub fn new(speak: F) -> Self {
        SpeechAnnouncer { speak }
    }
}

impl<F: FnMut(&str, Priority)> Announcer for SpeechAnnouncer<F> {
    fn announce(&mut self, text: &str, priority: Priority) {
        (self.speak)(text, priority);
    }
}

/// Keeps the latest announcement for display on a status line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusLineAnnouncer {
    /// The text currently shown.
    pub current: Option<String>,
}

impl Announcer for StatusLineAnnouncer {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.current = Some(text.to_owned());
    }
}

/// Sends every announcement to several announcers.
#[derive(Default)]
pub struct MultiAnnouncer {
    targets: Vec<Box<dyn Announcer>>,
}

impl MultiAnnouncer {
    /// An announcer with no targets.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a target.
    pub fn push(&mut self, a: Box<dyn Announcer>) {
        self.targets.push(a);
    }
}

impl Announcer for MultiAnnouncer {
    fn announce(&mut self, text: &str, priority: Priority) {
        for t in &mut self.targets {
            t.announce(text, priority);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_filters() {
        let mut a = LogAnnouncer::default();
        a.announce_at(
            "detail",
            Priority::Polite,
            Verbosity::High,
            Verbosity::Normal,
        );
        a.announce_at("state", Priority::Polite, Verbosity::Low, Verbosity::Normal);
        assert_eq!(a.log, vec![("state".to_owned(), Priority::Polite)]);
    }

    #[test]
    fn speech_announcer_calls_back() {
        let mut said = Vec::new();
        {
            let mut a = SpeechAnnouncer::new(|t: &str, _| said.push(t.to_owned()));
            a.announce("Rate 290", Priority::Assertive);
        }
        assert_eq!(said, vec!["Rate 290"]);
    }
}
