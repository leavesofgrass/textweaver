//! Small queries over a document built only on the text crate's public API
//! (`units`, `navigate`, the rope), so they keep working when Agent A replaces
//! the unit rules.

use textweaver_core::{CharPos, CharRange, Direction, Unit};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_text::units::unit_at;
use textweaver_text::{Document, NavOptions, navigate};

/// Number of lines, not counting the empty line after a final newline.
pub fn line_count(doc: &Document) -> usize {
    let rope = doc.text();
    let n = rope.len_lines();
    if n > 1 && rope.len_chars() > 0 && rope.char(rope.len_chars() - 1) == '\n' {
        n - 1
    } else {
        n.max(1)
    }
}

/// The 0-based line containing `pos` (clamped to the last line).
pub fn line_of(doc: &Document, pos: CharPos) -> usize {
    let rope = doc.text();
    let pos = pos.0.min(rope.len_chars());
    rope.char_to_line(pos).min(line_count(doc) - 1)
}

/// The chars of line `line`, without its newline (clamped to the last line).
pub fn line_range(doc: &Document, line: usize) -> CharRange {
    let rope = doc.text();
    let line = line.min(line_count(doc) - 1);
    let start = rope.line_to_char(line);
    let slice = rope.line(line);
    let mut len = slice.len_chars();
    while len > 0 && matches!(slice.char(len - 1), '\n' | '\r') {
        len -= 1;
    }
    CharRange::new(start, start + len)
}

/// The text of line `line`, without its newline.
pub fn line_text(doc: &Document, line: usize) -> String {
    doc.slice(line_range(doc, line))
}

/// True when the range holds nothing but whitespace.
pub fn is_blank(doc: &Document, range: CharRange) -> bool {
    doc.text()
        .slice(range.clamp_to(doc.len_chars()).to_range())
        .chars()
        .all(char::is_whitespace)
}

/// Floored percentage of `pos` through the document (Star's rule).
pub fn percent(doc: &Document, pos: CharPos) -> u8 {
    let len = doc.len_chars().max(1);
    u8::try_from((pos.0.min(len) * 100) / len).unwrap_or(100)
}

/// The word containing `pos`, if `pos` is inside a word.
pub fn word_containing(doc: &Document, pos: CharPos) -> Option<CharRange> {
    unit_at(doc, pos, Unit::Word).filter(|r| r.contains(pos))
}

/// The first word starting at or after `pos`, else the last word, else
/// `pos` clamped (Star's restore rule, used for positions and bookmarks).
pub fn first_word_at_or_after(doc: &Document, pos: CharPos) -> CharPos {
    let pos = pos.clamp_to(doc.len_chars());
    let found = if pos.0 == 0 {
        unit_at(doc, pos, Unit::Word).map(|r| r.start)
    } else {
        navigate(
            doc,
            CharPos(pos.0 - 1),
            Unit::Word,
            Direction::Forward,
            NavOptions { wrap: false },
        )
        .map(|t| t.range.start)
    };
    found.or_else(|| last_start(doc, Unit::Word)).unwrap_or(pos)
}

/// Start of the last `unit` in the document.
pub fn last_start(doc: &Document, unit: Unit) -> Option<CharPos> {
    navigate(
        doc,
        CharPos(doc.len_chars() + 1),
        unit,
        Direction::Backward,
        NavOptions { wrap: false },
    )
    .map(|t| t.range.start)
}

/// The start of the word the reader is on: the word containing `pos`, or
/// `pos` itself between words.
pub fn word_start(doc: &Document, pos: CharPos) -> CharPos {
    word_containing(doc, pos).map_or(pos, |r| r.start)
}

/// How many words start in `from..to`, counting at most `limit`.
pub fn words_between(doc: &Document, from: CharPos, to: CharPos, limit: usize) -> usize {
    let mut n = 0;
    let mut at = to;
    while n < limit {
        match navigate(
            doc,
            at,
            Unit::Word,
            Direction::Backward,
            NavOptions { wrap: false },
        ) {
            Some(t) if t.range.start >= from => {
                n += 1;
                at = t.range.start;
            }
            _ => break,
        }
    }
    n
}

/// Up to `words` words of `range`, followed by an ellipsis when cut, with
/// whitespace collapsed: a preview that reads well aloud.
pub fn preview(doc: &Document, range: CharRange, words: usize) -> String {
    let text = doc.slice(range);
    let all: Vec<&str> = text.split_whitespace().collect();
    if all.len() <= words {
        all.join(" ")
    } else {
        format!("{}…", all[..words].join(" "))
    }
}

/// The anchor for a saved position: the text starting at `pos`
/// ([`textweaver_store::Anchor`]), kept with positions and bookmarks so they
/// can be found again after outside edits.
pub fn anchor_at(doc: &Document, pos: CharPos) -> textweaver_store::Anchor {
    let rope = doc.text();
    let start = pos.0.min(rope.len_chars());
    let end = (start + textweaver_store::Anchor::CONTEXT_CHARS).min(rope.len_chars());
    textweaver_store::Anchor::from_text_at(rope.slice(start..end).chars())
}

/// What is said when a selection grows or shrinks by `text` (`what` is
/// "selected" or "unselected"): the text itself, a character's name for
/// white space, or above
/// [`SUMMARY_THRESHOLD`](textweaver_editor::echo::SUMMARY_THRESHOLD)
/// characters a summary ("3,412 characters selected, from ... to ...").
pub fn selection_change_message(text: &str, what: &str) -> String {
    if let Some(summary) = textweaver_editor::echo::summarize(text, what) {
        return summary;
    }
    format!("{} {what}", spoken_fragment(text))
}

/// Words kept at each end of a long text's summary.
const SUMMARY_WORDS: usize = 4;

/// A long text's spoken summary in the language of `c`, instead of the
/// text: "3,412 characters selected, from The first words to the last
/// words" (as [`textweaver_editor::echo::summarize`] says it in English).
/// `change` is what happened to the text: `selected`, `unselected`,
/// `deleted`, `copied`, or `cut`. `None` for text of
/// [`SUMMARY_THRESHOLD`](textweaver_editor::echo::SUMMARY_THRESHOLD)
/// characters or fewer, which is read as it is.
pub fn summary_text(c: &Catalog, text: &str, change: &str) -> Option<String> {
    let n = text.chars().count();
    if n <= textweaver_editor::echo::SUMMARY_THRESHOLD {
        return None;
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let count = textweaver_editor::echo::thousands(n);
    if words.len() <= SUMMARY_WORDS {
        // A few very long tokens (a URL, a base64 blob): the count alone.
        return Some(c.fmt("text-summary", &args!["count" => count, "change" => change]));
    }
    let first = words[..SUMMARY_WORDS].join(" ");
    let last = words[words.len() - SUMMARY_WORDS..].join(" ");
    Some(c.fmt(
        "text-summary-range",
        &args!["count" => count, "change" => change, "first" => first, "last" => last],
    ))
}

/// [`selection_change_message`] in the language of `c`: `selected` is
/// true when the selection grew by `text`, false when it shrank.
pub fn selection_change_text(c: &Catalog, text: &str, selected: bool) -> String {
    let change = if selected { "selected" } else { "unselected" };
    if let Some(summary) = summary_text(c, text, change) {
        return summary;
    }
    c.fmt(
        "text-selection-change",
        &args![
            "text" => spoken_fragment_text(c, text),
            "change" => if selected { "selected" } else { "unselected" }
        ],
    )
}

/// A short piece of text as spoken: trimmed, or the name of its first
/// character when it is only white space ("space", "new line").
pub fn spoken_fragment(text: &str) -> String {
    spoken_fragment_text(&Catalog::english(), text)
}

/// [`spoken_fragment`] in the language of `c`.
pub fn spoken_fragment_text(c: &Catalog, text: &str) -> String {
    if text.trim().is_empty() {
        char_name_text(c, text.chars().next().unwrap_or(' '))
    } else {
        text.trim().to_owned()
    }
}

/// A spoken name for a character read on its own.
pub fn char_name(c: char) -> String {
    char_name_text(&Catalog::english(), c)
}

/// [`char_name`] in the language of `c`: white space by name ("space",
/// "new line"), any other character as itself.
pub fn char_name_text(c: &Catalog, ch: char) -> String {
    let key = match ch {
        ' ' => "space",
        '\n' => "new-line",
        '\t' => "tab",
        '\u{a0}' => "no-break-space",
        ch if ch.is_whitespace() => "white-space",
        ch => return ch.to_string(),
    };
    c.fmt("text-char-name", &args!["name" => key])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_ignore_trailing_newline() {
        let d = Document::from_plain_text("ab\n\ncd\n");
        assert_eq!(line_count(&d), 3);
        assert_eq!(line_text(&d, 0), "ab");
        assert_eq!(line_text(&d, 1), "");
        assert_eq!(line_text(&d, 2), "cd");
        assert_eq!(line_of(&d, CharPos(7)), 2);
        assert_eq!(line_count(&Document::from_plain_text("")), 1);
    }

    #[test]
    fn restore_rule_is_first_word_at_or_after() {
        let d = Document::from_plain_text("one two three");
        assert_eq!(first_word_at_or_after(&d, CharPos(0)), CharPos(0));
        assert_eq!(first_word_at_or_after(&d, CharPos(4)), CharPos(4));
        assert_eq!(first_word_at_or_after(&d, CharPos(5)), CharPos(8));
        assert_eq!(first_word_at_or_after(&d, CharPos(99)), CharPos(8));
    }

    #[test]
    fn counts_words_backwards() {
        let d = Document::from_plain_text("one two three four five");
        assert_eq!(words_between(&d, CharPos(0), CharPos(19), 10), 4);
        assert_eq!(words_between(&d, CharPos(4), CharPos(19), 2), 2);
    }

    #[test]
    fn preview_cuts_long_text() {
        let d = Document::from_plain_text("a b  c\nd e f g");
        assert_eq!(preview(&d, CharRange::new(0, 14), 3), "a b c…");
        assert_eq!(percent(&d, CharPos(7)), 50);
    }
}
