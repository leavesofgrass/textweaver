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

**Privacy: hard rule (Jon, 2026-09-26).** Never send any of Jon's personal identifiers to any outside service. That means his email address or any part of it, his usernames or callsigns, his name, and his machine or account names. It covers HTTP headers (including User-Agent), URLs, query strings, request bodies, search queries, and API calls. Use only a neutral User-Agent: `textweaver-research (+https://github.com/leavesofgrass/textweaver)`, or the tool's default. Never build one from the session's user email. Never write an identifier into docs, commits, or anything public. If an identifier ever leaves the machine, stop and report it at once. This rule overrides every other instruction.

**Project.** textweaver is a Rust reimplementation of Star, an accessible text-to-speech document reader for students with print disabilities. Read, in order: `docs/history/plan.md` (including "Phase 0 amendments"), the ADRs in `docs/adr/`, your sections of `docs/history/star-parity.md`, and the Phase 0 code in the crates you own and the crates you depend on. The Phase 0 code **is** the contract: public types and signatures you must keep, with deliberately naive bodies you replace.

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

**Read:** ADR-0002, ADR-0005; `docs/history/star-parity.md` Part 1 sections 1–5 and 7; `fixtures/star-parity/README.md` and the JSON files.

**Deliverables:**
- Rope-based `Document` with markers and a per-kind `MarkerIndex` (index tables, binary search, wrap, level filter); `Document::apply` shifting markers.
- Units by segment iterators over the rope: grapheme, word (UAX #29, alphanumeric-bearing), sentence (UAX #29 refined by an abbreviation list, including `Dr.`, `Mr.`, `Mrs.`, `e.g.`, `i.e.`, `a.m.`, `p.m.`, `St.`, `pp.`), line, paragraph, marker units. Word and sentence rules parity-tested against the Star corpus with documented deltas.
- `navigate`, `go_to` (percent, line, char, start, end, heading n), `History` (forward entries truncated on a new jump; back from the live position remembers it so forward returns; no duplicate entries), `search` (plain, regex, case, whole word, wrap, direction; the backward bug fixed; decide and document overlap), `narrate::plan` (sentence-sized utterances; long sentences split at whitespace under `max_chunk_chars`; heading and table narration as `Inserted` spans per `NarrationPolicy` and verbosity; `skip_code`).
- Loaders: text (exists), markdown (pulldown-cmark events → canonical text + markers; front matter parsed into `meta`, not spoken; footnotes handled per `LoadOptions`), html (scraper; Star's skip list and alt rules, but not its unclosed-`<meta>` bug that empties documents); registry priorities; cache keyed by `(path, mtime, size, options fingerprint)` under the cache directory the caller passes.
- `tw text FILE [--format text|markdown|json] [--structure]`, `tw info FILE [--json]`, `tw search FILE PATTERN [--regex --case-sensitive --whole-word --json]`.
- `cargo xtask parity`: writes `docs/history/parity-report.md` comparing word tokens, sentence starts, and paragraph starts with `fixtures/star-parity/*.json`, aligned by word sequence (canonical texts differ by design, ADR-0002).

**Acceptance:**
- `cargo test -p textweaver-text -p textweaver-formats` green, including proptests (segment ranges tile or nest correctly; marker shifting keeps ranges ordered and in bounds; history invariants).
- `insta` snapshots of the loaded document (text + markers as JSON) for every fixture.
- `docs/history/parity-report.md` exists with every delta explained.
- `tw text fixtures/sample.md` prints the canonical text with headings, list items, and table rows on their own lines.

**Status:** Wave 1 deliverables done on `agent/a-text-formats` (Friday, September 25, 2026); awaiting integration. Parity report: `docs/history/parity-report.md` (0 unexplained deltas).

## Agent B — Speech

**Owns:** `crates/textweaver-speech/`, `crates/textweaver-cli/src/cmd/{speak,voices,backends}.rs`.

**Read:** ADR-0003, ADR-0004, ADR-0005; `docs/history/star-parity.md` Part 2 (all of it; section 6 holds the test vectors).

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

**Read:** ADR-0001, ADR-0006; `docs/history/star-parity.md` Part 1 section 6 (keys), Part 3 (all).

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

**Read:** ADR-0002, ADR-0003, ADR-0006; `docs/history/star-parity.md` Part 1 sections 3, 4, 6, 7 and Part 3 sections 2 and 6.

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

**Read:** ADR-0003, ADR-0004, ADR-0007; `tools/eci-spike/` (a working 32-bit program that drives `eci.dll`); `docs/history/star-parity.md` Part 2 sections 1–4; the Phase 0 code in `crates/textweaver-core` and `crates/textweaver-speech` (the `SpeechBackend` trait, `EventSink`, `RawEvent`, `Caps`, `BackendInfo`, `BackendFactory`).

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

**Read:** ADR-0003, ADR-0004, ADR-0007, ADR-0008; `tools/avspeech-spike/` (probe scripts and their results); `docs/history/star-parity.md` Part 2 sections 1–4; the Phase 0 code in `crates/textweaver-core` and `crates/textweaver-speech`.

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

**Read:** ADR-0003, ADR-0004, ADR-0007, ADR-0009; `docs/history/star-parity.md` Part 2 sections 1–4; the Phase 0 code in `crates/textweaver-core` and `crates/textweaver-speech`.

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
- **Toolchain:** `rust-version` is 1.92 (was 1.89; krilla needs 1.92); let-chains are expected (clippy's `collapsible_if`).
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

**Owns:** `crates/textweaver-text/`, `crates/textweaver-formats/`, `crates/textweaver-cli/src/cmd/{text,info,search}.rs` (`convert.rs` moved to Agent L on 2026-09-25), `fixtures/a/`, `xtask/src/parity.rs`, `docs/adr/0010-pdf-loader.md`, `docs/history/parity-report.md`.

**Deliverables:**
- EPUB loader (zip, OPF spine, NAV or NCX table of contents to `SectionBreak` markers with chapter titles, images as alt text) and DOCX loader (`word/document.xml`: heading styles, lists with levels, bold/italic/underline runs, `docPr` alt text, tables in place, footnotes), both on the shared builder, with `insta` snapshots on new fixtures you create (keep fixture files small and your own).
- PDF: choose between `lopdf`, `pdf-extract`, and `pdfium-render` (all in the workspace table; the unchosen ones are removed at integration), write ADR-0010 with the measured trade-offs (text quality on a multi-column fixture, reading order, speed, native dependencies), and implement the loader behind a `pdf` feature. Star's column-aware reading order (`docs/history/star-parity.md`) is the quality bar. The `paperback` feature stays a stub unless `paperback-core` is on crates.io and suits; report either way.
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

**Deliverables:** speech followed by `ReadingGeneration` with capability changes announced; notes and highlights on `DocState`'s typed fields (legacy `app_notes`/`app_highlights` migrated once) and moved with `DocState::shift`; `[keyboard] character_keys` applied and F9 wired; `confirm` for JSON-RPC actions that ask first and the pending question on the TUI status line; the library list (Alt+L) with bookshelf, recent files, and sidecar sync; `read_paragraph` and the select actions; `[speech.eci]`, `[speech.sapi]`, `[speech.apple]`, `[normalization.community_lexicon]`, and `[export]` mapped; modifier chords for the most used palette-only actions. Added later the same day: themes from `textweaver-theme` (Galaxy default, 23 built-ins plus user themes, OS following, cached terminal styles); `Command::SetCursor` and Play after "read current word" reading on (Agent K); reading aids from `textweaver-aids` (RSVP, bionic reading, reading ruler and current line, terminal text spacing, reading level, `[reading_aids]` settings); Agent U's settings export and import as actions.

**Status:** done on `wave2/d3-app-wiring` (Friday, September 25, 2026), merged with `main` at c4b0496; awaiting integration. Only `choose_voice` still says "not available yet" (the speech service offers no voice list). `tw export-audio` does not read `[export]` yet: `textweaver_app::subtitle_plan` is ready for B2's `export_audio.rs` (patch in the D3 report). Not done: syllable display and the math exploration mode (follow-ups). New tests: `crates/textweaver-app/tests/{wiring,aids}.rs`, `crates/textweaver-tui/tests/{wiring,aids}.rs`.

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
- `textweaver-vault`: export a document's notes and highlights to an Obsidian vault as Markdown notes with front matter and wikilinks, and import vault notes as documents with their links, ported from `star/obsidian.py` (see `docs/history/star-parity.md` Part 3 §4.8); build against C's Phase 0 store types and request what you need from C2.
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
- **Spoken math**: natural English with ClearSpeak-style wording and three verbosity levels (for example "x squared", "the fraction a over b end fraction" at high verbosity, "a over b" at low), built with `SpokenBuilder` so the offset map points each spoken word back to its source span (ADR-0005). This replaces Star's math normalization (see `docs/history/star-parity.md` Part 2 §5 and Star's `star/ttstext/mathspeech.py` at D:\star for its wording and its bugs); Star's test vectors for math must pass or have a documented, better wording.
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

### Agent V — Connect writers and math

**Owns (this task):** the writer seam in `crates/textweaver-convert/`, math in `crates/textweaver-render/` and `crates/textweaver-speech/src/normalize/`, the two math settings in `crates/textweaver-store/`, their mapping in `crates/textweaver-app/src/backends.rs`, and `tw convert`'s new flags. Branch `wave2/v-convert-math`.

**Status:** done on `wave2/v-convert-math` (Friday, September 25, 2026); awaiting integration. The converter uses `textweaver_writers::Writer` directly, and `Writers::builtin()` registers EPUB, DOCX, BRF, and PDF; writer warnings reach the summary; PDF checks for a font once per batch. The renderer and the speech pipeline use `textweaver-math`; `pulldown-latex` is gone; math speaks first in the pipeline with `[normalization] math_verbosity` and `asciimath_delimiter`. Pandoc: A2's loader (`wave2/a-formats`, not on `main` yet) should become the one Pandoc path at integration; see the Agent V report. Bundling Atkinson Hyperlegible for PDF is proposed, not done.

### Agent S — Reading aids (added 2026-09-25 at Jon's request)

**Owns:** `crates/textweaver-aids/`, `docs/adr/0022-reading-aids.md`, `docs/reading-aids.md`.

**Status:** done on `wave2/s-reading-aids` (Friday, September 25, 2026); awaiting integration. `textweaver-aids` holds RSVP (clock-driven state machine, recognition point, WPM pauses, nine positions, terminal box), bionic reading, WCAG 1.4.12 text spacing with CSS, font settings with Star's three OFL reading fonts (not bundled; fetched by the GUI after asking), the reading ruler and current-line marks, difficult-word marking (no word list vendored: licences need Jon's decision), reading level, and rule-based syllable display with an offset map. One workspace dependency added: `unicode-width` (already in the tree through ratatui). Integration steps for Agents D3 (TUI) and K (GUI) are in the Agent S report and ADR-0022.
### Agent U — Settings import and export (added 2026-09-25 at Jon's request)

Jon asked for an easy way to import and export settings, preferring JSON. TOML stays the on-disk format.

**Owns:** `crates/textweaver-store/src/settings_io.rs` (and its tests), the settings validation hooks in `settings.rs`, `crates/textweaver-cli/src/cmd/settings.rs`, `docs/settings.md`, `fixtures/u/`.

**Status:** done on `wave2/u-settings-io`, not yet integrated. `tw settings export|import|path|reset` work; import validates, merges or replaces, backs up, and writes atomically; export then import changes nothing (tested with every setting non-default). The app's palette actions `export_settings` and `import_settings` are a verified patch for Agent D3 in `fixtures/u/d3-settings-palette.patch`.
### Agent T — Audit and quick fixes (added 2026-09-25 at Jon's request)

**Owns:** `docs/history/audit-2026-09.md`, `docs/history/audit-2026-09/` (suggested patches for owned areas), `xtask/src/bench.rs`; small fixes in `textweaver-text`, `-speech`, `-core`, `-store`, `-editor`, and CI, each in its own commit with a test.

**Status:** done on `wave2/t-audit` (Friday, September 25, 2026); awaiting integration. Findings ranked in `docs/history/audit-2026-09.md`, with a low-hanging-fruit list. Fixed here: quadratic sentence segmentation and marker lookups in narration (a 10 MB document went from 13 s to 1.2 s to first speech, 0.23 s with the app patches; next sentence in a 50,000-item list from 4.6 s to 0.03 ms), a failing engine flooding and looping errors, rate changes heard sentences late on Eloquence and SAPI, corrupt state files overwritten, saves replacing links and read-only or non-UTF-8 files, reversed ranges panicking, recent paths, CI triggers and Omnivox on Windows and macOS. `cargo xtask bench` measures the hot paths. Seven tested patches for `textweaver-app` and `textweaver-tui` (D3) in `docs/history/audit-2026-09/`, also as one file: reading generations (S1), windowed planning (S3), single-key shortcuts and missing actions (S2), highlight latency in the terminal loop (S4), quit after save (S5), table mode (S6), ordered list items (S7). Checks: native Windows fmt, clippy, and tests (916 passed); Docker Linux with all features (922 passed).

### Agent X — Scripts and tools (added 2026-09-25 at Jon's request)

Jon asked for install scripts for Linux (any distribution: he uses Debian, Arch, and Fedora) and macOS, and for speech tools and other helpers.

**Owns:** `scripts/` (with `scripts/README.md` and `scripts/linux/textweaver.desktop`), `.github/workflows/scripts.yml`, the `-Skip` parameter of `tools/sapi_probe.ps1`, and the "install with a script" lines in `README.md`, `docs/install.md`, `docs/quickstart.md`, and `docs/dev/releasing.md`.

**Status:** done on `wave2/x-scripts` (Friday, September 25, 2026); awaiting integration. Installers for Linux (apt, dnf, pacman, zypper, apk; source build), macOS (release or source), and Windows (release or source), plus update, speech-check, doctor, dev-check, convert-folder, and voxin-docker. Real source installs pass in `debian:stable`, `fedora:latest`, and `archlinux:latest`; package names are also checked on openSUSE Tumbleweed, Alpine, and Ubuntu 24.04. On Fedora 44 and current Arch, `espeakng-sys` 0.3.0 does not build (its bindgen layout test for `_IO_FILE` fails), so the installer falls back to building without the `espeak` feature; that needs a fix in the speech crate.

### Agent W — Bundled fonts, PDF options, GUI font chooser, SCOWL (added 2026-09-25 at Jon's request)

**Owns:** `crates/textweaver-fonts/` (new), `third_party/fonts/`, `third_party/scowl/`, `tools/scowl_levels.py`, the fonts and PDF-option parts of `crates/textweaver-writers/`, `crates/textweaver-aids/src/{fonts,difficult}.rs`, `crates/textweaver-cli/src/cmd/convert_layout.rs`, the Fonts dialog in `crates/textweaver-gui/` (`fonts.rs`, `font_dialog.rs`, `tools/font-dialog-report.ps1`).

**Status:** done on `wave2/w-fonts-pdf` (Friday, September 25, 2026); awaiting integration. Atkinson Hyperlegible Next and Mono and OpenDyslexic are bundled (SIL OFL 1.1, 1.35 MB) and are the PDF default; PDF gains font choice by name, page size, margins, spacing, large print, page numbers on or off, a title page, a linked table of contents, working internal links, and alt-text warnings, all through `tw convert`; EPUB can embed a bundled font. The GUI has View, Fonts (checked through UI Automation in background mode; settings in a new `[display.font]`, since D3's `[reading_aids.font]` was not on this branch's base; after merging, `python fixtures/w/to-reading-aids.py` moves it to `[reading_aids.font]`, and its docstring lists the five small conflicts, all checked on a trial merge). SCOWL word levels are built into difficult-word marking. Agent V's commit 06f029f (writers wired into `tw convert`) is cherry-picked on this branch without its `--asciimath` line.
### Agent D4 — Audit fixes (T's patches S1 to S7 and the list after them)

**Owns:** the fixes from `docs/history/audit-2026-09.md` on branch `wave2/d4-audit-fixes`, in `textweaver-app`, `-tui`, `-cli` (`open`, `serve`, `export_audio`), `-speech`, `-formats`, `-eci`, and `-sapi`, each with a test.

**Status:** done on `wave2/d4-audit-fixes` (Saturday, September 26, 2026); awaiting integration. S1 and S2 were already fixed by D3 (T's tests pass on `main`); S3 to S7 applied, adapted. Also: the first list item and "k of n" spoken, repeated status messages repeated, a rotating log file with `--log`, AltGr typing, a warning before saving over a file changed on disk and a reload offer, binary files refused and UTF-16 without a BOM decoded, GFM task lists and HTML text, engine crash resume and a stall watchdog, flaky timing tests fixed, `tw export-audio` reading the settings, `SpeechService::voices()` and Choose voice (Alt+V), speech-dispatcher found without `XDG_RUNTIME_DIR`, and `espeakng-sys` replaced by hand-written declarations (the `espeak` feature builds on Fedora 44 and current Arch, without clang). Bench on a 10 MB file: next sentence while reading 248 ms to 2 ms, open to first speech 705 ms to 226 ms, entering edit mode 462 ms to 151 ms. Notes for integration: the root `Cargo.toml` still lists `espeakng-sys` (unused now; remove it); Agent X's installers can drop clang and their no-espeak fallback.

### Agent Y — Documentation sweep and interactive pages (added 2026-09-26 at Jon's request)

**Status:** done on `wave2/y-docs` (Saturday, September 26, 2026), merged with `main` at 89f270e (the roadmap); awaiting integration. Every doc checked against the code and the programs' help: README, changelog (every merge since 0.1.0-alpha.3), quick start, install, the existing guides, scripts README, Docker, releasing, a dated status update in every ADR, status notes on the plan, audit, and Star references, and a status per item in `docs/star-gaps.md`. New: `docs/README.md` (index), `docs/dev/architecture.md`, `CONTRIBUTING.md`, a full settings reference in `docs/settings.md`, and guides for reading, editing, notes, the library, speech, math, citations, audio export, the vault, dictation, JSON-RPC, troubleshooting, and screen readers; every doc ends with See also. Six accessible pages in `docs/site/` with `tools/gen_site_data.py` (data from cargo metadata, `docs/keyboard.md`, and the themes); `tools/check_links.py` and `tools/check_site_a11y.py`; all three run in `scripts/dev-check` and a new CI `docs` job. Code changes are doc comments and Cargo descriptions only. Not changed: `docs/keyboard.md` and `docs/history/parity-report.md` (generated; no See also), and this log's history.

### Agent P1c — CI, releases, and notices (Phase 1)

**Owns:** `.github/`, `xtask/`, the root `Cargo.toml`, `deny.toml`, `about.toml`, `about/`, `.gitignore`, `THIRD-PARTY-NOTICES.md`, `docker/Dockerfile`, and the macOS font fix in `crates/textweaver-gui`.

**Status:** done on `phase1/c-ci-release` (Saturday, September 26, 2026), merged with `main` at 0031aaa; awaiting integration. `THIRD-PARTY-NOTICES.md` comes from `cargo xtask notices` (cargo-about, plus a hand-written data section: the OFL fonts, SCOWL, the IBMTTS dictionaries, hayagriva's CC BY-SA styles, and the Adobe AFM terms). `cargo xtask dist` packages it with `licenses/`, every user guide from `docs/README.md`, and `docs/site/`, and fails if a notice is missing. `deny.toml` passes: the licences are clean, and five upstream advisories are ignored with reasons. The macOS GUI hang: wx's `AddPrivateFont` logged errors, which opened a modal box that kept the process alive. macOS now skips it, and `--exit-after` has a watchdog; this is unverified until the GUI job runs. Workflows gain concurrency, timeouts, Node 24 actions, `--locked`, rust-cache saving on main only, the wx cache keyed on `wxdragon-sys`, the PowerShell lint loop, the `keyboard`, `deps`, and `notices` checks, cargo-deny, a Windows release job, one checksums job, and provenance attestations. New: `cargo xtask deps --check`, `cargo xtask release X.Y.Z [--dry-run]`, `[profile.dist]` with fat LTO, `strip = "symbols"`, Dependabot, issue forms, a PR template, and `.github/SECURITY.md`. Removed: seven unused workspace dependencies, and clang from CI, the Dockerfile, and the Linux installer. Checks: native Windows, 1,591 tests passed; Docker with all features, 1,600 passed (image rebuilt without clang); GUI clippy and tests on Windows; actionlint clean.
### Agent P1b — App safety, settings that work, authoring quick wins (Phase 1)

**Status:** done on `phase1/b-app-authoring` (Saturday, September 26, 2026), merged with `main` at 0031aaa; awaiting integration. All 28 items and the docs sweep's three extras are done, each with a test. Save As names from the title and asks before overwriting; panics and signals (ctrlc, plus a Windows close handler) keep unsaved work and restore the terminal; saves retry on Windows; failed snapshots back off and are said once; bookmarks are not called set when saving failed; positions and bookmarks carry text anchors; big selections are summarized; the terminal uses a 256-entry ring announcer; highlight colours and favourite voices work; a test catches settings nobody reads. New keys: 1 to 6 and `! @ # $ % ^` (heading levels), `Shift+W` and `Alt+Shift+Y` (say position, moved from `%`), `Alt+Shift+T` (word count), `Alt+Shift+K` and `Shift+K` (link address), `Alt+N` (add note), `Ctrl+C` (copy), `Ctrl+X` in edit mode (cut), `Tab` and `Shift+Tab` in edit mode (table cells), `Shift+F9` (typing echo). Clipboard is OSC 52 only (no `arboard`). The speech-thread-death hook (`App::speech_thread_died`) waits for Agent P1a's API, marked `TODO(P1a speech thread death)`. Tests: 1,624 pass natively on Windows, 1,632 in Docker with all features.
### Agent P1d — Robust loaders and command-line tools (Phase 1)

**Status:** done on `phase1/d-loaders-cli` (Saturday, September 26, 2026), merged with `main` at 0031aaa; awaiting integration. Hostile DOCX list starts and PDF page labels clamped; HTML, EPUB, and DOCX walkers stop at 256 levels (deep XML flattened before roxmltree, which recurses per element); binary files refused from their first 8 KB; one Pandoc path (the formats loader, `--sandbox`, stderr on its own thread, 120 s timeout, `TEXTWEAVER_PANDOC` kept), registered only by `tw convert`; Markdown math, GFM alerts, wiki links, heading attributes, and new `Strikethrough`, `Rule`, and `Math` markers (core change); math typeset in PDF, DOCX, EPUB, and braille; `tw convert` formats citations with `textweaver-cite` and appends References (`--bibliography`, `--style`); `tw info` 2.3-2.7 s to 0.7-0.8 s on 10 MB; `tw backends` probes once; `tw backends`, `tw voices`, `tw speak` read the settings; timing tests use signals; vault imports reach the library and keep links to notes outside the vault; loader property tests and `fuzz/`. Native: 1,630 tests passed; container (all features, Pandoc 3.1): 1,637 passed. Left for others: the app still probes SAPI and DECtalk once at registration (P1b).

### Agent P1a — Speech never loops, never dies silently (Phase 1)

**Status:** done on `phase1/a-speech-stability` (Saturday, September 26, 2026), merged with `main` at f81efa3; awaiting integration. Crash recovery restarts only after progress past the resume point, at most three times per sentence, then stops with a message. A speech-thread panic is caught and reported; `SpeechService::is_alive`, `failure`, and `poll_status` tell a dead thread from an empty queue, and the app's `poll_speech` now calls `App::speech_thread_died` (the P1b TODO is wired). Engine hosts exit within 1.5 s of their input closing even when the engine is stuck, join a kill-on-close Job Object on Windows, and get `PR_SET_PDEATHSIG` on Linux; a host that already failed is killed at once. Host stderr is always drained (lossy UTF-8) and hosts log with `serve::log_line`, which never panics. SAPI maps control characters and NUL to spaces. Frames over 16 MiB are refused before sending and skipped by hosts. ECI works out its installed dialects once per host start. Timing tests wait for signals and deadlines. Stretch: a stalled audio output is reopened after 1 s (backing off to 8 s); speechd Stop no longer waits for the server. ADR-0003 and ADR-0012 updated. Not done: availability probes are still not cached in the app's registry (the registry was not touched). Tests: 1,723 pass natively on Windows, 1,729 in Docker with all features; the changed timing tests ran 40 times each natively and in Docker (280 runs each, 0 failures).

### Agent P2c — Screen reader coexistence (Phase 2)

**Status:** done on `phase2/c-screen-readers` (Saturday, September 26, 2026); awaiting integration. `[accessibility] mode` (self-voicing, hybrid, screen-reader) routes every message, echo, caret move, and piece of read text through `textweaver_a11y::route`, so nothing is spoken and shown at once with a screen reader; Alt+Shift+A cycles and saves it, `--mode` sets it for a run, `--no-speech` is screen-reader mode (the JSON-RPC server stays self-voicing). Screen-reader mode never speaks: read text goes to the status line narrated (math in words), and Space steps a sentence at a time on the status line (`say_all = "voice"` uses the voice). Hybrid voices reading and leaves messages, echo, and caret moves to the screen reader. `quiet_screen` freezes the title line's position and keeps read text off the status line; `cursor = "status"` parks the cursor there. Screen reader detection (`textweaver_a11y::detect`: Windows flag and processes, VoiceOver, Orca) offers hybrid once on the first run. `[keyboard] preset = "screen-reader"` (`h`, `l`, `k` keys) with `cargo xtask keyboard` documenting it. Store keeps its own setting enums (store depends only on core); the app maps them. `docs/screen-readers.md` has the modes, NVDA and JAWS settings (to verify on Jon's machine), Windows Terminal clashes checked against 1.24, and a checklist. `docs/site/` regenerated (it was stale on main). New action `cycle_access_mode` (P2b adds actions too: expect a merge in `action.rs` and `app.rs`'s match). Tests: 1,746 pass natively on Windows, 1,752 in Docker with all features; clippy, rustdoc, `cargo xtask keyboard --check`, and the link check are clean.

### Agent P2b — Authoring and navigation (Phase 2)

**Status:** done on `phase2/b-authoring` (Saturday, September 26, 2026), merged with `main` at d39ce4b (P2c's screen-reader mode; authoring output goes through its routes); awaiting integration. All twelve items are done, each with tests. Markdown source is parsed with pulldown-cmark offsets, so headings, lists, links, tables, the outline, and "say position" work while writing. It is parsed again after a 300 ms pause, and large files in the background. The paired block markers replace the word aligner for entering and leaving edit mode: on 10 MB, entering went from 276 ms and 213 MB to 52 ms and 121 MB (`cargo xtask bench`). New keys: Alt+O (outline, filters as you type), Alt+H and Alt+Shift+H (terminal headings, also in edit mode), Ctrl+Alt+arrows (table rows and cells), Alt+Shift+F (follow link or footnote), Alt+M, Alt+Shift+M, and Alt+J (spelling), Alt+Shift+V and Alt+Shift+N (verbosity, punctuation), and in edit mode Ctrl+A, Alt+Backspace (GUI Ctrl+Backspace), Ctrl+Delete, Ctrl+V, Alt+C (insert citation), plus Alt+Shift+D (add reference). New palette commands with no keys: export html, pdf, docx, epub, and brf; preview in browser; listen rendered; insert bibliography; check citations; import references; export study sheet; new from template. Find and replace now goes one match at a time. Shared files touched: small arms in `app.rs`, a status hook and `read_planned` in `playback.rs`, the `on_saved` hook after "Saved" in `edit.rs` (P2a: keep it when saves move to the writer thread), the TUI list filter in `ui.rs`, and `Command::FilterList` plus four `PromptPurpose` variants. Tests: 1,797 pass natively on Windows, 1,803 in Docker with all features. fmt, clippy, rustdoc, `keyboard --check`, `deps --check`, the link check, and the site check are clean.
### Agent P2d — Releases, quality gates, and binary size (Phase 2)

**Owns:** `.github/`, `xtask/`, `scripts/`, `fuzz/`, `docker/`, the root `Cargo.toml`, and feature flags in crate manifests.

**Status:** done on `phase2/d-release-quality` (Saturday, September 26, 2026), from `main` at c2d5eed; awaiting integration. Linux: `cargo xtask appimage [--docker]` builds `textweaver-VERSION-linux-x86_64.AppImage` (17.5 MB, with a `.zsync` file; the tarball is 18.1 MB) and the tarball on Ubuntu 22.04 (glibc 2.35), with appimagetool 1.9.1 and the type 2 runtime pinned and checked against GitHub's digests; `AppRun` runs `textweaver`, or `tw` through a `tw` link or `--tw`, and offers `--install` and `--uninstall`. espeak-ng is loaded at run time with `libloading` (`sys.rs` only, in P2a's crate; it also works on Windows now). Both packages pass on Debian stable, Fedora, and Arch, with and without espeak-ng (`docker/appimage/test-distros.sh`), and `install-linux.sh --release TAG|latest` installs and uninstalls either. `release.yml` has a Linux job; the checksums job waits for it. Bench gate: `--baseline` and `--max-ratio` gate peak heap and allocation counts (two runs differ by 4% at most; `--quick` takes about 20 s once built), plus startup timings; `bench.yml` compares with main's artifact. `nightly.yml`: fuzzing (9 targets, 3 new: keymap, state, frame), Miri, AddressSanitizer, release tests, MSRV, Docker, and `cargo xtask soak` (passes: 2 minutes in Docker, 31 host kills, heap 35.7 to 35.8 MB); `cargo hack` weekly. `ci.yml`: the 32-bit DECtalk host, fake-host tests against all three 32-bit hosts, and silent real-engine jobs (espeak-ng, SAPI). Size: ammonia held on 4.1.4 so one html5ever is linked (10 duplicate crates gone), comrak is a default feature of `textweaver-render`, and `deps --check` keeps the reader off the conversion and citation stack; `tw.exe` 32,625,152 to 32,299,008 bytes, `textweaver.exe` unchanged at 11,185,664, startup unchanged. Not done: only the styles offered bundled (hayagriva's archive is all or nothing, and every style is reachable by name), aarch64 AppImage. Found: krilla 0.8 declares Rust 1.92, so the MSRV job uses `--ignore-rust-version` (the workspace checks on 1.89). Checks: native Windows 1,767 tests passed, Docker with all features 1,774 passed; clippy, rustdoc, fmt, `cargo deny`, actionlint, and shellcheck clean.

### Agent P2a — Reliability (Phase 2)

**Status:** done on `phase2/a-reliability` (Saturday, September 26, 2026), merged with `main` at 0c7b694; awaiting integration. All ten items, each with tests. One background writer (`app/src/writer.rs`, `writes.rs`) does saves (a rope clone handed over; the file version is checked there, so saving over outside changes still asks), autosave snapshots in order with their locks, positions, bookmarks, notes, sidecars, the bookshelf, and the two-second disk check; quitting waits at most 10 s and says "Still saving" after 300 ms; JSON-RPC requests finish their writes before answering. Voices are listed once into a `VoiceCache` (SAPI in the background; `select_voice` no longer lists tokens); Alt+V says "still loading" and opens when they arrive. ECI, SAPI, and DECtalk restarts and the first 32-bit SAPI voice start inside `poll` (`enginehost::HostStart`), Stop and Pause work meanwhile; the audio device opens on its own thread. Restart Speech (Shift+F8, new action) and one automatic restart after the speech thread dies (`App::set_speech_starter`, set by the TUI setup). Probes are cached per process (`register_cached`). Positions, bookmarks, notes, and highlights are found again after outside edits (`app/src/relocate.rs`; state keeps a text stamp; not-found items are marked), announced once. Backspace and Delete remove graphemes; undo capped by `[editing] undo_steps` and `undo_memory_mb`. Find streams over the rope and keeps 10,000 matches around the cursor; edit-mode find folds without a string per character and P2b's replace searches once per step. speechd makes no round trips after connecting; DECtalk synthesizes a sentence at a time. The library scans on a thread with a count. Bench, 10 MB (UI thread, before to after): autosave 41 to 0.6 ms (snapshot written off-thread in 34 ms), Ctrl+S 75 to 0.01 ms, position save 3.8 to 0.02 ms, find "the" 15 to 5 ms, find a rare word 12 to 2 ms, edit-mode replace count 1,035 to 69 ms and replace all 2,411 to 137 ms. Left: P2b's `on_saved` misspelling count runs on the input thread when a save is reported (about 0.6 s on 10 MB); first engine starts still wait in `new`; DECtalk does not stream within a sentence. Tests: native Windows 1,854 passed; Docker with all features 1,861 passed; the new timing tests ran 40 times each (0 failures).

### Agent DOC2 — Documentation refresh after Phase 2

**Status:** done on `docs/phase2-refresh` (Saturday, September 26, 2026), from `main` at c4e7179; awaiting integration. Every doc outside P2e's files checked against the code and every `tw` and `textweaver` `--help`: README, the index, quick start, install (0.1.0-alpha.3 has no Linux package; the AppImage starts with the next release), settings, speech (`tw speak`, `voices`, and `backends` read the settings; favourite voices; verbosity and punctuation cycling; access modes), library, notes (marks found again after outside edits), troubleshooting, architecture (the app's render, convert, and cite edges; `route`; the writer thread; `deps --check`), converting, math, vault (wiki links are links; imports reach the library), dictation, JSON-RPC (self-voicing server, writes before answers, four new prompt purposes), CONTRIBUTING (no espeak-ng or speech-dispatcher headers to build; the CI-only checks), the scripts README, the changelog (P1b, P1d, P2b, P2c), and dated status lines in 15 ADRs (ADR-0014 records the Xilem choice). `docs/roadmap.md` has a dated status for each phase and the Wave 3 plan; `docs/star-gaps.md` names the Wave 3 agent for each gap; the site's feature list (`tools/gen_site_data.py`) reflects Phase 2 and the pages are regenerated. No Rust code changed. Checks: `check_links.py`, `gen_site_data.py --check`, and `check_site_a11y.py` pass. Stale text in P2e's files (reading, citations, screen readers) is listed in the DOC2 report.
### Agent P2e — Gaps, NVDA and JAWS keys (Phase 2)

**Status:** done on `phase2/e-gaps-keys` (Saturday, September 26, 2026), merged with `main` at d192f15; awaiting integration. The default keymap is NVDA's quick navigation (`h`, `1`-`6`, `l`, `i`, `t`, `k`, `q`, `s`, `g`, `d`, Shift for previous, `Backspace` back, `Alt+Down`/`Alt+Up` sentences, `Ctrl+Down`/`Ctrl+Up` paragraphs); the old keys are `preset = "classic"` and `screen-reader` reads as default; "What changed" heads `docs/keyboard.md` and CHANGELOG. The digit row follows the physical key: the TUI peeks Windows console input records (VK_0-VK_9, Windows Terminal and conhost), else knows US, UK, German, Spanish, Nordic, and Italian shifted digits, and French with `[keyboard] digit_row = "azerty"`. Citations: `[reading] citations = "off" | "words"` (Alt+Shift+Q), Elided or Expanded spans through `textweaver_text::plan_with`, the renderer's flavor rule shared as `counts_as_citation`. Preview: "Preview updated. Press F5 in the browser."; `[preview] auto_reload` and `live` (127.0.0.1 server, secret path, SSE reload to the heading nearest the caret, stopped on close and quit). Export progress after 2 s, then every 10 s. Syllables (Alt+Shift+Z) drawn between chars with exact highlights; difficult words (Alt+Shift+J) underlined and named at high verbosity; `RESERVED_SETTINGS` is empty. Math exploration (Alt+Shift+X). Windows Terminal: RSVP gains Alt+Shift+PageUp/PageDown, terminal add-reference is Alt+B; checked against WT 1.24's own defaults.json (Ctrl+Alt+Left, table previous column, is MoveFocusPrevious there: documented). PDF strikethrough; DOCX OMML to LaTeX. macOS GUI says once when a built-in font falls back. Tests: 1,897 pass natively on Windows (23 ignored); Docker with all features 1,904 pass, 1 fails (`textweaver-theme` `truecolor_and_256_keep_the_floor`, from main's c0acf8f theme colours, not this branch); GUI clippy clean and 20 tests pass; fmt, clippy, rustdoc, `keyboard --check`, `deps --check`, links, and site check clean. Remaining: aarch64 AppImage, signing, Nemeth braille, native clipboard; syllables and difficult words in the GUI.

## Wave 3 (planned 2026-09-26; starts after Agent P2e merges and Docker is restarted)

Jon asked to go "full steam ahead" with six agents.

**Setup:** each agent works in its own worktree and branch, `wave3/<letter>-<name>`, from `main`. Each reads the shared preamble above, `docs/roadmap.md`, `docs/dev/architecture.md`, and the ADRs for its area.

**Checks, natively on Windows:**
- fmt;
- clippy with `-D warnings`;
- tests with `--no-fail-fast`;
- rustdoc with `-D warnings`;
- `cargo xtask keyboard --check` and `cargo xtask deps --check`;
- `python tools/check_links.py` and `python tools/gen_site_data.py --check`.

Also run the Docker all-features clippy and tests once, at the end (Jon's memory rule).

**Rules for new code:**
- Route every new announcement through `textweaver_a11y::route`.
- Make every save through the writer thread.
- New keys must pass the keymap's conflict and WCAG 2.1.4 tests, and follow the NVDA and JAWS-style defaults (Agent P2e).

**Reporting:** plain, short sentences, with headings and lists and no tables. Add a status line under your own heading here.

Areas are split to keep merges small.

**Spirit of Wave 3 (Jon, 2026-09-26).** textweaver is an experimental alpha, for Jon's own use first. The aim is to push the envelope with Rust, not to be conservative.
- Prefer pure-Rust, in-process solutions over subprocesses and C or C++ dependencies whenever they are viable.
- Accept alpha crates and API churn.
- Keep the tests and CI gates: they are what let us move fast.
- Record each bold choice, and its fallback, in an ADR.

### Agent W3a — App core for the GUI (Phase 3, first half)

**Status (Saturday, September 26, 2026): done, on `wave3/a-app-core`.** All six items, with ADR-0024. `DocWindow` (paragraph-aligned, slides and recentres, UTF-16, UTF-8, and char offsets; `DisplayIndex` is now the rope, 29 µs instead of 1.1 s and 40 MB on 10 million characters); `ListModel` and `PromptModel` in the app, which the TUI now only maps keys to and draws; `App::set_waker` and `SpeechService::set_waker`; `Command::ReplaceRange`; `SettingsSchema` from the store's keys, a settings screen (`Shift+F10`, GUI `Ctrl+,`), and JSON-RPC `settings_schema`, `get_setting`, `set_setting`, `list_state`, `list_key`, `prompt_state`, `prompt_key`; large files open in the background with progress and cancel (the key returns in 0.3 ms instead of 2.7 s on 10 MB, debug build), settings saves on the writer, the misspelling count after a save on a helper thread, and the speech engine's first start in the background. Small edits outside the app: `SpeechService::set_waker` (speech), `DisplayIndex` (text), the Settings action (keymap). Tests: 1,923 pass natively on Windows and 1,936 in Docker with all features; clippy clean in both.

**Owns:** `crates/textweaver-app` (new modules), and the list and prompt code moved out of `crates/textweaver-tui`.

1. **Document window model.**
   - It holds about 500,000 UTF-16 units at a time, aligned to paragraphs.
   - It extends while reading and recentres on jumps.
   - It maps control offsets to `CharPos` through `DisplayIndex`.
   - The TUI keeps its own slicing but shares the model where that fits.
2. **List and prompt state in the app.** Move the selected item, type-to-filter, the "k of n" announcements, and prompt editing out of `tui/src/ui.rs` and `widgets.rs`. The TUI, the GUI dialogs, and JSON-RPC then share one implementation.
3. **A waker.** A callback or channel, so a frontend reacts to speech events instead of polling every 30 ms.
4. **`Command::ReplaceRange`**, for edits made in a native text control.
5. **Settings schema,** generated from the store: labels, help, ranges, steps, and choices.
   - It drives a new TUI settings screen (F10 or the palette), the GUI dialog, and RPC.
6. **Work off the input thread.**
   - Open documents in the background, with progress and cancel.
   - Move settings saves to the writer thread.
   - Move P2b's misspelling count on save (0.6 s on 10 MB).
   - Make an engine's first start non-blocking, not just restarts.

### Agent W3b — The Xilem GUI (Phase 3; Jon chose Xilem on 2026-09-26)

**Status (Saturday, September 26, 2026):** done on `worktree-agent-a7c54b19d90c611c7` (W3b's work continued by W3b-3 from `wave3/b-xilem-gui`), main merged in (W3a, W3c, W3e, W3f); awaiting integration and Jon's session. The record is ADR-0027 (renumbered from 0023). `textweaver-xilem` has `DocumentView` (text runs with stable ids, the spoken word as a background-colour attribute, the caret as the selection), the live-region announcer, in-window dialogs on W3a's list and prompt models (prompts keep the app's history, Tab completes paths), a settings dialog built from W3a's schema (check boxes, sliders, combo boxes, edit fields, per section), the command palette, the font chooser, Galaxy by default with Galaxy Light and high contrast, windowing through `DocWindow`, and `cargo xtask gui-dist`. A Masonry patch sends screen-reader actions on text runs, list options, and settings rows to the widget that owns them (they were a debug panic). The UI Automation report passes on Windows 11, settings dialog included; the AT-SPI check and the macOS smoke run are CI's (Docker is off). 10 million characters open in about 78 ms; a highlight move takes 0.27 ms (median). Screenshots at 100% and 200% are in `docs/screenshots/xilem-gui/`. wxDragon stays until Jon's NVDA and JAWS session passes. `THIRD-PARTY-NOTICES.md` was regenerated with GPL-3.0-or-later accepted for the run only: main's `about.toml` and `deny.toml` need it for W3f's espeak-ng crates.

**Jon's checklist for the Xilem GUI** (NVDA, then JAWS; the release binary is `textweaver-xilem.exe`):

1. Open a document: `textweaver-xilem.exe path\to\file.md`. The window title and "Document" are read, and the arrows read by character, word, and line.
2. Press Space to read. Does the screen reader's caret follow the spoken word? With NVDA's formatting reports on, is the word's background colour reported?
3. Pause, Stop, and the rate keys. Is each announcement heard once, in NVDA and in JAWS? If JAWS is silent, note it: that decides the UIA Notification question in ADR-0027.
4. Press Ctrl+O. Type part of a path and press Tab: is the completion read? Press Up and Down: are earlier answers recalled? Escape closes.
5. Press Ctrl+Comma for Settings. The first setting is read with its role and value ("Rate, slider, 265 words per minute").
   - Up and Down move; Left and Right change. Is each new value said once, not twice?
   - Space turns a check box on or off. Enter on "Voice" opens a prompt; Escape comes back to the same setting.
   - Ctrl+Page Down moves to the next section, and its name is said. Shift+Tab reaches the section list.
   - Escape closes and says "Settings closed."
6. Press F2 for the command palette. Type "set": is the number of matches said? Up and Down move through the matches; Enter runs one.
7. Fonts: choose a family, then a size. Is the change said and drawn?
8. A list dialog (bookmarks or help): arrows, Home, End, and first letters.
9. NVDA object navigation inside the settings form and a long list: are settings below the fold reachable? (AccessKit leaves out clipped rows until they scroll into view.)
10. Compare by ear: `--select-spoken`, `--edit-role`, `--app-list-announcements`, and `--settings-list`.

Jon chose Xilem, from Linebender, for the GUI on every platform, to "keep as much of it Rust as I can". The stack is:
- Xilem and Masonry for the widgets;
- Vello for rendering;
- Parley for text layout;
- AccessKit for accessibility;
- winit for windows.

The wxDragon spike (`crates/textweaver-gui`, ADR-0014) stays as a fallback. It is removed only after the Xilem GUI passes the same accessibility checks.

**Owns:** a new crate, `crates/textweaver-xilem` (binary `textweaver-gui`, once it replaces the spike), plus GUI packaging in `xtask` and the workflows.

**Read `docs/research/xilem-gui.md` first.** Key points:
- Write your own `DocumentView` widget against Masonry and AccessKit. Masonry's read-only text can't take focus, and Parley's editor has a single style.
- Patch the workspace to `accesskit_winit` 0.34 or later, so Orca detects the app.
- Build a live-region announcer node, using a fresh node for each repeat.
- Use windowing with stable run IDs.
- Depend on Masonry more than on Xilem.
- Keep wxDragon working on Windows until the UI Automation report passes.
- Propose upstream PRs for the gaps.
- **Visual starting point (Jon, 2026-09-26):** Jon likes the dark look of Xilem's `to_do_mvc` example (github.com/linebender/xilem, the `examples` folder). Use its layout, spacing, and dark styling as the base for the main window and dialogs.
  - Map textweaver's Galaxy theme onto it. Not every theme needs to meet WCAG AA (Jon, 2026-09-26), but Galaxy, Galaxy Light, and the high-contrast themes must.
  - Keep the same structure for Galaxy Light and high contrast.
- **Look and feel (Jon, 2026-09-26): a highly polished, modern, dark-mode look is a goal in its own right.**
  - Galaxy is the default.
  - Keep the spacing, type scale, and alignment consistent.
  - Use subtle surfaces and elevation for panels, dialogs, and the status bar.
  - Rounded corners, clear focus rings (at least 3:1 against their background), and smooth, restrained motion that respects the reduced-motion setting.
  - Crisp text rendering at every DPI, with good default fonts from the bundled set.
  - No clutter.
  - Produce screenshots (`--background --screenshot PATH` or a test helper) at 100% and 200% scale for review.
- **Manual accessibility testing:** Jon tests with screen readers himself. Automated checks (the UI Automation report, AT-SPI dumps) still run in CI. After each milestone, write down exactly what Jon should try, with a short checklist.

1. **ADR-0027, "Xilem GUI"** (planned as ADR-0023; renumbered because main took 0023 to 0026). It supersedes ADR-0014 and records:
   - the choice;
   - the Xilem version pinned;
   - the accessibility bar;
   - the fallback plan;
   - what we may need to contribute upstream to AccessKit or Masonry.
2. **The main window.**
   - A document view built on W3a's window model, with the caret following speech, the selection, and highlight attributes, all exposed through AccessKit. Read-only first, then editable.
   - Announcements through AccessKit live regions.
   - Labelled play and stop buttons, a status bar, and a menu, or a command palette where menus are missing.
   - Keyboard-only operation, using the keymap's GUI layer with native caret keys.
3. **Dialogs.** Find, go to, bookmarks, notes, voices, library, command palette, help, and a settings dialog from W3a's schema. Build them on the app's list and prompt model.
4. **Themes and fonts.**
   - Themes through `Theme::rgb_table`, including Galaxy.
   - A system or high-contrast theme.
   - The bundled fonts, loaded straight into Parley, so no OS registration is needed.
   - The font chooser, ported from the wx spike.
5. **Accessibility checks on every OS.**
   - Port the UI Automation report tool (`crates/textweaver-gui/tools/uia-report.ps1`) to the new window. It checks names, roles, text, caret movement while reading, and announcements.
   - Add an AT-SPI tree dump (pyatspi) on Linux under Xvfb.
   - Add a macOS smoke test.
   - Run everything with `--background`, and never steal focus.
   - Record where AccessKit falls short, especially text-range support in UI Automation, and propose fixes or upstream contributions.
6. **Large documents.** A 10-million-character document opens in under 300 ms, and the highlight moves in under 30 ms per word. Measure both.
7. **Packaging.** The GUI binary goes into the Windows zip, a macOS `.app`, and a Linux AppImage of its own. There are no GTK or wxWidgets dependencies; winit and Vello only need the system's graphics stack.

### Agent W3c — Architecture consolidation

**Status:** done on `wave3/c-architecture` (Saturday, September 26, 2026), main merged in at 782b455 (W3a); awaiting integration. Store depends only on core: the `[reading_aids]` settings are plain data in `textweaver_store::reading_aids`, and aids converts them (`From` both ways, tested for defaults, round trips, and identical TOML). Font choice and resolution moved to `textweaver-fonts` (`choice`, and `system::installed`, one scan per process, used by the PDF writer); aids re-exports it and keeps `describe_font` and `font_css`. One notes model: the vault uses the store's `Note`, `Highlight`, and `Relation`; `RelationType` moved to store; the app's `app_notes` shim is gone. New crate `textweaver-engines` holds the backend registry, engine options, `service_config`, and the engine features; the app re-exports them. In-reader export, preview, and citations are the app feature `publish` (on by default, forwarded by the reader, asked for by `tw`, the GUI spike, and the benchmarks); without it `src/lean/` stand-ins say "not in this build". The workspace table declares `textweaver-app` with default features off. `cargo xtask deps --check` resolves features and refuses the conversion and citation edges for the reader built with `--no-default-features`; no store exception is left. Docs: `dev/` (architecture, building, testing, docker, releasing), `adr/README.md` index, `history/` (plan, tasks, audit, star-parity, parity report). Tests: 1,933 pass natively (24 ignored), 235 app tests with `publish` off, 1,940 in Docker with all features (19 ignored); fmt, clippy (both feature sets), rustdoc, `keyboard --check`, `deps --check`, and the site check clean. Links: one broken, `CLAUDE.md` still names `docs/tasks.md` (now `docs/history/tasks.md`), left for Jon. Not built here: the wxDragon GUI spike (its `fonts.rs` changed; needs libclang and the wxWidgets download).

**Owns:** crate manifests and module moves across `store`, `aids`, `vault`, `fonts`, and a new `textweaver-engines`. Keep app edits to imports and registry wiring, to avoid conflicts with W3a.

1. **Take `store` off `aids`.**
   - The settings types move into `store`, and `aids` converts them.
   - `cargo xtask deps --check` then allows no exception for store.
2. **One notes model.**
   - `textweaver-vault` uses the store's `Note` and `Highlight` types directly.
   - Remove the app's `app_notes` migration shim, which is one release old.
3. **Font resolution in one place:** `textweaver-fonts`, used by aids, the writers, and the GUI.
4. **A `textweaver-engines` crate** for the backend registry and engine features.
   - The TUI, CLI, export, and GUI share it.
   - `app` no longer depends on each engine crate directly.
5. **Make in-reader export, preview, and citations an app feature.**
   - It is on by default and in releases.
   - `cargo xtask deps --check` refuses the edge again when the feature is off.
6. **Split `docs/`.**
   - User guides stay where they are.
   - `docs/dev/` gets architecture, building, testing, releasing, and Docker.
   - `docs/adr/` gets a README index with statuses.
   - `docs/history/` gets plan, tasks, audits, and star-parity.
   - Fix every link, and update `xtask dist`, the site generator, and the link checker.

### Agent W3d — Formats for students (Phase 4)

**Status (Saturday, September 26, 2026): done, on `worktree-agent-a4ac0c5fbef60676c` (W3d's work, finished by W3d-3), main merged in; ADR-0026.** OCR in process with ocrs on RTen (a scanned page's one image taken straight out with hayro's interpreter, else the page rendered with hayro), accented and non-English text routed to Tesseract, and PaddleOCR PP-OCRv5 Latin through RTen as an experimental engine; Tesseract found on `PATH` or in its install folders; per-page progress and cancel; clear messages when models or Tesseract are missing. DAISY 3, DTBook, and Bookshare zips; zip, tar, tar.gz, and 7z with `book.zip!inner.pdf` paths; web addresses (`url` feature, in `tw`); PowerPoint; CSV, TSV, ODS, and XLSX read cell by cell (calamine). Hostile-input tests, and fuzz targets `daisy`, `pptx`, `sheet`, `archive`, `image`, and `web`. OCR on the test scans (release): English PDF, ocrs 9.6% word errors in 1.6 s, Tesseract 0% in 1.1 s; French, ocrs 40.3%, Paddle 16.9%, Tesseract 2.6%. Left: `cargo xtask notices` (cargo-about is not installed here; the notices were merged by hand), the new fuzz targets in the nightly matrix, the Linux check (to CI), and five stray build-lock files under the junk `C*/Program Files` folder, which the guard would not let `git rm` touch.

**Owns:** `crates/textweaver-formats`, and a new `textweaver-ocr` crate if needed.

1. **OCR for scanned PDFs and images.**
   - **Read `docs/research/pure-rust-wave3.md` first.**
   - Use the pure-Rust `ocrs` engine (0.13.1, on rten 0.26) in-process first, running only on pages with no text layer.
     - Render pages with `hayro` 0.7.1. Before rendering, try pulling the page's single scanned image out with lopdf.
     - ocrs reads only ASCII plus the euro sign, so route accented or non-English text to the fallback. Also evaluate the PaddleOCR PP-OCRv5 Latin models through rten.
     - The model files download only after the user confirms (12.2 MB, CC BY-SA 4.0, checked by SHA-256). Credit them in the notices.
   - Keep a Tesseract subprocess as a fallback for languages ocrs lacks, selected with the `ocr_lang` setting.
   - Detect Tesseract on PATH (the installers offer it). Measure the quality of both engines on a few test pages.
   - Show progress and allow cancel.
   - Announce clearly when Tesseract is missing.
2. **DAISY 3 / DTBook and DAISY zips** (Bookshare), in spine order, with NCX navigation.
3. **Archives.**
   - ZIP and TAR, with 7z optional.
   - Opening one lists its readable files.
   - `book.zip!inner.pdf` opens a member, and notes and positions are keyed by that form.
4. **Open a web page by URL.** Fetch it, detect the encoding, and read it as HTML. For a PDF, save it to the cache and open it.
5. **PPTX.** Slide titles become headings, speaker notes follow each slide, and images use their alt text.
6. **Spreadsheets.** XLSX and CSV/TSV become tables, using `calamine` if its licence and size are acceptable.
7. **Hostile-input limits and fuzz targets** for every new loader, following P1d's patterns.

### Agent W3e — Language and study aids (Phase 4)

**Status (Saturday, September 26, 2026): done, on `wave3/e-lexicon-study`.** New crate `textweaver-lexicon`: define word offline (glossary, then Open English WordNet 2025 through morphy, with CMUdict pronunciations respelled), from `third_party/lexicon/lexicon-en.twlex` (9,988,663 bytes, fst plus ruzstd, built by `tools/build_lexicon.py`); and a Fluent-subset message catalog with English complete, `en-XA`, and `ar-XB` (ADR-0025). Store: `[lexicon]`, `[stats]`, `[interface]`, `profiles.toml`, `stats.json`, and Star's reading statistics imported. App: define word (Ctrl+Shift+D, Alt+E), profiles (Ctrl+Shift+U, Alt+U), statistics (Ctrl+Shift+Y, Alt+Y) on the existing list model. CLI: `tw define`, `tw stats`, `tw settings profile`. Left: the rest of the interface's strings into the catalog (W4d), the study lists into W3a's list model, loading the dictionary off the input thread, and Star's profiles import.

**Owns:** a new `textweaver-lexicon` crate, plus the app wiring for its actions and store settings.

1. **Define word, offline.**
   - Look up the user's own glossary first, then **Open English WordNet 2025** (CC BY 4.0; Princeton WordNet 3.1 is the alternative), then CMUdict pronunciations (BSD). See `docs/research/pure-rust-wave3.md`.
   - Use `fst` plus `ruzstd` for the compact data file, and add a "morphy" step that reduces inflected forms.
   - **Jon approved on 2026-09-26** downloading Open English WordNet 2025 (the WNDB zip) and CMUdict, from their official GitHub releases only, for the one-time data build. Record the SHA-256 sums and add the licences to the notices.
   - Build a compact derived data file with a script in `tools/`.
   - Record licences and SHA-256 sums in `third_party/`, and add them to the notices.
   - Ask the orchestrator before downloading the source data.
   - A list shows definitions, synonyms, and pronunciation. Open it for the word at the cursor with a chord such as Alt+Shift+W, or another that fits.
2. **Settings profiles.**
   - Named sets of voice, rate, theme, font, spacing, highlight, and access-mode settings.
   - Switch, save, rename, delete, import, and export them.
   - Build on `tw settings`.
3. **Reading statistics.**
   - Time read, furthest point, sessions per document, and a "most read" list.
   - Store them in state, with an opt-out.
   - Add `tw stats`.
4. **Scaffolding for interface translations.**
   - A message-catalog mechanism for spoken and displayed strings: fluent, or a small in-crate catalog (justify the choice).
   - English complete, plus a pseudo-locale to test coverage and a right-to-left check.
   - Actual translations come later.

### Agent W3f — Voices and speech (Phase 4)

**Status (Saturday, September 26, 2026):** done on `wave3/f-voices`, not merged. Piper voices run in-process on RTen with word timing from `w_ceil` (real-time factor 0.13, first audio 77 to 284 ms, on a quiet machine); the voice manager lists every engine's voices with language and engine filters, downloads Piper voices after a yes with the licence said and every file hash-checked, and each voice keeps its own rate and pitch; Whisper base.en int8 runs in-process on RTen (`tw dictate`, microphone included), after a fix for RTen's int8 saturation on CPUs without VNNI; `cargo xtask listen` and a listening checklist in `docs/dev/releasing.md`. ADR-0023. candle was never built, so no candle feature exists. main (W3a, W3c) is merged in: Piper is registered in `textweaver-engines`, voice downloads sit behind the app's `publish` feature, and the voice manager uses W3a's list model. Native tests: 1,933 passed, 0 failed, 29 ignored.

**Owns:** `crates/textweaver-speech` (new backends), a new `textweaver-piper` crate, and the voice manager in the app.

1. **Piper neural voices.**
   - **Read `docs/research/pure-rust-wave3.md` first.**
   - Run the Piper voice models in-process with **rten** 0.26, the pure-Rust ONNX runtime, following `rten-examples/src/piper.rs`. tract and candle cannot run the VITS voice graphs yet.
   - Phonemize with our libespeak-ng loader, falling back to the pure-Rust `espeak-ng` crate when the library is missing.
   - Get word timing from the `w_ceil` duration tensor.
   - Measure the real-time factor and the time to first audio.
   - Show each voice's licence (from its `MODEL_CARD`) before downloading it, and never bundle non-commercial voices.
   - Keep a `piper` subprocess only as a fallback.
   - Word timing comes from Piper if it reports it, otherwise it is estimated. `tw backends` says which.
   - A voice catalog lists language, quality, size, and licence.
   - A voice is downloaded only after the user confirms, with a SHA-256 check, into the data folder.
2. **Voice manager.**
   - Every voice from every engine, filterable by language and engine.
   - Preview, favourites, download (Piper), and remove.
   - Build it in the terminal on the app's list model (W3a), and in the GUI later.
3. **In-process dictation.** Move Whisper dictation into the process with **rten** (the ONNX int8 models from onnx-community, following `rten-examples/src/whisper.rs`). The research found rten purer than candle, which builds a C library.
   - Jon approved rten over candle on 2026-09-26. Record the switch from candle in an ADR.
   - Capture audio with rodio's `recording` feature, resample with `rubato`, and detect speech with `earshot`.
   - Keep candle behind a feature, and the whisper.cpp subprocess as the fallback.
   - Measure latency on the CPU with `base.en`.
4. **Rate and pitch per voice.** Remember them for each voice, as screen readers do.
5. **Real-engine listening checklist.**
   - Add steps to `docs/dev/releasing.md` for Jon to hear Eloquence, SAPI, and Piper before each release.
   - Add `cargo xtask` helpers that write sample WAV files to listen to.
   - Never play audio in tests.

### After Wave 3

- Jon's NVDA and JAWS listening session for the Xilem GUI.
- The GUI's edit mode and reading aids.
- VoiceOver and Orca testing.
- The aarch64 AppImage, on GitHub's arm64 runners.
- Signing, when funding allows.
- Release `0.1.0-alpha.4` or `beta.1` when Jon says so.

## Usability pass (between Wave 3 and Wave 4; Jon, 2026-09-26)

Wave 4 starts only after this pass is done. It has four steps.

1. **Audit, with two agents in parallel.**
   - **Terminal reader and `tw`.** Walk the real flows a student uses:
     - first run;
     - opening Markdown, PDF, and EPUB;
     - reading with the NVDA- and JAWS-style keys;
     - notes, outline, and search;
     - edit mode, citations, spell check, and export;
     - settings, and the three access modes.
   
     List every rough edge: unclear or missing announcements, surprising keys, dead ends, slow steps, and inconsistent wording.
   - **The Xilem GUI.**
     - Build it, run it in `--background`, and take screenshots at 100% and 200% scale.
     - Run the UI Automation report and the AT-SPI dump.
     - Check the polished dark look against W3b's goals: focus order, labels, the announcements, large documents, and theme and font changes.
     - Compare the GUI with the terminal reader, and list what is missing or rough.

   Both audits produce one ranked list. Each item has its evidence, its size, and whether it is a quick win (under half a day).

   **Status (Agent UX-1, terminal reader and `tw`, 2026-09-26):** audit done and the quick wins fixed on branch `ux/terminal-pass`; findings, the ranked remainder, and Wave 4 suggestions in [docs/research/usability-terminal.md](../research/usability-terminal.md). Fixed: questions spoken while reading aloud, a first-run welcome, plain open failures in the reader and `tw`, a fuller F1 help with at most two keys per line, startup warnings that no longer cut off "Opened", `tw text | head` no longer panics, `tw define` no longer reports twice, `tw marks --home`, and three stale doc passages.
2. **Fold in the quick wins.** One or two agents fix every quick win, with tests. Larger items go into the Wave 4 briefs.
3. **Jon's check.** The orchestrator builds the terminal reader and the GUI in release mode and opens them for Jon. He gets a short checklist for NVDA and JAWS: the GUI, the modes, and the main flows. His findings become fixes or Wave 4 items.
4. **Then Wave 4.** Restart Docker, then launch the nine Wave 4 agents.

## Wave 4 (refined 2026-09-26; starts when Wave 3 is merged and Docker is restarted)

**ADOPTED PLAN, Sunday, September 27, 2026.** This supersedes everything else in this section, including the "Refined plan" and the "Earlier layout" below; those are kept as history. The plan, the ready-to-paste briefs and the orchestrator's runbook are in `docs/research/wave4-orchestration.md`, with these decisions from Jon.

**Shape:** three sub-waves, with at most three agents building at once. Memory limits this, not disk: Docker's machine has about 31 GB.
- **Wave 4a, "hear it first":** W4h (terminal polish), W4s (GUI session prep), W4c1 (MathCAT speech). Gate: Jon's NVDA and JAWS session 1.
- **Wave 4b:** W4g (authoring extras), W4b (speed), W4a2 (the GUI after the session). Gate: Jon's session 2.
- **Wave 4c:**
  - W4c2 (documents);
  - W4d (translations);
  - **W4a3 (GUI edit mode, new, below);**
  - W4f (platforms and CI), running as a light fourth agent.

  Gate: Jon's check in Spanish or French, then compacting Docker's disk.

**Jon's decisions (September 27, 2026):**
1. **W4e (offline intelligence) is deferred to Wave 5.**
2. **GUI edit mode stays in Wave 4,** as W4a3 in sub-wave 4c after session 2.
3. **No merge gate for now.** W4f doesn't change any repository settings (no rulesets, no required checks, no auto-merge).
4. **The wxDragon spike (`crates/textweaver-gui`) is removed** once Jon's session 2 on the Xilem GUI passes. The orchestrator does this as its own commit, with the file list shown first.
5. **Unbound keys in the reader are silent,** as in NVDA's browse mode. W4h adds no tone.

**W4a3: GUI edit mode (sub-wave 4c, P1; ADR 0033).**
- **Edit mode in `DocumentView`,** using `Command::ReplaceRange`, and keeping structure while editing as the terminal reader does.
- **Based on Parley's `examples/editor`,** on the vendored Parley 0.8.0. Don't upgrade it.
- **Accessibility:**
  - use AccessKit's `MultilineTextInput` role;
  - handle `SetTextSelection`, `SetValue` and `ReplaceSelectedText`;
  - draw selection and backgrounds yourself, since Parley has no background style;
  - undo comes from `textweaver-editor`.
- **The GUI features that waited for edit mode:**
  - citations while writing;
  - export and preview;
  - spell check.
- **Uses the highlight and announcement designs** decided in Jon's sessions 1 and 2.
- **Owns:** `textweaver-xilem`'s editing code. Don't touch `textweaver-app` beyond one-line hooks, `.github/`, or the workspace members.
- **Report checklist,** at most five items: type and hear each character; select a word and hear it; undo; a citation while writing; spell check on a misspelled word.


**Lessons from Waves 2 and 3.** Every Wave 4 agent follows these. They come from what went wrong or cost time.

- **Checks.** Run every check CI runs before reporting. CI went red three times from things the usual check run skipped. The full set:
  - `cargo fmt --all --check`.
  - Workspace clippy with `-D warnings`.
  - Tests with `--no-fail-fast`, so one failure doesn't hide others.
  - `RUSTDOCFLAGS="-D warnings" cargo doc`. Rustdoc link errors turned CI red twice: redundant link targets, links to private items, and `[text]` in doc comments.
  - The Docker all-features clippy and tests. That is where Linux-only warnings show up, such as `unused mut` behind `cfg(windows)`.
  - `cargo xtask keyboard --check`, `cargo xtask deps --check`, `python tools/check_links.py`, `python tools/gen_site_data.py --check`, and `cargo xtask notices --check` when dependencies change.
  - A build with the reader's `publish` feature off, if you touch the app or the reader.
- **Timing in tests.** Tests that sleep or assert wall-clock bounds fail under load, and they failed repeatedly while many agents built at once.
  - Wait for a signal or a generous deadline instead.
  - Run any new timing-sensitive test 40 times before reporting.
  - A "flaky" test is a bug until proven otherwise.
- **A new setting touches four places.**
  - The store type, with a default and a test.
  - The settings export fixture, which fails if a setting keeps its default.
  - The settings schema (`crates/textweaver-app/src/settings_schema.rs`), which fails if a key is missing.
  - A reader somewhere: the "every setting is used" test fails otherwise.
- **A new key goes through three checks.**
  - The keymap conflict and reachability tests.
  - WCAG 2.1.4: a chord, or a single key that F9 can turn off.
  - Windows Terminal's default keys. Alt+Shift+arrows, Alt+Shift+D, and Ctrl+Alt+Left are taken there; see `docs/screen-readers.md`.

  Follow the NVDA and JAWS quick-key conventions, and regenerate `docs/keyboard.md` and the site data.
- **Shared rules in the code.**
  - Announcements go through `textweaver_a11y::route`, so hybrid and screen-reader modes behave.
  - Saves and other disk writes go through the writer thread. Never block the input or speech threads.
  - Speech runs on its own thread, with no async runtime (ADR-0003).
  - New crates respect `cargo xtask deps --check`. The store depends only on core.
- **Merging with other agents.**
  - Merge `main` into your branch early and often, not only at the end.
  - Keep edits in other agents' areas to one-line hooks, and report anything bigger.
  - Shared files conflict every time: the root `Cargo.toml` members, `CHANGELOG.md`, `docs/history/tasks.md`, and `docs/site/*`. Make small, additive edits there, and regenerate generated files after merging instead of hand-merging them.
  - Check `docs/adr/README.md` for the next free ADR number; two agents picked the same one before.
- **Current paths, after W3c.**
  - Briefs are in `docs/history/tasks.md`. Developer docs are in `docs/dev/`, and the ADR index in `docs/adr/README.md`.
  - The backend registry is in `crates/textweaver-engines`, and font resolution in `crates/textweaver-fonts`.
  - Export, preview, and citations are behind the app's `publish` feature.
- **GUI and audio.**
  - Automated GUI runs use `--background` and never take the foreground. A dialog once stole focus from Jon.
  - Never play audio.
  - Jon tests with screen readers himself. Give him a short checklist.
- **Downloads.**
  - Only download what Jon approved: official sources, SHA-256 recorded, licence added to the notices (`about/data-files.md`, then `cargo xtask notices`).
  - Never bundle non-commercial models or voices.
  - Keep models out of git, in an ignored cache.
- **Privacy.** The hard rule in the shared preamble: no personal identifiers in any request, header, URL, or public file, and a neutral User-Agent. Two research threads broke this once.
- **Dates.** Take dates from the machine and compute weekdays. Never write a date from memory.
- **Disk and memory.**
  - Use one target directory, and build through the shared cache: `RUSTC_WRAPPER=sccache` is set for everyone.
  - Avoid extra release builds. Drive D: once filled up because of per-worktree build folders.
  - Always prefix Docker commands in Git Bash with `MSYS_NO_PATHCONV=1`. Without it, `-e CARGO_TARGET_DIR=/target/<agent>` becomes a Windows path, and build files were committed under a junk `C:/Program Files/Git/target` folder.
  - Run the Docker check once, at the end. Never restart Docker yourself: force-quitting it crashed Docker Desktop, and the orchestrator restarts it between waves.
- **Research sub-agents.** Don't spawn nested research agents: their reports went to the orchestrator, not to the agent that asked. Do the research yourself, or ask the orchestrator.
- **Reports.** Keep reports plain and short: headings and lists, no tables. Name what could not be verified. Add your status line in `docs/history/tasks.md`.

**Refined plan, Sunday, September 27, 2026 (after Wave 3 and the deletion incident).** This supersedes the nine-agent layout below; the briefs themselves still apply.

Run it in **two groups of five**, with a listed flush of merged build output between them. At Wave 3's measured sizes, nine agents' build folders could take up to about 360 GB and push D: through its 200 GB floor.

This plan folds in the Fable review (`docs/research/wave4-plan-review.md`), adopted by Jon on Sunday, September 27, 2026. Where a brief below disagrees with this plan, this plan wins.

**Before Group 1 (orchestrator):**
- Add the six W3d fuzz targets (`daisy`, `pptx`, `sheet`, `archive`, `image`, `web`) to the nightly workflow, with their corpus lines. Only the orchestrator or W4f may edit `.github/`.
- Confirm `cargo xtask notices --check` passes on main.

**Group 1 (starts when Jon says go):**
1. **W4h, terminal polish (P1, new).** Small commits, merged first; W4g starts from W4h's merged work.
   - UX-1's top findings from `docs/research/usability-terminal.md`:
     - key names in spoken form for textweaver's own voice;
     - "say status" and "repeat last message" actions;
     - Escape in edit mode saying how to finish;
     - announcements queued until the speech engine is ready. These are W4h's, not W4b's. After the change, run `cargo xtask listen` and report the result.
   - Quick wins:
     - "Ready" on the title line until the first play, instead of "Stopped";
     - `tw search --json` and `tw info --json` on a closed pipe, through `print_all`;
     - a two-line hint for `tw` with no arguments;
     - the command palette's opening sentence;
     - a key that repeats an open list's introduction;
     - the quick start's `q` line;
     - `py -3` in `CONTRIBUTING.md`, `docs/dev/testing.md`, `docs/dev/building.md`, and this file's date command.
   - A test that every key named in a spoken message comes from the keymap, not from a fixed string.
   - New keys go through the keymap conflict and reachability tests, WCAG 2.1.4, and Windows Terminal's taken keys.
2. **W4b, speed and memory (P1).**
   - Provide a peak-allocation counter behind a feature, and the `--log` numbers. W4a1 fixes the GUI's memory growth with them.
   - Measure ropey 2.0.0-beta.1 and crop 0.4.3 on the edit traces and report the numbers. **Don't change the rope in Wave 4;** that's a Wave 5 ADR.
   - The rustls-graviola TLS item is dropped.
   - Make the zip feature change in one early commit, and tell W4c2.
   - Startup announcements are W4h's; don't touch `launch`.
   - Run new timing tests 40 times while another build is running.
3. **W4c1, MathCAT (P2).** Independent of the other Group 1 agents, so it takes the fifth slot from the start.
   - MathCAT 0.7.6 has no final release, so use 0.7.6-rc.3 behind the feature and record the version in the ADR.
   - Do speech first. Add braille only if MathCAT issue #827 (a panic in `GetNavigationBraille` in no-unsafe builds) is closed, because this workspace denies unsafe code.
   - EPUB 3 MathML is W4c1's; W4c2 leaves the EPUB loader alone.
   - ADR 0029.
   - **Status (W4c1, Sunday, September 27, 2026):** done on `wave4/c1-mathcat`, not merged. MathCAT 0.7.6-rc.3 speech (ClearSpeak and SimpleSpeak) behind the off-by-default `mathcat` feature and `[reading] math_engine`, EPUB 3 MathML read as math, ADR-0029. Braille waits for #827. Two dependency checks need Jon's decision before merge: the `bzip2-1.0.6` licence (libbz2-rs-sys) and the unmaintained yaml-rust, both from MathCAT itself.
4. **W4a1, GUI accessibility first, then reading aids and edit mode (P1).** In this order:
   1. The direct `UiaRaiseNotificationEvent` option for JAWS.
   2. The fix for clipped options in `ChoiceList` and `SettingsGrid`, so options scrolled out of view stay in the accessibility tree. This is a listed exception to W4a2 owning the dialogs.
   3. The GUI's memory growth (98 to 172–187 MB), first measured with the engines and `publish` off, using W4b's tools. The likely cause is engines and data loaded at startup instead of on first use.
   4. **Merge 1 to 3 and tell the orchestrator,** who builds the GUI for Jon's NVDA and JAWS session while W4a1 continues.
   5. The reading aids.
   6. Edit mode, on the vendored Parley 0.8.0. Don't upgrade it.

   The RSVP word node never has a live setting and never takes focus; test that. Parity and the wxDragon removal belong to W4a2. ADR 0028.
5. **W4g, authoring extras (P1).** Starts from main after W4h merges.
   - **harper-core** 2.11.0 behind a `grammar` feature, with `thesaurus` off, and the binary size measured before and after.
   - **rumdl** 0.2.77 (`rumdl_lib`) behind a feature, pinned exactly, with a fixed rule set. If its size or API churn is too much, write our own lint for the rules a blind author needs, and say which in the report.
   - **syntect** 5.3 with `default-features = false, features = ["default-fancy"]`, and **two-face** 0.5.2 with `syntect-fancy`. The defaults build the Oniguruma C library.
   - **arboard**, with `wayland-data-control` on Linux.
   - Unicode math in the plain reading view.
   - Notes export to BibTeX, RIS and JSON, using the citation crate's record types.

**Group 2** (after a listed flush of Group 1's merged build volumes):
6. **W4a2, GUI parity and wx removal (P2).** Starts only after Jon's session.
   - First: a window slide during reading keeps the screen reader's place. Check it with the UI Automation report, and put it first on Jon's second checklist.
   - Parity means Star's features, not Star's bugs; the Phase 0 inventory lists the bugs.
   - A test that the first nine themes keep their cycle order.
7. **W4c2, documents (P2).** Merge main after W4b's zip commit lands. Leave the EPUB loader alone.
8. **W4f, platforms and CI (P3).**
   - Do the six fuzz targets, if they weren't done before launch.
   - The first GUI packages for macOS and Linux.
   - A doc pass against the code, file by file, before release notes.
   - **Branch pruning is removed.** It happens only after a wave is complete, and only with Jon's approval.
9. **W4d, translations (P3).**
   - Extend W3e's Fluent-subset catalog (`textweaver_lexicon::i18n`, ADR-0025) first. Switch to `fluent-bundle` only if a language needs attributes, functions or number formatting, and record that as a status update on ADR-0025.
   - A missing voice for a language keeps the current voice and says so; never go silent.
   - A language change applies live, with no restart.
   - The first-run language choice and per-language voices are settings, so they touch the four places.
   - ADR 0030.
10. **W4e, offline intelligence (P3, experimental; the first to cut if Group 2 needs the disk or attention).**
    - The no-model LexRank summary first.
    - Translation only if there's disk to spare, and only with models Jon has approved.
    - Pin `tokenizers` to 0.23, since 1.0 is at release candidate.
    - ADR 0031.

**Rules for every Wave 4 agent** (see `CLAUDE.md`):
- **Only the owner overrides rules.**
- **Deleting from Bash** is a last resort that needs the owner's approval. Use PowerShell's `Remove-Item -LiteralPath`.
  - Never chain a delete onto another command.
  - Never hand-type an escaped file name; use `git rm` or `git clean`, and show the owner first.
- **Work only in your own worktree and build folder.**
- **Develop in the container** (`docs/dev/docker.md`), so shell commands stay off the host.
  - Use `TW_AGENT=<agent>`, so you get your own `tw-target-<agent>` volume.
  - Each container gets 8 GB of RAM and 6 CPUs, and shares the 30 GB compile cache.
  - Check D:'s free space and `docker system df` before heavy runs, and stop if D: is near 200 GB free.
  - **Never delete inside a Docker volume.** The orchestrator lists and removes merged agents' volumes.
- **The heavy all-features Docker check is the orchestrator's,** run one agent at a time at integration. Agents run the native checks, plus their own crates' tests in the container.
- **ADR numbers:** W4a1 0028, W4c1 0029, W4d 0030, W4e 0031. `CHANGELOG.md` entries go under a heading with your agent's name.
- **Dates:** on Windows, run `py -3 -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"`. `python` on this machine is the Windows Store stub.
- **Every report ends with a checklist** of at most five things for Jon to try with NVDA and JAWS, or the line "nothing to hear".

**Earlier layout: nine agents (Jon, 2026-09-26: use more resources), superseded by the refined plan above.** The briefs below were split like this:
1. **W4a1, GUI edit mode and reading aids.** From W4a: edit mode in `DocumentView`, the reading aids in the GUI, and the research notes on editable text, RSVP overlays, and announcements.
2. **W4a2, GUI parity and wx removal.** From W4a: parity with the terminal reader, fixes from Jon's listening session, and removing the wxDragon spike. This agent owns the dialogs, W4a1 owns `DocumentView`, and they coordinate through small commits.
3. **W4b, speed.** As written.
4. **W4c1, MathCAT.** From W4c: MathCAT for math speech, navigating inside formulas, Nemeth and UEB braille in the BRF writer, and EPUB 3 MathML.
5. **W4c2, documents.** From W4c: native RTF and ODT, DOCX comments and tracked changes, the LaTeX subset, EML and MHTML, and fuzz targets with hostile-input limits.
6. **W4d, translations.** As written.
7. **W4e, offline intelligence.** As written.
8. **W4f, platforms and releases.** As written.
9. **W4g, authoring extras.** The "Also for Wave 4" list at the end of this section: grammar checking with harper, Markdown lint and format with rumdl, the native clipboard with arboard, code highlighting with syntect, Unicode math in the reading view, and notes export to BibTeX, RIS, and JSON.

**Build speed for many agents.** Every agent builds through the shared compile cache, so dependencies compile once for all worktrees. Set these environment variables for every cargo command on Windows:
- `RUSTC_WRAPPER=sccache`
- `SCCACHE_DIR=D:\sccache`
- `SCCACHE_CACHE_SIZE=50G` on the Windows host (`D:\sccache`); the container's shared cache is capped at 30 GB (`compose.yaml`)

In Docker, keep your own target directory: `-e CARGO_TARGET_DIR=/target/<agent>`. Build only what you need while working, and run the full workspace checks at the end.

**Every Wave 4 agent reads `docs/research/wave4.md` first.** It records the research behind these briefs: crate versions, licences, APIs, and risks. The privacy hard rule in the shared preamble applies to every request.

A draft, so the next wave can start the moment Wave 3 lands. It follows the same rules and spirit as Wave 3: pure-Rust first, experimental, and Jon's own use first. The orchestrator will refine these briefs from Wave 3's reports before launch.

### Agent W4a: The Xilem GUI, part two

- **Edit mode** in `DocumentView`. It uses `Command::ReplaceRange`, and keeps structure while editing, as the terminal reader does.
- **Reading aids in the GUI:**
  - an RSVP panel that never covers the caret;
  - a reading ruler and current-line band;
  - text spacing;
  - bionic reading;
  - difficult words.
- **Parity with the terminal reader:**
  - outline;
  - citations while writing;
  - export and preview;
  - spell check;
  - tables and links;
  - notes;
  - the access modes.
- **Fixes from Jon's NVDA and JAWS listening session.** Where AccessKit falls short, write the changes and send them upstream.
- **Remove the wxDragon spike** once the Xilem GUI passes the UI Automation report and Jon's session.
- **From the research.**
  - Base the editable text on Parley's `examples/editor`, which uses accesskit 0.25 and accesskit_winit 0.34. Parley main removed its AccessKit feature.
  - Use `MultilineTextInput` in edit mode, and handle `SetTextSelection`, `SetValue`, and `ReplaceSelectedText`.
  - Draw backgrounds yourself; Parley has no background style.
  - Undo comes from textweaver's editor crate.
  - For the RSVP overlay, hide the flashing word node, and expose a quiet labelled status node with `Live::Off`.
  - Offer a direct `UiaRaiseNotificationEvent` on Windows as an announcement option, for Jon to compare with the live regions.
  - Use Slint's AccessKit integration as a reference.

### Agent W4b: Speed, round two

- **Faster text search and segmentation.**
  - Adopt `icu_segmenter` for word and sentence boundaries, measured against `unicode-segmentation`.
  - Use `memchr::memmem` for literal find.
  - Use aho-corasick for highlighting many terms at once.
- **Pure-Rust compression.** Turn on the zip crate's pure-Rust features: zlib-rs, lzma, xz, bzip2, and ppmd.
- **Allocation and memory.** Cut allocations and peak memory, using the benchmark gate's numbers.
- **Startup time** of `textweaver` and `tw`.
- **Ropey 2.** Evaluate it (byte indexing, still beta) and `crop` (byte-indexed, O(1) Arc clone, and 3 to 4 times faster on edit traces). `jumprope` deep-copies on clone, so it does not suit the writer thread.
- **Tools.**
  - Allocations: dhat and divan.
  - Startup: hyperfine `-N --warmup`, plus a first-frame-exit flag.
  - Binary size: cargo-bloat and cargo-llvm-lines. twiggy and bloaty do not handle Windows binaries.
  - `icu_segmenter` has no abbreviation handling, so keep ours.
  - For zip, use `default-features = false`. Note that the feature named `bzip2-rs` is C, and the one named `bzip2` is pure Rust.
- **Binary size.** Reduce it now that the reader's export and citations are a feature.
- **Optional TLS change.** Consider `rustls-graviola` in place of ring's C code, with ring as the fallback.

### Agent W4c: Formats, round two

- **MathCAT** (DAISY, MIT; the math engine NVDA and JAWS use). Adopt it behind a feature for math speech (ClearSpeak and SimpleSpeak, in about 15 languages), for navigation within formulas, and for **Nemeth and UEB math braille** in the BRF writer.
  - Pin 0.7.6 once it is stable. 0.7.5 pulls in zip 6 and yaml-rust.
  - Run it on one dedicated thread, because its state is thread-local.
  - Keep textweaver's own math speech as the fallback.
- **Math from EPUB 3.** Read EPUB 3 MathML as math: MathCAT first, then `alttext`, then the alt text of `altimg`.
- **More from DOCX.** Read Word comments, tracked changes, and footnotes into notes.
- **RTF and ODT natively,** so neither needs Pandoc.
  - RTF: write our own iterative parser with an explicit group stack and caps, decoding with encoding_rs for `\ansicpg` and `\fcharset`. rtf-parser is too shallow.
  - ODT: roxmltree plus zip (ODF 1.4).
- **DOCX comments and tracked changes** extend our own roxmltree reader: `w:comment*`, `w15:commentEx` (replies and resolved state), `w:ins`, `w:del`, and `w:moveFrom`/`w:moveTo`. Footnotes already load.
- **A native LaTeX subset.** Sections, lists, math, citations, and tables are enough for course notes.
- **Emails and web archives.** EML and MHTML through `mail-parser` 0.11.9 with its encoding_rs feature (RFC 2557 for MHTML).
- **Fuzz targets and hostile-input limits** for each new loader.

### Agent W4d: Interface translations

- **Catalogs.** Fill the message catalogs from W3e for Spanish, French, German, Portuguese, and Arabic, the languages Star had. Take Star's catalogs where they apply.
  - Use **Fluent**: fluent-bundle with fluent-templates `static_loader!`, or i18n-embed with `fl!`. It gives CLDR plurals, bidi isolation, and fluent-pseudo.
  - Star's catalogs (`D:\star\star\locale`) are flat JSON, mapping English to the translation, with no plurals and `{name}` placeholders. There are 664 strings each for es, fr, de, and pt, and 129 for ar. Many are Qt or Star specific.
  - Script-convert the strings that overlap with textweaver's.
  - Extract textweaver's strings: about 60 `Announcement` kinds and about 364 `format!` messages in the app and TUI.
- **Right-to-left layout** in the terminal reader and the GUI. Keep text in logical order in the model, in AccessKit, and in speech. Reorder it for display only, with `unicode-bidi`, behind a setting that is off where the terminal does its own BiDi (VTE, Konsole, mlterm, and macOS terminals). Windows Terminal has no RTL support. Parley handles bidi in the GUI.
- **A first-run language choice.**
- **Speech in the same language.** Each language gets a default voice for that language.
- **A pseudo-locale in CI,** to catch strings that were never translated.

### Agent W4e: Offline intelligence on rten (experimental)

- **Offline translation of a document or selection.** OPUS-MT or Marian models in ONNX, run on rten. The user confirms any model download, and sees its licence.
  - Use the Xenova quantized ports (about 110 MB per language pair; show the licence as CC-BY 4.0), with rten-generate's merged-decoder KV cache on the Whisper example's pattern.
  - Mask the pad token (65000).
  - Tokenize with `kitoken` (pure Rust, Unigram), checking for an exact token-ID match against a reference. Fall back to `tokenizers` with `default-features = false, features = ["fancy-regex"]`.
- **Extractive summaries.** Star used sumy's LexRank on word counts with no model, so make the no-model version the default. That is in-house LexRank: TF-IDF cosine, damping 0.85, and power iteration. Offer embeddings (all-MiniLM-L6-v2, 23 MB quantized, WordPiece through rten-text) as an option.
- **The difficult-word overlay** gets definitions from W3e's lexicon.
- **Every model** is optional, downloaded only on request, and checked by SHA-256.

### Agent W4f: Platforms and releases

- **An aarch64 AppImage,** built on GitHub's arm64 runners (`ubuntu-22.04-arm`, free for public repositories) with appimagetool and the type2-runtime for aarch64.
- **VoiceOver on macOS and Orca on Linux.** Fix what the accessibility dumps and tester reports show.
- **Release automation.**
  - `cargo xtask release` checks that the listening checklist is done.
  - Changelog grouping.
- **The pull-request merge gate.** Integrations go through pull requests with required checks.
  - Merge queue is for organisations only. Use rulesets instead: require a pull request with 0 approvals, require status checks, and block force pushes.
  - Turn on auto-merge, then use `gh pr merge --auto --squash --delete-branch`.
  - Changing repository settings needs Jon's approval.
- **Automated screen-reader tests** complement Jon's manual testing. Guidepup drives NVDA and VoiceOver in CI; use `npx @guidepup/setup setup --ci`, because the action was archived. Also try DioxusLabs accessibility-cli for tree dumps.
- **Prune merged branches.** 85 of the 91 are merged. Use git-delete-merged-branches with `--effort=3` to catch squash merges, run a dry run first, back up refs, and ask Jon before deleting remote branches.
- **Release notes.** Prepare `0.1.0-beta.1` notes, for when Jon asks for a release.

### Also for Wave 4, for the first free agent or for Wave 5

These are high-value items for terminal-first Markdown work, found by the research:
- **Grammar checking** in edit mode with `harper-core` 2.11.0 (Apache-2.0, pure Rust, offline).
- **Markdown lint and format** with `rumdl` 0.2.77 (MIT).
- **A native clipboard** fallback with `arboard` 3.6.1, for terminals without OSC 52.
- **Code highlighting** in the terminal with syntect 5.3 and two-face 0.5.2.
- **Math as Unicode in the plain reading view**, as Star's `mathrender.py` did (x², √2).
- **Notes export** as BibTeX, RIS, and JSON, as Star did.
