//! macOS keys with logical equivalents (Wave 6, W6u; ADR-0043).
//!
//! A Mac is not a PC with Ctrl renamed. On macOS:
//!
//! - **Option moves by word** (Option+Left and Option+Right) and **by
//!   paragraph** (Option+Up and Option+Down), and deletes a word with
//!   Option+Backspace.
//! - **Command moves to the ends**: Cmd+Left and Cmd+Right to the start and
//!   end of the line, Cmd+Up and Cmd+Down to the start and end of the
//!   document; Cmd+[ and Cmd+] go back and forward.
//! - **Command runs commands**: Cmd+O, Cmd+S, Cmd+Comma, Cmd+Q. Ctrl is
//!   used only where macOS itself uses it (Ctrl+A and Ctrl+E in text).
//! - **Option with a letter types a character** (Option+E is an accent,
//!   Option+M a micro sign), so a command never takes Option with a
//!   letter alone: the GUI's `Alt` chords with a letter or punctuation
//!   become Cmd+Option chords.
//! - **VoiceOver owns Ctrl+Option**, and macOS owns Cmd+Space (Spotlight),
//!   Cmd+H (hide), Cmd+M (minimize), Cmd+Tab, Cmd+` and more: no default
//!   key uses them ([`MAC_RESERVED_GUI`], [`MAC_RESERVED_TERMINAL`]).
//!
//! The defaults in [`ActionId`](crate::ActionId) are written for Windows
//! and Linux. On macOS the GUI takes each default through
//! [`translate_gui`] (Ctrl becomes Cmd; Option with a character gains
//! Cmd), except the commands listed in [`gui_keys`], whose Mac keys are
//! written out. The terminal keeps its keys (a terminal never sends Cmd),
//! except the few in [`terminal_keys`] that macOS or VoiceOver would take.
//! Text caret keys in an edit field, which the GUI's document view handles
//! itself, come from [`text_motion`], so a widget never swaps modifiers on
//! its own.

use crate::{ActionId, Key, KeyChord, Modifiers, Platform};

/// The Mac GUI's keys for commands whose Windows keys have no logical
/// translation, written out whole (browse and Speech Cursor keys
/// included). `None` for the rest, which [`translate_gui`] handles.
pub(crate) fn gui_keys(action: ActionId) -> Option<&'static [&'static str]> {
    use ActionId as A;
    Some(match action {
        // Cmd+Space is Spotlight.
        A::ReadFromCursor => &["g:Cmd+Enter", "b:Enter"],
        // Cmd+H hides the application.
        A::NextHeading => &["g:Cmd+Alt+PageDown", "b:>"],
        A::PreviousHeading => &["g:Cmd+Shift+H", "g:Cmd+Alt+PageUp", "b:<"],
        // Cmd+M minimizes; Cmd+D is Add Bookmark in Safari.
        A::AddBookmark => &["g:Cmd+D", "b:m"],
        // Cmd+T is Show Fonts in every Mac text application.
        A::ChooseFont => &["g:Cmd+T"],
        A::NextTable => &["g:Cmd+Alt+T", "b:t"],
        // Cmd+` switches windows.
        A::InlineCode => &["e:Cmd+Alt+K"],
        // Cmd+Up and Cmd+Down go to the ends of a document.
        A::DocumentStart => &["g:Cmd+Up", "b:Home"],
        A::DocumentEnd => &["g:Cmd+Down", "b:End"],
        // Option+Up and Option+Down move by paragraph; Ctrl+Up and
        // Ctrl+Down are Mission Control.
        A::NextParagraph => &["g:Alt+Down", "b:p", "b:]", "s:PageDown"],
        A::PreviousParagraph => &["g:Alt+Up", "b:Shift+P", "b:[", "s:PageUp"],
        A::NextSentence => &["g:Cmd+Alt+."],
        A::PreviousSentence => &["g:Cmd+Alt+,"],
        // Option+Left and Option+Right move by word; Cmd+[ and Cmd+] go
        // back and forward, as in Safari and Finder.
        A::HistoryBack => &["g:Cmd+[", "b:Backspace"],
        A::HistoryForward => &["g:Cmd+]", "b:\\"],
        A::CaretNextWord => &["b:Right", "b:Alt+Right"],
        A::CaretPreviousWord => &["b:Left", "b:Alt+Left"],
        A::SelectNextWord => &["b:Shift+Right", "b:Alt+Shift+Right"],
        A::SelectPreviousWord => &["b:Shift+Left", "b:Alt+Shift+Left"],
        // F11 shows the desktop.
        A::RateUp => &["g:Cmd+Alt+]", "b:+", "b:="],
        A::RateDown => &["g:Cmd+Alt+[", "b:-"],
        // Cmd+Option+= and Cmd+Option+- are Zoom.
        A::PitchUp => &["g:Cmd+Alt+Shift+Up", "b:)"],
        A::PitchDown => &["g:Cmd+Alt+Shift+Down", "b:("],
        // Cmd+; is Find Next Misspelling in every Mac text application;
        // Cmd+Option+M minimizes all windows.
        A::NextMisspelling => &["g:Cmd+;"],
        // Cmd+Option+Shift+Q logs out at once.
        A::ToggleCitations => &["g:Cmd+Alt+Shift+C"],
        // Cmd+Shift+Y is a system service (a new sticky note).
        A::ReadingStatistics => &["g:Cmd+Alt+Y"],
        // Cmd+Shift+Q logs out.
        A::BlockQuote => &["e:Cmd+Alt+Q"],
        // Option+Backspace deletes a word, as everywhere on a Mac.
        A::DeleteWordBefore => &["e:Alt+Backspace"],
        A::DeleteWordAfter => &["e:Alt+Delete"],
        _ => return None,
    })
}

/// The terminal's keys on macOS for the few commands whose keys macOS or
/// VoiceOver keep: Ctrl+Option is VoiceOver's, Ctrl+Up and Ctrl+Down are
/// Mission Control, Ctrl+Space switches the input source, and Ctrl with
/// F1 to F8 moves the keyboard focus to the menu bar, the Dock, and the
/// rest. `None` keeps the terminal's keys.
pub(crate) fn terminal_keys(action: ActionId) -> Option<&'static [&'static str]> {
    use ActionId as A;
    Some(match action {
        A::TableNextRow => &["g:Ctrl+Shift+Down"],
        A::TablePreviousRow => &["g:Ctrl+Shift+Up"],
        A::TableNextColumn => &["g:Ctrl+Shift+Right"],
        A::TablePreviousColumn => &["g:Ctrl+Shift+Left"],
        A::NextParagraph => &["g:Ctrl+P", "b:p", "b:]", "s:PageDown"],
        A::PreviousParagraph => &["b:Shift+P", "b:[", "s:PageUp"],
        A::ReadFromCursor => &["b:Enter"],
        A::NextGrammarProblem => &["g:Alt+F7"],
        A::PreviousGrammarProblem => &["g:Alt+Shift+F7"],
        A::NextLintProblem => &["g:Alt+F8"],
        A::PreviousLintProblem => &["g:Alt+Shift+F8"],
        _ => return None,
    })
}

/// A Windows default translated for the Mac GUI: Ctrl becomes Cmd, and
/// Option with a letter, digit, or punctuation (which types a character on
/// a Mac) gains Cmd. Named keys with Option (Option+Down, Option+End) stay.
pub fn translate_gui(chord: KeyChord) -> KeyChord {
    let mut chord = chord.ctrl_to_meta();
    if matches!(chord.key, Key::Char(_)) && chord.mods.contains(Modifiers::ALT) {
        chord.mods.insert(Modifiers::META);
    }
    chord
}

/// Chords the Mac GUI never binds by default: macOS's own shortcuts and
/// VoiceOver's. Any chord with both Ctrl and Option is VoiceOver's too.
pub const MAC_RESERVED_GUI: &[&str] = &[
    "Cmd+Space",
    "Cmd+Alt+Space",
    "Cmd+Tab",
    "Cmd+`",
    "Cmd+H",
    "Cmd+Alt+H",
    "Cmd+M",
    "Cmd+Alt+M",
    "Cmd+Alt+W",
    "Cmd+Alt+D",
    "Cmd+Alt+Escape",
    "Cmd+Alt+8",
    "Cmd+Alt+=",
    "Cmd+Alt+-",
    "Cmd+F5",
    "Cmd+Alt+F5",
    "Cmd+Shift+3",
    "Cmd+Shift+4",
    "Cmd+Shift+5",
    "Cmd+Shift+Q",
    "Cmd+Alt+Shift+Q",
    "Cmd+Shift+/",
    "Cmd+Shift+Y",
    "Ctrl+Up",
    "Ctrl+Down",
    "Ctrl+Left",
    "Ctrl+Right",
    "Ctrl+Space",
    "Ctrl+Cmd+Q",
    "Ctrl+Cmd+F",
    "Ctrl+Cmd+D",
    "Ctrl+Cmd+Space",
    "F11",
    "Ctrl+F1",
    "Ctrl+F2",
    "Ctrl+F3",
    "Ctrl+F4",
    "Ctrl+F5",
    "Ctrl+F6",
    "Ctrl+F7",
    "Ctrl+F8",
];

/// Chords the terminal never binds on macOS by default (see
/// [`terminal_keys`]).
pub const MAC_RESERVED_TERMINAL: &[&str] = &[
    "Ctrl+Up",
    "Ctrl+Down",
    "Ctrl+Left",
    "Ctrl+Right",
    "Ctrl+Space",
    "Ctrl+F1",
    "Ctrl+F2",
    "Ctrl+F3",
    "Ctrl+F4",
    "Ctrl+F5",
    "Ctrl+F6",
    "Ctrl+F7",
    "Ctrl+F8",
];

/// True for a chord VoiceOver takes: Ctrl and Option together (the
/// VoiceOver modifier), or Cmd+F5 (VoiceOver on and off).
pub fn is_voiceover_chord(chord: &KeyChord) -> bool {
    let vo = Modifiers::CTRL | Modifiers::ALT;
    chord.mods.contains(vo) || (chord.key == Key::F(5) && chord.mods.contains(Modifiers::META))
}

/// A caret move in an edit field, which a GUI text view handles itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextMotion {
    /// The start of the word before.
    WordLeft,
    /// The end of the word after.
    WordRight,
    /// The start of the line.
    LineStart,
    /// The end of the line.
    LineEnd,
    /// The start of the paragraph, or the one before.
    ParagraphUp,
    /// The end of the paragraph, or the next one.
    ParagraphDown,
    /// The start of the document.
    DocumentStart,
    /// The end of the document.
    DocumentEnd,
}

impl TextMotion {
    /// Every motion.
    pub const ALL: [TextMotion; 8] = [
        TextMotion::WordLeft,
        TextMotion::WordRight,
        TextMotion::LineStart,
        TextMotion::LineEnd,
        TextMotion::ParagraphUp,
        TextMotion::ParagraphDown,
        TextMotion::DocumentStart,
        TextMotion::DocumentEnd,
    ];

    /// The keys for this motion on `platform`, first the main one; Shift
    /// with any of them extends the selection.
    pub fn chords(self, platform: Platform) -> &'static [&'static str] {
        use TextMotion as T;
        match (platform, self) {
            (Platform::MacOs, T::WordLeft) => &["Alt+Left"],
            (Platform::MacOs, T::WordRight) => &["Alt+Right"],
            // Ctrl+A and Ctrl+E are macOS's own, from its text system.
            (Platform::MacOs, T::LineStart) => &["Cmd+Left", "Ctrl+A"],
            (Platform::MacOs, T::LineEnd) => &["Cmd+Right", "Ctrl+E"],
            (Platform::MacOs, T::ParagraphUp) => &["Alt+Up"],
            (Platform::MacOs, T::ParagraphDown) => &["Alt+Down"],
            (Platform::MacOs, T::DocumentStart) => &["Cmd+Up"],
            (Platform::MacOs, T::DocumentEnd) => &["Cmd+Down"],
            (_, T::WordLeft) => &["Ctrl+Left"],
            (_, T::WordRight) => &["Ctrl+Right"],
            (_, T::LineStart) => &["Home"],
            (_, T::LineEnd) => &["End"],
            (_, T::ParagraphUp) => &["Ctrl+Up"],
            (_, T::ParagraphDown) => &["Ctrl+Down"],
            (_, T::DocumentStart) => &["Ctrl+Home"],
            (_, T::DocumentEnd) => &["Ctrl+End"],
        }
    }
}

/// The caret move `chord` makes in an edit field on `platform`, and
/// whether it extends the selection (Shift); `None` for any other chord.
/// A GUI text view asks this instead of checking modifiers itself, so
/// Option+Right moves by word on a Mac and Ctrl+Right everywhere else.
pub fn text_motion(platform: Platform, chord: &KeyChord) -> Option<(TextMotion, bool)> {
    let extend = chord.mods.contains(Modifiers::SHIFT);
    let mut plain = *chord;
    plain.mods.remove(Modifiers::SHIFT);
    TextMotion::ALL.into_iter().find_map(|m| {
        m.chords(platform)
            .iter()
            .filter_map(|s| s.parse::<KeyChord>().ok())
            .any(|c| c == plain)
            .then_some((m, extend))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Frontend, Keymap, Layer};

    fn k(s: &str) -> KeyChord {
        s.parse().unwrap()
    }

    #[test]
    fn text_motions_follow_each_platform() {
        let mac = Platform::MacOs;
        assert_eq!(
            text_motion(mac, &k("Alt+Right")),
            Some((TextMotion::WordRight, false))
        );
        assert_eq!(
            text_motion(mac, &k("Alt+Shift+Left")),
            Some((TextMotion::WordLeft, true))
        );
        assert_eq!(
            text_motion(mac, &k("Cmd+Right")),
            Some((TextMotion::LineEnd, false))
        );
        assert_eq!(
            text_motion(mac, &k("Cmd+Up")),
            Some((TextMotion::DocumentStart, false))
        );
        assert_eq!(
            text_motion(mac, &k("Alt+Down")),
            Some((TextMotion::ParagraphDown, false))
        );
        assert_eq!(
            text_motion(mac, &k("Ctrl+E")),
            Some((TextMotion::LineEnd, false))
        );
        // A Mac's Cmd+Right is the end of the line, never a word move.
        assert_ne!(
            text_motion(mac, &k("Cmd+Right")).map(|m| m.0),
            Some(TextMotion::WordRight)
        );
        assert_eq!(text_motion(mac, &k("Ctrl+Right")), None);
        for p in [Platform::Windows, Platform::Linux] {
            assert_eq!(
                text_motion(p, &k("Ctrl+Right")),
                Some((TextMotion::WordRight, false))
            );
            assert_eq!(
                text_motion(p, &k("Ctrl+Shift+End")),
                Some((TextMotion::DocumentEnd, true))
            );
            assert_eq!(
                text_motion(p, &k("Home")),
                Some((TextMotion::LineStart, false))
            );
            assert_eq!(text_motion(p, &k("Alt+Right")), None);
        }
        for p in Platform::ALL {
            for m in TextMotion::ALL {
                for s in m.chords(p) {
                    assert!(s.parse::<KeyChord>().is_ok(), "{s}");
                }
            }
        }
    }

    /// Every command has a key on the Mac, in both frontends, and each Mac
    /// key follows the platform: Cmd for commands, Option only with named
    /// keys, Ctrl only in the terminal (which cannot send Cmd), and none of
    /// macOS's or VoiceOver's own chords.
    #[test]
    fn mac_keys_are_logical_and_clash_with_nothing() {
        let reserved_gui: Vec<KeyChord> = MAC_RESERVED_GUI.iter().map(|s| k(s)).collect();
        let reserved_term: Vec<KeyChord> = MAC_RESERVED_TERMINAL.iter().map(|s| k(s)).collect();
        let gui = Keymap::defaults(Platform::MacOs, Frontend::Gui);
        let term = Keymap::defaults(Platform::MacOs, Frontend::Terminal);
        for b in gui.bindings() {
            let c = b.chord;
            assert!(
                !is_voiceover_chord(&c),
                "{:?}: {c} is VoiceOver's",
                b.action
            );
            assert!(!reserved_gui.contains(&c), "{:?}: {c} is macOS's", b.action);
            assert!(
                !c.mods.contains(Modifiers::CTRL),
                "{:?}: {c} uses Ctrl on a Mac",
                b.action
            );
            if matches!(c.key, Key::Char(_)) && c.mods.contains(Modifiers::ALT) {
                assert!(
                    c.mods.contains(Modifiers::META),
                    "{:?}: Option with {c} would type a character",
                    b.action
                );
            }
            if c == k("Cmd+Q") {
                assert_eq!(b.action, ActionId::Quit);
            }
        }
        for b in term.bindings() {
            let c = b.chord;
            assert!(
                !is_voiceover_chord(&c),
                "{:?}: {c} is VoiceOver's",
                b.action
            );
            assert!(
                !reserved_term.contains(&c),
                "{:?}: {c} is macOS's",
                b.action
            );
        }
        // The commands macOS names.
        for (chord, action) in [
            ("Cmd+O", ActionId::Open),
            ("Cmd+S", ActionId::Save),
            ("Cmd+,", ActionId::Settings),
            ("Cmd+Q", ActionId::Quit),
            ("Cmd+Up", ActionId::DocumentStart),
            ("Cmd+Down", ActionId::DocumentEnd),
            ("Alt+Down", ActionId::NextParagraph),
            ("Alt+Up", ActionId::PreviousParagraph),
            ("Cmd+[", ActionId::HistoryBack),
            ("Cmd+]", ActionId::HistoryForward),
            ("Cmd+T", ActionId::ChooseFont),
            ("Cmd+;", ActionId::NextMisspelling),
            ("Cmd+Shift+V", ActionId::ChooseVoice),
        ] {
            assert_eq!(
                gui.lookup(&k(chord), Layer::Browse),
                Some(action),
                "{chord}"
            );
        }
        assert_eq!(
            gui.lookup(&k("Alt+Right"), Layer::Browse),
            Some(ActionId::CaretNextWord)
        );
        assert_eq!(
            gui.lookup(&k("Alt+Backspace"), Layer::Edit),
            Some(ActionId::DeleteWordBefore)
        );
        assert_eq!(
            gui.lookup(&k("Cmd+Shift+Z"), Layer::Edit),
            Some(ActionId::Redo)
        );
        assert_eq!(gui.lookup(&k("Cmd+Right"), Layer::Browse), None);
    }

    #[test]
    fn translation_keeps_named_option_keys() {
        assert_eq!(translate_gui(k("Ctrl+O")), k("Cmd+O"));
        assert_eq!(translate_gui(k("Alt+N")), k("Cmd+Alt+N"));
        assert_eq!(translate_gui(k("Alt+Shift+B")), k("Cmd+Alt+Shift+B"));
        assert_eq!(translate_gui(k("Alt+End")), k("Alt+End"));
        assert_eq!(translate_gui(k("Ctrl+Alt+Down")), k("Cmd+Alt+Down"));
        assert!(is_voiceover_chord(&k("Ctrl+Alt+Right")));
        assert!(!is_voiceover_chord(&k("Cmd+Alt+Right")));
    }
}
