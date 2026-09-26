//! textweaver's GUI on Masonry, Xilem's widget layer (ADR-0023).
//!
//! The stack is all Rust: Masonry widgets, Vello rendering, Parley text
//! layout, AccessKit accessibility, and winit windows. The app core
//! (`textweaver-app`) does the work; this crate draws it and exposes it to
//! screen readers.
//!
//! - [`runs`]: the document as AccessKit text runs, with stable ids.
//! - [`window`]: the part of a large document the view holds.
//! - [`caret`]: caret moves that need no layout.
//! - [`theme`]: textweaver's themes on Masonry's widgets.
//!
//! Owner: Agent W3b.

pub mod caret;
pub mod runs;
pub mod theme;
pub mod window;
