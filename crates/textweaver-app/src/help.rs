//! Keyboard help, the command palette, and the help screen, all generated
//! from the keymap so they cannot drift from the real bindings (ADR-0006).

use textweaver_keymap::{ActionId, Category, Keymap};

use crate::app::{App, ListKind};
use crate::command::{Effect, NoteCommand};

/// Category order in help (the order `Category` declares).
const CATEGORIES: [Category; 9] = [
    Category::Reading,
    Category::Navigation,
    Category::SpeechCursor,
    Category::Voice,
    Category::Search,
    Category::Bookmarks,
    Category::File,
    Category::Editing,
    Category::View,
];

/// The chords bound to `action`, joined for reading aloud: `". or Alt+."`.
/// A key bound in two layers (Tab in browse and in Speech Cursor mode) is
/// named once, not "Tab or Tab".
pub fn chords_text(keymap: &Keymap, action: ActionId) -> String {
    let mut chords: Vec<String> = Vec::new();
    for c in keymap.chords_for(action) {
        let s = c.to_string();
        if !chords.contains(&s) {
            chords.push(s);
        }
    }
    if chords.is_empty() && action.is_palette_command() {
        "the command palette".into()
    } else if chords.is_empty() {
        "not bound".into()
    } else {
        chords.join(" or ")
    }
}

/// The one key that best stands for `action`: its single key (a browse
/// key such as `h`) while single-key shortcuts are on, else its first
/// chord, which still works with them off; `None` without keys.
fn main_chord(keymap: &Keymap, action: ActionId) -> Option<textweaver_keymap::KeyChord> {
    let chords = keymap.chords_for(action);
    let single = chords.iter().find(|c| c.is_text_input());
    let chord = chords.iter().find(|c| !c.is_text_input());
    let pick = if keymap.character_keys() {
        single.or(chord)
    } else {
        chord.or(single)
    };
    pick.or(chords.first()).copied()
}

/// At most two keys for `action`, for the help read aloud: the main key
/// (the main chord) and one chord that works with single-key shortcuts
/// off, such as "Space or Alt+P" and "h or Alt+H". The `?` list has every
/// key; five of them in a row cannot be followed by ear.
pub fn short_chords_text(keymap: &Keymap, action: ActionId) -> String {
    let Some(main) = main_chord(keymap, action) else {
        return chords_text(keymap, action);
    };
    if !keymap.character_keys() && main.is_text_input() {
        // Only single keys, and they are off: the palette is the way.
        return "the command palette".to_owned();
    }
    let chords = keymap.chords_for(action);
    let other = chords.iter().find(|c| **c != main && !c.is_text_input());
    match other {
        Some(o) => format!("{main} or {o}"),
        None => main.to_string(),
    }
}

/// One key for `action`, as it is spoken: "Control O", "F1", "Space"
/// (the main chord); "the command palette" for an action without keys.
/// For messages that name a key in prose, such as "Press Control O to open
/// one."
pub fn spoken_key(keymap: &Keymap, action: ActionId) -> String {
    main_chord(keymap, action).map_or_else(|| "the command palette".to_owned(), |c| c.spoken())
}

/// One key for `action`, written: "Ctrl+O", "h", "1" (the main chord);
/// "the command palette" for an action without keys. For help lines that
/// list many keys, where every chord of each would be too much.
pub fn key_text(keymap: &Keymap, action: ActionId) -> String {
    main_chord(keymap, action).map_or_else(|| "the command palette".to_owned(), |c| c.to_string())
}

/// Every action with its category and keys, in help order.
pub fn help_entries(keymap: &Keymap) -> Vec<(ActionId, String)> {
    let mut out = Vec::new();
    for cat in CATEGORIES {
        for &a in ActionId::ALL.iter().filter(|a| a.category() == cat) {
            out.push((
                a,
                format!("{}: {}. {}", cat.title(), a.help(), chords_text(keymap, a)),
            ));
        }
    }
    // Categories added later still appear.
    for &a in ActionId::ALL {
        if !CATEGORIES.contains(&a.category()) {
            out.push((
                a,
                format!(
                    "{}: {}. {}",
                    a.category().title(),
                    a.help(),
                    chords_text(keymap, a)
                ),
            ));
        }
    }
    out
}

fn normalize(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .collect()
}

/// Actions matching a palette query: ids that start with it first, then
/// ids or help texts containing every word of it.
pub fn palette_matches(query: &str) -> Vec<ActionId> {
    let q = normalize(query);
    if q.is_empty() {
        return ActionId::ALL.to_vec();
    }
    let words: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let mut prefix = Vec::new();
    let mut other = Vec::new();
    for &a in ActionId::ALL {
        let id = a.id();
        let help = a.help().to_lowercase();
        if id.starts_with(&q) {
            prefix.push(a);
        } else if id.contains(&q)
            || words
                .iter()
                .all(|w| help.contains(w.as_str()) || id.contains(w.as_str()))
        {
            other.push(a);
        }
    }
    prefix.extend(other);
    prefix
}

/// The action a palette answer names: an exact id, else the best match.
pub fn resolve_command(text: &str) -> Option<ActionId> {
    ActionId::from_id(&normalize(text)).or_else(|| palette_matches(text).first().copied())
}

impl App {
    /// Candidates for the command palette as `(id, "id: help")` pairs.
    pub fn palette_candidates(&self, query: &str) -> Vec<(ActionId, String)> {
        palette_matches(query)
            .into_iter()
            .map(|a| {
                (
                    a,
                    format!("{}: {}. {}", a.id(), a.help(), chords_text(&self.keymap, a)),
                )
            })
            .collect()
    }

    pub(crate) fn run_named_command(&mut self, text: &str) -> Vec<Effect> {
        if text.trim().is_empty() {
            self.note("Cancelled.");
            return vec![Effect::Redraw];
        }
        if let Some(c) = crate::command::NoteCommand::from_name(text) {
            return self.notes_command(c);
        }
        match resolve_command(text) {
            Some(ActionId::CommandPalette) => vec![Effect::Redraw],
            Some(a) => self.action(a),
            None => {
                self.error(&format!("Unknown command: {text}."));
                vec![Effect::Redraw]
            }
        }
    }

    pub(crate) fn keyboard_help(&mut self) -> Vec<Effect> {
        let entries = help_entries(&self.keymap);
        let (actions, items): (Vec<ActionId>, Vec<String>) = entries.into_iter().unzip();
        let n = items.len();
        self.list = Some(ListKind::Actions(actions));
        self.tell(&format!(
            "Keyboard shortcuts, {n} commands. Up and Down move, Enter runs, Escape closes."
        ));
        vec![Effect::ShowList {
            title: "Keyboard shortcuts".into(),
            items,
        }]
    }

    pub(crate) fn help(&mut self) -> Vec<Effect> {
        let k = |a| short_chords_text(&self.keymap, a);
        let one = |a| key_text(&self.keymap, a);
        let x = |c: NoteCommand| {
            let chords: Vec<String> = crate::extra::extra_chords(c)
                .iter()
                .map(ToString::to_string)
                .collect();
            if chords.is_empty() {
                format!("the command {}", c.name().replace('_', " "))
            } else {
                chords.join(" or ")
            }
        };
        let items = vec![
            "textweaver reads documents aloud. Keys below are the current bindings.".to_owned(),
            format!(
                "Open a document: {}. Library and recent files: {}.",
                k(ActionId::Open),
                k(ActionId::OpenLibrary)
            ),
            format!("Play or pause: {}.", k(ActionId::PlayPause)),
            format!("Read from the cursor: {}.", k(ActionId::ReadFromCursor)),
            format!("Stop: {}.", k(ActionId::Stop)),
            format!(
                "Next and previous sentence: {} and {}.",
                k(ActionId::NextSentence),
                k(ActionId::PreviousSentence)
            ),
            format!(
                "Next and previous paragraph: {} and {}.",
                k(ActionId::NextParagraph),
                k(ActionId::PreviousParagraph)
            ),
            format!(
                "Next and previous heading: {} and {}. Heading at a level: {} to {}, with Shift for the previous one.",
                k(ActionId::SkipNextHeading),
                k(ActionId::SkipPreviousHeading),
                one(ActionId::NextHeadingLevel1),
                one(ActionId::NextHeadingLevel6)
            ),
            format!(
                "Read from the next and previous heading: {} and {}.",
                k(ActionId::NextHeading),
                k(ActionId::PreviousHeading)
            ),
            format!(
                "Quick keys, as in NVDA and JAWS: list {}, list item {}, table {}, link {}, block quote {}, separator {}, graphic {}, section {}. Shift with the key goes to the previous one.",
                one(ActionId::NextList),
                one(ActionId::NextListItem),
                one(ActionId::NextTable),
                one(ActionId::NextLink),
                one(ActionId::NextBlockQuote),
                one(ActionId::NextSeparator),
                one(ActionId::NextGraphic),
                one(ActionId::NextChapter)
            ),
            format!(
                "Speech Cursor, line by line: {}.",
                k(ActionId::SpeechCursorToggle)
            ),
            format!("Find: {}.", k(ActionId::Find)),
            format!("Add a bookmark: {}.", k(ActionId::AddBookmark)),
            format!(
                "Back and forward through your jumps: {} and {}.",
                k(ActionId::HistoryBack),
                k(ActionId::HistoryForward)
            ),
            format!(
                "Faster and slower: {} and {}.",
                k(ActionId::RateUp),
                k(ActionId::RateDown)
            ),
            format!("Where am I: {}.", k(ActionId::SayPosition)),
            format!(
                "Notes: add {}, list {}, next and previous {} and {}, delete the one at the cursor {}. In the list, Delete deletes and F2 edits.",
                k(ActionId::AddNote),
                k(ActionId::ListNotes),
                k(ActionId::NextNote),
                k(ActionId::PreviousNote),
                k(ActionId::DeleteNote)
            ),
            format!(
                "Highlight the selection or sentence, or remove a highlight: {}. List highlights: {}.",
                k(ActionId::HighlightSelection),
                x(NoteCommand::ListHighlights)
            ),
            "Bookmarks list: Delete deletes a bookmark, F2 renames it.".to_owned(),
            format!(
                "Edit the document: {}. Save: {}. Save as: {}. New document: {}.",
                k(ActionId::ToggleEditMode),
                k(ActionId::Save),
                k(ActionId::SaveAs),
                k(ActionId::NewDocument)
            ),
            format!(
                "While editing: undo {}, redo {}, bold {}. Every formatting command is in the keyboard shortcuts.",
                k(ActionId::Undo),
                k(ActionId::Redo),
                k(ActionId::Bold)
            ),
            format!(
                "Outline of the headings, type to filter: {}. Follow a link or footnote: {}.",
                k(ActionId::Outline),
                k(ActionId::FollowLink)
            ),
            format!(
                "Tables: {} and {} move by row, {} and {} by cell.",
                k(ActionId::TableNextRow),
                k(ActionId::TablePreviousRow),
                k(ActionId::TableNextColumn),
                k(ActionId::TablePreviousColumn)
            ),
            format!(
                "Citations while editing: insert {}, add a reference by DOI or ISBN {}. Spelling: next and previous misspelling {} and {}, suggestions {}.",
                k(ActionId::InsertCitation),
                k(ActionId::AddReference),
                k(ActionId::NextMisspelling),
                k(ActionId::PreviousMisspelling),
                k(ActionId::SpellingSuggestions)
            ),
            "Export to HTML, PDF, Word, EPUB, or braille, preview in the browser, and start from a template: type export, preview, or template in the command palette.".to_owned(),
            format!(
                "How much is said: {}. How much punctuation: {}.",
                k(ActionId::CycleVerbosity),
                k(ActionId::CyclePunctuation)
            ),
            format!(
                "Choose a voice: {}. Restart speech if it stops: {}.",
                k(ActionId::ChooseVoice),
                k(ActionId::RestartSpeech)
            ),
            format!(
                "With a screen reader, who speaks: {} cycles self-voicing, hybrid, and screen reader mode.",
                k(ActionId::CycleAccessMode)
            ),
            format!(
                "Single-key shortcuts on or off, for dictation: {}. Settings: {}.",
                k(ActionId::ToggleCharacterKeys),
                k(ActionId::Settings)
            ),
            format!("All keyboard shortcuts: {}.", k(ActionId::KeyboardHelp)),
            format!("Run any command by name: {}.", k(ActionId::CommandPalette)),
            format!(
                "Quit, saving your place: {}, then y to confirm; n, a, or Escape cancels.",
                k(ActionId::Quit)
            ),
        ];
        self.list = Some(ListKind::Info);
        self.tell("Help. Up and Down move, Escape closes.");
        vec![Effect::ShowList {
            title: "Help".into(),
            items,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_key_per_action_spoken_and_written() {
        use textweaver_keymap::{Frontend, Platform};
        let map = Keymap::defaults(Platform::Linux, Frontend::Terminal);
        assert_eq!(spoken_key(&map, ActionId::Open), "Control O");
        assert_eq!(key_text(&map, ActionId::Open), "Ctrl+O");
        // The single key wins over a chord defined before it.
        assert_eq!(spoken_key(&map, ActionId::PlayPause), "Space");
        assert_eq!(key_text(&map, ActionId::NextChapter), "d");
        assert_eq!(key_text(&map, ActionId::NextHeadingLevel1), "1");
        assert_eq!(spoken_key(&map, ActionId::ExportPdf), "the command palette");
        // The short form: the main key and one chord, never two single
        // keys, and a key bound in two layers only once.
        assert_eq!(
            short_chords_text(&map, ActionId::NextParagraph),
            "p or Ctrl+P"
        );
        assert_eq!(
            short_chords_text(&map, ActionId::PlayPause),
            "Space or Alt+P"
        );
        assert_eq!(
            short_chords_text(&map, ActionId::NextSentence),
            "Alt+. or Alt+Down"
        );
        assert_eq!(short_chords_text(&map, ActionId::SpeechCursorToggle), "Tab");
        assert_eq!(chords_text(&map, ActionId::SpeechCursorToggle), "Tab");
        assert_eq!(short_chords_text(&map, ActionId::AddBookmark), "m");
        // With single-key shortcuts off, the chord comes first and a
        // single key is not offered.
        let mut off = map.clone();
        off.set_character_keys(false);
        assert_eq!(spoken_key(&off, ActionId::PlayPause), "Alt P");
        assert_eq!(
            short_chords_text(&off, ActionId::NextParagraph),
            "Ctrl+P or Ctrl+Down"
        );
        assert_eq!(
            short_chords_text(&off, ActionId::AddBookmark),
            "the command palette"
        );
    }

    #[test]
    fn palette_finds_by_id_and_help() {
        assert_eq!(
            resolve_command("next_sentence"),
            Some(ActionId::NextSentence)
        );
        assert_eq!(
            resolve_command("Next Sentence"),
            Some(ActionId::NextSentence)
        );
        assert!(palette_matches("bookmark").contains(&ActionId::AddBookmark));
        assert_eq!(resolve_command("zzzz nothing"), None);
    }
}
