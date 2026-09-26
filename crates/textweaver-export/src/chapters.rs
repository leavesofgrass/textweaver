//! Chapters from a document's structure, placed on the audio's timeline,
//! and the ffmpeg metadata that carries them into an M4B (or MP3).
//!
//! Star (`star/audiobook.py`) made a chapter of every Markdown heading and
//! titled any text before the first heading after the document. Here the
//! loaders' markers say where chapters start: every `SectionBreak` (an EPUB
//! spine item, a DOCX section) and every `Heading` up to
//! [`ChapterOptions::max_heading_level`] (default 6, Star's "every
//! heading"). Starts at the same place merge (a section that opens with a
//! heading is one chapter, titled by the heading). A chapter's time is the
//! start of the first sentence that reaches its position.

use serde::Serialize;
use textweaver_core::{CharPos, MarkerKind};
use textweaver_text::Document;

use crate::timeline::{Chapter, TimedSentence, collapse};

/// Which markers start chapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ChapterOptions {
    /// Headings at this level or above (1 is the top) start chapters; 0
    /// uses section breaks only.
    pub max_heading_level: u8,
}

impl Default for ChapterOptions {
    fn default() -> Self {
        ChapterOptions {
            max_heading_level: 6,
        }
    }
}

/// Longest chapter title kept, in chars.
const MAX_TITLE: usize = 200;

fn title_of(doc: &Document, range: textweaver_core::CharRange) -> String {
    let (t, _) = collapse(&doc.slice(range));
    t.chars().take(MAX_TITLE).collect()
}

/// Chapter starts in the document, in order: `(position, title)`.
pub fn starts(doc: &Document, opts: &ChapterOptions) -> Vec<(CharPos, String)> {
    let mut out: Vec<(CharPos, String, bool)> = Vec::new(); // (pos, title, from heading)
    for m in doc.markers() {
        match m.kind {
            MarkerKind::Heading if m.level >= 1 && m.level <= opts.max_heading_level => {
                out.push((m.range.start, title_of(doc, m.range), true));
            }
            MarkerKind::SectionBreak => {
                let title = m.label.clone().unwrap_or_default();
                out.push((m.range.start, title, false));
            }
            _ => {}
        }
    }
    out.sort_by_key(|(pos, _, heading)| (*pos, !*heading));
    let mut merged: Vec<(CharPos, String)> = Vec::new();
    for (pos, title, _) in out {
        match merged.last_mut() {
            Some((p, t)) if *p == pos => {
                if t.is_empty() {
                    *t = title;
                }
            }
            _ => merged.push((pos, title)),
        }
    }
    merged
}

/// Places chapters on the timeline. Text before the first chapter start
/// becomes a leading chapter titled after the document (Star's rule);
/// untitled chapters are numbered; chapters without audio are dropped.
pub fn place(
    doc: &Document,
    sentences: &[TimedSentence],
    duration_ms: u64,
    opts: &ChapterOptions,
) -> Vec<Chapter> {
    if duration_ms == 0 || sentences.is_empty() {
        return Vec::new();
    }
    let doc_title = doc
        .meta
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "Audiobook".to_owned());
    // The time at which reading reaches `pos`.
    let time_of = |pos: CharPos| {
        sentences
            .iter()
            .find(|s| s.source.is_some_and(|r| r.end > pos))
            .map(|s| s.start_ms)
    };
    let mut timed: Vec<(u64, String, CharPos)> = Vec::new();
    for (pos, title) in starts(doc, opts) {
        let Some(t) = time_of(pos) else { continue };
        timed.push((t, title, pos));
    }
    if timed.first().is_none_or(|(t, _, _)| *t > 0) {
        timed.insert(0, (0, doc_title, CharPos::ZERO));
    }
    let mut chapters: Vec<Chapter> = Vec::new();
    for (i, (start, title, pos)) in timed.iter().enumerate() {
        let end = timed.get(i + 1).map_or(duration_ms, |(t, _, _)| *t);
        if end <= *start {
            continue; // no audio of its own (two headings in one sentence)
        }
        let title = if title.trim().is_empty() {
            format!("Chapter {}", chapters.len() + 1)
        } else {
            title.clone()
        };
        chapters.push(Chapter {
            title,
            source_start: *pos,
            start_ms: *start,
            end_ms: end,
        });
    }
    chapters
}

/// Escapes a value for ffmpeg's metadata file: `=`, `;`, `#`, `\`, and
/// newlines get a backslash (newlines inside titles become spaces first).
pub fn escape_metadata(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\n' | '\r' => out.push(' '),
            '=' | ';' | '#' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// An ffmpeg `;FFMETADATA1` file with the title, author, and one
/// `[CHAPTER]` per chapter (times in ms), as Star's
/// `build_chapters_metadata` wrote.
pub fn ffmetadata(title: Option<&str>, author: Option<&str>, chapters: &[Chapter]) -> String {
    let mut lines = vec![";FFMETADATA1".to_owned()];
    if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
        lines.push(format!("title={}", escape_metadata(t)));
        lines.push(format!("album={}", escape_metadata(t)));
    }
    if let Some(a) = author.filter(|a| !a.trim().is_empty()) {
        lines.push(format!("artist={}", escape_metadata(a)));
    }
    lines.push("genre=Audiobook".to_owned());
    for c in chapters {
        lines.push("[CHAPTER]".to_owned());
        lines.push("TIMEBASE=1/1000".to_owned());
        lines.push(format!("START={}", c.start_ms));
        lines.push(format!("END={}", c.end_ms));
        lines.push(format!("title={}", escape_metadata(&c.title)));
    }
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use textweaver_core::CharRange;
    use textweaver_text::{DocumentData, DocumentMeta, Marker};

    use super::*;

    fn sentence(a: usize, b: usize, start: u64, end: u64) -> TimedSentence {
        TimedSentence {
            start_ms: start,
            end_ms: end,
            source: Some(CharRange::new(a, b)),
            text: String::new(),
            spoken: String::new(),
            words: Vec::new(),
        }
    }

    fn doc() -> Document {
        //          0         1         2         3
        //          0123456789012345678901234567890123456
        let text = "Preface.\nOne\nFirst body.\nTwo\nSecond.";
        let markers = vec![
            Marker::new(MarkerKind::Heading, CharRange::new(9, 12)).with_level(1),
            Marker::new(MarkerKind::SectionBreak, CharRange::new(25, 36)).with_label("Part B"),
            Marker::new(MarkerKind::Heading, CharRange::new(25, 28)).with_level(2),
        ];
        let meta = DocumentMeta {
            title: Some("My Book".into()),
            ..DocumentMeta::default()
        };
        Document::from(DocumentData {
            meta,
            text: text.into(),
            markers,
        })
    }

    #[test]
    fn starts_merge_sections_with_their_headings() {
        let d = doc();
        assert_eq!(
            starts(&d, &ChapterOptions::default()),
            [
                (CharPos(9), "One".to_owned()),
                (CharPos(25), "Two".to_owned())
            ]
        );
        let sections_only = ChapterOptions {
            max_heading_level: 0,
        };
        assert_eq!(
            starts(&d, &sections_only),
            [(CharPos(25), "Part B".to_owned())]
        );
    }

    #[test]
    fn chapters_are_placed_with_a_leading_preface() {
        let d = doc();
        let s = vec![
            sentence(0, 8, 0, 800),
            sentence(9, 12, 800, 1200),
            sentence(13, 24, 1200, 2000),
            sentence(25, 28, 2000, 2400),
            sentence(29, 36, 2400, 3000),
        ];
        let c = place(&d, &s, 3000, &ChapterOptions::default());
        let got: Vec<(&str, u64, u64)> = c
            .iter()
            .map(|c| (c.title.as_str(), c.start_ms, c.end_ms))
            .collect();
        assert_eq!(
            got,
            [("My Book", 0, 800), ("One", 800, 2000), ("Two", 2000, 3000)]
        );
        let meta = ffmetadata(Some("My Book"), Some("A; B"), &c);
        assert_eq!(
            meta,
            ";FFMETADATA1\ntitle=My Book\nalbum=My Book\nartist=A\\; B\ngenre=Audiobook\n\
             [CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=800\ntitle=My Book\n\
             [CHAPTER]\nTIMEBASE=1/1000\nSTART=800\nEND=2000\ntitle=One\n\
             [CHAPTER]\nTIMEBASE=1/1000\nSTART=2000\nEND=3000\ntitle=Two\n"
        );
    }

    #[test]
    fn a_document_without_structure_is_one_chapter() {
        let d = Document::from_plain_text("Just text.");
        let c = place(
            &d,
            &[sentence(0, 10, 0, 500)],
            500,
            &ChapterOptions::default(),
        );
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].title.as_str(), c[0].end_ms), ("Audiobook", 500));
        assert!(place(&d, &[], 0, &ChapterOptions::default()).is_empty());
    }

    #[test]
    fn metadata_escaping() {
        assert_eq!(escape_metadata("a=b;c#d\\e\nf"), "a\\=b\\;c\\#d\\\\e f");
    }
}
