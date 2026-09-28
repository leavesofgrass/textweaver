# The 2026 roadmap: from Wave 4 to the final alpha, and beyond

Written on Sunday, September 27, 2026, by the roadmap planner (Fable 5.1), for the owner and the orchestrator. It answers "what is left, and what does the roadmap look like", refines Wave 5, and sketches the waves after it. The inventory behind it is [What is left](whats-left.md). It changes nothing by itself: the owner adopts it, or parts of it, and the orchestrator applies it to `docs/history/tasks.md`.

What it is built on: everything the inventory names, plus the owner's versioning preference, relayed by the orchestrator today.

**Versioning, from the owner.** textweaver keeps iterating alpha releases (0.1.0-alpha.4, alpha.5, and so on) until it is feature-complete and relatively stable. The final alpha is what most projects would call 1.0: feature-complete. Beta and stable versions come after, only when the owner starts them. Every release happens only when he asks. So this roadmap plans readiness, not release dates: each wave ends with a readiness point, and the owner chooses whether it becomes an alpha.

Dates below are durations laid onto a calendar for planning, taken from the machine's clock with weekdays computed. None is a promise, and none gates anything: the gates are the owner's sessions and the checks.

## The short version

- **Five milestones to the final alpha:** Wave 4 (running), Wave 5, Wave 6, a stabilization wave (Wave 7), and the final alpha's readiness review. Each wave ends with an alpha readiness point. Beta and stable come after, when the owner starts them.
- **Feature complete** has an explicit list (section 3). Most of it is done or in Wave 4; Wave 5 adds Braille, math braille, the course formats, and summaries; Wave 6 closes the GUI, the publishing templates, and translation if approved; Wave 7 fixes and hardens.
- **The critical path** runs through the owner's sessions and the GUI: session 1 decides the GUI's designs, W4a2 and W4a3 build on them, session 3 in Wave 5 confirms the upgraded view, and Wave 6's GUI parity is the last large feature. Everything else can slip a wave without moving the final alpha.
- **Wave 5 is kept as planned in shape** (three sub-waves of three, a Braille session as the first gate) with these changes: "beta readiness" becomes "alpha.5 readiness"; five gaps the inventory found are folded in (PDF page navigation, settings that do nothing, MathML in HTML, the RSVP flashing check, the stale docs W4f does not cover); two overlaps are resolved (the lexicon loading, the nightly fuzz lines); the budget uses the measured sizes; and every branch on Wave 4 is kept.
- **Eight questions for the owner,** each with a recommended default (section 8).

## 1. Where we are

- Sunday, September 27, 2026. `main` is at `bbd16c5`. Sub-wave 4a (W4h, W4s, W4c1) is building; nothing from Wave 4 has merged.
- The newest release is 0.1.0-alpha.3 (Friday, September 25, 2026). `main` holds Wave 3 in full: the Xilem GUI, OCR and the student formats, Piper and in-process dictation, define word, profiles, statistics, and the usability pass.
- About 1,933 tests native and 1,940 in Docker at the end of Wave 3; 15 fuzz targets, all nightly.
- The method that works: sub-waves of three agents, one app-message agent per sub-wave, one merge point, one thing for the owner to hear, the measured budget (16 GB native and 8.5 GB per container volume for a lean build; allowances 24 GB and 13 GB), at most three containers at 6 GB each, and a flush after each sub-wave.

## 2. Milestones in order

Each milestone has a goal, its contents, its gates (what the owner listens to or reads on the display), and its entry and exit criteria. "Readiness" means the orchestrator hands the owner one page: every package built by the release workflow on a branch, the listening and Braille checklists dated, the doc checks green, the nightly fuzz clean for a week, the open issues list. The owner decides the release; nothing tags.

### Milestone 1: Wave 4, then alpha.4 readiness

**Goal.** Fix what the owner hears on every run, decide the GUI's designs by ear, add the terminal authoring tools, the speed and memory pass, MathCAT speech, RTF and ODT, the five languages, GUI edit mode, and the release machinery for the next alpha.

**Contents.** As adopted on September 27, 2026: sub-wave 4a (W4h terminal polish, W4s GUI session prep, W4c1 MathCAT speech); 4b (W4g authoring extras, W4b speed and memory with the rope measured, W4a2 the GUI after the session); 4c (W4c2 documents, W4d translations, W4a3 GUI edit mode, W4f platforms and CI as the light fourth). The wxDragon spike is removed after session 2.

**Gates.**

- Session 1 (after 4a): NVDA, then JAWS: W4h's five items; the Xilem GUI's checklist; the two designs (highlight as selection or background; live regions or a UIA notification). Also, folded in from Phase 2's leftover: the NVDA and JAWS settings in `docs/screen-readers.md` that are marked "verify", and a note on Windows Terminal against the classic console.
- Session 2 (after W4a2 merges): the window slide, the reading aids, the session-1 fixes. It decides the wxDragon removal.
- Check 3 (after 4c): the reader in Spanish or French for five minutes; the pseudo-locale; W4a3's edit-mode checklist on the GUI with the display on the caret line.

**Entry criteria.** Met: the lean build measured, the six fuzz targets nightly, Docker restarted, the plan files fixed.

**Exit criteria.** Every 4c branch merged and flushed; Docker's disk compacted with the owner; Wave 4's five outcomes written under a "Wave 5" heading in `docs/history/tasks.md` (the rope numbers, the memory table, MathCAT #827's state, W4a3's state, W4d's state); the alpha.4 readiness page (W4f: the release workflow run on a branch, the dated listening checklist, the notes) handed to the owner.

**Alpha.4 would carry:** the first Linux AppImage, the NVDA and JAWS style keys, Wave 3, and Wave 4, with the GUI marked experimental unless the owner says otherwise (question 4). Planned end of Wave 4: about Wednesday, October 7, 2026.

### Milestone 2: Wave 5, then alpha.5 readiness

**Goal.** Braille first class, the foundations settled (the rope, memory, hostile input), the course formats and math braille, summaries, the GUI's third pass, automated accessibility checks beside the owner's sessions, and alpha.5 readiness. Section 5 has the refined plan.

**Gates.** Session B1 (Braille, after 5a); check 2 (by ear and on the display, after 5b); session 3 (the GUI, after 5c); the alpha.5 readiness review.

**Entry criteria.** Wave 4's exit criteria; the owner's answers to the questions in section 8; D: at least 400 GB free and 32 GB of memory free with Docker idle; the caches warm.

**Exit criteria.** Every 5c branch merged and flushed; ADR-0034 to ADR-0039 written or marked unused; session B1's findings written into `docs/screen-readers.md` with the "to verify" marks removed; the alpha.5 readiness page handed to the owner.

Planned: Monday, October 12 to about Thursday, October 29, 2026, if Wave 4 ends on time.

### Milestone 3: Wave 6, then alpha.6 readiness

**Goal.** Close the feature list: the GUI at parity with the terminal reader, the publishing templates, document translation if approved, the leftovers Wave 5 deferred, and the second-tool accessibility checks. Section 6 sketches it.

**Gates.** Session 4 (the GUI at parity, on NVDA, JAWS, and the display); check 5 (the templates: an APA paper exported and read back; a translated chapter if W6e ran); the alpha.6 readiness review.

**Entry criteria.** Wave 5's exit criteria; the feature-complete list (section 3) reviewed by the owner, with anything he adds or removes recorded in `docs/history/tasks.md`.

**Exit criteria.** Every feature-complete item is done or the owner has struck it; the alpha.6 readiness page handed to the owner.

Planned: about Monday, November 2 to Friday, November 20, 2026.

### Milestone 4: Wave 7, stabilization, then the final alpha's readiness

**Goal.** No new features. Fix what the owner's daily use and the sessions found; harden; make every check trustworthy; finish the docs. This is the wave that makes "relatively stable" true (section 3).

**Contents.** Bug-only agents in small sub-waves; the soak test on the owner's own documents; real-engine tests where a runner can run them; the second-tool checks (veraPDF or PAC, epubcheck, liblouis grade 2 in CI); the docs read end to end against the program by an agent that has never seen them; the final alpha's release notes as a full feature list.

**Gates.** the owner's two weeks of daily use on the release build of the terminal reader, with a log of every stall, lost place, or wrong announcement, each fixed or struck; one full session on the GUI; one Braille session on the BRF writer and the display.

**Entry criteria.** Wave 6's exit criteria; the open issues list under twenty items, none high.

**Exit criteria.** The final alpha's readiness page: the criteria in section 3 met line by line, with evidence.

Planned: about Monday, November 23 to Friday, December 11, 2026.

### Milestone 5: the feature-complete final alpha

Released only when the owner says. It is the last 0.1.0-alpha.N. Its release notes list every feature and every known limit. After it, no feature work lands on `main` until the owner starts the beta.

### Later milestones: beta and stable, when the owner starts them

- **Beta (0.1.0-beta.1 and later).** the owner's call, after the final alpha. Proposed contents: the fixes from other users' reports, signing if funded, a tester's VoiceOver and Orca sessions, and any feature the owner adds after using the final alpha. A beta changes no public format: the state files, the settings, the keymap, and the JSON-RPC protocol are frozen at the final alpha, with migrations for anything that must change.
- **1.0 (stable).** the owner's call. Proposed criteria: two betas without a high issue; the listening and Braille checklists passed on the release build on all three systems by a person (the owner on Windows and Linux; a tester on macOS, or macOS marked "built, untested by ear" per question 8); the docs checked by tool and by a reader; the packages signed where funding allows.

## 3. What "feature complete" and "relatively stable" mean

These are the criteria the final alpha's readiness page checks. The owner edits this list; the roadmap proposes it.

### Feature complete

The terminal reader and `tw`:

- Reads every format on the list: text, Markdown, HTML (with MathML), PDF (with OCR and page navigation), EPUB (with MathML), DOCX (with comments and revisions), RTF, ODT, DAISY and DTBook, PowerPoint, spreadsheets, LaTeX (the subset), EML and MHTML, archives, web addresses, images. Pandoc only for what is not on the list.
- Speaks with every engine on the list (Eloquence, SAPI5 and OneCore, Apple, espeak-ng, speech-dispatcher, DECtalk, Omnivox, Piper) with exact highlighting where the engine reports words, and never stalls, loops, or dies silently.
- Math: spoken by the built-in engine and MathCAT; explored by formula; braille in Nemeth and UEB; Unicode in the plain view.
- Braille: BRF grade 1 native and grade 2 through liblouis, with math; every status line meaning first, tested on the owner's display.
- Authoring: edit mode with structure, spell check, grammar, lint, citations, templates, export and preview, the clipboard everywhere, notes export, summaries, the outline.
- Study: notes, highlights, bookmarks, the vault, define word, statistics, profiles, the library with metadata search, sync.
- The interface in English, Spanish, French, German, Portuguese, and Arabic, right to left where the terminal allows.
- The three access modes, the NVDA and JAWS style keys with the classic preset, F9, the settings screen, JSON-RPC.
- Dictation in process, and audio export with subtitles and chapters.

The GUI:

- Everything the terminal reader does, on the same app core: reading, edit mode, every dialog, the reading aids, syllables and difficult words, the voice manager, the settings dialog, themes and fonts, the five languages.
- Passes the UI Automation report, the AT-SPI dump, and the owner's sessions on NVDA, JAWS, and the display.
- Packaged on the three systems, if question 4 says it ships as supported.

Platforms and packaging: Windows x86_64, macOS universal, Linux x86_64 and aarch64 AppImages and tarballs; checksums and attestations; the install and update scripts.

Not required for feature complete (proposed; question 7): document translation, grade 2 braille in pure Rust, streaming dictation, signing, the peripheral Star list (karaoke, feeds, plugins, study tools, cloud voices), Windows on arm64.

### Relatively stable

- The owner's daily use of the release build for two weeks without a stall, a lost place, a crash, or a lost edit, on his own documents, with Eloquence and one other engine.
- The soak test clean on the 10 MB corpus and on the owner's documents; no host process left over; memory level.
- The nightly fuzz clean for thirty days on every target; Miri and AddressSanitizer clean.
- The benchmark gate green against the previous alpha; startup, first speech, and edit-mode entry within the numbers `docs/dev/testing.md` records.
- Every release package started on its system by a person or a runner, with speech heard where a person is available.
- The open issues list has nothing high, and every medium item is either fixed or written into the release notes as a known limit.
- Every ADR's status updates current; the docs checks green; every "verify" mark in the guides resolved.

## 4. The critical path to the final alpha, and what could slip

The path (each step waits for the one before it):

1. **Session 1** decides the GUI's highlight and announcement designs. Until it happens, W4a2 cannot start. Risk: the owner's time; a JAWS silence that forces the UIA notification path (W4s has it ready).
2. **W4a2, then session 2,** then the wxDragon removal, then **W4a3 (GUI edit mode)** in 4c. Risk: edit mode on multi-line text with a screen reader is the hardest GUI item in the plan (no Rust app does it well yet); if it slips, it takes W5a4's slot in Wave 5 and pushes the GUI's third pass to Wave 6 (the Wave 5 plan's branch 4).
3. **W4b's rope numbers** decide W5r's branch. Risk: none to the path; either branch fits Wave 5. A "go" that must change `CharPos` moves the migration to Wave 6 (question 2), still before the final alpha.
4. **MathCAT issue #827** decides whether W5c4 pins a release or vendors a patch. Risk: a day for the vendoring; the plan has it.
5. **Session 3 (Wave 5)** confirms the upgraded Parley view. Risk: a regression reverts the upgrade to 0.8 and defers it; edit mode on 0.8 is enough for the final alpha.
6. **Wave 6's GUI parity** is the last large feature. Risk: it is large; if it runs long, the stabilization wave starts on the terminal reader while the GUI agent finishes.
7. **Wave 7's two weeks of daily use** is the last gate, and only the owner can run it.

What is not on the path: every terminal-only item (W4h, W4g, W5x, W5s), the formats (W4c2, W5c3), translations (W4d, W5e), platforms (W4f, W5p, W5t). Each can slip a wave without moving the final alpha, because the next wave's plan carries it forward as the first item of the same area's agent.

What slips the whole path: the owner's sessions not happening (the plan has nothing else to gate the GUI on; the automated checks in W5t are a complement, never a substitute); a memory floor breached (a wave pauses until the flush); the deletion rules broken (everything stops).

## 5. Wave 5, refined

The Wave 5 plan (`wave5-plan.md`) holds. This section lists what changes, checked against the inventory. Everything not named stays as the plan wrote it: the common rules, the Braille checklist order, the branches on Wave 4 (section 3 there), the crate ownership map, the runbook.

### 5.1 The name and the aim

"Beta readiness" becomes **alpha.5 readiness** everywhere: in W5p's brief, the runbook's step 6, and the questions. Wave 5's purpose is unchanged: remove every way to stall or lose your place, finish what students need for reading and writing, make the release machinery trustworthy. The difference is that alpha.5 is one more alpha, not a beta: the state files and settings may still change with a migration, and the GUI may still be marked experimental.

### 5.2 Gaps folded in

Each is a new deliverable, added at the end of the named agent's list so the order in the plan stands.

1. **PDF page navigation** (W5x, small): "go to" takes a page number when the document has `PageBreak` markers; the position report and `say_status` name the page; the outline lists pages when there are no headings. The owner's checklist gains one item: go to page 12 of a PDF and hear it.
2. **Settings that do nothing** (W5x, small): `[fonts] fetch_missing` either offers the download in the font chooser after a yes or is removed with a migration; `[speech.dectalk]`, `[speech.voice_params]`, and the engine sections become typed settings so `tw settings export` lists them; the "every setting is used" test is extended to catch a setting that no reader touches. The Star lesson applies: a stored setting must work.
3. **MathML in HTML input** (W5c3, small): the HTML loader turns `<math>` into the `Math` marker the way W4c1's EPUB path does, so a converted page reads as math and not as symbols run together.
4. **The RSVP flashing check** (W5s, small): a data test in `textweaver-aids` that the RSVP schedule at the highest rate and the smallest word never flashes more than three times a second on the same region (WCAG 2.3.1), and a cap if it can.
5. **The stale passages W4f's list does not cover** (W5p and W5x): `docs/README.md` (the Decisions list, the crate count, the roadmap line), `docs/roadmap.md`'s status, `docs/star-gaps.md` refreshed with the "done since" list from the inventory, `CHANGELOG.md`'s Unreleased section naming the Xilem GUI, Piper, in-process dictation, and define word, the ADR index's lines for ADR-0014 and ADR-0020, `README.md`'s "Coming next"; and, for W5x, the GUI section of `docs/screen-readers.md` rewritten for the Xilem GUI in one paragraph that points at `docs/gui.md`.
6. **Braille in the release checklist** (W5p, tiny): the five items from session B1 added to the listening checklist in `docs/dev/releasing.md`, so `cargo xtask release` dates them too.
7. **The lexicon off the input thread** (W5x only): the plan lists it under W5x's item 6 and again under W5m's branch 2. W5x owns it in every branch; W5m's branch 2 covers the engines and the OCR, Piper, and Whisper models only.
8. **The nightly fuzz lines:** the plan says the matrix runs nine targets and the orchestrator adds six; commit `132382f` did that, so W5m's day-one item is only W4c2's three targets if W4f did not add them, plus `cargo xtask fuzz-seed`.

### 5.3 Overlaps and ordering fixed

- **W5c4's day-one gate moves to step 0.** The orchestrator reads MathCAT issue #827 when Wave 5 starts and writes the path (pinned release, or vendored with the patch) into the brief, so W5c4 does not spend its first day deciding, and `third_party/mathcat` is in the budget from the start.
- **W5x and W5p both edit docs:** W5x owns `docs/screen-readers.md` and `docs/library.md`; W5p owns the rest. Neither touches the other's file; a needed line goes in the report.
- **W5s and W5x both touch the list model:** W5x's "study lists on the list model" merges first (5a), and W5s builds its summary list on it in 5b. Already the plan's order; stated here so nobody reorders it.
- **W5t and W5p in `.github/`:** unchanged (W5t owns `gui-xilem.yml` and `a11y-tests.yml`; W5p owns `release.yml`, `ci.yml`, `nightly.yml`). W5m's nightly lines land in 5a before W5p starts.
- **W5e's slot when cut:** if question 1 says no, the slot in 5c stays empty rather than pulling a Wave 6 item forward, because 5c already has a GUI agent building; W5p may build the GUI packages natively in it.

### 5.4 Quick wins, with owners

From the plan's list of eighteen, kept, plus the new ones. Sizes: tiny under an hour, small under half a day.

- Orchestrator at step 0: `docs/README.md`'s ADR list and crate count (tiny), if W5p is not to do it; the MathCAT #827 reading (tiny).
- W5x: the status line meaning first (small); the Braille section of the guide (small); `y` and `n` in the "Open it?" lists (tiny); library search by DOI, ISBN, and author (small); Star profiles imported (tiny); the lexicon off the input thread (small); PDF page navigation (small); settings that do nothing (small); the screen-reader guide's GUI paragraph (tiny).
- W5m: the math and citation fuzz targets (small); the theme, lexicon, vault, and JSON-RPC targets (small); `cargo xtask fuzz-seed` (tiny).
- W5r: nothing quick; the ADR is the deliverable.
- W5c3: EML text-only on day one (small); MathML in HTML (small).
- W5c4: nothing quick; the BRF math path is medium.
- W5s: `tw summarize` (small); the difficult-word definitions (small); the RSVP flashing check (small).
- W5a4: the measurement harness run on main before touching Parley (tiny).
- W5t: the accessibility-cli tree dump as a CI artifact with a diff (small).
- W5p: `cargo xtask settings-doc --check` (small); `cargo xtask docs --check` (small); `docs/roadmap.md`'s status (tiny); the GUI packages in `release.yml` (small, if question 4 says yes); the Braille lines in the release checklist (tiny); the stale passages (small).

### 5.5 The method and the budget

Unchanged in method: three sub-waves; at most three agents building at once, plus one light fourth in 5c; one app-message agent per sub-wave (W5x, W5c4, W5e); one merge point per sub-wave; the owner's session as the gate; the flush after each sub-wave; the memory rule (free memory at least 26 GB before a container starts, 30 GB before the native check).

The budget, with the measured sizes in place of the plan's allowances:

- Per agent: 24 GB native and 13 GB per container volume (1.5 times the measured 16 GB and 8.5 GB), so **37 GB** for an ordinary agent, GUI agent included. rten agents (W5e): add 5 GB for release-mode test binaries, **42 GB**. Light agents (W5t, W5p): under 5 GB, no container.
- The peak is 5c: W5e 42 plus W5a4 37 plus W5t 5 plus W5p 5 equals **89 GB** of agent build output, against the plan's 125. With the orchestrator's 40 GB, the caches' 60 GB, Docker's other data at about 150 GB, and the models under 1 GB, the peak is about **340 GB** used. D: should be near 850 GB free after Wave 4's flush and compaction, leaving about **500 GB free at the peak**, two and a half times the 200 GB floor. If every estimate is wrong by two, about 430 GB free.
- Memory: three containers at 6 GB with four jobs each (18 GB) inside the 20 GB available; the same rule as Wave 4.
- Models: only with the owner's yes (question 1); about 450 MB for four OPUS-MT pairs, one direction each.

### 5.6 Where Wave 5 depends on Wave 4: the branches, kept

The plan's five branches stand, with their names:

1. **The rope** (W5r): branch A "stay" if ropey 1.6 is within 1.5 times of the fastest on every trace a user feels; branch B "migrate inside `textweaver-text`" if a felt trace is 2 times slower; if `CharPos` must change meaning, the migration is Wave 6 with a state migration (question 2). If W4b did not measure, W5r measures first.
2. **Memory** (W5m): drop the GUI item if the GUI is back near 100 MB; take the engine and app lazy loading if W4s pointed there and nobody fixed it; leave renderer buffers to W5a4.
3. **MathCAT** (W5c4): pin a release if #827 is closed and released; vendor 0.7.6-rc.3 with the described fix if open. If W4c1 slipped, W5c4 does W4c1's brief first and braille moves to 5c.
4. **The GUI** (W5a4): the Parley upgrade first if W4a3 landed and session 2 passed; edit mode first if W4a3 slipped, with the voice manager moving to Wave 6.
5. **Messages** (every agent): catalog ids with English fallback if W4d landed; `format!` strings if it slipped, with W4d's brief taking 5c's slot from W5e.

### 5.7 Timeline

Wave 4 is planned to end about Wednesday, October 7, 2026. Wave 5 then needs Wave 4's numbers written up, a Docker restart, and the plan files fixed: the earliest start is Monday, October 12. Sub-wave 5a about three days (to Wednesday, October 14), session B1 Thursday, October 15; 5b about three days (Friday, October 16 to Tuesday, October 20), check 2 Wednesday, October 21; 5c about four days (Thursday, October 22 to Tuesday, October 27), session 3 Wednesday, October 28, the alpha.5 readiness review about Thursday, October 29, 2026. If Wave 4 slips a week, everything slips a week.

## 6. Wave 6 and beyond: a first sketch

Wave 6 closes the feature list. Same method: three sub-waves of three, one app-message agent per sub-wave, the owner's session as each gate. Its briefs are written after Wave 5's reports, from these candidates, in this order of value.

### Sub-wave 6a: the GUI at parity, and the terminal's leftovers

- **W6a5, GUI parity.** Everything the terminal reader has that the GUI still lacks after W5a4: citations, export and preview, spell check, grammar and lint in edit mode; the library, the vault commands, define word, statistics, profiles; the reading aids' remaining settings; the command palette and font chooser on the app's models; the system theme from the platform palette and Windows High Contrast. The one GUI agent. Gate: session 4.
- **W6x, terminal leftovers.** Speed presets (Star's skim, normal, study, slow, cycled by one key); `WordTrack` off the input thread; the highlight styles if the owner wants them; whatever session B1 and 3 left; the JSON-RPC `insert` method. The app-message agent of 6a.
- **W6r, the rope migration** if Wave 5 deferred it (branch B with a `CharPos` change and a state migration), else the `textweaver-text` hot spots that remain. Otherwise the slot goes to W6e.

### Sub-wave 6b: publishing, PDF, and OCR

- **W6g, publishing templates.** Star's accessible templates as textweaver's: Word templates for APA student papers and AMA manuscripts, the EPUB cover, the stylesheets (large print, dyslexia-friendly, high contrast, manuscript), single-file HTML already done; real Word footnotes; page labels in PDF from print page breaks. Value: a student handing in a paper.
- **W6c5, PDF and OCR extras.** Link targets and annotations (a lecturer's comments as notes), form fields, rotated scans, OCR table structure, captions marked by pattern.
- **W6b, braille extras.** BANA table formats, typeform indicators, the capitals passage indicator; grade 2 in pure Rust only if the owner asks (question 7).

### Sub-wave 6c: translation, checks, and platforms

- **W6e, document translation** if it was cut from Wave 5 and the owner approves the models; else streaming dictation.
- **W6t, second-tool checks.** veraPDF or PAC on the tagged PDF, epubcheck on the EPUB, liblouis grade 2 in a CI job; the real-engine tests on runners that can run them; the 32-bit hosts tested; the upstream proposals to Masonry and AccessKit filed.
- **W6p, alpha.6 readiness.** The release checks; the docs for the new features; the feature-complete list checked line by line.

### Wave 7: stabilization

Described under milestone 4. No new features; bug-only sub-waves; the two weeks of daily use; the final alpha's notes.

### After the final alpha

- The beta, when the owner starts it: other users' reports, signing if funded, a tester's VoiceOver and Orca sessions, frozen formats.
- 1.0, when the owner says.
- Beyond: the peripheral Star list only if users ask (feeds, karaoke, plugins, study tools), Windows on arm64, a Mac.

## 7. Housekeeping the waves need

- **Pruning merged branches:** after a wave, with the owner's approval, never by an agent; a dry run listed first.
- **The stray build-lock files** under the junk `C*/Program Files` folder in the repo (W3d's report): the guard blocks `git rm` on them; the owner removes them or approves the command, by reference, never by a retyped name.
- **`cargo-about` on this machine,** so `cargo xtask notices` runs here instead of by hand (W3d merged the notices by hand).
- **The wiki's project-state memory** says "waiting for the owner's go"; the orchestrator updates it with Wave 4's launch.
- **The Docker disk:** compacted after each wave with the owner at the administrator prompt.

## 8. Questions for the owner

At most eight, each a decision only he can make, each with a recommended default. The first five restate the Wave 5 plan's questions for the new versioning; the last three are new.

1. **Translation models.** May W5e download the OPUS-MT quantized pairs from Hugging Face (about 113 MB each; English to Spanish, French, German, and Arabic, one direction each, about 450 MB in all), after each download's license is shown? Or does document translation wait for Wave 6? **Default: wait for Wave 6.** Nothing depends on it, it is the first thing to cut, and Wave 5's slot is better spent on the GUI and the Braille work; the no-model summaries stay in Wave 5.
2. **The rope during the alphas.** If W4b's numbers show a gain of two times or more on a trace you feel, may Wave 5 change the rope (ADR-0034, branch B) inside `textweaver-text`? And if `CharPos` would have to become a byte position, may Wave 6 do that migration with a state-file migration, still before the final alpha? **Default: yes to both.** The alphas are the time for a text-model change; the final alpha freezes the formats.
3. **Math braille.** Nemeth or UEB first for formulas on the display and in BRF files? Both are built behind `[braille] math_code`; your answer sets the default and the testing order. And will you do the Braille session B1 as sub-wave 5a's gate, and with which display (the wiki does not record the model)? **Default: Nemeth first,** since United States students most often meet Nemeth within UEB; session B1 on your everyday display.
4. **The GUI in the final alpha.** Does the final alpha ship the GUI as a supported package on all three systems, or terminal-first with the GUI marked experimental and packaged only from the GUI workflow? This decides W5p's release work now and how hard sessions 3 and 4 gate the final alpha. **Default: supported on Windows, experimental on macOS and Linux** until a person has heard them (question 8); alpha.4 and alpha.5 carry it as experimental everywhere.
5. **Signing.** Do the alphas, including the final alpha, ship unsigned with the SmartScreen and Gatekeeper notes in the install guide, as alpha.3 did? **Default: yes;** signing is a beta item, when funding allows.
6. **A readiness point after every wave.** Do you want the orchestrator to hand you an alpha readiness page after each wave (alpha.4 after Wave 4, alpha.5 after Wave 5, and so on), so you can release whenever you choose, or only when you ask? **Default: after every wave.** It costs the light agent a day and keeps the release machinery exercised.
7. **What "feature complete" leaves out.** Section 3 proposes that document translation, grade 2 braille in pure Rust, streaming dictation, signing, and the peripheral Star list are not required for the final alpha. Do you want any of them in, or anything else out (for example the aarch64 AppImage, or Arabic)? **Default: the list as written.**
8. **macOS and Linux by ear.** You have no Mac and no Linux desktop tester on record. For the final alpha, should macOS be marked "built and checked by tool, not heard by a person", or should we look for a tester (a VoiceOver user, and an Orca user) before the final alpha? **Default: mark it,** with the tree dumps and the smoke test as the evidence, and look for testers during the beta.

## The owner's answers (Sunday, September 27, 2026)

1. **Translation models:** wait until Wave 6. W5e is not run in Wave 5; the no-model summaries stay in Wave 5.
2. **The text engine may change during the alphas,** and probably the betas too. W5r may change the rope on a clear gain, and a later wave may change `CharPos` with a state migration. The formats freeze no earlier than the final alpha.
3. **Math Braille:** Nemeth first. The owner's everyday display is the **HumanWare Mantis Q40, 40 cells**. Session B1 uses it, and Braille output is checked against a 40-cell line.
4. **The GUI doesn't wait for the final alpha.** It ships as a supported part of the alpha releases as soon as it's ready, on Windows first. macOS and Linux are marked "built and tool-checked, not yet heard" until someone listens to them.
5. **An alpha after each wave:** alpha.4 after Wave 4, alpha.5 after Wave 5, and so on, each released only when the owner says.
6. **A readiness page for each wave.**
7. **Defaults accepted for the rest:**
   - the alphas ship unsigned;
   - "feature complete" leaves out document translation, grade 2 in pure Rust, streaming dictation, signing, and the peripheral Star list;
   - macOS and Linux are marked "built, checked by tool, not heard", with testers sought during the beta.

## Pause after Wave 4, and recalibration (the owner, Sunday, September 27, 2026)

- **After all of Wave 4 is done, there's a short recalibration pause.** Wave 5 launches when the owner says, and doesn't wait for the Cloud Agent.
- **A Cloud Agent contribution is planned** in `docs/research/cloud-agent-plan.md`: tasks chosen to run in a separate cloud session, delivered by pull request, within a $125 budget.
- **No duplicate work** (the owner, the same afternoon).
  - The Cloud Agent's tasks are listed in a reservation list in the repository, with their items and files.
  - Wave 5 briefs mark them "reserved, not yours".
  - Before each Wave 5 sub-wave, the orchestrator checks the open pull requests: merged tasks come off Wave 5's list, and open ones stay reserved.
  - Wave 5 and the Cloud Agent run in parallel.
- **Before Wave 5 launches,** this roadmap and `wave5-plan.md` are recalibrated to account for what the Cloud Agent covers.

## The owner's changes to the feature-complete list (Monday, September 28, 2026)

- **Streaming dictation is on the list** ("if at all possible"). It gets its own agent in Wave 6, W6d, whether or not document translation runs, building on the in-process Whisper dictation. Wave 5's recalibration adds a short research step on live, low-latency speech-to-text in pure Rust, so W6d starts from findings. It is no longer among the items not required.
- The rest of the list is under review by the owner before the Wave 5 briefs are written.

## See also

- [What is left](whats-left.md): the inventory behind this roadmap.
- [Wave 5 plan](wave5-plan.md): the briefs this roadmap refines.
- [Wave 4 orchestration plan](wave4-orchestration.md): what is running now.
- [Roadmap](../roadmap.md): the phases up to Wave 3.
- [Tasks and agent briefs](../history/tasks.md): where the adopted plan goes.
- [Releasing](../dev/releasing.md): the release process and the listening checklist.
- [Documentation index](../README.md)
