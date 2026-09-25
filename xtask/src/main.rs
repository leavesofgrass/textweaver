//! `cargo xtask <task>`: maintenance tasks.
//!
//! - `keyboard`: regenerate `docs/keyboard.md` from the keymap (Agent C).
//! - `parity`: compare word and sentence segmentation with the Star corpus
//!   in `fixtures/star-parity/` and write the report (Agent A).

mod keyboard;
mod parity;

fn main() -> anyhow::Result<()> {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "keyboard" => keyboard::run(),
        "parity" => parity::run(),
        _ => {
            eprintln!("usage: cargo xtask <keyboard|parity>");
            std::process::exit(2);
        }
    }
}
