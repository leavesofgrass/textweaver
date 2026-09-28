# ADR-0028: The Xilem GUI after Jon's session

- Status: accepted (Wave 4: drafted by Agent W4s; finished by Agent W4a2 after Jon's session 1, Sunday, September 27, 2026)
- Date: 2026-09-27
- Builds on: [ADR-0027](0027-xilem-gui.md) (the Xilem GUI)

## Context

ADR-0027 left three questions for Jon's first NVDA and JAWS session on the Xilem GUI:

- Do announcements reach JAWS? AccessKit raises only UI Automation's LiveRegionChanged, which NVDA speaks and JAWS has handled inconsistently.
- Can a screen reader reach list options and settings that are scrolled out of view? The UI Automation report saw 13 of 15 settings sections.
- Why did the GUI's memory grow from 98 MB to 172 to 187 MB during Wave 3?

This draft records what W4s changed before the session, what it measured, and the two questions the session decides. W4a2 records Jon's answers and the decisions here.

## Two ways to announce

The GUI now has two announcement paths, chosen with `--announce live|uia` or, in `settings.toml`, with `announce = "live"` or `"uia"` in a `[gui]` table. The command line wins over the setting; the live region is the default.

- **`live` (the default):** as in ADR-0027. Each message is a new AccessKit node with its live setting, and AccessKit raises LiveRegionChanged (AT-SPI's Announcement on Linux).
- **`uia` (Windows only):** for each message the GUI raises a UI Automation Notification event with `UiaRaiseNotificationEvent`, kind "other", activity id `textweaver.announcement`. Assertive messages are sent as "important, most recent"; polite ones as "all". The message nodes stay in the tree with their live setting off, so object navigation still finds them and no screen reader is told twice. On other systems `uia` falls back to the live region and says so once at startup.

How it is raised matters. Raised on the window's host provider (`UiaHostProviderFromHwnd`), the events never reached a client, because the host provider is a client-side provider. They are raised from a small server-side provider (`widgets::notify`) whose host is the window; UI Automation merges it with the window's element, whose provider is AccessKit's.

W4s read the `[gui]` table from the store's preserved unknown tables (`Settings::extra`), because the store and the schema were outside its brief. **Update (W4a2):** the setting is now a proper one in the four places: `GuiSettings { announce }` in `textweaver-store` (default `"live"`; a bad value warns with the other settings and keeps the default), the export fixture, a `gui.announce` choice in the settings schema (the settings dialog shows it under "Window"), and the GUI reading `settings.gui.announce`.

**Checked:** the UI Automation report (`tools/uia-report.ps1 -Announce live` and `-Announce uia`, Sunday, September 27, 2026, Windows 11 26200) passes both ways. With `live`, LiveRegionChanged events arrive for each message (10 in the run) and no Notification events. With `uia`, Notification events arrive with their text and activity id (10) and no LiveRegionChanged events. Managed UI Automation gained LiveRegionChanged in .NET Framework 4.7.1, so the report now subscribes to it instead of only watching for new elements.

**Found:** both paths lose the announcements made at startup, before any UI Automation client has asked for the window: "Opened Sample Markdown Document." and "Reading at 265 words per minute." in the report's runs. The report's client connects about two seconds after launch. A screen reader that is already running asks sooner, when the window appears, so Jon may hear them; the session checks it.

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

The reader's `publish` feature is already off in the GUI: the workspace declares `textweaver-app` with `default-features = false`, and `textweaver-xilem` turns nothing on, so no citation, conversion, or rendering crate is in its tree. The engine crates are not optional in `textweaver-engines`, so excluding each in turn needs changes in crates W4s may not touch. Instead, `examples/memory_probe.rs` builds the app exactly as the GUI does, with every engine, opens the document, and reads with the paced backend, with no window and no renderer. That bounds what any engine or data file could cost.

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
- **The 98 MB of Wave 3 was most likely read while the renderer was still starting.** Runs measured about ten seconds after launch sometimes read 89 to 94 MB (35 to 46 MB private working set) and then rose to the steady 170 MB. The growth ADR-0027 recorded is the renderer finishing its start, not a leak: no run grew after the first 15 seconds.

Nothing inside `textweaver-xilem` accounts for more than a few megabytes, so nothing was changed there. Proposed, not made (all outside the crate or a design choice):

1. **Vello with area antialiasing only.** `imaging_vello` creates Vello's renderer with `RendererOptions::default()`, which compiles the pipelines for three antialiasing methods, and uses only one (`AaConfig::Area`). `AaSupport::area_only()` should cut shader memory and startup time. A small change upstream (imaging_vello or Masonry); not measured.
2. **One wgpu backend on Windows.** Limiting wgpu to Vulkan saved about 25 MB of working set here; DirectX 12 cost more. This depends on the graphics driver, so it needs measuring on other machines first. `WGPU_BACKEND` already selects one for testing.
3. **The hybrid renderer on Windows.** It halves private bytes (650 to 350 MB) and saves about 20 MB of working set, at some cost in rendering speed on large windows. A choice for W4a2 after the session, with the screenshots at 100% and 200% compared.

## The two questions the session decides

1. **Announcements: `live` or `uia`?** Jon tries Pause, Stop, and a rate key with each, in NVDA and then in JAWS, and says which is heard once, reliably, without cutting off the reading. The default follows his choice. If JAWS needs `uia` and NVDA works with both, `uia` becomes the Windows default, and the store and schema get the `[gui] announce` setting properly.
2. **The highlight: selection or background?** While reading, the caret sits on the spoken word, which has its own background color (the default), or `--select-spoken` selects the word. Jon says which reads better in NVDA and JAWS, with speech and with the Braille display. That decides the document view's design, which W4a2 then builds on.

W4a2 records the answers here, changes the status to accepted, and removes whichever option is not kept, or keeps it as a setting if Jon prefers.

**Jon's answers (Sunday, September 27, 2026, after his first screen reader and Braille display checks):**
1. **Announcements:** the live region stays the default. `uia` stays available as an option.
2. **The highlight:** the background color stays the default, and the caret is not moved by selection. `--select-spoken` stays available as an option.

W4a2 builds on these, adds `[gui] announce` to the store and schema so it appears in the settings dialog, and changes the status to accepted.

## Decision

Jon's first checks with NVDA, JAWS, and his Braille display were good, so the designs W4s prepared stay as they are:

- **Announcements:** the live region is the default everywhere. UI Automation notifications stay as an option, `[gui] announce = "uia"` in the settings (the settings dialog, under "Window") or `--announce uia` for one run. The command line wins over the setting.
- **The highlight:** the spoken word has its own background color, and the caret (the document's text selection, collapsed) sits at its start. The word is not selected. `--select-spoken` stays as an option for anyone who prefers the word selected. Everything later in the document view builds on this design: the window slide, the reading aids, and edit mode (W4a3).
- **The hybrid renderer** stays a build option (`--no-default-features --features screenshot,renderer-hybrid`), not the default. The session did not raise memory, and the hybrid renderer's cost in drawing speed on large windows has not been measured with the reading aids drawn. Revisit it with Jon's second session.

The key named in "No document is open" now comes from the keymap (`named_key`), and the list introduction (`ListKey::Introduce`) and the title line's parts (`App::title_parts`) reach the GUI as they reach the terminal reader.

## Consequences

- The GUI has a small, reviewed `unsafe` block for the Notification event (one COM call and one COM object), the first in the crate. It is Windows-only and needs `windows` and `windows-core`, which AccessKit already brings.
- The UI Automation report checks both announcement paths and a long list, and reads LiveRegionChanged directly.
- `examples/memory_probe.rs` measures the app's share of the GUI's memory without a window.

## See also

- [ADR-0027: Xilem GUI](0027-xilem-gui.md)
- [Research for Wave 4](../research/wave4.md): announcements on Windows.
- [Wave 4 orchestration plan](../research/wave4-orchestration.md): W4s's and W4a2's briefs.
