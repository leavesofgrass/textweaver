# Performance audit and optimization plan

This document is a code-level audit of where textweaver spends time and memory, with a ranked plan for making it faster, smaller, and leaner. It is for the contributors who will do the work in the alpha.8 and alpha.9 waves, and for the owner deciding what to schedule. Every claim about textweaver's own code names the file and line it comes from, every number says where it was measured, and every estimate is marked as one. Written on Friday, October 2, 2026, against branch `claude/affectionate-meitner-jduzl8` at 0.1.0-alpha.7.

## What the numbers say today

The recorded numbers are in [Testing](../testing.md), "Benchmarks": on Windows, a 10 MB Markdown file loads in 275 ms, the whole-document narration plan takes 610 ms with 844,757 allocations, open to first speech is 353 ms, and ICU4X's sentence segmentation (115 ms per 10 MB) was called the floor for finding sentences. `tw --version` starts in 23 to 35 ms. The reader is 47 MB, `tw` 53 MB, and the lean reader without `publish` 28.7 MB.

For this audit I built `textweaver-core`, `textweaver-text`, and `textweaver-formats` in release mode (2 minutes 57 seconds on a 4-core Linux machine with Rust 1.96.1), then ran a standalone probe outside the workspace with a counting allocator on the same generated corpora `cargo xtask bench` uses; the generator was copied from `xtask/src/bench.rs`. `cargo xtask bench --quick --no-startup --only md-1mb` did not run: after 4 minutes 31 seconds of building, `alsa-sys` (rodio, through the engines) failed for want of the ALSA development library, so the numbers below are the probe's. Its allocation counts agree with the harness within 2 percent (830,821 against 844,757 for the plan), so they are comparable; its times are faster than the Windows numbers, as a quiet Linux machine usually is. `cargo bloat` is not installed and was not run.

The probe on the 10 MB corpus (9,140,703 chars, 72,898 lines, 159,128 markers), best of three:

- Load: 208 ms, 67,862 allocations, 116 MB allocated in all, peak 45 MB above baseline. Retained by the loaded `Document`: 30.2 MB, of which the marker vector is 11.5 MB (159,128 markers at 72 bytes).
- Sentences alone, through `Units`: 170 ms, 214,689 allocations, 53.5 MB allocated. ICU4X's share is 115 ms, so about 55 ms and all of those allocations are textweaver's own wrapping.
- Words alone: 343 ms, 368,707 allocations, 177 MB allocated, for 1,432,384 words.
- Whole-document plan: 301 ms, 830,821 allocations (7.5 per utterance), 113 MB allocated, 111,480 utterances with 185,083 spans. One 32,768-char reading window, what Space plans: 0.97 ms for 397 utterances.
- Marker tables, built on first use: 2.7 ms. Blank-line table: 17.7 ms and 9,355 allocations. One insert through `Document::apply` with markers kept: 1.64 ms; rebuilding the blank lines and tables after it: 18 ms.
- `find_all` of "the" (28,481 hits): 6.9 ms, of which copying the rope to a `String` is 1.8 ms.
- Normalizing an utterance through the default pipeline (`math`, `abbreviations`, `numbers`, `punctuation`): 8 microseconds and 69 allocations. Building the pipeline: 3.7 ms.

On the 1 MB corpus everything is about a tenth of that (load 16 ms, plan 27 ms, sentences 16 ms): the costs scale linearly, and nothing quadratic is left in these paths.

Three conclusions follow. The per-keystroke and per-word paths are already cheap: a reading window plans in a millisecond and an utterance normalizes in 8 microseconds. The whole-document costs that remain are allocation and copying, not algorithms. And memory is dominated by the marker vector, a third of the retained document.

## Loading

The Markdown loader parses once since the September work: `convert_in` (`crates/textweaver-formats/src/markdown.rs:152`) runs one `pulldown-cmark` pass at `markdown.rs:173`, and only inline footnotes pay the second pass in `collect_footnotes` (`markdown.rs:233`). The `Builder` (`crates/textweaver-formats/src/builder.rs`) is the document builder every loader shares; `Document::new` (`crates/textweaver-text/src/document.rs:160`) clamps and sorts the markers.

- `Builder` starts from `String::new()` and `Vec::new()` (`builder.rs:40`). On 10 MB the text doubles about 24 times and the marker vector about 18 times, each a copy; loading allocates 116 MB for a 10 MB file. A size hint from the loaders (the source length, and `source.len() / 64` markers, close to the 159,128 seen) removes most of it. Estimate: 10 to 20 ms and 20 MB of allocation.
- `push_raw` (`builder.rs:98`) counts chars on every word (`builder.rs:100`). An `is_ascii` fast path is a few milliseconds on 10 MB (estimate). `verbatim` (`builder.rs:181`) runs two `replace` calls per code block even without a `\r`; a `contains` check first removes both allocations.
- The idiom `split_whitespace().collect::<Vec<_>>().join(" ")` appears at `markdown.rs:130`, `markdown.rs:267`, `markdown.rs:1099`, `pdf/mod.rs:251`, `pdf/mod.rs:391`, `docx.rs:192`, and `epub.rs:260`, allocating a vector and a string per call. One `collapse_ws(&str) -> Cow<str>` that returns the input when it is already collapsed serves all of them.
- `Document::new` sorts every marker (`document.rs:167`), 5.6 ms on 10 MB whether or not they are already sorted; loaders produce them nearly in order, so an `is_sorted_by_key` check first skips the sort for the common case.
- `Marker` (`crates/textweaver-text/src/marker.rs:20`) is 72 bytes: two `usize` positions, kind, level, and two `Option<String>`. Most of the 159,128 markers carry no label or reference. A compact in-memory marker with `u32` positions and the strings interned in a per-document table would be 16 to 24 bytes: the vector drops from 11.5 MB to about 3.5 MB (estimate), and `MarkerTables::build` (`marker.rs:143`), `enclosing` (`marker.rs:329`), and `Document::apply` touch a third of the cache lines. It is the largest memory win in the model and a wide change: `Marker` is public and serialized in state files through `DocumentData` (`document.rs:128`). Keep the serialized form; change the in-memory one.

The PDF loader reads the file into memory (`pdf/mod.rs:101`) and parses it with lopdf, whose `rayon` feature parses objects in parallel. Pages are then interpreted one by one (`pdf/mod.rs:143`) with a shared `FontCache` (`pdf/interp.rs:146`), and the per-page `alts` map is rebuilt by filtering the whole tag table per page (`pdf/mod.rs:194`), a pages-times-images loop. The real PDF win is page parallelism (see Concurrency). EPUB and DOCX read each part through `Package::read` (`package.rs:92`) and parse it with roxmltree or scraper; the parsers dominate, and both gain from `collapse_ws` and the size hints.

## The narration plan and the OffsetMap

`plan` is at `crates/textweaver-text/src/narrate.rs:146`; composition is `OffsetMap::compose` at `crates/textweaver-core/src/offset_map.rs:295`. The 7.5 allocations per utterance come from `Planner::sentence` (`narrate.rs:246`):

1. `prefix_at` (`narrate.rs:296`) returns a `String` even when empty, and each `format!` in it allocates again.
2. `pieces` is a fresh `Vec<Piece>` per sentence (`narrate.rs:284`), and each `Piece` owns a `String`.
3. `build` (`narrate.rs:411`) calls `doc.slice` (`document.rs:232`), which copies the rope slice into a `String`, and `push_literal` (`offset_map.rs:485`) copies it again into the builder. Two allocations and two copies for one literal piece.
4. `SpokenBuilder` grows its `String` and `Vec<Span>` from empty.
5. `push` (`narrate.rs:449`) calls `split_long` (`narrate.rs:642`), which returns `vec![(text, map)]` even when nothing is split: one `Vec` per utterance, dropped at once.

The three `enclosing` lookups per sentence (`narrate.rs:265`, `narrate.rs:266`, `narrate.rs:273`) are binary searches plus a short backward scan bounded by the `reach` table (`marker.rs:329`); they allocate nothing and cost microseconds. They stay.

Recommendations, in order of value:

- **Write literals from the rope.** Add `SpokenBuilder::push_literal_chunks` taking `RopeSlice::chunks()` and have `build` use it. Removes one allocation and one copy per literal piece: about 111,000 allocations and 9.5 MB of copying on 10 MB. Estimate 20 to 30 ms.
- **No `Vec` from `split_long`**: return a `SmallVec<[_; 1]>` or extend the caller's vector. 111,000 allocations; estimate 5 to 10 ms.
- **Pre-size the builder** with `clip.len()` and four spans; most sentences have one to three. About 200,000 growth reallocations (estimate).
- **Reuse `pieces` and the prefix buffer** on the `Planner`, and make `Piece` hold `Cow<'static, str>` so "link, " and the list labels do not allocate. About 100,000 allocations (estimate).
- A bump arena (bumpalo, see Sources) would cover only the temporaries the changes above remove, since the outputs must be owned; not worth the dependency.

Together: 830,000 allocations to about 250,000, and 50 to 80 ms of 301 (estimates), with the same changes shortening the 1 ms reading window and so open to first speech. A line-start table was rejected in September because keeping it current costs typing; the measured `line_breaks` path (`document.rs:303`) is 35 ms of the 170 ms sentence cost, and it should stay until the allocation work is done.

On the service side, `Pipeline::apply` (`crates/textweaver-speech/src/normalize/mod.rs:261`) composes every transform's map, and `compose` builds a `char_to_byte` vector of the intermediate text (`offset_map.rs:297`) each time. At 69 allocations and 8 microseconds per utterance this is fine at speech rate; measure it, change nothing.

## Text units

`crates/textweaver-text/src/units.rs` walks the rope block by block and never materializes the document, which is right. What it pays:

- `sentences_in` (`units.rs:621`) collects the paragraph into `Vec<char>` (`units.rs:626`), 36 MB of allocation on 10 MB and most of the 53.5 MB the sentence path allocates, for random access in the abbreviation and ellipsis checks. Working on the `&str` with byte offsets, converting to char offsets once per boundary with `pieces` (`units.rs:503`, which counts chars as it goes), removes the vector. Estimate: 20 to 30 ms of the 55 ms that is not ICU4X.
- `words_in` (`units.rs:568`) collects every boundary segment, whitespace included, into a vector of 32-byte tuples through `classify` (`units.rs:557`) before joining hyphenated compounds. Streaming the join with a two-segment lookahead removes most of the 177 MB allocated and 60 to 90 ms of 343 (estimate). Whole-range word segmentation is what difficult words, bionic reading, and syllables do, so this also shortens the first frame with those aids on.
- There is no word or sentence cache in the text crate, and none is needed: a word step segments one line window, and the app caches the aid ranges per revision and range (`crates/textweaver-app/src/frame_cache.rs`).
- `blank_lines` (`document.rs:339`) rescans every line after every edit, 17.7 ms on 10 MB, paid by the next sentence step or Space after a keystroke. An edit outcome says which lines were replaced by how many: shift the indices after the edit and rescan only the edited lines. Estimate: 18 ms to under 0.1 ms per edit. The marker tables are the same shape at 2.7 ms; do them second.
- `Document::apply` (`document.rs:370`) re-sorts every marker on every edit (`document.rs:376`), 1.64 ms on 10 MB. Shifting is monotone, so only markers touching the edit can change order; sort that slice, not the vector. The app sidesteps this for large Markdown by dropping the markers in edit mode (`crates/textweaver-app/src/edit.rs:865`); other formats and the GUI's `ReplaceRange` still pay.
- Many-term highlighting: the September measurement had aho-corasick at 43 ms against 656 ms for a 19-word regex alternation. Difficult words do not need it (they look each word up in the sorted SCOWL index, `crates/textweaver-aids/src/difficult.rs:290`), and the abbreviation regex (`normalize/abbreviations.rs:176`) runs in microseconds. Adopt aho-corasick when a feature matches hundreds of terms over a window, not before.

## The speech thread

`run` (`crates/textweaver-speech/src/service.rs:758`) waits on the command channel with the timeout from `next_wakeup` (`service.rs:1058`): `POLL_INTERVAL`, 10 ms (`service.rs:255`), while anything is queued, and no timeout when idle, so an idle reader costs nothing. Each step polls the backend, drains events, pumps the queue, and runs the timers (`service.rs:1522`).

- Normalization happens in `refill` (`service.rs:1663`), one utterance at a time as the queue drops below `lookahead + 2`, so a long reading starts after normalizing four sentences: 32 microseconds. The pipeline is built once per settings change, and every rule regex is compiled in a constructor (`normalize/rewrite.rs:94`, `normalize/rewrite.rs:107`, `normalize/numbers.rs:284`); the community lexicon uses a `OnceLock` (`normalize/community.rs:212`).
- The exception is `normalize/ssml.rs:10`: `re()` compiles a regex on every call, and `text_to_ssml` calls it several times per utterance for engines that take SSML. That is a few hundred microseconds per sentence on the speech thread. Replace with `LazyLock<Regex>` statics: small, certain, alpha.8.
- `begin` (`service.rs:1980`) splits the utterance into words once, `handle_event` (`service.rs:1793`) does a binary search per word event, and `to_submit` (`queue.rs:171`) clones each utterance once for the engine: fine at speech rate.
- The status channel is an unbounded `mpsc` (`service.rs:372`); `StatusLink::forward` (`service.rs:699`) rings the waker once per batch, and the app drains with `try_recv` (`crates/textweaver-app/src/playback.rs:751`). At 2 to 5 words per second there is no contention anywhere.

Stop to first audio: `read_as` (`service.rs:1122`) calls `clear_engine` (`service.rs:1603`), which stops the backend synchronously, then normalizes four utterances and speaks. The service's own share is under 100 microseconds; what a user hears is the backend's `stop` (a round trip to a host process and an audio flush) plus the engine's time to its first sample, and neither is measured today. The harness's `ClockBackend` can measure the service share exactly and `tw speak` with a real engine the rest (see the measurement plan). Until then any number here would be a guess, so I give none.

## The GUI document widget

`DocumentView` (`crates/textweaver-xilem/src/document.rs`) does not re-lay out the window per word. The spoken word arrives through `set_state` (`document.rs:695`), which marks the paragraph that lost the word and the one that gained it dirty (`mark_dirty`, `document.rs:827`). The layout pass (`document.rs:1851`) lays out only the paragraphs on screen through `ensure_layout` (`document.rs:1021`), a per-paragraph Parley layout cache capped at 600 and trimmed to the paragraphs near the view (`document.rs:1030`); the layouts are dropped only when the column, font, or spacing changes (`document.rs:1855`, `document.rs:743`, `document.rs:760`). So the per-paragraph layout cache already exists, and its gain is already taken.

What the GUI pays per word:

- The paint pass (`document.rs:1868`) re-encodes every visible paragraph with `render_text` (`document.rs:2040`) each frame, and the spoken word's paragraph twice more, clipped to the word (`document.rs:2066`) and the selection (`document.rs:2075`). Masonry hands the widget a fresh `Painter` per frame, so retaining a Vello scene per paragraph needs a change in the vendored Masonry (`third_party/xilem/masonry`). Vello caches glyph outlines; the per-frame cost is encoding perhaps 20 to 40 paragraphs. Estimate 0.3 to 1.5 ms of CPU per frame, unverified; run the frame-time probe before building anything here.
- The accessibility pass (`document.rs:2188`) rebuilds the two dirty paragraphs' runs through `runs_of` (`document.rs:1594`): `clusters` (`runs.rs:154`) segments the paragraph into graphemes and words and fills a `HashSet` of word starts, `make_run` (`runs.rs:358`) copies each run's text, and the pass clones the runs again (`document.rs:2225`). On the order of 100 microseconds for a 2,000-char paragraph (estimate).

Slicing the 500,000-unit window: `DocWindow::span` (`crates/textweaver-app/src/window.rs:133`) finds paragraph boundaries by walking at most a quarter of the budget. On a slide (`follow`, `window.rs:245`), about once per eighth of the window, `refresh_host` builds a new model through `model_for` (`gui.rs:943`, called at `gui.rs:1084`): the window text as one `String`, then one `String` per paragraph (`runs::paragraphs`, `runs.rs:462`), then run strings for the paragraphs sent. The window's text is held about three times (rope, paragraphs, runs), a few MB for a 1 MB window; the document itself is held once, in the rope.

## The terminal reader

The loop (`crates/textweaver-tui/src/lib.rs:108`) draws only when `wants_draw` (`ui.rs:852`) says so, and `view_signature` (`ui.rs:813`) hashes the spoken range, so a word move does draw. The draw (`ui.rs:878`) is a full ratatui frame: ratatui diffs cells and writes only the changed ones, but the CPU work is a whole frame. `draw_body` (`ui.rs:1028`) re-wraps the visible lines through `layout::window_decor` (`layout.rs:230`), collecting each line into a `Vec<char>` (`layout.rs:195`), then `row_spans` builds styled spans per row; the aid ranges come from the app's frame caches. Estimate for one frame of 40 rows: under one millisecond, so at five words a second it is well under 1 percent of a core. The loop waits 10 ms for a key while reading and 40 ms when idle (`lib.rs:78`), and draws once a second when nothing changes.

## Startup

Before the first announcement the terminal reader (`crates/textweaver-tui/src/setup.rs:135`) reads `settings.toml` and `keymap.toml`, builds the keymap from the preset and overrides, starts the speech engine on a helper thread (`setup.rs:209`), probes the OS color scheme on another (`setup.rs:193`), and builds the `App` (`crates/textweaver-app/src/app.rs:468`), which parses the message catalog in `Study::new` (`crates/textweaver-app/src/study.rs:161`), builds the theme registry (23 TOML themes parsed once per process, `crates/textweaver-theme/src/builtin.rs:65`), loads user themes, and sends the voice settings to speech (`crates/textweaver-app/src/voice.rs:1103`).

The Fluent catalog is parsed from `include_str!` sources (`crates/textweaver-lexicon/src/i18n/mod.rs:65`): `en.ftl` is 175 KB, parsed once per process (`i18n/mod.rs:262`), and a translation (187 to 237 KB) on top of it. Estimate 1 to 3 ms, measured nowhere; `tw --version` is already only 7 to 10 ms above an empty program, so a build-time parse is not worth its tooling. The SCOWL list (692 KB zipped, 225,038 words) unpacks on first use only (`difficult.rs:317`), and `difficult_ranges` checks the setting first (`crates/textweaver-app/src/reading_aids.rs:427`); with the aid on at startup the first frame pays the unpack (estimate 20 to 40 ms, unverified). Startup is in good shape; the thing to add is a record of the two `--log debug` timings (`setup.rs:187`, `setup.rs:248`) in the harness.

## Store

`settings.rs` is 3,073 lines because it is the schema, not run-time work. Settings parse from TOML once at startup; `to_minimal_toml` (`crates/textweaver-store/src/settings.rs:1828`) serializes the settings and the defaults to compare them on every save, a few hundred microseconds (estimate) on the writer thread (`crates/textweaver-app/src/writer.rs`), never on the input thread. Document state is written as pretty JSON (`doc_state.rs:1328`) through `atomic_write` (`atomic.rs:9`), which syncs the file, every 30 seconds while the position moves (`app.rs:1353`).

Identity (`crates/textweaver-sync/src/docid.rs`): `file_sha256` (`docid.rs:95`) streams the file through a buffer, so the 100 MB bench stays near the buffer size; `TextHasher::update` (`docid.rs:130`) walks the text char by char and pushes one char at a time, 9 million `encode_utf8` calls on 10 MB, estimate 30 to 60 ms on the writer. A fast path that finds the next whitespace and extends the buffer with the whole run is 5 to 10 times faster (estimate). Nothing in the store is on the input thread.

## Build, profile, and allocator

The profiles are at the end of `Cargo.toml`: `release` with `lto = false`, `codegen-units = 16`, `strip = "symbols"`; `dist` with fat LTO and one codegen unit.

- **Do not set `panic = "abort"`.** The reader catches panics on purpose: the event loop (`crates/textweaver-tui/src/lib.rs:211`) to save work and restore the terminal, the PDF loader for a page that panics (`crates/textweaver-formats/src/pdf/mod.rs:144`), and the speech thread so the app can restart speech (`service.rs:393`). Abort would turn each into a lost document, for a few percent of size (estimate).
- **`opt-level = "s"` for cold crates in `dist`**: per-package overrides for `harper-core`, `hayagriva`, `krilla`, `calamine`, `mail-parser`, `syntect`, `two-face`, `comrak`, and `minijinja`. With fat LTO the final pass follows the top crate's level, so the gain is smaller than without it; estimate 1 to 3 MB of 47, measured one crate at a time. Never for `rten`, `icu_segmenter`, `regex`, `ropey`, or textweaver's own crates.
- **`-C target-cpu`: no.** Students run old laptops; the baseline target is right for packages, and RTen dispatches SIMD at run time.
- **Linker.** Rust has used lld by default on x86_64 Linux since 1.90 (see Sources), so `-fuse-ld=lld` is no longer needed; nothing to change on Windows.
- **PGO** through `cargo-pgo`, profiled on the bench corpora and `tw read`: typically 5 to 15 percent on parse-heavy code (estimate; cargo-pgo cites no number and warns that an unrepresentative profile can slow the binary). A day in `cargo xtask dist`, to try once on the load and plan numbers. BOLT is Linux ELF only; defer.
- **Allocator.** Loading makes 68,000 allocations and the plan 830,000 per 10 MB, so allocator speed matters most to the whole-document paths. mimalloc's README claims it outperforms jemalloc and tcmalloc and often uses less memory (see Sources); a Rust write-up found through search reports 5 to 6 times faster small allocations than Windows's default allocator (unverified: the page could not be fetched through this proxy). The `mimalloc` crate (0.1.52, May 22, 2026) is C compiled with `cc`, which the workspace already needs for LAME and Opus. Add a `mimalloc` feature on the three binaries, off by default, and measure `cargo xtask bench` on Windows with and without it. Estimate: 10 to 20 percent off load and plan on Windows, less on Linux, for 100 to 200 KB of binary.
- **Feature pruning.** The reader's defaults (`crates/textweaver-tui/Cargo.toml:17`) are `publish`, `lint`, `grammar`, `clipboard`, `highlight`, `dictation`, `audio-export`, and `opus`; `--no-default-features` already drops `publish` (18.6 MB), `grammar` (11 MB), `highlight` (syntect and two-face's syntax dumps, several MB, estimate), dictation (Whisper on RTen), and the audio encoders. What the lean reader still carries that it could drop: the in-process OCR engine, since `textweaver-formats` has `ocr` on by default; a reader feature that turns it into `images` (Tesseract only) saves 2 to 4 MB (estimate, unverified). The SCOWL list (692 KB) and ICU4X data (58 KB) should stay.
- **Duplicate crates.** `cargo tree -d --workspace --edges normal` lists 59 crate names in two or more versions. The runtime ones: `hashbrown` in three versions; `read-fonts` and `skrifa` in three each (the GUI, through Parley, Vello, and krilla); `sha2` 0.10 and 0.11 with `digest` and `md-5` in both; `tiny-skia`, `vello_cpu`, `lzma-rust2` (noted in Testing), `quick-xml`, `roxmltree`, `rand`, `thiserror`, `bitflags`, `fancy-regex`, `itertools`, `strum`, `libloading`, `miniz_oxide`, `base64`, `bincode`, and `derive_more` in two each; `syn` and `proc-macro2` duplicates are build-time only. `cargo tree -i NAME@VERSION` finds each owner. Estimate 1 to 2 MB across all of them, a day of dependency work.

## Concurrency

Rayon is used in one place, `tw convert` (`crates/textweaver-convert/src/lib.rs:714`), with its own pool, and batch conversion from the reader leaves one core free (`crates/textweaver-app/src/batch.rs:250`). lopdf's `rayon` feature parses objects on the global pool, which rayon sizes to every core: on a 2-core laptop that is both cores for the parse while the speech and audio threads also want time. Recommendation: one `worker_threads()` helper returning `available_parallelism() - 1`, at least 1, and a global pool built with it once per binary, so lopdf and any future `par_iter` share the policy.

Where parallelism would pay: PDF pages. `pdf/mod.rs:143` interprets pages one by one with one `&mut FontCache`. With a font cache per thread (fonts are keyed by object id, so merging is a map union), pages can be interpreted and laid out with `par_iter`. Estimate 1.6 times faster on 2 cores and 3 times on 4 for the interpretation share; measure with a 300-page fixture under `--file`. OCR already runs on RTen, which threads inside the model. Segmenting paragraphs in parallel for the plan would not be felt in the reader, which plans 32 KB windows in a millisecond; it would help audio export's whole-document plan, after the allocation work. Loading already moves to a helper thread at 512 KB (`crates/textweaver-app/src/opening.rs:41`). Lock contention: the wake slot is a mutex locked once per status batch and the frame caches are mutexes on one thread; there is none to find.

## Memory

On 10 MB the loaded document retains 30.2 MB: the rope (about 17 MB with ropey's node overhead, estimated from the remainder), the markers (11.5 MB plus their strings), and about 2 MB of tables when built. Loading peaks 45 MB above that. A whole-document plan holds about 24 MB (spoken text, 185,083 spans at 32 bytes, 72 bytes per `Utterance`), which only audio export and the bench build; the reader holds one window, the GUI the window's text about three times, the terminal reader a screenful. The one structural win is the compact marker: about 8 MB off a 10 MB document.

## A measurement plan

Add to `xtask/src/bench.rs`, each as one report line and one JSON key, so the gate covers it:

1. **Segmentation on its own**: `sentences_ms`, `sentences_allocs`, `words_ms`, and `words_allocs` over the whole document through `Units`, before the plan. The plan number hides them today.
2. **Normalization**: `normalize_ms` and `normalize_allocs` for the first 2,000 utterances of the plan through `Pipeline::for_settings` with the default config.
3. **Stop to first audio, service share**: with the `ClockBackend`, start reading, wait for the first `speak`, dispatch `Stop` then `ReadFromCursor`, and time to the next `speak`; `stop_to_speak_ms` mean and max over 20 repetitions.
4. **Stop to first audio, engine share**: a `tw speak --time` flag that logs, for a real engine, the time from `speak` to `Started` and to the first audio frame; run by hand on Windows with SAPI and Eloquence and recorded in Testing.
5. **GUI frame time**: the GUI records `highlight_ms` and `load_ms` already (`gui.rs:385`). Add, under the `screenshot` feature's headless harness, a `--measure` run that opens the 1 MB corpus, moves the spoken word 200 times, and prints the p50 and max of the layout, paint, and accessibility passes per word. Record it in Testing; gate it once it is stable.
6. **Edit on a large document**: `apply_ms` for one insert on 10 MB with markers kept, and the blank-line rebuild after it; both regress silently today.
7. **Startup breakdown**: read the two `--log debug` timings from a reader run that exits at once (the GUI has `exit_at`, `gui.rs:308`; the terminal reader needs the same flag).
8. **Allocator A/B on Windows**: a `--features mimalloc` `tw` passed with `--tw`, compared with the baseline JSON.
9. **A nightly flamegraph**: a `profiling` profile inheriting `release` with `debug = "line-tables-only"` and `strip = false`; the nightly job runs `cargo flamegraph --profile profiling --bin tw -- read target/bench-corpus/md-10mb.md --backend null` on Linux (perf) and uploads the SVG, and `samply record` on the macOS and Windows runners (see Sources). They are there when a number moves.

## Ranked plan

Sizes: small is hours, medium a day or two, large a week or more. Gains are estimates unless a probe number is cited.

| # | Item | Where | Expected gain | How to measure | Size | Target |
|---|---|---|---|---|---|---|
| 1 | Push literals from rope chunks, no `doc.slice` String per piece | `narrate.rs:411`, `offset_map.rs:485` | 111,000 fewer allocations and 9.5 MB less copying per 10 MB plan; 20 to 30 ms of 301 (estimate) | `plan_all_allocs`, `plan_all_ms` | small | alpha.8 |
| 2 | `split_long` without a `Vec`; pre-sized `SpokenBuilder`; reused `pieces` and prefix buffers | `narrate.rs:642`, `narrate.rs:449`, `narrate.rs:296`, `narrate.rs:284` | about 400,000 fewer allocations per 10 MB plan; 30 to 50 ms (estimate) | `plan_all_allocs`, `open_to_first_speech_ms` | small | alpha.8 |
| 3 | `LazyLock` regexes in SSML markup | `normalize/ssml.rs:10` | a few hundred microseconds per sentence off the speech thread for SSML engines | `normalize_ms` with an SSML backend | small | alpha.8 |
| 4 | Incremental blank-line table after edits | `document.rs:339`, `document.rs:370` | 18 ms to under 0.1 ms per edit on 10 MB | new `apply_ms` and `blank_lines_ms` | medium | alpha.8 |
| 5 | Builder size hints, ASCII fast path in `push_raw`, `verbatim` without two `replace` calls | `builder.rs:40`, `builder.rs:100`, `builder.rs:181` | 10 to 20 ms and 20 MB of allocation off a 10 MB load (estimate) | `load_ms`, `load_allocs`, `load_peak_mb` | small | alpha.8 |
| 6 | Skip the marker sort when sorted; sort only the touched slice after an edit | `document.rs:167`, `document.rs:376` | 5 ms per load; 1.6 ms to microseconds per edit on 10 MB | `load_ms`, `apply_ms` | small | alpha.8 |
| 7 | Add the measurements: segmentation, normalization, stop-to-speak, edit, GUI frame time | `xtask/src/bench.rs`, `gui.rs` | regressions in five paths become visible | the new keys | medium | alpha.8 |
| 8 | Measure the engine share of stop-to-first-audio by hand on Windows | `tw speak` | the number users feel, recorded for the first time | item 4 of the plan | small | alpha.8 |
| 9 | `sentences_in` on byte offsets, no `Vec<char>` per paragraph | `units.rs:621` | 36 MB less allocation and 20 to 30 ms of 170 per 10 MB (estimate) | `sentences_ms`, `sentences_allocs` | medium | alpha.9 |
| 10 | Stream the hyphen join in `words_in`, no segment vector | `units.rs:568`, `units.rs:557` | most of 177 MB of allocation and 60 to 90 ms of 343 per 10 MB (estimate); a faster first frame with the aids on | `words_ms`, `words_allocs` | medium | alpha.9 |
| 11 | Compact `Marker`: `u32` positions, interned label and reference | `marker.rs:20`, `document.rs:128`, every loader | 11.5 MB to about 3.5 MB per 10 MB document (estimate); faster shifts and lookups | `load_peak_mb`, retained MB in the document line | large | alpha.9 |
| 12 | A `mimalloc` feature on the binaries, measured on Windows | `Cargo.toml`, the three `main.rs` | 10 to 20 percent off load and plan on Windows (estimate); 100 to 200 KB | bench A/B, item 8 of the plan | small to measure | alpha.9 |
| 13 | Page-parallel PDF interpretation with a per-thread font cache, on a shared `worker_threads()` policy | `pdf/mod.rs:143`, `pdf/interp.rs:146`, `batch.rs:250` | 1.6 times on 2 cores, 3 times on 4 for the interpretation share (estimate) | a 300-page PDF under `--file`, `load_ms` | medium | alpha.9 |
| 14 | One `collapse_ws` returning `Cow` for the whitespace idiom in six loaders | `markdown.rs:130`, `pdf/mod.rs:251`, `docx.rs:192`, `epub.rs:260` | thousands of allocations per load | `load_allocs` on the fixtures | small | alpha.9 |
| 15 | Per-page `alts` built once from the tag table | `pdf/mod.rs:194` | removes a pages-times-images loop in tagged textbooks | `load_ms` on a tagged PDF fixture | small | alpha.9 |
| 16 | `opt-level = "s"` per cold package in `dist`, one at a time | `Cargo.toml` profiles | 1 to 3 MB of 47 (estimate) | the release size table in Testing | small | alpha.9 |
| 17 | Remove runtime duplicate crates (`sha2`, `roxmltree`, `quick-xml`, `lzma-rust2`, `thiserror`, `bitflags`, the font stacks) | `Cargo.toml`, `cargo tree -i` | 1 to 2 MB (estimate) | binary size; the `cargo tree -d` count | medium | alpha.9 |
| 18 | PGO through `cargo-pgo` in `cargo xtask dist`, profiled on the bench corpora | `xtask/src/dist.rs` | 5 to 15 percent on load and plan (estimate) | the bench on the dist binary | medium | later |
| 19 | Retained per-paragraph scenes in the GUI paint pass (needs a Masonry change) | `document.rs:1868`, `third_party/xilem/masonry` | 0.3 to 1.5 ms of CPU per frame (estimate, unverified) | the GUI frame-time probe | large | later |
| 20 | `TextHasher` fast path over whitespace-free runs; a cached search `String` per revision | `docid.rs:130`, `search.rs:105` | 30 to 60 ms per 10 MB identity (estimate) on the writer; 1.8 ms per find | the `identity` bench; `search_ms` | small | later |

Three things the plan leaves alone on purpose: the rope (ADR-0034 stands; ropey 2 is still `2.0.0-beta.1` from August 2, 2025, and crop is still 0.4.3 from April 25, 2025, see Sources); the three `enclosing` lookups per sentence, which cost microseconds; and `panic = "abort"`, which would break the panic recovery the reader relies on. One note for the harness: the ALSA failure means `cargo xtask bench` needs `libasound2-dev` on Linux, which is worth a line in [Building](../building.md) if it is not there.

## Sources

- ropey versions on crates.io (2.0.0-beta.1 on August 2, 2025; 1.6.1 the newest stable): https://crates.io/api/v1/crates/ropey
- crop versions on crates.io (0.4.3 on April 25, 2025): https://crates.io/api/v1/crates/crop
- icu_segmenter versions (2.3.0 on August 13, 2026): https://crates.io/api/v1/crates/icu_segmenter
- ICU4X releases, 2.3.0 with the redesigned segmentation engine (`new_neo`, Unicode PRI #555) and Unicode 17 line breaking: https://github.com/unicode-org/icu4x/releases
- Vello versions (0.10.0 on August 14, 2026): https://crates.io/api/v1/crates/vello
- Parley releases (0.11.1 on August 16, 2026, a dependency update): https://github.com/linebender/parley/releases
- mimalloc README (performance claims, Windows support): https://github.com/microsoft/mimalloc
- mimalloc crate (0.1.52, May 22, 2026): https://crates.io/api/v1/crates/mimalloc
- Rust 1.90.0: lld the default linker on x86_64-unknown-linux-gnu: https://blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable
- Rust 1.96.0 release notes: https://github.com/rust-lang/rust/releases/tag/1.96.0
- Rust 1.97.0 release notes (v0 symbol mangling by default, new integer bit APIs, `build.warnings`): https://github.com/rust-lang/rust/releases/tag/1.97.0
- Rust 1.97.1 (an LLVM miscompilation fix): https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/
- cargo-pgo (PGO and BOLT workflow): https://github.com/Kobzol/cargo-pgo
- BOLT (x86-64 and AArch64 ELF only): https://github.com/llvm/llvm-project/tree/main/bolt
- cargo flamegraph (perf, xctrace, blondie): https://github.com/flamegraph-rs/flamegraph
- samply (macOS, Linux, Windows): https://github.com/mstange/samply
- aho-corasick: https://github.com/BurntSushi/aho-corasick
- bumpalo: https://github.com/fitzgen/bumpalo

## See also

- [Research index](README.md)
- [Testing](../testing.md): the bench harness and the September 2026 numbers this audit builds on.
- [Architecture](../architecture.md): the crates, threads, and the path from a file to a highlighted word.
- [ADR-0034: The rope after measurement](../../adr/0034-rope-after-measurement.md): why the rope stays, and when to look again.
