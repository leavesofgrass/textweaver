# Releasing textweaver

A release is a git tag `vX.Y.Z[-pre]` and a GitHub release with these files:

- the Windows package, built on Jon's machine;
- the macOS package, built by the `Release` workflow (`.github/workflows/release.yml`);
- `SHA256SUMS.txt`.

Releases before 1.0 are marked as pre-releases.

## Steps

1. **Version.** Set `version` in `[workspace.package]` in the root `Cargo.toml`, then add a section to `CHANGELOG.md`. Before you push, run the checks CI runs, locally: `scripts/dev-check.sh` on Linux or macOS (`--docker` for the full Linux set), or `scripts\dev-check.ps1` on Windows. They include the link check and the site data check. Commit both files on `main` and push. Wait until CI is green.
2. **Tag.** Tag the commit and push the tag:

   ```bash
   git tag -a v0.1.0-alpha.3 -m "textweaver 0.1.0-alpha.3"
   ```

   ```bash
   git push origin v0.1.0-alpha.3
   ```

   Pushing the tag starts the `Release` workflow. The workflow creates the GitHub release, as a pre-release, with notes taken from the matching `CHANGELOG.md` section. It then builds the macOS package, uploads it, and updates `SHA256SUMS.txt`.
3. **Windows package.** Build it locally. It needs the MSVC toolchain and the 32-bit target (`rustup target add i686-pc-windows-msvc`):

   ```bash
   cargo xtask dist
   ```

   This writes `target/dist/textweaver-VERSION-windows-x86_64.zip`. The C runtime is linked statically, so the package does not need the Visual C++ redistributable. The zip holds:

   - `textweaver.exe` and `tw.exe`;
   - the engine hosts for Eloquence, SAPI5, and DECtalk, each for x64 and x86;
   - the IBMTTS community dictionaries;
   - `QUICKSTART.md`, `README.md`, `LICENSE`, `CHANGELOG.md`, and `INSTALL.md` at the top;
   - in `docs/`, the Eloquence, keyboard, themes, settings, converting, reading aids, DECtalk, and quick start guides, and the scripts README.

   The file list is in `xtask/src/dist.rs` (`DOCS` and `OPTIONAL_DOCS`). The newer guides (reading, editing, speech, and the rest in the [documentation index](README.md)) and the `docs/site/` pages are not in the package yet.

4. **Upload.** Upload the zip, then refresh the checksums. `tools/release-upload.sh` does both:

   ```bash
   tools/release-upload.sh v0.1.0-alpha.3 target/dist/textweaver-0.1.0-alpha.3-windows-x86_64.zip
   ```

   If the workflow has not yet created the release, the script creates it. The workflow then only uploads.

5. **Check.** Read the release page. It should have both packages and `SHA256SUMS.txt`, with the pre-release flag set.

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
