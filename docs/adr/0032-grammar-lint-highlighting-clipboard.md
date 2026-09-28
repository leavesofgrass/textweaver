# ADR-0032: Grammar, lint, highlighting, and clipboard crates

- Status: accepted for lint, the clipboard, Unicode math, and notes export; proposed for grammar and highlighting, which wait for the owner's decision on one advisory (see "Held for the owner")
- Date: 2026-09-28

## Context

Wave 4's authoring extras (Agent W4g) are six small features for a blind author working in the terminal: grammar checking, Markdown lint, code highlighting, a clipboard for terminals without OSC 52, math drawn as Unicode in the reading view, and notes exported as references. Each could bring a new crate. The workspace denies unsafe code, prefers pure Rust, keeps the lean reader (`textweaver-tui --no-default-features`) small, and runs `cargo deny` on every change. The research (docs/research/wave4.md, docs/research/wave4-plan-review.md section 3) named the candidates: harper-core 2.11.0, rumdl 0.2.77, syntect 5.3.0 with two-face 0.5.2, and arboard 3.6.1.

## Decision

Each feature is behind a feature flag of the crate that uses it. The terminal reader turns them on by default; the lean reader has none of them.

### Markdown lint: our own rules, not rumdl

`textweaver-app`'s `lint` feature (no dependencies) checks five rules on the Markdown parser the app already uses (pulldown-cmark): heading levels that skip, bullet lists that change their marker, trailing spaces (not a two-space line break), link references with no definition, and bare web addresses. Ctrl+F8 and Ctrl+Shift+F8 step through them in edit mode; `tw lint` checks files.

rumdl was not used. As a library, `rumdl` 0.2.77 brings 189 crates (tokio, rayon, and its configuration system) for these five rules, and its 0.2 series released several times a week with no API promise. The five rules are about 300 lines on structure the app already parses.

### Clipboard: arboard where the terminal cannot take OSC 52

`arboard` 3.6.1, default features off (`image-data` would bring the image crate), with `wayland-data-control` on Linux (x11rb and wl-clipboard-rs, both pure Rust), behind `textweaver-tui`'s `clipboard` feature. The route is chosen once at startup from the environment: OSC 52 over SSH and in tmux (the system clipboard there is another computer's), in Windows Terminal, and in most terminals; the system clipboard in the old Windows console, macOS Terminal, VTE terminals, and the Linux console; both in Konsole. The first copy through the system clipboard is said once. A test-made `Tui` always uses OSC 52, so tests never touch the real clipboard.

### Unicode math: our own writer

`textweaver-math` gains `to_unicode` (a new module; the parsers are untouched): scripts as Unicode script characters when every character has one, the fraction slash, root signs, math-font letters, combining accents. `[reading] math_display = "unicode"` draws each formula that way in the reading view. Only the drawing changes: the terminal's layout draws the Unicode on the formula's first character and nothing on the rest, so speech, the highlight, bookmarks, and the cursor keep the document's positions. Edit mode and math exploration show the source. The default is the source, which reads better on a Braille display.

### Notes export: the citation crate's records

Notes and highlights become `textweaver_cite::Reference` records (type `document`) and are written by the citation crate's own BibTeX, BibLaTeX, RIS, and CSL-JSON writers, behind the app's `publish` feature: `tw marks FILE --export FORMAT`.

### Grammar: harper-core (held)

`harper-core` 2.11.0, pinned exactly, default features off (no thesaurus), behind the app's `grammar` feature. Ctrl+F7 and Ctrl+Shift+F7 step through problems and say them with their first fix; Alt+J lists the fixes. Harper's spelling problems are dropped, since textweaver's own checker (Alt+M) has them.

### Code highlighting: syntect and two-face (held)

`syntect` 5.3.0 with default features off and only `parsing` and `regex-fancy`, and `two-face` 0.5.2 with `syntect-fancy`, behind the terminal's `highlight` feature. The research said `default-fancy`; this takes less of it: `default-fancy` adds YAML and plist loading (yaml-rust, RUSTSEC-2024-0320, which the workspace ignores only for MathCAT), HTML output, and syntect's own syntaxes and themes, none of which the reader uses. Token kinds take their colors from the theme's roles (dim text, headings, quote), and keywords are bold and comments italic, so the kinds differ by more than color. The caret names a block's language when it enters it ("code, Python"); that part needs no crate and is in.

### Held for the owner

Both held features fail `cargo deny` on the same advisory: RUSTSEC-2025-0141, "bincode is unmaintained".

- syntect 5.3 loads its syntax dumps with bincode 1.3.3.
- harper-core brings the burn machine-learning framework (through harper-brill and harper-pos-utils, for part-of-speech tagging), 260 crates in all, and burn-core uses bincode 2.0.1.

Both are on their own branches, `wave4/g-highlight-syntect` and `wave4/g-grammar-harper`, each a single commit on top of the other extras, ready to merge if the owner adds an ignore for RUSTSEC-2025-0141 with a reason. Nothing that fails `cargo deny` is on the main W4g branch.

## Measurements

Release builds of the terminal reader (`cargo build --release -p textweaver-tui --bin textweaver`, thin LTO, Windows x64):

- `main` before W4g (9d54971): 46,827,520 bytes.
- The W4g branch (lint, clipboard, Unicode math, notes export): 46,953,984 bytes, 126,464 more.
- With code highlighting (`wave4/g-highlight-syntect`): 48,377,344 bytes, 1,423,360 more than the W4g branch (bat's syntaxes are most of it).
- With grammar (`wave4/g-grammar-harper`): 57,558,016 bytes, 10,604,032 more than the W4g branch (Harper's dictionary, rules, and the burn tensor code for its part-of-speech tagger).

Building grammar also compiles burn: a clean release build of the reader took 18 minutes with it and 10 without.

## Consequences

- The lean reader links none of these crates.
- Every feature is announced in words, meaning first ("Lint: …", "Grammar: …", "code, Python"), and none relies on color.
- New default keys: Ctrl+F8 and Ctrl+Shift+F8 (lint), and on the grammar branch Ctrl+F7 and Ctrl+Shift+F7. F7 and F8 alone keep their volume and speed-preset actions.
- When the continuous reader enters a code block it says "code block" (textweaver-text's narration); naming the language there too needs a change in `textweaver-text`, requested from its owner.

## See also

- [Editing](../editing.md): lint and the clipboard (grammar on its branch).
- [Math](../math.md#see-math-as-unicode): math as Unicode.
- [Notes](../notes.md): exporting notes as references.
- [Research for Wave 4](../research/wave4.md)
- [ADR-0018: Math](0018-math.md)
