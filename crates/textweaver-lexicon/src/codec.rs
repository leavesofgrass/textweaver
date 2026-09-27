//! Byte-level encoding for the lexicon file: variable-length integers and
//! length-prefixed strings, read with bounds checks so a damaged file gives
//! an error instead of a panic.

use crate::LexiconError;

/// Appends `v` as a LEB128 variable-length integer.
pub(crate) fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Appends a length-prefixed UTF-8 string.
pub(crate) fn put_str(out: &mut Vec<u8>, s: &str) {
    put_varint(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

/// Appends a count and then each string.
pub(crate) fn put_strs(out: &mut Vec<u8>, items: &[String]) {
    put_varint(out, items.len() as u64);
    for s in items {
        put_str(out, s);
    }
}

/// A cursor over a byte slice.
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, at: 0 }
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.at >= self.bytes.len()
    }

    pub(crate) fn u8(&mut self) -> Result<u8, LexiconError> {
        let b = *self
            .bytes
            .get(self.at)
            .ok_or(LexiconError::Corrupt("record ends early"))?;
        self.at += 1;
        Ok(b)
    }

    pub(crate) fn varint(&mut self) -> Result<u64, LexiconError> {
        let mut v: u64 = 0;
        for shift in (0..64).step_by(7) {
            let b = self.u8()?;
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(LexiconError::Corrupt("number too long"))
    }

    /// A count, limited to what the rest of the record could hold, so a
    /// damaged count cannot ask for a huge allocation.
    pub(crate) fn count(&mut self) -> Result<usize, LexiconError> {
        let n = self.varint()?;
        let left = (self.bytes.len() - self.at.min(self.bytes.len())) as u64;
        if n > left {
            return Err(LexiconError::Corrupt("count larger than the record"));
        }
        Ok(n as usize)
    }

    pub(crate) fn str(&mut self) -> Result<&'a str, LexiconError> {
        let n = self.count()?;
        let end = self.at + n;
        let s = self
            .bytes
            .get(self.at..end)
            .ok_or(LexiconError::Corrupt("text ends early"))?;
        self.at = end;
        std::str::from_utf8(s).map_err(|_| LexiconError::Corrupt("text is not UTF-8"))
    }

    pub(crate) fn strs(&mut self) -> Result<Vec<String>, LexiconError> {
        let n = self.count()?;
        (0..n).map(|_| self.str().map(str::to_owned)).collect()
    }

    pub(crate) fn u32_le(&mut self) -> Result<u32, LexiconError> {
        let s = self
            .bytes
            .get(self.at..self.at + 4)
            .ok_or(LexiconError::Corrupt("file ends early"))?;
        self.at += 4;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    pub(crate) fn u64_le(&mut self) -> Result<u64, LexiconError> {
        let lo = u64::from(self.u32_le()?);
        let hi = u64::from(self.u32_le()?);
        Ok(lo | (hi << 32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints_and_strings_round_trip() {
        let mut out = Vec::new();
        for v in [0u64, 1, 127, 128, 300, u64::from(u32::MAX), u64::MAX] {
            put_varint(&mut out, v);
        }
        put_str(&mut out, "déjà vu");
        put_strs(&mut out, &["a".into(), "bc".into()]);
        let mut r = Reader::new(&out);
        for v in [0u64, 1, 127, 128, 300, u64::from(u32::MAX), u64::MAX] {
            assert_eq!(r.varint().unwrap(), v);
        }
        assert_eq!(r.str().unwrap(), "déjà vu");
        assert_eq!(r.strs().unwrap(), vec!["a".to_owned(), "bc".to_owned()]);
        assert!(r.is_empty());
    }

    #[test]
    fn damaged_input_is_an_error() {
        // A string claiming 200 bytes in a 3-byte record.
        let mut r = Reader::new(&[200, 1, b'a']);
        assert!(r.str().is_err());
        let mut r = Reader::new(&[0xff; 12]);
        assert!(r.varint().is_err());
        let mut r = Reader::new(&[]);
        assert!(r.u8().is_err());
        assert!(Reader::new(&[1, 2]).u32_le().is_err());
    }
}
