//! The message style guide (`docs/dev/messages.md`) as tests over the
//! English catalog's raw values.
//!
//! Each test checks one rule. Where English still breaks a rule, the ids
//! that do are listed by name: the lists may only shrink. Fixing a message
//! means taking its id off the list; a new message that breaks a rule fails
//! the test, with the rule's number in the guide.

use std::collections::{BTreeMap, BTreeSet};

use super::ENGLISH;

/// Every message in en.ftl as (id, value): continuation lines joined with
/// a line break, terms and comments left out.
fn messages() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut open = false;
    for line in ENGLISH.lines() {
        if let Some((id, value)) = line.split_once(" =")
            && !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            out.push((id.to_owned(), value.trim().to_owned()));
            open = true;
        } else if open && line.starts_with(' ') {
            if let Some((_, v)) = out.last_mut() {
                v.push('\n');
                v.push_str(line.trim());
            }
        } else {
            open = false;
        }
    }
    out
}

/// A value without its placeables (`{ $name }`, `{ -brand }`, selectors).
fn plain(value: &str) -> String {
    let re = regex::Regex::new(r"\{[^{}]*\}").unwrap();
    re.replace_all(value, "").into_owned()
}

/// Rule 1 (grammar): no verb stacked on another ("Table row added
/// removed.") and no "Exported to PDF failed".
#[test]
fn english_catalog_has_no_stacked_verbs() {
    let stacked =
        regex::Regex::new(r"(?i)\b(inserted|added) removed\b|Exported to .* failed").unwrap();
    let bad: Vec<String> = messages()
        .into_iter()
        .filter(|(_, v)| stacked.is_match(v))
        .map(|(id, _)| id)
        .collect();
    assert!(
        bad.is_empty(),
        "stacked verbs (messages.md, rule 1): {bad:?}"
    );
}

/// Rule 7: one word for each thing. "cursor", never "caret"; "speech
/// engine", never "backend"; "window", never "GUI"; "version", never
/// "build"; the Voices command by its name; "words per minute"; "center".
#[test]
fn english_catalog_uses_one_word_for_each_thing() {
    const WORDS: [&str; 8] = [
        "backend",
        "caret",
        "GUI",
        "this build",
        "Choose Voice",
        "words a minute",
        "centre",
        "e-mail",
    ];
    // The key named Caret (^) is a key, not the cursor.
    let allowed: BTreeSet<&str> = ["keyname-caret"].into();
    let mut bad = Vec::new();
    for (id, v) in messages() {
        let text = plain(&v);
        for w in WORDS {
            if text.contains(w) && !allowed.contains(id.as_str()) {
                bad.push(format!("{id}: {w}"));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "one word for each thing (messages.md, rule 7): {bad:?}"
    );
}

/// Rule 6: a screen reader says "#rrggbb" as "number sign r r g g b b".
/// Only the two messages that teach the typed form show it.
#[test]
fn hex_color_codes_appear_only_where_typing_is_taught() {
    let allowed: BTreeSet<&str> = ["colors-intro", "gui-colors-help"].into();
    let bad: Vec<String> = messages()
        .into_iter()
        .filter(|(id, v)| v.contains("#rrggbb") && !allowed.contains(id.as_str()))
        .map(|(id, _)| id)
        .collect();
    assert!(
        bad.is_empty(),
        "#rrggbb in a help (messages.md, rule 6): {bad:?}"
    );
}

/// Rule 3: questions end "y or n", lower case, with a period only when
/// more text follows it.
#[test]
fn questions_end_with_y_or_n() {
    let mut bad = Vec::new();
    for (id, v) in messages() {
        for line in v.lines() {
            let line = line.trim();
            if line.contains("Y or N") {
                bad.push(format!("{id}: Y or N"));
            }
            // "Press y or n." is an instruction, not a question.
            if line.ends_with("y or n.") && id != "common-press-y-or-n" {
                bad.push(format!("{id}: a period after y or n"));
            }
        }
    }
    assert!(bad.is_empty(), "questions (messages.md, rule 3): {bad:?}");
}

/// Rule 2: an error ends with what to do next. The system's words
/// (`{ $error }`) come before it, with no period after them; these messages
/// still end with the error itself and may only become fewer.
#[test]
fn errors_get_a_next_step() {
    const NO_NEXT_STEP_YET: [&str; 51] = [
        "audio-failed",
        "batch-report-failed",
        "batch-start-failed",
        "citations-bibliography-insert-failed",
        "citations-check-failed",
        "citations-format-failed",
        "citations-import-failed",
        "citations-insert-failed",
        "citations-library-save-failed",
        "citations-lookup-failed",
        "citations-lookup-not-started",
        "citations-style-unusable",
        "define-glossary-problem",
        "dictation-failed",
        "edit-change-failed",
        "edit-delete-failed",
        "edit-image-failed",
        "edit-insert-failed",
        "font-download-failed",
        "grammar-change-failed",
        "marks-cannot-search",
        "notes-study-sheet-failed",
        "playback-speech-error",
        "profiles-export-failed",
        "profiles-read-failed",
        "publish-cannot-write",
        "publish-cannot-write-to",
        "publish-export-error",
        "publish-export-failed",
        "publish-preview-error",
        "publish-preview-failed",
        "publish-render-failed",
        "publish-start-failed",
        "replace-failed",
        "restart-failed",
        "settings-cannot-be",
        "settingsio-export-failed",
        "settingsio-import-failed",
        "settingsio-read-failed",
        "spell-replace-failed",
        "sync-sidecar-failed",
        "sync-status-error",
        "sync-write-failed",
        "tui-setup-cannot-save",
        "tui-setup-keymap-ignored",
        "voice-catalog-failed",
        "voice-details-failed",
        "voice-list-failed",
        "voice-preview-failed",
        "voice-remove-failed",
        "writes-bookmark-not-saved",
    ];
    // Messages whose error is a detail of something already said: the
    // file could not be opened, and the reason is the next step.
    const REASON_IS_THE_STEP: [&str; 3] = [
        "gui-open-failed",
        "tasks-could-not-open",
        "tui-could-not-open",
    ];
    let known: BTreeSet<&str> = NO_NEXT_STEP_YET
        .iter()
        .chain(REASON_IS_THE_STEP.iter())
        .copied()
        .collect();
    let mut new = Vec::new();
    let mut fixed = Vec::new();
    let mut period = Vec::new();
    let all = messages();
    for (id, v) in &all {
        let v = v.trim_end();
        if v.ends_with("{ $error }.") {
            period.push(id.clone());
        }
        if !v.contains('\n') && v.ends_with("{ $error }") && !known.contains(id.as_str()) {
            new.push(id.clone());
        }
    }
    for id in NO_NEXT_STEP_YET {
        if let Some((_, v)) = all.iter().find(|(i, _)| i == id)
            && !v.trim_end().ends_with("{ $error }")
        {
            fixed.push(id);
        }
    }
    assert!(
        period.is_empty(),
        "a period after the error (messages.md, rule 2): {period:?}"
    );
    assert!(
        new.is_empty(),
        "an error with no next step (messages.md, rule 2): {new:?}"
    );
    assert!(
        fixed.is_empty(),
        "these now give a next step: take them off NO_NEXT_STEP_YET: {fixed:?}"
    );
}

/// Rule 7 again: one sentence, one id. These groups say the same sentence
/// under two or three ids and wait to be merged; no new group may appear.
#[test]
fn no_new_duplicate_sentences() {
    const KNOWN: [&[&str]; 11] = [
        &["app-no-document-open", "gui-no-document"],
        &["publish-no-document", "tui-empty-no-document"],
        &["marks-bookmark-set", "writes-bookmark-set"],
        &["authoring-link-no-address", "links-no-address"],
        &["gui-settings-closed", "settings-closed"],
        &["edit-no-matches", "replace-no-matches"],
        &["edit-recovery-write-failed", "writes-recovery-copy-failed"],
        &[
            "edit-recovery-writing-again",
            "writes-recovery-copy-resumed",
        ],
        &["authoring-not-in-table", "tables-not-in-table"],
        &["grammar-line", "lint-line", "spell-line"],
        &["grammar-left-as-is", "spell-left-as-is"],
    ];
    // Labels share their text on purpose (a menu item, a setting, and a
    // section can all be "Reading statistics").
    const LABELS: [&str; 9] = [
        "name-",
        "menu-",
        "setting-",
        "choice-",
        "action-",
        "keyname-",
        "pos-",
        "settings-unit-",
        "lang-",
    ];
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, v) in messages() {
        let sentence = v.ends_with('.') && v.contains(' ') && v.chars().count() >= 12;
        if LABELS.iter().any(|p| id.starts_with(p)) || v.contains('\n') || !sentence {
            continue;
        }
        groups.entry(v).or_default().push(id);
    }
    let known: BTreeSet<Vec<String>> = KNOWN
        .iter()
        .map(|g| g.iter().map(|s| (*s).to_owned()).collect())
        .collect();
    let mut new = Vec::new();
    for (v, mut ids) in groups {
        if ids.len() < 2 {
            continue;
        }
        ids.sort();
        if !known.contains(&ids) {
            new.push(format!("{ids:?}: {v}"));
        }
    }
    assert!(
        new.is_empty(),
        "the same sentence under two ids (messages.md, rule 7): {new:?}"
    );
}

/// Rule 11: the window's button descriptions fit one Braille line in every
/// language (the cell count itself is checked in the terminal's tests).
#[test]
fn button_hints_are_short_in_every_language() {
    for (tag, text) in std::iter::once(("en", ENGLISH)).chain(super::BUILTIN.iter().copied()) {
        let c = super::Catalog::parse(tag, text).unwrap_or_else(|e| panic!("{tag}: {e:?}"));
        for id in c.ids() {
            if id.starts_with("gui-hint-") {
                let s = c.tr(id);
                assert!(
                    s.chars().count() <= 40,
                    "{tag}: {id} is {} characters: {s}",
                    s.chars().count()
                );
            }
        }
    }
}
