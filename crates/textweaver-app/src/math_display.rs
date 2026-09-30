//! Unicode math in the reading view (`[reading] math_display = "unicode"`,
//! Agent W4g): the frontend draws each formula as one line of Unicode
//! (`x²`, `√2`, `1⁄2`) in place of its source, as Star's `mathrender.py`
//! did.
//!
//! Only the drawing changes. The document keeps its text, so speech, the
//! highlight, bookmarks, and the cursor keep their places: the frontend
//! shows a formula's Unicode on its first char and nothing on the rest.
//! Edit mode and math exploration (which highlights parts of the source)
//! show the source.

use textweaver_core::{CharRange, MarkerKind};
use textweaver_math::DetectOptions;
use textweaver_store::MathDisplay;

use crate::app::App;

impl App {
    /// The formulas in `range` to draw as Unicode, with their Unicode
    /// text, in document order; empty unless `[reading] math_display` is
    /// `unicode` and the reading view shows the document.
    pub fn math_display(&self, range: CharRange) -> Vec<(CharRange, String)> {
        if self.settings.reading.math_display != MathDisplay::Unicode
            || self.edit.is_some()
            || self.math_exploring()
        {
            return Vec::new();
        }
        let Some(s) = self.session.as_ref() else {
            return Vec::new();
        };
        let key = crate::frame_cache::MathKey {
            revision: s.revision,
            range,
            asciimath: self.settings.normalization.asciimath_delimiter,
        };
        crate::frame_cache::cached(&self.frame_cache.math, key, || self.math_display_now(range))
    }

    /// [`math_display`](Self::math_display) worked out anew: every math
    /// marker in `range` whose formula has a Unicode form.
    fn math_display_now(&self, range: CharRange) -> Vec<(CharRange, String)> {
        let Some(s) = self.session.as_ref() else {
            return Vec::new();
        };
        let opts = DetectOptions {
            asciimath: self.settings.normalization.asciimath_delimiter,
            ..DetectOptions::default()
        };
        let mut out: Vec<(CharRange, String)> = s
            .doc
            .markers()
            .iter()
            .filter(|m| {
                m.kind == MarkerKind::Math
                    && !m.range.is_empty()
                    && m.range.start < range.end
                    && range.start < m.range.end
            })
            .filter_map(|m| {
                let unicode = unicode_math(&s.doc.slice(m.range), &opts)?;
                Some((m.range, unicode))
            })
            .collect();
        out.sort_by_key(|(r, _)| r.start);
        // Nested or overlapping markers: the outer one wins.
        out.dedup_by(|b, a| b.0.start < a.0.end);
        out
    }
}

/// A formula's source (with its delimiters, as the reading view shows it)
/// as Unicode: `$x^2$` becomes `x²`. Text around the math in the marker
/// is kept. `None` when no math is found, or when the Unicode would be
/// empty.
pub(crate) fn unicode_math(source: &str, opts: &DetectOptions) -> Option<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut regions = textweaver_math::find_math(source, opts);
    if regions.is_empty() {
        // A marker may hold the formula without delimiters (DOCX, HTML).
        let math = textweaver_math::parse_latex(source.trim());
        let text = textweaver_math::to_unicode(&math);
        return (!text.is_empty()).then_some(text);
    }
    regions.sort_by_key(|r| r.range.start);
    let mut out = String::new();
    let mut at = 0usize;
    for r in regions {
        let (start, end) = (r.range.start.0, r.range.end.0);
        if start < at || end > chars.len() {
            continue;
        }
        out.extend(&chars[at..start]);
        let content: String = chars[r.content.start.0..r.content.end.0].iter().collect();
        out.push_str(&textweaver_math::to_unicode(&textweaver_math::parse(
            &content, r.notation,
        )));
        at = end;
    }
    out.extend(&chars[at.min(chars.len())..]);
    let out = out.trim().to_owned();
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delimiters_are_dropped_and_the_math_drawn() {
        let o = DetectOptions::default();
        assert_eq!(unicode_math("$x^2$", &o).as_deref(), Some("x²"));
        assert_eq!(unicode_math(r"$$\sqrt{2}$$", &o).as_deref(), Some("√2"));
        assert_eq!(unicode_math(r"\(\frac{1}{2}\)", &o).as_deref(), Some("1⁄2"));
        assert_eq!(unicode_math("", &o), None);
    }
}
