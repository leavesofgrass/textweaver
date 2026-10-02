//! A fetcher for tests: it hands out fixtures by address, goes on from an
//! offset when asked, records every request, and can fail, stop, or cancel
//! part way. Nothing goes to the network.

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::fetch::{Fetched, Fetcher};

/// One request a [`FakeFetcher`] answered: the address and the offset.
pub type Request = (String, u64);

/// Hands out fixtures by address and records each request.
#[derive(Debug, Default)]
pub struct FakeFetcher {
    files: HashMap<String, Vec<u8>>,
    asked: Mutex<Vec<Request>>,
    /// Ignore offsets: answer every request with the whole file, as a
    /// server without `Range` support does.
    pub whole_files_only: bool,
    /// Set this flag after handing out this many bytes of one answer
    /// (to cancel a download part way).
    cancel_after: Option<(u64, Arc<AtomicBool>)>,
    /// Fail with an error after this many bytes of one answer (a dropped
    /// connection).
    fail_after: Option<u64>,
}

impl FakeFetcher {
    /// A fetcher that has no files.
    pub fn new() -> Self {
        FakeFetcher::default()
    }

    /// Serves `bytes` at `address`.
    pub fn with(mut self, address: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.insert(address, bytes);
        self
    }

    /// Serves `bytes` at `address`.
    pub fn insert(&mut self, address: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.files.insert(address.into(), bytes.into());
    }

    /// The bytes served at `address`, to change in a test.
    pub fn bytes_mut(&mut self, address: &str) -> Option<&mut Vec<u8>> {
        self.files.get_mut(address)
    }

    /// Sets `flag` once `n` bytes of an answer were read.
    pub fn cancel_after(mut self, n: u64, flag: Arc<AtomicBool>) -> Self {
        self.cancel_after = Some((n, flag));
        self
    }

    /// Fails once `n` bytes of an answer were read.
    pub fn fail_after(mut self, n: u64) -> Self {
        self.fail_after = Some(n);
        self
    }

    /// Stops failing and cancelling.
    pub fn heal(&mut self) {
        self.fail_after = None;
        self.cancel_after = None;
    }

    /// Every request so far, in order.
    pub fn requests(&self) -> Vec<Request> {
        self.asked.lock().map(|a| a.clone()).unwrap_or_default()
    }

    /// How many requests were made.
    pub fn request_count(&self) -> usize {
        self.asked.lock().map(|a| a.len()).unwrap_or(0)
    }
}

/// Reads a fixture, cancelling or failing part way when asked.
struct Feed {
    data: Vec<u8>,
    at: usize,
    read: u64,
    cancel_after: Option<(u64, Arc<AtomicBool>)>,
    fail_after: Option<u64>,
}

impl Read for Feed {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(limit) = self.fail_after
            && self.read >= limit
        {
            return Err(std::io::Error::other("the connection dropped"));
        }
        if let Some((limit, flag)) = &self.cancel_after
            && self.read >= *limit
        {
            flag.store(true, Ordering::Relaxed);
        }
        // Small pieces, so cancelling and failing land part way.
        let n = buf.len().min(1024).min(self.data.len() - self.at);
        buf[..n].copy_from_slice(&self.data[self.at..self.at + n]);
        self.at += n;
        self.read += n as u64;
        Ok(n)
    }
}

impl Fetcher for FakeFetcher {
    fn open(&self, address: &str, from: u64) -> Result<Fetched, String> {
        if let Ok(mut a) = self.asked.lock() {
            a.push((address.to_owned(), from));
        }
        let data = self.files.get(address).ok_or("not found")?;
        let start = if self.whole_files_only || from > data.len() as u64 {
            0
        } else {
            from
        };
        let at = usize::try_from(start).unwrap_or(0);
        Ok(Fetched {
            reader: Box::new(Feed {
                data: data[at..].to_vec(),
                at: 0,
                read: 0,
                cancel_after: self.cancel_after.clone(),
                fail_after: self.fail_after,
            }),
            start,
        })
    }
}
