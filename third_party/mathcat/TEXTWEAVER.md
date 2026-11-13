# Vendored MathCAT

A copy of [MathCAT](https://github.com/daisy/MathCAT) 0.7.7 (DAISY; Neil Soiffer), the crates.io package `mathcat-0.7.7.crate`, SHA-256 `581e1d6d09391322b7827d5948349489ae5b58310f31b9cd2616ed3fefe6833a` (the checksum crates.io gives in a lock file), with its manifest trimmed for textweaver. The decision is [ADR-0036](../../docs/adr/0036-math-braille-and-navigation.md). Licence: MIT, as upstream (`LICENSE`), unchanged from 0.7.6-rc.3.

## Why

textweaver builds MathCAT with its `no-unsafe` feature, because the workspace denies unsafe code. In that configuration MathCAT 0.7.6-rc.3's `get_navigation_braille` panicked whenever the part of a formula being explored was not the whole formula: [issue #827](https://github.com/daisy/MathCAT/issues/827). textweaver vendored MathCAT to carry a fix for it.

MathCAT 0.7.7 (October 21, 2026) fixes the issue itself: `copy_mathml` now takes the destination document and allocates the whole copy there, which is what textweaver's `copy_mathml_to` did. The issue was closed on Thursday, October 22, 2026. Since beta 6 the source part of the patch is dropped, and `src`, `Rules` and `build.rs` are the package's own, unchanged.

## textweaver's change

`textweaver.patch` holds it as a unified diff against the crates.io package:

- `Cargo.toml`: the three `[[test]]` targets are removed, because their files are not vendored (see "Left out").

The regression test is `crates/textweaver-mathcat/tests/it/navigation.rs`: it failed with the 0.7.6-rc.3 package (the step into the fraction answered "MathCAT crash") and passes with 0.7.7.

## Keeping it in step

- The root `Cargo.toml` pins `mathcat = "=0.7.7"`, and `[patch.crates-io]` points it here. The workspace excludes this folder, and `.gitattributes` keeps its line ends as published.
- With issue #827 fixed upstream, this copy is no longer needed: the next step is to use the crates.io package directly, remove this folder and the `[patch.crates-io]` line, and drop the `.gitattributes` line and the references in ADR-0036. That touches files outside this folder, so it waits for the owner's go.
- At each MathCAT upgrade, read the recorded wording in `crates/textweaver-mathcat/tests/it/vectors.rs` and the braille tests again, and compare the speech and braille of the math fixtures before and after.
- To check the copy: download the package above, check its SHA-256, unpack it, and apply `textweaver.patch` with `patch -p1`; `src`, `Rules` and `build.rs` then match this folder exactly.

## Left out

`tests/` (MathCAT's own tests), `notes/`, the logo images (`logo.png`, `logo-inline.png`, `mathcat_logo_regular.png`), `AGENTS.md`, `_config.yml`, and the package's `Cargo.lock`. Everything the library and its build script use (`src/`, `Rules/`, `build.rs`) is here unchanged.
