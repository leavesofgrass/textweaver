# ADR-0039: Automated screen-reader checks beside the listening sessions

- Status: proposed (Wave 5, Agent W5t). The tree dump becomes a standing check once it has been green on main; each session becomes standing only when its first answer is recorded below.
- Date: 2026-09-28
- Builds on: [ADR-0027](0027-xilem-gui.md) (the accessibility checks that exist), [ADR-0028](0028-xilem-gui-after-the-session.md) (the owner's session 1), and [ADR-0033](0033-gui-session-2-and-edit-mode.md) (session 2)

## Context

The GUI's accessibility bar is the owner's listening sessions with NVDA, JAWS, and the Braille display. Between sessions, CI checks what assistive technology is given, never what it says:

- Windows: the UI Automation report (`crates/textweaver-xilem/tools/uia-report.ps1`), in the GUI workflow.
- Linux: the AT-SPI check under Xvfb (`crates/textweaver-xilem/tools/atspi-check.sh` and `atspi-dump.py`).
- macOS: a smoke run that reads with the silent paced backend and exits.
- The harness tests, through `accesskit_consumer`.

Wave 5 asked four questions, each answered in writing before the next: can a tree dump on all three systems catch regressions between merges; can Guidepup drive NVDA against a native winit window on a Windows runner; can a scripted session under Xvfb check what Orca follows, without AT-SPI's Collection interface (AccessKit pull request 758 is a draft); and can Guidepup enable VoiceOver on a macOS runner.

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
- A change never fails the job: a person reads it and says whether it was meant. A dump that fails will fail the job once the dump has been green on main; until then its steps are report-only (`continue-on-error`), so a new tool cannot turn the GUI workflow red.

This is the regression check Wave 4 asked for: the tree of every merge, kept as an artifact, and the difference from main in words.

### The sessions: report-only, in their own workflow

`a11y-tests.yml` runs by hand (`workflow_dispatch`, with a choice of session) and when its own files change. Each session opens `fixtures/t/reading.md`, reads, pauses, moves to the next heading, opens a dialog, and closes it, and checks what was said against `fixtures/t/expected-phrases.json`: the design the owner chose in session 1 (announcements through the live region, heard once). "Must" patterns fail the session; "should" patterns are warnings; the patterns are loose, since each screen reader words things its own way.

- **NVDA** (`tools/a11y/nvda-session.mjs`, a Windows runner): Guidepup starts NVDA, the GUI (hybrid renderer, because WARP crashes on Vello's compute shaders) is brought to the foreground, and keys go through Guidepup: Ctrl+Shift+Space to play and pause, h, Ctrl+Comma, Escape. The report's first line answers the question: "Answer: yes" when NVDA spoke from the textweaver window.
- **Orca** (`tools/a11y/orca-session.sh` and `atspi-session.py`, Ubuntu under Xvfb): the session is checked through AT-SPI events, built on the crate's `atspi-dump.py`: the Opened announcement, the caret following the reading, Paused announced once, the caret on the next heading's line, the outline (Alt+O) taking the focus, Escape closing it. The tree is walked by hand; nothing uses Collection. Orca runs beside it with a debug log, and what it said at each step is listed, not checked.
- **VoiceOver** (`tools/a11y/voiceover-session.mjs`, `macos-14`): after `guidepup setup --ci`, the same steps with Guidepup's VoiceOver. If the runner cannot enable it, the setup log says why, and the macOS tree dump stays the macOS check.

A session becomes standing (its job no longer `continue-on-error`) only when its first run's answer is recorded in "The answers" below, and its must checks have passed three runs in a row.

### Rules

- **Runners only.** The session scripts refuse to run unless `GITHUB_ACTIONS` is `true`. NVDA and JAWS on a developer's machine are that person's working screen readers; a script must never take them over. Nothing is downloaded or installed on a development machine.
- **Pinned and checked.** accessibility-cli by commit and its lock file; Guidepup and its setup tool by version, with every package's integrity hash in `tools/a11y/package-lock.json`, installed with `npm ci --ignore-scripts`; NVDA by Guidepup's SHA-256 check.
- **No personal identifier** in any workflow, script, log, or artifact name.
- **A green run never replaces a session.** Every check here is a tripwire between the owner's sessions.

### What only the owner can check

- JAWS: no tool drives it in CI.
- The Braille display (the HumanWare Mantis Q40): what reaches 40 cells, and in what order.
- Whether speech is understandable and well timed: interruptions, double speech the logs cannot show, and the reading voice (Eloquence) against the screen reader's.
- Focus mode and browse mode as a person moves through them, and the feel of edit mode.
- Anything the design leaves to judgment: which highlight reads better, which announcement path is heard reliably.

## The answers

Filled in from the first runs; until then, each is "not yet run".

1. **The tree dump:** not yet run. Look for three artifacts (`a11y-tree-windows`, `-macos`, `-linux`), each with a `tree.txt` holding the window, the document, the toolbar, and the status bar, and "No baseline" in the summary on the first run.
2. **Guidepup and NVDA:** not yet run. Look for "Answer: yes" or "Answer: no" in the `nvda` job's summary, the foreground note, and the phrases under "open" and "settings".
3. **Orca under Xvfb:** not yet run. Look for the AT-SPI must checks, and whether Orca said anything ("Orca said: N phrases").
4. **VoiceOver on `macos-14`:** not yet run. Look at `voiceover-setup.log` for whether AppleScript control was allowed, then the answer line.

## Consequences

- The GUI workflow takes a few minutes longer on each system the first time, while accessibility-cli builds; later runs restore it from the cache.
- The Linux job installs three more development packages (`libdbus-1-dev`, `libatspi2.0-dev`, `libx11-xcb-dev`) for accessibility-cli.
- Node joins CI, for Guidepup only, in `tools/a11y`; no Node tool is part of the product or the build.
- `atspi-session.py` imports the crate's `atspi-dump.py` rather than copying it, so a change there that renames `walk`, `find_app`, `ours`, or `bus_listing` must update the session too.
- The session workflow builds the GUI from cold caches; its runs are sequenced with the release dry run, so runners are not contended.

## See also

- [Testing](../dev/testing.md#automated-screen-reader-checks): how to run the checks and read their reports.
- [ADR-0027: Xilem GUI](0027-xilem-gui.md): the accessibility checks that existed before.
- [ADR-0028: The Xilem GUI after the owner's session](0028-xilem-gui-after-the-session.md): the design the expectations follow.
- [Wave 5, recalibrated](../research/wave5-recalibrated.md): the brief.
