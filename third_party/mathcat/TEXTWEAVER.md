# Vendored MathCAT

A copy of [MathCAT](https://github.com/daisy/MathCAT) 0.7.6-rc.3 (DAISY; Neil Soiffer), the crates.io package `mathcat-0.7.6-rc.3.crate`, SHA-256 `e02ae55d13d1cab996bdb0ed0921011d1ba621868cf3d5d05afdd83d910b2bc8` (the checksum in `Cargo.lock`), with one fix for textweaver. The decision is [ADR-0036](../../docs/adr/0036-math-braille-and-navigation.md). Licence: MIT, as upstream (`LICENSE`).

## Why

textweaver builds MathCAT with its `no-unsafe` feature, because the workspace denies unsafe code. In that configuration MathCAT's `get_navigation_braille` panics whenever the part of a formula being explored is not the whole formula: [issue #827](https://github.com/daisy/MathCAT/issues/827), open on Monday, September 28, 2026, with no release that fixes it. Math braille on the display while exploring a formula needs that function.

## textweaver's change

`textweaver.patch` holds it as a unified diff against the crates.io package:

- `src/interface.rs`: a new public `copy_mathml_to(mathml, doc)` copies an element into a given document, allocating every node of the copy there. `get_navigation_braille` uses it for the part it wraps in a new `<math>` element. Before, the copy was made in the original document and then appended to the new one; the unsafe backend tolerates that, but the no-unsafe backend's nodes are indices into their own document, so it read past the end of the new document's storage. `copy_mathml` keeps its behavior (a copy in the element's own document).
- `Cargo.toml`: the three `[[test]]` targets are removed, because their files are not vendored (see "Left out").

The upstream pull request text is kept outside the repository, for the owner to file. The regression test is `crates/textweaver-mathcat/tests/it/navigation.rs`: it fails with the crates.io package (the step into the fraction answers "MathCAT crash") and passes with this copy.

## Keeping it in step

- The root `Cargo.toml` still pins `mathcat = "=0.7.6-rc.3"`, and `[patch.crates-io]` points it here. The workspace excludes this folder.
- Before each merge of main, re-check issue #827. When a MathCAT release fixes it, pin that release, remove this folder and the `[patch.crates-io]` line, and read the recorded wording in `crates/textweaver-mathcat/tests/it/vectors.rs` and the braille snapshots again.
- To check the copy: download the package above, check its SHA-256, unpack it, and apply `textweaver.patch` with `patch -p1`; `src` and `Rules` then match this folder exactly.

## Left out

`tests/` (5.7 MB of MathCAT's own tests), `notes/`, `logo.png`, `AGENTS.md`, `_config.yml`, and the package's `Cargo.lock`. Everything the library and its build script use (`src/`, `Rules/`, `build.rs`) is here unchanged apart from the patch.
