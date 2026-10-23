//! The International Morse code.
//!
//! Source: Recommendation ITU-R M.1677-1, "International Morse code"
//! (10/2009), Part I: the letters, the accented E, the figures, the
//! punctuation marks and miscellaneous signs, and the signals used in
//! transmission (understood, error, invitation to transmit, wait, end of
//! work, starting signal). The cross sign `.-.-.` is also the
//! recommendation's end-of-message signal, written AR, so AR, the `+`
//! sign and RN are one code. The double hyphen `-...-` is both `=` and the
//! separation signal BT. The codes were checked against the table in the
//! research for this crate (Wikipedia's transcription of M.1677-1); the
//! recommendation itself is a PDF and was not fetched.
//!
//! Written as dots and dashes: `.` is a dit, `-` a dah. They are never
//! shown to the reader (the owner's decision): only the renderer reads
//! them.

/// A sendable symbol: one character, or a prosign sent as one unbroken
/// character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbol {
    /// A character, uppercase where it has a case.
    Char(char),
    /// A prosign by its usual name ("BT", "AR").
    Prosign(&'static str),
}

impl Symbol {
    /// The code in dots and dashes.
    pub fn code(self) -> &'static str {
        match self {
            // Every `Char` and `Prosign` is made from a table entry.
            Symbol::Char(c) => code(c).unwrap_or(""),
            Symbol::Prosign(p) => prosign(p).unwrap_or(""),
        }
    }

    /// How the symbol is written for a reader: the character, or the
    /// prosign in angle brackets (`<BT>`), the usual way to write one.
    pub fn label(self) -> String {
        match self {
            Symbol::Char(c) => c.to_string(),
            Symbol::Prosign(p) => format!("<{p}>"),
        }
    }
}

/// The prosigns textweaver sends, by name, with their codes. A prosign is
/// written in a text between angle brackets (`<BT>`, `<AR>`) and sent as
/// one character with no gap between its letters.
pub const PROSIGNS: &[(&str, &str)] = &[
    // End of message (the cross sign).
    ("AR", ".-.-."),
    // Wait.
    ("AS", ".-..."),
    // Separation, new paragraph (the double hyphen).
    ("BT", "-...-"),
    // Starting signal.
    ("CT", "-.-.-"),
    // Error: eight dits.
    ("HH", "........"),
    // Invitation for a named station to transmit (the open bracket).
    ("KN", "-.--."),
    // End of work.
    ("SK", "...-.-"),
    // Understood.
    ("SN", "...-."),
];

/// The code of prosign `name` (uppercase letters, no brackets).
pub fn prosign(name: &str) -> Option<&'static str> {
    PROSIGNS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, c)| *c)
}

/// The name as stored in [`PROSIGNS`], for a name in any case.
pub(crate) fn prosign_name(name: &str) -> Option<&'static str> {
    PROSIGNS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(n, _)| *n)
}

/// The code of `c`, in either case; `None` for a character Morse code
/// does not have (it is skipped and counted).
pub fn code(c: char) -> Option<&'static str> {
    let c = match c {
        'a'..='z' => c.to_ascii_uppercase(),
        'é' => 'É',
        // The multiplication sign is sent as the letter X (M.1677-1).
        '×' => 'X',
        // Typographic quotes and apostrophes as their plain forms.
        '\u{2018}' | '\u{2019}' => '\'',
        '\u{201C}' | '\u{201D}' => '"',
        _ => c,
    };
    Some(match c {
        'A' => ".-",
        'B' => "-...",
        'C' => "-.-.",
        'D' => "-..",
        'E' => ".",
        'É' => "..-..",
        'F' => "..-.",
        'G' => "--.",
        'H' => "....",
        'I' => "..",
        'J' => ".---",
        'K' => "-.-",
        'L' => ".-..",
        'M' => "--",
        'N' => "-.",
        'O' => "---",
        'P' => ".--.",
        'Q' => "--.-",
        'R' => ".-.",
        'S' => "...",
        'T' => "-",
        'U' => "..-",
        'V' => "...-",
        'W' => ".--",
        'X' => "-..-",
        'Y' => "-.--",
        'Z' => "--..",
        '1' => ".----",
        '2' => "..---",
        '3' => "...--",
        '4' => "....-",
        '5' => ".....",
        '6' => "-....",
        '7' => "--...",
        '8' => "---..",
        '9' => "----.",
        '0' => "-----",
        // Punctuation marks and miscellaneous signs (M.1677-1).
        '.' => ".-.-.-",
        ',' => "--..--",
        ':' => "---...",
        '?' => "..--..",
        '\'' => ".----.",
        '-' => "-....-",
        '/' => "-..-.",
        '(' => "-.--.",
        ')' => "-.--.-",
        '"' => ".-..-.",
        '=' => "-...-",
        '+' => ".-.-.",
        '@' => ".--.-.",
        // In wide use, not in M.1677-1.
        '!' => "-.-.--",
        '&' => ".-...",
        ';' => "-.-.-.",
        '_' => "..--.-",
        '$' => "...-..-",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_figures_follow_the_itu_table() {
        let letters = [
            ".-", "-...", "-.-.", "-..", ".", "..-.", "--.", "....", "..", ".---", "-.-", ".-..",
            "--", "-.", "---", ".--.", "--.-", ".-.", "...", "-", "..-", "...-", ".--", "-..-",
            "-.--", "--..",
        ];
        for (c, want) in ('A'..='Z').zip(letters) {
            assert_eq!(code(c), Some(want), "{c}");
            assert_eq!(code(c.to_ascii_lowercase()), Some(want), "{c}");
        }
        let figures = [
            "-----", ".----", "..---", "...--", "....-", ".....", "-....", "--...", "---..",
            "----.",
        ];
        for (c, want) in ('0'..='9').zip(figures) {
            assert_eq!(code(c), Some(want), "{c}");
        }
        assert_eq!(code('é'), Some("..-.."));
    }

    #[test]
    fn punctuation_follows_the_itu_table() {
        for (c, want) in [
            ('.', ".-.-.-"),
            (',', "--..--"),
            (':', "---..."),
            ('?', "..--.."),
            ('\'', ".----."),
            ('-', "-....-"),
            ('/', "-..-."),
            ('(', "-.--."),
            (')', "-.--.-"),
            ('"', ".-..-."),
            ('=', "-...-"),
            ('+', ".-.-."),
            ('@', ".--.-."),
            ('×', "-..-"),
        ] {
            assert_eq!(code(c), Some(want), "{c}");
        }
        for c in ['#', '*', '<', '\u{263A}', ' '] {
            assert_eq!(code(c), None, "{c}");
        }
    }

    #[test]
    fn prosigns_share_codes_as_the_itu_says() {
        assert_eq!(prosign("AR"), code('+'));
        assert_eq!(prosign("bt"), code('='));
        assert_eq!(prosign("KN"), code('('));
        assert_eq!(prosign("AS"), code('&'));
        assert_eq!(prosign("SK"), Some("...-.-"));
        assert_eq!(prosign("SN"), Some("...-."));
        assert_eq!(prosign("CT"), Some("-.-.-"));
        assert_eq!(prosign("HH"), Some("........"));
        assert_eq!(prosign("XX"), None);
        assert_eq!(Symbol::Prosign("BT").label(), "<BT>");
        assert_eq!(Symbol::Char('A').code(), ".-");
    }
}
