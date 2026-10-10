//! `tw`: textweaver's terminal program. The terminal reader and every
//! command live in the `textweaver_cli` library; this binary only runs it.

fn main() -> std::process::ExitCode {
    textweaver_cli::main()
}
