//! The integration tests of `textweaver-formats`, as one test program.
//!
//! Each module was a file of its own in `tests/`. Every test program
//! links the crate and its dependencies again, so they are kept as
//! one (W7t). Add a new integration test as a module in this folder,
//! never as a new file directly in `tests/`.

mod archives;
mod c3;
mod c5;
mod documents;
mod epub_mathml;
mod fixtures;
mod hostile;
mod hostile_w3d;
mod ocr;
mod ocr_missing;
mod pdf;
mod positions;
mod w6o;
