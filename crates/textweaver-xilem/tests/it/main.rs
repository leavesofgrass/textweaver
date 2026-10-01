//! The integration tests of `textweaver-xilem`, as one test program.
//!
//! Each module was a file of its own in `tests/`. Every test program
//! links the crate and its dependencies again, so they are kept as
//! one (W7t). Add a new integration test as a module in this folder,
//! never as a new file directly in `tests/`.

mod announcements;
mod colors_dialog;
mod document_view;
mod edit_mode;
mod settings_dialog;
mod voice_manager;
mod window_tree;
