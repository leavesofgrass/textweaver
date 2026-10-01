//! Starting a host without waiting for it (Phase 2).
//!
//! A host takes a while to report `Ready`: a cold Eloquence loads its
//! dictionaries, a 32-bit SAPI voice starts a second host. Waiting for that
//! inside `speak` held the speech thread for up to ten seconds, so Stop and
//! Pause could not reach it. A [`HostStart`] is the start-up state: the
//! backend queues requests while it is [`Start::Pending`], and its `poll`
//! (called every few milliseconds by the speech service) calls
//! [`HostStart::poll`], which reads what the host has sent and enforces
//! the deadline. When one candidate executable fails, the next is tried.
//! Where waiting is right (a backend's first start, synthesizing to a
//! file), [`HostStart::wait`] blocks until the outcome.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::clock::Clock;
use crate::process::{HostMsg, HostProcess};
use crate::protocol::Message;

/// How a reply received before `Ready` counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Class {
    /// This is `Ready` (its version checked): the host has started.
    Ready,
    /// Progress (ECI's dictionary reports): kept, and the deadline starts
    /// again.
    Progress,
    /// The host cannot start: this is why.
    Fail(String),
}

/// A host that reported `Ready`.
#[derive(Debug)]
pub struct Started<R> {
    /// The executable that started.
    pub path: PathBuf,
    /// The running host.
    pub process: HostProcess<R>,
    /// Its `Ready` reply.
    pub ready: R,
    /// Replies that came before `Ready` ([`Class::Progress`]), in order.
    pub early: Vec<R>,
}

/// Where a start stands.
#[derive(Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "returned once per poll and taken apart at once; boxing would only add an allocation"
)]
pub enum Start<R> {
    /// Still starting.
    Pending,
    /// Started.
    Ready(Started<R>),
    /// No candidate started; every failure, one per executable tried.
    Failed(String),
}

/// Starts one executable (spawns it, without waiting).
pub type Spawner<R> = Box<dyn FnMut(&Path) -> Result<HostProcess<R>, String>>;

/// A host start in progress. See the module docs.
pub struct HostStart<R> {
    candidates: VecDeque<PathBuf>,
    spawn: Spawner<R>,
    current: Option<(PathBuf, HostProcess<R>)>,
    timeout: Duration,
    /// The clock the deadline is on (also given to each host started).
    clock: Clock,
    deadline: Instant,
    early: Vec<R>,
    errors: Vec<String>,
}

impl<R> std::fmt::Debug for HostStart<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostStart")
            .field("current", &self.current.as_ref().map(|(p, _)| p))
            .field("candidates", &self.candidates)
            .finish_non_exhaustive()
    }
}

impl<R: Message + Send + 'static> HostStart<R> {
    /// Starts the first of `candidates` with `spawn` (at once, without
    /// waiting). Each may take `timeout` to report `Ready`, restarted by
    /// every [`Class::Progress`] reply.
    pub fn begin(candidates: Vec<PathBuf>, timeout: Duration, spawn: Spawner<R>) -> Self {
        Self::begin_with_clock(candidates, timeout, spawn, Clock::system())
    }

    /// [`begin`](Self::begin) with the deadline on `clock`, which each
    /// host started also gets for its stall timer
    /// ([`HostProcess::set_clock`]). With a [`Clock::manual`] one, a test
    /// moves the deadline past by hand and [`poll`](Self::poll) sees it.
    pub fn begin_with_clock(
        candidates: Vec<PathBuf>,
        timeout: Duration,
        spawn: Spawner<R>,
        clock: Clock,
    ) -> Self {
        let mut s = HostStart {
            candidates: candidates.into(),
            spawn,
            current: None,
            timeout,
            deadline: clock.now() + timeout,
            clock,
            early: Vec::new(),
            errors: Vec::new(),
        };
        s.next_candidate();
        s
    }

    /// Spawns the next candidate; false when none is left.
    fn next_candidate(&mut self) -> bool {
        while let Some(path) = self.candidates.pop_front() {
            match (self.spawn)(&path) {
                Ok(mut p) => {
                    p.set_clock(self.clock.clone());
                    self.current = Some((path, p));
                    self.deadline = self.clock.now() + self.timeout;
                    self.early.clear();
                    return true;
                }
                Err(e) => self.errors.push(format!("{}: {e}", path.display())),
            }
        }
        false
    }

    /// The current candidate failed: kills it and moves to the next.
    fn fail_current(&mut self, why: String) {
        if let Some((path, mut p)) = self.current.take() {
            p.kill();
            self.errors.push(format!("{}: {why}", path.display()));
        }
        self.next_candidate();
    }

    fn outcome(&mut self) -> Start<R> {
        if self.current.is_some() {
            Start::Pending
        } else if self.errors.is_empty() {
            Start::Failed("no host to start".into())
        } else {
            Start::Failed(self.errors.join("; "))
        }
    }

    /// Handles one message; `Some` when the start is decided.
    fn handle(
        &mut self,
        msg: HostMsg<R>,
        classify: &mut impl FnMut(&R) -> Class,
    ) -> Option<Start<R>> {
        match msg {
            HostMsg::Reply(r) => match classify(&r) {
                Class::Ready => {
                    let (path, process) = self.current.take()?;
                    Some(Start::Ready(Started {
                        path,
                        process,
                        ready: r,
                        early: std::mem::take(&mut self.early),
                    }))
                }
                Class::Progress => {
                    self.early.push(r);
                    self.deadline = self.clock.now() + self.timeout;
                    None
                }
                Class::Fail(why) => {
                    self.fail_current(why);
                    None
                }
            },
            HostMsg::Closed(why) => {
                self.fail_current(why);
                None
            }
        }
    }

    /// Reads what the host sent so far, without waiting, and enforces the
    /// deadline.
    pub fn poll(&mut self, mut classify: impl FnMut(&R) -> Class) -> Start<R> {
        loop {
            let Some((_, p)) = self.current.as_mut() else {
                return self.outcome();
            };
            match p.try_recv() {
                Some(msg) => {
                    if let Some(done) = self.handle(msg, &mut classify) {
                        return done;
                    }
                }
                None => {
                    if self.clock.now() >= self.deadline {
                        self.fail_current("the host did not start in time".into());
                        continue;
                    }
                    return Start::Pending;
                }
            }
        }
    }

    /// Waits until the host has started or every candidate has failed.
    /// The wait is real time: as long as the clock's time left before the
    /// deadline.
    pub fn wait(&mut self, mut classify: impl FnMut(&R) -> Class) -> Start<R> {
        loop {
            let left = self.deadline.saturating_duration_since(self.clock.now());
            let Some((_, p)) = self.current.as_mut() else {
                return self.outcome();
            };
            match p.recv_timeout(left) {
                Some(msg) => {
                    if let Some(done) = self.handle(msg, &mut classify) {
                        return done;
                    }
                }
                None => self.fail_current("the host did not start in time".into()),
            }
        }
    }

    /// Gives up: kills the host being started.
    pub fn cancel(&mut self) {
        if let Some((_, mut p)) = self.current.take() {
            p.kill();
        }
        self.candidates.clear();
    }
}

impl<R> Drop for HostStart<R> {
    fn drop(&mut self) {
        if let Some((_, p)) = self.current.as_mut() {
            p.kill();
        }
    }
}
