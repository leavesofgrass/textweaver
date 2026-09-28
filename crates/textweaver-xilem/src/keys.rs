//! Keyboard input: Masonry's key events as the keymap's [`KeyChord`]s.
//!
//! The document view handles the caret keys itself (arrows, Home, End,
//! Page Up, Page Down, with Shift or Ctrl), so the screen reader reads
//! what the caret moves over. Tab moves focus. Every other key reaches the
//! root widget and becomes a chord for `Keymap::lookup`.
//!
//! A printable key arrives as the character the layout typed (`>` rather
//! than Shift with `.`), which is what the keymap's Browse layer stores;
//! with Ctrl, Alt, or Command a letter is reduced to its lowercase form,
//! with Shift kept as a modifier.

use masonry::core::keyboard::{Key, KeyboardEvent, NamedKey};
use textweaver_app::keymap::{Key as TwKey, KeyChord, Modifiers, Platform};

/// The chord for `event`, or `None` for a lone modifier or an unknown key.
pub fn chord(event: &KeyboardEvent, platform: Platform) -> Option<KeyChord> {
    let m = event.modifiers;
    let mut mods = Modifiers::empty();
    mods.set(Modifiers::CTRL, m.ctrl());
    mods.set(Modifiers::ALT, m.alt());
    mods.set(Modifiers::SHIFT, m.shift());
    mods.set(Modifiers::META, m.meta() && platform == Platform::MacOs);
    let command = m.ctrl() || m.alt() || (m.meta() && platform == Platform::MacOs);
    let key = match &event.key {
        Key::Named(n) => match n {
            NamedKey::Enter => TwKey::Enter,
            NamedKey::Escape => TwKey::Escape,
            NamedKey::Tab => TwKey::Tab,
            NamedKey::Backspace => TwKey::Backspace,
            NamedKey::Delete => TwKey::Delete,
            NamedKey::Insert => TwKey::Insert,
            NamedKey::Home => TwKey::Home,
            NamedKey::End => TwKey::End,
            NamedKey::PageUp => TwKey::PageUp,
            NamedKey::PageDown => TwKey::PageDown,
            NamedKey::ArrowUp => TwKey::Up,
            NamedKey::ArrowDown => TwKey::Down,
            NamedKey::ArrowLeft => TwKey::Left,
            NamedKey::ArrowRight => TwKey::Right,
            other => TwKey::F(function_number(other)?),
        },
        Key::Character(s) => {
            let mut chars = s.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            if c == ' ' {
                TwKey::Space
            } else if command {
                if c.is_ascii_alphabetic() {
                    // KeyChord::new folds Shift into an uppercase letter.
                    TwKey::Char(c.to_ascii_lowercase())
                } else {
                    // The character already carries Shift.
                    mods.remove(Modifiers::SHIFT);
                    TwKey::Char(c)
                }
            } else {
                if c.is_control() {
                    return None;
                }
                mods.remove(Modifiers::SHIFT);
                TwKey::Char(c)
            }
        }
    };
    Some(KeyChord::new(key, mods))
}

fn function_number(k: &NamedKey) -> Option<u8> {
    use NamedKey::*;
    let n = match k {
        F1 => 1,
        F2 => 2,
        F3 => 3,
        F4 => 4,
        F5 => 5,
        F6 => 6,
        F7 => 7,
        F8 => 8,
        F9 => 9,
        F10 => 10,
        F11 => 11,
        F12 => 12,
        F13 => 13,
        F14 => 14,
        F15 => 15,
        F16 => 16,
        F17 => 17,
        F18 => 18,
        F19 => 19,
        F20 => 20,
        F21 => 21,
        F22 => 22,
        F23 => 23,
        F24 => 24,
        _ => return None,
    };
    Some(n)
}

/// True for F1 to F24.
pub fn is_function_key(k: &NamedKey) -> bool {
    function_number(k).is_some()
}

/// True for chords the document view keeps: caret movement and selection
/// (with Shift or Ctrl), and Tab.
pub fn is_native(chord: &KeyChord) -> bool {
    let caret = matches!(
        chord.key,
        TwKey::Up
            | TwKey::Down
            | TwKey::Left
            | TwKey::Right
            | TwKey::Home
            | TwKey::End
            | TwKey::PageUp
            | TwKey::PageDown
    );
    (caret || chord.key == TwKey::Tab)
        && (chord.mods - (Modifiers::SHIFT | Modifiers::CTRL)).is_empty()
}

/// How a chord is written for a screen reader's shortcut property
/// (`Ctrl+O`, `Space`, `Alt+Period` is written `Alt+.`).
pub fn shortcut_text(chord: &KeyChord) -> String {
    chord.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use masonry::core::keyboard::{Code, KeyState, Modifiers as Mods};

    fn ev(key: Key, mods: Mods) -> KeyboardEvent {
        KeyboardEvent {
            state: KeyState::Down,
            key,
            code: Code::Unidentified,
            modifiers: mods,
            ..Default::default()
        }
    }

    fn c(s: &str) -> Key {
        Key::Character(s.into())
    }

    const WIN: Platform = Platform::Windows;

    #[test]
    fn printable_keys_are_the_typed_character() {
        assert_eq!(
            chord(&ev(c("p"), Mods::empty()), WIN),
            Some("p".parse().unwrap())
        );
        assert_eq!(
            chord(&ev(c(">"), Mods::SHIFT), WIN),
            Some(">".parse().unwrap())
        );
        assert_eq!(
            chord(&ev(c(" "), Mods::empty()), WIN),
            Some("Space".parse().unwrap())
        );
        assert_eq!(
            chord(&ev(c("P"), Mods::SHIFT), WIN),
            Some("P".parse().unwrap())
        );
    }

    #[test]
    fn command_chords_use_lowercase_letters() {
        assert_eq!(
            chord(&ev(c("o"), Mods::CONTROL), WIN),
            Some("Ctrl+O".parse().unwrap())
        );
        assert_eq!(
            chord(&ev(c("O"), Mods::CONTROL | Mods::SHIFT), WIN),
            Some("Ctrl+Shift+O".parse().unwrap())
        );
        assert_eq!(
            chord(&ev(c("."), Mods::ALT), WIN),
            Some("Alt+.".parse().unwrap())
        );
    }

    #[test]
    fn named_and_function_keys() {
        assert_eq!(
            chord(&ev(Key::Named(NamedKey::F3), Mods::empty()), WIN),
            Some("F3".parse().unwrap())
        );
        assert_eq!(
            chord(&ev(Key::Named(NamedKey::Escape), Mods::empty()), WIN),
            Some("Escape".parse().unwrap())
        );
        assert_eq!(
            chord(&ev(Key::Named(NamedKey::Shift), Mods::SHIFT), WIN),
            None
        );
    }

    #[test]
    fn caret_keys_are_native() {
        assert!(is_native(&"Down".parse().unwrap()));
        assert!(is_native(&"Ctrl+Shift+End".parse().unwrap()));
        assert!(is_native(&"Tab".parse().unwrap()));
        assert!(!is_native(&"Alt+Down".parse().unwrap()));
        assert!(!is_native(&"Space".parse().unwrap()));
    }
}
