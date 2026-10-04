//! Where the RSVP word goes on screen: Star's nine positions, the GUI
//! overlay's placement, and a terminal box laid out cell by cell.
//!
//! Lessons from star's accessibility audits (the TUI reflow and `A_DIM`
//! audit, and the WCAG perceivable and operable audit) built in here:
//!
//! - The terminal box is capped to the area, and a word too long for it is
//!   cut with a visible `…` instead of silently losing its tail.
//! - The box can be told which row the caret or spoken word is on, and moves
//!   off it rather than covering it (WCAG 2.4.11, focus not obscured).
//! - Context words are marked [`SegmentRole::Context`], not "dim": the
//!   guidance is normal weight on the panel colour, because terminal dimming
//!   failed contrast on every dark theme Star measured.

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::RsvpFrame;

/// One of Star's nine RSVP positions: top, middle, or bottom by left,
/// centre, or right. The default is top centre, as in Star.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RsvpPosition {
    /// Top left.
    TopLeft,
    /// Top centre (Star's default).
    #[default]
    TopCenter,
    /// Top right.
    TopRight,
    /// Middle left.
    CenterLeft,
    /// Middle of the view.
    Center,
    /// Middle right.
    CenterRight,
    /// Bottom left.
    BottomLeft,
    /// Bottom centre.
    BottomCenter,
    /// Bottom right.
    BottomRight,
}

impl RsvpPosition {
    /// All nine, top to bottom, left to right (Star's cycling order).
    pub const ALL: [RsvpPosition; 9] = [
        RsvpPosition::TopLeft,
        RsvpPosition::TopCenter,
        RsvpPosition::TopRight,
        RsvpPosition::CenterLeft,
        RsvpPosition::Center,
        RsvpPosition::CenterRight,
        RsvpPosition::BottomLeft,
        RsvpPosition::BottomCenter,
        RsvpPosition::BottomRight,
    ];

    /// Star's settings key ("top-center"), also the serialized form.
    pub fn key(self) -> &'static str {
        match self {
            RsvpPosition::TopLeft => "top-left",
            RsvpPosition::TopCenter => "top-center",
            RsvpPosition::TopRight => "top-right",
            RsvpPosition::CenterLeft => "center-left",
            RsvpPosition::Center => "center",
            RsvpPosition::CenterRight => "center-right",
            RsvpPosition::BottomLeft => "bottom-left",
            RsvpPosition::BottomCenter => "bottom-center",
            RsvpPosition::BottomRight => "bottom-right",
        }
    }

    /// The name to show and speak ("top centre" reads oddly aloud in some
    /// voices, so the words are plain: "top center").
    pub fn label(self) -> &'static str {
        match self {
            RsvpPosition::TopLeft => "top left",
            RsvpPosition::TopCenter => "top center",
            RsvpPosition::TopRight => "top right",
            RsvpPosition::CenterLeft => "middle left",
            RsvpPosition::Center => "center",
            RsvpPosition::CenterRight => "middle right",
            RsvpPosition::BottomLeft => "bottom left",
            RsvpPosition::BottomCenter => "bottom center",
            RsvpPosition::BottomRight => "bottom right",
        }
    }

    /// Parses Star's key ("bottom-right"); `None` for anything else.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == key.trim())
    }

    /// `(row, column)` in the 3 × 3 picker grid.
    pub fn grid(self) -> (u8, u8) {
        let i = Self::ALL.iter().position(|p| *p == self).unwrap_or(1) as u8;
        (i / 3, i % 3)
    }

    /// The position at `(row, column)` of the grid, each 0 to 2.
    pub fn from_grid(row: u8, col: u8) -> Option<Self> {
        (row < 3 && col < 3).then(|| Self::ALL[usize::from(row * 3 + col)])
    }

    /// The next position in Star's cycling order, wrapping.
    pub fn next(self) -> Self {
        let (r, c) = self.grid();
        Self::ALL[(usize::from(r * 3 + c) + 1) % 9]
    }

    /// The previous position in Star's cycling order, wrapping.
    pub fn previous(self) -> Self {
        let (r, c) = self.grid();
        Self::ALL[(usize::from(r * 3 + c) + 8) % 9]
    }

    /// Star's anchor fractions `(x, y)`: 0.02, 0.5, or 0.98 of the parent.
    /// The box's own matching point sits there (its top-left corner at 2 %,
    /// its centre at 50 %, its bottom-right corner at 98 %).
    pub fn fractions(self) -> (f64, f64) {
        let (r, c) = self.grid();
        let f = |i: u8| match i {
            0 => 0.02,
            1 => 0.5,
            _ => 0.98,
        };
        (f(c), f(r))
    }

    /// Top-left corner, in pixels, for a GUI overlay of `size` inside a
    /// parent of `parent` size, kept `margin` pixels inside the parent
    /// (Star's `_RSVPOverlay._reposition`, margin 8).
    pub fn gui_origin(self, parent: (i32, i32), size: (i32, i32), margin: i32) -> (i32, i32) {
        let (fx, fy) = self.fractions();
        let (pw, ph) = (f64::from(parent.0), f64::from(parent.1));
        let (w, h) = (size.0.max(1), size.1.max(1));
        let x = (pw * fx - f64::from(w) * fx) as i32;
        let y = (ph * fy - f64::from(h) * fy) as i32;
        let x = x.min(parent.0 - w - margin).max(margin);
        let y = y.min(parent.1 - h - margin).max(margin);
        (x, y)
    }
}

/// A rectangle of terminal cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Area {
    /// Left column.
    pub x: u16,
    /// Top row.
    pub y: u16,
    /// Width in cells.
    pub width: u16,
    /// Height in rows.
    pub height: u16,
}

impl Area {
    /// An area.
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Area {
            x,
            y,
            width,
            height,
        }
    }
}

/// What a piece of the RSVP box shows, so the frontend can style it.
///
/// Suggested terminal styles (never colour alone): the box on the theme's
/// panel or status colours (or reverse video without colours); `Word` bold;
/// `Pivot` bold and underlined; `Context` normal weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentRole {
    /// A previous or next word.
    Context,
    /// The current word, outside its pivot.
    Word,
    /// The pivot grapheme of the current word.
    Pivot,
    /// The `…` shown where a word was cut to fit.
    Clipped,
}

/// A run of text at a column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// Absolute terminal column.
    pub col: u16,
    /// The text; its width in cells is at most what is left of the box.
    pub text: String,
    /// How to style it.
    pub role: SegmentRole,
}

/// The RSVP box for a terminal, cell by cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuiBox {
    /// The box, including its one-cell padding on each side. Fill it with
    /// the panel style first.
    pub area: Area,
    /// Rows from the top of the box: `(absolute row, segments)`.
    pub rows: Vec<(u16, Vec<Segment>)>,
    /// Absolute column of the pivot, which stays put from word to word.
    pub pivot_col: u16,
    /// True when some text was cut to fit.
    pub clipped: bool,
}

/// Options for [`tui_box`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TuiBoxOptions {
    /// Where to put the box.
    pub position: RsvpPosition,
    /// Width of the box in cells, padding included; capped to the area.
    /// A fixed width keeps the pivot column still.
    pub width: u16,
    /// Show the previous word above.
    pub show_previous: bool,
    /// Show the next word below.
    pub show_next: bool,
    /// A row the box must not cover (the caret or the spoken word).
    pub avoid_row: Option<u16>,
}

impl Default for TuiBoxOptions {
    fn default() -> Self {
        TuiBoxOptions {
            position: RsvpPosition::TopCenter,
            width: 32,
            show_previous: true,
            show_next: true,
            avoid_row: None,
        }
    }
}

/// Cuts `s` to at most `max` cells, grapheme by grapheme. Returns the kept
/// text and whether anything was cut.
fn fit(s: &str, max: usize) -> (String, bool) {
    if s.width() <= max {
        return (s.to_owned(), false);
    }
    let mut out = String::new();
    let mut w = 0;
    for g in s.graphemes(true) {
        let gw = g.width();
        if w + gw > max {
            break;
        }
        out.push_str(g);
        w += gw;
    }
    (out, true)
}

/// Lays out the RSVP box for `frame` inside `area` (the document view).
/// `None` when the area is too small to hold even one cell of text.
pub fn tui_box(frame: &RsvpFrame<'_>, area: Area, opts: &TuiBoxOptions) -> Option<TuiBox> {
    if area.width < 3 || area.height == 0 {
        return None;
    }
    let width = opts.width.max(5).min(area.width);
    let inner = usize::from(width - 2);
    let mut rows_wanted = 1u16;
    if opts.show_previous {
        rows_wanted += 1;
    }
    if opts.show_next {
        rows_wanted += 1;
    }
    let height = rows_wanted.min(area.height);

    // Place the box inside the area: flush to an edge or centred. (Star's
    // 2 % margins are less than a cell in a terminal.)
    let (r, c) = opts.position.grid();
    let frac = |i: u8| f64::from(i) / 2.0;
    let (fx, fy) = (frac(c), frac(r));
    let max_x = area.x.saturating_add(area.width - width);
    let max_y = area.y.saturating_add(area.height - height);
    let x = (f64::from(area.x) + f64::from(area.width - width) * fx).round() as u16;
    let mut y = (f64::from(area.y) + f64::from(area.height - height) * fy).round() as u16;
    let x = x.clamp(area.x, max_x);
    y = y.clamp(area.y, max_y);
    if let Some(avoid) = opts.avoid_row
        && (y..y + height).contains(&avoid)
    {
        if avoid >= area.y + height {
            y = avoid - height; // just above
        } else if avoid < max_y {
            y = avoid + 1; // just below
        }
    }

    let left = x + 1;
    // The pivot sits three tenths in, at least past the text before it.
    let before_w = frame.before.width();
    let pivot_off = (inner * 3 / 10).max(before_w).min(inner.saturating_sub(1));
    let pivot_col = left + pivot_off as u16;
    let mut clipped = false;

    let context_row = |text: Option<&str>, clipped: &mut bool| -> Vec<Segment> {
        let Some(t) = text.filter(|t| !t.is_empty()) else {
            return Vec::new();
        };
        let (s, cut) = fit(t, if t.width() > inner { inner - 1 } else { inner });
        *clipped |= cut;
        // Centre the context word on the pivot column where it fits.
        let w = s.width() + usize::from(cut);
        let start = pivot_off.saturating_sub(w / 2).min(inner - w);
        let mut v = vec![Segment {
            col: left + start as u16,
            text: s,
            role: SegmentRole::Context,
        }];
        if cut {
            v.push(Segment {
                col: left + (start + w - 1) as u16,
                text: "\u{2026}".into(),
                role: SegmentRole::Clipped,
            });
        }
        v
    };

    let mut focus = Vec::new();
    {
        let start = pivot_off - before_w.min(pivot_off);
        let mut col = start;
        let mut room = inner - start;
        let parts = [
            (frame.before, SegmentRole::Word),
            (frame.pivot, SegmentRole::Pivot),
            (frame.after, SegmentRole::Word),
        ];
        let total: usize = parts.iter().map(|(p, _)| p.width()).sum();
        let need_cut = total > room;
        if need_cut {
            room -= 1; // leave a cell for the ellipsis
        }
        for (text, role) in parts {
            if text.is_empty() || room == 0 {
                continue;
            }
            let (s, cut) = fit(text, room);
            let w = s.width();
            if !s.is_empty() {
                focus.push(Segment {
                    col: left + col as u16,
                    text: s,
                    role,
                });
            }
            col += w;
            room -= w;
            if cut {
                room = 0;
            }
        }
        if need_cut {
            clipped = true;
            focus.push(Segment {
                col: left + col as u16,
                text: "\u{2026}".into(),
                role: SegmentRole::Clipped,
            });
        }
    }

    let mut rows = Vec::new();
    let mut row = y;
    let mut push = |segs: Vec<Segment>, rows: &mut Vec<(u16, Vec<Segment>)>| {
        if row < y + height {
            rows.push((row, segs));
            row += 1;
        }
    };
    if opts.show_previous && height == rows_wanted {
        push(context_row(frame.previous, &mut clipped), &mut rows);
    }
    push(focus, &mut rows);
    if opts.show_next && height == rows_wanted {
        push(context_row(frame.next, &mut clipped), &mut rows);
    }
    let height = rows.len() as u16;
    Some(TuiBox {
        area: Area::new(x, y, width, height),
        rows,
        pivot_col,
        clipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rsvp::{Rsvp, RsvpSettings, WordTrack};

    #[test]
    fn keys_labels_and_cycling() {
        assert_eq!(RsvpPosition::default().key(), "top-center");
        for p in RsvpPosition::ALL {
            assert_eq!(RsvpPosition::from_key(p.key()), Some(p));
            let (r, c) = p.grid();
            assert_eq!(RsvpPosition::from_grid(r, c), Some(p));
            assert_eq!(p.next().previous(), p);
        }
        assert_eq!(RsvpPosition::BottomRight.next(), RsvpPosition::TopLeft);
        assert_eq!(RsvpPosition::from_key("sideways"), None);
        assert_eq!(RsvpPosition::from_grid(3, 0), None);
        let json = serde_json::to_string(&RsvpPosition::CenterRight).unwrap();
        assert_eq!(json, "\"center-right\"");
    }

    /// Ported from Star's tests/test_rsvp.py (800 × 600 parent, 200 × 80 box).
    #[test]
    fn gui_origin_matches_star() {
        let o = |p: RsvpPosition| p.gui_origin((800, 600), (200, 80), 8);
        assert_eq!(o(RsvpPosition::TopLeft), (12, 10));
        assert_eq!(o(RsvpPosition::TopRight), (588, 10));
        assert_eq!(o(RsvpPosition::Center), (300, 260));
        let (x, y) = o(RsvpPosition::BottomRight);
        assert!(x > 400 && y > 400);
        for p in RsvpPosition::ALL {
            let (x, y) = o(p);
            assert!((8..=592).contains(&x) && (8..=512).contains(&y), "{p:?}");
        }
    }

    fn frame_box(text: &str, index: usize, area: Area, opts: &TuiBoxOptions) -> TuiBox {
        let mut r = Rsvp::new(WordTrack::from_text(text), RsvpSettings::default());
        r.seek_word(index, 0);
        tui_box(&r.frame().unwrap(), area, opts).unwrap()
    }

    fn row_text(b: &TuiBox, i: usize) -> String {
        b.rows[i].1.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn box_positions_in_a_terminal() {
        let area = Area::new(0, 1, 80, 20);
        let top = frame_box("one two three", 1, area, &TuiBoxOptions::default());
        assert_eq!(top.area.y, 1);
        assert_eq!(top.area.height, 3);
        assert!(top.area.x > 20 && top.area.x + top.area.width <= 80);
        assert_eq!(row_text(&top, 0), "one");
        assert_eq!(row_text(&top, 1), "two");
        assert_eq!(row_text(&top, 2), "three");
        let br = frame_box(
            "one two three",
            1,
            area,
            &TuiBoxOptions {
                position: RsvpPosition::BottomRight,
                ..TuiBoxOptions::default()
            },
        );
        assert_eq!(br.area.y + br.area.height, 21);
        assert_eq!(br.area.x + br.area.width, 80);
    }

    #[test]
    fn pivot_column_is_stable() {
        let area = Area::new(0, 0, 80, 10);
        let opts = TuiBoxOptions::default();
        let a = frame_box("a extraordinary be", 0, area, &opts);
        let b = frame_box("a extraordinary be", 1, area, &opts);
        assert_eq!(a.pivot_col, b.pivot_col);
        let piv = b.rows[1]
            .1
            .iter()
            .find(|s| s.role == SegmentRole::Pivot)
            .unwrap();
        assert_eq!(piv.col, b.pivot_col);
        assert_eq!(piv.text, "r");
    }

    #[test]
    fn long_words_are_cut_visibly_at_narrow_widths() {
        let word = "pneumonoultramicroscopicsilicovolcanoconiosis";
        for w in [20u16, 40] {
            let b = frame_box(word, 0, Area::new(0, 0, w, 10), &TuiBoxOptions::default());
            assert!(b.clipped);
            assert!(b.area.width <= w);
            let focus = &b.rows[1].1;
            assert_eq!(focus.last().unwrap().role, SegmentRole::Clipped);
            let end = focus.last().unwrap().col + 1;
            assert!(end < b.area.x + b.area.width, "inside the padding");
            for s in focus {
                assert!(s.col > b.area.x);
            }
        }
    }

    #[test]
    fn box_moves_off_the_caret_row() {
        let area = Area::new(0, 0, 80, 20);
        let opts = TuiBoxOptions {
            position: RsvpPosition::Center,
            avoid_row: Some(9),
            ..TuiBoxOptions::default()
        };
        let b = frame_box("one two three", 1, area, &opts);
        assert!(!(b.area.y..b.area.y + b.area.height).contains(&9));
        let top = TuiBoxOptions {
            position: RsvpPosition::TopCenter,
            avoid_row: Some(1),
            ..TuiBoxOptions::default()
        };
        let b = frame_box("one two three", 1, area, &top);
        assert_eq!(b.area.y, 2);
    }

    #[test]
    fn tiny_areas() {
        let b = frame_box(
            "hello there",
            0,
            Area::new(0, 0, 6, 1),
            &TuiBoxOptions::default(),
        );
        assert_eq!(b.area.height, 1);
        assert_eq!(b.rows.len(), 1);
        let r = Rsvp::new(WordTrack::from_text("x"), RsvpSettings::default());
        assert!(
            tui_box(
                &r.frame().unwrap(),
                Area::new(0, 0, 2, 5),
                &TuiBoxOptions::default()
            )
            .is_none()
        );
    }
}
