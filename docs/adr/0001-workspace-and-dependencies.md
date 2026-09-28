# ADR-0001: Workspace layout and dependency policy

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): The workspace now has 28 crates and `xtask`; [the architecture guide](../dev/architecture.md) maps them. Two dependency rules changed: `textweaver-speech` also depends on `textweaver-math`, because math is spoken inside the normalization pipeline (ADR-0018), and `textweaver-store` depends on `textweaver-aids` (and through it on `textweaver-text` and `textweaver-fonts`) for the `[reading_aids]` settings types. Speech still never receives a `Document`. `espeakng-sys` was replaced by hand-written declarations, so the espeak feature needs no clang. `live-region` is used by the GUI spike (ADR-0014), not yet by `textweaver-a11y`. `ureq` serves citation lookups (ADR-0019) and `rayon` bulk conversion (ADR-0016). `pdf-extract` and `pdfium-render` are still listed in the workspace table but no crate uses them (ADR-0010).
- Status update (Saturday, September 26, 2026, Phases 1 and 2): seven unused workspace dependencies were removed, among them `pdf-extract` and `pdfium-render` (Agent P1c). `rust-version` is 1.92, because krilla needs it; the toolchain stays pinned to 1.96. `cargo xtask deps --check` checks the dependency direction in CI: it refuses forbidden edges, and reports two as allowed for now, store on aids and the reader reaching the conversion and citation crates through the app, which exports and inserts citations since Agent P2b. espeak-ng is loaded at run time with `libloading`, not linked (Agent P2d). Wave 3 plans to remove both exceptions (Agent W3c), and prefers pure-Rust, in-process crates over subprocesses and C or C++ libraries, recording each bold choice and its fallback in an ADR.
- Status update (Saturday, September 26, 2026, Wave 3, Agent W3c): both exceptions are gone. Store depends only on core again: the `[reading_aids]` settings types are store's own plain data (`textweaver_store::reading_aids`), and `textweaver-aids` converts them. In-reader export, preview, and citations are `textweaver-app`'s `publish` feature, on by default and in releases; the workspace table declares the app with default features off, and the reader forwards `publish` as its own default. `cargo xtask deps --check` resolves features and refuses any edge the reader reaches without its default features. A new crate, `textweaver-engines`, holds the backend registry, so the app no longer depends on each engine crate. There is one notes model (the store's), and font resolution lives in `textweaver-fonts`. The workspace has 29 crates and `xtask`.

## Context

textweaver reimplements Star (Python, 45K lines) in Rust. Four agents build it in parallel, so the crate boundaries have to let each agent compile and test alone against a stable contract. Star's history shows the cost of the opposite: 40+ GUI and TUI mixins sharing state, and a long tail of optional dependencies.

## Decision

**One Cargo workspace, one crate per job**, with a strict dependency direction:

```
core ← text, speech, keymap, a11y, store, editor
text ← formats
all of the above ← app ← tui, cli
```

- `textweaver-core` holds only leaf types shared across crates: `CharPos`, `CharRange`, `Direction`, `Bias`, `Unit`, `MarkerKind`, `OffsetMap`, `Edit`, `EditOutcome`, `Rate`, `Pitch`, `Volume`, `Utterance`, and the preference enums (`Verbosity`, `PunctuationLevel`, `HighlightGranularity`, `CapsIndication`). The orchestrator owns it; agents request changes.
- `speech` depends only on `core`. It receives `Utterance`s, never a `Document`.
- `store` depends only on `core`. Keymap overrides are stored as strings and interpreted by `keymap`.
- `a11y` depends only on `core`. The speech announcer takes a callback, so it does not depend on `speech`.
- `editor` works on a `ropey::Rope` plus `core::Edit`; `text::Document::apply` is the document side of the same edit.

**Every third-party crate is declared once** in `[workspace.dependencies]` in the root `Cargo.toml`, and crates opt in with `name.workspace = true`. That table is the approved list. Adding to it is an orchestrator decision; agents ask in their report. At Phase 0 it holds: `ropey` 1.6, `unicode-segmentation`, `regex`, `pulldown-cmark`, `scraper`, `zip`, `roxmltree`, `serde`, `serde_json`, `toml`, `directories`, `thiserror`, `anyhow`, `bitflags`, `log`, `clap`, `ratatui`, `crossterm`, `espeakng-sys`, `speech-dispatcher`, `tts`, `rodio`, `proptest`, `insta`, `tempfile`.

**Rules:**
- No async runtime in speech: `textweaver-speech`, the engine backends, and anything the speech thread calls use threads and `std::sync::mpsc` channels, because engines have thread affinity (SAPI and WinRT COM apartments, Apple engines on the main run loop, espeak-ng's process-wide state; ADR-0003). Other crates may use an async runtime where it clearly helps, such as network-heavy work, as long as it stays out of the speech path (clarified by the owner, 2026-09-25).
- **Performance is a requirement** (the owner, 2026-09-25): measure hot paths with benchmarks before and after optimizing; use `rayon` for CPU-parallel batch work (conversion, indexing, library scans); an async runtime (`tokio`) for concurrent I/O outside the speech path (network lookups, many-file reads, watching); streaming or memory-mapped reading (`memmap2`) for large files instead of reading them whole; caches keyed by content or by path, size, and modification time; allocation-light parsers that borrow from their input. Report timings and memory for anything that handles whole documents or folders.
- `thiserror` in libraries, `anyhow` only in binaries (`tui`, `cli`, `xtask`).
- Engines and heavy formats behind cargo features, off by default: `textweaver-speech/{espeak, omnivox, speechd, tts-crate}`, `textweaver-formats/{paperback, pandoc}`, `textweaver-a11y/live-region`.
- Workspace lints: `unsafe_code = "deny"` (the espeak backend's FFI module opts out with `#[allow(unsafe_code)]` and a `// SAFETY:` comment on every block), `missing_docs = "warn"`, clippy `all = "warn"`. CI and `-D warnings` make all of them errors.
- Edition 2024. Toolchain pinned in `rust-toolchain.toml` to 1.96 so Windows, CI, and the Docker image agree; `rust-version = "1.92"` states the true minimum (raised from 1.85 at Integration 1: `libloading` needs 1.88, `File::try_lock` 1.89; raised to 1.92 on 2026-09-26 because krilla 0.8, the PDF writer, declares 1.92).

**Environments.** Native Windows (the owner's machine) and Linux in Docker (`docker/Dockerfile`, `compose.yaml`, see `docs/dev/docker.md`). CI runs fmt, clippy, tests, and rustdoc on Ubuntu, macOS, and Windows; only Ubuntu enables the features that link system libraries.

**`live-region` is not approved for the terminal build.** Version 0.3.2 depends on wxDragon, which would pull wxWidgets into every build. It returns with the GUI in wave 3 behind `textweaver-a11y/live-region`.

## Consequences

- Each agent can run `cargo test -p <crate>` without the other agents' work.
- Moving a type into `core` is the standard fix for a would-be cycle; it is cheap but centralizes change requests on the orchestrator.
- Features that link system libraries (espeak-ng) are exercised only on Linux in CI and Docker; Windows and macOS rely on native backends from wave 3.

## See also

- [CONTRIBUTING.md](../../CONTRIBUTING.md): how to build, check, and add a dependency.
- [Docker development container](../dev/docker.md): where Linux-only features are built and tested.
- [Architecture](../dev/architecture.md): the crate map, the threads, and the path from a file to a spoken, highlighted word.
- [Documentation index](../README.md)
