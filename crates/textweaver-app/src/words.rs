//! The interface's words for the app's own types (Wave 4, W4d): the names
//! of units, structure, and modes, from the message catalog, and the
//! system's language.
//!
//! Messages that name a unit or a kind of structure take two values: the
//! noun itself (`$what`, "heading", from `kind-*` or `unit-*`) and a key
//! that never changes with the language (`$unit`, "heading", "list-item").
//! English uses the noun; a language whose adjectives agree with the noun
//! chooses by the key (`{ $unit -> [sentence] ... *[other] ... }`).

use textweaver_core::{Direction, MarkerKind, Unit};
use textweaver_lexicon::i18n::Catalog;

use crate::app::Mode;

/// The key of a kind of structure in message ids and selectors:
/// `list-item` for [`MarkerKind::ListItem`].
pub(crate) fn kind_key(kind: MarkerKind) -> &'static str {
    match kind {
        MarkerKind::Heading => "heading",
        MarkerKind::Paragraph => "paragraph",
        MarkerKind::ListItem => "list-item",
        MarkerKind::List => "list",
        MarkerKind::Table => "table",
        MarkerKind::TableRow => "row",
        MarkerKind::TableCell => "cell",
        MarkerKind::Link => "link",
        MarkerKind::Image => "graphic",
        MarkerKind::Code => "code",
        MarkerKind::Quote => "quote",
        MarkerKind::PageBreak => "page",
        MarkerKind::SectionBreak => "section",
        MarkerKind::Bold => "bold",
        MarkerKind::Italic => "italic",
        MarkerKind::Underline => "underline",
        MarkerKind::Footnote => "footnote",
        MarkerKind::Strikethrough => "strikethrough",
        MarkerKind::Rule => "separator",
        MarkerKind::Math => "math",
    }
}

/// The spoken name of a kind of structure ("list item"), in the
/// catalog's language.
pub(crate) fn kind_name(c: &Catalog, kind: MarkerKind) -> String {
    c.tr(&format!("kind-{}", kind_key(kind)))
}

/// The key of a unit in message ids and selectors: `sentence`, or a
/// kind's key ([`kind_key`]).
pub(crate) fn unit_key(unit: Unit) -> &'static str {
    match unit {
        Unit::Grapheme => "character",
        Unit::Word => "word",
        Unit::Sentence => "sentence",
        Unit::Line => "line",
        Unit::Paragraph => "paragraph",
        Unit::Document => "document",
        Unit::Marker { kind, .. } => kind_key(kind),
    }
}

/// The spoken name of a unit ("sentence", "heading level 2").
pub(crate) fn unit_name(c: &Catalog, unit: Unit) -> String {
    match unit {
        Unit::Marker { kind, level: None } => kind_name(c, kind),
        Unit::Marker {
            kind,
            level: Some(l),
        } => c.fmt(
            "unit-with-level",
            &textweaver_lexicon::args!["what" => kind_name(c, kind), "unit" => kind_key(kind), "level" => l],
        ),
        u => c.tr(&format!("unit-{}", unit_key(u))),
    }
}

/// `next` or `previous`, the key a message chooses its words by.
pub(crate) fn dir_key(dir: Direction) -> &'static str {
    match dir {
        Direction::Forward => "next",
        Direction::Backward => "previous",
    }
}

/// A mode's short name ("Speech Cursor"), in the catalog's language.
pub(crate) fn mode_name(c: &Catalog, mode: Mode) -> String {
    let key = match mode {
        Mode::Browse => "browse",
        Mode::SpeechCursor => "speech-cursor",
        Mode::Edit => "edit",
        Mode::Find => "find",
        Mode::Command => "command",
        Mode::GoTo => "go-to",
        Mode::Open => "open",
        Mode::Prompt => "prompt",
    };
    c.tr(&format!("mode-{key}"))
}

/// "on" or "off", in the catalog's language.
pub(crate) fn on_off(c: &Catalog, on: bool) -> String {
    c.tr(if on { "common-on" } else { "common-off" })
}

/// The language of the system's user interface, as a tag (`es-MX`):
/// `LC_ALL`, `LC_MESSAGES`, or `LANG` where they are set, and on Windows
/// the user's locale. `None` when nothing says (or it says `C`).
pub fn system_language() -> Option<String> {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        let Ok(v) = std::env::var(var) else {
            continue;
        };
        let v = v.trim();
        if v.is_empty() {
            continue;
        }
        let plain = v.split('.').next().unwrap_or(v);
        if plain.eq_ignore_ascii_case("c") || plain.eq_ignore_ascii_case("posix") {
            return None;
        }
        return Some(textweaver_lexicon::i18n::normalize_tag(v));
    }
    windows_language()
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn windows_language() -> Option<String> {
    let mut buf = [0u16; 85];
    // SAFETY: the buffer is LOCALE_NAME_MAX_LENGTH (85) wide characters, as
    // the function asks; it writes at most that many and returns how many.
    let n = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buf) };
    let n = usize::try_from(n).ok().filter(|&n| n > 1)?;
    let tag = String::from_utf16_lossy(&buf[..n - 1]);
    Some(textweaver_lexicon::i18n::normalize_tag(&tag))
}

#[cfg(not(windows))]
fn windows_language() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English's names are the core's spoken names, so nothing English
    /// users hear changed when the names moved into the catalog.
    #[test]
    fn english_names_match_the_core() {
        let c = Catalog::english();
        for k in MarkerKind::ALL {
            assert_eq!(kind_name(&c, k), k.spoken_name(), "{k:?}");
        }
        for u in [
            Unit::Grapheme,
            Unit::Word,
            Unit::Sentence,
            Unit::Line,
            Unit::Paragraph,
            Unit::Document,
            Unit::heading(2),
            Unit::marker(MarkerKind::ListItem),
        ] {
            assert_eq!(unit_name(&c, u), u.spoken_name(), "{u:?}");
        }
        for m in [
            Mode::Browse,
            Mode::SpeechCursor,
            Mode::Edit,
            Mode::Find,
            Mode::Command,
            Mode::GoTo,
            Mode::Open,
            Mode::Prompt,
        ] {
            assert_eq!(mode_name(&c, m), m.name(), "{m:?}");
        }
    }
}
