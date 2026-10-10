# Optional components

Some of what textweaver can do needs files that are not in its packages: the Whisper model for dictation, the OCR models for reading scanned pages, the Lexend reading font, and three helper programs (FFmpeg, liblouis, and Pandoc). These are optional components. textweaver never downloads one on its own. It asks first, names the size and the license, and downloads only when you say yes. Saying no is always fine: the feature says what it needs, in words, and the rest of textweaver works as before.

This guide is written to be read with a screen reader. Each section starts with what you do, then explains it.

## The components

| Component | Id | Size | License | What needs it |
| --- | --- | --- | --- | --- |
| Whisper base.en, English dictation | `whisper-base.en` | 79.3 MB | MIT, unconfirmed | dictation (the default model) |
| Whisper small.en, better English dictation | `whisper-small.en` | 251 MB | MIT, unconfirmed | dictation (when chosen in the settings) |
| The ocrs text recognition models for English | `ocr-ocrs` | 12.2 MB | CC BY-SA 4.0 | reading scanned pages |
| The PaddleOCR Latin text recognition model (experimental) | `ocr-paddle-latin` | 8.0 MB | Apache-2.0 | reading scanned pages in Latin-script languages |
| The Lexend reading font | `lexend` | 206 KB | SIL Open Font License | the Lexend reading font |
| FFmpeg 9.0.2, for M4B and MP4 export | `ffmpeg` | 171 MB on Windows, 138 MB on Linux | LGPL-2.1-or-later | M4B audiobooks and MP4 video |
| liblouis 3.39.0, for braille translation | `liblouis` | 5.8 MB, Windows only | LGPL-2.1-or-later; its tools GPL-3.0-or-later | braille files read as print, contracted braille |
| Pandoc 3.12.1, for more document formats | `pandoc` | 42.2 MB on Windows, 35.4 MB on Linux, 26 to 42 MB on macOS | GPL-2.0-or-later | the formats only Pandoc reads (Org, reStructuredText, and more) |

The three helper programs are listed only where their publishers make a build for your computer; see [Helper programs](#helper-programs). The sizes for ARM computers differ a little.

Piper voices are optional components too. The voice manager lists them from Piper's catalog, with each voice's size and license; see [Speech](speech.md).

The Whisper models' license is MIT, from OpenAI. The onnx-community copies textweaver downloads declare no license of their own, so textweaver says "MIT, unconfirmed".

## When a feature needs one

When you use a feature whose component is missing, textweaver asks at that moment. For dictation, you hear:

"Dictation needs the Whisper model, 79.3 MB, license MIT, unconfirmed. Download it now? y or n"

- Press `y`: the download starts on its own. Its progress is said every 10 percent ("40 percent downloaded."), never as a spinner. Escape stops it ("Download stopped; it resumes later."), and the next download goes on from where it stopped. When it finishes you hear "Ready", and the feature starts: dictation begins at once, with no restart.
- Press `n`: nothing is downloaded. You are not asked again until textweaver starts again; the feature says "No model, so no dictation for now." Manage optional components can get it at any time.

Choosing Lexend as the reading font asks the same way. The OCR models are offered by `tw ocr download` and by Manage optional components.

The helper programs are asked for the same way, at the moment a feature needs one, in one question instead of a "not found" message:

- **Export audio without FFmpeg.** "M4B and MP4 export needs FFmpeg 9.0.2, for M4B and MP4 export, 171 MB, license LGPL-2.1-or-later. Download it now? y or n". After a yes and the download, the list of formats opens again with M4B and MP4 in it. After a no, the list opens without them.
- **A braille file opened without liblouis.** The file opens as braille, and you hear "Reading braille as print needs liblouis 3.39.0, ...". After a yes, the file opens again, read as print.
- **`tw convert` on a file only Pandoc reads**, such as an `.org` or `.rst` file, asks on a terminal before converting, then carries on with Pandoc.

A no is remembered until textweaver starts again. Where a helper has no build for your computer, the message names the command that installs it from your system's packages instead, for you to copy: "To get liblouis, run: sudo apt install liblouis-bin".

## Manage optional components

Tools menu, Manage optional components. The command palette finds it by name too.

The list names each component, meaning first: "Whisper base.en, English dictation: not installed, 79.3 MB, license MIT, unconfirmed, for dictation". Enter on one offers:

- **Download**, with its size: asks first, then downloads, checks, and installs it.
- **Verify the files**: checks each file's size and SHA-256. It says "Files check out" or names each file that does not.
- **Remove**: asks first, then removes the component's own files, and its folder when nothing else is in it. A file you put there yourself stays.
- **Install from a zip file…** and **Install from a folder…**: see [Installing from a file](#installing-from-a-file).

Delete on a component asks to remove it. A change is seen at once by every feature: a downloaded model is used by the next Dictate, a removed one is asked for again, and the window registers a downloaded font, with no restart.

## The first-run list

The first time the window or the terminal reader starts, after the language list, textweaver shows the optional components once, with nothing chosen:

- Each item says whether it is chosen, in words, then what it is, what it is for, its size, and its license: "Not chosen: Whisper base.en, English dictation, for dictation, 79.3 MB, license MIT, unconfirmed".
- Space (or Enter) chooses an item or takes it back, and says "Chosen" or "Not chosen".
- "Download the chosen ones" downloads them, one after another, with progress said every 10 percent.
- "Skip for now" or Escape closes the list. Nothing is downloaded.

It is the same list as Manage optional components, shown once (the `[components] chooser_shown` setting remembers). Tools, Ask again about first-run choices shows it again at the next start. `tw` never shows it.

## From the command line

```text
tw components list
tw components list --json
tw components download whisper-base.en
tw components download whisper-base.en --yes
tw components download ffmpeg
tw components verify
tw components verify ocr-ocrs
tw components remove lexend
tw components install whisper-base.en D:\Downloads\whisper-base.en.zip
Get-Clipboard | tw components sign-in
tw components forget-token
```

- `list` prints one line per component: its title, whether it is installed, its size, its license, what it is for, and its id, then how many are installed.
- `download ID` says what it downloads, from where, its size, license, and credit, and asks; `--yes` answers for you. Progress is printed every 10 percent on standard error.
- `verify` checks every installed component's files by size and SHA-256 (or one, by its id) and fails when a file does not check out.
- `remove ID` asks (or `--yes`), prints each path it removes, then removes the component's own files and nothing else.
- `install ID PATH` installs from a zip or a folder.
- For an archive (`unpack`), `download` and `install` unpack it into the component's folder. For an installer, they then say its name, version, and license note and ask before launching it (`--yes` answers both questions); the system's own prompt follows.
- `sign-in` keeps a GitHub token for a private components source in the system credential store. It reads the token from standard input and refuses to read it from a terminal, where it would show on the screen and be read aloud; pipe it in instead, as above (on Linux, `xclip -o -selection clipboard | tw components sign-in`). A signed-in GitHub CLI needs no token here.
- `forget-token` removes that token, and says whether the GitHub CLI's own sign-in is still in use.

`tw dictate download` downloads the dictation model chosen in the settings, and `tw dictate` offers it when it is missing. `tw ocr download` downloads an OCR model set the same way as `tw components download ocr-ocrs`: the mirror first, every file checked. `tw info` ends with how many optional components are installed.

## Installing from a file

For a computer without internet, a managed laptop, or an accommodation office that prepares machines: download the files once, then install them on each computer from a zip file or a folder (a memory stick, a network share, a clone of a mirror).

- In Manage optional components, choose the component, then Install from a zip file… or Install from a folder…, and choose it in the file browser.
- On the command line, `tw components install ID PATH`.

The files are found by name anywhere in the zip or folder (for the Whisper models, `encoder_model_int8.onnx`, `decoder_model_merged_int8.onnx`, and `tokenizer.json`, at the top or in an `onnx` folder). A file renamed since it was downloaded is found by its size and SHA-256. Each file is checked against the same pins as a download before anything is installed:

- A file with the wrong size or hash is refused: "Not installed: a file did not match." Nothing is installed.
- A missing file is named: "Not installed: a file is missing."
- Files the component does not pin are left out, each named with the reason ("not one of this component's files").
- A zip member whose name has a parent step (`..`), an absolute path, or an odd character is refused and never written.

A folder that already holds the right files is adopted as it is: Download and Install check the files that are there and fetch only what is missing.

## Helper programs

Three features rest on programs that textweaver runs but does not contain: FFmpeg writes M4B audiobooks and MP4 video, liblouis translates braille in both directions, and Pandoc reads the document formats for which textweaver has no reader of its own. Each is a mature project with its own license and its own release schedule, and bundling all three would roughly triple the size of every package for features that many readers never use. textweaver therefore treats each as an optional component: it knows a pinned release of the program, the address of the publisher's own build, the file's size and SHA-256, and the program's license, and it fetches that build only when you agree to it.

**The pinned releases.** Each pin names one file from the publisher's official release page on GitHub:

- **FFmpeg 9.0.2**, from BtbN's FFmpeg builds, the LGPL variant (Windows x64 and ARM64 as a zip; Linux x86-64 and ARM64 as a static `tar.xz`). The LGPL build leaves out the GPL-only encoders, which textweaver does not use.
- **liblouis 3.39.0**, from the liblouis project's releases (Windows x64 as a zip). The library is LGPL-2.1-or-later; the `lou_translate` program textweaver runs is GPL-3.0-or-later.
- **Pandoc 3.12.1**, from John MacFarlane's releases (Windows x86-64 as a zip; Linux x86-64 and ARM64 as a tarball; macOS as the publisher's installer package, for Intel and Apple processors).

None of the three publishes a separate checksum file for these builds, so each SHA-256 is the digest GitHub records for the release file, read from the release page when the pin was made. A file that does not match it is refused like any other component file. BtbN replaces its daily builds after a while, so the FFmpeg pin moves with each textweaver release; a components source or a mirror that keeps the pinned file continues to serve it.

**Where they go, and how textweaver finds them.** An archive is unpacked into its own folder inside the components folder (`components/ffmpeg`, `components/liblouis`, `components/pandoc`), and a small receipt replaces the archive, recording its SHA-256 and every file it unpacked; Remove takes away exactly those files. Every lookup searches the components folder first, then the environment variable that names the program (`TEXTWEAVER_FFMPEG`, `TEXTWEAVER_PANDOC`), then the PATH. A program you installed some other way, through a package manager or by hand, is therefore found without textweaver ever asking. For a copy of liblouis in the components folder, textweaver also tells `lou_translate` where its translation tables are, unless `LOUIS_TABLEPATH` is already set.

**Where there is no build.** liblouis publishes no binaries for Linux or macOS, and FFmpeg's builders publish none for macOS, so on those systems there is no component to fetch. Instead, the moment of need names the system's own package command, for you to copy into a terminal: `sudo apt install liblouis-bin` on Debian and Ubuntu, `sudo dnf install liblouis-utils` on Fedora, `sudo pacman -S liblouis` on Arch, and `brew install liblouis` or `brew install ffmpeg` with Homebrew on macOS. textweaver reads `/etc/os-release` to choose the Linux command; on a distribution it does not recognize, it says only what is missing. Once the package is installed, textweaver finds the program on the PATH the next time the feature runs.

**An installer, never silent.** The macOS Pandoc package is an installer rather than an archive. textweaver downloads and checks it like any other file, then says its name, version, and license note and asks before starting it: "Launch installer: Pandoc 3.12.1, for more document formats, version 3.12.1, license GPL-2.0-or-later? The system asks next. y or n". A yes opens it with the system, whose own permission prompt follows; a no starts nothing, keeps the checked installer for later, and is remembered until textweaver starts again.

## Your own components source

If you keep components of your own (licensed engines, helper programs, offline copies of the models), point textweaver at them: Settings, Optional components, Components source (`[components] source`). It takes either of these:

- **A folder on this computer**, such as a clone of your repository or a memory stick. No sign-in is needed.
- **A GitHub repository**, written `owner/name`. Its releases hold the files, one release per component, and a release tagged `manifest` holds the list. A public repository needs no sign-in; a private one does (see "Signing in to a private repository" below).

The source is tried first, before a mirror and before each file's public address. Its list, `components.toml`, says for each component its name, version, platform (`windows`, `linux`, `macos` or `any`), files with size and SHA-256, license note, and what installing it does (`place`, `unpack` or `installer`). In a folder the list is `components.toml` at the top, or `manifest/components.toml`; each component's files are in a folder named after it, `<id>-<version>` when it has a version (`ffmpeg-9.0.2`), else `<id>`.

Only the components for your computer's platform are listed. Every file is checked against its SHA-256: a file that does not match is refused, "Not installed: a file did not match. Get it again, or check the source.", and nothing is installed. Fields textweaver does not know yet are ignored, so a list written for a later version still loads.

Components from a source go into the components folder, `components` in the data folder. textweaver looks there first for FFmpeg, Pandoc, liblouis, the eSpeak NG library and the Eloquence engine library, before the environment variables and the usual install places.

### What installing does

Each entry in the list says what installing it means, in its `action` field, and textweaver carries out exactly that and nothing more.

- **`place`** (the default) puts the checked files into the component's folder as they are. This suits models, voices, fonts, and a single library file such as an engine's `eci.dll`, which the engine discovery then finds in the components folder.
- **`unpack`** treats each file as an archive (`.zip`, `.tar.gz`, or `.tar.xz`) and unpacks it into the component's folder. Every member must have a plain name; a member with a parent step, an absolute path, an odd character, or a link is left out and named with the reason, so an archive can never write outside its folder. A receipt, `<archive>.unpacked`, replaces the archive: it records the archive's SHA-256 and each unpacked file, so the component counts as installed without the archive, a second download fetches nothing, and Remove takes away only what the archive put there. Zip archives and gzip tarballs are read by textweaver itself; an xz tarball is unpacked with the system's `tar`.
- **`installer`** keeps the checked installer (an `.msi`, a setup program, a `.pkg`) and offers to start it, saying its name, version, and the list's license note first. The system's own prompt follows a yes; a no starts nothing and is remembered for the session. Manage optional components lists such a component as "installer downloaded", and choosing Download on it again offers the launch.

`tw components list --json` reports each component's `action`, `version`, and `platform`, and its `status` (`installer-downloaded` for a downloaded installer).

**Supplying the helper programs.** Your source may list `ffmpeg`, `liblouis`, or `pandoc` itself, with its own version and files. Such an entry takes the place of the public helper of the same id, so your own copy is used first, before any public download. No other built-in component can be replaced this way, and a mirror cannot replace any.

### Generating the list from a table

If you already keep a table of your files, with each file's name, size, and SHA-256, the list is best generated from that table by a short script kept beside it, so the two never disagree. The format is a proposal and may grow; fields textweaver does not know are ignored, and a list that loaded before keeps loading.

### Signing in to a private repository

A private repository answers only to a GitHub token that may read its contents. textweaver finds one by itself where it can, and asks for one only when it cannot, so most readers never see the question.

- **With the GitHub CLI.** When `gh` is installed and signed in (`gh auth login`), textweaver asks it for its token (`gh auth token`) when the token is needed, and uses it for that session. The token remains the CLI's: textweaver keeps no copy, and signing out of the CLI (`gh auth logout`) ends its use.
- **Otherwise, asked once.** When the source is a GitHub repository and no token is found, Manage optional components first asks for one: "GitHub token for your components source, Enter skips." What you type is shown as stars, is never spoken back or kept in the prompt's history, and is kept only in the system credential store: Windows Credential Manager, the macOS Keychain, or the Secret Service on Linux (GNOME Keyring, KWallet). Press Enter on an empty prompt to go on without one, as a public repository needs none; the question is not asked again in that session. From the command line, `tw components sign-in` keeps a token the same way.
- **A folder needs nothing.** A clone or a memory stick is read as it is, with no sign-in.

A fine-grained token limited to the one repository, with read-only access to its contents, is enough and is the safest choice.

**Forgetting the token.** The command Forget the GitHub token (in the Tools menu and the command palette, `forget_github_token`), or `tw components forget-token`, removes the token from the credential store. The next time Manage optional components needs one, it asks again. The GitHub CLI's sign-in is not touched, and textweaver says so when it is still in use.

**Where the token goes.** For a private repository, textweaver fetches each file through the GitHub REST API, as GitHub documents for release assets: it reads the release by its tag, finds the file in the release's list of assets, and asks the API for it, which answers with a redirect to a short-lived signed download. The token is sent in the `Authorization` header of the requests to `api.github.com` and to nothing else. The signed download receives no token, and neither does a mirror, a public address, or any address a list names outside the API. The token never appears in `settings.toml`, in `textweaver.log` (even at the trace level, where the HTTP library's own records of each request are left out for this reason), in anything said or shown, or in any file textweaver writes. A test enforces this: after a signed-in download from a fake GitHub on the test computer, it searches the log, the settings, every message, and every file the test made for any piece of the token.

## A mirror

A mirror is a place tried before the public sources: a web address, or a folder on this computer. Set it in the settings (Optional components, Components mirror, `[components] mirror`) or with the `TEXTWEAVER_COMPONENTS_MIRROR` environment variable, which wins over the setting.

A mirror holds each component's files under its id:

```text
<mirror>/whisper-base.en/encoder_model_int8.onnx
<mirror>/whisper-base.en/decoder_model_merged_int8.onnx
<mirror>/whisper-base.en/tokenizer.json
<mirror>/ocr-ocrs/text-detection.onnx
<mirror>/lexend/Lexend-Regular.ttf
```

On GitHub, that is one release per component, tagged with the component's id: `https://github.com/OWNER/REPOSITORY/releases/download` is then the mirror's address. A folder works the same way: a clone of a mirror, or a memory stick.

The pins are the contract, not the addresses. A file from a mirror is checked against the same size and SHA-256 as one from its public source, and a file that does not match is refused and the public source is tried next. A mirror is a convenience for components with a public source, never a requirement.

### The mirror's own components

A mirror may list components of its own, in `<mirror>/manifest/components.toml` (on GitHub, a release tagged `manifest`). textweaver reads it only when a mirror is set, and adds its components to Manage optional components and `tw components list`. The format is the registry's:

```toml
format = 1

[[component]]
id = "example-voice-pack"          # a plain name; its files are under <mirror>/example-voice-pack/
title = "Example voice pack"
license = "CC0-1.0"
credit = "Who made it, and where it comes from"
features = ["voice"]               # what needs it: dictation, ocr, reading-font, or voice
folder = "components/example-voice-pack"   # where it is installed, under the data folder; this is the default

[[component.file]]
name = "example-voice.onnx"
size = 1234
sha256 = "the file's SHA-256, 64 hex digits"
url = "https://example.org/example-voice.onnx"   # optional public address
```

The rules, checked before anything in the list is used:

- Every id, file name, and folder part is a plain name: letters, digits, `-`, `_`, and `.`.
- Every size is more than zero, and every hash is 64 hex digits.
- A public address, when given, starts with `https://`.
- A mirror may add components, never change a built-in one: an entry with a built-in id (such as `lexend`) is refused, and the built-in pins stay.

A refused entry is named, with the reason, in the log and by `tw components list`. The fixture `fixtures/w8a-d/mirror/manifest/components.toml` is an example with made-up components only.

### Private mirrors and credentials

textweaver never writes a password or token into the settings, a file, or a manifest. A mirror is read without credentials: a public web address or a folder. A private repository is signed in to only as the components source (see "Signing in to a private repository" above), with the token kept in the system credential store; to use one as a mirror, clone it with your own login and set the clone as a folder mirror, or use Install from a folder.

Licensing: a private mirror for your own use is a copy for yourself. The moment other people download from it, it is redistribution: each component's license and notice must go with it. The current components' licenses (MIT, Apache-2.0, the SIL Open Font License, and CC BY-SA) all allow that, with attribution.

## Privacy

Every request carries the same neutral User-Agent, `textweaver-research (+https://github.com/leavesofgrass/textweaver)`, and nothing about you or your computer. A GitHub token, when a private source needs one, goes only to GitHub's API, as described above. Nothing is requested before you say yes, with one exception you choose: when you set a mirror, textweaver reads the mirror's list of components from it. The data folder is the only place components are written. textweaver itself installs nothing system-wide; the one program it ever starts is an installer you said yes to, after hearing its name, version, and license note, and that installer then asks for the system's own permission.

## For developers

One crate, `textweaver-components`, does every download: one pin type (size and SHA-256, or git's blob SHA-1 for Piper's small files), a `.part` file in a staging folder beside the component's folder, the check, then a rename; files that already check out are kept; a stopped download resumes with a `Range` request; progress through a callback with a cancel flag; and a lock, so two downloads of one component never run at once. HTTP is its `download` feature, so the lean reader links no HTTP client. The OCR models, Lexend, Piper voices, and the Whisper models all go through it.

The registry is `textweaver_app::components::Registry`. A test in the app enforces it, as star's dependency registry was enforced: no other crate opens HTTP for files, the downloader is called only from known sites whose components are registered with the same pins, and with an empty data folder nothing is reported installed. Tests use the fake fetcher (`textweaver_components::fake::FakeFetcher`), which records every request: a test fails if anything is fetched before a yes.

## See also

- [Dictation](dictation.md)
- [Reading aids](reading-aids.md): Lexend and the other reading fonts.
- [Settings](settings.md): `[components]` and `[dictation]`.
- `THIRD-PARTY-NOTICES.md`, in each package: the components' licenses and credits.
