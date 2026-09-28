# Vendored Xilem and Masonry

This folder is a trimmed copy of [Xilem](https://github.com/linebender/xilem), Linebender's all-Rust GUI toolkit, used by textweaver's GUI crate, `crates/textweaver-xilem`. The decision is [ADR-0027](../../docs/adr/0027-xilem-gui.md).

## What is here

- **Upstream revision:** `271a27a6d4a930f7878d404f9014e3c50a3a9b88` on `main`, "xilem: Expose imaging backends as features (#1847)", committed on Monday, September 14, 2026. The crates call themselves version 0.4.0, the last release (October 29, 2025), but main has moved well past it.
- **Crates:** `masonry`, `masonry_core`, `masonry_winit`, `masonry_imaging`, `masonry_testing`, `tree_arena`, and `include_doc_path`, which textweaver uses; and `xilem`, `xilem_core`, and `xilem_masonry`, Xilem's reactive layer, kept so it can be adopted without another vendoring step.
- **Parley 0.11.1** (`parley/`), from crates.io (August 16, 2026), for its `accesskit` feature on AccessKit 0.25. The root `Cargo.toml` patches crates.io's `parley` with it. It replaced Parley 0.8.0 in Wave 5 (Agent W5a4); see ADR-0027's status update. Parley's main branch has since removed the `accesskit` feature (its integration moved to an example), so the next upgrade past 0.11 means Masonry's `TextArea` builds its own accessibility nodes; textweaver's document view already does.
- **Left out:** `xilem_web`, `placehero`, the examples, benches, stress tests, and the screenshot folders used by Masonry's own tests.

It is its own Cargo workspace. The root workspace excludes it (`exclude = ["third_party/xilem"]`) and depends on its crates by path.

## textweaver's changes

`textweaver.patch` holds every change against upstream, as a unified diff. In short:

1. **AccessKit 0.25.1 and accesskit_winit 0.34.1** (from 0.24 and 0.32.2), and accesskit_consumer 0.39.1 (from 0.35). This brings the Orca-detection fix (`accesskit_unix` 0.22 and later) that Xilem issue #1733 is about, plus text attributes on macOS and the AT-SPI EditableText and Document interfaces.
2. `masonry_core`: `accesskit::Tree` became `TreeInfo` in AccessKit 0.25.
3. `masonry_testing`: `accesskit_consumer::Node` became `NodeRef`. `TestHarness::access_node` now uses `node_by_tree_local_id` (AccessKit PR #707) instead of an `unsafe` write into a private `NodeId`, which closes the workaround for AccessKit issue #701.
4. **Parley 0.11.1**: its optional `accesskit` dependency moved from 0.24.0 to 0.25.1 (`parley/Cargo.toml`, the published manifest; `Cargo.toml.orig` is left as published). No source changes were needed.
5. Manifests: the removed crates, examples, tests, and benches are gone from the workspace member list and the crate manifests, so the copy builds without them.

These are small enough to send upstream as one pull request: "Update to AccessKit 0.25 and accesskit_winit 0.34". See ADR-0027, "Upstream".

textweaver's own fixes, each small, each a candidate for its own pull request:

6. `masonry`: `TextArea` takes an accessible label (`with_accessible_label`), so a prompt's field has a name.
7. `masonry_winit`: `DriverCtx::exit` is honoured after an async action too, so a window can close on a timer or a background message.
8. `masonry_testing`: the harness registers fonts and draws at the scale factor it lays out at (the screenshots at 200%).
9. **Actions on nodes a widget adds itself** (Wave 3, second half). A widget may add AccessKit nodes that are not widgets: text runs, list options, the settings form's rows. Masonry sent an action aimed at one of them to a widget id that does not exist, which panics in a debug build. Now the accessibility pass records which widget added each such node (`RenderRootState::access_node_owners`), and the action goes to that widget, with the node named in the new `AccessEvent::node` field. The appended part of `textweaver.patch` (it applies after the earlier parts) holds this change.
10. `masonry`: a one-line `TextArea` (`InsertNewline::Never`) leaves Up and Down to its parent, as a one-line edit does on Windows, so a prompt can recall earlier answers with them.
11. `masonry`: forced by Parley 0.9 (Wave 5, W5a4). `Layout::align` no longer takes a width, so `Label` aligns its lines within the width they were broken at (or the layout's own width when it was not broken). Start-aligned labels, which are all textweaver uses, are unchanged; a centered or end-aligned label reused from the cache at a different width can sit differently. Upstream Masonry makes the same change when it moves to Parley 0.9.

## Updating

1. Pick a new upstream revision and copy the crates listed above over this folder, leaving out the same things.
2. Apply `textweaver.patch` (parts may already be upstream; drop those).
3. Build and test `textweaver-xilem`, then regenerate the patch and update the revision here, in the workspace manifest's comment, and in ADR-0027.

The licences travel with the code: Xilem and Masonry are Apache-2.0 (`LICENSE`), Parley is Apache-2.0 or MIT (`parley/LICENSE-APACHE`, `parley/LICENSE-MIT`).
