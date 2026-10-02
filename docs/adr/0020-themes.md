# ADR-0020: Themes

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): The TUI, the app's theme cycle, and the settings moved onto this crate: the terminal reader runs Galaxy by default, cycles all 23 built-ins plus user themes with F5, and follows the system at startup. The TUI's three hand-made themes are gone. The GUI `system` theme is still future work.
- Status update (Saturday, September 26, 2026): `[highlight] color` and `sentence_color` are laid over the theme's highlight, with a warning when the text falls below 4.5 to 1 contrast (7 to 1 in high-contrast themes). The Xilem GUI takes its colors from `Theme::rgb_table`.

## Context

Star shipped 23 color palettes (`star/themes.py`), user CSS themes, and OS light/dark following. Its themes serve low-vision and dyslexic readers, so their colors are an accessibility feature, not decoration, and three audits measured how they fell short:

- "star TUI palette contrast audit": the terminal UI kept its own, different palette table with no contrast test. On 256-color terminals 6 of 18 themes were clean; on 16-color terminals none were, and `sepia`, `gruvbox-light`, and `solarized-light` drew headings bright yellow on white (1.00:1). Five GUI themes had no terminal entry, including the default `galaxy`, so the terminal silently ran another theme. On a colorless terminal every attribute was dropped with the colors. In `phosphor` the current search match looked like body text.
- "Color Contrast Algorithms and Checking Tools": WCAG 2.x is the enforceable formula; APCA is informative; base-8 terminal colors have no fixed RGB and cannot be guaranteed; do not rely on dim text.
- "Platform Accessibility Settings and Application Theme Engines (Section 508 §503.2)": warn about, never block, a user theme that fails contrast; share one ratio function between the runtime warning and the test gate; honor `NO_COLOR` and never change the terminal's palette; keep a light/dark pair per theme so the system setting can be followed inside the theme engine.

The primary user reads in Galaxy, Star's Obsidian-style dark theme; it was asked to be the default.

## Decision

**One crate, one palette per theme, every surface.** `textweaver-theme` defines a `Theme` once; the terminal, the GUI, and HTML all render from it, so the terminal can no longer drift from the GUI.

**Semantic roles.** 15 colors (`background`, `surface`, `text`, `dim_text`, `heading1`–`heading6`, `link`, `code`, `code_background`, `quote`, `error`) and 9 styles, each a foreground, an optional band, and attributes (`selection`, `spoken_word`, `spoken_sentence`, `find_hit`, `current_find_hit`, `bookmark`, `note`, `status_bar`, `focus`), plus a list of named reader highlight colors (yellow, green, blue, pink by default). A kind (`light`, `dark`, `high-contrast`), a display name, a description, an origin, and a light/dark `counterpart` complete the metadata.

**Never color alone.** Every highlight carries at least one attribute (bold, italic, underline, reverse). Highlights that appear together differ by attribute: the spoken word (bold on a band) from its sentence (underlined), the current find match (bold, underlined) from the other matches (underlined). Headings are bold, links underlined, quotes italic, and errors bold in every renderer. With color off, a fixed attribute table takes over.

**Themes are TOML files.** Every key is optional in a file. Without `inherits`, a file needs only `background` and `text`; everything else is worked out the way Star's terminal derived its chrome: highlight bands use an accent (heading 1 for the spoken word and status bar, heading 3 and heading 2 for find matches, link for focus) with the page color as text, so a band's legibility is the accent's own contrast. With `inherits = "<built-in>"`, missing keys come from that theme. Unknown keys are kept and written back. Errors name the key or line (`colors.text: "white" is not a color; write it as #rrggbb`).

**Star's 23 palettes are built in, faithfully.** Each is generated from Star's 11 keys into a complete, explicit file (`crates/textweaver-theme/themes/*.toml`, embedded with `include_str!`); a test fails if a file drifts from its generator. Star's values are kept wherever they pass. A value that fails is moved by the smallest OKLab lightness change that reaches the floor (hue and chroma kept where the gamut allows), and every change is recorded in the file's header and in the table below. Star had no error color; the terminal's `err` color is used where Star had one (14 themes), otherwise the color Star's hand-written terminal tables used (magenta for `dark`, `light`, `contrast`, avoiding red and green; green for `phosphor`) or the scheme's own red.

**Galaxy is the default** (`DEFAULT_THEME`, first in the cycle, first in `docs/themes.md`), with `galaxy-light` as its pair. Star's Galaxy matches Obsidian's dark palette in intent: background `#1e1e1e` (Obsidian `#1c1c1c`), text `#dadada` (Obsidian's), purple accent `#a882ff` for links (Obsidian's accent text is about `#a68af9`). Its one change is muted text, `#7d7d7d` (4.05:1) to `#858585` (4.52:1), which moves toward Obsidian's own muted gray `#b3b3b3`.

**Contrast rules (one function, `check`).** WCAG 2.x relative luminance (sRGB knee 0.03928). Text 4.5:1 against its background (AA 1.4.3), 7:1 in high-contrast themes (AAA 1.4.6); non-text indicators 3:1 (1.4.11: the focus band against the page); every highlight's text on its band; the attribute and co-occurrence rules above (1.4.1). Headings are held to the text floor, not the 3:1 large-text floor, because the same palette drives the terminal, where every cell is one size; `Requirement::LargeText` remains for callers. APCA Lc is computed for every pair and reported for information only. Built-in themes must pass every check (a test). User themes that fail load anyway, with a spoken summary of each failing pair ("Dim text on background: 4.0 to 1, needs 4.5 to 1."); `Repair::Explicit` offers the same minimal adjustment as an explicit, reversible fix.

**Terminal levels.** Detected from the environment (`TEXTWEAVER_COLOR` overrides; non-empty `NO_COLOR` or `TERM=dumb` turn color off; `COLORTERM`, `WT_SESSION`, known `TERM_PROGRAM`s, `*-direct` mean truecolor; `*256color` means 256; Windows without `TERM` means truecolor).
- Truecolor: exact colors.
- 256: the nearest cube or gray index (16–255, whose RGB is the same everywhere) that still meets the pair's floor; quantizing never turns a passing pair into a failing one.
- 16: the base colors' RGB belongs to the terminal, so the page is painted explicitly (0 for dark themes, 15 for light) and every color is chosen from the indexes that pass against it in three reference palettes (xterm, VGA/Linux console, Windows Terminal's Campbell), allowing for terminals that brighten bold text; where brightening would break contrast, bold becomes underline. Faint bands are dropped (the attributes carry the highlight); strong bands use dark base colors under bright white text. Choice among passing indexes weighs hue over lightness, so grays stay gray.
- No color: no colors, all attributes.

No level emits sequences that change the terminal's palette or cursor (502.2.2).

**HTML.** CSS custom properties (`--tw-<role>`, `--tw-<style>-fg/-bg/-weight/-decoration/-style`, `--tw-highlight-<name>-*`). The default stylesheet puts Galaxy on `:root`, Galaxy Light under `prefers-color-scheme: light`, High Contrast under `prefers-contrast: more`, and system colors under `forced-colors: active`, followed by element rules (links always underlined, a 3 px focus ring, never `outline: none`). A single-theme stylesheet serves an explicit choice.

**GUI.** `Theme::rgb_table()`: a flat, ordered `(key, Rgb)` list with styles resolved as shown (reverse applied, page filled in).

**Following the OS.** Pure decisions (`theme_for_os_scheme`, Star's mapping: dark → galaxy, light → galaxy-light, high contrast → high-contrast; `follow_os`, which stays within the current theme's pair; `startup_theme`, Star's rule that an explicit choice stops following) and pure parsers for each platform's output, plus a small probe (`reg query` on Windows, `defaults` on macOS, `gsettings` and `GTK_THEME` on Linux) with a 500 ms limit that answers `Unknown` rather than failing.

**Names.** Lookups ignore case and accept Star's old names (`obsidian` → `galaxy`, `zed-one-dark` → `one-dark`, ...) and `high_contrast`. An unknown name falls back to Galaxy and says so (Star fell back silently); cycling from an unknown name starts at Galaxy (Star skipped to the second theme and saved it into the shared settings).

## Adjustments to Star's palettes

Generated by `star::adjustments_table()` (also printed by the generator example). "Original" is Star's value (for `colors.error`, the error source named in the theme file). Ratios are WCAG 2.x against the color listed.

| Theme | Key | Original | textweaver | Before | After | Against | Floor |
|---|---|---|---|---|---|---|---|
| galaxy | `colors.dim_text` | `#7d7d7d` | `#858585` | 4.05 | 4.52 | `#1e1e1e` | 4.5 |
| galaxy-light | `colors.dim_text` | `#8a8f98` | `#727780` | 3.25 | 4.50 | `#ffffff` | 4.5 |
| galaxy-light | `colors.heading4` | `#1a9e8f` | `#008577` | 3.32 | 4.54 | `#ffffff` | 4.5 |
| galaxy-light | `colors.code` | `#b5673a` | `#a65a2c` | 3.75 | 4.52 | `#f2f0f9` | 4.5 |
| one-dark | `colors.dim_text` | `#5c6370` | `#8b93a1` | 2.32 | 4.52 | `#282c34` | 4.5 |
| one-dark | `colors.error` | `#e06c75` | `#e36e77` | 4.38 | 4.51 | `#282c34` | 4.5 |
| one-light | `colors.dim_text` | `#a0a1a7` | `#727379` | 2.47 | 4.53 | `#fafafa` | 4.5 |
| one-light | `colors.heading1` | `#4078f2` | `#356ce5` | 3.88 | 4.54 | `#fafafa` | 4.5 |
| one-light | `colors.heading3` | `#0184bc` | `#007ab2` | 4.00 | 4.55 | `#fafafa` | 4.5 |
| one-light | `colors.heading4` | `#50a14f` | `#318332` | 3.07 | 4.55 | `#fafafa` | 4.5 |
| one-light | `colors.link` | `#4078f2` | `#356ce5` | 3.88 | 4.54 | `#fafafa` | 4.5 |
| one-light | `colors.error` | `#e45649` | `#ce4237` | 3.51 | 4.51 | `#fafafa` | 4.5 |
| one-light | `colors.code` | `#c18401` | `#975d00` | 2.66 | 4.50 | `#eaeaeb` | 4.5 |
| dark | `colors.dim_text` | `#5c6370` | `#79818e` | 2.94 | 4.52 | `#16181d` | 4.5 |
| light | `colors.dim_text` | `#8c8fa1` | `#6f7284` | 3.07 | 4.56 | `#fafafa` | 4.5 |
| light | `colors.heading2` | `#209fb5` | `#007f95` | 3.01 | 4.50 | `#fafafa` | 4.5 |
| light | `colors.heading4` | `#e64553` | `#d63547` | 3.77 | 4.51 | `#fafafa` | 4.5 |
| light | `colors.code` | `#40a02b` | `#167e00` | 2.90 | 4.53 | `#eceff4` | 4.5 |
| phosphor | `colors.dim_text` | `#008800` | `#108e0e` | 4.16 | 4.51 | `#001200` | 4.5 |
| phosphor | `colors.code` | `#009900` | `#099c07` | 4.35 | 4.52 | `#002600` | 4.5 |
| nord | `colors.error` | `#af5f5f` | `#d98583` | 2.75 | 4.54 | `#2e3440` | 4.5 |
| solarized-dark | `colors.error` | `#d70000` | `#ff4b3b` | 2.78 | 4.52 | `#002b36` | 4.5 |
| solarized-light | `colors.code_background` | `#eee8d5` | `#f1ebd8` | 4.39 | 4.51 | `#586e75` | 4.5 |
| solarized-light | `styles.selection.background` | `#eee8d5` | `#f1ebd8` | 4.39 | 4.51 | `#586e75` | 4.5 |
| monokai | `colors.error` | `#ff005f` | `#ff447b` | 3.83 | 4.50 | `#272822` | 4.5 |
| everforest-dark | `colors.error` | `#d75f5f` | `#f17674` | 3.38 | 4.50 | `#2d353b` | 4.5 |

Quotes use the dim-text color (Star's `muted`), so each `dim_text` change applies to quotes too; headings 5 and 6 follow heading 4 as in Star's CSS. The 11 themes not listed (contrast, high-contrast, dracula, gruvbox-dark, tokyo-night, catppuccin-mocha, sepia, amber, rose-pine, kanagawa, gruvbox-light) keep every Star value; `sepia` and `gruvbox-light`, which lost their headings on Star's 16-color terminals, needed no palette change, only the new 16-color rules.

## Consequences

- Terminal and GUI share one palette per theme, and the default configuration really runs Galaxy in every frontend.
- A palette change that breaks contrast fails a test, at every terminal level, including 16-color terminals under three palettes with bold brightening.
- The generated files are the thing users copy; regenerating them after a derivation change is one command, and the sync test says when it is needed.
- Faint bands (the sentence band, the selection) disappear on 16-color terminals, and light themes there underline headings instead of bolding them; attributes carry the meaning.
- The TUI (`textweaver-tui/src/theme.rs`), the app's theme cycle, and the store's settings move onto this crate at integration; until then the TUI's three hand-made themes remain, and its `galaxy` is not Star's.
- A GUI `system` theme built from the platform palette (503.2's follow-system mode) belongs to the GUI in wave 3; the terminal's equivalent is the no-color level, which uses the terminal's own colors.


**Status update (Saturday, September 26, 2026).** Decision: not every theme has to meet WCAG AA, as long as some do. The contrast check becomes a label for most themes, not a gate. Each theme reports whether it meets AA, and the theme list and `docs/themes.md` say so. These themes must still pass:
- Galaxy (the default) and Galaxy Light;
- the high-contrast themes (`contrast`, `high-contrast`), at 7:1.

The test gate applies to those themes only.

## See also

- [Themes](../themes.md): the user guide.
- [Settings](../settings.md#display): the `[display]` settings.
- [Interactive pages](../site/index.html): the docs site uses Galaxy and Galaxy Light.
- [Documentation index](../README.md)
