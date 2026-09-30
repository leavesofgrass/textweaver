//! Which way up a page image is: sideways and upside-down scans are
//! detected and turned before they are read (ADR-0048).
//!
//! The engines read upright text only. A page scanned sideways (a book laid
//! across the scanner, a landscape handout fed the wrong way) gave nothing
//! but noise before this check. It is measured on the picture itself, with
//! no engine and no model, in a few milliseconds:
//!
//! 1. **Which axis the lines run along.** The image is shrunk and made
//!    black and white. Lines of text leave empty rows between them, so the
//!    row profile (ink per row) of horizontal text has many gaps, while its
//!    column profile has almost none; sideways text is the other way round.
//! 2. **Which way is up.** With the lines horizontal, two cues vote. Latin
//!    letters have more ink above the middle band of a line (capitals,
//!    ascenders, the dots of i and j) than below it (the descenders of g, p,
//!    q, y), so upside-down text has more ink below. And most text starts at
//!    a common left margin with ragged line ends, so upside down, the ends
//!    line up instead.
//!
//! [`detect`] reports how sure each step is. When the axis is clear but up
//! and down are not, `crate::recognize` reads both candidate turns and keeps
//! the reading with more real words; when the axis is unclear (a photograph,
//! a nearly empty page), the image is read as it is.

use crate::image::GrayImage;

/// The longer side the image is shrunk to before measuring.
const WORK_SIDE: u32 = 1_200;

/// What [`detect`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// Upright, or no clear sign otherwise: read as it is.
    Upright,
    /// Sure: turn the image this many quarter turns clockwise (1, 2, or 3)
    /// to make it upright.
    Turn(u8),
    /// The lines run up and down, but which way is not clear: turn it 1 or
    /// 3 quarter turns, and read both.
    Sideways,
}

impl Orientation {
    /// The quarter turns clockwise to try, most likely first.
    pub fn candidates(self) -> &'static [u8] {
        match self {
            Orientation::Upright => &[0],
            Orientation::Turn(1) => &[1],
            Orientation::Turn(2) => &[2],
            Orientation::Turn(3) => &[3],
            Orientation::Turn(_) => &[0],
            Orientation::Sideways => &[1, 3],
        }
    }
}

/// Ink as a black and white grid.
struct Ink {
    width: usize,
    height: usize,
    /// Row-major, true for ink.
    data: Vec<bool>,
}

impl Ink {
    fn of(image: &GrayImage) -> Option<Ink> {
        let small = image.fit_within(WORK_SIDE);
        let t = otsu(&small.data)?;
        let data: Vec<bool> = small.data.iter().map(|&v| v < t).collect();
        let inked = data.iter().filter(|&&b| b).count();
        // Nearly blank, or nearly all dark (a photograph, a dark cover).
        let frac = inked as f64 / data.len().max(1) as f64;
        if !(0.002..=0.4).contains(&frac) {
            return None;
        }
        Some(Ink {
            width: small.width as usize,
            height: small.height as usize,
            data,
        })
    }

    /// Ink per row.
    fn rows(&self) -> Vec<u32> {
        self.data
            .chunks_exact(self.width)
            .map(|r| r.iter().filter(|&&b| b).count() as u32)
            .collect()
    }

    /// Ink per column.
    fn columns(&self) -> Vec<u32> {
        let mut out = vec![0u32; self.width];
        for row in self.data.chunks_exact(self.width) {
            for (c, &b) in out.iter_mut().zip(row) {
                *c += u32::from(b);
            }
        }
        out
    }

    /// The grid turned a quarter clockwise.
    fn turned(&self) -> Ink {
        let (w, h) = (self.width, self.height);
        let mut data = vec![false; w * h];
        for y in 0..h {
            for x in 0..w {
                data[x * h + (h - 1 - y)] = self.data[y * w + x];
            }
        }
        Ink {
            width: h,
            height: w,
            data,
        }
    }
}

/// Otsu's threshold over the image's histogram; `None` for a flat image.
fn otsu(data: &[u8]) -> Option<u8> {
    let mut hist = [0u64; 256];
    for &v in data {
        hist[usize::from(v)] += 1;
    }
    let total = data.len() as f64;
    let sum: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &n)| i as f64 * n as f64)
        .sum();
    let (mut w0, mut sum0, mut best, mut at) = (0.0, 0.0, 0.0, None);
    for (t, &n) in hist.iter().enumerate() {
        w0 += n as f64;
        if w0 == 0.0 {
            continue;
        }
        let w1 = total - w0;
        if w1 == 0.0 {
            break;
        }
        sum0 += t as f64 * n as f64;
        let m0 = sum0 / w0;
        let m1 = (sum - sum0) / w1;
        let between = w0 * w1 * (m0 - m1) * (m0 - m1);
        if between > best {
            best = between;
            at = Some(t);
        }
    }
    // Pixels at or below the class boundary are ink.
    at.map(|t| u8::try_from(t + 1).unwrap_or(u8::MAX))
}

/// The inked extent of a profile, trimmed of its outer 1% of ink.
fn extent(p: &[u32]) -> Option<(usize, usize)> {
    let total: u64 = p.iter().map(|&v| u64::from(v)).sum();
    if total == 0 {
        return None;
    }
    let cut = total / 100;
    let mut acc = 0u64;
    let lo = p.iter().position(|&v| {
        acc += u64::from(v);
        acc > cut
    })?;
    acc = 0;
    let hi = p.len()
        - 1
        - p.iter().rev().position(|&v| {
            acc += u64::from(v);
            acc > cut
        })?;
    (hi > lo).then_some((lo, hi))
}

/// The share of a profile's bins, within its inked extent, that are nearly
/// empty: high across lines of text (the gaps between lines), low along
/// them.
fn gaps(p: &[u32]) -> f64 {
    let Some((lo, hi)) = extent(p) else {
        return 0.0;
    };
    let span = &p[lo..=hi];
    let mean = span.iter().map(|&v| f64::from(v)).sum::<f64>() / span.len() as f64;
    let low = 0.15 * mean;
    span.iter().filter(|&&v| f64::from(v) <= low).count() as f64 / span.len() as f64
}

/// Lines of text in a grid whose lines run across: row ranges, top to
/// bottom.
fn text_lines(rows: &[u32]) -> Vec<(usize, usize)> {
    let Some((lo, hi)) = extent(rows) else {
        return Vec::new();
    };
    let span = &rows[lo..=hi];
    let mean = span.iter().map(|&v| f64::from(v)).sum::<f64>() / span.len() as f64;
    let on = |v: u32| f64::from(v) > 0.15 * mean;
    let mut out = Vec::new();
    let mut y = lo;
    while y <= hi {
        if on(rows[y]) {
            let start = y;
            while y <= hi && on(rows[y]) {
                y += 1;
            }
            if y - start >= 4 {
                out.push((start, y - 1));
            }
        } else {
            y += 1;
        }
    }
    out
}

/// A vote on whether horizontal text is upright: positive for upright,
/// negative for upside down, near 0 for no sign.
fn upright_vote(ink: &Ink) -> f64 {
    let rows = ink.rows();
    let lines = text_lines(&rows);
    if lines.len() < 2 {
        return 0.0;
    }
    // Ink above and below each line's middle band.
    let (mut above, mut below) = (0u64, 0u64);
    // Where each line starts and ends.
    let mut starts = Vec::with_capacity(lines.len());
    let mut ends = Vec::with_capacity(lines.len());
    for &(top, bottom) in &lines {
        let line = &rows[top..=bottom];
        let peak = line.iter().copied().max().unwrap_or(0);
        let core = |v: u32| v * 2 >= peak;
        let Some(c0) = line.iter().position(|&v| core(v)) else {
            continue;
        };
        let c1 = line.len() - 1 - line.iter().rev().position(|&v| core(v)).unwrap_or(0);
        above += line[..c0].iter().map(|&v| u64::from(v)).sum::<u64>();
        below += line[c1 + 1..].iter().map(|&v| u64::from(v)).sum::<u64>();
        let mut x0 = usize::MAX;
        let mut x1 = 0;
        for y in top..=bottom {
            let row = &ink.data[y * ink.width..(y + 1) * ink.width];
            if let Some(a) = row.iter().position(|&b| b) {
                x0 = x0.min(a);
                x1 = x1.max(ink.width - 1 - row.iter().rev().position(|&b| b).unwrap_or(0));
            }
        }
        if x0 != usize::MAX {
            starts.push(x0 as f64);
            ends.push(x1 as f64);
        }
    }
    let total = (above + below) as f64;
    let shape = if total > 0.0 {
        // From -1 (all below) to 1 (all above).
        (above as f64 - below as f64) / total
    } else {
        0.0
    };
    // Lines that start together and end raggedly read left to right; a
    // column of text turned over ends together instead. The share of lines
    // within a few pixels of the leftmost start, less the share at the
    // rightmost end.
    let min_start = starts.iter().copied().fold(f64::MAX, f64::min);
    let max_end = ends.iter().copied().fold(f64::MIN, f64::max);
    let at_left = starts.iter().filter(|&&x| x - min_start <= 4.0).count() as f64;
    let at_right = ends.iter().filter(|&&x| max_end - x <= 4.0).count() as f64;
    let n = starts.len().max(1) as f64;
    let align = (at_left - at_right) / n;
    // The ink shape is the stronger cue; the margin breaks ties.
    shape * 2.0 + align * 0.5
}

/// Which way up `image` is.
pub fn detect(image: &GrayImage) -> Orientation {
    let Some(ink) = Ink::of(image) else {
        return Orientation::Upright;
    };
    let across = gaps(&ink.rows());
    let down = gaps(&ink.columns());
    let horizontal = across >= 0.12 && across > 1.6 * down;
    let vertical = down >= 0.12 && down > 1.6 * across;
    if horizontal {
        let v = upright_vote(&ink);
        if v < -0.25 {
            Orientation::Turn(2)
        } else {
            Orientation::Upright
        }
    } else if vertical {
        let v = upright_vote(&ink.turned());
        if v > 0.25 {
            Orientation::Turn(1)
        } else if v < -0.25 {
            Orientation::Turn(3)
        } else {
            Orientation::Sideways
        }
    } else {
        Orientation::Upright
    }
}

/// How much a reading looks like real words: the share of its letters that
/// are in words of two or more letters with a vowel. Readings of a page the
/// wrong way up are mostly short fragments without vowels.
pub fn word_score(page: &crate::OcrPage) -> f64 {
    let (mut good, mut all) = (0usize, 0usize);
    for w in page.lines.iter().flat_map(|l| &l.words) {
        let letters: Vec<char> = w.text.chars().filter(|c| c.is_alphabetic()).collect();
        all += w.text.chars().filter(|c| !c.is_whitespace()).count();
        let vowel = letters
            .iter()
            .any(|c| "aeiouyAEIOUY".contains(*c) || !c.is_ascii());
        if letters.len() >= 2 && vowel && letters.len() * 3 >= w.text.chars().count() * 2 {
            good += letters.len();
        }
    }
    if all == 0 {
        0.0
    } else {
        good as f64 / all as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page of fake text: lines of "letters" with ascenders more often
    /// than descenders, starting at a left margin, ragged at the right.
    fn page() -> GrayImage {
        let (w, h) = (600usize, 800usize);
        let mut data = vec![255u8; w * h];
        let mut seed = 7u32;
        let mut rand = move || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 16) % 100
        };
        let mut y = 60;
        let mut line = 0;
        while y + 30 < h - 60 {
            let end = 520 - (rand() as usize % 5) * 40 * usize::from(line % 3 != 0);
            let mut x = 50;
            while x + 8 < end {
                // A letter: a middle band of 10 rows, sometimes an ascender
                // (6 rows above) or, less often, a descender (6 rows below).
                let r = rand();
                let (top, bottom) = if r < 45 {
                    (y - 6, y + 10)
                } else if r < 60 {
                    (y, y + 16)
                } else {
                    (y, y + 10)
                };
                // Letters of different widths, so no column is empty in
                // every line.
                let width = 3 + rand() as usize % 6;
                for yy in top..bottom {
                    for xx in x..x + width {
                        data[yy * w + xx] = 0;
                    }
                }
                x += width + 1 + rand() as usize % 3;
                if rand() < 18 {
                    x += 7;
                }
            }
            y += 30;
            line += 1;
        }
        GrayImage::new(w as u32, h as u32, data).unwrap()
    }

    #[test]
    fn every_turn_is_found() {
        let upright = page();
        assert_eq!(detect(&upright), Orientation::Upright);
        assert_eq!(detect(&upright.rotate(1)), Orientation::Turn(3));
        assert_eq!(detect(&upright.rotate(2)), Orientation::Turn(2));
        assert_eq!(detect(&upright.rotate(3)), Orientation::Turn(1));
    }

    #[test]
    fn scanned_pages_are_found_every_way_round() {
        // The W3d scans: a tilted, noisy greyscale JPEG page and a 1-bit
        // fax page, as office scanners write them.
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/w3d/scan-en.pdf");
        let bytes = std::fs::read(path).unwrap();
        let pdf = crate::pdf::PdfPages::open(bytes).unwrap();
        for index in 0..pdf.len() {
            let (upright, _) = pdf.page_image(index).unwrap();
            for turns in 0..4u8 {
                let want = match (4 - turns) % 4 {
                    0 => Orientation::Upright,
                    t => Orientation::Turn(t),
                };
                assert_eq!(
                    detect(&upright.rotate(turns)),
                    want,
                    "page {} turned {turns}",
                    index + 1
                );
            }
        }
    }

    #[test]
    fn blank_and_dark_pages_are_left_alone() {
        let white = GrayImage::new(300, 400, vec![255; 300 * 400]).unwrap();
        assert_eq!(detect(&white), Orientation::Upright);
        let black = GrayImage::new(300, 400, vec![0; 300 * 400]).unwrap();
        assert_eq!(detect(&black), Orientation::Upright);
        // Noise has no lines either way.
        let mut seed = 1u32;
        let noise: Vec<u8> = (0..300 * 400)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                if (seed >> 16).is_multiple_of(10) { 0 } else { 255 }
            })
            .collect();
        let noise = GrayImage::new(300, 400, noise).unwrap();
        assert_eq!(detect(&noise), Orientation::Upright);
    }

    #[test]
    fn candidates_follow_the_guess() {
        assert_eq!(Orientation::Upright.candidates(), &[0]);
        assert_eq!(Orientation::Turn(3).candidates(), &[3]);
        assert_eq!(Orientation::Sideways.candidates(), &[1, 3]);
    }

    #[test]
    fn real_words_score_higher_than_fragments() {
        let page = |words: &[&str]| {
            let mut line = crate::OcrLine::default();
            for w in words {
                line.push(crate::OcrWord {
                    text: (*w).to_owned(),
                    rect: crate::Rect::default(),
                    confidence: None,
                });
            }
            crate::OcrPage {
                width: 1,
                height: 1,
                lines: vec![line],
                turned: 0,
            }
        };
        let good = word_score(&page(&["Reading", "scanned", "pages", "aloud"]));
        let bad = word_score(&page(&["pnoy", "l", "'1M", "p;,", "sp"]));
        assert!(good > 0.9, "{good}");
        assert!(bad < good / 2.0, "{bad}");
    }
}
