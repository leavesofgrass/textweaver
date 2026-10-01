//! The integration tests of `textweaver-convert`, as one test program.
//!
//! Each module was a file of its own in `tests/`. Every test program
//! links the crate and its dependencies again, so they are kept as
//! one (W7t). Add a new integration test as a module in this folder,
//! never as a new file directly in `tests/`.

mod bulk;
mod citations;
mod matrix;
mod pandoc_env;
mod progress;
mod writers;
