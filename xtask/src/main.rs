//! `cargo xtask <task>`: maintenance tasks.
//!
//! - `bench`: time the reading and authoring hot paths on generated Markdown
//!   corpora and the fixtures (release build; see `bench.rs`).
//! - `hosts`: build every speech-engine host for this platform and install
//!   them with the dictionaries (`--dest DIR` for a package directory).
//! - `dist`: build a release package for this platform (`--universal` on
//!   macOS); see `docs/releasing.md`.
//! - `eci-host`, `sapi-host`: build one engine's hosts.
//! - `keyboard`: regenerate `docs/keyboard.md` from the keymap (Agent C).
//! - `parity`: compare word and sentence segmentation with the Star corpus
//!   in `fixtures/star-parity/` and write the report (Agent A).

mod bench;
mod dist;
mod eci;
mod keyboard;
mod parity;
mod sapi;

#[cfg(feature = "bench")]
#[global_allocator]
static ALLOC: bench::alloc::Counting = bench::alloc::Counting;

fn main() -> anyhow::Result<()> {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "bench" => bench::run(),
        #[cfg(feature = "bench")]
        "bench-run" => bench::run_inner(),
        "dist" => dist::run(),
        "hosts" => eci::hosts(),
        "eci-host" => eci::run(),
        "keyboard" => keyboard::run(),
        "sapi-host" => sapi::run(),
        "parity" => parity::run(),
        _ => {
            eprintln!("usage: cargo xtask <bench|dist|hosts|eci-host|keyboard|parity|sapi-host>");
            std::process::exit(2);
        }
    }
}
