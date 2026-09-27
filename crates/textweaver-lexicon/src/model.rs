//! What a lookup returns, and the records the data file stores.

use serde::Serialize;

use crate::LexiconError;
use crate::codec::{Reader, put_str, put_strs, put_varint};

/// A part of speech, as WordNet has them (adjective satellites count as
/// adjectives).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pos {
    /// Noun.
    Noun,
    /// Verb.
    Verb,
    /// Adjective.
    Adjective,
    /// Adverb.
    Adverb,
}

impl Pos {
    /// Every part of speech, in the order results list them.
    pub const ALL: [Pos; 4] = [Pos::Noun, Pos::Verb, Pos::Adjective, Pos::Adverb];

    /// The English name: `noun`, `verb`, `adjective`, `adverb`.
    pub fn name(self) -> &'static str {
        match self {
            Pos::Noun => "noun",
            Pos::Verb => "verb",
            Pos::Adjective => "adjective",
            Pos::Adverb => "adverb",
        }
    }

    /// The part of speech for a name or WordNet letter (`n`, `v`, `a`,
    /// `s`, `r`, `noun`, `adj`, ...), if it is one.
    pub fn parse(s: &str) -> Option<Pos> {
        match s.trim().to_ascii_lowercase().as_str() {
            "n" | "noun" => Some(Pos::Noun),
            "v" | "verb" => Some(Pos::Verb),
            "a" | "s" | "adj" | "adjective" => Some(Pos::Adjective),
            "r" | "adv" | "adverb" => Some(Pos::Adverb),
            _ => None,
        }
    }

    pub(crate) fn bit(self) -> u64 {
        1 << (self as u64)
    }

    pub(crate) fn code(self) -> u8 {
        self as u8
    }

    pub(crate) fn from_code(c: u8) -> Result<Pos, LexiconError> {
        Pos::ALL
            .get(usize::from(c))
            .copied()
            .ok_or(LexiconError::Corrupt("unknown part of speech"))
    }
}

/// Where a definition came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// The user's own glossary.
    Glossary,
    /// Open English WordNet.
    WordNet,
    /// No definition was found; only the pronunciation (CMUdict).
    Pronunciation,
}

impl Source {
    /// The name said and shown for the source.
    pub fn name(self) -> &'static str {
        match self {
            Source::Glossary => "your glossary",
            Source::WordNet => "Open English WordNet",
            Source::Pronunciation => "the CMU Pronouncing Dictionary",
        }
    }
}

/// One pronunciation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Pronunciation {
    /// ARPAbet phonemes with stress digits, as CMUdict writes them:
    /// `R AH1 N IH0 NG`. A glossary may give any text here.
    pub arpabet: String,
    /// A respelling to read aloud, stressed syllable in capitals:
    /// `RUH-ning` (empty when `arpabet` is not ARPAbet).
    pub respelling: String,
}

/// One sense of a word.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Sense {
    /// The definition.
    pub definition: String,
    /// Example sentences.
    pub examples: Vec<String>,
    /// Other words with this sense.
    pub synonyms: Vec<String>,
    /// Words of opposite meaning.
    pub antonyms: Vec<String>,
    /// What this is a kind of (WordNet's hypernyms, first word of each).
    pub kind_of: Vec<String>,
}

/// The senses of one headword in one part of speech.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SenseGroup {
    /// The headword: the word looked up, or its base form (`run` for
    /// `running`).
    pub lemma: String,
    /// The part of speech; `None` for a glossary entry that gives none.
    pub pos: Option<Pos>,
    /// Senses, most common first.
    pub senses: Vec<Sense>,
}

/// Everything found for a word.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Definition {
    /// The word as looked up (lowercased, punctuation trimmed).
    pub word: String,
    /// Where the senses came from.
    pub source: Source,
    /// Pronunciations, most common first.
    pub pronunciations: Vec<Pronunciation>,
    /// Sense groups: the word's own first, then its base forms'.
    pub groups: Vec<SenseGroup>,
}

impl Definition {
    /// How many senses in all.
    pub fn sense_count(&self) -> usize {
        self.groups.iter().map(|g| g.senses.len()).sum()
    }
}

/// A headword's record: pronunciations, synsets per part of speech, and
/// base forms from WordNet's exception lists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WordRecord {
    pub(crate) prons: Vec<String>,
    pub(crate) synsets: Vec<(Pos, Vec<u32>)>,
    pub(crate) bases: Vec<(Pos, String)>,
}

impl WordRecord {
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_varint(&mut out, self.prons.len() as u64);
        for p in &self.prons {
            put_pron(&mut out, p);
        }
        put_varint(&mut out, self.synsets.len() as u64);
        for (pos, ids) in &self.synsets {
            out.push(pos.code());
            put_varint(&mut out, ids.len() as u64);
            // The first number, then each one's difference from the one
            // before (zigzag): numbers given in first-use order are
            // mostly close together.
            let mut prev = 0i64;
            for id in ids {
                let d = i64::from(*id) - prev;
                put_varint(&mut out, ((d << 1) ^ (d >> 63)) as u64);
                prev = i64::from(*id);
            }
        }
        put_varint(&mut out, self.bases.len() as u64);
        for (pos, base) in &self.bases {
            out.push(pos.code());
            put_str(&mut out, base);
        }
        out
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, LexiconError> {
        let mut r = Reader::new(bytes);
        let n = r.count()?;
        let prons = (0..n)
            .map(|_| get_pron(&mut r))
            .collect::<Result<Vec<_>, _>>()?;
        let n = r.count()?;
        let mut synsets = Vec::with_capacity(n);
        for _ in 0..n {
            let pos = Pos::from_code(r.u8()?)?;
            let k = r.count()?;
            let mut ids = Vec::with_capacity(k);
            let mut prev = 0i64;
            for _ in 0..k {
                let z = r.varint()?;
                let d = ((z >> 1) as i64) ^ -((z & 1) as i64);
                prev = prev
                    .checked_add(d)
                    .ok_or(LexiconError::Corrupt("synset number"))?;
                ids.push(u32::try_from(prev).map_err(|_| LexiconError::Corrupt("synset number"))?);
            }
            synsets.push((pos, ids));
        }
        let n = r.count()?;
        let mut bases = Vec::with_capacity(n);
        for _ in 0..n {
            let pos = Pos::from_code(r.u8()?)?;
            bases.push((pos, r.str()?.to_owned()));
        }
        Ok(WordRecord {
            prons,
            synsets,
            bases,
        })
    }
}

/// ARPAbet phones, in the order their byte codes use.
const PHONES: [&str; 39] = [
    "AA", "AE", "AH", "AO", "AW", "AY", "B", "CH", "D", "DH", "EH", "ER", "EY", "F", "G", "HH",
    "IH", "IY", "JH", "K", "L", "M", "N", "NG", "OW", "OY", "P", "R", "S", "SH", "T", "TH", "UH",
    "UW", "V", "W", "Y", "Z", "ZH",
];

/// Marks a pronunciation stored as text (not ARPAbet from CMUdict).
const PRON_TEXT: u8 = 0xff;

/// A pronunciation: one byte per ARPAbet phone (phone number times four
/// plus the stress digit, or plus three for none), or [`PRON_TEXT`] and
/// the text when it is not ARPAbet.
fn put_pron(out: &mut Vec<u8>, p: &str) {
    let codes: Option<Vec<u8>> = p
        .split_whitespace()
        .map(|ph| {
            let (base, stress) = match ph.chars().last().and_then(|c| c.to_digit(10)) {
                Some(d @ 0..=2) => (&ph[..ph.len() - 1], d as u8),
                Some(_) => return None,
                None => (ph, 3),
            };
            let i = PHONES.iter().position(|x| *x == base)?;
            Some(i as u8 * 4 + stress)
        })
        .collect();
    match codes {
        Some(c) if !c.is_empty() => {
            put_varint(out, c.len() as u64);
            out.extend_from_slice(&c);
        }
        _ => {
            put_varint(out, u64::from(PRON_TEXT));
            put_str(out, p);
        }
    }
}

fn get_pron(r: &mut Reader<'_>) -> Result<String, LexiconError> {
    let n = r.varint()?;
    if n == u64::from(PRON_TEXT) {
        return Ok(r.str()?.to_owned());
    }
    let mut out = String::new();
    for _ in 0..n {
        let c = r.u8()?;
        let phone = PHONES
            .get(usize::from(c / 4))
            .ok_or(LexiconError::Corrupt("unknown phone"))?;
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(phone);
        if c % 4 < 3 {
            out.push(char::from(b'0' + c % 4));
        }
    }
    Ok(out)
}

/// A synset's record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SynsetRecord {
    pub(crate) words: Vec<String>,
    pub(crate) gloss: String,
    pub(crate) examples: Vec<String>,
    pub(crate) antonyms: Vec<String>,
    pub(crate) kind_of: Vec<String>,
}

impl SynsetRecord {
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_strs(&mut out, &self.words);
        put_str(&mut out, &self.gloss);
        put_strs(&mut out, &self.examples);
        put_strs(&mut out, &self.antonyms);
        put_strs(&mut out, &self.kind_of);
        out
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, LexiconError> {
        let mut r = Reader::new(bytes);
        Ok(SynsetRecord {
            words: r.strs()?,
            gloss: r.str()?.to_owned(),
            examples: r.strs()?,
            antonyms: r.strs()?,
            kind_of: r.strs()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip() {
        let w = WordRecord {
            prons: vec!["R AH1 N".into(), "kuh-NIGH-nee".into(), "HH M".into()],
            synsets: vec![(Pos::Noun, vec![1, 900_000]), (Pos::Verb, vec![7])],
            bases: vec![(Pos::Verb, "run".into())],
        };
        assert_eq!(WordRecord::decode(&w.encode()).unwrap(), w);
        let s = SynsetRecord {
            words: vec!["dog".into(), "domestic dog".into()],
            gloss: "a member of the genus Canis".into(),
            examples: vec!["the dog barked all night".into()],
            antonyms: vec![],
            kind_of: vec!["canine".into()],
        };
        assert_eq!(SynsetRecord::decode(&s.encode()).unwrap(), s);
        assert!(WordRecord::decode(&[1]).is_err());
    }

    #[test]
    fn parts_of_speech() {
        assert_eq!(Pos::parse("s"), Some(Pos::Adjective));
        assert_eq!(Pos::parse("Adverb"), Some(Pos::Adverb));
        assert_eq!(Pos::parse("x"), None);
        for p in Pos::ALL {
            assert_eq!(Pos::from_code(p.code()).unwrap(), p);
        }
        assert!(Pos::from_code(9).is_err());
    }
}
