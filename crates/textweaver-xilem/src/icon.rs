//! The window's icon (Wave 9, W9a-w): a plain lettermark, a lowercase
//! "t" on a square, drawn here so no image file ships. In Windows High
//! Contrast the icon is white on black with a white border, so it stays
//! clear whatever the system's colors.

/// The icon's size, in pixels (the title bar and the taskbar scale it).
pub const SIZE: u32 = 64;

/// The usual icon's square: a deep blue.
const SQUARE: [u8; 3] = [0x1f, 0x3a, 0x68];
/// The usual icon's letter: near white.
const LETTER: [u8; 3] = [0xf5, 0xf5, 0xf0];

/// The parts of the letter on a 32 by 32 grid: the stem, the crossbar,
/// and the foot turning right (left, top, right, bottom; right and bottom
/// excluded).
const STROKES: [(u32, u32, u32, u32); 3] = [(13, 5, 18, 26), (8, 10, 24, 15), (13, 22, 24, 26)];

/// The icon as RGBA pixels, `SIZE` by `SIZE`: the usual one, or the
/// high-contrast one.
pub fn rgba(high_contrast: bool) -> Vec<u8> {
    let (square, letter) = if high_contrast {
        ([0, 0, 0], [0xff, 0xff, 0xff])
    } else {
        (SQUARE, LETTER)
    };
    let scale = SIZE / 32;
    let border = if high_contrast { 2 * scale } else { 0 };
    let mut out = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (gx, gy) = (x / scale, y / scale);
            let in_letter = STROKES
                .iter()
                .any(|&(l, t, r, b)| gx >= l && gx < r && gy >= t && gy < b);
            let edge = x < border || y < border || x >= SIZE - border || y >= SIZE - border;
            let rgb = if in_letter || edge { letter } else { square };
            out.extend_from_slice(&rgb);
            out.push(0xff);
        }
    }
    out
}

/// The icon for winit, or `None` if winit refuses the pixels.
pub fn window_icon(high_contrast: bool) -> Option<masonry_winit::winit::window::Icon> {
    masonry_winit::winit::window::Icon::from_rgba(rgba(high_contrast), SIZE, SIZE).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(p: &[u8], x: u32, y: u32) -> [u8; 3] {
        let i = ((y * SIZE + x) * 4) as usize;
        [p[i], p[i + 1], p[i + 2]]
    }

    #[test]
    fn the_letter_stands_on_the_square() {
        for hc in [false, true] {
            let p = rgba(hc);
            assert_eq!(p.len(), (SIZE * SIZE * 4) as usize);
            // The stem's middle is the letter; a far corner inside the
            // border is the square.
            let letter = pixel(&p, 31, 36);
            let square = pixel(&p, 56, 56);
            assert_ne!(letter, square, "high contrast {hc}");
        }
        let hc = rgba(true);
        assert_eq!(pixel(&hc, 31, 36), [0xff, 0xff, 0xff]);
        assert_eq!(pixel(&hc, 56, 56), [0, 0, 0]);
        assert_eq!(pixel(&hc, 0, 0), [0xff, 0xff, 0xff], "the border");
        assert!(window_icon(false).is_some());
    }
}
