//! Parsing the go-to prompt.

use textweaver_core::CharPos;
use textweaver_text::GoTo;

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
}
