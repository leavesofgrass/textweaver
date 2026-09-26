# Research for Wave 4

Researched on Saturday, September 26, 2026, by four research threads, for the Wave 4 agents (`docs/tasks.md`, "Wave 4"). It records the versions, licences, and dates that were live on that day. Each agent reads its own section before starting.

**Privacy note.** Two threads sent the local part of Jon's email address as their User-Agent in requests to crates.io's public API. That was about 125 requests in all. It was noticed, stopped, and reported to Jon. All research requests now use a neutral User-Agent (`textweaver-research (+https://github.com/leavesofgrass/textweaver)`), and no request may carry personal data.

## W4a: the Xilem GUI, part two

### AccessKit and Parley

- **AccessKit released new versions on 2026-09-25:** accesskit 0.25.1, accesskit_winit 0.34.1 (Apache-2.0), accesskit_windows 0.35.1 (MSRV 1.87), accesskit_macos 0.27.1, and accesskit_unix 0.24.0.
- **Xilem main still pins the older versions:** accesskit 0.24, accesskit_winit 0.32.2, and parley 0.8. There is no open pull request to bump them.
- **Parley main dropped its AccessKit feature.** PR #716 (merged 2026-09-20, not yet released) removes it. The reference implementation now lives in Parley's `examples/editor` (vello_editor), which uses accesskit 0.25 and accesskit_winit 0.34. Model `DocumentView` on it.

### How AccessKit exposes text

- **Text runs.**
  - Each run holds at most 256 characters, because `word_starts` is a u8.
  - `character_lengths` counts UTF-8 bytes.
  - A hard line break is the last character of its run.
- **Selection and caret.** The selection is set on the input node as anchor and focus text positions. The caret is a selection where anchor equals focus.
- **Roles.**
  - `Role::Document` is always read-only in `accesskit_consumer`, so use it for the reading view.
  - Use `MultilineTextInput` for edit mode.
- **Actions to handle.**
  - `SetTextSelection` is essential: every desktop platform sends it.
  - `SetValue` comes from Windows ValuePattern (single-line only), AT-SPI `SetTextContents`, and macOS.
  - `ReplaceSelectedText` is sent only by iOS, but it is cheap to handle.
- **AT-SPI EditableText.** Only `SetTextContents` works. Insert, delete, cut, copy, and paste return NotSupported.
- **Known bug.** PR #704: NVDA can miss the deletion of the last character when the number of runs changes.

### Parley 0.11.1 (2026-08-16)

- It has letter spacing, word spacing, line height, weight, underline, and strikethrough.
- **It has no background style.** Draw the highlight behind the spoken word ourselves.
- Ranged styles come through `RangedBuilder::push`, for bionic reading and difficult words.
- `PlainEditor` applies one style to the whole text and has no undo. textweaver's editor crate provides undo.
- IME is supported through `set_compose` and `finish_compose`.

### Masonry TextArea

- It handles only `SetTextSelection`.
- Undo is missing (issue #1417).
- Clipboard uses `set_clipboard` and `TextEvent::ClipboardPaste`.

### RSVP overlays

- Mark the node for the flashing word as hidden (`Node::set_hidden()`), and keep focus elsewhere. textweaver speaks for itself.
- Also expose a separate, rarely changing, labelled status node, such as "RSVP: paused, word 120 of 900", with `Live::Off`.
- A hidden node that has focus is still exposed.

### Announcements on Windows

- AccessKit raises only UIA LiveRegionChanged, and only when `Live` is not Off and the node has a name. It never raises `UiaRaiseNotificationEvent`.
- NVDA honours UIA notifications (since 2018.1). JAWS handling of LiveRegionChanged has been inconsistent.
- **Option:** a direct `UiaRaiseNotificationEvent` call for announcements, which needs Windows 10 1709 or later. Jon compares the two by ear.

### Reference apps

- Slint 1.18.1 has the most complete editable-text AccessKit support (`internal/backends/winit/accesskit.rs`).
- egui 0.36.2 TextEdit is also a useful reference.
- Zed/GPUI merged AccessKit in May 2026, but its code editor does not appear to be exposed yet.
- No Rust app is documented as having good screen-reader editing of multi-line text. textweaver would be early.

## W4b: speed

### Text segmentation

- **icu_segmenter 2.3.0** (2026-08-13, Unicode-3.0, MSRV 1.88).
  - Words: `WordSegmenter::new_auto`, `new_lstm`, `new_dictionary`, or `new_for_non_complex_scripts`. The last leaves out the LSTM and dictionary data.
  - Sentences: `SentenceSegmenter`, plain UAX #29. It has no abbreviation handling, so "Mr." handling stays our own code.
  - Data: the data crate is 3.4 MB, and CJK dictionaries added about 3.8 MB to one binary.
  - Speed: graphemes are faster than unicode-segmentation (1.82 µs against 3.40 µs on English). No word or sentence benchmark has been published, so measure it ourselves.

### Ropes

- **ropey.** Stable is 1.6.1. 2.0.0-beta.1 (2025-08-02) moves to byte indexing, puts character indexing behind the `metric_chars` feature, and adds a `LineType` to every line call. Clone stays O(1). The maintainer says 2.0 is "not battle-tested".
- **crop** 0.4.3 (MIT). Byte-indexed, with an O(1) Arc clone that can go to another thread. It knows only LF and CRLF line breaks. It is 3 to 4 times faster than ropey on edit traces.
- **jumprope.** It indexes by character, and its clone is a deep copy. It does not suit the writer thread.

### Profiling and size tools that work on Windows

- **Allocations:**
  - dhat 0.3.3 gives heap tests with `dhat::assert!`, but backtraces are slow on Windows.
  - divan `AllocProfiler`, stats_alloc, cap, and peak_alloc are lightweight counters.
  - Tracy (tracy-client 0.19) needs C++.
  - WPR/WPA heap snapshots come with Windows.
- **Startup:** hyperfine 1.20.0 with `-N --warmup`, plus a "first frame then exit" flag. Also `tracing-etw` 0.2.3.
- **Binary size:** cargo-bloat 0.12.1 supports PE files. cargo-llvm-lines 0.4.48 works on any platform. twiggy is Wasm-only and archived, and bloaty has no PE support.

### TLS and archives

- **TLS:** rustls-graviola 0.4.0 needs x86_64 with AVX2, ADX, BMI2, and AES-NI, or aarch64 with AES and SHA2; its CI covers Windows. It fits ureq 3.4.2 through `rustls-no-provider` and `unversioned_rustls_crypto_provider`, which ureq marks unstable.
- **zip 8.6.0.** Use `default-features = false` to avoid C zstd.
  - Pure Rust: `deflate-flate2-zlib-rs`, `lzma`, `xz`, `ppmd`, and `bzip2`, which uses libbz2-rs-sys.
  - C, despite the name: `bzip2-rs`, which uses bzip2-sys.
- **Search:** memchr 2.8.3 and aho-corasick 1.1.5.

## W4c: formats

### Math: MathCAT

- **MathCAT** (DAISY, Neil Soiffer, MIT) is the math engine NVDA and JAWS use.
  - Latest stable is 0.7.5. 0.7.6-rc.3 came on 2026-08-23 and 0.7.7-alpha.1 on 2026-09-23.
  - It gives ClearSpeak and SimpleSpeak math speech in about 15 languages.
  - It gives Nemeth and UEB math braille, plus other codes.
  - It supports navigating within a formula.
- **API:** `set_rules_dir`, `set_preference`, `set_mathml`, then `get_spoken_text`, `get_braille`, and the navigate calls.
- **Constraints:**
  - Its state is thread-local, so run it on one dedicated thread.
  - The `include-zip` feature embeds the rules. The full rules tree is 9.6 MB.
  - 0.7.5 pulls in zip 6 (a duplicate) and yaml-rust (unmaintained). 0.7.6 moves to zip 8.2.
- **Plan:** adopt MathCAT behind a feature, pinned to 0.7.6 once it is stable. Keep textweaver's own math speech as the fallback.
- **Speech order for EPUB 3 MathML:** MathCAT first, then `alttext`, then the alt text of `altimg`.

### DOCX comments and tracked changes

- Extend textweaver's own roxmltree reader. The existing crates would each add a quick-xml stack.
- **Comments:** `w:commentRangeStart` and `w:commentRangeEnd`, plus `w:commentReference`, point by `w:id` to word/comments.xml.
- **Replies and resolved state:** `w15:commentEx` in commentsExtended.xml, with `paraIdParent` for replies and `done="1"` for resolved comments.
- **Revisions:**
  - `w:ins` and `w:del` hold insertions and deletions, with `w:delText` inside deletions. Each has an author and a date.
  - Moves are `w:moveFrom` and `w:moveTo`, with their ranges.
  - More than 40 kinds of property changes can be ignored.

### RTF

- Write our own iterative parser with an explicit group stack. Cap the depth, `\bin` lengths, and `\uc` skip counts.
- Decode with `\ansicpg` and `\fcharset` through encoding_rs.
- Handle `\u` and `\uc`, `\*` destinations, tables (`\trowd`, `\cell`, `\row`), `\footnote`, and fields.
- rtf-parser 0.4.3 is too shallow: it has no code pages, no tables, and a recursive lexer.

### ODT, LaTeX, email

- **ODT.** Write our own on roxmltree and zip. The schema is ODF 1.4, an OASIS Standard since 2025.
- **LaTeX subset.** Keep our math parser, and optionally compare it with math-core 0.8.2 in tests. Write our own tokenizer for sections, lists, tabular, cite, ref, label, footnote, and math. mitex's rowan grammar is a design reference. Avoid tectonic, which needs C libraries, and unicodeit, which is LPPL.
- **EML and MHTML.** Use mail-parser 0.11.9 with its encoding_rs feature. It is Apache-2.0 or MIT, safe Rust, and fuzzed. It supports RFC 2557 (MHTML): resolve `Content-Location` and `Content-ID` references, then read the root HTML.

### Hostile input

- **ZIP:** cap the number of entries, the total uncompressed size, and the compression ratio. Read through `take()`, and reject overlapping entries.
- **MIME:** cap the number of parts, the nesting depth, and the decoded size.
- **XML:** cap the number of nodes and the nesting depth. roxmltree does not expand entities.
- **Fuzzing:** one cargo-fuzz target per parser.

## W4d: interface translations

### Catalogs and plurals

- **Fluent.** Use fluent-bundle 0.16 with fluent-templates 0.15.1 `static_loader!`, or i18n-embed with `fl!`, which checks message ids at compile time.
  - It is pure Rust, with CLDR plural rules.
  - It wraps placeables in bidi isolation (FSI and PDI) by default.
  - Its `fluent-pseudo` crate provides pseudo-localization.
- **Not chosen:**
  - rust-i18n has no plural support.
  - gettext-rs binds C.
  - ICU4X has no MessageFormat 2 crate.
  - The `mf2*` crates published on 2026-09-26 have no repository, so treat them as suspect.
- **Plurals.** Arabic has six categories (zero, one, two, few, many, other). French has one, many, and other.

### Right-to-left text

- **In terminals:**
  - VTE (GNOME Terminal and others) implements BiDi.
  - Konsole, mlterm, iTerm2 3.7, and macOS Terminal reorder text in their own ways.
  - Windows Terminal has no RTL support (issue #538).
  - ratatui has no BiDi support.
- **Rule:** keep text in logical order in the document model, in AccessKit, and in speech. Reorder it only for display, with the `unicode-bidi` crate, as a user setting that is off where the terminal already reorders.
- **In the GUI:**
  - Parley has its own UAX #9 implementation, with an automatic, left-to-right, or right-to-left base direction.
  - Masonry does not mirror layouts for RTL.
  - AccessKit has `set_text_direction`.

### Testing translations

- **Pseudo-locale:** accented letters, about 30% longer strings, and brackets around each message. A snapshot test fails when plain English letters appear outside brackets.
- **Missing and unused messages:** a `fluent-syntax` checker compares each language's ids and variables with en-US.

## W4e: offline intelligence on rten

### The runtime

- rten, rten-generate, and rten-text 0.26.0 (2026-08-29, MSRV 1.94).
- rten-generate runs encoder-decoder models with a KV cache, in the Optimum "merged decoder" layout.
- The Whisper example (`encoder_model.onnx` plus `decoder_model_merged.onnx`) is the template for translation.

### Translation models

- **OPUS-MT** (Helsinki-NLP): the upstream models are CC-BY 4.0. The Hugging Face licence tags disagree, so show CC-BY 4.0 with attribution.
- **Files:** the Xenova ONNX ports include `tokenizer.json`. A quantized pair is about 110 MB: en-es 52.9 + 60.2 MB, and en-fr, en-de, and en-ar are similar.
  - The config has `decoder_start_token_id` and `pad_token_id` both set to 65000, and eos 0.
  - Mask the pad token while decoding.
  - Reported garbled output from other ONNX exports points to masking errors.
- **Avoid:** NLLB, which is non-commercial, and M2M100, which is MIT but about 630 MB quantized.

### The tokenizer gap

- rten-text has only WordPiece and BPE, with no Unigram or SentencePiece, so it can't tokenize Marian models.
- **Pure-Rust choice:** kitoken 0.11.0 (BSD-2-Clause), which loads `tokenizer.json` and `.spm` files. Test that its token IDs match exactly.
- **Fallback:** `tokenizers` 0.23.2 with `default-features = false, features = ["fancy-regex"]`, which leaves out its C and C++ parts.

### Embeddings

- all-MiniLM-L6-v2 (Apache-2.0) is 23 MB quantized and uses a WordPiece tokenizer, so rten-text handles it.
- Snowflake arctic-embed-xs and jina-embeddings-v2-small-en work the same way.
- potion-base-8M (MIT, about 30 MB) is a static model: safetensors, a WordPiece lookup, then a mean.
- The multilingual models (e5, paraphrase-multilingual) need a Unigram tokenizer.

### Summaries

- No solid LexRank crate exists, so write it in-house in about 100 lines:
  1. TF-IDF or embedding vectors for each sentence.
  2. A cosine similarity matrix.
  3. Row-normalize and dampen by 0.85.
  4. Power iteration.
  5. The top k sentences, in document order.

## W4f: platforms and releases

### Runners and AppImage

- **arm64 runners:** GitHub's Linux and Windows arm64 runners are free for public repositories (`ubuntu-24.04-arm`, `ubuntu-22.04-arm`, `windows-11-arm`).
- **AppImage tools for aarch64:** appimagetool, type2-runtime (static, no libfuse2 needed), and linuxdeploy all build for aarch64. In containers, use `APPIMAGE_EXTRACT_AND_RUN=1`.

### Automated screen-reader tests

- **Guidepup** 0.34.0 (MIT) drives VoiceOver and NVDA in CI and returns the spoken phrases. They complement Jon's manual testing.
  - `guidepup/setup-action` was archived on 2026-09-26. Use `npx @guidepup/setup setup --ci` instead.
  - Whether it works with native apps and with Orca is unverified.
- **Other tools:**
  - accessibility-cli (DioxusLabs, early) dumps accessibility trees on every OS.
  - NVDA's own system tests use a speech-spy plugin.

### Merge gate and branches

- **Merge gate:**
  - Rulesets are free on public repositories owned by a personal account: a pull request with 0 approvals, required status checks, and blocked force pushes.
  - Turn on auto-merge, and merge with `gh pr merge --auto --squash --delete-branch`.
  - Merge queue is available only to organizations, not to personal accounts.
- **Pruning branches:** `git branch --merged` misses squash merges. git-delete-merged-branches 7.6.1 (GPLv3+, `--effort=3`, dry run, asks before deleting) handles them. Back up refs before deleting anything.

### Changelog

- Keep the hand-written changelog.
- cargo-release 1.1.6 fills in "Unreleased" and the date.
- changie or knope suit plain-sentence changelog fragments.
- git-cliff can group commits without conventional-commit messages.

## See also

- [Pure-Rust choices for Wave 3](pure-rust-wave3.md)
- [Xilem GUI research](xilem-gui.md)
- [Roadmap](../roadmap.md)
- [Documentation index](../README.md)
