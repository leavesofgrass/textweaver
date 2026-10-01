# Third-party data

textweaver bundles a few data files it did not write, so it can read documents, speak with pronunciation dictionaries, and mark difficult words offline, with no download and no network access needed for the base features. Each one keeps its own licence file next to it in the repository.

- `third_party/ibmtts-dictionaries/`: the community IBMTTS pronunciation dictionaries by amirsol81, x0, thunderdrop and contributors (CC0 1.0), used by the Eloquence backend.
- `third_party/fonts/`: fonts built into textweaver for PDF and EPUB output and the GUI, each with its licence file and a README giving its source, version, and SHA-256:
  - Atkinson Hyperlegible Next and Atkinson Hyperlegible Mono, copyright 2020-2024 The Atkinson Hyperlegible Next Project Authors and The Atkinson Hyperlegible Mono Project Authors (Braille Institute of America), SIL Open Font License 1.1.
  - OpenDyslexic, copyright 2019 Abbie Gonzalez, with Reserved Font Name OpenDyslexic, SIL Open Font License 1.1.
  - Lexend: only its license and README are kept (`third_party/fonts/lexend/`). The font is not bundled; the app downloads it into the data folder the first time a reader chooses it, after asking, from pinned URLs checked by SHA-256 (`crates/textweaver-fonts/src/downloaded.rs`). Copyright 2018 The Lexend Project Authors, with Reserved Font Name "RevReading Lexend", SIL Open Font License 1.1. The two files are also test fixtures in `fixtures/w7l/`, so no test downloads.
- `third_party/scowl/`: word levels for difficult-word marking, derived from SCOWL (Spell Checker Oriented Word Lists) version 2, release 2026.02.25, copyright 2000-2026 Kevin Atkinson, with the Australian English data copyright 2016 Benjamin Titze; MIT-like licence. The full notices are in `third_party/scowl/Copyright`, and `tools/scowl_levels.py` rebuilds the list.

`cargo xtask notices` writes `THIRD-PARTY-NOTICES.md` at the repository root from every dependency's own licence, Rust crates included; that file, not this page, is the complete legal notice shipped with a release. This page is a shorter map to the data files kept in the repository itself, for anyone working on the code that reads them.

## See also

- [Building textweaver](building.md): Scripts and Repository layout.
- [Architecture](architecture.md): `textweaver-fonts`, `textweaver-lexicon`, and the other crates that read this data.
- [LICENSE](../../LICENSE): textweaver's own licence, GPL-3.0-or-later.
- [Documentation index](../README.md)
