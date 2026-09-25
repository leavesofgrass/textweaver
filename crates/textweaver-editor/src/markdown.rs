//! Markdown formatting commands as pure functions.
//!
//! Each command takes the text and the selection and returns the edits to
//! apply (as one undo step) and the selection afterwards. Agent C ports
//! Star's exact semantics and the 56 authoring tests
//! (docs/star-parity.md, "Authoring"); Phase 0 wraps inline styles and
//! prefixes lines, nothing more.

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, Edit};

use crate::Selection;

/// A formatting command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownOp {
    /// `**bold**`
    Bold,
    /// `*italic*`
    Italic,
    /// `` `code` ``
    InlineCode,
    /// `~~strikethrough~~`
    Strikethrough,
    /// `# ` .. `###### ` at the start of each selected line.
    Heading(u8),
    /// `- ` list items.
    BulletList,
    /// `1. ` numbered items.
    NumberedList,
    /// `> ` block quote.
    Quote,
    /// A fenced code block around the selected lines.
    CodeBlock,
    /// `[text](url)`
    Link,
}

/// The edits for one command and the selection afterwards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formatted {
    /// Edits, each in the coordinates left by the previous one.
    pub edits: Vec<Edit>,
    /// Selection after applying the edits.
    pub selection: Selection,
}

fn wrap(sel: CharRange, open: &str, close: &str) -> Formatted {
    let o = open.chars().count();
    Formatted {
        edits: vec![
            Edit::insert(sel.start, open),
            Edit::insert(sel.end.saturating_add(o), close),
        ],
        selection: Selection {
            anchor: sel.start.saturating_add(o),
            head: sel.end.saturating_add(o),
        },
    }
}

fn prefix_lines(text: &Rope, sel: CharRange, prefix: impl Fn(usize) -> String) -> Formatted {
    let first = text.char_to_line(sel.start.0.min(text.len_chars()));
    let last = text.char_to_line(sel.end.0.min(text.len_chars()));
    let mut edits = Vec::new();
    let mut added = 0usize;
    for (i, line) in (first..=last).enumerate() {
        let at = text.line_to_char(line) + added;
        let p = prefix(i);
        added += p.chars().count();
        edits.push(Edit::insert(at, p));
    }
    let start = CharPos(text.line_to_char(first));
    let end = CharPos(sel.end.0 + added);
    Formatted {
        edits,
        selection: Selection {
            anchor: start,
            head: end,
        },
    }
}

/// The edits that apply `op` to `sel` in `text`.
pub fn apply(text: &Rope, sel: Selection, op: MarkdownOp) -> Formatted {
    let r = sel.range();
    match op {
        MarkdownOp::Bold => wrap(r, "**", "**"),
        MarkdownOp::Italic => wrap(r, "*", "*"),
        MarkdownOp::InlineCode => wrap(r, "`", "`"),
        MarkdownOp::Strikethrough => wrap(r, "~~", "~~"),
        MarkdownOp::Link => wrap(r, "[", "]()"),
        MarkdownOp::Heading(level) => {
            let hashes = "#".repeat(usize::from(level.clamp(1, 6)));
            prefix_lines(text, r, |_| format!("{hashes} "))
        }
        MarkdownOp::BulletList => prefix_lines(text, r, |_| "- ".to_owned()),
        MarkdownOp::NumberedList => prefix_lines(text, r, |i| format!("{}. ", i + 1)),
        MarkdownOp::Quote => prefix_lines(text, r, |_| "> ".to_owned()),
        MarkdownOp::CodeBlock => wrap(r, "```\n", "\n```"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Editor;

    #[test]
    fn bold_is_one_undo_step() {
        let mut ed = Editor::new("make this bold");
        let sel = Selection {
            anchor: CharPos(10),
            head: CharPos(14),
        };
        let f = apply(ed.text(), sel, MarkdownOp::Bold);
        ed.apply_group(f.edits).unwrap();
        assert_eq!(ed.text().to_string(), "make this **bold**");
        ed.undo();
        assert_eq!(ed.text().to_string(), "make this bold");
    }

    #[test]
    fn numbered_list_prefixes_each_line() {
        let text = Rope::from_str("a\nb");
        let f = apply(
            &text,
            Selection {
                anchor: CharPos(0),
                head: CharPos(3),
            },
            MarkdownOp::NumberedList,
        );
        let mut ed = Editor::new("a\nb");
        ed.apply_group(f.edits).unwrap();
        assert_eq!(ed.text().to_string(), "1. a\n2. b");
    }
}
