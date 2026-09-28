# ADR-0029: MathCAT speech

- Status: accepted, behind the `mathcat` feature; two dependency checks wait for the owner (see "Dependency checks")
- Date: 2026-09-27

## Context

textweaver speaks math with its own engine (`textweaver-math`, ADR-0018): ClearSpeak-style English at three verbosity levels, with a word-by-word offset map. It is English only and is not a full ClearSpeak implementation.

MathCAT (DAISY, Neil Soiffer, MIT) is the math engine NVDA and JAWS use. It speaks MathML in ClearSpeak and SimpleSpeak, in about 15 languages, and also writes Nemeth and UEB math braille and navigates inside a formula. Students who use NVDA or JAWS already know its wording.

MathCAT's stable release is 0.7.5, which pulls in a second zip version. 0.7.6 has three release candidates and no final release. Issue #827 reports a panic in `GetNavigationBraille` in builds with MathCAT's `no-unsafe` feature, and issue #817 a test failure on Windows.

## Decision

A new crate, `textweaver-mathcat`, speaks math through MathCAT, behind the speech crate's `mathcat` feature. textweaver's own engine stays: it is the default, the fallback, and the only engine for everything but speech (parsing, MathML for the renderer, navigation, the BRF writer).

### Version

`mathcat = "=0.7.6-rc.3"`, pinned exactly, with `default-features = false` and the features `include-zip` and `no-unsafe`. 0.7.6-rc.3 (August 23, 2026) is the last 0.7.6 build. Move to 0.7.6 when it is final, and read the recorded wording in `crates/textweaver-mathcat/tests/vectors.rs` again when upgrading.

### One thread

MathCAT keeps its rules, preferences, and current expression in thread-local state. So every call runs on one dedicated thread, `textweaver-mathcat`, started on first use:

- Callers send a request with its own reply channel and wait up to 10 seconds. No async runtime (ADR-0003).
- After a timeout, later requests fail at once until the thread answers again, so a stuck expression costs one wait, not one per formula.
- Preferences (`SpeechStyle`, `Verbosity`, `Language`) are set only when they change. `CheckRuleFiles` is set to `None`: the rules are embedded and never change.
- The speech pipeline starts the thread and loads the rules in the background when MathCAT is chosen (`warm_up`), so the first formula does not wait. Loading takes about 0.3 seconds in a debug build.

**Panics.** Every request runs inside `catch_unwind`. MathCAT also catches its own panics and returns them as errors starting "MathCAT crash". Either way the caller gets `Error::Crashed`, the log says so once, and the thread goes on to the next request. A test makes the engine panic twice and then speaks a formula.

**MathCAT's panic hook.** MathCAT installs a process-wide panic hook that records the message and prints nothing, and does not call the hook it replaced. That would silence every other thread's panics and drop the terminal reader's hook, which restores the terminal. The thread wraps it: panics on the MathCAT thread go to MathCAT's hook, and panics anywhere else go to the hook that was there before. The terminal reader installs its hook at startup, before MathCAT starts, so it keeps working.

### Rules: embedded

MathCAT's rules (9.4 MB of YAML: speech rules for every language, braille codes, and definitions) are embedded with MathCAT's `include-zip` feature. Its build script zips them, with bzip2, from the `Rules` folder inside the crate package that cargo fetched; nothing is downloaded, and no rule file is read from disk at run time. The other option, loading the rules from the data folder after the user says yes, was not taken:

- Embedding works offline, from the first run, with no download to approve and no files to keep in step with the program.
- MathCAT reads the embedded rules from memory, so there is nothing on disk to go missing or be tampered with.
- The cost is the size below.

MathCAT still asks the file system for the modification time of a user preferences file, `MathCAT/prefs.yaml` in the user's configuration folder (on Windows, under the roaming AppData folder), once when it starts. It checks for the file in the embedded rules only, so it never opens or reads it, and it writes nothing. This is recorded because textweaver's rules for agents treat the user profile as off limits.

### Speech

`textweaver_mathcat::speak_text` is the body of the math transform when MathCAT is the engine. It finds math exactly as `textweaver_math::speak_text` does, parses each formula into textweaver's tree, writes it as MathML (no `alttext`, no annotation), and asks MathCAT for its words:

- **Style:** ClearSpeak or SimpleSpeak, from the setting.
- **Verbosity:** textweaver's `math_verbosity` mapped to MathCAT's three levels: low is `Terse`, normal is `Medium`, high is `Verbose`.
- **Language:** the open document's language (`DocumentMeta::language`, from `dc:language`, `<html lang>`, and so on). Its primary subtag is used when MathCAT has rules for it (de, el, en, es, fi, fr, hu, id, nb, pl, ru, sv, vi, zh; `no` is read as `nb`), else English. The app passes the language to the speech pipeline when a document opens.
- **Pauses:** MathCAT puts commas and semicolons in for pauses. At punctuation level `All`, where every mark is read by name, they are left out (a comma inside a number, "3,5", stays).
- **Cleaning:** control characters are removed from MathCAT's words; it passes an escape sequence in the source through to its output.
- **Fallback:** a formula the parser had to repair (it has diagnostics), or one MathCAT refuses, times out on, or crashes on, is spoken by `textweaver-math`, word by word, as before.

**Offset map: span level.** MathCAT speaks a whole expression and gives no positions for its words. So each formula it speaks becomes one `Expanded` span over the formula's content, and the delimiters are `Elided` spans. Any word of it highlights the whole formula, and pause and resume inside it resume at the formula's start. ADR-0005 allows this: an expanded span stands for a whole source token, and "any position inside highlights the whole token". The built-in engine keeps its word-level map. MathCAT can return `id`-tagged speech for synchronized highlighting (its `Bookmark` preference); mapping those ids back to source positions is left for later.

**Limits.** MathML over 256 KiB, nested more than 128 levels, or with more than 1,000 elements is refused before it reaches MathCAT, and read by the built-in engine. MathCAT's time grows with the square of a row's length: in a release build, 400 terms took 0.21 seconds and 800 took 0.74. An ordinary formula has well under a hundred elements and takes a few milliseconds.

### The setting

`[reading] math_engine`: `"builtin"` (the default), `"mathcat"` (ClearSpeak), or `"mathcat_simplespeak"`. It touches the four places: the store type and default with a test, the settings export fixture, the settings schema ("Math speech"), and the engines' service configuration, which reads it. Changing it in the settings screen rebuilds the speech pipeline at once.

Without the `mathcat` feature, the setting changes nothing; a test holds that the pipeline's output is identical. The feature passes through `textweaver-engines`, `textweaver-app`, the reader (`textweaver-tui`), and `tw` (`textweaver-cli`). It is off by default.

### EPUB 3 MathML

The EPUB loader reads each `<math>` element as math: LaTeX with its delimiters under a `Math` marker, as the Markdown and DOCX loaders write math (level 1 for display math, on its own line). The LaTeX is the book's own TeX when a `<semantics>` annotation carries it, else the presentation MathML converted (`crates/textweaver-formats/src/mathml.rs`). So the speech order for EPUB math is:

1. MathCAT, when it is the engine (through the LaTeX, re-parsed and written back as MathML);
2. textweaver's own math speech, otherwise or as MathCAT's fallback;
3. the `alttext`, as plain text, when the element holds no math;
4. an image fallback's alt text: the `epub:default` of an `epub:switch` whose case has no MathML, or an `<img>` in the math, which the HTML parser moves out after it and which is then read as every image is.

MathCAT sees textweaver's MathML for the formula, not the book's original MathML. The speech pipeline works on text, and the `Math` marker is the one place a formula lives in a document. Passing the book's MathML through to MathCAT would need the marker to carry it (its `reference`) and the narration plan to hand it to the speech service; that is a contract change for later. The HTML loader is unchanged.

### What waits for issue #827

Braille stays with textweaver's own path: the BRF writer writes a formula as its spoken words in grade 1 braille. Waiting for MathCAT issue #827 ("GetNavigationBraille panics in no-unsafe builds") to be fixed in a tagged release:

- Nemeth and UEB math braille in the BRF writer (`get_braille`);
- math braille on the Braille display while exploring a formula (`get_navigation_braille`, `get_braille_position`);
- exploring a formula with MathCAT's navigation (`do_navigate_command`, `do_navigate_keypress`, `get_navigation_mathml`): the reader's exploration mode (`math_explore.rs`) stays on textweaver's `Navigator`.

The workspace builds MathCAT with `no-unsafe`, the configuration the issue is about. Speech does not call the braille or navigation functions.

### Size

Measured on Windows (x86_64-pc-windows-msvc), release profile, `cargo build --release -p textweaver-tui`:

- without the feature: 46,809,088 bytes;
- with `--features mathcat`: 49,980,416 bytes;
- added: 3,171,328 bytes (about 3.0 MiB). The embedded rules archive is 799,278 bytes of that; the rest is MathCAT's code and its XML, XPath, YAML, and bzip2 libraries.

### Dependencies

MathCAT's own dependencies (normal), each with its licence:

- anyhow, bitflags, cfg-if, clap, log, regex, fastrand, strum, phf, unicode-script: already in the tree.
- dirs 6.0.0 (MIT or Apache-2.0), with dirs-sys already in the tree.
- env_logger 0.11.11 (MIT or Apache-2.0), with env_filter 2.0.0 and jiff 0.2.37 (Unlicense or MIT). MathCAT's library never starts a logger; they come with its command-line tool.
- html-escape 0.2.15 (MIT).
- radix_fmt 1.0.0 (Apache-2.0).
- roman-numerals-rs 4.1.0 (0BSD or CC0-1.0), a second version beside 3.1.0 from biblatex.
- sxd-document-no-unsafe 0.4.2 (MIT), with peresil 0.3.0 (MIT) and typed-arena.
- sxd-xpath-no-unsafe 0.5.1 (MIT or Apache-2.0), with snafu 0.5.0 (MIT or Apache-2.0), backtrace 0.3.76, addr2line, gimli, object, rustc-demangle, and doc-comment. snafu-derive 0.5.0 builds with syn 0.15, proc-macro2 0.4, quote 0.6, and unicode-xid 0.1, old versions beside the ones already in the tree (build time only).
- yaml-rust 0.4.5 (MIT or Apache-2.0), with linked-hash-map 0.5.6.
- zip 8.6.0, the workspace's own version (no second zip). MathCAT turns on its `bzip2` feature, which brings bzip2 0.6.1 (MIT or Apache-2.0) and libbz2-rs-sys 0.2.5, pure Rust.

### Dependency checks

Two checks cannot pass with MathCAT 0.7.6-rc.3 as it is, and both need the owner's decision, not an agent's:

- **Licence:** libbz2-rs-sys 0.2.5 is under the `bzip2-1.0.6` licence, which is not in the accepted lists of `deny.toml` and `about.toml`. `cargo about generate --offline --locked --all-features --fail` fails on it and on nothing else. The licence is permissive (a BSD-style licence), and it would have to be added to both lists. MathCAT's `zip` dependency asks for `bzip2` itself, so it cannot be turned off from here.
- **Unmaintained crate:** yaml-rust 0.4.5 is unmaintained (RustSec advisory RUSTSEC-2024-0320), and `deny.toml` reports unmaintained crates for the whole tree. Wave 4's plan expected 0.7.6 to drop it; 0.7.6-rc.3 still depends on it, for reading its rules. It would need an entry in `deny.toml`'s `ignore` list with a reason, or a MathCAT release that moves to a maintained YAML parser.

`cargo deny` itself is not installed on the development machine, and installing it is a download that was not approved, so it was not run.

## Consequences

- Students can hear math in the wording their screen reader uses, in the document's language, with textweaver's own speech as a safe fallback.
- The highlight inside a MathCAT formula is the whole formula, not the word. The built-in engine stays word-exact.
- The feature is off by default. Until the owner decides on the two dependency checks, builds with it are for testing.
- MathCAT's panic hook is contained; any future call into MathCAT must go through the thread.
- Upgrading MathCAT means reading the recorded wording again, and checking whether #827 and the yaml-rust dependency are resolved.

## See also

- [ADR-0018: Math](0018-math.md): textweaver's own math engine.
- [ADR-0005: Narration and the OffsetMap](0005-narration-and-offset-map.md): span kinds, and why a span-level map is allowed.
- [ADR-0003: Speech threading and event timing](0003-speech-threading-and-event-timing.md): threads and no async runtime.
- [Reading and writing math](../math.md): the user guide, including "Hear math with MathCAT".
- [Documentation index](../README.md)
