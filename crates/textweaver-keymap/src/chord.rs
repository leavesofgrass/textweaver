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

    /// Why a terminal cannot deliver this chord, or `None` when it can.
    ///
    /// Terminals send bytes, not key events: `Ctrl+H` is the Backspace byte,
    /// `Ctrl+I` is Tab, `Ctrl+M` and `Ctrl+J` are Enter, `Ctrl+Shift+P` is
    /// the same byte as `Ctrl+P`, Ctrl with digits or punctuation sends
    /// nothing useful, and the Command or Windows key never arrives. Use it
    /// to warn about user overrides; the terminal defaults pass it (a test).
    pub fn terminal_limitation(&self) -> Option<&'static str> {
        let m = self.mods;
        let ctrl = m.contains(Modifiers::CTRL);
        let alt = m.contains(Modifiers::ALT);
        let shift = m.contains(Modifiers::SHIFT);
        if m.contains(Modifiers::META) {
            return Some("terminals do not pass the Command or Windows key");
        }
        match self.key {
            Key::Char(c) if ctrl => match c {
                'A'..='Z' => Some("terminals cannot tell Ctrl+Shift+letter from Ctrl+letter"),
                'h' => Some("Ctrl+H is Backspace in a terminal"),
                'i' => Some("Ctrl+I is Tab in a terminal"),
                'm' | 'j' => Some("Ctrl+M and Ctrl+J are Enter in a terminal"),
                'a'..='z' => None,
                _ => Some("terminals cannot send Ctrl with this key"),
            },
            Key::Space if ctrl && (shift || alt) => {
                Some("terminals send Ctrl+Space without Shift or Alt")
            }
            Key::Enter | Key::Tab | Key::Backspace | Key::Escape if ctrl => {
                Some("terminals cannot send Ctrl with this key")
            }
            Key::Enter | Key::Backspace | Key::Space if shift => {
                Some("terminals cannot tell this key with Shift from without")
            }
            Key::Enter if alt => Some("Alt+Enter toggles full screen in many terminals"),
            Key::Tab if alt => Some("Alt+Tab switches windows"),
            _ => None,
        }
    }

    /// True when the chord means the same key on every keyboard layout:
    /// unmodified keys always do; modified ones must use a letter, a digit,
    /// an unshifted US punctuation key (`` ` - = [ ] \ ; ' , . / ``), or a
    /// named key. `Ctrl+*` is `Ctrl+Shift+8` on a US layout and something
    /// else elsewhere, so defaults avoid it.
    pub fn is_layout_independent(&self) -> bool {
        match self.key {
            Key::Char(c) if !self.mods.is_empty() => {
                c.is_ascii_alphanumeric() || "`-=[]\\;',./".contains(c)
            }
            _ => true,
        }
    }

    /// The chord as it should be spoken: `Control Shift P`, `Alt period`,
    /// `question mark`. Punctuation is named, so speech with punctuation
    /// turned off still says it.
    pub fn spoken(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.mods.contains(Modifiers::CTRL) {
            parts.push("Control");
        }
        if self.mods.contains(Modifiers::META) {
            parts.push("Command");
        }
        if self.mods.contains(Modifiers::ALT) {
            parts.push("Alt");
        }
        let key = match self.key {
            Key::Char(c) if c.is_ascii_uppercase() => {
                parts.push("Shift");
                c.to_string()
            }
            Key::Char(c) if c.is_ascii_lowercase() => c.to_ascii_uppercase().to_string(),
            Key::Char(c) => char_name(c).map_or_else(|| c.to_string(), str::to_owned),
            k => {
                if self.mods.contains(Modifiers::SHIFT) {
                    parts.push("Shift");
                }
                spoken_key_name(k)
            }
        };
        let mut out = parts.join(" ");
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&key);
        out
    }
}

/// Spoken names for punctuation keys.
fn char_name(c: char) -> Option<&'static str> {
    Some(match c {
        '.' => "period",
        ',' => "comma",
        ';' => "semicolon",
        ':' => "colon",
        '\'' => "apostrophe",
        '"' => "quote",
        '`' => "grave accent",
        '~' => "tilde",
        '!' => "exclamation mark",
        '?' => "question mark",
        '@' => "at sign",
        '#' => "number sign",
        '$' => "dollar sign",
        '%' => "percent",
        '^' => "caret",
        '&' => "ampersand",
        '*' => "asterisk",
        '(' => "left parenthesis",
        ')' => "right parenthesis",
        '[' => "left bracket",
        ']' => "right bracket",
        '{' => "left brace",
        '}' => "right brace",
        '<' => "less than",
        '>' => "greater than",
        '+' => "plus",
        '-' => "minus",
        '=' => "equals",
        '_' => "underscore",
        '/' => "slash",
        '\\' => "backslash",
        '|' => "vertical bar",
        _ => return None,
    })
}

fn spoken_key_name(k: Key) -> String {
    match k {
        Key::PageUp => "Page Up".into(),
        Key::PageDown => "Page Down".into(),
        Key::Up => "Up Arrow".into(),
        Key::Down => "Down Arrow".into(),
        Key::Left => "Left Arrow".into(),
        Key::Right => "Right Arrow".into(),
        Key::Escape => "Escape".into(),
        other => key_name(other),
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
    fn terminal_limitations() {
        for bad in [
            "Ctrl+H",
            "Ctrl+I",
            "Ctrl+M",
            "Ctrl+J",
            "Ctrl+Shift+P",
            "Ctrl+1",
            "Ctrl+=",
            "Ctrl+`",
            "Cmd+O",
            "Ctrl+Enter",
            "Shift+Enter",
            "Alt+Enter",
            "Ctrl+Shift+Space",
        ] {
            assert!(p(bad).terminal_limitation().is_some(), "{bad}");
        }
        for good in [
            "Ctrl+P",
            "Ctrl+Space",
            "Alt+.",
            "Alt+Shift+P",
            "Ctrl+Home",
            "Alt+Left",
            "F3",
            "Shift+Tab",
            "T",
            "Escape",
        ] {
            assert_eq!(p(good).terminal_limitation(), None, "{good}");
        }
    }

    #[test]
    fn layout_independence() {
        assert!(p("Ctrl+Shift+P").is_layout_independent());
        assert!(p("Alt+.").is_layout_independent());
        assert!(p("%").is_layout_independent());
        assert!(!p("Ctrl+*").is_layout_independent());
        assert!(!p("Alt+(").is_layout_independent());
        // Shift+digit normalizes to the digit, which is why defaults never
        // spell chords that way.
        assert_eq!(p("Ctrl+Shift+8"), p("Ctrl+8"));
    }

    #[test]
    fn spoken_names() {
        assert_eq!(p("Ctrl+Shift+P").spoken(), "Control Shift P");
        assert_eq!(p("Alt+.").spoken(), "Alt period");
        assert_eq!(p("?").spoken(), "question mark");
        assert_eq!(p("t").spoken(), "T");
        assert_eq!(p("T").spoken(), "Shift T");
        assert_eq!(p("PageDown").spoken(), "Page Down");
        assert_eq!(p("Ctrl++").spoken(), "Control plus");
        assert_eq!(p("Shift+Tab").spoken(), "Shift Tab");
        assert_eq!(p("Space").spoken(), "Space");
    }

    #[test]
    fn errors() {
        assert!("".parse::<KeyChord>().is_err());
        assert!("Hyper+X".parse::<KeyChord>().is_err());
        assert!("Ctrl+Nope".parse::<KeyChord>().is_err());
    }
}
