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
    fn from_prefix(p: &str) -> Option<Layer> {
        match p {
            "g" => Some(Layer::Global),
            "b" => Some(Layer::Browse),
            "s" => Some(Layer::SpeechCursor),
            "e" => Some(Layer::Edit),
            _ => None,
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

/// A complete set of bindings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

fn parse_binding(s: &str, action: ActionId, platform: Platform, frontend: Frontend) -> Binding {
    let (layer, chord) = s
        .split_once(':')
        .and_then(|(p, c)| Layer::from_prefix(p).map(|l| (l, c)))
        .expect("default chord strings carry a layer prefix");
    let mut chord: KeyChord = chord.parse().expect("default chord strings parse");
    if frontend == Frontend::Gui && platform == Platform::MacOs {
        chord = chord.ctrl_to_meta();
    }
    Binding {
        chord,
        layer,
        action,
    }
}

impl Keymap {
    /// The default keymap.
    pub fn defaults(platform: Platform, frontend: Frontend) -> Self {
        let mut bindings = Vec::new();
        for &action in ActionId::ALL {
            let d = action.defaults();
            let own = match frontend {
                Frontend::Gui => d.gui,
                Frontend::Terminal => d.terminal,
            };
            for s in own.iter().chain(d.shared) {
                bindings.push(parse_binding(s, action, platform, frontend));
            }
        }
        Keymap { bindings }
    }

    /// Defaults with user overrides applied. Each override replaces all of
    /// an action's bindings (an empty list unbinds it). Chords go to the
    /// action's first default layer, or to Browse for single keys and
    /// Global otherwise. Returns warnings for unknown actions and bad chords.
    pub fn with_overrides(
        platform: Platform,
        frontend: Frontend,
        overrides: &BTreeMap<String, Vec<String>>,
    ) -> (Self, Vec<String>) {
        let mut map = Keymap::defaults(platform, frontend);
        let mut warnings = Vec::new();
        for (id, chords) in overrides {
            let Some(action) = ActionId::from_id(id) else {
                warnings.push(format!("unknown action {id:?} in keymap.toml"));
                continue;
            };
            let default_layer = map
                .bindings
                .iter()
                .find(|b| b.action == action)
                .map(|b| b.layer);
            map.bindings.retain(|b| b.action != action);
            for c in chords {
                match c.parse::<KeyChord>() {
                    Ok(chord) => {
                        let layer = default_layer.unwrap_or(if chord.is_text_input() {
                            Layer::Browse
                        } else {
                            Layer::Global
                        });
                        map.bindings.push(Binding {
                            chord,
                            layer,
                            action,
                        });
                    }
                    Err(e) => warnings.push(format!("{id}: {e}")),
                }
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
            for mode in [
                Layer::Global,
                Layer::Browse,
                Layer::SpeechCursor,
                Layer::Edit,
            ] {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &str) -> KeyChord {
        s.parse().unwrap()
    }

    #[test]
    fn defaults_have_no_conflicts() {
        for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
            for frontend in [Frontend::Gui, Frontend::Terminal] {
                let c = Keymap::defaults(platform, frontend).conflicts();
                assert!(c.is_empty(), "{platform:?} {frontend:?}: {c:#?}");
            }
        }
    }

    #[test]
    fn every_action_has_a_default() {
        for frontend in [Frontend::Gui, Frontend::Terminal] {
            let map = Keymap::defaults(Platform::Linux, frontend);
            for a in ActionId::ALL {
                assert!(
                    !map.chords_for(*a).is_empty(),
                    "{a:?} unbound on {frontend:?}"
                );
            }
        }
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

    #[test]
    fn mac_gui_uses_cmd() {
        let map = Keymap::defaults(Platform::MacOs, Frontend::Gui);
        assert_eq!(map.lookup(&k("Cmd+O"), Layer::Browse), Some(ActionId::Open));
    }
}
