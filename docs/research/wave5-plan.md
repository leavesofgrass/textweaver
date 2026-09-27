# Wave 5 plan

Written on Sunday, September 27, 2026, by the Wave 5 planner (Fable 5.1), for the orchestrator and Jon. Wave 4 has not run yet. This plan treats Wave 4's outcomes as gates and branches, never as guesses. It changes nothing by itself: Jon adopts it, or parts of it, and the orchestrator applies it to `docs/history/tasks.md` after Wave 4 ends.

What it is built on: `CLAUDE.md`; the Wave 4 adopted plan, its lessons, and the Wave 2 and Wave 3 status lines in `docs/history/tasks.md`; `docs/research/wave4-orchestration.md` (the adopted shape, budget, runbook, cuts, and Jon's answers); `docs/research/wave4-plan-review.md`; `docs/research/wave4.md`; `docs/research/usability-terminal.md`; `docs/roadmap.md`; `docs/star-gaps.md`; `docs/research/pure-rust-wave3.md`; `docs/research/xilem-gui.md`; every ADR (0001 to 0027); `docs/dev/docker.md`, `docs/dev/releasing.md`, and `compose.yaml`; the wiki (the textweaver and Star hubs, the Wave 4 and Wave 5 planning pages, the deletion incident knowledge base, and the global rules); Star's code at `D:\star` (read only); the git log; and measurements taken on this machine today. Crate versions were checked on crates.io and GitHub today with the tool's default User-Agent. No personal identifier was sent.

Nothing was built, run in Docker, downloaded, or deleted for this plan.

## The short version

- **Wave 5 has one purpose: get textweaver ready for `0.1.0-beta.1`.** Every agent either removes a way to stall or lose your place, finishes a reading or authoring feature students need, or makes the release machinery and its checks trustworthy. Jon decides when the release happens; the wave makes it possible.
- **Shape: three sub-waves of three agents, plus one light fourth in the last.** Same method as Wave 4: one goal per sub-wave, one merge point, one thing for Jon to hear, at most three agents building at once.
- **Sub-wave 5a settles the foundations:** the rope decision as an ADR from W4b's numbers, memory and fuzz coverage, and the Star leftovers plus a Braille display pass. Gate: Jon's first Braille session.
- **Sub-wave 5b is documents and math:** the LaTeX subset with EML and MHTML, math braille and formula navigation on MathCAT (or on a patched copy if issue #827 is still open), and no-model summaries. Gate: Jon's check by ear and on the display.
- **Sub-wave 5c is the GUI's third pass, automated screen-reader tests, offline translation if Jon approves the models, and beta readiness.** Gate: Jon's session 3, then a beta readiness review.
- **Five things depend on Wave 4 in ways that change the plan.** They are branches, not guesses: the rope numbers (W4b), the GUI memory result (W4s, W4a2), MathCAT issue #827, whether W4a3 (GUI edit mode) lands, and the translation catalog (W4d). Section 3 spells each out.
- **Memory remains the binding limit,** not disk: three containers at 6 GB each, 26 GB of host RAM free before any build. Model downloads for translation total about 450 MB for four language pairs and each needs Jon's approval.

## 1. Goals for Wave 5, ranked

Jon's goal, as the brief states it: terminal-first Markdown reading and authoring, with speech, Braille, and highlighting that never stall or lose your place. Everything below is ranked against that.

1. **Never stall, never lose your place, on every frontend.** The rope decision (ADR-0034) is settled from measurements, so the text model stops being an open question before the beta. The GUI's memory growth is either fixed in Wave 4 or finished here. Fuzz coverage reaches every parser a student's file can hit, so a hostile file cannot take the reader down.
2. **Braille is first class.** Jon reads on a Braille display, and `docs/screen-readers.md` says the display has "not been tested yet". Wave 5 gives the display its own pass in the terminal reader (meaning first on every status line, short messages, the status-line cursor), a Braille session as a gate, and math braille (Nemeth or UEB) in the BRF writer and, through MathCAT, for the display.
3. **Beta readiness.** The release pipeline builds every package, including the GUI's, in CI; the docs are checked against the code by a tool, not by memory; the beta release notes exist; the listening checklist is dated. Jon decides the release; the wave removes every reason to wait.
4. **Reading the documents students bring.** The LaTeX subset (course notes), EML and MHTML (emails and saved pages), and math braille. Each with hostile-input limits and a fuzz target.
5. **Automated screen-reader tests that complement Jon's sessions.** A spike, not a promise: Guidepup for NVDA on a Windows runner against the GUI, accessibility-tree dumps on all three systems, an Orca script under Xvfb. Whether it works with a native app is the first thing the spike answers.
6. **The GUI's third pass.** The Parley upgrade with a measurement (deferred by name from Wave 4), syllables and difficult words drawn, the voice manager, the catalog's languages in the drawn labels, and whatever sessions 2 and 3 leave.
7. **Offline intelligence.** No-model summaries first (LexRank in-house, a half-day item). Translation on rten only with Jon's approval of each model download.
8. **Star parity leftovers that are cheap.** Library search by DOI, ISBN, and author; Star profiles imported; the lexicon loaded off the input thread; study lists on the list model. The rest of Star's peripheral list stays dropped (section 10).

## 2. Research

Checked today, Sunday, September 27, 2026. "Unchanged" means Wave 4's research still holds. Anything not checked today is marked unverified.

### 2.1 The rope (W5r)

- **ropey** (`https://crates.io/crates/ropey`, MIT or Apache-2.0): stable 1.6.1, which the workspace uses (`ropey = "1.6"`). 2.0.0-beta.1 (August 2, 2025) is still the newest, so the beta is now over a year old with no further release. 2.0 moves to byte indexing, puts character indexing behind `metric_chars`, and adds a `LineType` to every line call. Clone stays O(1).
- **crop** (`https://crates.io/crates/crop`, MIT): 0.4.3 (April 25, 2025), no release since. Byte-indexed, O(1) `Arc` clone that can cross threads, LF and CRLF only. Wave 4's research says 3 to 4 times faster than ropey on edit traces; W4b measures it on ours.
- **What the model depends on.** ADR-0002 makes `CharPos` the persisted position, the rope the canonical text, and `DisplayIndex` (now the rope itself, per W3a) the map to UTF-16 and byte offsets. A rope change touches `textweaver-text` and every crate that indexes by `CharPos` through it. The writer thread relies on the O(1) clone (ADR-0001, P2a).
- **Risk:** a byte-indexed rope makes `CharPos` a conversion at every boundary. Either the conversion is cached (a char-to-byte index per chunk) or `CharPos` becomes a byte position and every state file migrates. The second is a beta-breaking change; the ADR must say which, and the migration cost is the reason the decision may be "stay".
- **Advice:** W5r writes ADR-0034 from W4b's numbers, with a branch (section 3). The default answer is "stay on ropey 1.6 until 2.0 is final and battle-tested", unless the numbers show a stall that users feel.

### 2.2 Memory, fuzzing, and hostile input (W5m)

- **cargo-fuzz** 0.13.2 (June 9, 2026) unchanged; `fuzz/Cargo.toml` has 15 targets; `nightly.yml` runs 9 and Wave 4's orchestrator adds the six W3d targets before launch. W4c2 adds `rtf`, `odt`, and `docx_revisions`. What still has no target: the LaTeX and ASCIIMath parsers in `textweaver-math` (they have proptests, not fuzzing), the BibTeX, RIS, and CSL-JSON importers in `textweaver-cite`, the theme TOML reader, the lexicon data file reader, the vault importer, the JSON-RPC request decoder, and Wave 5's own LaTeX and EML parsers.
- **GUI memory:** ADR-0027 records 98 MB working set before main's merge and 172 to 187 MB after (650 to 670 MB private). W4s attributes it and fixes what is inside xilem; fixes in engines and the app (lazy loading of Piper, Whisper, OCR, and the 9.9 MB lexicon) are proposals in W4s's report, and W4a2 may or may not take them. W5m takes whatever is left, guided by W4s's table.
- **Tools** (unchanged from Wave 4): dhat 0.3.3, divan's `AllocProfiler`, cargo-bloat 0.12.1; `cargo xtask bench --quick` and `bench.yml`'s gate on peak heap and allocations.

### 2.3 LaTeX, EML, and MHTML (W5c3)

- **LaTeX subset.** Wave 4's research: keep our own math parser, write our own tokenizer for sections, lists, `tabular`, `cite`, `ref`, `label`, `footnote`, and math; use mitex's rowan grammar as a design reference; avoid tectonic (C libraries) and unicodeit (LPPL). Checked today: **mitex-parser** 0.2.4 (Apache-2.0) has had no release since June 17, 2024, so it is a reference, not a dependency. **math-core** 0.8.2 (September 1, 2026, MIT, MSRV 1.96) is a test comparison only, as Wave 4 said. Our `textweaver-math` LaTeX math parser already covers the common subset (ADR-0018), so the LaTeX loader parses document structure and hands `$...$`, `\[...\]`, and the math environments to it.
- **EML and MHTML.** **mail-parser** 0.11.9 (`https://crates.io/crates/mail-parser`, Apache-2.0 or MIT), unchanged since September 9, 2026; releases monthly; safe Rust, fuzzed, with an `encoding_rs` feature. RFC 2557 (MHTML): resolve `Content-Location` and `Content-ID` references, then read the root HTML through the existing HTML loader. Hostile input: cap parts, nesting depth, and decoded size.
- **Risk:** LaTeX course notes use packages and macros freely; the subset must say what it skips and read the rest as text, never fail the whole file. Braille and speech of the skipped commands: say the command name at high verbosity, nothing at low.

### 2.4 MathCAT: braille and navigation (W5c4)

- **mathcat** (`https://crates.io/crates/mathcat`, MIT): stable 0.7.5; 0.7.6-rc.3 (August 23, 2026) is what W4c1 pins; 0.7.7-alpha.1 (September 23, 2026) is newest. No 0.7.6 final today.
- **Issue #827** (`https://github.com/daisy/MathCAT/issues/827`, "GetNavigationBraille panics in no-unsafe builds due to cross-document node insertion"): **open**, created September 26, 2026, in the project's triage column, no assignee, no linked pull request. The workspace has `unsafe_code = "deny"` (root `Cargo.toml` line 261), so this is the gate for braille and for navigation through MathCAT. The issue itself describes the fix (copy subtrees into the destination document rather than inserting nodes across documents).
- **Branch if #827 is still open when 5b launches:** vendor `mathcat` 0.7.6-rc.3 under `third_party/mathcat` with the described fix as a patch, the way Xilem, Masonry, and Parley are vendored and patched (ADR-0027, `third_party/xilem/textweaver.patch`), and offer the patch upstream. That keeps the workspace on safe Rust and the wave on schedule. It costs about a day and a 9.6 MB rules tree already in the crate.
- **Braille codes.** MathCAT produces Nemeth and UEB (and other codes). The BRF writer today is UEB grade 1 for text (ADR-0017, pure Rust tables), with grade 2 only through the optional `liblouis` feature. Which math code goes first is Jon's decision (open question 3); the plan builds both behind one setting, `[braille] math_code = "nemeth" | "ueb"`, and defaults to the one he names.
- **Navigation.** MathCAT's navigate calls give the spoken text and braille for the current node. The terminal reader's math exploration (Alt+Shift+X, P2e) runs on our own tree; W4c1's brief asks it to propose the MathCAT navigation API in its report. W5c4 wires it as a second engine behind `[reading] math_engine`, with the built-in tree as the fallback.

### 2.5 Summaries and translation (W5s, W5e)

- **rten, rten-generate, rten-text** 0.26.0 (August 29, 2026, MIT or Apache-2.0, MSRV 1.94), unchanged; the workspace is on 0.26 already (`rten`, `rten-generate` with `text-decoder`, `rten-text`). rten-generate's merged-decoder KV cache is the template (the Whisper example, which W3f followed).
- **kitoken** 0.11.0 (May 10, 2026, BSD-2-Clause), unchanged; loads `tokenizer.json` and `.spm`, Unigram included. Test that its token ids match the reference exactly.
- **tokenizers** (the fallback): 0.23.2 stable (September 3, 2026); **1.0.0-rc.2 (September 21, 2026)** is newest. Pin `0.23` with `default-features = false, features = ["fancy-regex"]` if used at all; better, do not add it unless kitoken fails the id test.
- **OPUS-MT models** (`https://huggingface.co/Xenova/opus-mt-en-es`): the quantized pair is `encoder_model_quantized.onnx` 52.9 MB plus `decoder_model_merged_quantized.onnx` 60.2 MB, **113.1 MB per direction**, plus `tokenizer.json`. Four pairs (es, fr, de, ar, matching W4d's languages) in one direction each are about 450 MB; both directions about 900 MB. The model page shows no license statement; Wave 4's research says the upstream Helsinki-NLP models are CC-BY 4.0 and the Hugging Face tags disagree, so **the license is unverified** and the download prompt must show "CC-BY 4.0, attribution required" only after W5e reads the upstream model card and records it in `about/data-files.md`. Mask the pad token (65000) while decoding.
- **Summaries.** No solid LexRank crate; write it in-house (TF-IDF cosine, damping 0.85, power iteration, top k in document order), as Wave 4 said. Star's `summarize.py` used sumy's LexRank with English stemming and stop words; textweaver's version needs a stop-word list (SCOWL is already bundled) and no stemmer to start. Embeddings (all-MiniLM-L6-v2, 23 MB) stay optional and off.
- **ort** 2.0.0-rc.13 (July 28, 2026) remains a release candidate and is not pure Rust; not used.

### 2.6 The GUI, third pass (W5a4)

- **AccessKit** 0.25.1 (September 25, 2026), unchanged; the workspace vendors it through Masonry's patch.
- **Parley** 0.11.1 (August 16, 2026), unchanged. The vendored copy is 0.8.0 patched to AccessKit 0.25. Parley main dropped its AccessKit feature (PR #716), so an upgrade moves the accessibility bridge into our `DocumentView`. Wave 4 says: do edit mode on 0.8, then upgrade "as a separate task in Wave 5 with a measurement". That is W5a4's first item, and the measurement is the ADR-0027 numbers (open 10 million characters under 300 ms, a highlight move under 30 ms, memory).
- **Xilem** on crates.io is still 0.4.0 (October 29, 2025); the vendored copy is the September 14, 2026 revision of main. No new release. **Unverified today:** whether Xilem main has bumped AccessKit since ADR-0027's patch (a GitHub search returned nothing in the Wave 4 review either).
- **AccessKit PR #758** (AT-SPI Collection, for Orca's structural navigation): **draft, open, last updated September 5, 2026**; it relies on unpublished dependencies and has no tests. Orca heading navigation in the GUI waits for it; W5t's Orca script must not assume it.

### 2.7 Automated screen-reader tests (W5t)

- **Guidepup** (`https://github.com/guidepup/guidepup`, MIT): newest release **0.34.0 (August 31, 2025)**, "now uses NVDA 2026.2"; drives VoiceOver on macOS and NVDA on Windows with one API and returns the spoken phrases. Its README is browser-first ("framework agnostic" but no native-app example). `guidepup/setup-action` was archived on September 26, 2026 (Wave 4 research); use `npx @guidepup/setup setup --ci`. **Whether it can drive a native winit window is unverified; that is the spike's first question.** npm refused the package page today (HTTP 403), so the npm version was not read.
- **accessibility-cli** (`https://github.com/DioxusLabs/accessibility-cli`, Apache-2.0 or MIT): reads and queries accessibility trees on macOS, Windows, Linux (AT-SPI), iOS Simulator, and Android; built from source (`cargo install --path packages/accessibility-cli`); **not on crates.io yet** (a published flow is "planned"); 105 commits, no version tag seen. Good enough for a CI tree dump on three systems with one tool; not a dependency of the product.
- **What exists already:** `crates/textweaver-xilem/tools/uia-report.ps1` (Windows, passes), `tools/atspi-check.sh` and `tools/atspi-dump.py` (Linux under Xvfb, runs in CI), a macOS smoke test. W5t builds on these rather than replacing them.
- **VoiceOver by a person:** still out of scope; no Mac. A VoiceOver run through Guidepup on a `macos-14` runner is possible only if the runner's VoiceOver can be enabled by `@guidepup/setup`; **unverified**.

### 2.8 Beta readiness and packaging (W5p)

- **Release pipeline today** (`docs/dev/releasing.md`): `release.yml` builds Windows, macOS universal (ad hoc signed, not notarized), and the Linux AppImage plus tarball on Ubuntu 22.04, with one checksums job and provenance attestations. `cargo xtask release X.Y.Z` sets the version, dates the changelog from the clock, and tags. W4f adds the aarch64 AppImage, the dated listening-checklist check, and the beta release notes.
- **The GUI's packages:** `cargo xtask gui-dist` makes the Windows zip (13.8 MB holding a 32.8 MB `textweaver-gui.exe`), and the GUI workflow uploads the macOS `.app` zip and the Linux tarball and AppImage on main and by hand (ADR-0027). None of them is in `release.yml` yet. W5p adds them, after Jon decides whether the GUI ships in the beta (open question 4).
- **Signing:** Windows Authenticode and Apple notarization both need paid certificates or an Apple Developer ID; `docs/dev/releasing.md` says "when funding allows". The beta can ship unsigned with the Gatekeeper and SmartScreen notes in `docs/install.md`, as the alphas did. Jon's decision (open question 5).
- **Documentation drift, measured today:** `docs/README.md` lists ADR-0027 out of order and omits ADR-0023 from its Decisions list; it says "29 crates" while `crates/` holds 33 folders (32 default members plus the two GUI crates); `docs/roadmap.md`'s status is dated September 26 and still says "Wave 3 is planned" and "Agent P2e is still running". W4f's doc pass covers a listed set of user guides, not these. The cure is a tool: `cargo xtask settings-doc --check` (the settings reference generated from `SettingsSchema`) and a `docs-check` that compares the ADR index, `docs/README.md`, and the crate count with the tree.

### 2.9 What could not be verified today

- The npm page for Guidepup (HTTP 403); the version comes from GitHub releases.
- Whether Guidepup drives a native (non-browser) window, and whether VoiceOver can be enabled on a GitHub macOS runner.
- The license of the Xenova OPUS-MT ONNX ports (the model page shows none; the upstream model card was not fetched).
- Whether Xilem main has moved past AccessKit 0.24 since ADR-0027.
- MathCAT's 0.7.7-alpha.1 contents beyond its date; it predates #827, so it does not fix it.
- Every Wave 4 number this plan branches on (rope timings, GUI memory after W4s, the lean build size): they do not exist yet.

## 3. Dependencies on Wave 4

Each item names the Wave 4 outcome it depends on, and what changes if Wave 4 finds something different. These are the gates and branches the orchestrator checks at Wave 5's step 0.

1. **W5r (the rope ADR) depends on W4b's step 7,** the ropey 2.0.0-beta.1 and crop 0.4.3 numbers on the edit traces, recorded as a status update on ADR-0002.
   - **Branch A (expected): ropey 1.6 is within 1.5 times of the fastest on every trace, or the slow traces are ones a user never feels (under 5 ms).** W5r writes ADR-0034 as "stay on ropey 1.6; revisit when ropey 2.0 is final", a half-day, and spends the rest of its slot on `textweaver-text` hot spots W4b listed but did not fix. No rope change.
   - **Branch B: crop or ropey 2 is at least 2 times faster on a trace a user feels (typing echo, a window slide, a paste of 10 MB).** W5r writes ADR-0034 as a migration plan behind `DisplayIndex`, and implements it in 5a only if the change stays inside `textweaver-text` and `CharPos` keeps its meaning (conversion cached per chunk). If `CharPos` would have to become a byte position, the migration is Wave 6 and after the beta (open question 2).
   - **If W4b did not run the rope measurement,** W5r runs it first (numbers only, no change), then decides.
2. **W5m (memory and fuzz) depends on W4s's memory attribution and on whether W4a2 took the engine-side fixes.**
   - **If the GUI is back near 100 MB working set after 4b:** W5m drops the GUI item and is fuzz and hostile input only, which makes it light enough to share 5a with a fourth agent if one is needed.
   - **If the growth is attributed to engines and the app (lazy loading of Piper, Whisper, OCR, the lexicon) and nobody fixed it:** W5m takes those fixes, which touch `textweaver-engines`, `textweaver-app` opening, and `textweaver-lexicon`. Then W5m is 5a's second app agent, and W5x's app edits are confined to messages and the store. The orchestrator checks the two file lists at their first merge of main.
   - **If W4s found renderer buffers:** the fix is inside xilem and belongs to W5a4, not W5m.
3. **W5c4 (math braille and navigation) depends on two things.**
   - **MathCAT issue #827.** Closed and in a tagged release: pin that release, no vendoring. Open: vendor 0.7.6-rc.3 with the fix as a patch (section 2.4) and offer it upstream; ADR-0036 records both paths.
   - **W4c1 landing.** If W4c1 slipped or was cut, W5c4 does W4c1's brief first (speech, EPUB MathML, the `[reading] math_engine` setting, ADR-0029) and braille second; the sub-wave's gate then hears speech, and braille moves to 5c.
4. **W5a4 (the GUI's third pass) depends on W4a3 (GUI edit mode) and on Jon's sessions 1 and 2.**
   - **W4a3 landed and session 2 passed:** W5a4 starts with the Parley upgrade and the measurement, then syllables and difficult words, the voice manager, and the drawn labels from the catalog.
   - **W4a3 slipped:** edit mode comes first in W5a4 (ADR-0033 keeps its number and W4a3's brief), the Parley upgrade waits for edit mode to be measured on 0.8, and the voice manager moves to Wave 6.
   - **The wxDragon spike:** Jon said it goes after session 2. If it is still there at Wave 5's start, the orchestrator removes it in step 0 after showing the file list, and `gui.yml` with it; W5t and W5p never touch `crates/textweaver-gui`.
5. **W5x and every agent that adds a message depend on W4d (translations).**
   - **W4d landed:** every new message is a catalog id in `en` with the five other languages falling back to English and listed by the checker as "needs translation". W4d's checker must allow a fallback with a list, not fail; if it fails on a missing translation, step 0 adds the fallback list to the checker before any Wave 5 agent starts. No Wave 5 agent writes a `format!` message the catalog does not know.
   - **W4d slipped:** messages stay `format!` strings as in Wave 4, and W4d's brief moves to Wave 5's sub-wave 5c as the only app-message agent there (displacing W5e, which then waits for Wave 6). Section 10 lists this as a cut.
6. **W5e (translation) depends on W4d's language list and on Jon's model approval** (open question 1). Without approval it is cut; without W4d, it still works but its prompts are English.
7. **W5c3 (LaTeX, EML, MHTML) depends on W4c2's hostile-input patterns and its three fuzz targets** being in the nightly matrix (W4f). If they are not, W5m adds them with its own.
8. **W5p (beta readiness) depends on W4f's** aarch64 AppImage, dated listening check, and beta release notes. Anything W4f left undone is W5p's first item. W5p also depends on open questions 4 and 5.
9. **Every budget number depends on Wave 4's runbook step 2,** the measured lean build. Section 6 uses Wave 4's allowances and says where the measured numbers replace them.

## 4. The proposed shape

Three sub-waves of three agents, one light fourth in 5c. Each sub-wave has one goal, one merge point, and one thing for Jon to hear or feel on the display. At most three agents run at once, and at most two build heavily at the same moment. A sub-wave launches only after the previous one's last merge, flush, and memory check.

### Sub-wave 5a: settle the foundations, and the Braille pass

**Goal.** Close the questions the beta cannot ship with open (the rope, memory, hostile input), and make the terminal reader right on a Braille display.

**Agents.**

- **W5x, Star leftovers and the Braille pass.** The one app-message agent of 5a. Library search by DOI, ISBN, and author; Star profiles imported; the lexicon loaded off the input thread; study lists on the list model; UX-1's item 9 (`y` and `n` in the "Open it?" lists); the Braille pass over every status-line message (meaning first, short, counts and positions before labels); a "Braille displays" section of `docs/screen-readers.md` that is tested, not "not tested yet". Terminal only.
- **W5m, memory and fuzz.** Fuzz targets for math, cite, theme, lexicon, vault, and JSON-RPC, with seed corpora and hostile-input limits; the nightly matrix; whatever GUI or engine memory work Wave 4 left (branch 2 above). No app messages.
- **W5r, the rope decision.** ADR-0034 from W4b's numbers (branch 1 above); `textweaver-text` hot spots either way. No app edits beyond a one-line hook.

Why together: three different crates most of the time (app messages and store; fuzz and engines; text). W5x and W5m may both touch the app only in branch 2, and then in different modules (messages against opening). The rope work is the risky one; it runs beside two agents that do not depend on it, so a "stay" or a "go" costs nobody else time.

**Gates.**

- W5x merges first, W5m second, W5r last (its `textweaver-text` change is the widest).
- **Jon's Braille session (session B1):** the terminal reader with NVDA, then JAWS, on his display: the status line in screen-reader mode with `cursor = "status"`, a list, a prompt, a position report, the outline, a math formula in grade 1, and "read current line" on the title line. His findings become fixes in W5c4's and W5a4's briefs.
- W5r's ADR is the second gate: Jon reads it and says stay or go (open question 2 if it is "go").

**Merge point and checks.** The Wave 4 common set (native workspace checks, the Docker all-features check one at a time by the orchestrator, `keyboard --check`, `deps --check`, `notices --check` when `Cargo.lock` changed, the link and site checks, the `publish`-off build for app changes, `-p textweaver-xilem` for GUI changes), plus the bench gate against main's artifact for W5r, and the catalog checker for W5x. Then the flush.

### Sub-wave 5b: documents, math braille, and summaries

**Goal.** The formats and the math a student meets in a course, on speech and on the display.

**Agents.**

- **W5c3, LaTeX, EML, and MHTML.** `latex.rs` and `eml.rs` in `textweaver-formats`, each with hostile-input limits and a fuzz target; `docs/converting.md` updated with the file list. ADR-0035.
- **W5c4, math braille and navigation.** Nemeth and UEB math through MathCAT for the BRF writer and for the display; formula navigation through MathCAT behind `[reading] math_engine`; the built-in tree stays the fallback. The one app-message agent of 5b (the exploration wording). ADR-0036.
- **W5s, summaries.** A new crate, `textweaver-summary`, with in-house LexRank, `tw summarize`, one palette command ("Summarize", which opens the summary as a list of sentences that jump to their place), and the difficult-word overlay's definitions from the lexicon. ADR-0037 (part 1 of offline intelligence). Light.

Why together: formats, math plus writers, and a new crate: disjoint. W5c4 is the only one in the app's messages; W5c3 adds one open-failure id per format and W5s one palette command, each in its own file.

**Gates.**

- W5c4 merges first (its app changes are widest), then W5c3, then W5s.
- **Jon's check 2:** open `fixtures/c3/notes.tex` and `fixtures/c3/message.eml`; read a formula and hear MathCAT's navigation; `tw convert --to brf` of `fixtures/c4/quadratic.md` on the display in the code he chose; `tw summarize` of a chapter, then the Summarize command in the reader.

**Merge point and checks.** As in 5a, plus `cargo fuzz build` for the new targets and ten minutes of each in the container, and the BRF snapshot tests on the display's line length.

### Sub-wave 5c: the GUI's third pass, automated screen-reader tests, translation, and beta readiness

**Goal.** Make the GUI ready to ship if Jon says it ships, put automated screen-reader checks beside his sessions, and remove the last reasons the beta waits.

**Agents.**

- **W5a4, the GUI's third pass.** The Parley upgrade with the ADR-0027 measurements repeated; syllables and difficult words drawn; the voice manager dialog on the app's list model; the catalog's languages in drawn labels (W4d left them English); session findings. `textweaver-xilem` only. Status updates on ADR-0027 and ADR-0033; ADR-0040 only if the upgrade changes the accessibility bridge's design.
- **W5t, automated screen-reader tests.** A spike with a written answer first: does Guidepup drive `textweaver-xilem.exe` on a Windows runner? Then the accessibility-cli tree dump on three systems, an Orca script under Xvfb built on `tools/atspi-dump.py`, and a VoiceOver attempt on the macOS runner. It builds only in CI; on this machine it writes scripts and workflows. ADR-0039 if any of it becomes a standing check. Light on this machine.
- **W5e, offline translation.** Only with Jon's approval of the models (open question 1). A new crate, `textweaver-translate`, on rten-generate with kitoken; the document or the selection, in a background job with progress and cancel; downloads after a yes with the license said and SHA-256 checked. ADR-0038. Heavy to build (rten is already in the tree, so mostly its own code).
- **W5p, beta readiness (light fourth).** The GUI packages in `release.yml` if Jon says the GUI ships; `cargo xtask settings-doc --check` and a docs consistency check; the doc pass over the files W4f did not list (`docs/README.md`, `docs/roadmap.md`, `docs/star-gaps.md`, `docs/dev/architecture.md`, the ADR index); the beta checklist in `docs/dev/releasing.md`; the signing note. No crate code. No ADR.

Why together: xilem, workflows and tools, a new crate, and docs: disjoint. W5t and W5p both touch `.github/`: W5t owns `gui-xilem.yml` and a new `a11y-tests.yml`; W5p owns `release.yml`, `ci.yml`, and `nightly.yml`. W5e's app edits are one palette command and one settings section (four places); it is the app-message agent of 5c by default, with W5a4 confined to xilem.

**Gates.**

- Merge order: W5e (app and store), then W5a4, then W5t, then W5p last (its docs describe the merged state).
- **Jon's session 3 (GUI):** NVDA, then JAWS, on the display: the upgraded view reads the same as before (his session 2 checklist repeated), syllables on, the voice manager, a translated selection if W5e ran.
- **The beta readiness review:** the orchestrator gives Jon one page: every package built by the release workflow on a branch (`workflow_dispatch`, no tag), the listening checklist dated, the docs checks green, the fuzz run clean for a week, the open issues list. Jon decides the release. Nothing tags.

**Merge point and checks.** As in 5b, plus the release workflow run by hand on W5p's branch, `cargo xtask release --dry-run`, the GUI workflow on all three systems, and W5t's new workflow green or its failures explained.

### Timeline

Durations, not promises. Wave 4 is planned to end around Wednesday, October 7, 2026 if it starts on Monday, September 28. Wave 5 then needs Wave 4's numbers written up, a Docker restart, and the plan files fixed, so the earliest realistic start is **Monday, October 12, 2026**. Sub-wave 5a about three days (to Wednesday, October 14), the Braille session Thursday, October 15; 5b about three days (Friday, October 16 to Tuesday, October 20), Jon's check Wednesday, October 21; 5c about four days (Thursday, October 22 to Tuesday, October 27), session 3 Wednesday, October 28, and the beta readiness review around **Thursday, October 29, 2026**. If Wave 4 slips a week, every date slips a week; nothing in Wave 5 is tied to a calendar date.

## 5. The dependency and conflict map

### 5.1 Crates each agent owns

- **W5x:** `crates/textweaver-app/src/{lists,list_model,help,library,study}.rs` and the message strings they build (catalog ids after W4d), `crates/textweaver-store` (document metadata fields: DOI, ISBN, author, one commit; Star profiles import in `migrate`), `crates/textweaver-cli/src/cmd/{library,migrate}.rs`, `crates/textweaver-lexicon` (loading off the input thread only), `crates/textweaver-tui/src/ui.rs` (the status line's composition order), `docs/screen-readers.md` (the Braille section and checklist), `docs/library.md`. Not: `opening.rs`, `playback.rs`, `textweaver-xilem`, engines, formats.
- **W5m:** `fuzz/` (new targets, seed corpora, `README.md`), `.github/workflows/nightly.yml` (the fuzz matrix lines only, given to W5p if W5p is running; in 5a W5m edits it directly), hostile-input limits inside `textweaver-math`, `textweaver-cite`, `textweaver-theme`, `textweaver-lexicon` (reader), `textweaver-vault` (importer), and the JSON-RPC decoder in `textweaver-app/src/rpc`; in branch 2, `textweaver-engines` and `textweaver-app/src/opening.rs` for lazy loading. Not: `textweaver-xilem` (W5a4's), messages, the store.
- **W5r:** `crates/textweaver-text` (all of it), ADR-0034, a status update on ADR-0002, `xtask/src/bench.rs` (the edit traces), `docs/dev/testing.md` (the measurement section). Not: any other crate beyond a one-line adapter it reports.
- **W5c3:** `crates/textweaver-formats/src/{latex,eml}.rs` (new), the registry lines, `fuzz/` (two targets: `latex`, `eml`), `fixtures/c3/`, `docs/converting.md`, ADR-0035, one open-failure id per format. Not: the Markdown, HTML, EPUB, RTF, or ODT loaders; `textweaver-math` (it calls the existing parser).
- **W5c4:** `crates/textweaver-mathcat` (W4c1's crate; if #827 is open, `third_party/mathcat/` with its patch and `TEXTWEAVER.md`), the BRF math path in `crates/textweaver-writers`, `crates/textweaver-app/src/math_explore.rs`, the `[braille] math_code` setting (four places), `docs/math.md`, `docs/converting.md`'s braille lines, `fixtures/c4/`, ADR-0036. Not: `textweaver-math`'s parsers, the EPUB loader, `textweaver-xilem`.
- **W5s:** `crates/textweaver-summary` (new), `crates/textweaver-cli/src/cmd/summarize.rs`, one palette command and its list in `textweaver-app` (a new `summary.rs`), the difficult-word definitions hook in `textweaver-aids`, `docs/reading.md` (one section), ADR-0037. Not: `textweaver-lexicon` beyond its public API, the store beyond one `[summary] sentences` setting (four places).
- **W5a4:** `crates/textweaver-xilem` (all of it), `third_party/xilem/parley` (the upgrade, with `textweaver.patch` and `TEXTWEAVER.md` updated), the UI Automation report, `docs/gui.md`, `docs/screenshots/xilem-gui/`, status updates on ADR-0027 and ADR-0033, ADR-0040 reserved. Not: `textweaver-app` beyond one-line hooks, `.github/`, workspace members.
- **W5t:** `tools/a11y/` (new: the Guidepup scripts, the accessibility-cli invocations, the Orca script), `tools/atspi-dump.py` (extended), `.github/workflows/gui-xilem.yml` and a new `.github/workflows/a11y-tests.yml`, `docs/dev/testing.md` (a section), ADR-0039. Not: any crate's Rust code beyond a `--script` flag it proposes in its report; `release.yml`, `ci.yml`, `nightly.yml` (W5p's).
- **W5e:** `crates/textweaver-translate` (new), `crates/textweaver-cli/src/cmd/translate.rs`, one palette command and a background job in `textweaver-app` (a new `translate.rs`), the `[translate]` settings section (four places), `about/data-files.md` (the model licenses), `docs/reading.md` (one section), the dependency block "W5e" (kitoken), ADR-0038. Not: `textweaver-lexicon`, engines, `textweaver-xilem`.
- **W5p:** `.github/workflows/{release,ci,nightly}.yml`, `xtask/src/{release,dist,docs_check}.rs` (the last new), `docs/README.md`, `docs/roadmap.md`, `docs/star-gaps.md`, `docs/dev/{architecture,releasing,building}.md`, `docs/adr/README.md` (index lines only), `docs/settings.md` (generated section), `CHANGELOG.md`'s "Unreleased" section, `docs/install.md` (signing notes). Not: any crate's Rust code beyond a compile fix it reports; repository settings; branches.

### 5.2 Shared-file hotspots

Ranked as in Wave 4, with Wave 5's rule for each.

1. **`crates/textweaver-app`.** One message agent per sub-wave: W5x (5a), W5c4 (5b), W5e (5c). Others make one-line hooks in their own new file (`summary.rs`, `translate.rs`) and one palette entry, and say so. In 5a's branch 2, W5m's app edits are confined to `opening.rs` and the engine registry.
2. **`docs/history/tasks.md`, `docs/site/*`, `docs/adr/README.md`.** Append-only status lines; the site regenerated after merging, never hand-merged; ADR numbers fixed here (section 8).
3. **Root `Cargo.toml` and `Cargo.lock`.** New dependencies in a block headed with the agent's name (W5c3: mail-parser; W5e: kitoken; W5c4: mathcat or the vendored path; W5r: crop or ropey 2 only in branch B). `Cargo.lock` regenerated after merging main. `THIRD-PARTY-NOTICES.md` never committed by an agent.
4. **`crates/textweaver-store` and `settings_schema.rs`.** New settings: W5x (metadata is state, not settings; profiles import touches `migrate`), W5c4 (`[braille] math_code`), W5s (`[summary] sentences`), W5e (`[translate] *`). Appended at the end of their section, one commit each, four places each.
5. **`crates/textweaver-cli/src/main.rs`.** New subcommands (`summarize`, `translate`) in alphabetical position, one line each.
6. **`crates/textweaver-formats`.** Only W5c3 in Wave 5 (W4c2 finished in 4c). No overlap.
7. **`crates/textweaver-writers`.** Only W5c4 (the BRF math path).
8. **`fuzz/`.** W5m in 5a, W5c3 in 5b: sequenced. Each adds targets at the end of `Cargo.toml` and `README.md`.
9. **`.github/`.** W5m (5a, nightly matrix), W5t and W5p (5c, disjoint files). If W5m and W5p both need `nightly.yml`, W5m's lines land in 5a and W5p edits after.
10. **`third_party/`.** W5c4 (mathcat, if vendored) and W5a4 (parley): different folders, different sub-waves.
11. **The message catalog (`crates/textweaver-lexicon/locales/` or where W4d put it).** Every message agent adds `en` ids; the checker lists the untranslated ones. No agent edits another language's file except to add a translation it is sure of.

### 5.3 The order that minimizes conflicts

- 5a: W5x (app messages, store, cli, tui) and W5m (fuzz, small parser limits, maybe engines and opening) and W5r (text): disjoint except branch 2, where the orchestrator checks W5x's and W5m's app file lists at their first merge of main.
- 5b: W5c4 (mathcat, writers, app math exploration) and W5c3 (formats) and W5s (new crate, one app file): disjoint.
- 5c: W5e (new crate, one app file, store) and W5a4 (xilem, parley) and W5t (tools, two workflows) and W5p (docs, xtask, three workflows): disjoint.

Within each sub-wave the merge order is the agent with the widest app change first (W5x, W5c4, W5e), so the others merge main once and adapt.

## 6. Resource budget

Measured today, Sunday, September 27, 2026, at 11:48: D: 871.7 GB free of 1,862 GB (floor 200 GB); host RAM 63.8 GB with 40.3 GB free and Docker idle (so about 24 GB in use before any build); Docker's machine 12 CPUs and 33.6 GB (31.3 GiB); Docker images 55.2 GB (47.4 reclaimable, Jon's other projects, never touched), volumes 26.4 GB, build cache 37.4 GB, virtual disk 125.6 GB; `D:\sccache` 5.4 GB; `tw-target-orch` 22 MB (the container cache is cold until Wave 4's step 2 warms it); native build folders 55 GB (`target\orch` 35.5, `ux1` 9.3, `release` 4.6, `debug` 3.3, `dist-build` 1.0), all from before the lean profiles, and all flushed in Wave 4's step 1.

The numbers Wave 5 starts from come from Wave 4's end (runbook step 7.4): the measured lean build sizes and the Docker disk after compaction. Until then, Wave 4's allowances hold.

### 6.1 Disk per agent

- Ordinary agent: **45 GB** (native `target` 20 GB, `tw-target-<agent>` 25 GB), to be replaced by 1.5 times Wave 4's measured lean sizes.
- GUI agent (W5a4): **65 GB**.
- rten agents (W5e, and W5s if it takes the optional embeddings): the rten and ocrs crates are already in the tree, built at `opt-level = 3` in the dev profile; add **5 GB** for release-mode test binaries: **50 GB**.
- Light agents (W5t, W5p): no container, no native workspace build: under 5 GB.

### 6.2 Shared

- `D:\sccache`: cap 30 GB. The container cache `textweaver-sccache`: cap 30 GB, warm after Wave 4.
- The orchestrator's own build output: about 40 GB (15 native, 25 container).
- Docker's other data: about 120 GB, unchanged.
- Model downloads (each needs Jon's approval; none happen without it):
  - OPUS-MT quantized pairs, one direction each, four languages: about **450 MB** (113.1 MB each, measured on en-es today). Both directions: about 900 MB. Into the data folder, outside git.
  - all-MiniLM-L6-v2 for embedding summaries: 23 MB, optional, off by default. Not planned; listed so nobody adds it silently.
  - MathCAT's rules tree: 9.6 MB, inside the crate, no download.
  - Guidepup installs NVDA on the **CI runner**, not on this machine; nothing is downloaded here for W5t.
  - accessibility-cli is built from source on CI; if the spike needs it on this machine, that is a source checkout of a few MB, asked for first.

### 6.3 Peak disk

The heaviest moment is sub-wave 5c with W5e and W5a4 building and W5t and W5p light:

- Agents: 50 + 65 + 5 + 5 = 125 GB.
- Orchestrator: 40 GB.
- Caches: 60 GB.
- Existing Docker data: 120 GB (plus whatever Wave 4 leaves before its final compaction; assume 150 GB).
- Models: under 1 GB.

Peak use about 375 GB against about 850 GB free after Wave 4's flush and compaction: **about 475 GB free at the peak**, more than twice the floor. If every agent estimate is wrong by two, the peak is about 500 GB used and 350 GB free, still above the floor. Docker's virtual disk: compact when it passes 300 GB and after 5c.

### 6.4 Peak RAM

- Host 63.8 GB; floor 20 GB free; baseline 24 GB; available for builds about 20 GB.
- Three containers at `TW_MEM=6g`, `TW_CPUS=4`, `TW_JOBS=4`: 18 GB, within the 20.
- The orchestrator's native integration check (8 to 10 GB at the link peak) runs only when at most one container is building.
- **Before any heavy process, free memory must be at least 26 GB** (20 plus a 6 GB container), or 30 GB for the native check. Same rule as Wave 4.
- rten translation at run time: about 300 to 500 MB per loaded pair (the two quantized models plus the KV cache); W5e measures it and unloads a pair after ten minutes idle.
- W5t runs no NVDA on this machine. Jon's own NVDA and JAWS must never be driven by a script here (they are his working tools; a script taking them over would take his screen reader away). That is a rule in W5t's brief.

### 6.5 How many agents at once

Three building, plus one light fourth (5c). Same as Wave 4: disk allows five, memory allows three at 6 GB, merges allow one message agent per sub-wave, Jon's attention allows one session per sub-wave.

### 6.6 Flush points

1. **Before 5a:** Wave 4's step 7.4 flush and compaction; `build-hygiene.ps1` must show every `tw-target-w4*` volume gone and D: near 850 GB.
2. **After each sub-wave's last merge:** `build-hygiene.ps1 -RemoveVolume tw-target-<agent>` (list, then `-Apply`) and `-Flush` (list, then `-Apply`) for each merged agent, as in Wave 4. Never inside a volume from a container, never `docker system prune`, never an unmerged branch's folder.
3. **Models:** never deleted by an agent. A model Jon approved stays in the data folder; removing one is his choice, through the app's own "remove" with a listing first.

## 7. Quick wins

Each is under half a day for one agent. Sizes: tiny is under an hour, small is under half a day.

1. **`docs/README.md`: ADR-0023 is missing from the Decisions list, and ADR-0027 sits before 0024** (tiny; the orchestrator at step 0, or W5p). Also "29 crates" against 33 folders. Value: the index is what a new reader trusts.
2. **`docs/roadmap.md`'s status is stale** (tiny; W5p): it says Wave 3 is planned and P2e is running. Replace the status block with one dated paragraph and a pointer to `docs/history/tasks.md`.
3. **`tw summarize`, no model** (small; W5s): in-house LexRank, top k sentences in document order, the count from `[summary] sentences`. Star had it; the Wave 4 review called it a half-day item.
4. **`cargo xtask settings-doc --check`** (small; W5p): the settings reference in `docs/settings.md` generated from `SettingsSchema` (labels, help, ranges, defaults), with a check in CI. Value: the doc that drifts most is fixed by a tool.
5. **A docs consistency check** (small; W5p, in the same xtask): the ADR index against `docs/adr/`, `docs/README.md`'s Decisions list against the index, the crate count against `crates/`, every `docs/*.md` ending with "See also". Runs in the `docs` CI job.
6. **Fuzz targets for the math parsers and the citation importers** (small; W5m): `latex_math`, `asciimath`, `bibtex`, `ris`, `csl_json`, each with a seed corpus from `fixtures/o/` and `fixtures/p/`. Value: the parsers a student's own writing goes through.
7. **Fuzz targets for the theme reader, the lexicon file, the vault importer, and the JSON-RPC decoder** (small; W5m). Same pattern.
8. **The status line: meaning first** (small; W5x): audit every composed status-line string so the position or the count comes before the label ("Line 12 of 400, Reading" rather than "Reading, line 12 of 400"), and no message runs past 80 cells without the key fact in the first 40. Value: this is what Jon's display shows.
9. **A tested "Braille displays" section in `docs/screen-readers.md`** (small; W5x writes it, Jon tests it in session B1): which mode, which cursor setting, which NVDA and JAWS braille settings, and a five-item checklist.
10. **Library search by DOI, ISBN, and author** (small; W5x): Star's `discovery.py` on the library index and the document metadata, in `tw library --search` and the library list's filter.
11. **Star profiles imported by `tw migrate-star`** (tiny; W5x): W3e's leftover; the format is Star's `gui/mixin_presets.py` JSON.
12. **The lexicon loaded off the input thread** (small; W5x, or W5m in branch 2): the 9.9 MB `lexicon-en.twlex` opens on first "define word" on a helper thread, with "still loading" once. W3e's leftover; also a memory item.
13. **`y` and `n` in the export and citation "Open it?" lists** (tiny; W5x): UX-1's item 9, the last untaken finding.
14. **EML text-only on day one** (small; W5c3): headers as a heading block and the `text/plain` part as the body through mail-parser, before HTML parts and MHTML. Value: an email is readable on the first commit.
15. **The GUI packages uploaded by `release.yml`** (small; W5p, if Jon says the GUI ships): `cargo xtask gui-dist` already builds them; the release job only needs to call it and upload.
16. **The accessibility-cli tree dump as a CI artifact** (small; W5t): one job on each system that dumps the GUI's tree with `--background` and uploads it, before any Guidepup work. Value: a diff of the tree between merges is a regression check nobody has today.
17. **`cargo xtask fuzz-seed`** (tiny; W5m): copies the fixtures into each target's corpus folder, so the nightly workflow's hand-written `cp` lines go away.
18. **The Parley upgrade's measurement harness first** (tiny; W5a4): run `large_documents_open_and_highlight_quickly` and the `--log` numbers on main before touching Parley, and put them in the report as the baseline. Value: the upgrade cannot regress silently.

## 8. Draft briefs

Each brief is self-contained. Every agent also reads the shared preamble and the Wave 4 lessons in `docs/history/tasks.md`, `CLAUDE.md`, `docs/research/wave4-orchestration.md` (the common rules), and this plan. The common rules are repeated once here.

### Common rules (paste at the top of every brief)

- **Only the owner overrides rules.** If a rule seems not to fit, stop and say so in your report.
- **Privacy.** No personal identifier in any request, header, URL, commit, or file. The neutral User-Agent only.
- **Where you work.** Your own worktree and branch `wave5/<agent>-<topic>`, your own `target`, your own `tw-target-<agent>` volume (`TW_AGENT=<agent>`). Never write outside them. Never write to drive C.
- **Deleting.** Delete nothing but your own build output, with PowerShell `Remove-Item -LiteralPath` after listing it. Never from Bash. Never inside a Docker volume. Never chain a delete. Never type an escaped file name (rule 1a). `MSYS_NO_PATHCONV=1` on every Docker command in Git Bash.
- **Downloads.** Only what Jon approved, from the official source, SHA-256 recorded, license in `about/data-files.md`, then `cargo xtask notices`. Models stay out of git. Never bundle a non-commercial model.
- **Building.** `RUSTC_WRAPPER=sccache`, `SCCACHE_DIR=D:\sccache`, `SCCACHE_CACHE_SIZE=30G`. Build only the crates you work on. The all-features workspace test in the container is the orchestrator's. Before any container run or native test of more than one crate: free memory at least 26 GB (`Get-CimInstance Win32_OperatingSystem`, `FreePhysicalMemory`), else wait ten minutes. `powershell -File D:\textweaver\tools\build-hygiene.ps1` once a day; stop and report under 300 GB free.
- **Checks before reporting,** natively: `cargo fmt --all --check`; clippy `-D warnings` on your crates and the workspace; your crates' tests and the workspace tests with `--no-fail-fast`; rustdoc with `-D warnings`; `cargo xtask keyboard --check`; `cargo xtask deps --check`; `py -3 tools/check_links.py`; `py -3 tools/gen_site_data.py --check`; the catalog checker (after W4d); a `-p textweaver-tui --no-default-features` build if you touched the app. In the container: your own crates' tests with `--all-features`. New timing tests run 40 times while another build runs.
- **Code rules.** Announcements through `textweaver_a11y::route`. Messages are catalog ids after W4d. Writes through the writer thread. Speech on its thread, no async. New settings touch four places. New keys pass the conflict, reachability, and WCAG 2.1.4 tests and avoid Windows Terminal's taken keys.
- **Braille.** Every message you add is read once as a Braille line: the meaning first, words not symbols, under 80 cells with the key fact in the first 40. Every list item and status line you compose follows the same rule.
- **Merging.** Merge `main` into your branch at least every second day and before reporting. Regenerate generated files; never hand-merge them. New dependencies in a block headed with your agent's name. Do not commit `THIRD-PARTY-NOTICES.md`. `CHANGELOG.md` lines under a heading with your agent's name.
- **Dates** from the machine: `py -3 -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"`.
- **Report** (plain sentences, headings and lists, no tables): summary in five lines; files changed by crate; the check result lines, native and container; contract change requests or "none"; open issues; measured build output size; what the next agent should do first; **a checklist of at most five things for Jon to try with NVDA, JAWS, and the Braille display, or "nothing to hear"**. One status line under your heading in `docs/history/tasks.md`.

### The NVDA, JAWS, and Braille checklist (every agent that changes what is heard or shown)

Each report's checklist follows this order, and drops what does not apply:

1. **NVDA, self-voicing mode:** the change as textweaver's own voice says it, once, not twice.
2. **NVDA, screen-reader mode, `cursor = "status"`:** the same change read from the status line with "read current line".
3. **JAWS, the same two modes:** the same, and whether JAWS said anything NVDA did not, or stayed quiet where NVDA spoke.
4. **The Braille display:** the status line as it appears in cells: is the meaning in the first 40 cells; does a list item show its position; does a prompt show its label and the typed text; does math show as the chosen code.
5. **The GUI (if touched):** the same change with NVDA and JAWS in the GUI's announcer, then the display on the document's caret line.

### W5x: Star leftovers and the Braille pass (sub-wave 5a, P1)

**Branch** `wave5/x-leftovers-braille`. **ADR:** none. **Merged first in 5a.**

**Owns:** `crates/textweaver-app/src/{lists,list_model,help,library,study}.rs` and their messages; `crates/textweaver-store` (metadata fields on the library entry: DOI, ISBN, author; `migrate` for Star profiles); `crates/textweaver-cli/src/cmd/{library,migrate}.rs`; `crates/textweaver-lexicon` (loading off the input thread only); `crates/textweaver-tui/src/ui.rs` (the status line's composition); `docs/screen-readers.md` (the Braille section and its checklist); `docs/library.md`; `fixtures/x/`.

**Not to touch:** `opening.rs`, `playback.rs`, `window.rs`, `textweaver-xilem`, engines, formats, `textweaver-text`.

**Read first:** `docs/research/usability-terminal.md` (item 9), `docs/screen-readers.md` (all, especially "Braille displays" and the checklist), `docs/star-gaps.md` (document metadata, profiles, study tools), Star's `discovery.py` and `gui/mixin_presets.py` (read only), W3e's status line, W4h's and W4d's status lines and reports.

**Deliverables, in order, one commit each:**

1. **The status line, meaning first.** Audit every string the status line and the title line compose (a test lists them). Reorder so the position, count, or result comes before its label, and no message's key fact sits past cell 40. Keep the spoken form the same where it already reads well; the display and the voice may differ in order only where a test shows both.
2. **The Braille section of `docs/screen-readers.md`,** written to be tested: which mode and cursor setting to use with a display, the NVDA and JAWS braille settings to try (braille tethered to focus or to review, message timeout, "show messages"), and a five-item checklist. Mark each setting "to verify" until Jon's session B1.
3. **`y` and `n` in the "Open it?" lists** after an export and a citation check, consistent with the save list's letters.
4. **Library search by DOI, ISBN, and author:** metadata on the library entry (from the document's front matter, DOCX and EPUB metadata, and `tw cite`'s record when a DOI is known), `tw library --search` matching title, path, text, DOI, ISBN, and author, and the library list's filter using the same matcher.
5. **Star profiles imported** by `tw migrate-star`, with the report line per profile.
6. **The lexicon off the input thread:** the data file opens on a helper thread on the first "define word"; "Dictionary still loading" once; the result list opens when it arrives (the same pattern as the voice cache).
7. **Study lists on the list model** (W3e's leftover: the definitions list and the statistics list on W3a's `ListModel`, so the GUI gets them for free).

**Checks:** the common set; a test that every composed status-line string keeps its key fact in the first 40 cells; the catalog checker.

**Jon's checklist (five, session B1, on the display):** (1) screen-reader mode with `cursor = "status"`: open a document and read the status line by cells; is "Opened" and the title in the first 40? (2) Alt+O outline: does each item show "3 of 12" before its text? (3) a Find prompt: label, then typed text? (4) Shift+W position: position first? (5) `tw library --search 10.1000/xyz` finds the paper by DOI.

### W5m: memory and fuzz (sub-wave 5a, P1)

**Branch** `wave5/m-memory-fuzz`. **ADR:** none; measurements as a status update on ADR-0027 (memory) and in `fuzz/README.md`.

**Owns:** `fuzz/` (targets, corpora, `README.md`), `.github/workflows/nightly.yml` (the fuzz matrix and corpus lines), hostile-input limits in `crates/textweaver-math` (parser depth and token count), `crates/textweaver-cite` (importer sizes), `crates/textweaver-theme` (file size and key count), `crates/textweaver-lexicon` (the reader's block and entry caps), `crates/textweaver-vault` (import file count and size), `crates/textweaver-app/src/rpc` (request size and depth), `xtask/src/fuzz_seed.rs` (new). **In branch 2 only:** `crates/textweaver-engines` (lazy registration) and `crates/textweaver-app/src/opening.rs` (lazy loading of OCR, Piper, Whisper, and the lexicon).

**Not to touch:** `textweaver-xilem` (renderer memory is W5a4's), messages (W5x's), the store, `textweaver-text` (W5r's).

**Read first:** `fuzz/README.md`, `docs/research/wave4.md` (hostile input), P1d's status line (the walker limits), W4s's report (the memory table) and W4a2's status line, W4c2's status line (its three targets).

**Deliverables:**

1. **Day one:** the nightly matrix carries every target in `fuzz/Cargo.toml` (W3d's six, W4c2's three if W4f did not add them). `cargo xtask fuzz-seed` fills each corpus from `fixtures/`.
2. New targets, each with a seed corpus and a hostile-input limit in the parser it feeds: `latex_math`, `asciimath` (the `textweaver-math` parsers, which must never panic and must keep every span inside the source), `bibtex`, `ris`, `csl_json`, `theme`, `lexicon` (a corrupt data file is refused, never read out of bounds), `vault_import`, `rpc` (a request that decodes must encode to one that decodes the same).
3. Ten minutes of each new target locally in the container (nightly Rust lives there, not natively), with any crash fixed and its input kept as a regression test.
4. **Branch 2 (only if W4s's table points at engines or the app and nobody fixed it):** the engine registry probes on first use, the OCR and Piper and Whisper models load on first use, the lexicon on first define; measured with `--log` before and after, working set and private bytes, on the sample document and the 10-million-character one, in the terminal reader and, read-only, in the GUI.
5. A status update on ADR-0027 with the memory numbers, or the line "the GUI was already within 10 percent of its Wave 3 baseline; nothing to do".

**Checks:** the common set plus `cargo fuzz build` for every target, the bench gate against main's artifact (branch 2 changes startup).

**Jon's checklist:** nothing to hear, unless branch 2 changed the order of startup messages; then: "Opened" first, "Dictionary still loading" once on the first define, with NVDA.

### W5r: the rope decision (sub-wave 5a, P1)

**Branch** `wave5/r-rope`. **ADR-0034** ("The rope after measurement: stay on ropey 1.6, or move").

**Owns:** `crates/textweaver-text` (all of it), `xtask/src/bench.rs` (the edit traces), `docs/dev/testing.md` (the measurement section), ADR-0034, a status update on ADR-0002.

**Not to touch:** any other crate beyond a one-line adapter you report; the state file format; `CharPos`'s meaning (a contract change request if you think it must change).

**Read first:** ADR-0002 and its status updates, W4b's report and its ADR-0002 status update (the rope numbers), `docs/research/wave4.md` (ropes), `crates/textweaver-text/src/{document,display}.rs`, P2a's status line (the writer thread's rope clone).

**Deliverables:**

1. **If W4b's numbers do not exist:** run the measurement first, numbers only: ropey 1.6.1, ropey 2.0.0-beta.1, and crop 0.4.3 on the edit traces (typing echo per key, a 10 MB paste, a window slide, `Document::apply` with 10,000 markers, the writer thread's clone), each 40 times under load, medians and worst cases.
2. **ADR-0034,** with the numbers, the decision, and the fallback:
   - **Branch A (ropey 1.6 within 1.5 times of the fastest on every trace a user feels):** "stay; revisit when ropey 2.0 is final". Then spend the slot on the `textweaver-text` hot spots W4b listed and did not fix (segmentation caches, marker index rebuilds), each with a bench number.
   - **Branch B (a 2 times or larger gap on a trace a user feels):** the migration plan. `CharPos` keeps its meaning; a per-chunk char-to-byte index makes the conversion O(log n); `DisplayIndex` moves onto it; every public signature of `textweaver-text` stays. Implement it only if it stays inside `textweaver-text`; measure the same traces after; the bench gate must not regress peak heap by more than 10 percent. If `CharPos` would have to become a byte position, stop, write that in the ADR, and leave the migration for Wave 6 (open question 2).
3. Proptests that the new or unchanged rope keeps every invariant ADR-0002 states: markers ordered and in bounds after `apply`, segment ranges tiling, history invariants, `DisplayIndex` round trips for UTF-16 and bytes on multibyte text.

**Checks:** the common set; the bench gate; the parity report still with zero unexplained deltas (`cargo xtask parity`); the 40-run rule on every timing test.

**Jon's checklist:** nothing to hear. Jon reads ADR-0034 and says stay or go.

### W5c3: LaTeX, EML, and MHTML (sub-wave 5b, P2)

**Branch** `wave5/c3-latex-email`, from `main` after 5a's last merge. **ADR-0035** ("Native LaTeX subset, and email and web archives").

**Owns:** `crates/textweaver-formats/src/{latex,eml}.rs` (new) and their registry lines, `fuzz/` (`latex`, `eml` targets, at the end of the lists), `fixtures/c3/`, `docs/converting.md` (the file list and two sections), one open-failure catalog id per format, dependency block "W5c3" (mail-parser 0.11.9 with `encoding_rs`).

**Not to touch:** the Markdown, HTML, EPUB, DOCX, RTF, ODT loaders; `textweaver-math` (call `parse_latex_math`; a contract change request if it lacks something); `textweaver-app` beyond the two ids.

**Read first:** `docs/research/wave4.md` (LaTeX, email, hostile input), ADR-0018 (what the math parser covers), ADR-0026 and ADR-0031 (W4c2's limits and patterns), the HTML loader (the MHTML root document goes through it), Star's `documents/pandoc.py` (read only: what Star sent to Pandoc for LaTeX).

**Deliverables:**

1. **EML, text first (day one):** headers (From, To, Date, Subject) as a heading and a short block; the `text/plain` part as the body; attachments listed by name and size under a heading, not opened. Then `text/html` parts through the HTML loader when there is no plain part, and inline images' alt text.
2. **MHTML (RFC 2557):** the root HTML part found by `Content-Type` and `Start`, `Content-Location` and `Content-ID` references resolved to their parts for alt text and link targets, then the HTML loader. Caps: parts, nesting depth, decoded size.
3. **LaTeX subset:** our own tokenizer with an explicit group stack and caps (depth, token count, macro expansion count: no user macro expansion beyond `\newcommand` with no arguments); `\section` levels to headings, `itemize` and `enumerate` with depth, `tabular` to tables, `\cite` to citation spans the renderer's rule already knows, `\ref` and `\label` to links, `\footnote` to footnotes, `\emph` and `\textbf` to markers, `verbatim` and `lstlisting` to code, math delimiters and the `equation`, `align`, and `gather` environments to the math parser; `\input` and `\include` read from the same folder with a depth cap and never outside it; unknown commands skipped with their arguments read as text, the command named at high verbosity. Pandoc no longer needed for LaTeX.
4. Hostile-input tests and the two fuzz targets with seed corpora.
5. `tw convert` and `tw text` on every fixture; `docs/converting.md` updated.
6. ADR-0035: the subset, what is skipped and how it is announced, the caps, and mail-parser's role.

**Checks:** the common set, `cargo fuzz build` for the two targets and ten minutes of each in the container, the parity report unchanged.

**Jon's checklist (three):** open `fixtures/c3/notes.tex`: the sections by `h`, a table by `t`, a formula read by the math engine; open `fixtures/c3/message.eml`: the headers, then the body; `fixtures/c3/page.mhtml` reads as the page.

### W5c4: math braille and navigation on MathCAT (sub-wave 5b, P1)

**Branch** `wave5/c4-math-braille`, from `main` after 5a. **ADR-0036** ("Math braille and formula navigation through MathCAT").

**Owns:** `crates/textweaver-mathcat` (W4c1's crate), and if issue #827 is open at launch, `third_party/mathcat/` with `textweaver.patch` and `TEXTWEAVER.md`; the BRF math path in `crates/textweaver-writers` (the `Math` marker's braille); `crates/textweaver-app/src/math_explore.rs` (the exploration messages: the one app-message agent of 5b); the `[braille] math_code = "nemeth" | "ueb"` setting (four places); `docs/math.md`; the braille lines of `docs/converting.md`; `fixtures/c4/`.

**Not to touch:** `textweaver-math`'s parsers (ask W5c3's or your own contract change), the EPUB loader, `textweaver-xilem`, the BRF text tables (UEB grade 1 text stays as it is; grade 2 stays behind `liblouis`).

**Read first:** ADR-0018, ADR-0029 (W4c1) and its report (the proposed navigation API), ADR-0017 (the BRF writer), `docs/research/wave4.md` (MathCAT), `docs/research/wave4-plan-review.md` (MathCAT), MathCAT issue #827 as it stands on launch day, P2e's status line (math exploration), Jon's session B1 write-up.

**Deliverables, in order:**

1. **Day one, the gate:** read #827. Closed and released: pin the release and record it. Open: vendor 0.7.6-rc.3 under `third_party/mathcat`, apply the fix the issue describes (copy subtrees into the destination document), keep `unsafe_code = "deny"`, write the patch file and its description, and put the upstream pull request text in the report. Either way, the crate's tests run on the dedicated thread and survive a panic (W4c1's test).
2. **Math braille in BRF:** for every `Math` marker, MathCAT's braille in the code from `[braille] math_code`, with the code's own indicators, wrapped to the page width without breaking a braille indicator from its symbol; the fallback (MathCAT missing or the feature off) is today's spelled-out speech text in grade 1, and the summary says so. Snapshot tests on `fixtures/c4/` for both codes, checked against published Nemeth and UEB examples (cite the source in the fixture's README).
3. **Formula navigation through MathCAT** behind `[reading] math_engine = "mathcat"`: Alt+Shift+X enters the formula; the arrows and Enter map to MathCAT's navigate calls; each step speaks MathCAT's text and shows its braille on the status line (so the display gets the code); the built-in tree stays for `"builtin"`. Messages meaning-first: the node's text before its kind.
4. The math exploration messages reordered for the display where session B1 showed a problem.
5. ADR-0036: the version or the patch, the codes, the fallback, what is proposed upstream, and the size added.

**Checks:** the common set plus `--features mathcat` natively and in the container, the BRF snapshots, `cargo deny` (no second zip, no yaml-rust), the binary size before and after.

**Jon's checklist (five):** (1) `tw convert fixtures/c4/quadratic.md --to brf` on the display: is the formula in the code you chose? (2) In the reader, Alt+Shift+X on the same formula with `math_engine = "mathcat"`: Right through the terms; is each step's braille on the display and its speech once? (3) the same with `"builtin"`; which is clearer? (4) `[braille] math_code` switched: the BRF changes and the summary says which code; (5) a formula MathCAT cannot parse: the fallback is announced once, not silent.

### W5s: summaries, no model (sub-wave 5b, P2, light)

**Branch** `wave5/s-summaries`, from `main` after 5a. **ADR-0037** ("Offline intelligence, part 1: extractive summaries without a model").

**Owns:** `crates/textweaver-summary` (new: LexRank on TF-IDF cosine, damping 0.85, power iteration, a stop-word list from SCOWL's common words, sentences from `textweaver-text`'s segmenter), `crates/textweaver-cli/src/cmd/summarize.rs`, `crates/textweaver-app/src/summary.rs` (new: the palette command "Summarize" and its list of sentences on `ListModel`, Enter jumps to the sentence), the `[summary] sentences` setting (four places), the difficult-word overlay's definitions hook in `crates/textweaver-aids` (a difficult word at high verbosity says its first definition from the lexicon), `docs/reading.md` (one section), `fixtures/s/`.

**Not to touch:** `textweaver-lexicon` beyond its public API, `textweaver-text`, `textweaver-xilem`, the store beyond the one setting.

**Read first:** `docs/research/wave4.md` (summaries), Star's `summarize.py` and `vocab.py` (read only), ADR-0022 (difficult words), W3e's status line, `crates/textweaver-app/src/lists.rs` (the list model after W5x).

**Deliverables:**

1. The crate: `summarize(&Document, k) -> Vec<(CharRange, String)>` in document order, deterministic, under 200 ms on the 10 MB corpus (measure; sentences over 50,000 are sampled, and the report says how).
2. `tw summarize FILE [--sentences N] [--json]`; the sentences printed one per line, meaning first (no numbering before the text).
3. The palette command, the list ("Summary, 7 sentences"), and the jump.
4. The difficult-word definitions at high verbosity, off by default under `[reading_aids] difficult_definitions`.
5. ADR-0037 with the algorithm, the limits, and the embeddings option left off (why).

**Checks:** the common set; a test that a summary's ranges lie inside the document and are in order; the 40-run rule on the timing test.

**Jon's checklist (two):** `tw summarize fixtures/sample.md`; then Summarize in the reader on a chapter, Down through the list, Enter: does the reader land on the sentence and say it?

### W5a4: the GUI's third pass (sub-wave 5c, P1)

**Branch** `wave5/a4-gui-third-pass`, from `main` after 5b and Jon's session 3 write-up of sub-wave 4c (W4a3). **ADR:** status updates on ADR-0027 and ADR-0033; **ADR-0040** reserved, used only if the Parley upgrade changes the accessibility bridge's design.

**Owns:** `crates/textweaver-xilem` (all of it), `third_party/xilem/parley` (the upgrade to 0.11.1, with `textweaver.patch` and `TEXTWEAVER.md` updated), `crates/textweaver-xilem/tools/uia-report.ps1`, `docs/gui.md`, `docs/screenshots/xilem-gui/`.

**Not to touch:** `textweaver-app` beyond one-line hooks, `.github/` (W5t's and W5p's), the workspace members, `third_party/xilem/masonry*` beyond what the Parley bump forces (say what).

**Read first:** ADR-0027 (the measurements), ADR-0028 and ADR-0033 and their reports, the write-ups of Jon's sessions 1, 2, and 3 (Wave 4), `docs/research/wave4.md` (Parley 0.11, AccessKit), `docs/research/wave4-plan-review.md` (the Parley advice), W4d's status line (drawn labels), W5x's report (study lists on the list model).

**Deliverables, in order:**

1. **Baseline first (tiny):** on main before any change, the harness numbers (open 10 million characters, highlight move median and worst) and the `--log` memory numbers, in the report.
2. **If W4a3 slipped:** edit mode first, per W4a3's brief and ADR-0033; then stop and hand Jon the W4a3 checklist; the rest of this brief waits for his session.
3. **The Parley upgrade** to 0.11.1: the AccessKit bridge moves into `DocumentView` (Parley main dropped its feature), ranged styles through `RangedBuilder::push` for the spoken word, bionic reading, and difficult words; edit mode kept on `PlainEditor`'s successor or our own editor state; the same measurements repeated; the UI Automation report and the harness tests unchanged and green. If the upgrade regresses a measurement by more than 20 percent or breaks a screen-reader action, revert to 0.8.0 and write why in the ADR-0027 status update.
4. **Syllables and difficult words drawn** (Alt+Shift+Z and Alt+Shift+J in the GUI), as the terminal draws them, never changing the accessible text (the syllable dots are paint only; the text runs stay the words).
5. **The voice manager** dialog on the app's list model (W3f's terminal one), with the language and engine filters as rows, and the download after a yes with the license said.
6. **Drawn labels from the catalog:** every drawn label the GUI owns (buttons, dialog titles, the status bar's prefixes) reads its text from the catalog, so W4d's languages reach the GUI.
7. Session 3's fixes, one commit each.

**Checks:** the common set plus the GUI crate's clippy and tests, the UI Automation report in `--background`, the AT-SPI dump in CI, screenshots at 100 and 200 percent, never the foreground, never audio.

**Jon's checklist (five, session 3, with the display on the caret line):** (1) read past the window edge: does the caret and the display stay on the spoken word? (2) syllables on: does NVDA still read whole words? (3) Alt+V: the voice manager, a filter row, Enter; (4) the settings dialog in Spanish (`[interface] language = "es"`): are the drawn labels Spanish and the announcements once? (5) edit mode: type a word, select it, undo (repeated from W4a3, on the upgraded Parley).

### W5t: automated screen-reader tests (sub-wave 5c, P2, light on this machine)

**Branch** `wave5/t-a11y-tests`. **ADR-0039** ("Automated screen-reader checks beside the listening sessions"), written only if a check becomes standing.

**Owns:** `tools/a11y/` (new: the Guidepup scripts, the accessibility-cli invocations, the Orca script), `tools/atspi-dump.py` (extended to a scripted reading session), `.github/workflows/gui-xilem.yml` and a new `.github/workflows/a11y-tests.yml`, `docs/dev/testing.md` (a section), `fixtures/t/`.

**Not to touch:** any crate's Rust code (a `--script FILE` flag for the GUI, if needed, is a proposal in the report for W5a4 to add), `release.yml`, `ci.yml`, `nightly.yml` (W5p's), `crates/textweaver-gui` (removed or leaving).

**Rules of its own:** builds and runs only on CI runners. **Never drive NVDA or JAWS on this machine**: they are Jon's working screen readers. Nothing is downloaded here; `@guidepup/setup` and NVDA are installed on the runner. No personal identifier in any workflow, log, or artifact name.

**Read first:** `docs/research/wave4.md` (automated screen-reader tests), `docs/research/xilem-gui.md` (acceptance tests), ADR-0027 (the checks that exist), `crates/textweaver-xilem/tools/uia-report.ps1`, `tools/atspi-check.sh`, `tools/atspi-dump.py`, the write-ups of Jon's sessions.

**Deliverables, in order, each a written answer before the next starts:**

1. **The tree dump on three systems** (small): accessibility-cli built from source on each runner (pinned to a commit, checked out by hash), the GUI opened with `--background` on a fixture, the tree dumped and uploaded as an artifact; a diff against the previous run's artifact posted in the job summary. This is the regression check that exists after day one whatever the spike finds.
2. **The Guidepup question:** on a `windows-latest` runner with `npx @guidepup/setup setup --ci`, can Guidepup's NVDA API focus `textweaver-xilem.exe`, press Space, and return the spoken phrases? Write the answer with the log. If yes: a script that opens a fixture, reads three sentences, pauses, moves by heading, opens Settings, and asserts the phrases NVDA spoke against Jon's session 1 write-up (the design he chose). If no: record why, and stop the Guidepup line.
3. **Orca under Xvfb:** extend `tools/atspi-dump.py` to a scripted session (open, read, move by heading, open a list) and assert the AT-SPI events and announcements; run it in `a11y-tests.yml`. Do not assume Collection (AccessKit PR #758 is a draft).
4. **VoiceOver on `macos-14`:** try `@guidepup/setup` for VoiceOver; if the runner cannot enable it, record that and keep the macOS tree dump only.
5. ADR-0039 if any of 2 to 4 becomes a standing check: what it checks, what only Jon can check, and the rule that a green run never replaces his session.

**Checks:** actionlint, shellcheck, the PowerShell lint, the new workflow green on the branch or its failures explained with logs.

**Jon's checklist:** nothing to hear. Jon reads the spoken phrases the NVDA script recorded and says whether they match what he heard in his session.

### W5e: offline translation on rten (sub-wave 5c, P3, only with Jon's model approval)

**Branch** `wave5/e-translate`. **ADR-0038** ("Offline translation on rten with OPUS-MT").

**Owns:** `crates/textweaver-translate` (new), `crates/textweaver-cli/src/cmd/translate.rs`, `crates/textweaver-app/src/translate.rs` (new: the palette command, the background job with progress and cancel, the result opened as a new document beside the original with a link back), the `[translate]` settings section (four places: `target`, `pairs_folder`, `unload_after_minutes`), `about/data-files.md` (the model licenses), dependency block "W5e" (kitoken 0.11.0; `tokenizers` 0.23 only if kitoken fails the id test, with `default-features = false, features = ["fancy-regex"]`), `docs/reading.md` (one section), `fixtures/e/`.

**Not to touch:** `textweaver-lexicon`, engines, `textweaver-xilem`, `textweaver-piper` and `textweaver-ocr` (share their download and hash-check code through `textweaver-engines` or a contract change request, not a copy).

**Read first:** `docs/research/wave4.md` (W4e), ADR-0023 (rten, the download and hash pattern, the license prompt), `docs/research/pure-rust-wave3.md` (rten), `rten-examples`' Whisper and any translation example, W4d's status line (the language list and the catalog), Jon's answer to open question 1 (which pairs).

**Deliverables:**

1. **The model card first:** read the upstream Helsinki-NLP model cards for the approved pairs, record each license and SHA-256 in `about/data-files.md`, and make the download prompt say the license and the size (about 113 MB per pair). Nothing downloads without a yes.
2. The crate: the encoder and the merged decoder through rten-generate with the KV cache; kitoken on `tokenizer.json` with a test that its ids match a recorded reference for ten sentences; the pad token (65000) masked; sentences from `textweaver-text`'s segmenter, translated one at a time so progress and cancel work and a failure loses one sentence, not the document; the pair unloaded after `[translate] unload_after_minutes`.
3. `tw translate FILE --to es [--out FILE]` and the palette command; the result as a Markdown document with the source's headings kept; the app's job on a helper thread with "Translating, 12 of 300 sentences" every 2 seconds and Escape to cancel.
4. Measured: time per sentence and RAM per loaded pair on this machine, in the ADR.
5. ADR-0038: models, licenses, the tokenizer choice, what is not done (both directions, other languages), and the fallback (none: without a model the command says so and stops).

**Checks:** the common set; the crate's tests with a tiny recorded model stub (no real model in tests); the real-model test `#[ignore]`d behind `TEXTWEAVER_TRANSLATE=1`, run once locally with output in the report.

**Jon's checklist (three):** (1) the download prompt for one pair: size and license said, "no" once, "yes" once; (2) Translate a chapter to Spanish: progress every 2 seconds, Escape cancels; (3) the result opens beside the original and reads with the Spanish voice W4d chose, or says the current voice was kept.

### W5p: beta readiness (sub-wave 5c, P2, light fourth)

**Branch** `wave5/p-beta-readiness`. **ADR:** none.

**Owns:** `.github/workflows/{release,ci,nightly}.yml`, `xtask/src/{release,dist}.rs` and a new `xtask/src/docs_check.rs` (`cargo xtask settings-doc --check`, `cargo xtask docs --check`), `docs/README.md`, `docs/roadmap.md`, `docs/star-gaps.md`, `docs/dev/{architecture,releasing,building}.md`, `docs/adr/README.md` (index lines), `docs/settings.md` (the generated section), `docs/install.md` (signing notes), `CHANGELOG.md`'s "Unreleased" section.

**Not to touch:** any crate's Rust code beyond a compile fix it reports; repository settings (Jon said no merge gate); branches (no pruning); `gui-xilem.yml` and `a11y-tests.yml` (W5t's).

**Read first:** `docs/dev/releasing.md`, W4f's report and status line, `docs/README.md`, `docs/roadmap.md`, `docs/star-gaps.md`, ADR-0027 (the GUI packages), Jon's answers to open questions 4 and 5.

**Deliverables:**

1. Whatever W4f left undone from its brief (the aarch64 AppImage, the dated listening check, the release notes).
2. **`cargo xtask settings-doc --check`:** the settings reference generated from `SettingsSchema` into a marked section of `docs/settings.md`; the check fails when the file and the schema differ; run in the `docs` CI job.
3. **`cargo xtask docs --check`:** the ADR index against `docs/adr/`, `docs/README.md`'s Decisions list against the index, the crate count against `crates/`, every guide ending with "See also", every `docs/*.md` linked from the index. Fix what it finds (quick wins 1 and 2).
4. **The GUI packages in `release.yml`** if Jon says the GUI ships (question 4): `cargo xtask gui-dist` on the three package jobs, the artifacts uploaded, the checksums job covering them, `docs/install.md` describing them and Gatekeeper and SmartScreen for unsigned binaries (question 5).
5. **The beta checklist** in `docs/dev/releasing.md`: the release workflow run on a branch with `workflow_dispatch`, every package downloaded and started, the listening checklist dated within a week, `docs --check` and `settings-doc --check` green, the nightly fuzz clean for seven days, the open issues list, the sessions' write-ups linked. `cargo xtask release --dry-run` checks the dated items.
6. The doc pass over the files listed under "owns", one commit per file, with a list in the report of every passage changed.
7. The beta release notes finished in `CHANGELOG.md`'s "Unreleased" section, grouped by area, with the "Keys: what changed" section kept at the top.

**Checks:** actionlint, shellcheck, the PowerShell lint, the link and site checks, the two new checks, a manual run of `release.yml` on the branch (no tag), `cargo xtask release --dry-run`.

**Jon's checklist:** nothing to hear. Jon reads the beta readiness page and decides.

## 9. Execution runbook outline

Every step names its checks. "Free memory" is `FreePhysicalMemory` from `Get-CimInstance Win32_OperatingSystem`; "D: free" is the first line of `powershell -File D:\textweaver\tools\build-hygiene.ps1`.

### Step 0: close Wave 4 and fix the plan files

1. Wave 4's runbook step 7.4 is done: everything merged is flushed, Docker's disk compacted with Jon, the final numbers in `docs/history/tasks.md`, the wiki's Wave 4 page filled in.
2. Read the five Wave 4 outcomes this plan branches on (section 3) and write each branch taken into `docs/history/tasks.md` under a "Wave 5" heading: the rope numbers and W5r's branch; W4s's memory table and W5m's branch; #827's state and W5c4's path; W4a3's state and W5a4's first item; W4d's state and the message rule.
3. Apply the briefs to `docs/history/tasks.md`: the three sub-waves, the agents, the ADR numbers (0034 W5r, 0035 W5c3, 0036 W5c4, 0037 W5s, 0038 W5e, 0039 W5t, 0040 reserved for W5a4), the common rules, the Braille checklist order.
4. If W4d landed, confirm the catalog checker allows English fallback with a "needs translation" list; if it fails on a missing translation, add the fallback before any agent starts.
5. If the wxDragon spike is still in the tree, remove it now as its own commit, after showing Jon the file list (his decision from Wave 4).
6. Fix `docs/README.md`'s ADR list (quick win 1) if W5p is not to do it.
7. Ask Jon the five open questions (section 11). Restart Docker Desktop from its menu (Jon does it; never force-quit).

### Step 1: measure and check the floors

1. `build-hygiene.ps1`: D: free at least 400 GB (expected about 850); no `tw-target-w4*` volumes; Docker's virtual disk under 200 GB after compaction.
2. Free memory at least 32 GB with Docker idle.
3. Native and container caches warm from Wave 4; if the container cache is cold (a compaction removed the volume), repeat Wave 4's step 2 (one `cargo test --workspace --no-run --all-features` in `tw-target-orch`) and record the lean sizes.
4. Replace section 6's allowances with 1.5 times the measured numbers.

### Step 2: launch sub-wave 5a

1. Three worktrees from main: `wave5/x-leftovers-braille`, `wave5/m-memory-fuzz`, `wave5/r-rope`. `TW_AGENT=w5x`, `w5m`, `w5r`; `TW_MEM=6g`, `TW_CPUS=4`, `TW_JOBS=4`.
2. Each agent gets its brief from section 8 with the common rules pasted in and its branch from step 0.2 stated in the first line.
3. Twice a day: `build-hygiene.ps1` and free memory into a one-line log under the sub-wave heading.

### Step 3: integrate 5a, in the order W5x, W5m, W5r

For each report: check the measured sizes against the allowance; free memory at least 30 GB and at most one container building; the native checks on the branch; the container all-features check, one at a time; merge, regenerate the site and the notices, commit; flush that agent (volume, then folder, each listed then applied); one line to Jon.

After W5x: build the release terminal reader and hand Jon session B1's checklist (W5x's five items plus the "Braille displays" section's five). After W5r: hand Jon ADR-0034 and ask stay or go.

### Step 4: Jon's session B1 and the rope answer

1. Jon tests when he chooses. The orchestrator writes his findings into `docs/history/tasks.md` under "Session B1", one line each, and the settings he verified into `docs/screen-readers.md` (removing "to verify").
2. His findings become the first items of W5c4's and W5a4's briefs. Nothing in 5b launches before the write-up exists, except W5c3 and W5s, which do not depend on it.
3. Rope: "stay" closes it; "go" with branch B implemented is already merged; "go" with the migration deferred goes into Wave 6's list.

### Step 5: sub-wave 5b

1. Same checks as step 2. Launch W5c4, W5c3, W5s from main after 5a's last merge and flush.
2. W5c4's day-one gate (issue #827) is reported to Jon in one line: pinned release, or vendored with the patch.
3. Integrate W5c4, then W5c3, then W5s. After W5c4: Jon's check 2 (its five items plus W5c3's three and W5s's two, in that order).
4. Flush each merged agent. Check Docker's disk; compact with Jon if over 300 GB.

### Step 6: sub-wave 5c

1. Same checks. Launch W5e (only if Jon approved the models; else the slot stays empty and W5p may build the GUI packages natively in it), W5a4, W5t; W5p as the light fourth.
2. W5t builds nothing here; its workflow runs must never take a runner while the release workflow's manual run (W5p) needs one: sequence them through the orchestrator.
3. Integrate W5e, then W5a4 (release build of the GUI, Jon's session 3), then W5t, then W5p last.
4. Session 3 write-up. Then the beta readiness page (W5p's checklist filled in) to Jon in one message.
5. Flush everything merged. Compact Docker's disk with Jon. Record the final numbers in `docs/history/tasks.md` and hand the wiki's Wave 5 page its results section (the orchestrator edits the wiki; agents never do).
6. Wave 6 planning starts from: the beta decision, ADR-0034's branch, W5t's answers, and section 10's deferred list.

### If a floor is reached

Same as Wave 4: D: under 300 GB, no new heavy process and a flush; under 250 GB, compaction with Jon; under 200 GB, every build stops and Jon is told in one line. Free memory under 20 GB: nothing starts; the next heavy process waits for 26 GB. An out-of-memory kill in a container: `TW_JOBS=2` for that agent, then serialized container builds.

### Keeping Jon informed

One line per merge; one message per sub-wave end with the gate's checklist as a numbered list; a message the moment a floor is crossed; nothing for routine passes. Everything he decides goes into `docs/history/tasks.md` with the machine's date and weekday, and the orchestrator mirrors it to the wiki.

## 10. What to cut or defer to Wave 6

- **W5e (translation): the first to cut.** It needs model approval, about 450 MB of downloads, and a fourth tokenizer path; nothing depends on it. Cut it if Jon says no to question 1, if 5c is short of memory, or if W4d slipped and takes its slot (section 3, item 5).
- **W5t beyond the tree dump: cut to the spike's written answer** if the runners cannot drive a native window. The tree dump stays either way.
- **The rope migration (W5r branch B) if `CharPos` must change meaning:** Wave 6, after the beta (question 2).
- **The Parley upgrade (W5a4 item 3):** revert and defer if it regresses a measurement or a screen-reader action; edit mode on 0.8 is enough for the beta.
- **The voice manager in the GUI (W5a4 item 5):** Wave 6 if W4a3 slipped and edit mode takes W5a4's slot.
- **Embedding summaries (MiniLM):** not planned; Wave 6 only if the no-model summary proves too weak on Jon's documents.
- **Grade 2 (contracted) braille in pure Rust:** Wave 6. It is a table-driven translator with many rules; `liblouis` behind the feature covers it now. Math braille (W5c4) is the higher-value braille item.
- **Star's peripheral list stays dropped:** karaoke video export, feeds, Wikipedia and PubMed quick open, SSML pauses, OGG export and the M4B cover, plugins, an update checker inside the program, a guided tour, a key-code inspector, tapping Ctrl to pause (a terminal cannot see key-up), study tools, the knowledge graph exports. Star's publishing templates (Word APA and AMA templates, the EPUB cover, the publish stylesheets) are worth a Wave 6 agent if students ask; not before the beta.
- **VoiceOver by a person:** out of scope until there is a Mac or a tester.
- **Signing:** a Wave 6 item once funding and question 5's answer exist; the beta ships as the alphas did if Jon says so.
- **A merge gate:** Jon said no in Wave 4; not raised again unless he raises it.
- **Branch pruning:** only after the wave, only with Jon's approval, never by an agent.

## 11. Open questions for Jon

1. **Translation models.** May W5e download the OPUS-MT quantized pairs from Hugging Face (about 113 MB each; English to Spanish, French, German, and Arabic, one direction each, about 450 MB in all), after each download's license is shown? Or is translation deferred to Wave 6, keeping only the no-model summaries in Wave 5?
2. **The rope before the beta.** If W4b's numbers show a gain of two times or more, may Wave 5 change the rope (ADR-0034, branch B) before `0.1.0-beta.1`, or does the text model freeze until after the beta ships?
3. **Math braille code.** Nemeth or UEB first for formulas on the display and in BRF files? The setting will offer both; the default and the order of testing follow your answer. And will you do the Braille session B1 as sub-wave 5a's gate, with which display?
4. **The GUI in the beta.** Does `0.1.0-beta.1` ship the GUI as a supported package on all three systems, or is the beta terminal-first with the GUI marked experimental and packaged only from the GUI workflow? This decides W5p's release work and how hard W5a4's session 3 gates the release.
5. **Signing.** Does the beta ship unsigned (SmartScreen and Gatekeeper notes in the install guide, as the alphas did), or wait for a Windows certificate and an Apple Developer ID?

## See also

- [Wave 4 orchestration plan](wave4-orchestration.md): the shape, budget, and runbook this plan follows.
- [Wave 4 plan review](wave4-plan-review.md) and [Research for Wave 4](wave4.md): the research Wave 5 builds on.
- [Usability pass: the terminal reader and tw](usability-terminal.md): the findings W5x closes.
- [Star features not yet planned](../star-gaps.md): the parity list.
- [Tasks and agent briefs](../history/tasks.md): where the adopted plan goes.
- [Roadmap](../roadmap.md)
- [Documentation index](../README.md)
