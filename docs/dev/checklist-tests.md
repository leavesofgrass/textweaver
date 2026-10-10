# The polish checklist and its tests

Wave 9 measures "done" against one checklist of consistency, accessibility, visual and documentation rules. This page maps each row to the automated check that holds it, or to the owner's manual session that covers it. A row with both has both.

The sessions are short scripted passes the owner runs with a screen reader and the Mantis Q40 Braille display:

- S1: first run.
- S2: reading and moving.
- S3: lists and dialogs.
- S4: edit mode.
- S5: documents and conversion.
- S6: low vision and dyslexia, with a sighted helper.
- S7: Braille on the Mantis Q40.
- S8: components and consent.
- S9: keyboard only, no screen reader.
- S10: Linux with Orca (optional).

"Ignored" below means a test that holds a known defect: it runs with `cargo test -- --ignored`, and the fix removes the `#[ignore]`.

## Consistency

**C1. Every dialog is modal, named, hides the window behind it, focuses its first control, closes on Escape, returns the focus, and says it closed.**

- Test: `crates/textweaver-xilem/tests/it/dialogs.rs`, `every_dialog_has_a_close_escape_closes_and_focus_returns` (focus in, a Close or No, Escape, focus back) and `every_dialog_is_modal_named_and_hides_the_window_behind` (one modal dialog, named by its title, the window behind disabled and hidden from readers, back after closing). Both run over the seven kinds: list, prompt, palette, question, settings, colors, voices.
- Test: "Canceled." on Escape in an app list, `crates/textweaver-app/tests/it/list_contract.rs`.
- Report: the UI Automation report, `crates/textweaver-xilem/tools/uia-report.ps1`.
- Manual, session S3: the settings, colors and voice dialogs say they closed (the app's driver says it, which the headless tests do not run).

**C2. Every list says its title and introduction first, each item "n of m" first, with letter jump, F1 and the Say Status key, Home, End, Enter and Escape.**

- Test: `crates/textweaver-app/tests/it/list_contract.rs`. It dispatches every action on the sample document and checks each list that opens: the introduction once, "1 of n" on open, F1 repeats the introduction, the Say Status key says something, End and Home say the place, a letter jumps (or filters a filtering list), Escape closes and says so, and Enter chooses.
- Test: the terminal's 40-cell list lines, `crates/textweaver-tui/tests/it/braille.rs`.
- Ignored: `the_known_list_defects_are_fixed`. The file browser's Say Status key says nothing on a Places row, and Escape closes the settings profiles and colors lists without a word. The `KNOWN` list in the test names each; take a line off when its fix lands.
- Manual, session S3.

**C3. Every message has a level, a catalog entry in six languages, keys from the keymap, and its key fact first.**

- Test: `crates/textweaver-xilem/tests/it/announcements.rs` (every app message has a level); `crates/textweaver-app/tests/it/pseudo_locale.rs` and `crates/textweaver-app/tests/it/keys_from_keymap.rs`; the catalog style tests named in [Writing messages](messages.md).
- Test: the key fact first, `crates/textweaver-tui/tests/it/braille_first.rs`.

**C4. Every label has one case style, an ellipsis only before a chooser, the name equal to the label without the key, and the label in the name.**

- Test: `crates/textweaver-xilem/tests/it/window_tree.rs`, `every_button_shows_its_name_then_its_key` (header and toolbar).
- Test: `crates/textweaver-xilem/tests/it/dialogs.rs`, `every_dialog_button_shows_its_name_and_keeps_its_key_out_of_it` (every button of every dialog kind).
- Manual: the glossary of one word per thing is the owner's read.

**C5. Every key comes from the keymap and is shown only if bound; F1, F6 and F10 keep their platform meanings.**

- Test: `cargo xtask keyboard --check`, `crates/textweaver-app/tests/it/keys_from_keymap.rs`, `crates/textweaver-app/tests/it/tests_ask_the_keymap.rs`.

**C6. Every setting has a row, help, default, sync flag and an effect; the app and the terminal reader agree.**

- Test: `cargo xtask settings-doc --check`, `crates/textweaver-app/tests/it/settings_reference.rs`, `crates/textweaver-xilem/tests/it/settings_dialog.rs`.

**C7. Nothing opens a dialog or takes focus unasked; nothing traps focus.**

- Test: `crates/textweaver-app/tests/it/list_contract.rs`, `opening_a_list_under_a_screen_reader_says_only_its_introduction` (nothing is said over the reader when a list opens).
- Report: the UI Automation report.
- Manual, session S9.

**C8. Wording: US English; an error says its cause and next step; no terminal-only claim in the app's text.**

- Test: the catalog style tests ([Writing messages](messages.md)); `button_descriptions_are_short_and_about_the_window` in `window_tree.rs`.
- Manual: the owner's read.

**C9. The terminal reader and the app offer the same commands.**

- Test: `crates/textweaver-xilem/src/parity.rs`.

## Accessibility

**A1. Every control has a role and a name; no key inside a name.** Test: `crates/textweaver-xilem/tests/it/window_tree.rs`, and the two label-in-name tests above. Report: the tree dump on three systems.

**A2. The tree diff is read.** Script: `tools/a11y/tree_report.py` (report only).

**A3. The UI Automation report opens every dialog kind.** Report: `crates/textweaver-xilem/tools/uia-report.ps1`.

**A4. A second tool reads the tree.** Not automated yet (Axe.Windows, report only, is planned).

**A5 and A6. The NVDA and Orca sessions.** Scripts: `tools/a11y/nvda-session.mjs`, `tools/a11y/orca-session.sh`, `tools/a11y/atspi-session.py`. Manual, sessions S1 to S3 with JAWS (no tool drives JAWS).

**A7. Speech once.** Test: `crates/textweaver-app/tests/it/list_contract.rs` (the introduction once). Manual, session S2 with NVDA and JAWS.

**A8. Braille: the key fact in the first 40 cells.**

- Test: `crates/textweaver-tui/tests/it/braille_first.rs`, counted in Braille cells with the uncontracted UEB counter `textweaver_tui::ui::braille_cells`: the title line, list rows, status messages in every language, `every_prompt_label_fits_a_braille_line` (every prompt the app opens) and `the_window_position_line_puts_its_key_facts_inside_forty_cells` (the window's status bar).
- Test: `crates/textweaver-tui/tests/it/braille.rs` (the terminal's rows).
- Ignored: `the_position_line_keeps_modified_inside_forty_cells_at_long_lines`. In edit mode at a three-digit line, "modified" ends at cell 41.
- Manual, session S7 on the Mantis Q40.

**A9. Keyboard only, no screen reader.** Manual, session S9.

**A10. The WCAG 2.2 AA and UAAG 2.0 AA rows.** Manual: the conformance page, with the evidence from the rows here.

## Visual

**V1. Review screenshots at the review sizes and themes.** Test: `screenshots_are_written_at_both_scales` and `screenshots_draw_edit_mode_no_document_the_ruler_and_the_panels` in `window_tree.rs` (they draw). Manual, session S6 (the read).

**V2. Contrast per token.** Test: `crates/textweaver-theme/tests/it/builtin.rs`, `crates/textweaver-xilem/tests/it/frame_theme.rs`.

**V3. No control is clipped or outside the window; targets at least 24 by 24.**

- Test: `no_control_leaves_the_window_at_any_review_size` in `window_tree.rs` (the window), and `no_dialog_control_leaves_a_small_window` in `dialogs.rs` (every dialog kind at 420 by 320 and at 683 by 384 at 200 percent).
- Ignored: `the_voice_manager_fits_the_smallest_window`. The voice manager's controls leave a 420 by 320 window, so it is checked at the laptop size only.

**V4. Frame time.** Manual: the performance probe on the owner's machine; counts are gated, times reported.

**V5. System preferences.** Manual, session S6.

**V6. Every mark has a shape.** Test: the theme's cue requirement, `crates/textweaver-theme/tests/it/builtin.rs`, and `crates/textweaver-xilem/tests/it/highlight_paint.rs`.

## Documentation

**D1. Links, anchors, site accessibility, generated files current.** Script: `tools/check_links.py`, `tools/check_site_a11y.py`, `cargo xtask docs --check`, `cargo xtask regen --check`.

**D2. US English.** Test: the catalog style tests. Manual: the owner's read of the docs.

**D3 to D7. Statements tested, screenshots fresh, the conformance page, keys first in "What changed", the catalog freeze.** Manual: the owner's read and the release steps in [Releasing](releasing.md).

## See also

- [Testing](testing.md): the checks every change must pass.
- [Writing messages](messages.md): the message rules and their tests.
- [Using textweaver with a screen reader](../screen-readers.md).
- [Documentation index](../README.md).
