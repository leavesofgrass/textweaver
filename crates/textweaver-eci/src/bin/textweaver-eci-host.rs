//! `textweaver-eci-host`: the eci host as a separate program, for development
//! and this crate's tests only; the packages do not ship it. textweaver's
//! programs run the host as a copy of themselves (`--engine-host eci`),
//! and the 32-bit one is `textweaver-host-x86.exe eci`. See
//! `textweaver_eci::host::main` for the arguments.

fn main() -> std::process::ExitCode {
    textweaver_eci::host::main(std::env::args_os().skip(1).collect())
}
