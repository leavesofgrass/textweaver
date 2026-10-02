# Performance audit and optimization plan

This document is a code-level audit of where textweaver spends time and memory, with a ranked plan for making it faster, smaller, and leaner. It is for contributors who will do the work in the alpha.8 and alpha.9 waves, and for the owner deciding what to schedule. Every claim about textweaver's own code names the file and line it comes from, every number says where it was measured, and every estimate is marked as one. Written on Friday, October 2, 2026, against branch `claude/affectionate-meitner-jduzl8` at 0.1.0-alpha.7.

## What the numbers say today

The recorded numbers are in [Testing](../testing.md), "Benchmarks": on Windows, a 10 MB Markdown file loads in 275 ms, the whole-document narration plan takes 610 ms with 844,757 allocations, open to first speech is 353 ms, and ICU4X's sentence segmentation (115 ms per 10 MB) was called the floor for finding sentences. `tw --version` starts in 23 to 35 ms. The reader is 47 MB, `tw` 53 MB, and the lean reader without `publish` 28.7 MB.

For this audit I built `textweaver-core`, `textweaver-text`, and `textweaver-formats` in release mode (2 minutes 57 seconds on this 4-core Linux machine, Rust 1.96.1), then ran a standalone probe outside the workspace with a counting allocator on the same generated corpora `cargo xtask bench` uses (the generator was copied from `xtask/src/bench.rs`). `cargo xtask bench --quick --no-startup --only md-1mb` itself did not run: after 4 minutes 31 seconds of building, `alsa-sys` (rodio, through the app's engines) failed for want of the ALSA development library, so the numbers below are the probe's, not the harness's. Allocation counts agree with the harness within 2 percent (830,821 against 844,757 for the plan), so they are comparable; times are faster than the Windows numbers, as a quiet Linux machine usually is.

The probe on the 10 MB corpus (9,140,703 chars, 72,898 lines, 159,128 markers), best of three runs:

- Load: 208 ms, 67,862 allocations, 116 MB allocated in all, peak 45 MB above the baseline. Retained by the loaded `Document`: 30.2 MB, of which the marker vector is 11.5 MB (159,128 markers at 72 bytes each).
- Sentences alone, through `Units`: 170 ms, 214,689 allocations, 53.5 MB allocated. The documented ICU4X cost is 115 ms, so about 55 ms and all of those allocations are textweaver's own wrapping.
- Words alone: 343 ms, 368,707 allocations, 177 MB allocated for 1,432,384 words.
- Whole-document plan: 301 ms, 830,821 allocations (7.5 per utterance), 113 MB allocated, 111,480 utterances with 185,083 spans. One 32,768-char reading window, which is what Space plans: 0.97 ms for 397 utterances.
- Marker tables (built on first use): 2.7 ms. Blank-line table: 17.7 ms and 9,355 allocations. One insert through `Document::apply` with markers kept: 1.64 ms; rebuilding the blank lines and tables after it: 18 ms.
- `find_all` of "the" (28,481 hits): 6.9 ms, of which copying the rope to a `String` is 1.8 ms; peak 16 MB.
- Normalizing 2,000 utterances through the default pipeline (`math`, `abbreviations`, `numbers`, `punctuation`): 8 microseconds and 69 allocations per utterance. Building the pipeline: 3.7 ms, 23,857 allocations.

On the 1 MB corpus everything is about a tenth of that (load 16 ms, plan 27 ms, sentences 16 ms), so the costs scale linearly and nothing quadratic is left in these paths.

Three conclusions follow. First, the reader's per-keystroke and per-word paths are already cheap: a reading window plans in a millisecond and an utterance normalizes in 8 microseconds. Second, the whole-document costs that remain (load, segmentation, plan) are dominated by allocation and copying, not by algorithms. Third, memory is dominated by the marker vector, which is a third of the retained document.

## Loading

The Markdown loader (`crates/textweaver-formats/src/markdown.rs`) parses once since the September work: `convert_in` at `markdown.rs:152` runs one `pulldown-cmark` pass at `markdown.rs:173`, and only inline footnotes pay the second pass in `collect_footnotes` at `markdown.rs:233`. The `Builder` (`crates/textweaver-formats/src/builder.rs`) is the document builder every loader shares; `Document::new` at `crates/textweaver-text/src/document.rs:160` clamps and sorts the markers.

Findings:

- `Builder` starts with `String::new()` and `Vec::new()` (`builder.rs:40` through `builder.rs:48`). On 10 MB the text grows by doubling about 24 times and the marker vector about 18 times, each a copy. Loading allocates 116 MB for a 10 MB file and peaks 45 MB above baseline. `Builder::new` could take a size hint (the source length, and `source.len() / 64` markers, which is close to the 159,128 seen for 10 MB) from the Markdown, HTML, and text loaders. Estimate: 10 to 20 ms and 20 MB of allocation off the 10 MB load.
- `push_raw` at `builder.rs:98` counts chars on every word (`s.chars().count()` at `builder.rs:100`). The bench corpus is nearly all ASCII; `if s.is_ascii() { s.len() }` first is a few milliseconds on 10 MB (estimate).
- `Builder::verbatim` at `builder.rs:181` does two `replace` calls on every code block, allocating twice even when there is no `\r`. A `contains('\r')` check first removes both allocations for most files.
- `plain_title` at `markdown.rs:130` and the same `split_whitespace().collect::<Vec<_>>().join(" ")` idiom at `markdown.rs:267`, `markdown.rs:1099`, `pdf/mod.rs:251`, `pdf/mod.rs:391`, `docx.rs:192`, and `epub.rs:260` allocate a vector and a string to collapse whitespace. A small `collapse_ws(&str) -> Cow<str>` in the formats crate that returns the input when it is already collapsed would serve all of them. Small gain each, but it is the one idiom repeated across loaders.
- `html_tag` at `markdown.rs:762` collects the tag name into a `String` per inline HTML tag; a stack buffer or `eq_ignore_ascii_case` against the slice avoids it. Only HTML-heavy Markdown notices.
- `Document::new` sorts all markers (`document.rs:167`), 5.6 ms on 10 MB whether or not they are already sorted. Loaders produce markers in close order; `is_sorted_by_key` first would skip the sort for the common case. Estimate: 5 ms.
- `Marker` (`crates/textweaver-text/src/marker.rs:21`) is 72 bytes: two `usize` positions, a `u8` kind and level, and two `Option<String>` for the label and reference. Of 159,128 markers on the bench corpus, only list items, links, images, code blocks, headers, and footnotes carry a label or reference. A compact marker with `u32` positions and the label and reference as indices into a per-document string table would be 16 to 24 bytes: the vector goes from 11.5 MB to about 3.5 MB (estimate), `MarkerTables::build` (`marker.rs:143`) and `enclosing` (`marker.rs:329`) touch a third of the cache lines, and `Document::apply` shifts a third of the bytes. This is the largest memory win in the document model, and it is a wide change: `Marker` is public, serialized in state files through `DocumentData` (`document.rs:117`), and read by every loader and writer. Keep the serialized form and change the in-memory one.
- The text goes through two full copies after the builder: `Builder::finish` hands back a `String` (`builder.rs:372`) and the loader builds the rope from it with `Rope::from_str`. The rope build is 12 ms (from the September measurement) and unavoidable with ropey 1; the `String` could be pre-sized as above.

The PDF loader (`crates/textweaver-formats/src/pdf/`) reads the whole file into memory (`pdf/mod.rs:101`) and parses it with `lopdf::Document::load_mem`, which uses lopdf's `rayon` feature for object parsing (see `Cargo.toml`, the `lopdf` entry). Pages are then interpreted one by one at `pdf/mod.rs:143` with a shared `FontCache` (`pdf/interp.rs:147`), and `layout::layout` runs per page at `pdf/mod.rs:201` with the per-page `alts` map rebuilt by filtering the whole tag table each time (`pdf/mod.rs:194` through `pdf/mod.rs:199`), which is quadratic in pages times tagged images. `layout.rs` sorts lines three times on the way through (`layout.rs:265`, `layout.rs:371`, `layout.rs:538`, `layout.rs:693`), each a `sort_by` with float comparisons; one sort of spans by `(y, x0)` up front and stable passes after would do, though the sorts are small per page. The real PDF win is page parallelism (see Concurrency).

EPUB (`epub.rs`) and DOCX (`docx.rs`) read each part through `Package::read` (`package.rs:92`), which reads the member to a `Vec<u8>` and then to a `String`, and parse with `roxmltree` or `scraper`. The parsers dominate; `docx.rs:121` collects descendant text to a `String` and collapses it again. Neither loader has a quadratic step left. Both would benefit from the shared `collapse_ws` and from `Builder` size hints keyed on the uncompressed part size.

## The narration plan and the OffsetMap

`plan` lives in `crates/textweaver-text/src/narrate.rs:146`; the service-side composition is `OffsetMap::compose` in `crates/textweaver-core/src/offset_map.rs:295`. The 830,821 allocations on 10 MB (7.5 per utterance) come from, per sentence in `Planner::sentence` (`narrate.rs:246`):

1. `prefix_at` (`narrate.rs:296`) returns a `String` even when empty, and each `format!` inside it allocates again.
2. `pieces` is a fresh `Vec<Piece>` per sentence (`narrate.rs:285`), and each `Piece` carries its own `String`.
3. `build` (`narrate.rs:411`) calls `doc.slice` (`document.rs:232`), which copies the rope slice to a `String`, then `push_literal` (`offset_map.rs:485`) copies it again into the builder's text. For a plain sentence that is two allocations and two copies for one literal piece.
4. `SpokenBuilder` grows its `String` and `Vec<Span>` from empty.
5. `push` at `narrate.rs:449` calls `split_long` (`narrate.rs:642`), which returns `vec![(text, map)]` even when nothing is split: one `Vec` per utterance that is dropped at once by `extend`.
6. The `Utterance` wrapping in `plan_with` moves the `String` and `OffsetMap` without copying, but `out` grows by doubling.

The three `enclosing` lookups per sentence (`narrate.rs:264`, `narrate.rs:265`, `narrate.rs:272`) are binary searches plus a short backward scan bounded by the `reach` table (`marker.rs:329` through `marker.rs:347`); they allocate nothing and cost microseconds. They are not where the time goes, and they stay.

Recommendations, in order of value:

- **Write literals from the rope, not through a String.** Add `SpokenBuilder::push_literal_chunks(impl Iterator<Item = &str>, CharPos)` and have `build` walk `doc.text().slice(range).chunks()`. Removes one allocation and one copy per literal piece: about 111,000 allocations and 9.5 MB of copying on 10 MB. Estimate 20 to 30 ms.
- **Return without a Vec from `split_long`.** Make it return `SmallVec<[(String, OffsetMap); 1]>` or take the output `Vec` to extend into. Removes 111,000 allocations. Estimate 5 to 10 ms.
- **Pre-size the builder.** `SpokenBuilder::with_capacity(clip.len() + 16, 4)`: most sentences have one to three spans and the text length is known. Removes most growth reallocations, about 200,000 on 10 MB (estimate).
- **Reuse `pieces` and the prefix buffer.** Keep one `Vec<Piece>` and one `String` on the `Planner` and clear them per sentence; make `Piece` hold `Cow<'static, str>` so "link, " and "strikethrough, " and the list labels (already `String`s on the marker) do not allocate. Estimate 100,000 allocations.
- **A bump arena per plan call** (bumpalo, see Sources) would make every temporary above free, but the outputs must outlive the call as owned `String`s, so the arena only covers the temporaries that the four changes above already remove. Not worth the dependency.

Together these cut the plan's allocations from 830,000 to about 250,000 (estimate) and its time by 50 to 80 ms of 301 (estimate), and the same changes apply to the 1 ms reading window, which matters for the open-to-first-speech number. A line-start table was considered in September and rejected because keeping it up to date costs typing; the measured `line_breaks` path (`document.rs:303`, one `line_to_char` per line) is 35 ms of the 170 ms sentence cost, and I agree it should stay as it is until the allocation work above is done.

On the service side, `Pipeline::apply` (`crates/textweaver-speech/src/normalize/mod.rs:260`) composes each transform's map with `OffsetMap::compose`, which builds a `char_to_byte` vector of the whole intermediate text (`offset_map.rs:297`) per transform per utterance. With 69 allocations and 8 microseconds per utterance, this is fine at speech rate and I recommend no change beyond measuring it in the harness.

## Text units

`crates/textweaver-text/src/units.rs` walks the rope block by block and never materializes the document, which is right. The costs it does pay:

- `sentences_in` at `units.rs:621` collects the paragraph into `Vec<char>` (`units.rs:626`), 4 bytes per char: 36 MB of allocation on 10 MB and most of the 53.5 MB the sentence path allocates. It needs random access by char index for the abbreviation and ellipsis checks. Working on the `&str` with byte offsets, and converting to char offsets once per sentence boundary with `pieces` (which already counts chars as it goes, `units.rs:503`), removes the vector. Estimate: 20 to 30 ms of the 55 ms that is not ICU4X.
- `words_in` at `units.rs:568` collects every word-boundary segment, whitespace and punctuation included, into a `Vec<(usize, usize, bool, bool)>` at `units.rs:570` through `classify` (`units.rs:557`), 32 bytes each, before joining hyphenated compounds. Streaming the join with a two-segment lookahead removes the vector: on 10 MB that is most of the 177 MB allocated and some of the 343 ms (estimate: 60 to 90 ms). Word segmentation of a whole document is what difficult words, bionic reading, syllables, and `tw info` do over their ranges, so this also shortens the first frame with those aids on.
- `block_text` (`units.rs:773`) already borrows from the rope when the block lies in one chunk, and copies otherwise; paragraphs longer than a rope chunk (about 1 KB in ropey 1) copy. That is most paragraphs of real prose. The copy is unavoidable for ICU4X, which wants a `&str`, so it stays.
- Caches: there is no word or sentence cache in the text crate; the app's `frame_cache.rs` caches the aid ranges per revision and range, and `Document` caches the marker tables and blank lines per revision. Incremental re-segmentation after an edit is not needed for navigation (a word step segments one line window) and not worth its complexity for the aids, which are already cached per frame.
- `blank_lines` (`document.rs:339`) rescans every line after every edit, 17.7 ms on 10 MB, and the next sentence step or Space after a keystroke pays it. An incremental update is simple here: an edit outcome says which lines were replaced by how many; shift the indices after the edit and rescan only the lines inside it. Estimate: 18 ms to under 0.1 ms per edit on 10 MB. The marker tables are the same shape of problem but cheaper (2.7 ms); do them second.
- `Document::apply` (`document.rs:370`) re-sorts every marker on every edit (`document.rs:376`), 1.64 ms on 10 MB. Shifting is monotone, so only markers that collapsed can change order; sorting the slice between the first and last marker that touches the edit, not the whole vector, keeps it correct and makes it microseconds. The app already avoids this for large Markdown by dropping the markers in edit mode (`crates/textweaver-app/src/edit.rs:862`), but other formats and the GUI's `ReplaceRange` path still pay it.
- `find_all` (`search.rs:105`) copies the rope to a `String` on every search (1.8 ms of 6.9 ms on 10 MB). Caching that `String` on the session per revision would make repeated finds cheaper, but the regex itself is the larger part and the current cost is already under 10 ms. Low priority.
- Many-term highlighting: the September measurement had aho-corasick at 43 ms against 656 ms for a 19-word regex alternation. Difficult words do not need it (they segment words and look each up in the sorted SCOWL index, `crates/textweaver-aids/src/difficult.rs:290`); the candidates are the abbreviation transform (one alternation regex over about a hundred abbreviations built in `normalize/abbreviations.rs:176`) and a future grammar or glossary highlighter. At 8 microseconds per utterance the abbreviations do not need it either. Adopt aho-corasick when a feature needs hundreds of terms over a window, not before.

## The speech thread

`crates/textweaver-speech/src/service.rs` runs one thread: `run` (`service.rs:758`) waits on the command channel with a timeout from `next_wakeup` (`service.rs:1058`), which is `POLL_INTERVAL` (10 ms, `service.rs:255`) while anything is queued and `None` when idle, so an idle reader sleeps on the channel and costs nothing. Per step it polls the backend, drains events, pumps the queue, and runs the timers (`service.rs:1522`).

Per-utterance work:

- Normalization happens in `refill` (`service.rs:1663`), one utterance at a time as the queue drops below `lookahead + 2`, so a long reading starts after normalizing four sentences: 32 microseconds by the probe. The pipeline is built once per settings change (`rebuild_pipeline`, 3.7 ms), and every rule regex is compiled in a constructor (`normalize/rewrite.rs:94` and `normalize/rewrite.rs:107`, `normalize/numbers.rs:284`), never per utterance. The community lexicon uses a `OnceLock` (`normalize/community.rs:212`). This is as it should be.
- The exception is `normalize/ssml.rs:10`: `re()` compiles a regex on every call, and `text_to_ssml` calls it several times per utterance, for the engines that take SSML (speech-dispatcher, and SAPI through the host). Each compile is tens to hundreds of microseconds, so this is a few hundred microseconds per sentence on the speech thread. Replace with `LazyLock<Regex>` statics. Small, certain, alpha.8.
- `begin` (`service.rs:1980`) splits the utterance into words with `spoken_words` once per utterance; `handle_event` (`service.rs:1793`) does a binary search per word event and `source_for` through the map. `to_submit` (`queue.rs:201`) clones each utterance once when handing it to the engine, so the queue and the backend each hold a copy; that is one `String` and one `Vec<Span>` per sentence, fine at speech rate.
- The status channel is an unbounded `mpsc` (`service.rs:372`); `StatusLink::forward` (`service.rs:699`) sends each status and rings the waker once per batch. At 2 to 5 words per second there is no contention anywhere; the frontends drain with `try_recv` (`crates/textweaver-app/src/playback.rs:751`). No change.

Stop to first audio. A new read goes `read_as` (`service.rs:1122`) to `clear_engine` (`service.rs:1603`), which calls `backend.stop()` synchronously, then normalizes four utterances and calls `speak`. The service's own share is under 100 microseconds; the latency a user hears is the backend's `stop` (for host engines, a round trip to the host process and the audio device flush) plus the engine's time to the first audio sample. Neither is measured today. The harness's `ClockBackend` can measure the service share exactly (see the measurement plan), and `tw speak` with a real engine can measure the rest; until then any number here would be a guess, so I give none.

## The GUI document widget

`DocumentView` (`crates/textweaver-xilem/src/document.rs`) does not re-lay out the window per word. The spoken word arrives through `set_state` (`document.rs:695`), which marks the paragraph that lost the word and the one that gained it dirty (`mark_dirty`, `document.rs:827`), requests a layout and a render. The layout pass (`document.rs:1851`) calls `layout_view`, which lays out only the paragraphs on screen through `ensure_layout` (`document.rs:1021`), a per-paragraph Parley layout cache capped at `CACHE_LIMIT` 600 and trimmed to the paragraphs near the view (`document.rs:1030` through `document.rs:1034`). The layouts are dropped only when the column width, the font, or the spacing changes (`document.rs:1855`, `document.rs:743`, `document.rs:760`). So the per-paragraph layout cache the brief asks about already exists; what it would gain is already gained.

What the GUI does pay per word:

- The paint pass (`document.rs:1868`) re-encodes every visible paragraph with `render_text` (`document.rs:2040`) on every frame, and the paragraph holding the spoken word twice more, clipped to the word's band (`document.rs:2066`) and the selection (`document.rs:2075`). Masonry gives the widget a fresh `Painter` per frame, so a retained per-paragraph Vello scene is not possible without a change in the vendored Masonry (`third_party/xilem/masonry`). Vello caches glyph outlines, so the per-frame cost is scene encoding of perhaps 20 to 40 paragraphs. Estimate: 0.3 to 1.5 ms of CPU per frame on a laptop, unverified; the frame-time probe below should be run before anything is built here.
- The accessibility pass (`document.rs:2188`) rebuilds the runs of the two dirty paragraphs through `runs_of` (`document.rs:1594`), which clones the paragraph, builds a `RunSet`, and in `clusters` (`runs.rs:154`) segments the paragraph into graphemes and words and fills a `HashSet` of word starts, then `make_run` (`runs.rs:358`) copies each run's text. Then the pass clones the runs again with `to_vec()` (`document.rs:2225`). For a 2,000-char paragraph that is on the order of 100 microseconds (estimate). A `Vec<bool>` indexed by byte would replace the `HashSet`, and `runs_of` could return a borrow by splitting the borrow of `para_runs` from `self`; both are small.
- The GUI's refresh (`gui.rs:1036`) rebuilds `marks_in` and compares it with the shown marks on every tick (`gui.rs:1125` through `gui.rs:1131`), and `state_for` the same; cheap unless a document has thousands of notes.

Slicing the 500,000-unit window: `DocWindow::span` (`crates/textweaver-app/src/window.rs:138`) finds paragraph boundaries with `boundary_before` and `boundary_after` (`window.rs:332`, `window.rs:368`), which walk at most a quarter of the budget in chars and keep a `Vec<char>` of the current line. On a slide (`follow`, `window.rs:262`), `refresh_host` builds a whole new model through `model_for` (`gui.rs:1084`): the window text as one `String` (`DocWindow::text`), then one `String` per paragraph (`runs::paragraphs`, `runs.rs:462`), then run strings for the paragraphs that are sent. So the window's text is held about three times (rope, paragraphs, runs): on the order of 3 MB for a 1 MB window, which is not a problem. A slide happens once per eighth of the window, about every ten minutes of reading, and the GUI logs its time ("loaded N chars ... in ms"). The app keeps one copy of the document (the rope); the GUI does not keep a second copy of the document, only of the window.

What to do: nothing until measured. Add the frame-time probe, then decide whether per-paragraph scene retention in Masonry is worth a patch to the vendored tree.

## The terminal reader

The event loop in `crates/textweaver-tui/src/lib.rs:95` draws only when something changed (`Tui::wants_draw`, `ui.rs:852`), using `view_signature` (`ui.rs:813`), which hashes the spoken range, so a word move does trigger a draw. The draw (`draw_at`, `ui.rs:878`) is a full ratatui frame: ratatui diffs cells and writes only the changed ones to the terminal, so the terminal traffic per word is a few cells, but the CPU work is a whole frame: `draw_body` (`ui.rs:1028`) re-wraps the visible lines through `layout::window_decor` (`layout.rs:230`), which collects each line into a `Vec<char>` (`line_chars`, `layout.rs:195`) and wraps it, then `row_spans` builds styled spans per row. With the reading aids on, the aid ranges come from the app's per-frame caches (`crates/textweaver-app/src/frame_cache.rs`), so they are not recomputed. The loop waits 10 ms for a key while reading (`READING_POLL`, `lib.rs:78`) and 40 ms when idle, so an idle reader wakes 25 times a second, but draws only on the one-second heartbeat.

Estimate for one frame at 40 rows of prose: under one millisecond, so at 5 words a second this is well under 1 percent of one core. A cache of wrapped rows keyed by (revision, top line, width, cells, decor) would make the per-word frame a span rebuild only; small gain, later. There is no full re-layout of the document per word and no redraw while idle.

## Startup

Before the first announcement the terminal reader (`crates/textweaver-tui/src/setup.rs:135`) reads `settings.toml` and `keymap.toml`, builds the keymap from the preset and overrides (`setup.rs:160`), starts the speech engine on a helper thread, and builds the `App` (`crates/textweaver-app/src/app.rs:468`), which parses the message catalog through `Study::new` (`crates/textweaver-app/src/study.rs:161`), builds the theme registry (23 built-in themes parsed from TOML once per process behind a `OnceLock`, `crates/textweaver-theme/src/builtin.rs:65`), loads user themes, and sends the voice settings to the speech service (`crates/textweaver-app/src/voice.rs:1103`).

- The Fluent catalog is parsed from `include_str!` sources (`crates/textweaver-lexicon/src/i18n/mod.rs:65`): `en.ftl` is 175 KB and the five translations 187 to 237 KB each. English is parsed once per process (`i18n/mod.rs:262`), and a translation is parsed on top of it. Estimate: 1 to 3 ms per catalog; measured nowhere yet. A build-time parse into a static table would save it, but `tw --version` is already 7 to 10 ms above a program that does nothing, so this is the whole budget, not a big piece of it.
- The SCOWL list (692 KB zipped) is unpacked and parsed on first use only (`difficult.rs:313`), and `difficult_ranges` checks the setting before touching it (`crates/textweaver-app/src/reading_aids.rs:427`). Nothing to do; keep it lazy, and keep the first use off the input thread when difficult words are on at startup (today the first frame pays it when the aid is on: estimate 20 to 40 ms, unverified).
- The OS color-scheme probe runs on a helper thread (`setup.rs:193`), and the speech engine starts in the background (`setup.rs:209`). Both are right.
- `Registry::with_builtins()` and `Keymap::with_preset_and_overrides` build tables in memory; neither reads a file. Small.

Startup is in good shape. The one thing to add is a measurement: `--log debug` already prints "settings and keys read" and "reader built" times (`setup.rs:186`, `setup.rs:247`); the harness should record them.

## Store

`crates/textweaver-store/src/settings.rs` is 3,073 lines because it is the schema, not because it does work at run time. Settings are parsed from TOML once at startup, and `to_minimal_toml` (`settings.rs:1828`) serializes both the settings and the defaults to compare them on every save: two `toml::Table::try_from` calls, estimate a few hundred microseconds, on the writer thread (`crates/textweaver-app/src/writer.rs`), never on the input thread. Document state is written with `serde_json::to_string_pretty` (`doc_state.rs:1328`) through `atomic_write` (`atomic.rs:9`), which syncs the file; positions are saved every 30 seconds while they move (`app.rs:1353`) and on quit, so the sync cost is paid 120 times an hour at most, on the writer. The library file is one JSON document saved whole (`library.rs:693`); with thousands of entries that is milliseconds per save, still on the writer.

Document identity (`crates/textweaver-sync/src/docid.rs`): `file_sha256` streams the file through a buffer (`docid.rs:95`), so the 100 MB sync-id bench stays near the buffer size; `TextHasher::update` (`docid.rs:124`) walks the text char by char and pushes bytes one char at a time. For a 10 MB text that is 9 million `encode_utf8` calls, estimate 30 to 60 ms on the writer. A fast path that finds the next whitespace with `find` and extends the buffer with the whole run would be 5 to 10 times faster (estimate). Small, later; nothing here is on the input thread.

## Build, profile, and allocator

The profiles are at the end of `Cargo.toml`: `release` with `lto = false`, `codegen-units = 16`, `strip = "symbols"` for contributors, and `dist` with fat LTO and one codegen unit for packages. Findings and recommendations:

- **Do not set `panic = "abort"`.** The reader catches panics on purpose: the event loop in `crates/textweaver-tui/src/lib.rs:236` catches one to save work and restore the terminal, the PDF loader catches a page that panics at `crates/textweaver-formats/src/pdf/mod.rs:144`, and the speech thread's panic is caught so the app can restart speech (`service.rs:750`). Abort would turn each of those into a lost document. The size saved (unwind tables, a few percent, estimate) is not worth it.
- **`opt-level = "s"` for cold crates in `dist`.** Per-package overrides (`[profile.dist.package.harper-core] opt-level = "s"`, and the same for `hayagriva`, `krilla`, `calamine`, `mail-parser`, `syntect`, `two-face`, `comrak`, `minijinja`) shrink code that runs rarely. With fat LTO the final optimization is driven by the top crate's level, so the gain is smaller than without LTO; estimate 1 to 3 MB of the 47 MB, to be measured one crate at a time. Never for `rten`, `icu_segmenter`, `regex`, `ropey`, or textweaver's own crates.
- **`-C target-cpu`: no.** Students run old laptops; the baseline x86-64 target is right for packages. RTen dispatches SIMD at run time already. A contributor can set it in their own `.cargo/config.toml`.
- **Linker.** Rust uses lld by default on `x86_64-unknown-linux-gnu` since 1.90 (see Sources), so `-C link-arg=-fuse-ld=lld` is no longer needed there; on Windows the MSVC linker is used, and no change is recommended.
- **PGO.** `cargo-pgo` automates instrumented build, profile run, and optimized build; the bench corpora and `tw read` give a representative workload. Gains on parse-heavy code are typically 5 to 15 percent (estimate; cargo-pgo's README cites no number and warns that an unrepresentative profile can slow the binary). It is a release-pipeline change (`cargo xtask dist`), a day's work, and should be tried once on the plan and load numbers. BOLT works on Linux ELF only, so it could serve the AppImage, not Windows; defer.
- **Allocator.** The plan path makes 830,000 allocations per 10 MB and loading 68,000, so allocator speed matters on the paths that matter least to users (whole-document work), and little on the per-word paths. Windows's default allocator is slower than mimalloc for small blocks; mimalloc's README claims it outperforms jemalloc and tcmalloc and often uses less memory (see Sources), and a Rust write-up reports 5 to 6 times faster small allocations on Windows (unverified; the page could not be fetched through this proxy). The `mimalloc` crate (0.1.52, May 22, 2026) is C compiled with `cc`, which the workspace already requires for LAME and Opus. Recommendation: add a `mimalloc` cargo feature on the three binaries, off by default, and measure `cargo xtask bench` on Windows with and without it. Estimate: 10 to 20 percent off load and plan on Windows, less on Linux, at about 100 to 200 KB of binary. Adopt in `dist` only if the Windows numbers say so.
- **Feature pruning and the lean reader.** `textweaver-tui`'s defaults (`crates/textweaver-tui/Cargo.toml:15`) are `publish`, `lint`, `grammar`, `clipboard`, `highlight`, `dictation`, `audio-export`, `opus`. The lean reader (`--no-default-features`) already drops `publish` (18.6 MB: convert, render, writers, cite, hayagriva, comrak, minijinja, krilla, ammonia, ureq), `grammar` (11 MB: harper-core and the burn framework), `highlight` (syntect and two-face's syntax dumps, several MB, estimate), `dictation` (Whisper on RTen), and audio export (LAME, libopus, flacenc). What it keeps that it could drop: the SCOWL list (692 KB, needed for difficult words and spelling, keep), ICU4X data (58 KB, keep), the bundled fonts are already `publish`-only. The remaining lean size is the engines, RTen for Piper, and the formats crate with its OCR engine (`ocr` is a default feature of formats). A `textweaver-tui` feature that turns `textweaver-formats/ocr` into `images` would drop the in-process ocrs model code for a reader that uses Tesseract or no OCR; estimate 2 to 4 MB, unverified. Measure with `cargo bloat --crates` on a developer machine; it is not installed here.
- **Duplicate crates.** `cargo tree -d --workspace --edges normal` lists 59 crate names present in two or more versions. The ones that reach the binaries, by inspection: `hashbrown` 0.15, 0.16, and 0.17; `rand` 0.9 and 0.10; `sha2` 0.10 and 0.11 with `digest`, `block-buffer`, and `md-5` in both; `read-fonts` 0.37, 0.39, and 0.41 with `skrifa` 0.40, 0.42, and 0.44; `tiny-skia` 0.11 and 0.12; `vello_cpu` and `vello_common` 0.0.7 and 0.0.8; `lzma-rust2` 0.16 and 0.21 (already noted in Testing); `quick-xml` 0.38 and 0.41; `roxmltree` 0.20 and 0.21; `thiserror` 1 and 2; `bitflags` 1 and 2; `fancy-regex` 0.16 and 0.18; `itertools` 0.14 and 0.15; `strum` 0.27 and 0.28; `libloading` 0.8 and 0.9; `miniz_oxide` 0.8 and 0.9; `base64` 0.22 and 0.23; `bincode` 1 and 2; `derive_more` 1 and 2. (`syn` 0.15, 2, and 3 and `proc-macro2` 0.4 are build-time only.) Each runtime duplicate is tens to hundreds of KB of code. The three font stacks (`read-fonts`, `skrifa`) and the two `tiny-skia` are in the GUI through Parley, Vello, and krilla; `sha2` 0.11 and `roxmltree` 0.20 come from dependencies that moved ahead of textweaver's own pins (`sha2 = "0.10"`, `roxmltree = "0.21"`). `cargo tree -i sha2@0.11` and the same for each name finds the owner; bumping textweaver's pin or waiting for the dependency to move removes the copy. Estimate: 1 to 2 MB across all of them, a day of dependency work.

## Concurrency

Rayon is used in one place, `tw convert` (`crates/textweaver-convert/src/lib.rs:714`), with its own pool sized to the files, and the app's batch conversion leaves one core for the reader (`crates/textweaver-app/src/batch.rs:250`). lopdf's `rayon` feature parses objects on the global pool, which rayon sizes to every core: on a 2-core laptop that is both cores for the parse while the speech thread and the audio thread also want time. Recommendation: one `textweaver_core::worker_threads()` helper returning `available_parallelism() - 1`, at least 1, and a `rayon::ThreadPoolBuilder::new().num_threads(..).build_global()` once at startup in each binary, so lopdf and any future `par_iter` share that policy.

Where parallelism would pay:

- **PDF pages.** `pdf/mod.rs:143` interprets pages sequentially with one `&mut FontCache`. Make the font cache per thread (fonts are keyed by object id, so merging is a map union), interpret pages with `par_iter`, then run `layout::layout` per page in the same pass; OCR of blank pages (`ocr::recognize_pages`) already runs on RTen, which uses threads inside the model. Estimate 1.6 times faster on 2 cores and 3 times on 4 for the interpretation and layout share of a text PDF; measure with a 300-page fixture added to `--file`.
- **Segmenting paragraphs in parallel for the plan.** The reader plans 32 KB windows in a millisecond, so parallel planning would not be felt there; the whole-document plan is used by audio export and the bench, where paragraphs could be segmented with `par_chunks` and the utterances concatenated in order. Worth doing only for export; after the allocation work.
- **Loading on a background thread** is already done for files of 512 KB or more (`crates/textweaver-app/src/opening.rs`).

Lock contention: `WakeSlot` is a mutex locked once per status batch; the frame caches are mutexes on one thread; the voice cache is read without waiting. There is none to find.

## Memory

On 10 MB the loaded document retains 30.2 MB: the rope (about 17 MB including ropey's node overhead, estimate from the remainder), the markers (11.5 MB plus their label and reference strings), and the per-document tables when built (about 2 MB). Loading peaks 45 MB above that, from the builder's `String` and marker vector doubling and the second copy into the rope. A whole-document plan holds 111,480 utterances: 9.5 MB of spoken text, 185,083 spans at 32 bytes (6 MB), and 72 bytes per `Utterance` (8 MB), about 24 MB, which only audio export and the bench build; the reader holds one 32 KB window's worth. The GUI holds the window's text about three times (rope slice, paragraphs, runs), a few MB. The terminal reader holds the rope and a screenful.

The one structural win is the compact marker (above): about 8 MB off the 10 MB document, and faster shifts, sorts, and lookups. `Span` could be 24 bytes with `u32` source positions, but spans are per utterance, short-lived in the reader, and 32 bytes is fine.

## A measurement plan

Add to `xtask/src/bench.rs`, each one line of the report and one JSON key, so the gate covers it:

1. **Segmentation on its own**: `sentences_ms` and `sentences_allocs`, `words_ms` and `words_allocs` over the whole document through `Units`, before the plan. Today the plan number hides them.
2. **Normalization**: `normalize_ms` and `normalize_allocs` for the first 2,000 utterances of the plan through `Pipeline::for_settings` with the default `NormalizeConfig`, so a transform that regresses is caught.
3. **Stop to first audio, service share**: with the `ClockBackend`, start reading, wait for the first `speak`, dispatch `Stop` then `ReadFromCursor`, and time from the dispatch to the next `speak` call. Report `stop_to_speak_ms` mean and max over 20 repetitions.
4. **Stop to first audio, engine share**: a `tw speak --time` flag (or `cargo xtask listen`) that logs, for a real engine, the time from `speak` to the first `Started` and to the first audio frame the playback client receives; run by hand on Windows with SAPI and Eloquence and recorded in Testing, as the September numbers were.
5. **GUI frame time**: the GUI already records `highlight_ms` and `load_ms` (`gui.rs:384`). Add, under the `screenshot` feature's headless harness, a `--measure` run that opens the 1 MB corpus, moves the spoken word through 200 words, and prints the p50 and max of the layout, paint, and accessibility passes per word, from Masonry's pass timings or `Instant`s around `set_state`. Record it in Testing with the machine, then gate it later once it is stable.
6. **Edit on a large document**: `apply_ms` for one insert on the 10 MB corpus with markers kept, and the blank-line rebuild after it; both regress silently today.
7. **Startup breakdown**: have `bench-run` read the `--log debug` lines "settings and keys read" and "reader built" from a `textweaver --exit-after 0` run, if such a flag exists for the GUI (`exit_at` in `gui.rs:3095`) and is added to the terminal reader.
8. **Allocator A/B on Windows**: a `--features mimalloc` build of `tw` passed with `--tw`, compared with the baseline JSON.
9. **A nightly flamegraph**: a `profiling` profile inheriting `release` with `debug = "line-tables-only"` and `strip = false`; the nightly job runs `cargo flamegraph --profile profiling --bin tw -- read target/bench-corpus/md-10mb.md --backend null` on Linux (perf) and uploads the SVG, and `samply record` on the macOS and Windows runners (samply runs on all three, flamegraph uses perf, xctrace, and blondie respectively; see Sources). Nobody has to read them weekly; they are there when a number moves.

## Ranked plan

Sizes: small is hours, medium a day or two, large a week or more. Gains are estimates unless a probe number is cited.

| # | Item | Where | Expected gain | How to measure | Size | Target |
|---|---|---|---|---|---|---|
| 1 | Push literals from rope chunks, no `doc.slice` String per piece | `narrate.rs:411`, `offset_map.rs:485` | 111,000 fewer allocations and 9.5 MB less copying per 10 MB plan; 20 to 30 ms of 301 (estimate) | `plan_all_allocs`, `plan_all_ms` | small | alpha.8 |
| 2 | `split_long` without a `Vec` per utterance; pre-sized `SpokenBuilder`; reused `pieces` and prefix buffers | `narrate.rs:642`, `narrate.rs:449`, `narrate.rs:296`, `narrate.rs:285` | about 400,000 fewer allocations per 10 MB plan; 30 to 50 ms (estimate) | `plan_all_allocs`, `open_to_first_speech_ms` | small | alpha.8 |
| 3 | `LazyLock` regexes in SSML markup | `normalize/ssml.rs:10` | a few hundred microseconds per sentence off the speech thread for SSML engines | `normalize_ms` with an SSML backend in the harness | small | alpha.8 |
| 4 | Incremental blank-line table after edits | `document.rs:339`, `document.rs:370` | 18 ms to under 0.1 ms per edit on 10 MB | new `apply_ms` and `blank_lines_ms` | medium | alpha.8 |
| 5 | Builder size hints and ASCII fast path in `push_raw`; `verbatim` without two `replace` calls | `builder.rs:40`, `builder.rs:100`, `builder.rs:181` | 10 to 20 ms and 20 MB of allocation off a 10 MB load (estimate) | `load_ms`, `load_allocs`, `load_peak_mb` | small | alpha.8 |
| 6 | Skip the marker sort when already sorted; sort only the touched slice after an edit | `document.rs:167`, `document.rs:376` | 5 ms per load; 1.6 ms to microseconds per edit on 10 MB | `load_ms`, `apply_ms` | small | alpha.8 |
| 7 | Add the measurements: segmentation, normalization, stop-to-speak, edit, GUI frame time | `xtask/src/bench.rs`, `gui.rs` | regressions in five paths become visible | the new keys themselves | medium | alpha.8 |
| 8 | Measure the stop-to-first-audio engine share by hand on Windows | `tw speak` or `xtask listen` | the number users feel, recorded for the first time | item 4 of the measurement plan | small | alpha.8 |
| 9 | `sentences_in` on byte offsets, no `Vec<char>` per paragraph | `units.rs:621` | 36 MB less allocation and 20 to 30 ms of 170 per 10 MB (estimate) | `sentences_ms`, `sentences_allocs` | medium | alpha.9 |
| 10 | Stream the hyphen join in `words_in`, no segment vector | `units.rs:568`, `units.rs:557` | most of 177 MB of allocation and 60 to 90 ms of 343 per 10 MB (estimate); faster first frame with difficult words and bionic on | `words_ms`, `words_allocs` | medium | alpha.9 |
| 11 | Compact `Marker`: `u32` positions, interned label and reference | `marker.rs:21`, `document.rs:117`, every loader | 11.5 MB to about 3.5 MB per 10 MB document (estimate); faster shifts and lookups | `load_peak_mb`, retained MB in the document line | large | alpha.9 |
| 12 | A `mimalloc` feature on the binaries, measured on Windows | `Cargo.toml`, the three `main.rs` | 10 to 20 percent off load and plan on Windows (estimate); 100 to 200 KB of binary | `cargo xtask bench` A/B, item 8 of the measurement plan | small to measure, then a decision | alpha.9 |
| 13 | Page-parallel PDF interpretation and layout with a per-thread font cache, on a shared `worker_threads()` policy | `pdf/mod.rs:143`, `pdf/interp.rs:147`, `batch.rs:250` | 1.6 times on 2 cores, 3 times on 4 for the interpretation share (estimate) | a 300-page PDF under `--file`, `load_ms` | medium | alpha.9 |
| 14 | A shared `collapse_ws` returning `Cow` for the whitespace idiom in six loaders | `markdown.rs:130`, `pdf/mod.rs:251`, `docx.rs:192`, `epub.rs:260` | thousands of allocations per load, small time | `load_allocs` on the fixtures | small | alpha.9 |
| 15 | Per-page `alts` map built once from the tag table, not filtered per page | `pdf/mod.rs:194` | removes a pages-times-images loop; matters for tagged textbooks with many figures | `load_ms` on a tagged PDF fixture | small | alpha.9 |
| 16 | `opt-level = "s"` per cold package in `dist`, measured one at a time | `Cargo.toml` profiles | 1 to 3 MB of 47 (estimate) | binary size in Testing's release table | small | alpha.9 |
| 17 | Remove runtime duplicate crates (`sha2`, `roxmltree`, `quick-xml`, `lzma-rust2`, `thiserror`, `bitflags`, font stacks) | `Cargo.toml`, `cargo tree -i` | 1 to 2 MB (estimate) | binary size; `cargo tree -d` count | medium | alpha.9 |
| 18 | PGO through `cargo-pgo` in `cargo xtask dist`, profiled on the bench corpora | `xtask/src/dist.rs` | 5 to 15 percent on load and plan (estimate) | `cargo xtask bench` on the dist binary | medium | later |
| 19 | Retained per-paragraph scenes in the GUI paint pass (needs a Masonry change) | `document.rs:1868`, `third_party/xilem/masonry` | 0.3 to 1.5 ms of CPU per frame (estimate, unverified) | the GUI frame-time probe | large | later |
| 20 | `TextHasher` fast path over whitespace-free runs; cached search `String` per revision | `docid.rs:124`, `search.rs:105` | 30 to 60 ms per 10 MB identity (estimate) on the writer; 1.8 ms per find | the `identity` bench; `search_ms` | small | later |

Three things the plan deliberately leaves alone: the rope (ADR-0034 stands; ropey 2 is still `2.0.0-beta.1` from August 2, 2025, and crop is still 0.4.3 from April 25, 2025, see Sources); the three `enclosing` lookups per sentence, which cost microseconds; and `panic = "abort"`, which would break the panic recovery the reader relies on.

## What I ran

- `rustup toolchain install 1.96` (the pinned toolchain was not installed), then `cargo build --release -p textweaver-core -p textweaver-text -p textweaver-formats`: finished in 2 minutes 57 seconds, 229 crates compiled, 578 MB target directory.
- `cargo xtask bench --quick --no-startup --only md-1mb`: failed after 4 minutes 31 seconds of building with "The system library `alsa` required by crate `alsa-sys` was not found" (rodio, through `textweaver-engines`). The harness needs `libasound2-dev` on Linux; worth a line in [Building](../building.md) if it is not there.
- A standalone probe crate in the session's scratch folder, depending on the four crates by path with the formats crate's default features off, run on the 10 MB and 1 MB generated corpora; its output is quoted in "What the numbers say today".
- `cargo tree -d --workspace --edges normal`: 59 crate names in more than one version.
- `cargo bloat` is not installed and was not run.

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
