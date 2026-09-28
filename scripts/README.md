# textweaver scripts

Scripts that install, update, check, and use textweaver on Linux, macOS, and Windows.

Every script:

- has `--help` (in PowerShell, `-Help`; `--help` works there too);
- has `--dry-run` (`-DryRun`), which prints each command instead of running it;
- says what it will do before it does it, in plain sentences, with no colour, spinners, or progress bars that redraw a line;
- is safe to run twice;
- asks before it uses sudo, installs Rust, or changes your PATH, and `--yes` (`-Yes`) answers yes.

The `.sh` scripts need bash, version 3.2 or later, so they run on the bash that macOS ships. The `.ps1` scripts run in Windows PowerShell 5.1 and in PowerShell 7. The PowerShell scripts also accept the GNU-style spellings, such as `--dry-run`.

On Windows, run a PowerShell script like this, so the execution policy does not block it:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1
```

## Installing

### install-linux.sh

Installs textweaver on any Linux distribution: a published release, or a build from source.

With `--release TAG` (a tag such as `v0.1.0-alpha.4`, or `latest` for the newest release or pre-release), nothing is built:

- It downloads the AppImage and `SHA256SUMS.txt` from the release with curl (or wget), and checks the AppImage against its line there. A mismatch stops the install.
- It installs the AppImage as `~/.local/bin/textweaver.AppImage` (or under `--prefix DIR`), links `textweaver` and `tw` to it, and adds a menu entry, an icon, and the quick start.
- Where FUSE is missing, so AppImages cannot run, it installs the plain tarball instead, checked the same way, into `lib/textweaver/` with `textweaver` and `tw` linked from `bin/`. `--tarball` asks for the tarball anyway; `--appimage` asks for the AppImage anyway.
- It is for x86_64 and aarch64 (arm64) computers. Elsewhere, build from source.
- It needs a release with Linux packages: 0.1.0-alpha.4 is the first, with the AppImage and tarball for both x86_64 and aarch64. Earlier releases (0.1.0-alpha.3 and before) shipped Windows and macOS packages only; build from source for those.
- `--uninstall` removes either kind of install; `scripts/update.sh` installs the newest release of the same kind.

```bash
scripts/install-linux.sh --release latest
```

Without `--release`, it builds textweaver from source:

- It finds your package manager from `/etc/os-release` and what is on your PATH: apt (Debian, Ubuntu), dnf or yum (Fedora, Red Hat), pacman (Arch, Manjaro), zypper (openSUSE), or apk (Alpine). On other systems it prints the list of packages to install by hand.
- It installs the build dependencies: a C toolchain, pkg-config, the ALSA headers, espeak-ng, and speech-dispatcher, with their development files.
- It offers ffmpeg and pandoc, and says where to get whisper.cpp for dictation.
- It installs rustup if cargo is missing, after asking. The Rust version comes from `rust-toolchain.toml`.
- It builds `textweaver` and `tw` in release mode with the espeak-ng, speech-dispatcher, and Omnivox engines, and builds the engine hosts with `cargo xtask hosts`. If the espeak-ng engine does not build on your distribution, it builds again without it (`--no-espeak` skips it from the start).
- It installs under `~/.local`, or `--prefix DIR`: the programs are linked from `bin/`, and the programs, engine hosts, and dictionaries live in `lib/textweaver/`. The guides, a `HELP.txt` with every command's help, manual pages, a menu entry, and these helper scripts are installed too.
- It offers to add `~/.local/bin` to your PATH.

To install:

```bash
scripts/install-linux.sh
```

To see what it would do, without doing it:

```bash
scripts/install-linux.sh --dry-run
```

To remove it again:

```bash
scripts/install-linux.sh --uninstall
```

`--deps-only` installs the system packages and stops. Alpine has no bash by default: run `apk add bash` first.

### install-macos.sh

Installs textweaver on a Mac.

- By default it downloads the newest release, the universal package for Apple silicon and Intel, with curl. It checks it against `SHA256SUMS.txt` with `shasum -a 256`, extracts it, removes the quarantine flag, and installs `textweaver` and `tw` into `~/.local/bin`, or `--prefix DIR`.
- `--release TAG` installs a given release.
- `--from-source` checks for the Xcode command line tools, installs rustup if needed (it asks first), and builds with `cargo xtask dist`.
- It offers ffmpeg and pandoc from Homebrew when Homebrew is installed. It never installs Homebrew itself.
- `--uninstall` removes it.

### install-windows.ps1

Installs textweaver on Windows.

- By default it downloads the newest release zip, checks its SHA-256 against `SHA256SUMS.txt`, and extracts it to `%LOCALAPPDATA%\Programs\textweaver`.
- It offers to add that folder to your user PATH, and to create a Start menu shortcut that opens textweaver in Windows Terminal, or in a command prompt.
- `-FromSource` checks for the MSVC build tools, rustup, and the `i686-pc-windows-msvc` target, then runs `cargo xtask dist` and installs the result.
- `-Uninstall` removes the folder, the PATH entry, and the shortcut. Settings are kept.

### update.sh and update.ps1

Update an installed textweaver. They read the install manifest that the installers write. A source install is pulled with `git pull --ff-only` and rebuilt. A release install runs the installer again for the newest release.

## Speech

### speech-check.sh and speech-check.ps1

A plain report on speech: textweaver's version, `tw backends`, the first few voices of each available engine, and `tw eloquence`.

- On Linux: espeak-ng, speech-dispatcher (`spd-say -O` and `spd-say -L`), PipeWire or PulseAudio, the ALSA cards, and whether the session has what speech-dispatcher and the sound server need.
- On macOS: the Eloquence voices in `say -v '?'`.
- On Windows: the SAPI5 (64-bit and 32-bit) and OneCore voices, from the registry. `-Probe` runs `tools\sapi_probe.ps1` to check each voice's word events; it writes temporary WAV files and plays nothing, but it loads each voice's engine, so `-SkipVoice` leaves some out.
- `--skip ID` leaves an engine's voices out, because listing voices starts the engine.
- `--speak` says one test sentence. It is off by default.

### voxin-docker.sh

Runs textweaver with Voxin, the Linux ETI-Eloquence, in the Docker development container, with your Voxin volume mounted through `compose.voxin.yaml`. Actions:

- `check` writes a WAV file with `voxin-say`;
- `test` runs the Eloquence backend's real-engine tests;
- `speak TEXT` writes `tw speak --backend eci` to a WAV file;
- `spike` runs the index-mark spike;
- `shell` opens a shell.

Nothing is played.

## Utilities

### doctor.sh and doctor.ps1

One plain-text report to paste into a bug report. It covers:

- the operating system and its version;
- the terminal, `TERM`, and `COLORTERM`;
- the locale, and the screen reader if one is running;
- the Rust toolchain;
- where textweaver is installed, and `tw settings path`;
- `tw backends`;
- whether the engine hosts and dictionaries sit beside the programs;
- the optional tools.

It reads no file contents and dumps no environment: only the names of `TEXTWEAVER_` variables that are set. Your home folder is shown as `~`. `--out FILE` also writes the report to a file.

### dev-check.sh and dev-check.ps1

Run every check CI runs, so contributors see CI's answer before they push:

- formatting;
- clippy with warnings as errors;
- the tests;
- rustdoc with `-D warnings`;
- `cargo xtask keyboard --check`;
- `tools/check_links.py`: every relative link and anchor in the docs resolves;
- `tools/gen_site_data.py --check`: the data in the `docs/site` pages is current;
- `tools/check_site_a11y.py`: static accessibility checks of the `docs/site` pages;
- shellcheck or PSScriptAnalyzer on these scripts, when installed.

The two Python steps need Python 3; without it they are skipped and the summary says so.

On Windows it uses `--features textweaver-speech/omnivox` instead of `--all-features`, and it also builds the 32-bit engine hosts. `--only fmt,clippy` runs some of the steps, and `--docker` runs everything in the development container.

Three CI checks are not in dev-check: `cargo xtask deps --check`, `cargo xtask notices --check`, and cargo-deny. [Testing](../docs/dev/testing.md#the-checks) shows how to run them.

### convert-folder.sh and convert-folder.ps1

Convert a folder of Markdown, or other documents, with `tw convert`. The output goes to a folder beside it named after the format, so `notes` becomes `notes-html`.

To convert the folder `notes` to HTML:

```bash
scripts/convert-folder.sh notes
```

To make EPUB books instead:

```bash
scripts/convert-folder.sh notes --to epub
```

To keep watching the folder and convert files as they arrive:

```bash
scripts/convert-folder.sh notes --watch
```

Formats are html (the default), epub, pdf, docx, brf (braille), txt, and md. Anything after `--` goes to `tw convert`. In PowerShell, use `-TwArgs "--flavor obsidian"` instead.

### linux/textweaver.desktop

The menu entry that `install-linux.sh` installs. It opens textweaver in a terminal (`Terminal=true`) for text, Markdown, HTML, EPUB, DOCX, and PDF files.

## See also

- [Installing textweaver](../docs/install.md): the packages, and what the install scripts do.
- [Troubleshooting](../docs/troubleshooting.md): using the doctor and speech-check reports.
- [Converting documents](../docs/converting.md): everything `tw convert` can do.
- [CONTRIBUTING.md](../CONTRIBUTING.md): the checks dev-check runs, and why.
- [Documentation index](../docs/README.md)
