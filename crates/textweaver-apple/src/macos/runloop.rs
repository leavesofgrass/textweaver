//! Run-loop helpers.
//!
//! `AVSpeechSynthesizer` delivers its buffers and delegate callbacks through
//! the main dispatch queue (probe 5): the synthesizer can be created and
//! driven on the speech thread, but its callbacks arrive only while the
//! process's main thread services the main queue, which it does while it
//! runs its run loop. A terminal application whose main thread runs its own
//! event loop therefore either runs that loop on another thread and gives
//! the main thread to [`run_main_loop_until`], or calls [`pump_main_loop`]
//! from the main thread on every tick.

use std::time::{Duration, Instant};

use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop, NSThread};

/// True on the process's main thread.
pub fn is_main_thread() -> bool {
    NSThread::isMainThread_class()
}

/// Runs the current thread's run loop once, waiting at most `max` for
/// something to handle. When the run loop has no sources it returns at
/// once; this then sleeps for up to 2 ms so callers polling in a loop do not
/// spin.
pub(crate) fn run_current_once(max: Duration) {
    let run_loop = NSRunLoop::currentRunLoop();
    let limit = NSDate::dateWithTimeIntervalSinceNow(max.as_secs_f64());
    // SAFETY: `NSDefaultRunLoopMode` is an immutable constant string.
    let mode = unsafe { NSDefaultRunLoopMode };
    if !run_loop.runMode_beforeDate(mode, &limit) {
        std::thread::sleep(max.min(Duration::from_millis(2)));
    }
}

/// Services the main run loop (and with it the main dispatch queue) for up
/// to `max`. Call it from the main thread; returns false, doing nothing,
/// elsewhere.
pub fn pump_main_loop(max: Duration) -> bool {
    if !is_main_thread() {
        return false;
    }
    let deadline = Instant::now() + max;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        run_current_once(left);
        if Instant::now() >= deadline {
            return true;
        }
    }
}

/// Runs the main run loop until `done` returns true, checking it at least
/// every 10 ms. Call it from the main thread (typically after moving the
/// application's own loop to another thread); returns false at once
/// elsewhere.
pub fn run_main_loop_until(mut done: impl FnMut() -> bool) -> bool {
    if !is_main_thread() {
        return false;
    }
    while !done() {
        run_current_once(Duration::from_millis(10));
    }
    true
}
