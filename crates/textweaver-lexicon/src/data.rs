//! The lexicon data file: building it from WordNet and CMUdict, and reading
//! it.
//!
//! # Layout
//!
//! All integers are little endian.
//!
//! - 8 bytes of magic, `TWLEX\r\n\x1a` (a text-mode copy breaks it, as in
//!   PNG), then `u32` format version ([`FORMAT`]) and a `u32` checksum of
//!   the metadata and headword map (FNV-1a, folded to 32 bits): the map
//!   crate trusts its input, so a damaged map is refused before use. The
//!   record blocks need no checksum: every read is bounds-checked, and a
//!   damaged block fails to unpack.
//! - Four sections, each `u64` offset and `u64` length: the metadata
//!   (JSON, [`DataInfo`]), the headword map (an `fst` map), the headword
//!   records, and the synset records (both record stores, `store`).
//! - The sections.
//!
//! The headword map's keys are lowercase headwords with spaces (`ice
//! cream`). Each value holds, from the lowest bit, one bit for each part
//! of speech the word has senses in (noun, verb, adjective, adverb), a bit
//! for a pronunciation, a bit for base forms from WordNet's exception lists,
//! and then the headword record's number. Records are numbered in key
//! order, so the values rise with the keys and the map stays small. Morphy asks the bits,
//! so trying a candidate form never unpacks a block.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::LexiconError;
use crate::codec::Reader;
use crate::model::{
    Definition, Pos, Pronunciation, Sense, SenseGroup, Source, SynsetRecord, WordRecord,
};
use crate::sources::WordNet;
use crate::store::{StoreReader, StoreWriter};

/// The file's magic bytes.
pub const MAGIC: &[u8; 8] = b"TWLEX\r\n\x1a";

/// The format version this crate writes and reads.
pub const FORMAT: u32 = 1;

/// Bytes before the sections.
const HEADER: usize = 16 + 4 * 16;

const PRON_BIT: u64 = 1 << 4;
const BASES_BIT: u64 = 1 << 5;
/// Flag bits below the record number in a map value.
const FLAG_BITS: u32 = 6;

/// Synonyms kept per sense.
const MAX_SYNONYMS: usize = 8;

/// One source of the data, as the metadata records it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Its name, such as `Open English WordNet`.
    pub name: String,
    /// The release or commit.
    pub version: String,
    /// The licence.
    pub licence: String,
    /// Where it came from.
    pub url: String,
    /// The SHA-256 of the downloaded file, in hex.
    pub sha256: String,
}

/// What the data file holds: its sources and counts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataInfo {
    /// The format version.
    pub format: u32,
    /// Where the data came from.
    pub sources: Vec<SourceInfo>,
    /// Headwords in the map.
    pub headwords: u32,
    /// WordNet synsets.
    pub synsets: u32,
    /// Headwords with a pronunciation.
    pub pronunciations: u32,
}

/// Builds the data file's bytes from a parsed WordNet and CMUdict.
///
/// Synsets are numbered word by word, the words with the most senses first,
/// so the senses of a word like `run` or `set` sit together in one or two
/// blocks and a lookup unpacks few.
pub fn build(
    wordnet: &WordNet,
    cmudict: &BTreeMap<String, Vec<String>>,
    sources: Vec<SourceInfo>,
) -> Result<Vec<u8>, LexiconError> {
    let mut synset_ids: HashMap<(Pos, u64), u32> = HashMap::new();
    let mut order: Vec<(Pos, u64)> = Vec::new();
    let mut by_senses: Vec<_> = wordnet.index.iter().collect();
    by_senses.sort_by_key(|(lemma, per_pos)| {
        let n: usize = per_pos.iter().map(|(_, o)| o.len()).sum();
        (std::cmp::Reverse(n), *lemma)
    });
    for (lemma, per_pos) in by_senses {
        for (pos, offsets) in per_pos {
            for &off in offsets {
                // The index lists adjective satellites under `a` too; data
                // lines are keyed by the file they are in.
                if !wordnet.synsets.contains_key(&(*pos, off)) {
                    return Err(LexiconError::Source {
                        what: format!("{lemma}: no {} synset at offset {off}", pos.name()),
                    });
                }
                let next = order.len() as u32;
                synset_ids.entry((*pos, off)).or_insert_with(|| {
                    order.push((*pos, off));
                    next
                });
            }
        }
    }
    // Every headword: WordNet lemmas, exception-list forms, CMUdict words.
    let mut keys: BTreeMap<String, WordRecord> = BTreeMap::new();
    for (lemma, per_pos) in &wordnet.index {
        let rec = keys.entry(lemma.clone()).or_default();
        for (pos, offsets) in per_pos {
            let ids: Vec<u32> = offsets.iter().map(|&o| synset_ids[&(*pos, o)]).collect();
            match rec.synsets.iter_mut().find(|(p, _)| p == pos) {
                Some((_, list)) => list.extend(ids),
                None => rec.synsets.push((*pos, ids)),
            }
        }
    }
    for (form, bases) in &wordnet.exceptions {
        let rec = keys.entry(form.clone()).or_default();
        for b in bases {
            if !rec.bases.contains(b) {
                rec.bases.push(b.clone());
            }
        }
    }
    for (word, prons) in cmudict {
        let key = word.replace('_', " ");
        keys.entry(key).or_default().prons = prons.clone();
    }

    // Synset records, in number order.
    let mut synsets = StoreWriter::new();
    let first_word = |target: &(Pos, u64), n: usize| -> Option<String> {
        let s = wordnet.synsets.get(target)?;
        let i = if n == 0 { 0 } else { n - 1 };
        s.words.get(i).cloned()
    };
    for key in &order {
        let s = &wordnet.synsets[key];
        let mut antonyms = Vec::new();
        let mut kind_of = Vec::new();
        for p in &s.pointers {
            let list = match p.symbol.as_str() {
                "!" => &mut antonyms,
                "@" | "@i" => &mut kind_of,
                _ => continue,
            };
            if let Some(w) = first_word(&p.target, p.target_word)
                && !list.contains(&w)
            {
                list.push(w);
            }
        }
        let rec = SynsetRecord {
            words: s.words.clone(),
            gloss: s.definition.clone(),
            examples: s.examples.clone(),
            antonyms,
            kind_of,
        };
        synsets.push(rec.encode());
    }

    // Headword records and the map.
    let mut words = StoreWriter::new();
    let mut map = fst::MapBuilder::memory();
    let mut pronunciations = 0u32;
    for (key, rec) in &keys {
        if key.is_empty() {
            continue;
        }
        let id = words.push(rec.encode());
        let mut value = u64::from(id) << FLAG_BITS;
        for (pos, ids) in &rec.synsets {
            if !ids.is_empty() {
                value |= pos.bit();
            }
        }
        if !rec.prons.is_empty() {
            value |= PRON_BIT;
            pronunciations += 1;
        }
        if !rec.bases.is_empty() {
            value |= BASES_BIT;
        }
        map.insert(key.as_bytes(), value)
            .map_err(|e| LexiconError::Build(e.to_string()))?;
    }
    let map = map
        .into_inner()
        .map_err(|e| LexiconError::Build(e.to_string()))?;
    let info = DataInfo {
        format: FORMAT,
        sources,
        headwords: words_count(&keys),
        synsets: order.len() as u32,
        pronunciations,
    };
    let meta = serde_json::to_vec(&info).map_err(|e| LexiconError::Build(e.to_string()))?;
    let sum = checksum(&[&meta, &map]);
    let sections = [meta, map, words.finish(), synsets.finish()];
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT.to_le_bytes());
    out.extend_from_slice(&sum.to_le_bytes());
    let mut offset = HEADER as u64;
    for s in &sections {
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&(s.len() as u64).to_le_bytes());
        offset += s.len() as u64;
    }
    for s in sections {
        out.extend_from_slice(&s);
    }
    Ok(out)
}

/// FNV-1a over parts, folded to 32 bits.
fn checksum(parts: &[&[u8]]) -> u32 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        for b in *p {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    (h ^ (h >> 32)) as u32
}

fn words_count(keys: &BTreeMap<String, WordRecord>) -> u32 {
    keys.keys().filter(|k| !k.is_empty()).count() as u32
}

/// A byte range of the shared file, for the `fst` map.
#[derive(Clone)]
struct Section {
    data: Arc<Vec<u8>>,
    range: Range<usize>,
}

impl AsRef<[u8]> for Section {
    fn as_ref(&self) -> &[u8] {
        &self.data[self.range.clone()]
    }
}

/// An open lexicon data file.
pub struct Lexicon {
    data: Arc<Vec<u8>>,
    map: fst::Map<Section>,
    words: (Range<usize>, StoreReader),
    synsets: (Range<usize>, StoreReader),
    info: DataInfo,
}

impl std::fmt::Debug for Lexicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lexicon")
            .field("bytes", &self.data.len())
            .field("info", &self.info)
            .finish()
    }
}

impl Lexicon {
    /// Opens the data file at `path`, reading it into memory.
    pub fn open(path: &Path) -> Result<Lexicon, LexiconError> {
        let bytes = std::fs::read(path).map_err(|source| LexiconError::Io {
            path: path.to_owned(),
            source,
        })?;
        Lexicon::from_bytes(bytes)
    }

    /// A lexicon over the bytes of a data file, checking its header, map,
    /// and block tables.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Lexicon, LexiconError> {
        if bytes.len() < HEADER || &bytes[..8] != MAGIC {
            return Err(LexiconError::NotLexicon);
        }
        let mut r = Reader::new(&bytes[8..HEADER]);
        let format = r.u32_le()?;
        if format != FORMAT {
            return Err(LexiconError::Version(format));
        }
        let sum = r.u32_le()?;
        let mut ranges = Vec::with_capacity(4);
        for _ in 0..4 {
            let off = usize::try_from(r.u64_le()?)
                .map_err(|_| LexiconError::Corrupt("section offset"))?;
            let len = usize::try_from(r.u64_le()?)
                .map_err(|_| LexiconError::Corrupt("section length"))?;
            let end = off
                .checked_add(len)
                .filter(|&e| e <= bytes.len() && off >= HEADER)
                .ok_or(LexiconError::Corrupt("section outside the file"))?;
            ranges.push(off..end);
        }
        if checksum(&[&bytes[ranges[0].clone()], &bytes[ranges[1].clone()]]) != sum {
            return Err(LexiconError::Corrupt("checksum does not match"));
        }
        let info: DataInfo = serde_json::from_slice(&bytes[ranges[0].clone()])
            .map_err(|_| LexiconError::Corrupt("metadata"))?;
        let data = Arc::new(bytes);
        let map = fst::Map::new(Section {
            data: data.clone(),
            range: ranges[1].clone(),
        })
        .map_err(|_| LexiconError::Corrupt("headword map"))?;
        // The map's own checksum: fst trusts its node addresses, and a
        // damaged map made it panic on an overflow (the lexicon fuzz target).
        map.as_fst()
            .verify()
            .map_err(|_| LexiconError::Corrupt("headword map checksum"))?;
        let words = StoreReader::new(&data[ranges[2].clone()])?;
        let synsets = StoreReader::new(&data[ranges[3].clone()])?;
        Ok(Lexicon {
            map,
            words: (ranges[2].clone(), words),
            synsets: (ranges[3].clone(), synsets),
            info,
            data,
        })
    }

    /// The file's sources and counts.
    pub fn info(&self) -> &DataInfo {
        &self.info
    }

    /// The file's size in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// How many headwords the map holds.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when the map holds no headwords.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// True when WordNet has `word` (lowercase) in `pos`.
    pub fn has(&self, word: &str, pos: Pos) -> bool {
        self.map.get(word).is_some_and(|v| v & pos.bit() != 0)
    }

    /// True when `word` (lowercase) is a headword at all.
    pub fn contains(&self, word: &str) -> bool {
        self.map.contains_key(word)
    }

    fn record(&self, word: &str) -> Result<Option<(u64, WordRecord)>, LexiconError> {
        let Some(v) = self.map.get(word) else {
            return Ok(None);
        };
        let id =
            u32::try_from(v >> FLAG_BITS).map_err(|_| LexiconError::Corrupt("record number"))?;
        let bytes = self.words.1.get(&self.data[self.words.0.clone()], id)?;
        Ok(Some((v, WordRecord::decode(&bytes)?)))
    }

    fn synset(&self, id: u32) -> Result<SynsetRecord, LexiconError> {
        let bytes = self.synsets.1.get(&self.data[self.synsets.0.clone()], id)?;
        SynsetRecord::decode(&bytes)
    }

    /// The pronunciations CMUdict gives for `word` (lowercase).
    pub fn pronunciations(&self, word: &str) -> Result<Vec<Pronunciation>, LexiconError> {
        Ok(match self.record(word)? {
            Some((v, rec)) if v & PRON_BIT != 0 => rec.prons.iter().map(|p| pron(p)).collect(),
            _ => Vec::new(),
        })
    }

    /// The base forms of `word` (lowercase) in each part of speech, the
    /// word itself first when WordNet has it: morphy.
    pub fn base_forms(&self, word: &str) -> Result<Vec<(String, Pos)>, LexiconError> {
        let bases = match self.map.get(word) {
            Some(v) if v & BASES_BIT != 0 => self.record(word)?.map(|r| r.1.bases),
            _ => None,
        }
        .unwrap_or_default();
        let mut out = Vec::new();
        for pos in Pos::ALL {
            let exceptions: Vec<String> = bases
                .iter()
                .filter(|(p, _)| *p == pos)
                .map(|(_, b)| b.clone())
                .collect();
            for f in crate::morphy::base_forms(word, pos, &exceptions, |f, p| self.has(f, p)) {
                out.push((f, pos));
            }
        }
        // The word's own entries first, keeping part-of-speech order.
        out.sort_by_key(|(f, _)| f != word);
        Ok(out)
    }

    /// Looks `word` up in WordNet (through morphy) and CMUdict. The word
    /// is normalized first ([`normalize`]); a possessive, and a hyphen or
    /// space in place of the other, are tried when the word itself is not
    /// found. `None` when nothing is known about it.
    pub fn define(&self, word: &str) -> Result<Option<Definition>, LexiconError> {
        let word = normalize(word);
        if word.is_empty() {
            return Ok(None);
        }
        for candidate in candidates(&word) {
            if let Some(d) = self.define_exact(&candidate)? {
                return Ok(Some(d));
            }
        }
        Ok(None)
    }

    fn define_exact(&self, word: &str) -> Result<Option<Definition>, LexiconError> {
        let lemmas = self.base_forms(word)?;
        let mut groups: Vec<SenseGroup> = Vec::new();
        for (lemma, pos) in &lemmas {
            let Some((_, rec)) = self.record(lemma)? else {
                continue;
            };
            let Some((_, ids)) = rec.synsets.iter().find(|(p, _)| p == pos) else {
                continue;
            };
            let mut senses = Vec::with_capacity(ids.len());
            for &id in ids {
                let s = self.synset(id)?;
                let synonyms: Vec<String> = s
                    .words
                    .iter()
                    .filter(|w| !w.eq_ignore_ascii_case(lemma))
                    .take(MAX_SYNONYMS)
                    .cloned()
                    .collect();
                senses.push(Sense {
                    definition: s.gloss,
                    examples: s.examples,
                    synonyms,
                    antonyms: s.antonyms,
                    kind_of: s.kind_of,
                });
            }
            groups.push(SenseGroup {
                lemma: lemma.clone(),
                pos: Some(*pos),
                senses,
            });
        }
        let mut pronunciations = self.pronunciations(word)?;
        if pronunciations.is_empty()
            && let Some((lemma, _)) = lemmas.first()
        {
            pronunciations = self.pronunciations(lemma)?;
        }
        if groups.is_empty() && pronunciations.is_empty() {
            return Ok(None);
        }
        Ok(Some(Definition {
            word: word.to_owned(),
            source: if groups.is_empty() {
                Source::Pronunciation
            } else {
                Source::WordNet
            },
            pronunciations,
            groups,
        }))
    }

    /// Headwords that start with `prefix` (lowercase), at most `limit`, in
    /// order: for completing a word as it is typed.
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<String> {
        use fst::{IntoStreamer, Streamer};
        let prefix = normalize(prefix);
        if prefix.is_empty() {
            return Vec::new();
        }
        let matcher = fst::automaton::Str::new(&prefix);
        let mut stream = self
            .map
            .search(fst::Automaton::starts_with(matcher))
            .into_stream();
        let mut out = Vec::new();
        while let Some((k, _)) = stream.next() {
            if out.len() >= limit {
                break;
            }
            out.push(String::from_utf8_lossy(k).into_owned());
        }
        out
    }
}

/// A pronunciation with its respelling.
pub(crate) fn pron(arpabet: &str) -> Pronunciation {
    Pronunciation {
        arpabet: arpabet.to_owned(),
        respelling: crate::pronounce::respell(arpabet),
    }
}

/// A word as the lexicon keys it: surrounding punctuation trimmed, curly
/// apostrophes made straight, spaces collapsed, lowercase.
pub fn normalize(word: &str) -> String {
    let w = word.replace(['\u{2019}', '\u{2018}', '\u{02bc}'], "'");
    let w = w.trim_matches(|c: char| !c.is_alphanumeric());
    w.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The forms to try for a normalized word: itself, without a possessive,
/// and with hyphens and spaces swapped.
fn candidates(word: &str) -> Vec<String> {
    let mut out = vec![word.to_owned()];
    let mut push = |s: String| {
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    };
    if let Some(stem) = word.strip_suffix("'s") {
        push(stem.to_owned());
    } else if let Some(stem) = word.strip_suffix('\'') {
        push(stem.to_owned());
    }
    if word.contains('-') {
        push(word.replace('-', " "));
        push(word.replace('-', ""));
    }
    if word.contains(' ') {
        push(word.replace(' ', "-"));
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::sources::read_cmudict;

    /// A tiny WordNet in WNDB form, for tests across the crate.
    pub(crate) fn tiny() -> Lexicon {
        let mut wn = WordNet::default();
        wn.add_data(
            Pos::Noun,
            "  licence\n\
             00000100 05 n 02 dog 0 domestic_dog 0 002 @ 00000200 n 0000 ! 00000300 n 0101 | a member of the genus Canis; \"the dog barked all night\"\n\
             00000200 05 n 01 canine 0 000 | a mammal with long jaws\n\
             00000300 05 n 01 cat 0 000 | a small domesticated feline\n\
             00000400 04 n 01 running 0 000 | the act of running; traveling on foot at a fast pace\n\
             00000500 05 n 01 goose 0 000 | web-footed long-necked bird\n\
             00000600 04 n 01 ice_cream 0 000 | frozen dessert\n",
        )
        .unwrap();
        wn.add_data(
            Pos::Verb,
            "00000700 38 v 02 run 0 go 0 000 01 + 02 00 | move fast by using one's feet; \"Don't run--you'll be out of breath\"\n\
             00000800 38 v 01 run 1 000 | be in charge of; \"She runs the department\"\n",
        )
        .unwrap();
        wn.add_index(
            Pos::Noun,
            "dog n 1 2 @ ! 1 0 00000100\ncanine n 1 0 1 0 00000200\ncat n 1 0 1 0 00000300\n\
             running n 1 0 1 0 00000400\ngoose n 1 0 1 0 00000500\nice_cream n 1 0 1 0 00000600\n",
        )
        .unwrap();
        wn.add_index(
            Pos::Verb,
            "run v 2 0 2 0 00000700 00000800\ngo v 1 0 1 0 00000700\n",
        )
        .unwrap();
        wn.add_exceptions(Pos::Noun, "geese goose\n");
        wn.add_exceptions(Pos::Verb, "ran run\nrunning run\n");
        let cmu = read_cmudict(
            "dog D AO1 G\nrunning R AH1 N IH0 NG\nrun R AH1 N\nthe DH AH0\nthe(2) DH IY0\n",
        );
        let bytes = build(&wn, &cmu, vec![SourceInfo::default()]).unwrap();
        Lexicon::from_bytes(bytes).unwrap()
    }

    #[test]
    fn a_damaged_headword_map_is_refused() {
        // The lexicon fuzz target found a damaged map that made fst panic
        // on an overflow. Every byte of the map, flipped, with the file's
        // own checksum made right again, is refused at open.
        let good = tiny().data.to_vec();
        let at = |i: usize| u64::from_le_bytes(good[i..i + 8].try_into().unwrap()) as usize;
        let (meta, map) = ((at(16), at(24)), (at(32), at(40)));
        for i in map.0..map.0 + map.1 {
            let mut bytes = good.clone();
            bytes[i] ^= 0xff;
            let sum = checksum(&[
                &bytes[meta.0..meta.0 + meta.1],
                &bytes[map.0..map.0 + map.1],
            ]);
            bytes[12..16].copy_from_slice(&sum.to_le_bytes());
            assert!(Lexicon::from_bytes(bytes).is_err(), "byte {i} flipped");
        }
    }

    #[test]
    fn defines_with_morphy_and_pronunciation() {
        let lex = tiny();
        assert_eq!(lex.info().synsets, 8);
        let d = lex.define("Dogs,").unwrap().unwrap();
        assert_eq!(d.word, "dogs");
        assert_eq!(d.source, Source::WordNet);
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].lemma, "dog");
        let s = &d.groups[0].senses[0];
        assert_eq!(s.definition, "a member of the genus Canis");
        assert_eq!(s.synonyms, ["domestic dog"]);
        assert_eq!(s.kind_of, ["canine"]);
        assert_eq!(s.antonyms, ["cat"]);
        assert_eq!(d.pronunciations[0].respelling, "DAWG");

        let d = lex.define("running").unwrap().unwrap();
        let groups: Vec<(&str, Option<Pos>)> =
            d.groups.iter().map(|g| (g.lemma.as_str(), g.pos)).collect();
        assert_eq!(
            groups,
            [("running", Some(Pos::Noun)), ("run", Some(Pos::Verb))]
        );
        assert_eq!(d.groups[1].senses.len(), 2);
        assert_eq!(d.groups[1].senses[0].synonyms, ["go"]);
        assert_eq!(d.pronunciations[0].respelling, "RUN-ing");

        let d = lex.define("geese").unwrap().unwrap();
        assert_eq!(d.groups[0].lemma, "goose");
        let d = lex.define("ran").unwrap().unwrap();
        assert_eq!(d.groups[0].lemma, "run");
        // The base form's pronunciation when the word has none.
        assert_eq!(d.pronunciations[0].arpabet, "R AH1 N");
        let d = lex.define("ice-cream").unwrap().unwrap();
        assert_eq!(d.groups[0].lemma, "ice cream");
        let d = lex.define("dog\u{2019}s").unwrap().unwrap();
        assert_eq!(d.groups[0].lemma, "dog");

        let d = lex.define("the").unwrap().unwrap();
        assert_eq!(d.source, Source::Pronunciation);
        assert_eq!(d.pronunciations.len(), 2);
        assert!(lex.define("zzzz").unwrap().is_none());
        assert!(lex.define("  ...  ").unwrap().is_none());
        assert_eq!(lex.complete("ru", 5), ["run", "running"]);
    }

    #[test]
    fn rejects_other_files() {
        assert!(matches!(
            Lexicon::from_bytes(b"not a lexicon".to_vec()),
            Err(LexiconError::NotLexicon)
        ));
        let lex = tiny();
        let mut bytes = lex.data.as_ref().clone();
        bytes[8] = 99;
        assert!(matches!(
            Lexicon::from_bytes(bytes),
            Err(LexiconError::Version(99))
        ));
    }

    #[test]
    fn damaged_files_are_errors_not_panics() {
        let bytes = tiny().data.as_ref().clone();
        for at in (0..bytes.len()).step_by(7) {
            let mut b = bytes.clone();
            b[at] ^= 0xa5;
            if let Ok(lex) = Lexicon::from_bytes(b) {
                for w in ["dog", "running", "geese", "the", "zz"] {
                    let _ = lex.define(w);
                }
            }
        }
        for cut in (0..bytes.len()).step_by(11) {
            let _ = Lexicon::from_bytes(bytes[..cut].to_vec());
        }
    }

    #[test]
    fn normalizing() {
        assert_eq!(normalize("  \u{201c}Hello,\u{201d} "), "hello");
        assert_eq!(normalize("ice   Cream!"), "ice cream");
        assert_eq!(normalize("rock\u{2019}n\u{2019}roll"), "rock'n'roll");
        assert_eq!(candidates("dog's"), ["dog's", "dog"]);
        assert_eq!(candidates("e-mail"), ["e-mail", "e mail", "email"]);
    }
}
