use serde::{Deserialize, Serialize};

/// Kinds of structural and inline markers a loader can attach to a document.
///
/// Markers annotate ranges of the canonical text; navigation by marker kind
/// ("next heading", "next table") is how structure is exposed to the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkerKind {
    /// A heading; `Marker::level` holds 1 through 6.
    Heading,
    /// A paragraph.
    Paragraph,
    /// One item of a list; `Marker::level` holds the nesting depth, from 1.
    ListItem,
    /// A whole list.
    List,
    /// A whole table.
    Table,
    /// One table row (one line of canonical text).
    TableRow,
    /// One table cell within a row.
    TableCell,
    /// A hyperlink; `Marker::reference` holds the target.
    Link,
    /// An image, represented in the text by its alt text.
    Image,
    /// A code block or inline code span.
    Code,
    /// A block quote.
    Quote,
    /// A page boundary from a paginated source.
    PageBreak,
    /// A chapter or section boundary (EPUB spine item, DOCX section).
    SectionBreak,
    /// Bold text.
    Bold,
    /// Italic text.
    Italic,
    /// Underlined text.
    Underline,
    /// A footnote body or reference; `Marker::reference` holds the id.
    Footnote,
    /// Struck-through text (Markdown `~~text~~`).
    Strikethrough,
    /// A horizontal rule (thematic break): an empty marker where the block
    /// after it starts.
    Rule,
    /// Math: the text is the source with its delimiters (`$…$`, or
    /// `$$…$$` for display math at level 1), which speech reads as math and
    /// writers typeset.
    Math,
}

impl MarkerKind {
    /// All marker kinds, in declaration order.
    pub const ALL: [MarkerKind; 20] = [
        MarkerKind::Heading,
        MarkerKind::Paragraph,
        MarkerKind::ListItem,
        MarkerKind::List,
        MarkerKind::Table,
        MarkerKind::TableRow,
        MarkerKind::TableCell,
        MarkerKind::Link,
        MarkerKind::Image,
        MarkerKind::Code,
        MarkerKind::Quote,
        MarkerKind::PageBreak,
        MarkerKind::SectionBreak,
        MarkerKind::Bold,
        MarkerKind::Italic,
        MarkerKind::Underline,
        MarkerKind::Footnote,
        MarkerKind::Strikethrough,
        MarkerKind::Rule,
        MarkerKind::Math,
    ];

    /// The spoken, user-facing name ("heading", "list item").
    pub fn spoken_name(self) -> &'static str {
        match self {
            MarkerKind::Heading => "heading",
            MarkerKind::Paragraph => "paragraph",
            MarkerKind::ListItem => "list item",
            MarkerKind::List => "list",
            MarkerKind::Table => "table",
            MarkerKind::TableRow => "row",
            MarkerKind::TableCell => "cell",
            MarkerKind::Link => "link",
            MarkerKind::Image => "graphic",
            MarkerKind::Code => "code",
            MarkerKind::Quote => "block quote",
            MarkerKind::PageBreak => "page",
            MarkerKind::SectionBreak => "section",
            MarkerKind::Bold => "bold",
            MarkerKind::Italic => "italic",
            MarkerKind::Underline => "underline",
            MarkerKind::Footnote => "footnote",
            MarkerKind::Strikethrough => "strikethrough",
            MarkerKind::Rule => "separator",
            MarkerKind::Math => "math",
        }
    }

    /// True for inline formatting kinds that never define navigation stops of
    /// their own (bold, italic, underline, strikethrough).
    pub fn is_formatting(self) -> bool {
        matches!(
            self,
            MarkerKind::Bold
                | MarkerKind::Italic
                | MarkerKind::Underline
                | MarkerKind::Strikethrough
        )
    }
}

/// What a reading or navigation command moves by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "unit")]
pub enum Unit {
    /// An extended grapheme cluster: what a user perceives as one character.
    Grapheme,
    /// A word: a UAX #29 word segment that contains at least one alphanumeric char.
    Word,
    /// A sentence: UAX #29 sentence boundaries refined by an abbreviation list.
    Sentence,
    /// A line of canonical text (up to a `\n`).
    Line,
    /// A paragraph: text between blank lines, or a `Paragraph` marker.
    Paragraph,
    /// The whole document.
    Document,
    /// A structural marker, optionally restricted to one level
    /// (heading level, list depth).
    Marker {
        /// The marker kind to move between.
        kind: MarkerKind,
        /// Only stop at markers of this level, when set.
        level: Option<u8>,
    },
}

impl Unit {
    /// Shorthand for `Unit::Marker { kind, level: None }`.
    pub const fn marker(kind: MarkerKind) -> Self {
        Unit::Marker { kind, level: None }
    }

    /// Shorthand for a heading of one level.
    pub const fn heading(level: u8) -> Self {
        Unit::Marker {
            kind: MarkerKind::Heading,
            level: Some(level),
        }
    }

    /// The spoken, user-facing name of the unit ("sentence", "heading level 2").
    pub fn spoken_name(&self) -> String {
        match self {
            Unit::Grapheme => "character".to_owned(),
            Unit::Word => "word".to_owned(),
            Unit::Sentence => "sentence".to_owned(),
            Unit::Line => "line".to_owned(),
            Unit::Paragraph => "paragraph".to_owned(),
            Unit::Document => "document".to_owned(),
            Unit::Marker { kind, level: None } => kind.spoken_name().to_owned(),
            Unit::Marker {
                kind,
                level: Some(l),
            } => format!("{} level {l}", kind.spoken_name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spoken_names() {
        assert_eq!(Unit::heading(2).spoken_name(), "heading level 2");
        assert_eq!(
            Unit::marker(MarkerKind::ListItem).spoken_name(),
            "list item"
        );
        assert!(MarkerKind::Bold.is_formatting());
        assert!(!MarkerKind::Heading.is_formatting());
    }

    #[test]
    fn all_is_complete() {
        let mut v = MarkerKind::ALL.to_vec();
        v.sort();
        v.dedup();
        assert_eq!(v.len(), MarkerKind::ALL.len());
    }
}
