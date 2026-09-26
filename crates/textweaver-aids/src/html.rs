//! HTML output for the aids that mark ranges of text (bionic reading,
//! difficult words): the text, escaped, with each range wrapped in a tag.

use textweaver_core::{CharPos, CharRange};

/// Escapes `&`, `<`, `>`, and `"` for HTML text and attribute values.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    escape_into(s, &mut out);
    out
}

fn escape_into(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// `text` (whose first char is at `base`) escaped for HTML, with every
/// range in `ranges` wrapped in `open` and `close` (for example `<b>` and
/// `</b>`, or `<mark class="rare">` and `</mark>`). Ranges must be sorted
/// and must not overlap; ranges outside the text are ignored, and an
/// overlapping range is clipped to start where the previous one ended.
///
/// ```
/// # use textweaver_aids::{html::wrap_ranges, bionic_text, BionicOptions};
/// # use textweaver_core::CharPos;
/// let t = "Read <this>";
/// let r = bionic_text(t, CharPos(0), &BionicOptions::star());
/// assert_eq!(wrap_ranges(t, CharPos(0), &r, "<b>", "</b>"), "<b>Re</b>ad &lt;<b>th</b>is&gt;");
/// ```
pub fn wrap_ranges(
    text: &str,
    base: CharPos,
    ranges: &[CharRange],
    open: &str,
    close: &str,
) -> String {
    // Clip each range to start where the previous one ended.
    let mut spans: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
    let mut last_end = base.0;
    for r in ranges {
        let s = r.start.0.max(last_end);
        if r.end.0 > s {
            spans.push((s, r.end.0));
            last_end = r.end.0;
        }
    }
    let mut out = String::with_capacity(text.len() + spans.len() * (open.len() + close.len()));
    let mut ri = 0usize;
    let mut inside = false;
    let mut buf = [0u8; 4];
    for (pos, c) in (base.0..).zip(text.chars()) {
        if inside && pos >= spans[ri].1 {
            out.push_str(close);
            inside = false;
            ri += 1;
        }
        if !inside && ri < spans.len() && spans[ri].0 == pos {
            out.push_str(open);
            inside = true;
        }
        escape_into(c.encode_utf8(&mut buf), &mut out);
    }
    if inside {
        out.push_str(close);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes() {
        assert_eq!(
            escape("a < b & \"c\" > d"),
            "a &lt; b &amp; &quot;c&quot; &gt; d"
        );
    }

    #[test]
    fn wraps_ranges_with_offsets() {
        let t = "über <alles> café";
        let r = [
            CharRange::new(10, 12),
            CharRange::new(16, 20),
            CharRange::new(23, 99),
        ];
        assert_eq!(
            wrap_ranges(t, CharPos(10), &r, "<b>", "</b>"),
            "<b>üb</b>er &lt;<b>alle</b>s&gt; <b>café</b>"
        );
    }

    #[test]
    fn odd_ranges() {
        let t = "abcdef";
        // Empty, overlapping, and out-of-text ranges.
        let r = [
            CharRange::new(1, 1),
            CharRange::new(1, 3),
            CharRange::new(2, 4),
            CharRange::new(10, 12),
        ];
        assert_eq!(wrap_ranges(t, CharPos(0), &r, "[", "]"), "a[bc][d]ef");
        assert_eq!(wrap_ranges("", CharPos(0), &r, "[", "]"), "");
        assert_eq!(wrap_ranges(t, CharPos(0), &[], "[", "]"), "abcdef");
    }
}
