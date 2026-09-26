# textweaver roadmap

Written on Saturday, September 26, 2026, from four planning reviews of `main` at `11f87d0`, after all the Wave 2 work was merged. The four reviews were:

1. **Star lessons.** Star's history in the Obsidian wiki, mapped to textweaver.
2. **Speed and stability.** Measured with `cargo xtask bench` and by timing the command-line tools.
3. **Markdown reading and authoring.** Realistic student sessions, and how textweaver coexists with screen readers.
4. **Architecture, CI, releases, and GUI readiness.**

The goal is Jon's: terminal-first Markdown reading and authoring, with speech and word highlighting that never stall, drift, or lose your place. It should be fast on large files, work the same on Windows, macOS, and Linux, and have a GUI for the many users who want one.

Sizes:
- **S**: under a day.
- **M**: a few days.
- **L**: one to two weeks.

## Where we are

- **Reading speed is interactive on big files.**
  - 10 MB of Markdown: speech starts in about 0.2 s. A sentence, paragraph, or heading step while reading takes about 2 to 5 ms.
  - Typing costs under 0.13 ms per key.
  - `tw --version` starts in 15 ms.
- **Formats.** textweaver reads text, Markdown, HTML, PDF, EPUB, and DOCX natively. It writes HTML, EPUB, DOCX, braille (BRF), and tagged PDF.
- **Speech.** Eloquence (ECI), SAPI5, Apple voices, espeak-ng, speech-dispatcher, DECtalk (detect-only), and Omnivox. Word highlighting is exact where the engine reports words. Crash recovery and a stall watchdog are in place.
- **Tests.** 1,572 pass natively on Windows and 1,581 on Linux, all features. The Wave 2 audit's worst problems are fixed.
- **What still hurts.**
  - A few stability holes: a crash-recovery loop, a silent speech-thread death, and engine hosts that can outlive textweaver.
  - Authoring gaps: no clipboard, no structure while editing, and citations only from the command line.
  - Settings that are stored but not used.
  - Missing licence notices in the packages.
  - CI that goes red after merges.

## Phase 1: quick wins and safety (this week)

Each of these is under about half a day. Together they remove the known ways to lose work, go silent, or overwrite a file.

### Never lose work, never overwrite

- **Save As.**
  - Ask before overwriting an existing file.
  - Suggest the new file's name from the first heading or the front matter title, not `document.md`.
- **Panic hook and signals.**
  - On a crash, a closed terminal, Ctrl+C, SIGTERM, or SIGHUP: save the position, write a recovery snapshot, and restore the terminal (including bracketed paste).
- **Saving and snapshots.**
  - Retry the save rename on Windows when another program has the file open (Obsidian, OneDrive, antivirus).
  - When writing a recovery snapshot fails, back off and announce it once. Today it retries about every 40 ms.
- **Bookmarks.** Do not announce "Bookmark set" when saving the bookmark failed.
- **Anchored positions.** Store a short context string with each position and bookmark now, so later work can find them again after outside edits (Phase 2).

### Speech never loops, never dies silently

- **Crash-recovery counter.** Reset it only when reading gets past the resume point, and cap restarts per utterance. Today an engine that crashes on the same text restarts forever.
- **Speech thread.** Catch a panic on the speech thread, report it, and let the app restart speech.
- **Engine hosts.**
  - A host exits when textweaver's end of the pipe closes.
  - On Windows, hosts run in a Job Object that kills them with the parent. On Linux they use a parent-death signal.
  - Kill a host at once when it has already died, instead of waiting 500 ms.
- **Host stderr.** Keep reading it even after a line that is not UTF-8. Today one bad line can later kill the host.
- **SAPI input.** Map control characters and NUL to spaces before synthesis.
- **Big selections and deletions.** Speak a summary ("3,412 characters selected, from … to …"), not the whole text. The terminal reader's announcer keeps a small ring of messages instead of every message.

### Settings that do nothing

Star's lesson: a stored setting must work.
- Use `highlight.color` and `highlight.sentence_color` in the themes, with a contrast warning.
- Use `speech.favorite_voices` in the voice chooser.
- Add a test that fails when a setting is stored but never read.

### Authoring quick wins

- Keys 1 to 6 jump to the next heading of that level; Shift plus the number goes back.
- A word count command, also added to "say position".
- A key that says the link address at the cursor.
- **Enter in lists.** Enter continues a list (the next bullet or number). Enter on an empty item ends the list.
- **Tables.** Tab and Shift+Tab move between table cells in edit mode and say the column header.
- **Speaking structure.**
  - Caret and Speech Cursor moves say the structure first: "heading level 2", "list item", "row 2".
  - Typing echo says `## Methods` as "Heading level 2, Methods".
- **Commands that work everywhere.**
  - A command that cycles typing echo (none, characters, words, both) and saves it.
  - An Add note chord that also works in edit mode.
- **Paste.** Speak the first words pasted, not only the character count.
- **Copy.** Copy to the clipboard with OSC 52 (works over SSH, as in Star), with a native clipboard as the fallback.
- **Prompts and lists.**
  - Tab completes file paths in the Open, Save As, and Insert image prompts.
  - First-letter jumps in lists. Accept s, d, and c in the Save, Discard, Cancel list.
- **Key hints line.** Show only keys that work in the current mode and with F9's setting.

### Robust loaders

- **Malformed files.**
  - Clamp DOCX and PDF list counters and label lengths.
  - Limit nesting depth in the HTML, EPUB, and DOCX walkers, so a malformed file cannot crash a whole `tw convert` batch.
- **Pandoc.**
  - Read Pandoc's stderr on its own thread, which removes a deadlock, and give it a timeout.
  - Use one Pandoc path, the formats loader, with `--sandbox`.
- **Markdown options.**
  - Turn on the reader's options for wiki links, GFM alerts, math, and heading attributes.
  - Add markers for strikethrough and horizontal rules, so they can be heard.
- **Command-line tools.**
  - `tw info` no longer segments every word of a 10 MB file (1 s today).
  - `tw backends` runs discovery once.
  - `tw backends`, `tw voices`, and `tw speak` load the settings.

### CI, releases, and notices

- **Licence notices in every package.**
  - The OFL font licences and the SCOWL copyright go in now.
  - `THIRD-PARTY-NOTICES` is generated with `cargo-about`. It covers the Rust crates, the CSL styles (CC BY-SA), and the Adobe AFM metrics.
- **The GUI job on macOS** has failed since the fonts merge (font registration). Fix it.
- **Workflows.**
  - Fix the PowerShell lint step, which has never run.
  - Add cancel-in-progress groups and job timeouts to every workflow.
  - Add `cargo xtask keyboard --check` to CI.
  - Bump actions past Node 20.
- **Flaky tests.** Fix the timing-dependent ones, listed in the speed and stability review. Wait for signals, not sleeps.
- **Dependencies.**
  - Remove unused workspace dependencies and empty features.
  - Turn off `lopdf` and `wxdragon` default features.
  - Retarget or delete `apple.yml`.
- **Housekeeping.**
  - Add `/target-*/` to `.gitignore`.
  - Prune merged branches.
- **Merge gate.** Merge through pull requests with required checks, or at least run `scripts/dev-check` and the Docker check before pushing to main.

## Phase 2: reliability and authoring depth (next two to three weeks)

### Stability

- **Nothing blocks the input or speech threads.**
  - Saves, autosave, position saves, and the disk-change check move to a writer thread.
  - Voices are listed once in the background and cached. Alt+V never waits.
  - Engine host start-up moves into `poll`, so Stop and Pause are never stuck behind a starting host.
- **Audio device recovery.** Recover when the audio device goes away (for example Bluetooth) outside a reading.
- **Non-blocking backends.** speechd uses non-blocking calls. DECtalk streams its audio as it synthesizes.
- **Editing correctness.**
  - Backspace deletes a whole character (grapheme), not one code point.
  - Undo history is capped.
- **Find.** Find no longer copies the whole document and caps its hits. Edit-mode Replace stops allocating per character.
- **Positions that survive outside edits.** Find each position, bookmark, and note again from its stored context and content hash when the file changed in Obsidian, git, or elsewhere.

### Authoring

- **Structure while editing.**
  - Re-parse the buffer when typing pauses, and keep markers in source positions. Heading, list, link, and table navigation then work while writing.
  - This also makes entering edit mode fast and light: today it takes 150 to 350 ms and 213 MB of peak memory on 10 MB.
- **Outline list** (Alt+O). The headings, in reading and editing, with type-to-filter.
- **Copy, cut, select all, and delete word** in edit mode.
- **Citations while writing,** with `textweaver-cite`'s insert API:
  - insert a citation (Alt+C, a filtered picker, then a page prompt);
  - add a reference by DOI or ISBN (Alt+Shift+D);
  - "insert bibliography here" and "check citations" from the palette.
- **Citations and math in output.**
  - Format citations and append a References section in HTML, PDF, DOCX, and EPUB (`--bibliography`, `--style`).
  - Math reaches the PDF and DOCX writers through the Markdown loader.
- **Export and preview from inside the reader.**
  - One palette command per format, working from the live buffer in edit mode.
  - "Preview in browser" writes an HTML page with MathML and refreshes it on each save.
  - "Listen to the rendered text" reads the essay as it will render, without leaving edit mode.
- **Spell check** on the built-in SCOWL list:
  - next and previous misspelling;
  - say the word, then spell it;
  - suggestions and a personal word list;
  - a count on save.
- **Tables in reading.** Move by row and cell, and hear the column header.
- **Links and footnotes.** Follow local Markdown links, and jump from a footnote to its note and back.
- **Settings at run time.** Change verbosity and punctuation while running.
- **Notes.**
  - Hear that a note is here while reading.
  - Export notes as a Markdown study sheet grouped by heading.
- **Find and replace.** One at a time, with case and whole-word options.
- **Templates.** A new document from a template, with front matter and a References heading.

### Screen reader coexistence

- **An accessibility mode setting,** with three choices:
  - **self-voicing**: textweaver speaks everything;
  - **screen-reader**: textweaver is silent;
  - **hybrid**: textweaver voices continuous reading, math, and tables. The screen reader handles typing echo, caret echo, and messages through the status line.
- **First run.** On Windows, detect a running screen reader and offer hybrid.
- **Quieter screen.**
  - A "quiet screen while reading" option.
  - A cursor setting: follow focus, or park on the status line so "read current line" repeats the last message.
- **Keymap preset.** An optional preset matching screen reader habits: h for heading, 1 to 6 for heading levels, l for list.
- **The guide.** `docs/screen-readers.md` gives NVDA and JAWS settings for use with textweaver. Each setting is marked to verify on Jon's machine.

### CI and quality gates

- **Benchmark gate.** `cargo xtask bench --quick` on every pull request, compared with main's own numbers on the same runner. It fails above about twice the baseline, and it gates on peak memory and allocations, which do not vary run to run.
- **Startup timings.** `tw --version`, `tw text`, `tw info`, and `tw backends`.
- **Fuzzing.** Nightly fuzz targets (cargo-fuzz) for the Markdown, HTML, EPUB, DOCX, and PDF loaders, the engine-host frame decoder, the settings and keymap files, and the state JSON.
- **Other nightly jobs.**
  - Property tests for the loaders.
  - AddressSanitizer for the FFI crates.
  - Miri for core, text, and the protocol code.
  - Release-mode tests.
  - An MSRV (Rust 1.92) check.
  - A Docker job.
- **Soak test.** Read the 10 MB corpus to the end with random navigation, edits, rate changes, and host kills. Memory must stay level and no host process may be left over.
- **More coverage.**
  - Build and test DECtalk's 32-bit host on i686.
  - Run the real-engine tests (SAPI, espeak-ng) on runners where they can run.
- **Supply chain.**
  - `cargo-deny`: licences, duplicates, and advisories.
  - Dependabot.
  - Build provenance attestations.
- **TUI render tests** at 20, 40, 60, and 80 columns, with a long heading, a wide table, and a long code line.

### Architecture

- **Dependency direction.**
  - Take `store` off `aids`: the settings types move into `store`. ADR-0001's rule is that store depends only on core.
  - Add an xtask check that fails on forbidden dependency edges.
- **Duplication.**
  - One notes model: `vault` uses the `store` types. Remove the app's migration shim after one release.
  - One place for font resolution: `textweaver-fonts`.
  - Align the two html5ever versions.
- **Engines.** A `textweaver-engines` crate for the backend registry, shared by the TUI, the CLI, export, and the GUI.
- **Docs layout.** Split `docs/` into user guides, `docs/dev/` (architecture, building, testing, releasing, Docker), `docs/adr/` with an index, and `docs/history/` (plan, tasks, audits).

### Binary size

`tw.exe` grew from 9.4 MB in alpha.3 to 31.4 MB. `cargo bloat` shows code of 19.8 MB. The rest is data: the fonts, SCOWL, and the CSL styles.

The biggest contributors are the citation stack (hayagriva and citationberg), the PDF stack (lopdf, krilla, and the font crates), minijinja, comrak and pulldown-cmark, two html5ever versions, rustls, and serde_path_to_error.

Steps:
- Align html5ever.
- Bundle only the CSL styles used.
- Put comrak behind a feature.
- Consider building `textweaver` (the reader) without the conversion and citation stack, so the reader stays small and starts fast.

### Releases

- **Windows in CI.** Build the Windows package in `release.yml`, keeping the local build as a fallback.
- **Checksums.** A final job that writes them once.
- **Linux: an AppImage** (Jon's choice, 2026-09-26).
  - One file that runs on Debian, Fedora, Arch, and most other distributions without installing anything.
  - Built on an older base (Ubuntu 22.04 era glibc) for x86_64, and aarch64 when practical.
  - It bundles `textweaver`, `tw`, the engine hosts, and the dictionaries. `tw` and `textweaver` are reached through the AppImage's own name, or through symlinks that `--install` creates in `~/.local/bin`.
  - It builds with speechd, and loads espeak-ng at run time with `libloading`, so one binary works with or without libespeak-ng installed.
  - It is signed with the AppImage's own signature support, or its checksum is published, and it is zsync-updatable.
  - `install-linux.sh --release TAG` downloads the AppImage and checks it.
  - A plain tarball stays as a fallback for systems without FUSE.
- **`cargo xtask release X.Y.Z`.** Sets the version, dates the changelog from the machine, updates the version examples, runs the checks, and tags.
- **Release checklist.** A person listens on real hardware with Eloquence and one other engine before each release. Date the release from the tag, in local time, with the weekday computed.

## Phase 3: the GUI (Wave 3)

Many users will want a GUI, even though Jon works in the terminal. The wxDragon spike is accessible on Windows (ADR-0014). The app core still needs these pieces first:

1. **A document window model** in the app: about 500,000 UTF-16 units at a time, aligned to paragraphs, with positions mapped through `DisplayIndex`. Today the GUI loads the whole document, which takes 9.3 s for 10 million characters.
2. **List and prompt state** in the app, shared by the TUI, the GUI dialogs, and JSON-RPC.
3. **GUI keymap defaults** that leave caret keys native.
4. **A waker,** so the GUI reacts to speech events instead of polling every 30 ms.
5. **A replace-range edit command** for native text controls.
6. **A settings schema:** labels, help, ranges, and choices, generated from the store. It drives an accessible settings dialog.
7. **Opening in the background,** with progress and cancel, for large PDF and EPUB files.

Then the GUI steps, in order:

1. **Fix the macOS smoke test,** and accept ADR-0014.
2. **Window slicing.** 10 million characters show in under 300 ms, and the highlight moves in under 30 ms per word.
3. **A listening session with Jon on NVDA and JAWS.** It decides whether the highlight stays the selection or becomes a background colour.
4. **Native labelled dialogs** for find, go to, bookmarks, notes, voices, library, the command palette, and help.
5. **A settings dialog** from the schema, themes through `rgb_table`, and a system theme that respects Windows High Contrast.
6. **Reading aids in the GUI:** text spacing, the ruler, and an RSVP panel that never covers the caret. The font chooser already exists.
7. **Edit mode** on the same control, with incremental updates.
8. **Packaging.** The Windows zip gets `textweaver-gui.exe`. macOS gets an `.app` with the fonts inside, which likely fixes font registration.
9. **Linux.** GTK builds in CI, a smoke test under Xvfb, an AT-SPI tree check, and an Orca test by a person. GTK 3 has no announcement API, so the Linux GUI keeps self-voicing on by default.
10. **VoiceOver** on a real Mac, by a tester.
11. **Ship it.** Make the GUI a default workspace member and include it in the packages.

## Phase 4: Star features for students

These are ranked for students with print disabilities, drawing on Star's history.

1. **OCR for scanned PDFs** (Tesseract, only on pages with no text layer). PDFs are students' main format.
2. **DAISY 3 and DTBook,** the Bookshare format.
3. **Define word:** an offline glossary, then WordNet, then CMUdict, as in Star 0.1.15.
4. **In the reader:** syllable display, and the math exploration mode. Both crates are ready.
5. **Voices.** Piper neural voices and a voice manager with favourites.
6. **More formats.** Archives (`book.zip!inner.pdf`), opening a URL, PPTX, and spreadsheets as tables.
7. **Settings profiles,** reading statistics, and interface translations.
8. **Later:** the difficult-word overlay in the reader, summaries, translation, and karaoke video export.
9. **Deliberately dropped for now:** study tools (spaced repetition, Anki), the knowledge graph, cloud voices, and plugins.

## Star lessons to keep honouring

- **Build spoken text and its map together.** Never align two texts afterwards.
- **Tag every speech event with its reading generation,** and drop old ones.
- **Show a word when the audio reaches it.** Resume from the last word heard.
- **Convert byte offsets** to character offsets at every engine boundary.
- **Automatic engine choice is fragile.** Fall back and say so.
- **Tests that fake the engine hide a silent engine.** A person listens before each release.
- **Single-letter shortcuts need an off switch,** and quitting asks first.
- **Generate help, the palette, and the docs from one key table.**
- **Handle settings and state carefully.**
  - Write atomically and keep unknown keys.
  - Tests never touch the real config.
- **A stored setting must do something.**
- **Test contrast on the colours actually drawn,** at every colour depth.
- **Keep the terminal layout robust.** Wrap every block, and fit any saved width to the terminal.
- **Move the real cursor to the spoken word.**
- **Announce clearly without flooding,** and repeat repeated messages.
- **Positions must survive edits.**
- **Tie hosts and threads to their owner's lifetime,** and isolate engine crashes.
- **Don't trust a green gate.**
  - Prove a new test fails first.
  - Put time limits on hangs.
  - Never call a crash a flake.

## How the work is organised

- **Agents.** Work runs in parallel agents, each in its own git worktree and branch, with briefs in `docs/tasks.md`. The orchestrator merges each branch.
- **Checks before main.** Every merge runs the full checks on Windows and Linux before it reaches main:
  - fmt;
  - clippy with `-D warnings`;
  - the tests;
  - rustdoc with `-D warnings`;
  - the keyboard check;
  - the Docker all-features run.
- **The wiki.** Progress and milestones go into the Obsidian wiki: `meta/textweaver-releases/textweaver release history.md` and `log.md`.

## See also

- [Audit, September 2026](audit-2026-09.md)
- [Star features not yet planned](star-gaps.md)
- [Star parity reference](star-parity.md)
- [Implementation plan](plan.md)
- [Tasks and agent briefs](tasks.md)
- [Releasing](releasing.md)
- [Architecture](architecture.md)
- [Documentation index](README.md)
