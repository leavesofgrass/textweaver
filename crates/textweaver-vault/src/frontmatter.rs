//! YAML front matter: the `---` block at the top of an Obsidian note.
//!
//! Only the subset Obsidian's Properties editor writes is understood:
//! `key: value` scalars (plain, single- or double-quoted), flow lists
//! (`[a, "b, c"]`), block lists (`key:` then `- item` lines), and literal or
//! folded block scalars (`|`, `>`). Nested maps are skipped. Star parsed
//! with PyYAML when installed and fell back to a similar minimal parser
//! (`star/obsidian.py:_parse_frontmatter`).
//!
//! Writing quotes every value that plain YAML would misread (Star wrote
//! values unquoted, so a title containing `: ` or `#` produced invalid or
//! mis-typed YAML, Part 3 §7 item 42).

/// One front matter value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FmValue {
    /// A scalar, as text.
    Text(String),
    /// A list of scalars.
    List(Vec<String>),
    /// An integer, written unquoted. Parsing never produces this; read
    /// numbers with [`FrontMatter::int`].
    Int(i64),
}

/// Front matter entries in file order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrontMatter {
    entries: Vec<(String, FmValue)>,
}

impl FrontMatter {
    /// No entries.
    pub fn new() -> Self {
        FrontMatter::default()
    }

    /// True when there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &FmValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// The value of `key` (the last one when a key repeats).
    pub fn get(&self, key: &str) -> Option<&FmValue> {
        self.entries
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// The value of `key` as text: a scalar, or an integer. Empty text
    /// counts as absent.
    pub fn text(&self, key: &str) -> Option<String> {
        match self.get(key)? {
            FmValue::Text(s) if !s.trim().is_empty() => Some(s.clone()),
            FmValue::Int(n) => Some(n.to_string()),
            _ => None,
        }
    }

    /// The value of `key` as an integer.
    pub fn int(&self, key: &str) -> Option<i64> {
        match self.get(key)? {
            FmValue::Int(n) => Some(*n),
            FmValue::Text(s) => s.trim().parse().ok(),
            FmValue::List(_) => None,
        }
    }

    /// The value of `key` as a list: a list as is, a scalar as its
    /// comma- or space-separated parts (Obsidian accepts `tags: a, b`).
    pub fn list(&self, key: &str) -> Vec<String> {
        match self.get(key) {
            Some(FmValue::List(v)) => v.clone(),
            Some(FmValue::Text(s)) => s
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect(),
            Some(FmValue::Int(n)) => vec![n.to_string()],
            None => Vec::new(),
        }
    }

    /// Sets `key`, replacing an existing entry in place or appending.
    pub fn set(&mut self, key: &str, value: FmValue) {
        match self.entries.iter_mut().find(|(k, _)| k == key) {
            Some(e) => e.1 = value,
            None => self.entries.push((key.to_owned(), value)),
        }
    }

    /// Sets `key` to a text value.
    pub fn set_text(&mut self, key: &str, value: impl Into<String>) {
        self.set(key, FmValue::Text(value.into()));
    }

    /// Sets `key` to a list.
    pub fn set_list(&mut self, key: &str, values: Vec<String>) {
        self.set(key, FmValue::List(values));
    }

    /// Renders the block, fences included, ending with a newline.
    pub fn render(&self) -> String {
        let mut out = String::from("---\n");
        for (key, value) in &self.entries {
            out.push_str(key);
            out.push(':');
            match value {
                FmValue::Text(s) => {
                    out.push(' ');
                    out.push_str(&scalar(s, false));
                }
                FmValue::Int(n) => {
                    out.push(' ');
                    out.push_str(&n.to_string());
                }
                FmValue::List(items) => {
                    out.push_str(" [");
                    let parts: Vec<String> = items.iter().map(|i| scalar(i, true)).collect();
                    out.push_str(&parts.join(", "));
                    out.push(']');
                }
            }
            out.push('\n');
        }
        out.push_str("---\n");
        out
    }
}

/// Splits a note into its front matter and body. A note has front matter
/// only when its first line is `---` and a later line is `---` or `...`
/// (Star's rule); otherwise the whole text is the body. A leading byte
/// order mark is ignored.
pub fn split(text: &str) -> (FrontMatter, &str) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return (FrontMatter::new(), text);
    };
    if first.trim() != "---" {
        return (FrontMatter::new(), text);
    }
    let block_start = first.len();
    let mut offset = block_start;
    for line in lines {
        let t = line.trim();
        if t == "---" || t == "..." {
            let block = &text[block_start..offset];
            let body = &text[offset + line.len()..];
            return (parse(block), body);
        }
        offset += line.len();
    }
    (FrontMatter::new(), text)
}

/// Parses the inside of a front matter block (without the fences).
pub fn parse(block: &str) -> FrontMatter {
    let mut fm = FrontMatter::new();
    let lines: Vec<&str> = block.lines().collect();
    let mut i = 0;
    // The key whose value continues on following lines: a block list.
    let mut list_key: Option<String> = None;
    while i < lines.len() {
        let raw = lines[i];
        i += 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indented = raw.starts_with([' ', '\t']);
        if let Some(item) = list_item(trimmed)
            && let Some(key) = &list_key
            && (indented || raw.starts_with('-'))
        {
            let value = scalar_value(item);
            if let Some(FmValue::List(v)) = fm
                .entries
                .iter_mut()
                .rev()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
                && !value.is_empty()
            {
                v.push(value);
            }
            continue;
        }
        if indented {
            // A nested map or a stray continuation: not understood, skipped.
            continue;
        }
        let Some((key, value)) = raw.split_once(':') else {
            list_key = None;
            continue;
        };
        let key = key.trim().trim_matches(['"', '\'']).to_owned();
        let value = value.trim();
        list_key = None;
        if value.is_empty() {
            fm.entries.push((key.clone(), FmValue::List(Vec::new())));
            list_key = Some(key);
        } else if value.starts_with('[') {
            fm.entries.push((key, FmValue::List(flow_list(value))));
        } else if let Some(style) = block_scalar_style(value) {
            let mut parts = Vec::new();
            while i < lines.len()
                && (lines[i].trim().is_empty() || lines[i].starts_with([' ', '\t']))
            {
                parts.push(lines[i].trim());
                i += 1;
            }
            while parts.last().is_some_and(|p| p.is_empty()) {
                parts.pop();
            }
            let joined = if style == '|' {
                parts.join("\n")
            } else {
                parts.join(" ")
            };
            fm.entries.push((key, FmValue::Text(joined)));
        } else {
            fm.entries.push((key, FmValue::Text(scalar_value(value))));
        }
    }
    fm
}

/// `- item` or a bare `-`.
fn list_item(trimmed: &str) -> Option<&str> {
    if trimmed == "-" {
        Some("")
    } else {
        trimmed.strip_prefix("- ").map(str::trim)
    }
}

/// `|`, `>`, and their chomping variants.
fn block_scalar_style(value: &str) -> Option<char> {
    let first = value.chars().next()?;
    let rest = &value[first.len_utf8()..];
    if matches!(first, '|' | '>') && rest.chars().all(|c| matches!(c, '+' | '-' | '0'..='9')) {
        Some(first)
    } else {
        None
    }
}

/// A scalar as written: quoted forms unescaped, plain forms with a trailing
/// ` # comment` removed.
fn scalar_value(value: &str) -> String {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        return double_quoted(rest).0;
    }
    if let Some(rest) = value.strip_prefix('\'') {
        return single_quoted(rest).0;
    }
    let plain = match value.find(" #") {
        Some(at) => &value[..at],
        None => value,
    };
    plain.trim().to_owned()
}

/// Reads a double-quoted scalar after its opening quote. Returns the text
/// and the number of bytes consumed, closing quote included.
fn double_quoted(s: &str) -> (String, usize) {
    let mut out = String::new();
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return (out, i + 1),
            '\\' => match chars.next() {
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                Some((_, 'r')) => out.push('\r'),
                Some((_, '0')) => out.push('\0'),
                Some((_, 'u')) => {
                    let hex: String = chars.by_ref().take(4).map(|(_, h)| h).collect();
                    if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        out.push(ch);
                    }
                }
                Some((_, other)) => out.push(other),
                None => {}
            },
            _ => out.push(c),
        }
    }
    (out, s.len())
}

/// Reads a single-quoted scalar after its opening quote (`''` is a quote).
fn single_quoted(s: &str) -> (String, usize) {
    let mut out = String::new();
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\'' {
            if chars.peek().is_some_and(|(_, n)| *n == '\'') {
                chars.next();
                out.push('\'');
            } else {
                return (out, i + 1);
            }
        } else {
            out.push(c);
        }
    }
    (out, s.len())
}

/// Splits `[a, "b, c", 'd']` into its items, respecting quotes.
fn flow_list(value: &str) -> Vec<String> {
    let inner = value.strip_prefix('[').unwrap_or(value);
    let inner = inner
        .strip_suffix(']')
        .unwrap_or_else(|| inner.trim_end_matches(']'));
    let mut items = Vec::new();
    let mut rest = inner.trim_start();
    while !rest.is_empty() {
        let (item, used) = if let Some(q) = rest.strip_prefix('"') {
            let (text, n) = double_quoted(q);
            (text, n + 1)
        } else if let Some(q) = rest.strip_prefix('\'') {
            let (text, n) = single_quoted(q);
            (text, n + 1)
        } else {
            let end = rest.find(',').unwrap_or(rest.len());
            (rest[..end].trim().to_owned(), end)
        };
        if !item.is_empty() {
            items.push(item);
        }
        rest = rest[used..].trim_start();
        rest = rest.strip_prefix(',').unwrap_or(rest).trim_start();
    }
    items
}

/// Writes a scalar, quoted when plain YAML would misread it. `in_flow`
/// also quotes the characters that end a flow-list item.
fn scalar(s: &str, in_flow: bool) -> String {
    if needs_quotes(s, in_flow) {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '\r' => out.push_str("\\r"),
                c if c.is_control() => out.push_str(&format!("\\u{:04x}", u32::from(c))),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    } else {
        s.to_owned()
    }
}

fn needs_quotes(s: &str, in_flow: bool) -> bool {
    let Some(first) = s.chars().next() else {
        return true;
    };
    if s.trim() != s {
        return true;
    }
    if "-?:,[]{}#&*!|>'\"%@`".contains(first) {
        return true;
    }
    if s.contains(": ") || s.contains(" #") || s.ends_with(':') {
        return true;
    }
    if s.chars().any(char::is_control) {
        return true;
    }
    if in_flow && s.contains([',', '[', ']', '{', '}']) {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off" | "y" | "n" | "null" | "~"
    ) {
        return true;
    }
    // Numbers (and things YAML reads as numbers or dates) stay text.
    s.parse::<f64>().is_ok() || looks_like_date(s)
}

fn looks_like_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_front_matter_leaves_the_body() {
        let (fm, body) = split("# Title\n\ntext");
        assert!(fm.is_empty());
        assert_eq!(body, "# Title\n\ntext");
        // An unclosed block is not front matter (Star's rule).
        let (fm, body) = split("---\ntitle: x\nno close");
        assert!(fm.is_empty());
        assert_eq!(body, "---\ntitle: x\nno close");
    }

    #[test]
    fn splits_and_parses_scalars_and_lists() {
        let text = "---\ntitle: My Note\ntags: [a, \"b, c\", 'd']\naliases:\n  - One\n  - \"Two\"\nstar_id: 1a2b3c4d\n---\nBody line\n";
        let (fm, body) = split(text);
        assert_eq!(body, "Body line\n");
        assert_eq!(fm.text("title").as_deref(), Some("My Note"));
        assert_eq!(fm.list("tags"), vec!["a", "b, c", "d"]);
        assert_eq!(fm.list("aliases"), vec!["One", "Two"]);
        assert_eq!(fm.text("star_id").as_deref(), Some("1a2b3c4d"));
    }

    #[test]
    fn handles_crlf_bom_and_dot_fence() {
        let text = "\u{feff}---\r\ntitle: X\r\n...\r\nBody\r\n";
        let (fm, body) = split(text);
        assert_eq!(fm.text("title").as_deref(), Some("X"));
        assert_eq!(body, "Body\r\n");
    }

    #[test]
    fn block_lists_at_column_zero_and_comments() {
        let fm = parse(
            "tags:\n- x\n- y # not a comment inside the list item\n# comment\ntitle: T # trailing\n",
        );
        assert_eq!(fm.list("tags"), vec!["x", "y"]);
        assert_eq!(fm.text("title").as_deref(), Some("T"));
    }

    #[test]
    fn nested_maps_are_skipped() {
        let fm = parse("meta:\n  a: 1\n  b: 2\ntitle: T\n");
        assert_eq!(fm.text("title").as_deref(), Some("T"));
        assert_eq!(fm.list("meta"), Vec::<String>::new());
    }

    #[test]
    fn block_scalars() {
        let fm = parse("summary: |\n  line one\n  line two\nfolded: >-\n  a\n  b\nafter: z\n");
        assert_eq!(fm.text("summary").as_deref(), Some("line one\nline two"));
        assert_eq!(fm.text("folded").as_deref(), Some("a b"));
        assert_eq!(fm.text("after").as_deref(), Some("z"));
    }

    #[test]
    fn scalar_tags_split_on_commas_and_spaces() {
        let fm = parse("tags: exam, reading  chapter-1\n");
        assert_eq!(fm.list("tags"), vec!["exam", "reading", "chapter-1"]);
    }

    #[test]
    fn writes_quoted_values_that_would_break_yaml() {
        // Star bug 42: `title: Chapter 1: Intro #2` was written unquoted.
        let mut fm = FrontMatter::new();
        fm.set_text("title", "Chapter 1: Intro #2");
        fm.set_text("plain", "Just words");
        fm.set_text("list_like", "[not a list]");
        fm.set_text("boolish", "yes");
        fm.set_text("numeric", "1984");
        fm.set_text("quote", "She said \"hi\"\\");
        fm.set_text("path", r"C:\Users\x.md");
        fm.set_text("empty", "");
        fm.set(
            "tags",
            FmValue::List(vec!["a".into(), "b, c".into(), "[x]".into()]),
        );
        fm.set("position", FmValue::Int(42));
        let text = fm.render();
        assert!(text.contains("title: \"Chapter 1: Intro #2\"\n"));
        assert!(text.contains("plain: Just words\n"));
        assert!(text.contains("list_like: \"[not a list]\"\n"));
        assert!(text.contains("boolish: \"yes\"\n"));
        assert!(text.contains("numeric: \"1984\"\n"));
        assert!(text.contains("path: C:\\Users\\x.md\n"));
        assert!(text.contains("tags: [a, \"b, c\", \"[x]\"]\n"));
        assert!(text.contains("position: 42\n"));
        // And it reads back to the same values.
        let (back, body) = split(&text);
        assert_eq!(body, "");
        for key in [
            "title",
            "plain",
            "list_like",
            "boolish",
            "numeric",
            "quote",
            "path",
        ] {
            assert_eq!(back.text(key), fm.text(key), "{key}");
        }
        assert_eq!(back.text("empty"), None);
        assert_eq!(back.list("tags"), vec!["a", "b, c", "[x]"]);
        assert_eq!(back.int("position"), Some(42));
    }

    #[test]
    fn unicode_escapes_round_trip() {
        let mut fm = FrontMatter::new();
        fm.set_text("t", "bell\u{7}é");
        let (back, _) = split(&fm.render());
        assert_eq!(back.text("t").as_deref(), Some("bell\u{7}é"));
    }
}
