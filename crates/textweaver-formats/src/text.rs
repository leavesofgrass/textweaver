//! Plain text loader.

use textweaver_text::Document;

use crate::{LoadError, LoadOptions, Loader, Source, meta_for};

/// Loads UTF-8 text (lossy for invalid bytes); a leading BOM is dropped.
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
        let bytes = source.read()?;
        let text = String::from_utf8_lossy(&bytes);
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let mut doc = Document::from_plain_text(text);
        doc.meta = meta_for(source, self.id());
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
}
