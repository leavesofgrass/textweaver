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

## Status (Saturday, September 26, 2026)

- **Phase 1 is done.** Agents P1a (speech stability), P1b (app safety and authoring quick wins), P1c (CI, releases, and notices), and P1d (loaders and command-line tools) are merged. Two housekeeping items are left: pruning merged branches, and merging through pull requests with required checks.
- **Phase 2 is done, with a few items moved to Wave 3.** Agents P2a (reliability), P2b (authoring), P2c (screen reader modes), and P2d (releases, quality gates, and binary size) are merged. Agent P2e (the remaining gaps, and keys that follow NVDA and JAWS habits) is still running.
- **Tests:** 1,854 pass natively on Windows and 1,861 in Docker with all features, at the end of P2a.
- **Releases:** 0.1.0-alpha.3 (Friday, September 25, 2026) is the newest. The next release is the first with the Linux AppImage. Jon decides when it happens; there is no alpha.4 until he says so.
- **Wave 3 is planned** in [tasks.md](tasks.md), with six agents: W3a (app core for the GUI), W3b (the Xilem GUI), W3c (architecture), W3d (formats for students), W3e (language and study aids), and W3f (voices and speech). It starts after P2e merges.
- **The GUI is Xilem.** Jon chose Linebender's all-Rust toolkit on Saturday, September 26, 2026, to keep as much of textweaver in Rust as he can. The wxDragon spike stays as a fallback until the Xilem GUI passes the same accessibility checks.
- **Wave 3 is pure Rust first.** textweaver is an experimental alpha, for Jon's own use first. Wave 3 prefers pure-Rust, in-process solutions over subprocesses and C or C++ libraries: `ocrs` for OCR, Piper voices through `tract` or `candle`, and Whisper through `candle`. It accepts alpha crates and API churn, keeps the tests and CI gates, and records each bold choice and its fallback in an ADR.

Each phase below has its own dated status note. Items marked **done** are on `main`; items marked **left** say where they went.

## Where we were before Phase 1

This section describes `main` at `11f87d0`, when the roadmap was written.

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

**Status (Saturday, September 26, 2026): done.** Every item below is on `main`, each with a test, except two housekeeping items under "CI, releases, and notices":
- **left:** pruning merged branches (the `agent/`, `wave2/`, `integration/`, and `phase1/` branches are still there);
- **left:** merging through pull requests with required checks. Merges are still made locally, after the full checks natively and in Docker.

Who did what: P1a the speech items; P1b saving, settings, authoring, prompts, and the TUI render tests; P1c CI, releases, notices, and dependencies; P1d the loaders and command-line tools. Their status lines in [tasks.md](tasks.md) have the details and test counts.

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

**Status (Saturday, September 26, 2026): done, a day after Phase 1, except the items marked left below.** P2a did the stability items, P2b the authoring items, P2c the screen reader items, and P2d CI, quality gates, binary size, and releases. The architecture items were not started in Phase 2; they are Agent W3c's brief in Wave 3. Agent P2e, running on this date, closes remaining gaps and moves the default keys toward NVDA and JAWS habits. Jon's decisions for it: citations are skipped in continuous reading by default, with a toggle, and spoken in words; and "preview in browser" says when it updates, with an optional automatic reload.

### Stability

**Status: done (P2a; audio device recovery by P1a).** Left, for Agent W3a: an engine's first start still waits, where restarts no longer do. DECtalk now synthesizes a sentence at a time rather than streaming within a sentence.

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

**Status: done (P2b; the citations and math in output by P1d).** Entering edit mode on 10 MB went from 276 ms and 213 MB to 52 ms and 121 MB. Left, for Agent W3a: the misspelling count on save runs on the input thread, about 0.6 s on 10 MB. The editing guide and the citations guide describe the features.

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

**Status: done (P2c).** Screen reader detection covers NVDA, JAWS, and Narrator on Windows, VoiceOver, and Orca. Left: Jon's check by ear of the NVDA and JAWS settings in [the screen reader guide](screen-readers.md), each marked there to verify.

- **An accessibility mode setting,** with three choices:
  - **self-voicing**: textweaver speaks everything;
  - **screen-reader**: textweaver is silent;
  - **hybrid**: textweaver voices continuous reading, math, and tables. The screen reader handles typing echo, caret echo, and messages through the status line.
- **First run.** On Windows, detect a running screen reader and offer hybrid.
- **Quieter screen.**
  - A "quiet screen while reading" option.
  - A cursor setting: follow focus, or park on the status line so "read current line" repeats the last message.
- **Keymap preset.** An optional preset matching screen reader habits: h for heading, 1 to 6 for heading levels, l for list. Since 2026-09-26 this is the default keymap (Agent P2e), and the earlier keys are the `classic` preset.
- **The guide.** `docs/screen-readers.md` gives NVDA and JAWS settings for use with textweaver. Each setting is marked to verify on Jon's machine.

### CI and quality gates

**Status: done (P2d; supply chain by P1c; loader property tests by P1d; TUI render tests by P1b).** The benchmark gate compares peak heap and allocation counts with main's own artifact (`bench.yml`); two runs differ by 4% at most. `nightly.yml` runs 9 fuzz targets, Miri, AddressSanitizer, release-mode tests, the MSRV check, the Docker job, and the soak test; `cargo hack` runs weekly. The MSRV job uses `--ignore-rust-version`, because krilla 0.8 declares Rust 1.92.

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

**Status (Saturday, September 26, 2026): done (P2d and Agent W3c).** The xtask check (`cargo xtask deps --check`, in CI) and one html5ever version came in Phase 2. Wave 3 took store off aids, made one notes model, put font resolution in `textweaver-fonts`, added the `textweaver-engines` crate, and split `docs/` into guides, `dev/`, `adr/`, and `history/`.

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

**Status: partly done (P2d).** html5ever is aligned (ammonia held on 4.1.4, which removed 10 duplicate crates) and comrak is a default feature of `textweaver-render`. `tw.exe` went from 32,625,152 to 32,299,008 bytes; `textweaver.exe` stayed at 11,185,664. Left: bundling only some CSL styles (hayagriva's archive is all or nothing, so it was not done). In-reader export, preview, and citations are now the app's `publish` feature (Agent W3c), on in releases; `cargo build -p textweaver-tui --no-default-features` builds a lean reader without them.

`tw.exe` grew from 9.4 MB in alpha.3 to 31.4 MB. `cargo bloat` shows code of 19.8 MB. The rest is data: the fonts, SCOWL, and the CSL styles.

The biggest contributors are the citation stack (hayagriva and citationberg), the PDF stack (lopdf, krilla, and the font crates), minijinja, comrak and pulldown-cmark, two html5ever versions, rustls, and serde_path_to_error.

Steps:
- Align html5ever.
- Bundle only the CSL styles used.
- Put comrak behind a feature.
- The reader offers export, preview, and citations (Phase 2 authoring), so it links the conversion and citation stack. Done in Wave 3: they are the app's `publish` feature, on in releases, so a lean reader can still be built, and `cargo xtask deps --check` refuses the edges when the feature is off.

### Releases

**Status: done (P1c and P2d), except the aarch64 AppImage.** `release.yml` builds Windows, macOS, and Linux, with one checksums job and provenance attestations. `cargo xtask appimage` builds `textweaver-VERSION-linux-x86_64.AppImage` (17.5 MB, with a `.zsync` file) and the tarball on Ubuntu 22.04; both pass on Debian stable, Fedora, and Arch, with and without espeak-ng. The AppImage is checked by its published checksum, not signed. No release has carried it yet: 0.1.0-alpha.3 came before it. Left: the aarch64 AppImage, on GitHub's arm64 runners, after Wave 3.

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

**Status (Saturday, September 26, 2026): planned for Wave 3.** Jon chose Xilem, Linebender's all-Rust toolkit, for the GUI on every platform: Xilem and Masonry for the widgets, Vello for drawing, Parley for text layout, AccessKit for accessibility, and winit for windows. The steps below were written for the wxDragon spike, which stays as a fallback until the Xilem GUI passes the same accessibility checks. In [tasks.md](tasks.md):

- **Agent W3a** builds the app-core pieces listed first below: the document window model, list and prompt state in the app, the waker, `Command::ReplaceRange`, the settings schema (with a new terminal settings screen), and opening in the background. It also takes Phase 2's leftovers off the input thread.
- **Agent W3b** builds the GUI in a new crate, `textweaver-xilem`, and writes ADR-0023, which supersedes ADR-0014: the main window, the dialogs, themes and fonts loaded straight into Parley, accessibility checks on every OS (UI Automation, AT-SPI under Xvfb, and a macOS smoke test), the large-document targets, and packaging with no GTK or wxWidgets.
- **After Wave 3:** Jon's NVDA and JAWS listening session, the GUI's edit mode and reading aids, VoiceOver and Orca testing, and signing when funding allows.

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
9. **Linux.** GTK builds in CI, a smoke test under Xvfb, an AT-SPI tree check, and an Orca test by a person. (Superseded, 2026-09-26: GTK 3 does announce through ATK's notification signal, which the `live-region` crate uses; see `docs/research/xilem-gui.md`.)
10. **VoiceOver** on a real Mac, by a tester.
11. **Ship it.** Make the GUI a default workspace member and include it in the packages.

## Phase 4: Star features for students

These are ranked for students with print disabilities, drawing on Star's history.

**Status (Saturday, September 26, 2026): most of this phase is planned for Wave 3,** pure Rust first:
- **Agent W3d:** OCR (the pure-Rust `ocrs` in process, only on pages with no text layer, with Tesseract as a fallback), DAISY 3 and DTBook, archives, opening a URL, PPTX, and spreadsheets.
- **Agent W3e:** define word (glossary, WordNet, CMUdict), settings profiles, reading statistics (`tw stats`), and the groundwork for interface translations.
- **Agent W3f:** Piper voices in process (`tract` or `candle`), a voice manager, Whisper dictation in process (`candle`), and rate and pitch remembered per voice. Favourites in the voice list were done in Phase 1.
- **Not yet assigned:** syllable display and the math exploration mode in the reader, and the later items in point 8.

1. **OCR for scanned PDFs** (Tesseract, only on pages with no text layer). PDFs are students' main format.
2. **DAISY 3 and DTBook,** the Bookshare format.
3. **Define word:** an offline glossary, then WordNet, then CMUdict, as in Star 0.1.15.
4. **In the reader:** syllable display, and the math exploration mode. Done in the terminal reader (Agent P2e, 2026-09-26); the GUI does not draw syllables yet.
5. **Voices.** Piper neural voices and a voice manager with favourites.
6. **More formats.** Archives (`book.zip!inner.pdf`), opening a URL, PPTX, and spreadsheets as tables.
7. **Settings profiles,** reading statistics, and interface translations.
8. **Later:** summaries, translation, and karaoke video export. (The difficult-word overlay is done in the terminal reader, Agent P2e.)
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
- **The wiki.** Progress and milestones go into the Obsidian wiki: the hub `domains/textweaver.md`, `meta/textweaver-releases/textweaver release history.md`, and `log.md`.

## See also

- [Audit, September 2026](audit-2026-09.md)
- [Star features not yet planned](star-gaps.md)
- [Star parity reference](star-parity.md)
- [Implementation plan](plan.md)
- [Tasks and agent briefs](tasks.md)
- [Releasing](releasing.md)
- [Architecture](architecture.md)
- [Documentation index](README.md)
