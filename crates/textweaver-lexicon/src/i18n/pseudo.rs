//! The pseudo-locales and the right-to-left check.

/// Right-to-left override.
const RLO: char = '\u{202e}';
/// Pop directional formatting.
const PDF: char = '\u{202c}';

/// Which pseudo-locale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `en-XA`: accented and longer.
    Accented,
    /// `ar-XB`: wrapped in right-to-left overrides.
    Bidi,
}

/// Text in the pseudo-locale named by `kind`: `Kind::Accented` swaps
/// letters for accented ones and pads each run by a third with `~`;
/// `Kind::Bidi` wraps each run in a right-to-left override. Values put
/// into messages are not changed, so what the user typed stays readable.
pub(crate) fn pseudo_text(kind: Kind, text: &str) -> String {
    match kind {
        Kind::Accented => {
            let mut out: String = text.chars().map(accent).collect();
            let letters = text.chars().filter(|c| c.is_alphabetic()).count();
            let pad = letters.div_ceil(3);
            if pad > 0 {
                out.push_str(&"~".repeat(pad));
            }
            out
        }
        Kind::Bidi => {
            if text.trim().is_empty() {
                return text.to_owned();
            }
            format!("{RLO}{text}{PDF}")
        }
    }
}

fn accent(c: char) -> char {
    match c {
        'a' => 'á',
        'b' => 'ƀ',
        'c' => 'ç',
        'd' => 'ð',
        'e' => 'é',
        'f' => 'ƒ',
        'g' => 'ĝ',
        'h' => 'ĥ',
        'i' => 'î',
        'j' => 'ĵ',
        'k' => 'ķ',
        'l' => 'ļ',
        'm' => 'ɱ',
        'n' => 'ñ',
        'o' => 'ö',
        'p' => 'þ',
        'r' => 'ŕ',
        's' => 'š',
        't' => 'ţ',
        'u' => 'û',
        'w' => 'ŵ',
        'y' => 'ý',
        'z' => 'ž',
        'A' => 'Å',
        'C' => 'Ç',
        'D' => 'Ð',
        'E' => 'É',
        'I' => 'Î',
        'N' => 'Ñ',
        'O' => 'Ö',
        'S' => 'Š',
        'T' => 'Ţ',
        'U' => 'Û',
        other => other,
    }
}

/// Problems with the direction marks in a rendered string: an embedding,
/// override, or isolate that is never closed, or a close with nothing
/// open. A string that passes cannot change the direction of the text
/// shown after it (a status line, the next list item). Empty when fine.
pub fn bidi_problems(text: &str) -> Vec<String> {
    let mut stack: Vec<char> = Vec::new();
    let mut problems = Vec::new();
    for c in text.chars() {
        match c {
            '\u{202c}' => match stack.last() {
                Some('\u{202a}'..='\u{202e}') => {
                    stack.pop();
                }
                _ => problems.push("a closing PDF with no embedding open".to_owned()),
            },
            '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2068}' => stack.push(c),
            '\u{2069}' => {
                // PDI closes the last isolate and any embeddings inside it.
                match stack
                    .iter()
                    .rposition(|c| ('\u{2066}'..='\u{2068}').contains(c))
                {
                    Some(i) => stack.truncate(i),
                    None => problems.push("a closing PDI with no isolate open".to_owned()),
                }
            }
            _ => {}
        }
    }
    if !stack.is_empty() {
        problems.push(format!("{} direction marks left open", stack.len()));
    }
    problems
}
