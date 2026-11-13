//! The failed-login limit: per address, so many failures within a window,
//! then a lockout. A failed login, a TLS handshake that fails, and a first
//! packet that is not a CONNECT all count.
//!
//! A login is counted before its password is checked ([`Limiter::begin`]):
//! logins still being checked count against the limit, so a burst of
//! simultaneous guesses gets no more answers than the limit allows.
//! IPv6 addresses are kept by their /64 prefix, the block one host or one
//! network usually holds. When the table is full, new addresses share one
//! overflow bucket, so a flood of addresses cannot switch the limit off.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr};
use std::time::{Duration, Instant};

/// The most addresses remembered; past this, entries with nothing left to
/// remember are forgotten first, and then new addresses share the overflow
/// bucket.
const MAX_ENTRIES: usize = 10_000;

/// The key an address is counted under: an IPv4 address whole (also when
/// written as an IPv4-mapped IPv6 address), an IPv6 address by its /64.
pub(crate) fn key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return IpAddr::V4(v4);
            }
            let s = v6.segments();
            IpAddr::V6(Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
        }
    }
}

/// Failures within the window, when the window began, the lockout's end,
/// and logins being checked now.
#[derive(Clone, Copy, Debug)]
struct Entry {
    fails: u32,
    since: Instant,
    locked_until: Option<Instant>,
    in_flight: u32,
}

impl Entry {
    fn new(now: Instant) -> Self {
        Entry {
            fails: 0,
            since: now,
            locked_until: None,
            in_flight: 0,
        }
    }

    fn locked(&self, now: Instant) -> bool {
        self.locked_until.is_some_and(|u| now < u)
    }
}

/// Where a login being checked is counted: its address's entry, or the
/// overflow bucket. Hand it back to [`Limiter::end`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    Addr(IpAddr),
    Overflow,
}

/// Counts failures per address.
#[derive(Debug)]
pub(crate) struct Limiter {
    max_fails: u32,
    window: Duration,
    lockout: Duration,
    by_ip: HashMap<IpAddr, Entry>,
    overflow: Entry,
}

impl Limiter {
    pub(crate) fn new(max_fails: u32, window: Duration, lockout: Duration) -> Self {
        Limiter {
            max_fails: max_fails.max(1),
            window,
            lockout,
            by_ip: HashMap::new(),
            overflow: Entry::new(Instant::now()),
        }
    }

    /// Whether `ip` is locked out now. An address the full table has no
    /// room for shares the overflow bucket's lockout.
    pub(crate) fn locked(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        match self.by_ip.get(&key(ip)) {
            Some(e) => e.locked(now),
            None => self.by_ip.len() >= MAX_ENTRIES && self.overflow.locked(now),
        }
    }

    /// The slot for `ip`, making room in the table when it is full.
    fn slot(&mut self, ip: IpAddr, now: Instant) -> Slot {
        let k = key(ip);
        if self.by_ip.contains_key(&k) {
            return Slot::Addr(k);
        }
        if self.by_ip.len() >= MAX_ENTRIES {
            let (window, lockout) = (self.window, self.lockout);
            self.by_ip.retain(|_, e| {
                e.in_flight > 0
                    || now.duration_since(e.since) <= window
                    || e.locked_until.is_some_and(|u| now < u + lockout)
            });
            if self.by_ip.len() >= MAX_ENTRIES {
                return Slot::Overflow;
            }
        }
        self.by_ip.insert(k, Entry::new(now));
        Slot::Addr(k)
    }

    fn entry(&mut self, slot: Slot) -> Option<&mut Entry> {
        match slot {
            Slot::Addr(k) => self.by_ip.get_mut(&k),
            Slot::Overflow => Some(&mut self.overflow),
        }
    }

    /// Starts checking a login from `ip`. `None` refuses it unchecked: the
    /// address is locked out, or its failures plus the logins still being
    /// checked have reached the limit. Otherwise call [`Limiter::end`] with
    /// the slot once the password is checked.
    pub(crate) fn begin(&mut self, ip: IpAddr) -> Option<Slot> {
        let now = Instant::now();
        let slot = self.slot(ip, now);
        let (max, window) = (self.max_fails, self.window);
        let e = self.entry(slot)?;
        if e.locked(now) {
            return None;
        }
        if e.locked_until.is_some() || now.duration_since(e.since) > window {
            // The lockout or the window ended: start a fresh window, but
            // keep the logins still being checked.
            *e = Entry {
                in_flight: e.in_flight,
                ..Entry::new(now)
            };
        }
        if e.fails.saturating_add(e.in_flight) >= max {
            return None;
        }
        e.in_flight += 1;
        Some(slot)
    }

    /// Ends a login [`Limiter::begin`] let through, counting a failure when
    /// it was not `ok`. Returns true when this failure locks the slot out.
    pub(crate) fn end(&mut self, slot: Slot, ok: bool) -> bool {
        if let Some(e) = self.entry(slot) {
            e.in_flight = e.in_flight.saturating_sub(1);
        }
        !ok && self.count(slot, Instant::now())
    }

    /// Counts a failure. Returns true when this one locks `ip` out.
    pub(crate) fn fail(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let slot = self.slot(ip, now);
        self.count(slot, now)
    }

    fn count(&mut self, slot: Slot, now: Instant) -> bool {
        let (max, window, lockout) = (self.max_fails, self.window, self.lockout);
        let Some(e) = self.entry(slot) else {
            return false;
        };
        if e.locked(now) {
            return false;
        }
        if e.locked_until.is_some() || now.duration_since(e.since) > window {
            *e = Entry {
                in_flight: e.in_flight,
                ..Entry::new(now)
            };
        }
        e.fails += 1;
        if e.fails >= max {
            e.locked_until = Some(now + lockout);
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

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn five_failures_lock_one_address_only() {
        let mut l = Limiter::new(5, Duration::from_secs(60), Duration::from_secs(300));
        let a = ip("192.0.2.1");
        let b = ip("192.0.2.2");
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
        let a = ip("192.0.2.1");
        l.fail(a);
        std::thread::sleep(Duration::from_millis(80));
        assert!(!l.fail(a), "the first failure aged out");
        assert!(l.fail(a));
        assert!(l.locked(a));
        std::thread::sleep(Duration::from_millis(80));
        assert!(!l.locked(a));
    }

    #[test]
    fn logins_being_checked_count_against_the_limit() {
        let mut l = Limiter::new(5, Duration::from_secs(60), Duration::from_secs(300));
        let a = ip("192.0.2.1");
        let slots: Vec<Slot> = (0..5).map(|_| l.begin(a).expect("room")).collect();
        assert!(l.begin(a).is_none(), "a sixth at once is refused unchecked");
        assert!(
            l.begin(ip("192.0.2.2")).is_some(),
            "other addresses are not"
        );
        let mut locked = 0;
        for s in slots {
            locked += usize::from(l.end(s, false));
        }
        assert_eq!(locked, 1);
        assert!(l.locked(a));
        assert!(l.begin(a).is_none());
    }

    #[test]
    fn a_right_login_frees_its_place() {
        let mut l = Limiter::new(2, Duration::from_secs(60), Duration::from_secs(300));
        let a = ip("192.0.2.1");
        for _ in 0..10 {
            let s = l.begin(a).unwrap();
            assert!(!l.end(s, true));
        }
        assert!(!l.locked(a));
    }

    #[test]
    fn one_ipv6_slash_64_shares_a_lockout() {
        let mut l = Limiter::new(2, Duration::from_secs(60), Duration::from_secs(300));
        assert!(!l.fail(ip("2001:db8:1:2::1")));
        assert!(l.fail(ip("2001:db8:1:2:ffff:1:2:3")));
        assert!(l.locked(ip("2001:db8:1:2::99")));
        assert!(!l.locked(ip("2001:db8:1:3::1")), "the next /64 is not");
        assert_eq!(key(ip("::ffff:192.0.2.7")), ip("192.0.2.7"));
    }

    #[test]
    fn a_full_table_still_limits_new_addresses() {
        let mut l = Limiter::new(3, Duration::from_secs(60), Duration::from_secs(300));
        for i in 0..MAX_ENTRIES as u32 {
            let a = IpAddr::from(std::net::Ipv4Addr::from(0x0A00_0000 + i));
            l.fail(a);
        }
        assert_eq!(l.by_ip.len(), MAX_ENTRIES);
        // New addresses land in the overflow bucket and lock it.
        assert!(!l.fail(ip("192.0.2.1")));
        assert!(!l.fail(ip("192.0.2.2")));
        assert!(l.fail(ip("192.0.2.3")));
        assert!(l.locked(ip("192.0.2.4")), "a fourth new address is locked");
        assert!(l.begin(ip("192.0.2.5")).is_none());
        assert!(
            !l.locked(IpAddr::from(std::net::Ipv4Addr::from(0x0A00_0000))),
            "a remembered address keeps its own count"
        );
    }
}
