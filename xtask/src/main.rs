//! `cargo xtask <task>`: maintenance tasks.
//!
//! - `affected [--base REF] [--args] [--feature PKG/FEAT] [--generated]`:
//!   the workspace crates changed since the merge base of REF (default
//!   `main`) and every crate that depends on them, for the fast check
//!   (see `affected.rs`).
//! - `appimage [--docker]`: the Linux AppImage and tarball, each holding
//!   the app and `tw` (see `appimage.rs`).
//! - `bench`: time the reading and authoring hot paths on generated Markdown
//!   corpora, the fixtures, and pathological inputs (release build; see
//!   `bench.rs`), and compare them with the committed baseline
//!   (`--baseline xtask/bench-baseline.json`, the two-way ratchet in
//!   `ratchet.rs`) or an earlier report (`--baseline FILE --max-ratio R`);
//!   `--update-baseline --reason TEXT` writes the baseline.
//! - `changelog [--check]`: fold the fragments in `changelog.d/` into
//!   `[Unreleased]` in `CHANGELOG.md` and remove them with `git rm`
//!   (see `changelog.rs`).
//! - `deps [--check]`: the dependency direction between workspace crates
//!   (see `deps.rs`).
//! - `hosts`: build the combined 32-bit engine host (Windows; the programs
//!   are their own native hosts) and install it with the dictionaries
//!   (`--dest DIR` for a package directory).
//! - `dist`: build the release package for this platform, one holding the
//!   app and `tw` (`--universal` on macOS); see `docs/dev/releasing.md`. It fails when the package grew
//!   more than 10 percent over the last release's without a note.
//! - `eci-host`: another name for `hosts`; `sapi-host`: the SAPI 5 hosts
//!   for development (see `sapi.rs`).
//! - `frames [--seconds N] [--json PATH]`: the GUI frame-time probe, the
//!   median and worst frame while the spoken word moves, with no window on
//!   screen (see `frames.rs`).
//! - `gui-dist`: another name for `dist`, kept for older notes; the app's
//!   part of the package is built in `gui_dist.rs`.
//! - `listen [--engine ID] [--text FILE] [--out DIR]`: write sample WAV
//!   files (and word-level subtitles) from every real speech engine here,
//!   for the listening checklist in `docs/releasing.md`; plays nothing
//!   (see `listen.rs`).
//! - `keyboard`: regenerate `docs/keyboard.md` from the keymap (Agent C).
//! - `notices [--check]`: regenerate `THIRD-PARTY-NOTICES.md` with
//!   `cargo about` (see `notices.rs`).
//! - `parity`: compare word and sentence segmentation with the star corpus
//!   in `fixtures/star-parity/` and write the report (Agent A).
//! - `regen [--check] [--require-all]`: rebuild every generated file in
//!   order (notices, settings reference, keyboard reference, site data,
//!   docs crate counts), or with `--check` report each on one line and fail
//!   if any is stale; `--require-all` (CI) fails a step whose tool is
//!   missing instead of skipping it (see `regen.rs`).
//! - `release X.Y.Z [--dry-run] [--no-checks]`: set the version, date the
//!   changelog, run the checks, commit, and tag (see `release.rs`);
//!   `release X.Y.Z --listened` records the listening check;
//!   `release X.Y.Z --sizes` records the published packages' sizes for
//!   the size budget that `dist`, `gui-dist`, and `appimage` check (see
//!   `sizes.rs`).
//! - `release-assets --dir DIR [--platform P]` or `--names FILE`: the
//!   exact set of release files, compared with a build folder or with a
//!   published release's asset names (see `release_assets.rs`).
//! - `sentinel record FILE`, `sentinel check FILE`: fail when the tests
//!   changed anything in the real data folders (see `sentinel.rs`).
//! - `soak [--minutes N]`: read the 10 MB corpus to the end with random
//!   navigation, edits, rate changes, and engine-host kills (see `soak.rs`).
//! - `tools [--check]`: the tool versions pinned in `tools.toml`, compared
//!   with the installed ones (see `tools.rs`).
//! - `startup [--baseline FILE --max-ratio R]`: time `tw --version`,
//!   `tw text`, `tw info`, and `tw backends` (see `bench.rs`).

mod affected;
mod appimage;
mod bench;
mod changelog;
mod deps;
mod dist;
mod docs_check;
mod eci;
mod frames;
mod fuzz_seed;
mod gui_dist;
mod keyboard;
mod listen;
mod notices;
mod parity;
mod pathological;
mod ratchet;
mod regen;
mod release;
mod release_assets;
mod sapi;
mod sentinel;
mod sizes;
mod soak;
mod tools;
#[cfg(test)]
mod workflows;

#[cfg(feature = "bench")]
#[global_allocator]
static ALLOC: bench::alloc::Counting = bench::alloc::Counting;

fn main() -> anyhow::Result<()> {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "affected" => affected::run(),
        "appimage" => appimage::run(),
        "bench" => bench::run(),
        #[cfg(feature = "bench")]
        "bench-run" => bench::run_inner(),
        "changelog" => changelog::run(),
        "deps" => deps::run(),
        "dist" => dist::run(),
        "docs" => docs_check::run(),
        "hosts" => eci::hosts(),
        "eci-host" => eci::run(),
        "frames" => frames::run(),
        "fuzz-seed" => fuzz_seed::run(),
        "gui-dist" => gui_dist::run(),
        "keyboard" => keyboard::run(),
        "listen" => listen::run(),
        "notices" => notices::run(),
        "sapi-host" => sapi::run(),
        "settings-doc" => docs_check::settings_doc(),
        "parity" => parity::run(),
        "regen" => regen::run(),
        "release" => release::run(),
        "release-assets" => release_assets::run(),
        "sentinel" => sentinel::run(),
        "soak" => soak::run(),
        #[cfg(feature = "bench")]
        "soak-run" => soak::run_inner(),
        "startup" => bench::startup(),
        "tools" => tools::run(),
        _ => {
            eprintln!(
                "usage: cargo xtask <affected|appimage|bench|changelog|deps|dist|docs|frames|fuzz-seed|gui-dist|hosts|eci-host|keyboard|listen|notices|parity|regen|release|sapi-host|settings-doc|soak|startup|tools>"
            );
            std::process::exit(2);
        }
    }
}
