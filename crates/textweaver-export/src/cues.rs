//! Subtitle cues (SRT and WebVTT), ported from Star's `star/tts/subtitles.py`
//! (the Star parity reference Part 2 §5.D).
//!
//! Star's rules, kept:
//!
//! - Caption text is split into whitespace-delimited tokens.
//! - Sentence cues group tokens into caption lines, breaking at every
//!   sentence and whenever a line reaches `max_words` (12) tokens or adding
//!   the next token would pass `max_chars` (90) characters.
//! - Word cues give each token its own cue.
//! - Without better timing, a token's share of its stretch of audio is its
//!   length plus one, so longer words take longer.
//! - Times are `HH:MM:SS,mmm` (SRT) or `HH:MM:SS.mmm` (WebVTT, after a
//!   `WEBVTT` header); negative times clamp to zero; a cue whose end is not
//!   after its start is stretched to 50 ms; the file ends with one newline.
//!
//! Deliberate differences:
//!
//! - Star timed one block of text against the whole file's duration, so a
//!   cue drifted further from the audio the longer the document. Here every
//!   sentence has its measured start and end in the audio (export
//!   synthesizes sentence by sentence), and the length weighting only
//!   spreads a sentence's own duration over its tokens.
//! - When the engine reports where each word sounds, tokens take those
//!   times instead of the weighting, and word cues follow the engine's
//!   words (an expansion such as "$5" read as "five dollars" is one cue
//!   showing "$5").
//! - Captions show the document's text, not the normalized spoken text.

use serde::Serialize;

use crate::timeline::{TimedSentence, Timeline};

/// One subtitle cue, in whole milliseconds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Cue {
    /// Start, in ms from the start of the audio.
    pub start_ms: u64,
    /// End, in ms.
    pub end_ms: u64,
    /// Caption text (one line).
    pub text: String,
}

/// SRT or WebVTT.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleFormat {
    /// SubRip (`.srt`), Star's default.
    #[default]
    Srt,
    /// WebVTT (`.vtt`).
    Vtt,
}

impl SubtitleFormat {
    /// The format for a file name's extension (`srt` or `vtt`, any case).
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "srt" => Some(SubtitleFormat::Srt),
            "vtt" => Some(SubtitleFormat::Vtt),
            _ => None,
        }
    }
}

/// How cues are built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CueOptions {
    /// One cue per word instead of caption lines (Star's
    /// `subtitle_word_level`, default off).
    pub word_level: bool,
    /// Most tokens in one caption line (Star: 12).
    pub max_words: usize,
    /// Most characters in one caption line (Star: 90).
    pub max_chars: usize,
}

impl Default for CueOptions {
    fn default() -> Self {
        CueOptions {
            word_level: false,
            max_words: 12,
            max_chars: 90,
        }
    }
}

/// A token of caption text with its timing.
#[derive(Clone, Debug)]
struct Token<'a> {
    text: &'a str,
    start: u64,
    end: u64,
}

/// Star's weighting: each token gets `(len + 1) / total` of `[start, end)`,
/// cumulatively, rounded to whole milliseconds.
fn weighted<'a>(tokens: &[&'a str], start: u64, end: u64) -> Vec<Token<'a>> {
    let weights: Vec<u64> = tokens
        .iter()
        .map(|t| t.chars().count() as u64 + 1)
        .collect();
    let total: u64 = weights.iter().sum::<u64>().max(1);
    let span = end.saturating_sub(start);
    let at = |acc: u64| start + (span * acc + total / 2) / total;
    let mut acc = 0;
    tokens
        .iter()
        .zip(weights)
        .map(|(text, w)| {
            let s = at(acc);
            acc += w;
            Token {
                text,
                start: s,
                end: at(acc),
            }
        })
        .collect()
}

/// Tokens of one sentence's caption, timed from the engine's words where
/// they cover a token and by weighting otherwise.
fn sentence_tokens(s: &TimedSentence) -> Vec<Token<'_>> {
    let spans: Vec<(usize, &str)> = token_spans(&s.text);
    let texts: Vec<&str> = spans.iter().map(|(_, t)| *t).collect();
    let mut tokens = weighted(&texts, s.start_ms, s.end_ms);
    if s.words.is_empty() {
        return tokens;
    }
    // A token takes the start of the first engine word that overlaps it
    // (caption chars relative to the sentence's caption text).
    let mut starts: Vec<Option<u64>> = spans
        .iter()
        .map(|(at, t)| {
            let end = at + t.chars().count();
            s.words
                .iter()
                .find(|w| w.caption.start < end && *at < w.caption.end)
                .map(|w| w.start_ms)
        })
        .collect();
    // Tokens no word touched (a lone dash) start where the previous ended.
    let mut prev = s.start_ms;
    for st in &mut starts {
        let v = st.unwrap_or(prev).max(prev);
        *st = Some(v);
        prev = v;
    }
    for (i, t) in tokens.iter_mut().enumerate() {
        t.start = starts[i].unwrap_or(s.start_ms);
        t.end = starts
            .get(i + 1)
            .copied()
            .flatten()
            .unwrap_or(s.end_ms)
            .max(t.start);
    }
    tokens
}

/// `(char offset, token)` for each whitespace-delimited token of `text`.
fn token_spans(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start: Option<(usize, usize)> = None; // (char index, byte index)
    for (ci, (bi, c)) in text.char_indices().enumerate() {
        match (c.is_whitespace(), start) {
            (true, Some((sc, sb))) => {
                out.push((sc, &text[sb..bi]));
                start = None;
            }
            (false, None) => start = Some((ci, bi)),
            _ => {}
        }
    }
    if let Some((sc, sb)) = start {
        out.push((sc, &text[sb..]));
    }
    out
}

/// Groups one sentence's tokens into caption lines (Star's rules).
fn group(tokens: &[Token<'_>], opts: &CueOptions, out: &mut Vec<Cue>) {
    let mut cur: Vec<&str> = Vec::new();
    let mut cur_start = 0;
    let mut cur_end = 0;
    let mut cur_chars = 0;
    for t in tokens {
        let len = t.text.chars().count();
        let too_long = !cur.is_empty()
            && (cur.len() >= opts.max_words || cur_chars + len + 1 > opts.max_chars);
        if too_long {
            out.push(Cue {
                start_ms: cur_start,
                end_ms: cur_end,
                text: cur.join(" "),
            });
            cur.clear();
            cur_chars = 0;
        }
        if cur.is_empty() {
            cur_start = t.start;
        }
        cur.push(t.text);
        cur_end = t.end;
        cur_chars += len + 1;
    }
    if !cur.is_empty() {
        out.push(Cue {
            start_ms: cur_start,
            end_ms: cur_end,
            text: cur.join(" "),
        });
    }
}

/// The cues for a timeline.
pub fn build(timeline: &Timeline, opts: &CueOptions) -> Vec<Cue> {
    let mut out = Vec::new();
    for s in &timeline.sentences {
        if s.text.is_empty() {
            continue;
        }
        if opts.word_level {
            if s.words.is_empty() {
                for t in sentence_tokens(s) {
                    out.push(Cue {
                        start_ms: t.start,
                        end_ms: t.end,
                        text: t.text.to_owned(),
                    });
                }
            } else {
                for w in s.words.iter().filter(|w| !w.text.is_empty()) {
                    out.push(Cue {
                        start_ms: w.start_ms,
                        end_ms: w.end_ms,
                        text: w.text.clone(),
                    });
                }
            }
        } else {
            group(&sentence_tokens(s), opts, &mut out);
        }
    }
    out
}

/// Star's `_build_subtitle_cues(text, duration)`: cues for `text` spread
/// over `duration_ms` by token length, breaking lines at sentence ends
/// (after `.`, `!`, `?`, or `…` followed by whitespace) as well as at
/// `max_words` and `max_chars`. Used when only a whole file's duration is
/// known.
pub fn from_text(text: &str, duration_ms: u64, opts: &CueOptions) -> Vec<Cue> {
    let spans = token_spans(text);
    if spans.is_empty() || duration_ms == 0 {
        return Vec::new();
    }
    let texts: Vec<&str> = spans.iter().map(|(_, t)| *t).collect();
    let tokens = weighted(&texts, 0, duration_ms);
    if opts.word_level {
        return tokens
            .into_iter()
            .map(|t| Cue {
                start_ms: t.start,
                end_ms: t.end,
                text: t.text.to_owned(),
            })
            .collect();
    }
    let mut out = Vec::new();
    let mut sentence: Vec<Token<'_>> = Vec::new();
    for t in tokens {
        let ends = t.text.ends_with(['.', '!', '?', '\u{2026}']);
        sentence.push(t);
        if ends {
            group(&sentence, opts, &mut out);
            sentence.clear();
        }
    }
    group(&sentence, opts, &mut out);
    out
}

/// `HH:MM:SS,mmm` (SRT) or `HH:MM:SS.mmm` (WebVTT).
pub fn format_time(ms: u64, format: SubtitleFormat) -> String {
    let (hh, rest) = (ms / 3_600_000, ms % 3_600_000);
    let (mm, rest) = (rest / 60_000, rest % 60_000);
    let (ss, ms) = (rest / 1000, rest % 1000);
    let sep = match format {
        SubtitleFormat::Srt => ',',
        SubtitleFormat::Vtt => '.',
    };
    format!("{hh:02}:{mm:02}:{ss:02}{sep}{ms:03}")
}

/// Renders cues as an SRT or WebVTT file.
pub fn render(cues: &[Cue], format: SubtitleFormat) -> String {
    let mut out: Vec<String> = Vec::new();
    if format == SubtitleFormat::Vtt {
        out.push("WEBVTT".into());
        out.push(String::new());
    }
    for (i, c) in cues.iter().enumerate() {
        // Players reject cues that end before they start.
        let end = if c.end_ms <= c.start_ms {
            c.start_ms + 50
        } else {
            c.end_ms
        };
        if format == SubtitleFormat::Srt {
            out.push((i + 1).to_string());
        }
        out.push(format!(
            "{} --> {}",
            format_time(c.start_ms, format),
            format_time(end, format)
        ));
        out.push(c.text.clone());
        out.push(String::new());
    }
    let joined = out.join("\n");
    format!("{}\n", joined.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::TimedWord;

    fn cue(a: u64, b: u64, t: &str) -> Cue {
        Cue {
            start_ms: a,
            end_ms: b,
            text: t.into(),
        }
    }

    /// Star's `_fmt_subtitle_time` vectors (tests/test_tts.py:67-81).
    #[test]
    fn star_time_vectors() {
        assert_eq!(format_time(0, SubtitleFormat::Srt), "00:00:00,000");
        assert_eq!(format_time(1500, SubtitleFormat::Vtt), "00:00:01.500");
        assert_eq!(format_time(3_661_250, SubtitleFormat::Srt), "01:01:01,250");
    }

    /// Star's `_build_subtitle_cues` vectors (tests/test_tts.py:141-181).
    #[test]
    fn star_cue_vectors() {
        let o = CueOptions::default();
        assert!(from_text("", 10_000, &o).is_empty());
        assert!(from_text("hello world", 0, &o).is_empty());
        let words = CueOptions {
            word_level: true,
            ..o
        };
        let c = from_text("alpha beta gamma", 9000, &words);
        let texts: Vec<&str> = c.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(texts, ["alpha", "beta", "gamma"]);
        let c = from_text("one two three four", 12_000, &words);
        assert_eq!(c[0].start_ms, 0);
        assert_eq!(c.last().unwrap().end_ms, 12_000);
        for w in c.windows(2) {
            assert_eq!(w[0].end_ms, w[1].start_ms);
        }
        assert_eq!(
            from_text("Hello world. Goodbye now.", 10_000, &o),
            [
                cue(0, 5000, "Hello world."),
                cue(5000, 10_000, "Goodbye now.")
            ]
        );
        let text: Vec<String> = (0..30).map(|i| format!("w{i}")).collect();
        let text = text.join(" ");
        let five = CueOptions { max_words: 5, ..o };
        let c = from_text(&text, 30_000, &five);
        assert!(c.iter().all(|c| c.text.split(' ').count() <= 5));
        let joined: Vec<&str> = c.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(joined.join(" "), text);
    }

    /// Star's `_format_subtitles` vectors (tests/test_tts.py:188-212).
    #[test]
    fn star_format_vectors() {
        let srt = render(
            &[cue(0, 1000, "hi"), cue(1000, 2000, "there")],
            SubtitleFormat::Srt,
        );
        assert_eq!(
            srt,
            "1\n00:00:00,000 --> 00:00:01,000\nhi\n\n2\n00:00:01,000 --> 00:00:02,000\nthere\n"
        );
        let vtt = render(&[cue(0, 1000, "hi")], SubtitleFormat::Vtt);
        assert_eq!(vtt, "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhi\n");
        let nudged = render(&[cue(5000, 5000, "x")], SubtitleFormat::Srt);
        assert!(nudged.contains("00:00:05,000 --> 00:00:05,050"));
        assert_eq!(render(&[], SubtitleFormat::Srt), "\n");
    }

    fn sentence(text: &str, start: u64, end: u64, words: Vec<TimedWord>) -> TimedSentence {
        TimedSentence {
            start_ms: start,
            end_ms: end,
            source: None,
            text: text.into(),
            spoken: text.into(),
            words,
        }
    }

    fn word(text: &str, caption: (usize, usize), a: u64, b: u64) -> TimedWord {
        TimedWord {
            start_ms: a,
            end_ms: b,
            source: None,
            caption: caption.0..caption.1,
            text: text.into(),
        }
    }

    #[test]
    fn sentences_keep_their_measured_times() {
        let t = Timeline {
            sentences: vec![
                sentence("Hello world.", 0, 1000, Vec::new()),
                sentence("Paid $5 today.", 1500, 3000, Vec::new()),
            ],
            duration_ms: 3000,
            ..Timeline::default()
        };
        assert_eq!(
            build(&t, &CueOptions::default()),
            [
                cue(0, 1000, "Hello world."),
                cue(1500, 3000, "Paid $5 today.")
            ]
        );
    }

    #[test]
    fn engine_words_time_tokens_and_word_cues() {
        // "Paid $5 today." spoken "Paid five dollars today."
        let words = vec![
            word("Paid", (0, 4), 1000, 1200),
            word("$5", (5, 7), 1200, 1800),
            word("today.", (8, 13), 1800, 2400),
        ];
        let t = Timeline {
            sentences: vec![sentence("Paid $5 today.", 1000, 2500, words)],
            duration_ms: 2500,
            ..Timeline::default()
        };
        let lines = build(
            &t,
            &CueOptions {
                max_words: 2,
                ..CueOptions::default()
            },
        );
        assert_eq!(
            lines,
            [cue(1000, 1800, "Paid $5"), cue(1800, 2500, "today.")]
        );
        let w = build(
            &t,
            &CueOptions {
                word_level: true,
                ..CueOptions::default()
            },
        );
        assert_eq!(
            w,
            [
                cue(1000, 1200, "Paid"),
                cue(1200, 1800, "$5"),
                cue(1800, 2400, "today.")
            ]
        );
    }

    #[test]
    fn subtitle_format_from_extension() {
        use std::path::Path;
        assert_eq!(
            SubtitleFormat::from_path(Path::new("a.SRT")),
            Some(SubtitleFormat::Srt)
        );
        assert_eq!(
            SubtitleFormat::from_path(Path::new("a.vtt")),
            Some(SubtitleFormat::Vtt)
        );
        assert_eq!(SubtitleFormat::from_path(Path::new("a.txt")), None);
    }
}
