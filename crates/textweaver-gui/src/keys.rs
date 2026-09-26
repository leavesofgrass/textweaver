//! Keyboard input: wxWidgets key events to [`KeyChord`]s, and the keymap's
//! chords to menu accelerator strings.
//!
//! Three paths reach the app:
//!
//! 1. **Menu accelerators** (`Ctrl+O`, `Alt+.`, `F3`, `Escape`): wxWidgets
//!    translates them before the text control sees the key, and the menu
//!    event dispatches the action. Screen readers read them from the menu.
//! 2. **Key down** on the document: every other chord with a modifier, a
//!    function key, or a named key (Space, Enter, Escape).
//! 3. **Char** on the document: printable characters, which carry the
//!    layout's shifted character (`>` rather than `Shift+.`), matched against
//!    the Browse layer's single keys (`.` `,` `p` `h` `?` ...).
//!
//! Plain caret keys (arrows, Home, End, Page Up, Page Down, with Shift or
//! Ctrl) and Tab are left to the native control, so the screen reader reads
//! characters, words, and lines as the caret moves and focus moves as usual;
//! the app's cursor follows the native caret before the next action.

use textweaver_app::keymap::{Key, KeyChord, Modifiers, Platform};
use wxdragon::keycode as wxk;

/// The modifier state of a key event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    /// Control (on macOS, the Control key, not Command).
    pub ctrl: bool,
    /// Alt / Option.
    pub alt: bool,
    /// Shift.
    pub shift: bool,
    /// Command on macOS; ignored elsewhere (the Windows key is the system's).
    pub meta: bool,
}

impl Mods {
    fn to_modifiers(self, platform: Platform) -> Modifiers {
        let mut m = Modifiers::empty();
        m.set(Modifiers::CTRL, self.ctrl);
        m.set(Modifiers::ALT, self.alt);
        m.set(Modifiers::SHIFT, self.shift);
        m.set(Modifiers::META, self.meta && platform == Platform::MacOs);
        m
    }

    fn any_command(self, platform: Platform) -> bool {
        self.ctrl || self.alt || (self.meta && platform == Platform::MacOs)
    }
}

/// The chord for a key-down event, or `None` when the key is left to the
/// char event (a printable key without Ctrl, Alt, or Command), is a lone
/// modifier, or is unknown.
pub fn chord_from_key_down(code: i32, mods: Mods, platform: Platform) -> Option<KeyChord> {
    let named = match code {
        wxk::WXK_SPACE => Some(Key::Space),
        wxk::WXK_RETURN | wxk::WXK_NUMPAD_ENTER => Some(Key::Enter),
        wxk::WXK_ESCAPE => Some(Key::Escape),
        wxk::WXK_TAB => Some(Key::Tab),
        wxk::WXK_BACK => Some(Key::Backspace),
        wxk::WXK_DELETE => Some(Key::Delete),
        wxk::WXK_INSERT => Some(Key::Insert),
        wxk::WXK_HOME => Some(Key::Home),
        wxk::WXK_END => Some(Key::End),
        wxk::WXK_PAGEUP => Some(Key::PageUp),
        wxk::WXK_PAGEDOWN => Some(Key::PageDown),
        wxk::WXK_UP => Some(Key::Up),
        wxk::WXK_DOWN => Some(Key::Down),
        wxk::WXK_LEFT => Some(Key::Left),
        wxk::WXK_RIGHT => Some(Key::Right),
        c if (wxk::WXK_F1..=wxk::WXK_F24).contains(&c) => {
            u8::try_from(c - wxk::WXK_F1 + 1).ok().map(Key::F)
        }
        _ => None,
    };
    if let Some(key) = named {
        return Some(KeyChord::new(key, mods.to_modifiers(platform)));
    }
    // Printable ASCII: only as a command chord. Key-down codes are the
    // unshifted key (letters uppercase), so Shift with punctuation is
    // ambiguous across layouts and left alone.
    let c = u8::try_from(code).ok().filter(u8::is_ascii_graphic)? as char;
    if !mods.any_command(platform) || (mods.shift && !c.is_ascii_alphabetic()) {
        return None;
    }
    let c = c.to_ascii_lowercase();
    Some(KeyChord::new(Key::Char(c), mods.to_modifiers(platform)))
}

/// The chord for a char event: a printable character typed without Ctrl,
/// Alt, or Command (those arrive as key-down chords).
pub fn chord_from_char(unicode: i32, mods: Mods, platform: Platform) -> Option<KeyChord> {
    if mods.any_command(platform) {
        return None;
    }
    let c = u32::try_from(unicode).ok().and_then(char::from_u32)?;
    if c.is_control() || c == ' ' {
        return None;
    }
    Some(KeyChord::plain(Key::Char(c)))
}

/// True for chords the native text control should handle itself: caret
/// movement and selection (with Shift or Ctrl) and Tab focus movement.
pub fn is_native(chord: &KeyChord) -> bool {
    let caret = matches!(
        chord.key,
        Key::Up
            | Key::Down
            | Key::Left
            | Key::Right
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
    );
    let focus = chord.key == Key::Tab;
    (caret || focus) && (chord.mods - (Modifiers::SHIFT | Modifiers::CTRL)).is_empty()
}

/// True for chords that can be menu accelerators without stealing keys from
/// other controls: chords with Ctrl, Alt, or Command, function keys, and
/// Escape. Plain characters and Space stay document-only keys.
pub fn is_accelerator(chord: &KeyChord) -> bool {
    let named_ok = matches!(chord.key, Key::F(_) | Key::Escape);
    let with_command = chord
        .mods
        .intersects(Modifiers::CTRL | Modifiers::ALT | Modifiers::META);
    (named_ok || with_command) && !is_native(chord)
}

/// The wxWidgets accelerator string for `chord` (after a tab in a menu
/// label), or `None` if wxWidgets cannot express it. On macOS wxWidgets
/// spells Command as `Ctrl` (and the real Control key as `RawCtrl`).
pub fn accelerator_text(chord: &KeyChord, platform: Platform) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let m = chord.mods;
    if platform == Platform::MacOs {
        if m.contains(Modifiers::META) {
            parts.push("Ctrl".into());
        }
        if m.contains(Modifiers::CTRL) {
            parts.push("RawCtrl".into());
        }
    } else if m.contains(Modifiers::CTRL) {
        parts.push("Ctrl".into());
    }
    if m.contains(Modifiers::ALT) {
        parts.push("Alt".into());
    }
    let key = match chord.key {
        Key::Char('+') => return None,
        Key::Char(c) if c.is_ascii_uppercase() => {
            parts.push("Shift".into());
            c.to_string()
        }
        Key::Char(c) => {
            if m.contains(Modifiers::SHIFT) {
                parts.push("Shift".into());
            }
            c.to_ascii_uppercase().to_string()
        }
        k => {
            if m.contains(Modifiers::SHIFT) {
                parts.push("Shift".into());
            }
            match k {
                Key::F(n) => format!("F{n}"),
                Key::Space => "Space".into(),
                Key::Enter => "Enter".into(),
                Key::Escape => "Esc".into(),
                Key::Tab => "Tab".into(),
                Key::Backspace => "Back".into(),
                Key::Delete => "Delete".into(),
                Key::Insert => "Insert".into(),
                Key::Home => "Home".into(),
                Key::End => "End".into(),
                Key::PageUp => "PageUp".into(),
                Key::PageDown => "PageDown".into(),
                Key::Up => "Up".into(),
                Key::Down => "Down".into(),
                Key::Left => "Left".into(),
                Key::Right => "Right".into(),
                Key::Char(_) => return None,
            }
        }
    };
    parts.push(key);
    Some(parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIN: Platform = Platform::Windows;

    fn k(s: &str) -> KeyChord {
        s.parse().unwrap()
    }

    fn mods(ctrl: bool, alt: bool, shift: bool) -> Mods {
        Mods {
            ctrl,
            alt,
            shift,
            meta: false,
        }
    }

    #[test]
    fn key_down_named_and_command_chords() {
        let none = Mods::default();
        assert_eq!(
            chord_from_key_down(wxk::WXK_SPACE, none, WIN),
            Some(k("Space"))
        );
        assert_eq!(
            chord_from_key_down(wxk::WXK_F1 + 2, none, WIN),
            Some(k("F3"))
        );
        assert_eq!(
            chord_from_key_down('P' as i32, mods(true, false, true), WIN),
            Some(k("Ctrl+Shift+P"))
        );
        assert_eq!(
            chord_from_key_down('.' as i32, mods(false, true, false), WIN),
            Some(k("Alt+."))
        );
        assert_eq!(
            chord_from_key_down(wxk::WXK_LEFT, mods(false, true, false), WIN),
            Some(k("Alt+Left"))
        );
    }

    #[test]
    fn key_down_leaves_plain_characters_to_char_events() {
        assert_eq!(chord_from_key_down('P' as i32, Mods::default(), WIN), None);
        assert_eq!(
            chord_from_key_down('.' as i32, mods(false, false, true), WIN),
            None
        );
        // Shifted punctuation with Alt is layout dependent: left alone.
        assert_eq!(
            chord_from_key_down('.' as i32, mods(false, true, true), WIN),
            None
        );
        assert_eq!(
            chord_from_key_down(wxk::WXK_SHIFT, mods(false, false, true), WIN),
            None
        );
    }

    #[test]
    fn char_events_are_plain_characters() {
        let none = Mods::default();
        assert_eq!(chord_from_char('>' as i32, none, WIN), Some(k(">")));
        assert_eq!(
            chord_from_char('P' as i32, mods(false, false, true), WIN),
            Some(k("Shift+P"))
        );
        assert_eq!(chord_from_char(' ' as i32, none, WIN), None);
        assert_eq!(chord_from_char(13, none, WIN), None);
        assert_eq!(
            chord_from_char('p' as i32, mods(true, false, false), WIN),
            None
        );
    }

    #[test]
    fn caret_keys_and_tab_stay_native() {
        for s in [
            "Up",
            "Shift+Down",
            "Ctrl+Home",
            "Ctrl+Shift+End",
            "PageDown",
            "Tab",
            "Shift+Tab",
        ] {
            assert!(is_native(&k(s)), "{s}");
        }
        for s in ["Alt+Left", "Alt+PageDown", "Space", "."] {
            assert!(!is_native(&k(s)), "{s}");
        }
    }

    #[test]
    fn accelerators_skip_document_keys() {
        for s in ["Ctrl+O", "Alt+.", "F3", "Escape", "Ctrl+Space", "Alt+Right"] {
            assert!(is_accelerator(&k(s)), "{s}");
        }
        for s in ["Space", ".", "Shift+P", "Ctrl+Home", "Tab"] {
            assert!(!is_accelerator(&k(s)), "{s}");
        }
    }

    #[test]
    fn accelerator_strings_wx_understands() {
        let t = |s: &str| accelerator_text(&k(s), WIN);
        assert_eq!(t("Ctrl+Shift+P").as_deref(), Some("Ctrl+Shift+P"));
        assert_eq!(t("Alt+.").as_deref(), Some("Alt+."));
        assert_eq!(t("Ctrl+=").as_deref(), Some("Ctrl+="));
        assert_eq!(t("Escape").as_deref(), Some("Esc"));
        assert_eq!(t("Alt+PageDown").as_deref(), Some("Alt+PageDown"));
        assert_eq!(t("Ctrl++"), None);
        let mac = accelerator_text(&k("Cmd+O"), Platform::MacOs);
        assert_eq!(mac.as_deref(), Some("Ctrl+O"));
    }
}
