//! The failed-login limit: per address, so many failures within a window,
//! then a lockout. A failed login, a TLS handshake that fails, and a first
//! packet that is not a CONNECT all count.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// The most addresses remembered; past this, entries with nothing left to
/// remember are forgotten first.
const MAX_ENTRIES: usize = 10_000;

/// Failures within the window, when the window began, and the lockout's
/// end.
#[derive(Clone, Copy, Debug)]
struct Entry {
    fails: u32,
    since: Instant,
    locked_until: Option<Instant>,
}

/// Counts failures per address.
#[derive(Debug)]
pub(crate) struct Limiter {
    max_fails: u32,
    window: Duration,
    lockout: Duration,
    by_ip: HashMap<IpAddr, Entry>,
}

impl Limiter {
    pub(crate) fn new(max_fails: u32, window: Duration, lockout: Duration) -> Self {
        Limiter {
            max_fails: max_fails.max(1),
            window,
            lockout,
            by_ip: HashMap::new(),
        }
    }

    /// Whether `ip` is locked out now.
    pub(crate) fn locked(&self, ip: IpAddr) -> bool {
        self.by_ip
            .get(&ip)
            .and_then(|e| e.locked_until)
            .is_some_and(|until| Instant::now() < until)
    }

    /// Counts a failure. Returns true when this one locks `ip` out.
    pub(crate) fn fail(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        if self.by_ip.len() >= MAX_ENTRIES && !self.by_ip.contains_key(&ip) {
            let (window, lockout) = (self.window, self.lockout);
            self.by_ip.retain(|_, e| {
                now.duration_since(e.since) <= window
                    || e.locked_until.is_some_and(|u| now < u + lockout)
            });
            if self.by_ip.len() >= MAX_ENTRIES {
                return false;
            }
        }
        let e = self.by_ip.entry(ip).or_insert(Entry {
            fails: 0,
            since: now,
            locked_until: None,
        });
        if e.locked_until.is_some_and(|u| now < u) {
            return false;
        }
        if now.duration_since(e.since) > self.window {
            *e = Entry {
                fails: 0,
                since: now,
                locked_until: None,
            };
        }
        e.fails += 1;
        if e.fails >= self.max_fails {
            e.locked_until = Some(now + self.lockout);
            e.fails = 0;
            e.since = now;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_failures_lock_one_address_only() {
        let mut l = Limiter::new(5, Duration::from_secs(60), Duration::from_secs(300));
        let a: IpAddr = "192.0.2.1".parse().unwrap();
        let b: IpAddr = "192.0.2.2".parse().unwrap();
        for _ in 0..4 {
            assert!(!l.fail(a));
        }
        assert!(!l.locked(a));
        assert!(l.fail(a));
        assert!(l.locked(a));
        assert!(!l.locked(b));
        assert!(!l.fail(a), "a locked address is not locked again");
    }

    #[test]
    fn the_window_and_the_lockout_end() {
        let mut l = Limiter::new(2, Duration::from_millis(50), Duration::from_millis(50));
        let a: IpAddr = "192.0.2.1".parse().unwrap();
        l.fail(a);
        std::thread::sleep(Duration::from_millis(80));
        assert!(!l.fail(a), "the first failure aged out");
        assert!(l.fail(a));
        assert!(l.locked(a));
        std::thread::sleep(Duration::from_millis(80));
        assert!(!l.locked(a));
    }
}
