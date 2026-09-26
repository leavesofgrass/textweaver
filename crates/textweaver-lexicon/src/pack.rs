//! zstd compression with a better match finder than `ruzstd`'s own.
//!
//! `ruzstd` 0.9 implements only its fastest level, whose match finder keeps
//! one candidate per hash and needs five equal bytes; on the lexicon's text
//! it packs about 1.6 to 1. This module plugs a hash-chain match finder
//! with lazy matching into `ruzstd`'s encoder through its [`Matcher`]
//! trait: the output is ordinary zstd, read by any decoder, and the data
//! file comes out about a quarter smaller. Only the build uses it.
//!
//! It also steers around two bugs in `ruzstd` 0.9's encoder, worth
//! reporting upstream: match lengths over 65,538 are written with the wrong
//! baseline (code 52 subtracts 32,771 instead of 65,539), and building a
//! table panics when every sequence of a block has literal length code 0.

use ruzstd::encoding::{CompressionLevel, FrameCompressor, Matcher, Sequence};

/// Shortest match worth a sequence.
const MIN_MATCH: usize = 4;
/// Longest match: ruzstd 0.9 writes longer ones wrongly (its match length
/// code 52 subtracts the wrong baseline), so they are split.
const MAX_MATCH: usize = 65538;
/// Chain links followed per position.
const MAX_CHAIN: usize = 256;
/// Hash table size, as a power of two.
const HASH_BITS: u32 = 16;
/// Bytes per space the encoder asks for (zstd's largest block).
const SPACE: usize = 128 * 1024;
/// The window: how far back a match may reach. The lexicon's frames are
/// smaller, so every match may reach the start of its frame, and a reader
/// needs no more than this much buffer.
const WINDOW: usize = 1 << 17;

/// A hash-chain match finder over the whole frame.
pub(crate) struct ChainMatcher {
    history: Vec<u8>,
    last_start: usize,
    head: Vec<u32>,
    prev: Vec<u32>,
    inserted: usize,
}

impl ChainMatcher {
    pub(crate) fn new() -> Self {
        ChainMatcher {
            history: Vec::new(),
            last_start: 0,
            head: vec![0; 1 << HASH_BITS],
            prev: Vec::new(),
            inserted: 0,
        }
    }

    fn hash(&self, at: usize) -> usize {
        let b = &self.history[at..at + MIN_MATCH];
        let v = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        (v.wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize
    }

    /// Adds every position before `upto` to the chains.
    fn insert_until(&mut self, upto: usize) {
        let last = self.history.len().saturating_sub(MIN_MATCH - 1);
        let upto = upto.min(last);
        while self.inserted < upto {
            let at = self.inserted;
            let h = self.hash(at);
            self.prev[at] = self.head[h];
            self.head[h] = (at + 1) as u32;
            self.inserted += 1;
        }
    }

    /// The longest earlier match for `at`: `(length, distance)`.
    fn find(&self, at: usize, end: usize) -> (usize, usize) {
        if at + MIN_MATCH > end {
            return (0, 0);
        }
        let mut best = (0, 0);
        let mut cand = self.head[self.hash(at)];
        let mut depth = 0;
        let limit = (end - at).min(MAX_MATCH);
        while cand != 0 && depth < MAX_CHAIN {
            let p = cand as usize - 1;
            if at - p > WINDOW {
                break;
            }
            let len = self.history[p..]
                .iter()
                .zip(&self.history[at..at + limit])
                .take_while(|(a, b)| a == b)
                .count();
            if len > best.0 {
                best = (len, at - p);
                if len == limit {
                    break;
                }
            }
            cand = self.prev[p];
            depth += 1;
        }
        if best.0 >= MIN_MATCH { best } else { (0, 0) }
    }
}

impl Matcher for ChainMatcher {
    fn get_next_space(&mut self) -> Vec<u8> {
        vec![0; SPACE]
    }

    fn get_last_space(&mut self) -> &[u8] {
        &self.history[self.last_start..]
    }

    fn commit_space(&mut self, space: Vec<u8>) {
        self.last_start = self.history.len();
        self.history.extend_from_slice(&space);
        self.prev.resize(self.history.len(), 0);
    }

    fn skip_matching(&mut self) {
        self.insert_until(self.history.len());
    }

    fn start_matching(&mut self, mut handle_sequence: impl for<'a> FnMut(Sequence<'a>)) {
        let end = self.history.len();
        let mut at = self.last_start;
        let mut literals_from = at;
        // (literals start, match start, distance, length)
        let mut found: Vec<(usize, usize, usize, usize)> = Vec::new();
        while at + MIN_MATCH <= end {
            self.insert_until(at);
            let (len, dist) = self.find(at, end);
            if len == 0 {
                at += 1;
                continue;
            }
            // Lazy matching: a longer match one byte on wins.
            self.insert_until(at + 1);
            let (next_len, _) = self.find(at + 1, end);
            if next_len > len + 1 {
                at += 1;
                continue;
            }
            found.push((literals_from, at, dist, len));
            at += len;
            literals_from = at;
        }
        self.insert_until(end);
        // ruzstd's table builder panics when every sequence of a block has
        // the same literal length code 0 (no literals at all): give the
        // first sequence one literal. Its match stays at least 3 bytes,
        // zstd's shortest.
        if found.iter().all(|f| f.0 == f.1)
            && let Some(first) = found.first_mut()
        {
            first.1 += 1;
            first.3 -= 1;
        }
        for (lit, start, offset, match_len) in found {
            handle_sequence(Sequence::Triple {
                literals: &self.history[lit..start],
                offset,
                match_len,
            });
        }
        if literals_from < end {
            handle_sequence(Sequence::Literals {
                literals: &self.history[literals_from..end],
            });
        }
    }

    fn reset(&mut self, _level: CompressionLevel) {
        self.history.clear();
        self.prev.clear();
        self.head.iter_mut().for_each(|h| *h = 0);
        self.last_start = 0;
        self.inserted = 0;
    }

    fn window_size(&self) -> u64 {
        WINDOW as u64
    }
}

/// `data` as one zstd frame.
pub(crate) fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut c = FrameCompressor::new_with_matcher(ChainMatcher::new(), CompressionLevel::Fastest);
    c.set_source(data);
    c.set_drain(&mut out);
    c.compress();
    out
}

#[cfg(test)]
mod tests {
    use ruzstd::decoding::FrameDecoder;

    use super::*;

    fn unpack(packed: &[u8], len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        FrameDecoder::new()
            .decode_all_to_vec(packed, &mut out)
            .unwrap_or_else(|e| panic!("{len} bytes: {e:?}"));
        out
    }

    #[test]
    fn round_trips_and_packs_text_well() {
        let text: String = (0..3000)
            .map(|i| {
                format!(
                    "sense {i}: a member of the genus Canis; \"the dog barked\" {}\n",
                    i % 17
                )
            })
            .collect();
        let packed = compress(text.as_bytes());
        assert_eq!(unpack(&packed, text.len()), text.as_bytes());
        let fastest = ruzstd::encoding::compress_to_vec(text.as_bytes(), CompressionLevel::Fastest);
        assert!(
            packed.len() < fastest.len(),
            "{} vs {}",
            packed.len(),
            fastest.len()
        );
        for data in [
            &b""[..],
            b"a",
            b"abcd",
            b"aaaaaaaaaaaaaaaaaaaaaaaa",
            b"abcabcabcabcabcabc",
        ] {
            assert_eq!(unpack(&compress(data), data.len()), data);
        }
        // Over one zstd block (128 KiB), so the window spans spaces.
        let big: Vec<u8> = (0..300_000u32).map(|i| (i * 7 % 251) as u8).collect();
        assert_eq!(unpack(&compress(&big), big.len()), big);
    }
}
