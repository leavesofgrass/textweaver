//! Host process management on the backend side: start a host, read its
//! replies on a background thread, send it requests, notice when it dies
//! or hangs, and shut it down.
//!
//! A backend keeps one [`HostProcess`] per engine process it runs (ECI: one;
//! SAPI: one per architecture). When a host dies ([`HostMsg::Closed`]) or
//! stays silent while it owes audio for longer than the backend's stall
//! timeout ([`HostProcess::stalled`]), the backend drops it, fails what it
//! owed ([`crate::Playback::host_died`]), and starts a new one on the next
//! request: restart after a crash or a hang.

use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

use crate::protocol::{self, Message};

/// How long [`HostProcess::shutdown`] waits for a clean exit after `Quit`
/// before killing the process.
pub const QUIT_GRACE: Duration = Duration::from_millis(500);

/// Something from a host's stdout.
#[derive(Debug)]
pub enum HostMsg<R> {
    /// A decoded reply.
    Reply(R),
    /// The host is gone (it exited, its pipe failed, or it sent a frame
    /// that does not decode); the text says which.
    Closed(String),
}

/// A running host process.
pub struct HostProcess<R> {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<HostMsg<R>>,
    path: PathBuf,
    label: &'static str,
    last_activity: Instant,
    _reply: PhantomData<fn() -> R>,
}

impl<R> std::fmt::Debug for HostProcess<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostProcess")
            .field("path", &self.path)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl<R: Message + Send + 'static> HostProcess<R> {
    /// Starts `path` with `args`. Its stderr goes to the log (debug level,
    /// prefixed with `label`); its replies are decoded on a reader thread.
    /// On Windows no console window opens for it.
    pub fn spawn<I, S>(path: &Path, args: I, label: &'static str) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(path);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: no console window flashes up for the host.
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", path.display()))?;
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("host has no stdout".into());
        };
        if let Some(stderr) = child.stderr.take() {
            let _ = std::thread::Builder::new()
                .name(format!("{label}-host-stderr"))
                .spawn(move || {
                    for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                        log::debug!("{label} host: {line}");
                    }
                });
        }
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::Builder::new()
            .name(format!("{label}-host-reader"))
            .spawn(move || {
                let mut r = BufReader::with_capacity(64 * 1024, stdout);
                loop {
                    let msg = match protocol::read_body(&mut r) {
                        Ok(Some(body)) => match R::decode(&body) {
                            Ok(reply) => HostMsg::Reply(reply),
                            Err(e) => HostMsg::Closed(format!("bad frame from host: {e}")),
                        },
                        Ok(None) => HostMsg::Closed("host exited".into()),
                        Err(e) => HostMsg::Closed(format!("host pipe failed: {e}")),
                    };
                    let closed = matches!(msg, HostMsg::Closed(_));
                    if tx.send(msg).is_err() || closed {
                        break;
                    }
                }
            });
        if let Err(e) = reader {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.to_string());
        }
        let stdin = child.stdin.take();
        Ok(HostProcess {
            child,
            stdin,
            rx,
            path: path.to_path_buf(),
            label,
            last_activity: Instant::now(),
            _reply: PhantomData,
        })
    }
}

impl<R> HostProcess<R> {
    /// The executable.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes one encoded frame to the host's stdin.
    pub fn send_frame(&mut self, frame: &[u8]) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("host input closed")?;
        protocol::write_frame(stdin, frame).map_err(|e| format!("host pipe: {e}"))
    }

    /// Sends one request.
    pub fn send<M: Message>(&mut self, request: &M) -> Result<(), String> {
        self.send_frame(&request.encode())
    }

    /// The next message the host sent, if one is waiting. A host whose
    /// reader thread has ended reports [`HostMsg::Closed`].
    pub fn try_recv(&mut self) -> Option<HostMsg<R>> {
        let msg = match self.rx.try_recv() {
            Ok(m) => m,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => HostMsg::Closed("host exited".into()),
        };
        self.touch();
        Some(msg)
    }

    /// Waits up to `timeout` for the next message; `None` when none came.
    pub fn recv_timeout(&mut self, timeout: Duration) -> Option<HostMsg<R>> {
        let msg = match self.rx.recv_timeout(timeout) {
            Ok(m) => m,
            Err(RecvTimeoutError::Timeout) => return None,
            Err(RecvTimeoutError::Disconnected) => HostMsg::Closed("host exited".into()),
        };
        self.touch();
        Some(msg)
    }

    /// Waits until `deadline` for the next message; `None` when none came.
    pub fn recv_until(&mut self, deadline: Instant) -> Option<HostMsg<R>> {
        self.recv_timeout(deadline.saturating_duration_since(Instant::now()))
    }

    /// Marks the host as active now: it just sent something, or was just
    /// given work, so its stall timer restarts.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// How long since the host last sent anything (or was last touched).
    pub fn idle_for(&self) -> Duration {
        self.last_activity.elapsed()
    }

    /// True when the host owes output (`owes`) and has been silent for
    /// longer than `timeout`: the engine is hung.
    pub fn stalled(&self, owes: bool, timeout: Duration) -> bool {
        owes && self.idle_for() > timeout
    }

    /// Asks the host to quit, waits up to [`QUIT_GRACE`] for it to exit,
    /// then kills it.
    pub fn shutdown(&mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let _ = protocol::write_frame(&mut stdin, &protocol::encode_quit());
        }
        let deadline = Instant::now() + QUIT_GRACE;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                _ => break,
            }
        }
        self.kill();
    }

    /// Kills the host at once (a hung engine, or a one-shot listing run).
    pub fn kill(&mut self) {
        self.stdin = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl<R> Drop for HostProcess<R> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
