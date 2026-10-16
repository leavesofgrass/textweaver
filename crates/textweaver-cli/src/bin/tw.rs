//! `tw`: textweaver's terminal program. The terminal reader and every
//! command live in the `textweaver_cli` library; this binary only runs it.
//!
//! `tw --engine-host <engine>` is host mode, checked first: textweaver
//! starts a copy of `tw` that way to run a speech engine in a process of
//! its own (`textweaver_host`).

fn main() -> std::process::ExitCode {
    if let Some(status) = textweaver_host::host_mode() {
        return status;
    }
    textweaver_cli::main()
}
