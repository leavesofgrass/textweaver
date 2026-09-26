//! Plain text loader.
//!
//! Plain text keeps its lines (ADR-0002): unlike Star, which ran `.txt`
//! files through its Markdown stripper and joined every single line break
//! into a space, a line starting with `#` is not a heading and line
//! structure survives for line navigation. Sentences still flow across the
//! line breaks of a wrapped paragraph (see `textweaver_text::units`).

use textweaver_text::Document;

use crate::{LoadError, LoadOptions, Loader, Source, meta_for, source_text, title_from_path};

/// Loads UTF-8 text (lossy for invalid bytes); a leading BOM is dropped and
/// line endings become `\n`. The title is the file name.
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
        let text = source_text(source)?;
        let mut doc = Document::from_plain_text(&text);
        doc.meta = meta_for(source, self.id());
        doc.meta.title = title_from_path(source);
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
    fn invalid_utf8_is_replaced_not_an_error() {
        let src = Source::Bytes {
            data: vec![b'a', 0xff, b'b'],
            hint: "txt".into(),
        };
        let doc = TextLoader.load(&src, &LoadOptions::default()).unwrap();
        assert_eq!(doc.text().to_string(), "a\u{fffd}b");
    }
}
