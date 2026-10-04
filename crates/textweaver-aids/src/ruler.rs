//! The reading ruler and the current-line band, as data.
//!
//! star had two separate aids (`gui/main_window.py:263-349`,
//! `gui/mixin_commands.py:353-362`): a thin tint behind the caret's line,
//! and a translucent band (a typoscope) floating over the text around it.
//! Here both are one computation: given the rows a view shows, which rows
//! are the focus, which are the band around it, and (optionally) which are
//! masked outside it. The frontend decides how each mark looks.
//!
//! Lessons from star's accessibility audits, which the style guidance
//! in [`TermStyle::recommended`] and the crate guide follow:
//!
//! - star painted the ruler *over* the text at 22 % opacity, which pulled
//!   body text below 4.5:1 on four themes. Paint bands *behind* text.
//! - star's current-line band reused the selection colour, 1.1 to 1.9:1
//!   against the page: invisible to many low-vision readers (WCAG 1.4.11
//!   wants 3:1). Never mark a row by colour alone: add underline, bold,
//!   reverse video, or a gutter mark.
//! - Terminal dimming (SGR 2) failed contrast on every dark theme star
//!   measured, so masking is off by default and uses dimming only when the
//!   reader turns it on.

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange};

/// One row a view shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ViewRow {
    /// The canonical chars on this row (a wrapped piece of a line, or the
    /// whole line). Empty for a blank line, at its position.
    pub range: CharRange,
    /// The logical line (document line index) the row belongs to, so the
    /// wrapped rows of one line can be banded together.
    pub line: usize,
}

/// Which aid is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulerMode {
    /// No marks.
    #[default]
    Off,
    /// Mark only the focus rows (star's current-line highlight).
    CurrentLine,
    /// Mark the focus rows and a band of rows around them (star's reading
    /// ruler).
    Ruler,
}

/// What counts as "the line" for the focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulerScope {
    /// The one screen row holding the position.
    Row,
    /// Every wrapped row of the logical line holding the position.
    #[default]
    Line,
}

/// Reading ruler settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct RulerSettings {
    /// Off, current line, or ruler.
    pub mode: RulerMode,
    /// Row or whole line.
    pub scope: RulerScope,
    /// Band rows above the focus (ruler mode).
    pub rows_above: u8,
    /// Band rows below the focus (ruler mode).
    pub rows_below: u8,
    /// Mask the rows outside the band (a typoscope). Off by default.
    pub mask_outside: bool,
}

impl Default for RulerSettings {
    fn default() -> Self {
        RulerSettings {
            mode: RulerMode::Off,
            scope: RulerScope::Line,
            rows_above: 1,
            rows_below: 1,
            mask_outside: false,
        }
    }
}

/// How one row is marked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RowMark {
    /// Not marked.
    #[default]
    Normal,
    /// The row holding the reading position.
    Focus,
    /// Inside the ruler band, next to the focus.
    Band,
    /// Outside the band while masking is on.
    Masked,
}

/// The index of the row showing `pos`: the row whose range contains it, or
/// the row it ends (a caret at the end of a line), or an empty row at it.
pub fn focus_row(rows: &[ViewRow], pos: CharPos) -> Option<usize> {
    rows.iter()
        .position(|r| r.range.contains(pos))
        .or_else(|| rows.iter().position(|r| r.range.end == pos))
}

/// Marks each of `rows` for the reading position `pos` (the caret, or the
/// start of the spoken word). All rows are [`RowMark::Normal`] when the mode
/// is off or `pos` is not on screen.
pub fn ruler_rows(rows: &[ViewRow], pos: CharPos, settings: &RulerSettings) -> Vec<RowMark> {
    let mut marks = vec![RowMark::Normal; rows.len()];
    if settings.mode == RulerMode::Off {
        return marks;
    }
    let Some(f) = focus_row(rows, pos) else {
        return marks;
    };
    let (mut first, mut last) = (f, f);
    if settings.scope == RulerScope::Line {
        let line = rows[f].line;
        while first > 0 && rows[first - 1].line == line {
            first -= 1;
        }
        while last + 1 < rows.len() && rows[last + 1].line == line {
            last += 1;
        }
    }
    for m in &mut marks[first..=last] {
        *m = RowMark::Focus;
    }
    if settings.mode == RulerMode::Ruler {
        let band_first = first.saturating_sub(usize::from(settings.rows_above));
        let band_last = (last + usize::from(settings.rows_below)).min(rows.len() - 1);
        for (i, m) in marks.iter_mut().enumerate() {
            if *m == RowMark::Focus {
                continue;
            }
            if (band_first..=band_last).contains(&i) {
                *m = RowMark::Band;
            } else if settings.mask_outside {
                *m = RowMark::Masked;
            }
        }
    }
    marks
}

/// Terminal attributes for a row mark. Colours come from the theme; these
/// are the non-colour cues that must be present too.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TermStyle {
    /// Bold text.
    pub bold: bool,
    /// Underlined text.
    pub underline: bool,
    /// Reverse video.
    pub reverse: bool,
    /// Dimmed text (only for masking, which the reader opts into).
    pub dim: bool,
    /// A mark to draw in the left gutter, if the view has one.
    pub gutter: Option<char>,
}

impl TermStyle {
    /// The recommended style for `mark` in a terminal. The spoken word
    /// already uses reverse video in the TUI, so the focus line is
    /// underlined with a gutter bar, and the band gets a lighter gutter
    /// mark; neither relies on colour.
    pub fn recommended(mark: RowMark) -> Self {
        match mark {
            RowMark::Normal => TermStyle::default(),
            RowMark::Focus => TermStyle {
                underline: true,
                gutter: Some('\u{258c}'), // ▌
                ..TermStyle::default()
            },
            RowMark::Band => TermStyle {
                gutter: Some('\u{2502}'), // │
                ..TermStyle::default()
            },
            RowMark::Masked => TermStyle {
                dim: true,
                ..TermStyle::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Line 0 wraps onto rows 0-1, line 1 is row 2, line 2 (blank) row 3,
    /// line 3 wraps onto rows 4-6.
    fn rows() -> Vec<ViewRow> {
        let r = |a: usize, b: usize, line: usize| ViewRow {
            range: CharRange::new(a, b),
            line,
        };
        vec![
            r(0, 10, 0),
            r(10, 18, 0),
            r(19, 30, 1),
            r(31, 31, 2),
            r(32, 40, 3),
            r(40, 50, 3),
            r(50, 55, 3),
        ]
    }

    fn settings(mode: RulerMode, scope: RulerScope) -> RulerSettings {
        RulerSettings {
            mode,
            scope,
            ..RulerSettings::default()
        }
    }

    use RowMark::{Band as B, Focus as F, Masked as M, Normal as N};

    #[test]
    fn off_and_offscreen() {
        let rs = rows();
        assert_eq!(
            ruler_rows(&rs, CharPos(5), &RulerSettings::default()),
            [N; 7]
        );
        let s = settings(RulerMode::Ruler, RulerScope::Row);
        assert_eq!(ruler_rows(&rs, CharPos(999), &s), [N; 7]);
        assert_eq!(ruler_rows(&[], CharPos(0), &s), Vec::<RowMark>::new());
    }

    #[test]
    fn current_line_by_row_and_by_line() {
        let rs = rows();
        let row = settings(RulerMode::CurrentLine, RulerScope::Row);
        assert_eq!(ruler_rows(&rs, CharPos(12), &row), [N, F, N, N, N, N, N]);
        let line = settings(RulerMode::CurrentLine, RulerScope::Line);
        assert_eq!(ruler_rows(&rs, CharPos(12), &line), [F, F, N, N, N, N, N]);
        // Caret at the end of a line, and on a blank line.
        assert_eq!(ruler_rows(&rs, CharPos(30), &line), [N, N, F, N, N, N, N]);
        assert_eq!(ruler_rows(&rs, CharPos(31), &line), [N, N, N, F, N, N, N]);
    }

    #[test]
    fn ruler_band_and_mask() {
        let rs = rows();
        let s = settings(RulerMode::Ruler, RulerScope::Line);
        assert_eq!(ruler_rows(&rs, CharPos(20), &s), [N, B, F, B, N, N, N]);
        assert_eq!(ruler_rows(&rs, CharPos(45), &s), [N, N, N, B, F, F, F]);
        let masked = RulerSettings {
            mask_outside: true,
            rows_above: 0,
            rows_below: 2,
            ..s
        };
        assert_eq!(ruler_rows(&rs, CharPos(0), &masked), [F, F, B, B, M, M, M]);
    }

    #[test]
    fn styles_never_rely_on_colour() {
        for m in [RowMark::Focus, RowMark::Band, RowMark::Masked] {
            let s = TermStyle::recommended(m);
            assert!(s.bold || s.underline || s.reverse || s.dim || s.gutter.is_some());
        }
        assert_eq!(
            TermStyle::recommended(RowMark::Normal),
            TermStyle::default()
        );
    }

    #[test]
    fn settings_serialize() {
        let s = RulerSettings {
            mode: RulerMode::Ruler,
            ..RulerSettings::default()
        };
        let t = toml::to_string(&s).unwrap();
        assert!(t.contains("mode = \"ruler\""), "{t}");
        assert_eq!(toml::from_str::<RulerSettings>(&t).unwrap(), s);
    }
}
