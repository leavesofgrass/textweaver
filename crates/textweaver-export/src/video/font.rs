//! Just enough of a TrueType reader to draw the bundled fonts: the
//! character map (formats 4 and 12), advance widths, vertical metrics, and
//! `glyf` outlines (simple and composite) as paths in font units.
//!
//! No shaping and no kerning: the video shows plain running text, one
//! glyph per character, which the bundled Latin fonts draw well.

use std::collections::HashMap;

use hayro::vello_cpu::kurbo::{Affine, BezPath, Point};

/// A font file that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontError(pub String);

fn bad(what: &str) -> FontError {
    FontError(format!("the font's {what} could not be read"))
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(o)?, *d.get(o + 1)?]))
}

fn i16_at(d: &[u8], o: usize) -> Option<i16> {
    u16_at(d, o).map(|v| v as i16)
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *d.get(o)?,
        *d.get(o + 1)?,
        *d.get(o + 2)?,
        *d.get(o + 3)?,
    ]))
}

/// How characters map to glyphs.
enum Cmap<'a> {
    /// Format 4: segments of the Basic Multilingual Plane.
    Segments(&'a [u8]),
    /// Format 12: groups over all planes.
    Groups(&'a [u8]),
}

/// One parsed font face.
pub struct Font<'a> {
    units_per_em: f64,
    ascender: f64,
    descender: f64,
    line_gap: f64,
    num_glyphs: u16,
    num_hmetrics: u16,
    hmtx: &'a [u8],
    loca: Vec<u32>,
    glyf: &'a [u8],
    cmap: Cmap<'a>,
    outlines: HashMap<u16, BezPath>,
}

impl<'a> Font<'a> {
    /// Reads the tables the renderer needs.
    pub fn parse(data: &'a [u8]) -> Result<Self, FontError> {
        let num_tables = u16_at(data, 4).ok_or_else(|| bad("table directory"))?;
        let table = |tag: &[u8; 4]| -> Option<&'a [u8]> {
            (0..usize::from(num_tables)).find_map(|i| {
                let rec = 12 + 16 * i;
                if data.get(rec..rec + 4)? != tag {
                    return None;
                }
                let off = u32_at(data, rec + 8)? as usize;
                let len = u32_at(data, rec + 12)? as usize;
                data.get(off..off.checked_add(len)?)
            })
        };
        let head = table(b"head").ok_or_else(|| bad("head table"))?;
        let hhea = table(b"hhea").ok_or_else(|| bad("hhea table"))?;
        let maxp = table(b"maxp").ok_or_else(|| bad("maxp table"))?;
        let hmtx = table(b"hmtx").ok_or_else(|| bad("hmtx table"))?;
        let loca_t = table(b"loca").ok_or_else(|| bad("loca table"))?;
        let glyf = table(b"glyf").ok_or_else(|| bad("glyf table (outlines)"))?;
        let cmap_t = table(b"cmap").ok_or_else(|| bad("cmap table"))?;
        let units_per_em = f64::from(u16_at(head, 18).ok_or_else(|| bad("head table"))?);
        let long_loca = i16_at(head, 50).ok_or_else(|| bad("head table"))? != 0;
        let num_glyphs = u16_at(maxp, 4).ok_or_else(|| bad("maxp table"))?;
        let ascender = f64::from(i16_at(hhea, 4).ok_or_else(|| bad("hhea table"))?);
        let descender = f64::from(i16_at(hhea, 6).ok_or_else(|| bad("hhea table"))?);
        let line_gap = f64::from(i16_at(hhea, 8).ok_or_else(|| bad("hhea table"))?);
        let num_hmetrics = u16_at(hhea, 34).ok_or_else(|| bad("hhea table"))?;
        let loca = (0..=usize::from(num_glyphs))
            .map(|i| {
                if long_loca {
                    u32_at(loca_t, 4 * i)
                } else {
                    u16_at(loca_t, 2 * i).map(|v| u32::from(v) * 2)
                }
            })
            .collect::<Option<Vec<u32>>>()
            .ok_or_else(|| bad("loca table"))?;
        let cmap = Self::pick_cmap(cmap_t).ok_or_else(|| bad("character map"))?;
        if units_per_em <= 0.0 {
            return Err(bad("head table"));
        }
        Ok(Font {
            units_per_em,
            ascender,
            descender,
            line_gap,
            num_glyphs,
            num_hmetrics,
            hmtx,
            loca,
            glyf,
            cmap,
            outlines: HashMap::new(),
        })
    }

    fn pick_cmap(cmap: &'a [u8]) -> Option<Cmap<'a>> {
        let n = u16_at(cmap, 2)?;
        let mut seg = None;
        for i in 0..usize::from(n) {
            let rec = 4 + 8 * i;
            let platform = u16_at(cmap, rec)?;
            let encoding = u16_at(cmap, rec + 2)?;
            let off = u32_at(cmap, rec + 4)? as usize;
            let sub = cmap.get(off..)?;
            let unicode = platform == 0 || (platform == 3 && (encoding == 1 || encoding == 10));
            if !unicode {
                continue;
            }
            match u16_at(sub, 0)? {
                12 => return Some(Cmap::Groups(sub)),
                4 if seg.is_none() => seg = Some(Cmap::Segments(sub)),
                _ => {}
            }
        }
        seg
    }

    /// Pixels per font unit at `size_px`.
    pub fn scale(&self, size_px: f64) -> f64 {
        size_px / self.units_per_em
    }

    /// Ascender, in font units (positive, above the baseline).
    pub fn ascender(&self) -> f64 {
        self.ascender
    }

    /// Distance from one baseline to the next, in font units.
    pub fn line_height(&self) -> f64 {
        self.ascender - self.descender + self.line_gap
    }

    /// The glyph for a character (0, the missing glyph, when there is none).
    pub fn glyph(&self, c: char) -> u16 {
        let c = u32::from(c);
        let found = match self.cmap {
            Cmap::Groups(sub) => Self::lookup_groups(sub, c),
            Cmap::Segments(sub) => u16::try_from(c)
                .ok()
                .and_then(|c| Self::lookup_segments(sub, c)),
        };
        found.filter(|g| *g < self.num_glyphs).unwrap_or(0)
    }

    fn lookup_groups(sub: &[u8], c: u32) -> Option<u16> {
        let n = u32_at(sub, 12)? as usize;
        (0..n).find_map(|i| {
            let g = 16 + 12 * i;
            let start = u32_at(sub, g)?;
            let end = u32_at(sub, g + 4)?;
            (start <= c && c <= end)
                .then(|| u32_at(sub, g + 8).map(|first| first + (c - start)))
                .flatten()
                .and_then(|v| u16::try_from(v).ok())
        })
    }

    fn lookup_segments(sub: &[u8], c: u16) -> Option<u16> {
        let seg_x2 = usize::from(u16_at(sub, 6)?);
        let ends = 14;
        let starts = ends + seg_x2 + 2;
        let deltas = starts + seg_x2;
        let ranges = deltas + seg_x2;
        for i in 0..seg_x2 / 2 {
            let end = u16_at(sub, ends + 2 * i)?;
            if end < c {
                continue;
            }
            let start = u16_at(sub, starts + 2 * i)?;
            if start > c {
                return None;
            }
            let delta = u16_at(sub, deltas + 2 * i)?;
            let ro = usize::from(u16_at(sub, ranges + 2 * i)?);
            if ro == 0 {
                return Some(c.wrapping_add(delta));
            }
            let at = ranges + 2 * i + ro + 2 * usize::from(c - start);
            let g = u16_at(sub, at)?;
            return Some(if g == 0 { 0 } else { g.wrapping_add(delta) });
        }
        None
    }

    /// The glyph's advance width, in font units.
    pub fn advance(&self, glyph: u16) -> f64 {
        let i = usize::from(glyph.min(self.num_hmetrics.saturating_sub(1)));
        f64::from(u16_at(self.hmtx, 4 * i).unwrap_or(0))
    }

    /// The width of `text` in font units.
    pub fn width(&self, text: &str) -> f64 {
        text.chars().map(|c| self.advance(self.glyph(c))).sum()
    }

    /// The glyph's outline in font units (y up), read once and kept.
    pub fn outline(&mut self, glyph: u16) -> &BezPath {
        if !self.outlines.contains_key(&glyph) {
            let mut path = BezPath::new();
            self.append_outline(glyph, Affine::IDENTITY, 0, &mut path);
            self.outlines.insert(glyph, path);
        }
        &self.outlines[&glyph]
    }

    fn glyph_data(&self, glyph: u16) -> Option<&'a [u8]> {
        let i = usize::from(glyph);
        let start = *self.loca.get(i)? as usize;
        let end = *self.loca.get(i + 1)? as usize;
        if end <= start {
            return None;
        }
        self.glyf.get(start..end)
    }

    fn append_outline(&self, glyph: u16, at: Affine, depth: u8, out: &mut BezPath) {
        let Some(d) = self.glyph_data(glyph) else {
            return;
        };
        let Some(contours) = i16_at(d, 0) else {
            return;
        };
        if contours >= 0 {
            let _ = simple(d, usize::from(contours as u16), at, out);
        } else if depth < 8 {
            let _ = self.composite(d, at, depth, out);
        }
    }

    fn composite(&self, d: &[u8], at: Affine, depth: u8, out: &mut BezPath) -> Option<()> {
        const WORDS: u16 = 0x0001;
        const XY_VALUES: u16 = 0x0002;
        const SCALE: u16 = 0x0008;
        const MORE: u16 = 0x0020;
        const XY_SCALE: u16 = 0x0040;
        const TWO_BY_TWO: u16 = 0x0080;
        let f2dot14 = |o: usize| i16_at(d, o).map(|v| f64::from(v) / 16384.0);
        let mut o = 10;
        loop {
            let flags = u16_at(d, o)?;
            let component = u16_at(d, o + 2)?;
            o += 4;
            let (dx, dy) = if flags & WORDS != 0 {
                let v = (i16_at(d, o)?, i16_at(d, o + 2)?);
                o += 4;
                (f64::from(v.0), f64::from(v.1))
            } else {
                let v = (*d.get(o)? as i8, *d.get(o + 1)? as i8);
                o += 2;
                (f64::from(v.0), f64::from(v.1))
            };
            let (dx, dy) = if flags & XY_VALUES != 0 {
                (dx, dy)
            } else {
                (0.0, 0.0)
            };
            let (a, b, c, e) = if flags & SCALE != 0 {
                let s = f2dot14(o)?;
                o += 2;
                (s, 0.0, 0.0, s)
            } else if flags & XY_SCALE != 0 {
                let v = (f2dot14(o)?, f2dot14(o + 2)?);
                o += 4;
                (v.0, 0.0, 0.0, v.1)
            } else if flags & TWO_BY_TWO != 0 {
                let v = (
                    f2dot14(o)?,
                    f2dot14(o + 2)?,
                    f2dot14(o + 4)?,
                    f2dot14(o + 6)?,
                );
                o += 8;
                v
            } else {
                (1.0, 0.0, 0.0, 1.0)
            };
            let local = Affine::new([a, b, c, e, dx, dy]);
            self.append_outline(component, at * local, depth + 1, out);
            if flags & MORE == 0 {
                return Some(());
            }
        }
    }
}

/// A simple glyph's contours, with TrueType's implied on-curve points.
fn simple(d: &[u8], contours: usize, at: Affine, out: &mut BezPath) -> Option<()> {
    let mut ends = Vec::with_capacity(contours);
    for i in 0..contours {
        ends.push(usize::from(u16_at(d, 10 + 2 * i)?));
    }
    let n = ends.last().map_or(0, |e| e + 1);
    let instr_len = usize::from(u16_at(d, 10 + 2 * contours)?);
    let mut o = 12 + 2 * contours + instr_len;
    let mut flags = Vec::with_capacity(n);
    while flags.len() < n {
        let f = *d.get(o)?;
        o += 1;
        flags.push(f);
        if f & 0x08 != 0 {
            let repeat = *d.get(o)?;
            o += 1;
            for _ in 0..repeat {
                flags.push(f);
            }
        }
    }
    flags.truncate(n);
    let mut read_axis = |short: u8, same: u8| -> Option<Vec<f64>> {
        let mut v = 0i32;
        let mut out = Vec::with_capacity(n);
        for f in &flags {
            if f & short != 0 {
                let b = i32::from(*d.get(o)?);
                o += 1;
                v += if f & same != 0 { b } else { -b };
            } else if f & same == 0 {
                v += i32::from(i16_at(d, o)?);
                o += 2;
            }
            out.push(f64::from(v));
        }
        Some(out)
    };
    let xs = read_axis(0x02, 0x10)?;
    let ys = read_axis(0x04, 0x20)?;
    let mut start = 0;
    for end in ends {
        if end < start || end >= n {
            return None;
        }
        let pts: Vec<(Point, bool)> = (start..=end)
            .map(|i| (at * Point::new(xs[i], ys[i]), flags[i] & 0x01 != 0))
            .collect();
        contour(&pts, out);
        start = end + 1;
    }
    Some(())
}

fn mid(a: Point, b: Point) -> Point {
    a.midpoint(b)
}

/// One closed contour from quadratic TrueType points.
fn contour(pts: &[(Point, bool)], out: &mut BezPath) {
    let Some(&(first, first_on)) = pts.first() else {
        return;
    };
    let &(last, last_on) = pts.last().unwrap_or(&(first, first_on));
    let (begin, rest): (Point, &[(Point, bool)]) = if first_on {
        (first, &pts[1..])
    } else if last_on {
        (last, &pts[..pts.len() - 1])
    } else {
        (mid(last, first), pts)
    };
    out.move_to(begin);
    let mut pending: Option<Point> = None;
    for &(p, on) in rest {
        match (on, pending) {
            (true, Some(c)) => {
                out.quad_to(c, p);
                pending = None;
            }
            (true, None) => out.line_to(p),
            (false, Some(c)) => {
                out.quad_to(c, mid(c, p));
                pending = Some(p);
            }
            (false, None) => pending = Some(p),
        }
    }
    match pending {
        Some(c) => out.quad_to(c, begin),
        None => out.line_to(begin),
    }
    out.close_path();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regular() -> &'static [u8] {
        textweaver_fonts::bundled::default_text()
            .map(|f| f.face(textweaver_fonts::Style::Regular).data)
            .unwrap_or_default()
    }

    #[test]
    fn reads_the_bundled_font() {
        let mut f = Font::parse(regular()).unwrap();
        let a = f.glyph('a');
        assert_ne!(a, 0);
        assert_ne!(f.glyph('A'), a);
        assert!(f.advance(a) > 0.0);
        assert!(f.width("mm") > f.width("i"));
        assert!(f.line_height() > 0.0);
        let path = f.outline(a).clone();
        assert!(!path.elements().is_empty());
        // A composite or accented glyph still draws.
        let e = f.glyph('é');
        assert_ne!(e, 0);
        assert!(!f.outline(e).elements().is_empty());
        // The space has no outline but has a width.
        let space = f.glyph(' ');
        assert!(f.outline(space).elements().is_empty());
        assert!(f.advance(space) > 0.0);
    }

    #[test]
    fn rejects_what_is_not_a_font() {
        assert!(Font::parse(b"not a font").is_err());
    }
}
