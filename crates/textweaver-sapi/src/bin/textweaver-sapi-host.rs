//! `textweaver-sapi-host`: runs SAPI5 voices in their own process
//! (ADR-0009). See `textweaver_sapi::host` for the command line and the
//! protocol.

fn main() -> std::process::ExitCode {
    textweaver_sapi::host::main()
}
