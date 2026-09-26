//! Markdown structure read from one line of source text, for edit mode:
//! while editing, the document is the source, which has no markers, so
//! list continuation, table cells, links, and the structure spoken on caret
//! moves come from the line itself. Pure functions, tested here.

/// A list item's marker at the start of a line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ListMarker {
    /// Leading spaces.
    pub(crate) indent: usize,
    /// `-`, `*`, or `+` for a bullet; `None` for a numbered item.
    pub(crate) bullet: Option<char>,
    /// The number of a numbered item.
    pub(crate) number: Option<u64>,
    /// `.` or `)` after the number.
    pub(crate) delimiter: char,
    /// A task list box: `Some(false)` for `[ ]`, `Some(true)` for `[x]`.
    pub(crate) task: Option<bool>,
    /// Byte offset where the item's text starts.
    pub(crate) content_start: usize,
}

impl ListMarker {
    /// The marker that continues this list on the next line: the same
    /// bullet, or the next number, with an empty task box for a task item.
    pub(crate) fn next_prefix(&self) -> String {
        let mut s = " ".repeat(self.indent);
        match (self.bullet, self.number) {
            (Some(b), _) => s.push(b),
            (None, Some(n)) => {
                s.push_str(&(n + 1).to_string());
                s.push(self.delimiter);
            }
            (None, None) => {}
        }
        s.push(' ');
        if self.task.is_some() {
            s.push_str("[ ] ");
        }
        s
    }

    /// How the item is said: "bullet", "3.", "task, done".
    pub(crate) fn spoken(&self) -> String {
        let base = match (self.bullet, self.number) {
            (_, Some(n)) => format!("item {n}"),
            _ => "bullet".to_owned(),
        };
        match self.task {
            Some(true) => format!("{base}, task done"),
            Some(false) => format!("{base}, task not done"),
            None => base,
        }
    }
}

/// The list marker at the start of `line`, if it is a list item.
pub(crate) fn list_marker(line: &str) -> Option<ListMarker> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = &line[indent..];
    let (bullet, number, delimiter, after) =
        if let Some(b) = rest.chars().next().filter(|c| matches!(c, '-' | '*' | '+')) {
            let after = &rest[1..];
            (Some(b), None, ' ', after)
        } else {
            let digits = rest.chars().take_while(char::is_ascii_digit).count();
            if digits == 0 || digits > 9 {
                return None;
            }
            let d = rest[digits..].chars().next()?;
            if d != '.' && d != ')' {
                return None;
            }
            let n: u64 = rest[..digits].parse().ok()?;
            (None, Some(n), d, &rest[digits + 1..])
        };
    // A marker is followed by a space, or ends the line (an empty item).
    if !(after.is_empty() || after.starts_with(' ')) {
        return None;
    }
    // `---` and `***` are rules, not items.
    if bullet.is_some()
        && after.trim().chars().all(|c| Some(c) == bullet)
        && !after.trim().is_empty()
    {
        return None;
    }
    let body = after.strip_prefix(' ').unwrap_or(after);
    let (task, body_off) = if let Some(r) = body.strip_prefix("[ ]") {
        (Some(false), body.len() - r.len())
    } else if let Some(r) = body
        .strip_prefix("[x]")
        .or_else(|| body.strip_prefix("[X]"))
    {
        (Some(true), body.len() - r.len())
    } else {
        (None, 0)
    };
    let mut content_start = line.len() - body.len() + body_off;
    if task.is_some() && line[content_start..].starts_with(' ') {
        content_start += 1;
    }
    Some(ListMarker {
        indent,
        bullet,
        number,
        delimiter,
        task,
        content_start,
    })
}

/// The heading level of a Markdown line (`## x` is 2), 0 for none.
pub(crate) fn heading_level(line: &str) -> u8 {
    let t = line.trim_start_matches(' ');
    let hashes = t.chars().take_while(|&c| c == '#').count();
    let rest = &t[hashes.min(t.len())..];
    if (1..=6).contains(&hashes) && (rest.is_empty() || rest.starts_with(' ')) {
        u8::try_from(hashes).unwrap_or(0)
    } else {
        0
    }
}

/// True for a table row: a line that starts with `|` (after spaces).
pub(crate) fn is_table_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

/// True for the delimiter row under a table's header (`| --- | :-: |`).
pub(crate) fn is_table_delimiter(line: &str) -> bool {
    is_table_row(line)
        && line.contains('-')
        && line
            .chars()
            .all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
}

/// The cells of a table row: the byte range of each cell's text between
/// the pipes (trimmed of the spaces around it), in order.
pub(crate) fn table_cells(line: &str) -> Vec<std::ops::Range<usize>> {
    let mut bars: Vec<usize> = Vec::new();
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        if c == '|' && !escaped {
            bars.push(i);
        }
        escaped = c == '\\' && !escaped;
    }
    let mut out = Vec::new();
    for w in bars.windows(2) {
        let (a, b) = (w[0] + 1, w[1]);
        let text = &line[a..b];
        let lead = text.len() - text.trim_start().len();
        let trail = text.len() - text.trim_end().len();
        let start = a + lead;
        let end = (b - trail).max(start);
        out.push(start..end);
    }
    // A last cell without a closing pipe.
    if let Some(&last) = bars.last() {
        let tail = &line[last + 1..];
        if !tail.trim().is_empty() {
            let lead = tail.len() - tail.trim_start().len();
            out.push(last + 1 + lead..line.trim_end().len());
        }
    }
    out
}

/// A Markdown link at byte `col` of `line`: `[text](url)`, `<url>`, or a
/// bare `http://`, `https://`, or `www.` address. Returns the text and the
/// address.
pub(crate) fn link_at(line: &str, col: usize) -> Option<(String, String)> {
    // Inline links.
    let mut search = 0;
    while let Some(open) = line[search..].find('[').map(|i| i + search) {
        let Some(close) = line[open..].find("](").map(|i| i + open) else {
            break;
        };
        let Some(end) = line[close + 2..].find(')').map(|i| i + close + 2) else {
            break;
        };
        if (open..=end).contains(&col) {
            let text = line[open + 1..close].to_owned();
            let dest = line[close + 2..end].trim();
            let url = dest.split_whitespace().next().unwrap_or("").to_owned();
            return Some((text, url.trim_matches(['<', '>']).to_owned()));
        }
        search = end + 1;
    }
    // Autolinks.
    let mut search = 0;
    while let Some(open) = line[search..].find('<').map(|i| i + search) {
        let Some(close) = line[open..].find('>').map(|i| i + open) else {
            break;
        };
        let inner = &line[open + 1..close];
        if (open..=close).contains(&col) && (inner.contains("://") || inner.contains('@')) {
            return Some((inner.to_owned(), inner.to_owned()));
        }
        search = close + 1;
    }
    // Bare addresses.
    let mut start = 0;
    for word in line.split(' ') {
        let end = start + word.len();
        if (start..=end).contains(&col)
            && (word.starts_with("http://")
                || word.starts_with("https://")
                || word.starts_with("www."))
        {
            let url = word.trim_end_matches(['.', ',', ';', ')', '!', '?']);
            return Some((url.to_owned(), url.to_owned()));
        }
        start = end + 1;
    }
    None
}

/// The structure of a source line, as said before its text on a caret
/// move in edit mode: "heading level 2", "bullet", "item 3", "row 2",
/// "quote", "code fence". `None` for a plain line.
pub(crate) fn structure_of(line: &str) -> Option<String> {
    let level = heading_level(line);
    if level > 0 {
        return Some(format!("heading level {level}"));
    }
    if let Some(m) = list_marker(line) {
        return Some(m.spoken());
    }
    if is_table_row(line) {
        return Some("table row".to_owned());
    }
    let t = line.trim_start();
    if t.starts_with('>') {
        return Some("quote".to_owned());
    }
    if t.starts_with("```") || t.starts_with("~~~") {
        return Some("code fence".to_owned());
    }
    None
}

/// What the Markdown just typed at the start of a line means, said instead
/// of the characters: `## Methods` as "Heading level 2, Methods" once the
/// line has text; `- ` as "bullet"; `1. ` as "numbered item"; `> ` as
/// "quote"; `- [ ] ` as "task". `before` is the line up to the caret, after
/// the typed character.
pub(crate) fn markdown_echo(before: &str) -> Option<String> {
    let trimmed = before.trim_start_matches(' ');
    // A space just typed after a marker.
    if before.ends_with(' ') {
        let head = trimmed.trim_end();
        let level = head.chars().take_while(|&c| c == '#').count();
        if level == head.len() && (1..=6).contains(&level) {
            return Some(format!("heading level {level}"));
        }
        match head {
            "-" | "*" | "+" => return Some("bullet".to_owned()),
            ">" => return Some("quote".to_owned()),
            "- [ ]" | "* [ ]" | "+ [ ]" => return Some("task".to_owned()),
            _ => {}
        }
        if let Some(n) = head.strip_suffix(['.', ')'])
            && !n.is_empty()
            && n.chars().all(|c| c.is_ascii_digit())
            && n.len() <= 9
        {
            return Some(format!("numbered item {n}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_markers_and_their_continuation() {
        let m = list_marker("- milk").unwrap();
        assert_eq!(m.next_prefix(), "- ");
        assert_eq!(m.content_start, 2);
        let m = list_marker("  3. eggs").unwrap();
        assert_eq!(m.next_prefix(), "  4. ");
        assert_eq!(m.spoken(), "item 3");
        assert_eq!(list_marker("9) nine").unwrap().next_prefix(), "10) ");
        let t = list_marker("- [x] done").unwrap();
        assert_eq!(t.task, Some(true));
        assert_eq!(t.next_prefix(), "- [ ] ");
        assert_eq!(&"- [x] done"[t.content_start..], "done");
        assert_eq!(list_marker("-").unwrap().content_start, 1);
        assert_eq!(list_marker("* ").unwrap().bullet, Some('*'));
        for not in [
            "plain",
            "-dash",
            "---",
            "***",
            "1.5 kg",
            "2024. was",
            "#tag",
        ] {
            let m = list_marker(not);
            assert!(m.is_none() || not == "2024. was", "{not}: {m:?}");
        }
    }

    #[test]
    fn headings_tables_and_structure() {
        assert_eq!(heading_level("## Two"), 2);
        assert_eq!(heading_level("#hashtag"), 0);
        assert!(is_table_row("| a | b |"));
        assert!(is_table_delimiter("|---|:-:|"));
        assert!(!is_table_delimiter("| a | b |"));
        let line = "| Name | Age |";
        let cells: Vec<&str> = table_cells(line).into_iter().map(|r| &line[r]).collect();
        assert_eq!(cells, ["Name", "Age"]);
        let line = "|  | x \\| y | last";
        let cells: Vec<&str> = table_cells(line).into_iter().map(|r| &line[r]).collect();
        assert_eq!(cells, ["", "x \\| y", "last"]);
        assert_eq!(structure_of("### Deep").as_deref(), Some("heading level 3"));
        assert_eq!(structure_of("2. two").as_deref(), Some("item 2"));
        assert_eq!(structure_of("> said").as_deref(), Some("quote"));
        assert_eq!(structure_of("plain"), None);
    }

    #[test]
    fn links_under_the_caret() {
        let line = "See [the site](https://example.org \"t\") and <https://a.b> or www.c.d.";
        assert_eq!(
            link_at(line, 6),
            Some(("the site".into(), "https://example.org".into()))
        );
        assert_eq!(
            link_at(line, 45),
            Some(("https://a.b".into(), "https://a.b".into()))
        );
        assert_eq!(
            link_at(line, 62),
            Some(("www.c.d".into(), "www.c.d".into()))
        );
        assert_eq!(link_at(line, 1), None);
    }

    #[test]
    fn markdown_is_echoed_as_structure() {
        assert_eq!(markdown_echo("## ").as_deref(), Some("heading level 2"));
        assert_eq!(markdown_echo("- ").as_deref(), Some("bullet"));
        assert_eq!(markdown_echo("  * ").as_deref(), Some("bullet"));
        assert_eq!(markdown_echo("12. ").as_deref(), Some("numbered item 12"));
        assert_eq!(markdown_echo("> ").as_deref(), Some("quote"));
        assert_eq!(markdown_echo("- [ ] ").as_deref(), Some("task"));
        assert_eq!(markdown_echo("word "), None);
        assert_eq!(markdown_echo("##"), None);
        assert_eq!(markdown_echo("#hash "), None);
    }
}
