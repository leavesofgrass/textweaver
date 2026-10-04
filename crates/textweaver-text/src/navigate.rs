//! Pure navigation: where the next or previous unit is, and go-to targets.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, Direction, MarkerKind, Unit};

use crate::Document;
use crate::units::{first_unit, last_unit, next_unit, prev_unit};

/// Options for a navigation step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavOptions {
    /// Wrap around the document ends (star's `wrap` setting).
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
/// starting before `from`. Marker units step between marker starts, filtered
/// by level when one is given. With `wrap`, stepping past an end continues
/// from the other end and `NavTarget::wrapped` is set; without it the result
/// is `None` (star clamps and says "No next ...").
///
/// star's "previous sentence rewinds to the start of the current sentence
/// when more than three words in" is an app-level rule built on top of this
/// (Agent D), not part of this function.
pub fn navigate(
    doc: &Document,
    from: CharPos,
    unit: Unit,
    dir: Direction,
    opts: NavOptions,
) -> Option<NavTarget> {
    if let Unit::Marker { kind, level } = unit {
        return doc
            .marker_index()
            .step(kind, level, from, dir, opts.wrap)
            .map(|(m, wrapped)| NavTarget {
                range: m.range,
                wrapped,
            });
    }
    let found = match dir {
        Direction::Forward => next_unit(doc, from, unit),
        Direction::Backward => prev_unit(doc, from, unit),
    };
    if let Some(range) = found {
        return Some(NavTarget {
            range,
            wrapped: false,
        });
    }
    if !opts.wrap {
        return None;
    }
    match dir {
        Direction::Forward => first_unit(doc, unit),
        Direction::Backward => last_unit(doc, unit),
    }
    .map(|range| NavTarget {
        range,
        wrapped: true,
    })
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
    /// The `n`th heading of any level, 1-based.
    Heading(usize),
}

/// Resolves a go-to target to a position, clamped to the document.
///
/// Percentages and lines past the end clamp to the end; a heading number
/// past the last heading goes to the last heading, and with no headings to
/// the start. Use [`go_to_checked`] to detect a missing heading.
pub fn go_to(doc: &Document, target: GoTo) -> CharPos {
    if let GoTo::Heading(n) = target {
        let index = doc.marker_index();
        let count = index.count(MarkerKind::Heading, None);
        if count == 0 {
            return CharPos::ZERO;
        }
        let n = n.clamp(1, count);
        return index
            .nth(MarkerKind::Heading, None, n - 1)
            .map_or(CharPos::ZERO, |m| m.range.start);
    }
    go_to_checked(doc, target).unwrap_or(CharPos::ZERO)
}

/// Resolves a go-to target, or `None` when it names a heading that does not
/// exist. Other targets clamp as in [`go_to`].
pub fn go_to_checked(doc: &Document, target: GoTo) -> Option<CharPos> {
    let len = doc.len_chars();
    Some(match target {
        GoTo::Percent(p) => CharPos(len * usize::from(p.min(100)) / 100),
        GoTo::Line(n) => {
            let line = n.saturating_sub(1);
            if line >= doc.line_count() {
                CharPos(len)
            } else {
                doc.line_range(line).start
            }
        }
        GoTo::Char(c) => c.clamp_to(len),
        GoTo::Start => CharPos::ZERO,
        GoTo::End => CharPos(len),
        GoTo::Heading(n) => {
            let index = n.checked_sub(1)?;
            doc.marker_index()
                .nth(MarkerKind::Heading, None, index)?
                .range
                .start
        }
    })
}

/// A go-to target that could not be parsed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "cannot go to \"{0}\": use a line number, a percentage (50%), start, end, h3 for the third heading, or c120 for a character"
)]
pub struct ParseGoToError(pub String);

impl FromStr for GoTo {
    type Err = ParseGoToError;

    /// Parses what a user types at a "Go to" prompt: `12` or `line 12` (a
    /// line), `50%` (a percentage), `start`/`top`, `end`/`bottom`, `h3` or
    /// `heading 3` (the third heading), `c120` or `char 120` (a character).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseGoToError(s.trim().to_owned());
        let t = s.trim().to_lowercase();
        let num = |x: &str| x.trim().parse::<usize>().map_err(|_| err());
        match t.as_str() {
            "start" | "top" | "beginning" => return Ok(GoTo::Start),
            "end" | "bottom" => return Ok(GoTo::End),
            _ => {}
        }
        if let Some(p) = t.strip_suffix('%') {
            let p = num(p)?;
            return u8::try_from(p.min(100))
                .map(GoTo::Percent)
                .map_err(|_| err());
        }
        for (prefix, make) in [
            ("heading", GoTo::Heading as fn(usize) -> GoTo),
            ("line", GoTo::Line),
            ("char", |n| GoTo::Char(CharPos(n))),
            ("h", GoTo::Heading),
            ("l", GoTo::Line),
            ("c", |n| GoTo::Char(CharPos(n))),
        ] {
            if let Some(rest) = t.strip_prefix(prefix)
                && let Ok(n) = num(rest)
            {
                return Ok(make(n));
            }
        }
        num(&t).map(GoTo::Line)
    }
}

impl fmt::Display for GoTo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GoTo::Percent(p) => write!(f, "{p}%"),
            GoTo::Line(n) => write!(f, "line {n}"),
            GoTo::Char(c) => write!(f, "char {c}"),
            GoTo::Start => f.write_str("start"),
            GoTo::End => f.write_str("end"),
            GoTo::Heading(n) => write!(f, "heading {n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use ropey::Rope;

    use super::*;
    use crate::{DocumentMeta, Marker};

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
        )
        .unwrap();
        assert!(back.wrapped);
        assert_eq!(back.range, CharRange::new(10, 16));
        let none = navigate(
            &d,
            CharPos(10),
            Unit::Sentence,
            Direction::Forward,
            NavOptions::default(),
        );
        assert!(none.is_none());
    }

    fn headed() -> Document {
        let text = "Top\n\nbody\n\nMiddle\n\nbody\n\nLow";
        Document::new(
            DocumentMeta::default(),
            Rope::from_str(text),
            vec![
                Marker::new(MarkerKind::Heading, CharRange::new(0, 3)).with_level(1),
                Marker::new(MarkerKind::Heading, CharRange::new(11, 17)).with_level(2),
                Marker::new(MarkerKind::Heading, CharRange::new(25, 28)).with_level(2),
            ],
        )
    }

    #[test]
    fn marker_navigation_with_levels() {
        let d = headed();
        let next2 = navigate(
            &d,
            CharPos(0),
            Unit::heading(2),
            Direction::Forward,
            NavOptions::default(),
        );
        assert_eq!(next2.unwrap().range.start, CharPos(11));
        let prev = navigate(
            &d,
            CharPos(11),
            Unit::marker(MarkerKind::Heading),
            Direction::Backward,
            NavOptions::default(),
        );
        assert_eq!(prev.unwrap().range.start, CharPos(0));
    }

    #[test]
    fn go_to_percent_line_and_heading() {
        let d = Document::from_plain_text("ab\ncd\nef");
        assert_eq!(go_to(&d, GoTo::Line(2)), CharPos(3));
        assert_eq!(go_to(&d, GoTo::Line(99)), CharPos(8));
        assert_eq!(go_to(&d, GoTo::Percent(100)), CharPos(8));
        assert_eq!(go_to(&d, GoTo::Percent(50)), CharPos(4));
        assert_eq!(go_to_checked(&d, GoTo::Heading(1)), None);
        let h = headed();
        assert_eq!(go_to(&h, GoTo::Heading(2)), CharPos(11));
        assert_eq!(go_to(&h, GoTo::Heading(9)), CharPos(25));
        assert_eq!(go_to_checked(&h, GoTo::Heading(9)), None);
        assert_eq!(go_to_checked(&h, GoTo::Heading(0)), None);
    }

    #[test]
    fn parse_go_to() {
        let p = |s: &str| s.parse::<GoTo>();
        assert_eq!(p("12"), Ok(GoTo::Line(12)));
        assert_eq!(p("line 7"), Ok(GoTo::Line(7)));
        assert_eq!(p(" 50% "), Ok(GoTo::Percent(50)));
        assert_eq!(p("250%"), Ok(GoTo::Percent(100)));
        assert_eq!(p("Start"), Ok(GoTo::Start));
        assert_eq!(p("end"), Ok(GoTo::End));
        assert_eq!(p("h3"), Ok(GoTo::Heading(3)));
        assert_eq!(p("heading 2"), Ok(GoTo::Heading(2)));
        assert_eq!(p("c120"), Ok(GoTo::Char(CharPos(120))));
        assert!(p("chapter").is_err());
        assert_eq!(GoTo::Heading(3).to_string(), "heading 3");
    }
}
