//! Plain text loader.
//!
//! Plain text keeps its lines (ADR-0002): unlike Star, which ran `.txt`
//! files through its Markdown stripper and joined every single line break
//! into a space, a line starting with `#` is not a heading and line
//! structure survives for line navigation. Sentences still flow across the
//! line breaks of a wrapped paragraph (see `textweaver_text::units`).

use textweaver_text::Document;

use crate::{
    LoadError, LoadOptions, Loader, Source, decode_source, meta_for, note_encoding, title_from_path,
};

/// Loads text in UTF-8, UTF-16 (with a BOM), or Windows-1252 (see
/// [`crate::encoding`]); a leading BOM is dropped and line endings become
/// `\n`. The title is the file name.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextLoader;

impl Loader for TextLoader {
    fn id(&self) -> &'static str {
        "text"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txt", "text", "log"]
    }

    fn load(&self, source: &Source, _options: &LoadOptions) -> Result<Document, LoadError> {
        let decoded = decode_source(source, None)?;
        let mut doc = Document::from_plain_text(&decoded.text);
        doc.meta = meta_for(source, self.id());
        doc.meta.title = title_from_path(source);
        note_encoding(&mut doc.meta, &decoded);
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Registry;

    #[test]
    fn loads_bytes_and_strips_bom() {
        let src = Source::Bytes {
            data: "\u{feff}hi\r\nthere".into(),
            hint: "txt".into(),
        };
        let doc = Registry::with_builtins()
            .load(&src, &LoadOptions::default())
            .unwrap();
        assert_eq!(doc.text().to_string(), "hi\nthere");
        assert_eq!(doc.meta.format, "text");
    }

    #[test]
    fn non_utf8_text_is_windows_1252() {
        let src = Source::Bytes {
            data: b"caf\xe9 \x93quoted\x94".to_vec(),
            hint: "txt".into(),
        };
        let doc = TextLoader.load(&src, &LoadOptions::default()).unwrap();
        assert_eq!(doc.text().to_string(), "café \u{201c}quoted\u{201d}");
        assert_eq!(
            doc.meta.properties.get("encoding").map(String::as_str),
            Some("windows-1252")
        );
        let utf16 = Source::Bytes {
            data: b"\xfe\xff\x00h\x00i".to_vec(),
            hint: "txt".into(),
        };
        let doc = TextLoader.load(&utf16, &LoadOptions::default()).unwrap();
        assert_eq!(doc.text().to_string(), "hi");
    }
}
