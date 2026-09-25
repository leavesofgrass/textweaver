//! Key chords: parsing, formatting, normalization.
//!
//! Normalization makes chords from different sources compare equal:
//! - a `Char` key never carries `SHIFT`: shifted letters are uppercase
//!   (`Shift+p` is `P`) and other shifted characters are the character
//!   produced (`Shift+5` on a US layout arrives as `%`);
//! - `Char(' ')` is [`Key::Space`]; `BackTab` is `Shift+Tab`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

bitflags::bitflags! {
    /// Modifier keys.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
    pub struct Modifiers: u8 {
        /// Control.
        const CTRL = 1 << 0;
        /// Alt (Option on macOS).
        const ALT = 1 << 1;
        /// Shift.
        const SHIFT = 1 << 2;
        /// Command on macOS, Windows key elsewhere.
        const META = 1 << 3;
    }
}

/// A key, independent of modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Key {
    /// A printable character.
    Char(char),
    /// A function key, F1 to F24.
    F(u8),
    /// Space bar.
    Space,
    /// Enter / Return.
    Enter,
    /// Escape.
    Escape,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
    /// Insert.
    Insert,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
}

/// A key plus modifiers, normalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KeyChord {
    /// The key.
    pub key: Key,
    /// The modifiers.
    pub mods: Modifiers,
}

/// Chord parse failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChordError {
    /// Empty string.
    #[error("empty key chord")]
    Empty,
    /// Unknown modifier name.
    #[error("unknown modifier {0:?}")]
    UnknownModifier(String),
    /// Unknown key name.
    #[error("unknown key {0:?}")]
    UnknownKey(String),
}

impl KeyChord {
    /// A normalized chord.
    pub fn new(key: Key, mods: Modifiers) -> Self {
        let mut mods = mods;
        let key = match key {
            Key::Char(' ') => Key::Space,
            Key::Char(c) => {
                let c = if mods.contains(Modifiers::SHIFT) && c.is_ascii_lowercase() {
                    c.to_ascii_uppercase()
                } else {
                    c
                };
                mods.remove(Modifiers::SHIFT);
                Key::Char(c)
            }
            k => k,
        };
        KeyChord { key, mods }
    }

    /// A chord with no modifiers.
    pub fn plain(key: Key) -> Self {
        KeyChord::new(key, Modifiers::empty())
    }

    /// True for chords that type text when no binding claims them: printable
    /// characters and Space with at most Shift. Browse-layer bindings are
    /// all of this kind.
    pub fn is_text_input(&self) -> bool {
        matches!(self.key, Key::Char(_) | Key::Space)
            && !self
                .mods
                .intersects(Modifiers::CTRL | Modifiers::ALT | Modifiers::META)
    }

    /// Replaces Ctrl with Cmd (macOS GUI convention).
    pub fn ctrl_to_meta(mut self) -> Self {
        if self.mods.contains(Modifiers::CTRL) {
            self.mods.remove(Modifiers::CTRL);
            self.mods.insert(Modifiers::META);
        }
        self
    }
}

fn key_name(k: Key) -> String {
    match k {
        Key::Char(c) => c.to_string(),
        Key::F(n) => format!("F{n}"),
        Key::Space => "Space".into(),
        Key::Enter => "Enter".into(),
        Key::Escape => "Escape".into(),
        Key::Tab => "Tab".into(),
        Key::Backspace => "Backspace".into(),
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
    }
}

impl fmt::Display for KeyChord {
    /// `Ctrl+Shift+P`, `Alt+.`, `Shift+T` for an uppercase letter, `t`,
    /// `Ctrl++`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.mods.contains(Modifiers::CTRL) {
            parts.push("Ctrl".into());
        }
        if self.mods.contains(Modifiers::META) {
            parts.push("Cmd".into());
        }
        if self.mods.contains(Modifiers::ALT) {
            parts.push("Alt".into());
        }
        let key = match self.key {
            Key::Char(c) if c.is_ascii_uppercase() => {
                parts.push("Shift".into());
                c.to_string()
            }
            Key::Char(c) if c.is_ascii_lowercase() && !self.mods.is_empty() => {
                c.to_ascii_uppercase().to_string()
            }
            k => {
                if self.mods.contains(Modifiers::SHIFT) {
                    parts.push("Shift".into());
                }
                key_name(k)
            }
        };
        parts.push(key);
        f.write_str(&parts.join("+"))
    }
}

fn parse_key(s: &str) -> Result<Key, ChordError> {
    let lower = s.to_ascii_lowercase();
    let k = match lower.as_str() {
        "space" => Key::Space,
        "enter" | "return" => Key::Enter,
        "esc" | "escape" => Key::Escape,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "del" | "delete" => Key::Delete,
        "ins" | "insert" => Key::Insert,
        "home" => Key::Home,
        "end" => Key::End,
        "pgup" | "pageup" => Key::PageUp,
        "pgdn" | "pagedown" => Key::PageDown,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "plus" => Key::Char('+'),
        "minus" => Key::Char('-'),
        _ => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Char(c),
                _ => match lower.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
                    Some(n) if (1..=24).contains(&n) => Key::F(n),
                    _ => return Err(ChordError::UnknownKey(s.to_owned())),
                },
            }
        }
    };
    Ok(k)
}

impl FromStr for KeyChord {
    type Err = ChordError;

    /// Parses `Ctrl+Shift+P`, `alt+.`, `Ctrl++`, `F3`, `Shift+t`, `T`.
    /// Letters combined with a modifier are case-insensitive (`Ctrl+p` is
    /// `Ctrl+P`); a bare uppercase letter means Shift.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ChordError::Empty);
        }
        let (prefix, key_str) = if s == "+" {
            ("", "+")
        } else if let Some(p) = s.strip_suffix("++") {
            (p, "+")
        } else {
            match s.rsplit_once('+') {
                Some((p, k)) => (p, k),
                None => ("", s),
            }
        };
        let mut mods = Modifiers::empty();
        for m in prefix.split('+').filter(|m| !m.is_empty()) {
            mods |= match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => Modifiers::CTRL,
                "alt" | "option" | "opt" => Modifiers::ALT,
                "shift" => Modifiers::SHIFT,
                "cmd" | "command" | "meta" | "super" | "win" => Modifiers::META,
                _ => return Err(ChordError::UnknownModifier(m.to_owned())),
            };
        }
        let mut key = parse_key(key_str)?;
        // With Ctrl/Alt/Meta, a letter's case carries no meaning; Shift must
        // be explicit.
        if let Key::Char(c) = key {
            if c.is_ascii_alphabetic()
                && mods.intersects(Modifiers::CTRL | Modifiers::ALT | Modifiers::META)
            {
                key = Key::Char(c.to_ascii_lowercase());
            }
        }
        Ok(KeyChord::new(key, mods))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> KeyChord {
        s.parse().unwrap()
    }

    #[test]
    fn parse_and_format_round_trip() {
        for s in [
            "Ctrl+Shift+P",
            "Alt+.",
            "Ctrl++",
            "F3",
            "Shift+T",
            "t",
            "Space",
            "Alt+Left",
            "Ctrl+-",
            "Ctrl+=",
        ] {
            assert_eq!(p(s).to_string(), s, "{s}");
        }
    }

    #[test]
    fn normalization() {
        assert_eq!(p("Shift+t"), p("T"));
        assert_eq!(p("ctrl+p"), p("Ctrl+P"));
        assert_ne!(p("Ctrl+P"), p("Ctrl+Shift+P"));
        assert_eq!(
            KeyChord::new(Key::Char(' '), Modifiers::empty()),
            p("Space")
        );
        assert_eq!(KeyChord::new(Key::Char('%'), Modifiers::SHIFT), p("%"));
        assert!(p("t").is_text_input());
        assert!(!p("Ctrl+T").is_text_input());
    }

    #[test]
    fn errors() {
        assert!("".parse::<KeyChord>().is_err());
        assert!("Hyper+X".parse::<KeyChord>().is_err());
        assert!("Ctrl+Nope".parse::<KeyChord>().is_err());
    }
}
