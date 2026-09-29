# User interface refinement: the plan

Written on Monday, September 28, 2026, by the UI refinement planner, for the orchestrator and the owner. It plans the owner's six items in [the UI refinement list](ui-refinement.md) and fits them into Wave 6, as `docs/history/tasks.md` asks ("Wave 6 takes the UI refinement plan"). It changes nothing by itself: the orchestrator applies it after Wave 5 and the second documentation pass.

What it is built on: `CLAUDE.md`; the UI refinement list; [the 2026 roadmap](roadmap-2026.md) (the feature-complete list, Wave 6's sketch, the owner's answers and changes); [Wave 5, recalibrated](wave5-recalibrated.md) (the method and section 4's common rules); `docs/history/tasks.md` (Wave 5 status lines, "Pace after Wave 4", "After Wave 5"); the code named under each item; `docs/gui.md`, `docs/audio-export.md`, `docs/converting.md`; ADR-0011, ADR-0016, ADR-0029, ADR-0033, ADR-0036, and ADR-0035 on the `wave5/c3-documents` branch (not yet merged); and a read-only search of the owner's wiki for Star and abax. Versions were read today from crates.io, docs.rs and GitHub with the tools' default User-Agent. No personal identifier was sent. Nothing was built, run, downloaded, or deleted.

## The short version

- **Menus come from one model in the app core.** A new `menu.rs` lists File, Edit, View, Reading, Speech, Tools and Help, each item an action or a setting, its label from the catalog and its shortcut from the keymap. A test proves every command is in a menu. The terminal shows it as a list on F10; the GUI shows native menus on Windows and macOS (the `muda` crate) and the same list-style menu on Linux.
- **The palette gains context, not a new widget.** Each match says its name, category, what it does, and its keys ("Open, File: open a document. Ctrl+O"); an empty palette lists recent commands first; matching ranks prefix, then word starts, then letters in order. All in the app core, so both frontends get it.
- **The file browser is a list, like every other list.** One pane, on the app's `ListModel`: Enter opens a folder, an archive (zip, tar, 7z, through the archive code W3d already has) or a document; Backspace goes up; a line of detail says type and size. It doubles as the chooser for batch conversion and export. No copy, move or delete (question 5).
- **Speech export without ffmpeg, for MP3 and FLAC.** MP3 through LAME built from source (`mp3lame-encoder`, LGPL, question 2); FLAC in pure Rust (`flacenc`); ID3 chapters written by us or the `id3` crate. **M4B keeps ffmpeg:** no GPL-compatible in-process AAC encoder exists (question 3). Export from the menus, in the background, progress in tens.
- **Batch conversion from the menus** reuses `textweaver-convert` with two small additions (a progress callback and a cancel flag), asks three things (folder, format, where), and ends with one sentence and a list of failures. A conversion matrix test proves every loader converts to Markdown and PDF.
- **Formats:** the reader's Markdown loader learns what the HTML renderer already knows about Obsidian (any callout type, embeds, block references, tags, highlights, comments); new JSON and SVG loaders; content MathML; LaTeX past W5c3 (macros with arguments, `.bib` files, `\multicolumn`). Then Jupyter, SRT and VTT, Org, and Typst, in that order.
- **Six agents** across Wave 6's three sub-waves, at most three building at once, one GUI agent at a time. W6u replaces W6x.

## 1. Menus, in the GUI and the terminal

### What exists today (verified)

- No menu anywhere. The GUI has a banner with five buttons (Open, Font, Edit, Settings, Commands) and a toolbar named "Reading" (`crates/textweaver-xilem/src/gui.rs`, `build_tree`). Each button's key comes from the keymap and reaches UI Automation as `AcceleratorKey`, through the vendored `third_party/accesskit_windows` patch (ADR-0033).
- The keymap has 194 actions in nine categories (`Category` in `crates/textweaver-keymap/src/action.rs`: Reading, Navigation, Speech Cursor, Voice, Search, Bookmarks and notes, File, Editing, View and help), with catalog ids `category-*` in all six languages.
- **F10 is taken in the terminal:** `previous_chapter` is F10 (with Alt+PageUp and browse `Shift+D`), `next_chapter` is F11, settings is Shift+F10. In the GUI, F10 is free.
- GUI Alt-letter chords today: Alt+O (outline), Alt+M, Alt+J, Alt+N, and Alt+C in the edit layer. None collides with the access keys F, E, V, R, S, T, H.
- Star (the wiki's usage guide) had Qt menus File, Highlight, View, Speech, Profiles, Notes, Citations, Tools and Help, every item with a shortcut, and a Reading Aids submenu under View. Star's terminal had no menu; the palette (F2, `M-x`, `:`) was its command surface.

### Findings

- **muda** 0.20.0 (September 11, 2026), Apache-2.0 or MIT: native menus on Windows (an `HMENU` attached with `Menu::init_for_hwnd`) and macOS (`init_for_nsapp`); on Linux it needs GTK 3 or 4, which the project avoided on purpose (rfd uses the XDG portal). With winit, menu events go to the event loop through an `EventLoopProxy`, which `gui.rs` already has. Its accelerators fire only through `TranslateAcceleratorW`, which winit does not call; that suits us, since the keymap handles keys and the menu only shows them. [docs.rs](https://docs.rs/crate/muda/latest)
- **Native Win32 menus are accessible without work:** UI Automation exposes MenuBar, Menu and MenuItem with `AccessKey` from the `&` letter and `AcceleratorKey` from the text after a tab; NVDA and JAWS read both; Alt and F10 enter the bar. [MenuItem control type](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportmenuitemcontroltype), [MenuBar control type](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportmenubarcontroltype)
- **Xilem and Masonry have no menus yet.** A layer system landed (PR #1405) and menus are tracked in [linebender/xilem#1343](https://github.com/linebender/xilem/issues/1343). AccessKit 0.25.1 has the MenuBar, Menu, MenuItem, MenuItemCheckBox and MenuItemRadio roles and `Node::set_keyboard_shortcut`, but a drawn menu must then get every screen-reader behavior right by hand.
- **Terminal menus** (Midnight Commander's F9 bar, `tui-menu` 0.3.1 for ratatui) are drawn characters; a screen reader sees text and a cursor. A list that reads one item per line, place first, is what works on a 40-cell display.
- The WAI-ARIA menubar pattern (arrows inside, Escape out, Home and End, letters as access keys) is the model for the list menu's keys. [APG keyboard interface](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/)

### Recommended approach

1. **A menu model in the app core,** `crates/textweaver-app/src/menu.rs`: menus of items, each an `ActionId`, a submenu, a separator, or a toggle bound to a setting (checked state shown). Labels are catalog ids (`menu-file`, `menu-item-open`, with the access key marked `&` in each language); the shortcut text is `named_key_in` from the live keymap, so a key the owner changes in `keymap.toml` shows at once.
2. **The seven menus the owner named**, with submenus to keep each under about 20 items: File (Open, Open Path, Browse files, Recent, Save, Save As, Export as, Export audio, Batch convert, Library, Quit); Edit (edit mode, undo, clipboard, find and replace, insert, spell and grammar); View (outline, lists, themes, text size, font, reading aids submenu, access mode, language); Reading (play and stop, Move by submenu, Go to, bookmarks and notes submenu, tables, math, citations); Speech (voice manager, rate, pitch, volume, speed presets, engine, speech cursor); Tools (palette, dictation, define word, summarize, statistics, lint, vault, settings); Help (help, keyboard help, about, what's new).
3. **Tests in the model:** every action is in at least one menu; no two items in one menu share an access key; the top-level access keys never equal a GUI Alt-letter chord (a keymap check, so a future binding cannot steal Alt+F).
4. **The terminal:** F10 opens the menu as a list, "Menus, 1 of 7, File"; Enter or Right opens a menu, whose items read "Open, Ctrl+O, 1 of 11"; a letter moves to its access key's item (like a platform menu, not a filter); Enter runs; Left or Backspace goes up; Escape closes. Previous chapter keeps Alt+PageUp and `Shift+D` (question 1).
5. **The GUI:** native menus through muda on Windows and macOS, built from the model at start and rebuilt on a language or keymap change; a chosen item arrives as an `ActionId` through the proxy. On Linux, F10 and Alt open the same list menu in the GUI's existing `ChoiceList` dialog. The toolbar and banner stay.

### Alternatives rejected

- **A drawn Masonry menu bar everywhere:** weeks of work to match what Win32 gives free, and unproven with NVDA and JAWS. Revisit when Masonry ships menus.
- **muda on Linux:** brings GTK into a winit process; both want the main loop.
- **Direct `windows` crate calls instead of muda:** possible (the crate is already a dependency), but muda also covers macOS and the event plumbing. Kept as the fallback if muda's subclassing fights winit.

### Effort and accessibility checks

- Model and terminal: about 4 days (the catalog work in six languages is most of it). GUI native menus: about 3 days, plus a half-day spike on the first day that proves Alt and F10 reach the menu through winit.
- Checks: NVDA and JAWS say "menu bar", each item with its shortcut; checked toggles say "checked"; Escape returns focus to the document with no announcement lost; the terminal list's lines put the place first in 40 cells; the catalog test passes in six languages.

## 2. The command palette, with context

### What exists today (verified)

- Both frontends call `App::palette_candidates` (`crates/textweaver-app/src/help.rs`), which gives "id: help. keys" (`help-palette-item`). Matching (`palette_matches_in`) puts ids or translated help that start with the query first, then those containing every word; no ranking beyond that.
- The terminal palette is a one-line prompt: Tab completes to the longest common prefix and says up to five names; Up and Down step through matches and say each (`list_model.rs`, `recall` and `complete_prompt`). Nothing is shown as a list, and the category is never said.
- The GUI palette (`open_palette` in `gui.rs`) is a filter field over a `ChoiceList` of the same strings.
- There are no recent commands; other prompts keep answer history, but the palette uses Up and Down for matches.
- Star's palette, per the wiki: F2 in both interfaces, `M-x` and `:` in the terminal, a searchable list of labels with no category, shortcut column, or recents. So the refinement goes past Star.

### Findings

- VS Code shows the category as a prefix ("View: Toggle Sidebar"), the keys at the right, and "recently used" at the top. Palettes commonly fail screen readers when the highlighted row is only painted, not exposed as the selected, focused item.
- Fuzzy matchers: `nucleo-matcher` 0.3.1 (MPL-2.0, on the accepted list), `fuzzy-matcher` 0.3.7 (MIT, not updated in years). `sublime_fuzzy` has a non-standard licence. With under 200 commands, a small scorer of our own is enough and adds no dependency.

### Recommended approach

1. **One spoken and written form,** name first: "Open, File: open a document. Ctrl+O." (a new `help-palette-item` with a `category` argument, all six languages). The name is the action's short label, not its snake-case id; the id stays typeable.
2. **Ranking:** exact name, then name prefix, then word starts ("ep" finds "export pdf"), then letters in order, then help words; ties in help order. Matches in the catalog's language and English.
3. **Recent commands:** the last eight run from the palette or menus, kept in state, listed first when the query is empty and marked "recent" in words.
4. **The terminal:** unchanged keys, plus Ctrl+L (or the first Down after typing) opens the matches as a filtered list, so a user can hear them in context without committing.
5. **The GUI:** the `ChoiceList` rows get the new text; the selected row is the focused, selected option (checked in the UIA report). This lands in W6a5, which already owns "the command palette on the app's models".

### Alternatives rejected

- `nucleo-matcher`: fine licence and fast, but built for tens of thousands of items and match positions we never speak. Revisit if the palette grows to files or settings.
- Showing the category as a separate column: columns do not reach the Braille display; one line does.

### Effort and accessibility checks

- About 2 days in the app core, half a day in the GUI.
- Checks: each match's first 40 cells hold the name and category; Tab still completes; an empty query says "recent" first; the GUI's selected row is announced once per move.

## 3. A file browser and chooser, with archives

### What exists today (verified)

- GUI: the system Open dialog through `rfd` 0.17.2 (`file_chooser.rs`, `IFileOpenDialog` on Windows, XDG portal on Linux), and Open Path (Ctrl+Shift+G), a typed path with Tab completion (`path_complete.rs`).
- Terminal: Open is a typed path with the same Tab completion.
- Archives: `crates/textweaver-formats/src/archive.rs` lists (`list`) and reads (`read_member`) zip, tar, tar.gz and 7z, with `book.zip!chapter.pdf` member paths, nesting to four, and limits against hostile archives. Opening an archive shows a document of links, one per readable member. Locked versions: `zip` 8.6.0, `tar` 0.4.46, `sevenz-rust2` 0.23.0.
- Lists: `ListModel` (`list_model.rs`) gives both frontends filtered, announced lists (outline, library, notes, voices).
- abax, per the wiki, had a Worker-style two-pane manager (Tools, File manager, Ctrl+Shift+F): copy, move, delete, rename, zip and tar.gz create and extract with path-traversal guards, recursive find, and Worker's configurable command buttons (`{dir}`, `{path}`, `{sel}`). It did not browse into archives, had no previews, and the wiki records no screen-reader design for its panes.

### Recommended approach

1. **A single-pane browser on `ListModel`,** `crates/textweaver-app/src/browse.rs`. Each row, meaning first: "notes.md, Markdown, 12 KB, 3 of 40"; folders "Week 1, folder, 12 items"; archives "course.zip, zip archive". Enter opens a folder or archive (listed through `archive::list`, members as `course.zip!week1/`), or opens a document. Backspace goes up, out of an archive to its folder. Typing filters, as in every list. Ctrl+Enter chooses the current folder (for batch conversion and export targets).
2. **A preview line:** Alt+End (already "say status") on a row says the document's title and first sentence, loaded on a helper thread with the loaders' limits. No drawn preview pane in the first version.
3. **Readable only, by default:** a toggle shows every file. Sorted folders first, then by name; Ctrl+R cycles name, date, size.
4. **Places:** the document's folder, the start folder, the library's folders, and drives on Windows, as the first list when opened from the menu.
5. **Frontends:** the terminal on "Browse files" (File menu, and a palette command); the GUI in its list dialog. The system Open dialog stays on Ctrl+O; the browser is the way into archives.

### Alternatives rejected

- **Two panes and file operations now:** textweaver is a reader, and a delete from a reader is the kind of action the owner's deletion rules exist for. A second pane only earns its place for copy and move. Question 5.
- **A tree view:** trees are harder by ear than a list that enters and leaves folders, and both frontends already have the list.
- **RAR:** the UnRAR licence forbids writing a RAR-compatible archiver; not GPL-clean.

### Effort and accessibility checks

- About 4 days in the app core and terminal, 1 day in the GUI.
- Checks: each row's first 40 cells hold name and kind; entering a folder says its name and count once; Backspace from an archive's root lands on the archive's row; a hostile archive (fixtures from W3d) is refused with a sentence.

## 4. Speech export to WAV and MP3, without ffmpeg

### What exists today (verified)

- `crates/textweaver-export` writes WAV itself (`wav.rs`), builds chapters and subtitles, and hands MP3 (LAME VBR quality 2, ID3 chapters) and M4B (AAC 64 kbit/s, chapters) to ffmpeg (`ffmpeg.rs`, ADR-0011). Only `tw export-audio` uses it; the app does not depend on the export crate, so the reader and GUI cannot export audio. The library API already supports cancelling between sentences.
- `deny.toml` accepts MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, GPL-3.0-or-later and a few more; no LGPL.
- Star exported MP3, OGG, MP4 and WAV through ffmpeg or pydub, and chaptered M4B (wiki: Star 0.1.21; the bitrate moved to 128 kbit/s in 0.1.22).

### Findings, by format

- **MP3:** `mp3lame-encoder` 0.2.5 (August 20, 2026) over `mp3lame-sys` 0.1.11, which builds the bundled LAME 3.100 statically (cc on Windows, autotools on Unix). Licence LGPL, compatible with a GPL-3.0-or-later program, but new to `deny.toml`. The pure-Rust option, `shine-rs` 0.1.3 (a port of the fixed-point shine encoder), is fast but its authors say it is not competitive with LAME; its licence chain needs checking. MP3 patents expired in 2017. [mp3lame-sys](https://docs.rs/crate/mp3lame-sys/latest), [shine-rs](https://lib.rs/crates/shine-rs)
- **AAC for M4B:** Fraunhofer's FDK AAC licence is not GPL-compatible, and no usable pure-Rust AAC encoder exists. MP4 writing is easy (`mp4ameta` 0.13, MIT or Apache-2.0, writes both chapter styles), but it needs AAC to put in it. [FDK-AAC licence](https://spdx.org/licenses/FDK-AAC.html)
- **Opus:** only libopus bindings encode (BSD); `audiopus_sys` is unmaintained (RUSTSEC-2026-0150); newer bindings need a close look. The `ogg` crate (BSD) writes the container. Chapters are Vorbis comments.
- **Vorbis:** `vorbis_rs` (BSD, C bindings with aoTuV). No pure-Rust encoder.
- **FLAC:** `flacenc` (pure Rust, Apache-2.0, 0.5 line). Lossless, about half of WAV's size for speech. [flacenc](https://github.com/yotarok/flacenc-rs)
- **Chapters and tags:** `id3` 1.17.1 (MIT) writes CHAP and CTOC frames; `lofty` is MIT or Apache-2.0 (MP4 chapter writing unconfirmed). Symphonia only decodes.

### Recommended approach

1. **An `Encoder` step in `textweaver-export`,** fed the same PCM the WAV writer gets, so no intermediate WAV is needed for MP3 and FLAC: `mp3` feature (LAME, VBR quality 5 by default for speech, mono; `[export] mp3_quality`), `flac` feature (flacenc). ID3v2 title, artist, album and CHAP/CTOC chapters with the `id3` crate. ffmpeg stays for M4B, and as an MP3 fallback when the `mp3` feature is off.
2. **From the reader and the GUI:** File, Export audio: the format (MP3, FLAC, WAV, M4B when ffmpeg is found), then where (next to the document by default). It runs as a background job like document export ("Exporting audio, 30 percent", in tens, never faster than every ten seconds), then "Wrote essay.mp3: 42 minutes, 12 chapters. Open it? y or n." Escape during the job asks "Stop the export? y or n."
3. **Only engines that write files are offered;** the menu item says why when none can.
4. **Later, if the owner asks:** Opus in Ogg (smallest files for speech) once a maintained libopus binding is chosen.

### Alternatives rejected

- **shine-rs as the default:** worse quality at speech bitrates and an unchecked licence. Kept as the answer if the owner declines LGPL.
- **FDK AAC in process:** not GPL-compatible.
- **Dropping M4B:** the owner's Star users used it, and ffmpeg still works.

### Effort and accessibility checks

- Encoders: about 3 days (LAME's build in the Docker image and on Windows is the risk). Reader and GUI export: about 2 days, after the menus.
- Checks: progress never faster than the current rule; the result sentence names format, length and chapters first; an MP3's chapters appear in a player that reads ID3 chapters; `cargo deny` passes with the owner's licence decision.

## 5. Batch conversion from the menus

### What exists today (verified)

- `tw convert` and `textweaver-convert` convert files and folder trees on every core, mirror the tree, skip outputs newer than their source, write atomically, fall back to Pandoc (sandboxed, with a timeout) for other formats, and give a `Summary` with `sentence()`, `failures()` and `warnings()`; `watch.rs` is the hot folder. Seven targets: Markdown, HTML, text, EPUB, DOCX, BRF, PDF (with `--template` since W5g).
- `Converter::run_plan` returns only at the end: no progress callback and no cancel flag.
- The app already depends on the convert crate (feature `publish`) for single-document export, with "Still exporting to PDF" at two seconds, then every ten.
- Star had File, Batch Convert (Ctrl+Shift+C) and Watch Folder (Ctrl+Shift+W), to Markdown, text or Braille (wiki usage guide).
- No test converts one fixture of every loader to every target.

### Recommended approach

1. **The convert crate:** `run_plan_with(plan, on_file: impl Fn(&FileResult) + Sync, cancel: &AtomicBool)`; the existing `run_plan` calls it. Workers check `cancel` before each file.
2. **The flow,** File, Batch convert: the folder (the browser from item 3, Ctrl+Enter), the target (a list of seven, Markdown first), where (beside the sources, or a chosen folder; default a `converted` folder beside the source folder), then "Convert 48 files to PDF into D:\Notes\converted? y or n." It runs in the background; progress in tens of percent, at most every ten seconds; the reader stays usable.
3. **The end:** "Converted 42 files; 4 were up to date; 2 failed." Then a list of the failures, each "report.docx: the file is damaged", with Enter opening the source; the same list saved as `conversion-report.txt` in the output folder.
4. **The matrix test** (`crates/textweaver-convert/tests/matrix.rs`): one small fixture per loader extension (reusing `fixtures/`), converted to Markdown and to PDF; each must succeed, produce a non-empty result, and keep the headings the fixture has. PDF output also runs the veraPDF job (`second-tool.yml`) in CI. A loader without a fixture fails the test, so a new loader brings one.

### Alternatives rejected

- A dialog with every option at once: harder by ear than three short questions, each with its default.
- Announcing each file: floods the screen reader on a large folder.

### Effort and accessibility checks

- Convert crate and matrix: about 2 days. The flow in the app and terminal: about 2 days; the GUI with W6a6.
- Checks: the confirm question fits 40 cells with the count first; progress respects the ten-second floor; the failure list reads name, then reason; cancel leaves no half-written file.

## 6. More formats, and deeper parsers

### What exists today (verified)

- **Markdown:** the reading loader (`crates/textweaver-formats/src/markdown.rs`, pulldown-cmark 0.13.4) enables GFM alerts (the five GitHub types), wiki links, math and front matter. The HTML renderer (`crates/textweaver-render`, `Flavor::Obsidian`) already handles any callout type with titles and folding, embeds, tags, block references and highlights, but only for output. So a reader hears `[!tip]` as text in an Obsidian note.
- **LaTeX:** W5c3 (`wave5/c3-documents`, ADR-0035) reads a large subset: sections, lists, tables, floats with captions, theorems, math environments with numbers, `\ref` and `\cite`, footnotes, argument-free macros, and `\input` inside the folder. Not read: macros with arguments, `\newenvironment`, `.bib` files, `\pageref` pages.
- **MathML:** `mathml.rs` converts presentation MathML (EPUB, and HTML after W5c3) to LaTeX under a `Math` marker; content MathML (`<apply>`, `<ci>`, `<cn>`) is not handled; no loader claims `.mml`.
- **JSON, SVG, Jupyter, Org, Typst, BibTeX:** no native loader; `.ipynb`, `.org`, `.typ`, `.bib` and `.fb2` go through Pandoc when it is installed. The HTML loader drops `<svg>` elements whole.
- `textweaver-cite` parses BibTeX already (`bibtex.rs`, `biblatex` 0.12).

### Findings

- **Obsidian:** callouts `> [!type]` with 13 built-in types and aliases, `+` and `-` folding; embeds `![[note]]`, `![[note#heading]]`, `![[note^id]]`, `![[image.png|300]]`; block ids `^id`; tags `#a/b`; `==highlight==`; `%%comment%%`; properties in YAML. [Obsidian callouts](https://help.obsidian.md/callouts)
- **LaTeX:** no Rust crate parses whole documents into a semantic tree; own parsers are the norm (W5c3 chose that). `texlab` is GPL-3.0 (a reference only), `tree-sitter-latex` MIT, `mitex` without a release since 2024.
- **MathML:** no Rust content-to-presentation converter was found. Whether MathCAT's canonicalization accepts content MathML is unverified.
- **JSON:** `serde_json` with `preserve_order` keeps key order; spans for highlighting need our own small parser or a span-keeping crate (not verified).
- **SVG:** `usvg` 0.48.1 is a render tree and does not promise to keep `<title>`, `<desc>` or ids; `roxmltree` 0.21 (already a dependency) keeps everything. SVG-AAM: `<title>` names an element, `<desc>` describes it, `role="img"` makes a drawing one graphic. [SVG-AAM](https://www.w3.org/TR/svg-aam-1.0/)

### Recommended approach

1. **Obsidian in the reader:** share the callout rules (types, aliases, titles, folding) between the renderer and the reading loader, so both agree; today they live in `textweaver-render`, which the formats crate does not depend on, so W6o moves the rule table into the formats crate and the render crate calls it (a new dependency edge, proposed in its report), and read a callout as a labeled block: "Tip: Remember. ..." with its fold state said once ("collapsed" is still read). Embeds of another note in the vault are read in place with "Embedded from Note" before and "End of embed" after (depth 2, cycles refused); image embeds as graphics by file name; block ids hidden and usable as link targets; tags read as words ("tag physics slash waves"); highlights as a `Highlight` marker; comments skipped. Properties are spoken as front matter already is.
2. **JSON** (`json.rs`): objects become headings and lists, "address, object, 3 keys"; arrays "items, array, 12 entries"; values plainly ("name: Ada Example"). Heading keys move with `h`, so a JSON file navigates like a document. Also `.jsonl` (one heading per line) and `.ipynb` on the same code: markdown cells through the Markdown loader, code cells as code with their language, text outputs as quotes, image outputs as graphics.
3. **SVG** (`svg.rs`, roxmltree): the title and description first, then each group with a title as a list item, then text elements in document order. A drawing with neither title, description nor text reads "Drawing with no description". Inline `<svg>` in HTML gets the same reading instead of being dropped.
4. **Content MathML:** a small converter in `mathml.rs` for the common operators (arithmetic, relations, powers, roots, fractions, sums, integrals, functions, sets) to the same LaTeX; unknown ones read by name. `.mml` files open as one formula.
5. **LaTeX past W5c3:** macros with up to nine plain `#n` arguments (no `\expandafter`, the same step budget); `\newenvironment` begin and end text; `\bibliography{refs}` read through `textweaver-cite` into a References section; `\multicolumn` and `\multirow` spans said ("spans 3 columns"); `\includegraphics` described by its caption or the `alt` key.
6. **The survey, ranked for a student or writer** (native loaders, each small unless noted): Jupyter notebooks (with JSON, above); SRT and VTT transcripts (we already write them); BibTeX files as a readable bibliography (cite's parser); Org mode (Pandoc today; a native reader of headings, lists, tables and blocks is about 3 days); Typst (`typst-syntax`, Apache-2.0; headings, lists, math through our parser; about 4 days); YAML and TOML as data on the JSON reader; FB2 (XML, 1 day). Left to Pandoc: AsciiDoc, reStructuredText. Out: CHM, DjVu, XPS, Pages (C libraries or proprietary).

### Alternatives rejected

- `usvg` for SVG: it throws away what a reader needs.
- An XSLT engine for content MathML: a large dependency for a rare input.
- `obsidian-export` (BSD-2-Clause-Patent, not on the accepted list): it exports a vault; it does not give a reading tree.

### Effort and accessibility checks

- Obsidian 3 days; JSON and Jupyter 3 days; SVG 1.5 days; content MathML 2 days; LaTeX extras 3 days; each further format as above.
- Checks: a callout's type is said before its text, in words; JSON keys and values read without punctuation noise; an SVG with no text says so once; every new loader gets fuzz targets and hostile-input tests (the rule since W3d).

## Quick wins

Small changes worth doing first. Sizes: small is under half a day, medium about a day.

1. **Category in the palette's line** (`help-palette-item` with `category`, six languages). W6u, or any free agent before Wave 6. Small.
2. **Recent commands** at the top of an empty palette. W6u. Small.
3. **Obsidian callouts of any type in the reader** ("Tip: Remember", not "[!tip] Remember"). W6o. Small.
4. **Inline `<svg>` in HTML** reads its `<title>` or `aria-label` as a graphic's description instead of vanishing (`html.rs`'s skip list). W6o. Small.
5. **`.json` opens as an indented outline** even before the full JSON loader: one heading per top-level key. W6o. Small.
6. **FLAC export** (`flacenc`, no licence question): `tw export-audio --out book.flac`. W6v. Medium.
7. **Batch failures saved to `conversion-report.txt`** from `tw convert` as well. W6k. Small.
8. **The menu-key check** in the keymap tests (no GUI Alt-letter chord equals a top menu's access key), before any menu exists. W6u. Small.
9. **F10 freed in the terminal** (previous chapter keeps Alt+PageUp and `Shift+D`), with the "Keys: what changed" note. W6u, after question 1. Small.

## Other suggestions that fit the owner's aims

- **Keyboard learning from the menus:** Help, "What does this key do?" (press a key, hear its action and menu path). Uses the keymap's reverse lookup; about a day.
- **Recent documents in the File menu** from the library's bookshelf, most recent first, with the reading position ("essay.md, 43 percent").
- **"Export as" remembers the last format and folder** per document, so a second export is one key.
- **A menu path in help:** F1's key list and `docs/keyboard.md` gain the menu path of each command ("File, Export as, PDF"), generated from the model by `cargo xtask`.
- **The GUI's window name on focus** and **startup announcements** (from the list's "found so far") belong in W6a5.

## How this fits the roadmap

### Joins, overlaps, replaces

- **W6u (menus and palette, app and terminal) replaces W6x** as sub-wave 6a's app-message agent. W6x's speed presets are done (W5y); its leftovers (`WordTrack` off the input thread, the JSON-RPC `insert` method, B1 and session 3 leftovers in the terminal) move into W6u.
- **W6a5 (GUI parity)** keeps its list and takes item 2's GUI part (it already owns "the command palette on the app's models").
- **W6a6 (the GUI's menus, browser, export and batch dialogs)** is new, after W6a5, because one GUI agent works at a time.
- **W6f (file browser), W6v (audio export), W6k (batch conversion and the matrix), W6o (formats)** are new. W6o's LaTeX part starts after W5c3 merges.
- **W6g** is done (W5g). **W6c5 and W6b** stay planned and are not required for the final alpha; they take slots after these. **W6d** (streaming dictation, required) keeps its place in 6c. **W6e, W6t, W6p** unchanged.

### Order and dependencies

- **Sub-wave 6a:** W6a5 (GUI), W6u (app and terminal, messages), W6o (formats; no app files). Three builders.
- **Sub-wave 6b:** W6f (after W6u merges: it adds a File menu item and shares list keys), W6k (convert crate first, needs nothing; its app flow uses W6f's folder choice, so W6k merges after W6f), W6v (export crate first; its app flow after W6u). Three builders.
- **Sub-wave 6c:** W6a6 (after W6a5, W6u, W6f, W6k, W6v merge), W6d, then W6c5 or W6b as slots free; W6t and W6p throughout.
- **Messages:** W6u is the app-message agent of 6a; in 6b, W6f, W6k and W6v each append ids in their own labeled block of the six `.ftl` files, as W5y and W5s did, and merge in that order.
- **Merges:** W6o, W6u, W6a5, W6f, W6k, W6v, W6a6, then 6c's others, W6p last.

### What waits for the owner's sessions

- **Session 4 (the GUI)** after W6a6: the native menus with NVDA and JAWS. W6a5's own session gate is unchanged.
- **A terminal session** after W6u: the F10 menu and the palette on the Mantis Q40. Gates only W6u's follow-up fixes.
- **Check 3 (documents)** after W6o: an Obsidian note with callouts and embeds, a JSON file, an SVG. Gates only W6o's follow-ups.
- Nothing else waits.

## The agent split

Each brief follows section 4 of [Wave 5, recalibrated](wave5-recalibrated.md) (the common rules), with `wave6/` branches: only the owner overrides rules; privacy; where you work; deleting; building; checks; waiting (every wait loop has a time limit and ends when you report); generated docs; messages in all six languages; the 40-cell rule; downloads pinned and digest-checked, `cargo deny` green, a new licence waits for the owner; the memory rule for light builders. ADR numbers: the orchestrator assigns the next free ones after 0042.

### W6u: menus and the palette, in the app and the terminal (6a, P1, app messages)

**Branch** `wave6/u-menus-palette`. **ADR:** one, "Menus from one model".
**Owns:** `crates/textweaver-app/src/menu.rs` (new) and its tests; the palette parts of `help.rs` and `list_model.rs`; recent commands in the app's state; `crates/textweaver-keymap` (the F10 change and the access-key check); the terminal's menu drawing in `crates/textweaver-tui/src/ui.rs`; `docs/keyboard.md` and `docs/reading.md` sections; the six `.ftl` files; `fixtures/u/`.
**Not to touch:** `textweaver-xilem`, formats, export, convert, `browse.rs`.
**Deliverables, one commit each:**
1. The access-key check and the F10 change (after question 1's default), with the "Keys: what changed" line.
2. The menu model: seven menus, submenus, toggles; labels and access keys in six languages; tests that every action is in a menu and no access key repeats.
3. The terminal menu on F10, as a list, keys as item 1's plan says.
4. The palette: the name-first line with category, the ranking, recent commands, the list view of matches.
5. W6x's leftovers: `WordTrack` off the input thread; JSON-RPC `insert`; session findings listed by the orchestrator.

**Checks:** the common set; the 40-cell test covers every menu line and palette line in six languages.
**The owner's checklist (terminal session):** (1) F10: "Menus, 1 of 7, File" first on the display? (2) File, Right, Down: each item's name then its keys; (3) press `x` in the File menu: lands on Export as? (4) F2, type `ep`: "Export PDF, File: ..." first? (5) F2 with nothing typed: recent commands first, said as recent.

### W6o: formats (6a, P2)

**Branch** `wave6/o-formats`. **ADR:** one, "Obsidian, JSON, SVG and content MathML in the reader".
**Owns:** `crates/textweaver-formats/src/{markdown,json,svg,mathml,html}.rs` (json and svg new; html only for inline SVG); the shared callout rules (moved from `crates/textweaver-render/src/preprocess.rs` into the formats crate, the render crate calling them); registry lines; `latex.rs` (after W5c3 merges); `fuzz/` targets `json`, `svg`, `obsidian` appended; `fixtures/o/`; `docs/converting.md`'s format list; one open-failure id per new format in six languages.
**Not to touch:** `textweaver-app` beyond ids, `textweaver-math`'s parsers (call them), the writers.
**Deliverables:**
1. Quick wins 3, 4, 5.
2. Obsidian in the reader: callouts, embeds (depth 2, no cycles, inside the vault folder only), block ids, tags, highlights, comments.
3. JSON, JSON Lines, and Jupyter notebooks.
4. SVG files and inline SVG.
5. Content MathML and `.mml` files.
6. After W5c3 merges: macros with arguments, `\newenvironment`, `\bibliography` through `textweaver-cite` (one-line dependency, ask in the report), spans in tables.

**Checks:** the common set; hostile-input tests; `cargo fuzz build` and ten minutes of each new target in the container; parity unchanged.
**The owner's checklist (check 3):** (1) an Obsidian note with a `[!warning]` callout: "Warning" first? (2) a note embedding another: the start and end of the embed said once; (3) a JSON file: `h` moves by key, values read without brackets; (4) an SVG chart with a title: title first; one without: "Drawing with no description"; (5) a notebook: code cells named by language.

### W6a5: GUI parity, with the palette's context (6a, P1, the GUI)

**As the roadmap's W6a5,** plus: the palette's `ChoiceList` uses W6u's line and ranking once merged (until then, the existing strings); the selected row is focused and selected in the UIA report; the window's name on focus and the startup announcements from the refinement list. **Not to touch:** menus (W6a6's). Its checklist is session 4's as the roadmap has it, with one line: (5) F2 in the GUI: each move says the name and category once.

### W6f: the file browser (6b, P1, messages in its own block)

**Branch** `wave6/f-browser`, after W6u merges. **ADR:** one, "A file browser on the list model".
**Owns:** `crates/textweaver-app/src/browse.rs` (new) and its ids (own labeled block); a public listing helper in `crates/textweaver-formats/src/archive.rs` if `list` needs one; the File menu's "Browse files" item in `menu.rs` (one line); `docs/reading.md`'s section; `fixtures/f/` (reuse `fixtures/w3d/` archives).
**Not to touch:** `textweaver-xilem`, the loaders, the palette.
**Deliverables:** places list; folders and archives entered and left; rows meaning first; the preview line on Alt+End on a helper thread with limits; readable-only toggle and sorting; Ctrl+Enter chooses a folder and returns it to a caller (the API W6k and W6v use).
**Checks:** the common set; the hostile archives refused with a sentence; the 40-cell test on rows.
**The owner's checklist (terminal session):** (1) File, Browse files: places first; (2) enter a zip, then a folder inside it, then Backspace twice: where you land said once; (3) Alt+End on a document: its title and first sentence; (4) Ctrl+Enter on a folder from Batch convert: the folder is taken.

### W6k: batch conversion and the conversion matrix (6b, P2, messages in its own block)

**Branch** `wave6/k-batch`. **ADR:** status update on ADR-0016.
**Owns:** `crates/textweaver-convert` (the callback and cancel API; `tests/matrix.rs`), `crates/textweaver-cli/src/cmd/convert.rs` (the report file), `crates/textweaver-app/src/batch.rs` (new) and its ids, one File menu line, `docs/converting.md`'s batch section, missing fixtures under `fixtures/k/`.
**Not to touch:** loaders (report a failing loader; W6o or the orchestrator fixes it), `browse.rs` beyond calling it.
**Deliverables:** 1. The callback and cancel API, `run_plan` unchanged. 2. The matrix test to Markdown and PDF, every loader with a fixture. 3. `conversion-report.txt`. 4. The app flow (starts after W6f merges): three questions, the confirm line, progress at the ten-second floor, the summary and the failures list.
**Checks:** the common set; the second-tool workflow on the branch for the matrix's PDFs.
**The owner's checklist:** (1) Batch convert a folder of mixed files to Markdown: the confirm line starts with the count; (2) progress no more than every ten seconds; (3) the summary sentence, then the failures list, Enter on one opens it; (4) Escape during a run: asked, then no half-written file.

### W6v: audio export in process (6b, P2, messages in its own block)

**Branch** `wave6/v-audio`. **ADR:** status update on ADR-0011, or a new one if the encoder choice is large.
**Owns:** `crates/textweaver-export` (encoder step, `mp3` and `flac` features, ID3 chapters), `crates/textweaver-cli/src/cmd/export_audio.rs`, `crates/textweaver-app/src/audio_export.rs` (new, the app then depends on the export crate behind a feature) and its ids, one File menu line, `[export] mp3_quality` (four places), `docs/audio-export.md`, `docker/` changes for LAME's build only if needed (say what), `fixtures/v/`.
**Not to touch:** engines, `textweaver-speech` beyond reading its API, the GUI.
**Deliverables:** 1. FLAC (quick win 6). 2. MP3 in process if the owner said yes to question 2 (else shine-rs after its licence is checked, or stop and report). 3. ID3 chapters without ffmpeg, checked by reading them back. 4. M4B still through ffmpeg, said as such when it is missing. 5. Export audio from the reader, in the background, with the result question.
**Checks:** the common set; `cargo deny`; binary size before and after; an export of `fixtures/sample.md` with the recording double, decoded back and compared in length.
**The owner's checklist:** (1) File, Export audio, MP3: progress in tens; (2) the result sentence: format, length, chapters first; (3) the MP3 in a player that shows chapters; (4) FLAC plays; (5) M4B without ffmpeg: one clear sentence before any work.

### W6a6: the GUI's menus and the new dialogs (6c, P1, the GUI)

**Branch** `wave6/a6-gui-menus`, after W6a5, W6u, W6f, W6k and W6v merge. **ADR:** one, "Native menus in the GUI".
**Owns:** `crates/textweaver-xilem` (all), `docs/gui.md`, `docs/screenshots/xilem-gui/`.
**Deliverables:** 1. Day one spike: muda's menu on the winit window; Alt and F10 enter it, NVDA says "menu bar"; if not, stop and report (fallback: the `windows` crate directly). 2. Native menus on Windows and macOS from the model, rebuilt on language and keymap changes; checked toggles. 3. The list menu on Linux. 4. The browser, batch and audio export flows in the window's dialogs.
**Checks:** the common set; the UIA report shows MenuBar, Menu and MenuItem with `AcceleratorKey`; the AT-SPI dump on Linux; screenshots at 100 and 200 percent; never the foreground, never audio.
**The owner's checklist (session 4):** (1) Alt, then Right to Reading: "Reading menu", then items with keys; (2) F10 does the same; (3) a checked toggle under View says "checked"; (4) Escape from a menu: back in the document, where you were; (5) File, Browse files, into a zip: each row name first.

## Risks

- **muda inside masonry_winit** may fight winit over the window procedure or Alt. The day-one spike decides; the fallback is direct Win32 calls, then the list menu everywhere.
- **LAME's build** needs autotools on Linux and macOS: the Docker image and CI runners may need a package. If it slips, FLAC and ffmpeg MP3 cover the release.
- **Catalog load:** the menus add about 250 ids in six languages. Mitigation: labels reuse action help where possible; the owner spot-checks listed ids.
- **Merge pressure on `menu.rs`:** W6f, W6k and W6v each add one line; merge in order, each rebasing on the last.
- **Embeds reading outside a vault:** limited to the note's own folder tree and depth 2, with the same traversal checks as LaTeX's `\input`.
- **Unverified:** MathCAT's handling of content MathML; `flacenc`'s exact latest version; whether `lofty` writes MP4 chapters; `shine-rs`'s licence chain.

## Questions for the owner

Each has a default, so work never waits.

1. **F10 in the terminal opens the menu,** and previous chapter keeps Alt+PageUp and `Shift+D` (F11 stays next chapter)? **Default: yes;** F10 is the menu key in Windows programs, and the GUI already uses Alt+PageUp.
2. **May the LGPL licence (LAME, through `mp3lame-encoder`) be added to `deny.toml`** so MP3 is written without ffmpeg? **Default: yes, behind a default-on `mp3` feature;** LGPL is compatible with GPL-3.0-or-later, and the build can drop it with one flag.
3. **M4B keeps needing ffmpeg,** since no GPL-compatible AAC encoder runs in process; MP3 with ID3 chapters becomes the in-process audiobook? **Default: yes.**
4. **Native menus in the GUI on Windows and macOS (muda), and the list menu on Linux?** **Default: yes;** native Win32 menus are what NVDA and JAWS know best.
5. **The file browser opens and chooses only, with no copy, move, rename or delete?** **Default: yes;** a two-pane Worker-style manager with file operations waits until you ask for it.

## See also

- [The UI refinement list](ui-refinement.md): the owner's six items and what was found.
- [The 2026 roadmap](roadmap-2026.md): Wave 6's sketch and the feature-complete list.
- [Wave 5, recalibrated](wave5-recalibrated.md): the common rules in section 4.
- [Tasks and agent briefs](../history/tasks.md): where the adopted plan goes.
- [ADR-0011: audio export](../adr/0011-audio-export.md), [ADR-0016: rendering and conversion](../adr/0016-rendering-and-conversion.md), [ADR-0033: the GUI after session 2](../adr/0033-gui-session-2-and-edit-mode.md).
- [Audio export](../audio-export.md), [Converting](../converting.md), [The GUI](../gui.md).
- [Documentation index](../README.md)
