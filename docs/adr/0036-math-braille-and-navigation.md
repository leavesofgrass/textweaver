# ADR-0036: Math braille and navigation on MathCAT

- Status: accepted, behind the `mathcat` feature; the order of the status line's words and braille waits for the owner's session B1 on the Braille display
- Date: 2026-09-28

## Context

ADR-0029 brought MathCAT (DAISY, MIT) in for math speech and left braille and navigation for later: MathCAT issue #827 made `get_navigation_braille` panic in builds with MathCAT's `no-unsafe` feature, which is how this workspace builds it, since it denies unsafe code. Until then the BRF writer wrote a formula as its spoken words, and exploring a formula (Alt+Shift+X) used textweaver's own navigator, with no braille.

On Monday, September 28, 2026, issue #827 was still open, with no comments and no linked pull request. crates.io's newest versions were 0.7.7-alpha.1 (September 23, before the issue) and 0.7.6-rc.3 (August 23), with no 0.7.6 final. The owner chose Nemeth first, read on a HumanWare Mantis Q40 (40 cells), and kept textweaver's own math speech as the default.

## Decision

### The patch

MathCAT 0.7.6-rc.3 is vendored under `third_party/mathcat`, byte for byte as its crates.io package (SHA-256 `e02ae55d13d1cab996bdb0ed0921011d1ba621868cf3d5d05afdd83d910b2bc8`, the checksum in `Cargo.lock`), without its tests, notes, and logo. The root `Cargo.toml` keeps `mathcat = "=0.7.6-rc.3"` and patches it in through `[patch.crates-io]`; the workspace excludes the folder, and `.gitattributes` keeps its CR LF line ends. `unsafe_code = "deny"` is unchanged.

The fix is the one the issue describes. `get_navigation_braille` wraps the part being explored in a new `<math>` element in a temporary document; it copied the part in the original document and appended the copy to the new one, which the no-unsafe backend cannot do, because its nodes are indices into their own document. A new `copy_mathml_to(mathml, doc)` allocates the whole copy in the destination document, and `get_navigation_braille` uses it. `copy_mathml` is unchanged for its other callers. `third_party/mathcat/textweaver.patch` holds the change as a diff, and `TEXTWEAVER.md` says how to check the copy and when to drop it.

The regression test is `crates/textweaver-mathcat/tests/navigation.rs`. Run against the crates.io package (with the patch line commented out), the step into a fraction answered "MathCAT crash"; with the vendored copy, every step in Nemeth and UEB passes. The pull request text for upstream is kept outside the repository, for the owner to file from their own account. When a MathCAT release fixes the issue, pin it and remove the folder and the patch line.

### The engine

`textweaver-mathcat` gains two calls, both on its one MathCAT thread (ADR-0029):

- `braille_mathml(mathml, options)`: Nemeth or UEB, as six-dot Unicode braille. `BrailleNavHighlight` is off, so no dots 7 and 8; UEB's start mode is grade 1 or 2 to match the text around the math, which decides UEB's grade 1 indicators.
- `navigate_start` and `navigate`: MathCAT's own navigation commands (`MoveNext`, `MovePrevious`, `ZoomIn`, `ZoomOut`, `MoveStart`, `MoveEnd`, `ReadCurrent`), each step answered with MathCAT's words, `get_navigation_braille` for the part reached, and whether the place changed.

MathCAT holds one expression at a time, shared by speech, braille, and navigation. The thread remembers which MathML is current and parses a new one only when it changes. Navigation keeps its expression and the moves made in it: if another formula was spoken in between, it parses the expression again and retraces the moves, because setting MathML resets MathCAT's place and its generated ids change. After 256 recorded moves, later moves are not recorded (a retrace then stops there); exploring a formula that long is not expected.

### The codes

`[braille] math_code = "nemeth" | "ueb"`, default `"nemeth"`, in a new `[braille]` section: the store type with a test, the export fixture, the schema ("Math braille"), and the app's export, which reads it. `tw convert` takes `--math-code`, since its other layout options are flags too, and prints "Math braille: Nemeth." after a BRF run.

**Nemeth** is written inside the UEB text as BANA's guidance for Nemeth in UEB contexts asks: the opening Nemeth indicator (dots 456, 146, braille ASCII `_%`) and a space before the math, a space and the Nemeth terminator (dots 456, 156, `_:`) after it, and punctuation after the terminator in UEB. A number alone stays in UEB, since the guidance switches only for math that is more than a single number. **UEB** mathematics needs no switch.

### BRF layout

The writer replaces each formula with a placeholder, translates the text around it (natively or through liblouis), and splices in the formula's braille ASCII. Lines stay 40 cells (the page width) and never separate an indicator from its symbol:

- a switch indicator is bound to the math beside it;
- a short spaced word (a comparison sign such as `.K`, or a function name) is bound to the word after it, so a line divides before a comparison sign, never after one;
- a long expression divides before an operation sign (plus or minus in Nemeth, with the baseline indicator kept before the sign; plus, minus, times, and division in UEB), never right after an indicator;
- failing all of those, it divides where the line ends, stepping back past indicators and signs, without the dot-5 line continuation indicator, which is a Nemeth symbol of its own (the baseline indicator).

Division rules for displayed math and Nemeth runover indents are not implemented; display math is its own paragraph, as before.

### The fallback

A formula the parser had to repair, one MathCAT refuses, and every formula in a build without the feature are written as their spoken words, translated with the text around them in the document's grade (grade 1 by default, today's behavior). The report says so once, naming the code: "1 formula is in spoken words: MathCAT could not write it in Nemeth braille." Without the feature: "... this build has no math braille."

### Exploring a formula

With `[reading] math_engine` set to a MathCAT style, Alt+Shift+X and the same keys drive MathCAT's navigation. Each step is spoken once through the caret channel's route (textweaver's voice, or the status line for the screen reader), and the status line shows the words, then the code and the part's braille (`mathx-step-braille`: "numerator ... Nemeth: ⠁⠬⠃"), for the display with `cursor = "status"`. A step that moves nothing plays the boundary sound, and MathCAT's words say why. MathCAT gives no source positions, so the highlight and the cursor stay on the whole formula. A formula MathCAT cannot read is explored with the built-in navigator. `"builtin"` is unchanged and stays the default.

Words first, then braille, is a first choice for session B1 to confirm: the status line is also what the screen reader speaks, and a screen reader may speak Unicode braille cells as symbols. Moving the braille first, or leaving it off in screen-reader mode, is a change to the one catalog message.

### Size

Measured on Windows (x86_64-pc-windows-msvc), release profile, `cargo build --release -p textweaver-tui --features mathcat`:

- before (main at 9076f34, MathCAT for speech only): 53,525,504 bytes;
- after: 53,688,320 bytes;
- added: 162,816 bytes (about 159 KiB). MathCAT's braille rules were already embedded (ADR-0029's 799,278-byte rules archive holds every braille code), so the added bytes are MathCAT's braille and navigation code that speech alone did not link, and textweaver's own.

### Dependencies

No new crate. The vendored MathCAT has the same dependencies as the crates.io package, so ADR-0029's dependency checks and the owner's exceptions (yaml-rust's advisory, libbz2-rs-sys's licence) are unchanged. `cargo deny --all-features check` (cargo-deny 0.20.2): advisories ok, bans ok, licenses ok, sources ok.

## Consequences

- Braille files carry real math braille, Nemeth by default, checked against published examples: three from the Nemeth Code (rules 62.a.3, 79.g.2, 80.a.1) and three from BANA's UEB mathematics guidance (section 5, examples 1 to 3), as recorded in MathCAT's own test suite, plus snapshots of `fixtures/c4/quadratic.md` in both codes.
- Exploring math with MathCAT shows the part's braille on the display, in the wording NVDA's MathCAT add-on uses.
- textweaver carries a patched MathCAT until upstream releases a fix. The patch is small and the pull request text is ready; re-check the issue before each merge of main.
- The highlight inside a formula is the whole formula with MathCAT, as it already is for MathCAT's speech.

## See also

- [ADR-0029: MathCAT speech](0029-mathcat-speech.md): the thread, the rules, and the dependency checks.
- [ADR-0018: Math](0018-math.md): textweaver's own math engine and navigator.
- [ADR-0017: Native writers](0017-writers.md): the BRF writer.
- [Reading and writing math](../math.md): "Math in braille files" and "Exploring with MathCAT".
- [The vendored copy](../../third_party/mathcat/TEXTWEAVER.md)
- [Documentation index](../README.md)
