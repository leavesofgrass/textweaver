//! Caption files read as transcripts: SubRip (`.srt`) and WebVTT (`.vtt`).
//!
//! A lecture's captions are often the only transcript a student has. The
//! cues' text is read as paragraphs; cue numbers, times and markup are not
//! read at all:
//!
//! - A new paragraph starts where the speaker changes (a WebVTT voice tag,
//!   `<v Ada Example>`, or the `>>` broadcast captions use), after a gap of
//!   more than [`GAP_MS`] between cues, and at the end of a sentence once a
//!   paragraph holds [`PARAGRAPH_WORDS`] words, so a long talk without
//!   pauses still has paragraphs to move by.
//! - A voice tag's name is read once, where that speaker starts: "Ada
//!   Example: Good morning."
//! - Every [`HEADING_MINUTES`] minutes of the recording a level-1 heading
//!   says where in the recording the text is, "Minute 5", so the heading
//!   keys move through the talk by time. Its words come from
//!   [`LoadOptions::minute_heading`].
//! - Styling (`<i>`, `<b>`, `<c.yellow>`, `<font>`, SubRip's `{\an8}`),
//!   karaoke timestamps, and the character references WebVTT allows
//!   (`&amp;`, `&lt;`, `&nbsp;`) are taken out or decoded. The WebVTT header
//!   and its `NOTE`, `STYLE` and `REGION` blocks are skipped.
//! - Captions that repeat the line before (the roll-up captions of
//!   automatic captioning) are read once.
//!
//! Robustness: a byte order mark, CRLF line ends, cues with no blank line
//! between them, a missing or damaged time (the cue is still read), and
//! text before the first cue. A file with no cue times at all is read as
//! plain text and says so. The file size cap is [`MAX_BYTES`], the JSON
//! loader's; a cue keeps at most [`MAX_CUE_LINES`] lines and a voice name
//! at most [`MAX_NAME_CHARS`] characters.

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, Marker};

use crate::builder::Builder;
use crate::{
    LoadError, LoadOptions, Loader, Source, add_warning, meta_for, note_encoding, title_from_path,
};

/// Largest caption file read (the JSON loader's cap).
pub const MAX_BYTES: usize = crate::json::MAX_BYTES;

/// A gap between cues longer than this (in milliseconds) starts a new
/// paragraph.
pub const GAP_MS: u64 = 4_000;

/// Once a paragraph holds this many words, the next sentence end closes it.
pub const PARAGRAPH_WORDS: usize = 120;

/// Minutes between the headings that mark time.
pub const HEADING_MINUTES: u64 = 5;

/// Most lines kept from one cue.
pub const MAX_CUE_LINES: usize = 64;

/// Most characters kept from a voice tag's name.
pub const MAX_NAME_CHARS: usize = 64;

/// The minute heading's words when [`LoadOptions::minute_heading`] is
/// empty: `{n}` stands for the minute.
pub const DEFAULT_MINUTE_HEADING: &str = "Minute {n}";

/// The warning a caption file with no cue times carries.
pub const NO_CUES_WARNING: &str = "This file has no caption times, so it is read as plain text.";

/// Loads SubRip and WebVTT caption files as transcripts.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaptionLoader;

impl Loader for CaptionLoader {
    fn id(&self) -> &'static str {
        "captions"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["srt", "vtt"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let decoded = crate::json::text_of(source)?;
        let mut meta = meta_for(source, self.id());
        note_encoding(&mut meta, &decoded);
        meta.title = title_from_path(source);
        let cues = parse(&decoded.text);
        if cues.is_empty() {
            add_warning(&mut meta, NO_CUES_WARNING);
            let text = decoded.text.trim().to_owned();
            return Ok(Document::new(meta, Rope::from_str(&text), Vec::new()));
        }
        let (text, markers) = transcript(&cues, &options.minute_heading);
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

/// One cue: its start and end in milliseconds, and its text lines as
/// written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    /// Start, in milliseconds.
    pub start_ms: u64,
    /// End, in milliseconds (the start when it could not be read).
    pub end_ms: u64,
    /// The text lines, with their markup.
    pub lines: Vec<String>,
}

/// A time as SubRip and WebVTT write it: `HH:MM:SS,mmm`, `HH:MM:SS.mmm`,
/// or `MM:SS.mmm`, in milliseconds.
pub fn parse_time(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.trim().split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let number = |p: &str| -> Option<u64> {
        (!p.is_empty() && p.len() <= 9 && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse().ok())
            .flatten()
    };
    let (secs, frac) = match parts[parts.len() - 1].split_once([',', '.']) {
        Some((s, f)) => (s, f),
        None => (parts[parts.len() - 1], "0"),
    };
    let frac_digits: String = frac.chars().take(3).collect();
    let ms = number(&frac_digits)? * 10u64.pow(3 - frac_digits.len() as u32);
    let secs = number(secs)?;
    let mins = number(parts[parts.len() - 2])?;
    let hours = if parts.len() == 3 {
        number(parts[0])?
    } else {
        0
    };
    Some(
        hours
            .saturating_mul(3_600_000)
            .saturating_add(mins.saturating_mul(60_000))
            .saturating_add(secs.saturating_mul(1_000))
            .saturating_add(ms),
    )
}

/// A cue's timing line, `start --> end [settings]`: its start and end.
fn timing(line: &str) -> Option<(u64, u64)> {
    let (left, right) = line.split_once("-->")?;
    let start = parse_time(left)?;
    let end = right
        .split_whitespace()
        .next()
        .and_then(parse_time)
        .unwrap_or(start);
    Some((start, end))
}

/// True when `line` is `word` alone or followed by white space (WebVTT's
/// `NOTE`, `STYLE`, `REGION`).
fn is_block(line: &str, word: &str) -> bool {
    line.strip_prefix(word)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

/// The cues of a SubRip or WebVTT text (line ends already `\n`): every
/// line holding `-->` starts one, and a cue whose times cannot be read
/// takes the start of the cue before it.
pub fn parse(text: &str) -> Vec<Cue> {
    let mut cues: Vec<Cue> = Vec::new();
    let mut first_block = true;
    let mut lines: Vec<&str> = Vec::new();
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    for line in text.trim_start_matches('\u{feff}').split('\n') {
        if line.trim().is_empty() {
            if !lines.is_empty() {
                blocks.push(std::mem::take(&mut lines));
            }
        } else {
            lines.push(line.trim_end());
        }
    }
    if !lines.is_empty() {
        blocks.push(lines);
    }
    for block in blocks {
        let head = block[0].trim_start();
        let header = first_block && head.starts_with("WEBVTT");
        first_block = false;
        if header
            || ["NOTE", "STYLE", "REGION"]
                .iter()
                .any(|w| is_block(head, w))
        {
            continue;
        }
        let mut i = 0;
        while i < block.len() {
            // Cue text never holds "-->" (WebVTT forbids it), so such a
            // line starts a cue even when its times are damaged: the cue
            // then takes the time of the one before.
            if block[i].contains("-->") {
                let before = cues.last().map_or(0, |c| c.start_ms);
                let (start_ms, end_ms) = timing(block[i]).unwrap_or((before, before));
                cues.push(Cue {
                    start_ms,
                    end_ms,
                    lines: Vec::new(),
                });
            } else if block.get(i + 1).is_some_and(|next| next.contains("-->"))
                && (i == 0 || block[i].trim().bytes().all(|b| b.is_ascii_digit()))
            {
                // A cue number or identifier: not read. Only the first
                // line of a block or a number, so the last line of a cue
                // with no blank line after it is still read.
            } else if let Some(cue) = cues.last_mut()
                && cue.lines.len() < MAX_CUE_LINES
            {
                cue.lines.push(block[i].to_owned());
            }
            i += 1;
        }
    }
    cues
}

/// A piece of cue text and who says it (a WebVTT voice tag's name).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Piece {
    speaker: Option<String>,
    text: String,
    /// The piece began with `>>`, a new speaker in broadcast captions.
    turn: bool,
}

/// Decodes the character references WebVTT allows, and numeric ones.
fn decode_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let n = name.strip_prefix('#')?;
            let code = match n.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => n.parse().ok()?,
            };
            char::from_u32(code).filter(|c| !c.is_control())
        }
    }
}

/// One line of cue text without its markup, split where a voice tag
/// starts; `voice` carries the speaker across the cue's lines.
fn pieces(line: &str, voice: &mut Option<String>, out: &mut Vec<Piece>) {
    let mut text = String::new();
    let mut speaker = voice.clone();
    let mut rest = line;
    let flush = |text: &mut String, speaker: &Option<String>, out: &mut Vec<Piece>| {
        let t = text.split_whitespace().collect::<Vec<_>>().join(" ");
        text.clear();
        if t.is_empty() {
            return;
        }
        let (turn, t) = match t.strip_prefix(">>") {
            Some(after) => (true, after.trim_start().to_owned()),
            None => (false, t),
        };
        if !t.is_empty() {
            out.push(Piece {
                speaker: speaker.clone(),
                text: t,
                turn,
            });
        }
    };
    while let Some(c) = rest.chars().next() {
        let after = &rest[c.len_utf8()..];
        match c {
            '<' => {
                let Some(end) = after.find('>') else {
                    text.push(c);
                    rest = after;
                    continue;
                };
                let tag = &after[..end];
                if let Some(v) = tag
                    .strip_prefix('v')
                    .filter(|v| v.starts_with(char::is_whitespace) || v.starts_with('.'))
                {
                    let name = v.split_once(char::is_whitespace).map_or("", |(_, n)| n);
                    let name: String = name
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .chars()
                        .filter(|c| !c.is_control())
                        .take(MAX_NAME_CHARS)
                        .collect();
                    flush(&mut text, &speaker, out);
                    speaker = Some(name).filter(|n| !n.is_empty());
                    *voice = speaker.clone();
                } else if tag == "/v" {
                    flush(&mut text, &speaker, out);
                    speaker = None;
                    *voice = None;
                }
                rest = &after[end + 1..];
            }
            '{' if after.starts_with('\\') => {
                // SubRip's override tags, such as {\an8}.
                rest = after.find('}').map_or("", |end| &after[end + 1..]);
            }
            '&' => {
                let name_end = after
                    .find(';')
                    .filter(|&e| e <= 10 && !after[..e].contains(char::is_whitespace));
                match name_end.and_then(|e| decode_entity(&after[..e]).map(|ch| (e, ch))) {
                    Some((e, ch)) => {
                        text.push(ch);
                        rest = &after[e + 1..];
                    }
                    None if after.starts_with("lrm;") || after.starts_with("rlm;") => {
                        rest = &after[4..];
                    }
                    None => {
                        text.push(c);
                        rest = after;
                    }
                }
            }
            c if c.is_control() => rest = after,
            c => {
                text.push(c);
                rest = after;
            }
        }
    }
    flush(&mut text, &speaker, out);
}

/// The minute heading's words for minute `n`.
fn minute_label(template: &str, n: u64) -> String {
    let template = if template.contains("{n}") {
        template
    } else {
        DEFAULT_MINUTE_HEADING
    };
    template.replace("{n}", &n.to_string())
}

/// The cues as canonical text and markers: paragraphs, speakers, and the
/// minute headings.
fn transcript(cues: &[Cue], minute_heading: &str) -> (String, Vec<Marker>) {
    let mut b = Builder::new();
    let block_ms = HEADING_MINUTES * 60_000;
    let mut block = 0u64;
    let mut speaker: Option<String> = None;
    let mut words = 0usize;
    let mut sentence_end = false;
    let mut prev_end: Option<u64> = None;
    let mut prev_last: Option<String> = None;
    let mut started = false;
    for cue in cues {
        let mut voice = None;
        let mut ps = Vec::new();
        let mut last_line = None;
        for (k, line) in cue.lines.iter().enumerate() {
            let mut line_pieces = Vec::new();
            pieces(line, &mut voice, &mut line_pieces);
            let plain: String = line_pieces
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            // Roll-up captions repeat the line before as their first line.
            if k == 0 && !plain.is_empty() && prev_last.as_deref() == Some(plain.as_str()) {
                continue;
            }
            if !plain.is_empty() {
                last_line = Some(plain);
            }
            ps.extend(line_pieces);
        }
        if last_line.is_some() {
            prev_last = last_line;
        }
        if ps.is_empty() {
            continue;
        }
        let this_block = cue.start_ms / block_ms;
        let mut new_paragraph =
            prev_end.is_some_and(|end| cue.start_ms > end.saturating_add(GAP_MS));
        if this_block > block {
            block = this_block;
            b.paragraph_break();
            let id = b.open(Marker::new(MarkerKind::Heading, CharRange::empty(0)).with_level(1));
            b.text(&minute_label(minute_heading, block * HEADING_MINUTES));
            b.close(id);
            new_paragraph = true;
        }
        prev_end = Some(cue.end_ms.max(cue.start_ms));
        for p in ps {
            let changed = p.speaker.is_some() && p.speaker != speaker;
            let long = words >= PARAGRAPH_WORDS && sentence_end;
            if new_paragraph || changed || p.turn || long || !started {
                b.paragraph_break();
                words = 0;
                new_paragraph = false;
            } else {
                b.space();
            }
            started = true;
            if changed {
                speaker = p.speaker.clone();
                if let Some(name) = &speaker {
                    b.text(&format!("{name}:"));
                    b.space();
                }
            }
            words += p.text.split_whitespace().count();
            sentence_end = p
                .text
                .trim_end_matches(['"', '\'', ')', '\u{201d}', '\u{2019}'])
                .ends_with(['.', '?', '!', '\u{2026}']);
            b.text(&p.text);
        }
    }
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_in_both_forms() {
        assert_eq!(parse_time("00:01:02,500"), Some(62_500));
        assert_eq!(parse_time("01:00:00.001"), Some(3_600_001));
        assert_eq!(parse_time("05:07.25"), Some(307_250));
        assert_eq!(parse_time("1:2:3"), Some(3_723_000));
        assert_eq!(parse_time("aa:00:00,000"), None);
        assert_eq!(parse_time("00,000"), None);
        assert_eq!(parse_time(""), None);
        assert_eq!(
            parse_time("999999999:00:00,000"),
            Some(999_999_999 * 3_600_000)
        );
    }

    #[test]
    fn markup_is_taken_out_and_voices_kept() {
        let mut voice = None;
        let mut out = Vec::new();
        pieces(
            "<v.loud Ada Example><i>Hello</i> &amp; {\\an8}welcome&nbsp;all</v> <c.yellow>back</c>",
            &mut voice,
            &mut out,
        );
        assert_eq!(
            out,
            [
                Piece {
                    speaker: Some("Ada Example".into()),
                    text: "Hello & welcome all".into(),
                    turn: false
                },
                Piece {
                    speaker: None,
                    text: "back".into(),
                    turn: false
                }
            ]
        );
        let mut out = Vec::new();
        pieces(">> Next speaker. 1 < 2 &bogus; &#65;", &mut None, &mut out);
        assert_eq!(out[0].text, "Next speaker. 1 < 2 &bogus; A");
        assert!(out[0].turn);
    }

    #[test]
    fn minute_labels() {
        assert_eq!(minute_label("", 5), "Minute 5");
        assert_eq!(minute_label("Minuto {n}", 10), "Minuto 10");
        assert_eq!(minute_label("no placeholder", 15), "Minute 15");
    }
}
