//! The seam to the native writers (EPUB, DOCX, BRF, PDF; Agent M's
//! `textweaver-writers`).
//!
//! The converter calls writers through [`DocumentWriter`], which has the
//! shape agreed for `textweaver_writers::Writer`: a document, options, and
//! a byte sink. Until that crate's writers exist, a [`Writers`] set starts
//! empty and formats without a writer are reported as unavailable before a
//! batch starts; at integration each writer is registered with
//! [`Writers::register`] (a one-line adapter per writer if the final trait
//! differs).

use std::io::Write;

use textweaver_text::Document;

use crate::OutputFormat;

/// Options every writer receives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteOptions {
    /// Document title (front matter, first heading, or file name).
    pub title: Option<String>,
    /// BCP 47 language tag; `en` when the source declares none.
    pub language: String,
    /// Author, when the source declares one.
    pub author: Option<String>,
    /// Braille cells per line (BRF), 40 by default.
    pub cells_per_line: u16,
    /// Braille lines per page (BRF), 25 by default.
    pub lines_per_page: u16,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions {
            title: None,
            language: "en".to_owned(),
            author: None,
            cells_per_line: 40,
            lines_per_page: 25,
        }
    }
}

/// A writer failure, as text for the batch summary.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct WriteError(pub String);

impl From<std::io::Error> for WriteError {
    fn from(e: std::io::Error) -> Self {
        WriteError(e.to_string())
    }
}

/// Writes a document in one output format.
pub trait DocumentWriter: Send + Sync {
    /// The format this writer produces.
    fn format(&self) -> OutputFormat;
    /// Writes `doc` to `out`.
    fn write(
        &self,
        doc: &Document,
        options: &WriteOptions,
        out: &mut dyn Write,
    ) -> Result<(), WriteError>;
}

/// The writers available to a converter, one per format.
#[derive(Default)]
pub struct Writers {
    writers: Vec<Box<dyn DocumentWriter>>,
}

impl std::fmt::Debug for Writers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formats: Vec<OutputFormat> = self.writers.iter().map(|w| w.format()).collect();
        f.debug_struct("Writers").field("formats", &formats).finish()
    }
}

impl Writers {
    /// The writers built into this build (none until the native writers
    /// are wired in).
    pub fn builtin() -> Self {
        Writers::default()
    }

    /// Adds or replaces the writer for its format.
    pub fn register(&mut self, writer: Box<dyn DocumentWriter>) {
        let format = writer.format();
        self.writers.retain(|w| w.format() != format);
        self.writers.push(writer);
    }

    /// The writer for `format`.
    pub fn get(&self, format: OutputFormat) -> Option<&dyn DocumentWriter> {
        self.writers
            .iter()
            .find(|w| w.format() == format)
            .map(|w| w.as_ref())
    }
}
