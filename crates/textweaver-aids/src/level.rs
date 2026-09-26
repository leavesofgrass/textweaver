//! Reading level: Flesch-Kincaid grade and Flesch reading ease.
//!
//! Star (`tui/mixin_document.py:225-265`) split the first 50,000 chars on
//! whitespace and on `[.!?]+`, so punctuation-only tokens counted as words,
//! every `Dr.` and `e.g.` ended a sentence, and long documents were judged
//! by their opening pages. textweaver counts the whole document (or a
//! range) with the same word and sentence units navigation uses, so
//! abbreviations do not end sentences and list items and headings are
//! sentences of their own.
//!
//! Syllables are estimated per word by counting vowel groups, with the
//! usual English corrections for silent `e`, `-es`, `-ed`, and `-le`.
//! Tools differ in this estimate, so grades differ between tools by a few
//! tenths; the formulas are the standard ones:
//!
//! - grade = 0.39 × (words / sentences) + 11.8 × (syllables / words) − 15.59
//! - ease = 206.835 − 1.015 × (words / sentences) − 84.6 × (syllables / words)

use serde::{Deserialize, Serialize};
use textweaver_core::{CharRange, Unit};
use textweaver_text::{Document, segments_in};

/// School-level band for a Flesch-Kincaid grade, as Star named them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeBand {
    /// Below grade 6.
    Elementary,
    /// Grades 6 to 8.
    MiddleSchool,
    /// Grades 9 to 12.
    HighSchool,
    /// Grades 13 to 15.
    College,
    /// Grade 16 and above.
    Graduate,
}

impl GradeBand {
    /// The band for `grade` (Star's thresholds: 6, 9, 13, 16).
    pub fn for_grade(grade: f64) -> Self {
        if grade < 6.0 {
            GradeBand::Elementary
        } else if grade < 9.0 {
            GradeBand::MiddleSchool
        } else if grade < 13.0 {
            GradeBand::HighSchool
        } else if grade < 16.0 {
            GradeBand::College
        } else {
            GradeBand::Graduate
        }
    }

    /// The name to show and speak ("middle school").
    pub fn name(self) -> &'static str {
        match self {
            GradeBand::Elementary => "elementary",
            GradeBand::MiddleSchool => "middle school",
            GradeBand::HighSchool => "high school",
            GradeBand::College => "college",
            GradeBand::Graduate => "graduate",
        }
    }
}

/// Counts and scores for a document or range.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReadingLevel {
    /// Words counted.
    pub words: usize,
    /// Sentences that contain at least one word.
    pub sentences: usize,
    /// Estimated syllables.
    pub syllables: usize,
    /// Words of three or more syllables.
    pub polysyllables: usize,
    /// Flesch-Kincaid grade level, unclamped (can be negative for very
    /// simple text).
    pub grade: f64,
    /// Flesch reading ease, unclamped (can leave 0 to 100).
    pub ease: f64,
}

impl ReadingLevel {
    fn from_counts(words: usize, sentences: usize, syllables: usize, polysyllables: usize) -> Self {
        let w = words.max(1) as f64;
        let s = sentences.max(1) as f64;
        let y = syllables as f64;
        ReadingLevel {
            words,
            sentences,
            syllables,
            polysyllables,
            grade: 0.39 * (w / s) + 11.8 * (y / w) - 15.59,
            ease: 206.835 - 1.015 * (w / s) - 84.6 * (y / w),
        }
    }

    /// The grade for display: at least 0, as Star showed it.
    pub fn display_grade(&self) -> f64 {
        self.grade.max(0.0)
    }

    /// The reading ease for display: 0 to 100, as Star showed it.
    pub fn display_ease(&self) -> f64 {
        self.ease.clamp(0.0, 100.0)
    }

    /// The school-level band of the grade.
    pub fn band(&self) -> GradeBand {
        GradeBand::for_grade(self.display_grade())
    }

    /// A sentence to show and speak: "Grade 8.2, middle school. Reading
    /// ease 64 out of 100. 1,234 words in 56 sentences."
    pub fn summary(&self) -> String {
        format!(
            "Grade {:.1}, {}. Reading ease {:.0} out of 100. {} {} in {} {}.",
            self.display_grade(),
            self.band().name(),
            self.display_ease(),
            thousands(self.words),
            if self.words == 1 { "word" } else { "words" },
            thousands(self.sentences),
            if self.sentences == 1 {
                "sentence"
            } else {
                "sentences"
            },
        )
    }
}

fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Estimated syllables in one word. Words without letters (numbers) count
/// as one, as in Star.
pub fn count_syllables(word: &str) -> usize {
    let lower: String = word
        .chars()
        .filter(|c| c.is_alphabetic())
        .flat_map(char::to_lowercase)
        .collect();
    if lower.is_empty() {
        return 1;
    }
    vowel_groups(&lower).max(1)
}

fn is_vowel(c: char) -> bool {
    matches!(
        c,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'y'
            | 'à'
            | 'á'
            | 'â'
            | 'ä'
            | 'è'
            | 'é'
            | 'ê'
            | 'ë'
            | 'ì'
            | 'í'
            | 'î'
            | 'ï'
            | 'ò'
            | 'ó'
            | 'ô'
            | 'ö'
            | 'ù'
            | 'ú'
            | 'û'
            | 'ü'
    )
}

/// Vowel groups after the usual English corrections.
fn vowel_groups(lower: &str) -> usize {
    let chars: Vec<char> = lower.chars().collect();
    let n = chars.len();
    if n <= 3 {
        return 1;
    }
    let mut end = n;
    let before = |i: usize| chars.get(i).copied().unwrap_or(' ');
    if chars.ends_with(&['e', 's']) && !is_vowel(before(n - 3)) && before(n - 3) != 'l' {
        // "makes" -> "mak"; but "tables", "horses" keep their sound below.
        if !matches!(before(n - 3), 's' | 'x' | 'z' | 'c' | 'g' | 'h') {
            end = n - 2;
        }
    } else if chars.ends_with(&['e', 'd']) && !matches!(before(n - 3), 't' | 'd') {
        // "jumped" -> "jump"; "wanted" keeps "-ed".
        if !is_vowel(before(n - 3)) {
            end = n - 2;
        }
    } else if chars.ends_with(&['e']) && !is_vowel(before(n - 2)) && before(n - 2) != 'l' {
        // Silent final e: "make". "-le" is sounded: "table".
        end = n - 1;
    }
    let mut start = 0;
    if chars[0] == 'y' {
        start = 1;
    }
    let mut groups = 0;
    let mut in_group = false;
    for &c in &chars[start..end] {
        let v = is_vowel(c);
        if v && !in_group {
            groups += 1;
        }
        in_group = v;
    }
    groups.max(1)
}

/// The reading level of `range` of `doc`, or `None` when it has no words.
pub fn reading_level(doc: &Document, range: CharRange) -> Option<ReadingLevel> {
    let range = range.clamp_to(doc.len_chars());
    let words = segments_in(doc, Unit::Word, range);
    if words.is_empty() {
        return None;
    }
    let sentences = segments_in(doc, Unit::Sentence, range);
    let (mut syllables, mut poly) = (0usize, 0usize);
    let mut buf = String::new();
    let text = doc.text();
    // Count sentences that contain a word, walking both lists once.
    let mut with_words = 0usize;
    let mut si = 0usize;
    let mut last_counted: Option<usize> = None;
    for w in &words {
        buf.clear();
        for chunk in text.slice(w.to_range()).chunks() {
            buf.push_str(chunk);
        }
        let n = count_syllables(&buf);
        syllables += n;
        if n >= 3 {
            poly += 1;
        }
        while si < sentences.len() && sentences[si].end <= w.start {
            si += 1;
        }
        let key = if si < sentences.len() && sentences[si].start <= w.start {
            si
        } else {
            // A word outside every sentence unit counts as its own.
            usize::MAX - with_words
        };
        if last_counted != Some(key) {
            with_words += 1;
            last_counted = Some(key);
        }
    }
    Some(ReadingLevel::from_counts(
        words.len(),
        with_words,
        syllables,
        poly,
    ))
}

/// The reading level of plain text.
pub fn reading_level_text(text: &str) -> Option<ReadingLevel> {
    let doc = Document::from_plain_text(text);
    reading_level(&doc, doc.full_range())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_estimates() {
        let cases = [
            ("the", 1),
            ("cat", 1),
            ("make", 1),
            ("makes", 1),
            ("table", 2),
            ("tables", 2),
            ("jumped", 1),
            ("wanted", 2),
            ("horses", 2),
            ("water", 2),
            ("beautiful", 3),
            ("reading", 2),
            ("readability", 5),
            ("university", 5),
            ("syllable", 3),
            ("happiness", 3),
            ("education", 4),
            ("yellow", 2),
            ("1990", 1),
            ("don't", 1),
        ];
        let mut wrong = Vec::new();
        for (w, n) in cases {
            if count_syllables(w) != n {
                wrong.push((w, count_syllables(w), n));
            }
        }
        assert!(wrong.is_empty(), "(word, got, expected): {wrong:?}");
    }

    #[test]
    fn simple_text_is_low_grade() {
        let l = reading_level_text("The cat sat on the mat. The dog ran to the cat.").unwrap();
        assert_eq!(l.words, 12);
        assert_eq!(l.sentences, 2);
        assert!(l.display_grade() < 2.0, "{l:?}");
        assert!(l.display_ease() > 90.0, "{l:?}");
        assert_eq!(l.band(), GradeBand::Elementary);
    }

    #[test]
    fn abbreviations_do_not_end_sentences() {
        let l = reading_level_text("Dr. Smith met Mr. Jones at noon. They talked.").unwrap();
        assert_eq!(l.sentences, 2);
    }

    #[test]
    fn dense_text_is_high_grade() {
        let t = "Institutional accountability mechanisms necessitate comprehensive \
                 interdisciplinary evaluation methodologies incorporating quantitative \
                 and qualitative considerations.";
        let l = reading_level_text(t).unwrap();
        assert_eq!(l.sentences, 1);
        assert!(l.grade > 16.0, "{l:?}");
        assert_eq!(l.band(), GradeBand::Graduate);
        assert_eq!(l.display_ease(), 0.0);
    }

    #[test]
    fn summary_reads_well() {
        let l = ReadingLevel::from_counts(1234, 56, 1800, 100);
        let s = l.summary();
        assert!(s.starts_with("Grade "), "{s}");
        assert!(s.contains("1,234 words in 56 sentences."), "{s}");
        assert_eq!(thousands(1_234_567), "1,234,567");
        assert_eq!(thousands(12), "12");
    }

    #[test]
    fn empty_has_no_level() {
        assert!(reading_level_text("").is_none());
        assert!(reading_level_text("  ... !!").is_none());
    }

    #[test]
    fn range_counts_only_its_words() {
        let doc = Document::from_plain_text("One two three. Four five six seven.");
        let l = reading_level(&doc, CharRange::new(15, 35)).unwrap();
        assert_eq!(l.words, 4);
        assert_eq!(l.sentences, 1);
    }
}
