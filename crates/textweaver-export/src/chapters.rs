//! Chapters from a document's structure, placed on the audio's timeline,
//! and the ffmpeg metadata that carries them into an M4B (or MP3).
//!
//! star (`star/audiobook.py`) made a chapter of every Markdown heading and
//! titled any text before the first heading after the document. Here the
//! loaders' markers say where chapters start: every `SectionBreak` (an EPUB
//! spine item, a DOCX section) and every `Heading` up to
//! [`ChapterOptions::max_heading_level`] (default 6, star's "every
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

/// The names chapters get when the document gives none, in the interface
/// language (the catalogs' `export-chapter-*` messages). English by
/// default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChapterNames {
    /// The leading chapter's title when the document has no title.
    pub untitled_document: String,
    /// An untitled chapter's name, with `{number}` where its number goes.
    pub numbered: String,
}

impl Default for ChapterNames {
    fn default() -> Self {
        ChapterNames {
            untitled_document: "Audiobook".to_owned(),
            numbered: "Chapter {number}".to_owned(),
        }
    }
}

impl ChapterNames {
    /// The name of untitled chapter `n` (from 1).
    pub fn number(&self, n: usize) -> String {
        if self.numbered.contains("{number}") {
            self.numbered.replace("{number}", &n.to_string())
        } else {
            format!("{} {n}", self.numbered.trim())
        }
    }
}

/// Places chapters on the timeline. Text before the first chapter start
/// becomes a leading chapter titled after the document (star's rule);
/// untitled chapters are numbered; chapters without audio are dropped.
/// Untitled names are English; see [`place_named`].
pub fn place(
    doc: &Document,
    sentences: &[TimedSentence],
    duration_ms: u64,
    opts: &ChapterOptions,
) -> Vec<Chapter> {
    place_named(doc, sentences, duration_ms, opts, &ChapterNames::default())
}

/// [`place`] with the names for untitled chapters given.
pub fn place_named(
    doc: &Document,
    sentences: &[TimedSentence],
    duration_ms: u64,
    opts: &ChapterOptions,
    names: &ChapterNames,
) -> Vec<Chapter> {
    if duration_ms == 0 || sentences.is_empty() {
        return Vec::new();
    }
    let doc_title = doc
        .meta
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| names.untitled_document.clone());
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
            names.number(chapters.len() + 1)
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

/// A WebVTT chapters file (`NAME.chapters.vtt`): one cue per chapter,
/// its title as the cue text (escaped), for a player's chapter menu.
pub fn vtt(chapters: &[Chapter]) -> String {
    let cues: Vec<crate::Cue> = chapters
        .iter()
        .map(|c| crate::Cue {
            start_ms: c.start_ms,
            end_ms: c.end_ms,
            text: c.title.clone(),
        })
        .collect();
    crate::cues::render(&cues, crate::SubtitleFormat::Vtt)
}

/// The chapters file beside `subtitles` or the audio: `essay.vtt` and
/// `essay.mp3` both give `essay.chapters.vtt`.
pub fn vtt_path(beside: &std::path::Path) -> std::path::PathBuf {
    let stem = beside
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "captions".to_owned());
    beside.with_file_name(format!("{stem}.chapters.vtt"))
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
/// `[CHAPTER]` per chapter (times in ms), as star's
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
        let german = ChapterNames {
            untitled_document: "Hörbuch".into(),
            numbered: "Kapitel {number}".into(),
        };
        let c = place_named(
            &d,
            &[sentence(0, 10, 0, 500)],
            500,
            &ChapterOptions::default(),
            &german,
        );
        assert_eq!(c[0].title, "Hörbuch");
        assert_eq!(german.number(3), "Kapitel 3");
        let bare = ChapterNames {
            numbered: "Capítulo".into(),
            ..ChapterNames::default()
        };
        assert_eq!(bare.number(2), "Capítulo 2");
    }

    #[test]
    fn chapters_file_is_webvtt_with_a_cue_per_chapter() {
        let chapters = [
            Chapter {
                title: "Photosynthesis".into(),
                source_start: CharPos::ZERO,
                start_ms: 0,
                end_ms: 2750,
            },
            Chapter {
                title: "Light & dark".into(),
                source_start: CharPos(40),
                start_ms: 2750,
                end_ms: 4000,
            },
        ];
        assert_eq!(
            vtt(&chapters),
            "WEBVTT\n\n00:00:00.000 --> 00:00:02.750\nPhotosynthesis\n\n\
             00:00:02.750 --> 00:00:04.000\nLight &amp; dark\n"
        );
        assert_eq!(
            vtt_path(std::path::Path::new("out/essay.mp3")),
            std::path::Path::new("out/essay.chapters.vtt")
        );
    }

    #[test]
    fn metadata_escaping() {
        assert_eq!(escape_metadata("a=b;c#d\\e\nf"), "a\\=b\\;c\\#d\\\\e f");
    }
}
