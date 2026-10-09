//! The integration tests of `textweaver-app`, as one test program.
//!
//! Each module was a file of its own in `tests/`. Every test program
//! links the crate and its dependencies again, so they are kept as
//! one (W7t). Add a new integration test as a module in this folder,
//! never as a new file directly in `tests/`.

mod about;
mod access;
mod aids;
mod app;
mod app_core;
// Export, preview, and citations are tested with the feature on.
#[cfg(feature = "publish")]
mod authoring;
mod authoring_extras;
mod batch;
mod browse;
#[cfg(feature = "publish")]
mod citations_reading;
mod command_list;
mod components_registry;
mod details;
mod edit;
mod fonts;
mod idea_cards;
mod keys_from_keymap;
mod language;
mod list_contract;
mod markup_pauses;
mod path_choosers;
mod pseudo_locale;
mod quick_wins;
mod reading_generation;
mod recovery;
mod reliability;
mod rpc;
mod settings_reference;
mod settings_update;
mod startup_speech;
mod sync;
mod sync_ids;
mod sync_library;
mod sync_settings;
mod tests_ask_the_keymap;
mod wiring;
