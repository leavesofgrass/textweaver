//! Small text helpers: CSL rich-text markup, entities, and escaping.
//!
//! CSL-JSON titles may carry a little HTML-like markup (`<i>`, `<b>`,
//! `<sup>`, `<sub>`, `<span class="nocase">`), and Crossref adds JATS tags
//! (`<jats:italic>`) and entities (`&amp;`). Read aloud, the tags would be
//! spelled out ("less than i greater than"), so every spoken or plain form
//! goes through [`plain_title`].

/// Strips markup tags, decodes the common entities, and collapses runs of
/// whitespace.
pub fn plain_title(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for seg in segments(s) {
        if let Segment::Text(t) = seg {
            out.push_str(&decode_entities(t));
        }
    }
    collapse_whitespace(&out)
}

/// Converts CSL rich text into a hayagriva formattable string: markup is
/// dropped, `<span class="nocase">…</span>` becomes a `{…}` group (case
/// kept), and hayagriva's control characters (`\`, `{`, `}`, `$`) are
/// escaped so a title such as "Costs of $5 and $10" is not read as math.
pub(crate) fn to_format_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let mut nocase_depth = 0usize;
    let mut span_stack: Vec<bool> = Vec::new();
    for seg in segments(s) {
        match seg {
            Segment::Text(t) => {
                for c in decode_entities(t).chars() {
                    if matches!(c, '\\' | '{' | '}' | '$') {
                        out.push('\\');
                    }
                    out.push(c);
                }
            }
            Segment::Tag(tag) => {
                let lower = tag.to_ascii_lowercase();
                if lower.starts_with("<span") {
                    let nocase = lower.contains("nocase");
                    span_stack.push(nocase);
                    if nocase {
                        nocase_depth += 1;
                        out.push('{');
                    }
                } else if lower.starts_with("</span")
                    && span_stack.pop() == Some(true)
                    && nocase_depth > 0
                {
                    nocase_depth -= 1;
                    out.push('}');
                }
            }
        }
    }
    for _ in 0..nocase_depth {
        out.push('}');
    }
    collapse_whitespace(&out)
}

enum Segment<'a> {
    Text(&'a str),
    Tag(&'a str),
}

/// Splits into text and `<tag>` segments. A `<` that does not start a tag
/// (`a < b`) stays text.
fn segments(s: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(lt) = rest.find('<') {
        let after = &rest[lt + 1..];
        let looks_like_tag = after
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '/');
        match (looks_like_tag, after.find('>')) {
            (true, Some(gt)) => {
                if lt > 0 {
                    out.push(Segment::Text(&rest[..lt]));
                }
                out.push(Segment::Tag(&rest[lt..lt + 1 + gt + 1]));
                rest = &after[gt + 1..];
            }
            _ => {
                out.push(Segment::Text(&rest[..lt + 1]));
                rest = after;
            }
        }
    }
    if !rest.is_empty() {
        out.push(Segment::Text(rest));
    }
    out
}

/// Decodes `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&#39;`, `&nbsp;`,
/// and numeric character references.
pub(crate) fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let decoded = tail.find(';').filter(|&semi| semi <= 10).and_then(|semi| {
            let name = &tail[1..semi];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                _ => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            }?;
            Some((c, semi + 1))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &tail[len..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Collapses whitespace runs to one space and trims.
pub(crate) fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Escapes text for HTML element content and attribute values.
pub(crate) fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escapes the Markdown characters that would otherwise start emphasis,
/// links, code, or HTML.
pub(crate) fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '*' | '_' | '[' | ']' | '`' | '<' | '>') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_and_entities_are_removed_for_speech() {
        assert_eq!(
            plain_title("The <i>E. coli</i>  genome &amp; <jats:italic>you</jats:italic>"),
            "The E. coli genome & you"
        );
        assert_eq!(plain_title("a < b and b > c"), "a < b and b > c");
        assert_eq!(plain_title("caf&#233; &#x2013; bar"), "café – bar");
        assert_eq!(plain_title("AT&T"), "AT&T");
    }

    #[test]
    fn format_strings_escape_hayagriva_controls() {
        assert_eq!(
            to_format_string("Costs of $5 and {x}"),
            "Costs of \\$5 and \\{x\\}"
        );
        assert_eq!(
            to_format_string("Life in <span class=\"nocase\">iOS</span> apps"),
            "Life in {iOS} apps"
        );
        assert_eq!(to_format_string("<span style=\"x\">plain</span>"), "plain");
    }
}
