//! Default keys for the commands the keymap has no actions for yet (notes,
//! highlights, bookmark management), shared by every frontend.
//!
//! The keymap now has actions for adding, listing, stepping through, and
//! deleting notes and for highlighting (`a`, `Shift+A`, `e`, `Shift+E`,
//! `Delete`, `y`); the extras list highlights on `Shift+Y`. Frontends
//! consult them only after the keymap found nothing, and a test keeps them
//! free of conflicts with the default keymap. The older `'` and `"` note
//! steps are gone: `"` is Shift+2 on UK, German, Spanish, Nordic, and
//! Italian keyboards, where it goes to the previous level 2 heading.

use std::str::FromStr;

use textweaver_keymap::{KeyChord, Keymap, Layer};

use crate::command::{Command, NoteCommand};

/// The default extra bindings: `(chord, layer, command)`. Browse-layer keys
/// also work in Speech Cursor mode (the same lookup order as the keymap).
pub fn extra_bindings() -> Vec<(KeyChord, Layer, Command)> {
    const TABLE: [(&str, NoteCommand); 1] = [("Shift+Y", NoteCommand::ListHighlights)];
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

/// The extra command bound to `chord`, as a frontend runs it: like
/// [`extra_lookup`], but a key that types text (`Shift+Y`) runs only while
/// the keymap's single-key shortcuts are on, as the keymap's own keys do.
pub fn extra_command(keymap: &Keymap, chord: &KeyChord, layer: Layer) -> Option<Command> {
    if !keymap.character_keys() && chord.is_text_input() {
        return None;
    }
    extra_lookup(chord, layer)
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
        assert_eq!(extra.len(), 1);
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
        let a = KeyChord::from_str("Shift+Y").unwrap();
        assert_eq!(
            extra_lookup(&a, Layer::SpeechCursor),
            Some(Command::Notes(NoteCommand::ListHighlights))
        );
        assert_eq!(extra_lookup(&a, Layer::Edit), None);
        assert_eq!(extra_chords(NoteCommand::ListHighlights).len(), 1);
    }

    /// The extra keys obey the single-key switch in every frontend: with
    /// single-key shortcuts off, Shift+Y types a Y instead of listing the
    /// highlights.
    #[test]
    fn extra_keys_obey_the_single_key_switch() {
        let y = KeyChord::from_str("Shift+Y").unwrap();
        for frontend in [Frontend::Terminal, Frontend::Gui] {
            let mut km = Keymap::defaults(Platform::Windows, frontend);
            assert_eq!(
                extra_command(&km, &y, Layer::Browse),
                Some(Command::Notes(NoteCommand::ListHighlights))
            );
            km.set_character_keys(false);
            assert_eq!(extra_command(&km, &y, Layer::Browse), None, "{frontend:?}");
        }
    }
}
