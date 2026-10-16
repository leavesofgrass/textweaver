//! `textweaver-host`: every speech engine's host in one program. `cargo
//! xtask hosts` builds it for `i686-pc-windows-msvc` and installs it as
//! `textweaver-host-x86.exe`, which runs 32-bit engine libraries:
//!
//! ```text
//! textweaver-host-x86.exe ENGINE [ARGUMENTS]
//! textweaver-host-x86.exe --engine-host ENGINE [ARGUMENTS]
//! ```
//!
//! ENGINE is `eci`, `espeak`, `dectalk` or `sapi`. textweaver starts it;
//! it is not meant to be run by hand. See `textweaver_host`.

fn main() -> std::process::ExitCode {
    textweaver_host::combined_main(std::env::args_os().skip(1).collect())
}
