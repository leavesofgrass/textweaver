# Wave 4 plan review

Written on Sunday, September 27, 2026, by the Wave 4 planning researcher, for the orchestrator and the owner. It is a proposal. It changes nothing in the plan itself. The plan it reviews is the refined Wave 4 plan of Sunday, September 27, 2026, in `docs/history/tasks.md`: two groups of five agents, W4h, W4a1, W4b, W4g, and W4c1 first, then W4a2, W4c2, W4f, W4d, and W4e.

Sources: `docs/history/tasks.md` (the Wave 4 section, its lessons, the briefs, and the Wave 3 status lines), `docs/research/wave4.md`, `docs/research/usability-terminal.md`, `docs/roadmap.md`, `docs/research/pure-rust-wave3.md`, ADRs 0023 to 0027, the last sixty commits on `main`, the owner's wiki (the Star hub and its audits, the textweaver hub, the deletion incident pages, and the global rules), and crates.io and GitHub on the day of writing, read with the neutral User-Agent. Every version below was checked on Sunday, September 27, 2026.

The owner's goal, as the brief states it: terminal-first Markdown reading and authoring, with speech and highlighting that never stall or lose your place. Everything below is measured against that.

## The short version

- The order is close to right. Two changes would serve the goal better: trim W4a1 so the owner's listening session can happen early in Group 1, and start W4g only after W4h merges, because both rewrite the same messages and keys.
- The biggest hidden risk is not disk. It is five agents finishing at once and each running the Docker all-features check on one shared Docker Desktop memory budget. Serialize those runs.
- Six quick wins are worth doing before Group 1 launches: the six fuzz targets in the nightly matrix, `py -3` in the docs, "Ready" on the title line, the JSON closed-pipe panic, the `tw` no-arguments hint, and the AccessKit clipped-options mitigation.
- Three research corrections: syntect's default features build C code, so W4g must turn them off; MathCAT 0.7.6 is still a release candidate with an open braille panic; and W4d's brief names `fluent-bundle`, but W3e already shipped its own Fluent-subset catalog by design (ADR-0025), so W4d should extend that first.
- Ropey 2 has been in beta for over a year with no new release, and crop has had no release since April 2025. W4b should measure both and switch to neither in Wave 4.

## 1. Priorities check

### What the order gets right

- W4h first is right. It is small, it is all terminal, and every item is something the owner hears on every run. Nothing else in Group 1 touches a user as often.
- W4b in Group 1 is right. Startup time, allocations, and the memory growth are the "never stall" half of the goal.
- W4g in Group 1 is right for "authoring". Grammar, lint, clipboard, and code highlighting are the terminal-first writing items.
- W4a2 waiting for the owner's session is right. Parity work built before the session risks rework if the session changes the highlight or announcement design.
- Removing branch pruning from W4f is right, and matches the wiki's rules after the deletion incident.

### What I would change

**Trim W4a1 so the session can happen early.** W4a1 is the largest brief in Group 1, and edit mode in `DocumentView` is the riskiest piece of GUI work in the wave: no Rust app has good screen-reader editing of multi-line text yet (`docs/research/wave4.md`). The owner's session needs only three small things from W4a1 first: the direct UI Automation notification option, the clipped-options mitigation in lists and the settings form, and the memory growth attributed. Put those first in W4a1's brief, in small commits, and ask the orchestrator to build the GUI for the owner's session as soon as they merge, while W4a1 goes on to reading aids and edit mode. That way W4a2 can start in Group 2 with the session's findings in hand, instead of waiting on all of W4a1.

**Start W4g after W4h merges, not beside it.** Both agents edit the app's message building, the keymap, `docs/keyboard.md`, the site data, and `CHANGELOG.md`. W4h is a day or two of work. Let it finish, merge it, then launch W4g from the merged main. If the orchestrator wants five agents busy from the first hour, W4c1 (MathCAT) is independent of every other Group 1 agent and can take the fifth slot from the start.

**Move the GUI memory investigation from W4b to W4a1.** The growth (98 MB to 172 MB) is inside `textweaver-xilem`, which W4a1 owns. Two agents in that crate will conflict. W4b should provide the measuring tools (a peak-allocation counter behind a feature, and the `--log` numbers) and the workspace-wide allocation cuts; W4a1 should find and fix the GUI's growth with them. The likely suspects are the new engines and data merged from main (Piper, Whisper, OCR, the 9.9 MB lexicon file) being loaded at startup rather than on first use, and the renderer's buffers.

**Clarify who owns startup announcements.** UX-1 suggested that "queue announcements until the engine is ready" go to W4b, since W4b owns startup. The refined plan gives it to W4h. Keep it in W4h, and say so in W4b's brief, so W4b does not also touch `launch`.

**Keep W4c1 at P2, and let it slip if MathCAT is not ready.** MathCAT's stable release is still 0.7.5. 0.7.6 has three release candidates (the last on August 23, 2026) and no final; 0.7.7-alpha.1 came on September 23, 2026. An open issue (#827, updated September 26, 2026) reports a panic in `GetNavigationBraille` in no-unsafe builds, and another (#817) a test failure on Windows. Math matters for students, but it is not on the path to the owner's goal. If 0.7.6 is not final when Group 1 starts, W4c1 should build on 0.7.6-rc.3 behind the feature, keep speech only, and leave braille for Wave 5.

**Consider dropping or shrinking W4e.** Offline translation and summaries are the furthest from the goal, they download about 110 MB per language pair, and they add a third tokenizer. If Group 2 needs the disk or the attention, W4e is the one to cut. If it runs, the no-model LexRank summary is the useful half; translation can wait.

**Do not split further.** Two agents in one crate cost more in merges than they gain. The W4a1 and W4a2 split (view against dialogs) is the only one that pays, because the owner's session sits between them.

### Proposed order, for comparison

Group 1: W4h; W4b; W4c1; W4a1 (session items first); then W4g as soon as W4h merges.

Group 2: W4a2 (after the session); W4c2; W4f; W4d; W4e if there is room.

## 2. Quick wins

Each is under half a day for one agent. Sizes: tiny is under an hour, small is under half a day.

### From UX-1's ranked list

1. **"Ready" on the title line before the first play** (tiny; W4h). `ui.rs` maps `Playback::Idle` to "Stopped". A screen reader reading the title line hears "Stopped" before anything was read. Say "Ready" until the first play.
2. **`tw search --json` and `tw info --json` on a closed pipe** (tiny; W4h). The text paths use `print_all`, the JSON branches still use `println!`. Route them through the same helper, with a test that closes the pipe.
3. **`tw` with no arguments** (tiny; W4h). Twenty-one commands scroll past as an error with exit status 2. Print a two-line hint instead, and keep `tw --help` for the full list.
4. **Escape in edit mode says how to finish** (small; W4h). Already in the refined plan. "Still editing. Control E finishes." in the spoken form of the key.
5. **The command palette's opening sentence** (small; W4h). "Command. Type part of a name; Tab completes, Up and Down list matches." said once when it opens. The drawn label stays short.
6. **Repeat a list's introduction** (small; W4h or W4g). A key inside an open list that says the list's title and count again, and the keys it takes. UX-1's item 8. Pairs with the "say status" action already planned.
7. **The quick start says `q` too with the classic keys** (tiny; W4h). UX-1's item 12.

### From Wave 3's leftovers

8. **The six new fuzz targets in the nightly workflow** (tiny; the orchestrator, before Group 1, or W4f). `fuzz/Cargo.toml` has fifteen targets; `.github/workflows/nightly.yml` runs nine. Missing: `daisy`, `pptx`, `sheet`, `archive`, `image`, and `web`. Add them to the matrix and add their corpus lines (`fixtures/` has DAISY, PPTX, and sheet files from W3d). Only the orchestrator or W4f may edit `.github/`, so do it before launch rather than waiting for Group 2.
9. **`py -3` in the docs** (tiny; W4h). `python` on this machine's PATH is the Windows Store stub since the profile was wiped, and `py -3` is what works. `CONTRIBUTING.md`, `docs/dev/testing.md`, `docs/dev/building.md`, and the tasks preamble's date command say `python` or `python3`. Add one line: on Windows, use `py -3`. Also update the date command in the shared preamble, since every agent copies it.
10. **AccessKit leaves out clipped options** (small; W4a1, before the session, as a listed exception to the "W4a2 owns dialogs" rule). The UI Automation report sees 13 of 15 settings sections, and options scrolled out of a list are not in the tree. The owner's checklist item 9 tests this. The mitigation to try first: do not mark the list box and the settings form as clipping their children, so every option node stays in the tree with its scrolled bounds. If AccessKit still drops them, record it for the upstream issue. Half a day, and it answers a checklist question before the owner spends time on it.
11. **`cargo xtask notices` after W3d** (tiny; the orchestrator). W3d's status line says cargo-about was not installed there and the notices were merged by hand. Run `cargo xtask notices --check` on main once, since the toolchain was reinstalled on D:.
12. **GUI memory: a first measurement with engines off** (small; W4a1). Before hunting, run the GUI with `--log` and the `publish` and voice features off, and with each new engine crate excluded in turn. If the working set drops back near 98 MB, the growth is startup loading that should be lazy, and that is a small fix.

### From the wiki and Star's history

13. **Notes export to BibTeX, RIS, and JSON** (small; W4g). Already in the "Also for Wave 4" list. Star had it, students used it with Zotero and Word, and the citation crate already has the record types.
14. **A "say status" and "repeat last message" action** (small; W4h). Already planned. Star's GUI had a status bar the screen reader could read; textweaver's self-voicing users have nothing equivalent. Give it one action and one browse key, in both frontends.
15. **Theme cycle order stays stable** (tiny; W4a2, as a test). Star's rule from 0.1.31: new themes go after the existing ones so the F5 cycle a user knows does not change. Add a test that the first nine themes in the cycle keep their order.
16. **A test that reads every toolbar or help hint from the keymap** (small; W4h). Star's 2.1.4 audit found eight tooltips that named keys the actions no longer had, and the test that should have caught it checked three fixed bindings. textweaver's help already reads from the keymap (UX-1's fix 6); add the test that every key named in any spoken message comes from the keymap and not from a string.

### From this research

17. **syntect without C** (tiny; W4g). Add it as `default-features = false, features = ["default-fancy"]`, and two-face with its `syntect-fancy` feature. syntect's default is `regex-onig`, which builds the Oniguruma C library. See section 3.
18. **Pin tokenizers below 1.0** (tiny; W4e). `tokenizers` 1.0.0-rc.2 was published on September 21, 2026. The fallback should pin 0.23.2 so a release candidate does not change the API under it.

## 3. Research updates

Checked against crates.io and GitHub on Sunday, September 27, 2026, with the neutral User-Agent. "Unchanged" means the version in `docs/research/wave4.md` is still the newest.

### Formats and authoring (W4c1, W4g, W4c2)

- **MathCAT.** Stable is still 0.7.5. Newest is 0.7.7-alpha.1 (September 23, 2026); 0.7.6-rc.3 (August 23, 2026) is still the last 0.7.6 build, so "pin 0.7.6 once it is stable" has no date yet. The GitHub releases page has only a rolling "Development Build" prerelease (September 26, 2026) after v0.7.0. Risks: issue #827, "GetNavigationBraille panics in no-unsafe builds", open, updated September 26, 2026, which matters because this workspace denies unsafe code; issue #817, a test failure on Windows, open. Advice: adopt 0.7.6-rc.3 behind the feature, speech first, braille only after #827 closes. Links: `https://crates.io/crates/mathcat`, `https://github.com/daisy/MathCAT/issues/827`, `https://github.com/daisy/MathCAT/issues/817`.
- **harper-core.** 2.11.0 (September 16, 2026, Apache-2.0) unchanged. It releases about every two weeks. Its manifest pulls `pulldown-cmark` 0.13, `ammonia` (an HTML sanitizer, which brings html5ever), `zip` 8.6 with `deflate` only (the same as ours), `regex`, and `harper-brill`; the `thesaurus` feature is on by default and can be turned off. It is pure Rust. Risk: binary size, which W4b is trying to cut. Put it behind a `grammar` feature and measure the size before and after. Link: `https://github.com/automattic/harper`.
- **rumdl.** 0.2.77 (September 23, 2026, MIT, MSRV 1.94.0; ours is 1.96) unchanged. It does expose a library, `rumdl_lib`, with default features `parallel` and `native`. Risks: it is a command-line tool first, it released four times in one week, and the 0.2 series makes no API promise. Advice: use it behind a feature, call a fixed rule set, and pin the exact version; or lint only the rules a blind author needs (heading levels, list markers, trailing spaces, unresolved link references), which the structure code already knows. Link: `https://github.com/rvben/rumdl`.
- **arboard.** 3.6.1 (August 23, 2025, MIT or Apache-2.0) unchanged, and quiet for a year. On Linux it uses x11rb and wl-clipboard-rs, both pure Rust; turn on `wayland-data-control` for Wayland. Link: `https://github.com/1Password/arboard`.
- **syntect.** 5.3.0 (September 27, 2025, MIT) unchanged. **Correction:** its default feature set is `default-onig`, which compiles the Oniguruma C library. The brief must say `default-features = false, features = ["default-fancy"]`, which uses `fancy-regex`. Link: `https://github.com/trishume/syntect`.
- **two-face.** 0.5.2 (August 7, 2026, MIT or Apache-2.0) unchanged. It bundles bat's syntaxes and has a `syntect-fancy` feature to match. Its repository moved to Codeberg: `https://codeberg.org/CosmicHarper/two-face`.
- **mail-parser.** 0.11.9 (September 9, 2026, Apache-2.0 or MIT) unchanged; releases monthly. Link: `https://github.com/stalwartlabs/mail-parser`.
- **rtf-parser** 0.4.3 unchanged. **math-core** 0.8.2 (September 1, 2026, MSRV 1.96) unchanged; it is only a test comparison.
- **zip.** 8.6.0 is still the newest stable. 9.0.0-pre3 (August 11, 2026) exists; do not move to it in Wave 4. harper-core also depends on 8.6, so there is no duplicate.

### The GUI (W4a1, W4a2)

- **AccessKit.** accesskit 0.25.1, accesskit_winit 0.34.1, accesskit_windows 0.35.1, accesskit_consumer 0.39.1, accesskit_unix 0.24.0, and accesskit_macos 0.27.1, all from September 25, 2026 (MSRV 1.87), unchanged. These are what ADR-0027 already vendored.
- **Parley.** 0.11.1 (August 16, 2026) unchanged. **Our vendored Parley is 0.8.0**, patched to AccessKit 0.25 (ADR-0027). Parley main dropped its AccessKit feature (PR #716). Advice: W4a1 should not upgrade Parley during edit mode. The gap from 0.8 to 0.11 changes the style and editor APIs, and the accessibility bridge would have to move into our code at the same time. Do edit mode on 0.8, then upgrade as a separate task in Wave 5 with a measurement.
- **Xilem and Masonry.** crates.io still has 0.4.0 (October 29, 2025). Our vendored copy is the September 14, 2026 revision of main. No new release. The bump to AccessKit 0.25 remains our patch; ADR-0027 proposes it upstream.

### Speed (W4b)

- **icu_segmenter** 2.3.0 (August 13, 2026, Unicode-3.0, MSRV 1.88) unchanged.
- **ropey.** Stable 1.6.1; 2.0.0-beta.1 from August 2, 2025 is still the newest, so the beta is over a year old with no further release. **crop** 0.4.3 (April 25, 2025, MIT) has had no release since. Advice: measure both on the edit traces and report, but do not change the rope in Wave 4. The rope is the text model (ADR-0002), W3a just made `DisplayIndex` the rope, and every position, marker, and offset map depends on it. A rope change is a Wave 5 ADR, not a speed item.
- **memchr** 2.8.3 and **aho-corasick** 1.1.5 unchanged.
- **rustls-graviola** 0.4.0 (June 17, 2026) unchanged; ureq 3.4.2 (September 13, 2026) still calls the provider hook unstable. Advice: drop this optional item from W4b. It touches only downloads, which are rare, and it needs CPU feature checks at run time.
- **dhat** 0.3.3 (February 2024) and **divan** 0.1.21 (April 2025) unchanged; both are quiet but work. **cargo-bloat** 0.12.1 (May 2024) unchanged.

### Translations (W4d)

- **fluent-bundle** 0.16.0 (May 22, 2025), **fluent-templates** 0.15.1 (August 5, 2026), **i18n-embed** 0.16.0 (July 9, 2025), **unicode-bidi** 0.3.18 (December 16, 2024): all unchanged.
- **Correction to the brief.** ADR-0025 records that W3e wrote a Fluent-subset catalog of its own (`textweaver_lexicon::i18n`) instead of `fluent-bundle`, on purpose: no new dependencies, a catalog as a value rather than global state, plural categories for English, the Romance languages, German, Arabic, Hebrew, and Persian, and bidi isolation. The files are valid Fluent so a later switch is possible. W4d's brief says "Use Fluent: fluent-bundle with fluent-templates, or i18n-embed." It should say: extend W3e's catalog first, and switch to `fluent-bundle` only if a language needs attributes, functions, or number formatting, as ADR-0025 already provides.

### Offline intelligence (W4e)

- **rten**, **rten-generate**, and **rten-text** 0.26.0 (August 29, 2026, MSRV 1.94) unchanged.
- **kitoken** 0.11.0 (May 10, 2026, BSD-2-Clause) unchanged.
- **tokenizers.** 0.23.2 is stable; **1.0.0-rc.2 was published on September 21, 2026.** Pin `0.23` for the fallback.
- **ocrs** 0.13.1 (September 13, 2026) is newer than Wave 3's research named; W3d should already be on it, and W4b's dependency refresh can confirm.

## 4. Risks and lessons

### Disk and memory

- **The numbers today.** D: has 466 GB free of 2 TB. `D:\sccache` holds 5.4 GB. Eleven worktrees exist under `.claude/worktrees`. The plan's floor is 200 GB free, so Group 1 has about 260 GB to spend. At Wave 3's measured sizes (about 40 GB per agent's build folders), five agents fit; nine did not.
- **The cache size is stated two ways.** The Wave 4 section says `SCCACHE_CACHE_SIZE=50G`; this review's brief says 30 GB. Pick one and write it in one place, `docs/dev/building.md`, and have the tasks preamble point there.
- **"Per-agent Docker build volumes" are one volume.** `compose.yaml` mounts a single `textweaver-target` volume at `/target`, and each agent uses `/target/agent-<letter>` inside it. Flushing merged output means deleting folders inside that volume, from inside a container. That is a delete, so the deletion rules apply in full: list the exact folder first, use a plain literal path, run it on its own, and let the orchestrator do it, never an agent. Write the exact command into the orchestrator's flush step now, so it is not improvised between groups.
- **8 GB per container is not enforced anywhere I can see.** `compose.yaml` sets no memory limit. Docker Desktop on WSL 2 gives all containers one shared budget, by default half of the machine's memory. Five agents each running `cargo test --workspace --all-features` in Docker at the same time will share it, and the linker for the GUI and OCR crates is memory-hungry. That is the likeliest way for Group 1 to stall. Advice: serialize the Docker checks. Either the orchestrator runs each agent's Docker check at integration, one at a time, or the agents take a lock file on D: before starting one and release it after. Say which in the preamble.
- **Timing tests under load.** The 40-run rule stands. Add: run the 40 runs while another build is going, not on a quiet machine, because that is when they failed in Wave 3.
- **Keep the `MSYS_NO_PATHCONV=1` rule loud.** The junk `C:` folder that started the deletion incident came from one Docker command without it. Every brief should carry the line, not only the preamble. The `.gitignore` now ignores `/C*/Program Files/`, which hides the symptom but not the cause.

### Merge conflicts

- **W4h and W4g** share the app's messages, the keymap, `docs/keyboard.md`, and the site data. Sequence them (section 1).
- **W4a1 and W4b** would both touch `textweaver-xilem` if the memory item stays in W4b. Move it (section 1).
- **W4b and W4c2** both touch `textweaver-formats` if W4b changes zip features while W4c2 adds loaders. Have W4b make the zip change first, in one commit, and tell W4c2 to merge main after it lands.
- **W4c1 and W4c2** both extend the math and DOCX readers' neighbours. W4c1 owns `textweaver-math` and the BRF writer; W4c2 owns loaders. The EPUB 3 MathML item is the seam: give it to W4c1, since its value depends on MathCAT, and have W4c2 leave the EPUB loader alone.
- **The usual shared files.** Root `Cargo.toml` members, `CHANGELOG.md`, `docs/history/tasks.md`, `docs/adr/README.md`, and `docs/site/*` conflict every wave. The lessons list already says to keep edits additive and regenerate generated files. Add: each agent puts its `CHANGELOG.md` lines under its own heading, so merges are line-disjoint.
- **ADR numbers.** The next free number is 0028. W4a1, W4c1, W4d, and W4e will each want one. Assign them in the briefs now: 0028 for W4a1 (editable text and reading aids in the GUI), 0029 for W4c1 (MathCAT), 0030 for W4d (translations and right-to-left), 0031 for W4e (offline models). Two agents picked the same number in Wave 3.

### Screen-reader checks

- Only the owner hears anything. Every agent that changes an announcement should end its report with a checklist of at most five things for the owner to try, in the order of his checklist under W3b.
- Star's most expensive regression was a silent default engine that a fully green suite let through, because the GUI tests stubbed the speech manager (wiki, the qtspeech lesson). W4h's "queue announcements until the engine is ready" and W4b's startup changes both sit exactly there. Both briefs should say: after the change, run `cargo xtask listen` and put the result in the report, and the owner confirms by ear before the item is called done.
- The GUI session decides two designs: the highlight as selection or as a background attribute, and live regions or a direct UIA notification. Nothing that depends on either should be built before the session. Reading aids and edit mode in W4a1 depend on the highlight design. That is the reason for trimming W4a1 (section 1).

### Lessons from Star that bear on planned items

- **RSVP must not talk.** Star's RSVP overlay carried no accessibility metadata, and its audit warned that a word change every 200 ms reaching a screen reader would make it unusable (wiki, the 502.3 audit, fix 2). W4a1's plan, a hidden flashing node and a quiet labelled status node with `Live::Off`, is the right answer. Add a test that the RSVP word node never gains a live setting and never takes focus.
- **A window swap must not go silent.** Star's pagination replaced the whole accessible text object mid-read with no event. `DocWindow` slides and recentres in the same way. W4a2's parity work should check, with the UI Automation report, that a slide during reading keeps the caret in a valid range and that NVDA does not lose its place. That is item 2 on the owner's checklist; make it item 1 for the second session.
- **The highlight must be the thing, not a picture of it.** Star painted the karaoke highlight with an extra selection, invisible to assistive technology. textweaver's GUI exposes it as a background attribute and moves the caret. Keep both, whichever the session picks for the visual.
- **Character keys need an off switch.** Star's TUI had 44 single-character keys with no way to turn them off. textweaver has F9 and the WCAG 2.1.4 test. W4h's new "say status" key and W4g's new keys go through that test, and the JAWS and NVDA conventions, before they are chosen.
- **Automatic choice must fall back and say so.** W4d's "each language gets a default voice for that language" is exactly the kind of automatic engine choice that went silent in Star. The rule: if the voice for the language is missing, keep the current voice and say so, never go quiet.
- **Settings are written atomically and unknown keys survive.** W4d's first-run language choice and per-language voices are new settings, so the four-places rule applies, and the settings export fixture will fail until they are used.
- **"Needs a restart" is usually a stale snapshot.** Star's dictation needed a restart for a year because a module-level detection ran once. A language change in W4d should apply live: the catalog is an `Arc` value already, so swap it and re-announce.
- **Docs go stale in clusters.** Star's dictation docs were wrong in seven files for a release cycle. W4h fixes three passages; W4f's release notes should include a doc pass against the code, and `tools/check_links.py` is not enough for that.
- **Doc-writing agents drift.** The wiki's lesson from abax: agents told to "update the docs" no-op or wander unless the brief names the files and the bullets. Every Wave 4 brief that includes docs should list the files.
- **Star's quirks are fixed, not ported.** The Phase 0 inventory found highlight drift from normalizing after mapping, an HTML loader that dropped everything after the first `<meta>`, dead chapter navigation, and a search that never ran. None of these should reappear through a "parity" item in W4a2. Parity means Star's features, not Star's bugs.

### Lessons from the deletion incident that bear on the wave

- The refined plan already has the rules: only the owner overrides; Bash deletes need approval; PowerShell `Remove-Item -LiteralPath` with a plain path; work only in your worktree.
- Two more from the wiki's knowledge base are worth adding to the preamble: never chain a delete onto another command, and never hand-type an escaped file name. The incident's command did both.
- "Don't make the owner approve every step" is also a rule. The guard covers the dangerous cases; the briefs should not add approval prompts for ordinary work.

## 5. Concrete brief changes

Exact wording to add or change. Additions are marked "add"; replacements quote the current text.

### Shared preamble and the Wave 4 lessons

- Under "Dates", replace the command with: `py -3 -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"` on Windows, `python3 -c ...` elsewhere. Add: "`python` on this machine is the Windows Store stub; use `py -3`."
- Under "Disk and memory", add: "The Docker all-features check is run one agent at a time. Take the lock file `D:\textweaver\.cache\docker.lock` before you start it and remove it after; if the file exists, wait." Or: "The orchestrator runs the Docker check at integration; agents run only the native checks." Choose one.
- Under "Disk and memory", add: "Never delete inside the Docker target volume. The orchestrator flushes merged agents' folders after listing them."
- Under "Deleting", add: "Never chain a delete onto another command. Never hand-type an escaped file name; use `git rm` or `git clean` and show the owner first."
- Under "Merging with other agents", add: "Your ADR number is in your brief. `CHANGELOG.md` entries go under a heading with your agent's name."
- Add: "The next free ADR number is 0028. W4a1 takes 0028, W4c1 0029, W4d 0030, and W4e 0031."
- Add: "Every report ends with a checklist of at most five things for the owner to try with NVDA and JAWS, or the line 'nothing to hear'."

### W4h: terminal polish

- Add, at the top: "Small commits, merged first; W4g starts from your merged branch."
- Add the quick wins: "Ready" until the first play; `tw search --json` and `tw info --json` on a closed pipe; the two-line `tw` hint; the palette's opening sentence; a key to repeat a list's introduction; the quick start's `q` line; `py -3` in `CONTRIBUTING.md`, `docs/dev/testing.md`, `docs/dev/building.md`, and the tasks preamble.
- Add: "Announcements queued until the engine is ready are yours, not W4b's. After the change, run `cargo xtask listen` and report the result."
- Add: "Add a test that every key named in a spoken message comes from the keymap, not from a fixed string."
- Add: "New keys: the keymap conflict and reachability tests, WCAG 2.1.4, and Windows Terminal's taken keys, before you pick one."

### W4a1: GUI edit mode and reading aids

- Replace the opening with an order: "1. The direct `UiaRaiseNotificationEvent` option. 2. The clipped-options mitigation in `ChoiceList` and `SettingsGrid` (a listed exception: you may edit these two dialog widgets). 3. The GUI's memory growth, measured first with engines and `publish` off. Merge these three and tell the orchestrator, so the owner's session can be built. 4. Then the reading aids. 5. Then edit mode."
- Add: "Do not upgrade the vendored Parley. Edit mode is built on 0.8.0 as vendored."
- Add: "The RSVP word node never has a live setting and never takes focus; test it."
- Add: "Your ADR is 0028."
- Add: "Parity with the terminal reader and the wxDragon removal are W4a2's, after the owner's session."

### W4b: speed and memory

- Remove: "Add finding the cause of the Xilem GUI's memory growth." Replace with: "Provide a peak-allocation counter behind a feature and the `--log` numbers; W4a1 fixes the GUI's growth with them."
- Replace "Ropey 2. Evaluate it ... and crop" with: "Measure ropey 2.0.0-beta.1 and crop 0.4.3 on the edit traces and report the numbers. Do not change the rope in Wave 4; that is a Wave 5 ADR."
- Remove: "Optional TLS change" (rustls-graviola).
- Add: "Make the zip feature change in one early commit and tell W4c2."
- Add: "Startup announcements are W4h's; do not touch `launch`."
- Add: "Run new timing tests 40 times while another build is running."

### W4g: authoring extras

- Add: "Start from main after W4h merges."
- Replace "Code highlighting in the terminal with syntect 5.3 and two-face 0.5.2" with: "syntect 5.3 with `default-features = false, features = ["default-fancy"]` and two-face 0.5.2 with `syntect-fancy`; the defaults build the Oniguruma C library."
- Replace "Grammar checking ... harper-core 2.11.0" with: "harper-core 2.11.0 behind a `grammar` feature, `thesaurus` off, with the binary size measured before and after."
- Replace "Markdown lint and format with rumdl 0.2.77" with: "rumdl 0.2.77 (`rumdl_lib`) behind a feature and pinned exactly, with a fixed rule set; or our own lint for the rules a blind author needs, if rumdl's size or API churn is too much. Say which in the report."
- Add: "arboard on Linux with `wayland-data-control`."
- Add: "Notes export: BibTeX, RIS, and JSON, using the citation crate's record types."

### W4c1: MathCAT

- Replace "Pin 0.7.6 once it is stable" with: "0.7.6 has no final release. Use 0.7.6-rc.3 behind the feature and record the version in ADR-0029. Speech first. Braille only if MathCAT issue #827 (a panic in `GetNavigationBraille` in no-unsafe builds) is closed; this workspace denies unsafe code."
- Add: "EPUB 3 MathML is yours; W4c2 leaves the EPUB loader alone."
- Add: "Your ADR is 0029."

### W4a2: GUI parity and wx removal

- Add: "First item after the session: a window slide during reading keeps the screen reader's place. Check it with the UI Automation report and put it first on the owner's second checklist."
- Add: "Parity means Star's features, not Star's bugs. The Phase 0 inventory lists the bugs."
- Add: "A test that the first nine themes keep their cycle order."

### W4c2: documents

- Add: "Merge main after W4b's zip commit lands."
- Add: "Do not touch the EPUB loader; EPUB 3 MathML is W4c1's."

### W4f: platforms and CI

- Add, if not done before launch: "The six W3d fuzz targets (`daisy`, `pptx`, `sheet`, `archive`, `image`, `web`) in the nightly matrix with corpus lines."
- Add: "A doc pass against the code before the release notes, file by file."
- Keep: branch pruning removed.

### W4d: translations

- Replace "Use Fluent: fluent-bundle with fluent-templates static_loader!, or i18n-embed with fl!" with: "Extend W3e's Fluent-subset catalog (`textweaver_lexicon::i18n`, ADR-0025) first. Switch to `fluent-bundle` only if a language needs attributes, functions, or number formatting, and record it as a status update on ADR-0025."
- Add: "A missing voice for a language keeps the current voice and says so. Never go silent."
- Add: "A language change applies live; no restart."
- Add: "The first-run language choice and per-language voices are settings: the four places."
- Add: "Your ADR is 0030."

### W4e: offline intelligence

- Add: "Pin `tokenizers` to 0.23; 1.0 is at release candidate."
- Add: "The no-model LexRank summary first. Translation only if Group 2 has the disk, and only models the owner has approved."
- Add: "Your ADR is 0031."

## What I could not verify

- Whether AccessKit's clipped-node behaviour can be turned off from our side without an upstream change. The mitigation in quick win 10 is a hypothesis to test.
- The body of MathCAT issue #827 beyond its title; the title alone is enough to hold braille back.
- Docker Desktop's memory setting on this machine, and whether any per-container limit exists outside `compose.yaml`. Docker was not started for this review.
- Whether harper-core's embedded dictionary size is acceptable; the size must be measured in W4g.
- Two GitHub searches (Xilem's AccessKit bump and AccessKit text issues) returned nothing, so their state is as `docs/research/wave4.md` recorded it.

## See also

- [Research for Wave 4](wave4.md)
- [Usability pass: the terminal reader and `tw`](usability-terminal.md)
- [Tasks and agent briefs](../history/tasks.md)
- [Roadmap](../roadmap.md)
- [ADR-0025: Define word offline, and the interface's message catalog](../adr/0025-lexicon-and-message-catalog.md)
- [ADR-0027: Xilem GUI](../adr/0027-xilem-gui.md)
- [Documentation index](../README.md)
