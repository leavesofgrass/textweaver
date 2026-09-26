# Themes

A theme sets the colors textweaver uses for the page, text, headings, links, code, and highlights such as the word being spoken. Every theme is checked for contrast, and no theme shows anything by color alone: highlights are also bold, underlined, italic, or reversed, so they stay visible in any color setting and with color turned off.

This guide covers choosing a theme, the built-in themes, following your system's light or dark setting, terminal colors, and writing your own theme.

## Galaxy, the default

Galaxy is textweaver's default theme and the first in the list. It is Star's default theme, carried over faithfully: a dark theme modeled on Obsidian's dark mode, with light gray text on a near-black page and purple accents.

- Page `#1e1e1e`, text `#dadada`.
- Headings in lavender, violet, periwinkle, and teal.
- Links in purple (`#a882ff`), always underlined.
- The word being spoken is dark text on a lavender band, in bold, inside an underlined sentence.

One color changed from Star: dim text (hints, line numbers, quotes) is `#858585` instead of `#7d7d7d`, because Star's gray measured 4.0 to 1 against the page and the minimum is 4.5 to 1.

Galaxy's light partner is Galaxy Light. When textweaver follows your system's appearance, it switches between the two.

## Choosing a theme

- Press F5 to move to the next theme. textweaver says the theme's name.
- Start with a theme for one session:

  ```bash
  textweaver --theme sepia notes.md
  ```

- Set it for good in `settings.toml`, under `[display]`:

  ```toml
  [display]
  theme = "nord"
  ```

Names ignore case. Star's older names still work (`obsidian` means Galaxy). If a name is not found, textweaver uses Galaxy and tells you so.

`settings.toml` is in textweaver's configuration folder: on Windows `%APPDATA%\leavesofgrass\textweaver\config`, on macOS `~/Library/Application Support/org.leavesofgrass.textweaver`, on Linux `~/.config/textweaver`. Setting `TEXTWEAVER_HOME` puts everything under one folder instead.

## The built-in themes

They are grouped here by kind. Each is one of Star's palettes. Where a Star color missed the contrast minimum, it was moved by the smallest change that passes; the theme file lists every change.

F5 goes through them in a different order: Galaxy, Galaxy Light, One Dark, One Light, Dark, Light, Contrast, High Contrast, Phosphor, Dracula, Nord, Solarized Dark, Solarized Light, Gruvbox Dark, Tokyo Night, Catppuccin Mocha, Monokai, Sepia, Amber, Everforest Dark, Rose Pine, Kanagawa, and Gruvbox Light. Your own themes come after them.

Dark themes:

1. Galaxy: purple accents on near black; the default.
2. One Dark: the One Dark editor palette.
3. Dark: Star's original dark theme.
4. Phosphor: green monochrome, like an old terminal.
5. Dracula.
6. Nord: cool arctic blues.
7. Solarized Dark.
8. Gruvbox Dark: warm retro colors.
9. Tokyo Night.
10. Catppuccin Mocha: soft pastels.
11. Monokai.
12. Amber: amber on near black, with little blue light.
13. Everforest Dark: soft greens.
14. Rose Pine: muted rose and pine.
15. Kanagawa.

Light themes:

1. Galaxy Light: purple accents on white; Galaxy's partner.
2. One Light.
3. Light: Star's original light theme.
4. Solarized Light.
5. Sepia: warm paper tones for long reading.
6. Gruvbox Light.

High-contrast themes, where every text color reaches 7 to 1 or more:

1. Contrast: pure yellow and cyan on black.
2. High Contrast: softer colors for low vision, all well above 7 to 1.

Light and dark partners: Galaxy and Galaxy Light, One Dark and One Light, Dark and Light, Solarized Dark and Solarized Light, Gruvbox Dark and Gruvbox Light.

## Following your system's appearance

When following is on and you have not picked a theme yourself, textweaver matches your system at startup:

- In dark mode, it uses your theme's dark partner, or Galaxy.
- In light mode, it uses your theme's light partner, or Galaxy Light.
- In high-contrast mode (Windows contrast themes, macOS Increase Contrast, GNOME High Contrast), it uses High Contrast.

Choosing a theme yourself stops following, as in Star. It reads these settings: on Windows, the app light or dark mode and the contrast theme switch; on macOS, Appearance and Increase Contrast; on Linux, GNOME's color scheme and high-contrast settings, or `GTK_THEME`.

## Terminal colors

textweaver uses as many colors as your terminal offers:

- Full color (Windows Terminal, most modern terminals): the theme's exact colors.
- 256 colors: the nearest colors the terminal has, chosen so every pair still meets the contrast minimum.
- 16 colors: the terminal decides what its 16 colors look like, so textweaver paints the page itself (black for dark themes, white for light) and uses only colors that stay readable in the common terminal color sets. Faint bands, such as the sentence band, are left out; the underline still shows the sentence. On light themes, headings are underlined instead of bold, because some terminals brighten bold text until it fades.
- No color: attributes only. Headings bold, links underlined, quotes italic, the spoken word in reverse video, the sentence underlined.

To turn color off, set the `NO_COLOR` environment variable to any value, such as `1`. To choose a level yourself, set `TEXTWEAVER_COLOR` to `truecolor`, `256`, `16`, or `none`. For example, in a Linux or macOS shell:

```bash
export TEXTWEAVER_COLOR=256
```

In a Windows command prompt:

```powershell
set TEXTWEAVER_COLOR=256
```

textweaver never changes your terminal's own colors or cursor.

## HTML output

Documents converted to HTML use Galaxy unless the reader's system asks for light, in which case they use Galaxy Light; they use High Contrast when the system asks for more contrast, and the system's own colors when a Windows contrast theme is on. Links are always underlined and keyboard focus always shows a ring.

## Writing your own theme

A theme is a small text file in TOML format. Put it in a folder called `themes` inside the configuration folder above, with a name ending in `.toml`. textweaver loads every theme there when it starts; your themes come after the built-in ones when you press F5.

### The shortest theme

Two colors are enough. Everything else is worked out from them:

```toml
[theme]
name = "midnight"

[colors]
background = "#101820"
text = "#e8e8e8"
```

Colors are written as `#` and six hexadecimal digits (red, green, blue), or three digits for short.

### Changing one thing in a built-in theme

Start from a built-in theme and change only what you want:

```toml
[theme]
name = "my-galaxy"
inherits = "galaxy"

[colors]
heading1 = "#e0d0ff"
```

Anything you leave out comes from the theme you inherit from. Styles are copied as they are, so if you change a color that a highlight uses (heading 1 is also the spoken-word band), set that highlight too.

To start from a complete copy instead, copy a built-in theme's file (in textweaver's source, `crates/textweaver-theme/themes/`) and change `name`.

### Everything a theme can set

`[theme]`: `name` (letters, digits, and hyphens; the file name is used if it is missing), `display_name` (the name read aloud), `kind` (`dark`, `light`, or `high-contrast`; high contrast raises the text minimum to 7 to 1), `description`, `author`, `counterpart` (the light or dark partner), and `inherits`.

`[colors]`: `background`, `surface` (lists and panels), `text`, `dim_text` (hints, line numbers, rules; used on the page only), `heading1` to `heading6`, `link`, `code`, `code_background`, `quote`, and `error`.

`[styles.NAME]`, one table each for `selection`, `spoken_word`, `spoken_sentence`, `find_hit`, `current_find_hit`, `bookmark`, `note`, `status_bar`, and `focus`. Each can set:

- `foreground`: the text color inside the highlight.
- `background`: the band color, or `"none"` for no band.
- `attributes`: a list of `"bold"`, `"italic"`, `"underline"`, and `"reverse"`.

For example:

```toml
[styles.spoken_word]
foreground = "#101820"
background = "#ffd166"
attributes = ["bold"]
```

`[[user_highlights]]`, repeated for each color you can highlight text with; each has a `name` and the same three keys:

```toml
[[user_highlights]]
name = "yellow"
background = "#5a4d10"
attributes = ["bold"]
```

Keys textweaver does not know are kept, so a theme written for a newer version still loads.

### Contrast checks

When textweaver loads your theme, it measures every color against what it sits on. Text needs 4.5 to 1 (7 to 1 in a high-contrast theme); the focus band needs 3 to 1 against the page. Every highlight needs at least one attribute, and the spoken word must differ from its sentence, and the current find match from the others, by attribute and not only by color.

A theme that falls short still loads. textweaver tells you what to fix, for example: "Theme Midnight: 1 of 43 checks fail. Dim text on background: 4.0 to 1, needs 4.5 to 1."

If a file has a mistake, textweaver skips that file, loads the rest, and says which key or line is wrong, for example: "midnight.toml was not loaded: colors.text: "white" is not a color; write it as #rrggbb, for example #1e1e1e."

### Previewing a theme

From textweaver's source folder, this prints a theme's contrast report and the colors each part gets at every terminal level, as plain text:

```bash
cargo run -p textweaver-theme --example preview -- galaxy
```

To preview your own theme file, give its path:

```bash
cargo run -p textweaver-theme --example preview -- path/to/midnight.toml
```

Add `--swatch` to also print sample lines in color, for sighted checking.

## See also

- [Reading aids](reading-aids.md): the ruler, RSVP, and bionic reading, which use the theme's colours.
- [Settings](settings.md#display): the `[display]` settings.
- [Converting documents](converting.md): HTML output uses these themes.
- [ADR-0020: Themes](adr/0020-themes.md): the design, the contrast rules, and every palette adjustment.
- [Interactive pages](site/index.html): the pages in `docs/site/` use Galaxy and Galaxy Light.
- [Documentation index](README.md)
