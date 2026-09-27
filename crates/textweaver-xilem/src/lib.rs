//! textweaver's GUI on Masonry, Xilem's widget layer (ADR-0027).
//!
//! The stack is all Rust: Masonry widgets, Vello rendering, Parley text
//! layout, AccessKit accessibility, and winit windows. The app core
//! (`textweaver-app`) does the work; this crate draws it and exposes it to
//! screen readers.
//!
//! - [`gui`]: the window's widget tree and the driver.
//! - [`document`]: the `DocumentView` widget.
//! - [`widgets`]: the root, panels, buttons, and the live-region announcer.
//! - [`dialog`]: in-window dialogs: prompts and lists.
//! - [`settings_dialog`]: the settings dialog, built from the app's schema.
//! - [`runs`]: the document as AccessKit text runs, with stable ids.
//! - [`window`]: the part of a large document the view holds.
//! - [`caret`]: caret moves that need no layout.
//! - [`keys`]: key events as keymap chords.
//! - [`theme`]: textweaver's themes on Masonry's widgets.
//! - [`fonts`]: the bundled fonts and the reader's font setting.
//! - [`font_chooser`]: the font chooser's lists, ported from the spike.
//! - [`setup`]: building the app for the GUI.
//! - `screenshot` (feature `screenshot`): the window drawn to a PNG.
//!
//! Owner: Agent W3b.

pub mod caret;
pub mod dialog;
pub mod document;
pub mod font_chooser;
pub mod fonts;
pub mod gui;
pub mod keys;
pub mod log;
pub mod runs;
#[cfg(feature = "screenshot")]
pub mod screenshot;
pub mod settings_dialog;
pub mod setup;
pub mod theme;
pub mod widgets;
pub mod window;
