//! Obsidian syntax rewritten in the Markdown source before it is parsed,
//! shared by the reader ([`crate::markdown`]) and the HTML renderer
//! (`textweaver-render`), so the two agree:
//!
//! - **Comments** (`%%hidden%%`, on one line or across blocks) are left
//!   out ([`strip_comments`]). An unclosed `%%` runs to the end of the
//!   note, as in Obsidian. A line that held only a comment goes with it,
//!   so the text around it stays one paragraph.
//! - **Inline footnotes** (`^[a note written in place]`) become ordinary
//!   footnotes ([`inline_footnotes`]): a reference numbered after the
//!   note's own footnotes, with its definition after the text, so every
//!   footnote mode (deferred, inline, skipped) reads them alike.
//!
//! Neither touches front matter, fenced code blocks, or inline code
//! spans. Indented code blocks are not recognized (Obsidian notes rarely
//! use them), so a `%%` in one is treated as a comment mark.
//!
//! Custom task statuses (`- [/]`, `- [-]`, `- [>]`) are named in words by
//! [`task_status_words`].

/// Most inline footnotes rewritten in one note; later ones stay as text.
const MAX_INLINE_NOTES: usize = 10_000;

/// Longest inline footnote looked for, in bytes: a `^[` with no `]` this
/// close is text, so a hostile note cannot make the search quadratic.
const MAX_NOTE_BYTES: usize = 8 << 10;

/// Where the front matter ends (`---` to `---` or `...` at the very
/// start), or 0 when there is none.
fn front_matter_end(src: &str) -> usize {
    let first = src.split_inclusive('\n').next().unwrap_or("");
    if first.trim_end() != "---" {
        return 0;
    }
    let mut at = first.len();
    for line in src[at..].split_inclusive('\n') {
        at += line.len();
        let t = line.trim_end();
        if t == "---" || t == "..." {
            return at;
        }
    }
    // An unclosed block is not front matter.
    0
}

/// A code fence opening at the start of `line` (any indentation): its
/// character and length.
fn fence_of(line: &str) -> Option<(u8, usize)> {
    let t = line.trim_start();
    let ch = *t.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = t.bytes().take_while(|&b| b == ch).count();
    (len >= 3).then_some((ch, len))
}

/// The end of the line starting at `at` (after its `\n`).
fn line_end(src: &str, at: usize) -> usize {
    src[at..].find('\n').map_or(src.len(), |i| at + i + 1)
}

/// If a fenced code block opens on the line at `at`, where it ends (after
/// its closing fence, or at the end of the text when it never closes).
fn fenced_block_end(src: &str, at: usize) -> Option<usize> {
    let first = line_end(src, at);
    let (ch, len) = fence_of(&src[at..first])?;
    // A backtick fence's info string may not hold a backtick.
    if ch == b'`' && src[at..first].trim_start()[len..].contains('`') {
        return None;
    }
    let mut i = first;
    while i < src.len() {
        let end = line_end(src, i);
        let line = &src[i..end];
        if let Some((c, n)) = fence_of(line)
            && c == ch
            && n >= len
            && line.trim_start()[n..].trim().is_empty()
        {
            return Some(end);
        }
        i = end;
    }
    Some(src.len())
}

/// The length of the run of `b` starting at `at`.
fn run_len(bytes: &[u8], at: usize, b: u8) -> usize {
    bytes[at..].iter().take_while(|&&x| x == b).count()
}

/// True when the line starting at `at` is blank (white space only).
fn blank_line_at(src: &str, at: usize) -> bool {
    src[at..line_end(src, at)].trim().is_empty()
}

/// If the backtick run at `at` opens an inline code span (a run of the
/// same length closes it before the paragraph ends), where the span ends.
fn code_span_end(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let n = run_len(bytes, at, b'`');
    let mut j = at + n;
    while j < bytes.len() {
        match bytes[j] {
            b'`' => {
                let m = run_len(bytes, j, b'`');
                if m == n {
                    return Some(j + m);
                }
                j += m;
            }
            b'\n' => {
                if j + 1 >= bytes.len() || blank_line_at(src, j + 1) {
                    return None;
                }
                j += 1;
            }
            _ => j += 1,
        }
    }
    None
}

/// What the scanner found at a place in the source.
enum Code {
    /// Front matter, a fenced code block, or an inline code span, which
    /// ends here: copied as it is.
    Until(usize),
    /// A run of backticks that opens nothing: copied as text.
    Ticks(usize),
}

/// Code that starts at `at`: a fenced block when `at` starts a line, or an
/// inline code span.
fn code_at(src: &str, at: usize, line_start: bool) -> Option<Code> {
    if line_start && let Some(end) = fenced_block_end(src, at) {
        return Some(Code::Until(end));
    }
    let bytes = src.as_bytes();
    if bytes[at] == b'`' {
        return Some(match code_span_end(src, at) {
            Some(end) => Code::Until(end),
            None => Code::Ticks(run_len(bytes, at, b'`')),
        });
    }
    None
}

/// The source without Obsidian comments (`%%...%%`), or `None` when it
/// has none outside code. Comments may span lines and blocks; one left
/// open runs to the end. Line breaks inside a comment are kept, except
/// that a line holding nothing but comment is left out whole.
pub fn strip_comments(src: &str) -> Option<String> {
    if !src.contains("%%") {
        return None;
    }
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(src.len());
    // The output line being built, and whether a comment touched it.
    let mut line: Vec<u8> = Vec::new();
    let mut touched = false;
    let mut found = false;
    let mut in_comment = false;
    let mut i = front_matter_end(src);
    out.extend_from_slice(&bytes[..i]);
    let mut line_start = true;
    let flush = |out: &mut Vec<u8>, line: &mut Vec<u8>, touched: &mut bool, newline: bool| {
        let blank = line.iter().all(u8::is_ascii_whitespace);
        if !(*touched && blank) {
            out.append(line);
            if newline {
                out.push(b'\n');
            }
        }
        line.clear();
        *touched = false;
    };
    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            touched = true;
            if b == b'%' && bytes.get(i + 1) == Some(&b'%') {
                in_comment = false;
                i += 2;
            } else if b == b'\n' {
                flush(&mut out, &mut line, &mut touched, true);
                line_start = true;
                i += 1;
            } else {
                i += 1;
            }
            continue;
        }
        if let Some(code) = code_at(src, i, line_start) {
            let end = match code {
                Code::Until(end) => end,
                Code::Ticks(n) => i + n,
            };
            let piece = &bytes[i..end];
            // A fenced block ends with its line: flush the lines it holds.
            match piece.iter().rposition(|&x| x == b'\n') {
                Some(last) => {
                    line.extend_from_slice(&piece[..last]);
                    flush(&mut out, &mut line, &mut touched, true);
                    line.extend_from_slice(&piece[last + 1..]);
                    line_start = last + 1 == piece.len();
                }
                None => {
                    line.extend_from_slice(piece);
                    line_start = false;
                }
            }
            i = end;
            continue;
        }
        line_start = false;
        match b {
            b'%' if bytes.get(i + 1) == Some(&b'%') => {
                in_comment = true;
                found = true;
                touched = true;
                i += 2;
            }
            // `\%` is a percent sign, never half of a comment mark.
            b'\\' if bytes.get(i + 1) == Some(&b'%') => {
                line.extend_from_slice(b"\\%");
                i += 2;
            }
            b'\n' => {
                flush(&mut out, &mut line, &mut touched, true);
                line_start = true;
                i += 1;
            }
            _ => {
                line.push(b);
                i += 1;
            }
        }
    }
    flush(&mut out, &mut line, &mut touched, false);
    // Only ASCII bytes were dropped or added, so the text is still UTF-8.
    found.then(|| String::from_utf8(out).unwrap_or_else(|_| src.to_owned()))
}

/// Footnote labels the note uses (`[^label]`), to number inline
/// footnotes after them.
fn used_labels(src: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("[^") {
        rest = &rest[i + 2..];
        if let Some(end) = rest.find(']')
            && end > 0
            && end <= 64
            && !rest[..end].contains(char::is_whitespace)
        {
            out.push(&rest[..end]);
        }
    }
    out
}

/// The end of an inline footnote's text: the `]` closing the `[` at `at`
/// (brackets inside nest), before the paragraph ends, skipping code
/// spans.
fn note_end(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    let mut j = at;
    let stop = bytes.len().min(at.saturating_add(MAX_NOTE_BYTES));
    while j < stop {
        match bytes[j] {
            b'\\' => j += 2,
            b'`' => match code_span_end(src, j) {
                Some(end) => j = end,
                None => j += run_len(bytes, j, b'`'),
            },
            b'[' => {
                depth += 1;
                j += 1;
            }
            b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(j);
                }
                j += 1;
            }
            b'\n' if j + 1 >= bytes.len() || blank_line_at(src, j + 1) => return None,
            _ => j += 1,
        }
    }
    None
}

/// The source with each inline footnote (`^[text]`) outside code turned
/// into a footnote reference (`[^3]`), numbered after the labels the note
/// already uses, and its definition added after the text; `None` when the
/// note has none.
pub fn inline_footnotes(src: &str) -> Option<String> {
    if !src.contains("^[") {
        return None;
    }
    let used = used_labels(src);
    let mut next = 1usize;
    let mut label = || {
        while used.contains(&next.to_string().as_str()) {
            next += 1;
        }
        next += 1;
        (next - 1).to_string()
    };
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len() + 64);
    let mut notes: Vec<(String, String)> = Vec::new();
    let mut i = front_matter_end(src);
    out.push_str(&src[..i]);
    let mut from = i;
    let mut line_start = true;
    let mut open_fence = false;
    while i < bytes.len() {
        if let Some(code) = code_at(src, i, line_start) {
            let end = match code {
                Code::Until(end) => end,
                Code::Ticks(n) => i + n,
            };
            open_fence = end == src.len() && fenced_block_end(src, i).is_some();
            line_start = bytes[..end].last() == Some(&b'\n');
            i = end;
            continue;
        }
        match bytes[i] {
            b'\\' => {
                i += 2;
                line_start = false;
                continue;
            }
            b'\n' => {
                line_start = true;
                i += 1;
                continue;
            }
            b'^' if bytes.get(i + 1) == Some(&b'[') && notes.len() < MAX_INLINE_NOTES => {
                if let Some(close) = note_end(src, i + 1) {
                    let text = src[i + 2..close]
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !text.is_empty() {
                        let l = label();
                        out.push_str(&src[from..i]);
                        out.push_str(&format!("[^{l}]"));
                        notes.push((l, text));
                        i = close + 1;
                        from = i;
                        line_start = false;
                        continue;
                    }
                }
            }
            _ => {}
        }
        line_start = false;
        i += 1;
    }
    // A note that ends inside an open code fence would swallow the
    // definitions: leave it as it is.
    if notes.is_empty() || open_fence {
        return None;
    }
    out.push_str(&src[from.min(src.len())..]);
    out.push_str("\n\n");
    for (l, text) in notes {
        out.push_str(&format!("[^{l}]: {text}\n\n"));
    }
    Some(out)
}

/// A task item's status in words, for the character between its brackets
/// (`- [/] Draft`): the statuses of Obsidian's Tasks plugin and popular
/// themes. `None` for a space and `x`, which the parser reads itself, and
/// for characters that cannot be a status.
pub fn task_status_words(c: char) -> Option<String> {
    let words = match c {
        ' ' | 'x' | 'X' => return None,
        '/' => "in progress",
        '-' => "cancelled",
        '>' => "forwarded",
        '<' => "scheduled",
        '!' => "important",
        '?' => "question",
        '*' => "starred",
        '"' => "quote",
        'i' => "information",
        'I' => "idea",
        'b' => "bookmark",
        'l' => "location",
        'S' => "savings",
        'p' => "pro",
        'c' => "con",
        'f' => "fire",
        'k' => "key",
        'w' => "win",
        'u' => "up",
        'd' => "down",
        c if c.is_alphanumeric() || c.is_ascii_punctuation() => {
            if matches!(c, '[' | ']' | '\\' | '`' | '&') {
                return None;
            }
            return Some(format!("marked {c}"));
        }
        _ => return None,
    };
    Some(words.to_owned())
}

/// A custom task status at the start of a list item whose source starts
/// at `at` (`- [/] Draft`, `1. [-] Old`): its words, and the byte offset
/// just after `]` and the space that follows it.
pub(crate) fn task_status_at(src: &str, at: usize) -> Option<(String, usize)> {
    let rest = src.get(at..)?;
    let line = rest.split('\n').next().unwrap_or(rest);
    let t = line.trim_start();
    let marker = if t.starts_with(['-', '*', '+']) {
        1
    } else {
        let digits = t.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || digits > 9 || !t[digits..].starts_with(['.', ')']) {
            return None;
        }
        digits + 1
    };
    let after = &t[marker..];
    let spaces = after.len() - after.trim_start().len();
    if spaces == 0 {
        return None;
    }
    let mut chars = after[spaces..].chars();
    if chars.next() != Some('[') {
        return None;
    }
    let c = chars.next()?;
    if chars.next() != Some(']') {
        return None;
    }
    let tail = chars.as_str();
    if !(tail.is_empty() || tail.starts_with([' ', '\t', '\r'])) {
        return None;
    }
    let words = task_status_words(c)?;
    let lead = line.len() - t.len();
    let end = at + lead + marker + spaces + 2 + c.len_utf8();
    let end = end + usize::from(tail.starts_with([' ', '\t']));
    Some((words, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_inline_spanning_and_unclosed() {
        assert_eq!(strip_comments("no marks"), None);
        assert_eq!(
            strip_comments("a %%x%% b").as_deref(),
            Some("a  b"),
            "inline"
        );
        assert_eq!(
            strip_comments("one\n%%whole line%%\ntwo\n").as_deref(),
            Some("one\ntwo\n"),
            "a comment-only line goes, the paragraph stays one"
        );
        assert_eq!(
            strip_comments("A\n\n%%\nsecret\n\n# head\n%%\n\nB\n").as_deref(),
            Some("A\n\n\nB\n"),
            "across blocks"
        );
        assert_eq!(
            strip_comments("A %%open\n\nnever closed\n").as_deref(),
            Some("A \n"),
            "unclosed runs to the end"
        );
    }

    #[test]
    fn comments_leave_code_and_front_matter_alone() {
        let src = "---\nnote: \"%%\"\n---\nKeep `%%a%%` and ``x %% y`` here %%gone%%.\n\n```\n%%fenced%%\n```\n\n~~~~\n%%\n~~~~\nEnd %%x%%\n";
        let out = strip_comments(src).unwrap();
        assert_eq!(
            out,
            "---\nnote: \"%%\"\n---\nKeep `%%a%%` and ``x %% y`` here .\n\n```\n%%fenced%%\n```\n\n~~~~\n%%\n~~~~\nEnd \n"
        );
        // An unmatched backtick is text, not the start of code.
        assert_eq!(
            strip_comments("don`t %%hide%% me").as_deref(),
            Some("don`t  me")
        );
        // A fence inside a comment is part of the comment.
        assert_eq!(
            strip_comments("a\n%%\n```\ncode\n```\n%%\nb").as_deref(),
            Some("a\nb")
        );
        assert_eq!(strip_comments("100\\%% sure").as_deref(), None);
    }

    #[test]
    fn inline_footnotes_become_numbered_footnotes() {
        assert_eq!(inline_footnotes("plain [link](x) ^2"), None);
        let out = inline_footnotes(
            "A^[first note] and B.[^1]\n\n[^1]: Old.\n\nC ^[with [nested] and `]` code].",
        )
        .unwrap();
        assert_eq!(
            out,
            "A[^2] and B.[^1]\n\n[^1]: Old.\n\nC [^3].\n\n[^2]: first note\n\n[^3]: with [nested] and `]` code\n\n"
        );
        // In code, unclosed, empty, or across a blank line: left alone.
        assert_eq!(inline_footnotes("`^[x]` and ^[ ] and ^[open"), None);
        assert_eq!(inline_footnotes("^[a\n\nb]"), None);
        assert_eq!(inline_footnotes("x\n\n```\n^[in code]\n```\n"), None);
        // A note that ends in an open fence is not rewritten.
        assert_eq!(inline_footnotes("^[n]\n\n```\nopen"), None);
    }

    #[test]
    fn task_statuses_in_words() {
        assert_eq!(
            task_status_at("- [/] Draft", 0),
            Some(("in progress".into(), 6))
        );
        assert_eq!(
            task_status_at("  12. [-] Old", 0),
            Some(("cancelled".into(), 10))
        );
        assert_eq!(task_status_at("* [>]", 0), Some(("forwarded".into(), 5)));
        assert_eq!(task_status_at("- [q] x", 0), Some(("marked q".into(), 6)));
        assert_eq!(task_status_at("- [ ] plain", 0), None);
        assert_eq!(task_status_at("- [x] done", 0), None);
        assert_eq!(task_status_at("- [/]text", 0), None);
        assert_eq!(task_status_at("- [ab] no", 0), None);
        assert_eq!(task_status_at("-[/] no", 0), None);
    }
}
