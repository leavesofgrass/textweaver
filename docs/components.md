# Optional components

Some of what textweaver can do needs files that are not in its packages: the Whisper model for dictation, the OCR models for reading scanned pages, and the Lexend reading font. These are optional components. textweaver never downloads one on its own. It asks first, names the size and the license, and downloads only when you say yes. Saying no is always fine: the feature says what it needs, in words, and the rest of textweaver works as before.

This guide is written to be read with a screen reader. Each section starts with what you do, then explains it.

## The components

| Component | Id | Size | License | What needs it |
| --- | --- | --- | --- | --- |
| Whisper base.en, English dictation | `whisper-base.en` | 79.3 MB | MIT, unconfirmed | dictation (the default model) |
| Whisper small.en, better English dictation | `whisper-small.en` | 251 MB | MIT, unconfirmed | dictation (when chosen in the settings) |
| The ocrs text recognition models for English | `ocr-ocrs` | 12.2 MB | CC BY-SA 4.0 | reading scanned pages |
| The PaddleOCR Latin text recognition model (experimental) | `ocr-paddle-latin` | 8.0 MB | Apache-2.0 | reading scanned pages in Latin-script languages |
| The Lexend reading font | `lexend` | 206 KB | SIL Open Font License | the Lexend reading font |

Piper voices are optional components too. The voice manager lists them from Piper's catalog, with each voice's size and license; see [Speech](speech.md).

The Whisper models' license is MIT, from OpenAI. The onnx-community copies textweaver downloads declare no license of their own, so textweaver says "MIT, unconfirmed".

## When a feature needs one

When you use a feature whose component is missing, textweaver asks at that moment. For dictation, you hear:

"Dictation needs the Whisper model, 79.3 MB, license MIT, unconfirmed. Download it now? y or n"

- Press `y`: the download starts on its own. Its progress is said every 10 percent ("40 percent downloaded."), never as a spinner. Escape stops it ("Download stopped; it resumes later."), and the next download goes on from where it stopped. When it finishes you hear "Ready", and the feature starts: dictation begins at once, with no restart.
- Press `n`: nothing is downloaded. You are not asked again until textweaver starts again; the feature says "No model, so no dictation for now." Manage optional components can get it at any time.

Choosing Lexend as the reading font asks the same way. The OCR models are offered by `tw ocr download` and by Manage optional components.

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
tw components verify
tw components verify ocr-ocrs
tw components remove lexend
tw components install whisper-base.en D:\Downloads\whisper-base.en.zip
```

- `list` prints one line per component: its title, whether it is installed, its size, its license, what it is for, and its id, then how many are installed.
- `download ID` says what it downloads, from where, its size, license, and credit, and asks; `--yes` answers for you. Progress is printed every 10 percent on standard error.
- `verify` checks every installed component's files by size and SHA-256 (or one, by its id) and fails when a file does not check out.
- `remove ID` asks (or `--yes`), prints each path it removes, then removes the component's own files and nothing else.
- `install ID PATH` installs from a zip or a folder.

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

textweaver never stores a password or token: not in the settings, not in a file, not in a manifest. Today a mirror is read without credentials: a public web address or a folder.

For a private repository, the simplest way is to clone it (or download its releases) with your own login, and use the clone as a folder mirror or with Install from a folder. If textweaver is to fetch from a private repository directly in a later version, the token comes at run time from `gh auth token` (the GitHub command line, signed in) or the system's credential store, is sent only to that repository's host, and is never written anywhere.

Licensing: a private mirror for your own use is a copy for yourself. The moment other people download from it, it is redistribution: each component's license and notice must go with it. The current components' licenses (MIT, Apache-2.0, the SIL Open Font License, and CC BY-SA) all allow that, with attribution.

## Privacy

Every request carries the same neutral User-Agent, `textweaver-research (+https://github.com/leavesofgrass/textweaver)`, and nothing about you or your computer. Nothing is requested before you say yes, with one exception you choose: when you set a mirror, textweaver reads the mirror's list of components from it. The data folder is the only place components are written; nothing is installed system-wide, and no program is run.

## For developers

One crate, `textweaver-components`, does every download: one pin type (size and SHA-256, or git's blob SHA-1 for Piper's small files), a `.part` file in a staging folder beside the component's folder, the check, then a rename; files that already check out are kept; a stopped download resumes with a `Range` request; progress through a callback with a cancel flag; and a lock, so two downloads of one component never run at once. HTTP is its `download` feature, so the lean reader links no HTTP client. The OCR models, Lexend, Piper voices, and the Whisper models all go through it.

The registry is `textweaver_app::components::Registry`. A test in the app enforces it, as star's dependency registry was enforced: no other crate opens HTTP for files, the downloader is called only from known sites whose components are registered with the same pins, and with an empty data folder nothing is reported installed. Tests use the fake fetcher (`textweaver_components::fake::FakeFetcher`), which records every request: a test fails if anything is fetched before a yes.

## See also

- [Dictation](dictation.md)
- [Reading aids](reading-aids.md): Lexend and the other reading fonts.
- [Settings](settings.md): `[components]` and `[dictation]`.
- `THIRD-PARTY-NOTICES.md`, in each package: the components' licenses and credits.
