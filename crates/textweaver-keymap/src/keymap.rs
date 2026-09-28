//! Keymaps: defaults plus overrides, looked up by layer.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ActionId, KeyChord, Preset};

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
    /// Single-key shortcuts are off: text-input chords trigger nothing.
    character_keys_off: bool,
    /// The preset the keys started from.
    preset: Preset,
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
        Keymap {
            bindings,
            character_keys_off: false,
            preset: Preset::Default,
        }
    }

    /// The default keymap changed by `preset` ([`Preset::changes`]): each
    /// chord of the preset leaves whatever it did by default and triggers
    /// the preset's action, or nothing.
    pub fn with_preset(platform: Platform, frontend: Frontend, preset: Preset) -> Self {
        let mut map = Keymap::defaults(platform, frontend);
        for (s, action) in preset.changes() {
            let Some((layer, chord)) = parse_default(s, platform, frontend) else {
                continue;
            };
            map.bindings
                .retain(|b| !(b.chord == chord && b.layer == layer));
            if let Some(action) = action {
                map.bindings.push(Binding {
                    chord,
                    layer,
                    action: *action,
                });
            }
        }
        map.preset = preset;
        map
    }

    /// The preset these keys started from.
    pub fn preset(&self) -> Preset {
        self.preset
    }

    /// Turns single-key shortcuts on or off (the `[keyboard]
    /// character_keys` setting and the `toggle_character_keys` action).
    /// While off, [`lookup`](Self::lookup) ignores every binding whose chord
    /// is text input (a printable character or Space, at most with Shift)
    /// in every layer, so dictated or typed text can never trigger a
    /// command (WCAG 2.1.4). Modifier chords, named keys, and the command
    /// palette keep working.
    pub fn set_character_keys(&mut self, on: bool) {
        self.character_keys_off = !on;
    }

    /// True when single-key shortcuts are on (the default).
    pub fn character_keys(&self) -> bool {
        !self.character_keys_off
    }

    /// True when `binding` can fire now: always, unless it is a text-input
    /// chord and single-key shortcuts are off.
    pub fn is_active(&self, binding: &Binding) -> bool {
        !(self.character_keys_off && binding.chord.is_text_input())
    }

    /// Actions no chord reaches in any mode, given the current single-key
    /// setting. They remain available from the command palette.
    pub fn palette_only(&self) -> Vec<ActionId> {
        ActionId::ALL
            .iter()
            .copied()
            .filter(|a| {
                Layer::ALL
                    .iter()
                    .all(|m| self.chords_in_mode(*a, *m).is_empty())
            })
            .collect()
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
    /// overrides introduce. A chord from an override that would reach two
    /// actions in one mode is not applied (the other action keeps it) and
    /// is reported; so is an action left with no keys because of that.
    /// Warnings never stop loading.
    pub fn with_overrides(
        platform: Platform,
        frontend: Frontend,
        overrides: &BTreeMap<String, Vec<String>>,
    ) -> (Self, Vec<String>) {
        Self::with_preset_and_overrides(platform, frontend, Preset::Default, overrides)
    }

    /// [`with_overrides`](Self::with_overrides) on top of a preset
    /// ([`with_preset`](Self::with_preset)) instead of the defaults.
    pub fn with_preset_and_overrides(
        platform: Platform,
        frontend: Frontend,
        preset: Preset,
        overrides: &BTreeMap<String, Vec<String>>,
    ) -> (Self, Vec<String>) {
        let mut map = Keymap::with_preset(platform, frontend, preset);
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
        let overridden: Vec<ActionId> = overrides
            .keys()
            .filter_map(|id| ActionId::from_id(id))
            .collect();
        for c in map.conflicts() {
            if before.contains(&c) {
                continue;
            }
            // Drop the override bindings behind the conflict; defaults never
            // conflict (a test), so what remains is conflict-free.
            let dropped: Vec<ActionId> = c
                .actions
                .iter()
                .copied()
                .filter(|a| overridden.contains(a))
                .collect();
            map.bindings.retain(|b| {
                !(b.chord == c.chord && c.layers.contains(&b.layer) && dropped.contains(&b.action))
            });
            let names: Vec<&str> = dropped.iter().map(|a| a.id()).collect();
            warnings.push(format!(
                "keymap.toml: {}; not applied for {}",
                c.describe(),
                names.join(" and ")
            ));
        }
        for &a in &overridden {
            let wanted = overrides.get(a.id()).is_some_and(|v| !v.is_empty());
            if wanted && map.bindings_for(a).is_empty() {
                warnings.push(format!(
                    "keymap.toml: {} has no keys left; use the command palette or choose other keys",
                    a.id()
                ));
            }
        }
        (map, warnings)
    }

    /// Every binding.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// The action for `chord` in a mode whose layer is `mode`. With
    /// single-key shortcuts off, text-input chords find nothing.
    pub fn lookup(&self, chord: &KeyChord, mode: Layer) -> Option<ActionId> {
        if self.character_keys_off && chord.is_text_input() {
            return None;
        }
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

    /// Every platform, frontend, and preset: the rules the defaults follow
    /// hold for each preset too.
    fn all_preset_maps() -> Vec<(Platform, Frontend, Keymap)> {
        let mut v = Vec::new();
        for platform in Platform::ALL {
            for frontend in Frontend::ALL {
                for preset in Preset::ALL {
                    v.push((
                        platform,
                        frontend,
                        Keymap::with_preset(platform, frontend, preset),
                    ));
                }
            }
        }
        v
    }

    #[test]
    fn presets_keep_the_rules() {
        for (platform, frontend, map) in all_preset_maps() {
            let what = format!("{platform:?} {frontend:?} {:?}", map.preset());
            assert!(map.conflicts().is_empty(), "{what}: {:#?}", map.conflicts());
            // Palette commands have no keys by design, and a preset may
            // give some actions' keys back to its own commands. Window-only
            // commands have no terminal keys.
            for a in ActionId::ALL.iter().filter(|a| {
                !a.is_palette_command()
                    && !map.preset().palette_only().contains(a)
                    && !(a.is_window_only() && frontend == Frontend::Terminal)
            }) {
                let reachable = Layer::ALL
                    .iter()
                    .any(|m| !map.chords_in_mode(*a, *m).is_empty());
                assert!(reachable, "{a:?} unreachable, {what}");
            }
            for b in map.bindings() {
                assert!(
                    b.chord.is_layout_independent(),
                    "{:?} {}",
                    b.action,
                    b.chord
                );
                if frontend == Frontend::Terminal {
                    assert_eq!(
                        b.chord.terminal_limitation(),
                        None,
                        "{:?} {}",
                        b.action,
                        b.chord
                    );
                }
                if matches!(b.layer, Layer::Global | Layer::Edit) {
                    assert!(!b.chord.is_text_input(), "{:?} {}", b.action, b.chord);
                }
            }
            // No action needs the command palette with single-key
            // shortcuts off unless it did already with the defaults.
            let mut off = map.clone();
            off.set_character_keys(false);
            let mut default_off = Keymap::defaults(platform, frontend);
            default_off.set_character_keys(false);
            let before = default_off.palette_only();
            for a in off.palette_only() {
                assert!(before.contains(&a), "{a:?} needs the palette, {what}");
            }
        }
    }

    /// The default keys mirror NVDA's and JAWS's browse-mode quick
    /// navigation (the owner's decision, 2026-09-26).
    #[test]
    fn default_keys_mirror_screen_reader_quick_navigation() {
        for platform in Platform::ALL {
            for frontend in Frontend::ALL {
                let map = Keymap::defaults(platform, frontend);
                assert_eq!(map.preset(), Preset::Default);
                assert_eq!(
                    map,
                    Keymap::with_preset(platform, frontend, Preset::Default)
                );
                for (chord, action) in [
                    ("h", ActionId::SkipNextHeading),
                    ("Shift+H", ActionId::SkipPreviousHeading),
                    ("1", ActionId::NextHeadingLevel1),
                    ("6", ActionId::NextHeadingLevel6),
                    ("!", ActionId::PreviousHeadingLevel1),
                    ("l", ActionId::NextList),
                    ("Shift+L", ActionId::PreviousList),
                    ("i", ActionId::NextListItem),
                    ("Shift+I", ActionId::PreviousListItem),
                    ("t", ActionId::NextTable),
                    ("Shift+T", ActionId::PreviousTable),
                    ("k", ActionId::NextLink),
                    ("Shift+K", ActionId::PreviousLink),
                    ("u", ActionId::NextLink),
                    ("q", ActionId::NextBlockQuote),
                    ("Shift+Q", ActionId::PreviousBlockQuote),
                    ("s", ActionId::NextSeparator),
                    ("Shift+S", ActionId::PreviousSeparator),
                    ("g", ActionId::NextGraphic),
                    ("Shift+G", ActionId::PreviousGraphic),
                    ("d", ActionId::NextChapter),
                    ("Shift+D", ActionId::PreviousChapter),
                    ("Backspace", ActionId::HistoryBack),
                    ("\\", ActionId::HistoryForward),
                    ("Alt+Left", ActionId::HistoryBack),
                    ("Alt+Right", ActionId::HistoryForward),
                    ("Shift+W", ActionId::SayPosition),
                    (".", ActionId::ReadCurrentSentence),
                    (",", ActionId::ReadParagraph),
                    ("Alt+Down", ActionId::NextSentence),
                    ("Alt+Up", ActionId::PreviousSentence),
                    ("Alt+.", ActionId::NextSentence),
                    ("Ctrl+Down", ActionId::NextParagraph),
                    ("Ctrl+Up", ActionId::PreviousParagraph),
                    ("p", ActionId::NextParagraph),
                    ("j", ActionId::ScrollDown),
                    ("Shift+J", ActionId::ScrollUp),
                    ("F12", ActionId::NextNote),
                    ("Shift+F12", ActionId::PreviousNote),
                    ("e", ActionId::NextNote),
                    ("Space", ActionId::PlayPause),
                ] {
                    let chord = if frontend == Frontend::Gui && platform == Platform::MacOs {
                        k(chord).ctrl_to_meta()
                    } else {
                        k(chord)
                    };
                    assert_eq!(
                        map.lookup(&chord, Layer::Browse),
                        Some(action),
                        "{chord} on {platform:?} {frontend:?}"
                    );
                }
                // Displaced keys: quit keeps only its chord, which asks
                // first.
                assert!(
                    map.chords_for(ActionId::Quit)
                        .iter()
                        .all(|c| !c.is_text_input())
                );
                assert!(!map.chords_for(ActionId::Quit).is_empty());
                assert_eq!(map.lookup(&k("o"), Layer::Browse), None);
            }
        }
    }

    /// The classic preset brings back the earlier single keys.
    #[test]
    fn classic_preset_keys() {
        for platform in Platform::ALL {
            for frontend in Frontend::ALL {
                let map = Keymap::with_preset(platform, frontend, Preset::Classic);
                assert_eq!(map.preset(), Preset::Classic);
                for (chord, action) in [
                    (".", ActionId::NextSentence),
                    (",", ActionId::PreviousSentence),
                    ("s", ActionId::ReadCurrentSentence),
                    ("Shift+S", ActionId::ReadParagraph),
                    ("l", ActionId::ReadCurrentLine),
                    ("Shift+H", ActionId::HistoryBack),
                    ("Shift+L", ActionId::HistoryForward),
                    ("o", ActionId::NextList),
                    ("Shift+O", ActionId::PreviousList),
                    ("k", ActionId::ScrollUp),
                    ("Shift+K", ActionId::LinkAddress),
                    ("q", ActionId::Quit),
                    ("Shift+Q", ActionId::Quit),
                    ("Alt+Down", ActionId::NextNote),
                    ("Alt+Up", ActionId::PreviousNote),
                    ("u", ActionId::NextLink),
                    ("h", ActionId::SkipNextHeading),
                    ("1", ActionId::NextHeadingLevel1),
                    ("Ctrl+Down", ActionId::NextParagraph),
                ] {
                    let chord = if frontend == Frontend::Gui && platform == Platform::MacOs {
                        k(chord).ctrl_to_meta()
                    } else {
                        k(chord)
                    };
                    assert_eq!(
                        map.lookup(&chord, Layer::Browse),
                        Some(action),
                        "{chord} on {platform:?} {frontend:?}"
                    );
                }
                for free in ["g", "Shift+G", "d", "Shift+D"] {
                    assert_eq!(map.lookup(&k(free), Layer::Browse), None, "{free}");
                }
                for a in Preset::Classic.palette_only() {
                    assert!(map.chords_for(*a).is_empty(), "{a:?}");
                }
            }
        }
    }

    /// Overrides apply on top of the preset.
    #[test]
    fn overrides_apply_over_a_preset() {
        let mut o = BTreeMap::new();
        o.insert("next_link".to_owned(), vec!["b:u".to_owned()]);
        let (map, warnings) = Keymap::with_preset_and_overrides(
            Platform::Linux,
            Frontend::Terminal,
            Preset::Classic,
            &o,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(map.lookup(&k("k"), Layer::Browse), Some(ActionId::ScrollUp));
        assert_eq!(map.lookup(&k("u"), Layer::Browse), Some(ActionId::NextLink));
        assert_eq!(
            map.lookup(&k("l"), Layer::Browse),
            Some(ActionId::ReadCurrentLine)
        );
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
        // Palette commands (exports, templates) have no default keys by
        // design, nor window-only commands in the terminal; every other
        // action has one on each frontend.
        for (platform, frontend, map) in all_maps() {
            for a in ActionId::ALL.iter().filter(|a| {
                !a.is_palette_command() && !(a.is_window_only() && frontend == Frontend::Terminal)
            }) {
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
            for a in ActionId::ALL.iter().filter(|a| {
                !a.is_palette_command() && !(a.is_window_only() && frontend == Frontend::Terminal)
            }) {
                let reachable = Layer::ALL
                    .iter()
                    .any(|m| !map.chords_in_mode(*a, *m).is_empty());
                assert!(reachable, "{a:?} unreachable on {platform:?} {frontend:?}");
            }
        }
    }

    #[test]
    fn window_only_commands_have_window_keys_only() {
        for platform in Platform::ALL {
            let gui = Keymap::defaults(platform, Frontend::Gui);
            let term = Keymap::defaults(platform, Frontend::Terminal);
            for a in ActionId::ALL.iter().filter(|a| a.is_window_only()) {
                assert!(!gui.chords_for(*a).is_empty(), "{a:?} on {platform:?}");
                assert!(term.chords_for(*a).is_empty(), "{a:?} on {platform:?}");
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

    /// Star's terminal keys, in the classic preset.
    #[test]
    fn star_tui_keys_are_kept() {
        let map = Keymap::with_preset(Platform::Linux, Frontend::Terminal, Preset::Classic);
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
    /// mode"): formatting commands are reachable only in edit mode. Edit
    /// mode on and off, copying, and the typing echo work everywhere.
    #[test]
    fn formatting_is_bound_only_in_edit_mode() {
        use crate::Category;
        for (_, _, map) in all_maps() {
            for a in ActionId::in_category(Category::Editing) {
                if matches!(
                    a,
                    ActionId::ToggleEditMode
                        | ActionId::Copy
                        | ActionId::CycleTypingEcho
                        | ActionId::AddReference
                ) {
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
                (",", ActionId::ReadParagraph),
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

    /// Phase 1 keys (Agent P1b): 1 to 6 jump to the next heading of that
    /// level and Shift with the digit (as a US layout sends it) to the
    /// previous one, in browse mode only; the new chords reach their
    /// actions in every mode that needs them.
    #[test]
    fn phase1_authoring_keys() {
        for (platform, frontend, map) in all_maps() {
            for (level, prev) in (1..=6u8).zip(["!", "@", "#", "$", "%", "^"]) {
                let next = ActionId::next_heading_at(level).unwrap();
                let back = ActionId::previous_heading_at(level).unwrap();
                assert_eq!(next.heading_level_jump(), Some((level, true)));
                assert_eq!(back.heading_level_jump(), Some((level, false)));
                assert_eq!(
                    map.lookup(&k(&level.to_string()), Layer::Browse),
                    Some(next),
                    "{platform:?} {frontend:?}"
                );
                assert_eq!(map.lookup(&k(prev), Layer::Browse), Some(back));
                // Digits type in edit mode.
                assert_eq!(map.lookup(&k(&level.to_string()), Layer::Edit), None);
            }
            assert_eq!(ActionId::next_heading_at(7), None);
            assert_eq!(ActionId::NextSentence.heading_level_jump(), None);
            assert_eq!(
                map.lookup(&k("Alt+N"), Layer::Edit),
                Some(ActionId::AddNote)
            );
            assert_eq!(
                map.lookup(&k("Alt+N"), Layer::Browse),
                Some(ActionId::AddNote)
            );
            let ctrl = if platform == Platform::MacOs && frontend == Frontend::Gui {
                "Cmd"
            } else {
                "Ctrl"
            };
            let c = k(&format!("{ctrl}+C"));
            assert_eq!(map.lookup(&c, Layer::Edit), Some(ActionId::Copy));
            assert_eq!(map.lookup(&c, Layer::Browse), Some(ActionId::Copy));
            let x = k(&format!("{ctrl}+X"));
            assert_eq!(map.lookup(&x, Layer::Edit), Some(ActionId::Cut));
            assert_eq!(
                map.lookup(&k("Tab"), Layer::Edit),
                Some(ActionId::NextTableCell)
            );
            assert_eq!(
                map.lookup(&k("Shift+Tab"), Layer::Edit),
                Some(ActionId::PreviousTableCell)
            );
            assert_eq!(
                map.lookup(&k("Tab"), Layer::Browse),
                Some(ActionId::SpeechCursorToggle)
            );
            assert_eq!(
                map.lookup(&k("Shift+F9"), Layer::Edit),
                Some(ActionId::CycleTypingEcho)
            );
            assert_eq!(
                map.lookup(&k("Alt+Shift+K"), Layer::Edit),
                Some(ActionId::LinkAddress)
            );
            assert_eq!(
                map.lookup(&k("Alt+Shift+T"), Layer::Browse),
                Some(ActionId::WordCount)
            );
            assert_eq!(
                map.lookup(&k("W"), Layer::Browse),
                Some(ActionId::SayPosition)
            );
            assert_eq!(
                map.lookup(&k("Alt+Shift+Y"), Layer::Edit),
                Some(ActionId::SayPosition)
            );
        }
    }

    /// Phase 2 keys (Agent P2b): heading chords reach edit mode in the
    /// terminal, the outline and the table, link, spelling, and settings
    /// chords work in every mode, the editing basics are the standard
    /// chords, and palette commands have no keys but are listed.
    #[test]
    fn phase2_authoring_keys() {
        for (platform, frontend, map) in all_maps() {
            let ctrl = if platform == Platform::MacOs && frontend == Frontend::Gui {
                "Cmd"
            } else {
                "Ctrl"
            };
            let c = |s: &str| k(&s.replace("Ctrl", ctrl));
            for mode in [Layer::Browse, Layer::SpeechCursor, Layer::Edit] {
                for (chord, action) in [
                    ("Alt+O", ActionId::Outline),
                    ("Alt+Shift+F", ActionId::FollowLink),
                    ("Ctrl+Alt+Down", ActionId::TableNextRow),
                    ("Ctrl+Alt+Up", ActionId::TablePreviousRow),
                    ("Ctrl+Alt+Right", ActionId::TableNextColumn),
                    ("Ctrl+Alt+Left", ActionId::TablePreviousColumn),
                    ("Alt+M", ActionId::NextMisspelling),
                    ("Alt+Shift+M", ActionId::PreviousMisspelling),
                    ("Alt+J", ActionId::SpellingSuggestions),
                    ("Alt+Shift+V", ActionId::CycleVerbosity),
                    ("Alt+Shift+N", ActionId::CyclePunctuation),
                    ("Alt+Shift+Q", ActionId::ToggleCitations),
                    ("Alt+Shift+X", ActionId::ExploreMath),
                    ("Alt+Shift+Z", ActionId::SyllablesToggle),
                    ("Alt+Shift+J", ActionId::DifficultWordsToggle),
                    ("Alt+Shift+PageUp", ActionId::RsvpFaster),
                    ("Alt+Shift+PageDown", ActionId::RsvpSlower),
                ] {
                    assert_eq!(
                        map.lookup(&c(chord), mode),
                        Some(action),
                        "{chord} in {mode:?} on {platform:?} {frontend:?}"
                    );
                }
            }
            for (chord, action) in [
                ("Ctrl+A", ActionId::SelectAll),
                ("Ctrl+V", ActionId::Paste),
                ("Ctrl+Delete", ActionId::DeleteWordAfter),
                ("Alt+C", ActionId::InsertCitation),
            ] {
                assert_eq!(map.lookup(&c(chord), Layer::Edit), Some(action), "{chord}");
                assert_eq!(map.lookup(&c(chord), Layer::Browse), None, "{chord}");
            }
            // Windows Terminal splits its window with Alt+Shift+D, so the
            // terminal adds references with Alt+B.
            let add_reference = if frontend == Frontend::Terminal {
                k("Alt+B")
            } else {
                k("Alt+Shift+D")
            };
            for mode in [Layer::Browse, Layer::Edit] {
                assert_eq!(
                    map.lookup(&add_reference, mode),
                    Some(ActionId::AddReference)
                );
            }
            if frontend == Frontend::Terminal {
                assert_eq!(map.lookup(&k("Alt+Shift+D"), Layer::Browse), None);
            }
            let word_back = if frontend == Frontend::Terminal {
                k("Alt+Backspace")
            } else {
                c("Ctrl+Backspace")
            };
            assert_eq!(
                map.lookup(&word_back, Layer::Edit),
                Some(ActionId::DeleteWordBefore)
            );
            if frontend == Frontend::Terminal {
                assert_eq!(
                    map.lookup(&k("Alt+H"), Layer::Edit),
                    Some(ActionId::SkipNextHeading)
                );
                assert_eq!(
                    map.lookup(&k("Alt+Shift+H"), Layer::Edit),
                    Some(ActionId::SkipPreviousHeading)
                );
            }
            for a in [
                ActionId::ExportHtml,
                ActionId::ExportPdf,
                ActionId::ExportDocx,
                ActionId::ExportEpub,
                ActionId::ExportBrf,
                ActionId::PreviewInBrowser,
                ActionId::ListenRendered,
                ActionId::InsertBibliography,
                ActionId::CheckCitations,
                ActionId::ImportReferences,
                ActionId::ExportStudySheet,
                ActionId::NewFromTemplate,
                ActionId::TogglePreviewAutoReload,
                ActionId::TogglePreviewLive,
            ] {
                assert!(a.is_palette_command(), "{a:?}");
                assert!(map.chords_for(a).is_empty(), "{a:?}");
                assert!(map.palette_only().contains(&a), "{a:?}");
            }
            assert!(!ActionId::Outline.is_palette_command());
        }
    }

    #[test]
    fn lookup_by_layer() {
        let map = Keymap::defaults(Platform::Linux, Frontend::Terminal);
        assert_eq!(
            map.lookup(&k("."), Layer::Browse),
            Some(ActionId::ReadCurrentSentence)
        );
        assert_eq!(
            map.lookup(&k("Alt+."), Layer::Edit),
            Some(ActionId::NextSentence)
        );
        assert_eq!(
            map.lookup(&k("Alt+Down"), Layer::Edit),
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
        assert_eq!(map.lookup(&k("Alt+."), Layer::Browse), None);
        assert_eq!(map.lookup(&k("Alt+Down"), Layer::Browse), None);
        assert_eq!(warnings.len(), 1);
    }

    /// Swapping two actions' keys is not a conflict once both overrides
    /// apply, whatever order they are read in.
    #[test]
    fn swapped_keys_apply() {
        let mut o = BTreeMap::new();
        o.insert("next_sentence".to_owned(), vec![",".to_owned()]);
        o.insert("previous_sentence".to_owned(), vec![".".to_owned()]);
        let (map, warnings) = Keymap::with_preset_and_overrides(
            Platform::Linux,
            Frontend::Terminal,
            Preset::Classic,
            &o,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            map.lookup(&k(","), Layer::Browse),
            Some(ActionId::NextSentence)
        );
        assert_eq!(
            map.lookup(&k("."), Layer::Browse),
            Some(ActionId::PreviousSentence)
        );
    }

    /// WCAG 2.1.4: no single printable key quits, leaves, or deletes
    /// without a confirmation, and with single-key shortcuts off nothing a
    /// dictation or typing produces triggers a command.
    #[test]
    fn character_keys_can_be_turned_off() {
        use crate::Category;
        for (platform, frontend, mut map) in all_maps() {
            for b in map.bindings() {
                // A single printable key may reach a quitting or
                // destructive action only through its confirmation.
                let risky = matches!(
                    b.action,
                    ActionId::Quit | ActionId::DeleteNote | ActionId::Open | ActionId::NewDocument
                );
                assert!(
                    !(risky && b.chord.is_text_input() && !b.action.needs_confirmation()),
                    "{:?} on {} ({platform:?} {frontend:?})",
                    b.action,
                    b.chord
                );
            }
            assert!(map.character_keys());
            assert_eq!(
                map.lookup(&k("."), Layer::Browse),
                Some(ActionId::ReadCurrentSentence)
            );
            map.set_character_keys(false);
            assert!(!map.character_keys());
            for text in ["a", "q", ".", "Space", "Shift+R", "?", ":", "j"] {
                for mode in Layer::ALL {
                    assert_eq!(map.lookup(&k(text), mode), None, "{text} in {mode:?}");
                }
            }
            // Modifier chords and named keys still work, including the way
            // back and the palette, which reaches every action.
            for mode in [Layer::Browse, Layer::SpeechCursor, Layer::Edit] {
                for a in [
                    ActionId::ToggleCharacterKeys,
                    ActionId::CommandPalette,
                    ActionId::Stop,
                    ActionId::Quit,
                ] {
                    assert!(
                        !map.chords_in_mode(a, mode).is_empty(),
                        "{a:?} in {mode:?} on {platform:?} {frontend:?}"
                    );
                }
            }
            assert_eq!(
                map.lookup(&k("F9"), Layer::Browse),
                Some(ActionId::ToggleCharacterKeys)
            );
            let palette_only = map.palette_only();
            assert!(!palette_only.contains(&ActionId::CommandPalette));
            for a in &palette_only {
                assert!(!a.palette_name().is_empty());
            }
            // Every other action is still reachable by a chord with a
            // modifier or a named key.
            for a in ActionId::ALL {
                if palette_only.contains(a) {
                    continue;
                }
                let reachable = Layer::ALL.iter().any(|m| {
                    map.chords_in_mode(*a, *m)
                        .iter()
                        .any(|c| !c.is_text_input())
                });
                assert!(reachable, "{a:?}");
            }
            // Reading basics never depend on character keys.
            for a in ActionId::in_category(Category::Reading) {
                if palette_only.contains(&a) {
                    continue;
                }
                assert!(map.bindings_for(a).iter().any(|b| map.is_active(b)));
            }
        }
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
        o.insert("bold".to_owned(), vec!["Alt+Z".to_owned()]);
        o.insert("stop".to_owned(), vec![]);
        let (map, warnings) = Keymap::with_preset_and_overrides(
            Platform::Linux,
            Frontend::Terminal,
            Preset::Classic,
            &o,
        );
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
        assert_eq!(map.lookup(&k("Alt+Z"), Layer::Edit), Some(ActionId::Bold));
        assert_eq!(map.lookup(&k("Alt+Z"), Layer::Browse), None);
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
        assert!(joined.contains("not applied for find_next"), "{joined}");
        assert!(joined.contains("find_next has no keys left"), "{joined}");
        // The conflicting override is not applied: p still moves by
        // paragraph.
        assert_eq!(
            map.lookup(&k("p"), Layer::Browse),
            Some(ActionId::NextParagraph)
        );
        assert!(map.conflicts().is_empty());
        assert!(joined.contains("find has no keys left"), "{joined}");
        assert_eq!(warnings.len(), 5, "{joined}");
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
