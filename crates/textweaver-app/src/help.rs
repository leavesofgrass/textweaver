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
pub fn chords_text(keymap: &Keymap, action: ActionId) -> String {
    let chords: Vec<String> = keymap
        .chords_for(action)
        .iter()
        .map(ToString::to_string)
        .collect();
    if chords.is_empty() {
        "not bound".into()
    } else {
        chords.join(" or ")
    }
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
        let k = |a| chords_text(&self.keymap, a);
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
                "Read the next and previous heading: {} and {}.",
                k(ActionId::NextHeading),
                k(ActionId::PreviousHeading)
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
                "Notes: add {}, list {}, next and previous {} and {}. In the list, Delete deletes and F2 edits.",
                x(NoteCommand::Add),
                x(NoteCommand::List),
                x(NoteCommand::Next),
                x(NoteCommand::Previous)
            ),
            format!(
                "Highlight the selection or sentence, or remove a highlight: {}. List highlights: {}.",
                x(NoteCommand::ToggleHighlight),
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
            format!("All keyboard shortcuts: {}.", k(ActionId::KeyboardHelp)),
            format!("Run any command by name: {}.", k(ActionId::CommandPalette)),
            format!("Quit, saving your place: {}.", k(ActionId::Quit)),
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
