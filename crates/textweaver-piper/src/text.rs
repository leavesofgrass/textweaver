//! Words, clauses, and synthesis chunks of an utterance's text.
//!
//! eSpeak NG phonemizes a clause at a time and drops the punctuation that
//! ended it; Piper's own phonemizer puts that punctuation back into the
//! phoneme string, because the voices were trained with it (it shapes the
//! pauses and the intonation). This module finds the clauses and their
//! ending punctuation, the words (for word timing), and the chunks the
//! backend synthesizes one at a time.

use std::ops::Range;

/// Characters that end a clause when followed by a space or the end.
const CLAUSE_END: [char; 7] = [',', ';', ':', '.', '!', '?', '…'];

/// Characters that may follow clause punctuation before the space:
/// closing quotes and brackets (`"Hi."`, `(see above).`).
const CLOSERS: [char; 8] = ['"', '\'', ')', ']', '}', '’', '”', '»'];

/// One clause of text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clause {
    /// The clause's text, without its ending punctuation (byte range).
    pub range: Range<usize>,
    /// The punctuation that ended it, as Piper writes it into the phoneme
    /// string (`…` becomes `.`); `None` when the text just ran out.
    pub end: Option<char>,
}

impl Clause {
    /// True when the clause ends a sentence (`.`, `!`, `?`, or the end of
    /// the text).
    pub fn ends_sentence(&self) -> bool {
        matches!(self.end, None | Some('.' | '!' | '?'))
    }

    /// The phonemes Piper appends after this clause's phonemes: the
    /// punctuation, and a space after a comma, colon, or semicolon. A clause
    /// that ended without punctuation gets a full stop, so the voice ends it
    /// with a falling tone.
    pub fn tail(&self) -> &'static str {
        match self.end {
            Some(',') => ", ",
            Some(';') => "; ",
            Some(':') => ": ",
            Some('!') => "!",
            Some('?') => "?",
            _ => ".",
        }
    }
}

/// Splits `text` into clauses at `, ; : . ! ? …` followed by white space
/// (optionally after closing quotes or brackets) or by the end of the
/// text. Clauses with no letters or digits are dropped.
pub fn clauses(text: &str) -> Vec<Clause> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        if !CLAUSE_END.contains(&c) {
            i += 1;
            continue;
        }
        // A run of clause punctuation ("?!", "...") ends one clause.
        let mut j = i;
        while j < chars.len() && CLAUSE_END.contains(&chars[j].1) {
            j += 1;
        }
        let mut k = j;
        while k < chars.len() && CLOSERS.contains(&chars[k].1) {
            k += 1;
        }
        let at_break = k == chars.len() || chars[k].1.is_whitespace();
        if !at_break {
            i = j;
            continue;
        }
        let end = match c {
            '…' => '.',
            c => c,
        };
        push_clause(&mut out, text, start..at, Some(end));
        start = chars.get(k).map_or(text.len(), |&(p, _)| p);
        i = k;
    }
    push_clause(&mut out, text, start..text.len(), None);
    out
}

fn push_clause(out: &mut Vec<Clause>, text: &str, range: Range<usize>, end: Option<char>) {
    let slice = &text[range.clone()];
    if !slice.chars().any(char::is_alphanumeric) {
        // Punctuation after an empty clause still ends the one before.
        if let (Some(last), Some(e)) = (out.last_mut(), end)
            && last.end.is_none()
        {
            last.end = Some(e);
        }
        return;
    }
    let lead = slice.len() - slice.trim_start().len();
    let trail = slice.len() - slice.trim_end().len();
    out.push(Clause {
        range: range.start + lead..range.end - trail,
        end,
    });
}

/// The words of `text`: runs of non-space characters with the
/// punctuation at either end trimmed off, kept only when they hold a
/// letter or digit. Byte ranges, in order.
pub fn words(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let flush = |out: &mut Vec<Range<usize>>, s: usize, e: usize| {
        let w = &text[s..e];
        let lead = w.len() - w.trim_start_matches(|c: char| !c.is_alphanumeric()).len();
        let trail = w.len() - w.trim_end_matches(|c: char| !c.is_alphanumeric()).len();
        if lead + trail < w.len() {
            out.push(s + lead..e - trail);
        }
    };
    for (i, c) in text.char_indices() {
        match (c.is_whitespace(), start) {
            (true, Some(s)) => {
                flush(&mut out, s, i);
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        flush(&mut out, s, text.len());
    }
    out
}

/// Groups clauses into the chunks synthesized one at a time: a sentence
/// each, except that the first clause of a long first sentence is a chunk
/// of its own, so the first audio comes sooner. Returns index ranges into
/// `clauses`.
pub fn chunks(text: &str, clauses: &[Clause]) -> Vec<Range<usize>> {
    /// A first sentence at least this long (bytes) has its first clause
    /// spoken on its own.
    const LONG_FIRST_SENTENCE: usize = 60;
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in clauses.iter().enumerate() {
        let first_sentence = out.is_empty();
        let split_first = first_sentence
            && i == start
            && !c.ends_sentence()
            && sentence_len(text, &clauses[start..]) >= LONG_FIRST_SENTENCE;
        if c.ends_sentence() || split_first {
            out.push(start..i + 1);
            start = i + 1;
        }
    }
    if start < clauses.len() {
        out.push(start..clauses.len());
    }
    out
}

/// Words a phrase may end before: a phrase break before one of them
/// sounds natural ("The library opens at nine | in the morning").
/// Lowercase English; other languages find no break and are never split.
const PHRASE_STARTERS: [&str; 32] = [
    "about", "after", "and", "at", "because", "before", "between", "but", "by", "during", "for",
    "from", "if", "in", "into", "on", "or", "over", "since", "so", "that", "through", "under",
    "unless", "until", "when", "where", "which", "while", "who", "with", "without",
];

/// The fewest words before a phrase break, and after it.
const PHRASE_MIN_WORDS: usize = 4;

/// The most words before a phrase break.
const PHRASE_MAX_WORDS: usize = 10;

/// Where to split a long `clause` of `text` into a short first phrase
/// and the rest: the byte where the rest starts, before the first word
/// that starts a phrase ("and", "but", "in", "with", "which", and other
/// common conjunctions and prepositions) with at least four words before
/// it (at most ten) and four after it. `None` when the clause is too short or has no
/// such word. The backend speaks the first phrase alone when a reading
/// starts, so the first audio comes after a few words of synthesis, not a
/// whole sentence; it ends with a comma's intonation.
pub fn phrase_break(text: &str, clause: &Clause) -> Option<usize> {
    let slice = text.get(clause.range.clone())?;
    let ws = words(slice);
    if ws.len() < 2 * PHRASE_MIN_WORDS {
        return None;
    }
    let last = PHRASE_MAX_WORDS.min(ws.len() - PHRASE_MIN_WORDS);
    (PHRASE_MIN_WORDS..=last)
        .find(|&k| {
            let w = &slice[ws[k].clone()];
            // A word that starts a phrase is plain: no punctuation joins
            // it to the word before ("rock-and-roll").
            let before = &slice[ws[k - 1].end..ws[k].start];
            before.chars().all(char::is_whitespace)
                && PHRASE_STARTERS.contains(&w.to_lowercase().as_str())
        })
        .map(|k| clause.range.start + ws[k].start)
}

/// Length in bytes of the sentence that starts at `clauses[0]`.
fn sentence_len(text: &str, clauses: &[Clause]) -> usize {
    let Some(first) = clauses.first() else {
        return 0;
    };
    let end = clauses
        .iter()
        .find(|c| c.ends_sentence())
        .or(clauses.last())
        .map_or(first.range.end, |c| c.range.end);
    text.get(first.range.start..end).map_or(0, str::len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts<'a>(text: &'a str, cs: &[Clause]) -> Vec<(&'a str, Option<char>)> {
        cs.iter().map(|c| (&text[c.range.clone()], c.end)).collect()
    }

    #[test]
    fn clauses_end_at_punctuation_before_a_space() {
        let t = "Hello, world. Is 3.14 pi? \"Yes!\" she said";
        let cs = clauses(t);
        assert_eq!(
            texts(t, &cs),
            vec![
                ("Hello", Some(',')),
                ("world", Some('.')),
                ("Is 3.14 pi", Some('?')),
                ("\"Yes", Some('!')),
                ("she said", None),
            ]
        );
        assert!(cs[1].ends_sentence() && !cs[0].ends_sentence() && cs[4].ends_sentence());
        assert_eq!(cs[0].tail(), ", ");
        assert_eq!(cs[2].tail(), "?");
        assert_eq!(cs[4].tail(), ".");
    }

    #[test]
    fn runs_of_punctuation_end_one_clause() {
        let t = "Wait... what?! Fine";
        assert_eq!(
            texts(t, &clauses(t)),
            vec![("Wait", Some('.')), ("what", Some('?')), ("Fine", None)]
        );
        let t = "So… yes";
        assert_eq!(
            texts(t, &clauses(t)),
            vec![("So", Some('.')), ("yes", None)]
        );
    }

    #[test]
    fn empty_clauses_are_dropped() {
        assert!(clauses("").is_empty());
        assert!(clauses(" ... ").is_empty());
        let t = "Hi - . there";
        assert_eq!(
            texts(t, &clauses(t)),
            vec![("Hi -", Some('.')), ("there", None)]
        );
    }

    #[test]
    fn words_trim_punctuation() {
        let t = "\"Hello,\" said Émile's   dog—really? 42% (x)";
        let w: Vec<&str> = words(t).into_iter().map(|r| &t[r]).collect();
        assert_eq!(w, vec!["Hello", "said", "Émile's", "dog—really", "42", "x"]);
        assert!(words(" -- ").is_empty());
    }

    #[test]
    fn chunks_are_sentences_with_a_fast_first_clause() {
        let t = "Short one. Then a second sentence, with two clauses.";
        let cs = clauses(t);
        assert_eq!(chunks(t, &cs), vec![0..1, 1..3]);
        let t = "A much longer first sentence goes here, and it keeps going on and on. Next.";
        let cs = clauses(t);
        assert_eq!(chunks(t, &cs), vec![0..1, 1..2, 2..3]);
        let t = "no punctuation at all";
        assert_eq!(chunks(t, &clauses(t)), vec![0..1]);
        assert!(chunks("", &[]).is_empty());
    }

    #[test]
    fn a_long_clause_breaks_before_a_phrase() {
        let t = "The library opens early at nine in the morning and closes at six.";
        let cs = clauses(t);
        let at = phrase_break(t, &cs[0]).unwrap();
        assert_eq!(&t[at..], "at nine in the morning and closes at six.");
        // Too short to split, or nowhere natural to split.
        let t = "The library opens at nine.";
        assert_eq!(phrase_break(t, &clauses(t)[0]), None);
        let t = "Highlight résumé writer code heading drifts markdown reliable list code.";
        assert_eq!(phrase_break(t, &clauses(t)[0]), None);
        // At least four words stay after the break.
        let t = "One two three four five six seven and eight nine.";
        assert_eq!(phrase_break(t, &clauses(t)[0]), None);
        // Joined words are one word ("salt-and-pepper"), and a quoted
        // word is not a phrase start.
        let t = "Some very old salt-and-pepper hair grew back in the early spring.";
        let at = phrase_break(t, &clauses(t)[0]).unwrap();
        assert_eq!(&t[at..], "in the early spring.");
        let t = "Some very old hair said \"in\" the early spring sun.";
        assert_eq!(phrase_break(t, &clauses(t)[0]), None);
    }

    proptest::proptest! {
        #[test]
        fn clause_and_word_ranges_are_valid(t in "\\PC{0,60}") {
            for c in clauses(&t) {
                proptest::prop_assert!(t.get(c.range.clone()).is_some());
                proptest::prop_assert!(c.range.start <= c.range.end);
            }
            let mut last = 0;
            for w in words(&t) {
                proptest::prop_assert!(t.get(w.clone()).is_some());
                proptest::prop_assert!(w.start >= last && w.start < w.end);
                last = w.end;
            }
            let cs = clauses(&t);
            let ch = chunks(&t, &cs);
            let covered: usize = ch.iter().map(|r| r.len()).sum();
            proptest::prop_assert_eq!(covered, cs.len());
        }
    }
}
