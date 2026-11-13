# Vendored accesskit_windows

A copy of [accesskit_windows](https://github.com/AccessKit/accesskit) 0.35.2 (AccessKit's Windows adapter), from crates.io, with two changes for textweaver's GUI. The decision is [ADR-0033](../../docs/adr/0033-gui-session-2-and-edit-mode.md). Licence: MIT OR Apache-2.0, as upstream.

## Why

Every button in the GUI has a key from the keymap, which the owner wants in the button's standard keyboard shortcut property, so NVDA and JAWS say it only when their "report shortcut keys" setting is on. AccessKit has the property (`Node::set_keyboard_shortcut`), but its Windows adapter (0.35.1, and still 0.35.2) never gives it to UI Automation, so no screen reader could see it.

In edit mode the GUI marks each misspelled word and each Markdown lint problem as its own text run, with AccessKit's `invalid` set to spelling or to grammar. The adapter never gives that to UI Automation either, so NVDA's "report spelling errors" could not work in the GUI.

## textweaver's changes

`textweaver.patch` holds them as a unified diff against the crates.io release. Re-vendored from 0.35.2 (October 25, 2026, which keeps the content of password inputs from UI Automation) in beta 6; the patch applied unchanged. `src/node.rs` answers UI Automation's `AcceleratorKey` property (`UIA_AcceleratorKeyPropertyId`) with the node's keyboard shortcut, and raises a property change when it changes (the adapter's `properties!` table does both).

Beta 6: `src/text.rs` answers the text attribute `UIA_AnnotationTypesAttributeId` for a text range: an array holding `AnnotationType_SpellingError` where the range's runs are invalid for spelling, `AnnotationType_GrammarError` where they are invalid for grammar, an empty array (the attribute's documented default) over plain text, and UI Automation's mixed attribute value when the range's runs differ, as for every other attribute. `src/util.rs` gains `variant_from_i32_slice`, which builds that array value. `src/text.rs` also has four tests (`textweaver_annotation_tests`) that read the attribute through the range provider itself, with no window: a spelling run, a grammar run, plain runs, and ranges over two kinds. Left as upstream: `UIA_AnnotationObjectsAttributeId` (annotation elements, for comments and the like; NVDA reads spelling and grammar errors from the types alone), `FindAttribute` (not implemented upstream for any attribute), and a run's description (UI Automation has no text attribute for a free-text note on a range). Nothing else differs.

To test the adapter without writing a `Cargo.lock` into this folder, copy it to a scratch folder under your own target directory, add an empty `[workspace]` table to the copy's `Cargo.toml`, copy in the root `Cargo.lock`, and run `cargo test --offline --lib -- --skip tests::simple::focus` there. That one upstream test shows and focuses a window, which a running screen reader would announce.

Each change is small enough to send upstream as its own pull request: "Windows: expose the keyboard shortcut as AcceleratorKey", and "Windows: expose invalid spelling and grammar as annotation types". Once a release has both, remove this folder and the `[patch.crates-io]` line in the root `Cargo.toml`.

## Left out

The crate's own `Cargo.lock` and packaging files. It is its own package; the root workspace excludes it and patches crates.io's `accesskit_windows` with it.
