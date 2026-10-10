# Releasing textweaver

A release is a git tag `vX.Y.Z[-pre]` and a GitHub release with these files. Since beta 1 there is one package per OS and CPU architecture, and each holds both programs, the app (`textweaver-gui`) and the terminal program (`tw`, with `textweaver` as a second name), with the complete documentation:

- the Windows package, `textweaver-VERSION-windows-x86_64.zip`;
- the macOS package, `textweaver-VERSION-macos-universal.zip` (`textweaver.app` beside `tw`, Apple silicon and Intel);
- the Linux AppImages, `textweaver-VERSION-linux-x86_64.AppImage` and `textweaver-VERSION-linux-aarch64.AppImage`, each with its `.zsync` file for delta updates, and a copy of that file named `textweaver-VERSION-linux-ARCH-gui.AppImage.zsync`, so the separate app AppImages of 0.1.0-alpha.9 and earlier are offered the single image;
- the Linux tarballs, `textweaver-VERSION-linux-x86_64.tar.gz` and `textweaver-VERSION-linux-aarch64.tar.gz`, for systems without FUSE;
- `SHA256SUMS.txt`, covering every package.

`cargo xtask release-assets` holds this list, and the release workflow checks each build folder and the finished release against it.

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

   Download the artifacts from the run's page (or `gh run download RUN_ID`), and start each package once: the app with a screen reader on Windows, and the terminal reader on each system you use. This is optional and never holds a release; it is most useful when the packages change.

4. **Push.** Push the commit, then the tag:

   ```bash
   git push origin main
   ```

   ```bash
   git push origin v0.1.0-alpha.9
   ```

   Pushing the tag starts the `Release` workflow:

   - **Create the release.** Checks that the version in `Cargo.toml` matches the tag, then creates the GitHub release as a pre-release, with notes taken from the matching `CHANGELOG.md` section.
   - **Windows package** (on `windows-latest`) and **macOS package** (on `macos-14`), in parallel. Each runs `cargo xtask dist` once, which builds both programs, then checks the package: the files inside (`tw`, `textweaver`, the app, every engine host including the eSpeak NG hosts on Windows, the notices and license files), `tw` with `tools/package-smoke.sh`, and the app with `textweaver-gui --version` and `--screenshot`, which draws the app's interface on the CPU with no display. On macOS it also reads a document silently with the paced backend in a background window, then closes. On Windows the checks start the app with `Start-Process -Wait`, since it is a GUI-subsystem program. Then it attests the package's build provenance and uploads it to the release.
   - **Linux AppImage and tarball, x86_64 and aarch64** (on `ubuntu-latest` and `ubuntu-22.04-arm`, in parallel with the others). Each runs `cargo xtask appimage` in the `docker/appimage` image (Ubuntu 22.04), then `docker/appimage/test-distros.sh`, which runs both packages on Debian stable and Fedora, and on Arch for x86_64 (Arch has no official arm64 image): `tw --version`, `tw backends` without and with espeak-ng, `tw text`, `--install` and `--uninstall`, and `install-linux.sh --release` with each package. It checks the app in the same packages (`textweaver-gui --version` and `--screenshot`). Then it attests and uploads the AppImage, its two `.zsync` files, and the tarball. The app is not yet run on other Linux distributions.
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

`cargo xtask dist` builds with the `dist` profile (the release profile with fat LTO, and symbols stripped) and writes the package to `target/dist/`. It builds the terminal program in one cargo run (`textweaver-cli`: `tw`, and on Windows the `textweaver` launcher) and the app in another (`textweaver-xilem`, installed as `textweaver-gui`), in the same build folder, so the two share their compiled dependencies and cargo never unifies their features. `cargo xtask gui-dist` is another name for the same task. The C runtime is linked statically on Windows, so the package does not need the Visual C++ redistributable. Each package holds:

- `tw`, the terminal program: the terminal reader with no command, and every command headless;
- `textweaver`, its second name: a link to `tw` on Linux and macOS, and on Windows `textweaver.exe`, a launcher of a few hundred kilobytes that runs `tw.exe` beside it with the same arguments (a copy would undo the size the single program saves);
- the app, `textweaver-gui` (on macOS inside `textweaver.app`, beside `tw`), with its guide `GUI.md` at the top and Xilem's license;
- on Windows, the engine hosts for Eloquence, SAPI5, DECtalk, and eSpeak NG, each for x64 and x86, and the IBMTTS community dictionaries;
- on Linux, the engine hosts for Eloquence (Voxin) and DECtalk, the IBMTTS community dictionaries, and the menu entries for the terminal reader and the app and the icon under `share/`. `tw` and the app are built with Omnivox, speech-dispatcher, and espeak-ng; espeak-ng is loaded when the program starts, if it is installed, so the same binaries work without it;
- `QUICKSTART.md`, `README.md`, `LICENSE`, `NOTICE` (the copyright notice), `CHANGELOG.md`, and `INSTALL.md` at the top;
- in `docs/`, the [documentation index](../README.md), every user guide it lists under "For users", and the offline interactive pages in `docs/site/`, each at its path in the repository;
- the platform's helper scripts (doctor, speech check, update) and their README;
- `THIRD-PARTY-NOTICES.md`, and under `licenses/`: each bundled font's `OFL.txt`, SCOWL's `Copyright`, and the IBMTTS dictionaries' license.

`cargo xtask dist` fails if a program, an engine host, or one of the notices and licenses is missing from the staged folder.

**A missing guide warns, and the package still builds.** The documentation is staged by one helper, `stage_user_docs` in `xtask/src/dist.rs`, which `cargo xtask dist` and `appimage` call. When a guide the index lists, the index itself, or `docs/site/` is missing, the build leaves it out, prints one line per missing file, such as "Warning: the package lacks the guide docs/reading.md.", and goes on. When `GITHUB_STEP_SUMMARY` is set, as in every GitHub Actions step, the same lines are added to the job summary, so the release run shows them. Read the summary before you publish: a warning there means a package went out without a guide. With nothing missing, nothing is printed.

## The app in the package

The app's part of the build is in `xtask/src/gui_dist.rs`. It builds `textweaver-xilem` with the speech engines `cargo xtask dist` builds into `tw` for the platform (espeak-ng, speech-dispatcher and Omnivox on Linux; Omnivox elsewhere), for each one the GUI crate declares as a feature, and names any engine it leaves out because the crate has no such feature. On macOS it puts the app in `textweaver.app` (signed ad hoc), built for the Mac's own architecture, or with `--universal` for Apple silicon and Intel joined with `lipo`, as the release does, and the package folder is zipped with `ditto`, which keeps the `textweaver` link.

**The screenshot harness stays in the package for now.** The app's default `screenshot` feature (`--screenshot` and `--review-screenshots`, drawn with Vello's CPU renderer, `image`, and `oxipng`) is the only way the release workflow can check that the packaged app draws its interface: its runners have no GPU Vello can use, and the Linux check runs with no display. So the checks on all three systems run `textweaver-gui --screenshot`. `cargo xtask dist --no-screenshot` builds the package without it, with every other default feature; it can become the default once the release checks no longer need the harness. Review screenshots come from a developer build either way.

Releases up to 0.1.0-alpha.9 had the app in packages of its own, named like the terminal packages with `-gui` at the end. Those are not made any more. The one name kept is the `-gui` copy of each Linux `.zsync` file, for the app AppImages already installed.

## The Linux packages

`cargo xtask appimage` stages the Linux package as `cargo xtask dist` does, writes the tarball, and then wraps the same folder in an AppImage with `appimagetool`. The folder sits whole under `usr/lib/textweaver/` inside the AppImage, so the programs find the hosts and dictionaries beside them, as in the tarball. `scripts/linux/AppRun` is the entry point: it starts the app by default (or with `--gui` first), `tw` when started through a link named `tw` or `textweaver` or with `--tw` or `--textweaver` first, and it offers `--install` and `--uninstall`, which link all three names and add menu entries for the app and the terminal reader. The AppImage carries `gh-releases-zsync` update information pointing at the newest release or pre-release, under the same name the terminal AppImages always had, so they update to the single image. The app AppImages of 0.1.0-alpha.9 and earlier look for `textweaver-*-linux-ARCH-gui.AppImage.zsync`; `cargo xtask appimage` writes a copy of the `.zsync` file under that name. Its `URL` line names the single image, which is beside it in the release, so they update to it too.

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

Every package has a size budget. `xtask/package-sizes.toml` records each package's size in bytes at the last release, named without the version (`windows-x86_64.zip`, `linux-x86_64.AppImage`). After writing a package, `cargo xtask dist` (or `gui-dist`) and `appimage` print one line for it, meaning first:

- `Size within budget:` with its size and the change in percent from the last release;
- `Size over budget, with a note:` and the note;
- `Size over budget:` when it grew more than 10 percent with no note. The command then fails, after the package is written, so the release workflow stops before it uploads.

A package with no recorded size (a new platform, or a Mac build that is not `--universal`) is printed and passes.

When a package is meant to grow, say why under `[notes]` in the file, in the same change:

```toml
[notes]
"windows-x86_64.zip" = "Opus encoding in process for Export audio"
```

A note named `"all"` covers every package. Until the first release with the single package records its sizes, each line holds the alpha.9 terminal and app packages added together, the two downloads it replaces. The release step (`cargo xtask release VERSION --sizes`, step 6 above) writes the new sizes and clears the notes, so each note covers one release. The sizes also go into the release notes, as a "Package sizes" list in the version's section of `CHANGELOG.md`. MB there, as in the release workflow's summaries, is 1,048,576 bytes.

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
