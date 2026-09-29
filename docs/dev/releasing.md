# Releasing textweaver

A release is a git tag `vX.Y.Z[-pre]` and a GitHub release with these files:

- the Windows package, `textweaver-VERSION-windows-x86_64.zip`;
- the macOS package, `textweaver-VERSION-macos-universal.tar.gz`;
- the Linux AppImages, `textweaver-VERSION-linux-x86_64.AppImage` and `textweaver-VERSION-linux-aarch64.AppImage`, each with its `.zsync` file for delta updates;
- the Linux tarballs, `textweaver-VERSION-linux-x86_64.tar.gz` and `textweaver-VERSION-linux-aarch64.tar.gz`, for systems without FUSE;
- the GUI's packages, named like the terminal's with `-gui` at the end: `textweaver-VERSION-windows-x86_64-gui.zip`, `textweaver-VERSION-macos-aarch64-gui.zip` (`textweaver.app`, Apple silicon), and for x86_64 and aarch64 `textweaver-VERSION-linux-ARCH-gui.AppImage` (with its `.zsync` file) and `textweaver-VERSION-linux-ARCH-gui.tar.gz`;
- `SHA256SUMS.txt`, covering every package.

The `Release` workflow (`.github/workflows/release.yml`) builds the packages, attests their build provenance, and writes the checksums. Releases before 1.0 are marked as pre-releases.

## Steps

1. **Listen.** A person listens on real hardware before every release. Tests that fake the engine cannot hear a silent one, and tests never play audio. Go through [the listening checklist](#listening-checklist) below, then record it with the machine's date:

   ```bash
   cargo xtask release 0.1.0-alpha.4 --listened
   ```

   This writes today's date and the version on the "Last listening check" line of this guide, and changes nothing else. Commit it. The release in the next step stops unless that line names the version being released and is at most 14 days old.

2. **Prepare.** Before you start, run the checks CI runs, locally: `scripts/dev-check.sh` on Linux or macOS (`--docker` for the full Linux set), or `scripts\dev-check.ps1` on Windows. They include the link check and the site data check. Then, on `main`, with a clean tree and CI green, try the release without changing anything:

   ```bash
   cargo xtask release 0.1.0-alpha.4 --dry-run
   ```

   It prints the date it will use, the files it will change, and the checks it will run. Right after a release, `[Unreleased]` in `CHANGELOG.md` is empty; a dry run lists that as a problem, the same way it lists the other things a real release would stop for, instead of stopping itself. Then run it for real:

   ```bash
   cargo xtask release 0.1.0-alpha.4
   ```

   This:

   - stops unless the tree is clean and on `main`, the listening check is recorded for this version, and the changelog is grouped by area (below);
   - sets `version` in `[workspace.package]` in the root `Cargo.toml` and runs `cargo update -w`;
   - turns `## [Unreleased]` in `CHANGELOG.md` into `## [0.1.0-alpha.4] - YYYY-MM-DD`, keeps an empty `[Unreleased]` above it, and adds the release link. The date comes from the machine's clock in local time, and the weekday is computed and printed so you can check it. It is never typed in;
   - updates the version examples in this guide, `docs/install.md`, and the workflows;
   - runs the checks CI runs: fmt, clippy, the tests, `cargo xtask keyboard --check`, and `cargo xtask deps --check` (`--no-checks` skips them, for a rerun after a failure you have fixed);
   - commits "Release 0.1.0-alpha.4" and makes the annotated tag `v0.1.0-alpha.4`. It pushes nothing.

   Before a release, also run `cargo xtask notices` (it needs `cargo install --locked cargo-about`) and commit `THIRD-PARTY-NOTICES.md` if it changed. CI fails when it is out of date.

   **The changelog, grouped by area.** While a wave runs, each agent writes its `CHANGELOG.md` lines under a heading with its own name, such as `### W4c2: documents`. Before a release, move those lines under area headings (reading and speech, documents, writing, the GUI, languages, packages) and add a short summary at the top of the section. `cargo xtask release` names any agent heading still in `[Unreleased]` and stops.

3. **Try the packages.** Start the `Release` workflow from the Actions tab on `main`, with the tag left empty. That is a dry run: it builds and checks every package, keeps them as workflow artifacts, and creates no release, tag, or attestation. It also works on a pushed branch, to try a change to the packaging before it merges:

   ```bash
   gh workflow run release.yml --ref BRANCH
   ```

   Download the artifacts from the run's page (or `gh run download RUN_ID`), and start each package once: the GUI with a screen reader on Windows, and the terminal reader on each system you use. Do this before every release that changes the packages, and at least once for a release with new packages.

4. **Push.** Push the commit, then the tag:

   ```bash
   git push origin main
   ```

   ```bash
   git push origin v0.1.0-alpha.4
   ```

   Pushing the tag starts the `Release` workflow:

   - **Create the release.** Checks that the version in `Cargo.toml` matches the tag, then creates the GitHub release as a pre-release, with notes taken from the matching `CHANGELOG.md` section.
   - **Windows package** (on `windows-latest`) and **macOS package** (on `macos-14`), in parallel. Each runs `cargo xtask dist`, checks the package (the binaries run, and the notices and licence files are inside), attests its build provenance, and uploads it to the release.
   - **Linux AppImage and tarball, x86_64 and aarch64** (on `ubuntu-latest` and `ubuntu-22.04-arm`, in parallel with the others). Each runs `cargo xtask appimage` in the `docker/appimage` image (Ubuntu 22.04), then `docker/appimage/test-distros.sh`, which runs both packages on Debian stable and Fedora, and on Arch for x86_64 (Arch has no official arm64 image): `tw --version`, `tw backends` without and with espeak-ng, `tw text`, `--install` and `--uninstall`, and `install-linux.sh --release` with each package. Then it attests and uploads the AppImage, its `.zsync` file, and the tarball.
   - **The GUI**, in the same three jobs, after the terminal package: `cargo xtask gui-dist` (in the `docker/appimage` image on Linux), then a check of the files in the package, `textweaver-gui --version`, and `--screenshot`, which draws the window on the CPU with no display. On macOS it also reads a document silently with the paced backend in a background window, then closes. On Windows the checks start the program with `Start-Process -Wait`, since it is a GUI-subsystem program. The GUI's packages are attested and uploaded with the terminal's. The GUI is not yet run on other Linux distributions.
   - **SHA256SUMS.txt.** Once all the packages are uploaded, one job writes the checksums of every package on the release and attests the checksum file. The package jobs never write checksums, so they cannot race.

5. **Check.** Read the release page. It should have every package and `SHA256SUMS.txt`, with the pre-release flag set. Anyone can check where a package was built:

   ```bash
   gh attestation verify textweaver-0.1.0-alpha.4-windows-x86_64.zip --repo leavesofgrass/textweaver
   ```

## Listening checklist

Do this before each release, on the machine you use every day, with Eloquence, SAPI 5, and Piper (and DECtalk if it is installed). Then record it with `cargo xtask release VERSION --listened`, which rewrites this line from the machine's clock:

**Last listening check:** 2026-09-29 (Tuesday, September 29, 2026), for 0.1.0-alpha.5.

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

5. **Piper.** With a Piper voice installed:
   - `tw backends` lists `piper` as available and says it supports word highlighting.
   - The first word of a long paragraph starts within about a third of a second of pressing Read.
   - In the voice manager, "Fetch the Piper voice list" asks before downloading, and a voice to download says its size and its licence before asking. Say no once, then yes once, and check the voice appears in `<data>/piper/voices/`.
   - Remove the downloaded voice with Delete; it asks first.

6. **Dictation.** With the Whisper model in `<data>/whisper/rten/base.en` ([dictation guide](../dictation.md#whisper-inside-textweaver)), run `tw dictate --timings`, say a sentence, and press Enter. Check the text, and write down the time from Enter to the text.

Write down what you heard in the release notes' testing section, including anything odd.

## What the packages hold

`cargo xtask dist` builds with the `dist` profile (the release profile with fat LTO, and symbols stripped) and writes the package to `target/dist/`. The C runtime is linked statically on Windows, so the package does not need the Visual C++ redistributable. Each package holds:

- `textweaver` and `tw`;
- on Windows, the engine hosts for Eloquence, SAPI5, and DECtalk, each for x64 and x86, and the IBMTTS community dictionaries;
- on Linux, the engine hosts for Eloquence (Voxin) and DECtalk, the IBMTTS community dictionaries, and the menu entry and icon under `share/`. `textweaver` and `tw` are built with Omnivox, speech-dispatcher, and espeak-ng; espeak-ng is loaded when the program starts, if it is installed, so the same binaries work without it;
- `QUICKSTART.md`, `README.md`, `LICENSE`, `CHANGELOG.md`, and `INSTALL.md` at the top;
- in `docs/`, every user guide listed under "For users" in the [documentation index](../README.md), and the offline interactive pages in `docs/site/`;
- the platform's helper scripts (doctor, speech check, update) and their README;
- `THIRD-PARTY-NOTICES.md`, and under `licenses/`: each bundled font's `OFL.txt`, SCOWL's `Copyright`, and the IBMTTS dictionaries' licence. `cargo xtask dist` fails if any of these is missing.

## The GUI packages

`cargo xtask gui-dist` builds the GUI (`textweaver-xilem`, installed as `textweaver-gui`) with the `dist` profile, in the same build folder as `cargo xtask dist`, with the static C runtime on Windows. It stages the program with the same engine hosts, IBMTTS dictionaries, define-word dictionary, notices, and licence files as the terminal package (the same check fails if one is missing), plus Xilem's licence, the quick start, and `GUI.md`, in `target/dist/textweaver-VERSION-PLATFORM-gui/`, and then:

- on Windows, zips it;
- on macOS, puts the program in `textweaver.app` (signed ad hoc) and zips the folder with `ditto`. It is built for the Mac's own architecture, so the release's package is for Apple silicon;
- on Linux, writes a tarball and, when `appimagetool` and the pinned runtime are found (as for `cargo xtask appimage`), an AppImage with the folder under `usr/lib/textweaver-gui/`, its own update information, and a `.zsync` file.

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
tools/release-upload.sh v0.1.0-alpha.4 target/dist/textweaver-0.1.0-alpha.4-windows-x86_64.zip
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
