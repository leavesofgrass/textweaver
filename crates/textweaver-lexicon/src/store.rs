//! Numbered records in zstd blocks.
//!
//! Records are packed into blocks of about [`BLOCK_TARGET`] bytes, and each
//! block is compressed on its own, so a lookup unpacks one block. A store
//! is laid out as:
//!
//! - `u32` record count, `u32` block count;
//! - one entry per block: `u32` first record, `u32` unpacked length,
//!   `u64` offset from the start of the store, `u32` packed length;
//! - the packed blocks.
//!
//! An unpacked block is `u32` record count, then `count + 1` `u32` offsets
//! into the block's record bytes, then the bytes. All integers are little
//! endian.

use std::sync::Mutex;

use ruzstd::decoding::FrameDecoder;

use crate::LexiconError;
use crate::codec::Reader;

/// Unpacked bytes per block. Bigger blocks pack better and cost more to
/// unpack per lookup; 48 KiB unpacks in well under a millisecond.
pub(crate) const BLOCK_TARGET: usize = 48 * 1024;

/// Bytes per block table entry.
const ENTRY: usize = 20;

/// Largest unpacked block a reader accepts (a damaged length cannot ask for
/// more memory than this).
const MAX_BLOCK: u32 = 16 * 1024 * 1024;

/// Builds a store from records given in order.
pub(crate) struct StoreWriter {
    blocks: Vec<(u32, Vec<Vec<u8>>)>,
    current: Vec<Vec<u8>>,
    current_len: usize,
    first: u32,
    count: u32,
}

impl StoreWriter {
    pub(crate) fn new() -> Self {
        StoreWriter {
            blocks: Vec::new(),
            current: Vec::new(),
            current_len: 0,
            first: 0,
            count: 0,
        }
    }

    /// Adds a record; returns its number.
    pub(crate) fn push(&mut self, record: Vec<u8>) -> u32 {
        if self.current_len >= BLOCK_TARGET {
            self.seal();
        }
        self.current_len += record.len() + 4;
        self.current.push(record);
        let id = self.count;
        self.count += 1;
        id
    }

    fn seal(&mut self) {
        if self.current.is_empty() {
            return;
        }
        let records = std::mem::take(&mut self.current);
        let n = records.len() as u32;
        self.blocks.push((self.first, records));
        self.first += n;
        self.current_len = 0;
    }

    /// The store's bytes.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.seal();
        let packed: Vec<(u32, u32, Vec<u8>)> = self
            .blocks
            .iter()
            .map(|(first, records)| {
                let raw = unpacked_block(records);
                let comp = crate::pack::compress(&raw);
                (*first, raw.len() as u32, comp)
            })
            .collect();
        let mut out = Vec::new();
        out.extend_from_slice(&self.count.to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        let mut offset = (8 + packed.len() * ENTRY) as u64;
        for (first, raw_len, comp) in &packed {
            out.extend_from_slice(&first.to_le_bytes());
            out.extend_from_slice(&raw_len.to_le_bytes());
            out.extend_from_slice(&offset.to_le_bytes());
            out.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            offset += comp.len() as u64;
        }
        for (_, _, comp) in packed {
            out.extend_from_slice(&comp);
        }
        out
    }
}

fn unpacked_block(records: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(records.len() as u32).to_le_bytes());
    let mut at = 0u32;
    for r in records {
        out.extend_from_slice(&at.to_le_bytes());
        at += r.len() as u32;
    }
    out.extend_from_slice(&at.to_le_bytes());
    for r in records {
        out.extend_from_slice(r);
    }
    out
}

/// One block table entry.
#[derive(Clone, Copy, Debug)]
struct Block {
    first: u32,
    raw_len: u32,
    offset: usize,
    len: usize,
}

/// Reads records from a store's bytes, keeping the last few unpacked
/// blocks.
pub(crate) struct StoreReader {
    count: u32,
    blocks: Vec<Block>,
    cache: Mutex<Vec<(usize, std::sync::Arc<Vec<u8>>)>>,
}

/// Unpacked blocks kept per store.
const CACHE_BLOCKS: usize = 16;

impl StoreReader {
    /// Reads the block table of the store in `bytes`, checking that every
    /// block lies inside it.
    pub(crate) fn new(bytes: &[u8]) -> Result<Self, LexiconError> {
        let mut r = Reader::new(bytes);
        let count = r.u32_le()?;
        let n_blocks = r.u32_le()? as usize;
        if n_blocks.saturating_mul(ENTRY) > bytes.len() {
            return Err(LexiconError::Corrupt("block table larger than the file"));
        }
        let mut blocks = Vec::with_capacity(n_blocks);
        let mut expect_first = 0u32;
        for _ in 0..n_blocks {
            let first = r.u32_le()?;
            let raw_len = r.u32_le()?;
            let offset = r.u64_le()?;
            let len = r.u32_le()?;
            let offset =
                usize::try_from(offset).map_err(|_| LexiconError::Corrupt("block offset"))?;
            let len = len as usize;
            if offset.checked_add(len).is_none_or(|end| end > bytes.len())
                || raw_len > MAX_BLOCK
                || first != expect_first && blocks.is_empty()
                || first < expect_first
            {
                return Err(LexiconError::Corrupt("block table entry out of range"));
            }
            expect_first = first;
            blocks.push(Block {
                first,
                raw_len,
                offset,
                len,
            });
        }
        Ok(StoreReader {
            count,
            blocks,
            cache: Mutex::new(Vec::new()),
        })
    }

    /// How many records the store holds.
    #[cfg(test)]
    pub(crate) fn len(&self) -> u32 {
        self.count
    }

    /// Record `id`, unpacked from `bytes` (the same bytes given to
    /// [`new`](Self::new)).
    pub(crate) fn get(&self, bytes: &[u8], id: u32) -> Result<Vec<u8>, LexiconError> {
        if id >= self.count {
            return Err(LexiconError::Corrupt("record number out of range"));
        }
        let b = self.blocks.partition_point(|b| b.first <= id);
        let index = b
            .checked_sub(1)
            .ok_or(LexiconError::Corrupt("record not in any block"))?;
        let block = self.block(bytes, index)?;
        let mut r = Reader::new(&block);
        let n = r.u32_le()?;
        let local = id - self.blocks[index].first;
        if local >= n {
            return Err(LexiconError::Corrupt("record not in its block"));
        }
        let table = 4 + (n as usize + 1) * 4;
        let at = |i: u32| -> Result<usize, LexiconError> {
            let p = 4 + i as usize * 4;
            let s = block
                .get(p..p + 4)
                .ok_or(LexiconError::Corrupt("record table ends early"))?;
            Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize)
        };
        let (start, end) = (at(local)?, at(local + 1)?);
        block
            .get(table + start..table + end)
            .map(<[u8]>::to_vec)
            .ok_or(LexiconError::Corrupt("record outside its block"))
    }

    fn block(&self, bytes: &[u8], index: usize) -> Result<std::sync::Arc<Vec<u8>>, LexiconError> {
        if let Ok(mut cache) = self.cache.lock()
            && let Some(pos) = cache.iter().position(|(i, _)| *i == index)
        {
            let hit = cache.remove(pos);
            let data = hit.1.clone();
            cache.push(hit);
            return Ok(data);
        }
        let b = self.blocks[index];
        let packed = bytes
            .get(b.offset..b.offset + b.len)
            .ok_or(LexiconError::Corrupt("block outside the file"))?;
        let mut out = Vec::with_capacity(b.raw_len as usize);
        FrameDecoder::new()
            .decode_all_to_vec(packed, &mut out)
            .map_err(|_| LexiconError::Corrupt("a block does not unpack"))?;
        if out.len() != b.raw_len as usize {
            return Err(LexiconError::Corrupt("a block unpacks to the wrong length"));
        }
        let data = std::sync::Arc::new(out);
        if let Ok(mut cache) = self.cache.lock() {
            if cache.len() >= CACHE_BLOCKS {
                cache.remove(0);
            }
            cache.push((index, data.clone()));
        }
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_across_blocks() {
        let mut w = StoreWriter::new();
        let records: Vec<Vec<u8>> = (0..5000u32)
            .map(|i| format!("record {i} {}", "x".repeat((i % 97) as usize)).into_bytes())
            .collect();
        for (i, r) in records.iter().enumerate() {
            assert_eq!(w.push(r.clone()), i as u32);
        }
        let bytes = w.finish();
        let r = StoreReader::new(&bytes).unwrap();
        assert_eq!(r.len(), 5000);
        assert!(r.blocks.len() > 1, "{} blocks", r.blocks.len());
        for i in [0u32, 1, 777, 2500, 4999, 3, 4998] {
            assert_eq!(r.get(&bytes, i).unwrap(), records[i as usize]);
        }
        assert!(r.get(&bytes, 5000).is_err());
    }

    #[test]
    fn empty_store() {
        let bytes = StoreWriter::new().finish();
        let r = StoreReader::new(&bytes).unwrap();
        assert_eq!(r.len(), 0);
        assert!(r.get(&bytes, 0).is_err());
    }

    #[test]
    fn damaged_stores_are_errors_not_panics() {
        let mut w = StoreWriter::new();
        for i in 0..2000u32 {
            w.push(format!("entry number {i}").into_bytes());
        }
        let bytes = w.finish();
        // Truncations and single-byte changes anywhere.
        for cut in [0, 3, 8, 20, 40, bytes.len() / 2, bytes.len() - 1] {
            if let Ok(r) = StoreReader::new(&bytes[..cut]) {
                for id in [0, 1000, 1999] {
                    let _ = r.get(&bytes[..cut], id);
                }
            }
        }
        for at in (0..bytes.len()).step_by(bytes.len() / 50 + 1) {
            let mut b = bytes.clone();
            b[at] ^= 0x5a;
            if let Ok(r) = StoreReader::new(&b) {
                for id in [0, 1000, 1999] {
                    let _ = r.get(&b, id);
                }
            }
        }
    }
}
