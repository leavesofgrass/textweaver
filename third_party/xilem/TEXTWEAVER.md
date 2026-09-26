# Vendored Xilem and Masonry

This folder is a trimmed copy of [Xilem](https://github.com/linebender/xilem), Linebender's all-Rust GUI toolkit, used by textweaver's GUI crate, `crates/textweaver-xilem`. The decision is [ADR-0023](../../docs/adr/0023-xilem-gui.md).

## What is here

- **Upstream revision:** `271a27a6d4a930f7878d404f9014e3c50a3a9b88` on `main`, "xilem: Expose imaging backends as features (#1847)", committed on Monday, September 14, 2026. The crates call themselves version 0.4.0, the last release (October 29, 2025), but main has moved well past it.
- **Crates:** `masonry`, `masonry_core`, `masonry_winit`, `masonry_imaging`, `masonry_testing`, `tree_arena`, and `include_doc_path`, which textweaver uses; and `xilem`, `xilem_core`, and `xilem_masonry`, Xilem's reactive layer, kept so it can be adopted without another vendoring step.
- **Parley 0.8.0** (`parley/`), from crates.io, for its `accesskit` feature on AccessKit 0.25. The root `Cargo.toml` patches crates.io's `parley` with it.
- **Left out:** `xilem_web`, `placehero`, the examples, benches, stress tests, and the screenshot folders used by Masonry's own tests.

It is its own Cargo workspace. The root workspace excludes it (`exclude = ["third_party/xilem"]`) and depends on its crates by path.

## textweaver's changes

`textweaver.patch` holds every change against upstream, as a unified diff. In short:

1. **AccessKit 0.25.1 and accesskit_winit 0.34.1** (from 0.24 and 0.32.2), and accesskit_consumer 0.39.1 (from 0.35). This brings the Orca-detection fix (`accesskit_unix` 0.22 and later) that Xilem issue #1733 is about, plus text attributes on macOS and the AT-SPI EditableText and Document interfaces.
2. `masonry_core`: `accesskit::Tree` became `TreeInfo` in AccessKit 0.25.
3. `masonry_testing`: `accesskit_consumer::Node` became `NodeRef`. `TestHarness::access_node` now uses `node_by_tree_local_id` (AccessKit PR #707) instead of an `unsafe` write into a private `NodeId`, which closes the workaround for AccessKit issue #701.
4. **Parley 0.8.0**: its optional `accesskit` dependency moved from 0.24 to 0.25.1. No source changes were needed.
5. Manifests: the removed crates, examples, tests, and benches are gone from the workspace member list and the crate manifests, so the copy builds without them.

These are small enough to send upstream as one pull request: "Update to AccessKit 0.25 and accesskit_winit 0.34". See ADR-0023, "Upstream".

## Updating

1. Pick a new upstream revision and copy the crates listed above over this folder, leaving out the same things.
2. Apply `textweaver.patch` (parts may already be upstream; drop those).
3. Build and test `textweaver-xilem`, then regenerate the patch and update the revision here, in the workspace manifest's comment, and in ADR-0023.

The licences travel with the code: Xilem and Masonry are Apache-2.0 (`LICENSE`), Parley is Apache-2.0 or MIT (`parley/LICENSE-APACHE`, `parley/LICENSE-MIT`).
