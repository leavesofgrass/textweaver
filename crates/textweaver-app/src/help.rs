//! Keyboard help, the command palette, and the help screen, all generated
//! from the keymap so they cannot drift from the real bindings (ADR-0006).
//!
//! # Keys in messages: written and spoken (Wave 4, W4h)
//!
//! A message that names a key carries it in both forms, between private
//! use characters: `Ctrl+S` for the status line, the screen reader, and
//! the lists drawn on screen, and `Control S` for textweaver's own voice
//! ([`KeyChord::spoken`]), which names punctuation, so "Alt period" is
//! heard even with punctuation off, where `Alt+.` was heard as "alt".
//! [`App::key`] and [`App::keys`] build such keys from the keymap; the
//! announcement paths ([`written_text`] and [`spoken_text`]) pick the form
//! for each destination, and lists and effects handed to a frontend never
//! carry the marks.
//!
//! [`KeyChord::spoken`]: textweaver_keymap::KeyChord::spoken

use std::borrow::Cow;

use textweaver_keymap::{ActionId, Category, Key, KeyChord, Keymap, Modifiers};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

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
    chords_text_in(&Catalog::english(), keymap, action)
}

/// [`chords_text`] in the catalog's language.
pub fn chords_text_in(c: &Catalog, keymap: &Keymap, action: ActionId) -> String {
    let mut chords: Vec<String> = Vec::new();
    for ch in keymap.chords_for(action) {
        let s = ch.to_string();
        if !chords.contains(&s) {
            chords.push(s);
        }
    }
    if chords.is_empty() && action.is_palette_command() {
        c.tr("help-the-command-palette")
    } else if chords.is_empty() {
        c.tr("help-not-bound")
    } else {
        join_or(c, chords)
    }
}

/// Keys joined with "or": "a or b or c".
fn join_or(c: &Catalog, items: Vec<String>) -> String {
    let mut items = items.into_iter().rev();
    let Some(mut out) = items.next() else {
        return String::new();
    };
    for before in items {
        out = c.fmt("help-or", &args!["a" => before, "b" => out]);
    }
    out
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
    let c = Catalog::english();
    let Some(main) = main_chord(keymap, action) else {
        return chords_text(keymap, action);
    };
    if !keymap.character_keys() && main.is_text_input() {
        // Only single keys, and they are off: the palette is the way.
        return c.tr("help-the-command-palette");
    }
    let chords = keymap.chords_for(action);
    let other = chords.iter().find(|x| **x != main && !x.is_text_input());
    match other {
        Some(o) => join_or(&c, vec![main.to_string(), o.to_string()]),
        None => main.to_string(),
    }
}

/// One key for `action`, as it is spoken: "Control O", "F1", "Space"
/// (the main chord); "the command palette" for an action without keys.
/// For messages that name a key in prose, such as "Press Control O to open
/// one."
pub fn spoken_key(keymap: &Keymap, action: ActionId) -> String {
    main_chord(keymap, action).map_or_else(
        || Catalog::english().tr("help-the-command-palette"),
        |c| c.spoken(),
    )
}

/// One key for `action`, written: "Ctrl+O", "h", "1" (the main chord);
/// "the command palette" for an action without keys. For help lines that
/// list many keys, where every chord of each would be too much.
pub fn key_text(keymap: &Keymap, action: ActionId) -> String {
    main_chord(keymap, action).map_or_else(
        || Catalog::english().tr("help-the-command-palette"),
        |c| c.to_string(),
    )
}

/// How textweaver's own voice says `chord` in the catalog's language:
/// "Control O", "Alt period". English gives [`KeyChord::spoken`]'s words.
pub fn spoken_chord(c: &Catalog, chord: &KeyChord) -> String {
    let mut parts: Vec<String> = Vec::new();
    if chord.mods.contains(Modifiers::CTRL) {
        parts.push(c.tr("keyname-control"));
    }
    if chord.mods.contains(Modifiers::META) {
        parts.push(c.tr("keyname-command"));
    }
    if chord.mods.contains(Modifiers::ALT) {
        parts.push(c.tr("keyname-alt"));
    }
    let key = match chord.key {
        Key::Char(ch) if ch.is_ascii_uppercase() => {
            parts.push(c.tr("keyname-shift"));
            ch.to_string()
        }
        Key::Char(ch) if ch.is_ascii_lowercase() => ch.to_ascii_uppercase().to_string(),
        Key::Char(ch) => match punctuation_key(ch) {
            Some(id) => c.tr(&format!("keyname-{id}")),
            None => ch.to_string(),
        },
        k => {
            if chord.mods.contains(Modifiers::SHIFT) {
                parts.push(c.tr("keyname-shift"));
            }
            match k {
                Key::F(n) => format!("F{n}"),
                Key::Char(ch) => ch.to_string(),
                other => c.tr(&format!("keyname-{}", named_key_id(other))),
            }
        }
    };
    parts.push(key);
    parts.join(" ")
}

/// The id part of a punctuation key's spoken name (`keyname-period`).
fn punctuation_key(ch: char) -> Option<&'static str> {
    Some(match ch {
        '.' => "period",
        ',' => "comma",
        ';' => "semicolon",
        ':' => "colon",
        '\'' => "apostrophe",
        '"' => "quote",
        '`' => "grave-accent",
        '~' => "tilde",
        '!' => "exclamation-mark",
        '?' => "question-mark",
        '@' => "at-sign",
        '#' => "number-sign",
        '$' => "dollar-sign",
        '%' => "percent",
        '^' => "caret",
        '&' => "ampersand",
        '*' => "asterisk",
        '(' => "left-parenthesis",
        ')' => "right-parenthesis",
        '[' => "left-bracket",
        ']' => "right-bracket",
        '{' => "left-brace",
        '}' => "right-brace",
        '<' => "less-than",
        '>' => "greater-than",
        '+' => "plus",
        '-' => "minus",
        '=' => "equals",
        '_' => "underscore",
        '/' => "slash",
        '\\' => "backslash",
        '|' => "vertical-bar",
        _ => return None,
    })
}

/// The id part of a named key's spoken name (`keyname-page-up`).
fn named_key_id(k: Key) -> &'static str {
    match k {
        Key::PageUp => "page-up",
        Key::PageDown => "page-down",
        Key::Up => "up-arrow",
        Key::Down => "down-arrow",
        Key::Left => "left-arrow",
        Key::Right => "right-arrow",
        Key::Escape => "escape",
        Key::Space => "space",
        Key::Enter => "enter",
        Key::Tab => "tab",
        Key::Backspace => "backspace",
        Key::Delete => "delete",
        Key::Insert => "insert",
        Key::Home => "home",
        Key::End => "end",
        Key::F(_) | Key::Char(_) => "space",
    }
}

/// Opens a key named in a message: the written form follows.
const KEY_OPEN: char = '\u{E000}';
/// Between a key's written and spoken forms.
const KEY_SPLIT: char = '\u{E001}';
/// Closes a key named in a message.
const KEY_CLOSE: char = '\u{E002}';

/// `chord` for a message, in both forms (see the module notes), spoken in
/// the catalog's language.
pub(crate) fn mark_chord(c: &Catalog, chord: &KeyChord) -> String {
    format!(
        "{KEY_OPEN}{chord}{KEY_SPLIT}{}{KEY_CLOSE}",
        spoken_chord(c, chord)
    )
}

/// One key for `action`, marked for a message: the main chord (the single
/// key while single-key shortcuts are on), else "the command palette".
/// Frontends pass messages built with it to [`App::announce`]; the status
/// line shows "Ctrl+O" and textweaver's voice says "Control O".
pub fn named_key(keymap: &Keymap, action: ActionId) -> String {
    named_key_in(&Catalog::english(), keymap, action)
}

/// [`named_key`] in the catalog's language ([`App::catalog`]).
pub fn named_key_in(c: &Catalog, keymap: &Keymap, action: ActionId) -> String {
    main_chord(keymap, action).map_or_else(
        || c.tr("help-the-command-palette"),
        |ch| mark_chord(c, &ch),
    )
}

/// Every chord bound to `action`, marked for a message and joined with
/// "or" (as [`chords_text`], which gives the written form only).
pub(crate) fn named_keys(c: &Catalog, keymap: &Keymap, action: ActionId) -> String {
    let mut chords: Vec<KeyChord> = Vec::new();
    for ch in keymap.chords_for(action) {
        if !chords.contains(&ch) {
            chords.push(ch);
        }
    }
    if chords.is_empty() {
        return chords_text_in(c, keymap, action);
    }
    join_or(c, chords.iter().map(|ch| mark_chord(c, ch)).collect())
}

/// At most two keys for `action`, marked ([`short_chords_text`]).
fn named_short_keys(c: &Catalog, keymap: &Keymap, action: ActionId) -> String {
    let Some(main) = main_chord(keymap, action) else {
        return chords_text_in(c, keymap, action);
    };
    if !keymap.character_keys() && main.is_text_input() {
        return c.tr("help-the-command-palette");
    }
    let chords = keymap.chords_for(action);
    let other = chords.iter().find(|x| **x != main && !x.is_text_input());
    match other {
        Some(o) => join_or(c, vec![mark_chord(c, &main), mark_chord(c, o)]),
        None => mark_chord(c, &main),
    }
}

/// An action's one-line help in the catalog's language (`action-*`
/// messages; English is the keymap's own [`ActionId::help`]).
pub fn action_help(c: &Catalog, a: ActionId) -> String {
    let id = format!("action-{}", a.id().replace('_', "-"));
    if c.has(&id) {
        c.tr(&id)
    } else {
        a.help().to_owned()
    }
}

/// A help category's title in the catalog's language.
pub fn category_title(c: &Catalog, cat: Category) -> String {
    let key = match cat {
        Category::Reading => "reading",
        Category::Navigation => "navigation",
        Category::SpeechCursor => "speech-cursor",
        Category::Voice => "voice",
        Category::Search => "search",
        Category::Bookmarks => "bookmarks",
        Category::File => "file",
        Category::Editing => "editing",
        Category::View => "view",
    };
    c.tr(&format!("category-{key}"))
}

/// Keeps one form of each marked key: the written one, or the spoken one.
fn pick_form(text: &str, spoken: bool) -> Cow<'_, str> {
    if !text.contains(KEY_OPEN) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find(KEY_OPEN) {
        out.push_str(&rest[..open]);
        let inner = &rest[open + KEY_OPEN.len_utf8()..];
        let Some(close) = inner.find(KEY_CLOSE) else {
            // An unclosed mark (never built here): keep the text as it is.
            out.push_str(inner);
            return Cow::Owned(out);
        };
        let key = &inner[..close];
        let (written, said) = key.split_once(KEY_SPLIT).unwrap_or((key, key));
        out.push_str(if spoken { said } else { written });
        rest = &inner[close + KEY_CLOSE.len_utf8()..];
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// `text` for the status line, the screen reader, and the screen: keys
/// in their written form ("Ctrl+S").
pub fn written_text(text: &str) -> Cow<'_, str> {
    pick_form(text, false)
}

/// `text` for textweaver's own voice: keys in their spoken form
/// ("Control S", "Alt period").
pub fn spoken_text(text: &str) -> Cow<'_, str> {
    pick_form(text, true)
}

/// Every action with its category and keys, in help order.
pub fn help_entries(keymap: &Keymap) -> Vec<(ActionId, String)> {
    entries_with(&Catalog::english(), keymap, chords_text_in)
}

/// [`help_entries`] with the keys named by `keys` (written, or marked for
/// the keyboard shortcuts list, whose focused item is spoken).
fn entries_with(
    c: &Catalog,
    keymap: &Keymap,
    keys: fn(&Catalog, &Keymap, ActionId) -> String,
) -> Vec<(ActionId, String)> {
    let entry = |a: ActionId| {
        c.fmt(
            "help-entry",
            &args![
                "category" => category_title(c, a.category()),
                "help" => action_help(c, a),
                "keys" => keys(c, keymap, a)
            ],
        )
    };
    let mut out = Vec::new();
    for cat in CATEGORIES {
        for &a in ActionId::ALL.iter().filter(|a| a.category() == cat) {
            out.push((a, entry(a)));
        }
    }
    // Categories added later still appear.
    for &a in ActionId::ALL {
        if !CATEGORIES.contains(&a.category()) {
            out.push((a, entry(a)));
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
    palette_matches_in(&Catalog::english(), query)
}

/// [`palette_matches`], also matching the help in the catalog's language.
pub fn palette_matches_in(c: &Catalog, query: &str) -> Vec<ActionId> {
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
        let translated = action_help(c, a).to_lowercase();
        let palette_name = normalize(&translated);
        if id.starts_with(&q) || palette_name.starts_with(&q) {
            prefix.push(a);
        } else if id.contains(&q)
            || words.iter().all(|w| {
                help.contains(w.as_str())
                    || translated.contains(w.as_str())
                    || id.contains(w.as_str())
            })
        {
            other.push(a);
        }
    }
    prefix.extend(other);
    prefix
}

/// The action a palette answer names: an exact id, else the best match.
pub fn resolve_command(text: &str) -> Option<ActionId> {
    resolve_command_in(&Catalog::english(), text)
}

/// [`resolve_command`], also by the help in the catalog's language.
pub fn resolve_command_in(c: &Catalog, text: &str) -> Option<ActionId> {
    ActionId::from_id(&normalize(text)).or_else(|| palette_matches_in(c, text).first().copied())
}

impl App {
    /// One key for `action` in a message ([`named_key`]): written on the
    /// status line, spoken by textweaver's voice.
    pub(crate) fn key(&self, action: ActionId) -> String {
        named_key_in(self.cat(), &self.keymap, action)
    }

    /// Every key for `action` in a message ([`named_keys`]).
    pub(crate) fn keys(&self, action: ActionId) -> String {
        named_keys(self.cat(), &self.keymap, action)
    }

    /// What is said for a command palette candidate, with its keys marked
    /// ([`App::palette_candidates`] gives the written form, to show).
    pub(crate) fn palette_said(&self, a: ActionId) -> String {
        self.msg_args(
            "help-palette-item",
            &args!["id" => a.id(), "help" => action_help(self.cat(), a), "keys" => self.keys(a)],
        )
    }

    /// Candidates for the command palette as `(id, "id: help")` pairs.
    pub fn palette_candidates(&self, query: &str) -> Vec<(ActionId, String)> {
        let c = self.cat();
        palette_matches_in(c, query)
            .into_iter()
            .map(|a| {
                (
                    a,
                    c.fmt(
                        "help-palette-item",
                        &args![
                            "id" => a.id(),
                            "help" => action_help(c, a),
                            "keys" => chords_text_in(c, &self.keymap, a)
                        ],
                    ),
                )
            })
            .collect()
    }

    pub(crate) fn run_named_command(&mut self, text: &str) -> Vec<Effect> {
        if text.trim().is_empty() {
            let msg = self.msg("common-cancelled");
            self.note(&msg);
            return vec![Effect::Redraw];
        }
        if let Some(c) = crate::command::NoteCommand::from_name(text) {
            return self.notes_command(c);
        }
        match resolve_command_in(self.cat(), text) {
            Some(ActionId::CommandPalette) => vec![Effect::Redraw],
            Some(a) => self.action(a),
            None => {
                let msg = self.msg_args("help-unknown-command", &args!["text" => text]);
                self.error(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    pub(crate) fn keyboard_help(&mut self) -> Vec<Effect> {
        let entries = entries_with(self.cat(), &self.keymap, named_keys);
        let (actions, items): (Vec<ActionId>, Vec<String>) = entries.into_iter().unzip();
        let n = items.len();
        self.list = Some(ListKind::Actions(actions));
        let msg = self.msg_args("help-shortcuts-intro", &args!["n" => n]);
        self.tell(&msg);
        vec![Effect::ShowList {
            title: self.msg("help-shortcuts-title"),
            items,
        }]
    }

    pub(crate) fn help(&mut self) -> Vec<Effect> {
        // Keys are marked: the list shows "Ctrl+O", the voice says
        // "Control O".
        let c = self.cat();
        let k = |a| named_short_keys(c, &self.keymap, a);
        let one = |a| named_key_in(c, &self.keymap, a);
        let x = |cmd: NoteCommand| {
            let chords: Vec<String> = crate::extra::extra_chords(cmd)
                .iter()
                .map(|ch| mark_chord(c, ch))
                .collect();
            if chords.is_empty() {
                c.fmt(
                    "help-the-command",
                    &args!["name" => cmd.name().replace('_', " ")],
                )
            } else {
                join_or(c, chords)
            }
        };
        let line = |id: &str, keys: &[(&str, String)]| {
            let values: Vec<(&str, textweaver_lexicon::i18n::Arg)> = keys
                .iter()
                .map(|(n, v)| (*n, textweaver_lexicon::i18n::Arg::from(v.as_str())))
                .collect();
            c.fmt(id, &values)
        };
        use ActionId as A;
        let items = vec![
            c.tr("help-about"),
            line("help-open", &[("open", k(A::Open)), ("library", k(A::OpenLibrary))]),
            line("help-play", &[("key", k(A::PlayPause))]),
            line("help-read-from-cursor", &[("key", k(A::ReadFromCursor))]),
            line("help-stop", &[("key", k(A::Stop))]),
            line(
                "help-sentences",
                &[("next", k(A::NextSentence)), ("previous", k(A::PreviousSentence))],
            ),
            line(
                "help-paragraphs",
                &[("next", k(A::NextParagraph)), ("previous", k(A::PreviousParagraph))],
            ),
            line(
                "help-headings",
                &[
                    ("next", k(A::SkipNextHeading)),
                    ("previous", k(A::SkipPreviousHeading)),
                    ("first", one(A::NextHeadingLevel1)),
                    ("last", one(A::NextHeadingLevel6)),
                ],
            ),
            line(
                "help-read-headings",
                &[("next", k(A::NextHeading)), ("previous", k(A::PreviousHeading))],
            ),
            line(
                "help-quick-keys",
                &[
                    ("list", one(A::NextList)),
                    ("item", one(A::NextListItem)),
                    ("table", one(A::NextTable)),
                    ("link", one(A::NextLink)),
                    ("quote", one(A::NextBlockQuote)),
                    ("separator", one(A::NextSeparator)),
                    ("graphic", one(A::NextGraphic)),
                    ("section", one(A::NextChapter)),
                ],
            ),
            line("help-speech-cursor", &[("key", k(A::SpeechCursorToggle))]),
            line("help-find", &[("key", k(A::Find))]),
            line("help-bookmark", &[("key", k(A::AddBookmark))]),
            line(
                "help-history",
                &[("back", k(A::HistoryBack)), ("forward", k(A::HistoryForward))],
            ),
            line("help-rate", &[("faster", k(A::RateUp)), ("slower", k(A::RateDown))]),
            line("help-where", &[("key", k(A::SayPosition))]),
            line(
                "help-repeat",
                &[("repeat", k(A::RepeatMessage)), ("status", k(A::SayStatus))],
            ),
            line(
                "help-notes",
                &[
                    ("add", k(A::AddNote)),
                    ("list", k(A::ListNotes)),
                    ("next", k(A::NextNote)),
                    ("previous", k(A::PreviousNote)),
                    ("delete", k(A::DeleteNote)),
                ],
            ),
            line(
                "help-highlights",
                &[
                    ("highlight", k(A::HighlightSelection)),
                    ("list", x(NoteCommand::ListHighlights)),
                ],
            ),
            c.tr("help-bookmarks-list"),
            line(
                "help-edit",
                &[
                    ("edit", k(A::ToggleEditMode)),
                    ("save", k(A::Save)),
                    ("saveas", k(A::SaveAs)),
                    ("new", k(A::NewDocument)),
                ],
            ),
            line(
                "help-editing",
                &[("undo", k(A::Undo)), ("redo", k(A::Redo)), ("bold", k(A::Bold))],
            ),
            line(
                "help-outline",
                &[("outline", k(A::Outline)), ("follow", k(A::FollowLink))],
            ),
            line(
                "help-tables",
                &[
                    ("nextrow", k(A::TableNextRow)),
                    ("previousrow", k(A::TablePreviousRow)),
                    ("nextcell", k(A::TableNextColumn)),
                    ("previouscell", k(A::TablePreviousColumn)),
                ],
            ),
            line(
                "help-citations",
                &[
                    ("insert", k(A::InsertCitation)),
                    ("reference", k(A::AddReference)),
                    ("next", k(A::NextMisspelling)),
                    ("previous", k(A::PreviousMisspelling)),
                    ("suggestions", k(A::SpellingSuggestions)),
                ],
            ),
            c.tr("help-export"),
            line(
                "help-verbosity",
                &[
                    ("verbosity", k(A::CycleVerbosity)),
                    ("punctuation", k(A::CyclePunctuation)),
                ],
            ),
            line(
                "help-voice",
                &[("voice", k(A::ChooseVoice)), ("restart", k(A::RestartSpeech))],
            ),
            line("help-access", &[("key", k(A::CycleAccessMode))]),
            line(
                "help-character-keys",
                &[("keys", k(A::ToggleCharacterKeys)), ("settings", k(A::Settings))],
            ),
            line("help-all-shortcuts", &[("key", k(A::KeyboardHelp))]),
            line("help-palette", &[("key", k(A::CommandPalette))]),
            line("help-quit", &[("key", k(A::Quit))]),
        ];
        let intro = c.tr("help-intro");
        let title = c.tr("help-title");
        self.list = Some(ListKind::Info);
        self.tell(&intro);
        vec![Effect::ShowList { title, items }]
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

    /// English help, categories, and spoken key names are the keymap's
    /// own, so the catalog cannot drift from them (and `cargo xtask
    /// keyboard` stays as it was).
    #[test]
    fn english_catalog_matches_the_keymap() {
        use textweaver_keymap::{Frontend, Platform};
        let c = Catalog::english();
        for &a in ActionId::ALL {
            let id = format!("action-{}", a.id().replace('_', "-"));
            assert!(c.has(&id), "en.ftl has no {id}");
            assert_eq!(action_help(&c, a), a.help(), "{id}");
        }
        for cat in CATEGORIES {
            assert_eq!(category_title(&c, cat), cat.title());
        }
        for platform in [Platform::Linux, Platform::Windows, Platform::MacOs] {
            for frontend in [Frontend::Terminal, Frontend::Gui] {
                let map = Keymap::defaults(platform, frontend);
                for &a in ActionId::ALL {
                    for ch in map.chords_for(a) {
                        assert_eq!(spoken_chord(&c, &ch), ch.spoken(), "{ch}");
                    }
                }
            }
        }
        let map = Keymap::defaults(Platform::Linux, Frontend::Terminal);
        assert_eq!(
            help_entries(&map)[0].1,
            "Reading: Play or pause reading from the current word. Alt+P or Space"
        );
        // Spanish help finds commands by its own words too.
        let es = Catalog::builtin("es").unwrap();
        assert_eq!(resolve_command_in(&es, "next_sentence"), Some(ActionId::NextSentence));
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
