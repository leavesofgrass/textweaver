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
    /// Advanced SubStation Alpha (`.ass`), karaoke by outline (`\ko`).
    Ass,
}

impl SubtitleFormat {
    /// The format for a file name's extension (`srt`, `vtt` or `ass`, any
    /// case).
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "srt" => Some(SubtitleFormat::Srt),
            "vtt" => Some(SubtitleFormat::Vtt),
            "ass" => Some(SubtitleFormat::Ass),
            _ => None,
        }
    }
}

/// How caption lines show the word being read. Every style is a shape
/// (underline, bold, outline), never a color alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Karaoke {
    /// Plain caption lines.
    #[default]
    Off,
    /// WebVTT timestamp tags before each word after a line's first, with a
    /// `STYLE` block underlining the words already spoken. Players without
    /// tag support show the plain line. SRT has no tags and stays plain.
    Tags,
    /// A cue per word showing the whole line with that word in bold and
    /// underline. The most portable karaoke, but a player that voices or
    /// brailles each cue repeats the line once per word, so it is never the
    /// default.
    Lines,
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
    /// Karaoke style for caption lines (off by default; ignored with
    /// `word_level`). ASS files are always karaoke by outline.
    pub karaoke: Karaoke,
}

impl Default for CueOptions {
    fn default() -> Self {
        CueOptions {
            word_level: false,
            max_words: 12,
            max_chars: 90,
            karaoke: Karaoke::Off,
        }
    }
}

/// What a caption file says about itself: a WebVTT `NOTE` block, or the
/// ASS script's `Title` and comment.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CaptionMeta {
    /// The document's title.
    pub title: Option<String>,
    /// Who reads it: the voice and engine, as people say them.
    pub voice: Option<String>,
    /// The reading rate in words per minute.
    pub words_per_minute: Option<u32>,
    /// Further sentences for the note, such as "Machine captions: check
    /// before sharing."
    pub notes: Vec<String>,
}

impl CaptionMeta {
    /// The note's sentences in one line: the title, who reads it and how
    /// fast, the notes, then "Made by textweaver." Newlines become spaces
    /// and `-->` (forbidden in a WebVTT note) becomes `->`.
    pub fn sentences(&self) -> String {
        let clean = |s: &str| {
            let s: String = s
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
            s.replace("-->", "->")
        };
        let end = |s: String| {
            if s.ends_with(['.', '!', '?', ':']) {
                s
            } else {
                format!("{s}.")
            }
        };
        let mut parts: Vec<String> = Vec::new();
        if let Some(t) = self.title.as_deref().map(clean).filter(|t| !t.is_empty()) {
            parts.push(end(t));
        }
        match (
            self.voice.as_deref().map(clean).filter(|v| !v.is_empty()),
            self.words_per_minute,
        ) {
            (Some(v), Some(w)) => parts.push(format!("Read by {v}, {w} words a minute.")),
            (Some(v), None) => parts.push(end(format!("Read by {v}"))),
            (None, Some(w)) => parts.push(format!("Read at {w} words a minute.")),
            (None, None) => {}
        }
        for n in &self.notes {
            let n = clean(n);
            if !n.is_empty() {
                parts.push(end(n));
            }
        }
        parts.push("Made by textweaver.".to_owned());
        parts.join(" ")
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

/// One caption line with the times of its words, the shape the karaoke
/// renderers need ([`lines`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaptionLine {
    /// Start, in ms from the start of the audio.
    pub start_ms: u64,
    /// End, in ms.
    pub end_ms: u64,
    /// The line's words in order, each with its own start and end.
    pub words: Vec<Cue>,
}

impl CaptionLine {
    /// The line as one plain cue (its words joined by spaces).
    pub fn cue(&self) -> Cue {
        Cue {
            start_ms: self.start_ms,
            end_ms: self.end_ms,
            text: self.text(),
        }
    }

    /// The line's text.
    pub fn text(&self) -> String {
        let words: Vec<&str> = self.words.iter().map(|w| w.text.as_str()).collect();
        words.join(" ")
    }
}

/// Groups one sentence's tokens into caption lines (Star's rules).
fn group(tokens: &[Token<'_>], opts: &CueOptions, out: &mut Vec<CaptionLine>) {
    fn flush(cur: &mut Vec<Cue>, out: &mut Vec<CaptionLine>) {
        if let (Some(first), Some(last)) = (cur.first(), cur.last()) {
            out.push(CaptionLine {
                start_ms: first.start_ms,
                end_ms: last.end_ms,
                words: std::mem::take(cur),
            });
        }
    }
    let mut cur: Vec<Cue> = Vec::new();
    let mut cur_chars = 0;
    for t in tokens {
        let len = t.text.chars().count();
        let too_long = !cur.is_empty()
            && (cur.len() >= opts.max_words || cur_chars + len + 1 > opts.max_chars);
        if too_long {
            flush(&mut cur, out);
            cur_chars = 0;
        }
        cur.push(Cue {
            start_ms: t.start,
            end_ms: t.end,
            text: t.text.to_owned(),
        });
        cur_chars += len + 1;
    }
    flush(&mut cur, out);
}

/// The caption lines for a timeline, with each word's time (Star's line
/// rules; [`CueOptions::word_level`] is not used here).
pub fn lines(timeline: &Timeline, opts: &CueOptions) -> Vec<CaptionLine> {
    let mut out = Vec::new();
    for s in &timeline.sentences {
        if !s.text.is_empty() {
            group(&sentence_tokens(s), opts, &mut out);
        }
    }
    out
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
            let mut lines = Vec::new();
            group(&sentence_tokens(s), opts, &mut lines);
            out.extend(lines.iter().map(CaptionLine::cue));
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
    let mut out: Vec<CaptionLine> = Vec::new();
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
    out.iter().map(CaptionLine::cue).collect()
}

/// `HH:MM:SS,mmm` (SRT), `HH:MM:SS.mmm` (WebVTT), or `H:MM:SS.cc` (ASS,
/// rounded to the nearest centisecond).
pub fn format_time(ms: u64, format: SubtitleFormat) -> String {
    if format == SubtitleFormat::Ass {
        let cs = centis(ms);
        let (h, rest) = (cs / 360_000, cs % 360_000);
        let (m, rest) = (rest / 6000, rest % 6000);
        let (s, cs) = (rest / 100, rest % 100);
        return format!("{h}:{m:02}:{s:02}.{cs:02}");
    }
    let (hh, rest) = (ms / 3_600_000, ms % 3_600_000);
    let (mm, rest) = (rest / 60_000, rest % 60_000);
    let (ss, ms) = (rest / 1000, rest % 1000);
    let sep = if format == SubtitleFormat::Srt {
        ','
    } else {
        '.'
    };
    format!("{hh:02}:{mm:02}:{ss:02}{sep}{ms:03}")
}

/// Milliseconds rounded to whole centiseconds.
fn centis(ms: u64) -> u64 {
    (ms + 5) / 10
}

/// WebVTT cue text with `&`, `<` and `>` escaped, as the WebVTT spec's cue
/// text rules ask: a raw `<` starts a tag, and `-->` is forbidden in cue
/// text (it becomes `--&gt;`). SRT has no escapes and keeps the text as is.
pub fn vtt_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

/// A cue's end, stretched to 50 ms after its start when it is not after it
/// (players reject cues that end before they start).
fn end_of(start: u64, end: u64) -> u64 {
    if end <= start { start + 50 } else { end }
}

/// Writes `(start, end, finished text)` cues as SRT or WebVTT after the
/// header blocks (`WEBVTT`, `STYLE`, `NOTE`) in `head`.
fn write_cues(
    head: Vec<String>,
    cues: impl IntoIterator<Item = (u64, u64, String)>,
    format: SubtitleFormat,
) -> String {
    let mut out = head;
    for (i, (start, end, text)) in cues.into_iter().enumerate() {
        if format == SubtitleFormat::Srt {
            out.push((i + 1).to_string());
        }
        out.push(format!(
            "{} --> {}",
            format_time(start, format),
            format_time(end_of(start, end), format)
        ));
        out.push(text);
        out.push(String::new());
    }
    let joined = out.join("\n");
    format!("{}\n", joined.trim())
}

/// Renders cues as an SRT, WebVTT or ASS file; WebVTT cue text is escaped
/// ([`vtt_escape`]). An ASS file made here has no karaoke (see
/// [`render_file`]).
pub fn render(cues: &[Cue], format: SubtitleFormat) -> String {
    match format {
        SubtitleFormat::Ass => {
            let lines: Vec<CaptionLine> = cues
                .iter()
                .map(|c| CaptionLine {
                    start_ms: c.start_ms,
                    end_ms: c.end_ms,
                    words: vec![c.clone()],
                })
                .collect();
            render_ass(&lines, None, false)
        }
        SubtitleFormat::Srt => write_cues(
            Vec::new(),
            cues.iter().map(|c| (c.start_ms, c.end_ms, c.text.clone())),
            format,
        ),
        SubtitleFormat::Vtt => write_cues(
            vtt_head(false, None),
            cues.iter()
                .map(|c| (c.start_ms, c.end_ms, vtt_escape(&c.text))),
            format,
        ),
    }
}

/// The WebVTT header: `WEBVTT`, then a `STYLE` block underlining spoken
/// words when `past_underline`, then a `NOTE` block from `meta`.
fn vtt_head(past_underline: bool, meta: Option<&CaptionMeta>) -> Vec<String> {
    let mut head = vec!["WEBVTT".to_owned(), String::new()];
    if past_underline {
        head.extend(
            [
                "STYLE",
                "::cue(:past) {",
                "  text-decoration: underline;",
                "}",
                "",
            ]
            .map(str::to_owned),
        );
    }
    if let Some(meta) = meta {
        head.push("NOTE".to_owned());
        head.extend(wrap(&meta.sentences(), 60));
        head.push(String::new());
    }
    head
}

/// Breaks `text` into lines of at most `width` chars at spaces (a longer
/// word stands on its own line).
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// A caption line with a WebVTT timestamp tag before each word after the
/// first. Tags rise strictly and stay inside the cue (WebVTT 4.2.2): a word
/// whose time would not is left untagged.
fn vtt_tagged(line: &CaptionLine) -> String {
    let end = end_of(line.start_ms, line.end_ms);
    let mut prev = line.start_ms;
    let mut s = String::new();
    for (i, w) in line.words.iter().enumerate() {
        if i > 0 {
            s.push(' ');
            let t = w.start_ms.max(prev + 1);
            if t < end {
                s.push('<');
                s.push_str(&format_time(t, SubtitleFormat::Vtt));
                s.push('>');
                prev = t;
            }
        }
        s.push_str(&vtt_escape(&w.text));
    }
    s
}

/// One cue per word of `line`: the whole line with that word in bold and
/// underline.
fn word_line_cues(line: &CaptionLine, format: SubtitleFormat) -> Vec<(u64, u64, String)> {
    let esc = |t: &str| match format {
        SubtitleFormat::Vtt => vtt_escape(t),
        _ => t.to_owned(),
    };
    (0..line.words.len())
        .map(|i| {
            let text: Vec<String> = line
                .words
                .iter()
                .enumerate()
                .map(|(j, w)| {
                    if i == j {
                        format!("<b><u>{}</u></b>", esc(&w.text))
                    } else {
                        esc(&w.text)
                    }
                })
                .collect();
            let w = &line.words[i];
            (w.start_ms, w.end_ms, text.join(" "))
        })
        .collect()
}

/// ASS dialogue text: braces (which start override blocks) become
/// parentheses, and a backslash (which starts `\N` and the like) becomes
/// the look-alike U+29F5, so the text never turns into tags.
fn ass_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '{' => '(',
            '}' => ')',
            '\\' => '\u{29F5}',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect()
}

/// The ASS style: unread words white with no outline; a reached word turns
/// yellow (#FFE14D, High Contrast's word band) and gains a 3 px black
/// outline, a shape cue as well as a color one.
const ASS_HEAD: &str = "ScriptType: v4.00+
PlayResX: 1280
PlayResY: 720

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Reading,Atkinson Hyperlegible Next,48,&H004DE1FF,&H00FFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,3,0,2,64,64,48,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
";

/// Renders caption lines as an ASS script. With `karaoke`, each word gets
/// `\ko` with its duration in centiseconds, taken from rounded absolute
/// times so the durations sum to the line.
fn render_ass(lines: &[CaptionLine], meta: Option<&CaptionMeta>, karaoke: bool) -> String {
    let mut out = String::from("[Script Info]\n");
    let made = meta.map_or_else(|| "Made by textweaver.".to_owned(), CaptionMeta::sentences);
    out.push_str(&format!("; {}\n", ass_escape(&made)));
    if let Some(title) = meta
        .and_then(|m| m.title.as_deref())
        .map(ass_escape)
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
    {
        out.push_str(&format!("Title: {title}\n"));
    }
    out.push_str(ASS_HEAD);
    for line in lines {
        let end = end_of(line.start_ms, line.end_ms);
        let mut text = String::new();
        for (i, w) in line.words.iter().enumerate() {
            if karaoke {
                let next = line.words.get(i + 1).map_or(end, |n| n.start_ms.min(end));
                let d = centis(next).saturating_sub(centis(w.start_ms));
                text.push_str(&format!("{{\\ko{d}}}"));
            }
            text.push_str(&ass_escape(&w.text));
            if i + 1 < line.words.len() {
                text.push(' ');
            }
        }
        out.push_str(&format!(
            "Dialogue: 0,{},{},Reading,,0,0,0,,{text}\n",
            format_time(line.start_ms, SubtitleFormat::Ass),
            format_time(end, SubtitleFormat::Ass)
        ));
    }
    out
}

/// Renders a timeline's captions as a whole file: the cues by `opts`
/// (lines, words, or karaoke), with a WebVTT `NOTE` block or the ASS
/// script's title and comment from `meta` when given. ASS is always karaoke
/// by outline; SRT has no timestamp tags, so `Karaoke::Tags` leaves it
/// plain.
pub fn render_file(
    timeline: &Timeline,
    format: SubtitleFormat,
    opts: &CueOptions,
    meta: Option<&CaptionMeta>,
) -> String {
    if format == SubtitleFormat::Ass {
        return render_ass(&lines(timeline, opts), meta, true);
    }
    let karaoke = if opts.word_level {
        Karaoke::Off
    } else {
        opts.karaoke
    };
    let tags = karaoke == Karaoke::Tags && format == SubtitleFormat::Vtt;
    let head = match format {
        SubtitleFormat::Vtt => vtt_head(tags, meta),
        _ => Vec::new(),
    };
    let escape = |t: &str| match format {
        SubtitleFormat::Vtt => vtt_escape(t),
        _ => t.to_owned(),
    };
    match karaoke {
        Karaoke::Lines => {
            let cues: Vec<(u64, u64, String)> = lines(timeline, opts)
                .iter()
                .flat_map(|l| word_line_cues(l, format))
                .collect();
            write_cues(head, cues, format)
        }
        Karaoke::Tags if tags => write_cues(
            head,
            lines(timeline, opts)
                .iter()
                .map(|l| (l.start_ms, l.end_ms, vtt_tagged(l))),
            format,
        ),
        _ => write_cues(
            head,
            build(timeline, opts)
                .into_iter()
                .map(|c| (c.start_ms, c.end_ms, escape(&c.text))),
            format,
        ),
    }
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
    fn webvtt_cue_text_is_escaped_and_srt_is_not() {
        let cues = [cue(0, 1000, "x < y & z --> w")];
        let vtt = render(&cues, SubtitleFormat::Vtt);
        assert!(vtt.contains("\nx &lt; y &amp; z --&gt; w\n"), "{vtt}");
        // The only arrow left is the timing line's.
        assert_eq!(vtt.matches("-->").count(), 1, "{vtt}");
        let srt = render(&cues, SubtitleFormat::Srt);
        assert!(srt.contains("\nx < y & z --> w\n"), "{srt}");
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
        assert_eq!(
            SubtitleFormat::from_path(Path::new("a.Ass")),
            Some(SubtitleFormat::Ass)
        );
    }

    /// A sentence whose words each take 250 ms from `start` (the recording
    /// double's pace in `docs/audio-export.md`).
    fn paced(text: &str, start: u64) -> TimedSentence {
        let mut words = Vec::new();
        let mut t = start;
        for (at, tok) in token_spans(text) {
            words.push(word(tok, (at, at + tok.chars().count()), t, t + 250));
            t += 250;
        }
        sentence(text, start, t, words)
    }

    /// The research report's "Photosynthesis" fixture.
    fn photosynthesis() -> (Timeline, CaptionMeta) {
        let t = Timeline {
            sentences: vec![
                paced("Plants make food from light.", 0),
                paced("They need water and carbon dioxide.", 1250),
            ],
            duration_ms: 2750,
            title: Some("Photosynthesis".into()),
            ..Timeline::default()
        };
        let meta = CaptionMeta {
            title: t.title.clone(),
            voice: Some("Recording (test double)".into()),
            words_per_minute: Some(240),
            notes: Vec::new(),
        };
        (t, meta)
    }

    #[test]
    fn webvtt_karaoke_tags_match_the_report() {
        let (t, meta) = photosynthesis();
        let opts = CueOptions {
            karaoke: Karaoke::Tags,
            ..CueOptions::default()
        };
        let vtt = render_file(&t, SubtitleFormat::Vtt, &opts, Some(&meta));
        let expected = "WEBVTT

STYLE
::cue(:past) {
  text-decoration: underline;
}

NOTE
Photosynthesis. Read by Recording (test double), 240 words a
minute. Made by textweaver.

00:00:00.000 --> 00:00:01.250
Plants <00:00:00.250>make <00:00:00.500>food <00:00:00.750>from <00:00:01.000>light.

00:00:01.250 --> 00:00:02.750
They <00:00:01.500>need <00:00:01.750>water <00:00:02.000>and <00:00:02.250>carbon <00:00:02.500>dioxide.
";
        assert_eq!(vtt, expected);
        // SRT has no tags: plain lines.
        let srt = render_file(&t, SubtitleFormat::Srt, &opts, Some(&meta));
        assert!(srt.contains("\nPlants make food from light.\n"), "{srt}");
        assert!(!srt.contains('<') && !srt.contains("NOTE"), "{srt}");
    }

    #[test]
    fn timestamp_tags_rise_strictly_inside_the_cue() {
        // Two words at the cue's start and one at its end: the second is
        // nudged a millisecond on, the third is left untagged.
        let words = vec![
            word("a", (0, 1), 1000, 1000),
            word("b", (2, 3), 1000, 1000),
            word("c&", (4, 6), 2000, 2000),
        ];
        let t = Timeline {
            sentences: vec![sentence("a b c&", 1000, 2000, words)],
            duration_ms: 2000,
            ..Timeline::default()
        };
        let opts = CueOptions {
            karaoke: Karaoke::Tags,
            ..CueOptions::default()
        };
        let vtt = render_file(&t, SubtitleFormat::Vtt, &opts, None);
        assert!(
            vtt.ends_with("00:00:01.000 --> 00:00:02.000\na <00:00:01.001>b c&amp;\n"),
            "{vtt}"
        );
        assert!(!vtt.contains("NOTE"));
    }

    #[test]
    fn word_lines_bold_and_underline_one_word_per_cue() {
        let (t, meta) = photosynthesis();
        let opts = CueOptions {
            karaoke: Karaoke::Lines,
            ..CueOptions::default()
        };
        let vtt = render_file(&t, SubtitleFormat::Vtt, &opts, Some(&meta));
        assert!(vtt.contains(
            "\n00:00:00.000 --> 00:00:00.250\n<b><u>Plants</u></b> make food from light.\n\n\
             00:00:00.250 --> 00:00:00.500\nPlants <b><u>make</u></b> food from light.\n"
        ));
        assert_eq!(vtt.matches(" --> ").count(), 11, "{vtt}");
        assert!(!vtt.contains("STYLE"), "only tags need the style block");
        // Escaping applies to the words, never to the tags.
        let amp = Timeline {
            sentences: vec![paced("Salt & pepper", 0)],
            duration_ms: 750,
            ..Timeline::default()
        };
        let vtt = render_file(&amp, SubtitleFormat::Vtt, &opts, None);
        assert!(vtt.contains("Salt <b><u>&amp;</u></b> pepper"), "{vtt}");
        let srt = render_file(&amp, SubtitleFormat::Srt, &opts, None);
        assert!(srt.contains("1\n00:00:00,000 --> 00:00:00,250\n<b><u>Salt</u></b> & pepper"));
    }

    #[test]
    fn word_level_wins_over_karaoke() {
        let (t, _) = photosynthesis();
        let opts = CueOptions {
            word_level: true,
            karaoke: Karaoke::Lines,
            ..CueOptions::default()
        };
        let vtt = render_file(&t, SubtitleFormat::Vtt, &opts, None);
        assert!(
            vtt.contains("\n00:00:00.000 --> 00:00:00.250\nPlants\n"),
            "{vtt}"
        );
        assert!(!vtt.contains("<b>"));
    }

    #[test]
    fn ass_karaoke_matches_the_report() {
        let (t, meta) = photosynthesis();
        let ass = render_file(&t, SubtitleFormat::Ass, &CueOptions::default(), Some(&meta));
        assert!(ass.starts_with(
            "[Script Info]\n; Photosynthesis. Read by Recording (test double), 240 words a minute. Made by textweaver.\nTitle: Photosynthesis\nScriptType: v4.00+\n"
        ), "{ass}");
        assert!(ass.contains("Style: Reading,Atkinson Hyperlegible Next,48,&H004DE1FF,&H00FFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,3,0,2,64,64,48,1\n"));
        assert!(ass.ends_with(
            "Dialogue: 0,0:00:00.00,0:00:01.25,Reading,,0,0,0,,{\\ko25}Plants {\\ko25}make {\\ko25}food {\\ko25}from {\\ko25}light.\n\
             Dialogue: 0,0:00:01.25,0:00:02.75,Reading,,0,0,0,,{\\ko25}They {\\ko25}need {\\ko25}water {\\ko25}and {\\ko25}carbon {\\ko25}dioxide.\n"
        ), "{ass}");
    }

    #[test]
    fn ass_durations_sum_to_the_line_and_text_never_becomes_tags() {
        // Uneven times that do not fall on centiseconds.
        let words = vec![word("{x}", (0, 3), 3, 337), word("a\\N", (4, 7), 337, 1001)];
        let t = Timeline {
            sentences: vec![sentence("{x} a\\N", 3, 1004, words)],
            duration_ms: 1004,
            ..Timeline::default()
        };
        let ass = render_file(&t, SubtitleFormat::Ass, &CueOptions::default(), None);
        let line = ass.lines().last().unwrap_or_default();
        assert_eq!(
            line,
            "Dialogue: 0,0:00:00.00,0:00:01.00,Reading,,0,0,0,,{\\ko34}(x) {\\ko66}a\u{29F5}N"
        );
        assert!(!ass.contains("Title:"));
    }

    #[test]
    fn note_text_is_one_safe_paragraph() {
        let meta = CaptionMeta {
            title: Some("A --> B\n\nC".into()),
            voice: None,
            words_per_minute: None,
            notes: vec!["Machine captions: check before sharing.".into()],
        };
        assert_eq!(
            meta.sentences(),
            "A -> B C. Machine captions: check before sharing. Made by textweaver."
        );
        let vtt = render(&[cue(0, 1000, "hi")], SubtitleFormat::Vtt);
        assert!(!vtt.contains("NOTE"));
        let ass = render(&[cue(0, 1000, "hi")], SubtitleFormat::Ass);
        assert!(ass.ends_with(",Reading,,0,0,0,,hi\n"), "{ass}");
    }
}
