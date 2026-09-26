//! Markdown rendering (ADR-0016): Markdown to accessible HTML with a choice
//! of engines (pulldown-cmark for speed, comrak for the full GFM spec),
//! flavors (GitHub, Obsidian, Pandoc Markdown), LaTeX math as MathML, and
//! MiniJinja templates.
//!
//! Owner: Agent L. Skeleton.

/// This crate's name, so the skeleton has one public item.
pub const CRATE: &str = env!("CARGO_PKG_NAME");
