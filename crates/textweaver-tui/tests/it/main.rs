//! The integration tests of `textweaver-tui`, as one test program.
//!
//! Each module was a file of its own in `tests/`. Every test program
//! links the crate and its dependencies again, so they are kept as
//! one (W7t). Add a new integration test as a module in this folder,
//! never as a new file directly in `tests/`.

mod aids;
mod altgr;
mod announce;
mod authoring;
mod authoring_extras;
mod braille;
mod braille_first;
mod browse;
mod edit;
mod ending;
mod event_loop;
mod gaps;
mod quick_wins;
mod screen_readers;
mod scripted;
mod settings_screen;
mod terminal_polish;
mod usability;
mod wiring;
