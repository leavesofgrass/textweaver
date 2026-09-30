//! ID3v2 tags with chapters, written into WAV and MP3 files (the `id3` crate).
//!
//! WAV has no chapter format of its own. An `ID3 ` chunk inside the RIFF
//! file carries the same tags MP3 files get through ffmpeg: the title
//! (also as the album), the author as the artist, the genre "Audiobook",
//! one CHAP frame per chapter (start and end in milliseconds, its title in
//! a TIT2 subframe), and one top-level, ordered CTOC frame that lists them.
//! Players that read ID3 in WAV files (and tools such as ffprobe) show the
//! chapters; others skip the chunk and play the audio as before.

use std::path::Path;

use id3::frame::{Chapter as Chap, TableOfContents};
use id3::{Frame, Tag, TagLike, Version};

use crate::ExportError;
use crate::timeline::Chapter;

/// The tag for an export.
pub fn tag(title: Option<&str>, author: Option<&str>, chapters: &[Chapter]) -> Tag {
    let mut tag = Tag::new();
    if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
        tag.set_title(t);
        tag.set_album(t);
    }
    if let Some(a) = author.filter(|a| !a.trim().is_empty()) {
        tag.set_artist(a);
    }
    tag.set_genre("Audiobook");
    let ids: Vec<String> = (1..=chapters.len()).map(|i| format!("chp{i}")).collect();
    if !chapters.is_empty() {
        tag.add_frame(TableOfContents {
            element_id: "toc".to_owned(),
            top_level: true,
            ordered: true,
            elements: ids.clone(),
            frames: Vec::new(),
        });
    }
    for (c, id) in chapters.iter().zip(ids) {
        tag.add_frame(Chap {
            element_id: id,
            start_time: u32::try_from(c.start_ms).unwrap_or(u32::MAX),
            end_time: u32::try_from(c.end_ms).unwrap_or(u32::MAX),
            start_offset: u32::MAX,
            end_offset: u32::MAX,
            frames: vec![Frame::text("TIT2", c.title.clone())],
        });
    }
    tag
}

/// Writes the tag into the WAV or MP3 file at `path` (ID3v2.3, as ffmpeg writes
/// for MP3, for the widest player support).
pub fn write(
    path: &Path,
    title: Option<&str>,
    author: Option<&str>,
    chapters: &[Chapter],
) -> Result<(), ExportError> {
    tag(title, author, chapters)
        .write_to_path(path, Version::Id3v23)
        .map_err(|e| ExportError::Tags {
            path: path.to_owned(),
            message: e.to_string(),
        })
}
