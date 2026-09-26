//! Keymaps: defaults plus overrides, looked up by layer.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ActionId, KeyChord};

/// Where a binding is active.
///
/// Lookup order by mode: Browse mode checks `Browse` then `Global`; Speech
/// Cursor checks `SpeechCursor`, then `Browse`, then `Global`; edit mode
/// checks `Edit` then `Global` (unbound text keys type text).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// Active in every mode.
    Global,
    /// Reading (browse) mode, where single keys navigate.
    Browse,
    /// Speech Cursor mode.
    SpeechCursor,
    /// Edit mode.
    Edit,
}

impl Layer {
    /// Every layer.
    pub const ALL: [Layer; 4] = [
        Layer::Global,
        Layer::Browse,
        Layer::SpeechCursor,
        Layer::Edit,
    ];

    /// The layer for a one-letter prefix: `g`, `b`, `s`, `e`.
    pub fn from_prefix(p: &str) -> Option<Layer> {
        match p {
            "g" => Some(Layer::Global),
            "b" => Some(Layer::Browse),
            "s" => Some(Layer::SpeechCursor),
            "e" => Some(Layer::Edit),
            _ => None,
        }
    }

    /// The one-letter prefix used in default tables and `keymap.toml`.
    pub fn prefix(self) -> &'static str {
        match self {
            Layer::Global => "g",
            Layer::Browse => "b",
            Layer::SpeechCursor => "s",
            Layer::Edit => "e",
        }
    }

    /// A short name for help: "global", "browse", "speech cursor", "edit".
    pub fn name(self) -> &'static str {
        match self {
            Layer::Global => "global",
            Layer::Browse => "browse",
            Layer::SpeechCursor => "speech cursor",
            Layer::Edit => "edit",
        }
    }

    /// Layers searched, in order, when the app is in this layer's mode.
    pub fn lookup_order(self) -> &'static [Layer] {
        match self {
            Layer::Global => &[Layer::Global],
            Layer::Browse => &[Layer::Browse, Layer::Global],
            Layer::SpeechCursor => &[Layer::SpeechCursor, Layer::Browse, Layer::Global],
            Layer::Edit => &[Layer::Edit, Layer::Global],
        }
    }
}

/// Operating system, for platform-specific defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux and other Unix.
    Linux,
}

impl Platform {
    /// Every platform.
    pub const ALL: [Platform; 3] = [Platform::Windows, Platform::MacOs, Platform::Linux];

    /// The platform this binary was built for.
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }
}

/// Which frontend the keymap is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frontend {
    /// The native GUI (wave 3).
    Gui,
    /// The terminal UI.
    Terminal,
}

impl Frontend {
    /// Both frontends.
    pub const ALL: [Frontend; 2] = [Frontend::Terminal, Frontend::Gui];
}

/// One chord bound to one action in one layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Binding {
    /// The chord.
    pub chord: KeyChord,
    /// Where it is active.
    pub layer: Layer,
    /// What it does.
    pub action: ActionId,
}

/// Two or more actions reachable by the same chord in the same mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The chord.
    pub chord: KeyChord,
    /// The layers involved.
    pub layers: Vec<Layer>,
    /// The actions it would trigger.
    pub actions: Vec<ActionId>,
}

impl Conflict {
    /// A sentence describing the conflict, for warnings.
    pub fn describe(&self) -> String {
        let actions: Vec<&str> = self.actions.iter().map(|a| a.id()).collect();
        let layers: Vec<&str> = self.layers.iter().map(|l| l.name()).collect();
        format!(
            "{} is bound to {} ({} layer{})",
            self.chord,
            actions.join(" and "),
            layers.join(" and "),
            if layers.len() == 1 { "" } else { "s" }
        )
    }
}

/// A complete set of bindings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

/// Splits `g:Ctrl+P` into its layer and chord text. `:` alone, and strings
/// whose prefix is not a layer, have no layer.
fn split_layer(s: &str) -> (Option<Layer>, &str) {
    match s.split_once(':') {
        Some((p, rest)) if !rest.is_empty() => match Layer::from_prefix(p) {
            Some(layer) => (Some(layer), rest),
            None => (None, s),
        },
        _ => (None, s),
    }
}

fn parse_default(s: &str, platform: Platform, frontend: Frontend) -> Option<(Layer, KeyChord)> {
    let (layer, chord) = split_layer(s);
    let mut chord: KeyChord = chord.parse().ok()?;
    if frontend == Frontend::Gui && platform == Platform::MacOs {
        chord = chord.ctrl_to_meta();
    }
    Some((layer?, chord))
}

/// Default chord strings for `action` on `frontend`, with layer prefixes.
pub(crate) fn default_strings(action: ActionId, frontend: Frontend) -> Vec<&'static str> {
    let d = action.defaults();
    let own = match frontend {
        Frontend::Gui => d.gui,
        Frontend::Terminal => d.terminal,
    };
    own.iter().chain(d.shared).copied().collect()
}

impl Keymap {
    /// The default keymap.
    pub fn defaults(platform: Platform, frontend: Frontend) -> Self {
        let mut bindings = Vec::new();
        for &action in ActionId::ALL {
            for s in default_strings(action, frontend) {
                // The tables are static and every entry is checked by a test;
                // an entry that failed to parse would simply be missing.
                if let Some((layer, chord)) = parse_default(s, platform, frontend) {
                    bindings.push(Binding {
                        chord,
                        layer,
                        action,
                    });
                }
            }
        }
        Keymap { bindings }
    }

    /// Defaults with user overrides applied. Each override replaces all of
    /// an action's bindings (an empty list unbinds it).
    ///
    /// A chord may name its layer with a prefix (`"b:x"`, `"s:j"`,
    /// `"e:Ctrl+B"`, `"g:F5"`). Without one, a text key (a character or
    /// Space, at most with Shift) goes to the browse layer, or to Speech
    /// Cursor for an action bound only there, so it never stops typing in
    /// edit mode; any other chord goes to the action's first default layer
    /// that is not browse, else global.
    ///
    /// Returns warnings for unknown actions, unparsable chords, chords a
    /// terminal cannot deliver (terminal frontend), and conflicts the
    /// overrides introduce. Warnings never stop loading.
    pub fn with_overrides(
        platform: Platform,
        frontend: Frontend,
        overrides: &BTreeMap<String, Vec<String>>,
    ) -> (Self, Vec<String>) {
        let mut map = Keymap::defaults(platform, frontend);
        let before: Vec<Conflict> = map.conflicts();
        let mut warnings = Vec::new();
        for (id, chords) in overrides {
            let Some(action) = ActionId::from_id(id) else {
                warnings.push(format!("unknown action {id:?} in keymap.toml"));
                continue;
            };
            let default_layers: Vec<Layer> = map
                .bindings
                .iter()
                .filter(|b| b.action == action)
                .map(|b| b.layer)
                .collect();
            map.bindings.retain(|b| b.action != action);
            for c in chords {
                let (explicit, text) = split_layer(c);
                let chord = match text.parse::<KeyChord>() {
                    Ok(chord) => chord,
                    Err(e) => {
                        warnings.push(format!("{id}: {e}"));
                        continue;
                    }
                };
                if frontend == Frontend::Terminal
                    && let Some(why) = chord.terminal_limitation()
                {
                    warnings.push(format!("{id}: {chord}: {why}"));
                }
                let layer = explicit.unwrap_or_else(|| infer_layer(&chord, &default_layers));
                map.bindings.push(Binding {
                    chord,
                    layer,
                    action,
                });
            }
        }
        for c in map.conflicts() {
            if !before.contains(&c) {
                warnings.push(format!("keymap.toml: {}", c.describe()));
            }
        }
        (map, warnings)
    }

    /// Every binding.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// The action for `chord` in a mode whose layer is `mode`.
    pub fn lookup(&self, chord: &KeyChord, mode: Layer) -> Option<ActionId> {
        mode.lookup_order().iter().find_map(|layer| {
            self.bindings
                .iter()
                .find(|b| b.layer == *layer && b.chord == *chord)
                .map(|b| b.action)
        })
    }

    /// Chords bound to `action`, in definition order.
    pub fn chords_for(&self, action: ActionId) -> Vec<KeyChord> {
        self.bindings
            .iter()
            .filter(|b| b.action == action)
            .map(|b| b.chord)
            .collect()
    }

    /// Bindings of `action`, with their layers, in definition order.
    pub fn bindings_for(&self, action: ActionId) -> Vec<Binding> {
        self.bindings
            .iter()
            .filter(|b| b.action == action)
            .copied()
            .collect()
    }

    /// Chords that reach `action` in a mode whose layer is `mode`, for key
    /// hints ("Press Tab to leave Speech Cursor").
    pub fn chords_in_mode(&self, action: ActionId, mode: Layer) -> Vec<KeyChord> {
        let mut out: Vec<KeyChord> = Vec::new();
        for b in &self.bindings {
            if b.action == action
                && mode.lookup_order().contains(&b.layer)
                && self.lookup(&b.chord, mode) == Some(action)
                && !out.contains(&b.chord)
            {
                out.push(b.chord);
            }
        }
        out
    }

    /// Chords that would trigger different actions in the same mode: bound
    /// twice in one layer, or bound in `Global` and in a mode layer.
    /// Speech Cursor bindings deliberately shadow Browse ones and are not
    /// conflicts.
    pub fn conflicts(&self) -> Vec<Conflict> {
        let mut by_chord: BTreeMap<KeyChord, Vec<&Binding>> = BTreeMap::new();
        for b in &self.bindings {
            by_chord.entry(b.chord).or_default().push(b);
        }
        let mut out = Vec::new();
        for (chord, bs) in by_chord {
            for mode in Layer::ALL {
                let hits: Vec<&&Binding> = bs
                    .iter()
                    .filter(|b| {
                        b.layer == mode || (mode != Layer::Global && b.layer == Layer::Global)
                    })
                    .collect();
                let mut actions: Vec<ActionId> = hits.iter().map(|b| b.action).collect();
                actions.sort();
                actions.dedup();
                if actions.len() > 1 {
                    let mut layers: Vec<Layer> = hits.iter().map(|b| b.layer).collect();
                    layers.sort();
                    layers.dedup();
                    let c = Conflict {
                        chord,
                        layers,
                        actions,
                    };
                    if !out.contains(&c) {
                        out.push(c);
                    }
                }
            }
        }
        out
    }
}

/// The layer for an override chord without a prefix (see
/// [`Keymap::with_overrides`]).
fn infer_layer(chord: &KeyChord, default_layers: &[Layer]) -> Layer {
    if chord.is_text_input() {
        if !default_layers.contains(&Layer::Browse) && default_layers.contains(&Layer::SpeechCursor)
        {
            Layer::SpeechCursor
        } else {
            Layer::Browse
        }
    } else {
        default_layers
            .iter()
            .copied()
            .find(|l| *l != Layer::Browse)
            .unwrap_or(Layer::Global)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &str) -> KeyChord {
        s.parse().unwrap()
    }

    fn all_maps() -> Vec<(Platform, Frontend, Keymap)> {
        let mut v = Vec::new();
        for platform in Platform::ALL {
            for frontend in Frontend::ALL {
                v.push((platform, frontend, Keymap::defaults(platform, frontend)));
            }
        }
        v
    }

    #[test]
    fn every_default_string_parses_with_a_layer() {
        for &a in ActionId::ALL {
            for frontend in Frontend::ALL {
                for s in default_strings(a, frontend) {
                    assert!(
                        parse_default(s, Platform::Linux, frontend).is_some(),
                        "{a:?}: {s}"
                    );
                }
            }
        }
    }

    #[test]
    fn defaults_have_no_conflicts() {
        for (platform, frontend, map) in all_maps() {
            let c = map.conflicts();
            assert!(c.is_empty(), "{platform:?} {frontend:?}: {c:#?}");
        }
    }

    #[test]
    fn every_action_has_a_default() {
        for (platform, frontend, map) in all_maps() {
            for a in ActionId::ALL {
                assert!(
                    !map.chords_for(*a).is_empty(),
                    "{a:?} unbound on {platform:?} {frontend:?}"
                );
            }
        }
    }

    #[test]
    fn every_action_is_reachable_in_its_modes() {
        // Each action has a chord that actually triggers it in at least one
        // mode (not fully shadowed).
        for (platform, frontend, map) in all_maps() {
            for a in ActionId::ALL {
                let reachable = Layer::ALL
                    .iter()
                    .any(|m| !map.chords_in_mode(*a, *m).is_empty());
                assert!(reachable, "{a:?} unreachable on {platform:?} {frontend:?}");
            }
        }
    }

    #[test]
    fn terminal_defaults_are_deliverable() {
        for platform in Platform::ALL {
            for b in Keymap::defaults(platform, Frontend::Terminal).bindings() {
                assert_eq!(
                    b.chord.terminal_limitation(),
                    None,
                    "{:?} {} on {platform:?}",
                    b.action,
                    b.chord
                );
            }
        }
    }

    #[test]
    fn defaults_are_layout_independent() {
        for (_, _, map) in all_maps() {
            for b in map.bindings() {
                assert!(
                    b.chord.is_layout_independent(),
                    "{:?} {}",
                    b.action,
                    b.chord
                );
            }
        }
    }

    #[test]
    fn browse_keys_are_text_keys_and_edit_keys_never_type() {
        use crate::{Key, Modifiers};
        for (_, _, map) in all_maps() {
            for b in map.bindings() {
                match b.layer {
                    // Global chords are active while typing, so none may be
                    // a text key; the same holds for the edit layer.
                    Layer::Global | Layer::Edit => {
                        assert!(!b.chord.is_text_input(), "{:?} {}", b.action, b.chord)
                    }
                    Layer::Browse | Layer::SpeechCursor => {}
                }
                if b.layer == Layer::Edit {
                    let altgr = b.chord.mods.contains(Modifiers::CTRL | Modifiers::ALT)
                        && matches!(b.chord.key, Key::Char(c) if c.is_ascii_alphabetic());
                    assert!(
                        !altgr,
                        "{:?} {} is AltGr on many layouts",
                        b.action, b.chord
                    );
                }
            }
        }
    }

    #[test]
    fn star_gui_chords_are_kept() {
        let map = Keymap::defaults(Platform::Windows, Frontend::Gui);
        for (chord, action) in [
            ("Space", ActionId::PlayPause),
            ("Escape", ActionId::Stop),
            ("Ctrl+Space", ActionId::ReadFromCursor),
            ("Ctrl+=", ActionId::RateUp),
            ("Ctrl+-", ActionId::RateDown),
            ("Alt+.", ActionId::NextSentence),
            ("Alt+,", ActionId::PreviousSentence),
            ("Alt+;", ActionId::ReplaySentence),
            ("Ctrl+P", ActionId::NextParagraph),
            ("Ctrl+Shift+P", ActionId::PreviousParagraph),
            ("Ctrl+R", ActionId::ReplayParagraph),
            ("Ctrl+H", ActionId::NextHeading),
            ("Ctrl+Shift+H", ActionId::PreviousHeading),
            ("Ctrl+T", ActionId::NextTable),
            ("Ctrl+Shift+T", ActionId::PreviousTable),
            ("Ctrl+M", ActionId::AddBookmark),
            ("Ctrl+F", ActionId::Find),
            ("Alt+Left", ActionId::HistoryBack),
            ("Alt+Right", ActionId::HistoryForward),
            ("Ctrl+E", ActionId::ToggleEditMode),
            ("Ctrl+S", ActionId::Save),
            ("Ctrl+N", ActionId::NewDocument),
            ("Ctrl+O", ActionId::Open),
            ("Ctrl+Q", ActionId::Quit),
            ("F2", ActionId::CommandPalette),
            ("F3", ActionId::KeyboardHelp),
            ("F5", ActionId::NextTheme),
            ("Tab", ActionId::SpeechCursorToggle),
        ] {
            assert_eq!(
                map.lookup(&k(chord), Layer::Browse),
                Some(action),
                "{chord}"
            );
        }
        for (chord, action) in [
            ("Ctrl+B", ActionId::Bold),
            ("Ctrl+I", ActionId::Italic),
            ("Ctrl+U", ActionId::Underline),
            ("Ctrl+K", ActionId::InsertLink),
            ("Ctrl+Z", ActionId::Undo),
            ("Ctrl+Y", ActionId::Redo),
        ] {
            assert_eq!(map.lookup(&k(chord), Layer::Edit), Some(action), "{chord}");
        }
    }

    #[test]
    fn star_tui_keys_are_kept() {
        let map = Keymap::defaults(Platform::Linux, Frontend::Terminal);
        for (chord, action) in [
            (".", ActionId::NextSentence),
            (",", ActionId::PreviousSentence),
            (";", ActionId::ReplaySentence),
            ("p", ActionId::NextParagraph),
            ("Ctrl+P", ActionId::NextParagraph),
            ("P", ActionId::PreviousParagraph),
            ("]", ActionId::NextParagraph),
            ("[", ActionId::PreviousParagraph),
            ("r", ActionId::ReplayParagraph),
            ("Ctrl+R", ActionId::ReplayParagraph),
            ("h", ActionId::SkipNextHeading),
            ("}", ActionId::SkipNextHeading),
            ("{", ActionId::SkipPreviousHeading),
            (">", ActionId::NextHeading),
            ("<", ActionId::PreviousHeading),
            ("t", ActionId::NextTable),
            ("T", ActionId::PreviousTable),
            ("n", ActionId::FindNext),
            ("N", ActionId::FindPrevious),
            ("F3", ActionId::FindNext),
            ("F4", ActionId::FindPrevious),
            ("H", ActionId::HistoryBack),
            ("L", ActionId::HistoryForward),
            ("j", ActionId::ScrollDown),
            ("k", ActionId::ScrollUp),
            ("Space", ActionId::PlayPause),
            ("Enter", ActionId::ReadFromCursor),
            ("+", ActionId::RateUp),
            ("=", ActionId::RateUp),
            ("-", ActionId::RateDown),
            ("Ctrl+X", ActionId::Stop),
            ("q", ActionId::Quit),
            ("Q", ActionId::Quit),
            ("Ctrl+Q", ActionId::Quit),
            ("?", ActionId::KeyboardHelp),
            (":", ActionId::CommandPalette),
            ("F2", ActionId::CommandPalette),
            ("F5", ActionId::NextTheme),
            ("F6", ActionId::ToggleLineNumbers),
            ("F8", ActionId::CycleSpeedPreset),
            ("F10", ActionId::PreviousChapter),
            ("F11", ActionId::NextChapter),
            ("Tab", ActionId::SpeechCursorToggle),
            ("Ctrl+Space", ActionId::ReadFromCursor),
            ("Ctrl+O", ActionId::Open),
            ("Ctrl+N", ActionId::NewDocument),
        ] {
            assert_eq!(
                map.lookup(&k(chord), Layer::Browse),
                Some(action),
                "{chord}"
            );
        }
        // Star's TUI Alt chords, which never worked there (ESC ate them),
        // work here.
        assert_eq!(
            map.lookup(&k("Alt+."), Layer::Browse),
            Some(ActionId::NextSentence)
        );
        // Speech Cursor keys (star/tui/mixin_speechcursor.py).
        for (chord, action) in [
            ("Down", ActionId::SpeechCursorNextLine),
            ("j", ActionId::SpeechCursorNextLine),
            ("Up", ActionId::SpeechCursorPreviousLine),
            ("k", ActionId::SpeechCursorPreviousLine),
            ("r", ActionId::SpeechCursorRereadLine),
            ("Enter", ActionId::SpeechCursorExitAndRead),
            ("Tab", ActionId::SpeechCursorToggle),
            ("PageDown", ActionId::NextParagraph),
            ("PageUp", ActionId::PreviousParagraph),
            (".", ActionId::NextSentence),
            ("t", ActionId::NextTable),
            ("Home", ActionId::DocumentStart),
            ("Space", ActionId::PlayPause),
            ("Escape", ActionId::Stop),
        ] {
            assert_eq!(
                map.lookup(&k(chord), Layer::SpeechCursor),
                Some(action),
                "{chord}"
            );
        }
    }

    /// Star's `test_authoring.py` 13-15: Ctrl+B, Ctrl+I, Ctrl+K, Ctrl+U, and
    /// Ctrl+M each have exactly one owner in the GUI.
    #[test]
    fn authoring_chords_have_one_owner() {
        let map = Keymap::defaults(Platform::Windows, Frontend::Gui);
        for (chord, action) in [
            ("Ctrl+B", ActionId::Bold),
            ("Ctrl+I", ActionId::Italic),
            ("Ctrl+K", ActionId::InsertLink),
            ("Ctrl+U", ActionId::Underline),
            ("Ctrl+M", ActionId::AddBookmark),
        ] {
            let owners: Vec<ActionId> = map
                .bindings()
                .iter()
                .filter(|b| b.chord == k(chord))
                .map(|b| b.action)
                .collect();
            assert_eq!(owners, vec![action], "{chord}");
        }
    }

    /// Star's `test_authoring.py` 5 ("the formatting toolbar follows edit
    /// mode"): formatting commands are reachable only in edit mode.
    #[test]
    fn formatting_is_bound_only_in_edit_mode() {
        use crate::Category;
        for (_, _, map) in all_maps() {
            for a in ActionId::in_category(Category::Editing) {
                if a == ActionId::ToggleEditMode {
                    continue;
                }
                for b in map.bindings_for(a) {
                    assert_eq!(b.layer, Layer::Edit, "{a:?}");
                }
            }
        }
    }

    /// Wave 1 requests: Shift+arrows select in browse mode on both
    /// frontends, `read_paragraph` exists, terminal F3 stays Find next, and
    /// keyboard help is `?` (plus F3 in the GUI) with F1 for the help.
    #[test]
    fn wave2_selection_help_and_notes_keys() {
        for (_, frontend, map) in all_maps() {
            for (chord, action) in [
                ("Shift+Right", ActionId::SelectNextWord),
                ("Shift+Left", ActionId::SelectPreviousWord),
                ("Shift+Down", ActionId::SelectNextLine),
                ("Shift+Up", ActionId::SelectPreviousLine),
                ("S", ActionId::ReadParagraph),
                ("?", ActionId::KeyboardHelp),
                ("F1", ActionId::Help),
                ("a", ActionId::AddNote),
                ("A", ActionId::ListNotes),
                ("e", ActionId::NextNote),
                ("E", ActionId::PreviousNote),
                ("Delete", ActionId::DeleteNote),
                ("y", ActionId::HighlightSelection),
            ] {
                assert_eq!(
                    map.lookup(&k(chord), Layer::Browse),
                    Some(action),
                    "{chord} on {frontend:?}"
                );
            }
            // Selection keys are browse keys: edit mode keeps Shift+arrows
            // for the text control's own selection.
            assert_eq!(map.lookup(&k("Shift+Right"), Layer::Edit), None);
        }
        let term = Keymap::defaults(Platform::Linux, Frontend::Terminal);
        assert_eq!(
            term.lookup(&k("F3"), Layer::Browse),
            Some(ActionId::FindNext)
        );
        assert_eq!(
            term.lookup(&k("Alt+L"), Layer::Browse),
            Some(ActionId::OpenLibrary)
        );
        let gui = Keymap::defaults(Platform::Windows, Frontend::Gui);
        assert_eq!(
            gui.lookup(&k("F3"), Layer::Browse),
            Some(ActionId::KeyboardHelp)
        );
        assert_eq!(
            gui.lookup(&k("Ctrl+Shift+B"), Layer::Browse),
            Some(ActionId::OpenLibrary),
            "Star's Library chord"
        );
        assert_eq!(
            gui.lookup(&k("Ctrl+Shift+N"), Layer::Browse),
            Some(ActionId::ListNotes),
            "Star's notes panel chord"
        );
    }

    #[test]
    fn lookup_by_layer() {
        let map = Keymap::defaults(Platform::Linux, Frontend::Terminal);
        assert_eq!(
            map.lookup(&k("."), Layer::Browse),
            Some(ActionId::NextSentence)
        );
        assert_eq!(
            map.lookup(&k("Alt+."), Layer::Edit),
            Some(ActionId::NextSentence)
        );
        assert_eq!(map.lookup(&k("."), Layer::Edit), None);
        assert_eq!(
            map.lookup(&k("j"), Layer::SpeechCursor),
            Some(ActionId::SpeechCursorNextLine)
        );
        assert_eq!(
            map.lookup(&k("j"), Layer::Browse),
            Some(ActionId::ScrollDown)
        );
        assert_eq!(
            map.lookup(&k("Shift+T"), Layer::Browse),
            Some(ActionId::PreviousTable)
        );
        assert_eq!(
            map.lookup(&k("Ctrl+F"), Layer::Global),
            Some(ActionId::Find)
        );
        assert_eq!(map.lookup(&k("n"), Layer::Global), None);
    }

    #[test]
    fn overrides_replace_and_warn() {
        let mut o = BTreeMap::new();
        o.insert("next_sentence".to_owned(), vec!["x".to_owned()]);
        o.insert("bogus".to_owned(), vec![]);
        let (map, warnings) = Keymap::with_overrides(Platform::Linux, Frontend::Terminal, &o);
        assert_eq!(
            map.lookup(&k("x"), Layer::Browse),
            Some(ActionId::NextSentence)
        );
        assert_eq!(map.lookup(&k("."), Layer::Browse), None);
        assert_eq!(warnings.len(), 1);
    }

    /// ADR-0006's example: `next_sentence = ["Alt+.", "."]` keeps `.` a
    /// browse key, so typing a period in edit mode still types.
    #[test]
    fn override_layers_are_inferred_per_chord() {
        let mut o = BTreeMap::new();
        o.insert(
            "next_sentence".to_owned(),
            vec!["Alt+.".to_owned(), ".".to_owned()],
        );
        o.insert("speech_cursor_next_line".to_owned(), vec!["n".to_owned()]);
        o.insert("bold".to_owned(), vec!["Alt+B".to_owned()]);
        o.insert("stop".to_owned(), vec![]);
        let (map, warnings) = Keymap::with_overrides(Platform::Linux, Frontend::Terminal, &o);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(map.lookup(&k("."), Layer::Edit), None);
        assert_eq!(
            map.lookup(&k("."), Layer::Browse),
            Some(ActionId::NextSentence)
        );
        assert_eq!(
            map.lookup(&k("Alt+."), Layer::Edit),
            Some(ActionId::NextSentence)
        );
        assert_eq!(
            map.lookup(&k("n"), Layer::SpeechCursor),
            Some(ActionId::SpeechCursorNextLine)
        );
        assert_eq!(map.lookup(&k("n"), Layer::Browse), Some(ActionId::FindNext));
        assert_eq!(map.lookup(&k("Alt+B"), Layer::Edit), Some(ActionId::Bold));
        assert_eq!(map.lookup(&k("Alt+B"), Layer::Browse), None);
        assert!(map.chords_for(ActionId::Stop).is_empty());
    }

    #[test]
    fn override_prefixes_and_warnings() {
        let mut o = BTreeMap::new();
        o.insert("next_theme".to_owned(), vec!["b:x".to_owned()]);
        o.insert("command_palette".to_owned(), vec!["b::".to_owned()]);
        o.insert("bold".to_owned(), vec!["e:Ctrl+H".to_owned()]);
        o.insert("find".to_owned(), vec!["Hyper+F".to_owned()]);
        o.insert("find_next".to_owned(), vec!["p".to_owned()]);
        let (map, warnings) = Keymap::with_overrides(Platform::Linux, Frontend::Terminal, &o);
        assert_eq!(
            map.lookup(&k("x"), Layer::Browse),
            Some(ActionId::NextTheme)
        );
        assert_eq!(map.lookup(&k("F5"), Layer::Browse), None);
        assert_eq!(
            map.lookup(&k(":"), Layer::Browse),
            Some(ActionId::CommandPalette)
        );
        let joined = warnings.join("\n");
        assert!(joined.contains("Backspace"), "{joined}");
        assert!(joined.contains("find: unknown modifier"), "{joined}");
        assert!(joined.contains("p is bound to"), "{joined}");
        assert_eq!(warnings.len(), 3, "{joined}");
    }

    #[test]
    fn mac_gui_uses_cmd() {
        let map = Keymap::defaults(Platform::MacOs, Frontend::Gui);
        assert_eq!(map.lookup(&k("Cmd+O"), Layer::Browse), Some(ActionId::Open));
        assert_eq!(map.lookup(&k("Ctrl+O"), Layer::Browse), None);
    }

    #[test]
    fn chords_in_mode_respects_shadowing() {
        let map = Keymap::defaults(Platform::Linux, Frontend::Terminal);
        // In Speech Cursor mode `j` is next line, so it is not a scroll key.
        assert!(
            map.chords_in_mode(ActionId::ScrollDown, Layer::SpeechCursor)
                .is_empty()
        );
        assert_eq!(
            map.chords_in_mode(ActionId::ScrollDown, Layer::Browse),
            vec![k("j")]
        );
        assert_eq!(
            map.chords_in_mode(ActionId::SpeechCursorToggle, Layer::SpeechCursor),
            vec![k("Tab")]
        );
    }
}
