//! The terminal a GUI program was started from (ADR-0033).
//!
//! On Windows `textweaver.exe` is a GUI-subsystem program, so no
//! console window opens with it. A GUI program started from a terminal is
//! not connected to that terminal, though, so `--help`, `--version`, and
//! startup errors would go nowhere. [`attach`] connects the program to the
//! terminal it was started from (`AttachConsole(ATTACH_PARENT_PROCESS)`)
//! when its output is not already redirected, before anything is printed;
//! [`detach`] lets go of it once the window runs, so Control C in that
//! terminal does not close the window; [`attach`] again reconnects it to
//! print an error at the end.
//!
//! The terminal program inside it (`textweaver --tw`, which `tw.exe` runs)
//! needs the console itself, not only its output: [`attach_terminal`]
//! attaches to the parent's console whether or not the standard handles
//! are set, and keeps any that were (a launcher's console handles, or a
//! pipe), so the terminal reader's keys and output work as in a console
//! program.
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

/// For the terminal program: attaches to the console of the program that
/// started this one (`tw.exe`, or a shell), keeping standard handles that
/// are already set, such as a pipe. Without a console to attach to (a
/// pipe from a program with none), the handles stay as they were.
pub fn attach_terminal() {
    imp::attach_terminal();
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
        eprintln!("textweaver: {text}");
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

    pub(super) fn attach_terminal() {
        if ATTACHED.load(Ordering::Acquire) {
            return;
        }
        let saved = [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE].map(|w| (w, handle(w)));
        // SAFETY: as in `attach`. AttachConsole may point the standard
        // handles at the console; SetStdHandle puts back each one that was
        // set before, a handle this process already owns.
        unsafe {
            if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
                return;
            }
            ATTACHED.store(true, Ordering::Release);
            for (which, h) in saved {
                if let Some(h) = h {
                    let _ = SetStdHandle(which, h);
                }
            }
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

    pub(super) fn attach_terminal() {}

    pub(super) fn detach() {}

    pub(super) fn message_box(_title: &str, _text: &str) {}
}
