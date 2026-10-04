# Known limits

textweaver is in alpha. This page says plainly what it does not do yet, or does not do well. Read it before you rely on textweaver for something important.

## What is tested, and what is not

- **Windows with a screen reader** is the system that has been listened to: NVDA and JAWS, and a 40-cell Braille display. Two listening sessions went through the window.
- **macOS and Linux** packages are built and checked by computer on every release. No one has listened to them with VoiceOver or Orca yet.
- **Other Braille displays and cell widths** have not been tried.
- **Settings marked "not yet verified"** in [the screen reader guide](screen-readers.md) have not been tried by ear.

See [the accessibility statement](accessibility.md) for the full record.

## The program

- **Not code-signed.** Windows warns the first time you start it. On a Mac you remove the quarantine flag once. See [Installing textweaver](install.md).
- **No update check inside the program.** Update with the scripts, or download the new release.
- **No first-run tour.** The [quick start](quickstart.md) is a document you open and read.
- **Some translations are not reviewed.** The interface has six languages. The newest messages in the five translations wait for a native speaker's review.
- **Alpha means anything may change.** Settings, keys, and file formats can change between releases. Read the [changelog](https://github.com/leavesofgrass/textweaver/blob/main/CHANGELOG.md) before you update.

## Reading and speech

- **Cloud voices, Coqui, Festival, and Qt Speech** are not offered.
- **Eloquence and DECtalk** need your own licensed copy. textweaver does not include them.
- **Source code** opens as plain text. It is not read as a structured document.

## Documents

- **Translating a document** is not offered.
- **RAR archives** do not open. ZIP, TAR, and 7z do.
- **Scanned pages** are read by OCR, and OCR makes mistakes. Check anything important against the original.

## Output

- **A cover image for audiobooks** is not added.

## Study tools

- **Spaced-repetition review** (Anki-style) is out of scope, on purpose.
- **Knowledge graphs** from notes are not drawn.

## What this page does not promise

textweaver does not claim to improve reading speed, comprehension, or comfort for any condition. The reading aids are options to try. Keep the ones that help you.

## See also

- [Roadmap](roadmap.md)
- [star features not yet planned](star-gaps.md)
- [Troubleshooting](troubleshooting.md)
