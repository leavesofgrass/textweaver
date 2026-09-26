//! Zip packages (EPUB, DOCX): reading members safely and resolving the
//! relative, percent-encoded paths their XML uses.

use std::io::{Cursor, Read};

use zip::ZipArchive;

use crate::LoadError;

/// Largest member read from a package (uncompressed), so a zip bomb cannot
/// exhaust memory.
pub(crate) const MAX_MEMBER_BYTES: u64 = 256 * 1024 * 1024;

/// An opened zip package.
pub(crate) struct Package {
    zip: ZipArchive<Cursor<Vec<u8>>>,
    /// Lowercased member names, for case-insensitive fallback lookups.
    names: Vec<(String, String)>,
}

impl Package {
    /// Opens `bytes` as a zip archive; `what` names the format in errors.
    pub(crate) fn open(bytes: Vec<u8>, what: &str) -> Result<Self, LoadError> {
        let zip = ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| LoadError::Parse(format!("not a valid {what} (zip) file: {e}")))?;
        let names = zip
            .file_names()
            .map(|n| (n.to_lowercase(), n.to_owned()))
            .collect();
        Ok(Package { zip, names })
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
        let mut out = Vec::new();
        file.take(MAX_MEMBER_BYTES + 1)
            .read_to_end(&mut out)
            .map_err(|e| LoadError::Parse(format!("{real}: {e}")))?;
        if out.len() as u64 > MAX_MEMBER_BYTES {
            return Err(LoadError::Parse(format!("{real} is too large")));
        }
        Ok(Some(out))
    }

    /// Member `name` as text (see [`crate::decode_bytes`], honoring an XML
    /// declaration), or `None` when missing.
    pub(crate) fn read_text(&mut self, name: &str) -> Result<Option<String>, LoadError> {
        Ok(self.read(name)?.map(|b| {
            let declared = crate::encoding::sniff_html_charset(&b);
            crate::decode_bytes(&b, declared.as_deref()).text
        }))
    }
}

/// Parses XML, allowing a DOCTYPE (NCX and XHTML files carry one).
pub(crate) fn parse_xml(text: &str) -> Result<roxmltree::Document<'_>, LoadError> {
    let opts = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    roxmltree::Document::parse_with_options(text, opts)
        .map_err(|e| LoadError::Parse(format!("XML: {e}")))
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
}
