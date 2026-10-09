//! The integration tests of `textweaver-espeak`, as one test program.
//!
//! Add a new integration test as a module in this folder, never as a new
//! file directly in `tests/` (each test program links the crate again).

mod fake_host;
mod real_engine;
