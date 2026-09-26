//! Keymap presets: whole sets of changes to the default keys, chosen with
//! `[keyboard] preset` in `settings.toml`. The user's `keymap.toml`
//! overrides apply on top of the preset.
//!
//! The **default** keys mirror the quick navigation keys of NVDA's and
//! JAWS's browse mode (see [`ActionId`] and `docs/keyboard.md`). The
//! **classic** preset brings back textweaver's earlier single keys, as they
//! were before 2026-09-26:
//!
//! - `.` and `,` move by sentence, `s` says the sentence, `Shift+S` the
//!   paragraph, and `l` the line;
//! - `Shift+H` and `Shift+L` go back and forward in history;
//! - `o` and `Shift+O` move by list, `k` scrolls up, `Shift+K` says the
//!   link address;
//! - `q` and `Shift+Q` quit (after "Quit textweaver? y or n");
//! - `Alt+Down` and `Alt+Up` step through notes.
//!
//! The quick navigation keys that classic gives back (`q`, `s`, `g`, `d`
//! and their Shift forms) leave block quotes, separators, and graphics to
//! the command palette there ([`Preset::palette_only`]); chapters keep
//! their chords. Chords that did not clash with classic keys stay:
//! `Ctrl+Down` and `Ctrl+Up` for paragraphs, `F12` and `Shift+F12` for
//! notes, `Backspace` and `\` for history, `Shift+J` to scroll up.
//!
//! The **screen-reader** preset of Phase 2 became the default; its id is
//! still read and means the default keys.

use serde::{Deserialize, Serialize};

use crate::ActionId;

/// A named set of changes to the default keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    /// The default keys: NVDA and JAWS browse-mode quick navigation.
    #[default]
    #[serde(alias = "screen-reader")]
    Default,
    /// textweaver's earlier single keys (Star's, plus what Star lacked).
    Classic,
}

impl Preset {
    /// Every preset.
    pub const ALL: [Preset; 2] = [Preset::Default, Preset::Classic];

    /// The id in `settings.toml`: `default` or `classic`.
    pub fn id(self) -> &'static str {
        match self {
            Preset::Default => "default",
            Preset::Classic => "classic",
        }
    }

    /// The preset for an id. `screen-reader` (and `screen_reader`), the
    /// Phase 2 preset that became the default, means [`Preset::Default`].
    pub fn from_id(id: &str) -> Option<Preset> {
        let id = id.trim().to_ascii_lowercase().replace('_', "-");
        if id == "screen-reader" {
            return Some(Preset::Default);
        }
        Self::ALL.into_iter().find(|p| p.id() == id)
    }

    /// The changes: each layer-prefixed chord, and the action it triggers
    /// under this preset, or `None` to leave the chord unbound. Whatever
    /// that chord did by default no longer reaches it.
    pub fn changes(self) -> &'static [(&'static str, Option<ActionId>)] {
        match self {
            Preset::Default => &[],
            Preset::Classic => CLASSIC,
        }
    }

    /// Actions with default keys that this preset leaves to the command
    /// palette, because their keys went back to classic commands.
    pub fn palette_only(self) -> &'static [ActionId] {
        match self {
            Preset::Default => &[],
            Preset::Classic => &[
                ActionId::NextBlockQuote,
                ActionId::PreviousBlockQuote,
                ActionId::NextSeparator,
                ActionId::PreviousSeparator,
                ActionId::NextGraphic,
                ActionId::PreviousGraphic,
            ],
        }
    }
}

const CLASSIC: &[(&str, Option<ActionId>)] = &[
    ("b:.", Some(ActionId::NextSentence)),
    ("b:,", Some(ActionId::PreviousSentence)),
    ("b:s", Some(ActionId::ReadCurrentSentence)),
    ("b:Shift+S", Some(ActionId::ReadParagraph)),
    ("b:l", Some(ActionId::ReadCurrentLine)),
    ("b:Shift+H", Some(ActionId::HistoryBack)),
    ("b:Shift+L", Some(ActionId::HistoryForward)),
    ("b:o", Some(ActionId::NextList)),
    ("b:Shift+O", Some(ActionId::PreviousList)),
    ("b:k", Some(ActionId::ScrollUp)),
    ("b:Shift+K", Some(ActionId::LinkAddress)),
    ("b:q", Some(ActionId::Quit)),
    ("b:Shift+Q", Some(ActionId::Quit)),
    ("b:g", None),
    ("b:Shift+G", None),
    ("b:d", None),
    ("b:Shift+D", None),
    ("g:Alt+Down", Some(ActionId::NextNote)),
    ("g:Alt+Up", Some(ActionId::PreviousNote)),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for p in Preset::ALL {
            assert_eq!(Preset::from_id(p.id()), Some(p));
        }
        assert_eq!(Preset::from_id("Screen_Reader"), Some(Preset::Default));
        assert_eq!(Preset::from_id(" Classic "), Some(Preset::Classic));
        assert_eq!(Preset::from_id("vim"), None);
        assert!(Preset::Default.changes().is_empty());
        assert!(Preset::Default.palette_only().is_empty());
    }
}
