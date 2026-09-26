//! Just enough of the OpenType file format to name a font: the table
//! directory (and collection header), `name` (family and style names),
//! `OS/2` (weight, italic, embedding permission), `head` (the older italic
//! and bold bits), and `post` (fixed pitch).
//!
//! Reads go through [`Source`], so a font file on disk is read a few
//! hundred bytes at a time rather than whole (a CJK font can be 20 MB).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// What a face says about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceInfo {
    /// The typographic family (`name` ID 16), else the family (ID 1).
    pub family: String,
    /// The family as older systems group it (ID 1): for example "Segoe UI
    /// Semibold", where [`FaceInfo::family`] is "Segoe UI". Windows' GDI,
    /// and so wxWidgets on Windows, names fonts this way.
    pub legacy_family: String,
    /// The style name (ID 17, else ID 2): "Bold Italic".
    pub subfamily: String,
    /// Weight class, 100 to 900 (400 regular, 700 bold).
    pub weight: u16,
    /// An italic or oblique face.
    pub italic: bool,
    /// Every glyph the same width (`post` `isFixedPitch`).
    pub monospace: bool,
    /// The licence allows embedding in documents (`OS/2` `fsType` is not
    /// "restricted license").
    pub embeddable: bool,
}

/// Random access to font bytes.
pub trait Source {
    /// `len` bytes at `offset`, or `None` past the end or on error.
    fn read_at(&mut self, offset: u64, len: usize) -> Option<Vec<u8>>;
}

impl Source for &[u8] {
    fn read_at(&mut self, offset: u64, len: usize) -> Option<Vec<u8>> {
        let start = usize::try_from(offset).ok()?;
        self.get(start..start.checked_add(len)?).map(<[u8]>::to_vec)
    }
}

impl Source for File {
    fn read_at(&mut self, offset: u64, len: usize) -> Option<Vec<u8>> {
        self.seek(SeekFrom::Start(offset)).ok()?;
        let mut buf = vec![0; len];
        self.read_exact(&mut buf).ok()?;
        Some(buf)
    }
}

fn u16_at(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?))
}

/// Number of faces in a font file: the collection's count for `ttcf`
/// files, 1 for a single font, 0 when the data is not a font.
pub fn face_count(src: &mut impl Source) -> u32 {
    let Some(head) = src.read_at(0, 12) else {
        return 0;
    };
    match head.get(0..4) {
        Some(b"ttcf") => u32_at(&head, 8).unwrap_or(0).min(256),
        Some([0, 1, 0, 0]) | Some(b"OTTO") | Some(b"true") => 1,
        _ => 0,
    }
}

/// Offsets and lengths of the tables of face `index`.
fn table_dir(src: &mut impl Source, index: u32) -> Option<Vec<([u8; 4], u32, u32)>> {
    let head = src.read_at(0, 12)?;
    let base = if head.get(0..4)? == b"ttcf" {
        let at = 12 + 4 * u64::from(index);
        u32_at(&src.read_at(at, 4)?, 0)?
    } else if index == 0 {
        0
    } else {
        return None;
    };
    let hdr = src.read_at(u64::from(base), 12)?;
    let n = usize::from(u16_at(&hdr, 4)?).min(512);
    let recs = src.read_at(u64::from(base) + 12, 16 * n)?;
    (0..n)
        .map(|k| {
            let r = &recs[16 * k..16 * k + 16];
            Some((r[0..4].try_into().ok()?, u32_at(r, 8)?, u32_at(r, 12)?))
        })
        .collect()
}

fn table(src: &mut impl Source, dir: &[([u8; 4], u32, u32)], tag: &[u8; 4]) -> Option<Vec<u8>> {
    let &(_, off, len) = dir.iter().find(|(t, _, _)| t == tag)?;
    // Names never need more than this; it bounds a hostile length.
    src.read_at(u64::from(off), (len as usize).min(1 << 20))
}

/// Decodes a `name` record: UTF-16BE for Unicode and Windows platforms,
/// Latin-1 as an approximation of Mac Roman.
fn decode(platform: u16, raw: &[u8]) -> String {
    if platform == 1 {
        return raw.iter().map(|&b| char::from(b)).collect();
    }
    let units: Vec<u16> = raw
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// The best string for name ID `id`: Windows English (US), then any
/// Windows or Unicode language, then Macintosh.
fn name(table: &[u8], id: u16) -> Option<String> {
    let count = usize::from(u16_at(table, 2)?);
    let strings = usize::from(u16_at(table, 4)?);
    let mut best: Option<(u8, String)> = None;
    for k in 0..count {
        let r = 6 + 12 * k;
        let (platform, encoding, lang, nid) = (
            u16_at(table, r)?,
            u16_at(table, r + 2)?,
            u16_at(table, r + 4)?,
            u16_at(table, r + 6)?,
        );
        if nid != id {
            continue;
        }
        let rank = match (platform, encoding, lang) {
            (3, 1 | 10, 0x0409) => 4,
            (3, 1 | 10, _) => 3,
            (0, _, _) => 2,
            (1, 0, 0) => 1,
            _ => 0,
        };
        if rank == 0 || best.as_ref().is_some_and(|b| b.0 >= rank) {
            continue;
        }
        let len = usize::from(u16_at(table, r + 8)?);
        let off = strings + usize::from(u16_at(table, r + 10)?);
        let Some(raw) = table.get(off..off + len) else {
            continue;
        };
        let s = decode(platform, raw).trim().to_owned();
        if !s.is_empty() {
            best = Some((rank, s));
        }
    }
    best.map(|b| b.1)
}

/// Reads face `index` of a font. `None` when it is not a font or has no
/// family name.
pub fn info_from(src: &mut impl Source, index: u32) -> Option<FaceInfo> {
    let dir = table_dir(src, index)?;
    let names = table(src, &dir, b"name")?;
    let legacy_family = name(&names, 1)?;
    let family = name(&names, 16).unwrap_or_else(|| legacy_family.clone());
    let subfamily = name(&names, 17)
        .or_else(|| name(&names, 2))
        .unwrap_or_else(|| "Regular".to_owned());
    let os2 = table(src, &dir, b"OS/2");
    let head = table(src, &dir, b"head");
    let post = table(src, &dir, b"post");
    let mac_style = head.as_deref().and_then(|h| u16_at(h, 44)).unwrap_or(0);
    let (weight, fs_type, fs_selection) = match os2.as_deref() {
        Some(o) => (
            u16_at(o, 4).unwrap_or(400),
            u16_at(o, 8).unwrap_or(0),
            u16_at(o, 62).unwrap_or(0),
        ),
        None => (if mac_style & 1 != 0 { 700 } else { 400 }, 0, 0),
    };
    // Some old fonts store weight classes 1 to 9.
    let weight = match weight {
        1..=9 => weight * 100,
        0 => 400,
        w => w.min(1000),
    };
    let italic = fs_selection & 0x0201 != 0 || mac_style & 2 != 0;
    let monospace = post
        .as_deref()
        .and_then(|p| u32_at(p, 12))
        .is_some_and(|v| v != 0);
    Some(FaceInfo {
        family,
        legacy_family,
        subfamily,
        weight,
        italic,
        monospace,
        embeddable: fs_type & 0x000F != 0x0002,
    })
}

/// Reads face `index` of a font held in memory.
pub fn info(data: &[u8], index: u32) -> Option<FaceInfo> {
    let mut src = data;
    info_from(&mut src, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal font: a table directory with `name` (IDs 1, 2, 16 in
    /// Windows UTF-16), `OS/2` (weight 700, italic), and `post` (fixed
    /// pitch).
    fn tiny_font() -> Vec<u8> {
        fn utf16(s: &str) -> Vec<u8> {
            s.encode_utf16().flat_map(u16::to_be_bytes).collect()
        }
        let strings = [(1u16, "Tiny Legacy"), (2, "Bold Italic"), (16, "Tiny")];
        let mut name = Vec::new();
        name.extend(0u16.to_be_bytes());
        name.extend((strings.len() as u16).to_be_bytes());
        name.extend((6 + 12 * strings.len() as u16).to_be_bytes());
        let mut pool = Vec::new();
        for (id, s) in strings {
            let bytes = utf16(s);
            for v in [3u16, 1, 0x0409, id, bytes.len() as u16, pool.len() as u16] {
                name.extend(v.to_be_bytes());
            }
            pool.extend(bytes);
        }
        name.extend(pool);
        let mut os2 = vec![0u8; 78];
        os2[4..6].copy_from_slice(&700u16.to_be_bytes());
        os2[62..64].copy_from_slice(&1u16.to_be_bytes());
        let mut post = vec![0u8; 32];
        post[12..16].copy_from_slice(&1u32.to_be_bytes());
        let tables: [(&[u8; 4], Vec<u8>); 3] = [(b"OS/2", os2), (b"name", name), (b"post", post)];
        let mut out = vec![0, 1, 0, 0];
        out.extend(3u16.to_be_bytes());
        out.extend([0u8; 6]);
        let mut off = 12 + 16 * tables.len();
        let mut body: Vec<u8> = Vec::new();
        for (tag, data) in &tables {
            out.extend_from_slice(*tag);
            out.extend(0u32.to_be_bytes());
            out.extend((off as u32).to_be_bytes());
            out.extend((data.len() as u32).to_be_bytes());
            off += data.len();
            body.extend(data);
        }
        out.extend(body);
        out
    }

    #[test]
    fn reads_names_and_style() {
        let data = tiny_font();
        let mut src = data.as_slice();
        assert_eq!(face_count(&mut src), 1);
        let i = info(&data, 0).unwrap();
        assert_eq!(i.family, "Tiny");
        assert_eq!(i.legacy_family, "Tiny Legacy");
        assert_eq!(i.subfamily, "Bold Italic");
        assert_eq!(i.weight, 700);
        assert!(i.italic && i.monospace && i.embeddable);
        assert!(info(&data, 1).is_none());
    }

    #[test]
    fn reads_from_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tiny.ttf");
        std::fs::write(&path, tiny_font()).unwrap();
        let mut f = File::open(&path).unwrap();
        assert_eq!(info_from(&mut f, 0).unwrap().family, "Tiny");
    }

    #[test]
    fn garbage_is_not_a_font() {
        for bad in [&b""[..], b"not a font at all", &[0, 1, 0, 0, 255, 255][..]] {
            let mut src = bad;
            assert!(info_from(&mut src, 0).is_none());
        }
        let mut truncated = &tiny_font()[..40];
        assert!(info_from(&mut truncated, 0).is_none());
    }
}
