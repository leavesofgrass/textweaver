//! The host side: what every host process shares.
//!
//! A host reads requests on stdin with a [`RequestReader`] thread and
//! writes replies to stdout. The reader handles `Stop` itself by bumping a
//! [`StopEpoch`], so a stop reaches synthesis already in progress (the
//! engine checks [`StopEpoch::is_current`] as it goes); every other request
//! is stamped with the epoch current when it was read, and a `Speak`
//! stamped before the latest `Stop` ends as `Aborted` without synthesis.
//! `Quit` ends the stream after what was read before it; end of input does
//! what [`AtEnd`] says (a host process uses [`AtEnd::Exit`]: textweaver is
//! gone, so the host stops and exits even if its engine is stuck).
//!
//! [`SharedOut`] lets several threads (a host's main thread and an engine's
//! audio thread) write whole frames to one output, and [`log_line`] writes
//! to stderr without ever failing.

use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::protocol::{self, Message, ReadFrame};

/// How long a host process may take to finish on its own after its input
/// ends ([`AtEnd::Exit`]) before it exits regardless.
pub const END_OF_INPUT_GRACE: Duration = Duration::from_millis(1500);

/// What the [`RequestReader`] does when its input ends (or breaks) without
/// a `Quit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtEnd {
    /// End the stream after the requests already read, which are still
    /// carried out (in-process tests that feed a finished buffer).
    Finish,
    /// Bump the stop epoch, so synthesis in progress aborts and every
    /// request already read is skipped, then end the stream.
    Stop,
    /// As [`Stop`](Self::Stop), then exit the whole process after the
    /// grace period if it is still running (an engine stuck in synthesis
    /// that never checks the epoch). What host processes use: their input
    /// ends when textweaver exits or dies, and nobody is listening any
    /// more.
    Exit(Duration),
}

impl AtEnd {
    /// [`AtEnd::Exit`] with [`END_OF_INPUT_GRACE`]: for host processes.
    pub fn host() -> Self {
        AtEnd::Exit(END_OF_INPUT_GRACE)
    }
}

/// Writes one line to stderr, ignoring every error. Hosts log with this
/// instead of `eprintln!`, which panics when stderr is closed; a panic in
/// an engine callback called from C aborts the host.
pub fn log_line(line: &str) {
    let mut err = io::stderr().lock();
    let _ = writeln!(err, "{line}");
    let _ = err.flush();
}

/// The stop epoch: how many `Stop` requests the host has read.
#[derive(Clone, Debug, Default)]
pub struct StopEpoch(Arc<AtomicU64>);

impl StopEpoch {
    /// A new epoch at 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current epoch.
    pub fn current(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }

    /// Records a `Stop`; returns the new epoch.
    pub fn bump(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// True when no `Stop` arrived since `stamp` was taken.
    pub fn is_current(&self, stamp: u64) -> bool {
        self.current() == stamp
    }
}

/// One item from the [`RequestReader`].
#[derive(Debug, PartialEq, Eq)]
pub enum Incoming<R> {
    /// A request, stamped with the stop epoch current when it was read.
    Request(R, u64),
    /// A frame that did not decode, or a failed read (after which the
    /// stream ends). The text says what went wrong.
    Bad(String),
}

/// Reads requests on a background thread (see the module docs).
#[derive(Debug)]
pub struct RequestReader<R> {
    rx: Receiver<Incoming<R>>,
    epoch: StopEpoch,
}

impl<R: Message + Send + 'static> RequestReader<R> {
    /// Starts reading frames from `input` on a thread named `name`, with
    /// [`AtEnd::Finish`].
    pub fn spawn(input: impl Read + Send + 'static, name: &str) -> io::Result<Self> {
        Self::spawn_with(input, name, AtEnd::Finish)
    }

    /// Starts reading frames from `input` on a thread named `name`; `at_end`
    /// says what happens when the input ends without a `Quit`.
    ///
    /// A request over the protocol's frame limit is read past and reported
    /// as [`Incoming::Bad`]; the host keeps working.
    pub fn spawn_with(
        input: impl Read + Send + 'static,
        name: &str,
        at_end: AtEnd,
    ) -> io::Result<Self> {
        let epoch = StopEpoch::new();
        let (tx, rx) = mpsc::channel();
        let e = epoch.clone();
        let mut input = input;
        std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let quit = read_requests(&mut input, &tx, &e);
                // The main thread sees the end of the stream now.
                drop(tx);
                if quit {
                    return;
                }
                match at_end {
                    AtEnd::Finish => {}
                    AtEnd::Stop => {
                        e.bump();
                    }
                    AtEnd::Exit(grace) => {
                        e.bump();
                        std::thread::sleep(grace);
                        log_line("engine host: input closed and the engine did not stop; exiting");
                        std::process::exit(0);
                    }
                }
            })?;
        Ok(RequestReader { rx, epoch })
    }
}

/// Reads and forwards requests until the input ends or breaks (false) or
/// a `Quit` arrives, or nobody listens any more (true).
fn read_requests<R: Message>(
    input: &mut impl Read,
    tx: &mpsc::Sender<Incoming<R>>,
    epoch: &StopEpoch,
) -> bool {
    loop {
        let item = match protocol::read_body_or_skip(input) {
            Ok(None) => return false,
            Ok(Some(ReadFrame::Skipped(len))) => Incoming::Bad(format!(
                "request of {len} bytes is over the {} byte limit; skipped",
                protocol::MAX_FRAME
            )),
            Ok(Some(ReadFrame::Body(body))) if protocol::is_stop(&body) => {
                epoch.bump();
                continue;
            }
            Ok(Some(ReadFrame::Body(body))) if protocol::is_quit(&body) => return true,
            Ok(Some(ReadFrame::Body(body))) => match R::decode(&body) {
                Ok(r) => Incoming::Request(r, epoch.current()),
                Err(err) => Incoming::Bad(err.to_string()),
            },
            Err(err) => {
                let _ = tx.send(Incoming::Bad(err.to_string()));
                return false;
            }
        };
        if tx.send(item).is_err() {
            return true;
        }
    }
}

impl<R> RequestReader<R> {
    /// The next item; `None` after `Quit`, end of input, or a failed read.
    pub fn next(&self) -> Option<Incoming<R>> {
        self.rx.recv().ok()
    }

    /// The stop epoch the reader bumps.
    pub fn epoch(&self) -> &StopEpoch {
        &self.epoch
    }
}

/// An output several threads write whole frames to.
#[derive(Debug)]
pub struct SharedOut<W: Write> {
    w: Mutex<W>,
}

impl<W: Write> SharedOut<W> {
    /// Wraps `w`.
    pub fn new(w: W) -> Self {
        SharedOut { w: Mutex::new(w) }
    }

    /// Writes one frame and flushes it.
    pub fn send_frame(&self, frame: &[u8]) -> io::Result<()> {
        let mut w = self.w.lock().unwrap_or_else(|e| e.into_inner());
        protocol::write_frame(&mut *w, frame)
    }

    /// Writes one message and flushes it.
    pub fn send<M: Message>(&self, message: &M) -> io::Result<()> {
        self.send_frame(&message.encode())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::protocol::{FrameReader, FrameWriter, ProtocolError, encode_quit, encode_stop};

    /// A toy request type: `0x01 token` (Speak) or `0x03 value` (Set).
    #[derive(Debug, PartialEq, Eq)]
    enum Req {
        Speak(u64),
        Set(u8),
    }

    impl Message for Req {
        fn encode(&self) -> Vec<u8> {
            match self {
                Req::Speak(t) => {
                    let mut e = FrameWriter::new(0x01);
                    e.u64(*t);
                    e.finish()
                }
                Req::Set(v) => {
                    let mut e = FrameWriter::new(0x03);
                    e.u8(*v);
                    e.finish()
                }
            }
        }
        fn decode(body: &[u8]) -> Result<Self, ProtocolError> {
            let (t, mut r) = FrameReader::open(body)?;
            let m = match t {
                0x01 => Req::Speak(r.u64()?),
                0x03 => Req::Set(r.u8()?),
                other => return Err(ProtocolError::BadTag(other)),
            };
            r.finish()?;
            Ok(m)
        }
    }

    fn read_all(input: Vec<u8>) -> Vec<Incoming<Req>> {
        let reader = RequestReader::<Req>::spawn(Cursor::new(input), "test-reader").unwrap();
        std::iter::from_fn(|| reader.next()).collect()
    }

    #[test]
    fn requests_are_stamped_with_the_stop_epoch() {
        let input = [
            Req::Speak(1).encode(),
            encode_stop(),
            Req::Set(4).encode(),
            encode_stop(),
            encode_stop(),
            Req::Speak(2).encode(),
        ]
        .concat();
        assert_eq!(
            read_all(input),
            [
                Incoming::Request(Req::Speak(1), 0),
                Incoming::Request(Req::Set(4), 1),
                Incoming::Request(Req::Speak(2), 3),
            ]
        );
    }

    #[test]
    fn quit_ends_the_stream_and_bad_frames_are_reported() {
        let mut bad = Req::Set(1).encode();
        bad[4] = 0x7e;
        let input = [
            bad,
            Req::Speak(5).encode(),
            encode_quit(),
            Req::Speak(6).encode(),
        ]
        .concat();
        let got = read_all(input);
        assert!(matches!(got[0], Incoming::Bad(_)));
        assert_eq!(got[1..], [Incoming::Request(Req::Speak(5), 0)]);
    }

    #[test]
    fn a_broken_stream_reports_once_and_ends() {
        let mut input = Req::Speak(1).encode();
        input.extend_from_slice(&[9, 0, 0]);
        let got = read_all(input);
        assert_eq!(got.len(), 2);
        assert!(matches!(&got[1], Incoming::Bad(m) if m.contains("ended inside")));
    }

    #[test]
    fn a_request_over_the_frame_limit_is_skipped_and_reading_goes_on() {
        // Before: the reader stopped at the oversized length, so the host
        // ended and textweaver had to start a new one.
        let len = protocol::MAX_FRAME + 10;
        let mut input = u32::try_from(len).unwrap().to_le_bytes().to_vec();
        input.resize(4 + len, 0x01);
        input.extend(Req::Speak(7).encode());
        let got = read_all(input);
        assert_eq!(got.len(), 2, "{:?}", got.first());
        assert!(
            matches!(&got[0], Incoming::Bad(m) if m.contains("skipped")),
            "{:?}",
            got[0]
        );
        assert_eq!(got[1], Incoming::Request(Req::Speak(7), 0));
    }

    fn read_all_with(input: Vec<u8>, at_end: AtEnd) -> (Vec<Incoming<Req>>, u64) {
        let reader =
            RequestReader::<Req>::spawn_with(Cursor::new(input), "test-reader", at_end).unwrap();
        let items: Vec<_> = std::iter::from_fn(|| reader.next()).collect();
        // The reader thread bumps the epoch after closing the stream.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while at_end != AtEnd::Finish
            && reader.epoch().current() == 0
            && std::time::Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        (items, reader.epoch().current())
    }

    #[test]
    fn end_of_input_stops_what_was_read_unless_asked_to_finish() {
        let input = Req::Speak(1).encode();
        let (items, epoch) = read_all_with(input.clone(), AtEnd::Stop);
        assert_eq!(items, [Incoming::Request(Req::Speak(1), 0)]);
        assert_eq!(epoch, 1, "the Speak read before the end is now stale");
        let (_, epoch) = read_all_with(input.clone(), AtEnd::Finish);
        assert_eq!(epoch, 0);
        // Quit is a clean end: what was read is still carried out.
        let quit = [input, encode_quit()].concat();
        let (items, epoch) = read_all_with(quit, AtEnd::Finish);
        assert_eq!(items.len(), 1);
        assert_eq!(epoch, 0);
    }

    #[test]
    fn epochs_count_stops() {
        let e = StopEpoch::new();
        let stamp = e.current();
        assert!(e.is_current(stamp));
        assert_eq!(e.bump(), 1);
        assert!(!e.is_current(stamp));
        assert!(e.clone().is_current(1));
    }

    #[test]
    fn shared_output_writes_whole_frames() {
        let out = SharedOut::new(Vec::new());
        out.send(&Req::Speak(3)).unwrap();
        out.send_frame(&encode_quit()).unwrap();
        let bytes = out.w.into_inner().unwrap();
        assert_eq!(bytes, [Req::Speak(3).encode(), encode_quit()].concat());
    }
}
