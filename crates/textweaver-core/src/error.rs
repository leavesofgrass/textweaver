use crate::CharPos;

/// Errors raised by core operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    /// A position lies beyond the end of the text it refers to.
    #[error("position {pos} is out of range for text of {len} chars")]
    PositionOutOfRange {
        /// The offending position.
        pos: CharPos,
        /// Length of the text in chars.
        len: usize,
    },
    /// A range whose start lies after its end, or that extends past the text.
    #[error("invalid range {start}..{end} for text of {len} chars")]
    InvalidRange {
        /// Range start.
        start: CharPos,
        /// Range end.
        end: CharPos,
        /// Length of the text in chars.
        len: usize,
    },
    /// An [`OffsetMap`](crate::OffsetMap) violates one of its invariants.
    #[error("invalid offset map: {0}")]
    InvalidOffsetMap(String),
}
