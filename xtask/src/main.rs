//! `cargo xtask <task>`: maintenance tasks.
//!
//! - `keyboard`: regenerate `docs/keyboard.md` from the keymap (Agent C).
//! - `parity`: compare word and sentence segmentation with the Star corpus
//!   in `fixtures/star-parity/` and write the report (Agent A).

mod eci;
mod keyboard;
mod parity;
mod sapi;

fn main() -> anyhow::Result<()> {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "eci-host" => eci::run(),
        "keyboard" => keyboard::run(),
        "sapi-host" => sapi::run(),
        "parity" => parity::run(),
        _ => {
            eprintln!("usage: cargo xtask <eci-host|keyboard|parity|sapi-host>");
            std::process::exit(2);
        }
    }
}
