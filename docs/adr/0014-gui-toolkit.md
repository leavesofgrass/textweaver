# ADR-0014: GUI toolkit (wxDragon)

- Status: proposed (Wave 2 feasibility spike; decided at Integration 2)
- Date: 2026-09-25

## Context

textweaver's native GUI arrives in Wave 3 (plan, section 7). The plan names wxWidgets through wxDragon, the toolkit Paperback validated with NVDA, JAWS, and VoiceOver, with the `live-region` crate (by Paperback's author) for screen-reader announcements. The primary user reads with NVDA and JAWS, so the bar is not "it draws a window" but: native controls a screen reader already knows, the document's text and caret exposed to it, every control named, announcements that reach it, and a build the project can live with.

Wave 2 built a spike, `textweaver-gui`, over the current `textweaver-app` API (no other crate changed) and measured it on Windows 11 (the development machine) and on GitHub's Windows and macOS runners (`.github/workflows/gui.yml`).

## What the spike is

- **Window.** A frame with a menu bar (File, Reading, Navigate, Voice, Help) whose items carry the keymap's accelerators; a "Document" label and the document in a read-only multi-line `wxTextCtrl` with `wxTE_RICH2` (RichEdit 4.1+, class `RICHEDIT50W`, as Paperback uses); Play/Pause and Stop buttons; a two-field status bar (the latest announcement; line and percentage); a hidden zero-size label for `live-region`.
- **App.** `App` built as the terminal UI builds it (`setup.rs`), with the GUI keymap (`Frontend::Gui`) and a `LiveRegionAnnouncer` (the `Announcer` trait) in place of the log announcer. A 30 ms wx timer calls `App::poll_speech`; while reading, the spoken word is selected in the control (so the caret is on it and sighted users see it); otherwise the caret sits on the app's cursor.
- **Positions.** The control addresses text in UTF-16 units on Windows and macOS (a line break is one unit in RichEdit) and in characters on GTK; `positions.rs` maps through the document's `DisplayIndex` (ADR-0002). Checked with astral characters: a 1,000,090-character document is 1,005,096 control units, and the caret maps back to the right character.
- **Keys.** Menu accelerators (Ctrl, Alt, function keys, Escape) are translated by wxWidgets before the control sees the key; other chords go through `Keymap::lookup` in the key-down handler; printable Browse keys (`.` `,` `p` `h` `?` ...) through the char handler, so layout-shifted characters arrive as typed. Caret keys (arrows, Home, End, Page Up, Page Down, with Shift or Ctrl) and Tab are left to the native control, so the screen reader reads by character, word, and line as usual; before the next action the app's cursor follows the native caret.
- **Automation.** `--backend paced` (the speech crate's recording backend in timed mode) times words like an engine and plays nothing; `--background` shows the window minimized and never activated, without a taskbar button; `--log`/`--log-file` records announcements, commands, keys, caret syncs, and load times. `tools/uia-report.ps1` launches the GUI that way (and with `SW_SHOWMINNOACTIVE`), reads every control through UI Automation and MSAA, samples the caret while it reads, chooses menu items with `WM_COMMAND`, moves the caret with TextPattern, records the UIA notification events, and closes the window: no keys typed, no focus taken, no audio.

## Findings

### Build cost (Windows 11, Visual Studio Community 18, measured)

| Build | Time |
|---|---|
| First `cargo build -p textweaver-gui` in a fresh target (205 crates, wxWidgets 3.3.3 download and build) | 466 s (7 min 46 s) |
| of which the wxdragon-sys build script (bindgen, CMake/Ninja build of wxWidgets and the wrapper), sources already downloaded | 294 s |
| of which the `wxdragon` crate | 16 s |
| Edit in `textweaver-gui`, rebuild | 4.1 to 6.0 s |
| No-op build | 0.4 s |
| First `cargo clippy` after a build | 142 s; then 1.4 to 2.1 s |

Space: the wxWidgets CMake tree is 2.2 GB and the source 177 MB, per profile (release builds wxWidgets again). The debug executable is 17.8 MB.

On GitHub's runners (first run, nothing cached; CMake 4.4.3): `cargo build -p textweaver-gui` took 753 s on `windows-latest` and 459 s on `macos-latest`. With the caches warm (the wxWidgets trees kept outside `target/` by `actions/cache`, the rest by `rust-cache`), the next run built in 27.6 s on Windows and 9 s on macOS.

How the build finds its tools (`tools/build-windows.ps1`, which changes nothing on the system):

- **CMake and Ninja**: Visual Studio bundles both under `Common7\IDE\CommonExtensions\Microsoft\CMake\` (`CMake\bin\cmake.exe` 4.3.1 in Visual Studio 18, 3.31.6 in 2022; `Ninja\ninja.exe` 1.13.2). The script finds the newest Visual Studio with `vswhere`, puts both on `PATH` for the cargo process, and imports `vcvars64.bat` so wxdragon-sys's Ninja generator finds `cl.exe` 14.51.
- **libclang**: wxdragon-sys runs bindgen, which needs `libclang.dll`. Visual Studio does not install it by default (the "C++ Clang tools for Windows" component does), and it was not on this machine. The script uses `LIBCLANG_PATH`, else LLVM's installer location, else Visual Studio's Clang component, else a conda `libclang13` (copied under `target\libclang` as `libclang.dll`, the name clang-sys looks for). For Wave 3 on a clean machine: install LLVM (`winget install LLVM.LLVM`) or the Visual Studio component.
- **Application manifest**: without Common Controls v6, wxWidgets shows a modal warning at startup. `build.rs` embeds `textweaver-gui.manifest` (Common Controls v6, per-monitor DPI awareness) with the MSVC linker's `/MANIFEST:EMBED`, with no build dependencies.

### Accessibility tree (UI Automation and MSAA, `tools/uia-report.ps1`)

| Control | UIA | MSAA (what NVDA mostly reads for Win32 controls) |
|---|---|---|
| Frame | Window, name "Sample Markdown Document - textweaver" | client, same name |
| Document label | Text "Document", access key Alt+D | text, read only |
| Document | Document, name "Document", LabeledBy the label, access key Alt+D, patterns Text, Value (read-only), Scroll | editable text, name "Document", read only, focusable, value = the text |
| Play/Pause button | Button "Play" / "Pause" (follows playback), Alt+P, Invoke | push button |
| Stop button | Button "Stop", Alt+S, Invoke | push button |
| Status bar | StatusBar named by its first field; parts "Reading at 265 words per minute." and "Line 5, 22%" | status bar |
| Menus | MenuBar "Application"; items with the accelerator after a tab: "Open..." Ctrl+O, "Next sentence" Alt+., "Stop" Esc, "Keyboard shortcuts" F3, ... Document-only keys are shown in the label: "Play/Pause (Space)", "Read current word (W)" | menu items |

- **Text exposure**: the RichEdit's TextPattern returns the whole document (671 units for `fixtures/sample.md`, 10,049,904 for a 10-million-character file), and its selection is the spoken word while reading: sampled every 400 ms, "This", "paragraph", "bold", "italic", "inline", "and", "link", "the". Moving the caret through TextPattern (as arrow keys or a click would) and then choosing "Read current word" made the app follow it ("caret sync: control 204 -> CharPos(204)"), and Play then read from there.
- **Naming**: the Win32 convention (a static label just before the control) names the RichEdit in both UIA and MSAA. wxDragon's `set_accessibility_label` does the opposite of what it promises on Windows: it installs a `wxAccessible` on the control, which turns the MSAA role from "editable text" into "client", drops the read-only state and the value, and leaves the UIA name as "RichEdit Control". Do not use it on native controls on Windows; use labels (and on macOS, where it sets the native label, test separately).
- **Announcements**: every announcement arrived as a UIA notification event from the hidden label, with the text intact ("Paused.", "Heading level 2: Lists", "Line 9 of 38, 30 percent. Under heading Lists.", "Stopped."). Polite announcements map to `live_region::Priority::Medium` (UIA `CurrentThenMostRecent`: after the current utterance, superseding staler ones), assertive ones to `High` (`ImportantMostRecent`, interrupts). NVDA speaks UIA notifications (Paperback relies on it); JAWS honours the same contract but is untested here.
- **Keyboard** (one interactive run before the non-intrusive harness existed): Space paused, `.` and `,` moved by sentence and announced the sentence, Down arrow moved the native caret, `w` made the app follow the caret and say the word, Escape (a menu accelerator) stopped, Alt+. (a menu accelerator) moved on. Ctrl+Home, left to the control, did not visibly move the caret in that run; to be checked by hand.

### What NVDA would read (from the UIA and MSAA properties; Jon verifies by ear)

- On start: the window title "Sample Markdown Document - textweaver", then focus on "Document, edit, read only, multi line" and the line at the caret; the startup announcements "Opened Sample Markdown Document." and "Reading at 265 words per minute." as notifications.
- Arrow keys in the document: NVDA's own character, word, and line reading (native RichEdit).
- Tab: "Play button, Alt+P" (or "Pause"), "Stop button, Alt+S".
- Alt: the menu bar; items read with their shortcuts ("Next sentence Alt+period").
- NVDA+End: the status bar ("Reading at 265 words per minute. Line 5, 22%").
- Open questions for listening: whether NVDA reports the moving selection while textweaver reads (it normally reports selection changes only after its own commands; if it does, Wave 3 highlights with a background colour instead of the selection); how JAWS treats the notifications; that NVDA does not re-announce the document on window re-activation (Paperback fires a focus event itself for that).

### Large documents

`set_value` blocks the UI thread: 1 to 18 ms for 671 characters, 255 ms for 1 million characters, 9.3 s for 10 million. Once loaded, moving the highlight near the end of the 10-million-character document stayed under 30 ms per word (no slow-highlight log lines), and TextPattern read the whole text. The load time settles it: Wave 3 needs Paperback's window slicing (at most about 500,000 units in the control, positions kept document-absolute, `session/window.rs` and `text_window.rs` in Paperback), which the plan already calls for (section 6.1).

### macOS and Linux

On the macOS runner the GUI builds, passes clippy and its unit tests, and launches: it opened `fixtures/sample.md` (671 characters, 671 `NSTextView` units, loaded in 72 ms), announced "Opened Sample Markdown Document." and "Reading at 265 words per minute.", read with the silent backend, and closed itself. The Windows runner produced the same UI Automation report as the development machine (text exposed, caret following, 14 notifications, the window never activated). VoiceOver behaviour needs a real Mac. Linux (GTK) was not built; wxDragon supports it and the character-unit position mapping is in place.

## Decision

wxDragon meets the bar on Windows: native Win32 controls with correct roles, names, and states; the document's text and caret exposed through the RichEdit's own UIA provider (the same control NVDA and JAWS already handle in Paperback and WordPad); announcements through UIA notifications; menus with accelerators that screen readers read. The costs are a one-time seven-to-eight-minute first build, 2.4 GB of build tree per profile, libclang on developer machines, and an embedded manifest; incremental builds are seconds.

Adopt wxDragon with `live-region` for the Wave 3 GUI, with these rules:

- Native controls only for anything a screen reader must read; name them with a label before the control (Windows) rather than `set_accessibility_label`.
- The document is a read-only `wxTE_RICH2` control holding a window of the document; positions are document-absolute everywhere else and mapped through `DisplayIndex`.
- Caret keys and Tab stay native; the app follows the native caret. Everything else goes through the keymap.
- Announcements go through a `live-region` announcer; self-voicing is off by default in the GUI (the screen reader speaks) and on with `--self-voicing`.

## Wave 3 plan

1. **Promote the spike**: `textweaver-gui` as a workspace default member once CI caches the wxWidgets tree; the `textweaver-a11y/live-region` feature (plan amendment 2) holding the announcer now in `announce.rs`.
2. **Window slicing** for large documents (Paperback's scheme): load at most about 500,000 units, extend forward while reading, recentre on jumps; keep TextPattern offsets stable during say-all.
3. **Contract changes below**: a quiet cursor command for caret sync, and GUI keymap defaults that leave caret keys native.
4. **Main-thread engines** (ADR-0003 `REQUIRES_MAIN_THREAD`): drive a `ServiceCore` from the wx timer on the main thread, using `next_wakeup` for the interval, behind the same `SpeechService` API (a local variant with a `pump` method), so `App` does not change.
5. **Dialogs**: find, go to, bookmarks, voices, and settings as native dialogs (wx's `wxTextEntryDialog`, `wxSingleChoiceDialog`, and custom dialogs with labelled controls); the spike already routes `Effect::Prompt` and `Effect::ShowList` to them. The first custom dialog, View, Fonts (Agent W, 2026-09-25: family list with the bundled fonts first, size, Bold, live preview; `font_dialog.rs`), showed that a modal dialog takes the foreground even when the main window was started with `--background`; automated runs therefore open a dialog's controls in their own window, minimized before it is first shown (`font_dialog::open_background`), and `tools/font-dialog-report.ps1` checks that the GUI never holds the foreground. wxDragon wraps neither `wxFontEnumerator` nor `ShowWithoutActivating`; installed families come from `textweaver-fonts`' scan of the font folders.
6. **Edit mode** on the same control (not read-only) with the editor's echo events, once Agent D2's edit mode lands in `App`.
7. **Highlight**: keep selection-as-highlight if NVDA and JAWS stay quiet about it, else a background-colour highlight (`TextAttr`) that leaves the caret alone.
8. **Platforms**: macOS (VoiceOver on a real Mac; menu accelerators become Cmd; `NSTextView` positions are UTF-16) and Linux (GTK, Orca; character positions).
9. **Release builds**: wxdragon features off by default as Paperback does (`default-features = false`) to cut build time and size; a release job with the cached wxWidgets tree.

## Contract change requests (from the spike)

- `textweaver-app`: `Command::SetCursor(CharPos)` (or `App::set_cursor`): move the cursor without recording history, announcing, or reading; while paused, make it the resume point. The GUI calls it when the user has moved the native caret. The spike uses `GoTo(Char)` with the announcer muted, which records a history entry.
- `textweaver-keymap`: for `Frontend::Gui`, leave plain and Shift/Ctrl-modified arrows, Home, End, Page Up, Page Down, and Tab unbound in the Browse layer (they are native caret and focus keys in a GUI text control); the spike passes them through regardless.
- Workspace: `wxdragon = { version = "0.9", default-features = false }` for release builds (Paperback's choice), and a CI cache of the wxWidgets tree (`gui.yml` does this with `WXWIDGETS_DIR`, `WXWIDGETS_BUILD_DIR`, `WXDRAGON_SYS_BUILD_DIR`).

## Consequences

- One toolkit gives native controls on Windows, macOS, and Linux; screen readers read them without custom accessibility code.
- Contributors on Windows need Visual Studio with the C++ and CMake components and libclang; the build script finds them. The first build takes minutes; CI caches it.
- The GUI crate stays out of `default-members` and out of `ci.yml`'s workspace commands until Wave 3 promotes it.
