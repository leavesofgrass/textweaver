# Cloud Agent plan: three pull requests beside Wave 4 and Wave 5

Written on Sunday, September 27, 2026, by the cloud-agent planner (Fable 5.1), for the owner and the orchestrator. It picks the work a Claude Cloud Agent (a Claude Code session in Anthropic's cloud, separate from this machine) can do by pull request against `main` while the local waves run, within a total budget of $125. Nothing here changes by itself: the owner starts each cloud session, and the orchestrator applies the reservations and the Wave 5 changes.

What it is built on: `CLAUDE.md`; `docs/research/roadmap-2026.md` with the owner's answers; `docs/research/whats-left.md`; `docs/research/wave5-plan.md`; `docs/research/wave4-orchestration.md` with the adopted plan, the owner's answers, and the measured sizes; the Wave 4 adopted plan in `docs/history/tasks.md`; the ADRs, `docs/dev/`, `CONTRIBUTING.md`, and every workflow in `.github/workflows/`; the crates the candidate tasks touch (`fuzz/`, `xtask/`, `textweaver-math`, `textweaver-cite`, `textweaver-theme`, `textweaver-lexicon`, `textweaver-vault`, `textweaver-app`'s `rpc.rs` and `settings_schema.rs`); the git log; and Anthropic's public pricing page and the Claude Code cloud documentation, read today with the tool's default User-Agent. No personal identifier was sent. Nothing was built, run, downloaded, or deleted.

## The short version

- **Three tasks, each one pull request:** the parser fuzz targets (Wave 5's W5m work, done early), the generated settings reference and the docs consistency check (W5p's two quick wins), and a second-tool check of the EPUB and PDF writers in CI (a Wave 6 item). Each can be done and verified on a Linux cloud machine with the repository and its CI. Nothing to hear.
- **Budget:** expected about $72 in all on Claude Opus 5, high about $130 if all three run their worst case. So the third task runs only if the first two leave at least $45. The first two alone: expected $54, high $98, a margin of $27 under $125.
- **Timing:** the fuzz task during Wave 4, now, so it merges before sub-wave 4c and before Wave 5's sub-wave 5a. The docs task during Wave 4's pause. The second-tool task after the pause, if the budget allows.
- **Wave 5 never waits and never duplicates:** a reservation list in the repository names each cloud task's items and files; Wave 5 briefs carry them as "not yours; reserved"; the orchestrator checks the pull requests before each sub-wave. Section 5 has the mechanism.

## 1. The tasks

Each task names why it was chosen, its value to the owner's goal, the files and crates it owns, the conflict check against Wave 4 and Wave 5, and its acceptance criteria. All three fit the cloud: a Linux machine with `rustc`, `cargo`, `rustup`, Python, Docker and the GitHub CLI, network access to crates.io, rustup and GitHub, and the repository's CI on the pull request. None needs Windows, a screen reader, a Braille display, an engine, audio, a local file, or the wiki.

### Task 1: fuzz targets for the parsers that have none

**Why.** Fifteen loaders are fuzzed nightly; the parsers a student's own writing goes through are not: the LaTeX and ASCIIMath math parsers, the BibTeX, RIS and CSL-JSON citation importers, the theme file reader, the lexicon data file, the vault importer, and the JSON-RPC request decoder. The roadmap rates this high ("a hostile file must never take the reader down") and lists it as three quick wins for W5m. It is pure logic with tests and CI as the judge, and it is the same pattern fifteen times over already.

**Value.** Removes a way for the reader to die on a hostile citation file, theme, vault note, or RPC request. Starts the "nightly fuzz clean for thirty days" clock (the roadmap's stability criterion) earlier for these parsers.

**Owns.**

- `fuzz/Cargo.toml` (new `[[bin]]` entries appended at the end, new dependencies), `fuzz/fuzz_targets/{latex_math,asciimath,bibtex,ris,csl_json,theme,lexicon,vault_import,rpc}.rs` (new), `fuzz/src/lib.rs` (shared checks), `fuzz/README.md` (the target list).
- `xtask/src/fuzz_seed.rs` (new) and one `mod` line plus one match arm in `xtask/src/main.rs`: `cargo xtask fuzz-seed [--target NAME]` copies the fixtures into each target's corpus folder.
- `.github/workflows/nightly.yml`: the nine names appended to the fuzz matrix, and the "Seed the corpus" step replaced by `cargo xtask fuzz-seed --target ${{ matrix.target }}`.
- Hostile-input caps, only where a target finds a crash or an unbounded walk, in the parser module that feeds the target: `crates/textweaver-math/src/` (parser depth and token count), `crates/textweaver-cite/src/{bibtex,ris,csljson}.rs` (input size and entry count), `crates/textweaver-theme/src/file.rs` (file size and key count), `crates/textweaver-lexicon/src/data.rs` (block and entry caps; a corrupt file is refused, never read out of bounds), `crates/textweaver-vault/src/import.rs` (file count and size), `crates/textweaver-app/src/rpc.rs` (request size and depth). Each cap is one small commit with a regression test made from the crashing input.
- `fixtures/cloud/` for any new seed file.

**Not to touch.** Any other file in those crates; `textweaver-formats`; `textweaver-xilem`; messages and settings; `docs/` beyond `fuzz/README.md`; `CHANGELOG.md` and `docs/history/tasks.md` (the orchestrator writes their lines at merge).

**Conflict check.** Wave 4: W4c1 (4a) makes one call in `textweaver-math` (the tree to MathML) and W4g (4b) adds `to_unicode` there; the caps live in the parser functions, different lines. W4g reads the citation crate's record types and does not edit the importers. W4d (4c) owns `textweaver-lexicon/src/i18n/`, not `data.rs`, and rewrites messages across the app, including any in `rpc.rs`; the cap is a few lines at the decode step, resolved by hand if both land. W4c2 (4c) adds three targets to `fuzz/Cargo.toml` and `fuzz/README.md`, and W4f (4c) adds their matrix lines: appends on both sides, so the merge is mechanical, and better still if this pull request merges before 4c starts. Wave 5: this is W5m's deliverables 1 to 3 in sub-wave 5a; section 5 reserves them so W5m does not repeat them. W5c3 (5b) appends two more targets the same way.

**Acceptance criteria.**

- `cargo +nightly fuzz build` builds every target, old and new, in the fuzz workspace.
- Each new target has run at least five minutes in the cloud session without a crash, or its crash is fixed with a cap and a regression test in the crate; the report says which.
- Every new target checks the loader contract the existing ones check: no panic, and every span or range a parser returns lies inside the source and in order.
- `cargo xtask fuzz-seed` fills every target's corpus from `fixtures/` and is what `nightly.yml` calls.
- CI on the pull request is green: `ci.yml` (fmt, docs, generated, deny, the three-system check, the real-engine jobs), `bench.yml`, `scripts.yml`. The nightly workflow's fuzz matrix runs green on `main` after the merge (the orchestrator dispatches it once by hand).
- If the `rpc` target cannot be built without system libraries in the fuzz workspace (the app crate pulls the engines), it is dropped and the report says why; the other eight are the deliverable.

### Task 2: the generated settings reference and the docs consistency check

**Why.** The roadmap calls `docs/settings.md` "the doc that drifts most" and gives W5p two quick wins: a settings reference generated from `SettingsSchema` with a check in CI, and a check that `docs/README.md`, the ADR index and the crate count match the tree. Both are code plus docs, verified by CI, with no screen or engine. The inventory already lists the drift the check will find (ADR-0023 missing from the Decisions list, ADR-0027 out of order, "29 crates" against 33 folders, `docs/roadmap.md`'s stale status).

**Value.** A guide that lies costs a blind user an hour. After this task, every new setting from Wave 4 and Wave 5 documents itself with one command, and CI refuses a stale index. It also makes the settings reference a screen-reader-friendly list: key, default, then help, meaning first, no tables.

**Owns.**

- `docs/settings-reference.md` (new, generated): every setting section by section as a list, each item "key: default; label. Help." with the range or the choices in words. Generated only; never edited by hand.
- `crates/textweaver-app/tests/settings_reference.rs` (new): renders the schema to Markdown and compares it with the file; with `TEXTWEAVER_UPDATE_DOCS=1` it rewrites the file. This keeps the generator next to the schema and adds no dependency to `xtask`.
- `xtask/src/docs_check.rs` (new) and its `mod` line and match arms in `xtask/src/main.rs`: `cargo xtask settings-doc [--check]` (runs that test, with the variable when not checking) and `cargo xtask docs --check` (the ADR index against `docs/adr/`, `docs/README.md`'s Decisions list against the index and in order, the crate count in `docs/README.md` and `docs/dev/architecture.md` against `crates/`, every guide in `docs/*.md` ending with a "See also" heading, every `docs/*.md` linked from `docs/README.md`).
- `.github/workflows/ci.yml`: two steps in the `docs` job that run the two checks.
- The fixes the check finds in `docs/README.md` (the Decisions list, the crate count, the Roadmap line), `docs/adr/README.md` (index lines only), `docs/roadmap.md` (the status block replaced by one dated paragraph that points at `docs/history/tasks.md` and `docs/research/`), and `docs/dev/architecture.md` (the crate count only).
- `docs/dev/testing.md`: two lines under "The checks" naming the new checks.

**Not to touch.** `docs/settings.md` itself (W4d owns it in 4c and W4f's doc pass covers it; the orchestrator adds one "See also" line to the reference after the merge), `CHANGELOG.md`, `docs/history/tasks.md`, `docs/screen-readers.md`, `docs/library.md`, `docs/star-gaps.md`, `README.md`, any crate beyond the one test file, `settings_schema.rs` itself (a schema gap is reported, not fixed).

**Conflict check.** Wave 4: W4f (4c) owns `.github/` and does a doc pass over a listed set of guides; `docs/README.md`, `docs/adr/README.md`, `docs/roadmap.md` and `docs/dev/architecture.md` are not on its list, and the `ci.yml` change is two added steps. W4c1, W4s, W4g and W4d add settings; after this merges, each must run `cargo xtask settings-doc` once or the check fails on their branch. That is the only new demand on Wave 4, and the orchestrator adds it to the common rules. Timing it during Wave 4's pause avoids even that. Wave 5: this is W5p's deliverables 2 and 3 and quick wins 1, 2, 4 and 5 in sub-wave 5c; section 5 reserves them. W5x (5a) edits `docs/screen-readers.md` and `docs/library.md`, which this task never touches.

**Acceptance criteria.**

- `cargo xtask settings-doc --check` and `cargo xtask docs --check` pass on the branch and run in the `docs` job of `ci.yml`.
- Removing one setting's `INFO` entry, or one ADR from the index, makes the matching check fail with a message that names the key or the file.
- `docs/settings-reference.md` lists every key `SettingsSchema::generate()` returns, including the internal ones under their own heading, as lists with the meaning first and no tables; `python tools/check_links.py` and `python tools/check_site_a11y.py` pass.
- The stale passages the check found are fixed, listed one per line in the pull request.
- CI on the pull request is green.

### Task 3 (optional): the EPUB and PDF writers checked by a second tool in CI

**Why.** ADR-0017 makes accessibility claims for the EPUB 3 and tagged PDF writers that only textweaver's own checks test. The inventory ranks "tagged PDF and EPUB checked by a second tool" tenth among the unplanned items and the roadmap puts it in Wave 6 (W6t) and Wave 7. It is a workflow and a few fixtures, verified only by CI, and it needs nothing local. It is optional because its value is medium and the budget's margin is where it belongs.

**Value.** A PDF/UA or EPUB accessibility claim checked by epubcheck and veraPDF, the tools libraries and publishers use, so a student's exported paper is not rejected by a checker textweaver never ran.

**Owns.**

- `.github/workflows/second-tool.yml` (new): on `workflow_dispatch`, on a weekly schedule, and on `pull_request` when this file or `crates/textweaver-writers/**` changes. One job builds `tw` (the `publish` feature on), converts the writer fixtures and `fixtures/sample.md` to EPUB and PDF, runs epubcheck and veraPDF from their GitHub releases (version pinned, SHA-256 checked, no third-party action), uploads the reports as artifacts, and fails on any error. Known warnings go in an allowlist file with one reason each.
- `tools/second_tool_allowlist.txt` (new) and `docs/dev/testing.md` (one paragraph).
- Nothing in any crate. A writer defect the tools find is an issue in the pull request description, not a fix.

**Not to touch.** `ci.yml`, `nightly.yml`, `release.yml`, `gui-xilem.yml`; any crate.

**Conflict check.** Wave 4: W4f (4c) owns every workflow, but this is a new file; run it after Wave 4's pause anyway. Wave 5: W5t (5c) owns `gui-xilem.yml` and a new `a11y-tests.yml`; W5p (5c) owns the three main workflows; W5c4 (5b) changes the BRF path of the writers, not EPUB or PDF. No shared lines. Wave 6: W6t's list shrinks by these two tools; liblouis stays with W6t.

**Acceptance criteria.**

- The workflow runs green on the pull request (its `pull_request` trigger matches the new file), and by hand on `main` after the merge.
- Every epubcheck error and every veraPDF failure is either zero or in the allowlist with a reason and an issue link.
- The downloads are pinned and checked; the workflow needs no secret.

## 2. The budget

### Prices, checked today

From Anthropic's pricing page (`https://platform.claude.com/docs/en/about-claude/pricing`), read on Sunday, September 27, 2026, per million tokens:

- Claude Opus 5 (`claude-opus-5`): input $5, 5-minute cache write $6.25, cache read $0.50, output $25.
- Claude Sonnet 5 (`claude-sonnet-5`): input $2, cache write $2.50, cache read $0.20, output $10.
- Claude Fable 5.1: input $10, output $50. Not recommended here: the tasks are well specified, and the price doubles the budget's risk.

The recommended model is Claude Opus 5, chosen in the session with `/model opus` (or the model picker), at Claude Code's default effort. Cloud sessions are billed as Claude usage on the owner's plan, with no separate charge for the virtual machine (the Claude Code cloud documentation says so). Whether the $125 is drawn from the plan's included usage or from extra usage billed at these rates depends on the owner's plan, and the plan was not read; the estimates below price every token at the rates above, which is the worst case.

### Assumptions

- One turn is one model call. A Claude Code session's context grows from about 20,000 to about 150,000 tokens, then compacts. Average per turn: about 70,000 tokens read from cache ($0.035), about 2,500 new input tokens written to cache ($0.016), and about 1,200 output tokens including thinking ($0.030). About $0.08 per turn, rounded up to $0.09 to cover compaction and the occasional large file read.
- A CI fix cycle (read the failing log, change, push, wait) costs 20 to 40 turns. Waiting for CI costs nothing while the session is idle.
- The newer tokenizer produces about 30 percent more tokens for the same text than the previous one; the per-turn figures already assume it.

### Per task (low, expected, high)

- **Task 1, fuzz targets:** 200 turns ($18), 350 turns ($32), 650 turns ($58). Share: $55. Stop point: $44 (80 percent).
- **Task 2, docs checks:** 150 turns ($14), 250 turns ($22), 450 turns ($40). Share: $40. Stop point: $32.
- **Task 3, second tool:** 120 turns ($11), 200 turns ($18), 350 turns ($32). Share: $30. Stop point: $24.

### Total and margin

- Tasks 1 and 2: expected $54, high $98. Margin under $125 at the high estimate: $27 (22 percent).
- All three: expected $72, high $130. So task 3 starts only if tasks 1 and 2 have spent $80 or less, which leaves task 3 at least $45, one and a half times its share.
- On Claude Sonnet 5 instead, every figure is about 40 percent of the above (expected $29 for all three). Use it if the first task overruns its expected cost by half.

### Stop points

- The cloud agent commits and pushes after every numbered deliverable, so the draft pull request always holds finished work and the owner can stop the session at any point without losing anything.
- The cloud agent stops on its own, and marks the pull request ready for review, when its deliverables are done, or when its context has been compacted twice, or when the owner says the task's stop point is reached.
- The owner watches spending in the Usage section of the claude.ai settings after each pushed checkpoint, and sets the plan's extra-usage limit to $125 so nothing can exceed the budget. The exact names of these settings were not verified today; the principle is a hard limit set outside the session, plus a look after each checkpoint.

## 3. The pull-request workflow

- **Branches:** `cloud/fuzz-parsers`, `cloud/docs-checks`, `cloud/second-tool-checks`, each from `main`. One pull request per task, against `main`.
- **Draft first.** The cloud agent opens the pull request as a draft after its first commit, so CI runs on every push (`ci.yml`, `bench.yml` and `scripts.yml` run on `pull_request`; pushes to `cloud/**` alone run nothing). It marks the pull request ready for review only at a stop point, with every check green or every red check explained in the description.
- **The checks a pull request must pass:** `ci.yml` (fmt; docs: links, site data, site accessibility; generated: keyboard, deps, notices; deny; clippy, tests and rustdoc on Ubuntu with every feature, macOS and Windows with Omnivox; the real-engine jobs), `bench.yml` (peak heap and allocations within twice main's), `scripts.yml` when scripts change, and the task's own new check.
- **Review and merge by the orchestrator, locally.** The owner declined a merge gate, so nothing merges on GitHub. The orchestrator fetches the branch into its own worktree, reads the diff, runs the native checks (`scripts/dev-check.ps1`, `cargo xtask deps --check`, `cargo xtask notices --check` if `Cargo.lock` changed), runs the container all-features check one at a time as for any agent, runs `cargo +nightly fuzz build` in the container for task 1, merges to `main` locally, regenerates the site and the notices, pushes, and closes the pull request as merged. It adds the `CHANGELOG.md` line and the status line in `docs/history/tasks.md` itself.
- **The screen-reader checklist.** Task 1 and task 3: nothing to hear. Task 2: one item for the owner, on the Braille display or with NVDA: open `docs/settings-reference.md` and read three settings; each line should give the key, then the default, then the help, and no table should be met.
- **Avoiding and resolving conflicts with Wave 4.** The cloud agent rebases on `main` before marking ready and again if asked. Its edits to shared files are appends (`fuzz/Cargo.toml`, the nightly matrix, `xtask/src/main.rs`) or new files, so a conflict is a one-line merge. If a Wave 4 merge lands first and CI goes red on the cloud branch, the owner sends the session one message: "rebase on main and fix CI". If the cloud pull request lands first and a Wave 4 branch goes red because a setting has no reference line, that agent runs `cargo xtask settings-doc` once.

## 4. Effect on Wave 5 and later

The owner's rule: after Wave 4, work pauses, and Wave 5 and beyond are recalibrated for the Cloud Agent's contribution. This section lists what each task completes or shrinks, by document and by agent, and what to change in the briefs if the pull request merges.

### Task 1, fuzz targets

Completes or shrinks:

- `wave5-plan.md`: W5m's deliverables 1 (the nightly matrix and `cargo xtask fuzz-seed`), 2 (the nine targets with caps) and 3 (ten minutes of each; the nightly job does it); quick wins 6, 7 and 17; section 5.1's W5m ownership of `fuzz/` and the nightly matrix lines; section 5.2 hotspot 8.
- `roadmap-2026.md`: section 5.2 item 8 (the nightly fuzz lines; only `fuzz-seed` was left, and it lands here); section 5.4's three W5m quick wins.
- `whats-left.md`: "Fuzz targets for the math parsers, the citation importers, the theme reader, the lexicon file, the vault importer, the JSON-RPC decoder; `cargo xtask fuzz-seed`" (Quality, Wave 5, W5m) becomes done.

Briefs to change if it merges: W5m keeps only branch 2 (memory work in `textweaver-engines` and `opening.rs`, if W4s's table points there) and the ADR-0027 status update. If the GUI is already near its baseline, W5m is empty; sub-wave 5a then runs W5x and W5r with a free slot, which the orchestrator may give to W5s (summaries, light, no 5a dependency) or leave empty. W5c3 (5b) keeps its two targets and uses `fuzz-seed` and the pattern. W4c2 (4c) uses `fuzz-seed` for its three targets if the pull request lands first.

Newly unblocked: the thirty-day clean-fuzz clock for these parsers starts before Wave 5; W5c3's LaTeX loader can reuse the math parser's caps.

### Task 2, docs checks

Completes or shrinks:

- `wave5-plan.md`: W5p's deliverables 2 (`settings-doc --check`) and 3 (`docs --check`, with quick wins 1 and 2 fixed); quick wins 1, 2, 4 and 5; section 5.1's W5p ownership of `xtask/src/docs_check.rs`, `docs/README.md`, `docs/roadmap.md`, `docs/adr/README.md` (index lines), and `docs/settings.md`'s generated section (which becomes `docs/settings-reference.md`).
- `roadmap-2026.md`: section 5.2 item 5 in part (`docs/README.md`'s Decisions list, crate count and roadmap line; `docs/roadmap.md`'s status; the ADR index's lines); section 5.4's orchestrator step-0 item ("`docs/README.md`'s ADR list and crate count") and W5p's first two quick wins. Not covered: `docs/star-gaps.md`, `CHANGELOG.md`'s Unreleased section, `README.md`'s "Coming next", the GUI paragraph of `docs/screen-readers.md` (W5x's).
- `whats-left.md`: "`cargo xtask settings-doc --check` and `cargo xtask docs --check`" (Documentation, Wave 5, W5p) becomes done; "Stale passages found today" shrinks to the items named above as not covered; "The items no plan covers yet" item 4 shrinks the same way.

Briefs to change if it merges: W5p drops deliverables 2 and 3 and the four quick wins, and keeps the release work (the GUI packages, the alpha checklist, the doc pass over `docs/star-gaps.md`, `docs/dev/releasing.md` and `docs/dev/building.md`, the release notes). The common rules for Wave 5 gain one line: "a new setting also runs `cargo xtask settings-doc`". The Wave 4 common rules gain the same line the day it merges, for W4c1, W4s, W4g and W4d.

Newly unblocked: every later ADR and crate is caught by CI if the index or the count is stale, so the "two agents picked the same ADR number" lesson has a check behind it (the check can also refuse two files with one number).

### Task 3, second-tool checks

Completes or shrinks:

- `roadmap-2026.md`: Wave 6's W6t ("veraPDF or PAC on the tagged PDF, epubcheck on the EPUB") in part; Wave 7's "second-tool checks" line in part. liblouis grade 2 in CI stays with W6t.
- `whats-left.md`: "Tagged PDF checked with veraPDF or PAC; EPUB with epubcheck; grade 2 braille with liblouis in CI" (Accessibility checks, unplanned) shrinks to liblouis; "The items no plan covers yet" item 10 the same.
- `wave5-plan.md`: nothing; Wave 5 does not plan it.

Briefs to change if it merges: none in Wave 5. W6t's brief starts from the workflow and adds liblouis and the real-engine runners. Any writer defect the tools report becomes a Wave 5 or Wave 6 item for the writers' owner (W5c4 touches the BRF path only, so a Wave 6 agent).

Newly unblocked: the final alpha's readiness page can cite a second tool for the PDF/UA and EPUB claims.

### Timing against Wave 4

- **During Wave 4:** task 1, now. It is the one task whose Wave 5 home is sub-wave 5a, so landing it before Wave 4 ends removes the only early-sub-wave overlap. Its shared-file edits are appends, and merging it before sub-wave 4c starts (about the first days of October) keeps W4c2's and W4f's fuzz lines a plain append on top.
- **During Wave 4's pause:** task 2. Nothing local is building, so its `ci.yml` steps and its index fixes meet no other branch, and the generated reference covers every Wave 4 setting at once, so no Wave 4 agent has to regenerate it.
- **After the pause, beside Wave 5:** task 3, if the budget allows. It has no Wave 5 home and no shared line with any Wave 5 agent.

## 5. Avoiding duplicate work

The owner's rule: Wave 5 must proceed without waiting for the Cloud Agent, and must never duplicate its work. The mechanism, for the orchestrator to implement:

1. **A reservation list in the repository:** `docs/history/reservations.md`, a small append-only file (the tasks file is a merge hotspot, so the list gets its own file, with one pointer line under the Wave 5 heading in `docs/history/tasks.md`). One entry per cloud task: the task's name and branch; its items with their `whats-left.md` and `wave5-plan.md` references (section 4 above has them, ready to copy); the exact files and crates it owns (section 1 above); the pull request link once opened; a status line (reserved, open, merged, released) with the machine's date. The orchestrator creates it when the owner starts the first cloud session.
2. **Wave 5 briefs** carry the reserved items and files under a heading "not yours; reserved for the Cloud Agent", and the orchestrator removes those items from the brief's deliverables and its "owns" list. W5m loses the fuzz items; W5p loses the two checks and the four quick wins; W5c3 keeps its own two targets.
3. **Before launching each sub-wave,** the orchestrator runs `gh pr list --state all --search "head:cloud/"` and reads the list against `reservations.md`: a merged pull request's entry is marked merged and its items are struck from the plan for good; an open one stays reserved; one with no commit and no comment for seven days is marked abandoned and its items are released back to Wave 5 with the owner's OK, in which case the cloud session is stopped and its branch left for the owner to delete or reuse. The orchestrator writes the result as one line per entry under the sub-wave's heading in `docs/history/tasks.md`.
4. **The cloud brief** (section 6) tells the agent to touch only its reserved files, to rebase on `main` before marking the pull request ready, and to stop and say so in the pull request if the task turns out to need a file that is not reserved. The orchestrator then either extends the reservation or narrows the task.

Which Wave 5 sub-waves can run beside each cloud task with the least coordination:

- **Task 1** beside 5b and 5c, and beside 5a only because W5m's fuzz items are reserved out; the best case is that it merged during Wave 4 and nothing runs beside it. If it is still open in 5b, W5c3 appends its two targets and the later of the two rebases.
- **Task 2** beside 5a and 5b without any shared file. Beside 5c, W5p and W5t edit `.github/`, but different files and different jobs; W5p's doc pass must not touch the files task 2 owns, which the reservation says.
- **Task 3** beside every sub-wave; the only sharing is GitHub runner time with W5t's and W5p's workflow runs in 5c, which the orchestrator sequences.

## 6. The ready-to-paste Cloud Agent brief

Paste the common part, then one task section. It is self-contained and holds no personal identifier. The repository is public; the owner is called "the owner" throughout.

### Common part

You are a Claude Code cloud session working on the public repository `leavesofgrass/textweaver`, a pure-Rust, accessible, self-voicing reader and writer for Markdown and other documents, built for blind and print-disabled students. You contribute one task by pull request against `main`. The owner starts you and reviews the pull request; a local orchestrator merges it. You never merge, never push to `main`, never tag.

**Rules. Only the owner overrides them.** If a rule seems not to fit your task, stop, say so in the pull request, and wait.

- **Privacy.** The owner's personal identifiers (names, email addresses or parts of one, usernames, callsigns, machine or account names) never appear in any request, header, URL, query, commit, file or pull request text. For any web request, use the User-Agent `textweaver-research (+https://github.com/leavesofgrass/textweaver)` or the tool's default. Never build a User-Agent or contact string from an account. Never write an identifier into a file, even to describe a mistake.
- **Where you work.** Your task names the files and crates it owns. Touch only those. If you find the task needs another file, stop, say so in the pull request, and wait for the owner's answer. Read anything you like.
- **Deleting.** Delete nothing that exists on `main`. Delete only files your own task created and no longer needs, and only after listing them. Never delete a fixture, a corpus, a workflow or a generated file. Never chain a delete onto another command.
- **File names.** No escape sequences (`$'...'`, `\x..`, `\u....`, `printf`, URL encoding) in any command that creates, moves, copies, writes or deletes a file. Plain literal names and paths only. Resolve and print a full path before any command that removes or moves something.
- **Writing.** US English ("color", "behavior"). Plain short sentences, headings and lists. Em dashes sparingly. Tables only with real headers, and none in generated docs meant to be read on a Braille display.
- **Accessibility first.** The owner reads with a screen reader and a 40-cell Braille display. In every message, doc line, list item, workflow log and pull request: the meaning first on each line, words over symbols, no emoji, no arrows, no box drawing, no ASCII art. Color never carries meaning alone; write "Pass" and "Fail" as words.
- **Commits.** One logical change per commit. Subject line: the area in lower case, a colon, a short summary in plain words (`fuzz: theme file reader target`). The body says why. End every commit message with the attribution line your session gives you, of the form `Co-Authored-By: Claude <model name> <noreply@anthropic.com>`. Take dates from the machine, never from memory.
- **Code rules.** Read `CLAUDE.md`, `CONTRIBUTING.md`, `docs/dev/testing.md` and the shared preamble of `docs/history/tasks.md` first. Never edit `docs/history/tasks.md`, `CHANGELOG.md`, `THIRD-PARTY-NOTICES.md`, `docs/site/*` or any generated file by hand; regenerate what your task says to regenerate. New dependencies go in a block in `fuzz/Cargo.toml` or the root `Cargo.toml` headed with a comment naming your task. Keep `unsafe_code = "deny"`.

**Checks before every push:** `cargo fmt --all --check`; `cargo clippy --workspace --exclude textweaver-gui --exclude textweaver-xilem --all-targets -- -D warnings` (with `--all-features` if the system libraries `libespeak-ng-dev`, `espeak-ng-data`, `libasound2-dev`, `libspeechd-dev` and `pkg-config` install; otherwise without, and say so); the tests of the crates you changed with `--no-fail-fast`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --exclude textweaver-gui --exclude textweaver-xilem --no-deps`; `cargo xtask keyboard --check`; `cargo xtask deps --check`; `python3 tools/check_links.py`; `python3 tools/gen_site_data.py --check`; `python3 tools/check_site_a11y.py`; plus the task's own checks. A command that needs more than two minutes runs in the background with its output to a log file you read afterwards. CI on the pull request is the final judge; read every failing job's log and fix it.

**Budget and stop points.** Commit and push after every numbered deliverable, so the draft pull request always holds finished work. Stop on your own, and mark the pull request ready for review, when your deliverables are done, or when your context has been compacted twice, or when the owner says the stop point is reached. Do not start a deliverable you cannot finish in the current context; push what is done and say what is left.

**Pull request steps.**

1. Create the branch named in your task from `main`.
2. After your first commit, push and open a draft pull request against `main` with the title from your task, so CI runs on every push.
3. Before marking it ready: rebase on `main`, run the checks, push, and wait for CI to be green or explain every red job.
4. Mark it ready for review only at a stop point. Never merge it.

**Pull request description**, in this order, plain sentences and lists, no tables:

- Summary in five lines.
- Files changed, by crate or folder.
- The check results, one line each, and the CI run's link.
- What was not done and why; anything that needs a file outside your reservation.
- Contract change requests, or "none".
- "For the owner with a screen reader:" a checklist of at most five things to try, or the words "nothing to hear".

### Task section 1: fuzz targets for the parsers that have none

Branch `cloud/fuzz-parsers`. Title: "fuzz: targets for the math, citation, theme, lexicon, vault and RPC parsers".

You own: `fuzz/Cargo.toml` (append), `fuzz/src/lib.rs`, `fuzz/README.md`, new files under `fuzz/fuzz_targets/`, new `xtask/src/fuzz_seed.rs` with one `mod` line and one match arm in `xtask/src/main.rs`, the fuzz matrix and the "Seed the corpus" step of `.github/workflows/nightly.yml`, new files under `fixtures/cloud/`, and, only for a crash or an unbounded walk a target finds, a small cap with a regression test in the parser module that feeds the target: `crates/textweaver-math/src/` (parser depth and token count), `crates/textweaver-cite/src/{bibtex,ris,csljson}.rs`, `crates/textweaver-theme/src/file.rs`, `crates/textweaver-lexicon/src/data.rs`, `crates/textweaver-vault/src/import.rs`, `crates/textweaver-app/src/rpc.rs`. Nothing else in those crates; not `textweaver-formats`, not messages or settings, not `docs/`.

Read first: `fuzz/README.md` and the fifteen existing targets (the contract each checks); `docs/research/wave4.md`, the hostile-input section; `crates/textweaver-formats/tests/hostile.rs` (the caps pattern); the public entry points `textweaver_math::parse`, the `parse` functions of `bibtex.rs`, `ris.rs` and `csljson.rs` in `textweaver-cite` (through the public `formats::parse` if the modules are private), the `parse` function in `textweaver-theme`'s `file.rs`, `textweaver_lexicon` `Lexicon::open` in `data.rs` (write the bytes to a temporary file), `textweaver_vault::import::{parse_note,read_vault}`, and `textweaver_app::rpc` (`Server::handle`, and the request type it decodes).

Deliverables, in order, one commit each:

1. `cargo xtask fuzz-seed [--target NAME]`: creates each target's corpus folder and copies the matching fixtures (the mapping the nightly workflow's `case` block has today, plus the new targets); `nightly.yml`'s "Seed the corpus" step calls it. Add nothing else to the workflow yet.
2. `latex_math` and `asciimath`: `textweaver_math::parse` on the bytes as text; no panic; every span in the returned tree lies inside the source and in order; MathML and speech generation on the result do not panic.
3. `bibtex`, `ris`, `csl_json`: the three importers; no panic; a parsed reference renders to text without panic.
4. `theme`: the theme file reader; no panic; a parsed theme passes the crate's own contrast check or is refused, never both.
5. `lexicon`: the data file reader on a temporary file made from the bytes; a corrupt file is refused with an error, never read out of bounds; a valid prefix of the real lexicon format is a good seed if a small one can be built in a test.
6. `vault_import`: `parse_note` on the bytes as text, and `read_vault` on a temporary folder holding the bytes as one note; every range lies inside the text.
7. `rpc`: the JSON-RPC request decoder; a request that decodes must encode to one that decodes the same; the server's `handle` on the request must not panic. Depend on `textweaver-app` with `default-features = false`; if the fuzz workspace cannot build it on Ubuntu without system libraries, drop this target and say so in the pull request.
8. For each new target, five minutes of `cargo +nightly fuzz run NAME -- -max_total_time=300 -rss_limit_mb=4096 -timeout=30` in the background with the log kept; a crash becomes a cap plus a regression test in the crate, then the run is repeated.
9. The nine names appended to the matrix in `nightly.yml`; `fuzz/README.md` lists each target in the style of the existing entries.

Checks of your own: `rustup toolchain install nightly --profile minimal` and `cargo install cargo-fuzz` (the cloud machine may install them; the repository's `rust-toolchain.toml` stays as it is); `cargo +nightly fuzz build` from the repository root; `cargo +nightly fuzz list` shows twenty-four targets; the changed crates' tests; the common checks.

For the owner with a screen reader: nothing to hear.

### Task section 2: the generated settings reference and the docs consistency check

Branch `cloud/docs-checks`. Title: "docs: generated settings reference and a docs consistency check in CI".

You own: new `docs/settings-reference.md` (generated only); new `crates/textweaver-app/tests/settings_reference.rs`; new `xtask/src/docs_check.rs` with its `mod` line and match arms in `xtask/src/main.rs`; two steps in the `docs` job of `.github/workflows/ci.yml`; the fixes the check finds in `docs/README.md`, `docs/adr/README.md` (index lines only), `docs/roadmap.md` (the status block only) and `docs/dev/architecture.md` (the crate count only); two lines in `docs/dev/testing.md`. Not `docs/settings.md`, not `settings_schema.rs`, not `README.md`, not `CHANGELOG.md`, not any other doc.

Read first: `crates/textweaver-app/src/settings_schema.rs` (`SettingsSchema::generate`, `Setting`, `SettingKind`, `Info`, the `INFO` table, and the tests at the end); `docs/settings.md` (the style and the section order); `docs/adr/README.md` and `docs/README.md`; `tools/check_links.py` (what "See also" and links mean here); `xtask/src/keyboard.rs` (a generated doc with a `--check`, the pattern to follow).

Deliverables, in order, one commit each:

1. The renderer, in the test file: for each section in the schema's order, a level-2 heading with the section's TOML name and title, then one list item per setting: the key in backticks, the default, then the label, then the help, then the range with its unit or the choices in words. Internal settings under their own heading at the end. No tables. A test compares the rendering with `docs/settings-reference.md` and fails with the first differing line; with `TEXTWEAVER_UPDATE_DOCS=1` it writes the file instead. Generate the file and commit it.
2. `cargo xtask settings-doc [--check]`: runs that test with or without the variable and reports in one line. It adds no dependency to `xtask`.
3. `cargo xtask docs --check`: the ADR index (`docs/adr/README.md`) lists every file in `docs/adr/` once, in number order, and no number twice; `docs/README.md`'s Decisions list matches the index and is in order; the crate count named in `docs/README.md` and `docs/dev/architecture.md` equals the number of folders in `crates/`; every `docs/*.md` ends with a "See also" heading; every `docs/*.md` is linked from `docs/README.md`. Each failure is one line naming the file and the fact.
4. Fix what the check found, one commit per file, and list every passage changed in the pull request. In `docs/roadmap.md`, replace the status block with one dated paragraph (date from the machine) that says the current plans live in `docs/history/tasks.md` and `docs/research/`.
5. The two steps in `ci.yml`'s `docs` job, and the two lines in `docs/dev/testing.md`.

Checks of your own: the two new checks; removing one `INFO` entry in a scratch build makes `settings-doc --check` fail with the key's name; the common checks, especially `python3 tools/check_links.py` and `python3 tools/check_site_a11y.py`.

For the owner with a screen reader: open `docs/settings-reference.md` and read three settings; each line should give the key, then the default, then the help, and there should be no table.

### Task section 3: the EPUB and PDF writers checked by a second tool in CI

Branch `cloud/second-tool-checks`. Title: "ci: epubcheck and veraPDF on the writers' output, weekly and on writer changes".

You own: new `.github/workflows/second-tool.yml`; new `tools/second_tool_allowlist.txt`; one paragraph in `docs/dev/testing.md`. No crate, no other workflow.

Read first: `docs/adr/0017-writers.md` (the claims and the checks that exist); `crates/textweaver-writers/tests/` and `fixtures/m/`; `.github/workflows/ci.yml`'s `docs` and `check` jobs (the style, the pinned actions, `permissions: contents: read`); `docs/converting.md` (the `tw convert` command lines).

Deliverables, in order, one commit each:

1. The workflow: triggers `workflow_dispatch`, a weekly schedule, and `pull_request` on paths `.github/workflows/second-tool.yml` and `crates/textweaver-writers/**`; one Ubuntu job that builds `tw` with the `publish` feature, converts `fixtures/sample.md` and the writers' fixtures to EPUB and to PDF, and uploads the outputs as an artifact.
2. epubcheck from its GitHub release, version pinned and SHA-256 checked in the workflow, run on every EPUB; the reports uploaded; errors fail the job unless the allowlist names them with a reason.
3. veraPDF from its GitHub release the same way, with the PDF/UA profile, on every PDF; the same rule.
4. The allowlist file (one line per known warning: tool, rule id, fixture, reason) and the paragraph in `docs/dev/testing.md`.

Checks of your own: `actionlint` if it installs; the workflow green on the pull request; every download pinned; no secret used. A writer defect the tools find is an issue described in the pull request, not a fix.

For the owner with a screen reader: nothing to hear.

## 7. Risks, and what the owner should decide before starting

Risks:

- **Billing basis unverified.** Cloud sessions draw on the plan's Claude usage; whether $125 means extra usage at the API rates above or a share of included usage depends on the plan. The estimates price every token, so they are the worst case, but the owner should set a hard limit outside the session.
- **Cost variance.** Fix cycles on CI are the main driver; a flaky real-engine job on a hosted runner can cost a cycle that has nothing to do with the task. The brief tells the agent to explain a red job rather than chase it past two attempts.
- **The `rpc` target may not build** in the fuzz workspace without system libraries; the brief allows dropping it.
- **Shared-file appends** (`fuzz/Cargo.toml`, the nightly matrix, `xtask/src/main.rs`, `ci.yml`) can conflict with W4c2 and W4f in sub-wave 4c if task 1 is still open then. Merging task 1 before 4c, or running it now, removes the risk.
- **A red Wave 4 branch after task 2 merges,** because a new setting has no reference line. Running task 2 in Wave 4's pause removes it; otherwise one command fixes it.
- **The generated reference duplicates `docs/settings.md`'s "Every setting" section.** That is intended: W4f's doc pass and W5p decide whether the hand-written section shrinks to prose and points at the reference.
- **Abandonment.** A cloud session the owner forgets holds a reservation; the seven-day rule in section 5 releases it.

What the owner should decide before starting:

1. The model: Claude Opus 5 as recommended, or Claude Sonnet 5 at about 40 percent of the cost.
2. The hard spending limit in the account's settings, at or under $125, and whether task 3 runs at all.
3. When task 1 starts: now, during sub-wave 4a, is the recommendation.
4. Whether task 2 waits for Wave 4's pause (recommended) or runs during sub-wave 4b.
5. Whether the orchestrator may create `docs/history/reservations.md` and the pointer line in `docs/history/tasks.md` now.

## See also

- [The 2026 roadmap](roadmap-2026.md): the milestones and the owner's answers.
- [Wave 5 plan](wave5-plan.md): the briefs the reservations shrink.
- [What is left](whats-left.md): the inventory the tasks come from.
- [Wave 4 orchestration plan](wave4-orchestration.md): what is running now and who owns what.
- [Tasks and agent briefs](../history/tasks.md): where the reservations pointer and the status lines go.
- [Testing](../dev/testing.md): the checks a pull request must pass.
- [Documentation index](../README.md)
