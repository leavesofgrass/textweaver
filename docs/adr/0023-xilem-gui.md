# ADR-0023: Xilem GUI

- Status: accepted (Wave 3, first milestone; Agent W3b)
- Date: 2026-09-26
- Supersedes: [ADR-0014](0014-gui-toolkit.md) (the wxDragon spike), which stays as the fallback described below.

## Context

On Saturday, September 26, 2026, Jon chose Xilem, Linebender's all-Rust toolkit, for textweaver's GUI on every platform, to "keep as much of it Rust as I can". The stack is Masonry (Xilem's widget layer), Vello for rendering, Parley for text layout, AccessKit for accessibility, and winit for windows. The research is in [docs/research/xilem-gui.md](../research/xilem-gui.md). Its findings that shaped this record:

- AccessKit is strong on all three platforms, but Xilem pinned AccessKit 0.24 and `accesskit_winit` 0.32.2, which miss the fix that lets Orca find an AccessKit app (Xilem issue #1733).
- Masonry's read-only text cannot take focus, Parley's editor has one style for the whole text, there is no announcement API, and there are no menus.
- A document must be exposed as text runs of at most about 255 characters, and a large document cannot be laid out or exposed all at once.

The bar is the one the wxDragon spike met on Windows: a document a screen reader reads as text, with its caret following the spoken word; every control named; announcements that reach the screen reader; keyboard-only operation; and a window that automated checks can drive without ever taking the foreground.

## Decision

### The stack, pinned

- **Xilem and Masonry at revision `271a27a6d4a930f7878d404f9014e3c50a3a9b88`** (main, "xilem: Expose imaging backends as features (#1847)", Monday, September 14, 2026; the crates still call themselves 0.4.0). They are vendored under `third_party/xilem`, trimmed to `masonry`, `masonry_core`, `masonry_winit`, `masonry_imaging`, `masonry_testing`, `tree_arena`, and `include_doc_path`, plus `xilem`, `xilem_core`, and `xilem_masonry` for later. The folder is its own Cargo workspace, excluded from ours and used by path, so it builds offline, in Docker, and in CI with no git dependency.
- **AccessKit 0.25.1, `accesskit_winit` 0.34.1, `accesskit_consumer` 0.39.1** (released Friday, September 25, 2026; the 0.25 and 0.34 series began on Saturday, August 29, 2026). The move from 0.24 needed two renames in Masonry (`Tree` to `TreeInfo`, `Node` to `NodeRef` in the consumer).
- **Parley 0.8.0**, vendored and patched into the workspace (`[patch.crates-io]`) so its `accesskit` feature uses AccessKit 0.25. No other crate here uses Parley.
- **Vello** through `masonry_winit`'s `imaging_vello` feature; the screenshot tool uses Vello's CPU renderer.
- Every change to the vendored code is in `third_party/xilem/textweaver.patch`, described in `third_party/xilem/TEXTWEAVER.md`.

### Masonry, not the Xilem reactive layer

The GUI is written against Masonry and `masonry_winit` directly, as the research recommended ("depend on Masonry more than on Xilem"). The app core already holds all state and returns effects; a driver (`gui.rs`) turns them into widget edits, the way the wxDragon spike did. Xilem's view layer is vendored and can be adopted later, for dialogs for instance, without another vendoring step. The look is Masonry's default one, which is what Xilem's `to_do_mvc` example shows, recoloured from textweaver's themes.

### Our own `DocumentView`

`crates/textweaver-xilem/src/document.rs`:

- **One focusable node**, role `Document`, read-only, named "Document" with the document's title as its description. Its children are `TextRun` nodes (not widgets), built by `runs.rs`: at most 255 grapheme clusters each (a screen reader's "character" is a cluster), split at word starts, with `character_lengths`, `word_starts`, and text direction; a paragraph's last run ends with its `\n`, which is how AccessKit finds paragraph ends.
- **Lines.** Runs on one visual line are chained with `next_on_line`. A paragraph that has been laid out (it has been on screen) is split at its visual lines, so UI Automation's and AT-SPI's line unit reads what the eye sees; a paragraph not yet laid out is one line.
- **Stable ids.** A run is keyed by its document position. Moving the spoken word rebuilds only its paragraph's runs; every other run keeps its id and is not sent again. A full rebuild (the tree enabled, the window resized) is detected through the root widget and sends every run.
- **The caret is the text selection**; while reading, the caret sits on the spoken word (`--select-spoken` selects the word instead, for comparison by ear). The spoken word is its own run with a background colour and foreground colour attribute; headings carry their size and weight.
- **Keys.** Arrows, Home, End, Page Up, Page Down, with Shift to select and Ctrl for words, paragraphs, and the document's ends, move the caret in the view; the move is reported to the app as `Command::SetCursor` (quiet), as the spike did with the native caret. Ctrl+C copies the selection. Every other key reaches the keymap.
- **Screen-reader actions.** `SetTextSelection` moves the caret (the report checks this), `Focus`, and `ScrollIntoView`.
- **Painting.** Parley lays out only the paragraphs on screen, from a cache; the spoken word's band is painted under the text and its text painted again in its own colour, clipped to the band, so a highlight move needs no relayout.

### Windowing

The view holds the app's `DocWindow` (Agent W3a, [ADR-0024](0024-app-core-for-the-gui.md)): about 120,000 UTF-16 units around the focus, aligned to paragraphs, sliding while reading and recentring on jumps, and reloaded when `Session::revision` says the text changed. Positions stay document-absolute. The budget is smaller than the app's default of 500,000: it was chosen by measurement (below), so a rebuild of the view and its accessibility tree stays well inside a frame. `window.rs` turns the window into paragraphs with heading levels and styles.

### Announcements

A live region (`widgets.rs`, `Announcer`): an invisible widget whose children are the latest messages, each a **new** AccessKit node (role `Label`, the text as its value, `live` polite or assertive). AccessKit raises UI Automation's LiveRegionChanged, and AT-SPI's Announcement, when such a node appears, so saying the same words twice is announced twice. A full tree rebuild drops old messages rather than announcing them again.

### Windows, dialogs, and focus

- The window: a header (document title, Open, Fonts, and Commands buttons), the document, a "Reading" toolbar (Play or Pause as the primary button, Stop, Previous and Next sentence, Slower, Faster), and a status bar whose name is its text. Each is a named region with a role (`Banner`, `Toolbar`, `Status`). Buttons are our own `ActionButton`, because Masonry's `Button` cannot carry an accessible name, a keyboard shortcut, or a description.
- **Dialogs are in the window**, not separate windows: a modal card with the `Dialog` role over a dimmed page; while one is open, the window behind it is disabled and hidden from screen readers. An in-window dialog never takes the foreground, so `--background` runs can open them. Prompts use Masonry's text field with an accessible label (a small Masonry patch); lists use our `ChoiceList`, one focusable `ListBox` whose options are AccessKit nodes, with arrows, Home, End, paging, first-letter search, Enter, and Escape. The app's lists (bookmarks, help, voices, the library, and the settings screen) are the app's `ListModel`: the list sends its keys as `Command::ListKey` and shows the model's items and focus in place, so Left and Right change a setting (Settings: `Ctrl+Comma`). The app does not announce each item (`App::set_announce_list_focus(false)`), because the options are AccessKit nodes and the list's active descendant is the screen reader's focus; `--app-list-announcements` turns them back on for comparison. The command palette filters as you type and says how many commands match; it stands in for a menu bar. The font chooser (Fonts) is ported from the spike: the families, bundled first, then a size.
- `--background` opens the window without activating it, off screen, and with no taskbar button on Windows; the report checks that the foreground window never changes.

### Themes and fonts

- `theme.rs` maps a textweaver theme onto Masonry's default properties: Galaxy by default. Panels have a surface colour, a hairline border, 10-pixel corners, and a soft shadow; buttons and fields have 6-pixel corners; focus is a 2-pixel ring. Unit tests hold Galaxy, Galaxy Light, Contrast, and High Contrast to 4.5 to 1 for text (7 to 1 in high contrast) and 3 to 1 for the focus ring against the page, the panels, and the buttons.
- Review screenshots, drawn with `textweaver-xilem --review-screenshots DIR` (Vello's CPU renderer, no window): Galaxy at 100% and 200%, Galaxy Light, High Contrast, and a list dialog at both scales, in [docs/screenshots/xilem-gui](../screenshots/xilem-gui/).
- The bundled fonts (Atkinson Hyperlegible Next and Mono, OpenDyslexic) are loaded straight into Parley's font collection; nothing is registered with the operating system, so macOS needs no bundle folder for them. The reader's `[reading_aids.font]` setting becomes a Parley family list.

### The accessibility bar, and how it is checked

- **Windows:** `crates/textweaver-xilem/tools/uia-report.ps1`, through UI Automation only. It passes (Saturday, September 26, 2026, Windows 11 26200): Document with TextPattern and ValuePattern (read-only); the text; UIA lines match the visual lines; the caret follows the paced reading; the spoken word's background colour is reported; caret moves made through TextPattern are followed by the app, and Play reads from there; buttons named and pressed with InvokePattern; the status bar named by its text; every announcement appears as a new live element; the window never takes the foreground. Managed UI Automation cannot subscribe to LiveRegionChanged (it predates the event), so the report watches for the new live elements; hearing them is on Jon's checklist.
- **Linux:** `tools/atspi-check.sh` runs the GUI under Xvfb with a private D-Bus session and the AT-SPI bus, and `tools/atspi-dump.py` (pyatspi) checks the tree, the Text interface, the attributes at the caret, and the caret-moved and announcement events. See "Results" for where it stands.
- **macOS:** a smoke test in CI launches the GUI with the paced backend and checks that it reads and exits; VoiceOver needs a person.
- **Unit and harness tests** (`tests/document_view.rs`) check the same tree through `accesskit_consumer`, the crate the platform adapters use.
- **People:** Jon tests with NVDA and JAWS; his checklist is in the W3b report and in `docs/tasks.md`.

## Measurements

Measured on Saturday, September 26, 2026, on the development machine (Windows 11, 12 threads), release build, with the harness test `large_documents_open_and_highlight_quickly` and the GUI's `--log`. The test document is 10,000,054 characters of plain text in about 22,800 paragraphs; the window around the caret is 120,286 characters.

- **Opening a 10-million-character document: about 88 ms**, under the 300 ms goal. Of that, `App::open` (loading, the rope, the markers) took 63 ms; building the window's paragraphs and styles, 2.5 ms; and the view's first layout, its text runs, and the whole accessibility tree, 22 ms. A 1-million-character document: `App::open` 31 ms, the rest the same (the window is the same size).
- **Moving the highlight: median 0.29 ms, worst 0.89 ms per word** (20 moves, each an edit, a layout check, a paint, and an accessibility update including AccessKit's own tree processing), against a 30 ms goal.
- In a debug build, the same steps take 528 ms, 3 ms, and 299 ms, and a highlight move 1.8 ms.
- **Memory:** 98 MB working set with the sample document open and reading, against about 430 MB reported for "Hello, World" in Xilem issue #918; 111 MB working set (305 MB private) with the 10-million-character document, most of it the app's document.
- **Build:** a cold `cargo build -p textweaver-xilem` took 19 minutes on this machine (the app's dependencies, wgpu, and Vello); a release build 15 minutes; an edit in the crate rebuilds in about 10 seconds.
- **The first launch** of a freshly built binary once took more than 20 seconds to show its window (Windows Defender scanning the new file, or the first shader compile); later launches open in about a second.

## Where AccessKit and Masonry fall short, and what we propose upstream

- **Masonry: update to AccessKit 0.25 and `accesskit_winit` 0.34.** Our patch is small (two renames and version numbers) and replaces the test harness's `unsafe` `NodeId` write with `accesskit_consumer`'s public lookup (AccessKit issue #701). Proposed as one pull request.
- **Masonry: exit from an async action.** `masonry_winit` only honours `DriverCtx::exit` after window events; an app that closes on a timer or a background message hangs. A two-line fix; proposed as a pull request.
- **Masonry: an accessible label for `TextArea` and `TextInput`**, and a way to name buttons (`Button` takes its name from its child only). Proposed as an issue with our patch.
- **Masonry's test harness: register fonts, and draw at the scale factor.** The harness laid widgets out at the scale factor but drew the scene unscaled. Proposed as a pull request.
- **Masonry: a focusable read-only document widget, ranged styles, Page Up and Page Down, an announcer.** `DocumentView` and `Announcer` are written so they could move upstream; we will offer them once Jon's listening session settles the design.
- **AccessKit: a UI Automation Notification event.** AccessKit raises only LiveRegionChanged. NVDA speaks it; JAWS's support is inconsistent. If JAWS stays quiet in Jon's session, we propose an optional `UiaRaiseNotificationEvent` for announcements (an AccessKit issue first).
- **AccessKit: text ranges that survive run changes.** A UIA text range points at a run node; when the spoken word moves, its paragraph's runs are replaced, and a range a screen reader holds into them goes stale (`ElementNotAvailable`). The report retries; NVDA re-reads on each event. If it bothers a screen reader in practice, the choice is between keeping the run structure fixed while reading (and giving up the background-colour attribute) and asking AccessKit for positions keyed by character offset. Recorded as an open question.
- **AccessKit on Linux:** Collection (#758) for Orca's structural navigation, shortcuts (#668), and heading level (#665) remain open upstream; we will help test them.

## The fallback

The wxDragon spike (`crates/textweaver-gui`, ADR-0014) keeps building and passing its own UI Automation report on Windows until this GUI passes the same report and Jon's NVDA and JAWS session. Only then is it removed (Wave 4, Agent W4a). If Xilem development stops or a screen reader cannot be served, the plan in the research page stands: wxDragon, and on Linux wxWidgets on GTK 3 with the `live-region` crate.

## Built on W3a's app core

The GUI started with thin local adapters while Agent W3a built the app pieces, and switched once W3a's branch was merged in (Saturday, September 26, 2026):

- **Window:** the app's `DocWindow` and `follow_session` (the local `TextWindow` is gone).
- **Waker:** `App::set_waker` posts a tick to the event loop whenever speech or background work rings, one per burst; a ticker thread covers the app's own timers, sleeping for `App::tick_interval`. The 30 ms polling is gone.
- **Lists:** the app's `ListModel` through `Command::ListKey` and `Command::ListFocus`; the settings screen works in the GUI through it.
- **Still local:** prompts send their answer with `Command::Answer` rather than `PromptKey::SetText`, and the command palette filters with `App::palette_candidates`; both could move to the app's `PromptModel`. The font chooser's lists are the GUI's own.
- **Not used yet:** `Command::ReplaceRange` (edit mode, Wave 4) and a dedicated settings dialog built from `SettingsSchema` (the settings screen's list serves meanwhile).

## Consequences

- The GUI is all Rust: no C++ toolkit, no CMake, no libclang. A cold build of the crate takes about 19 minutes on the development machine, most of it the app's own dependencies and wgpu; incremental builds take seconds.
- About 4 MB of vendored source is ours to update by hand, with a patch file that says what changed.
- The accessibility code is ours: it can do exactly what textweaver needs, and it must be tested by people with screen readers.
