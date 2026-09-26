//! Bulk conversion (ADR-0016): convert files and whole folder trees on all
//! cores, mirroring the tree and skipping outputs newer than their source,
//! with a timing summary; the engine behind `tw convert`.
//!
//! Owner: Agent L. Skeleton.

/// This crate's name, so the skeleton has one public item.
pub const CRATE: &str = env!("CARGO_PKG_NAME");
