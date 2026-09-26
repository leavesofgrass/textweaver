# Installing textweaver

In a hurry? The [quick start](quickstart.md) is the short version.

textweaver is in alpha. Each release on GitHub has a Windows package and a macOS package:

- `textweaver-VERSION-windows-x86_64.zip`
- `textweaver-VERSION-macos-universal.tar.gz`, for Apple silicon and Intel Macs

Both packages contain two programs:

- `textweaver`, the terminal reader. Run `textweaver FILE`.
- `tw`, the command-line tool. Run `tw --help`.

Download from the [releases page](https://github.com/leavesofgrass/textweaver/releases). There is no Linux package yet. On Linux, the install script builds textweaver from source.

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

On Linux, this installs the build dependencies with apt, dnf, pacman, zypper, or apk, builds textweaver, and installs it in `~/.local`:

```bash
bash scripts/install-linux.sh
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

There is no Linux package yet. The install script builds textweaver from source on Debian, Ubuntu, Fedora, Arch, openSUSE, and Alpine; see [Install with a script](#install-with-a-script). The Docker image in `docker/` has everything a build needs, including espeak-ng; see [docs/docker.md](docker.md).

On Linux, textweaver speaks with espeak-ng in process, or through speech-dispatcher. It can also use Eloquence through Voxin, and a licensed DECtalk.

## Checking a download

Each release has a `SHA256SUMS.txt` file. To check a download on Windows:

```powershell
certutil -hashfile textweaver-0.1.0-alpha.3-windows-x86_64.zip SHA256
```

On macOS:

```bash
shasum -a 256 textweaver-0.1.0-alpha.3-macos-universal.tar.gz
```

Compare the result with the line for that file in `SHA256SUMS.txt`.

## Building from source

The [README](../README.md#building) has the build commands, and [CONTRIBUTING.md](../CONTRIBUTING.md) has the full set-up for contributors.

## See also

- [Quick start](quickstart.md): what to do first.
- [Speech engines and voices](speech.md): choosing an engine and a voice.
- [Troubleshooting](troubleshooting.md): when something does not work.
- [scripts/README.md](../scripts/README.md): every install and helper script.
- [Releasing](releasing.md): how the packages are made.
- [Documentation index](README.md)
