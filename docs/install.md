# Installing textweaver

The [quick start](quickstart.md) is the short version of this guide. On Windows, the fastest route is to download `textweaver-VERSION-windows-x86_64.zip`, extract it, and run `textweaver.exe`, the textweaver app (see [The app](#the-app)).

textweaver is in beta. The newest release is 0.1.0-beta.4. Since beta 1, each release on GitHub provides one self-contained package for each system and processor:

- `textweaver-VERSION-windows-x86_64.zip`, for Windows on x86-64 computers (it also runs on Windows on ARM, under emulation)
- `textweaver-VERSION-macos-universal.zip`, for Apple silicon and Intel Macs
- `textweaver-VERSION-linux-x86_64.AppImage`, one file that runs on most Linux distributions
- `textweaver-VERSION-linux-x86_64.tar.gz`, the same programs as a plain folder, for Linux systems that cannot run AppImages
- `textweaver-VERSION-linux-aarch64.AppImage` and `textweaver-VERSION-linux-aarch64.tar.gz`, the equivalents for 64-bit ARM (arm64) computers

Since beta 3, textweaver is one program, and every package contains it with the complete documentation:

- `textweaver` starts the textweaver app, which supports the mouse as well as the keyboard; `textweaver FILE` opens a document in it. On macOS the program is inside `textweaver.app`.
- `tw` runs the terminal program from the same program. `tw FILE` opens the terminal reader, and `tw COMMAND` runs a command without it, such as `tw convert` (see [The command line](command-line.md)). On Linux and macOS `tw` is a link to `textweaver`; on Windows `tw.exe` is a small console program that starts `textweaver.exe` in terminal mode and waits for it, as a terminal expects.
- `textweaver --tw` (or `--cli`) does the same as `tw`, and `textweaver COMMAND`, such as `textweaver convert`, runs the command, so scripts written for earlier releases keep working.

Releases up to beta 2 had separate programs: the app as `textweaver-gui`, and `tw` with `textweaver` as a second name that opened the terminal reader. Releases up to 0.1.0-alpha.9 distributed the app in separate packages whose names ended in `-gui`. Neither is produced any more.

Download from the [releases page](https://github.com/leavesofgrass/textweaver/releases).

## Install with a script

The scripts in `scripts/` perform the steps below for you. Each states what it will do before doing it, asks before it changes your PATH or uses sudo, and accepts `--help` and `--dry-run`. [scripts/README.md](../scripts/README.md) describes them all.

On Windows, this downloads the newest release, verifies it, and installs it in `%LOCALAPPDATA%\Programs\textweaver`:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1
```

On macOS, this downloads the newest release, verifies it, copies `textweaver.app` into `~/Applications`, and links `textweaver` and `tw` to the program inside it from `~/.local/bin`:

```bash
bash scripts/install-macos.sh
```

On Linux, `--release` downloads a release, verifies it, and installs it in `~/.local`. It installs the AppImage, with `textweaver` and `tw` in `~/.local/bin`, or the tarball where AppImages cannot run:

```bash
bash scripts/install-linux.sh --release latest
```

Without `--release`, the script builds textweaver from source instead. It installs the build dependencies with apt, dnf, pacman, zypper, or apk, builds textweaver, and installs it in `~/.local`:

```bash
bash scripts/install-linux.sh
```

The scripts are part of the repository; to obtain a copy, clone it:

```bash
git clone https://github.com/leavesofgrass/textweaver
```

The app is always installed, since it is the program itself. On Windows the script offers two Start menu shortcuts: "textweaver app", which starts `textweaver.exe`, and "textweaver terminal", which opens the terminal reader, `tw.exe`, in Windows Terminal. On Linux it adds two menu entries, "textweaver" for the app and "textweaver terminal reader". An install made by an earlier release is upgraded in place: the scripts remove the old app, `textweaver-gui`, and the shortcuts, links, and menu entries they made for it, and only those. The `--gui` option is no longer needed; it still installs the separate app of a release up to beta 2 when you ask for one with `--release`.

Each script also accepts `--uninstall`, which removes textweaver, its links, and its shortcuts or menu entries.

To update later, run the update script for your system:

```bash
scripts/update.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\update.ps1
```

textweaver can also update itself from a release package. On its first run it asks once whether to check for updates automatically, and Help, Check for updates checks on demand; in both cases it asks before downloading. `tw update` performs the whole update from the command line. Every update is verified against the release's checksums and leaves your settings and notes untouched. [Updates](updates.md) explains each step.

If something does not work, the doctor script prints a report suitable for a bug report:

```bash
scripts/doctor.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\doctor.ps1
```

The speech check script tests your voices:

```bash
scripts/speech-check.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\speech-check.ps1
```

## Windows

1. Download the `.zip` and extract it to a folder of your choosing, for example `C:\Tools\textweaver`.
2. Optionally, add that folder to your `PATH`, so that `textweaver` and `tw` run from any folder.
3. Open Windows Terminal or a command prompt and list the speech engines textweaver found:

   ```powershell
   tw backends
   ```

To start the app, run `textweaver.exe`; to open the terminal reader, run `tw` in a terminal.

The package holds three programs, which must stay together in one folder:

- `textweaver.exe`, the program: the app, and the terminal program in terminal mode;
- `tw.exe`, which terminals run: it starts `textweaver.exe` in terminal mode, with the same arguments, and waits for it;
- `textweaver-host-x86.exe`, the 32-bit engine host.

`textweaver.exe` is a Windows program, not a console program, so starting the app from the Start menu or File Explorer opens no console window. The price is that a terminal does not wait for it, so in a terminal use `tw`, which does. `textweaver.exe --tw` typed in a terminal also runs the terminal program, attached to that terminal, but the terminal's prompt returns at once and competes with it for the keyboard; prefer `tw`.

The `ibmtts-dictionaries` folder must stay there as well. Eloquence, SAPI5 voices, DECtalk, and eSpeak NG each run in a separate process, so a crash in an engine cannot take the reader down. For a 64-bit engine, that process is a copy of the program you are using, started in a hidden `--engine-host` mode. For a 32-bit engine (Code Factory's Eloquence, most DECtalk libraries, older SAPI5 voices), it is the single 32-bit host, `textweaver-host-x86.exe`, which allows 32-bit voices to work with 64-bit textweaver. eSpeak NG itself is installed with its own Windows installer.

Packages before 0.1.0-beta.3 included a separate host program for each engine, such as `textweaver-eci-host.exe` and `textweaver-sapi-host-x86.exe`. These are no longer used; if an update left them in the folder, they can be deleted. <!-- history -->

- **SAPI5 voices**, the Windows voices, work immediately, including the OneCore voices and any SAPI5 voices you have installed.
- **Eloquence** requires an Eloquence library you are entitled to use. See [the Eloquence guide](eloquence.md) (`docs/eloquence.md` in the package), or run `tw eloquence --guide`.
- **DECtalk** requires a DECtalk installation you have licensed. See [the DECtalk guide](dectalk.md).

The programs are not code-signed, so Windows SmartScreen may warn the first time you run one; choose "More info", then "Run anyway". A command-line program started from a terminal usually shows no warning.

## macOS

The macOS build is not yet notarized by Apple, because notarization requires a paid developer account. The binaries are signed ad hoc, which is enough to run on Apple silicon, but Gatekeeper still blocks the first run of a download that is not notarized. The following steps are needed once.

1. Download the `.zip`. Extract it in Finder, or in Terminal:

   ```bash
   ditto -x -k textweaver-*-macos-universal.zip .
   ```

   The folder holds `textweaver.app`, the app, with `textweaver` and `tw` beside it: links to the program inside the app.

2. Remove the quarantine flag that the browser added, substituting the name of the folder you extracted:

   ```bash
   xattr -dr com.apple.quarantine textweaver-0.1.0-beta.4-macos-universal
   ```

   If you skip this step, macOS reports that the program "cannot be opened because Apple cannot check it for malicious software". In that case, open System Settings, go to Privacy & Security, choose "Open Anyway" next to the message about `textweaver.app` or `tw`, and run the program again.

3. Open `textweaver.app` to start the app; you may first move it to your Applications folder. To use the terminal program, link `tw` into a folder on your `PATH`, such as `/usr/local/bin`, pointing at the program inside the app where you keep it, for example `ln -s /Applications/textweaver.app/Contents/MacOS/textweaver /usr/local/bin/tw`. Link `textweaver` the same way to start the app from a terminal.

On macOS, textweaver speaks with Apple's voices, including the Eloquence voices built into the system; Reed is the default when it is installed. To list the voices:

```bash
tw voices
```

## Linux

The Linux package is an AppImage, a single file that holds the program (`textweaver`, with `tw` a link to it), the pronunciation dictionaries, the guides, and the licenses. It is built on Ubuntu 22.04 and therefore runs on distributions from 2022 onward, including Debian 12 and 13, Ubuntu 22.04 and later, Fedora, Arch, and openSUSE. One AppImage is built for x86_64 computers and one for 64-bit ARM (aarch64) computers, and the install script selects the one that matches your computer. If `uname -m` reports `aarch64`, substitute `aarch64` for `x86_64` in the steps below.

The simplest method is the install script, which verifies the download for you (see [Install with a script](#install-with-a-script)):

```bash
bash scripts/install-linux.sh --release latest
```

To install by hand:

1. Download `textweaver-VERSION-linux-x86_64.AppImage` and `SHA256SUMS.txt`, and verify the file (see [Checking a download](#checking-a-download)).
2. Move it to its permanent location, for example `~/Applications`, and make it executable:

   ```bash
   chmod +x textweaver-*-linux-x86_64.AppImage
   ```

3. Run it. Started without options, it opens the app:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage FILE
   ```

   With `--tw` (or `--cli`) as the first argument it runs the terminal program instead, either the terminal reader (with a file or none) or a command:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage --tw FILE
   ./textweaver-*-linux-x86_64.AppImage --tw backends
   ```

4. To run `textweaver` and `tw` from any folder, let the AppImage link itself into `~/.local/bin` and add menu entries for the app and the terminal reader, together with an icon. It states what it will do and asks first:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage --install
   ```

   A link named `textweaver` starts the app, and a link named `tw` runs the terminal program. The links point at the AppImage where it currently is, so move it into place before running `--install`. It removes the `textweaver-gui` link and menu entry that `--install` made before beta 3. `--uninstall` removes the links, the menu entries, and the icon.

**Without FUSE.** An AppImage mounts itself with FUSE, which most desktops provide. Where FUSE is missing (some containers and minimal systems), the AppImage reports it. You can then either set `APPIMAGE_EXTRACT_AND_RUN=1`, which makes the AppImage unpack itself to a temporary folder on each run, or use the tarball: extract it anywhere and run `textweaver` or `tw` from the folder. The install script chooses the tarball automatically when FUSE is missing, and `--tarball` requests it explicitly.

**Servers and containers.** `tw` is the same program as the app, but its commands and the terminal reader need no desktop: the app's libraries for windows and fonts are loaded only when the app starts.

**Sound.** textweaver requires the ALSA library, `libasound.so.2`, which every Linux desktop includes. On a minimal system, install `alsa-lib` (Fedora, Arch, openSUSE) or `libasound2` (Debian, Ubuntu).

**Speech engines.**

- **espeak-ng** speaks inside textweaver once the `espeak-ng` package is installed. Without it, the AppImage still works and speaks through the other engines.
- **speech-dispatcher** works when its server is installed and running, as it is on most desktops.
- **Eloquence** through Voxin, and a licensed **DECtalk**, run in an engine host, a copy of `textweaver` started in its hidden `--engine-host` mode; see [the Eloquence guide](eloquence.md) and [the DECtalk guide](dectalk.md). The host loads the engine's library for its own architecture, so an aarch64 computer needs an aarch64 build of the engine.

Run `tw backends` to see which engines textweaver found.

**Updates.** The AppImage carries update information, so AppImageUpdate and similar tools can update it by downloading only what changed. After an update from beta 2 or earlier, a link named `textweaver` opens the app rather than the terminal reader; a link named `tw` still runs the terminal program. The separate app AppImages of 0.1.0-alpha.9 and earlier are no longer offered updates; download the current AppImage instead. `scripts/update.sh` updates an install made by the script.

To build from source instead, run the install script without `--release`; it supports Debian, Ubuntu, Fedora, Arch, openSUSE, and Alpine, and other architectures. The Docker image in `docker/` contains everything a build needs, including espeak-ng; see [docs/dev/docker.md](dev/docker.md).

## The app

The textweaver app, started with `textweaver`, offers reading and writing with the mouse as well as the keyboard, for people who prefer it to a terminal. Its guide is [docs/gui.md](gui.md), also in the package as `GUI.md`. The app is supported on Windows. The macOS and Linux builds are built and checked automatically with every release, but they have not yet been tested with a screen reader.

The app and the terminal program are one program, with the same dictionaries (and, on Windows, the same 32-bit engine host); keep the folder's files together. The app and the terminal reader share their settings, reading positions, and notes.

- **Windows.** Extract `textweaver-VERSION-windows-x86_64.zip` to a folder of your choosing and run `textweaver.exe`. The program is not code-signed, so Windows SmartScreen warns the first time you start it from File Explorer; choose "More info", then "Run anyway".
- **macOS.** Extract the zip and remove the quarantine flag once, as in [macOS](#macos). Then open `textweaver.app`.
- **Linux.** Run the AppImage without options, or extract the tarball and run `textweaver` from its folder. The app requires a desktop session (Wayland or X11) and, like the terminal reader, the ALSA library.

## Optional components

A package does not contain every tool textweaver can use. ffmpeg (audio export), liblouis (braille), and Pandoc are fetched from their official releases the first time a feature needs one, and only after asking. Components can also come from your own repository or folder. [Optional components](components.md) explains what is fetched, where it is kept, and how to remove it.

## Checking a download

Each release has a `SHA256SUMS.txt` file. To check a download on Windows:

```powershell
certutil -hashfile textweaver-0.1.0-beta.4-windows-x86_64.zip SHA256
```

On macOS:

```bash
shasum -a 256 textweaver-0.1.0-beta.4-macos-universal.zip
```

On Linux:

```bash
sha256sum textweaver-*-linux-x86_64.AppImage
```

Compare the result with the line for that file in `SHA256SUMS.txt`. textweaver's own updates make this comparison automatically (see [Updates](updates.md)).

Every package also carries a build attestation, a signed record of the GitHub workflow that produced it. With the GitHub CLI installed, you can verify one with:

```bash
gh attestation verify textweaver-0.1.0-beta.4-windows-x86_64.zip --repo leavesofgrass/textweaver
```

## Building from source

[Building](dev/building.md) lists the build commands, and [CONTRIBUTING.md](../CONTRIBUTING.md) describes the full contributor setup.

## See also

- [Quick start](quickstart.md): what to do first.
- [Updates](updates.md): keeping textweaver current.
- [Optional components](components.md): the tools textweaver fetches for you.
- [Speech engines and voices](speech.md): choosing an engine and a voice.
- [Troubleshooting](troubleshooting.md): when something does not work.
- [scripts/README.md](../scripts/README.md): every install and helper script.
- [Releasing](dev/releasing.md): how the packages are made.
- [Documentation index](README.md)
