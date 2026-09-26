# Research: a Xilem GUI for textweaver

Researched on Saturday, September 26, 2026, for Agent W3b. The sources are listed at the end. Jon chose Xilem, Linebender's all-Rust toolkit, so that as much of the GUI as possible is Rust. This page says what that needs.

## Summary

- **Xilem can meet textweaver's needs**, but only with our own document widget. Xilem's built-in text widgets do not meet the bar today.
- **AccessKit**, the accessibility layer underneath, is in good shape on all three platforms.
- **Xilem itself is alpha, and its development has slowed.** Keep the frontend thin: depend on Masonry more than Xilem, and keep the wxDragon spike working on Windows until the Xilem GUI passes the same UI Automation report.

## AccessKit: the strong part

**Linux (AT-SPI).** These landed in 2026:
- text attributes (`accesskit_unix` 0.21, March 4);
- the Orca-detection fix (0.22, June 12);
- the EditableText and Document interfaces (0.23, August 29).

Before that, it already had the full Text interface, caret and selection events, and live regions sent as AT-SPI announcements with polite or assertive priority.

**Windows (UI Automation).**
- TextPattern and text ranges, with attributes including background and foreground colour.
- Text selection changed and live region changed events.

**macOS.** Text support since 2022, text attributes since March 2026, and announcement notifications.

**Limit.** Each text run holds about 255 characters at most, so a document is exposed as many run nodes. Stable run IDs keep updates small.

**Still open upstream:**
- AT-SPI Collection, needed for Orca structural navigation (PR #758);
- keyboard shortcuts on Linux (PR #668);
- heading level and expanded state on Linux (PR #665);
- UIA menus (issue #27).

## Xilem and Masonry: the gaps

- **Status.** "Alpha-quality" and "experimental". The last release is v0.4.0 (October 29, 2025), and main needs Rust 1.96 or newer.
- **Read-only text cannot take focus.** In Masonry's `text_area.rs`, `accepts_focus` returns `EDITABLE`, so a screen-reader user could not arrow through a read-only document.
- **One style for the whole text.** Parley's `PlainEditor` applies a single style, so there is no ranged highlight on the spoken word.
- **No announcements.** There is no announcement API. A Label sets text runs, not a name, so it cannot be a live region.
- **Missing features.** No menus, no system high contrast, and no Page Up or Page Down in text.
- **Old AccessKit.** It pins AccessKit 0.24, which misses the Orca-detection fix. Xilem issue #1733 reports Orca reading nothing until the user turned the screen-reader setting on by hand.
- **Large documents.**
  - Parley 0.7 fixed layout that grew non-linearly on long paragraphs.
  - `PlainEditor` rebuilds the whole buffer's layout after every change, and its accessibility code rebuilds every run node.
  - Xilem's `VirtualScroll` virtualises a list of widgets. A document built from it would reach the screen reader as many separate text objects.
- **Memory.** Issue #918 reports about 430 MB for "Hello, World", much of it renderer buffers.
- **Available now:**
  - bundled fonts (`register_fonts`), DPI scaling, and IME;
  - Windows, macOS, and Linux on Wayland or X11, with no GTK. The rfd file dialog has a desktop-portal backend;
  - a CPU renderer (Vello CPU) for CI.

## The design

- **A new crate on top of `textweaver-app`.** `App` lives in Xilem's app state and is changed only on the main thread.
- **Our own `DocumentView` widget, written against Masonry and AccessKit directly.**
  - One focusable node: a read-only Document, or a multi-line text input in edit mode.
  - Text-run children of up to 255 characters each, with stable IDs per paragraph chunk.
  - The spoken word as a background-colour attribute, and the caret as the selection.
  - It handles the caret and paging keys itself, and answers the set-selection, scroll-into-view, and focus actions.
  - It reports the user's caret moves to the app (`Command::SetCursor`).
- **Windowing, as in Paperback.**
  - Positions stay document-absolute and are mapped through `DisplayIndex`.
  - The window extends while reading and re-centres on jumps.
  - The window is smaller than the wx plan's, and sized by measurement.
  - Only visible lines are laid out and painted, from cached per-paragraph layouts.
- **Speech waker.** A Xilem task wakes on the speech service's channel, or on `next_wakeup`, and sends a Tick message. The handler polls speech on the main thread.
- **Announcements.**
  - A zero-size live node: set its name, and mark it assertive when the message is urgent.
  - Use a fresh node for a repeated message, because the same name twice raises no event.
  - JAWS support for live region changed events is untested.
- **AccessKit version.** Patch the workspace to `accesskit_winit` 0.34 or later (`accesskit_unix` 0.22 or later) so Orca finds the app, and send the bump upstream.
- **Keys, menus, and dialogs.**
  - The keymap sits in a root wrapper widget; caret keys stay with `DocumentView`.
  - An in-app menu bar and the command palette, with menu roles. Native menus through muda on Windows and macOS are an option; on Linux muda needs GTK.
  - Dialogs are separate windows with the Dialog role and labelled controls.
  - File dialogs use rfd, with the portal backend on Linux.
- **Theme and fonts.**
  - `Theme::rgb_table` feeds Masonry's properties.
  - Read the system high-contrast state per platform: `SPI_GETHIGHCONTRAST`, macOS increase contrast, and the Linux settings.
  - Register the bundled fonts at startup.

## Acceptance tests

**Windows.** Adapt `uia-report.ps1`. Check:
- the window name, and the Document control's name, labelled-by, read-only, focusable, and TextPattern text;
- the selection following the spoken word while reading;
- that moving the caret through TextPattern moves the app;
- the background-colour attribute on the highlighted range;
- live region changed events that carry the announcement text;
- that buttons, menus, and dialogs are named;
- that `--background` never takes the foreground;
- load and update times for 1 million and 10 million characters.

**Linux.** Under Xvfb with the Vello CPU renderer, dump the AT-SPI tree with pyatspi. Check:
- roles, names, states, and the Text interface: character count, text, caret offset, and attribute runs;
- caret-moved, selection-changed, and announcement events, with their priority;
- that accessibility turns on without the gsettings workaround.

Then a person tests with Orca: arrow reading, say-all, the text-attributes command, announcements, and dialogs.

**macOS.** Dump the accessibility tree (role, value, selected text range, announcements). A person then tests with VoiceOver.

**Memory.** Compare with Xilem issue #918.

## Upstream contributions we may make

- **Xilem and Masonry:**
  - the AccessKit bump;
  - a focusable read-only text or document widget;
  - ranged styles;
  - Page Up and Page Down;
  - an announcer;
  - shortcuts and menus;
  - system contrast.
- **AccessKit:**
  - help land Collection (#758), shortcuts (#668), and heading level (#665);
  - consider an optional UIA Notification event, if JAWS ignores live regions.

## What would change the plan

**In Xilem's favour:**
- a Xilem release on AccessKit 0.25 or later;
- Masonry gaining focusable read-only text, ranged styles, and an announcer, or our own widget passing the tests;
- Collection merging in AccessKit.

**Against Xilem:**
- JAWS ignoring AccessKit live regions or text;
- memory or large-document costs we cannot window away;
- Xilem development stopping.

**The fallback.** The wxDragon spike stays working on Windows. If we reconsider for Linux, the answer would be wxWidgets on GTK 3. The `live-region` crate already announces to Orca through ATK's notification signal, so the old note that "GTK 3 has no announcement API" is out of date. Paperback shipped a Linux build on this stack on September 3, 2026.

## Other options, briefly

- **GTK 4 through gtk4-rs.** It has an announce call (4.14), and recent Orca text-view fixes: wrapping issue #8140 in 4.23.1, and Orca 51's say-all fix. It would mean a second GUI codebase. Ubuntu 24.04's screen-reader help still suggests GTK 3 apps over GTK 4 ones.
- **Qt 6.** It has an announcement event (6.8). Jon's wiki notes the accessible-name quirks. Its Rust bindings (cxx-qt 0.10) are early and have no Qt Widgets. It is heavy.
- **Other AccessKit toolkits.**
  - egui has no live regions (issue #2647 is open).
  - Iced 0.14 has no AccessKit support.
  - Slint was hit by the same Orca-detection bug.
- **Web views.** WebKitGTK talks to AT-SPI directly, but it is heavy and rarely preinstalled.
- **FLTK.** No AT-SPI support was found.

## Sources

- [Xilem repository](https://github.com/linebender/xilem): README, v0.4.0 notes, issues #918, #1343, #1554, #1733, #1805, and PRs #1447, #1596, #1796
- [Masonry text_area.rs](https://github.com/linebender/xilem/blob/main/masonry/src/widgets/text_area.rs)
- [Parley changelog](https://github.com/linebender/parley/blob/main/CHANGELOG.md)
- [Linebender blog posts](https://github.com/linebender/linebender.github.io/tree/main/content/blog)
- [AccessKit repository](https://github.com/AccessKit/accesskit): releases, issues #27, #603, #713, and PRs #715, #758, #665, #668
- [at-spi2-core NEWS](https://github.com/GNOME/at-spi2-core/blob/main/NEWS)
- [live-region](https://github.com/trypsynth/live-region)
- [Paperback PR #762](https://github.com/trypsynth/paperback/pull/762)
- [wxWidgets GTK 4 PR #26968](https://github.com/wxWidgets/wxWidgets/pull/26968)
- [ATK notification signal](https://gnome.pages.gitlab.gnome.org/at-spi2-core/atk/signal.Object.notification.html)
- [Orca NEWS](https://gitlab.gnome.org/GNOME/orca/-/raw/main/NEWS)
- [GTK issue #8140](https://gitlab.gnome.org/GNOME/gtk/-/issues/8140)
- [GTK 4.14 accessibility](https://blog.gtk.org/2024/03/08/accessibility-improvements-in-gtk-4-14/)
- [GNOME accessibility update, May 2025](https://blogs.gnome.org/gtk/2025/05/12/an-accessibility-update/)
- [Newton update](https://blogs.gnome.org/a11y/2024/06/18/update-on-newton-the-wayland-native-accessibility-project/)
- [Ubuntu screen-reader usability](https://ubuntu.com/desktop/docs/en/24.04/how-to/troubleshoot/improve-screen-reader-usability/)
- [cxx-qt](https://github.com/KDAB/cxx-qt)
- [Iced issue #552](https://github.com/iced-rs/iced/issues/552)
- [egui issue #2647](https://github.com/emilk/egui/issues/2647)

## See also

- [ADR-0014: GUI toolkit (the wxDragon spike)](../adr/0014-gui-toolkit.md)
- [Roadmap](../roadmap.md)
- [Documentation index](../README.md)
