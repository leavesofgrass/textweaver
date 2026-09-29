//! RSVP and WCAG 2.3.1, Three Flashes or Below Threshold (ADR-0037).
//!
//! RSVP changes the word up to 25 times a second at the highest rate
//! ([`MAX_WPM`](super::MAX_WPM), 40 ms a word). WCAG counts a flash when a
//! pair of opposing changes in luminance covers enough of the screen: the
//! general flash threshold is a quarter of any 10-degree field of view,
//! which WCAG gives as 341 by 256 pixels on a 1024 by 768 screen
//! ([`FLASH_AREA_PX`]). Below that area, changes are not flashes at any
//! rate.
//!
//! The frontends never blank or restyle the panel between words; only the
//! glyphs change (the panel keeps its place and size, and there is no empty
//! frame). So the area that can change from one word to the next is at
//! most the glyph cells that differ, times the share of a cell's pixels a
//! glyph covers ([`GLYPH_INK`], set high, for bold text). [`FlashModel`]
//! measures that for a frontend's sizes; a change at or above
//! [`FLASH_AREA_PX`] counts. A frontend that draws words so large that
//! their changes count keeps each such word up for at least
//! [`SAFE_CHANGE_INTERVAL`] ([`capped_duration`]), which allows at most
//! six counted changes, three flashes, in any second.
//!
//! Measured in the tests: the terminal box (cells up to 16 by 32 pixels,
//! a 24-point terminal font) and the window's panel (a 40-pixel word,
//! 18-pixel context words) stay under the threshold at every rate, so
//! neither needs the cap; a 200-point word would need it.

use unicode_segmentation::UnicodeSegmentation;

use super::{Millis, RsvpFrame};

/// WCAG's general flash area: a quarter of 341 by 256 pixels, in square
/// pixels.
pub const FLASH_AREA_PX: f64 = 341.0 * 256.0 / 4.0;

/// The most flashes allowed in any one second.
pub const MAX_FLASHES_PER_SECOND: usize = 3;

/// The shortest time between two counted changes that keeps any second
/// to six changes (three flashes): 1000 / 6 ms, rounded up.
pub const SAFE_CHANGE_INTERVAL: Millis = 167;

/// The share of a glyph cell's pixels that change when one glyph replaces
/// another or a blank, at most. Regular text covers about a sixth of its
/// cells; bold about a quarter. 0.35 leaves room for heavy fonts.
pub const GLYPH_INK: f64 = 0.35;

/// The size of one character's box, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphBox {
    /// Width (the advance), in pixels.
    pub width: f64,
    /// Height (the line), in pixels.
    pub height: f64,
}

impl GlyphBox {
    /// A glyph box of `width` by `height` pixels.
    pub const fn new(width: f64, height: f64) -> Self {
        GlyphBox { width, height }
    }

    fn area(self) -> f64 {
        self.width * self.height
    }
}

/// How a frontend draws RSVP: the current word's glyph box and the context
/// words' (the previous and next words, when shown).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlashModel {
    /// The current word's glyphs.
    pub word: GlyphBox,
    /// The context words' glyphs.
    pub context: GlyphBox,
}

impl FlashModel {
    /// A terminal whose cells are `cell_width` by `cell_height` pixels.
    pub const fn terminal(cell_width: f64, cell_height: f64) -> Self {
        let cell = GlyphBox::new(cell_width, cell_height);
        FlashModel {
            word: cell,
            context: cell,
        }
    }

    /// The area, in square pixels, whose luminance can change when `b`
    /// replaces `a`. The word row is compared grapheme by grapheme from the
    /// pivot, which stays in one place; a context row counts every grapheme
    /// of the longer word, since context words are centered.
    pub fn changed_area(&self, a: &RsvpFrame<'_>, b: &RsvpFrame<'_>) -> f64 {
        let word = changed_word_cells(a, b) as f64 * self.word.area();
        let context = [(a.previous, b.previous), (a.next, b.next)]
            .into_iter()
            .map(|(x, y)| changed_context_cells(x, y) as f64 * self.context.area())
            .sum::<f64>();
        (word + context) * GLYPH_INK
    }

    /// True when the change from `a` to `b` is large enough to count
    /// toward a flash.
    pub fn counts(&self, a: &RsvpFrame<'_>, b: &RsvpFrame<'_>) -> bool {
        self.changed_area(a, b) >= FLASH_AREA_PX
    }
}

/// How long a word should show, given its scheduled `duration` and
/// whether the change to it counts toward a flash
/// ([`FlashModel::counts`]): at least [`SAFE_CHANGE_INTERVAL`] when it
/// does.
pub fn capped_duration(duration: Millis, counts: bool) -> Millis {
    if counts {
        duration.max(SAFE_CHANGE_INTERVAL)
    } else {
        duration
    }
}

/// The most flashes in any one second, given the times of the changes
/// that count, in order: a flash is a pair of changes, so the most
/// changes in any window of 1,000 ms, halved.
pub fn worst_second_flashes(times: &[Millis]) -> usize {
    let mut most = 0;
    let mut first = 0;
    for (i, &t) in times.iter().enumerate() {
        while t.saturating_sub(times[first]) >= 1000 {
            first += 1;
        }
        most = most.max(i + 1 - first);
    }
    most / 2
}

/// Grapheme positions of the word row, relative to the pivot, where `a`
/// and `b` differ (a blank against a glyph counts).
fn changed_word_cells(a: &RsvpFrame<'_>, b: &RsvpFrame<'_>) -> usize {
    let side = |x: &str, y: &str, reversed: bool| {
        let (gx, gy): (Vec<&str>, Vec<&str>) = if reversed {
            (
                x.graphemes(true).rev().collect(),
                y.graphemes(true).rev().collect(),
            )
        } else {
            (x.graphemes(true).collect(), y.graphemes(true).collect())
        };
        (0..gx.len().max(gy.len()))
            .filter(|&i| gx.get(i) != gy.get(i))
            .count()
    };
    side(a.before, b.before, true) + usize::from(a.pivot != b.pivot) + side(a.after, b.after, false)
}

/// The cells of a context row that may change: none when the word is the
/// same, else every grapheme of the longer of the two.
fn changed_context_cells(a: Option<&str>, b: Option<&str>) -> usize {
    if a == b {
        return 0;
    }
    let n = |s: Option<&str>| s.map_or(0, |s| s.graphemes(true).count());
    n(a).max(n(b))
}

#[cfg(test)]
mod tests {
    use super::super::{MAX_WPM, Rsvp, RsvpSettings, WordTrack};
    use super::*;
    use crate::rsvp::{Area, TuiBoxOptions, tui_box};

    /// The worst case for glyph changes: the shortest word and a long one
    /// in turn, then ordinary prose.
    fn worst_text() -> String {
        let mut s = "a incomprehensibilities ".repeat(60);
        s.push_str(
            "The reader shows one word at a time, and every word stays in place \
             so the eye does not move. Short words like a, I, or of alternate \
             with extraordinarily long ones in real text too.",
        );
        s
    }

    /// The fastest schedule: the highest rate, and no pauses or extra time
    /// for long words, so every word shows for the shortest time.
    fn fastest() -> RsvpSettings {
        RsvpSettings {
            wpm: MAX_WPM,
            clause_pause: 0,
            sentence_pause: 0,
            paragraph_pause: 0,
            long_word_step: 0,
            show_previous: true,
            show_next: true,
            ..RsvpSettings::default()
        }
    }

    /// Plays `text` through at `settings`, with each word's time capped
    /// when `cap` is set, and returns the times of the changes that count
    /// for `model`.
    fn counted_changes(
        text: &str,
        settings: RsvpSettings,
        model: &FlashModel,
        cap: bool,
    ) -> Vec<Millis> {
        let mut rsvp = Rsvp::new(WordTrack::from_text(text), settings);
        let mut now: Millis = 0;
        let mut times = Vec::new();
        for i in 1..rsvp.len() {
            rsvp.seek_word(i - 1, now);
            let (counts, duration) = {
                let a = rsvp.frame().unwrap();
                let a_owned = (a.before.to_owned(), a.pivot.to_owned(), a.after.to_owned());
                let (ap, an) = (a.previous.map(str::to_owned), a.next.map(str::to_owned));
                let duration = a.duration;
                rsvp.seek_word(i, now);
                let b = rsvp.frame().unwrap();
                let a = RsvpFrame {
                    before: &a_owned.0,
                    pivot: &a_owned.1,
                    after: &a_owned.2,
                    previous: ap.as_deref(),
                    next: an.as_deref(),
                    ..b
                };
                (model.counts(&a, &b), duration)
            };
            // Word i - 1 showed for its duration; then word i replaced it.
            let shown = if cap {
                let previous_counted = times.last().is_some_and(|&t| t == now);
                capped_duration(duration, previous_counted)
            } else {
                duration
            };
            now += shown;
            if counts {
                times.push(now);
            }
        }
        times
    }

    #[test]
    fn the_fastest_word_shows_for_40_ms() {
        let rsvp = Rsvp::new(WordTrack::from_text("a b c"), fastest());
        assert_eq!(rsvp.duration(0), 40);
    }

    #[test]
    fn the_terminal_box_never_flashes_at_the_highest_rate() {
        for (w, h) in [(8.0, 16.0), (10.0, 20.0), (12.0, 24.0), (16.0, 32.0)] {
            let model = FlashModel::terminal(w, h);
            let times = counted_changes(&worst_text(), fastest(), &model, false);
            assert!(
                worst_second_flashes(&times) <= MAX_FLASHES_PER_SECOND,
                "{w} by {h} pixel cells: {} flashes",
                worst_second_flashes(&times)
            );
        }
    }

    #[test]
    fn the_window_panel_never_flashes_at_the_highest_rate() {
        // textweaver-xilem's panel: a 40-pixel word and 18-pixel context
        // words, a character about six tenths of an em wide and the line
        // 1.2 em high.
        let model = FlashModel {
            word: GlyphBox::new(24.0, 48.0),
            context: GlyphBox::new(10.8, 21.6),
        };
        let times = counted_changes(&worst_text(), fastest(), &model, false);
        assert_eq!(worst_second_flashes(&times), 0);
    }

    #[test]
    fn huge_words_would_flash_and_the_cap_stops_it() {
        // A 200-point word at 96 dots per inch: 267 pixels high.
        let model = FlashModel {
            word: GlyphBox::new(160.0, 320.0),
            context: GlyphBox::new(10.8, 21.6),
        };
        let uncapped = counted_changes(&worst_text(), fastest(), &model, false);
        assert!(worst_second_flashes(&uncapped) > MAX_FLASHES_PER_SECOND);
        let capped = counted_changes(&worst_text(), fastest(), &model, true);
        assert!(!capped.is_empty());
        assert!(
            worst_second_flashes(&capped) <= MAX_FLASHES_PER_SECOND,
            "{}",
            worst_second_flashes(&capped)
        );
    }

    #[test]
    fn the_panel_never_blanks_or_moves_between_words() {
        let mut rsvp = Rsvp::new(WordTrack::from_text(&worst_text()), fastest());
        let area = Area::new(0, 0, 80, 24);
        let opts = TuiBoxOptions::default();
        let mut first = None;
        for i in 0..rsvp.len() {
            rsvp.seek_word(i, 0);
            let f = rsvp.frame().expect("a frame for every word");
            assert!(
                !f.text.is_empty() && !f.pivot.is_empty(),
                "word {i} is blank"
            );
            let b = tui_box(&f, area, &opts).expect("a box");
            let place = (b.area, b.pivot_col);
            assert_eq!(*first.get_or_insert(place), place, "word {i} moved the box");
        }
    }

    #[test]
    fn worst_second_counts_pairs_in_a_sliding_window() {
        assert_eq!(worst_second_flashes(&[]), 0);
        assert_eq!(worst_second_flashes(&[0, 100]), 1);
        // Six changes 167 ms apart fit in one second only as 5 intervals.
        let spaced: Vec<Millis> = (0..20).map(|i| i * SAFE_CHANGE_INTERVAL).collect();
        assert_eq!(worst_second_flashes(&spaced), 3);
        let fast: Vec<Millis> = (0..50).map(|i| i * 40).collect();
        assert_eq!(worst_second_flashes(&fast), 12);
    }
}
