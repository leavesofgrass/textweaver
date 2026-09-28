//! Zip packages (EPUB, DOCX): reading members safely and resolving the
//! relative, percent-encoded paths their XML uses.

use std::io::{Cursor, Read};

use zip::ZipArchive;

use crate::LoadError;

/// Largest member read from a package (uncompressed), so a zip bomb cannot
/// exhaust memory.
pub(crate) const MAX_MEMBER_BYTES: u64 = 256 * 1024 * 1024;

/// Most members a package may have. Word, OpenDocument, and PowerPoint
/// files have tens; a large EPUB a few thousand.
pub(crate) const MAX_ENTRIES: usize = 50_000;

/// Most bytes read, uncompressed, from all the members of one package.
pub(crate) const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024;

/// Highest compression ratio accepted for a member larger than
/// [`RATIO_FLOOR`]. XML compresses about ten to fifty times; deflate cannot
/// pass about 1,030, but bzip2 can, so a member claiming more is a bomb.
pub(crate) const MAX_RATIO: u64 = 1_000;

/// Members smaller than this (1 MiB) are not held to [`MAX_RATIO`].
pub(crate) const RATIO_FLOOR: u64 = 1024 * 1024;

/// An opened zip package.
pub(crate) struct Package {
    zip: ZipArchive<Cursor<Vec<u8>>>,
    /// Lowercased member names, for case-insensitive fallback lookups.
    names: Vec<(String, String)>,
    /// Set when an XML member was nested too deeply and was flattened.
    flattened: bool,
    /// Uncompressed bytes still allowed ([`MAX_PACKAGE_BYTES`]).
    left: u64,
}

impl Package {
    /// Opens `bytes` as a zip archive; `what` names the format in errors.
    /// Archives with more than [`MAX_ENTRIES`] members, or whose members
    /// overlap (the same compressed bytes read as many files, a way to
    /// build a zip bomb), are refused.
    pub(crate) fn open(bytes: Vec<u8>, what: &str) -> Result<Self, LoadError> {
        let mut zip = ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| LoadError::Parse(format!("not a valid {what} (zip) file: {e}")))?;
        if zip.len() > MAX_ENTRIES {
            return Err(LoadError::Parse(format!(
                "this {what} file holds more than {MAX_ENTRIES} files, too many for a document"
            )));
        }
        let mut extents = Vec::with_capacity(zip.len());
        for i in 0..zip.len() {
            let f = zip
                .by_index_raw(i)
                .map_err(|e| LoadError::Parse(format!("not a valid {what} (zip) file: {e}")))?;
            extents.push((f.header_start(), f.compressed_size()));
        }
        extents.sort_unstable();
        if extents
            .windows(2)
            .any(|w| w[0].0.saturating_add(w[0].1) > w[1].0)
        {
            return Err(LoadError::Parse(format!(
                "this {what} file is damaged or hostile: its members overlap"
            )));
        }
        let names = zip
            .file_names()
            .map(|n| (n.to_lowercase(), n.to_owned()))
            .collect();
        Ok(Package {
            zip,
            names,
            flattened: false,
            left: MAX_PACKAGE_BYTES,
        })
    }

    fn real_name(&self, name: &str) -> Option<String> {
        let name = name.trim_start_matches('/');
        let lower = name.to_lowercase();
        self.names
            .iter()
            .find(|(l, n)| n == name || *l == lower)
            .map(|(_, n)| n.clone())
    }

    /// The bytes of member `name` (exact name first, then any case), or
    /// `None` when there is no such member.
    pub(crate) fn read(&mut self, name: &str) -> Result<Option<Vec<u8>>, LoadError> {
        let Some(real) = self.real_name(name) else {
            return Ok(None);
        };
        let file = self
            .zip
            .by_name(&real)
            .map_err(|e| LoadError::Parse(format!("{real}: {e}")))?;
        // Refused before decompressing: LZMA sizes its dictionary from the
        // declared size, so a huge claim would cost that memory.
        if file.size() > MAX_MEMBER_BYTES {
            return Err(LoadError::Parse(format!("{real} is too large")));
        }
        if file.size() > RATIO_FLOOR && file.size() / file.compressed_size().max(1) > MAX_RATIO {
            return Err(LoadError::Parse(format!(
                "{real} claims to unpack to more than {MAX_RATIO} times its size, so it is not read"
            )));
        }
        let cap = MAX_MEMBER_BYTES.min(self.left);
        let mut out = Vec::new();
        file.take(cap + 1)
            .read_to_end(&mut out)
            .map_err(|e| LoadError::Parse(format!("{real}: {e}")))?;
        if out.len() as u64 > cap {
            return Err(LoadError::Parse(if cap < MAX_MEMBER_BYTES {
                "the file unpacks to more than a document can hold".to_owned()
            } else {
                format!("{real} is too large")
            }));
        }
        self.left -= out.len() as u64;
        Ok(Some(out))
    }

    /// Member `name` as text (see [`crate::decode_bytes`], honoring an XML
    /// declaration), or `None` when missing.
    ///
    /// XML members (`.xml`, `.rels`, `.opf`, `.ncx`) nested deeper than
    /// [`MAX_NESTING`](crate::MAX_NESTING) are flattened below that depth
    /// (see `xmldepth`), because the XML parser recurses per element.
    pub(crate) fn read_text(&mut self, name: &str) -> Result<Option<String>, LoadError> {
        let Some(bytes) = self.read(name)? else {
            return Ok(None);
        };
        let declared = crate::encoding::sniff_html_charset(&bytes);
        let text = crate::decode_bytes(&bytes, declared.as_deref()).text;
        let lower = name.to_ascii_lowercase();
        let is_xml = [".xml", ".rels", ".opf", ".ncx"]
            .iter()
            .any(|e| lower.ends_with(e));
        if is_xml && let Some(flat) = crate::xmldepth::limit_depth(&text, crate::MAX_NESTING) {
            self.flattened = true;
            return Ok(Some(flat));
        }
        Ok(Some(text))
    }

    /// True when some XML member had to be flattened (the loader then
    /// adds [`NESTING_WARNING`](crate::NESTING_WARNING)).
    pub(crate) fn flattened(&self) -> bool {
        self.flattened
    }
}

/// Most nodes (elements, attributes' owners, text) one XML part may have:
/// about a gigabyte of parsed tree. A 1,000-page Word document has a few
/// million.
pub(crate) const MAX_XML_NODES: u32 = 16_000_000;

/// Parses XML, allowing a DOCTYPE (NCX and XHTML files carry one), with at
/// most [`MAX_XML_NODES`] nodes. roxmltree expands no external entities.
pub(crate) fn parse_xml(text: &str) -> Result<roxmltree::Document<'_>, LoadError> {
    let opts = roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: MAX_XML_NODES,
        ..roxmltree::ParsingOptions::default()
    };
    roxmltree::Document::parse_with_options(text, opts).map_err(|e| match e {
        roxmltree::Error::NodesLimitReached => {
            LoadError::Parse("the document has too many parts to read".into())
        }
        e => LoadError::Parse(format!("XML: {e}")),
    })
}

/// The first descendant element of `node` with local name `name`.
pub(crate) fn child<'a, 'i>(
    node: roxmltree::Node<'a, 'i>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'i>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

/// Decodes `%XX` escapes (UTF-8); malformed escapes are kept as written.
pub(crate) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// The directory part of a member path, with a trailing slash (or empty).
pub(crate) fn dir_of(path: &str) -> &str {
    path.rfind('/').map_or("", |i| &path[..=i])
}

/// Resolves `href` (relative to directory `base`, percent-encoded, maybe
/// with a `#fragment`) to a normalized member path and the fragment.
pub(crate) fn resolve(base: &str, href: &str) -> (String, Option<String>) {
    let (path, frag) = match href.split_once('#') {
        Some((p, f)) => (p, Some(percent_decode(f)).filter(|f| !f.is_empty())),
        None => (href, None),
    };
    let path = percent_decode(path);
    let joined = if path.starts_with('/') {
        path.trim_start_matches('/').to_owned()
    } else {
        format!("{base}{path}")
    };
    let mut parts: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    (parts.join("/"), frag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_encoded_paths() {
        assert_eq!(
            resolve("OEBPS/text/", "../ch%201.xhtml#sec%202"),
            ("OEBPS/ch 1.xhtml".to_owned(), Some("sec 2".to_owned()))
        );
        assert_eq!(resolve("", "a.xhtml#"), ("a.xhtml".to_owned(), None));
        assert_eq!(resolve("x/", "/root.xhtml").0, "root.xhtml");
        assert_eq!(dir_of("OEBPS/content.opf"), "OEBPS/");
        assert_eq!(dir_of("content.opf"), "");
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("caf%C3%A9"), "café");
    }

    use std::io::Write;

    fn zip_of(entries: &[(&str, &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default().compression_method(method);
        for (name, data) in entries {
            z.start_file(*name, opts).expect("zip entry");
            z.write_all(data).expect("zip write");
        }
        z.finish().expect("zip finish").into_inner()
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn too_many_members_are_refused() {
        let names: Vec<String> = (0..=MAX_ENTRIES).map(|i| format!("{i}")).collect();
        let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b""[..])).collect();
        let bytes = zip_of(&entries, zip::CompressionMethod::Stored);
        let err = Package::open(bytes, "DOCX").err().map(|e| e.to_string());
        assert!(
            err.as_deref().is_some_and(|e| e.contains("too many")),
            "{err:?}"
        );
    }

    #[test]
    fn a_bomb_member_is_refused_before_it_is_read() {
        let zeros = vec![0u8; 4 * 1024 * 1024];
        let bytes = zip_of(
            &[("word/document.xml", &zeros), ("small.xml", b"<a/>")],
            zip::CompressionMethod::Deflated,
        );
        let mut pkg = Package::open(bytes, "DOCX").expect("the package opens");
        let err = pkg.read("word/document.xml").err().map(|e| e.to_string());
        assert!(
            err.as_deref().is_some_and(|e| e.contains("times its size")),
            "{err:?}"
        );
        assert_eq!(
            pkg.read("small.xml").ok().flatten().as_deref(),
            Some(&b"<a/>"[..])
        );
    }

    #[test]
    fn overlapping_members_are_refused() {
        // Two central directory entries naming the same stored bytes.
        let data = b"<a/>";
        let crc = crc32(data);
        let size = u32::try_from(data.len()).unwrap_or(0);
        let mut z = Vec::new();
        let local = |z: &mut Vec<u8>, name: &[u8]| {
            z.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            z.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            z.extend_from_slice(&crc.to_le_bytes());
            z.extend_from_slice(&size.to_le_bytes());
            z.extend_from_slice(&size.to_le_bytes());
            z.extend_from_slice(&u16::try_from(name.len()).unwrap_or(0).to_le_bytes());
            z.extend_from_slice(&[0, 0]);
            z.extend_from_slice(name);
        };
        local(&mut z, b"a.xml");
        z.extend_from_slice(data);
        let cd_start = u32::try_from(z.len()).unwrap_or(0);
        for name in [&b"a.xml"[..], &b"b.xml"[..]] {
            z.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            z.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            z.extend_from_slice(&crc.to_le_bytes());
            z.extend_from_slice(&size.to_le_bytes());
            z.extend_from_slice(&size.to_le_bytes());
            z.extend_from_slice(&u16::try_from(name.len()).unwrap_or(0).to_le_bytes());
            z.extend_from_slice(&[0; 12]);
            z.extend_from_slice(&0u32.to_le_bytes());
            z.extend_from_slice(name);
        }
        let cd_size = u32::try_from(z.len()).unwrap_or(0) - cd_start;
        z.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        z.extend_from_slice(&[0, 0, 0, 0, 2, 0, 2, 0]);
        z.extend_from_slice(&cd_size.to_le_bytes());
        z.extend_from_slice(&cd_start.to_le_bytes());
        z.extend_from_slice(&[0, 0]);
        let err = Package::open(z, "DOCX").err().map(|e| e.to_string());
        assert!(
            err.as_deref().is_some_and(|e| e.contains("overlap")),
            "{err:?}"
        );
    }
}
