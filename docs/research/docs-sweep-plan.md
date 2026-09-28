# Documentation sweep plan

Written on Monday, September 28, 2026, for the orchestrator and the owner, after `0.1.0-alpha.4`. It answers the owner's request: docs comprehensive and correct against the code, a short README with the quick start first and the roadmap gone, and a GitHub Pages site. It changes nothing by itself: the owner adopts it, or parts of it, and the orchestrator applies it as agent briefs.

What it is built on: `CLAUDE.md`, `docs/README.md`, `README.md`, `docs/quickstart.md`, `docs/history/tasks.md` ("Pace after Wave 4" and "After Wave 4"), every file under `docs/`, `docs/research/wave4-orchestration.md` section 6 (the common-rules and brief format this plan reuses), web research done today on GitHub Pages site generators (dated sources, no personal identifier sent), and a read-only look at the owner's wiki and at `D:\star` for how Star published its docs.

## The short version

- **Recommendation: mdBook**, the Rust-native docs tool. Smallest supply chain (one static binary, no Python or Ruby runtime), a plain low-JavaScript theme that is less likely to trap screen reader focus than Material's search widget, and it matches the project's own pure-Rust direction. Fallback: plain MkDocs (no Material theme).
- **Material for MkDocs is not a safe long-term choice today:** its maintainer announced end-of-life on November 5, 2026, with only security fixes after that; new work has moved to Zensical, which is still pre-beta (0.0.x). The owner's MkDocs experience was with another of their projects, abax, not Star (Star's docs are plain Markdown on GitHub), so plain MkDocs is a known, working fallback.
- **The README drops to about 65 lines:** what textweaver is, then the quick start, then download and license. The roadmap link, the deep `tw` command list, Scripts, Repository layout, and Third-party data move to CONTRIBUTING.md and `docs/dev/building.md`.
- **The docs are already close to accurate.** Wave 4's own doc pass (W4f) and the doc agents before it kept most guides current; `docs/gui.md` already reflects edit mode and the file chooser. The real gaps are: the GitHub Pages site itself (does not exist yet), `docs/screen-readers.md`'s GUI section (still says "no edit mode yet"), and a handful of stale historical markers in `docs/roadmap.md` and `docs/README.md` that are already flagged in `docs/research/whats-left.md`.
- **Five agents:** the Pages site and its build, README and quick start, user guides, developer guides and history, and a final reviewer who reads every page against the running program.

## Site tool recommendation and comparison

| Option | Current version (Sept. 2026) | Accessibility of the built site | Existing Markdown and `docs/site/` | GitHub Actions | Supply chain |
|---|---|---|---|---|---|
| mdBook | 0.5.4 stable | Plain HTML/CSS themes, small `book.js` (sidebar and theme toggle only); no combobox search widget with known focus-trap history. Default theme's landmark and skip-link quality is not independently audited; needs a manual NVDA and JAWS pass. | Most relative Markdown links carry over; needs a hand-written `SUMMARY.md` (no auto-discovery of folders); `docs/site/`'s standalone HTML pages copy in as static files under `theme/` or `src/` and link out normally. | No official action; well-established third-party ones (`peaceiris/actions-mdbook` plus `actions/deploy-pages`). | Single static Rust binary. No Python or Ruby runtime. Smallest footprint of the five, and matches the project's Rust-first choices elsewhere. |
| Plain MkDocs (no Material) | Core 1.6.1 stable; a 2.0 pre-release appeared August 30, 2026 | Simpler default themes (`mkdocs`, `readthedocs`) than Material; fewer moving parts, so a smaller failure surface, but not flawless (a past breadcrumb used an invalid `alt` instead of `aria-label`, since fixed). | Excellent: it is the same generator Star's docs were written for, even though Star never deployed it. `docs/site/` drops in as static files or `extra_css`/raw links. | Documented official pattern: `pip install mkdocs`, `mkdocs gh-deploy`, or `actions/deploy-pages`. | Python runtime; much smaller dependency tree than Material (no bundled JS/search assets). Pin with a `requirements.txt`. |
| Material for MkDocs | Actively releasing, but end-of-life announced for November 5, 2026; security fixes only after that | The search dialog has documented, still-open problems: missing ARIA dialog semantics, a keyboard focus trap reported on Safari, and ambiguous heading permalinks for screen readers. Admonitions and diffs use color-coded borders plus icons, not text labels, and would need an audit for red-green safety. | Excellent, same generator as plain MkDocs. | Same official pattern as plain MkDocs. | Python runtime, larger dependency tree (Pygments, Jinja2, `mkdocs-material-extensions`, bundled JS). |
| Zensical | Pre-beta, 0.0.x; 0.1.0 planned for November 5, 2026 | Built by the same team as Material and inherits its UI design, including the search widget pattern, so the same concerns likely apply; too new for any independent accessibility record. | Reads `mkdocs.yml` natively; claims most Material content carries over with little change (unverified in practice). | Not yet established; the project itself is pre-beta. | Rust core (build speed), MIT, but immature: no track record for pinning or supply-chain review yet. |
| GitHub Pages + Jekyll | GitHub's own build, Ruby/gem-based | Default themes (`minima` and similar) are simple, low-JS static HTML; skip-link and explicit landmark defaults could not be confirmed from documentation alone and would need direct inspection. | Jekyll processes `docs/` with Liquid templating; existing files that happen to contain `{{ }}` or front-matter-like text can misbehave. `docs/site/`'s standalone pages need explicit exclusion from Liquid processing. | Fully native: GitHub Pages can build Jekyll with no custom workflow, or `actions/jekyll-build-pages`. | Ruby/gem-based; GitHub Pages pins a specific, narrower gemset than an open Ruby install. |

**Recommendation: mdBook**, weighted on accessibility risk and supply-chain size, both of which the owner's rules put first. Its main cost is that navigation must be hand-authored in `SUMMARY.md` rather than discovered from `docs/README.md`'s groupings, and its theme's accessibility is not independently documented, so the reviewer agent's manual NVDA and JAWS pass on the built site is not optional.

**Fallback: plain MkDocs**, no Material theme. Keeps the workflow the owner already used for abax, auto-discovers nav from the folder tree, and avoids adopting a theme entering end-of-life in six weeks. Switch to this if the site agent's manual accessibility pass finds mdBook's theme wanting and there is no time to patch it before the owner wants to publish.

**Do not use** Material for MkDocs (end-of-life within weeks) or Zensical (pre-beta, no accessibility record) for the published site today. Jekyll is workable but was the weakest fit: unverified theme accessibility and a narrower, GitHub-controlled toolchain pin.

## The README outline

Target: about 65 lines, matching `docs/quickstart.md`'s plain, listener-first style. What textweaver is, then how to start it, in that order; the roadmap and the deep feature list move out.

1. **Title and one paragraph:** what textweaver is, the "Status: alpha" line, kept as today.
2. **Start here:** the quick start link, the documentation index link, the interactive pages link. Drop the roadmap link (the owner's instruction: the roadmap leaves the README).
3. **Download:** unchanged, short.
4. **Quick start, inline:** the shortest version of the three-platform steps from `docs/quickstart.md` (one command per platform), ending with a link to the full guide for the rest. This is the one substantive addition: today a reader has to leave the README to get moving.
5. **Building:** kept short (`cargo build --workspace`, `cargo test --workspace`, the Docker one-liner), with a link to `docs/dev/building.md` for the rest.
6. **License and See also.**

**Moves out, not deleted:**
- The full `tw` command list and the two-programs description move to `docs/README.md`'s existing "What it does" style, or stay in `docs/quickstart.md` and `docs/reading.md`/`docs/converting.md`, which already cover each command in depth. A short "What it does" paragraph (three or four sentences) stays in the README so a first-time visitor still knows what they are looking at.
- **Scripts** moves to `docs/dev/building.md` or `scripts/README.md` (already exists and is linked).
- **Repository layout** moves to `docs/dev/architecture.md`, which already describes the crates.
- **Third-party data** moves to a new short section in `docs/dev/building.md` or stays as its own file, `docs/dev/third-party-data.md`, linked from both README and the license section; the owner's decision (see the questions below).
- The roadmap link is dropped from README entirely; `docs/README.md` already links `docs/roadmap.md`, which stays as the historical record it already says it is.

## The docs inventory

Grouped as `docs/README.md` already groups them. "Status" is this review's finding, not a task list; each agent below turns "stale" and "missing" rows into commits.

### For users

| Doc | Audience | Status |
|---|---|---|
| `quickstart.md` | New users | Accurate; feeds the new README quick start. |
| `install.md` | New users | Accurate; check the Linux AppImage section against the aarch64 packaging follow-up noted in `docs/history/tasks.md` ("Follow-ups found in Wave 4"), which says the aarch64 AppImage was not yet built as of Monday. |
| `screen-readers.md` | Screen reader users | **Stale.** Its GUI section (line 227) still says "it has no edit mode yet"; W4a3 shipped edit mode. Needs a Braille display section too (the file's own "checklist for trying each mode" should include the Mantis Q40, which nothing in the repo documents yet). |
| `troubleshooting.md` | All users | Spot-check against the current log flags (`--log`, `--log-file`) and `textweaver.log` location; not flagged stale, but not verified line by line this pass. |
| `reading.md` | Readers | Not verified this pass against the NVDA/JAWS-style default keys from the "Keys: what changed" section of `CHANGELOG.md`; check every key example. |
| `keyboard.md` | Readers, reference | **Generated** by `cargo xtask keyboard`; never hand-edit. Run the generator, don't audit by eye. |
| `gui.md` | GUI users | Accurate and current as of W4a3 (edit mode, file chooser, font and size keys, no-console fix are all described). Light copyedit only. |
| `notes.md` | Readers, writers | Not verified against the notes export formats (BibTeX, BibLaTeX, RIS, CSL-JSON) W4g added (`tw marks --export`); likely missing. |
| `reading-aids.md` | Readers | Check against the GUI's reading aids section in `gui.md`, which was written after this file; make sure the two agree. |
| `themes.md` | Readers | Not verified; check the 23-theme count and the GUI's theme cycle (F5) match. |
| `math.md` | Readers, writers | **Missing:** `[reading] math_engine = "builtin" | "mathcat"` (W4c1) and `[reading] math_display` Unicode math in the plain view (W4g) are settings changes with no obvious doc home yet; likely belong here. |
| `editing.md` | Writers | **Missing:** Markdown lint (`Ctrl+F8`, `tw lint`, W4g's own rules, not rumdl) and the code-block language announcement (syntect/two-face held for the owner per W4g's status line, so check whether it shipped before documenting it as available). |
| `citations.md` | Writers | Spot-check against the GUI's citation picker (Alt+C, Alt+Shift+D) in `gui.md`. |
| `dictation.md` | Writers | Not verified this pass. |
| `speech.md` | All users | **Missing:** the six interface languages and their voice-following behavior (documented today in `settings.md` under `[interface] language`, not here); cross-link at least. |
| `eloquence.md`, `dectalk.md` | All users | Not verified this pass. |
| `audio-export.md` | Writers | Not verified this pass. |
| `converting.md` | All users | **Likely missing:** RTF and ODT (W4c2, native readers, "Pandoc no longer needed for RTF") and DOCX comments/tracked changes (`[reading] revisions`) are recent; the file list needs updating per W4c2's own deliverable 5, which may already be done, check the branch's merge status. |
| `library.md`, `vault.md` | All users | Not verified this pass. |
| `settings.md` | All users | Large (431 lines) and appears current: languages, RTL, and GUI announcement settings are already documented. Spot-check `[gui] announce` and `[reading_aids.font]` (both added in Wave 4) are present. |
| `settings-reference.md` | Reference | **Generated** by `cargo xtask settings-doc`; never hand-edit, just regenerate and run `--check`. |

### For contributors

| Doc | Audience | Status |
|---|---|---|
| `dev/building.md` | Contributors | Gains the Scripts, Repository layout, and Third-party data content moved from the README. |
| `dev/testing.md` | Contributors | Not verified this pass; check the benchmark gate description against `bench.yml`. |
| `dev/architecture.md` | Contributors | Crate count (33) matches `crates/` today; keep the generator or count check in the reviewer's pass since this drifts every wave. |
| `dev/docker.md` | Contributors | Not verified this pass. |
| `dev/releasing.md` | Contributors | Check the listening-checklist reference (`cargo xtask release … --listened`) matches `CHANGELOG.md`'s current Unreleased section. |
| `json-rpc.md` | Integrators | Not verified this pass. |
| `roadmap.md` | Contributors | Explicitly marked in its own text as a historical record of Phases 1–2 and Wave 3; leave it that way, just confirm the "kept as a record" framing still reads correctly once newer waves are long done. Not linked from the trimmed README. |
| `star-gaps.md` | Contributors | Cross-reference against `docs/research/whats-left.md`'s "The Star features textweaver still lacks" section, which is newer and may have superseded some rows. |
| `adr/README.md` and the 34 ADRs | Contributors | `cargo xtask docs --check` is the tool for index drift (order, missing entries, status text); run it rather than hand-auditing 34 files. |
| `history/tasks.md`, `plan.md`, `reservations.md`, `star-parity.md` | Contributors, history | Working documents, not polish targets for this sweep; leave as is except fixing any link breakage the link checker finds. |
| `history/parity-report.md` | Contributors | **Generated** by `cargo xtask parity`. |
| `research/*.md` (9 files) | Contributors, history | Point-in-time planning documents, including this one; not part of the accuracy sweep, only the link checker. |

### The GitHub Pages site itself

There is no `docs/site/` mkdocs.yml, book.toml, or Pages workflow in the repository today; only the five standalone interactive HTML pages exist. This is new work, not a correction, and it is the SITE agent's whole brief below.

## The parallel split

Five agents. Paste the common rules block below at the top of every brief, adapted from `docs/research/wave4-orchestration.md` section 6.

### Common rules (paste at the top of every brief)

- **Only the owner overrides rules.** If a rule seems not to fit, stop and say so in the report.
- **Privacy.** No personal identifier in any request, header, URL, commit, or file. Neutral User-Agent only. Call the owner "the owner" in every file and commit message.
- **Where you work.** `D:\textweaver` only, your own worktree and branch `docs-sweep/<agent>-<topic>`. Never write to drive C.
- **Deleting.** Delete nothing but your own scratch output, listed first, with PowerShell `Remove-Item -LiteralPath`. Never from Bash without the owner's approval.
- **Generated files are never hand-edited:** `docs/keyboard.md`, `docs/settings-reference.md`, `docs/history/parity-report.md`, and anything under `docs/site/*.html`'s embedded data. Regenerate and check instead.
- **US English, em dashes sparingly, no ASCII art, color never carries meaning alone.**
- **Checks before reporting:** `py -3 tools/check_links.py`; `cargo xtask docs --check`; `cargo xtask settings-doc --check` (if settings text changed); `py -3 tools/gen_site_data.py --check` (if `docs/site/` content changed); the site build (`mdbook build`, once the SITE agent's tooling exists; before that, skip and say so).
- **Report** (plain sentences, headings and lists, no tables): summary in five lines; files changed; check results; what the next agent should do first; a checklist of at most five things for the owner to try with NVDA, JAWS, or the Braille display, or "nothing to hear."

### Agent SITE: the Pages site and its workflow

**Branch** `docs-sweep/site-mdbook`.

**Owns:** a new `book.toml` at the repository root or under `docs/`; a new `docs/SUMMARY.md` (mdBook's nav file, hand-authored to match `docs/README.md`'s groupings); a new `.github/workflows/pages.yml`; a new `docs/theme/` folder for any color-blind-safe, skip-link, and landmark fixes the default theme needs after the manual accessibility pass; the mechanism that serves `docs/site/`'s five standalone HTML pages as static files inside the built site, reachable from the nav.

**Not to touch:** the prose content of any `docs/*.md` file (that is the README, USER, and DEV agents' work); `docs/site/*.html`'s own content (only its inclusion in the build).

**Deliverables, in order:**
1. `mdbook` builds `docs/` locally with a `SUMMARY.md` that mirrors `docs/README.md`'s three groups (users, contributors, decisions) and includes every file in the inventory above, including the ADRs.
2. `docs/site/`'s five pages are reachable from the built site's nav (a "Guides" or "Interactive pages" entry) without mdBook reprocessing their HTML.
3. A manual pass with NVDA and, separately, JAWS, or a documented reason it could not be run in this environment: skip link, heading structure, the search box's keyboard reachability, and contrast in light and dark. Record every finding, fixed or not.
4. `.github/workflows/pages.yml`: builds on push to `main` when `docs/` or `book.toml` changes, deploys with `actions/deploy-pages`.
5. If step 3 finds an accessibility problem in mdBook's theme that cannot be fixed quickly, switch to the plain MkDocs fallback and say so plainly in the report; do not ship a known-broken site.

**Checks:** the common set, plus the mdBook build itself and `py -3 tools/check_site_a11y.py` against every page the build produces (extend the tool's file list if it only scans `docs/site/` today).

**The owner's checklist (five):** on the published site, with NVDA then JAWS: (1) the skip-to-content link lands on the main heading; (2) type a search term, then reach and read a result with the keyboard alone, no mouse; (3) navigate by heading through `dev/architecture.md`, one of the longest pages, with no skipped levels; (4) on the Braille display, read a settings table and a fenced code block, checking for garbled box-drawing or truncation; (5) the light/dark toggle is announced and not color-only.

### Agent README: README and the quick start

**Branch** `docs-sweep/readme-quickstart`.

**Owns:** `README.md`, `docs/quickstart.md`, `docs/install.md`.

**Not to touch:** `docs/README.md` (the DEV agent's), any other guide.

**Deliverables, in order:**
1. Trim `README.md` to the outline above, about 65 lines: what it is, start here (roadmap link dropped), download, an inline one-command-per-platform quick start, building, license, see also.
2. Move the `tw` command list, Scripts, Repository layout, and Third-party data content: Scripts and Repository layout to `docs/dev/building.md`; Third-party data to a new `docs/dev/third-party-data.md` or into `docs/dev/building.md` too (the owner's call, question 2 below; write it into `docs/dev/building.md` by default and say so in the report so the DEV agent can move it again if the owner picks the other file).
3. Confirm `docs/quickstart.md` and `docs/install.md` match the current release (`0.1.0-alpha.4`) and the aarch64 AppImage's actual status.
4. Every link in the moved content still resolves.

**Checks:** the common set. **The owner's checklist:** nothing to hear (text-only change); confirm by eye or with NVDA that the trimmed README still reads sensibly top to bottom.

### Agent USER: user guide accuracy pass

**Branch** `docs-sweep/user-guides`.

**Owns:** every file in "For users" in the inventory table above except `quickstart.md` and `install.md` (the README agent's) and the two generated files.

**Not to touch:** `README.md`, `docs/README.md`, anything under `docs/dev/` or `docs/adr/`.

**Deliverables, in order (highest-value gaps first):**
1. Fix `screen-readers.md`'s stale GUI section (edit mode exists now) and add a Braille display section: what is tested, what is not, and how to reach the checklist.
2. Add the math engine setting and Unicode math display to `math.md`.
3. Add Markdown lint and the code-block language announcement to `editing.md` (checking first whether harper-core and syntect shipped or are still held per W4g's status line; document only what actually merged).
4. Add notes export formats to `notes.md`.
5. Update `converting.md`'s file list for RTF, ODT, and DOCX comments/revisions, checking `wave4/c2-documents`'s merge status first.
6. Cross-link the six interface languages from `speech.md` to `settings.md`.
7. Spot-check the rest of the owned files' key examples against `CHANGELOG.md`'s "Keys: what changed" section.

**Checks:** the common set. **The owner's checklist (four):** the math engine setting read aloud at each verbosity; a lint problem found with Ctrl+F8; `tw marks --export --format ris` on a document with notes; RTF and ODT fixtures opened in the reader.

### Agent DEV: developer guides, ADRs, and history

**Branch** `docs-sweep/dev-docs`.

**Owns:** `docs/dev/*.md`, `docs/json-rpc.md`, `docs/roadmap.md`, `docs/star-gaps.md`, `docs/README.md`, `docs/adr/README.md` (index text only, never an individual ADR's decision), `CONTRIBUTING.md`'s docs section.

**Not to touch:** individual ADR files, any file under "For users" in the inventory.

**Deliverables, in order:**
1. Receive the Scripts, Repository layout, and Third-party data content from the README agent and place it properly in `docs/dev/building.md` (and a new `docs/dev/third-party-data.md` if the owner picks that split).
2. Run `cargo xtask docs --check` and fix every drift it reports (ADR index, Decisions list, crate count, "See also" links) rather than hand-auditing.
3. Cross-reference `star-gaps.md` against `docs/research/whats-left.md`'s Star-parity section and reconcile.
4. Confirm `docs/README.md`'s index matches the SITE agent's `SUMMARY.md` groupings once the SITE agent's first commit lands (coordinate branch order: DEV merges its index changes before SITE finalizes `SUMMARY.md`, or SITE rebases).
5. `docs/dev/releasing.md`'s listening-checklist reference matches `CHANGELOG.md`.

**Checks:** the common set, plus `cargo xtask docs --check` clean.

**The owner's checklist:** nothing to hear.

### Agent REVIEW: final reviewer

**Branch** `docs-sweep/review`, starts after the other four merge.

**Owns:** small fixes anywhere in `docs/` found during the read-through; does not own large rewrites (files those back to the owning agent instead, listed in the report).

**Deliverables, in order:**
1. Read every doc end to end against the running program: `textweaver --help`, `tw --help` and every subcommand's `--help`, the keymap (`cargo xtask keyboard`'s output), and the settings schema (`cargo xtask settings-doc`'s output). Note every mismatch.
2. Run the full check set: `cargo fmt --all --check` is not relevant here, but `py -3 tools/check_links.py`, `cargo xtask docs --check`, `cargo xtask settings-doc --check`, `py -3 tools/gen_site_data.py --check`, the mdBook build, and `py -3 tools/check_site_a11y.py` all clean.
3. A final NVDA and JAWS pass on the published site (or the local build if not yet deployed), independent of the SITE agent's own pass.
4. Write the punch list: what is fixed, what is deferred (with a reason and, if known, which future wave it belongs to).

**Checks:** the full set above, clean.

**The owner's checklist (five, the final one to hand the owner):** (1) skip link and heading navigation on the built site; (2) the search box by keyboard alone; (3) a Braille display pass over one settings table and one code block; (4) light/dark toggle announced and not color-only; (5) one full guide read start to finish with NVDA, checking that nothing reads as stale against the running program.

## Risks and open questions

**Risks:**
- mdBook's own theme accessibility is not independently documented; the SITE agent's manual pass, not a search result, is the real evidence. Budget time for a fallback to plain MkDocs if it fails.
- `SUMMARY.md` has no auto-discovery, so it will drift from `docs/README.md`'s groupings over time unless a check script compares the two file lists; none exists today. Consider adding one in a later wave.
- `docs/site/`'s standalone pages must stay reachable and pass `check_site_a11y.py` once folded into the mdBook build; a build-time path collision (mdBook's own `theme/` folder versus `docs/theme/`) is an easy mistake, worth a dry run.
- Five agents touch `docs/README.md` and the trimmed `README.md` conceptually even though ownership is split; the DEV and README agents must coordinate the handoff of moved content once, not repeatedly, or content will duplicate or vanish.
- Generated files (`keyboard.md`, `settings-reference.md`, `history/parity-report.md`) are easy to hand-edit by habit; every agent's checklist calls this out, but the reviewer should grep for accidental hand edits (a diff against a fresh generator run) before closing out.

**Questions for the owner, each with a recommended default:**
1. mdBook or plain MkDocs? **Recommended: mdBook,** per the comparison above; switch to the fallback only if the SITE agent's accessibility pass fails it.
2. Where should the Third-party data section land once it leaves the README: folded into `docs/dev/building.md`, or its own `docs/dev/third-party-data.md`? **Recommended: its own file,** since it covers licenses and belongs somewhere linkable from the license section too.
3. Should GitHub Pages use the default `leavesofgrass.github.io/textweaver` URL, or a custom domain? **Recommended: the default URL** for now; a custom domain is a small, separate change later if wanted.
4. Should `docs/site/`'s interactive pages be linked from the published site's navigation, or kept as an offline-only extra not built into Pages? **Recommended: linked from the site,** since they already work offline and travel with every release regardless.
5. Is it acceptable to spend the SITE agent's early time hand-authoring `SUMMARY.md` rather than deferring to a tool that auto-discovers structure (which would mean picking MkDocs instead)? **Recommended: yes,** given the accessibility case for mdBook outweighs the convenience of auto-discovery.

## See also

- [Documentation index](../README.md)
- [What is left](whats-left.md): the existing stale-passage list this plan builds on.
- [The 2026 roadmap](roadmap-2026.md)
- [Wave 4 orchestration plan](wave4-orchestration.md): the brief format and common rules this plan reuses.
- [CONTRIBUTING.md](../../CONTRIBUTING.md): the doc-writing rules every agent follows.
