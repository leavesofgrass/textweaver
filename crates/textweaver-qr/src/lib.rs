//! QR codes for textweaver's join links.
//!
//! A join link carries a broker's address, its join code and, for a shared
//! document, its topic, so another computer or a phone can join a live
//! document by scanning it. This crate turns the link into a QR code and
//! reads one back:
//!
//! - [`encode`] makes a [`QrMatrix`], which [`QrMatrix::to_png`] and
//!   [`QrMatrix::to_svg`] write as image files;
//! - [`decode_file`], [`decode_bytes`] (a PNG or JPEG file's bytes),
//!   [`decode_rgba`] (clipboard pixels) and [`decode_image`] find every QR
//!   code in an image and return the text each one holds.
//!
//! The link is plain text here. Its type and parser live in
//! `textweaver-mqtt`; callers check what is read back with that parser,
//! which refuses anything else in words.
//!
//! A join link holds a secret. Nothing in this crate logs, and no error
//! message, `Debug` output or image metadata carries the text of a code.

mod decode;
mod encode;

pub use decode::{
    MAX_DECODE_SIDE, MAX_FILE_BYTES, decode_bytes, decode_file, decode_image, decode_rgba,
};
pub use encode::{DEFAULT_MODULE_PIXELS, MAX_MODULE_PIXELS, QUIET_ZONE, QrMatrix, encode};
/// The greyscale image the decoder reads, shared with OCR.
pub use textweaver_ocr::GrayImage;

/// Why a QR code could not be made or read. Messages are plain sentences
/// that never include the text of a code.
#[derive(Debug, thiserror::Error)]
pub enum QrError {
    /// The text to encode is empty.
    #[error("there is nothing to put in a QR code")]
    Empty,
    /// The text to encode does not fit in the largest QR code.
    #[error("the link is too long to fit in a QR code")]
    TooLong,
    /// The image could not be written (the encoder's reason).
    #[error("cannot write the QR code image: {0}")]
    Write(String),
    /// The image could not be read: not a PNG or JPEG file, damaged, or
    /// too large.
    #[error("{0}")]
    Image(String),
    /// The image file could not be read.
    #[error("cannot read the image file: {0}")]
    Io(#[from] std::io::Error),
    /// The image file is larger than [`MAX_FILE_BYTES`].
    #[error("the image file is too large to read a QR code from")]
    FileTooLarge,
    /// No QR code was found in the image.
    #[error("no QR code was found in the image")]
    NotFound,
    /// A QR code was found, but it is damaged, blurred or partly hidden,
    /// or holds something other than text.
    #[error("a QR code was found, but it could not be read; try a sharper or larger image")]
    Unreadable,
}

impl From<textweaver_ocr::OcrError> for QrError {
    fn from(e: textweaver_ocr::OcrError) -> Self {
        match e {
            textweaver_ocr::OcrError::Image(m) => QrError::Image(m),
            other => QrError::Image(other.to_string()),
        }
    }
}
