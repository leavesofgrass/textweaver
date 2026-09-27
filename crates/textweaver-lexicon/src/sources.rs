//! Parsers for the source data: WordNet's database files (WNDB, the format
//! Open English WordNet also publishes) and CMUdict.
//!
//! Only what define word needs is kept: words, definitions, examples,
//! antonyms, hypernyms, the sense order from the index files, and the
//! exception lists that morphy uses.

use std::collections::BTreeMap;
use std::path::Path;

use crate::LexiconError;
use crate::model::Pos;

/// The WNDB files, by part of speech: the data and index file suffixes and
/// the exception list.
const FILES: [(Pos, &str, &str); 4] = [
    (Pos::Noun, "noun", "noun.exc"),
    (Pos::Verb, "verb", "verb.exc"),
    (Pos::Adjective, "adj", "adj.exc"),
    (Pos::Adverb, "adv", "adv.exc"),
];

/// A pointer from one synset to another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pointer {
    /// WordNet's pointer symbol: `@` hypernym, `@i` instance hypernym,
    /// `!` antonym, ...
    pub symbol: String,
    /// The target synset: its data file's part of speech and byte offset.
    pub target: (Pos, u64),
    /// For a lexical pointer, the target word's number (from 1) in the
    /// target synset; 0 for a pointer between whole synsets.
    pub target_word: usize,
}

/// One synset from a data file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Synset {
    /// Its words, spaces in place of underscores, adjective markers such
    /// as `(a)` removed, capitals kept.
    pub words: Vec<String>,
    /// The definition, without the examples.
    pub definition: String,
    /// The examples, without their quotes.
    pub examples: Vec<String>,
    /// Pointers to other synsets.
    pub pointers: Vec<Pointer>,
}

/// A WordNet database, parsed.
#[derive(Clone, Debug, Default)]
pub struct WordNet {
    /// Synsets by part of speech and data-file offset.
    pub synsets: BTreeMap<(Pos, u64), Synset>,
    /// Lemma (lowercase, spaces for underscores) to its synsets in each
    /// part of speech, most frequent sense first.
    pub index: BTreeMap<String, Vec<(Pos, Vec<u64>)>>,
    /// Inflected form to its base forms, from the exception lists.
    pub exceptions: BTreeMap<String, Vec<(Pos, String)>>,
}

impl WordNet {
    /// Reads `data.*`, `index.*`, and `*.exc` from a WNDB directory.
    pub fn read_dir(dir: &Path) -> Result<WordNet, LexiconError> {
        let read = |name: &str| -> Result<String, LexiconError> {
            let path = dir.join(name);
            std::fs::read(&path)
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .map_err(|source| LexiconError::Io { path, source })
        };
        let mut wn = WordNet::default();
        for (pos, suffix, exc) in FILES {
            wn.add_data(pos, &read(&format!("data.{suffix}"))?)?;
            wn.add_index(pos, &read(&format!("index.{suffix}"))?)?;
            // Exception lists are optional (adv.exc is nearly empty).
            if let Ok(text) = read(exc) {
                wn.add_exceptions(pos, &text);
            }
        }
        Ok(wn)
    }

    /// Adds the synsets of one data file.
    pub fn add_data(&mut self, pos: Pos, text: &str) -> Result<(), LexiconError> {
        for (n, line) in text.lines().enumerate() {
            if line.starts_with("  ") || line.trim().is_empty() {
                continue; // the licence header
            }
            let (offset, synset) = parse_data_line(line).ok_or_else(|| LexiconError::Source {
                what: format!("data file for {}s, line {}", pos.name(), n + 1),
            })?;
            self.synsets.insert((pos, offset), synset);
        }
        Ok(())
    }

    /// Adds one index file.
    pub fn add_index(&mut self, pos: Pos, text: &str) -> Result<(), LexiconError> {
        for (n, line) in text.lines().enumerate() {
            if line.starts_with("  ") || line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split_whitespace().collect();
            let bad = || LexiconError::Source {
                what: format!("index file for {}s, line {}", pos.name(), n + 1),
            };
            let synset_cnt: usize = f.get(2).and_then(|s| s.parse().ok()).ok_or_else(bad)?;
            if synset_cnt > f.len() {
                return Err(bad());
            }
            let offsets = f[f.len() - synset_cnt..]
                .iter()
                .map(|s| s.parse::<u64>().map_err(|_| bad()))
                .collect::<Result<Vec<_>, _>>()?;
            let lemma = f[0].replace('_', " ");
            self.index.entry(lemma).or_default().push((pos, offsets));
        }
        Ok(())
    }

    /// Adds one exception list: `inflected base [base...]` per line.
    pub fn add_exceptions(&mut self, pos: Pos, text: &str) {
        for line in text.lines() {
            let mut f = line.split_whitespace();
            let Some(form) = f.next() else { continue };
            let form = form.replace('_', " ");
            for base in f {
                let base = base.replace('_', " ");
                let list = self.exceptions.entry(form.clone()).or_default();
                if !list.iter().any(|(p, b)| *p == pos && *b == base) {
                    list.push((pos, base));
                }
            }
        }
    }
}

/// Parses one data-file line into its offset and synset.
fn parse_data_line(line: &str) -> Option<(u64, Synset)> {
    let (fields, gloss) = match line.find(" | ") {
        Some(i) => (&line[..i], &line[i + 3..]),
        None => (line, ""),
    };
    let f: Vec<&str> = fields.split_whitespace().collect();
    let offset: u64 = f.first()?.parse().ok()?;
    let w_cnt = usize::from_str_radix(f.get(3)?, 16).ok()?;
    let mut at = 4;
    let mut words = Vec::with_capacity(w_cnt);
    for _ in 0..w_cnt {
        words.push(clean_word(f.get(at)?));
        at += 2; // the word and its lex_id
    }
    let p_cnt: usize = f.get(at)?.parse().ok()?;
    at += 1;
    let mut pointers = Vec::with_capacity(p_cnt);
    for _ in 0..p_cnt {
        let symbol = (*f.get(at)?).to_owned();
        let target_offset: u64 = f.get(at + 1)?.parse().ok()?;
        let target_pos = Pos::parse(f.get(at + 2)?)?;
        let st = f.get(at + 3)?;
        let target_word = usize::from_str_radix(st.get(2..4)?, 16).ok()?;
        pointers.push(Pointer {
            symbol,
            target: (target_pos, target_offset),
            target_word,
        });
        at += 4;
    }
    let (definition, examples) = split_gloss(gloss.trim());
    Some((
        offset,
        Synset {
            words,
            definition,
            examples,
            pointers,
        },
    ))
}

/// `beautiful(a)` to `beautiful`, `ice_cream` to `ice cream`.
fn clean_word(w: &str) -> String {
    let w = match w.find('(') {
        Some(i) if w.ends_with(')') => &w[..i],
        _ => w,
    };
    w.replace('_', " ")
}

/// Splits a gloss into its definition and its quoted examples. Parts are
/// separated by `; ` outside quotes; a part that starts with a quote is an
/// example (text after its closing quote, such as an attribution, is kept
/// in parentheses).
pub(crate) fn split_gloss(gloss: &str) -> (String, Vec<String>) {
    let mut parts: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = gloss.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            quoted = !quoted;
        }
        if c == ';' && !quoted && chars.peek().is_some_and(|n| n.is_whitespace()) {
            parts.push(std::mem::take(&mut cur));
            continue;
        }
        cur.push(c);
    }
    parts.push(cur);
    let mut definition: Vec<String> = Vec::new();
    let mut examples = Vec::new();
    for p in parts {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        if let Some(rest) = p.strip_prefix('"') {
            match rest.rfind('"') {
                Some(end) => {
                    let text = rest[..end].trim();
                    let after = rest[end + 1..].trim().trim_start_matches(['-', ' ']).trim();
                    if after.is_empty() {
                        examples.push(text.to_owned());
                    } else {
                        examples.push(format!("{text} ({after})"));
                    }
                }
                None => examples.push(rest.trim().to_owned()),
            }
        } else {
            definition.push(p.to_owned());
        }
    }
    (definition.join("; "), examples)
}

/// CMUdict: word (lowercase) to its pronunciations, first listed first.
pub fn read_cmudict(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in text.lines() {
        let line = match line.find(" #") {
            Some(i) => &line[..i],
            None => line,
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with(";;;") || line.starts_with('#') {
            continue;
        }
        let Some((word, pron)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let word = match word.find('(') {
            Some(i) if word.ends_with(')') => &word[..i],
            _ => word,
        };
        let pron = pron.split_whitespace().collect::<Vec<_>>().join(" ");
        if pron.is_empty() {
            continue;
        }
        let list = out.entry(word.to_lowercase()).or_default();
        if !list.contains(&pron) {
            list.push(pron);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_lines() {
        let line = "02086723 05 n 02 dog 0 domestic_dog 0 003 @ 02085998 n 0000 @ 01320032 n 0000 ! 09900001 n 0101 | a member of the genus Canis; \"the dog barked all night\"  ";
        let (offset, s) = parse_data_line(line).unwrap();
        assert_eq!(offset, 2086723);
        assert_eq!(s.words, ["dog", "domestic dog"]);
        assert_eq!(s.definition, "a member of the genus Canis");
        assert_eq!(s.examples, ["the dog barked all night"]);
        assert_eq!(s.pointers.len(), 3);
        assert_eq!(s.pointers[2].symbol, "!");
        assert_eq!(s.pointers[2].target, (Pos::Noun, 9900001));
        assert_eq!(s.pointers[2].target_word, 1);
        // Verb frames after the pointers are ignored.
        let verb = "01926311 38 v 01 run 0 001 @ 01835496 v 0000 02 + 01 00 + 02 00 | move fast by using one's feet; \"Don't run--you'll be out of breath\"; \"The children ran to the store\"";
        let (_, v) = parse_data_line(verb).unwrap();
        assert_eq!(v.words, ["run"]);
        assert_eq!(v.examples.len(), 2);
        let adj = "00001740 00 a 01 able(a) 0 000 | (usually followed by `to') having the necessary means";
        let (_, a) = parse_data_line(adj).unwrap();
        assert_eq!(a.words, ["able"]);
        assert!(parse_data_line("garbage").is_none());
        assert!(parse_data_line("00001740 00 a 05 able 0 000 | x").is_none());
    }

    #[test]
    fn glosses() {
        assert_eq!(
            split_gloss("a; b; \"one; two\"; \"three\" - Shakespeare"),
            (
                "a; b".to_owned(),
                vec!["one; two".to_owned(), "three (Shakespeare)".to_owned()]
            )
        );
        assert_eq!(split_gloss(""), (String::new(), vec![]));
    }

    #[test]
    fn index_and_exceptions() {
        let mut wn = WordNet::default();
        wn.add_index(
            Pos::Noun,
            "  licence line\nice_cream n 2 1 @ 2 0 07630221 07630000\n",
        )
        .unwrap();
        assert_eq!(
            wn.index["ice cream"],
            vec![(Pos::Noun, vec![7630221, 7630000])]
        );
        assert!(wn.add_index(Pos::Noun, "bad n x\n").is_err());
        wn.add_exceptions(Pos::Verb, "ran run\nbetter well good\n");
        assert_eq!(wn.exceptions["ran"], vec![(Pos::Verb, "run".to_owned())]);
        assert_eq!(wn.exceptions["better"].len(), 2);
    }

    #[test]
    fn cmudict() {
        let d = read_cmudict(
            ";;; comment\nrun R AH1 N\nread R EH1 D\nread(2) R IY1 D\nfoo F UW1 # a comment\n",
        );
        assert_eq!(d["run"], ["R AH1 N"]);
        assert_eq!(d["read"], ["R EH1 D", "R IY1 D"]);
        assert_eq!(d["foo"], ["F UW1"]);
    }
}
