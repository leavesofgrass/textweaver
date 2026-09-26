//! Default keys for the commands the keymap has no actions for yet (notes,
//! highlights, bookmark management), shared by every frontend.
//!
//! These are a stopgap until Agent C2 adds the actions to `ActionId` (see
//! the wave 2 report's contract change requests); frontends consult them
//! only after the keymap found nothing, and a test keeps them free of
//! conflicts with the default keymap.

use std::str::FromStr;

use textweaver_keymap::{KeyChord, Layer};

use crate::command::{Command, NoteCommand};

/// The default extra bindings: `(chord, layer, command)`. Browse-layer keys
/// also work in Speech Cursor mode (the same lookup order as the keymap).
pub fn extra_bindings() -> Vec<(KeyChord, Layer, Command)> {
    const TABLE: [(&str, NoteCommand); 6] = [
        ("a", NoteCommand::Add),
        ("Shift+A", NoteCommand::List),
        ("'", NoteCommand::Next),
        ("\"", NoteCommand::Previous),
        ("y", NoteCommand::ToggleHighlight),
        ("Shift+Y", NoteCommand::ListHighlights),
    ];
    TABLE
        .iter()
        .filter_map(|(chord, c)| {
            KeyChord::from_str(chord)
                .ok()
                .map(|k| (k, Layer::Browse, Command::Notes(*c)))
        })
        .collect()
}

/// The extra command bound to `chord` in a mode using `layer`, if any.
pub fn extra_lookup(chord: &KeyChord, layer: Layer) -> Option<Command> {
    let order = layer.lookup_order();
    extra_bindings()
        .into_iter()
        .find(|(k, l, _)| k == chord && order.contains(l))
        .map(|(_, _, c)| c)
}

/// The chords bound to an extra command, for help text.
pub fn extra_chords(cmd: NoteCommand) -> Vec<KeyChord> {
    extra_bindings()
        .into_iter()
        .filter(|(_, _, c)| *c == Command::Notes(cmd))
        .map(|(k, _, _)| k)
        .collect()
}

#[cfg(test)]
mod tests {
    use textweaver_keymap::{Frontend, Keymap, Platform};

    use super::*;

    #[test]
    fn extra_keys_parse_and_do_not_clash_with_the_keymap() {
        let extra = extra_bindings();
        assert_eq!(extra.len(), 6);
        for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
            for frontend in [Frontend::Terminal, Frontend::Gui] {
                let km = Keymap::defaults(platform, frontend);
                for (chord, layer, _) in &extra {
                    for mode in [Layer::Browse, Layer::SpeechCursor] {
                        assert!(
                            km.lookup(chord, mode).is_none(),
                            "{chord} in {layer:?} is taken on {platform:?} {frontend:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn lookup_follows_layer_order() {
        let a = KeyChord::from_str("a").unwrap();
        assert_eq!(
            extra_lookup(&a, Layer::SpeechCursor),
            Some(Command::Notes(NoteCommand::Add))
        );
        assert_eq!(extra_lookup(&a, Layer::Edit), None);
        assert_eq!(extra_chords(NoteCommand::List).len(), 1);
    }
}
