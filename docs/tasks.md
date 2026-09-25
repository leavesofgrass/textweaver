# Tasks: ownership, briefs, and acceptance

Kept current per wave by the orchestrator. Agents append to their own section's "Status" line only.

## Wave 1 status

| Agent | Branch | Status |
|---|---|---|
| A — Text & Formats | `agent/a-text-formats` | not started (waits for Jon's go-ahead after the Phase 0 report) |
| B — Speech | `agent/b-speech` | not started |
| C — State, Keys, Editing | `agent/c-state-keys-editing` | not started |
| D — App & TUI | `agent/d-app-tui` | not started |

## Shared preamble (every agent reads this first)

**Project.** textweaver is a Rust reimplementation of Star, an accessible text-to-speech document reader for students with print disabilities. Read, in order: `docs/plan.md` (including "Phase 0 amendments"), the ADRs in `docs/adr/`, your sections of `docs/star-parity.md`, and the Phase 0 code in the crates you own and the crates you depend on. The Phase 0 code **is** the contract: public types and signatures you must keep, with deliberately naive bodies you replace.

**Ownership.** Edit only the paths your brief lists. You may add files under `fixtures/<your-letter>/` and tests inside your crates. Never edit `crates/textweaver-core`, the root `Cargo.toml`, `rust-toolchain.toml`, `.github/`, `docker/`, `compose.yaml`, or another agent's paths.

**Dependencies.** Add third-party crates only with `name.workspace = true` in your own crate's `Cargo.toml`, choosing from `[workspace.dependencies]` in the root `Cargo.toml`. Need something else, or a feature flag on a workspace dependency? Work around it and ask under "Contract change requests" in your report.

**Contract changes.** If a public type in core or in another agent's crate must change, keep working around it (a local adapter, a private helper) and write the exact proposed change (type, signature, reason) under "Contract change requests". The orchestrator resolves these at integration. Within your own crates you may add public items freely; removing or changing a Phase 0 public signature is a contract change.

**Quality bar.**
- Rustdoc on every public item (`missing_docs` is denied in CI).
- No `unwrap()`/`expect()` on user input or I/O in library code; `thiserror` errors in libraries.
- No `todo!()`, `unimplemented!()`, or `dbg!()` left behind.
- Accessibility is the product: every user-visible state change must be announceable, and every string the user hears must read well aloud.
- Fix Star's listed bugs rather than port them; if you knowingly keep a Star quirk, say so in the parity notes.

**Verification before reporting** (all green, both environments):

```bash
cargo fmt --all --check
cargo clippy -p <crate> --all-targets --all-features -- -D warnings
cargo test -p <crate> --all-features
```

and the same inside the Linux container, from your worktree's root, with your own target directory so parallel agents do not share build locks:

```bash
docker compose -p textweaver run --rm -T -e CARGO_TARGET_DIR=/target/agent-<letter> dev cargo test -p <crate> --all-features
```

Agent B must also build `--features espeak` in the container (espeak-ng is installed there, not on Windows).

**Git.** Work on your branch in your worktree. Commit in small steps with clear messages; end each commit message with the line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Do not push, merge, rebase onto `main`, or tag; the orchestrator integrates.

**Dates.** Never write a date or weekday from memory. Get today's date from the machine (`python -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"`) before it goes into any file or commit.

**Report format** (your final message):
1. Summary (five lines).
2. Files changed, grouped by crate.
3. Tests run, with the result lines of the output (native and container).
4. Contract change requests (exact proposals), or "none".
5. Open issues and known gaps.
6. What the next wave should do first.

## Agent A — Text & Formats

**Owns:** `crates/textweaver-text/`, `crates/textweaver-formats/`, `crates/textweaver-cli/src/cmd/{text,info,search}.rs`, `fixtures/` (except other agents' subdirectories), `xtask/src/parity.rs`.

**Read:** ADR-0002, ADR-0005; `docs/star-parity.md` Part 1 sections 1–5 and 7; `fixtures/star-parity/README.md` and the JSON files.

**Deliverables:**
- Rope-based `Document` with markers and a per-kind `MarkerIndex` (index tables, binary search, wrap, level filter); `Document::apply` shifting markers.
- Units by segment iterators over the rope: grapheme, word (UAX #29, alphanumeric-bearing), sentence (UAX #29 refined by an abbreviation list, including `Dr.`, `Mr.`, `Mrs.`, `e.g.`, `i.e.`, `a.m.`, `p.m.`, `St.`, `pp.`), line, paragraph, marker units. Word and sentence rules parity-tested against the Star corpus with documented deltas.
- `navigate`, `go_to` (percent, line, char, start, end, heading n), `History` (forward entries truncated on a new jump; back from the live position remembers it so forward returns; no duplicate entries), `search` (plain, regex, case, whole word, wrap, direction; the backward bug fixed; decide and document overlap), `narrate::plan` (sentence-sized utterances; long sentences split at whitespace under `max_chunk_chars`; heading and table narration as `Inserted` spans per `NarrationPolicy` and verbosity; `skip_code`).
- Loaders: text (exists), markdown (pulldown-cmark events → canonical text + markers; front matter parsed into `meta`, not spoken; footnotes handled per `LoadOptions`), html (scraper; Star's skip list and alt rules, but not its unclosed-`<meta>` bug that empties documents); registry priorities; cache keyed by `(path, mtime, size, options fingerprint)` under the cache directory the caller passes.
- `tw text FILE [--format text|markdown|json] [--structure]`, `tw info FILE [--json]`, `tw search FILE PATTERN [--regex --case-sensitive --whole-word --json]`.
- `cargo xtask parity`: writes `docs/parity-report.md` comparing word tokens, sentence starts, and paragraph starts with `fixtures/star-parity/*.json`, aligned by word sequence (canonical texts differ by design, ADR-0002).

**Acceptance:**
- `cargo test -p textweaver-text -p textweaver-formats` green, including proptests (segment ranges tile or nest correctly; marker shifting keeps ranges ordered and in bounds; history invariants).
- `insta` snapshots of the loaded document (text + markers as JSON) for every fixture.
- `docs/parity-report.md` exists with every delta explained.
- `tw text fixtures/sample.md` prints the canonical text with headings, list items, and table rows on their own lines.

## Agent B — Speech

**Owns:** `crates/textweaver-speech/`, `crates/textweaver-cli/src/cmd/{speak,voices,backends}.rs`.

**Read:** ADR-0003, ADR-0004, ADR-0005; `docs/star-parity.md` Part 2 (all of it; section 6 holds the test vectors).

**Deliverables:**
- `SpeechService` per ADR-0003: command channel, queue with two-chunk lookahead, generation counter, stale-event sink, playback clock, word events mapped through each utterance's `OffsetMap` into `SpeechStatus::Position`, `Paused { resume_at }` from the last confirmed word, `say` modes, `speak_char` with caps indication, tones and earcons where the backend can, punctuation level and split caps applied through normalization.
- `queue.rs` and `pacing.rs` pure, driven by an injectable `Clock`; Star's constants and clamp rules.
- Normalization transforms with offset maps (`normalize.rs` or a `normalize/` module): Markdown residue, abbreviations, numbers, dates, times, currency, math, punctuation verbosity, split caps, pronunciation lexicon; each built with `SpokenBuilder`, applied per utterance. Fix the Star bugs the inventory lists (for example `3:45 today`, `9:30 a.m.`, `$5.` losing its period, `I said no.`, `snake_case`, `John 3:16`).
- Backends: `null` (exists), `recording` (test double that records calls and emits scripted events), `espeak` (feature; in-process espeak-ng through `espeakng-sys`, word events with `audio_position`, UTF-8 byte offsets; `synthesize_to_file` via retrieval mode if practical), `omnivox` (feature; subprocess driving the Emacspeak protocol on stdin; timer pacing), auto-selection per Star (explicit preference; priority order; opt-in skipped; `null` last). An explicit preference that is unavailable falls back to auto, not straight to silence (Star's bug).
- `tw speak [TEXT | --file F] [--backend --voice --rate --pitch] [--out FILE] [--json]` (with `--json` print utterances, offset maps, and the status stream), `tw voices [--backend] [--json]`, `tw backends [--json]`.

**Acceptance:**
- Service tests with `recording` and a fake clock: queue order, generation and chunk cancellation, stale events dropped, every pause/resume edge case in ADR-0003, the timer clamp scenarios from `test_highlight_pacing.py`.
- Every normalization vector from `test_ttstext.py` passes (or has a documented, deliberate fix), and every normalized utterance passes `check_invariants`.
- The omnivox adapter tested against a fake process (a small test binary or a pipe) that records the protocol lines.
- `--features espeak` builds and its tests pass in the container; `tw speak --backend null --json "test"` works everywhere.

## Agent C — State, Keys, Editing

**Owns:** `crates/textweaver-store/`, `crates/textweaver-keymap/`, `crates/textweaver-a11y/`, `crates/textweaver-editor/`, `crates/textweaver-cli/src/cmd/{marks,migrate}.rs`, `xtask/src/keyboard.rs`.

**Read:** ADR-0001, ADR-0006; `docs/star-parity.md` Part 1 section 6 (keys), Part 3 (all).

**Deliverables:**
- `store`: settings load/save (only on explicit change, atomic, unknown keys preserved, corrupt-file backup; port `tests/test_settings.py` where it applies to TOML); `StateStore` with a debouncer (position saves coalesced, flushed on drop and on demand; fixes Star's never-called `flush_pending`); bookmarks; recent; the sidecar (`<folder>/.textweaver/progress.json`) with Star's merge policies (`merge_progress`, `merge_annotations`, conflicts) ported from `star/sync.py`, fixing the bug where `record_progress` always keeps the local entry.
- `keymap`: finalize `ActionId` and defaults against Part 1 section 6 (keep the layer model and the terminal constraints of ADR-0006); every action has a default and help string; no conflicts on either frontend or any platform; `cargo xtask keyboard` writes `docs/keyboard.md` (by category, both frontends, layer noted) and exposes the same data for in-app help.
- `a11y`: announcers as in ADR-0006; a verbosity table of which announcements belong to which level (document it in the crate docs).
- `editor`: undo/redo grouping (typing coalesces into word-sized undo steps; formatting commands are one step each); Markdown operations ported with Star's exact semantics and all 56 authoring tests (fix the listed Star bugs, and document where textweaver toggles instead of Star's non-toggling behavior only if you choose to); `echo::for_edit` with word completion and `CursorMoved` ("blank" for empty lines); autosave policy and `RecoverySnapshot`; the save rule (never overwrite a converted source such as `.rst`, `.org`, `.adoc`, `.html`).
- `tw marks FILE [--json]`; `tw migrate-star` may remain a stub in wave 1 (it is wave 2) but its argument surface stays.

**Acceptance:**
- Crate tests green; sidecar merge tests ported one-to-one (45); authoring tests ported (56); proptests for echo events over random insert/delete sequences and for undo/redo returning the exact text.
- `cargo xtask keyboard` output committed as `docs/keyboard.md`.

## Agent D — App & TUI

**Owns:** `crates/textweaver-app/`, `crates/textweaver-tui/`, `crates/textweaver-cli/src/cmd/{open,serve}.rs`, `tests/` at the workspace root (create it; add it to no manifest; put integration tests inside `crates/textweaver-app/tests/` or `crates/textweaver-tui/tests/` instead if simpler, and say which).

**Read:** ADR-0002, ADR-0003, ADR-0006; `docs/star-parity.md` Part 1 sections 3, 4, 6, 7 and Part 3 sections 2 and 6.

**Deliverables:**
- `App`, `Session`, `Mode`, `Command`, `Effect`, `dispatch`, `poll_speech` implementing every Wave 1 `ActionId`: reading by character, word, sentence, line, paragraph, selection, and document; sentence next/previous/replay with Star's "more than three words in" rule; paragraph, heading (read and skip variants), table, list, list item, link, chapter navigation; Speech Cursor line mode ("blank" on empty lines, no wrap, Enter reads on); find with prompt, next and previous; bookmarks (add, list, next, previous); history (one consistent rule: every jump of a sentence or larger, every find, go-to, bookmark, and chapter jump records the departure point once); go-to; position save on quit and document switch and restore on open (first word at or after; `auto_resume`); rate, pitch, volume, speed presets; announcements for every state change through the announcer, respecting verbosity.
- Build against the Phase 0 stubs from day one; your code must keep working when A, B, and C replace the bodies.
- TUI: ratatui + crossterm raw mode; title line, a viewport over a window slice of the document, status line (also the `StatusLineAnnouncer`), key hints, minibuffer for prompts; the hardware cursor parked on the caret or the spoken word so screen readers and magnifiers follow; highlights for the spoken unit (word, sentence, or both), find hits, and bookmarks; themes (at least the default `galaxy`, a light theme, and high contrast); F2 command palette; `?`/F3 keyboard help from the keymap; self-voicing through the speech service, silent with `--no-speech`.
- `tw open FILE` launches the TUI; `tw serve --stdio` may remain a stub (wave 2).

**Acceptance:**
- `App` tests without a terminal.
- A ratatui `TestBackend` scripted test: open a fixture, navigate by each unit, read with a recording speech backend (use B's once merged; until then a local test double that implements `SpeechBackend`), check that highlight ranges equal the spoken source ranges, quit, relaunch with the same state directory, and verify the restored position.

## Seams to watch at integration

- `text::narrate::plan` (A) → `SpeechService::read` (B): utterance ids, offset maps, `Inserted` spans.
- `Document::apply` (A) ↔ `Editor` (C) ↔ bookmarks and history (C, D): one `EditOutcome` shifts all of them.
- `Keymap` (C) ↔ TUI key translation (D): chord normalization and layers.
- `Settings` (C) ↔ `ServiceConfig` (B) ↔ `App` (D): rate, pitch, volume, pacing, verbosity.

## Wave 2 (after Integration 1 and Jon's review)

| Agent | Scope |
|---|---|
| A | EPUB and DOCX loaders; `paperback` feature; `pandoc` feature; exports to Markdown / HTML / text; full-text index |
| B | `speech-dispatcher` backend with index marks; `tts`-crate backend; `synthesize_to_file` and a capture wrapper; audio export (WAV, ffmpeg, SRT / VTT, M4B chapters); omnivox in-process option |
| C | Notes and highlights with remapping across edits; library folders and recent; `tw migrate-star`; dictation trait with a whisper subprocess backend; Obsidian vault import / export |
| D | Edit mode in the TUI; notes and highlights UI; `tw serve --stdio` JSON-RPC; expanded scripted tests |
