//! Where each sentence and word of a document sounds in the exported audio.

use std::ops::Range;

use serde::Serialize;
use textweaver_core::{CharRange, Utterance};
use textweaver_speech::WordTiming;
use textweaver_text::Document;

/// A chapter of the audio: a title and where it starts and ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Chapter {
    /// Title (the heading's text, a section label, or the document title).
    pub title: String,
    /// Where the chapter starts in the document.
    pub source_start: textweaver_core::CharPos,
    /// Start, in ms from the start of the audio.
    pub start_ms: u64,
    /// End, in ms.
    pub end_ms: u64,
}

/// One word as heard: its time, its document range, and its caption text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimedWord {
    /// Start, in ms from the start of the audio.
    pub start_ms: u64,
    /// End (the next word's start, or the sentence's end), in ms.
    pub end_ms: u64,
    /// The document chars the word came from.
    pub source: Option<CharRange>,
    /// The word's chars within its sentence's caption text
    /// ([`TimedSentence::text`]).
    pub caption: Range<usize>,
    /// The document text of the word ("$5" for spoken "five dollars").
    pub text: String,
}

/// One utterance (a sentence, or part of a long one) as heard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimedSentence {
    /// Start, in ms from the start of the audio.
    pub start_ms: u64,
    /// End, in ms.
    pub end_ms: u64,
    /// The document chars the utterance covers.
    pub source: Option<CharRange>,
    /// The document text of the utterance with whitespace runs collapsed
    /// to single spaces: the caption. Empty for speech with no document
    /// text (a structure announcement on its own).
    pub text: String,
    /// What the engine spoke (normalized).
    pub spoken: String,
    /// Word timings, when the engine reported them.
    pub words: Vec<TimedWord>,
}

/// Everything export learned about the audio it wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Timeline {
    /// Every utterance, in order.
    pub sentences: Vec<TimedSentence>,
    /// Length of the whole audio, in ms.
    pub duration_ms: u64,
    /// Chapters (at least one when there is any audio).
    pub chapters: Vec<Chapter>,
    /// Document title, for audio metadata.
    pub title: Option<String>,
    /// Document author, for audio metadata.
    pub author: Option<String>,
}

/// `text` with every run of whitespace turned into one space and the ends
/// trimmed, plus the caption char index of every char of `text` (and one
/// past the end), so document ranges can be found in the caption.
pub(crate) fn collapse(text: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut index = Vec::with_capacity(text.chars().count() + 1);
    let mut n = 0usize; // chars in `out`
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            pending_space = n > 0;
            index.push(n);
            continue;
        }
        if pending_space {
            out.push(' ');
            n += 1;
            pending_space = false;
        }
        index.push(n);
        out.push(c);
        n += 1;
    }
    index.push(n);
    (out, index)
}

/// Builds a timed sentence for utterance `u`, heard from `start_ms` to
/// `end_ms`, with the engine's word timings (relative to the utterance's
/// own audio) placed on the export's clock.
pub(crate) fn timed_sentence(
    doc: &Document,
    u: &Utterance,
    start_ms: u64,
    end_ms: u64,
    words: &[WordTiming],
) -> TimedSentence {
    let source = u.source_range().filter(|r| !r.is_empty());
    let (text, index) =
        source.map_or_else(|| (String::new(), vec![0]), |r| collapse(&doc.slice(r)));
    let mut timed: Vec<TimedWord> = Vec::new();
    if let Some(sr) = source {
        let mut ordered: Vec<&WordTiming> = words.iter().collect();
        ordered.sort_by_key(|w| (w.audio_ms, w.byte_range.start));
        for (i, w) in ordered.iter().enumerate() {
            let Some(src) = u.source_for(w.byte_range.clone()) else {
                continue; // inserted speech ("heading level 2")
            };
            let Some(src) = clamp(src, sr) else {
                continue;
            };
            let start = (start_ms + u64::from(w.audio_ms)).min(end_ms);
            let next = ordered
                .get(i + 1)
                .map_or(end_ms, |n| (start_ms + u64::from(n.audio_ms)).min(end_ms));
            // Pieces of one source token ("five" and "dollars" for "$5")
            // make one word.
            if let Some(last) = timed.last_mut()
                && let Some(ls) = last.source
                && ls.end > src.start
            {
                let merged = CharRange::new(ls.start, ls.end.max(src.end));
                last.source = Some(merged);
                last.end_ms = next.max(last.end_ms);
                continue;
            }
            timed.push(TimedWord {
                start_ms: start,
                end_ms: next.max(start),
                source: Some(src),
                caption: 0..0,
                text: String::new(),
            });
        }
        let rel = |p: textweaver_core::CharPos| index[(p.0 - sr.start.0).min(index.len() - 1)];
        let starts: Vec<usize> = timed
            .iter()
            .map(|w| w.source.map_or(0, |s| rel(s.start)))
            .collect();
        // Each word's caption runs to the next word's start, so punctuation
        // stays with its word ("today." rather than "today"), as in star's
        // whitespace tokens; the first word also takes any leading
        // punctuation.
        let chars: Vec<char> = text.chars().collect();
        for (i, w) in timed.iter_mut().enumerate() {
            let start = if i == 0 { 0 } else { starts[i] };
            let end = starts.get(i + 1).copied().unwrap_or(chars.len()).max(start);
            let s: String = chars[start..end].iter().collect();
            let lead = s.len() - s.trim_start().len();
            let lead_chars = s[..lead].chars().count();
            w.text = s.trim().to_owned();
            w.caption = start + lead_chars..start + lead_chars + w.text.chars().count();
        }
        timed.retain(|w| !w.text.is_empty());
    }
    TimedSentence {
        start_ms,
        end_ms,
        source,
        text,
        spoken: u.text.clone(),
        words: timed,
    }
}

/// `r` cut to `within`, if anything is left.
fn clamp(r: CharRange, within: CharRange) -> Option<CharRange> {
    let start = r.start.max(within.start);
    let end = r.end.min(within.end);
    (start < end).then(|| CharRange::new(start, end))
}

#[cfg(test)]
mod tests {
    use textweaver_core::{CharPos, SpokenBuilder};

    use super::*;

    #[test]
    fn collapse_maps_every_char() {
        let (t, idx) = collapse("  One\n two  ");
        assert_eq!(t, "One two");
        assert_eq!(idx.len(), "  One\n two  ".chars().count() + 1);
        assert_eq!(idx[2], 0); // 'O'
        assert_eq!(idx[7], 4); // 't'
        assert_eq!(*idx.last().unwrap(), 7);
    }

    #[test]
    fn words_map_to_document_text_and_expansions_merge() {
        let doc = Document::from_plain_text("Paid $5 today.");
        let mut b = SpokenBuilder::new();
        b.push_literal("Paid ", CharPos(0));
        b.push_expanded("five dollars", CharRange::new(5, 7));
        b.push_literal(" today.", CharPos(7));
        let (spoken, map) = b.finish();
        let u = Utterance::with_map(spoken, map);
        let w = |a: u32, b: u32, ms: u32| WordTiming {
            byte_range: a..b,
            audio_ms: ms,
        };
        // Paid | five | dollars | today
        let words = [w(0, 4, 0), w(5, 9, 200), w(10, 17, 400), w(18, 23, 700)];
        let s = timed_sentence(&doc, &u, 1000, 2000, &words);
        let got: Vec<(&str, u64, u64)> = s
            .words
            .iter()
            .map(|w| (w.text.as_str(), w.start_ms, w.end_ms))
            .collect();
        assert_eq!(
            got,
            [
                ("Paid", 1000, 1200),
                ("$5", 1200, 1700),
                ("today.", 1700, 2000)
            ]
        );
        assert_eq!(s.text, "Paid $5 today.");
    }

    #[test]
    fn inserted_speech_has_no_caption() {
        let doc = Document::from_plain_text("Intro");
        let mut b = SpokenBuilder::new();
        b.push_inserted("heading level 1, ", CharPos(0));
        b.push_literal("Intro", CharPos(0));
        let (spoken, map) = b.finish();
        let u = Utterance::with_map(spoken, map);
        let words = [
            WordTiming {
                byte_range: 0..7,
                audio_ms: 0,
            },
            WordTiming {
                byte_range: 17..22,
                audio_ms: 900,
            },
        ];
        let s = timed_sentence(&doc, &u, 0, 1500, &words);
        assert_eq!(s.text, "Intro");
        assert_eq!(s.words.len(), 1);
        assert_eq!(
            (s.words[0].start_ms, s.words[0].text.as_str()),
            (900, "Intro")
        );
    }
}
