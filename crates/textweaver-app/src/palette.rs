//! The named highlight palette, `[[highlight.palette]]`.
//!
//! Up to eight entries, each a name the reader writes ("important",
//! "ask the professor"), a color and a shape. A highlight keeps the name
//! of the entry it was made with, so it is said by name ("Highlight,
//! important"), drawn by shape and color in both frontends, and written
//! with a typeform of its own in BRF: two entries never differ by color
//! alone. [`resolve`] is the one place a highlight becomes a name, a color
//! and a shape; the terminal reader, the window and the BRF export all ask
//! it, so they always agree.
//!
//! A highlight made before the palette has only a color (star's yellow,
//! green, cyan, pink and orange): it takes the first entry of that color,
//! so the starting palette gives every old highlight a name. A highlight
//! whose entry was renamed or removed falls back the same way.

use textweaver_lexicon::i18n::Catalog;
use textweaver_store::notes::color_name;
use textweaver_store::{Highlight, HighlightShape, PaletteEntry};
use textweaver_theme::Rgb;
use textweaver_theme::reading::parse_setting;

use crate::app::App;

/// How a highlight is drawn: its palette entry, its color and its shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteLook {
    /// Its entry's place in the palette, from 0 (Alt+1), or `None` for a
    /// highlight whose name and color are in no entry.
    pub entry: Option<usize>,
    /// Its color, when the color is one textweaver knows.
    pub color: Option<Rgb>,
    /// Its shape.
    pub shape: HighlightShape,
}

/// A highlight's look and the name said with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteMark {
    /// How it is drawn.
    pub look: PaletteLook,
    /// The name said with it: the entry's, else the highlight's own, else
    /// its color's.
    pub name: String,
}

/// A color setting's color: a known name or `#rrggbb`.
fn rgb(color: &str) -> Option<Rgb> {
    parse_setting(color).ok().flatten()
}

/// Entry `i` of `palette` as a mark.
pub fn entry_mark(palette: &[PaletteEntry], i: usize) -> Option<PaletteMark> {
    let e = palette.get(i)?;
    Some(PaletteMark {
        look: PaletteLook {
            entry: Some(i),
            color: rgb(&e.color),
            shape: e.shape,
        },
        name: e.name.trim().to_owned(),
    })
}

/// The entry named `name` (case and outer spaces ignored).
pub fn find_entry(palette: &[PaletteEntry], name: &str) -> Option<usize> {
    let name = name.trim();
    palette
        .iter()
        .position(|e| e.name.trim().eq_ignore_ascii_case(name))
}

/// What `h` is in `palette`: the entry of its name, else the first entry
/// of its color, else a mark of its own, with a shape no entry uses (so it
/// never differs from an entry by color alone while one is free).
pub fn resolve(palette: &[PaletteEntry], h: &Highlight) -> PaletteMark {
    let color = rgb(&h.color);
    let by_name = h.palette_name().and_then(|n| find_entry(palette, n));
    let by_color = || color.and_then(|c| palette.iter().position(|e| rgb(&e.color) == Some(c)));
    if let Some(m) = by_name
        .or_else(by_color)
        .and_then(|i| entry_mark(palette, i))
    {
        return m;
    }
    let shape = HighlightShape::ALL
        .into_iter()
        .find(|s| palette.iter().all(|e| e.shape != *s))
        .unwrap_or_default();
    PaletteMark {
        look: PaletteLook {
            entry: None,
            color,
            shape,
        },
        name: h
            .palette_name()
            .map_or_else(|| color_name(&h.color), str::to_owned),
    }
}

/// The catalog message for `shape`'s name in words ("double underline").
pub fn shape_message(shape: HighlightShape) -> &'static str {
    match shape {
        HighlightShape::Underline => "palette-shape-underline",
        HighlightShape::DoubleUnderline => "palette-shape-double-underline",
        HighlightShape::Bold => "palette-shape-bold",
        HighlightShape::Dotted => "palette-shape-dotted",
        HighlightShape::Brackets => "palette-shape-brackets",
        HighlightShape::Symbol => "palette-shape-symbol",
    }
}

/// A palette color in words: the catalog's name for a named color
/// ("sky blue"), else the name or `#rrggbb` as written.
pub fn color_words(c: &Catalog, color: &str) -> String {
    let key: String = color
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    let id = format!("color-name-{key}");
    if c.has(&id) {
        c.tr(&id)
    } else {
        color_name(color)
    }
}

/// A palette name made safe for a file name: lowercase letters and
/// digits, words joined by `-` ("ask the professor" gives
/// `ask-the-professor`), or `highlights` when nothing is left.
pub fn file_part(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_owned();
    if out.is_empty() {
        "highlights".to_owned()
    } else {
        out
    }
}

impl App {
    /// The highlight palette in effect.
    pub fn highlight_palette(&self) -> &[PaletteEntry] {
        &self.settings.highlight.palette
    }

    /// What highlight `h` is in the palette in effect ([`resolve`]).
    pub fn highlight_mark(&self, h: &Highlight) -> PaletteMark {
        resolve(self.highlight_palette(), h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_store::default_palette;

    fn highlight(color: &str, name: Option<&str>) -> Highlight {
        let mut h = Highlight {
            color: color.into(),
            ..Highlight::default()
        };
        if let Some(n) = name {
            h.set_palette_entry(n, color);
        }
        h
    }

    /// The one resolver both frontends and BRF use: by name, then by
    /// color for highlights made before the palette, then a mark of its
    /// own; two entries of one color still read apart by name and shape.
    #[test]
    fn highlights_resolve_to_one_name_color_and_shape() {
        let mut palette = default_palette();
        // Old highlights (star's colors) take the entry of their color.
        let old = resolve(&palette, &highlight("#ffff00", None));
        assert_eq!(old.name, "important");
        assert_eq!(old.look.entry, Some(0));
        assert_eq!(old.look.shape, HighlightShape::Underline);
        assert_eq!(old.look.color, Some(Rgb::from_u32(0xffff00)));
        assert_eq!(
            resolve(&palette, &highlight("#ffa500", None)).name,
            "review"
        );

        // Two entries of one color: the name decides, and the shapes differ.
        palette[1].color = "yellow".into();
        let define = resolve(&palette, &highlight("yellow", Some("Define")));
        assert_eq!(define.name, "define");
        assert_eq!(define.look.entry, Some(1));
        assert_eq!(define.look.color, old.look.color);
        assert_ne!(define.look.shape, old.look.shape);

        // A renamed entry: the highlight falls back to its color.
        let gone = resolve(&palette, &highlight("#ffc0cb", Some("ask the professor")));
        assert_eq!(gone.name, "example");

        // Off the palette: its own name or its color's, and a free shape.
        let off = resolve(&palette, &highlight("#123456", None));
        assert_eq!(off.look.entry, None);
        assert_eq!(off.name, "#123456");
        assert_eq!(off.look.shape, HighlightShape::Symbol);
        assert!(palette.iter().all(|e| e.shape != off.look.shape));
        assert_eq!(find_entry(&palette, " QUESTION "), Some(2));
        assert_eq!(file_part("Ask the Professor!"), "ask-the-professor");
        assert_eq!(file_part("??"), "highlights");
        assert_eq!(entry_mark(&palette, 9), None);
    }
}
