//! Ending well when the terminal reader is told to stop from outside:
//! Ctrl+C (outside raw mode), SIGTERM, SIGHUP (the terminal closed), and on
//! Windows the console window closing, logoff, and shutdown.
//!
//! A handler only sets a flag; the event loop checks it on every pass and
//! then shuts down as quitting does, except that nothing is asked: unsaved
//! edits go to the recovery snapshot, the reading position is saved, and
//! the terminal is restored. Windows ends a process soon after its console
//! closes, so there the handler waits (at most
//! [`CLOSE_GRACE`]) for the loop to say it has finished.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static REQUESTED: AtomicBool = AtomicBool::new(false);
static FINISHED: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// How long a closing Windows console waits for the reader to save.
pub const CLOSE_GRACE: Duration = Duration::from_millis(4500);

/// Installs the handlers (once; later calls do nothing). Returns a message
/// for the log when they could not be installed.
pub fn install() -> Option<String> {
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return None;
    }
    let result = ctrlc::set_handler(request);
    #[cfg(windows)]
    close::install();
    result
        .err()
        .map(|e| format!("cannot handle Ctrl+C and termination signals: {e}"))
}

/// Asks the event loop to shut down (what the handlers call).
pub fn request() {
    REQUESTED.store(true, Ordering::SeqCst);
}

/// True once a shutdown was asked for from outside.
pub fn requested() -> bool {
    REQUESTED.load(Ordering::SeqCst)
}

/// The loop has saved and restored the terminal; a waiting Windows close
/// handler may let the process end.
pub fn finished() {
    FINISHED.store(true, Ordering::SeqCst);
}

/// Clears both flags (tests).
pub fn reset() {
    REQUESTED.store(false, Ordering::SeqCst);
    FINISHED.store(false, Ordering::SeqCst);
}

/// Waits until [`finished`] is called or `limit` passes; true when it was.
pub fn wait_finished(limit: Duration) -> bool {
    let deadline = std::time::Instant::now() + limit;
    while !FINISHED.load(Ordering::SeqCst) {
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod close {
    //! The console closing, logoff, and shutdown: Windows ends the process
    //! when the handler returns, so this one waits for the loop first.

    use windows::Win32::Foundation::{FALSE, TRUE};
    use windows::Win32::System::Console::{
        CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT, SetConsoleCtrlHandler,
    };
    use windows::core::BOOL;

    unsafe extern "system" fn handler(event: u32) -> BOOL {
        if matches!(
            event,
            CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT
        ) {
            super::request();
            super::wait_finished(super::CLOSE_GRACE);
            TRUE
        } else {
            // Ctrl+C and Ctrl+Break: the ctrlc handler sets the flag.
            FALSE
        }
    }

    pub(super) fn install() {
        // SAFETY: `handler` is a plain function that lives for the whole
        // program; adding it (TRUE) puts it before the ctrlc handler.
        let added = unsafe { SetConsoleCtrlHandler(Some(handler), true) };
        if let Err(e) = added {
            log::warn!("cannot handle the console closing: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_is_seen_and_finishing_releases_the_wait() {
        reset();
        assert!(!requested());
        request();
        assert!(requested());
        let waiter = std::thread::spawn(|| wait_finished(Duration::from_secs(10)));
        finished();
        assert!(waiter.join().unwrap());
        reset();
        assert!(!wait_finished(Duration::from_millis(30)));
    }
}
