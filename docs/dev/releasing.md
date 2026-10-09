# Releasing textweaver

A release is a git tag `vX.Y.Z[-pre]` and a GitHub release with these files:

- the Windows package, `textweaver-VERSION-windows-x86_64.zip`;
- the macOS package, `textweaver-VERSION-macos-universal.tar.gz`;
- the Linux AppImages, `textweaver-VERSION-linux-x86_64.AppImage` and `textweaver-VERSION-linux-aarch64.AppImage`, each with its `.zsync` file for delta updates;
- the Linux tarballs, `textweaver-VERSION-linux-x86_64.tar.gz` and `textweaver-VERSION-linux-aarch64.tar.gz`, for systems without FUSE;
- the GUI's packages, named like the terminal's with `-gui` at the end: `textweaver-VERSION-windows-x86_64-gui.zip`, `textweaver-VERSION-macos-universal-gui.zip` (`textweaver.app`, Apple silicon and Intel), and for x86_64 and aarch64 `textweaver-VERSION-linux-ARCH-gui.AppImage` (with its `.zsync` file) and `textweaver-VERSION-linux-ARCH-gui.tar.gz`;
- `SHA256SUMS.txt`, covering every package.

The `Release` workflow (`.github/workflows/release.yml`) builds the packages, attests their build provenance, and writes the checksums. Releases before 1.0 are marked as pre-releases.

## Steps

1. **Listen, if you want to (optional).** A release does not wait for a listening check (the owner's decision, Wednesday, September 30, 2026). Tests that fake the engine cannot hear a silent one, so a listen on real hardware is still worth doing when speech has changed a lot. When you do one, go through [the listening checklist](#listening-checklist) below and record it:

   ```bash
   cargo xtask release 0.1.0-alpha.9 --listened
   ```

   This writes today's date and the version on the "Last listening check" line of this guide, and changes nothing else. The release prints a note when that line is old or for another version, and goes on.

2. **Prepare.** Before you start, run the checks CI runs, locally: `scripts/dev-check.sh` on Linux or macOS (`--docker` for the full Linux set), or `scripts\dev-check.ps1` on Windows. They include the link check and the site data check. Then, on `main`, with a clean tree and CI green, try the release without changing anything:

   ```bash
   cargo xtask release 0.1.0-alpha.9 --dry-run
   ```

   It prints the date it will use, the files it will change, and the checks it will run. Right after a release, `[Unreleased]` in `CHANGELOG.md` is empty; a dry run lists that as a problem, the same way it lists the other things a real release would stop for, instead of stopping itself. Then run it for real:

   ```bash
   cargo xtask release 0.1.0-alpha.9
   ```

   This:

   - stops unless the tree is clean and on `main` and the changelog is grouped by area (below), and notes an old or missing listening check without stopping;
   - sets `version` in `[workspace.package]` in the root `Cargo.toml` and runs `cargo update -w`;
   - turns `## [Unreleased]` in `CHANGELOG.md` into `## [0.1.0-alpha.9] - YYYY-MM-DD`, keeps an empty `[Unreleased]` above it, and adds the release link. The date comes from the machine's clock in local time, and the weekday is computed and printed so you can check it. It is never typed in;
   - updates the version examples in this guide, `docs/install.md`, the README, the crate map (`docs/site/architecture.html`), and the workflows, and lists every other line that still names the old version, so a file that should follow the release is seen in the dry run (lines that record history stay as they are);
   - runs the checks CI runs: fmt, clippy, the tests, `cargo xtask keyboard --check`, and `cargo xtask deps --check` (`--no-checks` skips them, for a rerun after a failure you have fixed);
   - commits "Release 0.1.0-alpha.9" and makes the annotated tag `v0.1.0-alpha.9`. It pushes nothing.

   Before a release, also run `cargo xtask notices` (it needs `cargo install --locked cargo-about`) and commit `THIRD-PARTY-NOTICES.md` if it changed. CI fails when it is out of date.

   **The changelog, grouped by area.** During development, contributors may add their `CHANGELOG.md` lines under a scratch heading of their own, to keep unrelated changes from colliding. Before a release, move those lines under area headings (reading and speech, documents, writing, the GUI, languages, packages) and add a short summary at the top of the section. `cargo xtask release` names any scratch heading it still finds in `[Unreleased]` and stops.

3. **Try the packages.** Start the `Release` workflow from the Actions tab on `main`, with the tag left empty. That is a dry run: it builds and checks every package, keeps them as workflow artifacts, and creates no release, tag, or attestation. It also works on a pushed branch, to try a change to the packaging before it merges:

   ```bash
   gh workflow run release.yml --ref BRANCH
   ```

   Download the artifacts from the run's page (or `gh run download RUN_ID`), and start each package once: the GUI with a screen reader on Windows, and the terminal reader on each system you use. This is optional and never holds a release; it is most useful when the packages change.

4. **Push.** Push the commit, then the tag:

   ```bash
   git push origin main
   ```

   ```bash
   git push origin v0.1.0-alpha.9
   ```

   Pushing the tag starts the `Release` workflow:

   - **Create the release.** Checks that the version in `Cargo.toml` matches the tag, then creates the GitHub release as a pre-release, with notes taken from the matching `CHANGELOG.md` section.
   - **Windows package** (on `windows-latest`) and **macOS package** (on `macos-14`), in parallel. Each runs `cargo xtask dist`, checks the package (the binaries run, and the notices and license files are inside), attests its build provenance, and uploads it to the release.
   - **Linux AppImage and tarball, x86_64 and aarch64** (on `ubuntu-latest` and `ubuntu-22.04-arm`, in parallel with the others). Each runs `cargo xtask appimage` in the `docker/appimage` image (Ubuntu 22.04), then `docker/appimage/test-distros.sh`, which runs both packages on Debian stable and Fedora, and on Arch for x86_64 (Arch has no official arm64 image): `tw --version`, `tw backends` without and with espeak-ng, `tw text`, `--install` and `--uninstall`, and `install-linux.sh --release` with each package. Then it attests and uploads the AppImage, its `.zsync` file, and the tarball.
   - **The GUI**, in the same three jobs, after the terminal package: `cargo xtask gui-dist` (`--universal` on macOS; in the `docker/appimage` image on Linux), then a check of the files in the package, `textweaver-gui --version`, and `--screenshot`, which draws the window on the CPU with no display. On macOS it also reads a document silently with the paced backend in a background window, then closes. On Windows the checks start the program with `Start-Process -Wait`, since it is a GUI-subsystem program. The GUI's packages are attested and uploaded with the terminal's. The GUI is not yet run on other Linux distributions.
   - **SHA256SUMS.txt.** Once all the packages are uploaded, one job writes the checksums of every package on the release and attests the checksum file. The package jobs never write checksums, so they cannot race.

5. **Check.** Read the release page. It should have every package and `SHA256SUMS.txt`, with the pre-release flag set. Anyone can check where a package was built:

   ```bash
   gh attestation verify textweaver-0.1.0-alpha.9-windows-x86_64.zip --repo leavesofgrass/textweaver
   ```

6. **Record the package sizes.** Once every package is on the release:

   ```bash
   cargo xtask release 0.1.0-alpha.9 --sizes
   ```

   It reads each package's size from the release (`gh release view`, read only), rewrites `xtask/package-sizes.toml` with them and clears its notes, adds a "Package sizes" list to the version's section of `CHANGELOG.md`, and writes that section to `target/release-notes-0.1.0-alpha.9.md`. Commit the two files. To show the sizes on the release page too, run the `gh release edit` command it prints. `--sizes-from DIR` reads the package files in a folder instead, such as a dry run's downloaded artifacts. See [Package sizes](#package-sizes).

## Listening checklist

Optional. When you want to listen, do this on the machine you use every day, with Eloquence, SAPI 5, and Piper (and DECtalk if it is installed). Then record it with `cargo xtask release VERSION --listened`, which rewrites this line from the machine's clock:

**Last listening check:** 2026-09-29 (Tuesday, September 29, 2026), for 0.1.0-alpha.9.

1. **Write the samples.**

   ```bash
   cargo xtask listen
   ```

   It writes `target/listen/<engine>.wav`, `<engine>-fast.wav` (400 words per minute), and `<engine>-low.wav` (4 semitones down) for every engine on this machine, each with word-level subtitles (`.srt`) beside it. An engine that is not installed is reported and skipped. It plays nothing. `--engine piper` limits it to one engine.

2. **Listen to each file.** For each engine:
   - The heading, "Chapter 3: The Library", is read, and "Chapter 3" is not "Chapter three colon".
   - "9:00 a.m." is a time, "$1.50" is money, and "Dr. Okafor" is "Doctor Okafor".
   - The question rises at the end.
   - The fast file is still clear; the low file is lower, not slower.
   - No clicks, no cut-off first or last word, and no long silence between sentences.

3. **Check the highlight timing.** Open a `.srt` file in a player that shows subtitles (or read the times): each word's time should match when you hear it, within about a tenth of a second.

4. **In the reader.** Run `textweaver` on a document and, with each engine:
   - Read, pause, resume, and stop.
   - Move by sentence and heading while reading.
   - Change the rate and pitch; then choose another voice (Alt+V) and back. The first voice comes back with its own rate and pitch.
   - In the voice manager (Alt+V), move the language and engine filters with Enter on their rows.
   - In the voice manager, press Alt+End on a voice of this engine and on one of another engine: each says its name and a sample in its own voice, and the voice in use stays as it was.
   - In the GUI's voice manager (Ctrl+Shift+V), Tab through the Language and Engine buttons, the list, and Use voice, Preview, Favorite, Remove, and Close: each is read with its name, role, and key. Press Engine and check the list and the button's name change.

5. **Piper.** With a Piper voice installed:
   - `tw backends` lists `piper` as available and says it supports word highlighting.
   - The first word of a long paragraph starts within about a third of a second of pressing Read.
   - In the voice manager, "Fetch the Piper voice list" asks before downloading, and a voice to download says its size and its license before asking. Say no once, then yes once, and check the voice appears in `<data>/piper/voices/`.
   - Remove the downloaded voice with Delete; it asks first.

6. **Dictation.** With the Whisper model in `<data>/whisper/rten/base.en` ([dictation guide](../dictation.md#whisper-inside-textweaver)), run `tw dictate --timings`, say a sentence, and press Enter. Check the text, and write down the time from Enter to the text.

7. **Braille: pending.** The first session with a Braille display (a 40-cell Mantis Q40 through NVDA and JAWS) has not been held yet. Its items join this list after it. Until then, go through [the checklist for the Mantis Q40](../screen-readers.md#checklist-for-the-mantis-q40) in the screen reader guide, and write down what the display showed.

Write down what you heard in the release notes' testing section, including anything odd.

## What the packages hold

`cargo xtask dist` builds with the `dist` profile (the release profile with fat LTO, and symbols stripped) and writes the package to `target/dist/`. It builds `textweaver` and `tw` in two cargo runs, one each: built together, cargo would unify their features and give the reader `tw`'s, such as `textweaver-formats`' `url` (opening a web address) and `textweaver-ocr`'s `download`, which the reader is meant to leave out. The reader still links an HTTP client of its own, for citation lookups (`textweaver-cite`). The C runtime is linked statically on Windows, so the package does not need the Visual C++ redistributable. Each package holds:

- `textweaver` and `tw`;
- on Windows, the engine hosts for Eloquence, SAPI5, and DECtalk, each for x64 and x86, and the IBMTTS community dictionaries;
- on Linux, the engine hosts for Eloquence (Voxin) and DECtalk, the IBMTTS community dictionaries, and the menu entry and icon under `share/`. `textweaver` and `tw` are built with Omnivox, speech-dispatcher, and espeak-ng; espeak-ng is loaded when the program starts, if it is installed, so the same binaries work without it;
- `QUICKSTART.md`, `README.md`, `LICENSE`, `NOTICE` (the copyright notice), `CHANGELOG.md`, and `INSTALL.md` at the top;
- in `docs/`, the [documentation index](../README.md), every user guide it lists under "For users", and the offline interactive pages in `docs/site/`, each at its path in the repository;
- the platform's helper scripts (doctor, speech check, update) and their README;
- `THIRD-PARTY-NOTICES.md`, and under `licenses/`: each bundled font's `OFL.txt`, SCOWL's `Copyright`, and the IBMTTS dictionaries' license. `cargo xtask dist` fails if any of these is missing.

**A missing guide warns, and the package still builds.** The documentation is staged by one helper, `stage_user_docs` in `xtask/src/dist.rs`, which `cargo xtask dist`, `appimage`, and `gui-dist` all call, so the packages carry the same `docs/` folder. When a guide the index lists, the index itself, or `docs/site/` is missing, the build leaves it out, prints one line per missing file, such as "Warning: the package lacks the guide docs/reading.md.", and goes on. When `GITHUB_STEP_SUMMARY` is set, as in every GitHub Actions step, the same lines are added to the job summary, so the release run shows them. Read the summary before you publish: a warning there means a package went out without a guide. With nothing missing, nothing is printed.

## The GUI packages

`cargo xtask gui-dist` builds the GUI (`textweaver-xilem`, installed as `textweaver-gui`) with the `dist` profile, in the same build folder as `cargo xtask dist`, with the static C runtime on Windows, and with the speech engines `cargo xtask dist` builds into the terminal programs for the platform (espeak-ng, speech-dispatcher and Omnivox on Linux; Omnivox elsewhere) for each one the GUI crate declares as a feature. It names any engine it leaves out because the crate has no such feature. It stages the program with the same engine hosts, IBMTTS dictionaries, define-word dictionary, `NOTICE`, notices, and license files as the terminal package (the same check fails if one is missing), plus Xilem's license, the quick start and `GUI.md` at the top, and the same complete `docs/` folder as the terminal package (a missing guide warns, as there), in `target/dist/textweaver-VERSION-PLATFORM-gui/`, and then:

- on Windows, zips it;
- on macOS, puts the program in `textweaver.app` (signed ad hoc) and zips the folder with `ditto`. It is built for the Mac's own architecture, or with `--universal` for Apple silicon and Intel joined with `lipo`, as the release does;
- on Linux, writes a tarball and, when `appimagetool` and the pinned runtime are found (as for `cargo xtask appimage`), an AppImage with the folder under `usr/lib/textweaver-gui/`, its own update information, and a `.zsync` file.

**The screenshot harness stays in the GUI package for now.** The GUI's default `screenshot` feature (`--screenshot` and `--review-screenshots`, drawn with Vello's CPU renderer, `image`, and `oxipng`) is the only way the release workflow can check that the packaged program draws a window: its runners have no GPU Vello can use, and the Linux check runs with no display. So the GUI checks on all three systems run `textweaver-gui --screenshot`. `cargo xtask gui-dist --no-screenshot` builds the package without it, with every other default feature; it can become the default once the release checks no longer need the harness. Review screenshots come from a developer build either way.

The names end in `-gui` so that no pattern for the terminal packages matches them. That matters most for the update information inside the terminal AppImages already released (`textweaver-*-linux-ARCH.AppImage.zsync`): a GUI name matching it would be offered as an update to the terminal reader. A test in `xtask/src/gui_dist.rs` checks every such pattern against every GUI name.

## The Linux packages

`cargo xtask appimage` stages the Linux package as `cargo xtask dist` does, writes the tarball, and then wraps the same folder in an AppImage with `appimagetool`. The folder sits whole under `usr/lib/textweaver/` inside the AppImage, so the programs find the hosts and dictionaries beside them, as in the tarball. `scripts/linux/AppRun` is the entry point: it starts `textweaver`, or `tw` when started through a link named `tw` or with `--tw` first, and it offers `--install` and `--uninstall`. The AppImage carries `gh-releases-zsync` update information pointing at the newest release or pre-release.

Build on an old glibc, so the packages run on older distributions. The `docker/appimage` image is Ubuntu 22.04 (glibc 2.35), with Rust from rustup, the AppImage tools, and, for the GUI build, `libfontconfig1-dev` (`yeslogic-fontconfig-sys` needs its headers). `docker/appimage/fetch-tools.sh` downloads appimagetool 1.9.1 and the type 2 runtime 20251108, for x86_64 or aarch64, from their GitHub releases, and checks each against the SHA-256 digest GitHub publishes for it; a changed file stops the build. The image builds for the machine it runs on, so the aarch64 packages are built on an arm64 machine: the release workflow uses GitHub's `ubuntu-22.04-arm` runner. To build locally on any system with Docker:

```bash
cargo xtask appimage --docker
```

The packages land in `target/dist/`. To check them on Debian, Fedora, and Arch, as the release job does (it needs Docker and bash; `ARCH=aarch64` checks the aarch64 packages, on an arm64 machine, with `debian:stable fedora:latest` after the folder):

```bash
bash docker/appimage/test-distros.sh target/dist
```

The AppImage is not signed; its checksum is in `SHA256SUMS.txt`, and its build provenance is attested. It needs the system's ALSA library (`libasound.so.2`), which every desktop has, and warns when it is missing.

## Package sizes

Every package has a size budget. `xtask/package-sizes.toml` records each package's size in bytes at the last release, named without the version (`windows-x86_64.zip`, `linux-x86_64-gui.AppImage`). After writing a package, `cargo xtask dist`, `gui-dist`, and `appimage` print one line for it, meaning first:

- `Size within budget:` with its size and the change in percent from the last release;
- `Size over budget, with a note:` and the note;
- `Size over budget:` when it grew more than 10 percent with no note. The command then fails, after the package is written, so the release workflow stops before it uploads.

A package with no recorded size (a new platform, or a Mac build that is not `--universal`) is printed and passes.

When a package is meant to grow, say why under `[notes]` in the file, in the same change:

```toml
[notes]
"windows-x86_64-gui.zip" = "Opus encoding in process for Export audio"
```

A note named `"all"` covers every package. The release step (`cargo xtask release VERSION --sizes`, step 6 above) writes the new sizes and clears the notes, so each note covers one release. The sizes also go into the release notes, as a "Package sizes" list in the version's section of `CHANGELOG.md`. MB there, as in the release workflow's summaries, is 1,048,576 bytes.

## Building a package by hand (fallback)

If a package job fails, build that package locally and upload it. On Windows this needs the MSVC toolchain and the 32-bit target (`rustup target add i686-pc-windows-msvc`):

```bash
cargo xtask dist
```

On macOS:

```bash
cargo xtask dist --universal
```

Then upload it and refresh the checksums. `tools/release-upload.sh` does both (it needs `gh`, logged in):

```bash
tools/release-upload.sh v0.1.0-alpha.9 target/dist/textweaver-0.1.0-alpha.9-windows-x86_64.zip
```

If the release does not exist yet, the script creates it. A package uploaded this way has no provenance attestation.

To rerun the workflow for an existing tag, start `Release` from the Actions tab and enter the tag.

## Notes

- **macOS signing.** The macOS binaries are universal (built with `lipo`) and signed ad hoc (`codesign -s -`). They are not notarized. `docs/install.md` tells users how to get past Gatekeeper. Notarizing needs an Apple Developer ID. When there is one, add `codesign --options runtime` with that identity and `xcrun notarytool submit --wait` to the workflow.
- **No engines are bundled.** No speech engines are in the packages: no Eloquence, no DECtalk, no voices. The packages hold only textweaver's own programs and hosts, and the CC0 IBMTTS dictionaries.
- **Linux.** The AppImage and the tarball are built for x86_64 and aarch64. Other systems build from source with `scripts/install-linux.sh`.

## See also

- [Installing textweaver](../install.md): what users do with the packages.
- [CONTRIBUTING.md](../../CONTRIBUTING.md): the checks and the commit style.
- [CHANGELOG.md](../../CHANGELOG.md): the release notes come from here.
- [ADR-0012: The engine host](../adr/0012-engine-host.md): `cargo xtask hosts` and host versioning.
- [Documentation index](../README.md)
