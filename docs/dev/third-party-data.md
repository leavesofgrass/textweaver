# Third-party data

textweaver bundles a few data files it did not write, so it can read documents, speak with pronunciation dictionaries, and mark difficult words offline, with no download and no network access needed for the base features. Each one keeps its own license file next to it in the repository.

- `third_party/ibmtts-dictionaries/`: the community IBMTTS pronunciation dictionaries by amirsol81, x0, thunderdrop and contributors (CC0 1.0), used by the Eloquence backend.
- `third_party/fonts/`: fonts built into textweaver for PDF and EPUB output and the GUI, each with its license file and a README giving its source, version, and SHA-256:
  - Atkinson Hyperlegible Next and Atkinson Hyperlegible Mono, copyright 2020-2024 The Atkinson Hyperlegible Next Project Authors and The Atkinson Hyperlegible Mono Project Authors (Braille Institute of America), SIL Open Font License 1.1.
  - OpenDyslexic, copyright 2019 Abbie Gonzalez, with Reserved Font Name OpenDyslexic, SIL Open Font License 1.1.
  - Lexend: only its license and README are kept (`third_party/fonts/lexend/`). The font is not bundled; the app downloads it into the data folder the first time a reader chooses it, after asking, from pinned URLs checked by SHA-256 (`crates/textweaver-fonts/src/downloaded.rs`). Copyright 2018 The Lexend Project Authors, with Reserved Font Name "RevReading Lexend", SIL Open Font License 1.1. The two files are also test fixtures in `fixtures/w7l/`, so no test downloads.
- `third_party/scowl/`: word levels for difficult-word marking, derived from SCOWL (Spell Checker Oriented Word Lists) version 2, release 2026.02.25, copyright 2000-2026 Kevin Atkinson, with the Australian English data copyright 2016 Benjamin Titze; MIT-like license. The full notices are in `third_party/scowl/Copyright`, and `tools/scowl_levels.py` rebuilds the list.
- `third_party/lexicon/`: the offline dictionary for define word, built from Open English WordNet 2025 (CC BY 4.0, with Princeton's WordNet notice) and the CMU Pronouncing Dictionary (a BSD-style license). It holds `lexicon-en.twlex`, the derived data file, the two license texts (`WORDNET-LICENSE` and `CMUDICT-LICENSE`), and a README with the sources, checksums, and how the file is built.
- `third_party/lame/`: the license of the LAME MP3 encoder (GNU LGPL, version 2). The encoder's source comes from the `mp3lame-sys` crate, which the `mp3lame-encoder` crate builds and links statically; only the license text lives here, so the packages can carry it.

`cargo xtask notices` writes `THIRD-PARTY-NOTICES.md` at the repository root from every dependency's own license, Rust crates included; that file, not this page, is the complete legal notice shipped with a release. This page is a shorter map to the data files kept in the repository itself, for anyone working on the code that reads them. Vendored code is below.

## Vendored code

Three folders under `third_party/` are copies of other people's code that textweaver builds from, because it needs a change that upstream does not have, or a release it cannot get from crates.io. Each has a `TEXTWEAVER.md` that says where the copy came from, what differs, how to check it, and when it can go. The root `Cargo.toml` points at each through `[patch.crates-io]` or a path dependency, and the root workspace excludes them.

- `third_party/mathcat/`: MathCAT 0.7.7 (DAISY; Neil Soiffer), the crates.io package `mathcat-0.7.7.crate`, with its SHA-256 in `TEXTWEAVER.md`. It was vendored in Wave 5 to carry a fix for MathCAT issue #827 (`get_navigation_braille` panicked in builds without unsafe code). Release 0.7.7 fixes the issue itself, so since beta 6 the source patch is gone: `src`, `Rules` and `build.rs` are the package's own, and `textweaver.patch` only removes the three `[[test]]` targets from `Cargo.toml`, because their test files are not vendored. The next step, using the crates.io package directly and removing the folder, waits for the owner's go. The decision is [ADR-0036](../adr/0036-math-braille-and-navigation.md), which still describes the earlier patch.
- `third_party/accesskit_windows/`: AccessKit's Windows adapter 0.35.2, with two changes, both in `textweaver.patch`. The first answers UI Automation's `AcceleratorKey` property with a node's keyboard shortcut, so NVDA and JAWS can say a button's key (`src/node.rs`). The second, new in beta 6, answers the text attribute `UIA_AnnotationTypesAttributeId` for a text range (`src/text.rs`, with `variant_from_i32_slice` in `src/util.rs`): a spelling error where the range's runs are invalid for spelling, a grammar error where they are invalid for grammar, an empty array over plain text, and the mixed value where they differ. Four tests in `src/text.rs` read the attribute through the range provider with no window. The decision is [ADR-0033](../adr/0033-gui-session-2-and-edit-mode.md). Each change is small enough to send upstream as its own pull request, and the folder goes once a release has both.
- `third_party/xilem/`: Xilem, Masonry and Parley, trimmed, with 22 numbered changes in `textweaver.patch`. `TEXTWEAVER.md` gives each change a status line, "Still needed" or "Upstream differently" with the pull request, and the date it was checked (Friday, November 13, 2026 for the last pass); read it before a catch-up with upstream, and update the lines when you check again. The decision is [ADR-0027](../adr/0027-xilem-gui.md).

To test a vendored crate outside the workspace, follow the steps in its `TEXTWEAVER.md`. For `accesskit_windows`, one upstream test (`tests::simple::focus`) shows and focuses a window, which a running screen reader would announce, so it is skipped: `cargo test --offline --lib -- --skip tests::simple::focus`.

## See also

- [Building textweaver](building.md): Scripts and Repository layout.
- [Architecture](architecture.md): `textweaver-fonts`, `textweaver-lexicon`, and the other crates that read this data.
- [LICENSE](../../LICENSE): textweaver's own license, GPL-3.0-or-later.
- [Documentation index](../README.md)
