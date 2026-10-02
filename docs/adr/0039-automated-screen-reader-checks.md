# ADR-0039: Automated screen-reader checks beside the listening sessions

- Status: accepted for the tree dump and the Orca session, which are standing checks, failing their job, since Tuesday, September 29, 2026; proposed for the NVDA and VoiceOver sessions, which still report only. The status update at the end has the evidence, and adds the braille and real-engine checks.
- Date: 2026-09-28; updated 2026-09-29
- Builds on: [ADR-0027](0027-xilem-gui.md) (the accessibility checks that exist), [ADR-0028](0028-xilem-gui-after-the-session.md) (the first accessibility session), and [ADR-0033](0033-gui-session-2-and-edit-mode.md) (the second session)

## Context

The GUI's accessibility bar is set by listening sessions with NVDA, JAWS, and the Braille display. Between sessions, CI checks what assistive technology is given, never what it says:

- Windows: the UI Automation report (`crates/textweaver-xilem/tools/uia-report.ps1`), in the GUI workflow.
- Linux: the AT-SPI check under Xvfb (`crates/textweaver-xilem/tools/atspi-check.sh` and `atspi-dump.py`).
- macOS: a smoke run that reads with the silent paced backend and exits.
- The harness tests, through `accesskit_consumer`.

This work set out to answer four questions, each answered in writing before the next: can a tree dump on all three systems catch regressions between merges; can Guidepup drive NVDA against a native winit window on a Windows runner; can a scripted session under Xvfb check what Orca follows, without AT-SPI's Collection interface (AccessKit pull request 758 is a draft); and can Guidepup enable VoiceOver on a macOS runner.

The tools, as read on Monday, September 28, 2026:

- **accessibility-cli** (DioxusLabs, MIT or Apache-2.0) reads accessibility trees on Windows (UI Automation), macOS (the AX API), and Linux (AT-SPI), and prints them as JSON. It is not on crates.io; it is built from source.
- **Guidepup** 0.34.0 (MIT, published August 31, 2026, "now uses NVDA 2026.2") drives NVDA and VoiceOver and returns the spoken phrases. Its examples are all browsers. **@guidepup/setup** 0.29.1 (MIT, September 27, 2026) is its setup tool: `guidepup install nvda` downloads NVDA and checks its SHA-256; `guidepup setup --ci` prepares macOS (AppleScript control of VoiceOver, no prompts). On Windows `setup` does nothing.
- **Orca** writes what it speaks to its debug log (`--debug-file`), as `SPEECH OUTPUT:` lines.

## Decision

### The tree dump: a standing check in the GUI workflow

`gui-xilem.yml` dumps the window's tree on Windows, macOS, and Linux:

- accessibility-cli is built from commit `f64c036915095fbb9187b6fdf0f039879618a159` with its own `Cargo.lock` (`cargo install --locked --git ... --rev`), and cached per system by that commit.
- `tools/a11y/tree-dump.sh` (Linux under Xvfb with a private D-Bus session and the AT-SPI bus; macOS) and `tools/a11y/tree-dump.ps1` (Windows) open `fixtures/t/reading.md` with `--background` and the silent paced backend, wait up to 40 seconds for the tree to hold the document, and dump it.
- `tools/a11y/tree_report.py` keeps what a screen reader reads (role, name, value, description, states, actions) as one indented line per element, without ids, bounds, or the process id, so two runs of the same code give the same text.
- It compares that with the tree from main's last successful run (`tools/a11y/fetch-baseline.sh`, through the GitHub CLI and the job's token). The run summary says "Pass", "Changed" (with each added and removed line, in words), "No baseline", or "Fail".
- A change never fails the job: a person reads it and says whether it was meant. A dump that fails does not fail the job either: every dump step is report-only (`continue-on-error`), so the GUI workflow's result comes from its build and tests alone, and a new tool cannot turn it red.

This is the regression check earlier work called for: the tree of every merge, kept as an artifact, and the difference from main in words.

### The sessions: report-only, in their own workflow

`a11y-tests.yml` runs by hand (`workflow_dispatch`, with a choice of session) and when its own files change. Each session opens `fixtures/t/reading.md`, reads, pauses, moves to the next heading, opens a dialog, and closes it, and checks what was said against `fixtures/t/expected-phrases.json`: the design chosen in the first accessibility session (announcements through the live region, heard once). "Must" patterns fail the session; "should" patterns are warnings; the patterns are loose, since each screen reader words things its own way.

- **NVDA** (`tools/a11y/nvda-session.mjs`, a Windows runner): Guidepup starts NVDA, the GUI (hybrid renderer, because WARP crashes on Vello's compute shaders) is brought to the foreground, and keys go through Guidepup: Ctrl+Shift+Space to play and pause, h, Ctrl+Comma, Escape. The report's first line answers the question: "Answer: yes" when NVDA spoke from the textweaver window.
- **Orca** (`tools/a11y/orca-session.sh` and `atspi-session.py`, Ubuntu under Xvfb): the session is checked through AT-SPI events, built on the crate's `atspi-dump.py`: the Opened announcement, the caret following the reading, Paused announced once, the caret on the next heading's line, the outline (Alt+O) taking the focus, Escape closing it. The tree is walked by hand; nothing uses Collection. Orca runs beside it with a debug log, and what it said at each step is listed, not checked.
- **VoiceOver** (`tools/a11y/voiceover-session.mjs`, `macos-14`): after `guidepup setup --ci`, the same steps with Guidepup's VoiceOver. If the runner cannot enable it, the setup log says why, and the macOS tree dump stays the macOS check.

Each session step is report-only (`continue-on-error`): the job fails only when the GUI or the tools cannot be built or installed. A session could become standing, its step failing the job, once its answer is recorded in "The answers" below, its must checks have passed three runs in a row, and that is confirmed.

### Rules

- **Runners only.** The session scripts refuse to run unless `GITHUB_ACTIONS` is `true`. NVDA and JAWS on a developer's machine are that person's working screen readers; a script must never take them over. Nothing is downloaded or installed on a development machine.
- **Pinned and checked.** accessibility-cli by commit and its lock file; Guidepup and its setup tool by version, with every package's integrity hash in `tools/a11y/package-lock.json`, installed with `npm ci --ignore-scripts`; NVDA by Guidepup's SHA-256 check.
- **No personal identifier** in any workflow, script, log, or artifact name.
- **A green run never replaces a session.** Every check here is a tripwire between listening sessions.

### What only manual testing can check

- JAWS: no tool drives it in CI.
- The Braille display (the HumanWare Mantis Q40): what reaches 40 cells, and in what order.
- Whether speech is understandable and well timed: interruptions, double speech the logs cannot show, and the reading voice (Eloquence) against the screen reader's.
- Focus mode and browse mode as a person moves through them, and the feel of edit mode.
- Anything the design leaves to judgment: which highlight reads better, which announcement path is heard reliably.

## The answers

From the runs on main on Monday, September 28, 2026: first at commit 61d7374 ("Screen-reader checks" run 36494179002, "GUI (Xilem)" run 36494179017), then at 355f722 (runs 36499593368 and 36499593411). Where a run did not reach the question, it says why and what was changed.

1. **The tree dump.**
   - **Windows: yes, and stable.** accessibility-cli read the window through UI Automation in the background: 24 elements, the window with its title bar, the header's label and five buttons (Open, Font, Edit, Settings, Commands), the document, the "Reading" toolbar's six buttons, the status bar with its text, and the live region's "Opened Reading check." The second run's tree was the same, line for line ("Pass"). Building accessibility-cli takes about eight and a half minutes; it is cached right after the build.
   - **macOS: yes.** 26 elements through the AX API: the same header, toolbar, status, and live region. One thing to look at: the document is exposed as a `group` (AXGroup) with its text as the value, not as a text area, which may change how VoiceOver reads it; that belongs to the GUI (ADR-0027's bridge), not to this check. The system's menu bar, which macOS lists under the application (the Apple menu, "System Settings..., 1 update"), belongs to the runner's image, so the normalized tree now leaves it out.
   - **Linux: not yet.** accessibility-cli found "No windows" and pyatspi, asked in the same session, listed no application at all, while the AT-SPI check in the same job finds the window. The difference: the AT-SPI check listens for events before it reads, and AccessKit's AT-SPI adapter shows the window only while assistive technology is active. The dump now sets `IsEnabled` and `ScreenReaderEnabled` on the accessibility bus and keeps an event listener running while it reads. `windows.txt` stays as the diagnosis if it still fails.
2. **Guidepup and NVDA: yes.** Guidepup 0.34.0 drives NVDA against the native winit window on a Windows runner. The window took the foreground, the keys went through Guidepup, and NVDA spoke every announcement the GUI made: "Reading at 265 words per minute." on Play, "Paused." once on Pause, "Heading level 2: Second heading" on h, "Settings, dialog, 4 entries Named rates that F 8 cycles through. none The voice for each interface language, ..." on Ctrl+Comma, and "Settings closed." on Escape. Nothing was heard in the open step, because the script cleared NVDA's log after bringing the window forward, so the window's name was thrown away; the open step now keeps what was said since the launch, and the answer counts the GUI's own announcements (from its log) as well as its name. The settings dialog opened on a table setting, so NVDA read "4 entries", the empty value "none", and the help text of the next setting in one breath; that is for a person to judge in a session.
3. **Orca under Xvfb: yes, and every must check passes.** With AccessKit's AT-SPI adapter and no Collection, Orca said, in order: "Reading check - textweaver frame.", "Document document frame Reading check.", "Reading at 265 words per minute." on Play, "Paused." once on Pause, "Heading level 2: Second heading" on h, "Outline, 3 headings dialog" and "Reading check, level 1." in the outline. It also echoed the keys pressed ("left control", "space"), its default. AccessKit answers `GetStringAtOffset` but not the older `GetTextAtOffset`, so the session reads the caret's line from the whole text. The Opened announcement comes before the session listens, as ADR-0028 found on Windows, and stays a warning.
4. **VoiceOver on `macos-14`: no, so far.** `guidepup setup --ci` reported "Environment setup complete", and then Guidepup said "VoiceOver cannot be started." The session now runs on `macos-14` and `macos-15`, and the setup log adds the system version, System Integrity Protection's status, and whether AppleScript may drive VoiceOver, so the next run says why.

The sessions and the dumps are report-only on the step (`continue-on-error`), so the GUI workflow's result comes from its build and tests alone.

**Status update (Monday, September 28, 2026, the run on main at `80d94be`):**
- **NVDA: yes, again,** with the open step fixed: NVDA spoke 5 phrases from the textweaver window, 4 of the GUI's 5 announcements, and the window's name 0 times. When the window takes the focus, NVDA says neither the window's nor the document's name; that is on the UI refinement list.
- **VoiceOver: no, on `macos-14` and `macos-15`.** "VoiceOver cannot be started" on both, after setup; `macos-14` also failed to turn on "Do not disturb". The macOS tree dump stays the macOS check, and VoiceOver waits for a person.

**Status update (Tuesday, September 29, 2026): which checks are standing, and new ones.**

A check becomes standing, failing its job, when it has given the same answer on consecutive runs on main and a failure would mean a regression rather than a flaky tool. The evidence, from the runs on main:

- **The tree dump: standing on all three systems.** The Linux fix above worked: the dump now finds the window. Five GUI runs in a row dumped the same number of elements on each system: Linux 19, Windows 24, macOS 26, at 7971b5d, 28a864c, f5adb22, d0778dd, and f41ce4c ("GUI (Xilem)" runs 36580565643, 36591061058, 36601139882, 36603780384, 36617921982). The first four said "No baseline", because no earlier run on main had succeeded to compare with; the fifth compared with the fourth and said "Pass" on all three. So a run that dumps no tree now fails the job (`gui-xilem.yml`, the "Accessibility tree dump" steps). The comparison with main stays report-only: "Changed" is for a person to judge, and a new button is not a failure.
- **Orca: standing.** Every must check passed in three runs in a row ("Screen-reader checks" runs 36499593368, 36507144175, and 36589983581, at 355f722, 80d94be, and f057f14): the window on the bus and named after the document, the Text interface with the document's text, the caret following the reading, Paused once, the caret on the next heading, the outline opening, taking the focus, naming a heading and its level, and closing. The Opened announcement stays a warning. A failed must check now fails the Orca job (`a11y-tests.yml`).
- **NVDA: report-only still.** The answer is yes in the last two runs, but the same must check failed in all three: NVDA says neither the window's nor the document's name when the window takes the focus. That is the GUI's to fix (it is on the UI refinement list), not the check's; once it is fixed and three runs pass, the NVDA session can be made standing the same way.
- **VoiceOver: no.** "VoiceOver cannot be started" on `macos-14` and `macos-15` in every run. The macOS tree dump stays the macOS check, and VoiceOver waits for a person.

New checks, in their own jobs, beside these (see [Testing](../dev/testing.md#braille-real-engines-and-timing)):

- **Braille against liblouis: standing from its first run** (`second-tool.yml`, the braille job). Liblouis 3.39.0, built from its tarball pinned by SHA-256, reads each BRF file back to print; a fixture fails when it reads back more than 2 points worse than liblouis's own round trip of the same text, or below 90 percent. Evidence before the first CI run, from the same script in the development container on Linux: in grade 1, all six fixtures read back 100 percent of their words in order (liblouis's own round trip: 94.6 to 100 percent); in grade 2, 90.5 to 100 percent (its own: 73.9 to 100 percent), the same figures as liblouis's Windows build gave. A seventh fixture, the Obsidian sample, is left out because it has math: without MathCAT the writer turns formulas into spoken words that are not in the document's text, and it read back 87 percent. Math braille has its own tests (ADR-0036). The words that differ are all layout the writer adds on purpose (list numbers, task states, bullets, table cell separators), plus one choice: a straight double quote is written as a closing quote chosen by position, where liblouis writes the nonspecific quote.
- **Real engines: standing within their own workflow** (`engines.yml`, which no branch rule requires). espeak-ng on Linux; SAPI 5, OneCore, and the 32-bit host on Windows; AVSpeech on macOS 14 and 15, each writing a WAV file through `tw export-audio`. A step fails only when an engine cannot export at all, or falls back to another engine; the length, level, speed, word times, and the word times against the audio's silences are reported, never failed, until a few runs show what is normal for each engine. A voice the runner lacks is reported as skipped, with a reason, and passes.

  The first run (run 36633310405, on main at 0c53e37) failed three jobs, each for a reason the check was wrong to fail on. espeak-ng passed: word times for 51 of 52 words, sentence starts a median 11 ms before the end of the silence before them. On Windows, the SAPI 5 voice (David Desktop) and the OneCore voice (David) each exported with times for all 52 words; the 32-bit step failed because the runner image has no 32-bit voice at all (it lists David Desktop, Zira Desktop, and three OneCore voices), though the 32-bit host itself started and listed voices. It is now skipped with that reason. On macOS 14.8.9 and 15.7.9, AVSpeech exported 16.4 seconds of audio matching the timeline, but with no word times: no Apple backend implements `synthesize_utterance`, so exports keep ADR-0011's default of none (live speech has word events). That is now one Warning, and the reason AVSpeech drift cannot be measured from exported files yet. One thing to explain: the SAPI 5 and OneCore exports had the same length and word times to the millisecond, which suggests the voice choice may not reach the engine. Each run now compares the two files and warns when they are identical.
- **The sanitizer pass: one-time, by hand** (`engines.yml` with "sanitizer"): Valgrind's memcheck over `tw` while espeak-ng reads the fixture, the audio path that crosses into a C library. Its result will be recorded here; it is not a standing job.
- **The fake-host timing tests** that failed under load (`textweaver-sapi` and `textweaver-eci`) now check the order of events, not the clock. Run 40 times each with eight copies of both suites at once, both whole suites passed every time. Under the same load, each suite's pause test, which ran at four times real time, had failed once in 40 runs; both now run at real time.

## Consequences

- The GUI workflow takes a few minutes longer on each system the first time, while accessibility-cli builds; later runs restore it from the cache.
- The Linux job installs three more development packages (`libdbus-1-dev`, `libatspi2.0-dev`, `libx11-xcb-dev`) for accessibility-cli.
- Node joins CI, for Guidepup only, in `tools/a11y`; no Node tool is part of the product or the build.
- `atspi-session.py` imports the crate's `atspi-dump.py` rather than copying it, so a change there that renames `walk`, `find_app`, `ours`, or `bus_listing` must update the session too.
- The session workflow builds the GUI from cold caches; its runs are sequenced with the release dry run, so runners are not contended.
- With the dump standing, a failed build of accessibility-cli now turns the GUI job red, because the dump has no tool. It is pinned by commit and cached, so this should only happen when the pin is moved.

**Update, Wave 8a.** For the failing "names the window or the document" check: the document's node is now named with the document's title first ("Reading check, document", not "Document"), so the name NVDA and JAWS say when the document takes the focus names it; the window's title and its node's name were already "Reading check - textweaver". Checked in the harness tree only. The NVDA session must pass three runs in a row before it becomes standing.

## See also

- [Testing](../dev/testing.md#automated-screen-reader-checks): how to run the checks and read their reports.
- [ADR-0027: Xilem GUI](0027-xilem-gui.md): the accessibility checks that existed before.
- [ADR-0028: The Xilem GUI after the first accessibility session](0028-xilem-gui-after-the-session.md): the design the expectations follow.
