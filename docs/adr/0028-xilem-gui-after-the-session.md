# ADR-0028: The Xilem GUI after the first listening session

- Status: accepted (finished after the first accessibility listening session, Sunday, September 27, 2026)
- Date: 2026-09-27
- Builds on: [ADR-0027](0027-xilem-gui.md) (the Xilem GUI)

## Context

ADR-0027 left three questions for the first NVDA and JAWS listening session on the Xilem GUI:

- Do announcements reach JAWS? AccessKit raises only UI Automation's LiveRegionChanged, which NVDA speaks and JAWS has handled inconsistently.
- Can a screen reader reach list options and settings that are scrolled out of view? The UI Automation report saw 13 of 15 settings sections.
- Why did the GUI's memory grow from 98 MB to 172 to 187 MB during that earlier work?

This record documents what changed before the session, what was measured, and the two questions the session decides. It also records the answers and the decisions.

## Two ways to announce

The GUI now has two announcement paths, chosen with `--announce live|uia` or, in `settings.toml`, with `announce = "live"` or `"uia"` in a `[gui]` table. The command line wins over the setting; the live region is the default.

- **`live` (the default):** as in ADR-0027. Each message is a new AccessKit node with its live setting, and AccessKit raises LiveRegionChanged (AT-SPI's Announcement on Linux).
- **`uia` (Windows only):** for each message the GUI raises a UI Automation Notification event with `UiaRaiseNotificationEvent`, kind "other", activity id `textweaver.announcement`. Assertive messages are sent as "important, most recent"; polite ones as "all". The message nodes stay in the tree with their live setting off, so object navigation still finds them and no screen reader is told twice. On other systems `uia` falls back to the live region and says so once at startup.

How it is raised matters. Raised on the window's host provider (`UiaHostProviderFromHwnd`), the events never reached a client, because the host provider is a client-side provider. They are raised from a small server-side provider (`widgets::notify`) whose host is the window; UI Automation merges it with the window's element, whose provider is AccessKit's.

The `[gui]` table was first read from the store's preserved unknown tables (`Settings::extra`), because the store and the schema were outside this change's initial scope. **Update:** the setting is now a proper one in the four places: `GuiSettings { announce }` in `textweaver-store` (default `"live"`; a bad value warns with the other settings and keeps the default), the export fixture, a `gui.announce` choice in the settings schema (the settings dialog shows it under "Window"), and the GUI reading `settings.gui.announce`.

**Checked:** the UI Automation report (`tools/uia-report.ps1 -Announce live` and `-Announce uia`, Sunday, September 27, 2026, Windows 11 26200) passes both ways. With `live`, LiveRegionChanged events arrive for each message (10 in the run) and no Notification events. With `uia`, Notification events arrive with their text and activity id (10) and no LiveRegionChanged events. Managed UI Automation gained LiveRegionChanged in .NET Framework 4.7.1, so the report now subscribes to it instead of only watching for new elements.

**Found:** both paths lose the announcements made at startup, before any UI Automation client has asked for the window: "Opened Sample Markdown Document." and "Reading at 265 words per minute." in the report's runs. The report's client connects about two seconds after launch. A screen reader that is already running asks sooner, when the window appears, so a person using one may hear them; the listening session checks it.

## Clipped options

The hypothesis in the plan held. `ChoiceList` and `SettingsGrid` clip their painting, and Masonry marks any widget with a clip path as clipping its children. AccessKit's filter, which the UI Automation and AT-SPI adapters share, then leaves out every child whose bounds lie outside the parent's, except the first one past each edge (kept so a screen reader can scroll to it). Measured through `accesskit_consumer::common_filter` in the harness:

- a list of 40 options kept 13 (the 12 in view and one more);
- the Speech section kept 12 of its 20 settings.

Both widgets now clear the flag in their accessibility pass and keep the paint clip, so every option and setting is in the tree with its scrolled bounds. In the UI Automation report the settings dialog shows all 15 sections, and the font family list all 194 of 194 options, before and after the last one is scrolled into view with ScrollItemPattern. No "read the whole list" fallback was needed.

Three related faults were found and fixed on the way:

- **"2 of" with no size.** AccessKit's position in set is zero-based (the adapters add one), and the adapters read the set's size from the container. The lists and the form set one-based positions and the size on each item, so UI Automation reported the first section as 2 and gave no size. They now report "1 of 15" and "194 of 194".
- **Blank rows.** Both widgets build their rows' text in `layout`, but moving the selection asked only for a paint, so rows that scrolled into view were drawn blank and the form kept the previous setting's help. Screen readers were not affected; sighted helpers were.
- **List options now offer ScrollIntoView**, which the list already handled.

## Memory

Measured on Sunday, September 27, 2026, on the development machine (Windows 11, NVIDIA GeForce RTX 3060), release builds, the GUI with `--background --log`, and PowerShell's `Get-Process` (working set and private bytes) and the private working set from the performance counters, read 10 to 15 seconds after launch, two to four runs each (the list gives the steady values; runs that had reached them agreed within 13 MB). Nothing played audio: `--no-speech`, or the silent paced backend reading.

The reader's `publish` feature is already off in the GUI: the workspace declares `textweaver-app` with `default-features = false`, and `textweaver-xilem` turns nothing on, so no citation, conversion, or rendering crate is in its tree. The engine crates are not optional in `textweaver-engines`, so excluding each in turn needs changes outside this change's scope. Instead, `examples/memory_probe.rs` builds the app exactly as the GUI does, with every engine, opens the document, and reads with the paced backend, with no window and no renderer. That bounds what any engine or data file could cost.

- **The app alone, sample document, reading:** 11.5 MB working set, 3.6 MB private bytes.
- **The app alone, 10-million-character document, reading:** 23 MB working set, 15 MB private bytes.
- **The GUI, Vello (the default renderer), no document:** 169 MB working set (112 MB private working set), 650 MB private bytes.
- **The GUI, Vello, sample document, reading:** 173 MB (114 MB private working set), 655 MB private bytes.
- **The GUI, Vello, 10-million-character document, reading:** 189 MB (130 MB private working set), 667 MB private bytes.
- **The GUI, Vello's hybrid renderer (`--no-default-features --features renderer-hybrid`), no document:** 148 MB (92 MB), 350 MB private bytes.
- **Hybrid, sample, reading:** 151 MB (94 MB), 350 MB private bytes.
- **Hybrid, 10 million characters, reading:** 164 MB (107 MB), 365 MB private bytes.
- **Vello with `WGPU_BACKEND=vulkan`, sample, reading:** 146 MB, 635 MB private bytes. **With `WGPU_BACKEND=dx12`:** 195 MB (peak 369 MB), 620 MB private bytes.

What this says:

- **The app, the engines, and their data are about 11 MB.** Nothing large is loaded at startup: not Piper, Whisper, OCR, or the lexicon. Lazy loading in the app would save almost nothing, so no change is proposed there.
- **The window costs about 160 MB of working set, and nearly all of it is the GPU stack.** The largest modules loaded are NVIDIA's shader compiler (`nvgpucomp64.dll`, 94 MB image), its Direct3D driver (`nvwgf2umx.dll`, 86 MB), and its OpenGL and Vulkan driver (`nvoglv64.dll`, 47 MB). wgpu enumerates every backend by default, so both drivers load. Most private bytes (650 MB) are memory the drivers commit and never touch; the private working set is about 113 MB.
- **The document view's own share is small:** the 10-million-character document adds about 16 MB over the sample in the GUI, and 11.5 MB of that is the app's text.
- **The 98 MB measured in the earlier work was most likely read while the renderer was still starting.** Runs measured about ten seconds after launch sometimes read 89 to 94 MB (35 to 46 MB private working set) and then rose to the steady 170 MB. The growth ADR-0027 recorded is the renderer finishing its start, not a leak: no run grew after the first 15 seconds.

Nothing inside `textweaver-xilem` accounts for more than a few megabytes, so nothing was changed there. Proposed, not made (all outside the crate or a design choice):

1. **Vello with area antialiasing only.** `imaging_vello` creates Vello's renderer with `RendererOptions::default()`, which compiles the pipelines for three antialiasing methods, and uses only one (`AaConfig::Area`). `AaSupport::area_only()` should cut shader memory and startup time. A small change upstream (imaging_vello or Masonry); not measured.
2. **One wgpu backend on Windows.** Limiting wgpu to Vulkan saved about 25 MB of working set here; DirectX 12 cost more. This depends on the graphics driver, so it needs measuring on other machines first. `WGPU_BACKEND` already selects one for testing.
3. **The hybrid renderer on Windows.** It halves private bytes (650 to 350 MB) and saves about 20 MB of working set, at some cost in rendering speed on large windows. A choice to make after the session, with the screenshots at 100% and 200% compared.

## The two questions the session decides

1. **Announcements: `live` or `uia`?** Pause, Stop, and a rate key are tried with each, in NVDA and then in JAWS, to say which is heard once, reliably, without cutting off the reading. The default follows that choice. If JAWS needs `uia` and NVDA works with both, `uia` becomes the Windows default, and the store and schema get the `[gui] announce` setting properly.
2. **The highlight: selection or background?** While reading, the caret sits on the spoken word, which has its own background color (the default), or `--select-spoken` selects the word. Testing says which reads better in NVDA and JAWS, with speech and with the Braille display. That decides the document view's design, built on after the session.

The answers are recorded here, the status changes to accepted, and whichever option is not kept is removed, or kept as a setting when preferred.

**The answers (Sunday, September 27, 2026, after the first screen reader and Braille display checks):**
1. **Announcements:** the live region stays the default. `uia` stays available as an option.
2. **The highlight:** the background color stays the default, and the caret is not moved by selection. `--select-spoken` stays available as an option.

This builds on these answers, adds `[gui] announce` to the store and schema so it appears in the settings dialog, and changes the status to accepted.

## Decision

The first checks with NVDA, JAWS, and a Braille display were good, so the designs prepared before the session stay as they are:

- **Announcements:** the live region is the default everywhere. UI Automation notifications stay as an option, `[gui] announce = "uia"` in the settings (the settings dialog, under "Window") or `--announce uia` for one run. The command line wins over the setting.
- **The highlight:** the spoken word has its own background color, and the caret (the document's text selection, collapsed) sits at its start. The word is not selected. `--select-spoken` stays as an option for anyone who prefers the word selected. Everything later in the document view builds on this design: the window slide, the reading aids, and edit mode (ADR-0033).
- **The hybrid renderer** stays a build option (`--no-default-features --features screenshot,renderer-hybrid`), not the default. The session did not raise memory, and the hybrid renderer's cost in drawing speed on large windows has not been measured with the reading aids drawn. Revisit it with the second listening session.

The key named in "No document is open" now comes from the keymap (`named_key`), and the list introduction (`ListKey::Introduce`) and the title line's parts (`App::title_parts`) reach the GUI as they reach the terminal reader.

## Built on the decision

**A window slide keeps the screen reader's place.** The GUI holds a window of about 120,000 UTF-16 units around the focus (ADR-0027). When reading reaches its edge, the window slides (`WindowChange::Forward` or `Backward`). Before, the view replaced every text run on any change, so every node a screen reader was on vanished mid-read, the problem star's pagination had. Now a slide keeps the runs that stay: the same node ids with the same text, their layouts and visual lines carried over, and the ids of runs that left the window dropped after the next accessibility pass. The caret, the collapsed text selection on the spoken word, is sent again on the new tree. A new document or a jump still replaces every run. Checked in the harness and in the UI Automation report's `-WindowEdge` probe (reading at 900 words per minute through a slide, in `--background`: the caret stays on the spoken word, and the Pause announcement after the slide arrives).

**Reading aids** (ADR-0022), all drawn only, so the text runs a screen reader gets never change:

- Text spacing sets Parley's line height, letter spacing, and word spacing, and the paragraph gap, in multiples of the font size (WCAG 1.4.12's units). The terminal rounds them; the window uses them exactly.
- The reading ruler builds the aids crate's `ViewRow`s from the visual lines on screen and draws `ruler_rows`' marks: a band on the reading line with a bar at its start (a shape, so color is not the only cue), a paler band around it, and a dimming mask outside when asked. The band colors are tints of the theme's focus color, adjusted until the text keeps its contrast on them.
- Bionic reading and difficult words are spans from the app (`App::bionic_ranges`, `App::difficult_ranges`); difficult words get a thick underline in the text's own color.
- RSVP is its own strip in the window's column, between the document and the toolbar, so it can never cover the text or the caret. The flashing word is a hidden node, never live and never focusable; beside it a status node ("RSVP paused, word 2 of 109") has its live setting off. The app announces what RSVP does (on, off, paused) once, through the announcer. RSVP's nine positions move the word left, center, or right in the strip; the words before and after sit to its left and right.

**Parity with the terminal reader** needed no new GUI code: the outline (Alt+O), the notes list (Ctrl+Shift+N), the access modes (Alt+Shift+A), and tables and links by key reach the app through the keymap, and the lists open as the GUI's list dialogs through `Effect::ShowList`. A test drives each from the document. In a list, F1 and Say Status (Alt+End) now repeat the list's introduction, as in the terminal.

## Status update: the renderer options measured (Monday, September 28, 2026)

The "Memory" section proposed three renderer options. The first two were measured on Parley 0.11.1, on the same machine (NVIDIA GeForce RTX 3060), release builds, `--background` with the silent paced backend, read 15 seconds after launch, three runs each, while other builds ran concurrently on the machine. The sample document, reading:

- **Default (Vello, every wgpu backend):** 178 MB working set, 117 MB private working set, 654 MB private bytes.
- **Vello with area antialiasing only** (`AaSupport::area_only()`, measured with a scratch copy of `imaging_vello`, not committed): 175 MB, 115 MB, 652 MB. With the 10-million-character document, 191 MB against 194 MB. About 3 MB, or 2 percent. Startup time was not measured separately. Not worth vendoring `imaging_vello` for; it is a one-line change upstream (`imaging_vello` creates its renderer with `RendererOptions::default()` and only ever renders with `AaConfig::Area`), proposed there.
- **Vulkan only (`WGPU_BACKEND=vulkan`):** 152 MB, 94 MB, 635 MB: 26 MB less working set, 23 MB less private working set. With area antialiasing too, 149 MB.
- **Direct3D 12 only:** 200 MB, 148 MB, 624 MB: 22 MB more.
- **OpenGL:** one run failed to start ("could not lock adapter context"), the other used 324 MB. Not an option on this machine.

**Decision.** One backend is now an opt-in, not a default: `--graphics vulkan` for one run, or `graphics = "vulkan"` in the `[gui]` table of `settings.toml` (also `dx12`, `metal`, `gl`, and `auto`, the default). The saving depends on the graphics driver and has been measured on one machine only, so the default stays `auto` until the third listening session and a measurement elsewhere. `WGPU_BACKEND`, when set, still wins. The GUI sets it at the start of `main`, before any thread exists (one small, commented `unsafe` call, since setting an environment variable is unsafe in Rust 2024). The setting is read from the `[gui]` table's preserved keys, as `announce` was before the update above: `textweaver-store` belongs to separate work, so the store type, the schema entry, and the export fixture are proposed for a later pass rather than made here. The hybrid renderer (item 3) stays a build option; nothing new was measured for it.

**Syllables drawn (same pass).** Syllables (Alt+Shift+Z, `[reading_aids] syllables`) are now drawn in the window as the terminal draws them, at the app's break positions (`App::syllable_breaks`). The text keeps its own bytes: the char before each break is laid out with extra letter spacing as wide as the separator, and the separator (a middle dot by default) is laid out once per paragraph in the paragraph's font and painted into that space on the line's baseline. So the text runs, the caret, hit testing, and the spoken word's band are unchanged, and no line can break inside a word at a separator (Parley allows a break after an inline box, which is why a box was not used). The harness checks that the document's text is the same with syllables on, that no middle dot reaches it, and that the run nodes keep their ids. Building the window's model with syllables on took about 120 to 250 ms for the 10-million-character document's window on the loaded machine (40 to 100 ms without), once per window slide. Difficult words were already drawn (see above).

**A fix on the way.** A reading aid turned on or off rebuilt the window's model as a slide, and a slide kept the old paragraph layouts wherever the text was the same, so bionic reading or difficult words switched on or off could keep their old drawing until the paragraph was laid out again. A slide now keeps a layout only when the paragraph's styles and syllable breaks are the same too; the run nodes (and the screen reader's place) are kept either way.

## Status update: the memory proposals decided, and startup announcements (Tuesday, September 29, 2026)

**The three memory proposals, decided.** Measured again on the same machine (NVIDIA GeForce RTX 3060), release builds of this branch, `--background` with the silent paced backend, the sample document, read 15 seconds after launch, three runs each, with other agents building (the first run of each was still settling and is left out):

- **Vello (the default):** 179 to 181 MB working set, 118 to 120 MB private working set, 655 to 660 MB private bytes.
- **The hybrid renderer** (`--no-default-features --features screenshot,renderer-hybrid,publish,lint`): 157 to 158 MB, 97 to 99 MB, 353 to 356 MB.

1. **Vello with area antialiasing only: not pursued here.** It saved about 3 MB (2 percent) when measured in Wave 5. It is a one-line change in Vello's imaging crate upstream, proposed there; vendoring that crate for it is not worth the upkeep.
2. **One wgpu backend: stays an opt-in** (`--graphics`, `[gui] graphics`). Its saving (26 MB with Vulkan on this machine, and a loss with Direct3D 12) depends on the graphics driver, and it has been measured on one machine only.
3. **The hybrid renderer: stays a build option, not the default.** It saves about 22 MB of working set and 300 MB of private bytes, confirmed. But the review screenshots at 100 and 200 percent are drawn by Vello's CPU renderer in both builds, so they cannot show a difference, and the hybrid renderer's drawing speed on a large window with the reading aids drawn cannot be measured without a window on screen, which the automated runs never take. The private bytes are memory the graphics driver commits and never touches, and the working set difference is about 12 percent. The default changes only after a sighted check of the hybrid renderer's drawing at 200 percent and a frame-time measurement on screen.

**Startup announcements wait for the screen reader.** "Opened" and "Reading at" could be lost when they came before a screen reader had asked for the window's tree: they were put in the first tree, and a live region already in the first tree is not announced. The announcer now holds such messages out of the first tree and says them once in the next pass (the driver looks again every 200 ms while they wait). A screen reader that restarts later does not hear them again. UI Automation notifications (`announce = "uia"`) wait the same way.

**The window's name on focus.** When the window takes the focus with the document focused, the self-voicing mode says the document's title and the window's name in textweaver's own voice. With a screen reader, the window's title (the document's title and "textweaver") and the focused document are the platform's to say, and textweaver adds nothing, so they are heard once.

**Windows High Contrast** is followed live: the window draws with the contrast theme's own colors, and marks that had a tint keep their shapes (see `docs/gui.md`).

## Consequences

- The GUI has a small, reviewed `unsafe` block for the Notification event (one COM call and one COM object), the first in the crate. It is Windows-only and needs `windows` and `windows-core`, which AccessKit already brings.
- The UI Automation report checks both announcement paths and a long list, and reads LiveRegionChanged directly.
- `examples/memory_probe.rs` measures the app's share of the GUI's memory without a window.

## See also

- [ADR-0027: Xilem GUI](0027-xilem-gui.md)
