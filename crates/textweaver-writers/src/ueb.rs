//! Unified English Braille, grade 1 (uncontracted), in North American
//! Braille ASCII.
//!
//! Braille ASCII writes each six-dot cell as one character from `' '` to
//! `'_'` (letters as capitals), the encoding BRF files and embossers use.
//! [`to_unicode`] turns it into Unicode braille patterns (U+2800 block) for
//! display, and [`from_unicode`] back.
//!
//! What the translator does (UEB Rules of Unified English Braille, 2013):
//!
//! - letters as themselves; a capital letter takes the capital indicator
//!   (dot 6, `,`); a run of two or more capitals takes the capitals word
//!   indicator (`,,`), and the capitals terminator (`,'`) when lowercase
//!   letters follow in the same word ("CDs" is `,,CD,'S`);
//! - digits take the numeric indicator (`#`) and the letters a to j; numeric
//!   mode continues through a decimal point or comma between digits and a
//!   fraction slash between digits, and a letter a to j directly after a
//!   number takes the grade 1 indicator (`;`) so "3a" is not read as "31";
//! - punctuation and symbols by the UEB tables, with directional quotation
//!   marks chosen by position for straight quotes (`"` opens at the start of
//!   a word, closes elsewhere; `'` is an apostrophe inside or at the end of a
//!   word);
//! - accented Latin letters as a modifier and the base letter (é is `^/E`),
//!   Greek letters with the Greek indicator (dot 4-6, `.`);
//! - characters without a UEB symbol are left out and reported in
//!   [`Translation::unsupported`];
//! - a passage of three or more fully capitalized symbols-sequences takes
//!   the capitals passage indicator (`,,,`) before its first capital and
//!   the capitals terminator (`,'`) after its last capitalized
//!   sequence, closing punctuation opened before the passage kept outside
//!   (Rules 8.5, 8.6). To keep the usual braille form of single letters
//!   (Rule 8.8.1), a passage needs at least two sequences of two or more
//!   capitals: "A B C" stays `,A ,B ,C`;
//! - typeforms (italic, bold, underline) marked with [`Typeform::open`] and
//!   [`Typeform::close`] take their indicators (Rules 9.2 to 9.8): the
//!   symbol indicator for one symbol, the word indicator for each of one
//!   or two symbols-sequences, with the terminator only where letters or
//!   digits follow in the same sequence (9.4.4, 9.7.3), and the passage
//!   indicator and terminator for three or more sequences, the terminator
//!   after punctuation that follows the passage (9.7.2); indicators nest,
//!   typeform outside capitals (8.6.2, 9.7.1, 9.8.1).
//!
//! Section numbers are those of The Rules of Unified English Braille,
//! second edition 2013 (International Council on English Braille), whose
//! examples the tests use in their grade 1 form.
//!
//! Not done: contractions (grade 2 goes through liblouis), and typeform or
//! capitals passages continued across paragraphs (Rules 8.5.5 and 9.9):
//! each text is its own element.

/// A translation: braille ASCII plus what could not be translated.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Translation {
    /// The braille, in braille ASCII; spaces and `\n` kept.
    pub braille: String,
    /// Characters left out because UEB grade 1 has no symbol for them
    /// here, each once, in order of first appearance.
    pub unsupported: Vec<char>,
}

/// Braille ASCII for dot patterns 0 to 63 (bit 0 is dot 1, bit 5 is dot 6).
const BRAILLE_ASCII: &[u8; 64] =
    b" A1B'K2L@CIF/MSP\"E3H9O6R^DJG>NTQ,*5<-U8V.%[$+X!&;:4\\0Z7(_?W]#Y)=";

/// Braille ASCII to Unicode braille patterns. Lowercase letters and the
/// lowercase-range symbols `` ` { | } ~ `` are accepted too; other chars
/// pass through.
pub fn to_unicode(ascii: &str) -> String {
    ascii
        .chars()
        .map(|c| {
            let up = match c {
                'a'..='z' => c.to_ascii_uppercase(),
                '`' => '@',
                '{' => '[',
                '|' => '\\',
                '}' => ']',
                '~' => '^',
                _ => c,
            };
            match BRAILLE_ASCII.iter().position(|&b| char::from(b) == up) {
                Some(dots) => char::from_u32(0x2800 + dots as u32).unwrap_or(c),
                None => c,
            }
        })
        .collect()
}

/// Unicode braille patterns (six-dot, U+2800 to U+283F) to braille ASCII;
/// other chars pass through.
pub fn from_unicode(s: &str) -> String {
    s.chars()
        .map(|c| {
            let v = c as u32;
            if (0x2800..0x2840).contains(&v) {
                char::from(BRAILLE_ASCII[(v - 0x2800) as usize])
            } else {
                c
            }
        })
        .collect()
}

/// A print typeform braille shows with its own indicators (UEB Rules,
/// section 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Typeform {
    /// Italic: indicators with the prefix dots 4-6 (`.`).
    Italic,
    /// Bold: indicators with the prefix dots 4-5 (`^`).
    Bold,
    /// Underline: indicators with the prefix dots 4-5-6 (`_`).
    Underline,
}

impl Typeform {
    const ALL: [Typeform; 3] = [Typeform::Italic, Typeform::Bold, Typeform::Underline];

    /// The mark that opens this typeform in text given to [`translate`]
    /// (a private-use character).
    pub fn open(self) -> char {
        match self {
            Typeform::Italic => '\u{E020}',
            Typeform::Bold => '\u{E022}',
            Typeform::Underline => '\u{E024}',
        }
    }

    /// The mark that closes this typeform in text given to [`translate`].
    pub fn close(self) -> char {
        match self {
            Typeform::Italic => '\u{E021}',
            Typeform::Bold => '\u{E023}',
            Typeform::Underline => '\u{E025}',
        }
    }

    /// The typeform and whether it opens, for a mark.
    fn of_mark(c: char) -> Option<(Typeform, bool)> {
        Typeform::ALL.into_iter().find_map(|t| {
            if c == t.open() {
                Some((t, true))
            } else if c == t.close() {
                Some((t, false))
            } else {
                None
            }
        })
    }

    fn index(self) -> usize {
        match self {
            Typeform::Italic => 0,
            Typeform::Bold => 1,
            Typeform::Underline => 2,
        }
    }

    /// The indicator's first cell (Rule 9.2 to 9.4).
    fn prefix(self) -> char {
        match self {
            Typeform::Italic => '.',
            Typeform::Bold => '^',
            Typeform::Underline => '_',
        }
    }
}

/// The mark for an empty table entry in text given to [`translate`]:
/// three guide dots (dot 5, `"""`), as BANA's Braille Formats (2016,
/// 11.16 and 11.18) writes a blank entry.
pub const BLANK_ENTRY: char = '\u{E030}';

/// `text` without typeform marks, and with [`BLANK_ENTRY`] left out: the
/// print text for a translator that does not know them.
pub fn strip_marks(text: &str) -> String {
    text.chars()
        .filter(|&c| Typeform::of_mark(c).is_none() && c != BLANK_ENTRY)
        .collect()
}

/// Translates print text to UEB grade 1 braille ASCII.
///
/// Typeform marks ([`Typeform::open`], [`Typeform::close`]) become the
/// typeform indicators; a mark left open runs to the end of the text, and a
/// close without an open runs from its start (a span divided between two
/// lines).
pub fn translate(text: &str) -> Translation {
    let mut chars: Vec<char> = Vec::with_capacity(text.len());
    let mut runs: Vec<Run> = Vec::new();
    let mut open: [Option<(usize, usize)>; 3] = [None; 3];
    for c in text.chars() {
        if matches!(
            c,
            '\r' | '\u{AD}' | '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}'
        ) {
            continue;
        }
        let Some((form, opens)) = Typeform::of_mark(c) else {
            chars.push(c);
            continue;
        };
        let slot = &mut open[form.index()];
        match (*slot, opens) {
            (None, true) => *slot = Some((chars.len(), 1)),
            (Some((s, d)), true) => *slot = Some((s, d + 1)),
            (Some((s, 1)), false) => {
                runs.push(Run {
                    form,
                    start: s,
                    end: chars.len(),
                });
                *slot = None;
            }
            (Some((s, d)), false) => *slot = Some((s, d - 1)),
            (None, false) => runs.push(Run {
                form,
                start: 0,
                end: chars.len(),
            }),
        }
    }
    for form in Typeform::ALL {
        if let Some((s, _)) = open[form.index()] {
            runs.push(Run {
                form,
                start: s,
                end: chars.len(),
            });
        }
    }
    let plan = Plan::new(&chars, runs);
    let mut t = Translator {
        chars: &chars,
        out: String::with_capacity(text.len() + text.len() / 4),
        unsupported: Vec::new(),
        numeric: false,
        single_open: false,
        plan,
    };
    t.run();
    Translation {
        braille: t.out,
        unsupported: t.unsupported,
    }
}

/// A stretch of print in one typeform, as char indices (end exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    form: Typeform,
    start: usize,
    end: usize,
}

/// An indicator written before a char: what opened it and where it ends,
/// so indicators at one place can be nested (Rules 8.6.2, 9.7.1, 9.8.1).
#[derive(Clone, Debug)]
struct Insert {
    cells: String,
    closes: bool,
    /// Where the indicated stretch starts and ends.
    start: usize,
    end: usize,
    /// Nesting order for stretches that open at the same place: italic,
    /// bold, underline, then capitals (typeform outside capitals, as in the
    /// Rules' 8.6.2 example `.7,,,romeo & juliet,'.'`).
    rank: u8,
}

/// Indicators to write, by char index, and the chars inside a capitals
/// passage.
struct Plan {
    inserts: Vec<Vec<Insert>>,
    in_caps_passage: Vec<bool>,
}

/// A symbols-sequence: a stretch of print between spaces (char indices).
#[derive(Clone, Copy, Debug)]
struct Seq {
    start: usize,
    end: usize,
}

fn sequences(chars: &[char]) -> Vec<Seq> {
    let mut seqs = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        seqs.push(Seq { start, end: i });
    }
    seqs
}

/// The kind of a symbols-sequence for capitals passages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    /// Letters, all capitals; with the count of capitals.
    Capitals(usize),
    /// Some lowercase letter.
    Lower,
    /// No letters with case (numbers, punctuation).
    Neutral,
}

fn case_of(chars: &[char]) -> Case {
    let mut upper = 0;
    for &c in chars {
        if c.is_lowercase() && is_letter(c) {
            return Case::Lower;
        }
        if c.is_uppercase() {
            upper += 1;
        }
    }
    if upper > 0 {
        Case::Capitals(upper)
    } else {
        Case::Neutral
    }
}

/// `end`, moved back over closing punctuation whose opening mark is not in
/// `chars[start..end]`, so the terminator nests inside it (Rules 8.6.2 and
/// 9.7.1: `8,,,I will not6,'0`).
fn nest_end(chars: &[char], start: usize, end: usize, floor: usize) -> usize {
    let mut depth = [0i32; 4];
    for (i, &c) in chars[start..end].iter().enumerate() {
        let at = start + i;
        let slot = match c {
            '(' | ')' => 0,
            '[' | ']' => 1,
            '{' | '}' => 2,
            '"' | '\u{201C}' | '\u{201D}' | '«' | '»' => 3,
            _ => continue,
        };
        let opens = match c {
            '(' | '[' | '{' | '\u{201C}' | '«' => true,
            // A straight quote opens at the start of a word.
            '"' => at == 0 || chars[at - 1].is_whitespace() || matches!(chars[at - 1], '(' | '['),
            _ => false,
        };
        depth[slot] += if opens { 1 } else { -1 };
    }
    let mut e = end;
    while e > floor {
        let c = chars[e - 1];
        let slot = match c {
            ')' => 0,
            ']' => 1,
            '}' => 2,
            '"' | '\u{201D}' | '»' => 3,
            _ => break,
        };
        if depth[slot] >= 0 {
            break;
        }
        depth[slot] += 1;
        e -= 1;
    }
    e
}

impl Plan {
    fn new(chars: &[char], runs: Vec<Run>) -> Plan {
        let mut plan = Plan {
            inserts: vec![Vec::new(); chars.len() + 1],
            in_caps_passage: vec![false; chars.len()],
        };
        let seqs = sequences(chars);
        plan.capitals(chars, &seqs);
        for run in merge_runs(chars, runs) {
            plan.typeform(chars, &seqs, run);
        }
        for at in &mut plan.inserts {
            // Closers first, the last opened first; then openers, the
            // widest first.
            at.sort_by(|a, b| match (a.closes, b.closes) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (true, true) => b.start.cmp(&a.start).then(b.rank.cmp(&a.rank)),
                (false, false) => b.end.cmp(&a.end).then(a.rank.cmp(&b.rank)),
            });
        }
        plan
    }

    fn add(&mut self, at: usize, cells: String, closes: bool, start: usize, end: usize, rank: u8) {
        self.inserts[at].push(Insert {
            cells,
            closes,
            start,
            end,
            rank,
        });
    }

    /// Capitals passages (Rules 8.5, 8.6): three or more sequences in
    /// capitals, numbers and punctuation allowed between them.
    fn capitals(&mut self, chars: &[char], seqs: &[Seq]) {
        let mut i = 0;
        while i < seqs.len() {
            if !matches!(
                case_of(&chars[seqs[i].start..seqs[i].end]),
                Case::Capitals(_)
            ) {
                i += 1;
                continue;
            }
            // Extend over capitals and neutral sequences; end on the last
            // sequence in capitals.
            let first = i;
            let mut last = i;
            let mut words = 0;
            let mut j = i;
            while j < seqs.len() {
                match case_of(&chars[seqs[j].start..seqs[j].end]) {
                    Case::Capitals(n) => {
                        last = j;
                        if n >= 2 {
                            words += 1;
                        }
                    }
                    Case::Neutral => {}
                    Case::Lower => break,
                }
                j += 1;
            }
            if last - first + 1 >= 3 && words >= 2 {
                let start = (seqs[first].start..seqs[first].end)
                    .find(|&k| chars[k].is_uppercase())
                    .unwrap_or(seqs[first].start);
                let end = nest_end(chars, start, seqs[last].end, start + 1);
                self.add(start, ",,,".into(), false, start, end, 3);
                self.add(end, ",'".into(), true, start, end, 3);
                for k in start..end {
                    self.in_caps_passage[k] = true;
                }
            }
            i = last + 1;
        }
    }

    /// One typeform stretch (Rules 9.2 to 9.4, 9.7).
    fn typeform(&mut self, chars: &[char], seqs: &[Seq], run: Run) {
        let p = run.form.prefix();
        let rank = run.form.index() as u8;
        let touched: Vec<Seq> = seqs
            .iter()
            .copied()
            .filter(|s| s.start < run.end && s.end > run.start)
            .collect();
        let Some(last) = touched.last().copied() else {
            return;
        };
        if touched.len() >= 3 {
            // A passage: the terminator after the last affected symbol,
            // punctuation right after it counted in unless letters follow
            // (9.7.2; the hyphen, dash and ellipsis are not).
            let mut end = run.end;
            while end < last.end
                && !chars[end].is_alphanumeric()
                && !matches!(
                    chars[end],
                    '-' | '\u{2010}' | '\u{2013}' | '\u{2014}' | '\u{2026}'
                )
            {
                end += 1;
            }
            if chars[end..last.end].iter().any(|c| c.is_alphanumeric()) {
                end = run.end;
            }
            let end = nest_end(chars, run.start, end, run.end);
            self.add(run.start, format!("{p}7"), false, run.start, end, rank);
            self.add(end, format!("{p}'"), true, run.start, end, rank);
            return;
        }
        for s in touched {
            let a = run.start.max(s.start);
            let b = run.end.min(s.end);
            if b - a == 1 {
                self.add(a, format!("{p}2"), false, a, b, rank);
                continue;
            }
            self.add(a, format!("{p}1"), false, a, b, rank);
            // Closing punctuation needs no terminator (9.7.3); letters or
            // digits after the stretch in the same sequence do (9.4.4).
            if chars[b..s.end].iter().any(|c| c.is_alphanumeric()) {
                self.add(b, format!("{p}'"), true, a, b, rank);
            }
        }
    }
}

/// Runs trimmed of spaces, and runs of one typeform joined when only
/// spaces lie between them ("*one* *two* *three*" is one passage).
fn merge_runs(chars: &[char], mut runs: Vec<Run>) -> Vec<Run> {
    for r in &mut runs {
        while r.start < r.end && chars[r.start].is_whitespace() {
            r.start += 1;
        }
        while r.end > r.start && chars[r.end - 1].is_whitespace() {
            r.end -= 1;
        }
    }
    runs.retain(|r| r.start < r.end);
    runs.sort_by_key(|r| (r.form.index(), r.start));
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for r in runs {
        if let Some(prev) = out.last_mut()
            && prev.form == r.form
            && chars[prev.end.min(r.start)..r.start]
                .iter()
                .all(|c| c.is_whitespace())
        {
            prev.end = prev.end.max(r.end);
            continue;
        }
        out.push(r);
    }
    out
}

struct Translator<'a> {
    chars: &'a [char],
    out: String,
    unsupported: Vec<char>,
    /// In numeric mode (after a numeric indicator, until a space or a
    /// non-numeric symbol).
    numeric: bool,
    /// A single quotation mark is open.
    single_open: bool,
    plan: Plan,
}

/// A letter's UEB form: optional prefix (modifier or Greek indicator) and the
/// base letter (lowercase), or a two-letter fold.
enum Letter {
    Plain(char),
    Prefixed(&'static str, char),
    Fold(&'static str),
}

fn letter(c: char) -> Option<Letter> {
    let lower = c.to_lowercase().next().unwrap_or(c);
    if lower.is_ascii_lowercase() {
        return Some(Letter::Plain(lower));
    }
    let accented = |m: &'static str, base: char| Some(Letter::Prefixed(m, base));
    match lower {
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' | 'ć' | 'ń' | 'ś' | 'ź' => {
            accented("^/", base_of(lower))
        }
        'à' | 'è' | 'ì' | 'ò' | 'ù' => accented("^*", base_of(lower)),
        'â' | 'ê' | 'î' | 'ô' | 'û' => accented("^%", base_of(lower)),
        'ã' | 'ñ' | 'õ' => accented("^]", base_of(lower)),
        'ä' | 'ë' | 'ï' | 'ö' | 'ü' | 'ÿ' => accented("^3", base_of(lower)),
        'ç' => accented("^&", 'c'),
        'å' => accented("^$", 'a'),
        'ß' => Some(Letter::Fold("ss")),
        'æ' => Some(Letter::Fold("ae")),
        'œ' => Some(Letter::Fold("oe")),
        'ø' => Some(Letter::Plain('o')),
        'ł' => Some(Letter::Plain('l')),
        'đ' => Some(Letter::Plain('d')),
        'ı' => Some(Letter::Plain('i')),
        // Greek, with the Greek indicator (dots 4-6).
        'α' => accented(".", 'a'),
        'β' => accented(".", 'b'),
        'γ' => accented(".", 'g'),
        'δ' => accented(".", 'd'),
        'ε' => accented(".", 'e'),
        'ζ' => accented(".", 'z'),
        'η' => accented(".", ':'),
        'θ' => accented(".", '?'),
        'ι' => accented(".", 'i'),
        'κ' => accented(".", 'k'),
        'λ' => accented(".", 'l'),
        'μ' => accented(".", 'm'),
        'ν' => accented(".", 'n'),
        'ξ' => accented(".", 'x'),
        'ο' => accented(".", 'o'),
        'π' => accented(".", 'p'),
        'ρ' => accented(".", 'r'),
        'σ' | 'ς' => accented(".", 's'),
        'τ' => accented(".", 't'),
        'υ' => accented(".", 'u'),
        'φ' => accented(".", 'f'),
        'χ' => accented(".", '&'),
        'ψ' => accented(".", 'y'),
        'ω' => accented(".", 'w'),
        _ => None,
    }
}

fn base_of(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'ı' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        'ñ' | 'ń' => 'n',
        'ç' | 'ć' => 'c',
        'ś' => 's',
        'ź' => 'z',
        other => other,
    }
}

fn is_letter(c: char) -> bool {
    letter(c).is_some()
}

/// UEB symbol for a punctuation mark or sign that needs no context.
fn symbol(c: char) -> Option<&'static str> {
    Some(match c {
        '.' => "4",
        ',' => "1",
        ';' => "2",
        ':' => "3",
        '!' => "6",
        '?' => "8",
        '-' | '\u{2010}' | '\u{2011}' => "-",
        '\u{2013}' => ",-",
        '\u{2014}' | '\u{2015}' => "\",-",
        '\u{2212}' => "\"-",
        '(' => "\"<",
        ')' => "\">",
        '[' => ".<",
        ']' => ".>",
        '{' => "_<",
        '}' => "_>",
        '/' => "_/",
        '\\' => "_*",
        '&' => "@&",
        '*' => "\"9",
        '#' => "_?",
        '%' => ".0",
        '@' => "@A",
        '$' => "@S",
        '+' => "\"6",
        '=' => "\"7",
        '<' => "@<",
        '>' => "@>",
        '~' => "@9",
        '_' => ".-",
        '|' => "_\\",
        '^' => "@5",
        '`' => "@*",
        '\u{2026}' => "444",
        '\u{2022}' => "_4",
        '°' => "^J",
        '©' => "^C",
        '®' => "^R",
        '™' => "^T",
        '§' => "^S",
        '¶' => "@P",
        '€' => "@E",
        '£' => "@L",
        '¢' => "@C",
        '¥' => "@Y",
        '×' => "\"8",
        '÷' => "\"/",
        '\u{201C}' | '\u{201E}' | '«' => "8",
        '\u{201D}' | '»' => "0",
        '\u{2018}' => ",8",
        _ => return None,
    })
}

impl Translator<'_> {
    fn prev(&self, i: usize) -> Option<char> {
        i.checked_sub(1).map(|j| self.chars[j])
    }

    fn next(&self, i: usize) -> Option<char> {
        self.chars.get(i + 1).copied()
    }

    /// True when position `i` begins a word: start of text, after
    /// whitespace, or after an opening bracket, dash, or quote.
    fn word_start(&self, i: usize) -> bool {
        match self.prev(i) {
            None => true,
            Some(p) => {
                p.is_whitespace()
                    || matches!(
                        p,
                        '(' | '[' | '{' | '\u{2014}' | '\u{2013}' | '"' | '\u{201C}' | '\u{2018}'
                    )
            }
        }
    }

    fn unsupported(&mut self, c: char) {
        if !self.unsupported.contains(&c) {
            self.unsupported.push(c);
        }
    }

    /// Writes the indicators planned before char `i`. An indicator ends
    /// numeric mode, so a digit after it takes the numeric indicator again
    /// (the Rules' 9.2 example `#e^2#e`).
    fn indicators(&mut self, i: usize) {
        let Some(at) = self.plan.inserts.get(i) else {
            return;
        };
        if at.is_empty() {
            return;
        }
        for ins in at {
            self.out.push_str(&ins.cells);
        }
        self.numeric = false;
    }

    fn run(&mut self) {
        let mut i = 0;
        while i < self.chars.len() {
            let c = self.chars[i];
            self.indicators(i);
            if c.is_whitespace() {
                self.out.push(if c == '\n' { '\n' } else { ' ' });
                self.numeric = false;
                if c == '\n' {
                    self.single_open = false;
                }
                i += 1;
            } else if c.is_ascii_digit() {
                if !self.numeric {
                    self.out.push('#');
                    self.numeric = true;
                }
                self.out.push(digit(c));
                i += 1;
            } else if self.numeric
                && matches!(c, '.' | ',' | '/')
                && self.next(i).is_some_and(|n| n.is_ascii_digit())
            {
                // Decimal point, digit-group comma, or simple fraction line
                // inside a number.
                self.out.push(match c {
                    '.' => '4',
                    ',' => '1',
                    _ => '/',
                });
                i += 1;
            } else if is_letter(c) {
                i = self.letters(i);
            } else if c == BLANK_ENTRY {
                self.numeric = false;
                self.out.push_str("\"\"\"");
                i += 1;
            } else {
                self.numeric = false;
                self.punctuation(i, c);
                i += 1;
            }
        }
        self.indicators(self.chars.len());
    }

    /// Translates the run of letters starting at `start`; returns its end.
    /// The run's own indicators are written by the caller; the run stops
    /// before a char with indicators of its own.
    fn letters(&mut self, start: usize) -> usize {
        let mut end = start + 1;
        while end < self.chars.len()
            && is_letter(self.chars[end])
            && self.plan.inserts[end].is_empty()
        {
            end += 1;
        }
        let first = self.chars[start];
        if self.numeric && first.is_lowercase() {
            let base = match letter(first) {
                Some(Letter::Plain(b)) => Some(b),
                Some(Letter::Fold(f)) => f.chars().next(),
                _ => None,
            };
            if base.is_some_and(|b| ('a'..='j').contains(&b)) {
                // Grade 1 indicator: "3a" is not "31".
                self.out.push(';');
            }
        }
        self.numeric = false;
        // Split the run into same-case segments.
        let mut i = start;
        let mut caps_word_open = false;
        while i < end {
            let upper = self.chars[i].is_uppercase();
            let mut j = i;
            while j < end && self.chars[j].is_uppercase() == upper {
                j += 1;
            }
            if upper && self.plan.in_caps_passage[i] {
                // The passage indicator is already written (Rule 8.5).
                for k in i..j {
                    self.letter_cells(self.chars[k], false);
                }
            } else if upper {
                if j - i >= 2 {
                    self.out.push_str(",,");
                    caps_word_open = true;
                    for k in i..j {
                        self.letter_cells(self.chars[k], false);
                    }
                } else {
                    self.letter_cells(self.chars[i], true);
                }
            } else {
                if caps_word_open {
                    self.out.push_str(",'");
                    caps_word_open = false;
                }
                for k in i..j {
                    self.letter_cells(self.chars[k], false);
                }
            }
            i = j;
        }
        end
    }

    fn letter_cells(&mut self, c: char, capital: bool) {
        if capital {
            self.out.push(',');
        }
        match letter(c) {
            Some(Letter::Plain(b)) => self.out.push(b.to_ascii_uppercase()),
            Some(Letter::Prefixed(p, b)) => {
                self.out.push_str(p);
                self.out.push(b.to_ascii_uppercase());
            }
            Some(Letter::Fold(f)) => self.out.push_str(&f.to_ascii_uppercase()),
            None => self.unsupported(c),
        }
    }

    fn punctuation(&mut self, i: usize, c: char) {
        match c {
            '"' => {
                let open = self.word_start(i);
                self.out.push(if open { '8' } else { '0' });
            }
            '\'' | '\u{2019}' => {
                let prev_word = self.prev(i).is_some_and(|p| p.is_alphanumeric());
                let next_word = self.next(i).is_some_and(|n| n.is_alphanumeric());
                if prev_word && next_word {
                    self.out.push('\'');
                } else if self.word_start(i) && c == '\'' && next_word {
                    self.out.push_str(",8");
                    self.single_open = true;
                } else if self.single_open && !next_word {
                    self.out.push_str(",0");
                    self.single_open = false;
                } else {
                    self.out.push('\'');
                }
            }
            '\u{2018}' => {
                self.out.push_str(",8");
                self.single_open = true;
            }
            '\u{A0}' | '\u{2007}' | '\u{202F}' => self.out.push(' '),
            '\t' => self.out.push(' '),
            _ => match symbol(c) {
                Some(s) => self.out.push_str(s),
                None => self.unsupported(c),
            },
        }
    }
}

fn digit(c: char) -> char {
    match c {
        '1' => 'A',
        '2' => 'B',
        '3' => 'C',
        '4' => 'D',
        '5' => 'E',
        '6' => 'F',
        '7' => 'G',
        '8' => 'H',
        '9' => 'I',
        _ => 'J',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected braille written in lowercase for readability.
    fn ueb(print: &str, braille: &str) {
        let t = translate(print);
        assert_eq!(
            t.braille,
            braille.to_ascii_uppercase(),
            "translating {print:?}"
        );
        assert!(t.unsupported.is_empty(), "{print:?}: {:?}", t.unsupported);
    }

    #[test]
    fn letters_and_capitals() {
        ueb("the cat", "the cat");
        ueb("Hello, World!", ",hello1 ,world6");
        ueb("I am", ",i am");
        ueb("NASA", ",,nasa");
        ueb("CDs", ",,cd,'s");
        ueb("McDONALD", ",mc,,donald");
        ueb("iPhone", "i,phone");
    }

    #[test]
    fn numbers() {
        ueb("123", "#abc");
        ueb("3.14", "#c4ad");
        ueb("1,000", "#a1jjj");
        ueb("3a", "#c;a");
        ueb("3rd", "#crd");
        ueb("3D", "#c,d");
        ueb("MP3", ",,mp#c");
        ueb("9:30", "#i3#cj");
        ueb("1/2", "#a/b");
        ueb("pages 3-4", "pages #c-#d");
        ueb("$5.", "@s#e4");
        ueb("50%", "#ej.0");
    }

    #[test]
    fn punctuation() {
        ueb("\"Hi,\" she said.", "8,hi10 she said4");
        ueb("don't", "don't");
        ueb("the students' books", "the students' books");
        ueb("'quoted'", ",8quoted,0");
        ueb("(see above)", "\"<see above\">");
        ueb("[1]", ".<#a.>");
        ueb("and/or", "and_/or");
        ueb("a & b", "a @& b");
        ueb("wait\u{2014}what?", "wait\",-what8");
        ueb("1\u{2013}2", "#a,-#b");
        ueb("e-mail", "e-mail");
        ueb("x@y.org", "x@ay4org");
        ueb("Wait\u{2026}", ",wait444");
        ueb("\u{201C}Yes\u{201D}", "8,yes0");
        ueb("it\u{2019}s", "it's");
    }

    #[test]
    fn accents_and_greek() {
        ueb("café", "caf^/e");
        ueb("École", ",^/ecole");
        ueb("naïve", "na^3ive");
        ueb("façade", "fa^&cade");
        ueb("señor", "se^]nor");
        ueb("π", ".p");
        ueb("Δ", ",.d");
        ueb("straße", "strasse");
    }

    /// `<i>`, `<b>` and `<u>` tags as typeform marks.
    fn marked(src: &str) -> String {
        let mut s = src.to_owned();
        for (tag, form) in [
            ("i", Typeform::Italic),
            ("b", Typeform::Bold),
            ("u", Typeform::Underline),
        ] {
            s = s
                .replace(&format!("<{tag}>"), &form.open().to_string())
                .replace(&format!("</{tag}>"), &form.close().to_string());
        }
        s
    }

    /// Capitals passages: the Rules' examples in section 8 (UEB Rules,
    /// 2013), in grade 1 where the Rules' braille is contracted.
    #[test]
    fn capitals_passages_follow_the_rules_examples() {
        // 8.5.3, grade 1: `,,,cau;n3 wet pa9t6,'` contracted.
        ueb("CAUTION: WET PAINT!", ",,,caution3 wet paint6,'");
        // 8.5.3, uncontracted as printed in the Rules.
        ueb("A SELF-MADE MAN", ",,,a self-made man,'");
        // 8.5.3: the passage ends before the lowercase word.
        ueb(
            "Please KEEP OFF THE GRASS in this area.",
            ",please ,,,keep off the grass,' in this area4",
        );
        ueb("FOR SALE: 1975 FIREBIRD", ",,,for sale3 #aige firebird,'");
        // 8.6.2: nested inside the quotation marks, outside the parentheses
        // opened in the passage.
        ueb(
            "He shouted \"I WILL NOT!\"",
            ",he shouted 8,,,i will not6,'0",
        );
        ueb(
            "IT'S A HOAX! (APRIL FOOL!)",
            ",,,it's a hoax6 \"<april fool6\">,'",
        );
        // 8.4.2: two words are two word indicators, no passage.
        ueb("WELCOME TO McDONALD'S", ",,welcome ,,to ,mc,,donald',s");
        ueb("NEW YORK", ",,new ,,york");
        // Single capitals keep their usual form (8.8.1).
        ueb("A B C", ",a ,b ,c");
    }

    fn ueb_marked(print: &str, braille: &str) {
        ueb(&marked(print), braille);
    }

    /// Typeform indicators: the Rules' examples in section 9, in grade 1.
    #[test]
    fn typeforms_follow_the_rules_examples() {
        // 9.3: one word.
        ueb_marked("She <b>was</b> right.", ",she ^1was right4");
        // 9.4: a passage of three words, no punctuation.
        ueb_marked(
            "Click the <b>Up One Level</b> button.",
            ",click the ^7,up ,one ,level^' button4",
        );
        // 9.4.4: the terminator inside a word; none at its end.
        ueb_marked("<b>text</b>book", "^1text^'book");
        ueb_marked("brief<i>ly</i>", "brief.1ly");
        ueb_marked("the <i>Globe</i>'s", "the .1,globe.''s");
        // 9.7.3: no terminator for closing punctuation after a word.
        ueb_marked("Did you read <u>Hamlet</u>?", ",did you read _1,hamlet8");
        // 9.7.2: the comma after the passage is counted in.
        ueb_marked(
            "<i>Hänsel und Gretel</i>, a fairy tale",
            ".7,h^3ansel und ,gretel1.' a fairy tale",
        );
        // 9.2: one symbol.
        ueb_marked(
            "Stop<b>!</b> May I help<b>?</b>",
            ",stop^26 ,may ,i help^28",
        );
        ueb_marked("55 not 5<b>6</b>", "#ee not #e^2#f");
        // 9.3: two sequences, a word indicator each.
        ueb_marked("<i>one two\u{2013}three</i>", ".1one .1two,-three");
        // 9.8.1: two typeforms nest; 8.6.2: typeform outside capitals.
        ueb_marked(
            "wrote <i><u>Anne of Green Gables.</u></i>",
            "wrote .7_7,anne of ,green ,gables4_'.'",
        );
        ueb_marked("<i>ROMEO AND JULIET</i>", ".7,,,romeo and juliet,'.'");
        // Adjacent spans of one typeform read as one passage.
        ueb_marked("<i>one</i> <i>two</i> <i>three</i>", ".7one two three.'");
        // A span divided between lines: open to the end, close from the
        // start.
        ueb_marked("an <i>open", "an .1open");
        ueb_marked("closed</i> here", ".1closed here");
    }

    #[test]
    fn marks_are_stripped_and_blank_entries_are_guide_dots() {
        let s = marked("<i>a</i> b");
        assert_eq!(strip_marks(&s), "a b");
        assert_eq!(translate(&BLANK_ENTRY.to_string()).braille, "\"\"\"");
        assert!(translate(&s).unsupported.is_empty());
    }

    #[test]
    fn unsupported_chars_are_reported_not_emitted() {
        let t = translate("a \u{1F600} b \u{1F600}");
        assert_eq!(t.braille, "A  B ");
        assert_eq!(t.unsupported, vec!['\u{1F600}']);
    }

    #[test]
    fn unicode_round_trip() {
        let ascii = ",HELLO1 #ABC";
        let uni = to_unicode(ascii);
        assert_eq!(
            uni,
            "\u{2820}\u{2813}\u{2811}\u{2807}\u{2807}\u{2815}\u{2802}\u{2800}\u{283C}\u{2801}\u{2803}\u{2809}"
        );
        assert_eq!(from_unicode(&uni), ascii);
        assert_eq!(to_unicode("hello"), to_unicode("HELLO"));
    }

    #[test]
    fn output_is_braille_ascii_only() {
        let t = translate("Mixed TEXT with 42 numbers, \"quotes\" & symbols: #1 (ok)?");
        assert!(
            t.braille
                .chars()
                .all(|c| (' '..='_').contains(&c) || c == '\n')
        );
    }
}
