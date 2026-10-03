//! textweaver's GUI on Masonry, Xilem's widget layer (ADR-0027).
//!
//! The stack is all Rust: Masonry widgets, Vello rendering, Parley text
//! layout, AccessKit accessibility, and winit windows. The app core
//! (`textweaver-app`) does the work; this crate draws it and exposes it to
//! screen readers.
//!
//! - [`gui`]: the window's widget tree and the driver.
//! - [`document`]: the `DocumentView` widget, with the reading aids it
//!   draws (text spacing, the ruler, bionic reading, difficult words).
//! - [`rsvp`]: the RSVP panel, one word at a time under the document.
//! - [`sidebar`]: the Contents and Notes panels beside the document, and
//!   F6 between the window's regions.
//! - [`widgets`]: the root, panels, buttons, and the live-region announcer.
//! - [`dialog`]: in-window dialogs: prompts and lists.
//! - [`menus`]: the menus, native on Windows and macOS, from the app's model.
//! - [`file_chooser`]: Open with the system's own file chooser.
//! - [`settings_dialog`]: the settings dialog, built from the app's schema.
//! - [`voices`]: the voice manager, every engine's voices.
//! - [`runs`]: the document as AccessKit text runs, with stable ids.
//! - [`window`]: the part of a large document the view holds.
//! - [`caret`]: caret moves that need no layout.
//! - [`keys`]: key events as keymap chords, and each platform's caret keys.
//! - [`parity`]: where each keymap action is handled in the window.
//! - [`theme`]: textweaver's themes on Masonry's widgets.
//! - [`fonts`]: the bundled fonts and the reader's font setting.
//! - [`font_chooser`]: the font chooser's lists, ported from the spike.
//! - [`setup`]: building the app for the GUI.
//! - [`system_colors`]: the system's colors in a high contrast mode.
//! - [`dark_mode`]: the title bar and menus follow the theme.
//! - [`background`]: `--background` windows that never take the
//!   foreground (Windows).
//! - [`console`]: the terminal the program was started from, on Windows.
//! - [`graphics`]: which graphics API the window draws with (opt-in).
//! - `screenshot` (feature `screenshot`): the window drawn to a PNG.
//! - `frames` (feature `screenshot`): the frame-time probe,
//!   `--measure-frames`.
//!
//! Owner: Agent W3b.

pub mod background;
pub mod caret;
pub mod console;
pub mod dark_mode;
pub mod dialog;
pub mod document;
pub mod file_chooser;
pub mod font_chooser;
pub mod fonts;
#[cfg(feature = "screenshot")]
pub mod frames;
pub mod graphics;
pub mod gui;
pub mod keys;
pub mod log;
pub mod menus;
pub mod parity;
pub mod rsvp;
pub mod runs;
#[cfg(feature = "screenshot")]
pub mod screenshot;
pub mod settings_dialog;
pub mod setup;
pub mod sidebar;
pub mod system_colors;
pub mod theme;
pub mod voices;
pub mod widgets;
pub mod window;
