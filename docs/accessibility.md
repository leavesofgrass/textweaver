# Accessibility statement

textweaver is built first for people who use a screen reader or a Braille display, and for students with print disabilities. This page says what has been tested, how, and what has not.

textweaver is in beta. This statement describes release 0.1.0-beta.3.

## What textweaver aims for

- Every function is operable from the keyboard.
- Every control exposes a name and role to assistive technology.
- Changes are announced, with the meaning first.
- Color never carries meaning alone. Status is written in words, and marks differ in shape as well as color.
- Lines are short and plainly worded, so that a 40-cell Braille display can show each one without panning.
- Text size, fonts, spacing, and themes are user-configurable, including a high contrast theme.

## Newer features and how they treat assistive technology

- **The Braille view** (beta 4, the textweaver app) is for sighted learners, teachers, and transcribers. It is one image named "Braille view" with a description saying it is visual only. Its braille is never in the accessibility tree and is never announced, because a screen reader already sends braille to the display. Its dots differ by shape (a disc or a ring), never by color alone. See [The Braille view](reading-aids.md#the-braille-view).
- **CW** (beta 4) sends text as Morse code. A Braille display user reads the characters sent, or keyed, on the status line, which holds 40 cells with the newest character last; nothing extra is announced. With CW as the engine, textweaver's own messages are also sent in Morse code, so a screen reader is the way to hear them as speech. See [CW](cw.md).
- **Links between documents** (beta 4) say where you arrived and how to go back, in words, in both forms.
- The keying adapters were checked from their manuals and not yet with the hardware; see [Known limits](known-limits.md).

## What has been tested

- **Windows, by listening.** A person who is blind has used the app and the terminal reader with NVDA and JAWS, and with a 40-cell Braille display. The app had two such sessions. The terminal reader's newest Braille layout is waiting for the checklist in [the screen reader guide](screen-readers.md).
- **By computer, on every release.** The app's accessibility tree is checked on Windows and Linux. On macOS a smoke run reads a document. These checks verify what is exposed to assistive technology, not what the assistive technology then says.
- **Contrast.** Every built-in theme is checked for contrast, and the spoken-word highlight is checked against the page.
- **Output files.** HTML, EPUB, Word, and PDF output are checked for structure and alternative text. See [Converting documents](converting.md).

## What has not been tested

- macOS with VoiceOver, and Linux with Orca, beyond basic functionality. Some basic testing has been done with both, but not every release gets a listening test with them, and the packages are otherwise built and checked by computer. More extensive testing with both is planned. NVDA and JAWS on Windows are tested by hand by the owner every day.
- Braille displays other than the one named above, and other cell widths.
- The settings marked "not yet verified" in [the screen reader guide](screen-readers.md).
- Other screen readers, such as Narrator.

## Known problems

See [Known limits](known-limits.md).

## Standards

This page makes no claim of conformance to WCAG or any other standard. No outside audit has been done, and no conformance report has been written.

## Tell us

If you encounter a barrier, report it on the project's [issue tracker](https://github.com/leavesofgrass/textweaver/issues). Say your system, your screen reader and its version, and what you heard or read on your display. A report from a combination listed under "not been tested" is especially welcome.

## See also

- [Using textweaver with a screen reader](screen-readers.md)
- [Known limits](known-limits.md)
- [Themes](themes.md)
