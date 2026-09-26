//! Stub.
use crate::{Format, WriteError, WriteOptions, WriteReport, Writer};
use std::io::Write;
use textweaver_text::Document;
/// Stub.
#[derive(Clone, Copy, Debug, Default)]
pub struct PdfWriter;
impl Writer for PdfWriter {
    fn format(&self) -> Format {
        Format::Pdf
    }
    fn write(
        &self,
        _d: &Document,
        _o: &WriteOptions,
        _out: &mut dyn Write,
    ) -> Result<WriteReport, WriteError> {
        Err(WriteError::Pdf("stub".into()))
    }
}
