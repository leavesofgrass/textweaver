//! Front matter: YAML metadata blocks (`---` … `---` or `...`) at the start
//! of a document, and Pandoc title blocks (`% Title`, `% Author`, `% Date`).
//!
//! The YAML reader is a small, safe subset written for metadata: block maps
//! nested by indentation, block and flow sequences, plain, single-quoted and
//! double-quoted scalars, literal (`|`) and folded (`>`) block scalars,
//! numbers, booleans, null, and `#` comments. Anchors, aliases, tags, and
//! multi-document streams are not supported; anything it cannot read stays a
//! string, so a malformed block never fails a conversion.

use serde_json::{Map, Number, Value};

/// Splits leading front matter from `src`: the metadata (empty when there
/// is none) and the Markdown body that follows it.
pub fn split(src: &str, pandoc_title_block: bool) -> (Map<String, Value>, &str) {
    let body = src.strip_prefix('\u{feff}').unwrap_or(src);
    if let Some((yaml, rest)) = yaml_block(body) {
        return (parse_yaml(yaml), rest);
    }
    if pandoc_title_block && let Some((meta, rest)) = title_block(body) {
        return (meta, rest);
    }
    (Map::new(), body)
}

/// The YAML text and the rest of the document when `src` opens with `---`.
fn yaml_block(src: &str) -> Option<(&str, &str)> {
    let first_end = src.find('\n')?;
    if src[..first_end].trim_end() != "---" {
        return None;
    }
    let inner_start = first_end + 1;
    let mut at = inner_start;
    while at <= src.len() {
        let line_end = src[at..].find('\n').map_or(src.len(), |i| at + i);
        let line = src[at..line_end].trim_end();
        if line == "---" || line == "..." {
            let rest_start = (line_end + 1).min(src.len());
            return Some((&src[inner_start..at], &src[rest_start..]));
        }
        if line_end >= src.len() {
            break;
        }
        at = line_end + 1;
    }
    None
}

/// A Pandoc title block: up to three leading lines starting with `%`.
fn title_block(src: &str) -> Option<(Map<String, Value>, &str)> {
    if !src.starts_with('%') {
        return None;
    }
    let mut meta = Map::new();
    let mut at = 0;
    for key in ["title", "author", "date"] {
        if !src[at..].starts_with('%') {
            break;
        }
        let end = src[at..].find('\n').map_or(src.len(), |i| at + i);
        let value = src[at + 1..end].trim();
        if !value.is_empty() {
            meta.insert(key.to_owned(), Value::String(value.to_owned()));
        }
        at = (end + 1).min(src.len());
    }
    Some((meta, &src[at..]))
}

/// One logical line: its indentation and its content without the comment.
struct Line<'a> {
    indent: usize,
    text: &'a str,
}

/// Parses the YAML subset into a JSON object; a top level that is not a map
/// gives an empty object.
pub fn parse_yaml(src: &str) -> Map<String, Value> {
    let lines: Vec<&str> = src.lines().collect();
    let mut p = Parser { lines, at: 0 };
    match p.block(0) {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    at: usize,
}

impl<'a> Parser<'a> {
    /// The next non-blank, non-comment line, without consuming it.
    fn peek(&mut self) -> Option<Line<'a>> {
        while self.at < self.lines.len() {
            let raw = self.lines[self.at];
            let trimmed = raw.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                self.at += 1;
                continue;
            }
            return Some(Line {
                indent: raw.len() - trimmed.len(),
                text: strip_comment(trimmed).trim_end(),
            });
        }
        None
    }

    /// A block (map or sequence) whose lines are indented at least `min`.
    fn block(&mut self, min: usize) -> Value {
        let Some(first) = self.peek() else {
            return Value::Null;
        };
        if first.indent < min {
            return Value::Null;
        }
        let indent = first.indent;
        if first.text == "-" || first.text.starts_with("- ") {
            self.sequence(indent)
        } else {
            self.map(indent)
        }
    }

    fn sequence(&mut self, indent: usize) -> Value {
        let mut items = Vec::new();
        while let Some(line) = self.peek() {
            if line.indent != indent || !(line.text == "-" || line.text.starts_with("- ")) {
                break;
            }
            self.at += 1;
            let rest = line.text[1..].trim_start();
            if rest.is_empty() {
                items.push(self.block(indent + 1));
            } else if let Some((k, v)) = split_key(rest) {
                // "- key: value" starts a map whose other keys are indented
                // to the column after "- ".
                let inner = indent + (line.text.len() - rest.len());
                let mut m = Map::new();
                let first = self.value_after_key(v, inner);
                m.insert(k, first);
                if let Value::Object(more) = self.map_at(inner) {
                    m.extend(more);
                }
                items.push(Value::Object(m));
            } else {
                items.push(scalar(rest));
            }
        }
        Value::Array(items)
    }

    fn map(&mut self, indent: usize) -> Value {
        self.map_at(indent)
    }

    fn map_at(&mut self, indent: usize) -> Value {
        let mut m = Map::new();
        while let Some(line) = self.peek() {
            if line.indent != indent {
                break;
            }
            let Some((k, v)) = split_key(line.text) else {
                break;
            };
            self.at += 1;
            let value = self.value_after_key(v, indent);
            m.insert(k, value);
        }
        Value::Object(m)
    }

    /// The value of a `key:` whose inline remainder is `v`.
    fn value_after_key(&mut self, v: &str, indent: usize) -> Value {
        match v {
            "" => {
                let child = self.peek();
                match child {
                    Some(c) if c.indent > indent => self.block(indent + 1),
                    // A sequence may sit at the same indent as its key.
                    Some(c)
                        if c.indent == indent && (c.text == "-" || c.text.starts_with("- ")) =>
                    {
                        self.sequence(indent)
                    }
                    _ => Value::Null,
                }
            }
            "|" | "|-" | "|+" | ">" | ">-" | ">+" => self.block_scalar(v, indent),
            _ => scalar(v),
        }
    }

    /// A literal or folded block scalar: the more-indented raw lines after
    /// the key.
    fn block_scalar(&mut self, style: &str, indent: usize) -> Value {
        let mut raw: Vec<&str> = Vec::new();
        while self.at < self.lines.len() {
            let l = self.lines[self.at];
            let t = l.trim_start();
            if !t.is_empty() && l.len() - t.len() <= indent {
                break;
            }
            raw.push(l);
            self.at += 1;
        }
        while raw.last().is_some_and(|l| l.trim().is_empty()) {
            raw.pop();
        }
        let cut = raw
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        let lines: Vec<&str> = raw
            .iter()
            .map(|l| if l.len() >= cut { &l[cut..] } else { "" })
            .collect();
        let mut text = if style.starts_with('|') {
            lines.join("\n")
        } else {
            let mut out = String::new();
            for (i, l) in lines.iter().enumerate() {
                if i > 0 {
                    out.push(if l.is_empty() || lines[i - 1].is_empty() {
                        '\n'
                    } else {
                        ' '
                    });
                }
                out.push_str(l);
            }
            out
        };
        if !style.ends_with('-') {
            text.push('\n');
        }
        Value::String(text)
    }
}

/// `key: rest` (the key unquoted), when the line is a map entry.
fn split_key(text: &str) -> Option<(String, &str)> {
    let (key, rest) = if let Some(q) = text.strip_prefix('"') {
        let end = q.find('"')?;
        (q[..end].to_owned(), q[end + 1..].strip_prefix(':')?)
    } else if let Some(q) = text.strip_prefix('\'') {
        let end = q.find('\'')?;
        (q[..end].to_owned(), q[end + 1..].strip_prefix(':')?)
    } else {
        let colon = text
            .char_indices()
            .find(|&(i, c)| {
                c == ':' && text[i + 1..].chars().next().is_none_or(char::is_whitespace)
            })?
            .0;
        (text[..colon].trim().to_owned(), &text[colon + 1..])
    };
    if key.is_empty() || !(rest.is_empty() || rest.starts_with(char::is_whitespace)) {
        return None;
    }
    Some((key, rest.trim()))
}

/// Removes a trailing ` # comment` outside quotes.
fn strip_comment(text: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut prev_space = true;
    for (i, c) in text.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '#' && prev_space => return &text[..i],
            None => {}
        }
        prev_space = c.is_whitespace();
    }
    text
}

/// A flow value: quoted string, flow sequence, flow map (kept as text),
/// number, boolean, null, or plain string.
fn scalar(v: &str) -> Value {
    let v = v.trim();
    if let Some(q) = v.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Value::String(unescape_double(q));
    }
    if let Some(q) = v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return Value::String(q.replace("''", "'"));
    }
    if let Some(inner) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        return Value::Array(
            split_flow(inner)
                .into_iter()
                .filter(|s| !s.trim().is_empty())
                .map(scalar)
                .collect(),
        );
    }
    match v {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" | "yes" | "Yes" => return Value::Bool(true),
        "false" | "False" | "FALSE" | "no" | "No" => return Value::Bool(false),
        _ => {}
    }
    if v.bytes()
        .all(|b| b.is_ascii_digit() || b == b'-' || b == b'+')
        && let Ok(n) = v.parse::<i64>()
    {
        return Value::Number(n.into());
    }
    if v.bytes().any(|b| b == b'.')
        && v.bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+' | b'e' | b'E'))
        && let Some(n) = v.parse::<f64>().ok().and_then(Number::from_f64)
    {
        return Value::Number(n);
    }
    Value::String(v.to_owned())
}

/// Splits a flow sequence body at top-level commas.
fn split_flow(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    out.push(&s[start..i]);
                    start = i + 1;
                }
                _ => {}
            },
        }
    }
    out.push(&s[start..]);
    out
}

fn unescape_double(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(o) => {
                out.push('\\');
                out.push(o);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// A metadata value as display text: strings as is, arrays joined with
/// ", ", other values in their JSON spelling.
pub fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(value_text).collect::<Vec<_>>().join(", "),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn splits_yaml_front_matter() {
        let (m, body) = split("---\ntitle: Hello\ntags: [a, b]\n---\n# Body\n", false);
        assert_eq!(m.get("title"), Some(&json!("Hello")));
        assert_eq!(m.get("tags"), Some(&json!(["a", "b"])));
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn no_front_matter_without_closing_fence() {
        let (m, body) = split("---\ntitle: x\n", false);
        assert!(m.is_empty());
        assert_eq!(body, "---\ntitle: x\n");
    }

    #[test]
    fn thematic_break_is_not_front_matter() {
        let (m, body) = split("Text\n\n---\n", false);
        assert!(m.is_empty());
        assert_eq!(body, "Text\n\n---\n");
    }

    #[test]
    fn nested_maps_sequences_and_scalars() {
        let m = parse_yaml(
            "title: \"A: quoted\"\nauthor:\n  - Ada\n  - name: Grace\n    role: admiral\nlang: en-GB # comment\ncount: 3\nratio: 0.5\ndraft: false\nnothing: ~\nabstract: |\n  Line one\n  line two\nsummary: >\n  folded\n  text\n",
        );
        assert_eq!(
            Value::Object(m),
            json!({
                "title": "A: quoted",
                "author": ["Ada", {"name": "Grace", "role": "admiral"}],
                "lang": "en-GB",
                "count": 3,
                "ratio": 0.5,
                "draft": false,
                "nothing": null,
                "abstract": "Line one\nline two\n",
                "summary": "folded text\n",
            })
        );
    }

    #[test]
    fn sequence_at_key_indent() {
        let m = parse_yaml("tags:\n- one\n- two\nnext: x\n");
        assert_eq!(
            Value::Object(m),
            json!({"tags": ["one", "two"], "next": "x"})
        );
    }

    #[test]
    fn urls_keep_their_colons() {
        let m = parse_yaml("link: https://example.org/a\n");
        assert_eq!(m.get("link"), Some(&json!("https://example.org/a")));
    }

    #[test]
    fn pandoc_title_block() {
        let (m, body) = split("% My Title\n% Jo Writer\n% 2026\n\nText\n", true);
        assert_eq!(m.get("title"), Some(&json!("My Title")));
        assert_eq!(m.get("author"), Some(&json!("Jo Writer")));
        assert_eq!(m.get("date"), Some(&json!("2026")));
        assert_eq!(body, "\nText\n");
    }
}
