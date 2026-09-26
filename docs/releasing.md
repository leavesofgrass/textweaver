# Releasing textweaver

A release is a git tag `vX.Y.Z[-pre]` and a GitHub release with these files:

- the Windows package, `textweaver-VERSION-windows-x86_64.zip`;
- the macOS package, `textweaver-VERSION-macos-universal.tar.gz`;
- `SHA256SUMS.txt`.

The `Release` workflow (`.github/workflows/release.yml`) builds both packages, attests their build provenance, and writes the checksums. Releases before 1.0 are marked as pre-releases.

## Steps

1. **Prepare.** Before you start, run the checks CI runs, locally: `scripts/dev-check.sh` on Linux or macOS (`--docker` for the full Linux set), or `scripts\dev-check.ps1` on Windows. They include the link check and the site data check. Then, on `main`, with a clean tree and CI green, try the release without changing anything:

   ```bash
   cargo xtask release 0.1.0-alpha.3 --dry-run
   ```

   It prints the date it will use, the files it will change, and the checks it will run. Then run it for real:

   ```bash
   cargo xtask release 0.1.0-alpha.3
   ```

   This:

   - stops unless the tree is clean and on `main`;
   - sets `version` in `[workspace.package]` in the root `Cargo.toml` and runs `cargo update -w`;
   - turns `## [Unreleased]` in `CHANGELOG.md` into `## [0.1.0-alpha.3] - YYYY-MM-DD`, keeps an empty `[Unreleased]` above it, and adds the release link. The date comes from the machine's clock in local time, and the weekday is computed and printed so you can check it. It is never typed in;
   - updates the version examples in this guide, `docs/install.md`, and the workflows;
   - runs the checks CI runs: fmt, clippy, the tests, `cargo xtask keyboard --check`, and `cargo xtask deps --check` (`--no-checks` skips them, for a rerun after a failure you have fixed);
   - commits "Release 0.1.0-alpha.3" and makes the annotated tag `v0.1.0-alpha.3`. It pushes nothing.

   Before a release, also run `cargo xtask notices` (it needs `cargo install --locked cargo-about`) and commit `THIRD-PARTY-NOTICES.md` if it changed. CI fails when it is out of date.

2. **Listen.** Before pushing, a person listens on real hardware with Eloquence and one other engine: open a document, read, pause, resume, move by sentence and heading, and change the rate. Tests that fake the engine cannot hear a silent one.

3. **Push.** Push the commit, then the tag:

   ```bash
   git push origin main
   ```

   ```bash
   git push origin v0.1.0-alpha.3
   ```

   Pushing the tag starts the `Release` workflow:

   - **Create the release.** Checks that the version in `Cargo.toml` matches the tag, then creates the GitHub release as a pre-release, with notes taken from the matching `CHANGELOG.md` section.
   - **Windows package** (on `windows-latest`) and **macOS package** (on `macos-14`), in parallel. Each runs `cargo xtask dist`, checks the package (the binaries run, and the notices and licence files are inside), attests its build provenance, and uploads it to the release.
   - **SHA256SUMS.txt.** Once both packages are uploaded, one job writes the checksums of every package on the release and attests the checksum file. The package jobs never write checksums, so they cannot race.

4. **Check.** Read the release page. It should have both packages and `SHA256SUMS.txt`, with the pre-release flag set. Anyone can check where a package was built:

   ```bash
   gh attestation verify textweaver-0.1.0-alpha.3-windows-x86_64.zip --repo leavesofgrass/textweaver
   ```

## What the packages hold

`cargo xtask dist` builds with the `dist` profile (the release profile with fat LTO, and symbols stripped) and writes the package to `target/dist/`. The C runtime is linked statically on Windows, so the package does not need the Visual C++ redistributable. Each package holds:

- `textweaver` and `tw`;
- on Windows, the engine hosts for Eloquence, SAPI5, and DECtalk, each for x64 and x86, and the IBMTTS community dictionaries;
- `QUICKSTART.md`, `README.md`, `LICENSE`, `CHANGELOG.md`, and `INSTALL.md` at the top;
- in `docs/`, every user guide listed under "For users" in the [documentation index](README.md), and the offline interactive pages in `docs/site/`;
- the platform's helper scripts (doctor, speech check, update) and their README;
- `THIRD-PARTY-NOTICES.md`, and under `licenses/`: each bundled font's `OFL.txt`, SCOWL's `Copyright`, and the IBMTTS dictionaries' licence. `cargo xtask dist` fails if any of these is missing.

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
tools/release-upload.sh v0.1.0-alpha.3 target/dist/textweaver-0.1.0-alpha.3-windows-x86_64.zip
```

If the release does not exist yet, the script creates it. A package uploaded this way has no provenance attestation.

To rerun the workflow for an existing tag, start `Release` from the Actions tab and enter the tag.

## Notes

- **macOS signing.** The macOS binaries are universal (built with `lipo`) and signed ad hoc (`codesign -s -`). They are not notarized. `docs/install.md` tells users how to get past Gatekeeper. Notarizing needs an Apple Developer ID. When there is one, add `codesign --options runtime` with that identity and `xcrun notarytool submit --wait` to the workflow.
- **No engines are bundled.** No speech engines are in the packages: no Eloquence, no DECtalk, no voices. The packages hold only textweaver's own programs and hosts, and the CC0 IBMTTS dictionaries.
- **Linux.** There is no Linux package yet. Linux users build from source with `scripts/install-linux.sh`, or use the Docker image. `cargo xtask dist` on Linux builds a `.tar.gz` with the ECI host for Voxin, but it is not published.

## See also

- [Installing textweaver](install.md): what users do with the packages.
- [CONTRIBUTING.md](../CONTRIBUTING.md): the checks and the commit style.
- [CHANGELOG.md](../CHANGELOG.md): the release notes come from here.
- [ADR-0012: The engine host](adr/0012-engine-host.md): `cargo xtask hosts` and host versioning.
- [Documentation index](README.md)
