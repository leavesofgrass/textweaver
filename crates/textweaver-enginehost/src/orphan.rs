//! Engine hosts must not outlive textweaver.
//!
//! A host exits by itself when its input closes (see
//! [`crate::serve::AtEnd::Exit`]), but that needs a host that still reads
//! its input. The operating system guarantees the rest:
//!
//! - **Windows:** every host joins one Job Object that textweaver creates
//!   with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Only textweaver holds the
//!   job's handle (it is not inheritable), so when textweaver exits, or is
//!   killed, the system closes the handle and ends every host in the job,
//!   and anything a host started.
//! - **Linux:** each host asks for `SIGKILL` when its parent dies
//!   (`prctl(PR_SET_PDEATHSIG)`), set between `fork` and `exec`. The signal
//!   follows the *thread* that started the host, so a host must be started
//!   on the thread that keeps it (the speech thread does).
//! - **Elsewhere** (macOS): only the end-of-input exit applies.

#![allow(unsafe_code)]

use std::process::{Child, Command};

/// Prepares `cmd` so the child dies with this process (Linux).
pub(crate) fn configure(cmd: &mut Command) {
    #[cfg(target_os = "linux")]
    linux::die_with_parent(cmd);
    #[cfg(not(target_os = "linux"))]
    let _ = cmd;
}

/// Ties a started child to this process's lifetime (Windows). A failure is
/// logged: the host still works, it just might outlive a crash.
pub(crate) fn adopt(child: &Child) {
    #[cfg(windows)]
    if let Err(e) = win::assign(child) {
        log::warn!(
            "engine host {} is not in the kill-on-close job: {e}",
            child.id()
        );
    }
    #[cfg(not(windows))]
    let _ = child;
}

#[cfg(target_os = "linux")]
mod linux {
    use std::ffi::{c_int, c_ulong};
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    const PR_SET_PDEATHSIG: c_int = 1;
    const SIGKILL: c_ulong = 9;

    unsafe extern "C" {
        // glibc and musl: `int prctl(int option, ...)`.
        fn prctl(option: c_int, ...) -> c_int;
        fn getppid() -> c_int;
    }

    pub(super) fn die_with_parent(cmd: &mut Command) {
        let parent = std::process::id();
        // SAFETY: `pre_exec` runs the closure in the child after `fork`,
        // before `exec`, where only async-signal-safe calls are allowed.
        // It makes two system calls (`prctl`, `getppid`), both
        // async-signal-safe, with plain integer arguments, allocates
        // nothing, and touches no lock or shared state; its errors are
        // built from `errno` without allocating.
        unsafe {
            cmd.pre_exec(move || {
                if prctl(PR_SET_PDEATHSIG, SIGKILL) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                // The parent died between `fork` and `prctl`: the signal
                // will never come, so do not start the host at all.
                if u32::try_from(getppid()) != Ok(parent) {
                    return Err(std::io::Error::from_raw_os_error(3)); // ESRCH
                }
                Ok(())
            });
        }
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use std::sync::OnceLock;

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };
    use windows::core::PCWSTR;

    /// The job every host joins, created on first use and never closed:
    /// the system closes it when this process ends, which kills the hosts.
    /// Kept as the handle's address, since `HANDLE` is not `Sync`.
    static JOB: OnceLock<Result<usize, String>> = OnceLock::new();

    fn create_job() -> Result<usize, String> {
        // SAFETY: no security attributes (a handle that is not inherited)
        // and no name; the returned handle is owned by `JOB` for the life
        // of the process.
        let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|e| e.to_string())?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let size = u32::try_from(std::mem::size_of_val(&info)).unwrap_or(u32::MAX);
        // SAFETY: `job` is the valid handle just created; the pointer and
        // size describe `info`, a live JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        // which is what this information class reads.
        unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&info).cast::<c_void>(),
                size,
            )
        }
        .map_err(|e| e.to_string())?;
        Ok(job.0 as usize)
    }

    pub(super) fn assign(child: &Child) -> Result<(), String> {
        let job = JOB.get_or_init(create_job).clone()?;
        let job = HANDLE(job as *mut c_void);
        let process = HANDLE(child.as_raw_handle());
        // SAFETY: both handles are valid: the job lives for the whole
        // process, and `child` owns its process handle for this call.
        unsafe { AssignProcessToJobObject(job, process) }.map_err(|e| e.to_string())
    }
}
