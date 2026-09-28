# Wave 4 orchestration plan

Written on Sunday, September 27, 2026, by the second Wave 4 planning review (Fable 5.1), for the orchestrator and the owner. It re-plans Wave 4 from first principles. It changes nothing by itself: the owner adopts it, or parts of it, and the orchestrator applies it to `docs/history/tasks.md`.

What it is built on: the Wave 4 section of `docs/history/tasks.md` (the refined plan of this morning, the folded-in first review, and the lessons list), every Wave 4 brief, the Wave 2 and Wave 3 status lines, `docs/research/wave4.md`, `docs/research/wave4-plan-review.md`, `docs/research/usability-terminal.md`, `docs/roadmap.md`, ADRs 0001 to 0027, `docs/dev/docker.md`, `compose.yaml`, `tools/build-hygiene.ps1`, the root `Cargo.toml`, the wiki (the textweaver hub, the Wave 4 planning page, the deletion incident knowledge base, the lessons page, and the global rules), the git history of `main` (the last 80 commits, and the file lists of the six Wave 3 merges), and measurements taken on this machine today. Crate versions were checked on crates.io today with the tool's default User-Agent; no personal identifier was sent.

Nothing was built, run in Docker, downloaded, or deleted for this review.

## The short version

- **Shape: three sub-waves of three agents, not two groups of five.** Each sub-wave has one goal, one merge point, and one thing for the owner to hear. At most three agents run at once, and at most two of them build heavily at the same moment.
- **The binding limit is memory, not disk.** D: has 872 GB free today, so even five agents fit on disk. But Docker's virtual machine has 31.3 GB in all, and the host has 40 GB free with Docker idle. Five containers at 8 GB each cannot run at once. Two or three can.
- **Cut W4e (offline intelligence) from Wave 4.** Defer GUI edit mode, the LaTeX subset, EML and MHTML, and automated screen-reader tests to Wave 5. Trim W4f to what works without a Mac.
- **Sequence by shared files.** Every Wave 3 agent touched `textweaver-app`; that crate is the hotspot. Each sub-wave has at most one agent that rewrites the app's messages or keys.
- **Measure the lean build once before launch.** Nobody has yet measured a build with the new profiles. The orchestrator's first build under them is the number every later budget uses.

## 1. Diagnosis

### 1.1 What the current plan gets right

- W4h first, small, terminal-only, merged before W4g.
- The GUI split around the owner's listening session.
- Rope changes, TLS changes, and branch pruning removed.
- Per-agent Docker volumes, the shared cache, and the leaner profiles.
- The privacy, deleting, and escape-sequence rules carried into every brief.

### 1.2 Size and coupling

Wave 4 as written is ten agents covering eight unrelated areas: terminal polish, GUI accessibility, GUI editing, speed, math, documents, translations, offline models, platforms, and authoring tools. Waves 2 and 3 show what that costs:

- **Every agent touches `textweaver-app`.** In Wave 3, the six merges changed the app crate 32, 27, 9, 8, 4, and 24 (GUI, through its own crate) files at a time. Even W3c, whose brief said "keep app edits to imports", changed 27 app files. The app is 28,000 lines and every feature ends in it.
- **Every agent touches the same six shared files.** Root `Cargo.toml` and `Cargo.lock` (four of six Wave 3 merges), `THIRD-PARTY-NOTICES.md` (three), `docs/adr/README.md`, `docs/site/*`, `CHANGELOG.md`, and `docs/history/tasks.md` (all six).
- **Merging main into a branch happened three times for W3b, twice for W3d, W3e, and W3f.** Each merge took an agent's time and produced follow-up commits ("notices: regenerate after merging main", "ADR renumbered to 0026"). The ADR number changed twice for W3d and once for W3b.
- **Ten agents means ten merge-main cycles, ten notices regenerations, and ten status lines in one file.** The conflict cost grows with the square of the number of agents in the same crate, not linearly.

### 1.3 Gates

- The current plan has one gate, between Group 1 and Group 2, and it is a disk flush. It is not tied to what the owner needs to hear.
- The owner's NVDA and JAWS session decides two designs (highlight as selection or background; live regions or a UIA notification). W4a1 in Group 1 builds reading aids and edit mode on top of those designs before the session decides them. The first review saw this and trimmed W4a1; the trimmed order still has one agent go on to edit mode inside Group 1 while the session waits on the orchestrator's build.
- Nothing gates on measurements. W4b measures ropes and allocations; nothing says what a good result looks like or who reads it.

### 1.4 Ordering

- W4g depends on W4h's merge, and the plan says so. But W4d (translations) depends on every message being stable, and it is placed in Group 2 next to W4c2 and W4f, which is right, and next to W4a2, whose GUI work adds messages. W4d is the single largest rewrite of the app's strings (about 364 `format!` messages and about 60 announcement kinds). It should run when no other agent is in the app.
- W4c1 (MathCAT) and W4c2 (documents) both touch `textweaver-formats`; the plan separates them by group, which is right, and gives EPUB MathML to W4c1.
- W4b's zip change must land before W4c2; the groups already order that.

### 1.5 Resources: what the numbers say today

Measured this morning (Sunday, September 27, 2026, 10:20 to 10:27):

- D: 871.8 GB free of 1,862 GB. Floor 200 GB.
- `D:\textweaver\target`: orch 35.5 GB (debug 35.5, of which PDB files 20.7, 324 test and tool executables 7.9, rlib and rmeta 5.4), ux1 9.3, release 4.6, debug 3.3, dist-build 1.0, the rest under 1 GB each. Every worktree's own `target` folder is empty (0 GB, twelve worktrees).
- `D:\sccache`: 5.4 GB.
- Docker: images 55.2 GB (47.4 reclaimable, mostly the owner's other projects: never touch), volumes 26.4 GB, build cache 37.4 GB, virtual disk 125.6 GB. `textweaver-dev:latest` is 3.9 GB. The `textweaver-sccache` volume holds 4 MB: the container cache is cold. `tw-target-orch` holds 22 MB.
- Docker's virtual machine: 12 CPUs and 33.6 GB (31.3 GiB) in all. That is the budget every container shares.
- RAM: 63.8 GB, 40.3 GB free, with Docker idle (2.4 GB), two Claude sessions, and the owner's usual programs. So the baseline is about 24 GB in use before any build.

Two conclusions the current plan does not draw:

1. **Five containers at 8 GB cannot run at once.** `compose.yaml` and `docs/dev/docker.md` say "five agents stay near 40 GB, leaving 20 GB free on the 64 GB host". Docker's machine has 31.3 GB. Five containers would exceed it, and on the host 24 GB baseline plus 40 GB of containers leaves 0 GB, not 20. Two 8 GB containers plus the baseline leave 24 GB free; three leave 16 GB, under the floor.
2. **Disk is not the limit this week.** After the cleanup that reclaimed 500 GB, the 200 GB floor is 670 GB away. The plan's "two groups of five to protect the disk" solves last week's problem. The limit is memory, merge conflicts, and the owner's attention.

The 107 GB (W3d) and 60 GB (W3b) figures from Wave 3 were reported by the orchestrator; the folders are gone, so they cannot be re-measured. They were built with full debug info, incremental data, and repeated release builds. The orch folder measured today was built before the lean profile commit (its newest files are from 00:43 on September 27, the same minute as commit a589d2b), so it too is an old-profile number.

### 1.6 Estimate of the lean profiles' effect

This is an estimate from the settings, not a measurement. The orch folder's 35.5 GB breaks down as 20.7 GB of PDB files (full debug info for our crates and for dependencies), 7.9 GB of executables (324 of them: one per test target, example, and tool), 5.4 GB of rlibs and rmeta, and about 1.5 GB of everything else.

Under the new profiles (`debug = "line-tables-only"` for our crates, `debug = false` for dependencies, `incremental = false`):

- PDB files: line tables alone are a small fraction of full debug info. Estimate 3 to 5 GB.
- Executables: on Windows the debug info is in the PDB, so the executables shrink little. Estimate 6 to 7 GB.
- rlibs: dependency objects lose their debug sections. Estimate 3 GB.
- Incremental data: 0 (it was 0 here already).

Estimated full native workspace test build: **13 to 16 GB**, about 60 percent smaller. On Linux the debug info lives inside the binaries, so the old numbers were larger and the saving is larger: estimate **15 to 20 GB** for a full all-features test build in a container, and 8 to 12 GB for one agent's own crates. The GUI crates (Masonry, Vello, wgpu, Parley) add an estimated 5 to 10 GB where they are built. **The orchestrator's first lean build replaces these estimates** (runbook step 2).

## 2. The proposed shape

Three sub-waves. Each has three agents, one merge point, and one thing for the owner to hear. A fourth, light agent may join a sub-wave only when it builds nothing heavy.

### Sub-wave 4a: hear it first

**Goal.** Fix what the owner hears on every run, and prepare the GUI so his one listening session decides both designs.

**Agents.**

- **W4h, terminal polish.** UX-1's findings and quick wins; a "say status" action; spoken key names; announcements queued until the engine is ready. Terminal only.
- **W4s, GUI session prep** (the first three items of the old W4a1). The direct `UiaRaiseNotificationEvent` option, the clipped-options fix in `ChoiceList` and `SettingsGrid`, and the GUI's memory growth attributed. Small commits, `textweaver-xilem` only.
- **W4c1, MathCAT speech.** A new crate behind a feature, speech only, on 0.7.6-rc.3. It touches the math and speech crates, which nobody else in this sub-wave touches.

Why together: W4h and W4s are what the owner must hear before anything else is designed. W4c1 is independent of both and fills the third slot without touching the app's messages (its app edits are one setting and one hook).

**Gates.**

- W4h and W4s merge first, in that order. The orchestrator builds `textweaver.exe` and `textweaver-xilem.exe` in release mode and gives the owner one checklist (terminal first, GUI second).
- **The owner's session 1** (NVDA, then JAWS): the W4h items, W3b's checklist for the Xilem GUI, and the two designs. His findings become fixes in W4a2's brief.
- W4c1 merges when its checks pass; it does not wait for the session.

**Merge point and checks.** Each merge: the native workspace checks, the Docker all-features check (one at a time, by the orchestrator), `keyboard --check`, `deps --check`, `notices --check` when `Cargo.lock` changed, the link and site checks, and the GUI crate built with `-p textweaver-xilem` whenever a GUI branch merges (it is not a default member, so the workspace commands skip it). Then the flush of the merged agents' volumes and folders (runbook step 6).

### Sub-wave 4b: authoring, speed, and the GUI after the session

**Goal.** The terminal-first authoring tools, the speed and memory pass, and the GUI fixes the session asked for.

**Agents.**

- **W4g, authoring extras.** From merged main after W4h: harper grammar, rumdl lint (or our own), syntect highlighting, arboard, Unicode math in the plain view, notes export.
- **W4b, speed and memory.** Text search and segmentation, the zip features in one early commit, allocation cuts with the bench gate, startup time, binary size, and the rope measurements (no rope change).
- **W4a2, the GUI after the session.** The session's fixes first, then the window slide during reading, the reading aids, the essential parity items, ADR-0028. The wxDragon removal is the last commit and only if the owner says so (open question 4).

Why together: three different crates most of the time (app authoring and editor; text, formats, and the app's opening path; xilem). W4g and W4b both touch the app, but W4g in authoring and W4b in opening and playback. Both merge main weekly at least.

**Gates.**

- W4b's zip commit merges to main within its first day, so W4c2 in 4c starts from it.
- **The owner's session 2**, on the GUI only, after W4a2 merges: the window slide, the reading aids, and the session-1 fixes. It decides whether the wx spike goes and whether GUI edit mode gets a slot in 4c (open questions 2 and 4).
- W4b's report gives numbers against a baseline: startup of `tw --version` and `textweaver --help`, peak heap on the 10 MB corpus, and the binary sizes. The orchestrator writes the before-and-after into `docs/roadmap.md`.

**Merge point and checks.** As in 4a, plus a build with the reader's `publish` feature off (W4b and W4g both touch the app), and the bench gate compared with main's artifact.

### Sub-wave 4c: documents, translations, platforms

**Goal.** The formats students bring, the interface in five languages, and the release machinery for `0.1.0-beta.1`.

**Agents.**

- **W4c2, documents.** RTF and ODT natively, DOCX comments and tracked changes, fuzz targets and hostile-input limits. LaTeX subset and EML/MHTML deferred (section 8).
- **W4d, translations.** The five catalogs on W3e's Fluent subset, right-to-left display behind a setting, the first-run language choice, per-language default voices, the pseudo-locale check in CI. It is the only agent in the app's messages.
- **W4f, platforms and CI** (light: it builds little on this machine). The six fuzz targets if not done at launch, the aarch64 AppImage, the Orca check in CI, the release checks, the doc pass, the release notes. The merge-gate rulesets only with the owner's approval (open question 3).

Why together: formats, strings, and workflows are disjoint. W4d needs an app nobody else is changing, and this is the only sub-wave where that is true.

**Gates.**

- W4d merges last in the sub-wave, after W4c2, so its catalog covers W4c2's new messages.
- **The owner's check 3**: the terminal reader in Spanish or French for five minutes with NVDA, and the pseudo-locale run. Nothing to hear from W4c2 and W4f beyond "nothing to hear" lines.
- After the last merge: the Docker disk is compacted with the owner (one administrator prompt), and Wave 5 planning starts from the measurements.

**Merge point and checks.** As in 4b, plus the release workflow run by hand on a branch (W4f) and `cargo xtask release --dry-run`.

### Timeline

Durations, not promises. Sub-wave 4a about two days, 4b about three, 4c about three, plus a day for each of the owner's sessions and the merges. If 4a starts on Monday, September 28, 2026, the wave ends around Wednesday, October 7, 2026. Sub-waves do not overlap: 4b's agents launch only after 4a's last merge and flush, because the flush and the memory check are what keep the floors.

## 3. Dependency map

### 3.1 Crates each agent owns and touches

- **W4h:** owns changes in `textweaver-app` (help, messages, launch, list intro), `textweaver-tui` (`ui.rs` title line, palette prompt), `textweaver-cli` (no-argument hint; `print_all` in `search` and `info`), `textweaver-keymap` (the `say_status` and `repeat_message` actions), `docs/quickstart.md`, `CONTRIBUTING.md`, `docs/dev/testing.md`, `docs/dev/building.md`, `docs/keyboard.md` (regenerated), and the date command in the tasks preamble. Does not touch: `textweaver-xilem`, engines, formats.
- **W4s:** owns `crates/textweaver-xilem` only (`widgets.rs`, `dialog.rs`, `settings_dialog.rs`, `setup.rs`, `log.rs`, `main.rs`). May read `textweaver-engines` and `textweaver-app` to attribute memory; any fix outside xilem is a proposal in the report. ADR-0028 (shared with W4a2; W4s writes the first draft).
- **W4c1:** owns a new `crates/textweaver-mathcat`, the `mathcat` feature and one call site in `textweaver-speech/src/normalize/` (the math transform), one call in `textweaver-math` (the tree to MathML, which exists), the EPUB MathML path in `textweaver-formats` (the EPUB loader only), the `[reading] math_engine` setting (four places), `docs/math.md`, ADR-0029. Does not touch: the BRF writer (braille waits for MathCAT issue #827), `textweaver-app` beyond the setting and the schema entry.
- **W4g:** owns `textweaver-app` authoring (`authoring.rs`, `spell.rs` neighbors, `notes.rs` export, a new `grammar.rs` and `lint.rs`), `textweaver-editor` where the grammar and lint hooks need it, `textweaver-tui` (code highlighting in the view; clipboard fallback), `textweaver-cli` (`tw lint` if added), new workspace dependencies (harper-core, rumdl, syntect, two-face, arboard), `docs/editing.md`, ADR-0032. Does not touch: `opening.rs`, `playback.rs`, `launch`, `textweaver-text`, `textweaver-formats`.
- **W4b:** owns `textweaver-text` (segmentation, find), the zip features in the root `Cargo.toml` and `textweaver-formats`' archive code, `textweaver-app/src/{opening,playback,window}.rs` for measured hotspots, startup in `textweaver-tui/src/setup.rs` (after W4h's launch changes; announcements stay W4h's), `xtask/src/bench.rs`, the new dependencies icu_segmenter, memchr, aho-corasick, and the measurement write-up in `docs/dev/testing.md`. Does not touch: `textweaver-xilem`, the authoring modules, the rope.
- **W4a2:** owns `crates/textweaver-xilem`, `crates/textweaver-xilem/tools/uia-report.ps1`, `xtask/src` GUI packaging, `docs/screenshots/xilem-gui/`, ADR-0028 (final). The wx removal (workspace members, `crates/textweaver-gui`, `.github/workflows/gui.yml`, docs) is the orchestrator's after the owner's yes.
- **W4c2:** owns `textweaver-formats` (new `rtf.rs`, `odt.rs`, the DOCX reader's comments and revisions), `fuzz/`, `fixtures/c2/`, the notes mapping from DOCX comments (through `textweaver-store` `Note` types, no store change), `docs/converting.md`, ADR-0031. Does not touch: the EPUB loader, `textweaver-app` beyond one open-failure message per format.
- **W4d:** owns `textweaver-lexicon/src/i18n`, the catalog files under `crates/textweaver-lexicon/locales/` (or wherever ADR-0025 put them), the message call sites across `textweaver-app` and `textweaver-tui` (the wide edit), the `[interface] language`, `[interface] rtl` and `[speech] voices_by_language` settings (four places each), `textweaver-engines` for the per-language voice choice, `textweaver-cli` (`tw settings language`), the pseudo-locale check in `scripts/dev-check`, `docs/settings.md`, ADR-0030. Does not touch: `.github/` (asks W4f or the orchestrator for the CI line).
- **W4f:** owns `.github/`, `xtask/src/release.rs` and `appimage.rs`, `docker/appimage/`, `scripts/`, `docs/dev/releasing.md`, `CHANGELOG.md`'s release section, the doc pass (a list of files in its brief). Does not touch: any crate's Rust code beyond a compile fix it reports.

### 3.2 Conflict hotspots, from the Wave 3 file lists

Ranked by how many Wave 3 merges changed them:

1. `crates/textweaver-app` (all six merges; up to 32 files). Wave 4 rule: at most one agent per sub-wave rewrites messages or keys there (4a: W4h; 4b: W4g; 4c: W4d). Others make one-line hooks and say so.
2. `docs/history/tasks.md`, `docs/site/*`, `docs/adr/README.md` (all six). Rule: append-only status lines; regenerate the site after merging, never hand-merge it; ADR numbers fixed in the briefs.
3. Root `Cargo.toml` and `Cargo.lock` (four merges). Rule: new dependencies go in a commented block headed with the agent's name; `Cargo.lock` is regenerated after merging main, not merged by hand.
4. `THIRD-PARTY-NOTICES.md` (three merges). Rule: agents do not commit it; the orchestrator runs `cargo xtask notices` after each merge that changed `Cargo.lock`.
5. `crates/textweaver-store` (three merges) and `settings_schema.rs` (1,985 lines): every new setting. Rule: settings are added at the end of their section, one commit each.
6. `crates/textweaver-cli` (four merges): new subcommands each add a line to `main.rs`. Rule: each agent adds its line in alphabetical position.
7. `crates/textweaver-formats`: W4c1 (EPUB) in 4a, W4b (archives) in 4b, W4c2 (new loaders) in 4c. Sequenced, so no overlap.

### 3.3 The order that minimizes conflicts

- 4a: W4h (app, tui, cli, keymap) and W4s (xilem) and W4c1 (new crate, speech, math, formats EPUB): disjoint except one setting.
- 4b: W4g (app authoring, editor, tui view) and W4b (text, formats archives, app opening) and W4a2 (xilem): the app is split by module; the orchestrator checks the two app agents' file lists at their first merge-main.
- 4c: W4c2 (formats) and W4d (app strings, lexicon, store, engines) and W4f (workflows, xtask, docs): disjoint.

Within each sub-wave the merge order is the agent with the widest app changes first (W4h, W4g, W4d), so the others merge main once and adapt.

## 4. Resource budget

All numbers are from today's measurements (section 1.5) and the estimates in section 1.6. "Allowance" is what the orchestrator plans for; the measured number from runbook step 2 replaces the estimate.

### 4.1 Disk per agent

- Native, in the agent's worktree `target`: estimate 13 to 16 GB for a full workspace test build; allowance **20 GB**. Agents build only their own crates natively, so most stay under 10 GB.
- In Docker, the `tw-target-<agent>` volume: estimate 8 to 12 GB for own-crate tests, 15 to 20 GB if the agent runs the all-features workspace tests (it should not; that is the orchestrator's); allowance **25 GB**.
- GUI agents (W4s, W4a2): add 10 GB to each figure; allowance **30 GB native, 35 GB Docker**.
- Per agent allowance, in one number: **45 GB** (GUI agents **65 GB**).

**Status update, Sunday, September 27, 2026 (runbook step 2, measured).** The lean profiles work.
- **Native** (`cargo test --workspace --exclude textweaver-gui --no-run` with `omnivox`, plus `cargo build -p textweaver-xilem`): **16 GB**, down from 35.5 GB with the old profiles. That took 13 minutes cold, 2 minutes warm, and 4 minutes for the GUI.
- **Container** (`--workspace --exclude textweaver-gui --all-features --no-run`, 6 GB and 4 jobs): **8.5 GB** in `tw-target-orch`, in 15 minutes cold. The shared container cache is 0.5 GB after one run.
- **New allowances,** at 1.5 times measured: **24 GB native and 13 GB per container volume** for every agent. GUI agents get the same, since the GUI build is included.

### 4.2 Shared

- `D:\sccache`: 5.4 GB today; cap **30 GB** (one number, the same as the container's; `tasks.md` says 50G and `compose.yaml` 30G: pick 30 and write it in `docs/dev/building.md`).
- Docker `textweaver-sccache` volume: 4 MB today, cold; cap 30 GB.
- The orchestrator's own build output: `target/orch` (35.5 GB, old profile; rebuilt lean at about 15 GB) and `tw-target-orch` (25 GB allowance).
- Docker's other data (images 55 GB, other volumes 26 GB, build cache 37 GB): about 120 GB, unchanged by the wave.

### 4.3 Peak disk

The heaviest moment is sub-wave 4b (two ordinary agents and one GUI agent), with the orchestrator's folders and both caches full:

- Agents: 45 + 45 + 65 = 155 GB.
- Orchestrator: 15 native + 25 Docker = 40 GB.
- Caches: 30 + 30 = 60 GB.
- Existing Docker data: 120 GB.
- Existing native leftovers before flushing: about 55 GB (orch, ux1, release, debug, dist-build).

Peak use about 430 GB. D: free at the peak: about **440 GB**, more than twice the floor. If every estimate is wrong by a factor of two on the agent side, the peak is 585 GB used and 287 GB free: still above the floor. That is the margin that lets three agents run without a mid-wave flush.

Docker's virtual disk grows to its peak and never shrinks: estimate **125 + 3 × 25 + 30 + 25 = 255 GB** at the 4b peak, less if 4a's volumes are removed first (their space is reused). Compact it when it passes 300 GB, and at the end of the wave.

### 4.4 Peak RAM

- Host: 63.8 GB. Floor: 20 GB free. Baseline in use with Docker idle: 24 GB (measured). Available for builds: **about 20 GB**.
- Docker's machine: 31.3 GB in all, shared by every container and the container cache.
- One container at `mem_limit: 8g` with six cargo jobs: 8 GB cap.
- One native workspace test run (the orchestrator's integration check): estimate 8 to 10 GB at the linking peak, less with `CARGO_BUILD_JOBS=6`.
- One Claude agent session: 0.4 to 1.1 GB each (measured).

So, with today's baseline, **two heavy processes fit** (16 GB of 20), and **three do not** at 8 GB each. Two ways to run three agents:

1. Keep `mem_limit: 8g` and let only two of the three build at once (the third writes tests, docs, or research meanwhile). Simple, but agents cannot see each other.
2. Lower each container to **6 GB and four cargo jobs** while three agents run (three containers: 18 GB, within the 20). This needs `compose.yaml` to read `mem_limit: ${TW_MEM:-8g}`, `cpus: ${TW_CPUS:-6}`, and `CARGO_BUILD_JOBS: ${TW_JOBS:-6}` (a one-line change each, by the orchestrator before launch). Rust compiles fine in 6 GB with four jobs; linking the largest test binary is the peak.

This plan uses option 2, plus the rule that **the orchestrator's native integration check runs only when at most one agent container is building** (10 + 6 = 16 GB). The check that makes this measurable: before any heavy process starts, `Get-CimInstance Win32_OperatingSystem` must show free memory of at least 20 GB plus that process's cap (26 GB for a 6 GB container, 30 GB for the native check).

### 4.5 How many agents at once

- Disk: five fit (5 × 45 = 225 GB of 670 available).
- RAM: three containers at 6 GB, or two at 8 GB.
- Merges: three, with one app-message agent per sub-wave.
- The owner's attention: one session per sub-wave, one checklist of at most five items per agent.

**Three at once.** A fourth is allowed only if it runs no container and no native workspace build (W4f qualifies in 4c).

### 4.6 Cache sizing and warming

- The container cache is empty. Three agents starting cold would each compile every dependency, sharing nothing, for about an hour each and 3 × 8 GB of memory. Warm it first: the orchestrator runs one all-features `cargo test --workspace --no-run` in `tw-target-orch` before launching 4a (runbook step 2). That also produces the measured lean-build size.
- Native: `RUSTC_WRAPPER=sccache`, `SCCACHE_DIR=D:\sccache`, `SCCACHE_CACHE_SIZE=30G` for every cargo command. sccache trims itself to the cap.

### 4.7 Flush points

1. **Before launch:** list and remove `target/ux1` (its branch is merged) and `target/orch` (old profile; rebuilt lean in step 2). `build-hygiene.ps1 -Flush -IncludeShared` lists them; `-Apply` removes exactly the listed folders. Routine project build output; no owner prompt needed, but the list is read first.
2. **After each sub-wave's last merge:** for each merged agent, `build-hygiene.ps1 -RemoveVolume tw-target-<agent>` (list), then `-Apply`; then `build-hygiene.ps1 -Flush` (worktree folders of merged branches), then `-Apply`. Volumes and folders of an unmerged branch are never candidates, and the script refuses anything else.
3. **When the virtual disk passes 300 GB, and after 4c:** compact it with Docker Desktop stopped from its menu (never force-quit), following `D:\recovery\work\compact-docker-vhdx.ps1`, with the owner at the administrator prompt.
4. **Never** inside a volume from a container, never `docker system prune` (the owner's other projects share the daemon), never a worktree's folder while its branch is unmerged.

## 5. Research gaps filled

Checked today, Sunday, September 27, 2026, against crates.io (`https://crates.io/api/v1/crates/<name>`) and GitHub, with the tool's default User-Agent.

- **The Docker memory statement is wrong.** `compose.yaml` and `docs/dev/docker.md` say five 8 GB containers leave 20 GB free on the host. Docker's machine has 31.3 GB (`docker info`), and the host baseline is 24 GB. Corrected in section 4.4. The orchestrator should fix the two comments when changing `compose.yaml`.
- **The container cache is cold** (4 MB in `textweaver-sccache`). Nothing in the plan warms it. Runbook step 2 does.
- **Nobody has measured a lean build.** The only full build on disk (`target/orch`, 35.5 GB) predates the profile change by minutes. Step 2 is the measurement.
- **The six fuzz targets are missing from nightly,** confirmed: `fuzz/Cargo.toml` has 15 targets; `nightly.yml` runs 9 (`markdown, html, epub, docx, pdf, settings, keymap, state, frame`). The orchestrator adds `daisy, pptx, sheet, archive, image, web` before launch (a one-line matrix change), or W4f does.
- **`textweaver-xilem` and `textweaver-gui` are not default members.** `cargo test --workspace` skips them. Every integration check for a GUI merge must add `-p textweaver-xilem`; the current checklist does not say so.
- **MathCAT** (`https://crates.io/crates/mathcat`): stable 0.7.5; 0.7.6-rc.3 (August 23, 2026) is the last 0.7.6 build; 0.7.7-alpha.1 (September 23, 2026) is newest. Issue #827 (`https://github.com/daisy/MathCAT/issues/827`, "GetNavigationBraille panics in no-unsafe builds") is open, with a proposed fix described (copy subtrees into the destination document). Speech first; braille after the fix ships in a tagged release. MathCAT is Rust, so it fits the pure-Rust rule.
- **harper-core 2.11.0** (September 16, 2026), **rumdl 0.2.77** (September 23, 2026; three releases in four days before it), **syntect 5.3.0** (September 27, 2025; defaults build the Oniguruma C library, so `default-features = false, features = ["default-fancy"]`), **arboard 3.6.1** (August 23, 2025), **mail-parser 0.11.9** (September 9, 2026): unchanged from the first review.
- **tokenizers**: 1.0.0-rc.2 (September 21, 2026) is newest, 0.23.2 stable. Moot if W4e is deferred; if not, pin 0.23.
- **ropey**: 2.0.0-beta.1 (August 2, 2025) is still the newest; the workspace uses 1.6. No change in Wave 4. **rten 0.26.0**, **kitoken 0.11.0**, **icu_segmenter 2.3.0**, **accesskit 0.25.1** (September 25, 2026), **zip** 8.6.0 stable (9.0.0-pre3 exists; stay on 8.6): unchanged.
- **The workspace already has `zip = { version = "8.6", default-features = false, features = ["deflate"] }`** (root `Cargo.toml` line 128). W4b's zip commit adds the pure-Rust `lzma`, `xz`, `bzip2` (libbz2-rs-sys), and `ppmd` features, and replaces `deflate` with `deflate-flate2-zlib-rs` only if `cargo deny` and the size measurement agree.
- **No Mac is available** (tasks.md, Agent F). W4f's "VoiceOver on macOS" item can only be the CI smoke test and the accessibility dump; a person's VoiceOver test waits for a tester. Guidepup with native apps is unverified (first review); deferred to a Wave 5 spike.
- **W3e's catalog** (`textweaver_lexicon::i18n`, ADR-0025) is the base for W4d, as the first review said; fluent-bundle only if a language needs attributes, functions, or number formatting.
- **What could not be verified:** whether AccessKit's clipped-node behavior can be changed from our side (W4s tests the hypothesis); MathCAT #827's fix status beyond the issue page; Docker Desktop's "Disk usage limit" setting (on drive C, not read); the exact peak memory of a native workspace test run (estimated at 8 to 10 GB; step 2 measures it).

## 6. Revised briefs

Each brief is self-contained. Every agent also reads the shared preamble and the Wave 4 lessons in `docs/history/tasks.md`, `CLAUDE.md`, and `docs/research/wave4.md`. The common rules are repeated once here and apply to every brief.

### Common rules (paste at the top of every brief)

- **Only the owner overrides rules.** If a rule seems not to fit, stop and say so in your report.
- **Privacy.** No personal identifier in any request, header, URL, commit, or file. Neutral User-Agent only.
- **Where you work.** Your own worktree and branch `wave4/<agent>-<topic>`, your own `target`, your own `tw-target-<agent>` volume (`TW_AGENT=<agent>`). Never write outside them. Never write to drive C.
- **Deleting.** Delete nothing but your own build output, and only with PowerShell `Remove-Item -LiteralPath` after listing it. Never from Bash. Never inside a Docker volume. Never chain a delete. Never type an escaped file name. `MSYS_NO_PATHCONV=1` on every Docker command in Git Bash.
- **Building.** `RUSTC_WRAPPER=sccache`, `SCCACHE_DIR=D:\sccache`, `SCCACHE_CACHE_SIZE=30G`. Build only the crates you work on. Never run the all-features workspace tests in the container: that is the orchestrator's. Before any container run or native test of more than one crate, check free memory (`Get-CimInstance Win32_OperatingSystem`, `FreePhysicalMemory`): if it is under 26 GB, wait and try again in ten minutes. Check D: with `powershell -File D:\textweaver\tools\build-hygiene.ps1` once a day; stop and report if it shows under 300 GB free.
- **Checks before reporting,** natively: `cargo fmt --all --check`; clippy `-D warnings` on your crates and on the workspace; your crates' tests and the workspace tests with `--no-fail-fast`; rustdoc with `-D warnings`; `cargo xtask keyboard --check`; `cargo xtask deps --check`; `py -3 tools/check_links.py`; `py -3 tools/gen_site_data.py --check`; a build with `-p textweaver-tui --no-default-features` if you touched the app. In the container: your own crates' tests with `--all-features`. New timing tests run 40 times while another build runs.
- **Code rules.** Announcements through `textweaver_a11y::route`. Writes through the writer thread. Speech on its thread, no async. New settings touch four places (store type and default, export fixture, schema, a reader). New keys pass the conflict, reachability, and WCAG 2.1.4 tests and avoid Windows Terminal's taken keys.
- **Generated docs.** A new or changed setting means running `cargo xtask settings-doc`. A new crate means updating the crate counts in `docs/README.md` and `docs/dev/architecture.md`. `cargo xtask docs --check` finds what was missed.
- **Merging.** Merge `main` into your branch at least every second day and before reporting. Regenerate generated files after merging; never hand-merge them. New dependencies go in a block in the root `Cargo.toml` headed with your agent's name. Do not commit `THIRD-PARTY-NOTICES.md`. `CHANGELOG.md` lines go under a heading with your agent's name.
- **Dates** from the machine: `py -3 -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"`.
- **Report** (plain sentences, headings and lists, no tables): summary in five lines; files changed by crate; the check result lines, native and container; contract change requests or "none"; open issues; measured build output size (native `target` and the volume, from `build-hygiene.ps1`); what the next agent should do first; **a checklist of at most five things for the owner to try with NVDA and JAWS, or "nothing to hear"**. Add one status line under your heading in `docs/history/tasks.md`.

### W4h: terminal polish (sub-wave 4a, P1)

**Branch** `wave4/h-terminal-polish`. **ADR:** none. **Merged first in 4a.**

**Owns:** `crates/textweaver-app/src/{help,launch,lists,list_model,playback}.rs` and the message strings they build; `crates/textweaver-tui/src/{ui,setup}.rs`; `crates/textweaver-cli/src/{main,cmd/search,cmd/info}.rs`; `crates/textweaver-keymap` (two new actions); `docs/quickstart.md`, `CONTRIBUTING.md`, `docs/dev/testing.md`, `docs/dev/building.md`, the date command in the shared preamble of `docs/history/tasks.md`; `docs/keyboard.md` (regenerated).

**Not to touch:** `crates/textweaver-xilem`, engines, formats, `authoring.rs`, `opening.rs`, the settings schema beyond nothing (no new settings).

**Read first:** `docs/research/usability-terminal.md` (all of it), `crates/textweaver-keymap/src/{chord,help}.rs`, `crates/textweaver-app/src/launch` and `help.rs`.

**Deliverables, in order, one commit each:**

1. "Ready" on the title line until the first play; then "Stopped" as now.
2. `tw search --json` and `tw info --json` through `print_all`, with a closed-pipe test.
3. `tw` with no arguments prints a two-line hint and exits 0; `tw --help` keeps the full list.
4. Spoken key names: when a message goes to textweaver's own voice, chords use `KeyChord::spoken` ("Alt period"); the status line keeps the written form. A test that every key named in any spoken message comes from the keymap, not from a fixed string.
5. Two actions with browse keys and palette entries, in both frontends' keymap layers: `say_status` (the last message, then the title line's parts: mode, rate, engine, position) and `repeat_message`. Pick keys through the keymap tests, WCAG 2.1.4, and Windows Terminal's list.
6. Escape in edit mode with nothing playing says "Still editing. <spoken key> finishes."
7. The command palette's opening sentence: "Command. Type part of a name; Tab completes, Up and Down list matches." Drawn label unchanged.
8. A key inside an open list that repeats the list's introduction (title, count, and the keys it takes).
9. Announcements made before the speech engine is ready are queued and said once it reports, in order, after "Opened". Run `cargo xtask listen` afterwards and put the result in the report.
10. Docs: the quick start's `q` line for the classic preset; `py -3` on Windows in `CONTRIBUTING.md`, `docs/dev/testing.md`, `docs/dev/building.md`, and the preamble's date command; `docs/keyboard.md` and the site data regenerated.

**Checks:** the common set. **The owner's checklist (five):** first run with NVDA: the welcome once; "Ready" on the title line; Alt+Shift+A cycled and `say_status`; a question asked while reading heard over the voice; Escape in edit mode.

### W4s: GUI session prep (sub-wave 4a, P1)

**Branch** `wave4/s-gui-session-prep`. **ADR-0028** (first draft: "The Xilem GUI after the owner's session"; W4a2 finishes it).

**Owns:** `crates/textweaver-xilem/src/{widgets,dialog,settings_dialog,setup,log,main}.rs`, `crates/textweaver-xilem/tools/uia-report.ps1`, `docs/adr/0028-xilem-gui-after-the-session.md`.

**Not to touch:** `document.rs` and `runs.rs` (the view's design waits for the session), `textweaver-app`, `textweaver-engines`, `third_party/xilem` (vendored; propose patches in the report), the vendored Parley (no upgrade).

**Read first:** ADR-0027, `docs/research/wave4.md` (the W4a section), `docs/research/wave4-plan-review.md` quick wins 10 and 12, W3b's checklist in `docs/history/tasks.md`.

**Deliverables, in order, small commits:**

1. A direct `UiaRaiseNotificationEvent` announcement option on Windows (`--announce uia|live`, and a `[gui] announce` setting: four places), beside the live-region announcer. Both reachable from the UI Automation report.
2. Clipped options: options scrolled out of view in `ChoiceList` and settings rows below the fold in `SettingsGrid` stay in the accessibility tree with their scrolled bounds. Test the hypothesis that not marking the containers as clipping their children is enough; if AccessKit still drops them, record exactly what it drops for an upstream issue, and put a "read the whole list" fallback (all nodes, no clipping) behind the same option.
3. Memory growth attributed: run the GUI with `--log` and the reader's `publish` feature off, then with each engine crate excluded in turn, on the sample document and the 10-million-character one. Report working set and private bytes for each run. Fix what is inside xilem (renderer buffers, retained text runs); propose, do not make, fixes in engines or the app (likely lazy loading of Piper, Whisper, OCR, and the 9.9 MB lexicon).
4. The UI Automation report extended to the notification option and to a list with 40 options scrolled to the end.
5. ADR-0028 draft: the two announcement paths, the clipped-options finding, the memory table, and the two questions the session decides.

**Checks:** the common set plus `cargo clippy -p textweaver-xilem` and its tests, the UI Automation report in `--background`, screenshots at 100 and 200 percent. Never take the foreground. Never play audio.

**The owner's checklist (five):** with NVDA then JAWS: (1) Space reads; does the caret follow the word? (2) Pause, Stop, and a rate key: each announcement once, first with `--announce live`, then with `--announce uia`; (3) Ctrl+Comma, arrow to a setting below the fold, then object-navigate to it; (4) a bookmarks list with more options than fit; End, then Home; (5) the highlight: selection or background, which reads better?

### W4c1: MathCAT speech (sub-wave 4a, P2)

**Branch** `wave4/c1-mathcat`. **ADR-0029.**

**Owns:** a new crate `crates/textweaver-mathcat` (the engine on one dedicated thread, its state is thread-local; the rules tree embedded through `include-zip`, 9.6 MB, or loaded from the data folder after a yes: say which and why); the `mathcat` feature of `textweaver-speech` and its one call in `normalize/` (the math transform asks MathCAT first, then falls back to `textweaver-math`'s spoken math); the EPUB 3 MathML path in `crates/textweaver-formats` (EPUB loader only: MathML to the `Math` marker; speech order MathCAT, then `alttext`, then `altimg`'s alt); the `[reading] math_engine = "builtin" | "mathcat"` setting (four places); `docs/math.md`; `fixtures/c1/`.

**Not to touch:** the BRF writer (braille waits for MathCAT issue #827 to ship in a tagged release), formula navigation in the app (`math_explore.rs` stays on the built-in tree; propose the MathCAT navigation API in the report), `textweaver-app` beyond the setting and one schema entry.

**Read first:** ADR-0018, `docs/research/wave4.md` (the W4c math section), `docs/research/wave4-plan-review.md` (MathCAT), `crates/textweaver-speech/src/normalize/` math transform, the EPUB loader.

**Deliverables:**

1. The crate on `mathcat = "0.7.6-rc.3"` pinned exactly, `cargo deny` clean (it must not bring a second zip or yaml-rust; check the tree and record it in the ADR).
2. ClearSpeak and SimpleSpeak, and the verbosity mapped to textweaver's three levels; language from the document, English default.
3. Offset maps: MathCAT speaks the whole formula, so the offset map points every spoken word at the formula's source span (ADR-0005 allows a span-level map; say so in the ADR).
4. EPUB 3 MathML read as math.
5. Tests: the Star math vectors through MathCAT with recorded output; a test that the feature off changes nothing; a hostile MathML test; the dedicated thread survives a panic in the engine (reported once, falls back).
6. ADR-0029: version, thread model, rules storage, what waits for #827, the size added to `textweaver.exe`.

**Checks:** the common set, plus the tests with `--features mathcat` natively and in the container, and the binary size before and after (`cargo xtask dist --dry-run` or a release build of `textweaver-tui`, reported in bytes).

**The owner's checklist (three):** open `fixtures/c1/quadratic.md`; read the formula with `math_engine = "builtin"`, then `"mathcat"`; say which wording is clearer at each verbosity.

### W4g: authoring extras (sub-wave 4b, P1)

**Branch** `wave4/g-authoring-extras`, from `main` after W4h merged. **ADR-0032** ("Grammar, lint, highlighting, and clipboard crates").

**Owns:** `crates/textweaver-app/src/{authoring,authoring_state,spell,notes,templates}.rs` and new `grammar.rs`, `lint.rs`; `crates/textweaver-editor` hooks the grammar and lint need; `crates/textweaver-tui/src/view` code highlighting and the clipboard fallback; `crates/textweaver-cli/src/cmd/lint.rs` if a `tw lint` is added; `crates/textweaver-math` for Unicode rendering of a math tree (`to_unicode`); dependency block "W4g" in the root `Cargo.toml`; `docs/editing.md`; `fixtures/g/`.

**Not to touch:** `opening.rs`, `playback.rs`, `window.rs`, `launch`, `textweaver-text`, `textweaver-formats`, `textweaver-xilem`.

**Read first:** `docs/research/wave4.md` ("Also for Wave 4"), `docs/research/wave4-plan-review.md` section 3 (formats and authoring), `docs/editing.md`, P2b's status line.

**Deliverables, each behind a feature and each measured:**

1. **harper-core 2.11.0** behind `grammar`, `thesaurus` off: next and previous grammar problem in edit mode, said as "grammar: <message>, <the words>", a fix offered when harper has one. Binary size before and after, in bytes.
2. **rumdl 0.2.77** (`rumdl_lib`) behind `lint`, pinned exactly, a fixed rule set for a blind author (heading levels, list markers, trailing spaces, unresolved link references, bare URLs). If its size or API churn is too much, write our own lint for those rules on the structure the app already parses, and say which you did and why.
3. **syntect 5.3.0** with `default-features = false, features = ["default-fancy"]` and **two-face 0.5.2** with `syntect-fancy`: code blocks highlighted in the terminal view, colors from the theme's roles, never color alone (the language is named when a block is entered).
4. **arboard 3.6.1** (Linux: `wayland-data-control`) as the clipboard fallback where OSC 52 is unavailable, detection reported once.
5. Unicode math in the plain reading view (x², √2, fractions with the fraction slash), as Star's `mathrender.py` did, behind `[reading] math_display`.
6. Notes export to BibTeX, RIS, and JSON with the citation crate's record types; `tw marks --export`.

**Checks:** the common set, each feature on and off, `cargo deny` after the new crates, `cargo xtask deps --check` (the lean reader links none of these).

**The owner's checklist (four):** in edit mode, Alt+G (or the chosen key) to the next grammar problem; a lint problem; Enter a code block and hear its language; the Unicode math view on `fixtures/g/math.md`.

### W4b: speed and memory (sub-wave 4b, P1)

**Branch** `wave4/b-speed`. **ADR:** none; measurements go as dated status updates on ADR-0002 (rope) and into `docs/dev/testing.md`.

**Owns:** `crates/textweaver-text` (segmentation, find), the archive code in `crates/textweaver-formats` and the zip features in the root `Cargo.toml` (one early commit), `crates/textweaver-app/src/{opening,playback,window}.rs` for measured hotspots only, `crates/textweaver-tui/src/setup.rs` startup (not its announcements), `xtask/src/bench.rs`, the dependency block "W4b" (icu_segmenter, memchr, aho-corasick), `docs/dev/testing.md` (the measurement section).

**Not to touch:** `textweaver-xilem`, the authoring modules, `launch` and its announcement order (W4h's), the rope, TLS.

**Read first:** `docs/research/wave4.md` (W4b), `docs/research/wave4-plan-review.md` (speed), `xtask/src/bench.rs`, P2d's and P2a's status lines (the bench gate and its numbers).

**Deliverables, in order:**

1. **Day one:** the zip feature commit: `default-features = false` with `deflate-flate2-zlib-rs`, `lzma`, `xz`, `bzip2`, `ppmd` (all pure Rust; `bzip2-rs` is C, avoid it), `cargo deny` clean; ask the orchestrator to merge it to main at once, and say so in `docs/history/tasks.md`.
2. Baseline numbers on main before any change, with the exact commands: `tw --version`, `tw info` on the 10 MB corpus, `textweaver --help`, first speech on 10 MB, peak heap and allocation count from `cargo xtask bench --quick`, `tw.exe` and `textweaver.exe` sizes in bytes (release).
3. `memchr::memmem` for literal find; aho-corasick for many-term highlighting; `icu_segmenter` measured against `unicode-segmentation` for words and sentences on the corpus (our abbreviation handling stays); adopt it only if it is faster and the data size is acceptable, and say the numbers either way.
4. Allocation and peak-memory cuts guided by the bench gate, in the hot paths the numbers point at.
5. Startup: what runs before the first announcement, and what can move after it or become lazy (engines, lexicon, fonts).
6. Binary size with the `publish` feature on and off, and one concrete cut.
7. The rope measurement: ropey 2.0.0-beta.1 and crop 0.4.3 on the edit traces, numbers only, in a status update on ADR-0002. No rope change.

**Checks:** the common set; the bench gate against main's artifact; new timing tests 40 times under load; the `publish`-off build.

**The owner's checklist:** nothing to hear, unless step 5 changed the order of startup messages; then: "Opened" and the first-run welcome once, with NVDA.

### W4a2: the GUI after the session (sub-wave 4b, P1)

**Branch** `wave4/a2-gui-after-session`, from `main` after W4s merged and the owner's session 1 is written up. **ADR-0028** (finish W4s's draft).

**Owns:** `crates/textweaver-xilem` (all of it now, including `document.rs` and `runs.rs`), its UI Automation report, `xtask/src` GUI packaging, `docs/screenshots/xilem-gui/`, `docs/gui.md` (create it if missing: how to start, the keys, the settings).

**Not to touch:** the vendored Parley (no upgrade; edit mode is Wave 5), `textweaver-app` beyond one-line hooks, `.github/` and the workspace members (the orchestrator removes the wx spike after the owner's yes), `textweaver-gui`.

**Read first:** the write-up of the owner's session 1, ADR-0027, ADR-0028 draft, ADR-0022 (reading aids), `docs/research/wave4.md` (RSVP overlays, announcements on Windows).

**Deliverables, in order:**

1. The session's fixes, one commit each, with the decision on highlight and announcements recorded in ADR-0028.
2. **A window slide during reading keeps the screen reader's place:** when `DocWindow` slides or recentres, the caret stays in a valid range, the selection is re-sent, and no announcement is lost. Checked in the UI Automation report (read past a window edge in `--background`). First on the owner's second checklist.
3. Reading aids: text spacing, the reading ruler and current-line band, bionic reading, difficult words (from `textweaver-aids`), and an RSVP panel that never covers the caret. The RSVP word node is hidden and never live and never focused; a quiet status node ("RSVP paused, word 120 of 900") with `Live::Off`. A test for both.
4. Parity essentials with the terminal reader: outline, notes list, the access modes, tables and links by key. Parity means Star's features, not Star's bugs (the Phase 0 inventory lists the bugs). Citations while writing, export, preview, and spell check wait for edit mode (Wave 5).
5. A test that the first nine themes keep their cycle order.
6. Packaging: `cargo xtask gui-dist` still builds; screenshots at 100 and 200 percent.

**Checks:** the common set plus the GUI crate's clippy and tests, the UI Automation report, the AT-SPI dump if Docker has Xvfb (else CI), never the foreground, never audio.

**The owner's checklist (five):** (1) read past the window edge on a long document: does NVDA keep its place? (2) RSVP on: is anything spoken by NVDA? (it must not be); (3) the reading ruler with a theme change; (4) Alt+O outline and Enter; (5) the notes list.

**Update for sub-wave 4c (Sunday, September 27, 2026, 11:23 PM).** These changes override the three briefs below. They follow the owner's answers and the Cloud Agent's reservations in `docs/history/reservations.md`:
- **W4c2 and fuzzing.** The Cloud Agent's task 1 owns `fuzz/Cargo.toml`, `fuzz/src/lib.rs`, `fuzz/README.md` and the nightly matrix while its pull request is open. W4c2 writes its three fuzz targets (`rtf`, `odt`, `docx_revisions`) and their seeds last:
  - If the Cloud Agent's pull request has merged, W4c2 merges main first, then appends its targets.
  - If the pull request is still open, W4c2 puts the targets and their `[[bin]]` lines in its report for the orchestrator to add after the merge.
- **W4f: deliverable 1 is dropped.** The six W3d targets were added at launch (`132382f`), and the nightly matrix is the Cloud Agent's while task 1 is open.
- **W4f: two steps in `ci.yml`'s docs job are the Cloud Agent's** (task 2). W4f leaves those steps alone.
- **W4f: deliverable 6.** It writes release notes for the next alpha (`0.1.0-alpha.4`), not a beta. The owner keeps iterating alphas until a feature-complete final alpha.
- **W4f: deliverable 7 is dropped.** the owner decided on no merge gate for now, so there are no rulesets and no auto-merge.
- **W4f: the doc pass skips files the Cloud Agent's task 2 owns** while it is open:
  - `docs/settings-reference.md`;
  - the Decisions list, crate count and Roadmap line in `docs/README.md`;
  - the index lines in `docs/adr/README.md`;
  - the status block in `docs/roadmap.md`;
  - the crate count in `docs/dev/architecture.md`.
- **W4d: one extra check.** It confirms Spanish and French with the voices the owner has. The built-in engines come first. Eloquence is his preference, through the ECI host, with Voxin only.

### W4c2: documents (sub-wave 4c, P2)

**Branch** `wave4/c2-documents`, from `main` after W4b's zip commit and 4b's merges. **ADR-0031** ("Native RTF, ODT, and Word revisions").

**Owns:** `crates/textweaver-formats/src/{rtf,odt}.rs` (new), the DOCX reader's comments and revisions, `fuzz/` (three new targets: `rtf`, `odt`, `docx_revisions`), `fixtures/c2/`, `docs/converting.md`, one open-failure message per new format in the app.

**Not to touch:** the EPUB loader (W4c1's), `textweaver-app` beyond those messages, `textweaver-store` (use the existing `Note` and `Highlight` types), `.github/` (give W4f the nightly matrix lines).

**Read first:** `docs/research/wave4.md` (RTF, ODT, DOCX, hostile input), ADR-0026, P1d's status line (the walker limits and the Pandoc path), the DOCX reader.

**Deliverables:**

1. **RTF:** our own iterative parser with an explicit group stack; caps on depth, `\bin` length, and `\uc` skip; `\ansicpg` and `\fcharset` through encoding_rs; `\u`; `\*` destinations skipped; tables (`\trowd`, `\cell`, `\row`); `\footnote`; fields; headings from `\s` styles when the stylesheet names them. Pandoc no longer needed for RTF.
2. **ODT:** roxmltree plus zip on ODF 1.4: headings, lists with levels, tables, images' alt text, footnotes, tracked changes as revisions.
3. **DOCX comments and tracked changes:** `w:comment*` and `w15:commentEx` (replies, resolved) to notes anchored at the range; `w:ins`, `w:del` (`w:delText`), `w:moveFrom`, `w:moveTo` as revisions with author and date, read as "inserted by <author>" and "deleted by <author>" at high verbosity and skipped at low, behind `[reading] revisions`.
4. Hostile-input limits per P1d's patterns (nesting, node count, decoded size, entry count and ratio for zip), and a fuzz target per parser with a seed corpus.
5. `tw convert` and `tw text` on every new format's fixture; `docs/converting.md` updated with the file list.

**Checks:** the common set, plus `cargo fuzz build` for the new targets and ten minutes of each locally (nightly Rust in the container, not natively: the toolchain lives on D:).

**The owner's checklist (two):** open `fixtures/c2/handout.rtf` and `fixtures/c2/notes.odt` in the reader; a DOCX with a comment: is the note announced while reading?

### W4d: translations (sub-wave 4c, P3)

**Branch** `wave4/d-translations`, from `main` after W4c2 merged (it merges last). **ADR-0030.**

**Owns:** `crates/textweaver-lexicon/src/i18n` and the catalog files; every message call site in `crates/textweaver-app` and `crates/textweaver-tui` (the wide edit, done in one pass per crate, one commit per module); `[interface] language`, `[interface] rtl`, and `[speech] voices_by_language` (four places each); `crates/textweaver-engines` (the default voice per language); `crates/textweaver-cli/src/cmd/settings.rs` (`tw settings language`); the pseudo-locale check in `scripts/dev-check.sh` and `.ps1`; `docs/settings.md`; the "Language" section of `docs/screen-readers.md`.

**Not to touch:** `.github/` (give W4f the CI line), `textweaver-xilem` (the GUI takes the catalog through the app; drawn labels follow later), formats.

**Read first:** ADR-0025 and `textweaver_lexicon::i18n`, `docs/research/wave4.md` (W4d), `docs/research/wave4-plan-review.md` (translations and the Star lessons: never go silent, live changes, four places), Star's catalogs at `D:\star\star\locale` (read only).

**Deliverables:**

1. Extend W3e's Fluent-subset catalog first. Switch to `fluent-bundle` only if a language needs attributes, functions, or number formatting; record it as a status update on ADR-0025.
2. Extract the strings: about 60 announcement kinds and about 364 `format!` messages become catalog ids with variables; English complete; a checker that every id used in code exists in `en` and every `en` id is used.
3. Spanish, French, German, Portuguese, and Arabic: script-convert Star's overlapping strings (flat JSON, `{name}` placeholders, no plurals: add the plural forms by hand where the id has a count), then translate the rest. Each language's file is valid Fluent.
4. Right-to-left: text stays in logical order in the model, in speech, and in AccessKit; display reordering with `unicode-bidi` behind `[interface] rtl = "auto" | "on" | "off"`, off where the terminal reorders (VTE, Konsole, mlterm, macOS Terminal), and documented as unsupported on Windows Terminal.
5. A first-run language choice (a list; the system locale first), and per-language default voices: **if no voice exists for the language, keep the current voice and say so; never go silent.** A language change applies live (the catalog is a value; swap it and re-announce the title line).
6. Pseudo-locale (`en-XA`) and `ar-XB` runs in `dev-check`: a snapshot fails when plain English appears outside brackets.

**Checks:** the common set; the four-places tests for the three settings; `cargo xtask keyboard --check` (help strings come from the catalog now).

**The owner's checklist (four):** first run with `--home` fresh: the language list; Spanish for five minutes with NVDA (Eloquence has no Spanish voice: the "kept the current voice" message); Alt+Shift+A in French; `en-XA` for one minute to hear the bracketed strings.

### W4f: platforms and CI (sub-wave 4c, P3, light)

**Branch** `wave4/f-platforms`. **ADR:** none.

**Owns:** `.github/` (every workflow), `xtask/src/{release,appimage}.rs`, `docker/appimage/`, `scripts/`, `docs/dev/releasing.md`, the "Unreleased" section of `CHANGELOG.md`, and the doc pass over these files against the code: `README.md`, `docs/quickstart.md`, `docs/install.md`, `docs/reading.md`, `docs/editing.md`, `docs/notes.md`, `docs/library.md`, `docs/speech.md`, `docs/math.md`, `docs/citations.md`, `docs/converting.md`, `docs/screen-readers.md`, `docs/troubleshooting.md`, `docs/settings.md`, `docs/dev/*.md`.

**Not to touch:** any crate's Rust code (report compile fixes), repository settings (open question 3), branches (no pruning).

**Read first:** `docs/research/wave4.md` (W4f), P2d's status line, `docs/dev/releasing.md`, the nightly workflow.

**Deliverables:**

1. The six fuzz targets in the nightly matrix with corpus lines, if the orchestrator did not add them at launch; plus W4c2's three.
2. The aarch64 AppImage on `ubuntu-22.04-arm` with appimagetool and the type2 runtime for aarch64, checked against their published digests, tested in the distro matrix under emulation or on the runner.
3. The Orca check: the AT-SPI dump under Xvfb in `gui.yml` for `textweaver-xilem`; the macOS smoke test kept. VoiceOver by a person is out of scope (no Mac).
4. `cargo xtask release` checks that the listening checklist in `docs/dev/releasing.md` is dated; the changelog grouped by area from the agents' headings.
5. The doc pass, file by file against `--help` output and the code, with one commit per file and a list in the report of every passage changed.
6. Release notes for `0.1.0-beta.1` in `CHANGELOG.md`'s "Unreleased" section, for the owner to release when he says so.
7. If the owner approves (open question 3): the rulesets (pull request required, 0 approvals, required status checks, force pushes blocked), auto-merge on, and `gh pr merge --auto --squash --delete-branch` documented in `CONTRIBUTING.md`.

**Checks:** actionlint, shellcheck, the PowerShell lint, the link and site checks, a manual run of `release.yml` on the branch (`workflow_dispatch`, no tag).

**The owner's checklist:** nothing to hear.

## 7. Execution runbook

Every step names its checks. "Free memory" means `FreePhysicalMemory` from `Get-CimInstance Win32_OperatingSystem`, in GB. "D: free" means the first line of `powershell -File D:\textweaver\tools\build-hygiene.ps1`.

### Step 0: fix the plan files (half an hour)

1. Apply the brief changes above to `docs/history/tasks.md`: the three sub-waves, the agents' names, the ADR numbers (0028 W4s and W4a2, 0029 W4c1, 0030 W4d, 0031 W4c2, 0032 W4g), the common rules.
2. `compose.yaml`: `mem_limit: ${TW_MEM:-8g}`, `cpus: ${TW_CPUS:-6}`, `CARGO_BUILD_JOBS: ${TW_JOBS:-6}`; fix the comment about five agents and 40 GB. `docs/dev/docker.md`: the same correction; the cache is 30 GB in both places; `docs/history/tasks.md` line "SCCACHE_CACHE_SIZE=50G" becomes 30G.
3. `.github/workflows/nightly.yml`: add `daisy, pptx, sheet, archive, image, web` to the fuzz matrix, with corpus lines.
4. Commit on main. Tell the owner the plan is in place and ask the open questions (section 9).

### Step 1: flush before launch

1. `powershell -File D:\textweaver\tools\build-hygiene.ps1 -Flush -IncludeShared`. Read the list: expected `target\ux1` (merged) and the shared folders `debug`, `release`, `doc`, `tmp`, cross targets. `target\orch` is not a candidate by name; remove it separately after reading its path: `Remove-Item -LiteralPath D:\textweaver\target\orch -Recurse` on its own line, after `Get-ChildItem -LiteralPath D:\textweaver\target\orch -Directory` shows only `debug`, `doc`, `tmp`.
2. `-Apply` for the listed folders. Expected gain: about 55 GB. D: free about 925 GB.
3. Check: `docker ps` shows no textweaver container; `docker volume ls` shows `tw-target-orch` only among `tw-target-*`.

### Step 2: warm the cache and measure the lean build (about one hour of machine time)

1. Free memory must be at least 30 GB. Docker Desktop running (restarted from its menu since Wave 3, per the rule; ask the owner if unsure).
2. Native, from `D:\textweaver` on main, with the sccache variables: `cargo test --workspace --no-run --features textweaver-speech/omnivox` then `cargo build -p textweaver-xilem`. Record the time and `target\debug`'s size from `build-hygiene.ps1`. **This is the measured lean native size**; write it into section 4 of this document as a status update and into `docs/dev/building.md`.
3. In the container, `TW_AGENT=orch MSYS_NO_PATHCONV=1 docker compose -p textweaver run --rm -T dev cargo test --workspace --all-features --no-run`. Record the time and `tw-target-orch`'s size, and `textweaver-sccache`'s size. **This is the measured lean container size.**
4. Replace the allowances in section 4.1 with 1.5 times the measured numbers. If the measured container size is over 40 GB, stop and re-plan the volume allowance before launching three agents.
5. Docker's virtual disk size after this step is the new baseline for the 300 GB compaction trigger.

### Step 3: launch sub-wave 4a

1. Checks: D: free at least 400 GB (expected about 850); free memory at least 32 GB (three agents at 6 GB); `docker system df` recorded.
2. Create three worktrees from main: `wave4/h-terminal-polish`, `wave4/s-gui-session-prep`, `wave4/c1-mathcat`. Set `TW_AGENT=w4h`, `w4s`, `w4c1`; `TW_MEM=6g`, `TW_CPUS=4`, `TW_JOBS=4`.
3. Give each agent its brief from section 6, with the common rules pasted in.
4. Twice a day: `build-hygiene.ps1` and free memory, into a one-line log in `docs/history/tasks.md` under the sub-wave heading (date and time from the machine, D: free, memory free, volumes' sizes). Tell the owner only when a number crosses a threshold (section 7, "if a floor is reached").

### Step 4: integrate, in the order W4h, W4s, W4c1

For each agent that reports:

1. Read the report and its status line. Check the report's measured build sizes against the allowance.
2. Free memory at least 30 GB and at most one agent container building (ask the other agents to pause builds for the check if needed, or wait).
3. Merge main into the branch (the agent did; verify), then run on the branch, natively: fmt, workspace clippy, workspace tests with `--no-fail-fast`, rustdoc, `keyboard --check`, `deps --check`, the link and site checks, `notices --check` if `Cargo.lock` changed, the `publish`-off build if the app changed, `-p textweaver-xilem` clippy and tests if the GUI changed.
4. In the container, one at a time: the all-features clippy and tests. Never two Docker checks at once.
5. Merge to main, regenerate the site and the notices, commit. Push only if the owner's rules for pushes allow it (they did for Wave 3).
6. Flush that agent: `build-hygiene.ps1 -RemoveVolume tw-target-<agent>`, read, then `-Apply`; `build-hygiene.ps1 -Flush`, read (the worktree's `target` is a candidate once its branch is merged), then `-Apply`. Record D: free after.
7. After W4h and W4s: build the release binaries (`cargo build --release -p textweaver-tui -p textweaver-cli` and `-p textweaver-xilem` with the sccache variables) and hand the owner the combined checklist: W4h's five items, then W4s's five, then W3b's ten. Say which binary and which command line for each.

### Step 5: the owner's session 1

1. The owner tests when he chooses. The orchestrator writes his findings into `docs/history/tasks.md` under "Session 1", one line each: what he heard, what he expected, which design he picked (highlight; announcements).
2. The findings become the first items of W4a2's brief. Nothing in 4b launches before the write-up exists, except W4b and W4g, which do not depend on it: they may start as soon as W4h and W4c1 are merged and flushed, to keep two agents busy.

### Step 6: sub-wave 4b

1. Same checks as step 3. W4g and W4b start from main after W4h's merge; W4a2 after the session write-up.
2. W4b's zip commit: merge it to main as its own integration (steps 4.2 to 4.5, small) within a day of its report.
3. Integrate in the order W4g, W4b, W4a2 (or W4b first if it reports first: W4g then merges main once more).
4. After W4a2: release build of the GUI, the owner's checklist (W4a2's five). **Session 2.** Write up. Ask the owner about the wx spike (question 4) and GUI edit mode (question 2).
5. Flush each merged agent as in step 4.6. Check the Docker virtual disk size; compact with the owner if over 300 GB.

### Step 7: sub-wave 4c

1. Same checks. W4c2 and W4f start from main after 4b's last merge. W4d starts at the same time but merges last; tell it to merge main after W4c2 lands.
2. Integrate W4c2, then W4f (workflows: run `ci.yml` and the manual release workflow once on the branch before merging), then W4d (its catalog must cover W4c2's messages; the checker fails otherwise).
3. The owner's check 3 (W4d's four items).
4. Flush everything merged. Compact Docker's disk with the owner. Record the final numbers in `docs/history/tasks.md` and hand the wiki page `meta/textweaver-releases/textweaver Wave 4 planning.md` its results section (the orchestrator edits the wiki; agents never do).
5. Wave 5 planning starts from: the measured build sizes, W4b's numbers, the two sessions' write-ups, and section 8's deferred list.

### If a floor is reached

- **D: under 300 GB free:** no new heavy process. Flush every merged agent (step 4.6) and the shared native folders (`-Flush -IncludeShared`). If still under 300 GB, ask the running agents to stop builds, and measure each volume; the largest unmerged one is paused (its agent works on tests and docs) until an integration frees space. Under 250 GB: Docker's disk is compacted with the owner before anything else runs. Under 200 GB is the failure condition: every build stops, and the owner is told in one line what happened and what is being removed.
- **Free memory under 20 GB:** whoever sees it (the orchestrator's twice-daily check, or an agent's pre-build check) stops starting anything. Running builds finish. The orchestrator finds the cause (`Get-Process` sorted by working set): a container over its cap cannot happen (Docker enforces it), so it is a native process, usually a workspace test link or a runaway `cargo doc`. It is allowed to finish; the next heavy process waits until 26 GB is free.
- **Docker's virtual disk over 300 GB:** compact at the next merge point, not mid-build.
- **A container fails with an out-of-memory kill:** lower `TW_JOBS` to 2 for that agent, not the cap; if it still fails, that agent runs its container builds only when no other container is building.

### Keeping the owner informed

- One line per merge in the chat: agent, what merged, test count, D: free, memory free, what he can test now.
- One message per sub-wave end: the sub-wave's goal met or not, the numbers, the next gate, and his checklist as a numbered list, at most five items per agent, in the order to try them.
- No message for routine checks that pass. A message the moment a floor threshold is crossed, with the cause and the remedy.
- Everything he decides is written into `docs/history/tasks.md` with the date and weekday from the machine, and mirrored to the wiki by the orchestrator.

## 8. What to cut or defer

- **W4e, offline intelligence: defer to Wave 5.** It is the furthest from the goal, it is the only agent that downloads models (110 MB per language pair, each needing the owner's approval), it adds a third tokenizer, and its no-model half (LexRank) is a half-day item that can join any later agent. Nothing in Wave 4 depends on it.
- **GUI edit mode: defer to Wave 5** unless session 2 is clean and the owner wants it in 4c (question 2). It is the riskiest GUI item (no Rust app has good screen-reader editing of multi-line text yet), it needs the highlight design settled, and it competes with W4a2 for the same crate.
- **W4c2's LaTeX subset and EML/MHTML: defer to Wave 5.** Course notes as LaTeX and email archives are rarer for students than RTF and ODT handouts and Word comments. Both need their own parsers and fuzz targets; W4c2 is large enough without them.
- **W4f's Guidepup automated screen-reader tests: defer to a Wave 5 spike.** Its use with native apps is unverified, and its setup action was archived. The owner's own sessions are the test this wave.
- **W4f's VoiceOver by a person: out of scope.** No Mac. Keep the CI smoke test and the accessibility dump.
- **MathCAT braille and formula navigation through MathCAT: wait for issue #827** and a tagged release; speech now.
- **The rope change and TLS: already removed. Keep them removed.**
- **W4a's "citations while writing, export and preview, spell check" in the GUI: wait for edit mode** (Wave 5). Reading-side parity (outline, notes, access modes, tables, links) stays in W4a2.
- **The wxDragon removal: only after session 2 and the owner's yes** (question 4). Until then it costs nothing: it is not a default member and CI builds it on its own job.
- **Not cut, but shrunk:** W4b keeps measurements and the pure-Rust zip change, drops nothing else; W4g keeps all six items because each is small and behind a feature.

## 9. Open questions for the owner

1. **Defer W4e (offline translation and summaries) to Wave 5?** If not, it takes the fourth slot in sub-wave 4c and needs your approval for each model download.
2. **GUI edit mode: Wave 5, or a slot in sub-wave 4c if session 2 goes well?** It would replace W4f's slot in the machine's budget (W4f would then run as a fourth, light agent).
3. **May W4f change the repository's settings** (rulesets: pull request required, required status checks, force pushes blocked; auto-merge on)? That is the merge gate from the roadmap. It changes how every merge after it is made.
4. **Remove the wxDragon spike after session 2 passes,** or keep it as a fallback through Wave 4?
5. **Unbound keys in the reader: silent, as NVDA's browse mode is, or a short tone at high verbosity?** (UX-1's item 7.) W4h implements whichever you choose.

## The owner's answers (Sunday, September 27, 2026)

1. **W4e is deferred to Wave 5.**
2. **GUI edit mode stays in Wave 4,** as a new agent, W4a3, in sub-wave 4c after session 2 (ADR 0033). It takes 4c's heavy slot, and W4f runs as a light fourth agent. Its brief is in `docs/history/tasks.md`, under "Adopted plan".
3. **No merge gate for now.** W4f changes no repository settings.
4. **The wxDragon spike is removed** once session 2 passes. The orchestrator does it as its own commit, after showing the file list.
5. **Unbound keys are silent,** as in NVDA's browse mode.

## See also

- [Wave 4 plan review](wave4-plan-review.md): the first review, folded into the plan.
- [Research for Wave 4](wave4.md): versions, licences, and APIs.
- [Usability pass: the terminal reader and `tw`](usability-terminal.md): W4h's source.
- [Tasks and agent briefs](../history/tasks.md): the plan this document proposes to change.
- [Docker development container](../dev/docker.md): volumes, limits, and the flush commands.
- [Roadmap](../roadmap.md)
- [Documentation index](../README.md)
