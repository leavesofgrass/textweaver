//! The window's size, place and maximized state, remembered on this
//! computer (Wave 9, W9a-w; `[gui.window]`, a machine setting that never
//! syncs).
//!
//! The window opens at the size it closed at, and at the same place when
//! that place is still on a screen: a saved place on a screen that has
//! since been unplugged is never used, so the window cannot open where no
//! one can see it.

use textweaver_app::store::GuiWindow;

/// The window's first size, in logical pixels, when none was saved.
pub const DEFAULT_SIZE: (f64, f64) = (1100.0, 780.0);
/// The window's smallest size, in logical pixels.
pub const MIN_SIZE: (f64, f64) = (420.0, 320.0);
/// A size larger than any screen, in logical pixels: a saved size past it
/// is not believed.
const MAX_SIDE: f64 = 16_000.0;
/// How much of the window's top edge, in physical pixels, must be on a
/// screen for the saved place to be used: enough to grab the title bar.
const VISIBLE_STRIP: (i64, i64) = (120, 40);

/// A screen's rectangle, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Screen {
    /// The left edge.
    pub x: i32,
    /// The top edge.
    pub y: i32,
    /// The width.
    pub width: u32,
    /// The height.
    pub height: u32,
}

/// The logical size to open at: the saved one, kept within the smallest
/// size and a believable largest, or the default.
pub fn initial_size(saved: Option<&GuiWindow>) -> (f64, f64) {
    let Some(w) = saved.filter(|w| w.width > 0 && w.height > 0) else {
        return DEFAULT_SIZE;
    };
    let side = |v: u32, min: f64| f64::from(v).clamp(min, MAX_SIDE);
    (side(w.width, MIN_SIZE.0), side(w.height, MIN_SIZE.1))
}

/// True when a window whose frame starts at (`x`, `y`) and is `width`
/// physical pixels wide shows enough of its top edge on one of `screens`
/// to be found and moved.
pub fn on_screen(x: i32, y: i32, width: u32, screens: &[Screen]) -> bool {
    let (left, top) = (i64::from(x), i64::from(y));
    let right = left + i64::from(width.max(1));
    screens.iter().any(|s| {
        let (sl, st) = (i64::from(s.x), i64::from(s.y));
        let (sr, sb) = (sl + i64::from(s.width), st + i64::from(s.height));
        let overlap_x = right.min(sr) - left.max(sl);
        // The title bar: the strip just below the frame's top edge.
        let overlap_y = (top + VISIBLE_STRIP.1).min(sb) - top.max(st);
        overlap_x >= VISIBLE_STRIP.0.min(i64::from(width.max(1))) && overlap_y >= VISIBLE_STRIP.1
    })
}

/// The place to open at: the saved one when it is on one of `screens`,
/// else `None` (the system's choice). With no screens known, the saved
/// place is not trusted.
pub fn initial_place(
    saved: Option<&GuiWindow>,
    width: u32,
    screens: &[Screen],
) -> Option<(i32, i32)> {
    let w = saved?;
    if w.width == 0 || w.height == 0 {
        return None;
    }
    on_screen(w.x, w.y, width, screens).then_some((w.x, w.y))
}

/// What the window looked like as it closed, to save: `place` is the
/// frame's top-left corner (physical pixels), `size` the content's size in
/// logical pixels. A maximized or minimized window keeps the place and
/// size it had before (the `previous` ones), so restoring it later still
/// gives a sensible window; a minimized one keeps the maximized state too.
pub fn capture(
    previous: Option<GuiWindow>,
    place: Option<(i32, i32)>,
    size: (f64, f64),
    maximized: bool,
    minimized: bool,
) -> Option<GuiWindow> {
    if minimized {
        return previous;
    }
    if maximized {
        let base = previous.unwrap_or(GuiWindow {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            maximized: false,
        });
        return Some(GuiWindow {
            maximized: true,
            ..base
        });
    }
    let (x, y) = place?;
    let side = |v: f64| {
        if v.is_finite() && v > 0.0 {
            // Within u32 after the clamp.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let px = v.round().min(MAX_SIDE) as u32;
            px
        } else {
            0
        }
    };
    Some(GuiWindow {
        x,
        y,
        width: side(size.0),
        height: side(size.1),
        maximized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Screen = Screen {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    fn saved(x: i32, y: i32, width: u32, height: u32) -> GuiWindow {
        GuiWindow {
            x,
            y,
            width,
            height,
            maximized: false,
        }
    }

    #[test]
    fn the_size_is_the_saved_one_within_limits() {
        assert_eq!(initial_size(None), DEFAULT_SIZE);
        assert_eq!(initial_size(Some(&saved(0, 0, 900, 600))), (900.0, 600.0));
        assert_eq!(initial_size(Some(&saved(0, 0, 10, 10))), MIN_SIZE);
        assert_eq!(initial_size(Some(&saved(0, 0, 0, 600))), DEFAULT_SIZE);
    }

    #[test]
    fn a_place_off_every_screen_is_never_used() {
        let w = saved(100, 100, 900, 600);
        assert_eq!(initial_place(Some(&w), 900, &[SCREEN]), Some((100, 100)));
        // The screen it was on is gone (a second monitor to the right).
        let gone = saved(2500, 100, 900, 600);
        assert_eq!(initial_place(Some(&gone), 900, &[SCREEN]), None);
        // Above the screen: the title bar would be out of reach.
        let above = saved(100, -500, 900, 600);
        assert_eq!(initial_place(Some(&above), 900, &[SCREEN]), None);
        // No screens known: not trusted.
        assert_eq!(initial_place(Some(&w), 900, &[]), None);
        // A second screen to the left, at negative coordinates.
        let left = Screen {
            x: -1280,
            y: 0,
            width: 1280,
            height: 1024,
        };
        let there = saved(-1000, 50, 900, 600);
        assert_eq!(
            initial_place(Some(&there), 900, &[SCREEN, left]),
            Some((-1000, 50))
        );
    }

    #[test]
    fn a_maximized_or_minimized_window_keeps_its_last_normal_place() {
        let before = saved(40, 50, 1000, 700);
        let max =
            capture(Some(before), Some((-8, -8)), (1920.0, 1040.0), true, false).expect("saved");
        assert!(max.maximized);
        assert_eq!((max.x, max.y, max.width, max.height), (40, 50, 1000, 700));
        let min = capture(Some(max), Some((-32000, -32000)), (0.0, 0.0), false, true);
        assert_eq!(min, Some(max));
        let normal =
            capture(Some(max), Some((60, 70)), (999.6, 650.2), false, false).expect("saved");
        assert_eq!(normal, saved(60, 70, 1000, 650));
    }
}
