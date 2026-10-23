//! A text in time: the tones to sound, and when each character and word
//! is sent.
//!
//! The text is sent exactly as written: letters in either case, figures,
//! the punctuation Morse code has, and, with prosigns on, a prosign
//! written in angle brackets (`<BT>`). Whitespace separates words. A
//! character Morse code does not have is skipped and counted
//! ([`Schedule::skipped`]); a word with nothing sendable in it sends
//! nothing and takes no time.
//!
//! Times are milliseconds from the start of the first tone. Every
//! schedule that sends anything ends with a word gap after its last tone,
//! so utterances played back to back stay a word apart.

use std::ops::Range;

use crate::table::{Symbol, code, prosign_name};
use crate::timing::Timing;

/// How a text is scheduled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    /// The speeds.
    pub timing: Timing,
    /// Read `<BT>` and the other prosigns written in angle brackets as
    /// prosigns. Off, `<` and `>` are characters Morse code does not
    /// have, and the letters between them are sent as letters.
    pub prosigns: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            timing: Timing::default(),
            prosigns: true,
        }
    }
}

/// One element: the tone is on from `start_ms` to `end_ms`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    /// When the tone starts.
    pub start_ms: f64,
    /// When it stops.
    pub end_ms: f64,
}

/// One character (or prosign) sent.
#[derive(Clone, Debug, PartialEq)]
pub struct SentChar {
    /// What was sent.
    pub symbol: Symbol,
    /// Where it is in the text, in bytes.
    pub byte_range: Range<u32>,
    /// Its first tone's start.
    pub start_ms: f64,
    /// Its last tone's end.
    pub end_ms: f64,
    /// The index of its word in [`Schedule::words`].
    pub word: usize,
}

/// One word sent: a run of text without whitespace that sends something.
#[derive(Clone, Debug, PartialEq)]
pub struct SentWord {
    /// Where it is in the text, in bytes (the whole run, skipped
    /// characters included).
    pub byte_range: Range<u32>,
    /// Its first tone's start.
    pub start_ms: f64,
    /// Its last tone's end.
    pub end_ms: f64,
}

/// A text's tones, characters and words in time.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schedule {
    /// The tones, in order.
    pub tones: Vec<Tone>,
    /// The characters sent, in order.
    pub chars: Vec<SentChar>,
    /// The words sent, in order.
    pub words: Vec<SentWord>,
    /// Characters Morse code does not have, skipped.
    pub skipped: u32,
    /// The whole length: the last tone's end plus a word gap, or 0 when
    /// nothing is sent.
    pub length_ms: f64,
}

/// The symbols of one word: each with its byte range in the text.
fn symbols(text: &str, base: usize, prosigns: bool) -> (Vec<(Symbol, Range<u32>)>, u32) {
    let mut out = Vec::new();
    let mut skipped = 0u32;
    let mut iter = text.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        let at = |a: usize, b: usize| {
            u32::try_from(base + a).unwrap_or(u32::MAX)..u32::try_from(base + b).unwrap_or(u32::MAX)
        };
        if c == '<'
            && prosigns
            && let Some(close) = text[i + 1..].find('>')
            && let Some(name) = prosign_name(&text[i + 1..i + 1 + close])
        {
            let end = i + 1 + close + 1;
            out.push((Symbol::Prosign(name), at(i, end)));
            while iter.peek().is_some_and(|(j, _)| *j < end) {
                iter.next();
            }
            continue;
        }
        let upper = if c.is_ascii_lowercase() {
            c.to_ascii_uppercase()
        } else if c == 'é' {
            'É'
        } else {
            c
        };
        if code(c).is_some() {
            out.push((Symbol::Char(upper), at(i, i + c.len_utf8())));
        } else {
            skipped += 1;
        }
    }
    (out, skipped)
}

/// Schedules `text` (see the module docs).
pub fn schedule(text: &str, options: &Options) -> Schedule {
    let t = options.timing;
    let mut s = Schedule::default();
    // When the next tone may start, and whether anything was sent yet.
    let mut clock = 0.0f64;
    let mut sent_any = false;
    let mut word_start: Option<usize> = None;
    let bytes = text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')));
    for (i, c) in bytes {
        if !c.is_whitespace() {
            word_start.get_or_insert(i);
            continue;
        }
        let Some(start) = word_start.take() else {
            continue;
        };
        let (syms, skipped) = symbols(&text[start..i], start, options.prosigns);
        s.skipped += skipped;
        if syms.is_empty() {
            continue;
        }
        if sent_any {
            clock += t.word_gap_ms;
        }
        sent_any = true;
        let word = s.words.len();
        let word_from = clock;
        for (k, (symbol, byte_range)) in syms.into_iter().enumerate() {
            if k > 0 {
                clock += t.char_gap_ms;
            }
            let char_from = clock;
            for (e, element) in symbol.code().chars().enumerate() {
                if e > 0 {
                    clock += t.dit_ms;
                }
                let len = if element == '-' { t.dah_ms() } else { t.dit_ms };
                s.tones.push(Tone {
                    start_ms: clock,
                    end_ms: clock + len,
                });
                clock += len;
            }
            s.chars.push(SentChar {
                symbol,
                byte_range,
                start_ms: char_from,
                end_ms: clock,
                word,
            });
        }
        s.words.push(SentWord {
            byte_range: u32::try_from(start).unwrap_or(u32::MAX)
                ..u32::try_from(i).unwrap_or(u32::MAX),
            start_ms: word_from,
            end_ms: clock,
        });
    }
    if sent_any {
        s.length_ms = clock + t.word_gap_ms;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn plain(wpm: u16) -> Options {
        Options {
            timing: Timing::new(wpm, wpm),
            prosigns: true,
        }
    }

    #[test]
    fn paris_is_fifty_units() {
        // 20 WPM: a unit is 60 ms; PARIS sounds for 43 units, and its
        // word gap brings it to 50, 3 seconds.
        let s = schedule("PARIS", &plain(20));
        assert!(close(s.words[0].end_ms, 43.0 * 60.0));
        assert!(close(s.length_ms, 3000.0));
        assert_eq!(s.tones.len(), 14);
        assert_eq!(s.chars.len(), 5);
        // P is .--. : tones at 0, 120, 360, 600 ms.
        let starts: Vec<f64> = s.tones[..4].iter().map(|t| t.start_ms).collect();
        for (got, want) in starts.iter().zip([0.0, 120.0, 360.0, 600.0]) {
            assert!(close(*got, want), "{starts:?}");
        }
        // A starts three units after P ends (P ends at 660 ms).
        assert!(close(s.chars[1].start_ms, 840.0));
    }

    #[test]
    fn farnsworth_paris_is_one_tenth_of_a_minute() {
        let s = schedule("paris", &Options::default());
        assert!(close(s.length_ms, 6000.0), "{}", s.length_ms);
    }

    #[test]
    fn words_carry_exact_byte_ranges_and_times() {
        let s = schedule("  ab, c\tE ", &plain(20));
        let ranges: Vec<Range<u32>> = s.words.iter().map(|w| w.byte_range.clone()).collect();
        assert_eq!(ranges, [2..5, 6..7, 8..9]);
        // A is .- (5 units), B is -... (9 units), comma --..-- (19
        // units), with two character gaps: 39 units.
        assert!(close(s.words[0].end_ms, 39.0 * 60.0));
        // The next word starts a word gap later.
        assert!(close(s.words[1].start_ms, 46.0 * 60.0));
        assert!(close(s.words[2].start_ms - s.words[1].end_ms, 420.0));
        let labels: Vec<String> = s.chars.iter().map(|c| c.symbol.label()).collect();
        assert_eq!(labels, ["A", "B", ",", "C", "E"]);
        assert_eq!(s.chars[2].byte_range, 4..5);
        assert_eq!(s.chars[3].word, 1);
    }

    #[test]
    fn unknown_characters_are_skipped_and_counted() {
        let s = schedule("a#b *** ☺", &plain(20));
        assert_eq!(s.skipped, 5);
        assert_eq!(s.words.len(), 1);
        assert_eq!(s.words[0].byte_range, 0..3);
        assert_eq!(s.chars.len(), 2);
        let none = schedule("### ", &plain(20));
        assert_eq!(none.skipped, 3);
        assert!(none.words.is_empty());
        assert!(close(none.length_ms, 0.0));
    }

    #[test]
    fn prosigns_are_one_character() {
        let s = schedule("<BT> Hi <ar>", &plain(20));
        let labels: Vec<String> = s.chars.iter().map(|c| c.symbol.label()).collect();
        assert_eq!(labels, ["<BT>", "H", "I", "<AR>"]);
        assert_eq!(s.chars[0].byte_range, 0..4);
        assert_eq!(s.chars[3].byte_range, 8..12);
        // BT is -...- : 3+1+1+1+1+1+1+1+3 = 13 units, no inner gaps.
        assert!(close(s.chars[0].end_ms, 13.0 * 60.0));
        // Off, the brackets are skipped and the letters sent.
        let off = schedule(
            "<BT>",
            &Options {
                prosigns: false,
                ..plain(20)
            },
        );
        assert_eq!(off.skipped, 2);
        assert_eq!(off.chars.len(), 2);
        // An unknown name is not a prosign.
        let odd = schedule("<QQ>", &plain(20));
        assert_eq!(odd.skipped, 2);
        assert_eq!(odd.chars.len(), 2);
    }

    #[test]
    fn element_gaps_are_one_unit() {
        let s = schedule("S", &plain(10));
        let unit = 120.0;
        for pair in s.tones.windows(2) {
            assert!(close(pair[1].start_ms - pair[0].end_ms, unit));
        }
        for tone in &s.tones {
            assert!(close(tone.end_ms - tone.start_ms, unit));
        }
    }
}
