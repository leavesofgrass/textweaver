//! Keymap presets: whole sets of changes to the default keys, chosen with
//! `[keyboard] preset` in `settings.toml`. The user's `keymap.toml`
//! overrides apply on top of the preset.
//!
//! The **screen-reader** preset follows the single-letter keys of NVDA's
//! and JAWS's browse mode: `h` and `Shift+H` for headings, `1` to `6` for
//! heading levels (as in the default keys), `l` and `Shift+L` for lists,
//! `i` for list items and `t` for tables (as in the default keys), and `k`
//! and `Shift+K` for links. What those keys did by default moves:
//!
//! - history back from `Shift+H` to `Backspace`, history forward from
//!   `Shift+L` to `\` (`Alt+Left` and `Alt+Right` stay);
//! - "read current line" loses `l` and keeps its chord (`Alt+Shift+L` in
//!   the terminal, `Ctrl+L` in the GUI);
//! - "scroll up" moves from `k` to `Ctrl+Up`, and "scroll down" gains
//!   `Ctrl+Down` beside `j`;
//! - "link address" loses `Shift+K` and keeps `Alt+Shift+K`.
//!
//! The default keys for lists (`o`, `Shift+O`) and links (`u`, `Shift+U`)
//! keep working.

use serde::{Deserialize, Serialize};

use crate::ActionId;

/// A named set of changes to the default keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    /// textweaver's own keys (Star's, plus what Star lacked).
    #[default]
    Default,
    /// Screen reader browse-mode habits: `h`, `l`, `k`, `t`, `i`, `1` to `6`.
    ScreenReader,
}

impl Preset {
    /// Every preset.
    pub const ALL: [Preset; 2] = [Preset::Default, Preset::ScreenReader];

    /// The id in `settings.toml`: `default` or `screen-reader`.
    pub fn id(self) -> &'static str {
        match self {
            Preset::Default => "default",
            Preset::ScreenReader => "screen-reader",
        }
    }

    /// The preset for an id (`screen_reader` is accepted too).
    pub fn from_id(id: &str) -> Option<Preset> {
        let id = id.trim().to_ascii_lowercase().replace('_', "-");
        Self::ALL.into_iter().find(|p| p.id() == id)
    }

    /// The changes: each layer-prefixed chord, and the action it triggers
    /// under this preset. Whatever that chord did by default no longer
    /// reaches it; every such action keeps another key (a test).
    pub fn changes(self) -> &'static [(&'static str, ActionId)] {
        match self {
            Preset::Default => &[],
            Preset::ScreenReader => SCREEN_READER,
        }
    }
}

const SCREEN_READER: &[(&str, ActionId)] = &[
    ("b:Shift+H", ActionId::SkipPreviousHeading),
    ("b:Backspace", ActionId::HistoryBack),
    ("b:l", ActionId::NextList),
    ("b:Shift+L", ActionId::PreviousList),
    ("b:\\", ActionId::HistoryForward),
    ("b:k", ActionId::NextLink),
    ("b:Shift+K", ActionId::PreviousLink),
    ("b:Ctrl+Up", ActionId::ScrollUp),
    ("b:Ctrl+Down", ActionId::ScrollDown),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for p in Preset::ALL {
            assert_eq!(Preset::from_id(p.id()), Some(p));
        }
        assert_eq!(Preset::from_id("Screen_Reader"), Some(Preset::ScreenReader));
        assert_eq!(Preset::from_id("vim"), None);
        assert!(Preset::Default.changes().is_empty());
    }
}
