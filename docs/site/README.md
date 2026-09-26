# Interactive guide pages

The folder `docs/site/` holds six interactive web pages about textweaver.
They explain how textweaver is built and how to use it.
Each page works offline, from your own disk.
Nothing is downloaded, and no page sends anything anywhere.

## The pages

- `index.html`: the start page. It says what textweaver is and who it is for, and links to the other pages.
- `architecture.html`: the crate map. Choose a crate to see its job, its design decisions, what it depends on, and what uses it. A table lists every crate in text.
- `speech-pipeline.html`: the speech pipeline. Step through the ten stages between pressing Space and hearing a word with its highlight. A worked example shows how a spoken word maps back to the source text.
- `keyboard.html`: the keyboard shortcuts. Search and filter every shortcut by category, layer, frontend, and platform.
- `features.html`: the features. See which features are done, partly done, or not there yet, with a link to each guide.
- `reading-aids.html`: the reading aids. Try RSVP, bionic reading, and the reading ruler on sample text.

## Open the pages

Double-click `docs/site/index.html` in your file manager.
It opens in your web browser.

You can also open the file from your browser's Open File command (usually Ctrl+O, or Cmd+O on macOS).
Any modern browser works.

The pages need JavaScript for the interactive parts.
Without it, each page says so and links to the Markdown guide with the same information.

## Change the color theme

Every page has a Color theme list at the top.
It has three choices:

- System: follow your computer's light or dark setting. This is the default.
- Dark (Galaxy): textweaver's default dark theme.
- Light (Galaxy Light): the light pair of Galaxy.

The pages remember your choice in the browser.
If the browser blocks storage, the choice lasts until you close the page.

## Regenerate the data

The pages carry their data inside them, so they work without a web server.
When the crates, the keymap, the themes, or the feature list change, regenerate the pages.
Run this from the repository folder:

```bash
python tools/gen_site_data.py
```

It prints one sentence per page, such as "Wrote docs/site/keyboard.html."
Running it twice in a row changes nothing the second time.

The script needs Python 3.11 or later and Rust's `cargo` command.
It uses the Python standard library only.
If `cargo` is missing, it says so and stops.

## Check that the data is current

To check without changing anything, run:

```bash
python tools/gen_site_data.py --check
```

It exits with status 1 and names each page that is out of date.
Continuous integration can run this check.

## Check the pages for accessibility

A small offline checker looks for common problems in the pages:

```bash
python tools/check_site_a11y.py
```

It checks the language, the title, the headings and their order, the skip link, the landmarks, labels on every control, text on every button, SVG and image text alternatives, duplicate ids, ARIA references, table captions and headers, the live region, focus outlines, and links to files that do not exist.
A link to a missing file is a problem.
If a guide is still being written, name it with `--allow-missing` (the path is relative to `docs/`) to report it as a note instead:

```bash
python tools/check_site_a11y.py --allow-missing README.md
```

The checker reads the pages as written, so it does not see what the scripts build when a page opens.
Check those parts in a browser with a screen reader and the keyboard.

## Where the data comes from

- The crates come from `cargo metadata --format-version 1 --no-deps`: names, versions, folders, programs, and dependencies. The dependents are worked out from the dependencies.
- Each crate's job is the first sentence of its crate documentation (the `//!` comments at the top of `src/lib.rs` or `src/main.rs`). Where that sentence is unclear, the script has a plainer sentence in its `JOB_OVERRIDES` table.
- Each crate's design decisions and its layer on the map come from two hand-kept tables in the script, `CRATE_ADRS` and `LAYERS`. The ADR titles are read from the first heading of each file in `docs/adr/`.
- The keyboard shortcuts come from `docs/keyboard.md`, which `cargo xtask keyboard` makes from the keymap. Never edit that file by hand. Three extra note keys come from `crates/textweaver-app/src/extra.rs`.
- The colors come from the Galaxy and Galaxy Light theme files in `crates/textweaver-theme/themes/`.
- The feature list is the hand-kept `FEATURES` table in the script. Update it when a feature's status changes.

## What the script writes

The script writes only between marker comments, such as `<!-- BEGIN generated: site-data -->` and `<!-- END generated: site-data -->`.
Everything else in a page is written by hand.
The generated parts are:

- `theme-init`: a tiny script that applies your saved color theme before the page is drawn.
- `base-style`: the theme colors and the styles every page shares.
- `header`: the skip link, the site name, the site navigation, and the Color theme list.
- `site-data`: the page's data, as JSON.
- `base-script`: the shared script for announcements and the theme switch.

To change the shared styles, the header, or the shared script, edit the script and run it again.

## Accessibility rules the pages follow

- Each page has a language, a meaningful title, one main heading, and headings in order.
- The first thing Tab reaches is a "Skip to main content" link.
- Each page has a header, a labelled site navigation, a main region, and a footer.
- Every control is a real HTML control with a visible label: buttons, lists, check boxes, radio buttons, sliders, and a search box.
- Everything works from the keyboard. Focus is always visible as a thick outline. Nothing traps the keyboard.
- Controls are at least 24 by 24 pixels.
- Nothing is shown by color alone. Marks also use borders, underlines, shapes, and words.
- Every picture has a text version next to it, such as a list or a table.
- Each page has one status region. Changes that matter, such as a new step or a new count of results, are announced there. Focus does not jump around.
- The pages follow your system's light or dark setting, and your reduced-motion setting. Nothing moves on its own; RSVP only runs when you start it.
- The pages work in Windows High Contrast and other forced-color modes.
- The layout fits a screen 320 pixels wide without scrolling sideways, and text can be enlarged to 200 percent.
- All text meets a contrast of at least 4.5 to 1, and focus outlines and control borders at least 3 to 1, in both themes.
- Links are always underlined.

## See also

- [Architecture guide](../architecture.md): how the crates fit together, in text.
- [Keyboard reference](../keyboard.md): the generated tables of every shortcut.
- [Reading aids guide](../reading-aids.md): every reading aid and its settings.
- [Themes guide](../themes.md): colors, contrast, and your own themes.
- [ADR-0001: Workspace layout and dependency policy](../adr/0001-workspace-and-dependencies.md): the rules behind the crate map.
- [ADR-0022: Reading aids](../adr/0022-reading-aids.md): the design of RSVP, bionic reading, and the ruler.
- [Documentation index](../README.md)
