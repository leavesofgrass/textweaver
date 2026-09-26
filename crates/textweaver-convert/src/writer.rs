//! The native writers (EPUB, DOCX, BRF, PDF): Agent M's
//! `textweaver-writers`, called through its [`Writer`] trait.
//!
//! The converter and the writers share one trait, one options type, and
//! one report type, re-exported here: a writer gets a document, its
//! [`WriteOptions`], and a byte sink, and returns a [`WriteReport`] whose
//! warnings (an image that could not be found, a character braille cannot
//! show) the converter keeps with the file's result and the batch summary.
//! [`Writers::builtin`] registers all four writers; a converter built with
//! a smaller set refuses the missing formats before a batch starts.

use textweaver_writers::Format;
pub use textweaver_writers::{
    BrailleGrade, BrailleOptions, EpubOptions, PageSize, PdfOptions, WriteError, WriteOptions,
    WriteReport, Writer, writer_for,
};

use crate::OutputFormat;

impl OutputFormat {
    /// The native writer format for this output, if it has one.
    pub fn writer_format(self) -> Option<Format> {
        match self {
            OutputFormat::Epub => Some(Format::Epub),
            OutputFormat::Docx => Some(Format::Docx),
            OutputFormat::Brf => Some(Format::Brf),
            OutputFormat::Pdf => Some(Format::Pdf),
            OutputFormat::Markdown | OutputFormat::Html | OutputFormat::Text => None,
        }
    }
}

/// The writers available to a converter, one per format.
#[derive(Default)]
pub struct Writers {
    writers: Vec<Box<dyn Writer>>,
}

impl std::fmt::Debug for Writers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formats: Vec<Format> = self.writers.iter().map(|w| w.format()).collect();
        f.debug_struct("Writers")
            .field("formats", &formats)
            .finish()
    }
}

impl Writers {
    /// Every native writer: EPUB, DOCX, BRF, and PDF.
    pub fn builtin() -> Self {
        let mut w = Writers::default();
        for format in Format::ALL {
            w.register(writer_for(format));
        }
        w
    }

    /// No writers (Markdown, HTML, and text output only).
    pub fn none() -> Self {
        Writers::default()
    }

    /// Adds or replaces the writer for its format.
    pub fn register(&mut self, writer: Box<dyn Writer>) {
        let format = writer.format();
        self.writers.retain(|w| w.format() != format);
        self.writers.push(writer);
    }

    /// The writer for `format`, when it has one.
    pub fn get(&self, format: OutputFormat) -> Option<&dyn Writer> {
        let wanted = format.writer_format()?;
        self.writers
            .iter()
            .find(|w| w.format() == wanted)
            .map(|w| w.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_has_every_writer_format() {
        let w = Writers::builtin();
        for f in OutputFormat::ALL {
            assert_eq!(w.get(f).is_some(), f.needs_writer(), "{f:?}");
            if let (Some(writer), Some(format)) = (w.get(f), f.writer_format()) {
                assert_eq!(writer.format(), format);
                assert_eq!(format.extension(), f.extension());
            }
        }
        assert!(Writers::none().get(OutputFormat::Epub).is_none());
    }
}
