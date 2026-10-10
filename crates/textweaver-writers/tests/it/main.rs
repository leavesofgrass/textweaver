//! The integration tests of `textweaver-writers`, as one test program.
//!
//! Each module was a file of its own in `tests/`. Every test program
//! links the crate and its dependencies again, so they are kept as
//! one (W7t). Add a new integration test as a module in this folder,
//! never as a new file directly in `tests/`.

mod brf_formats;
mod common;
mod docx;
mod docx_math;
mod docx_update;
mod epub;
mod lists;
mod math;
mod math_braille;
mod nested_lists;
mod pdf;
mod pdf_options;
mod templates;
