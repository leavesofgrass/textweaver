//! `--background` on Windows: a window that gives the foreground back.
//!
//! Automated checks (the UI Automation report, the tree dump) run the GUI
//! off screen, never activated, with no taskbar button. That kept it from
//! taking the foreground when it started, but pressing a control through
//! UI Automation still brought it to the front and took the focus from the
//! person at the machine (found by W6a6, Wave 6).
//!
//! **The cause (W7x).** No code in the window's process asks for it:
//! textweaver, Masonry, AccessKit and winit never call
//! `SetForegroundWindow`, `SetActiveWindow` or `BringWindowToTop` on a
//! button press (winit's `focus_window` is the only caller, and Masonry
//! never asks for it; muda calls it only for a context menu, which the GUI
//! does not use). winit shows the window with `SW_SHOWNOACTIVATE` and
//! changes its styles with `SWP_NOACTIVATE`. The report's one run with this
//! guard logged where the activation arrives: `WM_ACTIVATE` sent from
//! another thread, delivered inside winit's `PeekMessageW`, three times in a
//! row on the first `InvokePattern.Invoke` and once on the palette run's,
//! and not again once the window was in front. So the window is activated
//! from outside the window's own code, by UI Automation acting for the
//! client (which may move the foreground: it descends from the process the
//! person last used, or the foreground lock has timed out), and it does so
//! with an explicit activation that `WS_EX_NOACTIVATE` does not stop.
//!
//! A window cannot refuse an activation it is told about, so [`guard`]:
//!
//! - gives the window `WS_EX_NOACTIVATE` (the style for a window that
//!   "should not be activated through programmatic access ... by
//!   accessible technology"), and keeps it, because winit rewrites the
//!   extended style from its own flags whenever they change;
//! - answers `WM_MOUSEACTIVATE` with `MA_NOACTIVATE`;
//! - when the window is activated all the same, posts itself a message and,
//!   once the activation is over, hands the foreground back to the window
//!   that had it (giving it back inside `WM_ACTIVATE` lost the race in the
//!   report's run: the activation that was still under way won);
//! - and on every tick, if the window is somehow in front, gives the
//!   foreground back too.
//!
//! Each activation is written to the log (`--log`), with where it came
//! from the first time, so the UI Automation report can fail on it.
//!
//! Elsewhere [`guard`] does nothing: macOS and Linux windows are not
//! activated by accessibility actions.

/// The extended window style that keeps a window from being activated.
pub const WS_EX_NOACTIVATE: u32 = 0x0800_0000;

/// `ex` with [`WS_EX_NOACTIVATE`] added.
pub fn with_no_activate(ex: u32) -> u32 {
    ex | WS_EX_NOACTIVATE
}

/// Where the foreground goes back to when the background window has been
/// activated: the window `WM_ACTIVATE` says was deactivated, else the last
/// foreground window seen that was not this one. None when the message is
/// not an activation, or there is nothing to give it back to.
pub fn give_back_target(
    activated: bool,
    deactivated: isize,
    last_foreground: isize,
    ours: isize,
) -> Option<isize> {
    if !activated {
        return None;
    }
    [deactivated, last_foreground]
        .into_iter()
        .find(|&w| w != 0 && w != ours)
}

/// How many times the background window was activated (0 when [`guard`]
/// was never installed).
pub fn activations() -> usize {
    imp::activations()
}

/// Keeps the window `hwnd` (a Win32 handle; 0 does nothing) out of the
/// foreground; `log` writes each activation to the `--log`. Called once,
/// for `--background` runs. Returns true when the guard is in place.
pub fn guard(hwnd: isize, log: bool) -> bool {
    imp::guard(hwnd, log)
}

/// On every tick: remembers the current foreground window when it is not
/// the guarded one, so an activation can be given back even when Windows
/// does not say which window lost it, and gives the foreground back if the
/// guarded window has it. Cheap.
pub fn note_foreground() {
    imp::note_foreground();
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, GWL_EXSTYLE, GWLP_WNDPROC, GetForegroundWindow, GetWindowLongPtrW,
        IsWindowVisible, MA_NOACTIVATE, PostMessageW, STYLESTRUCT, SetForegroundWindow,
        SetWindowLongPtrW, WA_INACTIVE, WM_ACTIVATE, WM_APP, WM_MOUSEACTIVATE, WM_STYLECHANGING,
        WNDPROC,
    };

    use super::{give_back_target, with_no_activate};

    // windows-rs gives `SetWindowLongPtrW` an `i32` on 32-bit Windows.
    #[cfg(target_pointer_width = "64")]
    type LongPtr = isize;
    #[cfg(not(target_pointer_width = "64"))]
    type LongPtr = i32;

    /// Posted to the window when it has been activated: give the
    /// foreground back once the activation is over.
    const GIVE_BACK: u32 = WM_APP + 0x0731;

    /// The guarded window (one per process: the `--background` window).
    static OURS: AtomicIsize = AtomicIsize::new(0);
    /// The window procedure the guard replaced (winit's, or a subclass's).
    static PREVIOUS: AtomicIsize = AtomicIsize::new(0);
    /// The last foreground window seen that was not ours.
    static LAST_FOREGROUND: AtomicIsize = AtomicIsize::new(0);
    /// Where the pending give-back goes.
    static GIVE_BACK_TO: AtomicIsize = AtomicIsize::new(0);
    static LOG: AtomicBool = AtomicBool::new(false);
    static ACTIVATIONS: AtomicUsize = AtomicUsize::new(0);

    fn hwnd(h: isize) -> HWND {
        HWND(h as *mut core::ffi::c_void)
    }

    fn foreground() -> isize {
        // SAFETY: no arguments; returns a handle or null.
        unsafe { GetForegroundWindow() }.0 as isize
    }

    pub(super) fn activations() -> usize {
        ACTIVATIONS.load(Ordering::Relaxed)
    }

    pub(super) fn note_foreground() {
        let ours = OURS.load(Ordering::Relaxed);
        if ours == 0 {
            return;
        }
        let fg = foreground();
        if fg == ours {
            // In front with no activation seen (or one whose give-back
            // failed): give it back now.
            let back = LAST_FOREGROUND.load(Ordering::Relaxed);
            if back != 0 {
                let given = give_back(back);
                line(&format!(
                    "background: the window was in front at a tick; foreground given back: {}",
                    yes_no(given)
                ));
            }
        } else if fg != 0 {
            LAST_FOREGROUND.store(fg, Ordering::Relaxed);
        }
    }

    pub(super) fn guard(h: isize, log: bool) -> bool {
        if h == 0 {
            return false;
        }
        if OURS
            .compare_exchange(0, h, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            // Already guarded (this window, or the process's other one).
            return OURS.load(Ordering::Acquire) == h;
        }
        LOG.store(log, Ordering::Relaxed);
        let fg = foreground();
        if fg != 0 && fg != h {
            LAST_FOREGROUND.store(fg, Ordering::Relaxed);
        }
        let w = hwnd(h);
        // SAFETY: `w` is this thread's own live window. The previous
        // procedure is stored before ours can run, and ours always calls it.
        unsafe {
            let previous = GetWindowLongPtrW(w, GWLP_WNDPROC);
            PREVIOUS.store(previous as isize, Ordering::Release);
            let proc_: extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT = wnd_proc;
            SetWindowLongPtrW(w, GWLP_WNDPROC, proc_ as usize as LongPtr);
            let ex = GetWindowLongPtrW(w, GWL_EXSTYLE) as u32;
            // Through the new procedure, which keeps the bit from now on.
            SetWindowLongPtrW(w, GWL_EXSTYLE, with_no_activate(ex) as LongPtr);
        }
        true
    }

    fn yes_no(b: bool) -> &'static str {
        if b { "yes" } else { "no" }
    }

    fn line(text: &str) {
        if LOG.load(Ordering::Relaxed) {
            crate::log::line(text);
        }
    }

    /// Hands the foreground to `back`, if it is still a visible window.
    fn give_back(back: isize) -> bool {
        // SAFETY: plain calls with a window handle; a stale handle makes
        // them fail, which is harmless.
        unsafe {
            IsWindowVisible(hwnd(back)).as_bool() && SetForegroundWindow(hwnd(back)).as_bool()
        }
    }

    extern "system" fn wnd_proc(w: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
            WM_STYLECHANGING if wparam.0 as i32 == GWL_EXSTYLE.0 => {
                // SAFETY: for WM_STYLECHANGING, lParam points to the
                // STYLESTRUCT the system passes, valid for this call.
                if let Some(s) = unsafe { (lparam.0 as *mut STYLESTRUCT).as_mut() } {
                    s.styleNew = with_no_activate(s.styleNew);
                }
            }
            WM_ACTIVATE => {
                let ours = w.0 as isize;
                let activated = (wparam.0 & 0xffff) as u32 != WA_INACTIVE;
                if let Some(back) = give_back_target(
                    activated,
                    lparam.0,
                    LAST_FOREGROUND.load(Ordering::Relaxed),
                    ours,
                ) {
                    let n = ACTIVATIONS.fetch_add(1, Ordering::Relaxed);
                    GIVE_BACK_TO.store(back, Ordering::Relaxed);
                    // SAFETY: posting a message to this thread's own window.
                    let posted = unsafe { PostMessageW(Some(w), GIVE_BACK, WPARAM(0), LPARAM(0)) };
                    line(&format!(
                        "background: the window was activated; giving the foreground back: {}",
                        yes_no(posted.is_ok())
                    ));
                    if n == 0 && LOG.load(Ordering::Relaxed) {
                        // Where the first activation came from.
                        let trace = std::backtrace::Backtrace::force_capture().to_string();
                        for l in trace.lines().take(60) {
                            crate::log::line(&format!("background: activated from {}", l.trim()));
                        }
                    }
                }
            }
            GIVE_BACK => {
                let back = GIVE_BACK_TO.swap(0, Ordering::Relaxed);
                let ours = w.0 as isize;
                if back != 0 && foreground() == ours {
                    let given = give_back(back);
                    line(&format!(
                        "background: foreground given back: {}",
                        yes_no(given)
                    ));
                }
                return LRESULT(0);
            }
            _ => {}
        }
        let previous = PREVIOUS.load(Ordering::Acquire);
        // SAFETY: `previous` is the procedure this window had before the
        // guard (a valid WNDPROC, never 0 for a live window).
        unsafe {
            let previous: WNDPROC = std::mem::transmute::<isize, WNDPROC>(previous);
            CallWindowProcW(previous, w, msg, wparam, lparam)
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn activations() -> usize {
        0
    }

    pub(super) fn note_foreground() {}

    pub(super) fn guard(_hwnd: isize, _log: bool) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_style_keeps_other_bits() {
        assert_eq!(with_no_activate(0), WS_EX_NOACTIVATE);
        // WS_EX_TOOLWINDOW | WS_EX_APPWINDOW, as winit may set.
        assert_eq!(
            with_no_activate(0x80 | 0x4_0000),
            0x80 | 0x4_0000 | WS_EX_NOACTIVATE
        );
        assert_eq!(with_no_activate(WS_EX_NOACTIVATE), WS_EX_NOACTIVATE);
    }

    #[test]
    fn the_foreground_goes_back_to_the_window_that_had_it() {
        // Deactivation is not an activation: nothing to give back.
        assert_eq!(give_back_target(false, 7, 9, 5), None);
        // The window Windows names first, then the last one seen.
        assert_eq!(give_back_target(true, 7, 9, 5), Some(7));
        assert_eq!(give_back_target(true, 0, 9, 5), Some(9));
        // Never to the guarded window itself, and never to no window.
        assert_eq!(give_back_target(true, 5, 0, 5), None);
        assert_eq!(give_back_target(true, 0, 0, 5), None);
    }

    /// A real window, never shown (so it can never be the foreground):
    /// the guard gives it the style, keeps it when the style is rewritten
    /// as winit does, and refuses activation by the mouse.
    #[cfg(windows)]
    #[test]
    #[allow(unsafe_code)]
    fn a_hidden_window_keeps_the_style() {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GWL_EXSTYLE, GetWindowLongPtrW, IsWindowVisible,
            MA_NOACTIVATE, SendMessageW, SetWindowLongPtrW, WINDOW_EX_STYLE, WM_MOUSEACTIVATE,
            WS_POPUP,
        };
        use windows::core::w;
        // SAFETY: a window of this thread, created, used and destroyed here.
        unsafe {
            let w = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("textweaver background guard test"),
                WS_POPUP,
                -30_000,
                -30_000,
                10,
                10,
                None,
                None,
                None,
                None,
            )
            .expect("a hidden test window");
            assert!(!IsWindowVisible(w).as_bool());
            assert!(guard(w.0 as isize, false));
            let ex = GetWindowLongPtrW(w, GWL_EXSTYLE) as u32;
            assert_ne!(ex & WS_EX_NOACTIVATE, 0, "the guard sets the style");
            // winit rewrites the extended style from its flags.
            SetWindowLongPtrW(w, GWL_EXSTYLE, 0);
            let ex = GetWindowLongPtrW(w, GWL_EXSTYLE) as u32;
            assert_ne!(ex & WS_EX_NOACTIVATE, 0, "the guard keeps the style");
            let r = SendMessageW(w, WM_MOUSEACTIVATE, Some(WPARAM(0)), Some(LPARAM(0)));
            assert_eq!(r.0, MA_NOACTIVATE as isize);
            assert!(!IsWindowVisible(w).as_bool());
            assert_eq!(activations(), 0);
            let _ = DestroyWindow(w);
        }
    }
}
