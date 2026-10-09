//! The session ending (signing out, shutting down, restarting) while the
//! window is open (Wave 9, W9a-w), and on Linux and macOS a termination
//! signal (beta 1).
//!
//! winit does not tell an app that the session is ending: Windows sends
//! `WM_ENDSESSION` and may end the process as soon as the window procedure
//! returns, so unsaved edits were lost. [`watch`] subclasses the window,
//! as the `--background` guard does; on `WM_ENDSESSION` it posts
//! [`SessionEnding`] to the window's own event loop and pumps messages
//! until the window says it has saved ([`saved`]), for at most
//! [`SAVE_WAIT`]. The window then runs the same save as after a panic
//! ([`crate::safety::save_after_trouble`]): the recovery copy, the place,
//! and the settings.
//!
//! On Linux and macOS the end comes as a signal: SIGHUP when the terminal
//! the window was started from closes or the session ends, SIGTERM at
//! logout, shutdown, or from `kill`, and SIGINT for Ctrl+C in that
//! terminal. Each one ended the process at once, losing unsaved edits.
//! [`watch`] catches them (the `ctrlc` crate, as the terminal reader
//! does); its handler thread posts [`SessionEnding`] in the same way,
//! waits for [`saved`] for at most [`SAVE_WAIT`], and then ends the
//! process. A session ended through the desktop's own logout, which closes
//! the window, saves as Alt+F4 does.

use std::time::Duration;

/// Posted to the window's event loop when the session is ending.
#[derive(Debug)]
pub struct SessionEnding;

/// The longest the window procedure waits for the save before letting the
/// session end.
pub const SAVE_WAIT: Duration = Duration::from_secs(3);

/// The function that asks the window to save (it posts [`SessionEnding`]).
pub type Request = Box<dyn Fn() + Send + Sync>;

/// Watches for the session ending, calling `request` when it does: the
/// window `hwnd` on Windows (a Win32 handle; 0 does nothing), the
/// termination signals on Linux and macOS (`hwnd` unused). Returns true
/// when the watch was installed.
pub fn watch(hwnd: isize, request: Request) -> bool {
    imp::watch(hwnd, request)
}

/// Tells the waiting window procedure that the save is done.
pub fn saved() {
    imp::saved();
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
    use std::time::Instant;

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DispatchMessageW, GWLP_WNDPROC, GetWindowLongPtrW, MSG, PM_REMOVE,
        PeekMessageW, SetWindowLongPtrW, TranslateMessage, WM_ENDSESSION, WNDPROC,
    };

    use super::{Request, SAVE_WAIT};

    #[cfg(target_pointer_width = "64")]
    type LongPtr = isize;
    #[cfg(not(target_pointer_width = "64"))]
    type LongPtr = i32;

    /// The procedure the watch replaced (winit's, or the guard's).
    static PREVIOUS: AtomicIsize = AtomicIsize::new(0);
    static REQUEST: OnceLock<Request> = OnceLock::new();
    static SAVED: AtomicBool = AtomicBool::new(false);
    static ENDING: AtomicBool = AtomicBool::new(false);

    pub(super) fn watch(h: isize, request: Request) -> bool {
        if h == 0 || REQUEST.set(request).is_err() {
            return false;
        }
        let w = HWND(h as *mut core::ffi::c_void);
        // SAFETY: `w` is this thread's own live window. The previous
        // procedure is stored before ours can run, and ours always calls it.
        unsafe {
            let previous = GetWindowLongPtrW(w, GWLP_WNDPROC);
            PREVIOUS.store(previous as isize, Ordering::Release);
            let proc_: extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT = wnd_proc;
            SetWindowLongPtrW(w, GWLP_WNDPROC, proc_ as usize as LongPtr);
        }
        true
    }

    pub(super) fn saved() {
        SAVED.store(true, Ordering::Release);
    }

    /// Asks the window to save, then pumps this thread's messages (the
    /// request arrives as one) until it has, or [`SAVE_WAIT`] passes.
    fn save_before_ending() {
        if ENDING.swap(true, Ordering::AcqRel) {
            return;
        }
        log::warn!("the session is ending; saving before it does");
        if let Some(request) = REQUEST.get() {
            request();
        }
        let until = Instant::now() + SAVE_WAIT;
        while !SAVED.load(Ordering::Acquire) && Instant::now() < until {
            let mut msg = MSG::default();
            // SAFETY: `msg` lives for the calls; the message loop calls are
            // the thread's own, as winit's loop makes them.
            let got = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool();
            if got {
                // SAFETY: as above.
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        if !SAVED.load(Ordering::Acquire) {
            log::error!("the session ended before the window could save");
        }
    }

    extern "system" fn wnd_proc(w: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // wParam is nonzero when the session really ends.
        if msg == WM_ENDSESSION && wparam.0 != 0 {
            save_before_ending();
        }
        let previous = PREVIOUS.load(Ordering::Acquire);
        // SAFETY: `previous` is the procedure this window had before the
        // watch (a valid WNDPROC, never 0 for a live window).
        unsafe {
            let previous: WNDPROC = std::mem::transmute::<isize, WNDPROC>(previous);
            CallWindowProcW(previous, w, msg, wparam, lparam)
        }
    }
}

#[cfg(unix)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use super::{Request, SAVE_WAIT};

    static SAVED: AtomicBool = AtomicBool::new(false);

    pub(super) fn watch(_hwnd: isize, request: Request) -> bool {
        // The handler runs on ctrlc's own thread, never inside the signal.
        let installed = ctrlc::set_handler(move || {
            let saved = save_before_ending(&request, &SAVED, SAVE_WAIT);
            std::process::exit(if saved { 0 } else { 1 });
        });
        if let Err(e) = &installed {
            log::warn!("cannot save when the session ends: {e}");
        }
        installed.is_ok()
    }

    pub(super) fn saved() {
        SAVED.store(true, Ordering::Release);
    }

    /// Asks the window to save, then waits until it has (`saved` is set)
    /// or `wait` passes; true when it saved.
    pub(super) fn save_before_ending(
        request: &Request,
        saved: &AtomicBool,
        wait: Duration,
    ) -> bool {
        log::warn!("the session is ending; saving before it does");
        request();
        let until = Instant::now() + wait;
        while !saved.load(Ordering::Acquire) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        let done = saved.load(Ordering::Acquire);
        if !done {
            log::error!("the session ended before the window could save");
        }
        done
    }

    #[cfg(test)]
    mod tests {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;

        use super::save_before_ending;

        #[test]
        fn the_wait_ends_when_the_window_has_saved() {
            let saved = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&saved);
            // The window saves on its own thread once asked.
            let request: super::Request = Box::new(move || {
                let flag = Arc::clone(&flag);
                std::thread::spawn(move || flag.store(true, Ordering::Release));
            });
            assert!(save_before_ending(
                &request,
                &saved,
                Duration::from_secs(10)
            ));
        }

        #[test]
        fn the_wait_gives_up_when_the_window_does_not_save() {
            let saved = AtomicBool::new(false);
            let request: super::Request = Box::new(|| {});
            assert!(!save_before_ending(
                &request,
                &saved,
                Duration::from_millis(30)
            ));
        }
    }
}

#[cfg(not(any(windows, unix)))]
mod imp {
    use super::Request;

    pub(super) fn watch(_hwnd: isize, _request: Request) -> bool {
        false
    }

    pub(super) fn saved() {}
}
