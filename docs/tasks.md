# Tasks: ownership, briefs, and acceptance

Kept current per wave by the orchestrator. Agents append to their own section's "Status" line only.

## Wave 1 status

| Agent | Branch | Status |
|---|---|---|
| A — Text & Formats | `agent/a-text-formats` | done, integrated |
| B — Speech | `agent/b-speech` | done, integrated |
| C — State, Keys, Editing | `agent/c-state-keys-editing` | done, integrated |
| D — App & TUI | `agent/d-app-tui` | done, integrated |
| E — Eloquence | `agent/e-eloquence` | done, integrated |
| F — Apple speech (macOS) | `agent/f-apple` | running |
| G — SAPI5 voices (Windows) | `agent/g-sapi` | done, integrated |

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
RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --no-deps
```

CI treats rustdoc warnings as errors. The usual failures are a redundant link target (`[`X`](crate::X)`: write `[`X`]`), a link to a private item from public docs, and square brackets in prose (`[mm:ss]`, `[@key]`): put those in backticks.

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

**Status:** Wave 1 deliverables done on `agent/a-text-formats` (Friday, September 25, 2026); awaiting integration. Parity report: `docs/parity-report.md` (0 unexplained deltas).

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

**Status:** Wave 1 done on `agent/d-app-tui` (Friday, September 25, 2026). Integration tests live in `crates/textweaver-app/tests/app.rs` and `crates/textweaver-tui/tests/scripted.rs` (no workspace-root `tests/`); the recording test double is `textweaver_app::testing::RecordingBackend`; fixture `fixtures/d/reading.txt`.

## Agent E — Eloquence (added 2026-09-25 at Jon's request)

**Owns:** `crates/textweaver-eci/` (library and the `textweaver-eci-host` binary), `xtask/src/eci.rs` (create it; the orchestrator wires it into `xtask/src/main.rs` at integration, so document the exact lines to add), `fixtures/e/`.

**Read:** ADR-0003, ADR-0004, ADR-0007; `tools/eci-spike/` (a working 32-bit program that drives `eci.dll`); `docs/star-parity.md` Part 2 sections 1–4; the Phase 0 code in `crates/textweaver-core` and `crates/textweaver-speech` (the `SpeechBackend` trait, `EventSink`, `RawEvent`, `Caps`, `BackendInfo`, `BackendFactory`).

**Deliverables:**
- The host: loads the ECI library with `libloading` (path from an argument, else `default_library_path()`), creates an engine, applies parameters, synthesizes each utterance with an index mark before every word, and streams PCM plus index marks with sample offsets over stdout in a small framed binary protocol; handles stop (abort synthesis), parameter changes, and voice selection (the `eci.ini` presets and ECI languages); exits cleanly on EOF. Text is encoded for the engine's language (Windows-1252 for Western languages; document the mapping and the replacement for unrepresentable characters).
- The backend: `EciBackend` implementing `SpeechBackend`, spawning the host, playing PCM through rodio (`rodio = { workspace = true, features = ["playback"] }`), emitting `Started`, `Word { byte_range, audio_ms }` (each index mapped back to the word's byte range in `Utterance::text`), `Finished`, and `Cancelled` under ADR-0003's non-blocking `speak` plus `poll` contract; native pause and resume; `set_params` mapping rate, pitch, and volume per ADR-0004 with a measured rate calibration for `effective_wpm()`; `voices()`; `synthesize_to_file` (WAV). Expose `pub fn backend_info() -> BackendInfo` and `pub fn factory(...) -> BackendFactory` so the app can register it (Agent B is making the registry extensible; the orchestrator wires the two together at integration).
- Community dictionaries (added 2026-09-25): load `third_party/ibmtts-dictionaries/` (CC0; see its README and ADR-0007) with `eciNewDict`, `eciLoadDict` for the main, root, and abbreviation volumes, and `eciSetDict`, per language (ENU, DEU); on by default, switchable off, overridable with a directory (`TEXTWEAVER_ECI_DICTIONARIES` and a backend option); files are Windows-1252 with CRLF line ends and must reach the engine unchanged; the xtask copies them next to the host; report load results per volume and a before/after example that the dictionary changes (for instance "omg" or "tabindex").
- OpenEVV by default (added 2026-09-25): discovery order and x64/x86 host selection per ADR-0007; OpenEVV through its x86_64 library only (its 32-bit build is cdecl); never bundled, fetched, or loaded in tests (fake-file tests for discovery and PE bitness).
- `cargo xtask eci-host`: builds the host for `i686-pc-windows-msvc` on Windows (native elsewhere) and copies it next to the workspace's debug and release binaries; the backend finds the host beside the current executable, or via `TEXTWEAVER_ECI_HOST`.

**Acceptance:**
- Unit tests for the protocol (round-trip framing, partial reads) and for index-to-byte-range mapping, including multibyte text.
- An integration test with a fake host (a test binary speaking the protocol) covering speak, word events in order with rising `audio_ms`, stop mid-utterance (`Cancelled`, no late events), pause and resume.
- Real-engine tests, `#[ignore]`d unless `TEXTWEAVER_ECI=1`, run against **licensed Voxin only**, in the container with `compose.voxin.yaml`: synthesize the spike's sentence to WAV and check every word's mark arrives with rising offsets; include the output. The Code Factory installation on this machine is not licensed: never load it. Windows real-engine tests wait for a licensed engine. Never play audio aloud in tests; use `synthesize_to_file` or a silent sink. Never commit engine audio; local samples go to the git-ignored `target-local/`.
- clippy and tests green natively (fake host) and in the container, with and without the Voxin overlay.

## Agent F — Apple speech on macOS (added 2026-09-25 at Jon's request)

**Owns:** `crates/textweaver-apple/`, `tools/avspeech-spike/` (may extend), `.github/workflows/apple.yml` (create it if you need a macOS-only workflow beyond `ci.yml`).

**Read:** ADR-0003, ADR-0004, ADR-0007, ADR-0008; `tools/avspeech-spike/` (probe scripts and their results); `docs/star-parity.md` Part 2 sections 1–4; the Phase 0 code in `crates/textweaver-core` and `crates/textweaver-speech`.

**No Mac is available.** Develop on Windows (the crate must compile to an empty library there), and test on GitHub's macOS 14 and 15 runners: you may push **your own branch only**, `agent/f-apple`, which triggers `ci.yml`. The macOS runners have the Eloquence voices and can synthesize; tests must never play audio aloud.

**Deliverables:**
- `NsSpeechBackend` (`nsspeech`) implementing `SpeechBackend` over `NSSpeechSynthesizer`: voices (with Eloquence voices named clearly), rate in wpm, pitch, volume, `willSpeakWord` → `RawEvent::Word` (no `audio_ms`), native pause at a word boundary and resume, stop, `synthesize_to_file`; delegate callbacks pumped from `poll` on the speech thread's run loop (ADR-0003). First verify with a probe that `NSSpeechSynthesizer` works on a background thread with its own run loop; report the result.
- `AvSpeechBackend` (`avspeech`) over `AVSpeechSynthesizer.write`: buffers plus interleaved `willSpeakRangeOfSpeechString` callbacks → word sample offsets; playback through rodio (`features = ["playback"]`); `Word { byte_range, audio_ms }`; native pause and resume; WAV export.
- Range mapping from UTF-16 `NSRange` to UTF-8 byte ranges of `Utterance::text`, including Eloquence's sub-token ranges ("Dr" in "Dr.", "9" and "30" in "9:30").
- Voice selection defaulting to Eloquence Reed (`DEFAULT_VOICE`) when present; both backends expose `backend_info()` and a factory for the app to register (the orchestrator wires them).
- Declare native normalization for the Eloquence voices (Agent B is adding a way to declare it; describe your need in the report if you cannot see it).
- Measurements, reported: first-word latency for both backends and Reed versus Samantha; highlight offset accuracy of `avspeech` on macOS 14 and 15; the wpm calibration of each backend.

**Acceptance:**
- Unit tests for range mapping (UTF-16 to UTF-8, multibyte, sub-token ranges) run on every OS.
- macOS-only tests `#[ignore]`d unless `TEXTWEAVER_APPLE=1` (CI sets it on macOS): synthesize a sentence with Reed through each backend to a temporary file; check word events arrive in order and cover every word; `avspeech` offsets rise.
- `ci.yml` green on your branch on all three OSes.

## Agent G — SAPI5 voices on Windows (added 2026-09-25 at Jon's request)

**Owns:** `crates/textweaver-sapi/` (library and the `textweaver-sapi-host` binary), `xtask/src/sapi.rs` (create it; document the lines the orchestrator adds to `xtask/src/main.rs`), `fixtures/g/`.

**Read:** ADR-0003, ADR-0004, ADR-0007, ADR-0009; `docs/star-parity.md` Part 2 sections 1–4; the Phase 0 code in `crates/textweaver-core` and `crates/textweaver-speech`.

**Deliverables:**
- The host (x64 and x86 builds of one binary): `ISpVoice` via the `windows` crate (COM initialized on the host's thread), voice selection by token id, output to a memory stream in a fixed PCM format, `SPEI_WORD_BOUNDARY` (and `SPEI_END_INPUT_STREAM`) events with `ullAudioStreamOffset`, streamed with PCM over stdout in a framed binary protocol; stop (purge), parameter changes; clean exit on EOF. Voice enumeration for its architecture's registry.
- The backend: `SapiBackend` implementing `SpeechBackend` under ADR-0003's non-blocking `speak` plus `poll` contract; voice listing across both registries (64-bit preferred; each voice tagged with its host architecture); rodio playback (`features = ["playback"]`) with `Word { byte_range, audio_ms }` from the stream offsets and UTF-16 to UTF-8 range mapping; native pause and resume; rate, pitch, and volume mapping with measured wpm calibration; `synthesize_to_file` (WAV); `backend_info()` and a factory for the orchestrator to wire. Eloquence voices listed with a note that they give no word timing (never load them in tests; that product is not licensed on this machine).
- Probe and report whether OneCore voices (`HKLM\SOFTWARE\Microsoft\Speech_OneCore\Voices`: Microsoft Mark, David, Zira) are usable through SAPI5 by setting the token category, and include them if so.
- `cargo xtask sapi-host`: builds both hosts and places them next to the workspace binaries (`textweaver-sapi-host.exe` and `textweaver-sapi-host-x86.exe`); the backend finds them beside the current executable or via `TEXTWEAVER_SAPI_HOST` / `TEXTWEAVER_SAPI_HOST_X86`.

**Acceptance:**
- Unit tests for framing and UTF-16 to UTF-8 mapping (multibyte, repeated events on one range, sub-token ranges).
- A fake-host integration test: ordered word events with rising `audio_ms`, stop mid-utterance (`Cancelled`, no late events), pause and resume.
- Real-voice tests `#[ignore]`d unless `TEXTWEAVER_SAPI=1`, run locally before reporting with output included: Microsoft David (64-bit host) and eSpeak (32-bit host) synthesize to WAV with every word's event in order. Use only Microsoft voices and eSpeak in tests. Never play audio aloud. Never commit engine audio (local samples go to the git-ignored `target-local/`).
- clippy and tests green on Windows; the crate builds as an empty library in the Linux container.

## Seams to watch at integration

- The ECI (E) and SAPI (G) hosts each define a PCM-plus-events protocol and a playback client; merge them into one shared engine-host crate at integration.

- `text::narrate::plan` (A) → `SpeechService::read` (B): utterance ids, offset maps, `Inserted` spans.
- `Document::apply` (A) ↔ `Editor` (C) ↔ bookmarks and history (C, D): one `EditOutcome` shifts all of them.
- `Keymap` (C) ↔ TUI key translation (D): chord normalization and layers.
- `Settings` (C) ↔ `ServiceConfig` (B) ↔ `App` (D): rate, pitch, volume, pacing, verbosity.
- `SapiBackend` (G), `NsSpeechBackend`/`AvSpeechBackend` (F) and `EciBackend` (E) ↔ backend registry (B) ↔ app backend selection (D): Eloquence first when installed; normalization skipped for engines that normalize natively.

## Wave 2 (started 2026-09-25; Jon asked to keep going through the waves without pausing)

Wave 1 is integrated on `main` (tag `v0.1.0-alpha.1`); Agent F's Apple speech lands separately. Wave 2 agents branch from `main` and follow the shared preamble above, with these updates:

- **Branches:** `wave2/<letter>-<topic>`. Do not push (except Agent K, below); the orchestrator integrates.
- **Toolchain:** `rust-version` is 1.89; let-chains are expected (clippy's `collapsible_if`).
- **Native checks on Windows** use `--features textweaver-speech/omnivox` (only when `textweaver-speech` is among the packages checked; otherwise `--features textweaver-cli/omnivox` when the CLI is, or no features) instead of `--all-features` (the `espeak` feature needs libespeak-ng, Linux only) and `--exclude textweaver-gui` on workspace commands. The container runs `--all-features`.
- **Docker from Git Bash:** prefix with `MSYS_NO_PATHCONV=1`, or Git Bash rewrites `/target/...` into a Windows path.
- **Engines:** never load Code Factory's Eloquence (unlicensed on this machine) or OpenEVV in tests; real Eloquence tests use Voxin in the container (`compose.voxin.yaml`); real SAPI tests use Microsoft voices and eSpeak only. Never play audio aloud; never commit engine audio.
- **New ADRs:** each agent below owns the ADR number given in its brief.
- **Wave 1 contract requests** are assigned below; resolve the ones in your crates.

| Agent | Branch | Status |
|---|---|---|
| A2 — Formats and conversion | `wave2/a-formats` | not started |
| B2 — Speech service and audio export | `wave2/b-speech-export` | not started |
| C2 — State, notes, library, migration | `wave2/c-state-library` | not started |
| D2 — App, TUI editing, JSON-RPC | `wave2/d-app-edit-rpc` | not started |
| H — Shared engine host | `wave2/h-enginehost` | not started |
| J — Obsidian vault and dictation | `wave2/j-vault-dictation` | not started |
| K — GUI feasibility spike | `wave2/k-gui-spike` | not started |

### Agent A2 — Formats and conversion

**Owns:** `crates/textweaver-text/`, `crates/textweaver-formats/`, `crates/textweaver-cli/src/cmd/{text,info,search}.rs` (`convert.rs` moved to Agent L on 2026-09-25), `fixtures/a/`, `xtask/src/parity.rs`, `docs/adr/0010-pdf-loader.md`, `docs/parity-report.md`.

**Deliverables:**
- EPUB loader (zip, OPF spine, NAV or NCX table of contents to `SectionBreak` markers with chapter titles, images as alt text) and DOCX loader (`word/document.xml`: heading styles, lists with levels, bold/italic/underline runs, `docPr` alt text, tables in place, footnotes), both on the shared builder, with `insta` snapshots on new fixtures you create (keep fixture files small and your own).
- PDF: choose between `lopdf`, `pdf-extract`, and `pdfium-render` (all in the workspace table; the unchosen ones are removed at integration), write ADR-0010 with the measured trade-offs (text quality on a multi-column fixture, reading order, speed, native dependencies), and implement the loader behind a `pdf` feature. Star's column-aware reading order (`docs/star-parity.md`) is the quality bar. The `paperback` feature stays a stub unless `paperback-core` is on crates.io and suits; report either way.
- `pandoc` feature: a subprocess loader for the long tail (odt, rtf, rst, org, latex, docbook), available when `pandoc` is on PATH.
- Exports in `formats`: Markdown, HTML, and plain text, with Markdown escaping fixed.
- `tw convert FILES/FOLDERS --to markdown|html|text [--out DIR] [--watch]` (batch conversion and hot-folder watch with `notify`; Star's `watch_*` settings semantics).
- Wave 1 requests: `History::restore(entries, capacity)` (Agent D); `LoadOptions::footnotes: {Deferred, Inline, Skip}` replacing the bool; sentence windowing so sentence steps in multi-megabyte paragraphs stay fast; `encoding_rs` for non-UTF-8 text and HTML charset declarations.
- Full-text index over loaded documents (index side of `star/fulltext.py`): a small on-disk inverted index API that C2's library search calls.

**Acceptance:** crate tests and snapshots green; `tw text` works on every new format's fixture; `tw convert --watch` has a test with a temporary folder; the parity report still has zero unexplained deltas.

**Status:** done on `wave2/a-formats` (Friday, September 25, 2026): EPUB, DOCX, and PDF loaders (PDF on by default, pure Rust, ADR-0010), `pandoc` feature, Markdown/HTML/text exports, full-text index, and the Wave 1 requests. `tw convert` and `--watch` moved to Agent L and were not done here. Parity report: 0 unexplained deltas.

### Agent B2 — Speech service and audio export

**Owns:** `crates/textweaver-speech/`, `crates/textweaver-export/`, `crates/textweaver-cli/src/cmd/{speak,voices,backends,export_audio}.rs`, `docs/adr/0011-audio-export.md`.

**Deliverables:**
- Wave 1 requests: generation-tagged statuses (`read` returns its generation; `Finished`, `Stopped`, `Paused` carry it) so the app can drop stale status without heuristics (Agent D); `BackendInfo` gains the backend's `Caps` (Agent D); the service re-reads `capabilities()` after `set_params` (Agent G: some voices have no word timing); `Voice` gains `tags: Vec<String>` (Agent G: "OpenEVV", "Eloquence", "OneCore", "32-bit").
- `speechd` feature: a speech-dispatcher backend with SSML index marks for word events, tested in the container (start `speech-dispatcher` with its espeak-ng module in the test).
- `textweaver-export`: read a document to WAV through any backend with `SYNTH_TO_FILE` (utterance by utterance, recording each sentence's start and end), then optionally convert with `ffmpeg` if on PATH (MP3; M4B with chapters from `Heading`/`SectionBreak` markers); SRT and WebVTT cues per sentence (Star's `star/tts/subtitles.py` semantics, with optional word-level cues where word timings exist). Write ADR-0011.
- `tw export-audio FILE --out out.wav|mp3|m4b [--subtitles out.srt|vtt] [--backend --voice --rate]`.
- Pronunciation lexicon from the plain-text entries of the IBMTTS community dictionary (`third_party/ibmtts-dictionaries/`; `word<TAB>respelling`, skipping phoneme entries starting with a backquote), applied for engines without native normalization, off by default, as a normalization option.

**Acceptance:** service tests for the new status generations and the caps re-read; export tests with the `recording` backend (deterministic timings) checking cue files byte for byte; an espeak export in the container producing a WAV and an SRT; ffmpeg conversion tested when available (Windows has ffmpeg on PATH; skip cleanly where absent).

**Status:** Wave 2 deliverables done on `wave2/b-speech-export` (Friday, September 25, 2026); awaiting integration. Breaking API changes (statuses, `BackendInfo::caps`, `Voice::tags`) are adapted in the app, ECI, and SAPI crates by a separate "integration shim" commit on the branch; ADR-0011 written. The container needs `speech-dispatcher-espeak-ng` for the real speech-dispatcher test (installed ad hoc for this run).

### Agent C2 — State, notes, library, migration

**Owns:** `crates/textweaver-store/`, `crates/textweaver-keymap/`, `crates/textweaver-a11y/`, `crates/textweaver-editor/`, `crates/textweaver-cli/src/cmd/{marks,migrate,library}.rs`, `xtask/src/keyboard.rs`, `docs/keyboard.md`.

**Deliverables:**
- Notes and highlights in `DocState` (Star's annotation fields, colors, tags), remapped across edits with the same `EditOutcome` as bookmarks; export of notes as Markdown for Agent J's vault export to reuse.
- Library: folders, `library.json`, recent documents, sidecar sync wired to library folders (Star's `star/library.py`), and library search calling Agent A2's full-text index API (build against a trait if A2's API is not visible; the orchestrator wires them).
- `tw migrate-star [--from DIR] [--dry-run]`: settings, reading positions (mapped by word-sequence alignment, ADR-0002), bookmarks, notes, highlights, recent files, library folders, keybindings, and `.star/progress.json` sidecars converted to `.textweaver`; a report of everything imported or skipped.
- `tw library [--search TEXT] [--add DIR] [--json]`.
- Settings: `[speech.eci]` (`dictionaries` on/off/path, `library`, `code_factory`), `[speech.sapi]` (`onecore`), `[speech.apple]` (backend preference), all mapped for the app at integration.
- Wave 1 requests: keymap actions `select_next_word`, `select_previous_word`, `select_next_line`, `select_previous_line` (Shift+arrows in Browse), `read_paragraph`; keep terminal F3 as find next and document `?`/F1 for help; the recovery-snapshot lock with `File::try_lock` (Star bug 39) now that the MSRV is 1.89.

**Acceptance:** crate tests green; migration tested against a synthetic Star configuration directory built in the test (no real Star data); `docs/keyboard.md` regenerated.

**Status:** Wave 2 deliverables done on `wave2/c-state-library` (Friday, September 25, 2026), including the orchestrator's character-key additions (single-key shortcuts off switch, quit and delete confirmation marks, conflicting overrides rejected); awaiting integration.

### Agent D2 — App, TUI editing, JSON-RPC

**Owns:** `crates/textweaver-app/`, `crates/textweaver-tui/`, `crates/textweaver-cli/src/cmd/{open,serve}.rs`, `docs/adr/0015-json-rpc.md`.

**Deliverables:**
- Edit mode in the TUI on C's `EditSession`: typing with echo (characters, words, deletions, lines on move, caps indication) spoken through the speech service; Markdown formatting commands; undo and redo; Save (in place for text and Markdown), Save As, New; the autosave recovery prompt at startup; edits applied to the `Document` with `Document::apply` so markers, bookmarks, and notes move.
- Notes and highlights in the TUI: add, list, jump, delete, with announcements.
- `tw serve --stdio`: JSON-RPC 2.0 over stdin and stdout exposing open, navigate, read, stop, position, search, and status notifications, so editors and other frontends can drive textweaver; write ADR-0015 with the method list; a scripted test.
- Wave 1 follow-ups: `tw open` in-process (add `textweaver-tui` as a dependency of the CLI; the orchestrator adds it to the workspace table at integration, so depend on it by path in the meantime); `highlight.lead_words`; periodic position saves; bookmark rename and delete; settings saved when changed, not only at quit.
- Build against the current APIs; adopt B2's generation-tagged statuses and C2's notes and library at integration (or earlier if merged into `main` before you finish; watch `git log origin/main`).

**Acceptance:** app and TUI tests green, including a scripted edit-mode test (type, format, undo, save, reopen) and a JSON-RPC session test.

**Status:** Wave 2 done on `wave2/d-app-edit-rpc` (Friday, September 25, 2026). Tests: `crates/textweaver-app/tests/{edit,rpc}.rs`, `crates/textweaver-tui/tests/edit.rs`. The CLI depends on `textweaver-tui` by path (move it to the workspace table). Notes, highlights, and bookmark management use app-level `NoteCommand`s with stopgap keys (`extra_bindings`) until C2 adds keymap actions; notes persist in `DocState` extra keys `app_notes`/`app_highlights` until C2's typed notes are wired.

### Agent D3 — App and TUI wiring of B2, C2, and J

**Owns:** `crates/textweaver-app/`, `crates/textweaver-tui/`, the app-facing parts of `crates/textweaver-cli/` (not `cmd/cite.rs`, `cmd/export_audio.rs`). Branch `wave2/d3-app-wiring`.

**Deliverables:** speech followed by `ReadingGeneration` with capability changes announced; notes and highlights on `DocState`'s typed fields (legacy `app_notes`/`app_highlights` migrated once) and moved with `DocState::shift`; `[keyboard] character_keys` applied and F9 wired; `confirm` for JSON-RPC actions that ask first and the pending question on the TUI status line; the library list (Alt+L) with bookshelf, recent files, and sidecar sync; `read_paragraph` and the select actions; `[speech.eci]`, `[speech.sapi]`, `[speech.apple]`, `[normalization.community_lexicon]`, and `[export]` mapped; modifier chords for the most used palette-only actions.

**Status:** done on `wave2/d3-app-wiring` (Friday, September 25, 2026); awaiting integration. Only `choose_voice` still says "not available yet" (the speech service offers no voice list). `tw export-audio` does not read `[export]` yet: `textweaver_app::subtitle_plan` is ready for B2's `export_audio.rs` (patch in the D3 report). New tests: `crates/textweaver-app/tests/wiring.rs`, `crates/textweaver-tui/tests/wiring.rs`.

### Agent H — Shared engine host

**Owns:** `crates/textweaver-enginehost/`, `crates/textweaver-eci/`, `crates/textweaver-sapi/`, `xtask/src/{eci,sapi}.rs`, `docs/adr/0012-engine-host.md`.

**Deliverables:**
- Move the duplicated framed protocol, host process management (spawn, restart after a crash or hang), PCM playback with the audio clock, native pause and resume, and WAV writing from `textweaver-eci` and `textweaver-sapi` into `textweaver-enginehost`; make both crates use it without changing their behavior or public API; all existing tests stay green (fake hosts, Voxin real-engine tests in the container, SAPI real-voice tests with Microsoft voices and eSpeak).
- Resolve the capability questions on this side: both backends declare `PLAYBACK_EVENTS` where they own playback; SAPI reports caps per selected voice.
- Release packaging for hosts: `cargo xtask hosts` builds every host for the current platform (x64 and x86 on Windows) with the dictionaries, for both debug and release, and is what CI and the release job call.
- Write ADR-0012 (the protocol, message list, versioning).

**Acceptance:** no behavior change visible to users; test counts at least as high as before; both backends' fake-host suites run against the shared client.

### Agent J — Obsidian vault and dictation

**Owns:** `crates/textweaver-vault/`, `crates/textweaver-dictation/`, `crates/textweaver-cli/src/cmd/{vault,dictate}.rs`, `docs/adr/0013-dictation.md`.

**Deliverables:**
- `textweaver-vault`: export a document's notes and highlights to an Obsidian vault as Markdown notes with front matter and wikilinks, and import vault notes as documents with their links, ported from `star/obsidian.py` (see `docs/star-parity.md` Part 3 §4.8); build against C's Phase 0 store types and request what you need from C2.
- `textweaver-dictation`: a `Dictation` trait (start, stop, partial and final text events), a whisper subprocess backend (whisper.cpp's `whisper-cli` or `faster-whisper`, detected on PATH; model choice as Star's `WHISPER_MODELS`), file transcription (`tw dictate --file`), and microphone capture through `cpal` if it is in the workspace (request it otherwise; file transcription first). Spoken commands while dictating ("new line", "period") as a pure, tested transform. Write ADR-0013.
- `tw vault import|export` and `tw dictate`.

**Acceptance:** vault round-trip tests on a temporary vault; dictation tests with a fake whisper process; real whisper tested only if installed (report whether it is).

**Status:** done on `wave2/j-vault-dictation` (Friday, September 25, 2026); awaiting integration. Notes and highlights go through `textweaver_vault::AnnotationStore` (a bridge keeps them in `DocState::extra` until C2's typed notes land); microphone capture waits for `cpal` in the workspace (ADR-0013). Real Whisper: openai-whisper is installed on this machine and its ignored test passes; whisper.cpp and faster-whisper are not installed.

### Agent K — GUI feasibility spike (wxDragon)

**Owns:** `crates/textweaver-gui/`, `.github/workflows/gui.yml` (create it), `docs/adr/0014-gui-toolkit.md`. You may push your branch `wave2/k-gui-spike` to run CI on Windows and macOS runners (CI triggers on `agent/**`; add `wave2/k-*` to your own `gui.yml` trigger).

**Deliverables:**
- Build wxDragon (`wxdragon`, `live-region` are in the workspace table) on this Windows machine: CMake is bundled with Visual Studio 2022 and Visual Studio 18 under `C:\Program Files\Microsoft Visual Studio\`; find it and document how the build locates it. Record the first and incremental build times.
- A minimal accessible reader window over `textweaver-app`: a menu bar with keyboard accelerators, a read-only multi-line text control showing a document, the caret following the spoken word through `App::poll_speech`, Play/Pause and Stop, a status bar, and announcements through `live-region`. Every control has an accessible name.
- Verify accessibility programmatically: use Windows UI Automation (PowerShell `System.Windows.Automation` or `UIAutomationClient`) to list the window's elements, names, and roles, and check that the text control exposes its text and caret. Report what NVDA would read; Jon tests with NVDA by ear afterwards.
- `gui.yml`: build the GUI on Windows and macOS runners.
- ADR-0014: whether wxDragon meets the bar (build cost, accessibility tree, text control behavior with large documents, live-region behavior), and the recommended Wave 3 plan.

**Acceptance:** `cargo build -p textweaver-gui` succeeds on Windows; the UIA report is included; the GUI workflow is green or its failures are explained.

### Conversion agents (added 2026-09-25 at Jon's request)

Jon's direction: conversion and bulk conversion must be **lightning fast, native Rust, and memory safe**, even where that means writing custom parsers; Pandoc stays only as an optional fallback for formats textweaver has no native reader for. His choices: Markdown flavors GFM, Obsidian, Pandoc Markdown, and LaTeX math as MathML; MiniJinja templates; outputs EPUB, DOCX, BRF braille, and PDF beyond Markdown, HTML, and text; folders converted by mirroring the tree and skipping outputs newer than their source. `tw convert` moves from Agent A2 to Agent L; A2 keeps the loaders and `formats` exports.

| Agent | Branch | Status |
|---|---|---|
| L — Rendering and bulk conversion | `wave2/l-render-convert` | not started |
| M — Native writers | `wave2/m-writers` | not started |

#### Agent L — Rendering and bulk conversion

**Owns:** `crates/textweaver-render/`, `crates/textweaver-convert/`, `crates/textweaver-cli/src/cmd/convert.rs` (from A2), `fixtures/l/`, `docs/adr/0016-rendering-and-conversion.md`, `docs/converting.md` (a user guide).

**Deliverables:**
- `textweaver-render`: Markdown to HTML through selectable engines behind one API (`Engine::PulldownCmark` default for speed, `Engine::Comrak` for full GFM), flavors (GFM: tables, task lists, strikethrough, autolinks, footnotes, alerts; Obsidian: `[[wikilinks]]` with aliases and headings, `![[embeds]]` as links or inlined text, `> [!note]` callouts, `#tags`, `^block` references; Pandoc Markdown: definition lists, fenced divs and bracketed spans with attributes, YAML metadata blocks, citations `[@key]` rendered as links, custom where engines lack it), LaTeX math `$…$` and `$$…$$` to MathML (`pulldown-latex`) so screen readers can read it, heading ids and a table of contents, optional sanitization (`ammonia`).
- Templates with MiniJinja: built-in templates (an accessible default with `lang`, landmarks, skip link, a readable stylesheet respecting `prefers-color-scheme` and `prefers-reduced-motion`; a print template; a bare fragment), user templates from a folder, variables from front matter.
- `textweaver-convert`: convert files and folder trees on all cores (`rayon`), mirroring the tree, skipping outputs newer than their source, with `--jobs`, `--force`, and a summary (files, bytes, time, failures); output formats Markdown, HTML (through `render` for Markdown sources and templates for everything), text, and, through Agent M's writers, EPUB, DOCX, BRF, and PDF (build against the writer trait you agree on in your reports; the orchestrator wires M's crate if needed). Inputs through `textweaver-formats` loaders; Pandoc only as a fallback when no native loader exists.
- `tw convert FILES/DIRS --to md|html|txt|epub|docx|brf|pdf [--out DIR] [--engine pulldown|comrak] [--flavor gfm|obsidian|pandoc] [--template NAME|PATH] [--jobs N] [--force] [--watch] [--json]`; `--watch` with `notify` and Star's `watch_*` semantics.
- Performance: a benchmark (`cargo xtask bench-convert` is the orchestrator's, so provide `crates/textweaver-convert/benches/` or an example binary) converting a generated corpus of 1,000 Markdown files; report files per second and peak memory, and profile the hot path. Memory stays proportional to one document per worker.
- ADR-0016 and `docs/converting.md` (a short guide for users, written for screen-reader users).

**Acceptance:** CommonMark conformance for the pulldown path through its test suite; flavor features covered by snapshot tests; a bulk-conversion test on a temporary tree checking mirrored paths and skip-unchanged; the benchmark numbers in the report.

**Status:** done on `wave2/l-render-convert` (Friday, September 25, 2026); awaiting integration. Both engines pass all 652 CommonMark 0.31.2 examples; EPUB, DOCX, BRF, and PDF output wait for Agent M's writers behind `textweaver_convert::DocumentWriter` (ADR-0016).

#### Agent M — Native writers

**Owns:** `crates/textweaver-writers/`, `fixtures/m/`, `docs/adr/0017-writers.md`.

**Deliverables:** writers from a `Document` (text plus markers; `textweaver-text`) in pure Rust, each validated:
- **EPUB 3**: XHTML content documents with semantic headings, lists, tables, figures with alt text; nav document from headings; package metadata including `schema:accessibilityFeature` and related accessibility metadata; passes `epubcheck` if Java and epubcheck are available (skip cleanly otherwise).
- **DOCX**: WordprocessingML with real Heading 1 to 6 styles, list numbering, tables with header rows, alt text on images; opens in Word and LibreOffice (validate structure by round-tripping through A's DOCX loader).
- **BRF braille**: Braille Ready Format pages (40 cells by 25 lines by default, configurable) with Unified English Braille grade 1 translation in pure Rust (tables in code, tested against known UEB examples); grade 2 contractions as a second phase if time allows, or through `liblouis` behind an optional feature if installed (report which).
- **Tagged PDF**: `krilla` with structure tagging (headings, paragraphs, lists, tables, alt text) aiming at PDF/UA, an embedded font, and document language; report what krilla's validation says.
- A `Writer` trait (`fn write(&Document, &WriteOptions, &mut dyn Write)`) that Agent L's converter calls; agree on it in your reports.

**Acceptance:** each writer has tests and a round-trip or structural validation; ADR-0017 explains the choices and the accessibility checks.

**Status:** Wave 2 deliverables done on `wave2/m-writers` (Friday, September 25, 2026); awaiting integration. `Writer` trait and `writer_for`/`write_to_vec` in `textweaver-writers`; ADR-0017 lists the checks (epubcheck and liblouis not installed here, so skipped).

### Queued after Agent H: DECtalk

A `dectalk` backend on the shared engine host (`textweaver-enginehost`), mirroring ECI: the DECtalk TTS API in memory mode with `[:index mark]` word marks, loaded from a user-supplied library (`TEXTWEAVER_DECTALK_LIBRARY`, and the install locations of a licensed DECtalk). The community DECtalk source's own licence file states it is proprietary to Fonix and usable only under a written licence, so textweaver never bundles, downloads, or tests against it; Jon may point textweaver at a build he has (his emacspeak-docker image compiles one) for his own testing.

### Agent R — DECtalk and export word timings

**Owns:** `crates/textweaver-dectalk/`, `docs/adr/0021-dectalk.md`, `docs/dectalk.md`; the `synthesize_utterance` overrides in `crates/textweaver-eci/` and `crates/textweaver-sapi/`, and `word_timings` in `crates/textweaver-enginehost/`.

**Deliverables:** a `dectalk` backend on the shared engine host for a user-installed, licensed DECtalk (never bundled, downloaded, or tested against the community source), with index-mark word timing, the nine speakers as voices, rate, pitch, volume, pause, and WAV export; word timings in audio export for ECI, SAPI, and DECtalk. Registry and xtask wiring are the orchestrator's (lines in the report).

**Status:** done on `wave2/r-dectalk` (Friday, September 25, 2026); awaiting integration. The DECtalk FFI is tested against a stand-in library (64-bit, and 32-bit `cdecl` and `stdcall`), not yet against a licensed DECtalk (none on this machine; live tests wait behind `TEXTWEAVER_DECTALK=1`). Export word timings verified with Microsoft David (SAPI) and Voxin (ECI, in the container).

### Math and citations agents (added 2026-09-25 at Jon's request)

Jon asked to carry Star's lessons forward: reading aids, math normalization, live previews, citation support, and ASCIIMath. Math and citations start now; reading aids and live previews follow the Star-lessons research (`docs/star-lessons.md`, being written) and Agent L's renderer and Agent K's GUI findings.

| Agent | Branch | Status |
|---|---|---|
| O — Math | `wave2/o-math` | not started |
| P — Citations | `wave2/p-cite` | not started |

#### Agent O — Math

**Owns:** `crates/textweaver-math/`, `fixtures/o/`, `docs/adr/0018-math.md`.

**Deliverables:**
- Parsers for **LaTeX math** (the common subset: fractions, roots, scripts, Greek, operators, big operators with limits, matrices and cases, `\text`, accents, delimiters) and **ASCIIMath** (the full published grammar, including its symbol table), both producing one math tree with source spans.
- **MathML** output (presentation MathML with `alttext`), which Agent L's renderer can call instead of `pulldown-latex` (say in your report how to wire it).
- **Spoken math**: natural English with ClearSpeak-style wording and three verbosity levels (for example "x squared", "the fraction a over b end fraction" at high verbosity, "a over b" at low), built with `SpokenBuilder` so the offset map points each spoken word back to its source span (ADR-0005). This replaces Star's math normalization (see `docs/star-parity.md` Part 2 §5 and Star's `star/ttstext/mathspeech.py` at D:\star for its wording and its bugs); Star's test vectors for math must pass or have a documented, better wording.
- **Math navigation** model: a pure API to move through a math tree (next term, into a fraction's numerator and denominator, into scripts, out), returning the spoken text and source span at each step, for the app to use later.
- Detection helpers: find `$…$`, `$$…$$`, `\(…\)`, `\[…\]`, and backtick-ASCIIMath (`` `…` `` with a configurable delimiter) in plain text without false positives on prices ("$5 and $10") — Star's bug list mentions currency and math collisions.

**Acceptance:** parser tests including every ASCIIMath symbol-table entry; MathML snapshot tests; spoken-math tests at each verbosity with offset-map invariants; property tests that parsing never panics on arbitrary input.

**Status:** Wave 2 done on `wave2/o-math` (Friday, September 25, 2026). ADR-0018 describes how Agent B2's math transform (`speak_text`, run before numbers) and Agent L's renderer (`latex_to_mathml` in place of `pulldown-latex`) call the crate.

#### Agent P — Citations

**Owns:** `crates/textweaver-cite/`, `crates/textweaver-cli/src/cmd/cite.rs` (create it; the orchestrator wires the subcommand), `fixtures/p/`, `docs/adr/0019-citations.md`.

**Deliverables:**
- A reference library stored as CSL-JSON (per user, and per document folder), with import and export of **BibTeX/BibLaTeX** (`biblatex`), **RIS**, and **CSL-JSON**, round-trip tested.
- **DOI lookup** (doi.org content negotiation for CSL-JSON) and **ISBN lookup** (Open Library), over blocking HTTP (`ureq`; an async client is allowed outside speech per ADR-0001 if parallel lookups need it), with timeouts, a small on-disk cache, and offline-friendly errors; tests against recorded responses (no network in tests; one `#[ignore]`d live test).
- **Formatting with CSL styles** through `hayagriva` (APA, MLA, Chicago author-date, IEEE, Vancouver at least, plus loading a `.csl` file): in-text citations and bibliography entries as plain text and as Markdown/HTML, readable aloud (no visual-only formatting).
- **Citation keys and insertion**: Pandoc-style `[@key]`, `[@key, p. 12]`, `[@a; @b]` parsing and resolution against the library, so Agent L's Pandoc-flavor renderer can render citations and a bibliography, and the editor can insert them (describe the API the app needs).
- `tw cite add DOI|ISBN`, `tw cite import FILE`, `tw cite export --to bibtex|ris|csl-json`, `tw cite format KEY --style apa`, `tw cite list [--json]`.
- Compare with Star's `star/citations.py` (D:\star) and list what you carried over and fixed.

**Acceptance:** crate tests green; import/export round trips; formatting snapshots per style; the CLI commands tested with a temporary library.

### Agent Q — Themes (added 2026-09-25 at Jon's request)

Jon wants several themes, as Star had. Star shipped 23 palettes in `star/themes.py` (galaxy, galaxy-light, one-dark, one-light, dark, light, contrast, high-contrast, phosphor, dracula, nord, solarized-dark, solarized-light, gruvbox-dark, gruvbox-light, tokyo-night, catppuccin-mocha, monokai, sepia, amber, everforest-dark, rose-pine, kanagawa) plus user CSS themes and OS light/dark following; the wiki's "star TUI palette contrast audit" found contrast failures (for example sepia losing headings, galaxy missing a TUI entry).

**Owns:** `crates/textweaver-theme/`, `docs/adr/0020-themes.md`, `docs/themes.md` (a user guide). Agent D2's TUI theme code (`crates/textweaver-tui/src/theme.rs`) moves onto this crate at integration: describe exactly what the TUI and app must change, with a patch-ready snippet, in your report.

**Deliverables:**
- A `Theme` model with semantic roles (background, text, dim text, heading levels, link, code, quote, selection, spoken word, spoken sentence band, find hit, bookmark, note, user highlight colors, status bar, focus, error), light/dark/high-contrast kind, and metadata; themes as TOML files, with Star's 23 palettes ported faithfully and built in, and user themes loaded from the config `themes/` folder (unknown keys preserved, errors reported per theme).
- WCAG checks: contrast ratios (WCAG 2.x relative luminance; report APCA as information) for every text role against its background and for the highlight roles, requiring 4.5:1 for text, 3:1 for large text and non-text indicators, and a high-contrast theme at 7:1; a test that every built-in theme passes, adjusting palettes minimally where Star's failed and documenting each adjustment.
- Renderers: terminal colors (truecolor, with 256-color and 16-color fallbacks chosen by detected capability, and a `NO_COLOR` mode relying on bold, underline, and reverse video so highlights never depend on color alone), CSS custom properties for Agent L's HTML templates (`prefers-color-scheme` pairs), and a plain RGB table the GUI can use.
- OS light/dark detection on Windows, macOS, and Linux (a pure function plus a small platform probe), with Star's `theme_for_os_scheme` behavior.
- ADR-0020 and `docs/themes.md` (how to pick, preview, and write a theme).

**Acceptance:** contrast tests for every built-in theme; round-trip TOML tests; snapshot of the CSS output; the palette list and each adjustment in the report.

### Agent S — Reading aids (added 2026-09-25 at Jon's request)

**Owns:** `crates/textweaver-aids/`, `docs/adr/0022-reading-aids.md`, `docs/reading-aids.md`.

**Status:** done on `wave2/s-reading-aids` (Friday, September 25, 2026); awaiting integration. `textweaver-aids` holds RSVP (clock-driven state machine, recognition point, WPM pauses, nine positions, terminal box), bionic reading, WCAG 1.4.12 text spacing with CSS, font settings with Star's three OFL reading fonts (not bundled; fetched by the GUI after asking), the reading ruler and current-line marks, difficult-word marking (no word list vendored: licences need Jon's decision), reading level, and rule-based syllable display with an offset map. One workspace dependency added: `unicode-width` (already in the tree through ratatui). Integration steps for Agents D3 (TUI) and K (GUI) are in the Agent S report and ADR-0022.
### Agent U — Settings import and export (added 2026-09-25 at Jon's request)

Jon asked for an easy way to import and export settings, preferring JSON. TOML stays the on-disk format.

**Owns:** `crates/textweaver-store/src/settings_io.rs` (and its tests), the settings validation hooks in `settings.rs`, `crates/textweaver-cli/src/cmd/settings.rs`, `docs/settings.md`, `fixtures/u/`.

**Status:** done on `wave2/u-settings-io`, not yet integrated. `tw settings export|import|path|reset` work; import validates, merges or replaces, backs up, and writes atomically; export then import changes nothing (tested with every setting non-default). The app's palette actions `export_settings` and `import_settings` are a verified patch for Agent D3 in `fixtures/u/d3-settings-palette.patch`.

### Agent W — Bundled fonts, PDF options, GUI font chooser, SCOWL (added 2026-09-25 at Jon's request)

**Owns:** `crates/textweaver-fonts/` (new), `third_party/fonts/`, `third_party/scowl/`, `tools/scowl_levels.py`, the fonts and PDF-option parts of `crates/textweaver-writers/`, `crates/textweaver-aids/src/{fonts,difficult}.rs`, `crates/textweaver-cli/src/cmd/convert_layout.rs`, the Fonts dialog in `crates/textweaver-gui/` (`fonts.rs`, `font_dialog.rs`, `tools/font-dialog-report.ps1`).

**Status:** done on `wave2/w-fonts-pdf` (Friday, September 25, 2026); awaiting integration. Atkinson Hyperlegible Next and Mono and OpenDyslexic are bundled (SIL OFL 1.1, 1.35 MB) and are the PDF default; PDF gains font choice by name, page size, margins, spacing, large print, page numbers on or off, a title page, a linked table of contents, working internal links, and alt-text warnings, all through `tw convert`; EPUB can embed a bundled font. The GUI has View, Fonts (checked through UI Automation in background mode; settings in a new `[display.font]`, since D3's `[reading_aids.font]` is not on main yet). SCOWL word levels are built into difficult-word marking. Agent V's commit 06f029f (writers wired into `tw convert`) is cherry-picked on this branch without its `--asciimath` line.
