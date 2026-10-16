//! `textweaver-dectalk-host`: the dectalk host as a separate program, for development
//! and this crate's tests only; the packages do not ship it. textweaver's
//! programs run the host as a copy of themselves (`--engine-host dectalk`),
//! and the 32-bit one is `textweaver-host-x86.exe dectalk`. See
//! `textweaver_dectalk::host::main` for the arguments.

fn main() -> std::process::ExitCode {
    textweaver_dectalk::host::main(std::env::args_os().skip(1).collect())
}
