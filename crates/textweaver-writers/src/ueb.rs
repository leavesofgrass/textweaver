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
//!   [`Translation::unsupported`].
//!
//! Not done: the capitals passage indicator (three or more capitalized words
//! each take their own word indicator instead), typeform (bold, italic)
//! indicators, and contractions (grade 2 goes through liblouis).

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

/// Translates print text to UEB grade 1 braille ASCII.
pub fn translate(text: &str) -> Translation {
    let chars: Vec<char> = text
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '\r' | '\u{AD}' | '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect();
    let mut t = Translator {
        chars: &chars,
        out: String::with_capacity(text.len() + text.len() / 4),
        unsupported: Vec::new(),
        numeric: false,
        single_open: false,
    };
    t.run();
    Translation {
        braille: t.out,
        unsupported: t.unsupported,
    }
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

    fn run(&mut self) {
        let mut i = 0;
        while i < self.chars.len() {
            let c = self.chars[i];
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
            } else {
                self.numeric = false;
                self.punctuation(i, c);
                i += 1;
            }
        }
    }

    /// Translates the run of letters starting at `start`; returns its end.
    fn letters(&mut self, start: usize) -> usize {
        let mut end = start;
        while end < self.chars.len() && is_letter(self.chars[end]) {
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
            if upper {
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
