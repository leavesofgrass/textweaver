//! Screen reader coexistence: the accessibility mode and where each kind of
//! output goes (`[accessibility]` in `settings.toml`).
//!
//! textweaver can speak with its own voice, write to its status line, or
//! both. A screen reader that reads new text in a terminal (NVDA's dynamic
//! content reporting, JAWS's screen echo, Orca, VoiceOver) also speaks the
//! status line, so anything that goes to both is heard twice. The
//! [`AccessMode`] decides, per [`Channel`], which of the two each output
//! uses; [`route`] is the whole rule, and the app asks it before speaking or
//! writing anything.
//!
//! | Channel | self-voicing | screen-reader | hybrid |
//! |---|---|---|---|
//! | messages | voice and status line | status line | status line |
//! | typing echo | voice | nothing (the screen reader echoes) | nothing |
//! | caret moves | voice and status line | status line | status line |
//! | reading text | voice (status line without a voice) | status line | voice (status line without a voice) |
//! | Speech Cursor lines | voice and status line | status line | voice (status line without a voice) |
//!
//! "Quiet screen" ([`RouteContext::quiet_screen`]) takes the status-line
//! copy away from text textweaver is reading aloud, in every mode.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// How textweaver shares the work with a screen reader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccessMode {
    /// textweaver speaks everything with its own voice and also shows each
    /// message on the status line (the behaviour before this setting).
    #[default]
    SelfVoicing,
    /// textweaver never speaks on its own: every message goes to the status
    /// line for the screen reader. Continuous reading moves through the text
    /// a sentence at a time on the status line, or uses textweaver's voice
    /// when `[accessibility] say_all = "voice"`.
    ScreenReader,
    /// textweaver reads documents aloud (continuous reading, reading a unit,
    /// and the math, table, citation, and structure narration inside them);
    /// the screen reader speaks messages, typing echo, and caret moves from
    /// the status line and the cursor. Recommended with a screen reader.
    Hybrid,
}

impl AccessMode {
    /// Every mode, in the order [`next`](Self::next) cycles through them.
    pub const ALL: [AccessMode; 3] = [
        AccessMode::SelfVoicing,
        AccessMode::Hybrid,
        AccessMode::ScreenReader,
    ];

    /// The id used in `settings.toml` and `--mode`: `self-voicing`,
    /// `screen-reader`, or `hybrid`.
    pub fn id(self) -> &'static str {
        match self {
            AccessMode::SelfVoicing => "self-voicing",
            AccessMode::ScreenReader => "screen-reader",
            AccessMode::Hybrid => "hybrid",
        }
    }

    /// The name said and shown when the mode changes.
    pub fn name(self) -> &'static str {
        match self {
            AccessMode::SelfVoicing => "Self-voicing",
            AccessMode::ScreenReader => "Screen reader",
            AccessMode::Hybrid => "Hybrid",
        }
    }

    /// One sentence for the mode-change announcement.
    pub fn description(self) -> &'static str {
        match self {
            AccessMode::SelfVoicing => "textweaver speaks everything.",
            AccessMode::ScreenReader => {
                "textweaver is silent; your screen reader reads the status line."
            }
            AccessMode::Hybrid => {
                "textweaver reads documents aloud; your screen reader speaks messages and typing."
            }
        }
    }

    /// The mode after this one: self-voicing, hybrid, screen reader, and
    /// round again.
    pub fn next(self) -> AccessMode {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// The mode for an id, ignoring case, with `_` or a space accepted for
    /// `-` (`screen_reader`, `Screen Reader`).
    pub fn from_id(id: &str) -> Option<AccessMode> {
        let id = id.trim().to_ascii_lowercase().replace(['_', ' '], "-");
        Self::ALL.into_iter().find(|m| m.id() == id)
    }

    /// True when a screen reader is expected to read the status line.
    pub fn uses_screen_reader(self) -> bool {
        self != AccessMode::SelfVoicing
    }
}

impl fmt::Display for AccessMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// An unknown accessibility mode name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownMode(pub String);

impl fmt::Display for UnknownMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown mode {:?}; use self-voicing, screen-reader, or hybrid",
            self.0
        )
    }
}

impl std::error::Error for UnknownMode {}

impl FromStr for AccessMode {
    type Err = UnknownMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AccessMode::from_id(s).ok_or_else(|| UnknownMode(s.to_owned()))
    }
}

/// Where the terminal's cursor waits when no prompt is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorPlacement {
    /// On the spoken word while reading, else on the caret, the Speech
    /// Cursor line, or the chosen list item: magnifiers and braille follow
    /// the reading.
    #[default]
    Follow,
    /// At the start of the status line, as Star did, so a screen reader's
    /// "read current line" repeats the last message.
    Status,
}

/// How continuous reading works in screen-reader mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SayAll {
    /// The cursor moves a sentence at a time and each sentence goes to the
    /// status line for the screen reader, paced by textweaver's rate.
    #[default]
    Screen,
    /// textweaver reads aloud with its own voice (the one thing it says in
    /// screen-reader mode, because you asked for it).
    Voice,
}

/// A kind of output, for [`route`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Messages: results of commands, state changes, errors, questions,
    /// list items as they gain focus.
    Message,
    /// Typing echo: characters, words, and deletions while typing in the
    /// document or a prompt, and the line reached while editing.
    Echo,
    /// What a caret move reaches in reading mode (the word after Right,
    /// the line after Down): screen readers call this caret echo.
    Caret,
    /// Document text read aloud: continuous reading and reading a unit
    /// (character, word, sentence, paragraph, selection).
    Reading,
    /// A line read in Speech Cursor mode or by "read current line".
    Line,
}

/// Where one output goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Route {
    /// Speak it with textweaver's voice.
    pub speak: bool,
    /// Write it on the status line.
    pub status: bool,
}

/// What [`route`] needs to know besides the mode and the channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RouteContext {
    /// textweaver has a working voice and speaks messages (false with
    /// `--no-speech`, a failed engine, or the GUI, where messages go to the
    /// screen reader as notifications).
    pub voice: bool,
    /// `[accessibility] quiet_screen` is on and continuous reading is going
    /// on: text being read aloud is not copied to the status line.
    pub quiet_screen: bool,
}

/// Where output of `channel` goes in `mode`.
///
/// Caret moves in self-voicing mode are spoken even without
/// [`RouteContext::voice`]: the GUI keeps its voice for document text while
/// its messages go to the screen reader, and a caret move says document
/// text.
pub fn route(mode: AccessMode, channel: Channel, ctx: RouteContext) -> Route {
    let RouteContext {
        voice,
        quiet_screen,
    } = ctx;
    let (speak, status) = match (mode, channel) {
        (AccessMode::SelfVoicing, Channel::Message) => (voice, true),
        (AccessMode::SelfVoicing, Channel::Echo) => (voice, false),
        (AccessMode::SelfVoicing, Channel::Caret) => (true, true),
        (AccessMode::SelfVoicing, Channel::Reading) => (true, !voice),
        (AccessMode::SelfVoicing, Channel::Line) => (true, true),
        (AccessMode::ScreenReader, Channel::Echo) => (false, false),
        (AccessMode::ScreenReader, _) => (false, true),
        (AccessMode::Hybrid, Channel::Message | Channel::Caret) => (false, true),
        (AccessMode::Hybrid, Channel::Echo) => (false, false),
        (AccessMode::Hybrid, Channel::Reading | Channel::Line) => (voice, !voice),
    };
    let reading_aloud = speak && matches!(channel, Channel::Reading | Channel::Line);
    Route {
        speak,
        status: status && !(quiet_screen && reading_aloud),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VOICED: RouteContext = RouteContext {
        voice: true,
        quiet_screen: false,
    };

    fn r(speak: bool, status: bool) -> Route {
        Route { speak, status }
    }

    #[test]
    fn ids_names_and_cycling() {
        for m in AccessMode::ALL {
            assert_eq!(AccessMode::from_id(m.id()), Some(m));
            assert_eq!(m.id().parse::<AccessMode>(), Ok(m));
            assert!(m.description().ends_with('.'));
        }
        assert_eq!(
            AccessMode::from_id("Screen_Reader"),
            Some(AccessMode::ScreenReader)
        );
        assert!("loud".parse::<AccessMode>().is_err());
        assert_eq!(AccessMode::SelfVoicing.next(), AccessMode::Hybrid);
        assert_eq!(AccessMode::Hybrid.next(), AccessMode::ScreenReader);
        assert_eq!(AccessMode::ScreenReader.next(), AccessMode::SelfVoicing);
        assert!(!AccessMode::SelfVoicing.uses_screen_reader());
        assert!(AccessMode::Hybrid.uses_screen_reader());
    }

    #[test]
    fn serde_uses_the_documented_names() {
        let s = serde_json::to_string(&AccessMode::ScreenReader).unwrap();
        assert_eq!(s, "\"screen-reader\"");
        let m: AccessMode = serde_json::from_str("\"self-voicing\"").unwrap();
        assert_eq!(m, AccessMode::SelfVoicing);
        assert_eq!(
            serde_json::to_string(&CursorPlacement::Status).unwrap(),
            "\"status\""
        );
        assert_eq!(serde_json::to_string(&SayAll::Voice).unwrap(), "\"voice\"");
    }

    /// Self-voicing is the behaviour from before the setting existed.
    #[test]
    fn self_voicing_speaks_and_shows_messages() {
        let m = AccessMode::SelfVoicing;
        assert_eq!(route(m, Channel::Message, VOICED), r(true, true));
        assert_eq!(route(m, Channel::Echo, VOICED), r(true, false));
        assert_eq!(route(m, Channel::Caret, VOICED), r(true, true));
        assert_eq!(route(m, Channel::Reading, VOICED), r(true, false));
        assert_eq!(route(m, Channel::Line, VOICED), r(true, true));
        // Without a voice (--no-speech before this setting, or the GUI)
        // messages are only shown and read text goes to the status line.
        let silent = RouteContext::default();
        assert_eq!(route(m, Channel::Message, silent), r(false, true));
        assert_eq!(route(m, Channel::Echo, silent), r(false, false));
        assert_eq!(route(m, Channel::Reading, silent), r(true, true));
    }

    /// Screen-reader mode never speaks, and nothing is both spoken and
    /// shown in any mode but self-voicing.
    #[test]
    fn screen_reader_mode_is_silent_and_nothing_is_doubled() {
        let channels = [
            Channel::Message,
            Channel::Echo,
            Channel::Caret,
            Channel::Reading,
            Channel::Line,
        ];
        for ctx in [VOICED, RouteContext::default()] {
            for c in channels {
                let sr = route(AccessMode::ScreenReader, c, ctx);
                assert!(!sr.speak, "{c:?}");
                assert_eq!(sr.status, c != Channel::Echo, "{c:?}");
                let h = route(AccessMode::Hybrid, c, ctx);
                assert!(!(h.speak && h.status), "hybrid doubles {c:?}");
            }
        }
    }

    #[test]
    fn hybrid_voices_reading_and_leaves_the_rest_to_the_screen_reader() {
        let m = AccessMode::Hybrid;
        assert_eq!(route(m, Channel::Message, VOICED), r(false, true));
        assert_eq!(route(m, Channel::Echo, VOICED), r(false, false));
        assert_eq!(route(m, Channel::Caret, VOICED), r(false, true));
        assert_eq!(route(m, Channel::Reading, VOICED), r(true, false));
        assert_eq!(route(m, Channel::Line, VOICED), r(true, false));
        // With no voice the screen reader gets the text instead.
        let silent = RouteContext::default();
        assert_eq!(route(m, Channel::Reading, silent), r(false, true));
    }

    #[test]
    fn quiet_screen_drops_the_copy_of_text_read_aloud() {
        let quiet = RouteContext {
            voice: true,
            quiet_screen: true,
        };
        let m = AccessMode::SelfVoicing;
        assert_eq!(route(m, Channel::Line, quiet), r(true, false));
        assert_eq!(route(m, Channel::Reading, quiet), r(true, false));
        // Messages are not read text: they stay.
        assert_eq!(route(m, Channel::Message, quiet), r(true, true));
        // Text nobody speaks is still shown.
        assert_eq!(
            route(AccessMode::ScreenReader, Channel::Reading, quiet),
            r(false, true)
        );
    }
}
