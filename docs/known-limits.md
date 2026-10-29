# Known limits

textweaver is in beta. This page states what it does not do yet, or does not do well, so that you can decide what to rely on. Read it before you use textweaver for something important. Where a limit has a workaround or a planned fix, the next step is given with it.

## What has been tested, and what has not

- **Windows with a screen reader** is the combination that has been tested by ear: NVDA and JAWS, and a 40-cell Braille display. Two listening sessions went through the graphical version.
- **macOS and Linux** packages are built and checked automatically on every release. VoiceOver and Orca have had some basic testing, but not every release is tested with them by ear. More extensive testing is planned.
- **Other Braille displays and cell widths** have not been tried.
- **Settings marked "not yet verified"** in [the screen reader guide](screen-readers.md) have not been tried with a screen reader.

See [the accessibility statement](accessibility.md) for the full record.

## The program

- **Not code-signed.** Windows warns the first time you start it. On a Mac you remove the quarantine flag once. See [Installing textweaver](install.md).
- **No native Windows ARM64 package is planned.** On a Windows computer with an ARM processor, use the x86-64 package: Windows runs it under emulation.
- **Updates are your choice.** textweaver checks for a new release only if you say yes, at most once a day, and never downloads without asking. You can also update with `tw update`, the scripts, or a new download ([Updates](updates.md)). Packages are not code-signed or notarized, so each is checked against the release checksums instead.
- **On Windows, use `tw` in a terminal, not `textweaver.exe --tw`.** `textweaver.exe` is a windowed program, so a terminal does not wait for it: `textweaver.exe --tw` runs the terminal reader in that terminal, but the prompt returns at once and competes with it for the keyboard. `tw.exe` starts it and waits, as a terminal expects ([The command line](command-line.md#one-program)).
- **No guided first-run tour.** The [quick start](quickstart.md) is a document you open and read.
- **Some translations are not reviewed.** The interface has six languages. Native speakers have checked English and Spanish. German, French, Portuguese, and Arabic have not been checked yet. Feedback on any translation is welcome: use Help, Report a problem, or the [issue tracker](https://github.com/leavesofgrass/textweaver/issues).
- **Settings, keys, and file formats may still change** between releases. Read the [changelog](https://github.com/leavesofgrass/textweaver/blob/main/CHANGELOG.md) before you update.

## The graphical version

These limits apply to the textweaver app (`textweaver`). The terminal reader does not have them, unless stated.

- **Scrolling stops near the cursor in a very long document.** `j` and Shift+J scroll within the text the app holds around the cursor, about 120,000 characters; moving the cursor brings in the rest ([Scrolling and line numbers](gui.md#scrolling-and-line-numbers)).

## Braille

- **The Braille view is for looking, not for a Braille display.** It draws braille dots on the screen in the textweaver app, for learning and teaching, and sends nothing to a display or to a screen reader. In the terminal reader it shows Unicode braille in self-voicing mode only.
- **Contracted braille needs liblouis.** Without it, the Braille view and BRF files from the reader show uncontracted braille and say so ([The Braille view](reading-aids.md#the-braille-view)).
- **The print line under contracted braille is approximate.** liblouis does not say which print each cell came from, so textweaver pairs them word by word; a contraction that takes shape only at the end of a word is shown over the whole word.

## Live documents

- **A live source only receives.** It subscribes and never publishes, so a live document cannot change what the broker holds, and a message cannot delete an entry; the sender publishes the whole log again without it ([Running the broker for a live log](live-broker.md#example-messages)). Sending is separate: `tw publish`, or Publish a check-in and Publish this document live in the app.
- **textweaver's own broker is small on purpose.** `tw broker` speaks MQTT 3.1.1 only (QoS 2 is passed on at QoS 1), keeps retained messages in memory only, so they are gone after a restart, sends no last will messages, and has no WebSocket listener or bridge. A reader that falls 1,024 messages behind is disconnected. Accounts are read when the broker starts, so a change needs a restart ([textweaver's own broker](live-broker.md#textweavers-own-broker)).
- **The broker inside the app has no account editor.** Start your broker reads `broker.toml` as `tw broker` does; accounts are added and removed with `tw broker account`, and read when the broker starts. With a join code, the session has a key of its own, so readers who pinned the broker's usual key cannot connect until the join code is turned off.
- **A shared document is one document per source.** A source showing a shared document leaves out log entries that arrive on it, and says so once. A document is shared again only when it is saved in edit mode; sharing follows one document at a time, and the broker inside the app keeps retained messages in memory, so a restarted broker has no revision until the next save.
- **MQTT 3.1.1 only.** A broker that accepts only MQTT 5 is refused in words. Mosquitto and most brokers also speak 3.1.1. The connection is plain TCP, with or without TLS; WebSockets and client certificates are not supported.
- **"System" TLS means the bundled authorities,** Mozilla's list that textweaver carries, not your operating system's certificate store. A broker with a certificate from a company or school authority needs `tls = "ca_file"` with that authority's file. Certificate checking cannot be turned off.
- **One live source is open at a time,** and a log always opens as a list; switch to the table with `Alt+Shift+F12`.
- **Sources are written by hand for anything beyond the basics.** The live source form holds the name, host, encryption, port, topic, user name, and view. Field mapping (labels, units, decimals, value words, order, and which changes are announced), `id_field`, and `stale_after_seconds` are edited in `settings.toml` or come in a live source file, apart from the per-field show and speak choices, which have commands. Editing a source in the form keeps them.
- **In the textweaver app, the live source form's field count can lag** when a choice adds or removes the CA file field: the label read with the field ("Encryption, 3 of 7") keeps the old count until you move to another field. What is said when the choice changes is current.
- **A log holds 20,000 entries** and the values view 2,000 fields; more are left out, and textweaver says so. Messages over 256 KiB, and binary messages, are left out.
- **Not yet checked by ear.** Live announcements on a Braille display, and NVDA's and JAWS's message times with a live log, have been tested by computer but not on a Mantis Q40 at a real net ([Using textweaver with a screen reader](screen-readers.md)).
- **There is no webcam scanning for join QR codes.** Scan the code with a phone, or save a photo of it and use Join from a QR image.
- **A join code lasts one broker session.** The session's key is held in memory, so the code stops working when the broker stops, and a new run makes a new code. A QR code, like the code, lets anyone who sees it join until then. An image with a side over 16,384 pixels is refused when read from the clipboard.
- **A quiet topic and a forbidden topic look the same** to textweaver, since Mosquitto accepts a subscription its access list denies. After `quiet_hint_seconds` it says so once, in words.

## Obsidian notes and vaults

- **Obsidian Bases are not supported, and plugins are not run.** A Bases file is not opened as a database, and a plugin's query block is read as the text it is. Dataview `key::` fields are read by the `tw vault` commands (import and `properties`), not by the reader.
- **A vault is the nearest folder holding `.obsidian`.** A note in no vault resolves links only in its own folder and below, and a link never leaves its vault ([Obsidian vaults](vault.md#read-vault-notes-in-textweaver)).
- **The vault index is bounded:** at most 20,000 notes, 4 MB a note, and 256 MB in all. A canvas is read up to 5,000 cards and 20,000 connections.

## Reading and speech

- **Cloud voices, Coqui, Festival, and Qt Speech** are not offered.
- **Eloquence and DECtalk** need your own licensed copy. textweaver does not include them.
- **DAISY audio timing in MP3 books is untested on real books.** The playback of a talking book's recorded narration is tested with built files and with WAV clips. How accurately the highlight follows the clips in a real DAISY book whose audio is MP3 has not been checked. If the highlight drifts, set **Book audio** to **speech only** ([Talking books](reading.md#talking-books-the-recorded-narration)) and report the book's source and format.
- **Source code** opens as plain text. It is not read as a structured document.
- **CW sends messages in Morse code too.** With CW as the engine, textweaver's own messages are sent in Morse code; use a screen reader for messages in speech. Repeat sentence slower does not slow CW down ([CW](cw.md)).
- **CW keying in the terminal reader is MIDI only.** Keyboard key adapters (VBand, Vail, the TinyMIDI in keyboard mode) need each key's release, which most terminals never report; they work in the textweaver app. The sidetone plays only when the speech engine plays its own audio (not with the silent engine), and the keying adapters were tested from their manuals, not yet with the hardware ([CW keying](cw.md#key-morse-code-into-a-document)).

## Documents

- **Translating a document** is not offered.
- **RAR archives** do not open. ZIP, TAR, and 7z do. Extract a RAR archive first; the message says so.
- **Scanned pages** are read by OCR, and OCR makes mistakes. Check anything important against the original.

## Output

- **A cover image for audiobooks** is not added.
- **AAC audio** is not written. Ogg Vorbis, Opus, MP3, FLAC, WAV, and M4B are.

## Study tools

- **Study cards are scheduled with SM-2 only.** Cards are made from notes, highlights, and headings, studied, graded in words, and brought back when due ([Study with cards](notes.md#study-with-cards)). A card counts as due by the clock, from half a day before its interval ends, not by the calendar day in your time zone. FSRS scheduling is dropped on purpose.
- **Anki packages carry text and grades only.** Importing an `.apkg` skips cloze notes and leaves out pictures and sound, and textweaver works out each card's schedule again from its grades, so intervals can differ from Anki's ([Take cards to Anki and back](notes.md#take-cards-to-anki-and-back)). Fill-in-the-blank cards go to Anki as Basic cards, not as Anki clozes.
- **AnkiConnect sync is dropped on purpose.** It needs Anki running with an add-on and a local network port; cards go to Anki and back as `.apkg` files instead.
- **Knowledge graphs from notes are not drawn.** Links between notes are lists instead: each note's links, "What links here", and `tw notes links` ([notes guide](notes.md#links-between-notes)). The graph exports to files that other tools draw, with a Markdown list as their text equivalent ([Export the knowledge graph](notes.md#export-the-knowledge-graph)).

## Documentation

- **If a guide contradicts what the program does, the program is right.** Please report the page, with Help, Report a problem.

## What this page does not promise

textweaver does not claim to improve reading speed, comprehension, or comfort for any condition. The reading aids are options to try. Keep the ones that help you.

## See also

- [Roadmap](roadmap.md)
- [star features not yet planned](star-gaps.md)
- [Troubleshooting](troubleshooting.md)
