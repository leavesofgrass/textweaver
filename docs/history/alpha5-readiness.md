# Readiness for 0.1.0-alpha.5

Written on Monday, September 28, 2026, by Agent W5p, for the owner, who decides the release. It is filled in again at the end of Wave 5, after the other agents merge. Nothing here is released, tagged, or pushed.

## Verdict

Not ready yet. Four things stand between main and `cargo xtask release 0.1.0-alpha.5`:

1. The Release dry run with the GUI packages has not run: it needs this branch pushed (below).
2. The nightly checks have never passed; the fixes are on this branch, and seven clean nights come after it merges.
3. The listening check is recorded for 0.1.0-alpha.4, and sessions B1 (Braille) and 3 (the GUI) are still to come.
4. The alpha.5 notes wait for the Wave 5 agents to merge; `[Unreleased]` holds only W5p's lines.

## Packages

- **The GUI is in `release.yml`** on this branch: Windows zip, macOS `.app` zip (Apple silicon), Linux AppImage and tarball for x86_64 and aarch64. Each is built with `cargo xtask gui-dist`, checked, attested, uploaded, and in `SHA256SUMS.txt`. Names end in `-gui`, so no terminal pattern (or the update information in released AppImages) picks them up.
- **The GUI package now speaks** with the terminal package's engines: it carries the engine hosts and dictionaries, the lexicon, and every licence file, with the static C runtime on Windows. Before this, a GUI zip on Windows could speak only with a downloaded Piper voice.
- **Built so far:** the Windows zip and a Linux x86_64 AppImage were built by the GUI workflow on main (Monday, September 28, 2026, run 36479711304), before this branch's changes. Nothing has been built by `release.yml` with the GUI yet.
- **The aarch64 AppImage:** built and checked on Debian and Fedora in the 0.1.0-alpha.4 release run; that closes the Wave 4 follow-up. The aarch64 GUI AppImage is new and untested.
- **To do, needs the orchestrator:** push `wave5/p-readiness` and start the dry run, when no other run needs the runners: `gh workflow run release.yml --ref wave5/p-readiness`. Then download the artifacts and start the Windows GUI once with NVDA.

## Checklists

- **Listening check:** recorded Monday, September 28, 2026, for 0.1.0-alpha.4. A new one is needed for 0.1.0-alpha.5 (`cargo xtask release 0.1.0-alpha.5 --listened`).
- **Session B1, Braille on the Mantis Q40:** not yet. After it, W5p adds its five items to the listening checklist in `docs/dev/releasing.md`, so the release dates them.
- **Session 3, the GUI:** not yet. It gates the line "GUI supported on Windows" in the notes.
- **Check 2, documents and math:** not yet; it gates nothing in Wave 5.

## Checks

- **Pass:** `cargo xtask release 0.1.0-alpha.5 --dry-run` from a clean tree on this branch. It lists what would stop a real release (the branch, the listening check, the empty changelog section) and what it would change. Before the fix on this branch, the dry run itself stopped at the empty changelog section.
- **Pass:** `cargo xtask docs --check`, the link check, and the site data check, on this branch.
- **Pass on main, not run here:** `cargo xtask settings-doc --check` (it builds `textweaver-app`, which agents do not build on this machine); CI's docs job ran it on main at the W5t merge.
- **Pass:** actionlint with shellcheck on `release.yml`, `ci.yml`, and `nightly.yml`, and shellcheck on the Linux GUI check script.
- **Pass:** the xtask tests, clippy, and fmt.

## Nightly

Not clean. No nightly run has passed; there have been three (September 27 and 28, 2026). The fixes on this branch:

- **Fuzz, every target:** the prebuilt cargo-fuzz is a musl build and built every target for musl, where the sanitizer cannot link. The run now names the glibc target.
- **MSRV:** the dependencies need Rust 1.94 (rten 0.26), not the declared 1.92. `rust-version` is now 1.94.
- **Tests in release mode:** the cold build ran past 90 minutes; the limit is now 180.

Still failing, not W5p's code:

- **The weekly feature matrix:** `crates/textweaver-formats/src/pdf/mod.rs` line 136, the unused `blank` without the `ocr` feature. One `cfg_attr` line; W5r is splitting that feature.

## Other open items

- **`scripts.yml` fails on main:** PSScriptAnalyzer's empty-catch warning in `tools/build-hygiene.ps1`, line 82.
- **The GUI workflow's macOS tests fail on main:** `every_button_has_its_key_from_the_keymap` (run 36479711304). The release job does not run those tests; W5a4's area.
- **The GUI on Linux has no espeak-ng or speech-dispatcher:** `textweaver-xilem` has no engine features to pass through, so its Linux package speaks with Piper, Voxin, and DECtalk only. It needs `espeak`, `speechd`, and `omnivox` features on the GUI crate, then `gui-dist` turning them on as `dist` does.
- **No Intel Mac GUI:** `gui-dist` builds for the runner's architecture. A universal build, as `dist --universal` makes, is the next step.
- **The install scripts do not install the GUI yet;** `docs/install.md` says so.
- **GitHub:** no open issues; three dependency update pull requests from Dependabot (imaging_wgpu, resvg, sha2).

## See also

- [Releasing textweaver](../dev/releasing.md)
- [Installing textweaver](../install.md)
- [Tasks and agent briefs](tasks.md)
- [Wave 5, recalibrated](../research/wave5-recalibrated.md)
