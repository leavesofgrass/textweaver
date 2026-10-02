# GUI and visual design: a better looking textweaver

This page is for the people who build textweaver's window (`crates/textweaver-xilem`) and terminal reader (`crates/textweaver-tui`), and for the owner, who asked for a reader that is "better, faster, but also better looking." It surveys what good accessible reading interfaces look like in 2026, what the evidence says about type and color for readers with dyslexia, low vision, and light sensitivity, what the Xilem stack can draw cheaply, how a ratatui app looks polished without confusing a screen reader, and which small interactions matter most to disabled readers. It ends with a concrete design proposal and a ranked list of fifteen changes, each sized, targeted, and costed. Written on Friday, October 2, 2026, against 0.1.0-alpha.7.

## 1. What good reading interfaces do in 2026

The products below were checked against their own documentation or reviews. The same patterns recur in all of them, which is why the patterns matter more than any one product.

- **Thorium Reader** offers fonts (including a "Readable (Dyslexia)" family), size, margins, word, letter, paragraph, and line spacing, sepia and night themes, and a read-aloud that highlights the sentence, the word, or both [1, 2].
- **Microsoft Immersive Reader** has Line Focus (1, 3, or 5 lines, the rest blacked out), text spacing, syllables, and Read Aloud with a moving word highlight [3].
- **Voice Dream Reader** highlights by word, line, or sentence, with chosen foreground, background, and highlight colors, in light, dark, and custom themes [4, 5].
- **Speechify** highlights word by word and puts the toggle in a settings sheet in the corner of the canvas [6].
- **Apple Spoken Content** highlights words, sentences, or both, underlined or on a band, each in its own color [10]. **Apple Books** ships six named themes and separate line, word, and character spacing [9].
- **Kindle** shows "Time left in chapter" or "Time left in book", learned from the reader's own speed [7].
- **Readwise Reader** moves a visible focus indicator with the arrow keys, highlights with one key, undoes with Z, hides each sidebar with a bracket key, and lists every key under `?` [8].
- **calibre's viewer** reads aloud with Piper or the system voice, highlights the word or sentence, and shows a highlights panel sorted by chapter [11].
- **Zotero 7.1's reader** added an Appearance popup with Dark, Snow, and Sepia themes and custom themes made from two colors [12, 13].
- **Obsidian's** "Readable line length" caps the measure in reading view [14].

The patterns, and how textweaver stands against them today:

| Pattern | textweaver GUI today | Verdict |
|---|---|---|
| A calm canvas with a 60 to 75 character measure and generous margins | `MAX_COLUMN` 820 px and `INSET` 28 px in `crates/textweaver-xilem/src/document.rs`, centered, about 70 characters at the default size | Has it; `docs/screenshots/xilem-gui/galaxy-100.png` shows a clean column. |
| A persistent but quiet toolbar | Five text buttons above, six below, every label carrying its key, such as "Open… (Ctrl+O)" (`docs/gui.md`) | Has it, loudly. At 200 percent (`galaxy-200.png`) the keys in the labels fill the width. |
| A floating "now reading" indicator | The word band and the caret on the spoken word; the status bar says "line 1 of 34, 0%" | Partly. No marker stays where reading stopped. |
| Two tone highlight (word plus sentence) | `styles.spoken_word` and `styles.spoken_sentence` in every theme (`crates/textweaver-theme/themes/galaxy.toml`): bold word on a lavender band inside an underlined sentence band | Has it, and it is the best part of the design. In `high-contrast-100.png` the sentence band nearly vanishes against black. |
| A progress bar with time remaining | A percentage in the status bar; no time estimate anywhere in `crates/textweaver-app/src` | Lacks it. |
| A reading settings sheet that applies live | The full Settings dialog (Ctrl+,) with 13 sections, a Colors dialog, the Font list (Ctrl+D), size keys | Has the pieces, not the sheet: voice, rate, font, spacing, and theme live in four places. |
| A sidebar for contents, notes, and bookmarks | The outline (Alt+O), notes, and bookmarks are modal list dialogs (`galaxy-dialog-100.png`) | Lacks it; `docs/roadmap.md` lists the panels as missing. |
| Focus mode | The reading ruler with `mask_outside` (`docs/gui.md`, "Reading aids") | Has it, under another name. |

Two things in the screenshots that no competitor would ship: list items have no bullets or numbers ("First bullet item" and "Step one of the procedure." sit flush in `galaxy-100.png`), and the status bar mixes the last message with the position. Both are fixed below.

What textweaver does better than the field: every mark has a shape as well as a color (`docs/gui.md`, "Reading aids"), every theme is contrast checked at load (`crates/textweaver-theme/src/check.rs`), button keys come from the live keymap, and dialogs stay in the window so a screen reader never loses them (ADR-0027).

## 2. Typography and color for the readers textweaver serves

### Fonts

- **Atkinson Hyperlegible Next** (Braille Institute, February 2025) added seven weights, a variable version, 150 languages, and a monospace, free under the SIL Open Font License; it is textweaver's bundled default (`third_party/fonts/atkinson-hyperlegible-next`, 292 KB) [15, 16]. Keep it as the default: it was designed for low vision.
- **OpenDyslexic**: Rello and Baeza-Yates (2013, 48 readers with dyslexia, eye tracking) found it neither better nor worse than ordinary fonts; readers preferred Verdana and Helvetica, sans serif and roman helped, italic hurt [17, 18]. Keep it as a choice, never the default. It is the largest bundled font, 868 KB (`third_party/fonts/opendyslexic`).
- **Lexend**: its own studies report higher words correct per minute in children, from wide spacing that reduces crowding [19]. Zorzi and colleagues showed the mechanism in 2012: extra wide letter spacing doubled accuracy and raised speed by more than 20 percent in 74 children with dyslexia [20]. textweaver bundles 16 KB of Lexend and downloads the rest (ADR-0022).
- **APHont** embodies the large print research: tall x-height, open letters, heavy strokes, large punctuation [21]. Its license is non-commercial, so list it if installed, never bundle it.
- **Literata** (Google Play Books' face, OFL, variable) and **Source Serif** are the serifs for readers who want a book feel; Inter is the common UI sans [22]. The chooser already lists installed fonts (`docs/gui.md`, "Text size and font"), so none needs bundling.

### Size, measure, and spacing

- The British Dyslexia Association's style guide asks for 12 to 14 point text, letter spacing near 35 percent of the average letter width, word spacing at least 3.5 times that, line spacing of 1.5, and bold rather than italic for emphasis [23, 24]. textweaver's default is 14 points, and `[reading_aids.spacing]` offers WCAG 1.4.12's values as a preset (ADR-0022).
- Baymard puts 50 to 75 characters as the comfortable measure; Dyson found readers faster at 100 characters but preferring 45 to 72, and readers with dyslexia often want 45 to 50 [25]. `MAX_COLUMN` is a constant; it should become a setting (section 6).
- APH's large print guidelines ask for 18 point or more and 1.25 to 1.5 line spacing [21]. The GUI's size steps reach 72 points.

### Color

- **Highlights against text.** WCAG 1.4.11 asks 3 to 1 for the parts of a control a reader must see, and 1.4.1 says color must never be the only cue [26]. textweaver meets 1.4.1 by rule (`StyleRole::no_color_attributes` in `crates/textweaver-theme/src/model.rs`) and checks the word band's text at 4.5 to 1. It does not check a band against the page, so a sentence band at 1.2 to 1 passes and is invisible. The high contrast screenshot shows exactly that.
- **Dark, sepia, and soft themes.** Photophobia affects 80 to 90 percent of people with migraine, and most cope with a dark room; warm, low luminance screens reduce the light reaching the eye [27, 28]. A "soft" theme is a dark theme that stays at 4.5 to 1 and no higher: a warm near black page and off white text that does not glare. textweaver has 23 themes and no soft one; Galaxy's `#dadada` on `#1e1e1e` is about 11 to 1.
- **High contrast mode.** Windows 11 contrast themes replace every color with system colors; the right response is to draw with those colors and keep shapes for everything that had a tint [29, 30]. textweaver does this (`crates/textweaver-xilem/src/system_colors.rs`), and `system-contrast-100.png` confirms it, except that the sentence band disappears instead of keeping its underline.
- **Reduced motion.** WCAG 2.3.3 asks that motion from interaction can be turned off [26]. The signals are `SPI_GETCLIENTAREAANIMATION` on Windows, `NSWorkspace.accessibilityDisplayShouldReduceMotion` on macOS, and GNOME's reduced motion setting, now proxied by the desktop portal [31, 32]. Neither the vendored Masonry nor the GUI reads any of them (a grep of `third_party/xilem` and `crates/textweaver-xilem/src` for "reduce"). Harmless today, because the GUI animates nothing; a rule the moment anything moves.

### A token model, checked against `textweaver-theme`

The theme crate has most of a token model. `ColorRole` holds 15 colors and `StyleRole` nine styles, each a foreground, an optional band, and attributes (`crates/textweaver-theme/src/model.rs`). What is missing is derived in the GUI alone: `Palette::from_theme` in `crates/textweaver-xilem/src/theme.rs` mixes `raised`, `border`, `border_hover`, `accent`, `on_accent`, `ruler_focus`, and `ruler_band` from the roles it has, and `ensure()` nudges each until it passes.

Recommended tokens, with where each comes from:

| Token | Light | Dark | High contrast | Status |
|---|---|---|---|---|
| `background`, `surface`, `text`, `muted` | from the theme | from the theme | system colors when forced | Keep (`muted` is `dim_text`). |
| `accent`, `on_accent` | theme's link or heading 1 | same | `Highlight`, `HighlightText` | Add to `ColorRole`; today derived in the GUI only. |
| `highlight-word`, `highlight-sentence` | theme styles | theme styles | word on `Highlight`, sentence underlined, no band | Keep; add a 3 to 1 check of the sentence band against the page, or drop the band and keep the underline. |
| `caret` | `text` | `text` | `CanvasText` | Add as a style; Masonry has `CaretColor` and the GUI sets it from `text`. |
| `ruler-focus`, `ruler-band` | tints of `focus` | tints of `focus` | shapes only | Add to `StyleRole` with a derived default, so the Colors dialog can show them (it already lists "Reading ruler color" as a setting). |
| `focus-ring` | `focus` style's band | same | `Highlight`, falling back to `text` | Keep; add the inner contrast line from section 5. |
| `border`, `raised` | mixes of surface and text | same | `text` | Add as derived roles with file overrides. |
| `disabled` | `muted` at 3 to 1 against its surface | same | `GrayText` | Add; today disabled buttons are not drawn differently. |

Every added token gets the same checks the existing ones get in `crates/textweaver-theme/src/check.rs`: 4.5 to 1 for text, 3 to 1 for indicators, and an attribute for any highlight. Files that omit the new keys keep working, since every key in a theme file is optional (`docs/themes.md`, "Everything a theme can set"). The CSS writer (`crates/textweaver-theme/src/css.rs`) gains the same variables, so HTML exports match the window.

## 3. What Xilem, Masonry, Vello, and Parley can draw, and what it costs

Versions in the tree, from `Cargo.lock` and `third_party/xilem/TEXTWEAVER.md`: Masonry and Xilem at a main snapshot of Monday, September 14, 2026 (the crates call themselves 0.4.0, released Wednesday, October 29, 2025), Parley 0.11.1 (August 16, 2026), Vello 0.8.0 on wgpu 26, winit 0.30.13, AccessKit 0.25.1. Upstream, Vello's releases page lists 0.9.0 (wgpu 29, bicubic image sampling, VARC glyphs), 0.10.0 (August 14, gradient interpolation in unpremultiplied alpha), and the sparse strips renderers at 0.2.0 on August 7; search results report that `vello_hybrid` was renamed `vello_gpu` 0.1.0 on August 26, 2026 to become the main renderer (unverified against the repository) [33, 34, 35]. Parley past 0.11 removes its `accesskit` feature [36]. Masonry still has no menus; the tracking issue for layers and menus is open [48, 49].

What the vendored snapshot offers, and its cost:

- **Rounded surfaces, borders, padding.** `CornerRadius`, `BorderWidth`, `BorderColor`, `Padding`, and `Gap` are Masonry properties (`third_party/xilem/masonry_core/src/properties/`), and the GUI uses them (10 px panels, 6 px buttons, `crates/textweaver-xilem/src/theme.rs`). Cost: one path per widget per frame; negligible.
- **Shadows.** `BoxShadow` has a color, an offset, and a `blur_radius` (`properties/box_shadow.rs`; no spread yet). It is drawn with Vello 0.8's `draw_blurred_rounded_rect`, an analytic blur, not a filter pass (`vello-0.8.0/src/scene.rs`). Cost: one primitive. The GUI uses one on panels (8 px blur). Keep it to panels and dialogs.
- **Gradients.** `Gradient` with linear and radial shapes exists (`properties/types/gradient.rs`), evaluated in the shader. Cost: cheap, but a gradient behind text makes its contrast unpredictable, so the rule is no gradients under text.
- **Blur of content** (frosted glass behind dialogs) is a filter pass over the backdrop; Masonry's property set has none, and the sparse strips line is adding filter layers [33]. Cost: a full screen pass per frame while a dialog is open. Not worth it; the dimmed page already does the job (ADR-0027).
- **Variable fonts.** Parley selects weight and width axes, and Masonry's `VariableLabel` animates weight (`third_party/xilem/masonry/src/widgets/variable_label.rs`). Atkinson Hyperlegible Next is variable. Cost: relayout of the paragraph; set weight once, never animate it for body text.
- **Emoji.** Parley 0.10 and 0.11 fixed color emoji selection, and Vello draws COLR glyphs [36, 33]. Cost: a glyph per emoji, nothing per frame. Let documents show their emoji; keep them out of the chrome.
- **Text decorations.** Underline and strikethrough are Parley style properties, with skip ink in Vello's glyph path [33]. The document view already paints its own dashed, dotted, and thick lines (`docs/gui.md`, "Reading aids"). Cost: one line per run.
- **Smooth scrolling and animations.** Masonry has `on_anim_frame` (`masonry_core/src/core/widget.rs`), used by the spinner, the caret blink, and `VariableLabel`. There is no transition system and no reduced motion query. A scroll animation costs a layout check and a paint per frame, about 0.3 ms each here, so it is cheap in time and wrong in policy: the view keeps the caret a third of the way down (`scroll_to` in `document.rs`), and a jump is what a screen reader expects. No animated scrolling in the document, ever; a 120 ms fade on dialogs only when reduced motion is off.
- **High DPI.** Logical pixels, drawn at the scale factor (`TEXTWEAVER.md`, item 8); the 200 percent screenshots are sharp.
- **Tooltips, virtual scroll, progress bars, badges, split panes, SVG.** All present (`third_party/xilem/masonry/src/widgets/`, `layers/tooltip.rs`). The tooltip layer shows on pointer events only, with `Role::Tooltip`; a focus shown variant is textweaver's to write. `VirtualScroll` suits a long outline. `Svg` is how icons come in.

**Frame time today**, from ADR-0027 and ADR-0028: first layout, runs, and tree for a 120,725 character window, 20 to 34 ms; a highlight move, 0.28 ms median and under 1 ms worst; an edit key, 2.2 ms. Memory is 170 to 180 MB working set, almost all GPU driver; the hybrid renderer halves private bytes (650 MB to 350 MB). Nothing proposed below touches the highlight move path, which runs ten times a second while reading.

**Is updating the vendored snapshot worth it?** The snapshot is 18 days old. Vello 0.10 brings wgpu 29 and fixes, not features the reader needs. The sparse strips `vello_gpu` is the change with a payoff: ADR-0028 measured the hybrid renderer at 22 MB less working set and 300 MB less private bytes. The cost is reapplying the 13 patches in `textweaver.patch` and passing the 20 percent harness gate. Target: later, after the first beta, and only once Masonry's own main runs on `vello_gpu`, so textweaver does not carry that port alone.

## 4. The terminal reader

### Looking polished in ratatui

ratatui 0.30 (in the tree as 0.30.2) merges overlapping borders, ships twelve border types (plain, rounded, double, thick, dashed variants, quadrant), `Gauge`, `LineGauge`, and `Scrollbar` [37, 38]. All twelve border sets use Unicode box drawing; there is no ASCII set (`ratatui-widgets/src/borders.rs`), so an ASCII fallback is a custom `border::Set` of `+`, `-`, and `|`.

textweaver's terminal UI (`crates/textweaver-tui/src/ui.rs`) draws a title line, the body, a status area of one to three rows, and a hints line, with borders only around lists (`Block::bordered()` at line 1582). That restraint is right. What polish means here:

- **Borders:** rounded on lists and the RSVP box when the terminal is known to draw Unicode (`COLORTERM`, `WT_SESSION`, a known `TERM_PROGRAM`, the same probe `crates/textweaver-theme/src/terminal.rs` uses for color), ASCII otherwise, and none at all around the document.
- **A quiet status bar:** keep the status line for messages, as now, and move the position to the title line, as it already is. The status repeat blanking (`status_to_draw`, which blanks a repeated message for a moment so a screen reader speaks it again) is a detail no other TUI gets right; keep it.
- **A consistent two tone highlight:** the theme's `spoken_word` and `spoken_sentence` at every color level; at 16 colors the sentence band is dropped and the underline carries it (`docs/themes.md`, "Terminal colors"). Already done.
- **True color:** detected, with `TEXTWEAVER_COLOR` and `NO_COLOR` overrides [39]. Already done.
- **A progress gauge:** none today. A `LineGauge` in the title line, with the percentage and the time left as text beside it, costs one row and nothing else.
- **Nerd Font icons:** opt in only (`[display] icons = "nerd"`), with the text they replace kept in every accessible path. Nerd Fonts are MIT licensed and offer a symbols only fallback font, but nothing can detect whether the user's terminal font has the glyphs, so the default stays text [40].

### Screen readers and terminals

NVDA reads Windows Terminal through UI Automation and can speak new text via the terminal's own notifications; the whole buffer is reviewable [41, 42]. Orca works with gnome-terminal (VTE); GNOME Console had no screen reader support as of its tracker issue [43]. In both, decoration is noise: a screen reader reads a line of `─────` as "box drawings light horizontal" repeated, or as silence, depending on its symbol settings, and a gauge made of block characters is read as a string of "full block". The rules that follow:

- Everything a screen reader must know lives in the title line, the status line, or the document. The gauge, borders, and icons are extras that duplicate text, never replace it.
- In `braille_first` mode (`ui.rs`, `braille_title`), the position comes first on the title line and the gauge is left out, since a 40 cell display shows one line.
- A gauge must not change the status line, or the screen reader speaks it every second. Update it per whole percent, on the title line only.
- Keys keep their written names; `docs/screen-readers.md` already says symbols and box drawing are never the only signal.

## 5. Micro interactions that matter for disabled readers

- **Announce, then show.** Every message in the window goes through `App::announce_as` with a level (ADR-0046), and startup messages wait for the screen reader (ADR-0028). Keep that order for every new element: live region first, pixels second. A new sidebar says "Contents, 12 headings" before it is drawn.
- **A focus ring that never disappears.** The ring is 2 px (`FOCUS_WIDTH` in `theme.rs`), checked at 3 to 1 against page, panel, and button, and falls back to the text color in forced colors. WCAG 2.4.13 asks a 2 px ring at 3 to 1 against both states, and 2.4.11 asks that focus is never hidden [26, 44]. Two gaps: on the accent Play button the ring and the fill are both purple (`galaxy-100.png`), and a dialog's rows have no ring until the first key. Draw a double ring (2 px focus color outside a 1 px page colored line) and focus the first row on open.
- **Target size.** WCAG 2.5.8 asks 24 by 24 CSS pixels; Apple asks 44 points and Material 48 dp [44, 45, 46]. The GUI's buttons are 8 px padding plus 15 px text, about 35 px tall (`button_props` in `theme.rs`). Raise them to 44. The settings rows are already about 46 px.
- **Key hints in tooltips.** The key belongs in `AcceleratorKey` and in a tooltip on hover and focus, not in the label (ADR-0033's status update made that change for the name; the screenshots still show the key in the visible text).
- **A first run tour.** The window says a welcome with five keys (`docs/gui.md`, "Starting it"), and `docs/roadmap.md` plans a tour. Five text steps in one dialog, each a sentence with its key, is enough: open, play, next sentence, settings, help.
- **"I am reading from here."** When reading stops, the caret stays on the last spoken word, and the first caret move loses the place. A margin mark that stays where reading stopped, with one key to return to it, is what Line Focus and Kindle's position do implicitly.
- **Undo for accidental actions.** Edit mode has Ctrl+Z (`docs/gui.md`, "Editing"). Reset all colors, Remove voice, and settings import ask first and offer no undo after. One "Undo (Ctrl+Z)" that reverts the last settings change, said as "Undone: theme back to Galaxy", covers them.
- **Errors as sentences.** textweaver's messages are already sentences with a cause and a next step ("midnight.toml was not loaded: colors.text ..."). Keep the rule in the window's code, and show an error in the error color with the word "Error" in the text, never by color alone.

## 6. Design proposal

### The default window

```
+--------------------------------------------------------------------------+
| File  Edit  View  Reading  Speech  Tools  Help            (menu bar)     |
+--------------------------------------------------------------------------+
| [#] Sample Markdown Document                [Open] [Font] [Edit] [Aa] [>_]|  header, 56 px
+--------------------+-----------------------------------------------------+
| Contents  Notes    |                                                     |
| ------------------ |     Sample Markdown Document            (H1 35 px)  |
| > Sample Markdown  |                                                     |
|   Lists            |     This paragraph has bold text, italic text,      |
|   Emphasis         |     inline code, and a link. Dr. [Jones] arrived    |
|   Links            |     ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~    |
|   Conclusion       |                                                     |
|                    |     Lists                               (H2 29 px)  |
| (F6 cycles         |                                                     |
|  regions)          |     •  First bullet item                            |
|                    |     •  Second bullet item with emphasis             |
|                    |        •  Nested bullet under the second            |
|  sidebar 260 px    |     1. Step one of the procedure.                   |
|  collapsible       |                                                     |
|                    |                        measure 66 ch, max 820 px    |
+--------------------+-----------------------------------------------------+
| [> Play] [■ Stop] [|< Prev] [>| Next] [- Slower] [+ Faster]  |  toolbar 60 px
+--------------------------------------------------------------------------+
| Opened Sample Markdown Document.     [=====      ] 48%  12 min left  265 wpm |  status 32 px
+--------------------------------------------------------------------------+
```

- **Regions:** menu bar (unchanged), header (`Banner`), sidebar (`Navigation`, collapsible, F6 moves between regions), document, toolbar (`Toolbar`, "Reading"), status bar (`Status`). Dialogs stay in the window.
- **Type scale,** from `DOC_TEXT` 20 px and the heading scale in `document.rs`: body 20, H4 22, H3 25, H2 29, H1 35; interface text 15, small text 13, never below 13.
- **Spacing scale:** 4, 8, 12, 16, 24, 32 logical pixels. Gap 8 (`GAP`), panel padding 16 (`PAD`), paragraph spacing 0.75 em, heading margin above 1.25 em, list indent 1.5 em with a hanging marker.
- **Color tokens:** the table in section 2. Galaxy stays the default, with Galaxy Light, High Contrast, the system colors in forced mode, and one new soft theme.
- **Icon set:** Lucide, ISC licensed, 24 px outline icons in the current color [47]. About 20 icons (folder, type, pencil, settings, terminal, play, pause, stop, skip back and forward, minus, plus, list, bookmark, note, search, sun, moon, contrast, info) are under 20 KB of SVG, parsed once through Masonry's `Svg` widget. Every icon sits beside its label, and the accessible name is the label alone.
- **Controls:** 44 px tall buttons, 6 px corners, hairline border, one accent button (Play). Shadows on panels and dialogs only. Hover changes the border; focus draws the double ring.

### The reading view

Reading mode: the column at 66 characters by default (a setting from 45 to 90), the two tone highlight, the ruler as now, list markers in the muted color, a 2 px caret in the text color. Edit mode: the same column, a 1 px rule in the border color down the left of the text, misspellings dotted, lint marks dashed, and a thin "Editing" badge beside the title, said by the screen reader as the role change already is (ADR-0033).

### The reading settings sheet

```
+--------------------------- Reading (Alt+Shift+,) ---------------------------+
| Voice     [Eloquence, Reed            v]   Rate   [<] 265 wpm [>]           |
| Font      [Atkinson Hyperlegible Next v]   Size   [<] 14 pt   [>]           |
| Spacing   [Normal | Wide | Wider]           Theme  [Galaxy             v]  |
| Highlight [Word | Sentence | Both]          Ruler  [Off | Line | Band]      |
| Every change applies at once.                              [Close (Escape)] |
+-----------------------------------------------------------------------------+
```

One non modal card, eight settings, built from the same `SettingsForm` rows the Settings dialog uses (`crates/textweaver-xilem/src/settings_dialog.rs`), so nothing is duplicated. It applies live and is said as the full dialog is.

### The contents and notes sidebar

Two tabs, Contents and Notes (bookmarks listed among the notes with their marker). Each is a `ChoiceList` of the app's list model (`App::show_frontend_list`, ADR-0046), so letter jumps, positions ("3 of 12"), and F1 work as in every list. Enter moves the caret and returns focus to the document; the current heading has a bar before it and the word "current", never a color alone. Hidden below 900 px of window width, remembered in `[gui] sidebar`.

### The terminal layout

```
 Sample Markdown Document    [====      ] 48%  12 min left, line 16 of 34, Reading, 265 wpm
 ──────────────────────────────────────────────────────────────────────── (title rule, optional)

   This paragraph has bold text, italic text, inline code, and a link.
   Dr. Jones arrived at 3:30 p.m.
       ‾‾‾‾‾
   ## Lists

   - First bullet item
   - Second bullet item with emphasis

 Opened Sample Markdown Document.
 Space play  Esc stop  Alt+. next  F2 commands  F1 help
```

The title line gains the gauge and the time left, dropped first when the width is short (the title's parts are already dropped from the right, `draw_title`). Lists and the RSVP box get rounded borders where Unicode is safe, `+-|` elsewhere. The document never gets a border.

## 7. Fifteen improvements, ranked

Each names what it touches, its size (small: hours; medium: a day or two; large: a week or more), its target, its run time cost, and how it stays accessible. Measure GUI changes with the harness in `crates/textweaver-xilem` (first layout, highlight move) and `cargo xtask bench` for the app core.

1. **Progress and time remaining.** `crates/textweaver-app` (a `time_left` from words left at the current rate, in `title_parts`), `crates/textweaver-xilem/src/gui.rs` (status bar), `crates/textweaver-tui/src/ui.rs` (`draw_title`). Small, alpha.8. Performance: one division per status refresh; the word count comes from the narration plan already built, no allocation. Accessibility: plain text in the title parts, so Say Status and the braille title line get it; the gauge is decoration beside it.
2. **List markers and hanging indents in the GUI.** `crates/textweaver-xilem/src/document.rs` (`build_layout`). Medium, alpha.8. Performance: one small extra layout per list paragraph, cached with it; no change to the highlight move. Accessibility: drawn only, like syllable dots, so the runs a screen reader reads stay the document's text; the list role and count are the app's narration ("list with 3 items").
3. **Quiet toolbar with icons, 44 px targets, keys in tooltips.** `crates/textweaver-xilem/src/gui.rs` (`button`, `styled_button`), `theme.rs` (`button_props`), a new `icons.rs` with Lucide SVGs, a focus shown tooltip over Masonry's layer. Medium, alpha.8. Performance: 20 SVGs parsed once, then cached paths; tooltips only on hover or focus. Accessibility: the name stays the label, the key stays `AcceleratorKey` (the UI Automation report fails without it), the tooltip has `Role::Tooltip` and is `described_by`.
4. **Sentence band contrast rule and high contrast tuning.** `crates/textweaver-theme/src/check.rs` (band against page at 3 to 1, or no band), `themes/high-contrast.toml` and `contrast.toml`, `crates/textweaver-xilem/src/system_colors.rs` (keep the sentence underline when forced). Small, alpha.8. Performance: load time checks only. Accessibility: the sentence stays marked by its underline in every mode; color independence kept.
5. **Double focus ring, focus on open, focus never obscured.** `crates/textweaver-xilem/src/theme.rs`, `widgets.rs`, `dialog.rs`. Small, alpha.8. Performance: one extra stroke per focused widget. Accessibility: WCAG 2.4.13 and 2.4.11; the ring is visible on the accent button and in forced colors.
6. **"Reading from here" marker.** `crates/textweaver-app` (keep the last spoken position across stop and caret moves, with a `return_to_reading` command in the keymap and menus), `document.rs` (a bar in the margin, `DocMark`), `crates/textweaver-tui/src/ui.rs` (a gutter mark). Medium, alpha.8. Performance: one mark in the marks list; nothing per frame. Accessibility: the command says "Back to where reading stopped, line 16"; the mark has a shape.
7. **Measure setting and vertical rhythm.** `crates/textweaver-store/src/settings.rs` (`[display] measure`, 45 to 90 characters, default 66, shared with the terminal's `wrap_width`), `document.rs` (`MAX_COLUMN` from the setting and the font's average advance), the spacing scale above. Small, alpha.9. Performance: a relayout when it changes, the same as a size change. Accessibility: a number the Settings dialog reads; no visual only state.
8. **Theme tokens moved into the crate.** `crates/textweaver-theme/src/model.rs`, `file.rs`, `check.rs`, `css.rs`, `crates/textweaver-xilem/src/theme.rs` (`Palette::from_theme` shrinks to lookups), the Colors dialog rows. Medium, alpha.9. Performance: none at run time; a few more checks per theme at load (microseconds). Accessibility: every token checked; the Colors dialog says each one's contrast in words.
9. **Reading settings sheet.** `crates/textweaver-xilem/src/gui.rs` (a second `SettingsDialog` built from eight rows of the existing form), a command in the keymap and the Reading menu. Medium, alpha.9. Performance: the same rows as the Settings dialog; live apply already exists. Accessibility: the same grid widget NVDA and JAWS have already been tested on (ADR-0028).
10. **Terminal gauge, borders with ASCII fallback, Nerd Font opt in.** `crates/textweaver-tui/src/ui.rs` (`draw_title`, `draw_list`, `draw_rsvp`), `crates/textweaver-theme/src/terminal.rs` (a `unicode_ok` probe beside the color probe), `[display] icons`. Small, alpha.8. Performance: the gauge redraws only when the whole percent changes (`view_signature` already gates draws). Accessibility: the gauge is off in `braille_first`; borders never carry meaning; icons default to text.
11. **A soft theme for light sensitivity.** `crates/textweaver-theme/themes/dusk.toml` (warm near black page, text at about 5 to 1, muted accents), listed in `docs/themes.md`. Small, alpha.9. Performance: none. Accessibility: meets AA by design; a `kind = "soft"` label so the theme list says what it is for.
12. **Contents and Notes sidebar.** New `crates/textweaver-xilem/src/sidebar.rs` on `ChoiceList` and the app's list model, `gui.rs` (`build_tree`, F6 region cycling), `[gui] sidebar`. Large, alpha.9. Performance: the outline is built per revision, as Alt+O builds it now, off the highlight move path; `VirtualScroll` for long lists. Accessibility: a `Navigation` landmark, items with positions, Enter returns to the document, Escape closes; announced before it is shown.
13. **Reduced motion policy, and the OS probe.** `crates/textweaver-theme/src/os.rs` (read the three platform settings next to the color scheme probe, same 500 ms limit), a `motion` field on `Palette`; a test that fails on any `on_anim_frame` use in the GUI without the check. Small, alpha.9. Performance: one probe at startup. Accessibility: WCAG 2.3.3; nothing in the document ever animates.
14. **First run tour, undo for settings, errors as sentences.** `crates/textweaver-app` (tour text in the catalog, a settings undo stack of one), `gui.rs` (a question dialog variant with Next and Done), the lexicon's six languages. Medium, alpha.9. Performance: none. Accessibility: text only, keys named in each sentence, "Undone" said after Ctrl+Z.
15. **Update the vendored Masonry and move to `vello_gpu`.** `third_party/xilem`, `textweaver.patch`, `crates/textweaver-xilem/Cargo.toml` features. Large, later. Performance: the 20 percent gate on first layout and highlight move; expected memory saving in the range ADR-0028 measured for the hybrid renderer (about 22 MB working set, 300 MB private bytes). Accessibility: the UI Automation and AT-SPI reports must pass unchanged, and the 13 patches that give textweaver its accessible nodes come first.

What not to do: no gradients or blur behind text, no animated scrolling in the document, no icons without labels, no borders around the document in the terminal, and no change to the highlight move path for any of this.

## Sources

1. https://thorium.edrlab.org/en/docs/210_reading/215_readingparameters/
2. https://thorium.edrlab.org/en/docs/300_accessibility/310_natives/
3. https://support.microsoft.com/en-us/accessibility/word/use-immersive-reader-in-word
4. https://www.voicedream.com/reader/reader-feature-list/
5. https://afb.org/aw/14/8/15662
6. https://speechify.com/blog/how-do-i-enable-text-highlighting-on-the-computer/
7. https://www.bgr.com/2166895/tips-reading-more-kindle/
8. https://blog.readwise.io/p/bf87944f-b0fe-4f08-a461-f75ab8aded6a/
9. https://ios.gadgethacks.com/how-to/apple-books-just-got-its-biggest-iphone-update-years-0385075/
10. https://mcmw.abilitynet.org.uk/how-enable-your-device-read-aloud-text-you-have-selected-ios-14-iphone-ipad-and-ipod-touch
11. https://manual.calibre-ebook.com/viewer.html
12. https://www.zotero.org/blog/zotero-7/
13. https://forums.zotero.org/discussion/comment/485179
14. https://forum.obsidian.md/t/adjustable-readable-line-length/7564
15. https://en.wikipedia.org/wiki/Atkinson_Hyperlegible
16. https://fonts.adobe.com/fonts/atkinson-hyperlegible-next
17. https://editingresearch.byu.edu/2020/05/28/which-fonts-are-best-for-dyslexia/
18. https://pmc.ncbi.nlm.nih.gov/articles/PMC5629233
19. https://fonts.google.com/specimen/Lexend/about
20. https://pmc.ncbi.nlm.nih.gov/articles/PMC3396535
21. https://www.aph.org/app/uploads/2022/04/Research-Based-Large-Print-Guidelines.pdf
22. https://www.type-together.com/Literata
23. https://smashingmagazine.com/2021/11/dyslexia-friendly-mode-website
24. https://dyslexiascotland.org.uk/wp-content/uploads/2025/09/formats.pdf
25. https://baymard.com/blog/line-length-readability
26. https://www.w3.org/WAI/WCAG22/understanding/
27. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11015590/
28. https://link.springer.com/article/10.1186/s10194-024-01756-9
29. https://support.microsoft.com/help/13862/windows-use-high-contrast-mode
30. https://cssence.com/2024/forced-colors-mode-strategies
31. https://chromium.googlesource.com/chromium/src.git/+/HEAD/ui/gfx/animation/animation_win.cc
32. https://gitlab.gnome.org/GNOME/gnome-control-center/-/merge_requests/3253
33. https://github.com/linebender/vello/releases
34. https://linebender.org/blog/tmil-24/
35. https://linebender.org/blog/tmil-25/
36. https://github.com/linebender/parley/releases
37. https://ratatui.rs/highlights/v030/
38. https://github.com/ratatui/ratatui/blob/main/ratatui-widgets/src/borders.rs
39. https://no-color.org/
40. https://github.com/ryanoasis/nerd-fonts
41. https://download.nvaccess.org/releases/2026.1/documentation/userGuide.html
42. https://github.com/nvaccess/nvda/issues/11740
43. https://gitlab.gnome.org/GNOME/console/-/issues/244
44. https://kb.daisy.org/publishing/docs/wcag/target-size-minimum.html
45. https://dequeuniversity.com/rules/attest-ios/1.0/touch-target-size
46. https://support.google.com/accessibility/android/answer/7101858
47. https://lucide.dev/license
48. https://github.com/linebender/xilem/releases
49. https://github.com/linebender/xilem/issues/1343

## See also

- [Research index](README.md)
- [The textweaver window](../../gui.md)
- [Themes](../../themes.md)
- [ADR-0020: Themes](../../adr/0020-themes.md)
- [ADR-0027: Xilem GUI](../../adr/0027-xilem-gui.md)
