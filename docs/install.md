# Installing textweaver

In a hurry? The [quick start](quickstart.md) is the short version.

textweaver is in alpha. The newest release is 0.1.0-alpha.3. A release on GitHub has these packages:

- `textweaver-VERSION-windows-x86_64.zip`
- `textweaver-VERSION-macos-universal.tar.gz`, for Apple silicon and Intel Macs
- `textweaver-VERSION-linux-x86_64.AppImage`, one file that runs on most Linux distributions
- `textweaver-VERSION-linux-x86_64.tar.gz`, the same programs as a plain folder, for Linux systems where AppImages cannot run
- `textweaver-VERSION-linux-aarch64.AppImage` and `textweaver-VERSION-linux-aarch64.tar.gz`, the same for 64-bit ARM (arm64) computers

The Linux packages start with the next release. 0.1.0-alpha.3 has the Windows and macOS packages only; on Linux, build it from source with the install script (see [Install with a script](#install-with-a-script)).

Every package contains two programs:

- `textweaver`, the terminal reader. Run `textweaver FILE`.
- `tw`, the command-line tool. Run `tw --help`.

Download from the [releases page](https://github.com/leavesofgrass/textweaver/releases).

## Install with a script

The scripts in `scripts/` do the steps below for you. Each one says what it will do before it does it. It asks before it changes your PATH or uses sudo. Each has `--help` and `--dry-run`. [scripts/README.md](../scripts/README.md) describes them all.

On Windows, this downloads the newest release, checks it, and installs it in `%LOCALAPPDATA%\Programs\textweaver`:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1
```

On macOS, this downloads the newest release, checks it, and installs `textweaver` and `tw` in `~/.local/bin`:

```bash
bash scripts/install-macos.sh
```

On Linux, this builds textweaver from source: it installs the build dependencies with apt, dnf, pacman, zypper, or apk, builds textweaver, and installs it in `~/.local`. This is the way to install 0.1.0-alpha.3, which has no Linux package:

```bash
bash scripts/install-linux.sh
```

From the next release on, `--release` downloads a release instead, checks it, and installs it in `~/.local`: the AppImage, with `textweaver` and `tw` in `~/.local/bin`, or the tarball where AppImages cannot run:

```bash
bash scripts/install-linux.sh --release latest
```

The scripts are in a copy of the repository. To get one:

```bash
git clone https://github.com/leavesofgrass/textweaver
```

Each script also takes `--uninstall`.

To update later, run the update script for your system:

```bash
scripts/update.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\update.ps1
```

If something does not work, the doctor script prints a report to paste into a bug report:

```bash
scripts/doctor.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\doctor.ps1
```

The speech check script checks your voices:

```bash
scripts/speech-check.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\speech-check.ps1
```

## Windows

1. Download the `.zip` and extract it to a folder of your own, for example `C:\Tools\textweaver`.
2. Optionally, add that folder to your `PATH`, so you can run `textweaver` and `tw` from any folder.
3. Open Windows Terminal or a command prompt. Run this to see which speech engines textweaver found:

   ```powershell
   tw backends
   ```

Keep the files in the folder together. The engine hosts must stay next to `textweaver.exe` and `tw.exe`:

- `textweaver-eci-host.exe` and `textweaver-eci-host-x86.exe`, for Eloquence;
- `textweaver-sapi-host.exe` and `textweaver-sapi-host-x86.exe`, for SAPI5 voices;
- `textweaver-dectalk-host.exe` and `textweaver-dectalk-host-x86.exe`, for DECtalk.

The `ibmtts-dictionaries` folder must stay there too. The hosts run speech engines in their own processes, so a crash in an engine never takes the reader down. They also let 32-bit voices work with 64-bit textweaver.

- **SAPI5 voices**, the Windows voices, work straight away. That includes OneCore voices, and any SAPI5 voices you have installed.
- **Eloquence** needs an Eloquence library you have a right to use. See [the Eloquence guide](eloquence.md) (`docs/eloquence.md` in the package), or run `tw eloquence --guide`.
- **DECtalk** needs a DECtalk you have installed and licensed. See [the DECtalk guide](dectalk.md).

The programs are not code-signed. Windows SmartScreen may warn the first time you run one. Choose "More info", then "Run anyway". A command-line program run from a terminal usually shows no warning.

## macOS

The macOS build is not notarized by Apple yet, because notarization needs a paid developer account. The binaries are signed ad hoc, so they run on Apple silicon. Gatekeeper still blocks the first run of a download that is not notarized, so do this once.

1. Download the `.tar.gz`. Extract it in Finder, or in Terminal:

   ```bash
   tar -xzf textweaver-*-macos-universal.tar.gz
   ```

2. Remove the quarantine flag that the browser added. Replace the folder name with the one you extracted:

   ```bash
   xattr -dr com.apple.quarantine textweaver-0.1.0-alpha.3-macos-universal
   ```

   If you skip this, macOS says the program "cannot be opened because Apple cannot check it for malicious software". In that case open System Settings, go to Privacy & Security, and choose "Open Anyway" next to the message about `tw` or `textweaver`. Then run the program again.

3. Optionally, copy `textweaver` and `tw` to a folder on your `PATH`, such as `/usr/local/bin`.

textweaver speaks with Apple's voices, including the Eloquence voices built into macOS. Reed is the default when it is installed. To list the voices:

```bash
tw voices
```

## Linux

The Linux package is an AppImage: one file that holds `textweaver`, `tw`, the engine hosts for Eloquence (Voxin) and DECtalk, the pronunciation dictionaries, the guides, and the licences. It is built on Ubuntu 22.04, so it runs on distributions from 2022 on, including Debian 12 and 13, Ubuntu 22.04 and later, Fedora, Arch, and openSUSE. There is one for x86_64 computers and one for 64-bit ARM (aarch64) computers; the install script picks the one for your computer. In the steps below, write `aarch64` where they say `x86_64` if `uname -m` says `aarch64`.

The first release with the AppImage is the one after 0.1.0-alpha.3. Until it is out, build from source with `bash scripts/install-linux.sh`, as described at the end of this section.

The easiest way is the install script, which checks the download for you (see [Install with a script](#install-with-a-script)):

```bash
bash scripts/install-linux.sh --release latest
```

To install it by hand:

1. Download `textweaver-VERSION-linux-x86_64.AppImage` and `SHA256SUMS.txt`, and check the file (see [Checking a download](#checking-a-download)).
2. Put it where you want to keep it, for example `~/Applications`, and make it executable:

   ```bash
   chmod +x textweaver-*-linux-x86_64.AppImage
   ```

3. Run it. On its own it starts `textweaver`, the reader:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage FILE
   ```

   With `--tw` first it runs `tw` instead:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage --tw backends
   ```

4. To run `textweaver` and `tw` from any folder, let the AppImage link itself into `~/.local/bin` and add a menu entry and an icon. It says what it will do and asks first:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage --install
   ```

   A link named `tw` runs `tw`; a link named `textweaver` runs the reader. The links point at the AppImage where it is, so move it before you run `--install`. `--uninstall` removes the links, the menu entry, and the icon.

**Without FUSE.** An AppImage mounts itself with FUSE, which most desktops have. Where it is missing (some containers and minimal systems), the AppImage says so. Then either set `APPIMAGE_EXTRACT_AND_RUN=1`, which makes it unpack itself to a temporary folder each time it runs, or use the tarball: extract it anywhere and run `textweaver` and `tw` from the folder. The install script picks the tarball by itself when FUSE is missing (`--tarball` asks for it).

**Sound.** textweaver needs the ALSA library, `libasound.so.2`, which every Linux desktop has. On a minimal system, install `alsa-lib` (Fedora, Arch, openSUSE) or `libasound2` (Debian, Ubuntu).

**Speech engines.**

- **espeak-ng** speaks inside textweaver when it is installed: install the `espeak-ng` package. Without it, the same AppImage still works and speaks through the other engines.
- **speech-dispatcher** works when its server is installed and running, as it is on most desktops.
- **Eloquence** through Voxin, and a licensed **DECtalk**, work through the bundled engine hosts; see [the Eloquence guide](eloquence.md) and [the DECtalk guide](dectalk.md). A host loads the engine's library for its own architecture, so on an aarch64 computer it needs an aarch64 build of the engine.

Run `tw backends` to see which engines textweaver found.

**Updates.** The AppImage carries update information, so AppImageUpdate and similar tools can update it, downloading only what changed. `scripts/update.sh` updates an install made by the script.

To build from source instead, run the install script without `--release`; it works on Debian, Ubuntu, Fedora, Arch, openSUSE, and Alpine, and on other architectures. The Docker image in `docker/` has everything a build needs, including espeak-ng; see [docs/dev/docker.md](dev/docker.md).

## Checking a download

Each release has a `SHA256SUMS.txt` file. To check a download on Windows:

```powershell
certutil -hashfile textweaver-0.1.0-alpha.3-windows-x86_64.zip SHA256
```

On macOS:

```bash
shasum -a 256 textweaver-0.1.0-alpha.3-macos-universal.tar.gz
```

On Linux:

```bash
sha256sum textweaver-*-linux-x86_64.AppImage
```

Compare the result with the line for that file in `SHA256SUMS.txt`.

## Building from source

The [README](../README.md#building) has the build commands, and [CONTRIBUTING.md](../CONTRIBUTING.md) has the full set-up for contributors.

## See also

- [Quick start](quickstart.md): what to do first.
- [Speech engines and voices](speech.md): choosing an engine and a voice.
- [Troubleshooting](troubleshooting.md): when something does not work.
- [scripts/README.md](../scripts/README.md): every install and helper script.
- [Releasing](dev/releasing.md): how the packages are made.
- [Documentation index](README.md)
