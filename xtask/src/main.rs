//! `cargo xtask <task>`: maintenance tasks.
//!
//! - `appimage [--docker]`: the Linux AppImage and tarball (see
//!   `appimage.rs`).
//! - `bench`: time the reading and authoring hot paths on generated Markdown
//!   corpora and the fixtures (release build; see `bench.rs`), and compare
//!   them with a baseline (`--baseline FILE --max-ratio R`).
//! - `deps [--check]`: the dependency direction between workspace crates
//!   (see `deps.rs`).
//! - `hosts`: build every speech-engine host for this platform and install
//!   them with the dictionaries (`--dest DIR` for a package directory).
//! - `dist`: build a release package for this platform (`--universal` on
//!   macOS); see `docs/dev/releasing.md`.
//! - `eci-host`, `sapi-host`: build one engine's hosts.
//! - `gui-dist`: the Xilem GUI's own package: a zip on Windows, a `.app`
//!   on macOS, a tarball and an AppImage on Linux (see `gui_dist.rs`).
//! - `listen [--engine ID] [--text FILE] [--out DIR]`: write sample WAV
//!   files (and word-level subtitles) from every real speech engine here,
//!   for the listening checklist in `docs/releasing.md`; plays nothing
//!   (see `listen.rs`).
//! - `keyboard`: regenerate `docs/keyboard.md` from the keymap (Agent C).
//! - `notices [--check]`: regenerate `THIRD-PARTY-NOTICES.md` with
//!   `cargo about` (see `notices.rs`).
//! - `parity`: compare word and sentence segmentation with the Star corpus
//!   in `fixtures/star-parity/` and write the report (Agent A).
//! - `release X.Y.Z [--dry-run] [--no-checks]`: set the version, date the
//!   changelog, run the checks, commit, and tag (see `release.rs`);
//!   `release X.Y.Z --listened` records the listening check.
//! - `soak [--minutes N]`: read the 10 MB corpus to the end with random
//!   navigation, edits, rate changes, and engine-host kills (see `soak.rs`).
//! - `startup [--baseline FILE --max-ratio R]`: time `tw --version`,
//!   `tw text`, `tw info`, and `tw backends` (see `bench.rs`).

mod appimage;
mod bench;
mod deps;
mod dist;
mod docs_check;
mod eci;
mod fuzz_seed;
mod gui_dist;
mod keyboard;
mod listen;
mod notices;
mod parity;
mod release;
mod sapi;
mod soak;

#[cfg(feature = "bench")]
#[global_allocator]
static ALLOC: bench::alloc::Counting = bench::alloc::Counting;

fn main() -> anyhow::Result<()> {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "appimage" => appimage::run(),
        "bench" => bench::run(),
        #[cfg(feature = "bench")]
        "bench-run" => bench::run_inner(),
        "deps" => deps::run(),
        "dist" => dist::run(),
        "docs" => docs_check::run(),
        "hosts" => eci::hosts(),
        "eci-host" => eci::run(),
        "fuzz-seed" => fuzz_seed::run(),
        "gui-dist" => gui_dist::run(),
        "keyboard" => keyboard::run(),
        "listen" => listen::run(),
        "notices" => notices::run(),
        "sapi-host" => sapi::run(),
        "settings-doc" => docs_check::settings_doc(),
        "parity" => parity::run(),
        "release" => release::run(),
        "soak" => soak::run(),
        #[cfg(feature = "bench")]
        "soak-run" => soak::run_inner(),
        "startup" => bench::startup(),
        _ => {
            eprintln!(
                "usage: cargo xtask <appimage|bench|deps|dist|gui-dist|hosts|eci-host|keyboard|listen|notices|parity|release|sapi-host|soak|startup>"
            );
            std::process::exit(2);
        }
    }
}
