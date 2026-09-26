//! Fonts for PDF output: the bundled families (`textweaver-fonts`),
//! installed families found by name, and just enough of the OpenType
//! tables (`head`, `hhea`, `hmtx`, `cmap`, `OS/2`) to measure text and to
//! know which characters a font can show.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use krilla::text::Font as KrillaFont;
use textweaver_fonts::{FamilySource, Style, bundled, system};

use crate::{PdfOptions, WriteError};

/// Font bytes: embedded in the program, or read from a file.
#[derive(Clone)]
enum Bytes {
    Static(&'static [u8]),
    Owned(Arc<Vec<u8>>),
}

impl std::ops::Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Bytes::Static(b) => b,
            Bytes::Owned(v) => v,
        }
    }
}

impl From<Bytes> for krilla::Data {
    fn from(b: Bytes) -> krilla::Data {
        match b {
            Bytes::Static(s) => s.into(),
            Bytes::Owned(v) => v.into(),
        }
    }
}

/// A parsed font face.
pub(crate) struct Face {
    pub(crate) krilla: KrillaFont,
    /// Where it came from, for messages.
    pub(crate) name: String,
    data: Bytes,
    upem: f32,
    /// Ascender as a fraction of the em.
    pub(crate) ascent: f32,
    hmtx: usize,
    num_hmetrics: usize,
    cmap: Option<(usize, u16)>,
    cache: RefCell<HashMap<char, u16>>,
}

fn u16_at(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(i..i + 2)?.try_into().ok()?))
}

fn i16_at(d: &[u8], i: usize) -> Option<i16> {
    Some(i16::from_be_bytes(d.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?))
}

/// Offsets of the tables of the face at `base`.
fn tables(d: &[u8], base: usize) -> Option<HashMap<[u8; 4], usize>> {
    let n = usize::from(u16_at(d, base + 4)?);
    let mut map = HashMap::new();
    for k in 0..n {
        let rec = base + 12 + 16 * k;
        let tag: [u8; 4] = d.get(rec..rec + 4)?.try_into().ok()?;
        let off = u32_at(d, rec + 8)? as usize;
        map.insert(tag, off);
    }
    Some(map)
}

impl Face {
    /// Parses face `index` of a font file (TrueType, OpenType, or a
    /// collection).
    fn parse(data: Bytes, index: u32, name: &str) -> Result<Face, WriteError> {
        let bad = |why: &str| WriteError::Font(name.to_owned(), why.to_owned());
        let base = if data.starts_with(b"ttcf") {
            let at = 12 + 4 * index as usize;
            u32_at(&data, at).ok_or_else(|| bad("truncated collection"))? as usize
        } else {
            0
        };
        let t = tables(&data, base).ok_or_else(|| bad("no table directory"))?;
        let head = *t.get(b"head").ok_or_else(|| bad("no head table"))?;
        let hhea = *t.get(b"hhea").ok_or_else(|| bad("no hhea table"))?;
        let hmtx = *t.get(b"hmtx").ok_or_else(|| bad("no hmtx table"))?;
        let upem = f32::from(
            u16_at(&data, head + 18)
                .ok_or_else(|| bad("bad head"))?
                .max(16),
        );
        let ascender = f32::from(i16_at(&data, hhea + 4).ok_or_else(|| bad("bad hhea"))?);
        let num_hmetrics = usize::from(u16_at(&data, hhea + 34).ok_or_else(|| bad("bad hhea"))?);
        if num_hmetrics == 0 {
            return Err(bad("no horizontal metrics"));
        }
        // Fonts whose licence forbids embedding (OS/2 fsType bit 1, and no
        // other permission bit) cannot go into a PDF/UA file.
        if let Some(&os2) = t.get(b"OS/2") {
            let fs_type = u16_at(&data, os2 + 8).unwrap_or(0);
            if fs_type & 0x000F == 0x0002 {
                return Err(bad("its licence does not allow embedding"));
            }
        }
        let cmap = t.get(b"cmap").and_then(|&c| best_cmap(&data, c));
        if cmap.is_none() {
            return Err(bad("no Unicode character map"));
        }
        let krilla = KrillaFont::new(data.clone().into(), index)
            .ok_or_else(|| bad("the PDF library cannot read it"))?;
        Ok(Face {
            krilla,
            name: name.to_owned(),
            data,
            upem,
            ascent: (ascender / upem).clamp(0.5, 1.2),
            hmtx,
            num_hmetrics,
            cmap,
            cache: RefCell::new(HashMap::new()),
        })
    }

    /// Reads face `index` of a font file.
    pub(crate) fn load(path: &Path, index: u32) -> Result<Face, WriteError> {
        let name = path.display().to_string();
        let data =
            std::fs::read(path).map_err(|e| WriteError::Font(name.clone(), e.to_string()))?;
        Face::parse(Bytes::Owned(Arc::new(data)), index, &name)
    }

    /// Parses a font embedded in the program.
    pub(crate) fn embedded(data: &'static [u8], name: &str) -> Result<Face, WriteError> {
        Face::parse(Bytes::Static(data), 0, name)
    }

    /// Parses font bytes held in memory (tests).
    #[cfg(test)]
    pub(crate) fn from_vec(data: Vec<u8>, name: &str) -> Result<Face, WriteError> {
        Face::parse(Bytes::Owned(Arc::new(data)), 0, name)
    }

    /// The glyph for `c`, or 0 when the font has none.
    pub(crate) fn glyph(&self, c: char) -> u16 {
        if let Some(&g) = self.cache.borrow().get(&c) {
            return g;
        }
        let g = self
            .cmap
            .and_then(|(off, format)| lookup(&self.data, off, format, c as u32))
            .unwrap_or(0);
        self.cache.borrow_mut().insert(c, g);
        g
    }

    /// True when the font can show `c` (spaces always can).
    pub(crate) fn has(&self, c: char) -> bool {
        c == ' ' || self.glyph(c) != 0
    }

    /// Advance width of `c` at `size` points.
    pub(crate) fn advance(&self, c: char, size: f32) -> f32 {
        let g = usize::from(self.glyph(c));
        let i = g.min(self.num_hmetrics - 1);
        let adv = u16_at(&self.data, self.hmtx + 4 * i).unwrap_or(0);
        f32::from(adv) / self.upem * size
    }

    /// Width of `text` at `size` points (no kerning).
    pub(crate) fn width(&self, text: &str, size: f32) -> f32 {
        text.chars().map(|c| self.advance(c, size)).sum()
    }
}

/// The best Unicode subtable: (offset, format). Prefers full-repertoire
/// format 12, then BMP format 4.
fn best_cmap(d: &[u8], cmap: usize) -> Option<(usize, u16)> {
    let n = usize::from(u16_at(d, cmap + 2)?);
    let mut best: Option<(usize, u16, u8)> = None;
    for k in 0..n {
        let rec = cmap + 4 + 8 * k;
        let platform = u16_at(d, rec)?;
        let encoding = u16_at(d, rec + 2)?;
        let off = cmap + u32_at(d, rec + 4)? as usize;
        let format = u16_at(d, off)?;
        let rank = match (platform, encoding, format) {
            (3, 10, 12) | (0, 4, 12) | (0, 6, 12) => 3,
            (3, 1, 4) | (0, 3, 4) => 2,
            (0, _, 4) => 1,
            _ => 0,
        };
        if rank > 0 && best.is_none_or(|b| rank > b.2) {
            best = Some((off, format, rank));
        }
    }
    best.map(|(o, f, _)| (o, f))
}

fn lookup(d: &[u8], off: usize, format: u16, cp: u32) -> Option<u16> {
    match format {
        4 => {
            let cp = u16::try_from(cp).ok()?;
            let segs = usize::from(u16_at(d, off + 6)? / 2);
            let ends = off + 14;
            let starts = ends + 2 * segs + 2;
            let deltas = starts + 2 * segs;
            let ranges = deltas + 2 * segs;
            // Binary search for the first segment whose end >= cp.
            let (mut lo, mut hi) = (0, segs);
            while lo < hi {
                let mid = (lo + hi) / 2;
                if u16_at(d, ends + 2 * mid)? < cp {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            if lo == segs {
                return None;
            }
            let start = u16_at(d, starts + 2 * lo)?;
            if cp < start {
                return None;
            }
            let delta = u16_at(d, deltas + 2 * lo)?;
            let range = u16_at(d, ranges + 2 * lo)?;
            let g = if range == 0 {
                cp.wrapping_add(delta)
            } else {
                let at = ranges + 2 * lo + usize::from(range) + 2 * usize::from(cp - start);
                let g = u16_at(d, at)?;
                if g == 0 { 0 } else { g.wrapping_add(delta) }
            };
            (g != 0).then_some(g)
        }
        12 => {
            let groups = u32_at(d, off + 12)? as usize;
            let (mut lo, mut hi) = (0, groups);
            while lo < hi {
                let mid = (lo + hi) / 2;
                let g = off + 16 + 12 * mid;
                let start = u32_at(d, g)?;
                let end = u32_at(d, g + 4)?;
                if cp < start {
                    hi = mid;
                } else if cp > end {
                    lo = mid + 1;
                } else {
                    let first = u32_at(d, g + 8)?;
                    return u16::try_from(first + (cp - start)).ok().filter(|&x| x != 0);
                }
            }
            None
        }
        _ => None,
    }
}

/// A family's styles, as indexes into [`Fonts::faces`].
pub(crate) struct Family {
    pub(crate) regular: usize,
    pub(crate) bold: usize,
    pub(crate) italic: usize,
    pub(crate) bold_italic: usize,
    pub(crate) mono: usize,
    pub(crate) mono_bold: usize,
}

/// Font files by style for one family, as file names tried in the system
/// font folders when fonts are not bundled.
struct Candidate {
    regular: &'static str,
    bold: &'static str,
    italic: &'static str,
    bold_italic: &'static str,
}

/// System families tried in order when this build has no bundled fonts:
/// Atkinson Hyperlegible (designed for low-vision readers) when installed,
/// then common sans serif system fonts.
const FAMILIES: &[Candidate] = &[
    Candidate {
        regular: "AtkinsonHyperlegible-Regular.ttf",
        bold: "AtkinsonHyperlegible-Bold.ttf",
        italic: "AtkinsonHyperlegible-Italic.ttf",
        bold_italic: "AtkinsonHyperlegible-BoldItalic.ttf",
    },
    Candidate {
        regular: "verdana.ttf",
        bold: "verdanab.ttf",
        italic: "verdanai.ttf",
        bold_italic: "verdanaz.ttf",
    },
    Candidate {
        regular: "Verdana.ttf",
        bold: "Verdana Bold.ttf",
        italic: "Verdana Italic.ttf",
        bold_italic: "Verdana Bold Italic.ttf",
    },
    Candidate {
        regular: "segoeui.ttf",
        bold: "segoeuib.ttf",
        italic: "segoeuii.ttf",
        bold_italic: "segoeuiz.ttf",
    },
    Candidate {
        regular: "arial.ttf",
        bold: "arialbd.ttf",
        italic: "ariali.ttf",
        bold_italic: "arialbi.ttf",
    },
    Candidate {
        regular: "Arial.ttf",
        bold: "Arial Bold.ttf",
        italic: "Arial Italic.ttf",
        bold_italic: "Arial Bold Italic.ttf",
    },
    Candidate {
        regular: "DejaVuSans.ttf",
        bold: "DejaVuSans-Bold.ttf",
        italic: "DejaVuSans-Oblique.ttf",
        bold_italic: "DejaVuSans-BoldOblique.ttf",
    },
    Candidate {
        regular: "LiberationSans-Regular.ttf",
        bold: "LiberationSans-Bold.ttf",
        italic: "LiberationSans-Italic.ttf",
        bold_italic: "LiberationSans-BoldItalic.ttf",
    },
    Candidate {
        regular: "NotoSans-Regular.ttf",
        bold: "NotoSans-Bold.ttf",
        italic: "NotoSans-Italic.ttf",
        bold_italic: "NotoSans-BoldItalic.ttf",
    },
];

/// Monospaced system fonts tried when this build has no bundled fonts.
const MONO: &[&str] = &[
    "consola.ttf",
    "DejaVuSansMono.ttf",
    "LiberationMono-Regular.ttf",
    "cour.ttf",
    "Courier New.ttf",
    "NotoSansMono-Regular.ttf",
];

/// Fonts tried for characters the chosen fonts lack (other scripts,
/// symbols).
const FALLBACK: &[&str] = &[
    "DejaVuSans.ttf",
    "seguisym.ttf",
    "segoeui.ttf",
    "arial.ttf",
    "NotoSans-Regular.ttf",
    "NotoSansSymbols-Regular.ttf",
    "NotoSansSymbols2-Regular.ttf",
    "Arial Unicode.ttf",
];

/// Folders searched for the fallback font files.
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = system::font_dirs();
    if !cfg!(windows) && !cfg!(target_os = "macos") {
        for d in [
            "/usr/share/fonts/truetype/dejavu",
            "/usr/share/fonts/dejavu",
            "/usr/share/fonts/TTF",
            "/usr/share/fonts/truetype/liberation",
            "/usr/share/fonts/truetype/liberation2",
            "/usr/share/fonts/liberation",
            "/usr/share/fonts/truetype/noto",
            "/usr/share/fonts/noto",
            "/usr/share/fonts/truetype/atkinson-hyperlegible",
        ] {
            dirs.push(d.into());
        }
    }
    if cfg!(target_os = "macos") {
        dirs.insert(0, "/System/Library/Fonts/Supplemental".into());
    }
    dirs
}

fn find(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter().map(|d| d.join(name)).find(|p| p.is_file())
}

/// The four styles of a family, loaded.
struct Loaded {
    name: String,
    faces: [Face; 4],
}

/// Loads the family `name`: a bundled family, else an installed one.
fn load_family(name: &str) -> Result<Loaded, WriteError> {
    let not_found = || {
        let bundled: Vec<&str> = bundled::BUNDLED.iter().map(|f| f.name).collect();
        let hint = if bundled.is_empty() {
            "it is not installed".to_owned()
        } else {
            format!(
                "it is not installed and not bundled (bundled fonts: {})",
                bundled.join(", ")
            )
        };
        WriteError::Font(name.to_owned(), hint)
    };
    let source = if bundled::is_bundled(name) {
        textweaver_fonts::resolve_family(name, &[])
    } else {
        textweaver_fonts::resolve_family(name, system::installed())
    };
    match source.ok_or_else(not_found)? {
        FamilySource::Bundled(f) => {
            let load = |s: Style| Face::embedded(f.face(s).data, f.face(s).file_name);
            Ok(Loaded {
                name: f.name.to_owned(),
                faces: [
                    load(Style::Regular)?,
                    load(Style::Bold)?,
                    load(Style::Italic)?,
                    load(Style::BoldItalic)?,
                ],
            })
        }
        FamilySource::Installed(f) => {
            let load = |s: Style| {
                let r = f.face(s);
                Face::load(&r.path, r.index)
            };
            Ok(Loaded {
                name: f.name.clone(),
                faces: [
                    load(Style::Regular)?,
                    load(Style::Bold)?,
                    load(Style::Italic)?,
                    load(Style::BoldItalic)?,
                ],
            })
        }
    }
}

/// A font choice that is a file rather than a family name.
fn is_path(choice: &str) -> bool {
    let lower = choice.to_ascii_lowercase();
    [".ttf", ".otf", ".ttc", ".otc"]
        .iter()
        .any(|e| lower.ends_with(e))
        || choice.contains('/')
        || choice.contains('\\')
}

/// The fonts a PDF uses: the faces and which is which.
pub(crate) struct Fonts {
    pub(crate) faces: Vec<Face>,
    pub(crate) family: Family,
    /// The text family's name (checked by the tests).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) family_name: String,
    /// Fallback faces, by index into `faces`.
    fallback: Vec<usize>,
}

impl Fonts {
    /// Finds the fonts `options` ask for.
    ///
    /// Text: [`PdfOptions::font`] (a file), else [`PdfOptions::font_family`]
    /// (a bundled or installed family, or a file path), else the
    /// `TEXTWEAVER_PDF_FONT` environment variable (a file), else the bundled
    /// Atkinson Hyperlegible Next, else the first installed family of
    /// [`FAMILIES`]. Code: [`PdfOptions::code_font_family`], else the
    /// bundled Atkinson Hyperlegible Mono, else an installed monospaced
    /// font, else the text font. A family that was asked for by name and
    /// cannot be found is an error, so a typing mistake is not silently
    /// replaced.
    pub(crate) fn discover(options: &PdfOptions) -> Result<Fonts, WriteError> {
        let dirs = font_dirs();
        let mut faces: Vec<Face> = Vec::new();
        let env = std::env::var_os("TEXTWEAVER_PDF_FONT").map(PathBuf::from);
        let single = |faces: &mut Vec<Face>, face: Face| -> (String, [usize; 4]) {
            let name = face.name.clone();
            faces.push(face);
            let i = faces.len() - 1;
            (name, [i; 4])
        };
        let push_family = |faces: &mut Vec<Face>, l: Loaded| -> (String, [usize; 4]) {
            let base = faces.len();
            faces.extend(l.faces);
            (l.name, [base, base + 1, base + 2, base + 3])
        };
        let family_choice = options
            .font_family
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let (family_name, text) = if let Some(path) = &options.font {
            single(&mut faces, Face::load(path, 0)?)
        } else if let Some(choice) = family_choice {
            if is_path(choice) {
                single(&mut faces, Face::load(Path::new(choice), 0)?)
            } else {
                push_family(&mut faces, load_family(choice)?)
            }
        } else if let Some(path) = env {
            single(&mut faces, Face::load(&path, 0)?)
        } else if let Some(f) = bundled::default_text() {
            push_family(&mut faces, load_family(f.key)?)
        } else {
            let mut found = None;
            for c in FAMILIES {
                if let Some(p) = find(&dirs, c.regular)
                    && let Ok(face) = Face::load(&p, 0)
                {
                    let name = face.name.clone();
                    faces.push(face);
                    let regular = faces.len() - 1;
                    let mut style = |file: &str| -> usize {
                        find(&dirs, file)
                            .and_then(|p| Face::load(&p, 0).ok())
                            .map_or(regular, |f| {
                                faces.push(f);
                                faces.len() - 1
                            })
                    };
                    let bold = style(c.bold);
                    let italic = style(c.italic);
                    let bold_italic = style(c.bold_italic);
                    let bold_italic = if bold_italic == regular {
                        bold
                    } else {
                        bold_italic
                    };
                    found = Some((name, [regular, bold, italic, bold_italic]));
                    break;
                }
            }
            found.ok_or(WriteError::NoFont)?
        };
        let code_choice = options
            .code_font_family
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let (mono, mono_bold) = if let Some(choice) = code_choice {
            if is_path(choice) {
                let (_, s) = single(&mut faces, Face::load(Path::new(choice), 0)?);
                (s[0], s[1])
            } else {
                let (_, s) = push_family(&mut faces, load_family(choice)?);
                (s[0], s[1])
            }
        } else if let Some(f) = bundled::default_mono() {
            let (_, s) = push_family(&mut faces, load_family(f.key)?);
            (s[0], s[1])
        } else if let Some(face) = MONO
            .iter()
            .filter_map(|m| find(&dirs, m))
            .find_map(|p| Face::load(&p, 0).ok())
        {
            faces.push(face);
            (faces.len() - 1, faces.len() - 1)
        } else {
            (text[0], text[1])
        };
        let mut fallback = Vec::new();
        for name in FALLBACK {
            if let Some(p) = find(&dirs, name)
                && !faces.iter().any(|f| Path::new(&f.name) == p)
                && let Ok(face) = Face::load(&p, 0)
            {
                faces.push(face);
                fallback.push(faces.len() - 1);
            }
        }
        Ok(Fonts {
            faces,
            family: Family {
                regular: text[0],
                bold: text[1],
                italic: text[2],
                bold_italic: text[3],
                mono,
                mono_bold,
            },
            family_name,
            fallback,
        })
    }

    /// The face to draw `c` with when the preferred face is `preferred`:
    /// the preferred face, the regular face, then the fallbacks.
    pub(crate) fn resolve(&self, preferred: usize, c: char) -> Option<usize> {
        if self.faces[preferred].has(c) {
            return Some(preferred);
        }
        std::iter::once(self.family.regular)
            .chain(self.fallback.iter().copied())
            .find(|&i| self.faces[i].has(c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_fonts_measure_text() {
        let Ok(fonts) = Fonts::discover(&PdfOptions::default()) else {
            eprintln!("no bundled or system font; skipping");
            return;
        };
        let f = &fonts.faces[fonts.family.regular];
        assert!(f.has('A') && f.has('a') && f.has(' '));
        assert!(!f.has('\u{10FFFD}'));
        let w = f.width("Hello", 12.0);
        assert!(w > 20.0 && w < 45.0, "{w}");
        assert!(f.width("WWW", 12.0) > f.width("iii", 12.0));
        assert!(f.ascent > 0.5);
        if cfg!(feature = "bundled-fonts") {
            assert_eq!(fonts.family_name, "Atkinson Hyperlegible Next");
            // The code font is monospaced: every letter the same width.
            let m = &fonts.faces[fonts.family.mono];
            assert!((m.width("iii", 12.0) - m.width("WWW", 12.0)).abs() < 0.01);
            assert!(m.name.contains("AtkinsonHyperlegibleMono"));
            // Four distinct text faces.
            let fam = &fonts.family;
            let mut idx = vec![fam.regular, fam.bold, fam.italic, fam.bold_italic];
            idx.dedup();
            assert_eq!(idx.len(), 4);
        }
    }

    #[cfg(feature = "bundled-fonts")]
    #[test]
    fn bundled_families_by_name() {
        let o = PdfOptions {
            font_family: Some("OpenDyslexic".into()),
            code_font_family: Some("atkinson-hyperlegible-mono".into()),
            ..PdfOptions::default()
        };
        let fonts = Fonts::discover(&o).unwrap();
        assert_eq!(fonts.family_name, "OpenDyslexic");
        assert!(fonts.faces[fonts.family.bold].name.contains("Bold"));
        let bad = PdfOptions {
            font_family: Some("No Such Font Anywhere".into()),
            ..PdfOptions::default()
        };
        let e = Fonts::discover(&bad).err().unwrap().to_string();
        assert!(e.contains("No Such Font Anywhere"), "{e}");
        assert!(e.contains("Atkinson Hyperlegible Next"), "{e}");
    }

    #[test]
    fn paths_and_names_are_told_apart() {
        assert!(is_path("fonts/My.ttf"));
        assert!(is_path("C:\\Fonts\\x.otf"));
        assert!(is_path("Font.TTC"));
        assert!(!is_path("Atkinson Hyperlegible Next"));
        assert!(!is_path("Segoe UI"));
    }

    #[test]
    fn rejects_garbage() {
        assert!(Face::from_vec(b"not a font at all".to_vec(), "x").is_err());
        assert!(Face::from_vec(Vec::new(), "x").is_err());
    }
}
