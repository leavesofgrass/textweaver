//! The terminal a GUI program was started from (ADR-0033).
//!
//! On Windows `textweaver-xilem.exe` is a GUI-subsystem program, so no
//! console window opens with it. A GUI program started from a terminal is
//! not connected to that terminal, though, so `--help`, `--version`, and
//! startup errors would go nowhere. [`attach`] connects the program to the
//! terminal it was started from (`AttachConsole(ATTACH_PARENT_PROCESS)`)
//! when its output is not already redirected, before anything is printed;
//! [`detach`] lets go of it once the window runs, so Control C in that
//! terminal does not close the window; [`attach`] again reconnects it to
//! print an error at the end.
//!
//! With no terminal at all (started from Explorer or a shortcut), an error
//! is shown in a message box, which screen readers read like any dialog,
//! unless the run is `--background` (an automated check).
//!
//! Elsewhere these do nothing: a program started from a terminal on Linux
//! or macOS already writes to it.

/// Connects standard output and error to the terminal the program was
/// started from, when they are not redirected. Returns true when output
/// reaches a terminal or a file.
pub fn attach() -> bool {
    imp::attach()
}

/// Lets go of a terminal [`attach`] connected (standard output and error
/// go nowhere until the next [`attach`]).
pub fn detach() {
    imp::detach();
}

/// Reports a startup error: to the `--log-file` log, to the terminal the
/// program was started from, and, with neither and outside `--background`
/// runs, in a message box.
pub fn report_error(text: &str, background: bool) {
    let logged = crate::log::to_file_active();
    if logged {
        crate::log::line(&format!("error: {text}"));
    }
    if attach() {
        eprintln!("textweaver-xilem: {text}");
    } else if !background && !logged {
        imp::message_box("textweaver", text);
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    //! Three console calls and one message box. Each is a plain Win32 call
    //! with no pointers kept past the call.

    use std::sync::atomic::{AtomicBool, Ordering};

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole, GetStdHandle, STD_ERROR_HANDLE,
        STD_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
    };
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    use windows::core::HSTRING;

    /// True while this program holds a console it attached to.
    static ATTACHED: AtomicBool = AtomicBool::new(false);

    fn handle(which: STD_HANDLE) -> Option<HANDLE> {
        // SAFETY: GetStdHandle only reads the process's handle table.
        let h = unsafe { GetStdHandle(which) }.ok()?;
        (!h.is_invalid() && !h.0.is_null()).then_some(h)
    }

    pub(super) fn attach() -> bool {
        if ATTACHED.load(Ordering::Acquire) {
            return true;
        }
        // Redirected to a file or a pipe (the UI Automation report, a
        // script): keep it, and attach nothing.
        if handle(STD_ERROR_HANDLE).is_some() || handle(STD_OUTPUT_HANDLE).is_some() {
            return true;
        }
        // SAFETY: AttachConsole takes a process id constant and sets this
        // process's standard handles to the parent's console when it has
        // one; it fails harmlessly when there is none.
        if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok() {
            ATTACHED.store(true, Ordering::Release);
            true
        } else {
            false
        }
    }

    pub(super) fn detach() {
        if !ATTACHED.swap(false, Ordering::AcqRel) {
            return;
        }
        // SAFETY: FreeConsole detaches from the console this process
        // attached to above; SetStdHandle clears the handles that pointed
        // at it, so the next attach sets them again.
        unsafe {
            let _ = FreeConsole();
            for which in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
                let _ = SetStdHandle(which, HANDLE(std::ptr::null_mut()));
            }
        }
    }

    pub(super) fn message_box(title: &str, text: &str) {
        let text = HSTRING::from(text);
        let title = HSTRING::from(title);
        // SAFETY: both strings live until the call returns; no owner
        // window, so the box is a top-level dialog.
        unsafe {
            MessageBoxW(None, &text, &title, MB_OK | MB_ICONERROR);
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn attach() -> bool {
        true
    }

    pub(super) fn detach() {}

    pub(super) fn message_box(_title: &str, _text: &str) {}
}
