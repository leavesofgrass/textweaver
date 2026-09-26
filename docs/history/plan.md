# textweaver — implementation plan (4-subagent execution)

> Approved by Jon on 2026-09-25 (Friday). Approval started Phase 0 only; each later wave is reported before the next begins. Phase 0 amendments are recorded at the end of this document ("Phase 0 amendments") and in the ADRs; where they differ, the amendments and ADRs win.

> **Status update (Saturday, September 26, 2026).** This plan is kept as it was approved. Phase 0, Wave 1, and Wave 2 are done and merged on `main`; releases `v0.1.0-alpha.1` to `v0.1.0-alpha.3` are tagged. Wave 2 grew well beyond the four agents in section 7: formats and PDF, speech and audio export, state and library, app wiring, the shared engine host, the Obsidian vault and dictation, a GUI spike, rendering and bulk conversion, native writers, math, citations, themes, reading aids, DECtalk, settings import and export, scripts, bundled fonts, and an audit with its fixes. `docs/history/tasks.md` is the record of who did what. The `paperback` feature in section 6.2 is reserved and does nothing, because PDF is read natively ([ADR-0010](../adr/0010-pdf-loader.md)). Wave 3 (the GUI) has not started. For the system as built, read [the architecture guide](../dev/architecture.md) and the [documentation index](../README.md).

## 1. Context

Star (`leavesofgrass/star`, 45K lines of Python, v0.1.31) is a GUI-first TTS document reader and Markdown authoring tool for students with print disabilities. It works, but carries the costs of its history: a Qt GUI and a curses TUI split into 40+ mixins, a speech layer whose word-highlight sync depends on difflib-aligning two separately generated text streams, a regex normalization chain that cannot preserve offsets, a settings file rewritten on every change, and a long tail of optional Python dependencies.

textweaver is a Rust-native reimplementation of Star's core: read documents aloud with precise, keyboard-driven navigation and live highlighting; write with speech feedback; run fast on large documents; be first-class for screen-reader users. Paperback (Rust, MIT) is the reference for accessibility and document-handling quality. Omnivox (Rust, MIT) is the reference for a queued, multi-stream speech server and becomes one supported backend.

**Outcome:** a new repository with a clean Cargo workspace, ADRs, CI, and a first release that compiles and runs: a `tw` CLI and a self-voicing terminal UI that opens text, Markdown, and HTML documents, reads by character, word, sentence, paragraph, selection, and document with synchronized highlighting, restores position, supports bookmarks, find, history, and configurable shortcuts, and edits with speech feedback. Later waves add formats, a native GUI, and the parity extras selected.

## 2. Decisions already made

| Decision | Choice |
|---|---|
| Location | New repository `leavesofgrass/textweaver` (public, GPL-3.0-or-later like Star), created via the GitHub tools, attached with push access, cloned beside `star`. |
| UI staging | Terminal UI (ratatui + crossterm) first over a UI-agnostic app core; native GUI later via wxDragon (wxWidgets), the toolkit Paperback validated with NVDA, JAWS, and VoiceOver. |
| Formats | Own `Document` model and loader trait; built-in txt, Markdown, HTML, EPUB, DOCX; optional cargo feature `paperback` wrapping `paperback-core` for PDF and the long tail; optional `pandoc` subprocess loader. |
| Parity roadmap after core | Voice typing / dictation; audio and document exports; library and full-text search (incl. Obsidian vault import/export). Study tools are out of scope for now. |
| Execution | The orchestrating session writes the interface contract, briefs and merges four Opus subagents working in parallel git worktrees with disjoint crate ownership. |
| Development environment | Docker container (`docker/Dockerfile`, `compose.yaml`) for Linux builds and tests on Jon's Windows machine with Docker Desktop; native Windows builds alongside (added by Jon at approval). |
| Documentation | Everything documented in the Obsidian wiki (`D:\star\wiki`, domain page `textweaver`) as well as in `docs/` (added by Jon at approval). |

**Assumptions:** Rust edition 2024 on current stable (1.96 at Phase 0, pinned in `rust-toolchain.toml`); `tw` is the CLI binary and `textweaver` the TUI binary; US English strings with an i18n hook, no catalogs yet.

## 3. What we preserve from Star, and what we fix

Inventories were taken of `star/documents/`, `star/tts/`, `star/ttstext/`, `star/gui/`, `star/tui/`, `star/settings.py`, `star/library.py`, `star/annotations.py`, `star/sync.py`, `star/search.py`, `star/fulltext.py`, and `docs/`. They are `docs/history/star-parity.md` and are handed to the subagents as reference.

**Preserve:**
- Reading units: continuous from a word; sentence next / previous / replay (previous rewinds to the sentence start when more than 3 words in); paragraph; heading (read-aloud and scroll-only); table; chapter; Speech Cursor line mode saying "blank" on empty lines.
- Highlight pacing from `star/tts/manager/_playback.py`: generation bumped before every stop or restart; pause records the last callback-confirmed word (resume may repeat, never skips); timer estimate `60 / (wpm × highlight_speed)` with clamp guards (`_CB_TIMEOUT` 1.5 s, `_CB_DEAD` 6 s, `_MAX_AHEAD` 1 paced / 4 unpaced); lead / lag; word / sentence / both granularity.
- espeak-ng from `star/tts/espeak.py`: word events scheduled by `audio_position` plus a latency offset (default 120 ms), never fired on arrival; UTF-8 byte-to-char mapping; sentence-sized chunks.
- Backend selection: explicit preference, else priority-ordered auto-detect skipping opt-in backends, null as final fallback.
- Normalization semantics from `star/ttstext/` with the exact strings in `tests/test_ttstext.py`.
- Position resolution: save char offset of the current word plus floored pct; restore to the first word at or after; newest timestamp wins between sidecar and local; history truncates forward entries on a new jump.
- Settings surface (backend, rate 265 wpm, volume, voice, favorites, auto-resume, skip code, speed presets, normalization toggles, highlight options, `nav_history_size` 50, wrap, tab width, theme).
- Keyboard vocabulary: GUI map (Space, Escape, Ctrl+Space, Ctrl+= / Ctrl+-, Alt+. Alt+, Alt+;, Ctrl+P / Ctrl+Shift+P, Ctrl+H, Ctrl+T, Ctrl+M, Ctrl+F, Alt+Left / Alt+Right, Ctrl+E, Ctrl+S, Ctrl+N, Ctrl+O, F2, F3, Tab) and TUI single keys (`.` `,` `;` `[` `]` `{` `}` `<` `>` `t` `T` `n` `N` `H` `L` `j` `k`).
- Announcements on every state change; accessible names everywhere.
- Autosave recovery; Markdown formatting commands as single undo steps; save in place for text formats, save-as-Markdown otherwise.
- CLI scripting: extraction, voices, backend diagnostics, batch conversion.

**Fix, don't copy:** two text streams aligned with difflib; offset-less normalization; `on_done` without a generation check; inconsistent history recording; `search-backward` searching forward; TUI Alt keys eaten by ESC; `footnote_mode = inline` no-op; settings rewritten on every `set`. **Add what Star lacks:** pitch, punctuation verbosity, typing / character / word / deletion echo, read current character / word / selection, link and list navigation, configurable TUI keys.

## 4. What we take from the references

**Paperback:** buffer-plus-markers document with index tables incl. the UTF-16 display-unit table (`document/buffer.rs`, `marker.rs`); marker navigation with wrap and level filters (`reader_core/navigation.rs`); pure history (`reader_core/history.rs`); `ActionId` + `KeyChord` + TOML overrides keyed by action (`config/shortcuts.rs`); per-document state remapped on edit (`config/manager/document_state.rs`); `live-region` announcements; 500k-unit window slicing (`session/window.rs`); the `pb` CLI shape. Not its UI, FFI, or parsers (except through the optional feature).

**Omnivox:** queue of `Speech | Tone | Silence | AudioIcon`; generation counter checked before and after synthesis; independent speech / tone / sound streams; punctuation levels; split caps; character speaking at a scaled rate with pitch raised for capitals; the stdin protocol we drive. Constraints: the protocol is write-only (nothing on stdout) and `synthesize` returns a buffer without word timing, so the subprocess backend gets no events and word-level highlighting with Omnivox is not promised.

## 5. Workspace layout and ownership

```
textweaver/
  Cargo.toml  rust-toolchain.toml  .github/workflows/ci.yml   (orchestrator)
  docker/Dockerfile  compose.yaml                             (orchestrator)
  crates/
    textweaver-core/      shared leaf types: CharPos, Unit, OffsetMap, Edit/EditOutcome, Rate, Pitch, errors   (orchestrator)
    textweaver-text/      Document (rope + markers + index), units, navigation, history, search, narration     (Agent A)
    textweaver-formats/   Loader trait, registry, loaders, cache                                              (Agent A)
    textweaver-speech/    SpeechBackend, SpeechService, pacing, queue, normalization, backends                (Agent B)
    textweaver-store/     settings, keymap file, per-document state, sidecar merge, recent, library           (Agent C)
    textweaver-keymap/    ActionId, KeyChord, Keymap, help/doc generation                                     (Agent C)
    textweaver-a11y/      Announcer trait, verbosity, speech + status-line announcers                         (Agent C)
    textweaver-editor/    undo/redo over a Rope, Markdown ops, autosave policy, echo events                   (Agent C)
    textweaver-app/       AppState, Session, Command dispatch, effects                                        (Agent D)
    textweaver-tui/       ratatui frontend, binary `textweaver`                                               (Agent D)
    textweaver-cli/       `tw`: main.rs + cmd/ (text.rs info.rs search.rs → A; speak.rs voices.rs backends.rs → B; marks.rs migrate.rs → C; open.rs serve.rs → D)
  xtask/                  keyboard.md + star-parity generation, fixtures, release                             (C for keymap docs, A for parity corpus)
  tools/                  star_parity_export.py (one-off Star export)                                         (orchestrator)
  docs/adr/  docs/history/plan.md  docs/history/star-parity.md  docs/keyboard.md  docs/history/tasks.md  docs/dev/docker.md             (orchestrator; agents append)
  fixtures/                                                                                                   (A creates; all may add under their own subdir)
```

Dependency direction (no cycles): `core` ← `text`, `speech`, `keymap`, `a11y`, `store`, `editor` ← `formats` (on `text`) ← `app` ← `tui`, `cli`. `speech` depends only on `core` (takes `Utterance { text, offset_map }`, never a `Document`). `editor` works on `ropey::Rope` plus `core::Edit`; `text::Document::apply(Edit)` is where the document side lives. This is what lets four agents build simultaneously.

Dependency policy: one crate per job; engines and heavy formats behind features; no async runtime; `thiserror` in libraries, `anyhow` in binaries. Approved crates are declared once in the workspace `[workspace.dependencies]` table (ADR-0001).

## 6. Core designs (the contract each agent builds against)

The Phase 0 code in each crate is the contract; this section summarizes it. See the ADRs for the reasoning.

### 6.1 Text model (`core`, `text`) — ADR-0002, ADR-0005

- `Document { meta, text: Rope, markers: Vec<Marker>, display: OnceCell<DisplayIndex> }`; rope gives char ↔ byte ↔ line; `DisplayIndex` (char → UTF-16) built lazily for the GUI.
- `CharPos(usize)` is canonical and persisted. Canonical text: paragraphs separated by a blank line, headings and list items as bare lines, tables row-per-line under a `Table` marker, images as alt text, code as text under a `Code` marker.
- `Marker { kind, range, level, label, reference }`; `MarkerIndex` for next / previous with wrap and level filter; `shift(&EditOutcome)`.
- Units: Grapheme, Word (UAX #29, alphanumeric-bearing; parity-tested vs `\b\w[\w'-]*`), Sentence (UAX #29 + abbreviation list; parity-tested vs Star's regex, differences documented), Paragraph, Line, marker units.
- Pure `navigate`, `go_to`, `history`, `search` (plain / regex, case, whole word, wrap, direction), and `narrate::plan(&Document, range, policy) -> Vec<Utterance>` (sentence-sized; table narration and heading labels as `Inserted` spans).
- `OffsetMap`: sorted `Span { spoken: Range<u32 bytes>, source: CharRange, kind: Literal | Expanded | Inserted | Elided }`, binary search, composition; invariants checked by `check_invariants`; property-tested.
- Layout and the future GUI work on a window slice from day one; positions stay document-absolute.

### 6.2 Loaders (`formats`)

`trait Loader { id, extensions, available, priority, load(&Source, &LoadOptions) -> Result<Document> }`, `Registry` by priority, `Source` = path | bytes-with-hint | URL. Built-ins: text, markdown (pulldown-cmark events → text + markers), html (scraper; Star's skip list and alt rules, without Star's unclosed-`<meta>` bug), epub, docx. Features `paperback` and `pandoc`. Cache keyed by `(path, mtime, size, options fingerprint)`.

### 6.3 Speech (`speech`) — ADR-0003, ADR-0004

`SpeechBackend` has no `Send` bound; the service owns a speech thread and builds the backend there from a `Send` factory. `speak` returns without waiting for audio; later events come from `poll`, which the service calls while speech is active. `Caps` bitflags (`WORD_EVENTS | AUDIO_CLOCK | PAUSE | PITCH | VOLUME | LIVE_RATE | SSML_MARKS | SYNTH_TO_FILE | TONES | REQUIRES_MAIN_THREAD`). `Utterance { id: UtteranceId { generation, chunk }, text, kind, offset_map }` lives in core. Raw events `Started | Word { byte_range, audio_ms } | Finished | Cancelled | Error`; the sink drops stale generations. `SpeechService` API: `say(text, Interrupt | Queue | Announce)`, `read(Vec<Utterance>)`, `stop`, `pause`, `resume`, `skip`, `set_rate / pitch / volume / voice`, `set_punctuation`, `set_split_caps`, `speak_char`, `tone`, `earcon`; emits `SpeechStatus { Position { utterance, source_range }, Paused { resume_at }, Stopped, Finished, BackendError }`. Normalization is a `Transform` chain composing `OffsetMap`s per utterance. Backends: `null`, `recording`, `espeak` (feature), `omnivox` (feature), then `speechd`, `tts-crate`, native, piper, cloud.

### 6.4 Persistence (`store`)

Paths via `directories` (or `TEXTWEAVER_HOME`); `settings.toml` and `keymap.toml` written only on explicit change, atomically, unknown keys preserved; `state/<doc-key>.json` per document; `recent.json`; sidecar `.textweaver/progress.json` with Star's merge policies; `tw migrate-star` (wave 2).

### 6.5 Keymap and accessibility (`keymap`, `a11y`) — ADR-0006

`ActionId` with category, help, and defaults per frontend and platform; `KeyChord` parse / format / normalize; `Keymap` = defaults + overrides keyed by action id, looked up by `Layer` (Global, Browse, SpeechCursor, Edit); conflict detection; `docs/keyboard.md` generated by `cargo xtask keyboard`. `Announcer` trait with `SpeechAnnouncer`, `StatusLineAnnouncer`, `LogAnnouncer`; `Verbosity { Low, Normal, High }` (in core).

### 6.6 Editing (`editor`)

Undo / redo of grouped `core::Edit`s over a `Rope`; Markdown ops as pure `(text, selection) -> edits`; `EchoPolicy` and `EchoEvent`; `AutosavePolicy` (20 s while dirty); save-in-place vs save-as rule.

### 6.7 App and TUI (`app`, `tui`)

`App` is the only owner of mutable state; `dispatch(Command) -> Vec<Effect>` testable without a terminal; `poll_speech()` drains speech status. TUI: ratatui + crossterm; title, viewport over a window slice, status, key hints, minibuffer; hardware cursor parked on caret or spoken word; highlights for spoken unit, find, marks; themes; F2 palette; help.

### 6.8 CLI (`cli`, `tw`)

`tw open FILE`, `tw text FILE [--format text|markdown|json] [--structure]`, `tw info FILE`, `tw search FILE PATTERN [--regex]`, `tw speak [TEXT | --file F] [--backend --voice --rate --pitch] [--out out.wav]`, `tw voices`, `tw backends`, `tw marks FILE`, `tw migrate-star`, `tw serve --stdio` (wave 2). `--json` on every read-only command.

## 7. Execution model: one orchestrator, four Opus subagents

### Phase 0 — Contract (orchestrator, sequential)

1. Create `leavesofgrass/textweaver`, clone beside `star`; build the Docker development image.
2. Workspace skeleton: every crate exists and compiles; shared lints; CI (fmt, clippy `-D warnings`, test, doc on ubuntu / macos / windows).
3. `textweaver-core` complete and tested.
4. Contract stubs in each owned crate, `todo!()`-free.
5. ADRs 0001–0006, `docs/history/star-parity.md`, `docs/history/tasks.md`, fixtures, `fixtures/star-parity/`.
6. Push `main`. Report to Jon; **Wave 1 starts only after that report**.

### Wave 1 — four subagents in parallel (M1–M3 scope)

See `docs/history/tasks.md` for the briefs, ownership, and acceptance criteria.

### Integration 1 (orchestrator)

Merge A → B → C → D, resolve contract change requests, run the full matrix natively and in Docker, fix seams, tag `v0.1.0-alpha.1`, report with the parity report and known gaps. **Wave 2 starts only after that report.**

### Wave 2 — four subagents in parallel (M4–M5 scope)

| Agent | Scope |
|---|---|
| **A** | EPUB and DOCX loaders; `paperback` feature; `pandoc` feature; exports to Markdown / HTML / text; full-text index |
| **B** | `speech-dispatcher` backend with index marks; `tts`-crate backend; `synthesize_to_file` and a capture wrapper; audio export (WAV, ffmpeg, SRT / VTT, M4B chapters); omnivox in-process option |
| **C** | Notes and highlights with remapping across edits; library folders and recent; `tw migrate-star`; dictation trait with a whisper subprocess backend; Obsidian vault import / export |
| **D** | Edit mode in the TUI; notes and highlights UI; `tw serve --stdio` JSON-RPC; expanded scripted tests |

Integration 2, tag `v0.1.0-beta.1`, report.

### Wave 3 (after Jon reviews the beta)

GUI on wxDragon (with the `live-region` announcer), native word-boundary backends for Windows and macOS, batch convert and hot-folder watch, Star parity audit and docs.

## 8. Testing strategy

- Unit tests in every crate; `proptest` for segmentation, OffsetMap composition, history, marker shifting, editor sequences.
- `insta` snapshots for loader output and `tw text --format json`.
- Speech: `recording` backend plus fake clock; omnivox adapter against a fake process.
- TUI: ratatui `TestBackend` with scripted key events, assertions on the rendered buffer and the announcer log.
- Parity corpus in `fixtures/star-parity/` exported once from Star.
- CI on three OSes; `cargo doc --no-deps`; the same checks in the Docker image locally.

## 9. Documentation

- ADRs 0001–0006; later: PDF loader choice, GUI toolkit, native backends.
- `docs/history/plan.md`, `docs/history/star-parity.md`, `docs/keyboard.md` (generated), `docs/history/tasks.md`, `docs/dev/docker.md`.
- Rustdoc on every public item (`missing_docs` is a warning and CI denies warnings).
- The Obsidian wiki: domain page `textweaver`, one session note per phase and wave under `meta/textweaver-releases/`, and concept notes for the designs worth keeping (OffsetMap, speech threading).
- In `leavesofgrass/star`, `docs/textweaver.md` on branch `claude/textweaver-plan-m42jiy` pointing to the new repo and this plan (pushed in Phase 0, commit `06a5995`; not merged into star's `main`).

## 10. Verification (per wave, before reporting)

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo doc --workspace --no-deps` natively on Windows and in the Docker image.
- `tw text fixtures/sample.md`, `tw info fixtures/sample.html`, `tw backends`, `tw speak --backend null --json "test"`.
- The scripted TUI test in the Agent D acceptance row.
- Manual on a desktop with audio (Jon): `tw speak --backend espeak`, `textweaver README.md` with `docs/keyboard.md`; with Omnivox installed, `tw speak --backend omnivox`.

## 11. Risks

- Engines differ in word-boundary support; the timer pacer must be acceptable for Omnivox and the `tts` crate.
- `espeakng-sys` links the system library; Windows and macOS rely on native backends.
- wxDragon compiles wxWidgets; the first GUI build is slow and not testable in the container.
- Parallel agents can drift from the contract; mitigated by Phase 0 stubs, ownership boundaries, and change requests resolved only by the orchestrator.
- Sentence segmentation parity with Star's regex is approximate by design; the parity report shows the deltas.
- The `paperback` feature pulls pdfium and mathcat; off by default.

## Phase 0 amendments

Decisions taken while writing the contract. Each is reflected in the code and the named ADR.

1. **Environment.** Phase 0 ran on Jon's Windows 11 machine, not a Linux container. Linux builds use the Docker image (`docker/Dockerfile`: Rust 1.96 on Debian trixie with espeak-ng 1.52, ALSA, speech-dispatcher, clang, Python 3.13, pandoc). Toolchain pinned to 1.96 (ADR-0001).
2. **`live-region` is GUI-only.** The crate depends on wxDragon, so it cannot be part of the terminal build. The TUI announces through speech and its status line; the live-region announcer arrives with the GUI in wave 3 behind the `textweaver-a11y/live-region` feature (ADR-0006).
3. **`Utterance`, `UtteranceId`, `UtteranceKind`, `MarkerKind`, and the preference enums live in core** (`Verbosity`, `PunctuationLevel`, `HighlightGranularity`, `CapsIndication`), so `text` can produce utterances, `speech` can consume them, and `store` can hold settings, without cross-dependencies (ADR-0001).
4. **Backend timing contract.** `speak` returns without waiting for audio; events after that arrive through `SpeechBackend::poll`, which the service calls while speech is active. This is how `stop` and `pause` reach a backend mid-utterance on the same thread (ADR-0003).
5. **Rate and pitch serialize as plain numbers** (`rate = 265`, `pitch = -2`). `Pitch` has one variant, `Semitones(i8)`; zero is the voice default. Volume is a percentage `0..=100`; Star's `0.0..=1.0` float is multiplied by 100 on migration (ADR-0004).
6. **Canonical text differs from Star's.** Star's `plain_text` joins every single newline into a space (plain text included) and runs list items and table cells together. textweaver keeps line structure (list items and table rows one per line, plain-text lines preserved), which Speech Cursor line mode and line navigation need. Consequence: Star's saved character offsets do not map one-to-one; `tw migrate-star` maps positions by aligning Star's word sequence with textweaver's (ADR-0002).
7. **Keymap layers.** Bindings belong to a layer (Global, Browse, SpeechCursor, Edit). Browse-layer single keys are shared by both frontends; terminal chords avoid keys terminals cannot distinguish (`Ctrl+H`, `Ctrl+I`, `Ctrl+M`) (ADR-0006).
8. **Settings format.** Sectioned TOML (`[speech]`, `[highlight]`, `[normalization]`, `[reading]`, `[display]`, `[editing]`, `[library]`), with unknown keys preserved in every table.
9. **Document keys.** One key for every per-document store: file name plus a 64-bit FNV-1a hash of the absolute path (Star used three different keys).
10. **Eloquence (added during Wave 1 at Jon's request).** A fifth Wave 1 agent (E) builds `textweaver-eci`: ETI-Eloquence through its ECI library in a separate host process (32-bit on Windows), with audio-clock word timing from index marks. SAPI5 was measured and rejected for highlighting: Eloquence reports one word event per sentence through it (ADR-0007).
11. **Apple speech on macOS (added during Wave 1 at Jon's request).** A sixth Wave 1 agent (F) builds `textweaver-apple` with two selectable backends, `nsspeech` (NSSpeechSynthesizer, most responsive) and `avspeech` (AVSpeechSynthesizer into buffers with per-word sample offsets); Apple's bundled Eloquence voices, Reed by default. No Mac is available: testing runs on GitHub's macOS runners (ADR-0008).
12. **SAPI5 voices (added during Wave 1 at Jon's request).** A seventh Wave 1 agent (G) builds `textweaver-sapi`: SAPI5 voices in x64 and x86 host processes (32-bit-only voices such as VW Paul, Kate, James and eSpeak), synthesized to memory with per-word audio offsets; every non-Eloquence voice on the development machine reports word timing (ADR-0009).

## See also

- [Architecture](../dev/architecture.md): the system as built, with links to every ADR.
- [Tasks and ownership](tasks.md): the work log of every wave and agent.
- [Star parity reference](star-parity.md) and [Star features not yet planned](../star-gaps.md): what was carried over from Star, and what was not.
- [CHANGELOG.md](../../CHANGELOG.md): what each release contains.
- [Documentation index](../README.md)
