//! The clock the engine host's timing reads (Wave 7): the start deadline
//! ([`crate::HostStart`]), the stall timeout ([`crate::HostProcess`]), and
//! the reopen backoff of a stalled output ([`crate::Playback`]).
//!
//! In use it is the system's monotonic clock. Tests make a
//! [`Clock::manual`] one instead, which stands still until the test moves
//! it with [`Clock::advance`], so a test of a ten-second timeout neither
//! waits ten seconds nor fails on a busy machine: it checks what happens
//! just before the deadline and just after, in order.
//!
//! A clock is a cheap handle; clones share their time.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A clock. See the module docs.
#[derive(Clone, Debug, Default)]
pub struct Clock {
    /// `None` for the system clock.
    manual: Option<Arc<Manual>>,
}

/// A clock that moves only when told to.
#[derive(Debug)]
struct Manual {
    /// The instant the clock started at.
    start: Instant,
    /// How far it has been moved since.
    offset: Mutex<Duration>,
}

impl Clock {
    /// The system's monotonic clock (the default).
    pub fn system() -> Self {
        Clock::default()
    }

    /// A clock that starts at the current instant and moves only with
    /// [`advance`](Self::advance) (tests).
    pub fn manual() -> Self {
        Clock {
            manual: Some(Arc::new(Manual {
                start: Instant::now(),
                offset: Mutex::new(Duration::ZERO),
            })),
        }
    }

    /// Whether this is a [`manual`](Self::manual) clock.
    pub fn is_manual(&self) -> bool {
        self.manual.is_some()
    }

    /// The current instant.
    pub fn now(&self) -> Instant {
        match &self.manual {
            None => Instant::now(),
            Some(m) => m.start + *m.offset.lock().unwrap_or_else(|e| e.into_inner()),
        }
    }

    /// How long since `earlier` (zero when `earlier` is later).
    pub fn since(&self, earlier: Instant) -> Duration {
        self.now().saturating_duration_since(earlier)
    }

    /// Moves a manual clock, and every clone of it, forward by `by`. The
    /// system clock cannot be moved: this does nothing to it.
    pub fn advance(&self, by: Duration) {
        if let Some(m) = &self.manual {
            let mut offset = m.offset.lock().unwrap_or_else(|e| e.into_inner());
            *offset = offset.saturating_add(by);
        }
    }
}

/// Two system clocks are equal; a manual clock equals its clones.
impl PartialEq for Clock {
    fn eq(&self, other: &Self) -> bool {
        match (&self.manual, &other.manual) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manual_clock_stands_still_until_moved() {
        let c = Clock::manual();
        assert!(c.is_manual());
        let t0 = c.now();
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(c.now(), t0, "no real time passes on it");
        c.advance(Duration::from_secs(10));
        assert_eq!(c.since(t0), Duration::from_secs(10));
        // Clones share the time.
        let twin = c.clone();
        twin.advance(Duration::from_millis(1));
        assert_eq!(c.since(t0), Duration::from_millis(10_001));
        assert_eq!(c, twin);
        assert_ne!(c, Clock::manual());
        assert_ne!(c, Clock::system());
    }

    #[test]
    fn the_system_clock_moves_by_itself_and_cannot_be_moved() {
        let c = Clock::system();
        assert!(!c.is_manual());
        assert_eq!(c, Clock::default());
        let t0 = c.now();
        c.advance(Duration::from_secs(3600));
        assert!(c.since(t0) < Duration::from_secs(3600));
        // A later instant is not "since".
        assert_eq!(c.since(t0 + Duration::from_secs(60)), Duration::ZERO);
    }
}
