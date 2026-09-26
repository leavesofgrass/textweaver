//! Spoken commands while dictating: "new line", "period", "open quote",
//! and so on, turned into the text they stand for. A pure transform on
//! the transcript; Star had none.
//!
//! Whisper punctuates on its own, so the transform also tidies around
//! each command: the punctuation Whisper attached to a command word is
//! dropped ("Period." is just a period), and punctuation Whisper put just
//! before a punctuation or line command is replaced by it ("world, period"
//! gives "world."). After a sentence end or a line break the next word is
//! capitalized.
//!
//! Say "literal" before a command word to type the word itself: "literal
//! period" types "period".

/// What a command produces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    /// Punctuation that attaches to the word before it.
    Punct(&'static str),
    /// Punctuation that ends a sentence (capitalizes what follows).
    SentenceEnd(&'static str),
    /// Line breaks: 1 for a new line, 2 for a new paragraph.
    Break(usize),
    /// An opening mark: spaced before, attached after.
    Open(&'static str),
    /// A closing mark: attached before, spaced after.
    Close(&'static str),
    /// A mark attached on both sides (a hyphen).
    Join(&'static str),
    /// A mark spaced on both sides (a dash).
    Spaced(&'static str),
}

/// The commands, as word sequences (already lower-cased, punctuation
/// stripped). Longer phrases are tried first.
const COMMANDS: &[(&[&str], Action)] = &[
    (&["new", "paragraph"], Action::Break(2)),
    (&["next", "paragraph"], Action::Break(2)),
    (&["new", "line"], Action::Break(1)),
    (&["next", "line"], Action::Break(1)),
    (&["newline"], Action::Break(1)),
    (&["full", "stop"], Action::SentenceEnd(".")),
    (&["fullstop"], Action::SentenceEnd(".")),
    (&["period"], Action::SentenceEnd(".")),
    (&["question", "mark"], Action::SentenceEnd("?")),
    (&["exclamation", "mark"], Action::SentenceEnd("!")),
    (&["exclamation", "point"], Action::SentenceEnd("!")),
    (&["comma"], Action::Punct(",")),
    (&["semicolon"], Action::Punct(";")),
    (&["semi", "colon"], Action::Punct(";")),
    (&["colon"], Action::Punct(":")),
    (&["ellipsis"], Action::Punct("...")),
    (&["open", "quote"], Action::Open("\"")),
    (&["begin", "quote"], Action::Open("\"")),
    (&["close", "quote"], Action::Close("\"")),
    (&["end", "quote"], Action::Close("\"")),
    (&["unquote"], Action::Close("\"")),
    (&["open", "parenthesis"], Action::Open("(")),
    (&["open", "paren"], Action::Open("(")),
    (&["left", "paren"], Action::Open("(")),
    (&["close", "parenthesis"], Action::Close(")")),
    (&["close", "paren"], Action::Close(")")),
    (&["right", "paren"], Action::Close(")")),
    (&["hyphen"], Action::Join("-")),
    (&["dash"], Action::Spaced("\u{2014}")),
];

/// The command words recognized, for help text: `new line`, `period`, ...
pub fn command_phrases() -> Vec<String> {
    COMMANDS.iter().map(|(words, _)| words.join(" ")).collect()
}

/// Punctuation Whisper may attach to a word.
fn is_edge_punct(c: char) -> bool {
    matches!(
        c,
        '.' | ','
            | '!'
            | '?'
            | ';'
            | ':'
            | '"'
            | '\''
            | '('
            | ')'
            | '\u{201c}'
            | '\u{201d}'
            | '\u{2018}'
            | '\u{2019}'
            | '\u{2026}'
    )
}

/// A word as a command key: lower-cased, edge punctuation stripped,
/// hyphens removed (`New-line` is `newline`).
fn key(word: &str) -> String {
    word.trim_matches(is_edge_punct)
        .replace('-', "")
        .to_lowercase()
}

/// Sentence punctuation Whisper leaves at the end of the word before a
/// command, which the command replaces.
fn strip_trailing_punct(s: &mut String) {
    while s.ends_with(['.', ',', ';', ':', '!', '?', '\u{2026}']) {
        s.pop();
    }
}

/// The output being built.
#[derive(Default)]
struct Out {
    text: String,
    /// A space goes before the next word.
    space: bool,
    /// The next word starts a sentence.
    capitalize: bool,
    /// Bytes of `text` up to the end of the last dictated word, so
    /// punctuation Whisper appended can be found and replaced.
    last_word_end: Option<usize>,
}

impl Out {
    fn word(&mut self, w: &str) {
        if self.space {
            self.text.push(' ');
        }
        if self.capitalize {
            let mut chars = w.chars();
            if let Some(first) = chars.next() {
                self.text.extend(first.to_uppercase());
                self.text.push_str(chars.as_str());
            }
        } else {
            self.text.push_str(w);
        }
        self.capitalize = false;
        self.space = true;
        self.last_word_end = Some(self.text.len());
    }

    /// Removes the punctuation Whisper put after the last word, when
    /// nothing but that punctuation follows it.
    fn replace_trailing(&mut self) {
        if self.last_word_end == Some(self.text.len()) {
            strip_trailing_punct(&mut self.text);
            self.last_word_end = Some(self.text.len());
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Punct(p) => {
                self.replace_trailing();
                self.text.push_str(p);
                self.space = true;
            }
            Action::SentenceEnd(p) => {
                self.replace_trailing();
                self.text.push_str(p);
                self.space = true;
                self.capitalize = true;
            }
            Action::Break(n) => {
                self.replace_trailing();
                while self.text.ends_with(' ') {
                    self.text.pop();
                }
                for _ in 0..n {
                    self.text.push('\n');
                }
                self.space = false;
                self.capitalize = true;
            }
            Action::Open(p) => {
                if self.space {
                    self.text.push(' ');
                }
                self.text.push_str(p);
                self.space = false;
            }
            Action::Close(p) => {
                self.replace_trailing();
                self.text.push_str(p);
                self.space = true;
            }
            Action::Join(p) => {
                self.replace_trailing();
                self.text.push_str(p);
                self.space = false;
            }
            Action::Spaced(p) => {
                self.replace_trailing();
                if self.space {
                    self.text.push(' ');
                }
                self.text.push_str(p);
                self.space = true;
            }
        }
        self.last_word_end = None;
    }
}

/// The command starting at `words[i]`, with the number of words it spans.
fn match_command(keys: &[String], i: usize) -> Option<(Action, usize)> {
    COMMANDS
        .iter()
        .filter(|(phrase, _)| {
            phrase.len() <= keys.len() - i && phrase.iter().zip(&keys[i..]).all(|(p, k)| p == k)
        })
        .max_by_key(|(phrase, _)| phrase.len())
        .map(|(phrase, action)| (*action, phrase.len()))
}

/// Applies spoken commands to dictated `text`.
///
/// ```
/// use textweaver_dictation::apply_spoken_commands;
/// assert_eq!(
///     apply_spoken_commands("Dear Sam, comma. New line. Thanks for the book period"),
///     "Dear Sam,\nThanks for the book."
/// );
/// ```
pub fn apply_spoken_commands(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let keys: Vec<String> = words.iter().map(|w| key(w)).collect();
    let mut out = Out::default();
    let mut i = 0;
    while i < words.len() {
        if keys[i] == "literal" && i + 1 < words.len() {
            out.word(words[i + 1]);
            i += 2;
            continue;
        }
        match match_command(&keys, i) {
            Some((action, n)) => {
                out.apply(action);
                i += n;
            }
            None => {
                out.word(words[i]);
                i += 1;
            }
        }
    }
    out.text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(input: &str) -> String {
        apply_spoken_commands(input)
    }

    #[test]
    fn plain_text_is_unchanged() {
        assert_eq!(
            t("Hello, world. How are you?"),
            "Hello, world. How are you?"
        );
        assert_eq!(t("  spaced   out  "), "spaced out");
        assert_eq!(t(""), "");
    }

    #[test]
    fn punctuation_commands() {
        assert_eq!(t("hello world period"), "hello world.");
        assert_eq!(
            t("is it done question mark yes exclamation point"),
            "is it done? Yes!"
        );
        assert_eq!(
            t("apples comma pears semicolon plums colon figs"),
            "apples, pears; plums: figs"
        );
        assert_eq!(t("wait ellipsis okay full stop"), "wait... okay.");
    }

    #[test]
    fn whisper_punctuation_around_commands_is_tidied() {
        // Whisper often writes the command word as its own sentence.
        assert_eq!(
            t("Hello world, period. How are you, question mark?"),
            "Hello world. How are you?"
        );
        assert_eq!(t("Hello world. Period."), "Hello world.");
        assert_eq!(
            t("First item, comma, second item."),
            "First item, second item."
        );
    }

    #[test]
    fn line_and_paragraph_breaks() {
        assert_eq!(t("Dear Sam, new line. thanks"), "Dear Sam\nThanks");
        assert_eq!(t("one new paragraph two"), "one\n\nTwo");
        assert_eq!(t("New-line start"), "\nStart");
        assert_eq!(t("end new line"), "end\n");
    }

    #[test]
    fn quotes_parentheses_and_dashes() {
        // Punctuation Whisper attaches to a command word is its guess, and
        // goes with the word.
        assert_eq!(
            t("She said, open quote, hello, close quote, and left."),
            "She said, \"hello\" and left."
        );
        assert_eq!(t("see open paren page 4 close paren"), "see (page 4)");
        assert_eq!(t("well dash maybe"), "well \u{2014} maybe");
        assert_eq!(
            t("state hyphen of hyphen the hyphen art"),
            "state-of-the-art"
        );
    }

    #[test]
    fn literal_escapes_a_command_word() {
        assert_eq!(t("the literal period of history"), "the period of history");
        assert_eq!(t("type literal new line"), "type new line");
    }

    #[test]
    fn capitalizes_after_sentence_ends_only() {
        assert_eq!(t("one period two comma three"), "one. Two, three");
        assert_eq!(t("é period ça va"), "é. Ça va");
    }

    #[test]
    fn phrases_are_listed() {
        let p = command_phrases();
        assert!(p.contains(&"new line".to_owned()));
        assert!(p.contains(&"period".to_owned()));
    }
}
