//! Fuzz target: the lexicon data file reader (`textweaver-lexicon`'s
//! `data.rs`). A damaged file is refused with an error, never read out of
//! bounds and never a panic.
//!
//! Input that starts with the file's magic bytes is read as a whole file,
//! once as it is and once with its checksum made right, so the fuzzer gets
//! past the checksum to the headword map and the blocks. Any other input
//! is a list of patches to a small valid lexicon built in memory: each
//! patch is a four-byte little-endian offset, a length byte, and that many
//! bytes written over the file there. The checksum is made right after
//! patching. Every lexicon that opens is then looked up, completed, and
//! defined.

#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use textweaver_lexicon::sources::{WordNet, read_cmudict};
use textweaver_lexicon::{Lexicon, MAGIC, Pos, SourceInfo, build};

/// The header: magic (8), format (4), checksum (4), then four sections of
/// a u64 offset and a u64 length.
const HEADER: usize = 16 + 4 * 16;

/// A few words of WordNet and CMUdict, as the crate's own tests use.
fn tiny() -> &'static [u8] {
    static TINY: OnceLock<Vec<u8>> = OnceLock::new();
    TINY.get_or_init(|| {
        let mut wn = WordNet::default();
        wn.add_data(
            Pos::Noun,
            "00000100 05 n 02 dog 0 domestic_dog 0 002 @ 00000200 n 0000 ! 00000300 n 0101 | a member of the genus Canis; \"the dog barked\"\n\
             00000200 05 n 01 canine 0 000 | a mammal with long jaws\n\
             00000300 05 n 01 cat 0 000 | a small domesticated feline\n\
             00000500 05 n 01 goose 0 000 | web-footed long-necked bird\n",
        )
        .expect("tiny WordNet data");
        wn.add_data(
            Pos::Verb,
            "00000700 38 v 02 run 0 go 0 000 01 + 02 00 | move fast by using one's feet\n",
        )
        .expect("tiny WordNet data");
        wn.add_index(
            Pos::Noun,
            "dog n 1 2 @ ! 1 0 00000100\ncanine n 1 0 1 0 00000200\ncat n 1 0 1 0 00000300\ngoose n 1 0 1 0 00000500\n",
        )
        .expect("tiny WordNet index");
        wn.add_index(Pos::Verb, "run v 1 0 1 0 00000700\ngo v 1 0 1 0 00000700\n")
            .expect("tiny WordNet index");
        wn.add_exceptions(Pos::Noun, "geese goose\n");
        wn.add_exceptions(Pos::Verb, "ran run\n");
        let cmu = read_cmudict("dog D AO1 G\nrun R AH1 N\nthe DH AH0\nthe(2) DH IY0\n");
        build(&wn, &cmu, vec![SourceInfo::default()]).expect("tiny lexicon builds")
    })
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// Writes the checksum the reader expects (FNV-1a over the metadata and
/// the headword map, folded to 32 bits), when the header's first two
/// sections lie inside the file.
fn restamp(b: &mut [u8]) {
    let section = |i: usize| -> Option<std::ops::Range<usize>> {
        let off = usize::try_from(u64_at(b, 16 + 16 * i)?).ok()?;
        let len = usize::try_from(u64_at(b, 24 + 16 * i)?).ok()?;
        let end = off.checked_add(len).filter(|&e| e <= b.len())?;
        Some(off..end)
    };
    let (Some(meta), Some(map)) = (section(0), section(1)) else {
        return;
    };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in b[meta].iter().chain(&b[map]) {
        h ^= u64::from(*byte);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let sum = (h ^ (h >> 32)) as u32;
    b[12..16].copy_from_slice(&sum.to_le_bytes());
}

fn exercise(bytes: Vec<u8>) {
    let Ok(lex) = Lexicon::from_bytes(bytes) else {
        return;
    };
    let _ = lex.info();
    let _ = (lex.len(), lex.is_empty(), lex.size());
    for word in [
        "dog", "dogs", "geese", "ran", "running", "cat's", "the", "go", "",
    ] {
        let _ = lex.contains(word);
        let _ = lex.pronunciations(word);
        let _ = lex.base_forms(word);
        let _ = lex.define(word);
    }
    for prefix in ["d", "g", "ca", "z"] {
        let _ = lex.complete(prefix, 8);
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() >= HEADER && data.starts_with(MAGIC) {
        exercise(data.to_vec());
        let mut fixed = data.to_vec();
        restamp(&mut fixed);
        exercise(fixed);
        return;
    }
    let mut file = tiny().to_vec();
    let mut rest = data;
    while rest.len() >= 5 {
        let off = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize % file.len();
        let n = usize::from(rest[4]).min(rest.len() - 5);
        let patch = &rest[5..5 + n];
        let end = (off + patch.len()).min(file.len());
        file[off..end].copy_from_slice(&patch[..end - off]);
        rest = &rest[5 + n..];
    }
    restamp(&mut file);
    exercise(file);
});
