//! Interface announcements: how much textweaver says about itself
//! (`[accessibility] interface_announcements`; Wave 6, W6u, ADR-0043).
//!
//! Reading speech and math verbosity are about the document. This setting
//! is about the application's own messages: dialogs and lists opening and
//! closing, progress, hints ("Tab completes"), confirmations of routine
//! changes, and tips. A screen reader or a Braille display already tells a
//! user much of that, so the user can turn it down or off.
//!
//! Every announcement has an [`Importance`]; the [`InterfaceLevel`] lets
//! it through or not ([`lets_through`]). Errors, questions waiting for an
//! answer, and answers to what the user asked (a search's count, Where am
//! I, the item a move landed on) are never silenced.
//!
//! | Importance | off | minimal | normal | full |
//! |---|---|---|---|---|
//! | error, question, answer | said | said | said | said |
//! | result (a command's outcome, a list opened) | not said | said | said | said |
//! | routine (a list closed, a change confirmed), dialog, progress, tip | not said | not said | said | said |
//! | hint, detail (counts in progress) | not said | not said | not said | said |

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::AccessMode;

/// How much textweaver announces about itself.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceLevel {
    /// Only errors, questions, and answers to what the user asked.
    Off,
    /// Also the results of commands the user ran.
    Minimal,
    /// Also routine confirmations, dialogs opening and closing, progress at
    /// a calm pace, and tips.
    #[default]
    Normal,
    /// Also hints and counts in progress.
    Full,
}

impl InterfaceLevel {
    /// Every level, in the order the toggle cycles through them.
    pub const ALL: [InterfaceLevel; 4] = [
        InterfaceLevel::Off,
        InterfaceLevel::Minimal,
        InterfaceLevel::Normal,
        InterfaceLevel::Full,
    ];

    /// The id in `settings.toml`: `off`, `minimal`, `normal`, `full`.
    pub fn id(self) -> &'static str {
        match self {
            InterfaceLevel::Off => "off",
            InterfaceLevel::Minimal => "minimal",
            InterfaceLevel::Normal => "normal",
            InterfaceLevel::Full => "full",
        }
    }

    /// The next level, wrapping from full to off.
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|l| *l == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// The level when the setting is `auto`: minimal with a screen reader
    /// (screen-reader and hybrid modes), where the screen reader and the
    /// display already say what a window or list is; normal when
    /// textweaver speaks for itself.
    pub fn for_mode(mode: AccessMode) -> Self {
        match mode {
            AccessMode::SelfVoicing => InterfaceLevel::Normal,
            AccessMode::ScreenReader | AccessMode::Hybrid => InterfaceLevel::Minimal,
        }
    }
}

impl fmt::Display for InterfaceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for InterfaceLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|l| l.id() == s)
            .ok_or_else(|| format!("unknown interface announcement level {s:?}"))
    }
}

/// What an announcement is, which decides whether an [`InterfaceLevel`]
/// lets it through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Importance {
    /// Something failed, and what to do next. Never silenced.
    Error,
    /// A question waiting for an answer ("Quit textweaver? y or n"). Never
    /// silenced.
    Question,
    /// The answer to what the user asked: Where am I, a count, the item a
    /// move landed on, the text of a line. Never silenced.
    Answer,
    /// The outcome of a command the user ran ("Bionic reading on",
    /// "Saved"), or the name of a list or menu they opened.
    Result,
    /// A routine confirmation: a list or dialog closed, "Cancelled", a
    /// change textweaver made by itself.
    Routine,
    /// A dialog, window, or screen opening or closing on its own.
    Dialog,
    /// Progress of a long job before its result, at most every ten
    /// seconds.
    Progress,
    /// A first-run tip or a welcome.
    Tip,
    /// A hint about keys ("Tab completes; Up and Down list matches").
    Hint,
    /// Extra detail: counts in progress, positions in long lists.
    Detail,
}

impl Importance {
    /// Every kind.
    pub const ALL: [Importance; 10] = [
        Importance::Error,
        Importance::Question,
        Importance::Answer,
        Importance::Result,
        Importance::Routine,
        Importance::Dialog,
        Importance::Progress,
        Importance::Tip,
        Importance::Hint,
        Importance::Detail,
    ];

    /// The lowest level that says it; `None` when it is always said.
    pub fn min_level(self) -> Option<InterfaceLevel> {
        match self {
            Importance::Error | Importance::Question | Importance::Answer => None,
            Importance::Result => Some(InterfaceLevel::Minimal),
            Importance::Routine | Importance::Dialog | Importance::Progress | Importance::Tip => {
                Some(InterfaceLevel::Normal)
            }
            Importance::Hint | Importance::Detail => Some(InterfaceLevel::Full),
        }
    }
}

/// True when `level` lets an announcement of `importance` through.
pub fn lets_through(level: InterfaceLevel, importance: Importance) -> bool {
    importance.min_level().is_none_or(|min| level >= min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_questions_and_answers_are_never_silenced() {
        for level in InterfaceLevel::ALL {
            for i in [Importance::Error, Importance::Question, Importance::Answer] {
                assert!(lets_through(level, i), "{level} {i:?}");
            }
        }
    }

    #[test]
    fn each_level_adds_to_the_one_before() {
        use Importance as I;
        let said = |l| {
            Importance::ALL
                .into_iter()
                .filter(|i| lets_through(l, *i))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            said(InterfaceLevel::Off),
            [I::Error, I::Question, I::Answer]
        );
        assert_eq!(
            said(InterfaceLevel::Minimal),
            [I::Error, I::Question, I::Answer, I::Result]
        );
        assert_eq!(said(InterfaceLevel::Full), Importance::ALL);
        assert!(!lets_through(InterfaceLevel::Normal, I::Hint));
        assert!(lets_through(InterfaceLevel::Normal, I::Progress));
    }

    #[test]
    fn ids_and_cycle() {
        for l in InterfaceLevel::ALL {
            assert_eq!(l.id().parse::<InterfaceLevel>(), Ok(l));
        }
        assert_eq!(InterfaceLevel::Full.next(), InterfaceLevel::Off);
        assert_eq!(InterfaceLevel::Off.next(), InterfaceLevel::Minimal);
        assert_eq!(
            InterfaceLevel::for_mode(AccessMode::ScreenReader),
            InterfaceLevel::Minimal
        );
        assert_eq!(
            InterfaceLevel::for_mode(AccessMode::SelfVoicing),
            InterfaceLevel::Normal
        );
    }
}
