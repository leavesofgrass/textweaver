//! Vorbis comments: the tag format of FLAC and Ogg Opus files.
//!
//! Both formats carry the same body (a FLAC metadata block of type 4, an
//! Ogg Opus file's `OpusTags` header after its magic):
//!
//! - `TITLE` and `ALBUM`: the document's title; `ARTIST`: its author;
//!   `GENRE=Audiobook` (the same tags the M4B gets through ffmpeg);
//! - one `CHAPTERnnn=HH:MM:SS.mmm` and `CHAPTERnnnNAME=title` pair per
//!   chapter, the chapter convention audiobook players read from Vorbis
//!   comments.

use crate::timeline::Chapter;

/// `HH:MM:SS.mmm` for a time in milliseconds (the chapter time format).
pub fn chapter_time(ms: u64) -> String {
    let (h, rest) = (ms / 3_600_000, ms % 3_600_000);
    let (m, rest) = (rest / 60_000, rest % 60_000);
    let (s, milli) = (rest / 1000, rest % 1000);
    format!("{h:02}:{m:02}:{s:02}.{milli:03}")
}

/// The Vorbis comments for an export: tags, then the chapters in order.
pub fn comments(
    title: Option<&str>,
    author: Option<&str>,
    chapters: &[Chapter],
) -> Vec<(String, String)> {
    let mut c = Vec::new();
    if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
        c.push(("TITLE".to_owned(), t.to_owned()));
        c.push(("ALBUM".to_owned(), t.to_owned()));
    }
    if let Some(a) = author.filter(|a| !a.trim().is_empty()) {
        c.push(("ARTIST".to_owned(), a.to_owned()));
    }
    c.push(("GENRE".to_owned(), "Audiobook".to_owned()));
    for (i, ch) in chapters.iter().enumerate() {
        let key = format!("CHAPTER{:03}", i + 1);
        c.push((key.clone(), chapter_time(ch.start_ms)));
        c.push((format!("{key}NAME"), ch.title.clone()));
    }
    c
}

/// The body of a Vorbis comment block: the vendor string, then each
/// `KEY=value` comment, every length a 32-bit little-endian count of UTF-8
/// bytes. Newlines in values become spaces.
pub fn vorbis_comment_block(vendor: &str, comments: &[(String, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    let put = |s: &str, out: &mut Vec<u8>| {
        let len = u32::try_from(s.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&s.as_bytes()[..len as usize]);
    };
    put(vendor, &mut out);
    let count = u32::try_from(comments.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&count.to_le_bytes());
    for (k, v) in comments {
        let v = v.replace(['\r', '\n'], " ");
        put(&format!("{k}={v}"), &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_times_read_as_clock_times() {
        assert_eq!(chapter_time(0), "00:00:00.000");
        assert_eq!(chapter_time(3_723_045), "01:02:03.045");
    }

    #[test]
    fn comments_carry_tags_then_chapters() {
        let ch = |title: &str, start_ms| Chapter {
            title: title.to_owned(),
            start_ms,
            end_ms: start_ms + 1000,
            source_start: textweaver_core::CharPos(0),
        };
        let c = comments(Some("Plants"), None, &[ch("Intro", 0), ch("Light", 61_500)]);
        let keys: Vec<String> = c.iter().map(|(k, v)| format!("{k}={v}")).collect();
        assert_eq!(
            keys,
            [
                "TITLE=Plants",
                "ALBUM=Plants",
                "GENRE=Audiobook",
                "CHAPTER001=00:00:00.000",
                "CHAPTER001NAME=Intro",
                "CHAPTER002=00:01:01.500",
                "CHAPTER002NAME=Light",
            ]
        );
    }

    #[test]
    fn the_comment_block_is_length_prefixed() {
        let b = vorbis_comment_block("tw", &[("TITLE".into(), "a\nb".into())]);
        assert_eq!(
            b,
            [
                2, 0, 0, 0, b't', b'w', 1, 0, 0, 0, 9, 0, 0, 0, b'T', b'I', b'T', b'L', b'E', b'=',
                b'a', b' ', b'b'
            ]
        );
    }
}
