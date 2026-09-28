# Wave 5, recalibrated

Written on Monday, September 28, 2026, by the Wave 5 recalibration planner, for the orchestrator and the owner. It replaces the shape of [the Wave 5 plan](wave5-plan.md) and section 5 of [the 2026 roadmap](roadmap-2026.md) where they differ; everything not named here stands as those files wrote it. It changes nothing by itself: the orchestrator applies it to `docs/history/tasks.md` at step 0.

What it is built on: `CLAUDE.md`; the Wave 5 plan; the roadmap (its "Wave 5, refined", the feature-complete list, the owner's answers, and the owner's changes of Monday, September 28, 2026); [What is left](whats-left.md); every Wave 4 status line, the standing decisions, "Pace after Wave 4", the follow-ups and sessions 1 and 2 in `docs/history/tasks.md`; the Cloud Agent's outcome in `docs/history/reservations.md`; section 6 of the [Wave 4 orchestration plan](wave4-orchestration.md); the 0.1.0-alpha.4 changelog; ADR-0002, ADR-0013, ADR-0023, ADR-0028, ADR-0029; `docs/dictation.md` and `crates/textweaver-dictation`; the lexicon's translation tests. Versions were read from crates.io, GitHub, Hugging Face and arXiv today with the tools' default User-Agent. No personal identifier was sent. Nothing was built, run in Docker, downloaded, or deleted.

## The short version

- **Eleven agents instead of nine, in three overlapping sub-waves.** At most three build heavily at once; two agents never build on this machine (W5t, W5p) and run from the first hour; light builders fill a fourth slot only when memory allows.
- **Four of the plan's five branches are settled by Wave 4:** stay on ropey 1.6 (branch A); the GUI's memory is the graphics stack, so W5m is struck and its leftovers move; MathCAT #827 is still open, so W5c4 vendors 0.7.6-rc.3 with the fix; W4a3 landed, so the GUI's third pass starts with the Parley upgrade. The fifth (W4d landed) adds a rule: every new message ships in all six languages, because the catalog test demands it.
- **The GUI's third pass (W5a4) launches first,** beside the Braille pass (W5x) and the Star leftovers (W5y). It is on the critical path and touches nothing else.
- **Pulled forward from Wave 6** into slots Wave 5 no longer needs (W5m struck, W5e deferred): the publishing templates (W5g, on the feature-complete list) and speed presets (last item of W5y). A small streaming dictation spike (W5d) measures before W6d starts.
- **Streaming dictation recommendation:** for W6d, VAD-bounded LocalAgreement-2 on the in-process Whisper already shipped, with Moonshine (MIT, variable-length encoder) on RTen as the low-latency engine if the spike shows RTen runs it and the owner approves the model. The Wave 5 spike measures Whisper only, with no download.
- **The owner's gates:** Braille session B1 on the HumanWare Mantis Q40 after W5x merges; check 2 (documents and math) after 5b; GUI session 3 after W5a4 merges; the alpha.5 readiness page at the end. Each gates only the work that uses its findings.

## 1. What changed since the Wave 5 plan

### Struck (done elsewhere)

- **W5m's fuzz deliverables 1 to 3 and `cargo xtask fuzz-seed`:** the Cloud Agent, pull request 1 (nine targets; W4c2's three added after). Struck.
- **W5p's settings reference and `cargo xtask docs --check`, and quick wins 1, 2, 4 and 5:** the Cloud Agent. Struck. W5p keeps the release and readiness work.
- **epubcheck and veraPDF (from W6t):** the Cloud Agent (`second-tool.yml`). liblouis grade 2 in CI stays with W6t.
- **GUI citations, export, preview and spell check in edit mode:** W4a3. Struck from W6a5's list.
- **Translations:** W4d, six languages complete.
- **The wxDragon spike:** removed (W4f's merge).
- **W5e (translation):** deferred to Wave 6 by the owner. ADR-0038 stays reserved for it.
- **Grammar (harper, behind `--features grammar`) and code highlighting (syntect):** in alpha.4 (changelog), so nothing of W4g's "held" list reaches Wave 5.

### Added

- **W6d, streaming dictation,** is on the feature-complete list. Wave 5 adds the research in section 3 and a measuring spike, W5d.
- **W6g, the publishing templates, and W6x, speed presets,** are on the list. This plan pulls both into Wave 5 slots (question 1).
- **Follow-ups from Wave 4:** the fuzz crate builds the ocrs engine through the formats crate's `ocr` feature (to W5r); `[editing] author` is not in the store, schema or settings reference (to W5y); the aarch64 AppImage is untested until a Release dry run (to W5p).

### The five branches, now taken

1. **The rope (W5r): branch A, stay on ropey 1.6.** W4b's numbers (ADR-0002 status update): crop is 2 to 3 times as fast as ropey 1 on edits and ropey 2 about 1.3 to 1.8 times, but 20,000 keystrokes on 10 MB take 10.4 ms with ropey 1, so one edit is well under a microsecond with all three. No trace a user feels is slow. Through char offsets ropey 2's edge mostly goes, and crop counts only LF and CRLF as line breaks. W4b found loading and the narration plan to be the bottleneck. W5r writes ADR-0034 as "stay", then spends its slot on those.
2. **Memory (W5m): struck.** ADR-0028: the app with every engine is about 11 MB; the window's 170 MB is the GPU stack, and Wave 3's 98 MB was read before the renderer finished starting. No app or engine fix is needed. The renderer options ADR-0028 lists (Vello with area antialiasing only, one wgpu backend) are GUI work and go to W5a4. The fuzz feature follow-up goes to W5r.
3. **MathCAT (W5c4): vendor and patch.** Issue #827 is open today, with no linked pull request, and no release fixes it (section 3.2). W4c1 landed (speech, EPUB MathML, ADR-0029), so W5c4 does braille and navigation, not W4c1's brief. The owner chose Nemeth first.
4. **The GUI (W5a4): the Parley upgrade first.** W4a3 landed with session 2's four fixes and edit mode on Parley 0.8.0. The voice manager stays in Wave 5.
5. **Messages (every agent): catalog ids, in all six languages.** W4d landed. `built_in_translations_are_complete` in `crates/textweaver-lexicon/src/i18n/tests.rs` fails when any language lacks an English id, on purpose ("so no one hears English in the middle of their language"). The plan's step-0 fallback would weaken a deliberate design, so this plan keeps the test: an agent adding a message adds its Spanish, French, German, Portuguese and Arabic forms as ADR-0030 describes (Arabic with its direction marks), and lists them in its report for the owner to spot-check.

## 2. The recalibrated shape

### Agents

- **W5x, the Braille pass** (build; the app-message agent of the first batch): status and title lines, list items and prompts meaning first within 40 cells; the tested Braille section of `docs/screen-readers.md`; PDF page navigation; `y` and `n` in "Open it?" lists.
- **W5y, Star leftovers and settings** (build): library search by DOI, ISBN and author; Star profiles; the lexicon off the input thread; study lists on the list model; settings that do nothing; `[editing] author`; speed presets if time allows. Its messages live in its own files.
- **W5a4, the GUI's third pass** (build, GUI): the Parley upgrade with measurements, the renderer memory options, syllables and difficult words drawn, the voice manager, drawn labels from the catalog.
- **W5r, the rope ADR and loading** (writes first, builds later): ADR-0034 "stay", then load and narration-plan hot spots, then the fuzz feature split.
- **W5c3, LaTeX, EML, MHTML and MathML in HTML** (build).
- **W5c4, math braille and navigation** (build; the app-message agent of the second batch).
- **W5s, summaries and the RSVP flashing check** (light build).
- **W5d, the streaming dictation spike** (light build: one crate, one example).
- **W5g, the publishing templates** (build; pulled from W6g).
- **W5t, automated screen-reader tests** (no build here; CI only).
- **W5p, alpha.5 readiness** (no crate build; xtask and workflows).

### Overlap and order

- **Batch 1, launched together:** W5x, W5y, W5a4 building; W5r writing ADR-0034 (no build); W5t and W5p (no build).
- **Batch 2, as slots free:** W5r's build work takes the first free slot (W5y or W5x merging). W5c3 takes the next. W5c4 launches after W5x merges (one app-message agent at a time). W5s launches after W5y merges (it builds on the study lists' list model).
- **Batch 3:** W5g takes the slot W5c3 frees; it starts after W5c4's writers change is merged or, if earlier, keeps to its own writer files (section 5). W5d runs as the light fourth whenever free memory is at least 32 GB.
- **Merges, in order:** W5x, W5y, W5r, W5c4, W5c3, W5s, W5d, W5a4 (whenever it is ready; it touches only the GUI), W5g, W5t, then W5p last.
- **Non-building agents run throughout:** W5t writes workflows and scripts and runs them on CI runners; W5p builds the GUI release packaging in `release.yml` and writes the readiness page. A research or brief-writing agent for Wave 6 may join in batch 3, since writing costs no memory.

### Where the owner's sessions gate

- **Session B1 (Braille, the Mantis Q40, 40 cells), after W5x merges.** Gates only: W5x's follow-up fixes, W5c4's deliverable 4 (math messages reordered for the display), and the Braille lines W5p adds to the release checklist. Nothing else waits.
- **The rope answer:** the owner reads ADR-0034. "Stay" needs no action; nothing waits on it.
- **Check 2 (by ear and on the display), after W5c4, W5c3 and W5s merge.** Gates nothing in Wave 5; its findings go to Wave 6.
- **Session 3 (the GUI), after W5a4 merges.** Gates W5a4's session fixes (a short follow-up, or W6a5's first items) and the "GUI supported on Windows" line in the alpha.5 notes.
- **The alpha.5 readiness page, after W5p.** The owner decides the release.

### Queued checklists

The orchestrator keeps one file of queued checklists under "Wave 5" in `docs/history/tasks.md`, in this order, so the owner can do them in any sitting: B1 (W5x's five and W5y's two), check 2 (W5c4's five, W5c3's three, W5s's two), session 3 (W5a4's five, W5g's three), and the readiness page. A checklist is queued when its agent merges, never before.

## 3. Research findings

### 3.1 Streaming, low-latency speech-to-text in pure Rust

**Where textweaver is.** ADR-0013 and ADR-0023: Whisper runs in-process on RTen 0.26 with onnx-community's int8 exports (default `base.en`); earshot finds speech in 16 ms frames and splits utterances at pauses of about 600 ms (`vad.rs`: threshold 0.5, 200 ms minimum speech, 200 ms padding); rubato resamples; each utterance is transcribed on its own after Enter; a subprocess Whisper is the fallback. ADR-0013 left "true streaming" for later. Whisper's encoder always takes 30 seconds of audio, so every re-decode pays the full encoder cost; that is the core problem for streaming Whisper.

**Approaches.**

- **LocalAgreement-2 (Whisper-Streaming, Macháček and others, 2023):** re-transcribe a growing buffer every second or so, and commit only the prefix two consecutive runs agree on; trim the buffer at a committed sentence end. About 3.3 seconds latency on long-form speech in the paper, on a GPU with large models. Simple, model-agnostic, needs no new model. [arXiv 2307.14743](https://arxiv.org/html/2307.14743v2), [ufal/whisper_streaming](https://github.com/ufal/whisper_streaming).
- **AlignAtt (SimulStreaming, MIT):** uses the decoder's cross-attention to stop decoding when attention reaches the buffer's end. The authors report it about five times faster than their Whisper-Streaming, and call LocalAgreement "much easier to implement". It needs the attention weights as a graph output, which the onnx-community exports do not give; a re-export would be needed. [ufal/SimulStreaming](https://github.com/ufal/SimulStreaming).
- **VAD-bounded chunks:** what textweaver already does. Combined with LocalAgreement inside each utterance, the buffer never exceeds one utterance, and the pause ends it.
- **Moonshine (Useful Sensors, now moonshine-ai; code and models MIT except legacy non-English non-streaming models under a non-commercial community license):** an encoder that takes variable-length audio, so short buffers cost little. Moonshine v2 (February 2026) uses a sliding-window streaming encoder: Tiny 33.6 M parameters, Small 123.4 M, Medium 244.9 M; average WER 12.0, 7.8 and 6.7 percent; response latency 50, 148 and 258 ms on an Apple M3, reported as 5.8 times faster than Whisper Tiny for Tiny. The streaming checkpoints are published as safetensors (`UsefulSensors/moonshine-streaming-tiny`, 34 M, English, MIT); the non-streaming models have ONNX exports (`onnx-community/moonshine-base-ONNX`, MIT, quantized encoder and merged decoder, the same layout textweaver's Whisper uses). [arXiv 2602.12241](https://arxiv.org/html/2602.12241v1), [moonshine-ai/moonshine](https://github.com/moonshine-ai/moonshine), [the streaming model card](https://huggingface.co/UsefulSensors/moonshine-streaming-tiny), [the ONNX export](https://huggingface.co/onnx-community/moonshine-base-ONNX).
- **Kyutai STT (delayed streams modeling):** true streaming, 0.5 s delay for the 1 B English-French model, a Rust server on candle. At 1 B parameters it is a GPU model; issue reports describe the Rust server as slow from a microphone on CPU. Not suitable. [kyutai-labs/delayed-streams-modeling](https://github.com/kyutai-labs/delayed-streams-modeling/).
- **Parakeet and other Rust wrappers:** `parakeet-rs` (NVIDIA FastConformer, a cache-aware streaming variant with end-of-utterance detection) and `transcribe-rs` (Moonshine, Parakeet, Whisper and others, with a `moonshine_streaming` example) both run on ONNX Runtime through `ort`, which is C++ and not pure Rust; `whisper-stream-rs` wraps whisper.cpp. Useful references for the loop, not dependencies. [parakeet-rs](https://crates.io/crates/parakeet-rs), [transcribe-rs](https://crates.io/crates/transcribe-rs), [whisper-stream-rs](https://crates.io/crates/whisper-stream-rs).

**Versions today.** rten 0.26.0 (August 29, 2026), unchanged; earshot 1.2.2 (August 19, 2026, MIT or Apache-2.0).

**Latency and memory, as far as known.** Whisper `base.en` int8 on RTen transcribed 2.3 s of speech correctly on the owner's Ryzen 5 5600G under load (ADR-0023); a quiet-machine time per re-decode is not recorded. RAM per loaded model is not recorded either. Moonshine's figures are the authors', on Apple silicon with ONNX Runtime, not on RTen.

**Recommendation for W6d.**

1. **Ship live partials on what exists:** inside each earshot utterance, re-decode the growing buffer on a timer (about every 700 ms of new audio), commit with LocalAgreement-2, show and announce committed words (never the unstable tail, which would make a screen reader speak rewrites), and finalize at the pause. No new model, no new dependency. Announcements: committed words only, at most once per commit, through `textweaver_a11y::route`; the tail shown in the status line in screen-reader mode only if the owner wants it.
2. **Add Moonshine on RTen as the low-latency engine** behind the same `Dictation` trait if RTen runs its ONNX graphs and the owner approves the download (about 60 MB for the quantized base pair, unverified). Its variable-length encoder removes Whisper's 30-second cost per re-decode. The streaming (v2) encoder needs an ONNX export first; that is a W6d item, not a spike item.
3. **Not now:** AlignAtt (needs a re-export with attention outputs), Kyutai (GPU-sized), anything on `ort`.

**A spike belongs in Wave 5, small and download-free (W5d):** measure Whisper `base.en` on RTen per re-decode at 1, 2, 4 and 8 seconds of audio, a LocalAgreement-2 loop over recorded WAV fixtures, and the resulting commit latency and RAM. If the owner says yes to question 2, it also loads Moonshine base's ONNX pair in RTen and times it. Its answer is W6d's first page.

### 3.2 MathCAT issue #827

**Open today.** "GetNavigationBraille panics in no-unsafe builds due to cross-document node insertion", opened September 26, 2026; no comments, no linked pull request (a search of the repository's pull requests for 827 finds none). The issue describes the fix: let the recursive MathML copy take the destination document and allocate the whole subtree there. crates.io's newest versions: 0.7.7-alpha.1 (September 23, 2026, before the issue), 0.7.6-rc.3 (August 23, 2026, what W4c1 pins); no 0.7.6 final. GitHub shows a development pre-release built on September 27, 2026, with no notes naming the issue. The workspace still has `unsafe_code = "deny"`. **So W5c4 vendors 0.7.6-rc.3 under `third_party/mathcat` with the fix as a patch,** and re-checks the issue on its first day; if a release fixes it by then, it pins that release instead. [Issue #827](https://github.com/daisy/MathCAT/issues/827), [versions](https://crates.io/crates/mathcat/versions).

### 3.3 Other versions the briefs use

- **Parley 0.11.1** (August 16, 2026), unchanged; W5a4 upgrades the vendored 0.8.0 to it. [crates.io](https://crates.io/crates/parley)
- **Xilem 0.4.0** (October 29, 2025) is still the newest release; the vendored copy stays on its September 14, 2026 main revision. Whether main moved past AccessKit 0.24 is still unverified.
- **AccessKit 0.25.1** (September 25, 2026), unchanged.
- **Guidepup 0.34.0** (published August 31, 2026, "now uses NVDA 2026.2"), unchanged; still no native-app example. Whether it drives a winit window remains W5t's first question. [releases](https://github.com/guidepup/guidepup/releases)
- **mail-parser 0.11.9** as in the plan (not re-checked today).

## 4. Common rules (paste at the top of every brief)

Section 6 of the Wave 4 orchestration plan applies in full, with `wave5/` branches: only the owner overrides rules; privacy; where you work; deleting; building; checks; code rules; **waiting** (every wait loop has a time limit and ends when you report); **generated docs** (`cargo xtask settings-doc` after a settings change, crate counts after a new crate, `cargo xtask docs --check`); merging; dates; the report. Wave 5 adds:

- **Downloads:** crates and tools pinned and digest-checked are approved; `cargo deny` must pass; a new license or advisory, and every machine-learning model, waits for the owner.
- **Messages:** catalog ids in `en.ftl` and all five other languages; list the new ids and their translations in the report.
- **Braille:** every message and status line you add is read once as a 40-cell line: the key fact in the first 40 cells, words not symbols.
- **Checks** add `cargo xtask docs --check` and `cargo xtask settings-doc --check`.
- **Report checklist:** at most five things for NVDA, JAWS and the Mantis Q40, in the plan's order (self-voicing, screen-reader mode with `cursor = "status"`, JAWS, the display, the GUI), or "nothing to hear".
- **Memory:** a light builder (W5s, W5d) starts a build only with at least 32 GB free, and uses `TW_JOBS=2`.

## 5. Briefs

### W5x: the Braille pass (batch 1, P1)

**Branch** `wave5/x-braille`. **ADR:** none. **Merges first.**
**Owns:** `crates/textweaver-tui/src/ui.rs` (status and title line composition); the composition of messages in `crates/textweaver-app/src/{lists,help}.rs` and the position and status messages; the PDF page code in the app's navigation and position report; `docs/screen-readers.md` (the Braille section and the GUI paragraph); `fixtures/x/`; the new ids in the six `.ftl` files.
**Not to touch:** `library.rs`, `study.rs`, `list_model.rs` (W5y's), `opening.rs`, `playback.rs`, `textweaver-xilem`, formats, engines, `textweaver-text`.
**Deliverables, in order, one commit each:**
1. A test that lists every composed status-line, title-line, list-item and prompt string and asserts the key fact sits in the first 40 cells; then the reorder ("Line 12 of 400, Reading"; "3 of 12, Chapter two").
2. The Braille section of `docs/screen-readers.md` for the Mantis Q40: screen-reader mode, `cursor = "status"`, NVDA and JAWS braille settings to try (tethering, message timeout, show messages), each marked "to verify", and a five-item checklist. The GUI paragraph points at `docs/gui.md`.
3. PDF page navigation: "go to" takes a page when the document has `PageBreak` markers; the position report and say status name the page; the outline lists pages when there are no headings.
4. `y` and `n` in the "Open it?" lists after export and citation checks.

**Checks:** the common set; the 40-cell test.
**The owner's checklist (session B1, the Mantis Q40):** (1) open a document in screen-reader mode, `cursor = "status"`: are "Opened" and the title in the first 40 cells? (2) Alt+O: "3 of 12" before each heading's text? (3) a Find prompt: the label, then what you type? (4) Shift+W, then go to page 12 of a PDF: position and page first? (5) the NVDA and JAWS settings in the Braille section: which work?

### W5y: Star leftovers, settings, speed presets (batch 1, P2)

**Branch** `wave5/y-leftovers`. **ADR:** none. **Merges second.**
**Owns:** `crates/textweaver-app/src/{library,study,list_model}.rs` and their messages; `crates/textweaver-store` (library metadata DOI, ISBN, author; `[editing] author`; migrations for removed settings); `crates/textweaver-cli/src/cmd/{library,migrate}.rs`; `crates/textweaver-lexicon` (loading off the input thread only); `docs/library.md`; `fixtures/y/`; its new ids in the six `.ftl` files.
**Not to touch:** W5x's files, `textweaver-xilem`, formats, engines beyond typed engine settings, `textweaver-text`.
**Deliverables:**
1. Library metadata (front matter, DOCX and EPUB metadata, `tw cite`'s record) and `tw library --search` plus the library list's filter on title, path, text, DOI, ISBN and author.
2. Star profiles imported by `tw migrate-star` (Star's `gui/mixin_presets.py` JSON, read only), one report line per profile.
3. The lexicon opened on a helper thread on the first "define word"; "Dictionary still loading" once.
4. Study lists (definitions, statistics) on `ListModel`.
5. Settings that do nothing: `[fonts] fetch_missing` works after a yes or is removed with a migration; `[speech.dectalk]`, `[speech.voice_params]` and the engine sections typed; `[editing] author` in the four places; the "every setting is used" test catches a setting no reader touches.
6. **If time allows (pulled from W6x):** speed presets skim, normal, study and slow, cycled by one key that passes the keymap checks, said as "Study, rate 180".

**Checks:** the common set; settings-doc regenerated.
**The owner's checklist (with B1):** (1) `tw library --search 10.1000/xyz` finds the paper; (2) the first define word says "Dictionary still loading" once, then the list opens; (3) if built, the preset key cycles and says the preset first.

### W5a4: the GUI's third pass (batch 1, P1)

**Branch** `wave5/a4-gui-third-pass`. **ADR:** status updates on ADR-0027, ADR-0028 and ADR-0033; **ADR-0040** only if the upgrade changes the accessibility bridge's design.
**Owns:** `crates/textweaver-xilem` (all), `third_party/xilem/parley` (with `textweaver.patch`, `TEXTWEAVER.md`), `third_party/accesskit_windows` if the bump needs it, `crates/textweaver-xilem/tools/uia-report.ps1`, `docs/gui.md`, `docs/screenshots/xilem-gui/`.
**Not to touch:** `textweaver-app` beyond one-line hooks, `.github/`, workspace members, `third_party/xilem/masonry*` beyond what the bump forces (say what).
**Deliverables, in order:**
1. Baseline on main before any change: the harness numbers (open 10 million characters, highlight move median and worst) and the `--log` memory numbers.
2. Parley 0.8.0 to 0.11.1: the AccessKit bridge into `DocumentView`, ranged styles for the spoken word, bionic and difficult words, edit mode kept (W4a3's `MultilineTextInput` and actions). Measurements repeated; revert to 0.8.0 if one regresses more than 20 percent or a screen-reader action breaks, and say why.
3. ADR-0028's renderer options measured: Vello with area antialiasing only; one wgpu backend on Windows as an opt-in setting. Change a default only with a measurement and after session 3.
4. Syllables and difficult words drawn (Alt+Shift+Z, Alt+Shift+J); the accessible text stays the words.
5. The voice manager on the app's list model, with the download after a yes and the license said.
6. Drawn labels (buttons, dialog titles, status bar prefixes, the settings dialog) from the catalog.

**Checks:** the common set; the GUI crate's clippy and tests; the UI Automation report in `--background` with `-Announce live`, `-Announce uia` and `-WindowEdge`; screenshots at 100 and 200 percent; never the foreground, never audio.
**The owner's checklist (session 3, the display on the caret line):** (1) read past the window edge: caret and display stay on the spoken word? (2) syllables on: NVDA and JAWS still read whole words? (3) the voice manager: a filter row, Enter; (4) the settings dialog in Spanish: labels Spanish, announcements once; (5) edit mode: type, select a word, undo.

### W5r: the rope ADR and loading (batch 1 writing, batch 2 building, P2)

**Branch** `wave5/r-rope-loading`. **ADR-0034** ("The rope after measurement: stay on ropey 1.6").
**Owns:** `crates/textweaver-text`, the load path's hot spots W4b named (report which files before editing any outside `textweaver-text`), `xtask/src/bench.rs`, `docs/dev/testing.md` (measurement section), ADR-0034, a status update on ADR-0002; `crates/textweaver-formats/Cargo.toml` and `fuzz/Cargo.toml` for the feature split only.
**Not to touch:** the state file format, `CharPos`'s meaning, other crates beyond one-line adapters.
**Deliverables:**
1. **Before any build:** ADR-0034 from W4b's numbers: stay on ropey 1.6; revisit when ropey 2.0 is final; why crop's line model and missing char metric weigh against it.
2. The load and narration-plan hot spots, each with a bench number before and after, the bench gate green.
3. The fuzz feature split: the formats crate takes `textweaver-ocr` with `default-features = false`; `ocr` adds `textweaver-ocr/ocrs`; a new feature gives the image and PDF loaders without the engine; the fuzz crate uses it. Merge this before W5c3 starts.
4. Proptests for ADR-0002's invariants if any hot-spot change touches `textweaver-text`.

**Checks:** the common set; the bench gate; `cargo xtask parity` with no unexplained deltas; `cargo fuzz build` in the container.
**The owner's checklist:** nothing to hear; read ADR-0034.

### W5c3: LaTeX, EML, MHTML, MathML in HTML (batch 2, P2)

**Branch** `wave5/c3-latex-email`, after W5r's feature split merges. **ADR-0035.**
**Owns:** `crates/textweaver-formats/src/{latex,eml}.rs` (new) and registry lines; the HTML loader's `<math>` handling only; `fuzz/` targets `latex` and `eml` appended; `fixtures/c3/`; `docs/converting.md` (file list, two sections); one open-failure id per format in the six `.ftl` files; dependency block "W5c3" (mail-parser 0.11.9 with `encoding_rs`, version re-checked on launch).
**Not to touch:** other loaders, `textweaver-math` (call `parse_latex_math`), `textweaver-app` beyond the ids.
**Deliverables:** as the Wave 5 plan's W5c3 brief, items 1 to 6 (EML text first; MHTML per RFC 2557 with caps; the LaTeX subset with caps, `\input` inside the folder only, unknown commands read as text; hostile-input tests and two fuzz targets; `tw convert` and `tw text` on every fixture; ADR-0035), then **MathML in HTML:** `<math>` becomes the `Math` marker as W4c1's EPUB path does.
**Checks:** the common set; `cargo fuzz build` and ten minutes of each new target in the container; parity unchanged.
**The owner's checklist (check 2):** `fixtures/c3/notes.tex` by `h`, `t`, and a formula; `message.eml`, headers then body; `page.mhtml` reads as the page.

### W5c4: math braille and navigation on MathCAT (batch 2, P1)

**Branch** `wave5/c4-math-braille`, after W5x merges. **ADR-0036.**
**Owns:** `crates/textweaver-mathcat`; `third_party/mathcat/` (0.7.6-rc.3, `textweaver.patch`, `TEXTWEAVER.md`); `crates/textweaver-writers/src/{brf,math}.rs` (the `Math` marker's braille only); `crates/textweaver-app/src/math_explore.rs` and its messages; `[braille] math_code = "nemeth" | "ueb"`, default `"nemeth"` (four places); `docs/math.md`; the braille lines of `docs/converting.md`; `fixtures/c4/`.
**Not to touch:** `textweaver-math`'s parsers, the EPUB loader, `textweaver-xilem`, the BRF text tables, W5g's writer files.
**Deliverables:**
1. Day one: re-read #827. Fixed in a release: pin it. Still open: vendor, apply the described fix (the copy allocates the subtree in the destination document), keep `unsafe_code = "deny"`, add the regression test, and put the upstream pull request text in the report for the owner to file (question 3).
2. BRF math in Nemeth and UEB, wrapped to 40 cells without separating an indicator from its symbol; the fallback (feature off, parse failure) is today's spoken text in grade 1, said once in the summary. Snapshots checked against published examples, sources cited.
3. Navigation behind `[reading] math_engine = "mathcat"`: Alt+Shift+X, arrows and Enter mapped to MathCAT's navigate calls, each step's speech once and its braille on the status line; `"builtin"` unchanged (the owner's default).
4. After session B1: math messages reordered where the display showed a problem.
5. ADR-0036: the patch, the codes, the fallback, the size added.

**Checks:** the common set; `--features mathcat` natively and in the container; BRF snapshots; `cargo deny`; binary size before and after.
**The owner's checklist (check 2):** (1) `tw convert fixtures/c4/quadratic.md --to brf` on the display: Nemeth? (2) Alt+Shift+X with `"mathcat"`: Right through terms, braille shown and speech once; (3) the same with `"builtin"`: which is clearer? (4) `math_code = "ueb"`: the BRF changes and the summary names the code; (5) an unparsable formula: the fallback said once.

### W5s: summaries and the RSVP check (batch 2, light)

**Branch** `wave5/s-summaries`, after W5y merges. **ADR-0037.**
**Owns:** `crates/textweaver-summary` (new), `crates/textweaver-cli/src/cmd/summarize.rs`, `crates/textweaver-app/src/summary.rs` (new) and its ids, `[summary] sentences` and `[reading_aids] difficult_definitions` (four places each), the definitions hook and the RSVP data test in `crates/textweaver-aids`, `docs/reading.md` (one section), `fixtures/s/`, crate counts.
**Not to touch:** `textweaver-lexicon` beyond its API, `textweaver-text`, `textweaver-xilem`.
**Deliverables:** as the plan's W5s items 1 to 5 (LexRank in-house, under 200 ms on 10 MB; `tw summarize`; the palette list with jump; definitions at high verbosity, off by default; ADR-0037), then the RSVP flashing test (WCAG 2.3.1: never more than three flashes a second on one region at the highest rate) with a cap if needed.
**Checks:** the common set; ranges inside and ordered; the 40-run rule.
**The owner's checklist (check 2):** `tw summarize fixtures/sample.md`; Summarize in the reader, Down, Enter: lands on the sentence and says it.

### W5d: the streaming dictation spike (batch 2 or 3, light)

**Branch** `wave5/d-stream-spike`. **ADR:** none (ADR-0042 reserved for W6d).
**Owns:** `crates/textweaver-dictation/examples/stream_probe.rs` (new), `fixtures/d/` (recorded WAV made with textweaver's own TTS export, no microphone), `docs/research/streaming-dictation.md` (new).
**Not to touch:** the crate's library code, the app, settings. **No downloads.** Use the Whisper model only if already in `<data>/whisper/rten/base.en`; if absent, stop and say so. Never open the microphone; never play audio.
**Deliverables:**
1. Encoder and full-decode time for `base.en` on RTen at 1, 2, 4 and 8 seconds of audio, 40 runs, median and worst, with RAM (working set and private bytes).
2. A LocalAgreement-2 loop over the fixtures inside earshot utterances: commit latency from word end to commit, rewrites avoided, final text against the batch transcript.
3. Only if the owner says yes to question 2: Moonshine base's ONNX pair loaded in RTen, the same measurements, and which operators failed if any.
4. The research page: the numbers, a recommended design for W6d, and what announcing committed words would sound like.

**Checks:** fmt, clippy on the example, the crate's tests with `--features rten`.
**The owner's checklist:** nothing to hear; read the page.

### W5g: the publishing templates (batch 3, P2, pulled from W6g)

**Branch** `wave5/g-templates`. **ADR-0041.**
**Owns:** `crates/textweaver-writers/src/{docx,epub,model}.rs` and `pdf/` (templates, real Word footnotes, PDF page labels from print page breaks), new template assets under `crates/textweaver-writers/templates/` (APA student paper, AMA manuscript, EPUB cover, the large print, dyslexia-friendly, high contrast and manuscript stylesheets), `tw convert --template` in `crates/textweaver-cli`, `docs/converting.md`'s templates section (after W5c3 merges), `fixtures/g2/`.
**Not to touch:** the BRF writer (W5c4's), loaders, the app beyond a template setting (four places) if needed.
**Read first:** Star's templates (read only), ADR-0017, ADR-0016.
**Deliverables:** the four stylesheets; the APA and AMA DOCX templates with real footnotes and styles screen readers announce; the EPUB cover with alt text; PDF page labels; epubcheck and veraPDF (`second-tool.yml`) green on the new output; ADR-0041.
**Checks:** the common set; the second-tool workflow on the branch.
**The owner's checklist (with session 3):** (1) an APA paper exported and opened in Word with JAWS: title page, headings, a footnote; (2) the same in textweaver; (3) the large print EPUB's cover alt text.

### W5t: automated screen-reader tests (batch 1, no build here)

**Branch** `wave5/t-a11y-tests`. **ADR-0039** only if a check becomes standing.
As the Wave 5 plan's W5t brief, unchanged: the accessibility-cli tree dump on three systems pinned by commit with a diff in the job summary; the Guidepup question on a Windows runner (0.34.0, NVDA 2026.2); Orca under Xvfb without Collection (AccessKit PR #758); VoiceOver on `macos-14` if `@guidepup/setup` can enable it. **Never drive NVDA or JAWS on this machine.** Owns `tools/a11y/`, `tools/atspi-dump.py`, `.github/workflows/{gui-xilem,a11y-tests}.yml`, a section of `docs/dev/testing.md`. Not `release.yml`, `ci.yml`, `nightly.yml`. Runner time is sequenced with W5p's release dry run through the orchestrator.
**The owner's checklist:** nothing to hear; compare the recorded NVDA phrases with what you heard.

### W5p: alpha.5 readiness (batch 1 to the end, no crate build)

**Branch** `wave5/p-alpha5-readiness`. **ADR:** none. **Merges last.**
**Owns:** `.github/workflows/{release,ci,nightly}.yml`, `xtask/src/{release,dist}.rs`, `docs/dev/releasing.md`, `docs/install.md` (GUI packages, unsigned notes), `CHANGELOG.md`'s Unreleased section, the readiness page.
**Deliverables:**
1. The GUI packages in `release.yml` (the owner: supported, Windows first; macOS and Linux "built and tool-checked, not yet heard"): `cargo xtask gui-dist` in the package jobs, uploads, checksums, attestations.
2. A Release dry run (`workflow_dispatch`, no tag) that tests the aarch64 AppImage for the first time.
3. After session B1: its five items added to the listening checklist, so `cargo xtask release` dates them.
4. The alpha.5 notes grouped by area, "Keys: what changed" first.
5. The readiness page: packages built, checklists dated, docs checks green, nightly fuzz clean seven days, open issues.

**Checks:** actionlint, shellcheck, the PowerShell lint, the docs checks, `cargo xtask release --dry-run`.
**The owner's checklist:** nothing to hear; read the readiness page.

## 6. Budget, runbook, risks

### Disk and memory

Measured today at 13:37: D: 784.3 GB free; host RAM 63.8 GB with 38.9 GB free.

- Per building agent: 37 GB (the roadmap's measured allowance), GUI included; light builders (W5s, W5d, W5r while writing): under 24 GB, native only; non-builders (W5t, W5p): under 5 GB.
- Peak (batch 2): three builders 111 GB, two light builders 48 GB, two non-builders 10 GB, orchestrator 40 GB, caches 60 GB: about 270 GB, leaving about **515 GB free**, two and a half times the 200 GB floor.
- Memory: three containers at 6 GB (18 GB). With today's 38.9 GB free that leaves about 21 GB, just above the 20 GB floor, so the fourth slot opens only at 32 GB free. The orchestrator's native check runs with at most one container building.
- Models: none downloaded unless the owner says yes to question 2 (Moonshine base, about 60 MB, unverified).

### Runbook

1. **Step 0 (while the docs sweep finishes):** write the branches taken (section 1) under a "Wave 5" heading in `docs/history/tasks.md`; confirm the next free ADR number after the sweep (0034 W5r, 0035 W5c3, 0036 W5c4, 0037 W5s, 0038 reserved W6e, 0039 W5t, 0040 W5a4 reserved, 0041 W5g, 0042 reserved W6d); record the message rule; ask the three questions.
2. **Step 1:** `build-hygiene.ps1`: at least 400 GB free, no `tw-target-w4*` volumes; free memory at least 32 GB with Docker idle; the caches warm.
3. **Step 2, batch 1:** worktrees and volumes for W5x, W5y, W5a4 (building), W5r (writing), W5t and W5p (none). Log D: and memory twice a day.
4. **Step 3, integrate and refill:** each report checked for sizes and memory; native checks; the container all-features check one at a time; merge; regenerate site and notices; flush that agent (volume, then folder, each listed then applied); one line to the owner; start the next queued agent in the freed slot (W5r's build, W5c3, W5c4, W5s, W5g, W5d, in that order of priority).
5. **Flush points:** after every merge, not only at sub-wave ends; Docker disk compacted with the owner if it passes 300 GB and after the wave.
6. **Close:** session 3 and check 2 written up; W5p's readiness page to the owner; results to the wiki page by the orchestrator; Wave 6 briefs drafted from W5d's page, ADR-0034 and the session findings.

### Risks

- **The catalog rule slows message agents.** Mitigation: translations are short and W4d's patterns exist; the owner spot-checks listed ids.
- **The Parley upgrade regresses.** Revert to 0.8.0 (the rule in W5a4); edit mode on 0.8 is enough for the final alpha.
- **The MathCAT patch diverges from upstream's fix.** Keep the patch minimal and re-check the issue before each merge of main.
- **Memory:** three builders leave little headroom today; an out-of-memory kill drops that agent to `TW_JOBS=2`, then serializes.
- **Overlapping merges in `textweaver-app` and the `.ftl` files.** One message agent at a time (W5x, then W5c4); W5y and W5s keep to their own files and append their ids in their own labeled block.
- **Runner contention** between W5t's workflows and W5p's dry run: sequenced by the orchestrator.

### Questions for the owner (work never waits on them)

1. **Pull W6g (publishing templates) and W6x (speed presets) into Wave 5's free slots?** Default: **yes**; both are on the feature-complete list, and the slots are free because W5m and W5e left.
2. **May W5d download Moonshine base's ONNX pair (MIT, about 60 MB, unverified) to test it on RTen?** Default: **no in Wave 5**; the spike measures Whisper, and W6d asks again with the size and license read from the model card.
3. **Who files the MathCAT fix upstream?** Default: **W5c4 writes the pull request text; you file it from your account when you choose,** since it publishes under your name.

## See also

- [Wave 5 plan](wave5-plan.md): the briefs this file recalibrates.
- [The 2026 roadmap](roadmap-2026.md): the feature-complete list and the owner's answers.
- [What is left](whats-left.md): the inventory.
- [Wave 4 orchestration plan](wave4-orchestration.md): the common rules in section 6.
- [Cloud Agent reservations](../history/reservations.md): what was struck.
- [Tasks and agent briefs](../history/tasks.md): where the adopted plan goes.
- [Dictation](../dictation.md) and [ADR-0023](../adr/0023-in-process-neural-speech.md): where streaming dictation starts.
- [Documentation index](../README.md)
