//! The message style guide (`docs/dev/messages.md`) as tests over the
//! English catalog's raw values.
//!
//! Each test checks one rule. Where English still breaks a rule, the ids
//! that do are listed by name: the lists may only shrink. Fixing a message
//! means taking its id off the list; a new message that breaks a rule fails
//! the test, with the rule's number in the guide.

use std::collections::{BTreeMap, BTreeSet};

use super::ENGLISH;

/// Every message in locales/en as (id, value): continuation lines joined with
/// a line break, terms and comments left out.
fn messages() -> Vec<(String, String)> {
    messages_in(ENGLISH)
}

/// Every message in one catalog's text, as [`messages`] reads locales/en.
fn messages_in(catalog: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut open = false;
    for line in catalog.lines() {
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
/// (`{ $error }`) come before it, with no period after them. Every error
/// gives a next step in every language; one that ends with the error fails.
#[test]
fn errors_get_a_next_step() {
    // Messages whose error is a detail of something already said: the
    // file could not be opened, and the reason is the next step.
    const REASON_IS_THE_STEP: [&str; 3] = [
        "gui-open-failed",
        "tasks-could-not-open",
        "tui-could-not-open",
    ];
    let known: BTreeSet<&str> = REASON_IS_THE_STEP.iter().copied().collect();
    let mut new = Vec::new();
    let mut period = Vec::new();
    for (tag, text) in std::iter::once(("en", ENGLISH)).chain(super::BUILTIN.iter().copied()) {
        for (id, v) in messages_in(text) {
            let v = v.trim_end();
            if v.ends_with("{ $error }.") {
                period.push(format!("{tag}: {id}"));
            }
            if !v.contains('\n') && v.ends_with("{ $error }") && !known.contains(id.as_str()) {
                new.push(format!("{tag}: {id}"));
            }
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
}

/// Rule 7 again: one sentence, one id. No two ids may say the same
/// sentence; merge them into one id instead.
#[test]
fn no_new_duplicate_sentences() {
    // Every known group is merged; the list stays empty.
    const KNOWN: [&[&str]; 0] = [];
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

/// Rule 11 for settings: the help's first sentence, shown under the row and
/// given as its description, has at most fifteen words. The rest of the
/// help is said by F1 and kept in the settings reference. These helps
/// still start with a longer sentence and may only become fewer.
#[test]
fn setting_helps_start_with_a_short_sentence() {
    const LONG_FIRST_SENTENCE: [&str; 5] = [
        "setting-gui-speak-messages-help",
        "setting-highlight-lead-words-help",
        "setting-reading-aids-difficult-definitions-help",
        "setting-speech-latency-offset-ms-help",
        "setting-speech-voices-by-language-help",
    ];
    let end = regex::Regex::new(r"[^.\s]{2,}(\.) ").unwrap();
    let mut new = Vec::new();
    let mut fixed = Vec::new();
    for (id, v) in messages() {
        if !(id.starts_with("setting-") && id.ends_with("-help")) {
            continue;
        }
        let first = match end.captures(&v).and_then(|c| c.get(1)) {
            Some(m) => &v[..m.end()],
            None => v.as_str(),
        };
        let words = plain(first).split_whitespace().count();
        let listed = LONG_FIRST_SENTENCE.contains(&id.as_str());
        if words > 15 && !listed {
            new.push(format!("{id}: {words} words"));
        }
        if words <= 15 && listed {
            fixed.push(id);
        }
    }
    assert!(
        new.is_empty(),
        "a setting help's first sentence over fifteen words (messages.md, rule 11): {new:?}"
    );
    assert!(
        fixed.is_empty(),
        "these now start short: take them off LONG_FIRST_SENTENCE: {fixed:?}"
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

/// Messages heard or read as a status or an announcement: everything but
/// names and labels (menus, settings and their helps, choices, sections,
/// units, kinds, languages), command descriptions, key names, help-topic
/// fragments and the button hints, which have tests of their own. The
/// accessibility audit after beta 5 chose these the same way.
fn is_status(id: &str) -> bool {
    const NOT_STATUS: [&str; 15] = [
        "action-",
        "category-",
        "choice-",
        "gui-hint-",
        "help-",
        "keyname-",
        "kind-",
        "lang-",
        "menu-",
        "name-",
        "pos-",
        "section-",
        "setting-",
        "settings-unit-",
        "unit-",
    ];
    !NOT_STATUS.iter().any(|p| id.starts_with(p))
}

/// Message `id` with sample values: numbers as four digits, other text as
/// eight letters, and each select at its default variant.
fn rendered(c: &super::Catalog, id: &str) -> String {
    const NUMBERS: [&str; 24] = [
        "attempt", "col", "cols", "count", "days", "dropped", "i", "k", "kb", "len", "level",
        "line", "lines", "max", "minutes", "n", "page", "pages", "pct", "percent", "row", "rows",
        "total", "words",
    ];
    let vars = c.variables(id);
    let args: Vec<(&str, super::Arg)> = vars
        .iter()
        .map(|v| {
            let sample = if NUMBERS.contains(&v.as_str()) {
                "1234"
            } else {
                "abcdefgh"
            };
            (v.as_str(), super::Arg::Str(sample.to_owned()))
        })
        .collect();
    c.fmt(id, &args)
}

/// The text up to the first end of a clause: a period, colon, semicolon,
/// question mark or exclamation mark followed by a space or the end, or a
/// line break.
fn first_clause(s: &str) -> &str {
    let line = s.trim_start().lines().next().unwrap_or("");
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    for (k, &(i, ch)) in chars.iter().enumerate() {
        let next_is_space = chars.get(k + 1).is_none_or(|&(_, n)| n == ' ');
        if matches!(ch, '.' | ':' | ';' | '?' | '!') && next_is_space {
            return &line[..i + ch.len_utf8()];
        }
    }
    line
}

/// Checks a "may only shrink" list against the ids that break a rule now:
/// a new offender fails, and so does a listed id that no longer breaks it.
fn ratchet(rule: &str, list_name: &str, listed: &[&str], offenders: &BTreeMap<String, String>) {
    let listed: BTreeSet<&str> = listed.iter().copied().collect();
    let new: Vec<String> = offenders
        .iter()
        .filter(|(id, _)| !listed.contains(id.as_str()))
        .map(|(id, why)| format!("{id}: {why}"))
        .collect();
    let fixed: Vec<&str> = listed
        .iter()
        .copied()
        .filter(|id| !offenders.contains_key(*id))
        .collect();
    assert!(new.is_empty(), "{rule}: {new:#?}");
    assert!(
        fixed.is_empty(),
        "these now pass, or are gone: take them off {list_name}: {fixed:?}"
    );
}

/// Rule 3: a message that asks "? y or n" asks it in its first sentence,
/// before what the question is about. These still ask later and may only
/// become fewer.
#[test]
fn questions_come_first() {
    const QUESTION_LATE: [&str; 19] = [
        "about-docs-question",
        "about-quick-start-online",
        "about-report-question",
        "dictation-model-question",
        "dictation-needs-edit",
        "disk-overwrite-question",
        "disk-reload-question",
        "disk-replace-question",
        "docs-not-found",
        "edit-file-exists-question",
        "graph-export-saved",
        "helper-question",
        "notes-collect-saved",
        "notes-study-sheet-saved-both",
        "notes-study-sheet-saved-highlights",
        "notes-study-sheet-saved-notes",
        "publish-exported",
        "sync-place-question",
        "update-found",
    ];
    let c = super::Catalog::parse("en", ENGLISH).expect("English parses");
    let end = regex::Regex::new(r"[.?!](\s|$)").unwrap();
    let mut late = BTreeMap::new();
    for (id, raw) in messages() {
        if !raw.contains("? y or n") {
            continue;
        }
        let text = rendered(&c, &id);
        let first = end.find(&text).map(|m| m.as_str().chars().next());
        if first != Some(Some('?')) {
            late.insert(id, text);
        }
    }
    ratchet(
        "a question after other text (messages.md, rule 3)",
        "QUESTION_LATE",
        &QUESTION_LATE,
        &late,
    );
}

/// Rule 5: no arrows, check marks, box drawing, bullets, geometric shapes,
/// dingbats or emoji in any language's messages. A speech engine skips or
/// spells them out, and a Braille display shows them as noise. The
/// ellipsis on chooser names ("Open…") is punctuation and is allowed.
/// The messages listed still hold one, by language, and may only become
/// fewer; the translation pass words them.
#[test]
fn no_symbols_in_any_language() {
    // The Arabic table size example uses the multiplication sign.
    const KNOWN_SYMBOLS: [&str; 1] = ["ar: prompt-table-size"];
    fn symbol(ch: char) -> bool {
        matches!(ch as u32,
            0x00A6 | 0x00A9 | 0x00AE | 0x00B0 | 0x00B1 | 0x00D7 | 0x00F7
            | 0x2022 | 0x2023 | 0x2043 | 0x204C | 0x204D
            | 0x2190..=0x23FF
            | 0x2500..=0x27BF
            | 0x2900..=0x2BFF
            | 0x1F000..=0x1FAFF)
    }
    let mut bad = BTreeMap::new();
    for (tag, text) in std::iter::once(("en", ENGLISH)).chain(super::BUILTIN.iter().copied()) {
        for (id, v) in messages_in(text) {
            let found: String = v.chars().filter(|&ch| symbol(ch)).collect();
            if !found.is_empty() {
                bad.insert(format!("{tag}: {id}"), found.escape_unicode().to_string());
            }
        }
    }
    ratchet(
        "symbols in messages (messages.md, rule 5)",
        "KNOWN_SYMBOLS",
        &KNOWN_SYMBOLS,
        &bad,
    );
}

/// Rule 11: the first clause of a status or announcement message, with
/// sample values, fits one 40-cell Braille line. The owner reads computer
/// Braille, one cell per character, so this counts characters. These
/// still start with a longer clause and may only become fewer.
#[test]
fn status_messages_lead_within_forty_characters() {
    const LONG_FIRST_CLAUSE: [&str; 200] = [
        "aids-difficult-no-list",
        "aids-reading-level-too-short",
        "app-terminal-only",
        "app-window-only",
        "audio-confirm",
        "audio-no-engine",
        "audio-no-ffmpeg",
        "batch-busy",
        "batch-choose-output",
        "batch-confirm",
        "batch-confirm-beside",
        "batch-inaccessible",
        "batch-progress",
        "book-audio-rate",
        "braille-view-not-in-build",
        "browse-archive-too-deep",
        "browse-archive-unreadable",
        "browse-cannot-read",
        "browse-keys",
        "browse-no-archive-folder",
        "browse-preview-archive-folder",
        "browse-read-only",
        "buttons-added",
        "buttons-no-name",
        "cards-anki-skipped-media",
        "cards-anki-skipped-other",
        "cards-nothing-to-make",
        "changes-none",
        "changes-save-not-in-build",
        "changes-written-accepted",
        "changes-written-rejected",
        "citations-bibliography-inserted",
        "citations-nothing-to-list",
        "colors-palette-value",
        "common-recovery-writing-again",
        "component-feature-braille",
        "component-question",
        "components-token-kept",
        "continue-item",
        "cw-keying-midi-not-in-build",
        "cw-practice-koch-start",
        "cw-practice-not-yet",
        "cw-practice-progress",
        "cw-practice-unavailable",
        "cw-speed",
        "define-glossary-skipped",
        "define-no-dictionary",
        "define-title",
        "details-no-file",
        "dictation-lost",
        "dictation-model-question",
        "dictation-no-model",
        "dictation-not-typed",
        "disk-overwrite-question",
        "docs-not-found",
        "docs-read-only",
        "edit-not-editing",
        "edit-range-out-of-text",
        "edit-recovery-discarded",
        "edit-wikilink-anchor-intro",
        "edit-wikilink-no-notes",
        "font-download-question",
        "gui-braille-caption-fell-back",
        "gui-colors-help",
        "gui-colors-help-system",
        "gui-colors-reset-all-help",
        "gui-colors-value",
        "gui-crashed",
        "gui-find-replace-all-help",
        "gui-find-replace-help",
        "gui-folder-no-dialog",
        "gui-prompt-hint",
        "gui-prompt-path-hint",
        "gui-prompt-show-hint",
        "gui-question-hint",
        "gui-reading-form-help",
        "gui-reading-wcag-help",
        "gui-setting-value-hint",
        "gui-settings-press-enter",
        "gui-settings-saved-hint",
        "gui-uia-unavailable",
        "gui-voices-engine-help",
        "gui-voices-favorite-help",
        "gui-voices-fetch-help",
        "gui-voices-hint",
        "gui-voices-language-help",
        "gui-voices-preview-help",
        "gui-voices-remove-help",
        "gui-voices-use-help",
        "helper-question",
        "language-voice-kept",
        "language-voices-loading",
        "lean-citations-not-in-build",
        "lean-publish-not-in-build",
        "links-back-to-footnote-reference",
        "links-file-not-found",
        "links-footnote-unreferenced",
        "links-no-heading",
        "lint-link-reference",
        "lint-not-markdown",
        "lists-citations-title-filtered",
        "lists-templates-title",
        "live-broker-no-folder",
        "live-broker-nobody",
        "live-checkin-through-title",
        "live-connected-tailscale",
        "live-document-shown",
        "live-field-speak-off",
        "live-file-not-read",
        "live-file-note-join-code",
        "live-file-note-password",
        "live-forget-title",
        "live-form-choice-label",
        "live-form-saved-beside",
        "live-log-empty-not-saved",
        "live-log-nowhere",
        "live-no-passwords",
        "live-not-in-build",
        "live-password-kept",
        "live-qr-unreadable",
        "live-save-file-done",
        "live-setup-ca-file-needed",
        "live-setup-host-needed",
        "live-share-live-document",
        "live-values-full",
        "notes-bookmark-name-taken",
        "notes-choose-bookmark-rename",
        "notes-collect-saved",
        "notes-palette-row",
        "notes-study-sheet-saved-both",
        "notes-study-sheet-saved-highlights",
        "notes-study-sheet-saved-notes",
        "opening-damaged-odt",
        "opening-no-file-in",
        "opening-rar",
        "opening-still",
        "playback-caps-no-volume",
        "playback-repeat-slower",
        "profile-dropped",
        "profile-replaced",
        "profiles-read-failed",
        "profiles-save-new",
        "prompt-citation-locator",
        "prompt-export-settings",
        "prompt-github-token",
        "prompt-go-to-pages",
        "prompt-live-password-any",
        "prompt-profiles-export",
        "prompt-table-size",
        "publish-server-failed",
        "publish-still-exporting",
        "publish-still-previewing",
        "queue-no-file",
        "queue-read-failed",
        "reading-form-spacing-generous-done",
        "relations-no-other-documents",
        "relocate-lost",
        "reveal-no-dictation",
        "review-intro",
        "review-stats-off",
        "settings-all-opened",
        "settings-applies-engine",
        "settings-applies-engines",
        "settings-applies-not-engine",
        "settings-clamped",
        "settings-outside-kept",
        "settingsio-backend-next-start",
        "settingsio-import-question",
        "settingsio-import-question-names",
        "settingsio-no-persistence",
        "spell-added-for-session",
        "stats-clear-question",
        "stats-current-none",
        "stats-total",
        "status-cw-rate-farnsworth-spoken",
        "summary-intro-sampled",
        "tables-position",
        "tasks-launch-off",
        "tasks-lookup-stopped",
        "templates-intro",
        "text-summary-range",
        "tui-clip-failed",
        "tui-clip-system",
        "update-automatic-question",
        "update-error-no-package",
        "vault-no-file",
        "vault-unresolved-intro",
        "voice-catalog-fetched",
        "voice-chosen-rate",
        "voice-fetch-catalog-question",
        "voice-more-ready",
        "voice-no-data-folder",
        "voice-not-in-list",
        "voice-only-piper-removable",
        "voice-preview-engine-failed",
        "voice-preview-unavailable",
        "voice-sample",
        "voices-fetch-row",
        "writes-bookmark-not-saved",
        "writes-not-written-in-time",
    ];
    let c = super::Catalog::parse("en", ENGLISH).expect("English parses");
    let mut long = BTreeMap::new();
    for id in c.ids() {
        if !is_status(id) {
            continue;
        }
        let text = rendered(&c, id);
        let first = first_clause(&text);
        let n = first.chars().count();
        if n > 40 {
            long.insert(id.to_owned(), format!("{n}: {first}"));
        }
    }
    ratchet(
        "a first clause over 40 characters (messages.md, rule 11)",
        "LONG_FIRST_CLAUSE",
        &LONG_FIRST_CLAUSE,
        &long,
    );
}

/// Rule 1: a status or announcement message starts with the fact, not a
/// lead-in: not "The", "This", "Your", "A message on", or the brand. These
/// still start with one and may only become fewer.
#[test]
fn status_messages_start_with_the_fact() {
    const LEAD_IN: [&str; 75] = [
        "access-window-choice-reads-aloud",
        "access-window-mode-changed",
        "app-terminal-only",
        "app-window-only",
        "batch-report-failed",
        "braille-view-not-in-build",
        "browse-read-only",
        "cards-from-anki",
        "changes-not-a-change",
        "changes-not-a-comment",
        "changes-original-kept",
        "citations-library-empty",
        "citations-none-yet",
        "common-link-no-address",
        "common-recovery-writing-again",
        "components-token-gh-still",
        "cw-practice-unavailable",
        "define-dictionary-damaged",
        "define-glossary-problem",
        "define-no-dictionary",
        "details-no-file",
        "dictation-cannot-pause",
        "docs-read-only",
        "edit-wikilink-whole",
        "gui-chooser-no-dialog",
        "gui-colors-help-system",
        "gui-crashed",
        "gui-folder-no-dialog",
        "gui-open-no-dialog",
        "language-voice-changed",
        "language-voices-loading",
        "library-empty",
        "library-scan-stopped",
        "links-file-not-found",
        "lists-no-filter",
        "lists-no-headings",
        "live-broker-already",
        "live-document-one-view",
        "live-field-empty",
        "live-log-empty-not-saved",
        "live-log-saved-already",
        "live-newest-none",
        "live-qr-image-error",
        "live-qr-other-text",
        "live-qr-too-large",
        "live-qr-too-long",
        "live-read-only",
        "live-setup-port-bad",
        "live-values-full",
        "live-values-no-log",
        "live-values-no-table",
        "nav-no-chapters",
        "pages-none",
        "preview-being-written",
        "profiles-read-failed",
        "publish-report-failed",
        "queue-empty",
        "relocate-changed",
        "restart-silent-now",
        "restart-silent-use-key",
        "review-stats-off",
        "settingsio-backed-up",
        "settingsio-backend-next-start",
        "stats-current",
        "stats-current-none",
        "sync-status-this-computer",
        "tui-title",
        "vault-no-file",
        "vault-no-note-tags",
        "vault-no-properties",
        "voice-catalog-fetched",
        "voice-download-stopped",
        "voice-ready",
        "voice-sample",
        "voice-still-loading",
    ];
    let lead = regex::Regex::new(r"^(The|This|Your|A message on|textweaver)\b").unwrap();
    let c = super::Catalog::parse("en", ENGLISH).expect("English parses");
    let mut found = BTreeMap::new();
    for id in c.ids() {
        if !is_status(id) {
            continue;
        }
        let text = rendered(&c, id);
        if let Some(m) = lead.find(text.trim_start()) {
            found.insert(id.to_owned(), m.as_str().to_owned());
        }
    }
    ratchet(
        "a lead-in before the fact (messages.md, rule 1)",
        "LEAD_IN",
        &LEAD_IN,
        &found,
    );
}
