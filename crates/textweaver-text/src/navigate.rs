//! Pure navigation: where the next or previous unit is, and go-to targets.

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, Direction, Unit};

use crate::Document;
use crate::marker::MarkerIndex;
use crate::units::segments;

/// Options for a navigation step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavOptions {
    /// Wrap around the document ends (Star's `wrap` setting).
    pub wrap: bool,
}

/// Where a navigation step landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavTarget {
    /// The unit that was reached.
    pub range: CharRange,
    /// True when the step wrapped around an end of the document.
    pub wrapped: bool,
}

/// The next (forward) or previous (backward) `unit` relative to `from`.
///
/// Forward finds the first unit starting after `from`; backward the last unit
/// starting before `from`. Star's "previous sentence rewinds to the start of
/// the current sentence when more than three words in" is an app-level rule
/// built on top of this (Agent D), not part of this function.
pub fn navigate(
    doc: &Document,
    from: CharPos,
    unit: Unit,
    dir: Direction,
    opts: NavOptions,
) -> Option<NavTarget> {
    if let Unit::Marker { kind, level } = unit {
        return MarkerIndex::new(doc.markers())
            .step(kind, level, from, dir, opts.wrap)
            .map(|(m, wrapped)| NavTarget {
                range: m.range,
                wrapped,
            });
    }
    let segs = segments(doc, unit);
    let found = match dir {
        Direction::Forward => segs.iter().find(|r| r.start > from),
        Direction::Backward => segs.iter().rev().find(|r| r.start < from),
    };
    match (found, opts.wrap) {
        (Some(r), _) => Some(NavTarget {
            range: *r,
            wrapped: false,
        }),
        (None, true) => match dir {
            Direction::Forward => segs.first(),
            Direction::Backward => segs.last(),
        }
        .map(|r| NavTarget {
            range: *r,
            wrapped: true,
        }),
        (None, false) => None,
    }
}

/// An absolute destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoTo {
    /// A percentage of the document, 0 to 100.
    Percent(u8),
    /// A 1-based line number.
    Line(usize),
    /// A char position.
    Char(CharPos),
    /// The start of the document.
    Start,
    /// The end of the document.
    End,
}

/// Resolves a go-to target to a position, clamped to the document.
pub fn go_to(doc: &Document, target: GoTo) -> CharPos {
    let len = doc.len_chars();
    match target {
        GoTo::Percent(p) => CharPos(len * usize::from(p.min(100)) / 100),
        GoTo::Line(n) => {
            let line = n
                .saturating_sub(1)
                .min(doc.text().len_lines().saturating_sub(1));
            CharPos(doc.text().line_to_char(line))
        }
        GoTo::Char(c) => c.clamp_to(len),
        GoTo::Start => CharPos::ZERO,
        GoTo::End => CharPos(len),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_and_previous_sentence() {
        let d = Document::from_plain_text("One. Two. Three.");
        let fwd = navigate(
            &d,
            CharPos(0),
            Unit::Sentence,
            Direction::Forward,
            NavOptions::default(),
        );
        assert_eq!(fwd.unwrap().range, CharRange::new(5, 9));
        let back = navigate(
            &d,
            CharPos(0),
            Unit::Sentence,
            Direction::Backward,
            NavOptions { wrap: true },
        );
        assert!(back.unwrap().wrapped);
    }

    #[test]
    fn go_to_percent_and_line() {
        let d = Document::from_plain_text("ab\ncd\nef");
        assert_eq!(go_to(&d, GoTo::Line(2)), CharPos(3));
        assert_eq!(go_to(&d, GoTo::Percent(100)), CharPos(8));
    }
}
