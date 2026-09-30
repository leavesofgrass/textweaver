//! The hybrid logical clock.
//!
//! Every change carries a [`Stamp`] (ADR-0049): the wall time in
//! milliseconds, a counter, and the computer that made it. A computer's
//! clock never goes backward: a new stamp is the larger of the wall time and
//! the newest stamp seen, plus one tick (the counter rises while the wall
//! time has not passed the newest stamp). So a lab computer whose clock
//! runs slow cannot undo a later edit, and one computer's stamps always
//! rise. Stamps are ordered by time, then counter, then computer id, so
//! every computer picks the same winner and no two computers' stamps are
//! equal.
//!
//! A stamp more than [`AHEAD_LIMIT_MS`] ahead of this computer's wall time
//! is still accepted (refusing it would stop the folders converging), but
//! [`Clock::observe`] flags it, so the sync status can name the computer
//! whose clock is wrong.

use serde::{Deserialize, Serialize};

use crate::DeviceId;

/// How far ahead of the wall clock a seen stamp may be before it is
/// flagged: one day.
pub const AHEAD_LIMIT_MS: u64 = 24 * 60 * 60 * 1000;

/// When a change was made and by which computer. Ordered by time, then
/// counter, then computer id (the field order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Stamp {
    /// Milliseconds since 1970-01-01 UTC, by the hybrid clock. Written as
    /// `wall_ms`, the name the store's deletion records use (S3).
    #[serde(rename = "wall_ms")]
    pub time: u64,
    /// Ticks within one millisecond; left out of the file when zero.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub counter: u32,
    /// The computer that made the change.
    pub device: DeviceId,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

impl Stamp {
    /// A stamp at `time` with counter zero.
    pub const fn new(time: u64, device: DeviceId) -> Self {
        Self {
            time,
            counter: 0,
            device,
        }
    }

    /// A stamp from all its parts.
    pub const fn with_counter(time: u64, counter: u32, device: DeviceId) -> Self {
        Self {
            time,
            counter,
            device,
        }
    }
}

/// A stamp from another computer that is far ahead of this computer's wall
/// clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockAhead {
    /// The computer whose stamp it was.
    pub device: DeviceId,
    /// How far ahead it was, in milliseconds.
    pub ahead_ms: u64,
}

impl std::fmt::Display for ClockAhead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hours = self.ahead_ms / 3_600_000;
        write!(f, "Clock ahead: computer {} by {hours} hours", self.device)
    }
}

/// The wall clock in milliseconds since 1970-01-01 UTC.
pub fn wall_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// One computer's hybrid logical clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clock {
    device: DeviceId,
    /// The newest (time, counter) seen or stamped.
    last: (u64, u32),
}

impl Clock {
    /// A clock for `device` that has seen nothing yet. Call
    /// [`Clock::observe`] with the stamps already on disk before the first
    /// [`Clock::tick`], or start from a saved point with
    /// [`Clock::starting_at`].
    pub const fn new(device: DeviceId) -> Self {
        Self {
            device,
            last: (0, 0),
        }
    }

    /// A clock for `device` whose newest seen stamp is `last` (saved from
    /// [`Clock::last`]).
    pub const fn starting_at(device: DeviceId, last: (u64, u32)) -> Self {
        Self { device, last }
    }

    /// The computer this clock stamps for.
    pub const fn device(&self) -> DeviceId {
        self.device
    }

    /// The newest time and counter seen or stamped, to save between runs.
    pub const fn last(&self) -> (u64, u32) {
        self.last
    }

    /// A stamp for a change made now.
    pub fn tick(&mut self) -> Stamp {
        self.tick_at(wall_ms())
    }

    /// A stamp for a change made at wall time `now_ms` (for tests): `now_ms`
    /// when it is past the newest stamp seen, else that stamp plus one tick.
    pub fn tick_at(&mut self, now_ms: u64) -> Stamp {
        let (time, counter) = self.last;
        self.last = if now_ms > time {
            (now_ms, 0)
        } else if counter < u32::MAX {
            (time, counter + 1)
        } else {
            (time.saturating_add(1), 0)
        };
        Stamp::with_counter(self.last.0, self.last.1, self.device)
    }

    /// Takes in a stamp from a record, so later ticks come after it. Returns
    /// a warning when the stamp is more than a day ahead of the wall clock.
    pub fn observe(&mut self, stamp: Stamp) -> Option<ClockAhead> {
        self.observe_at(stamp, wall_ms())
    }

    /// [`Clock::observe`] against wall time `now_ms` (for tests).
    pub fn observe_at(&mut self, stamp: Stamp, now_ms: u64) -> Option<ClockAhead> {
        self.last = self.last.max((stamp.time, stamp.counter));
        let ahead_ms = stamp.time.saturating_sub(now_ms);
        (ahead_ms > AHEAD_LIMIT_MS).then_some(ClockAhead {
            device: stamp.device,
            ahead_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: DeviceId = DeviceId::from_u128(1);
    const B: DeviceId = DeviceId::from_u128(2);

    #[test]
    fn stamps_serialize_without_a_zero_counter() {
        let json = serde_json::to_string(&Stamp::new(7, A)).unwrap();
        assert!(!json.contains("counter"));
        let back: Stamp = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Stamp::new(7, A));
    }

    #[test]
    fn ticks_follow_the_wall_clock_and_always_rise() {
        let mut c = Clock::new(A);
        let a = c.tick_at(1000);
        assert_eq!((a.time, a.counter), (1000, 0));
        // The wall clock went back: the stamp still rises, by its counter.
        let b = c.tick_at(500);
        assert_eq!((b.time, b.counter), (1000, 1));
        let d = c.tick_at(1000);
        assert_eq!((d.time, d.counter), (1000, 2));
        assert!(a < b && b < d);
        assert_eq!(c.tick_at(5000), Stamp::new(5000, A));
    }

    #[test]
    fn a_slow_clock_stamps_after_what_it_has_seen() {
        let mut lab = Clock::new(A);
        assert!(lab.observe_at(Stamp::new(10_000, B), 10_000).is_none());
        // The lab's own wall clock says 2000, far behind.
        let s = lab.tick_at(2000);
        assert_eq!((s.time, s.counter), (10_000, 1));
        assert!(s > Stamp::new(10_000, B));
    }

    #[test]
    fn ties_break_by_counter_then_device() {
        assert!(Stamp::new(5, A) < Stamp::new(5, B));
        assert!(Stamp::with_counter(5, 1, A) > Stamp::new(5, B));
        assert!(Stamp::new(6, A) > Stamp::with_counter(5, 9, B));
    }

    #[test]
    fn a_stamp_more_than_a_day_ahead_is_flagged_and_still_taken_in() {
        let mut c = Clock::new(A);
        let now = 1_000_000_000;
        assert!(
            c.observe_at(Stamp::new(now + AHEAD_LIMIT_MS, B), now)
                .is_none()
        );
        let far = Stamp::new(now + AHEAD_LIMIT_MS + 3_600_000, B);
        let w = c.observe_at(far, now).expect("flagged");
        assert_eq!(w.device, B);
        assert_eq!(w.ahead_ms, AHEAD_LIMIT_MS + 3_600_000);
        assert!(w.to_string().starts_with("Clock ahead"));
        assert!(c.tick_at(now) > far);
    }
}
