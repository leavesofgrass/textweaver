//! Document positions ([`CharPos`], Unicode scalar values) to and from the
//! native text control's positions.
//!
//! - Windows (RichEdit, `wxTE_RICH2`) and macOS (`NSTextView`) count UTF-16
//!   code units, and a line break is one unit (RichEdit stores `\n` as a
//!   single `\r`), so a position is the document's UTF-16 offset: the
//!   `DisplayIndex` that `textweaver-text` builds for the GUI (ADR-0002).
//! - GTK (`GtkTextView`) counts characters, so a position is the char offset.
//!
//! Canonical text uses `\n` only, so no `\r\n` pairs need adjusting.

use textweaver_app::core::CharPos;
use textweaver_app::text::Document;

/// How the native control counts positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Units {
    /// UTF-16 code units (Windows, macOS).
    Utf16,
    /// Unicode scalar values (GTK).
    Chars,
}

impl Units {
    /// The units of this platform's multi-line text control.
    pub const NATIVE: Units = if cfg!(any(target_os = "windows", target_os = "macos")) {
        Units::Utf16
    } else {
        Units::Chars
    };

    /// The control position of `pos`.
    pub fn to_ctrl(self, doc: &Document, pos: CharPos) -> i64 {
        let pos = CharPos(pos.0.min(doc.len_chars()));
        match self {
            Units::Utf16 => i64::from(doc.display().to_utf16(pos)),
            Units::Chars => i64::try_from(pos.0).unwrap_or(i64::MAX),
        }
    }

    /// The document position of control position `ctrl` (clamped; a
    /// position inside a surrogate pair maps to its character).
    pub fn to_doc(self, doc: &Document, ctrl: i64) -> CharPos {
        let ctrl = u64::try_from(ctrl.max(0)).unwrap_or(0);
        match self {
            Units::Utf16 => {
                let unit = u32::try_from(ctrl).unwrap_or(u32::MAX);
                let end = doc.display().to_utf16(doc.end());
                doc.display().to_char(unit.min(end))
            }
            Units::Chars => CharPos(
                usize::try_from(ctrl)
                    .unwrap_or(usize::MAX)
                    .min(doc.len_chars()),
            ),
        }
    }

    /// The control's expected last position (its length) for `doc`.
    pub fn len(self, doc: &Document) -> i64 {
        self.to_ctrl(doc, doc.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_counts_astral_characters_twice() {
        // "a😀b\nc": the emoji is one char and two UTF-16 units.
        let doc = Document::from_plain_text("a\u{1F600}b\nc");
        let u = Units::Utf16;
        assert_eq!(u.to_ctrl(&doc, CharPos(0)), 0);
        assert_eq!(u.to_ctrl(&doc, CharPos(2)), 3);
        assert_eq!(u.to_ctrl(&doc, CharPos(4)), 5);
        assert_eq!(u.len(&doc), 6);
        assert_eq!(u.to_doc(&doc, 3), CharPos(2));
        assert_eq!(u.to_doc(&doc, 5), CharPos(4));
        assert_eq!(u.to_doc(&doc, 99), doc.end());
        assert_eq!(u.to_doc(&doc, -4), CharPos(0));
    }

    #[test]
    fn chars_are_identity_but_clamped() {
        let doc = Document::from_plain_text("a\u{1F600}b");
        let u = Units::Chars;
        assert_eq!(u.to_ctrl(&doc, CharPos(2)), 2);
        assert_eq!(u.to_doc(&doc, 2), CharPos(2));
        assert_eq!(u.to_doc(&doc, 50), doc.end());
        assert_eq!(u.len(&doc), 3);
    }

    #[test]
    fn round_trip_every_char_boundary() {
        let doc = Document::from_plain_text("Ünïcödé 🎉 text\nwith 𝄞 music\n\nand more.");
        for u in [Units::Utf16, Units::Chars] {
            for c in 0..=doc.len_chars() {
                let pos = CharPos(c);
                assert_eq!(u.to_doc(&doc, u.to_ctrl(&doc, pos)), pos, "{u:?} at {c}");
            }
        }
    }
}
