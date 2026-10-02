//! The system's own colors when a high contrast mode is on (Windows High
//! Contrast, "Contrast themes" in Windows 11).
//!
//! In that mode the person has chosen every color on the screen, and an
//! application should draw with them, not with its own theme: the window
//! background and text, the highlight and its text, links, and buttons.
//! [`high_contrast`] reads them (two cheap system calls, done on every
//! tick so a change applies at once), and [`palette`] turns them into the
//! window's palette, where every mark that had a tint of its own keeps its
//! shape instead (the focus ring, the ruler's bar, the boxes and lines of
//! notes and matches), since a high contrast scheme has no tints to spare.
//!
//! On macOS and Linux the system's high contrast setting chooses
//! textweaver's own high-contrast theme at startup (`theme::os::probe`), as
//! in the terminal.

use textweaver_theme::{Attrs, Rgb, ThemeKind};

use crate::theme::Palette;

/// The colors of the system's high contrast scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemColors {
    /// The window's background.
    pub window: Rgb,
    /// Text on it.
    pub window_text: Rgb,
    /// Selected items and the focus.
    pub highlight: Rgb,
    /// Text on the highlight.
    pub highlight_text: Rgb,
    /// Links.
    pub hot_light: Rgb,
    /// Buttons' face.
    pub button_face: Rgb,
    /// Buttons' text.
    pub button_text: Rgb,
    /// Disabled text.
    pub gray_text: Rgb,
}

/// Windows' "Night sky" contrast theme, near enough: a black page, white
/// text, a yellow highlight, and cyan links. For the review screenshots
/// (`--review-screenshots`) and the tests; the window reads the real ones.
pub const NIGHT_SKY: SystemColors = SystemColors {
    window: Rgb { r: 0, g: 0, b: 0 },
    window_text: Rgb {
        r: 255,
        g: 255,
        b: 255,
    },
    highlight: Rgb {
        r: 255,
        g: 255,
        b: 0,
    },
    highlight_text: Rgb { r: 0, g: 0, b: 0 },
    hot_light: Rgb {
        r: 0,
        g: 255,
        b: 255,
    },
    button_face: Rgb { r: 0, g: 0, b: 0 },
    button_text: Rgb {
        r: 255,
        g: 255,
        b: 255,
    },
    gray_text: Rgb {
        r: 63,
        g: 242,
        b: 63,
    },
};

/// The name the review screenshots give [`NIGHT_SKY`]'s palette.
pub const NIGHT_SKY_NAME: &str = "system-night-sky";

/// The system's high contrast colors, or `None` when no high contrast mode
/// is on (and on systems other than Windows).
pub fn high_contrast() -> Option<SystemColors> {
    #[cfg(windows)]
    {
        imp::high_contrast()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The window's palette drawn only with the system's colors.
pub fn palette(c: &SystemColors) -> Palette {
    let text = c.window_text;
    let bg = c.window;
    // The focus ring is the highlight, when it can be seen on the page;
    // else the text color.
    let focus = if contrast(c.highlight, bg) >= 3.0 {
        c.highlight
    } else {
        text
    };
    Palette {
        name: "system-high-contrast".into(),
        kind: ThemeKind::HighContrast,
        background: bg,
        surface: bg,
        raised: c.button_face,
        border: text,
        border_hover: c.highlight,
        text,
        dim_text: text,
        headings: [text; 6],
        link: c.hot_light,
        code: text,
        code_background: bg,
        quote: text,
        error: text,
        focus,
        accent: c.highlight,
        on_accent: c.highlight_text,
        spoken_word: (c.highlight_text, c.highlight),
        spoken_word_attrs: Attrs::BOLD,
        // No band under forced colors: the underline, in the system's text
        // color, is the sentence's only mark.
        spoken_sentence: bg,
        spoken_sentence_attrs: Attrs::UNDERLINE,
        sentence_line: text,
        selection: (c.highlight_text, c.highlight),
        // The system's own band, as its edit controls draw it.
        field_selection: c.highlight,
        find_hit: (text, bg),
        current_find_hit: bg,
        note: bg,
        bookmark: bg,
        user_highlight: bg,
        caret: text,
        ruler_focus: bg,
        ruler_band: bg,
    }
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    textweaver_theme::color::contrast_ratio(a, b)
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    use super::SystemColors;
    use textweaver_theme::Rgb;
    use windows::Win32::Graphics::Gdi::{
        COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT,
        COLOR_HOTLIGHT, COLOR_WINDOW, COLOR_WINDOWTEXT, GetSysColor, SYS_COLOR_INDEX,
    };
    use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };

    fn color(index: SYS_COLOR_INDEX) -> Rgb {
        // SAFETY: GetSysColor reads a system color by index and has no
        // other effect; every index passed is a named constant.
        let bgr = unsafe { GetSysColor(index) };
        Rgb {
            r: (bgr & 0xff) as u8,
            g: ((bgr >> 8) & 0xff) as u8,
            b: ((bgr >> 16) & 0xff) as u8,
        }
    }

    pub(super) fn high_contrast() -> Option<SystemColors> {
        let mut hc = HIGHCONTRASTW {
            cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
            ..Default::default()
        };
        // SAFETY: SPI_GETHIGHCONTRAST fills the HIGHCONTRASTW it is given,
        // whose size is set in `cbSize` as the call requires; the pointer
        // is to a live local for the duration of the call, and nothing is
        // written to the user's settings (no update flags).
        let read = unsafe {
            SystemParametersInfoW(
                SPI_GETHIGHCONTRAST,
                hc.cbSize,
                Some(std::ptr::from_mut(&mut hc).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        if read.is_err() || hc.dwFlags.0 & HCF_HIGHCONTRASTON.0 == 0 {
            return None;
        }
        Some(SystemColors {
            window: color(COLOR_WINDOW),
            window_text: color(COLOR_WINDOWTEXT),
            highlight: color(COLOR_HIGHLIGHT),
            highlight_text: color(COLOR_HIGHLIGHTTEXT),
            hot_light: color(COLOR_HOTLIGHT),
            button_face: color(COLOR_BTNFACE),
            button_text: color(COLOR_BTNTEXT),
            gray_text: color(COLOR_GRAYTEXT),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    fn night() -> SystemColors {
        NIGHT_SKY
    }

    /// The palette uses only the system's colors, the focus ring can be
    /// seen, and the text reads on the page and the selection.
    #[test]
    fn a_high_contrast_palette_is_the_systems_colors() {
        let c = night();
        let p = palette(&c);
        assert_eq!(p.kind, ThemeKind::HighContrast);
        assert_eq!(p.background, c.window);
        assert_eq!(p.text, c.window_text);
        assert_eq!(p.link, c.hot_light);
        assert_eq!(p.selection, (c.highlight_text, c.highlight));
        assert!(contrast(p.focus, p.background) >= 3.0);
        assert!(contrast(p.text, p.background) >= 7.0);
        let system = [
            c.window,
            c.window_text,
            c.highlight,
            c.highlight_text,
            c.hot_light,
            c.button_face,
            c.button_text,
        ];
        for (name, color) in [
            ("surface", p.surface),
            ("raised", p.raised),
            ("border", p.border),
            ("focus", p.focus),
            ("caret", p.caret),
            ("ruler", p.ruler_focus),
        ] {
            assert!(system.contains(&color), "{name}: {color:?}");
        }
    }

    /// A highlight too close to the page for a focus ring: the ring is
    /// drawn in the text color instead.
    #[test]
    fn the_focus_ring_stays_visible() {
        let mut c = night();
        c.highlight = rgb(10, 10, 10);
        let p = palette(&c);
        assert_eq!(p.focus, c.window_text);
    }

    /// Off Windows, the setting is read at startup through the theme probe
    /// instead; here the call answers, without panicking.
    #[test]
    fn reading_the_setting_answers() {
        let _ = high_contrast();
    }
}
