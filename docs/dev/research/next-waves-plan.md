# Next waves plan: alpha.8, alpha.9, and beyond

This is the plan for the releases after 0.1.0-alpha.7, written on Friday, October 2, 2026, from a survey of the code, the benchmark history, the roadmap, and the research reports in this folder. It is for the maintainer planning the next waves of work. It keeps the project's order of priorities: the highlight never drifts, nothing waits on the input thread, large files stay fast, and every change works with a screen reader, a braille display, and the keyboard alone.

Each item names where it goes, the gain expected, how it is measured, a size (small: hours; medium: a day or two; large: a week or more), and a target release. Numbers quoted from the benchmark history are from [Testing](../testing.md); anything marked "estimate" has not been measured yet.

## Where the time goes today

The benchmark history gives a clear picture of the hot paths, measured on 10 MB of Markdown:

| Path | Now | Was | Note |
| --- | --- | --- | --- |
| Loading Markdown | 275 ms | 420 ms | one parse, words passed as slices |
| Whole-document narration plan | 610 ms | 1,155 ms | 844,757 allocations, unchanged |
| Open to first speech | 353 ms | 470 ms | the first window's plan |
| ICU4X sentence segmentation | 115 ms | | the floor for finding sentences |
| `tw info` | 580 to 700 ms | | almost all loading |
| `tw --version` | 23 to 35 ms | | 7 to 10 ms above an empty program |

Binary size: the reader is 47 MB, `tw` 53 MB, and the lean reader without `publish` 28.7 MB. Grammar checking adds 11 MB. The rope is not the bottleneck ([ADR-0034](../../adr/0034-rope-after-measurement.md)).

Three things follow. First, the next wins are in the narration plan's allocations and in loading, not in the rope or the segmenter. Second, nothing measures the two numbers a listener feels most: the time from a key press to the first audio after a stop, and the GUI's frame time while the highlight moves. Third, binary size is now a release concern, since students install on managed laptops with slow disks.

## Principles for the next waves

1. **Measure first.** Every performance item records `cargo xtask bench` before and after, and the numbers go in the testing guide's history, as the September passes did.
2. **Latency the ear notices comes before throughput.** A stop-and-restart (pause, rate change, a jump) should reach first audio in under 150 ms on a 2-core laptop with Piper; that number is not measured yet and gets a probe first.
3. **Off the input thread, always.** Any new work over a few milliseconds goes to a worker, as loading, the writer, and the voice lists already do ([Architecture](../architecture.md)).
4. **Smaller by default.** Features that cost binary size are features, not defaults, when the lean reader does not need them.
5. **Accessibility is the product.** A visual improvement that a screen reader cannot follow, or that shows state by color alone, does not ship.

## Alpha.8: measure, then cut the hot paths

The theme of alpha.8 is instrumentation plus the two or three optimizations the instrumentation is certain to justify, with the GUI catching up on the panels readers ask for most.

### Instrumentation

- **Stop-to-first-audio probe.** A bench measurement that starts a reading on the recording and Piper backends, stops it, restarts from the cursor, and reports the time until the first audio frame leaves the playback client. Where: `xtask/src/bench.rs`, with a hook in `crates/textweaver-speech/src/service.rs` that stamps the first frame. Gain: the number every later speech change is judged by. Measure: itself. Size: medium. Target: alpha.8.
- **GUI frame-time probe.** A `--background` run of the GUI that reads a 1 MB document for 30 seconds and reports the median and worst frame time while the highlight moves, through the existing screenshot and automation path in `crates/textweaver-xilem/src/screenshot.rs`. Gain: a gate for the visual work in alpha.9. Measure: itself. Size: medium. Target: alpha.8.
- **Nightly profile.** A nightly job that runs `samply` or `cargo flamegraph` on `tw info` and on the narration plan of the 10 MB corpus and uploads the profile as an artifact. Gain: no more guessing where the plan's time goes per sentence. Size: small. Target: alpha.8.

### The narration plan

- **Cut the plan's allocations.** The plan makes 844,757 allocations on 10 MB: three `enclosing` marker lookups and a `String` per literal piece, per sentence. Build each utterance's spoken text into one reused buffer and keep offset-map spans in a `Vec` with capacity carried over from the previous utterance; replace the per-piece `String` with ranges into that buffer. Where: the narration planner in `crates/textweaver-app` and `OffsetMap` in `crates/textweaver-core`. Gain: estimate, a third to a half of the plan's allocations, and a shorter open-to-first-speech. Measure: `cargo xtask bench --only plan` and the open-to-first-speech number. Size: medium. Target: alpha.8.
- **Plan the first window smaller.** The first window after a key press only needs the two sentences the queue looks ahead; the ten-minute window can be planned on a worker once speech has started. Where: `crates/textweaver-app/src/playback.rs` and the speech service's queue. Gain: open-to-first-speech falls toward the segmenter's cost for one paragraph (estimate, under 50 ms). Measure: open to first speech, and the new stop-to-first-audio probe. Size: medium. Target: alpha.8.

### Loading

- **Page-parallel PDF loading.** The pure-Rust PDF loader works page by page; pages are independent until reading order is joined. Run page extraction on the rayon pool that bulk conversion already uses, then join in order. Where: the PDF loader in `crates/textweaver-formats`. Gain: estimate, 2 to 3 times faster on a 4-core laptop for a 900-page textbook; no change on 2 cores. Measure: a new `--file` bench on a 300-page PDF fixture. Size: medium. Target: alpha.8.
- **Lazy startup data.** The SCOWL list (692 KB, unpacked in 27 ms) and the Fluent catalog are needed at the first spell check or message, not before the first announcement. Load them on first use, on a worker. Where: `crates/textweaver-aids`, `crates/textweaver-lexicon`. Gain: a few tens of milliseconds off every start (estimate). Measure: `cargo xtask startup`. Size: small. Target: alpha.8.

### Speech

- **Pause as silence, without an engine.** SSML-style pauses at headings, paragraph ends, and list items are on the roadmap. Implement them in the playback client as inserted silence between utterances, with the pause length by structure in `[speech]` settings, so every engine gets them and the offset map is untouched. Engines that own playback (Apple, speech-dispatcher) get a timed gap in the queue instead. Where: `crates/textweaver-enginehost` playback client and `crates/textweaver-speech/src/queue.rs`. Gain: comprehension at high rates, where listeners lose structure first. Measure: the stop-to-first-audio probe must not move. Size: medium. Target: alpha.8.
- **A medical pronunciation layer.** Health sciences students hear drug names, anatomy, and abbreviations mispronounced many times an hour. Add a bundled medical lexicon layer in the normalization pipeline before the community lexicon, with a per-user overlay, and a test file of 20 commonly mispronounced terms. Where: `crates/textweaver-speech/src/normalize`, `crates/textweaver-lexicon`. Gain: fewer stops to re-read. Cost: a few hundred kilobytes in the binary, zstd-packed as the dictionary is; one fst lookup per word. Size: medium. Target: alpha.8. The health sciences report has the data sources and their licenses.

### The GUI

- **Contents and Notes panels.** The two panels star had and the GUI lacks. Both come from list models the core already has (the outline, the notes list), shown in a sidebar that is a list widget in the accessibility tree, toggled by key, and never steals focus from the document. Where: `crates/textweaver-xilem/src/gui.rs`, with the list state in `crates/textweaver-app`. Cost: one more list in the tree; no per-frame work when closed. Size: large. Target: alpha.8.

### Release

- **A `dist` size budget.** Record the size of each package in the release notes, and fail `cargo xtask dist` when a package grows more than 10 percent over the previous release without a note saying why. Where: `xtask`. Size: small. Target: alpha.8.
- **Conversion report with provenance.** `tw convert` and batch conversion write a report beside the output: the source file and its hash, textweaver's version, the date, what was converted, and what could not be made accessible (images without descriptions, tables OCR could not line up, math that did not parse). Accommodations offices need this for their files. Where: `crates/textweaver-convert`, `crates/textweaver-app/src/batch.rs`. Cost: one small file per run. Size: medium. Target: alpha.8.

## Alpha.9: a better looking reader, at the same frame time

Alpha.9 spends the frame-time budget that alpha.8 measured.

- **A token-based theme model.** The theme crate's palettes become named tokens (background, surface, text, muted, accent, highlight word, highlight sentence, caret, ruler, focus ring), each with light, dark, and high-contrast values, with the existing contrast rules checking every pair. Every theme file, the GUI, the terminal, and the HTML export read the same tokens. Where: `crates/textweaver-theme`, `crates/textweaver-xilem`, `crates/textweaver-render`. Cost: none at run time. Size: medium. Target: alpha.9.
- **Two-tone highlight and a calm canvas.** The spoken sentence in a soft tone, the spoken word in a stronger one, a reading measure of 60 to 75 characters with margins that grow with the window, a quiet toolbar, and a progress line with time remaining. Each is a setting, each is announced. Where: `crates/textweaver-xilem/src/document.rs`. Cost: measured with the frame-time probe; the sentence highlight is one more rectangle per visible line. Size: medium. Target: alpha.9.
- **Per-paragraph layout cache in the document widget.** If the frame-time probe shows the widget laying out the whole window on each highlight move, cache Parley layouts per paragraph and invalidate only the paragraph that changed. Where: `crates/textweaver-xilem/src/document.rs`, `runs.rs`. Gain: estimate, frame time flat as the window grows. Measure: the frame-time probe. Size: large. Target: alpha.9, or alpha.8 if the probe shows it is needed sooner.
- **Reading settings sheet.** Font, size, spacing, theme, voice, and rate in one dialog that applies live, with each control naming its key. Where: `crates/textweaver-xilem/src/settings_dialog.rs`. Size: medium. Target: alpha.9.
- **Terminal polish.** A consistent two-tone highlight, a quiet status bar, true-color detection with a 16-color fallback, and a progress gauge, with nothing added to what a screen reader reads. Where: `crates/textweaver-tui/src/ui.rs`. Cost: no extra redraws. Size: small. Target: alpha.9.
- **Reduced motion and high contrast follow the system.** The GUI reads the system's reduced-motion and high-contrast settings and the theme follows them, as it follows dark mode today. Size: small. Target: alpha.9.
- **First-run tour.** The quick start read aloud as a guided tour, one key per step, skippable. Size: medium. Target: alpha.9.

## Later

- **A second neural engine spike.** Measure one newer open voice model with word timing on RTen against Piper for real-time factor, first-audio latency, memory, and voice quality on a 2-core laptop. The speech engines report names the candidates and their licenses. Size: large.
- **Incremental re-segmentation in edit mode.** Re-segment only the paragraph that changed. Worth it only when a trace shows typing in a large file waiting on segmentation. Size: large.
- **Allocator and link-time options.** Measure mimalloc against the system allocator on Windows, `panic = "abort"` in `dist`, and `opt-level = "s"` for cold crates such as the writers. Keep only what the bench and the size budget justify. Size: small each.
- **An Accessibility Conformance Report for textweaver.** Fill the VPAT 2.5 template for WCAG 2.2 and Section 508 from the listening sessions and the automated tree checks, and publish it with the accessibility statement. The law and standards report has the outline. Size: medium.
- **Document translation, karaoke video export, quick-open for PubMed**, as the roadmap lists them, each after the speech path's latency number is where it should be.

## What not to do

- Do not move off ropey 1.6 until ropey 2 is final and measured again ([ADR-0034](../../adr/0034-rope-after-measurement.md)).
- Do not add a cloud speech engine or an async runtime ([ADR-0001](../../adr/0001-workspace-and-dependencies.md)).
- Do not add a visual feature without a frame-time number and a screen-reader check.

## See also

- [Research index](README.md): the reports this plan draws on.
- [Roadmap](../../roadmap.md): the public summary.
- [Testing](../testing.md): the benchmark harness and its history.
- [Architecture](../architecture.md)
- [ADR index](../../adr/README.md)
