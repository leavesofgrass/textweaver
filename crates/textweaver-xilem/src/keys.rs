//! Keyboard input: Masonry's key events as the keymap's [`KeyChord`]s, and
//! the caret keys of each platform.
//!
//! The document view handles the caret keys itself, so the screen reader
//! reads what the caret moves over. Which chord moves how far is the
//! platform's convention, from one table ([`caret_keys`]): Ctrl with the
//! arrows moves by word and paragraph on Windows and Linux; on macOS Option
//! does, Command with Left and Right goes to the line's ends, and Command
//! with Up and Down to the document's. Shift with any of them selects. Tab
//! moves focus. Every other key reaches the root widget and becomes a
//! chord for `Keymap::lookup`, whose GUI keymap is the platform's too.
//!
//! Widget code never asks which system it was built for: it takes a
//! [`Platform`] (normally [`Platform::current`]) and reads the tables here,
//! and the tests press keys built from the same tables ([`press`]), so a
//! test cannot press a key the platform does not use.
//!
//! A printable key arrives as the character the layout typed (`>` rather
//! than Shift with `.`), which is what the keymap's Browse layer stores;
//! with Ctrl, Alt, or Command a letter is reduced to its lowercase form,
//! with Shift kept as a modifier.

use masonry::core::keyboard::{Key, KeyboardEvent, Modifiers as KeyMods, NamedKey};
use textweaver_app::keymap::{Key as TwKey, KeyChord, Modifiers, Platform};

/// How far a caret key moves the caret in the document view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaretStep {
    /// One character.
    Char,
    /// One word.
    Word,
    /// One visual line, keeping the column.
    Line,
    /// One paragraph.
    Paragraph,
    /// To the start or end of the visual line.
    LineEdge,
    /// One screen of lines.
    Page,
    /// To the start or end of the document.
    DocumentEdge,
}

/// A caret key's move: how far, and which way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaretMove {
    /// How far.
    pub step: CaretStep,
    /// Toward the end of the document.
    pub forward: bool,
}

const fn mv(step: CaretStep, forward: bool) -> CaretMove {
    CaretMove { step, forward }
}

/// The caret keys of `platform`, without Shift (Shift with any of them
/// extends the selection): each chord and the move it makes.
///
/// - **Windows and Linux:** the arrows by character and line, Ctrl with
///   them by word and paragraph, Home and End to the line's ends, Ctrl with
///   them to the document's, Page Up and Page Down by screen.
/// - **macOS:** the arrows by character and line, Option (Alt) with them by
///   word and paragraph, Command with Left and Right to the line's ends and
///   with Up and Down to the document's, Home and End to the document's
///   ends (as Cocoa's text views scroll), Page Up and Page Down by screen.
///   No Control chord: Control with Option is VoiceOver's.
pub fn caret_keys(platform: Platform) -> Vec<(KeyChord, CaretMove)> {
    use CaretStep::*;
    use textweaver_app::keymap::TextMotion as T;
    let k = |key, mods| KeyChord::new(key, mods);
    let none = Modifiers::empty();
    let mut out = vec![
        (k(TwKey::Left, none), mv(Char, false)),
        (k(TwKey::Right, none), mv(Char, true)),
        (k(TwKey::Up, none), mv(Line, false)),
        (k(TwKey::Down, none), mv(Line, true)),
        (k(TwKey::PageUp, none), mv(Page, false)),
        (k(TwKey::PageDown, none), mv(Page, true)),
    ];
    // Words, paragraphs, and the ends: the keymap's table for the
    // platform (`textweaver_keymap::text_motion`), so the keymap and the
    // document view agree on every system.
    for motion in T::ALL {
        let m = match motion {
            T::WordLeft => mv(Word, false),
            T::WordRight => mv(Word, true),
            T::LineStart => mv(LineEdge, false),
            T::LineEnd => mv(LineEdge, true),
            T::ParagraphUp => mv(Paragraph, false),
            T::ParagraphDown => mv(Paragraph, true),
            T::DocumentStart => mv(DocumentEdge, false),
            T::DocumentEnd => mv(DocumentEdge, true),
        };
        for s in motion.chords(platform) {
            if let Ok(chord) = s.parse::<KeyChord>() {
                out.push((chord, m));
            }
        }
    }
    out
}

/// The caret move `chord` makes on `platform`, and whether it extends the
/// selection (Shift). `None` when the chord is not a caret key there.
pub fn caret_move(chord: &KeyChord, platform: Platform) -> Option<(CaretMove, bool)> {
    let extend = chord.mods.contains(Modifiers::SHIFT);
    let base = KeyChord::new(chord.key, chord.mods - Modifiers::SHIFT);
    caret_keys(platform)
        .into_iter()
        .find(|(c, _)| *c == base)
        .map(|(_, m)| (m, extend))
}

/// True when `mods` make a key a command rather than typing on `platform`:
/// Ctrl or Alt anywhere, and Command on macOS (the Windows key is the
/// system's elsewhere).
pub fn is_command(mods: KeyMods, platform: Platform) -> bool {
    mods.ctrl() || mods.alt() || (mods.meta() && platform == Platform::MacOs)
}

/// The key event a person presses for `chord` on `platform`: the inverse
/// of [`chord()`], for tests and tools that must press only keys the
/// platform uses. A shifted letter is typed uppercase with Shift held, as a
/// keyboard sends it.
pub fn press(chord: &KeyChord, platform: Platform) -> KeyboardEvent {
    let mut mods = KeyMods::empty();
    mods.set(KeyMods::CONTROL, chord.mods.contains(Modifiers::CTRL));
    mods.set(KeyMods::ALT, chord.mods.contains(Modifiers::ALT));
    mods.set(KeyMods::SHIFT, chord.mods.contains(Modifiers::SHIFT));
    mods.set(
        KeyMods::META,
        chord.mods.contains(Modifiers::META) && platform == Platform::MacOs,
    );
    let key = match chord.key {
        TwKey::Char(c) => {
            if c.is_ascii_uppercase() {
                mods.insert(KeyMods::SHIFT);
            }
            Key::Character(c.to_string())
        }
        TwKey::Space => Key::Character(" ".into()),
        TwKey::F(n) => Key::Named(function_key(n).unwrap_or(NamedKey::F1)),
        TwKey::Enter => Key::Named(NamedKey::Enter),
        TwKey::Escape => Key::Named(NamedKey::Escape),
        TwKey::Tab => Key::Named(NamedKey::Tab),
        TwKey::Backspace => Key::Named(NamedKey::Backspace),
        TwKey::Delete => Key::Named(NamedKey::Delete),
        TwKey::Insert => Key::Named(NamedKey::Insert),
        TwKey::Home => Key::Named(NamedKey::Home),
        TwKey::End => Key::Named(NamedKey::End),
        TwKey::PageUp => Key::Named(NamedKey::PageUp),
        TwKey::PageDown => Key::Named(NamedKey::PageDown),
        TwKey::Up => Key::Named(NamedKey::ArrowUp),
        TwKey::Down => Key::Named(NamedKey::ArrowDown),
        TwKey::Left => Key::Named(NamedKey::ArrowLeft),
        TwKey::Right => Key::Named(NamedKey::ArrowRight),
    };
    KeyboardEvent {
        key,
        modifiers: mods,
        ..Default::default()
    }
}

fn function_key(n: u8) -> Option<NamedKey> {
    use NamedKey::*;
    const KEYS: [NamedKey; 24] = [
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
        F21, F22, F23, F24,
    ];
    KEYS.get(usize::from(n).checked_sub(1)?).cloned()
}

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
                    // Ctrl+Plus, from the number pad or with Shift on the
                    // equals key, is the keymap's Ctrl+= (text larger).
                    TwKey::Char(if c == '+' { '=' } else { c })
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

/// The caret keys the document view leaves to the keymap in the mode whose
/// layer is `layer`: in Speech Cursor mode, the keys that mode binds itself
/// (Up and Down read the next and previous line, Page Up and Page Down move
/// by paragraph), with or without Shift. While a formula is explored
/// (`math`), the plain arrows, Home, and End, which move through it
/// ([`math_move`]). In browse and edit mode the view keeps every caret key,
/// as a document window does.
pub fn yielded_caret_keys(
    keymap: &textweaver_app::keymap::Keymap,
    layer: textweaver_app::keymap::Layer,
    math: bool,
    platform: Platform,
) -> Vec<KeyChord> {
    use textweaver_app::keymap::Layer;
    if math {
        return [
            TwKey::Left,
            TwKey::Right,
            TwKey::Up,
            TwKey::Down,
            TwKey::Home,
            TwKey::End,
        ]
        .into_iter()
        .map(KeyChord::plain)
        .collect();
    }
    if layer != Layer::SpeechCursor {
        return Vec::new();
    }
    caret_keys(platform)
        .into_iter()
        .flat_map(|(c, _)| [c, KeyChord::new(c.key, c.mods | Modifiers::SHIFT)])
        .filter(|c| {
            keymap
                .bindings()
                .iter()
                .any(|b| b.layer == Layer::SpeechCursor && b.chord == *c && keymap.is_active(b))
        })
        .collect()
}

/// The move a key makes while a formula is explored (Explore Math), as in
/// the terminal: Right and Left to the next and previous part, Down into a
/// part and Up out of it, Home and End to the first and last, Space or
/// Enter to hear it again, Escape to leave. Only plain keys: any other key
/// leaves the formula and does what it usually does.
pub fn math_move(chord: &KeyChord) -> Option<textweaver_app::MathMove> {
    use textweaver_app::MathMove as M;
    if !chord.mods.is_empty() {
        return None;
    }
    Some(match chord.key {
        TwKey::Right => M::Next,
        TwKey::Left => M::Previous,
        TwKey::Down => M::Enter,
        TwKey::Up => M::Exit,
        TwKey::Home => M::First,
        TwKey::End => M::Last,
        TwKey::Space | TwKey::Enter => M::Repeat,
        TwKey::Escape => M::Leave,
        _ => return None,
    })
}

/// True for chords the document view keeps on `platform`: its caret keys,
/// with or without Shift ([`caret_keys`]), and Tab.
pub fn is_native(chord: &KeyChord, platform: Platform) -> bool {
    caret_move(chord, platform).is_some()
        || (chord.key == TwKey::Tab
            && (chord.mods - (Modifiers::SHIFT | Modifiers::CTRL)).is_empty())
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
    fn control_plus_is_control_equals() {
        let want = Some("Ctrl+=".parse().unwrap());
        // The number pad's plus, and Shift with the equals key.
        assert_eq!(chord(&ev(c("+"), Mods::CONTROL), WIN), want);
        assert_eq!(chord(&ev(c("+"), Mods::CONTROL | Mods::SHIFT), WIN), want);
        assert_eq!(chord(&ev(c("="), Mods::CONTROL), WIN), want);
        // Without a command modifier, plus stays plus (faster, in browse).
        assert_eq!(
            chord(&ev(c("+"), Mods::SHIFT), WIN),
            Some("+".parse().unwrap())
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
        assert!(is_native(&"Down".parse().unwrap(), WIN));
        assert!(is_native(&"Ctrl+Shift+End".parse().unwrap(), WIN));
        assert!(is_native(&"Tab".parse().unwrap(), WIN));
        assert!(!is_native(&"Alt+Down".parse().unwrap(), WIN));
        assert!(!is_native(&"Space".parse().unwrap(), WIN));
        // macOS: Option and Command, never Ctrl.
        let mac = Platform::MacOs;
        assert!(is_native(&"Alt+Shift+Right".parse().unwrap(), mac));
        assert!(is_native(&"Cmd+Down".parse().unwrap(), mac));
        assert!(!is_native(&"Ctrl+Right".parse().unwrap(), mac));
        assert!(!is_native(&"Ctrl+Home".parse().unwrap(), mac));
    }

    fn step(chord: &str, platform: Platform) -> Option<(CaretStep, bool, bool)> {
        caret_move(&chord.parse().unwrap(), platform).map(|(m, x)| (m.step, m.forward, x))
    }

    /// The logical keys of each platform (the owner's rule, Tuesday,
    /// September 29, 2026): the same moves, each on the platform's own key.
    #[test]
    fn each_platform_moves_by_its_own_keys() {
        use CaretStep::*;
        for p in [Platform::Windows, Platform::Linux] {
            assert_eq!(step("Ctrl+Right", p), Some((Word, true, false)));
            assert_eq!(step("Ctrl+Shift+Left", p), Some((Word, false, true)));
            assert_eq!(step("Ctrl+Down", p), Some((Paragraph, true, false)));
            assert_eq!(step("End", p), Some((LineEdge, true, false)));
            assert_eq!(step("Ctrl+Home", p), Some((DocumentEdge, false, false)));
            assert_eq!(step("Alt+Right", p), None);
        }
        let mac = Platform::MacOs;
        assert_eq!(step("Alt+Right", mac), Some((Word, true, false)));
        assert_eq!(step("Alt+Shift+Left", mac), Some((Word, false, true)));
        assert_eq!(step("Alt+Up", mac), Some((Paragraph, false, false)));
        assert_eq!(step("Cmd+Right", mac), Some((LineEdge, true, false)));
        assert_eq!(step("Cmd+Shift+Left", mac), Some((LineEdge, false, true)));
        assert_eq!(step("Cmd+Down", mac), Some((DocumentEdge, true, false)));
        assert_eq!(step("Home", mac), Some((DocumentEdge, false, false)));
        assert_eq!(step("Ctrl+Right", mac), None);
        // Every platform: the plain arrows and the page keys.
        for p in Platform::ALL {
            assert_eq!(step("Right", p), Some((Char, true, false)));
            assert_eq!(step("Shift+Down", p), Some((Line, true, true)));
            assert_eq!(step("PageUp", p), Some((Page, false, false)));
        }
    }

    /// Each platform's table has every move in both directions, once, and
    /// no chord twice; and on macOS no chord uses Control, whose chords with
    /// Option are VoiceOver's.
    #[test]
    fn caret_tables_are_complete_and_unambiguous() {
        use CaretStep::*;
        for p in Platform::ALL {
            let table = caret_keys(p);
            let chords: std::collections::HashSet<_> = table.iter().map(|(c, _)| *c).collect();
            assert_eq!(chords.len(), table.len(), "{p:?}: a chord twice");
            for s in [Char, Word, Line, Paragraph, LineEdge, Page, DocumentEdge] {
                for forward in [false, true] {
                    assert!(
                        table.iter().any(|(_, m)| *m == mv(s, forward)),
                        "{p:?}: no key for {s:?} {forward}"
                    );
                }
            }
            if p == Platform::MacOs {
                assert!(table.iter().all(|(c, _)| !c.mods.contains(Modifiers::CTRL)));
            }
            for (c, _) in &table {
                assert!(!c.mods.contains(Modifiers::SHIFT), "{c}");
            }
        }
    }

    /// `press` is the inverse of `chord`: every caret key and every GUI
    /// keymap chord of each platform comes back as itself, so tests built
    /// from the tables press exactly the platform's keys.
    #[test]
    fn a_pressed_chord_reads_back_as_itself() {
        use textweaver_app::keymap::{Frontend, Keymap};
        for p in Platform::ALL {
            let mut all: Vec<KeyChord> = caret_keys(p).into_iter().map(|(c, _)| c).collect();
            all.extend(
                Keymap::defaults(p, Frontend::Gui)
                    .bindings()
                    .iter()
                    .map(|b| b.chord)
                    // The Windows key is the system's off macOS: never sent.
                    .filter(|c| p == Platform::MacOs || !c.mods.contains(Modifiers::META)),
            );
            for c in all {
                assert_eq!(chord(&press(&c, p), p), Some(c), "{p:?}: {c}");
            }
        }
    }

    /// The view's caret keys never take a key the platform's GUI keymap
    /// gives another meaning, in the layers the document is in (browse,
    /// speech cursor, edit). A keymap binding with the same meaning (Ctrl+Home
    /// for the document's start) is fine, and so are the keymap's own caret
    /// and selection commands, which the view does itself.
    ///
    /// On macOS the keymap still gives Option with the arrows to history and
    /// sentences; W6u's macOS column of the keymap moves them. Until then
    /// they are listed here, and the test fails on any other clash.
    #[test]
    fn caret_keys_never_hide_a_keymap_command() {
        use textweaver_app::keymap::{ActionId, Frontend, Keymap, Layer};
        let same_meaning = |a: ActionId, m: CaretMove| {
            let own = a.id().starts_with("caret_") || a.id().starts_with("select_");
            own || match m.step {
                CaretStep::DocumentEdge => {
                    a == if m.forward {
                        ActionId::DocumentEnd
                    } else {
                        ActionId::DocumentStart
                    }
                }
                CaretStep::Paragraph => {
                    a == if m.forward {
                        ActionId::NextParagraph
                    } else {
                        ActionId::PreviousParagraph
                    }
                }
                CaretStep::Page => {
                    a == if m.forward {
                        ActionId::PageDown
                    } else {
                        ActionId::PageUp
                    }
                }
                _ => false,
            }
        };
        // In browse mode (and Speech Cursor mode, which falls back to its
        // keys) Home and End go to the line's ends in the window,
        // as in any document window (and NVDA's and JAWS's browse mode);
        // the terminal's keymap sends them to the document's ends, which
        // Ctrl+Home and Ctrl+End (Command+Up and Down on macOS) do here.
        let window_convention = |a: ActionId, m: CaretMove, layer: Layer| {
            matches!(layer, Layer::Browse | Layer::SpeechCursor)
                && m.step == CaretStep::LineEdge
                && matches!(a, ActionId::DocumentStart | ActionId::DocumentEnd)
        };
        let pending_on_mac = [
            ActionId::RsvpFaster,
            ActionId::RsvpSlower,
            ActionId::HistoryBack,
            ActionId::HistoryForward,
            ActionId::NextSentence,
            ActionId::PreviousSentence,
            ActionId::NextParagraph,
            ActionId::PreviousParagraph,
            ActionId::DocumentStart,
            ActionId::DocumentEnd,
        ];
        let mut clashes = Vec::new();
        for p in Platform::ALL {
            let map = Keymap::defaults(p, Frontend::Gui);
            for (base, m) in caret_keys(p) {
                for chord in [base, KeyChord::new(base.key, base.mods | Modifiers::SHIFT)] {
                    for layer in [Layer::Browse, Layer::SpeechCursor, Layer::Edit] {
                        let Some(a) = map.lookup(&chord, layer) else {
                            continue;
                        };
                        if same_meaning(a, m)
                            || window_convention(a, m, layer)
                            || yielded_caret_keys(&map, layer, false, p).contains(&chord)
                            || (p == Platform::MacOs && pending_on_mac.contains(&a))
                        {
                            continue;
                        }
                        clashes.push(format!("{p:?} {layer:?}: {chord} ({m:?}) is {a:?}"));
                    }
                }
            }
        }
        assert!(clashes.is_empty(), "{}", clashes.join("\n"));
    }

    /// A command modifier is the platform's: Command on macOS only.
    #[test]
    fn command_modifiers_follow_the_platform() {
        assert!(is_command(Mods::META, Platform::MacOs));
        assert!(!is_command(Mods::META, WIN));
        assert!(is_command(Mods::CONTROL, Platform::MacOs));
        assert!(is_command(Mods::ALT, WIN));
        assert!(!is_command(Mods::SHIFT, WIN));
    }
}
