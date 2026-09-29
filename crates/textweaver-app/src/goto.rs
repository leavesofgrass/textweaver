//! Parsing the go-to prompt.

use textweaver_core::CharPos;
use textweaver_text::GoTo;

/// [`parse_go_to`], also taking the words for the start and the end in
/// the catalog's language (`goto-word-start`, `goto-word-end`), which
/// the go-to prompt tells the user to type.
pub fn parse_go_to_in(c: &textweaver_lexicon::i18n::Catalog, input: &str) -> Option<GoTo> {
    let s = input.trim().to_lowercase();
    let word = |id: &str| c.tr(id).trim().to_lowercase();
    if s == word("goto-word-start") {
        return Some(GoTo::Start);
    }
    if s == word("goto-word-end") {
        return Some(GoTo::End);
    }
    parse_go_to(input)
}

/// The page a go-to answer names, if it names one: `page 12`, `p 12`,
/// `p. 12`, `p12`, or the catalog's word for a page (`goto-word-page`)
/// before the label. The label is returned as typed (`iv`, `A-3`, `12`),
/// for [`crate::pages`] to find. A bare number is not a page here;
/// `App::page_answer` makes it one in a paged document.
pub(crate) fn parse_page_in(c: &textweaver_lexicon::i18n::Catalog, input: &str) -> Option<String> {
    let s = input.trim();
    let lower = s.to_lowercase();
    let own = c.tr("goto-word-page").trim().to_lowercase();
    let mut words = vec![own.as_str(), "page", "p.", "p"];
    words.dedup();
    for w in words {
        if w.is_empty() {
            continue;
        }
        let Some(rest) = lower.strip_prefix(w) else {
            continue;
        };
        // "p" must be followed by a space or a digit ("p12"), so "percent"
        // and "pages" are not pages.
        let next = rest.chars().next();
        let joined = w.ends_with('.') || matches!(next, Some(c) if c.is_whitespace());
        let digit = w == "p" && matches!(next, Some(c) if c.is_ascii_digit());
        if !(joined || digit) {
            continue;
        }
        // Keep the label's own case: skip as many chars of the original.
        let label: String = s.chars().skip(w.chars().count()).collect();
        let label = label.trim();
        if !label.is_empty() {
            return Some(label.to_owned());
        }
    }
    None
}

/// Parses a go-to answer: `42` or `line 42` (a line), `50%` or
/// `50 percent`, `char 120` (a character position), or `start`, `top`,
/// `beginning`, `end`, `bottom`.
pub fn parse_go_to(input: &str) -> Option<GoTo> {
    let s = input.trim().to_lowercase();
    match s.as_str() {
        "start" | "top" | "beginning" | "begin" => return Some(GoTo::Start),
        "end" | "bottom" => return Some(GoTo::End),
        _ => {}
    }
    let pct = s
        .strip_suffix('%')
        .or_else(|| s.strip_suffix("percent"))
        .map(str::trim);
    if let Some(p) = pct {
        return p
            .parse::<u8>()
            .ok()
            .filter(|p| *p <= 100)
            .map(GoTo::Percent);
    }
    if let Some(c) = s
        .strip_prefix("char")
        .or_else(|| s.strip_prefix("character"))
    {
        let c = c.trim_start_matches("acter").trim();
        return c.parse::<usize>().ok().map(|c| GoTo::Char(CharPos(c)));
    }
    let line = s.strip_prefix("line").unwrap_or(&s).trim();
    line.parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .map(GoTo::Line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_targets() {
        assert_eq!(parse_go_to("12"), Some(GoTo::Line(12)));
        assert_eq!(parse_go_to(" line 3 "), Some(GoTo::Line(3)));
        assert_eq!(parse_go_to("50%"), Some(GoTo::Percent(50)));
        assert_eq!(parse_go_to("25 percent"), Some(GoTo::Percent(25)));
        assert_eq!(parse_go_to("char 7"), Some(GoTo::Char(CharPos(7))));
        assert_eq!(parse_go_to("character 7"), Some(GoTo::Char(CharPos(7))));
        assert_eq!(parse_go_to("End"), Some(GoTo::End));
        assert_eq!(parse_go_to("101%"), None);
        assert_eq!(parse_go_to("0"), None);
        assert_eq!(parse_go_to("soon"), None);
    }

    #[test]
    fn parses_pages() {
        let c = textweaver_lexicon::i18n::Catalog::english();
        let page = |s: &str| parse_page_in(&c, s);
        assert_eq!(page("page 12").as_deref(), Some("12"));
        assert_eq!(page("Page iv").as_deref(), Some("iv"));
        assert_eq!(page("p 12").as_deref(), Some("12"));
        assert_eq!(page("p. A-3").as_deref(), Some("A-3"));
        assert_eq!(page("p12").as_deref(), Some("12"));
        assert_eq!(page("12"), None);
        assert_eq!(page("50 percent"), None);
        assert_eq!(page("pages"), None);
        assert_eq!(page("page"), None);
    }
}
