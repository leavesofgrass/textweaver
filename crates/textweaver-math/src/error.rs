//! Errors.

use textweaver_core::CharRange;

use crate::Notation;

/// Errors from strict parsing ([`Math::strict`](crate::Math::strict)).
///
/// The parsers themselves never fail: they recover and record
/// [`Diagnostic`](crate::Diagnostic)s, because a reader must always be able
/// to speak something for the math on the page.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MathError {
    /// The source is not well-formed.
    #[error("{} error at chars {span}: {message}", notation.name())]
    Parse {
        /// The notation being parsed.
        notation: Notation,
        /// What went wrong.
        message: String,
        /// Where, in chars of the source.
        span: CharRange,
    },
}
