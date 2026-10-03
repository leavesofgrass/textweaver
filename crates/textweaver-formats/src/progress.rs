//! Progress reports and cancelling for slow loads (OCR of a scanned book,
//! a large archive, a web page).
//!
//! A [`Progress`] travels in [`LoadOptions`](crate::LoadOptions). It is
//! not part of the options' identity: two options that differ only in
//! their progress handle are equal, hash alike, and serialize alike, so
//! the document cache is unaffected.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// One progress report: `done` of `total` steps, and what is happening,
/// as a sentence that reads well aloud ("Recognizing text on page 3 of
/// 40.").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressReport {
    /// Steps finished.
    pub done: usize,
    /// Steps in all.
    pub total: usize,
    /// What is happening.
    pub message: String,
}

type Callback = Box<dyn Fn(&ProgressReport) + Send + Sync>;

struct Inner {
    cancelled: AtomicBool,
    report: Callback,
}

/// A handle for progress reports and cancelling. The default reports
/// nothing and is never cancelled.
#[derive(Clone, Default)]
pub struct Progress {
    inner: Option<Arc<Inner>>,
}

/// Never set: the cancel flag of [`Progress::default`].
static NEVER: AtomicBool = AtomicBool::new(false);

impl Progress {
    /// A handle that calls `report` with each report (from the loading
    /// thread).
    pub fn new(report: impl Fn(&ProgressReport) + Send + Sync + 'static) -> Self {
        Progress {
            inner: Some(Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                report: Box::new(report),
            })),
        }
    }

    /// Asks the load to stop (from any thread). A loader stops at its next
    /// step and returns what it has, saying the rest was not read.
    pub fn cancel(&self) {
        if let Some(i) = &self.inner {
            i.cancelled.store(true, Ordering::Relaxed);
        }
    }

    /// True once [`cancel`](Self::cancel) was called.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag().load(Ordering::Relaxed)
    }

    /// The cancel flag itself (for engines that watch one).
    pub fn cancel_flag(&self) -> &AtomicBool {
        self.inner.as_ref().map_or(&NEVER, |i| &i.cancelled)
    }

    /// Sends a report.
    pub fn report(&self, done: usize, total: usize, message: impl Into<String>) {
        if let Some(i) = &self.inner {
            (i.report)(&ProgressReport {
                done,
                total,
                message: message.into(),
            });
        }
    }
}

impl std::fmt::Debug for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Progress")
            .field("reports", &self.inner.is_some())
            .field("canceled", &self.is_cancelled())
            .finish()
    }
}

impl PartialEq for Progress {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Progress {}

impl std::hash::Hash for Progress {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[test]
    fn reports_and_cancels() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = Arc::clone(&seen);
        let p = Progress::new(move |r| s.lock().unwrap().push(r.done));
        let q = p.clone();
        p.report(1, 2, "one");
        assert!(!q.is_cancelled());
        q.cancel();
        assert!(p.is_cancelled());
        assert_eq!(*seen.lock().unwrap(), vec![1]);
        let none = Progress::default();
        none.cancel();
        assert!(!none.is_cancelled());
        assert_eq!(p, none);
    }
}
