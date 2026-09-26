//! The host side: what every host process shares.
//!
//! A host reads requests on stdin with a [`RequestReader`] thread and
//! writes replies to stdout. The reader handles `Stop` itself by bumping a
//! [`StopEpoch`], so a stop reaches synthesis already in progress (the
//! engine checks [`StopEpoch::is_current`] as it goes); every other request
//! is stamped with the epoch current when it was read, and a `Speak`
//! stamped before the latest `Stop` ends as `Aborted` without synthesis.
//! `Quit` and end of input end the stream.
//!
//! [`SharedOut`] lets several threads (a host's main thread and an engine's
//! audio thread) write whole frames to one output.

use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use crate::protocol::{self, Message};

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
    /// Starts reading frames from `input` on a thread named `name`.
    pub fn spawn(input: impl Read + Send + 'static, name: &str) -> io::Result<Self> {
        let epoch = StopEpoch::new();
        let (tx, rx) = mpsc::channel();
        let e = epoch.clone();
        let mut input = input;
        std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                loop {
                    let item = match protocol::read_body(&mut input) {
                        Ok(None) => break,
                        Ok(Some(body)) if protocol::is_stop(&body) => {
                            e.bump();
                            continue;
                        }
                        Ok(Some(body)) if protocol::is_quit(&body) => break,
                        Ok(Some(body)) => match R::decode(&body) {
                            Ok(r) => Incoming::Request(r, e.current()),
                            Err(err) => Incoming::Bad(err.to_string()),
                        },
                        Err(err) => {
                            let _ = tx.send(Incoming::Bad(err.to_string()));
                            break;
                        }
                    };
                    if tx.send(item).is_err() {
                        break;
                    }
                }
            })?;
        Ok(RequestReader { rx, epoch })
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
