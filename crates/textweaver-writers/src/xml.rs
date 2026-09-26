//! Hand-written XML: escaping and a character filter for XML 1.0.

/// True for chars XML 1.0 allows in content.
fn allowed(c: char) -> bool {
    matches!(c,
        '\t' | '\n' | '\r'
        | '\u{20}'..='\u{D7FF}'
        | '\u{E000}'..='\u{FFFD}'
        | '\u{10000}'..='\u{10FFFF}')
}

/// Escapes text content (`&`, `<`, `>`), dropping chars XML cannot hold.
pub(crate) fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c if allowed(c) => out.push(c),
            _ => {}
        }
    }
    out
}

/// Escapes an attribute value (double-quoted), dropping chars XML cannot
/// hold. Line breaks and tabs become character references so they survive
/// attribute-value normalization.
pub(crate) fn attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            c if allowed(c) => out.push(c),
            _ => {}
        }
    }
    out
}

/// A valid XML id (NCName) from arbitrary text: ASCII letters, digits, `-`,
/// `_`, `.`; anything else becomes `_`; prefixed so it starts with a letter.
pub(crate) fn id(prefix: &str, raw: &str) -> String {
    let mut out = String::from(prefix);
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes() {
        assert_eq!(text("a < b & c > d\u{1}"), "a &lt; b &amp; c &gt; d");
        assert_eq!(attr("say \"hi\"\n"), "say &quot;hi&quot;&#10;");
        assert_eq!(id("fn-", "note 1/é"), "fn-note_1__");
    }
}
