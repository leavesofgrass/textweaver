//! Content streams: the glyphs a page shows, with their positions, sizes,
//! and styles, and where its images are.
//!
//! Coordinates are converted to the page as displayed (after `/Rotate`),
//! in points from the top-left corner, y growing downward; a glyph's `y`
//! is its baseline. Text in marked content tagged `Artifact` (running
//! headers, footers, and page numbers in tagged PDFs) is not collected, and
//! marked content with `/ActualText` replaces the glyphs it covers (tagged
//! PDFs use it for ligatures and hyphenation). Text that is not horizontal
//! (rotated margin labels) is dropped.

use std::collections::HashMap;

use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId};

use super::fonts::Font;

/// An affine matrix `[a b c d e f]`: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
pub(super) type Matrix = [f32; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m` then `n` (the PDF convention `m × n`).
fn mul(m: &Matrix, n: &Matrix) -> Matrix {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

fn apply(m: &Matrix, x: f32, y: f32) -> (f32, f32) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// "No marked-content id".
pub(super) const NO_MCID: u32 = u32::MAX;

/// One shown glyph (or an `/ActualText` replacement).
#[derive(Clone, Copy, Debug)]
pub(super) struct Glyph {
    /// Left edge.
    pub x: f32,
    /// Baseline.
    pub y: f32,
    /// Advance width.
    pub w: f32,
    /// Font size as displayed.
    pub size: f32,
    /// Byte range of the glyph's text in [`PageContent::text`].
    pub start: u32,
    pub len: u32,
    /// Font style bits (see `fonts`).
    pub style: u8,
    /// Marked-content id, or [`NO_MCID`].
    pub mcid: u32,
}

/// Where an image is drawn.
#[derive(Clone, Debug)]
pub(super) struct ImageBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub mcid: u32,
    /// Alternate text from the marked content around the image.
    pub alt: Option<String>,
}

/// Everything collected from one page.
#[derive(Debug, Default)]
pub(super) struct PageContent {
    /// Displayed width and height in points.
    pub width: f32,
    pub height: f32,
    /// The text of every glyph, back to back.
    pub text: String,
    pub glyphs: Vec<Glyph>,
    pub images: Vec<ImageBox>,
    /// True when the content stream could not be parsed.
    pub failed: bool,
}

impl PageContent {
    /// The text of a glyph.
    pub(super) fn glyph_text(&self, g: &Glyph) -> &str {
        self.text
            .get(g.start as usize..(g.start + g.len) as usize)
            .unwrap_or("")
    }
}

/// Fonts loaded so far in the document, by object id (or, for inline font
/// dictionaries, by address).
#[derive(Default)]
pub(super) struct FontCache {
    by_key: HashMap<(u32, u16, usize), usize>,
    pub fonts: Vec<Font>,
}

impl FontCache {
    fn index(&mut self, doc: &Document, font: &Object) -> usize {
        let key = match font {
            Object::Reference((n, g)) => (*n, *g, 0),
            other => (0, 0, std::ptr::from_ref(other) as usize),
        };
        if let Some(&i) = self.by_key.get(&key) {
            return i;
        }
        let dict = doc
            .dereference(font)
            .ok()
            .and_then(|(_, o)| o.as_dict().ok());
        let f = match dict {
            Some(d) => Font::load(doc, d),
            None => Font::fallback(),
        };
        self.fonts.push(f);
        let i = self.fonts.len() - 1;
        self.by_key.insert(key, i);
        i
    }

    fn fallback(&mut self) -> usize {
        let key = (0, 0, 0);
        if let Some(&i) = self.by_key.get(&key) {
            return i;
        }
        self.fonts.push(Font::fallback());
        let i = self.fonts.len() - 1;
        self.by_key.insert(key, i);
        i
    }
}

#[derive(Clone, Copy)]
struct State {
    ctm: Matrix,
    font: Option<usize>,
    size: f32,
    tc: f32,
    tw: f32,
    th: f32,
    tl: f32,
    rise: f32,
}

struct Marked {
    artifact: bool,
    mcid: u32,
    actual: Option<String>,
    alt: Option<String>,
    glyph_start: usize,
}

/// Upper bound on operators interpreted per page (form recursion included),
/// so a malicious file cannot loop for long.
const MAX_OPS: usize = 5_000_000;
/// Deepest form XObject nesting followed.
const MAX_DEPTH: u8 = 12;

/// Interprets page `page_id` into `out`.
pub(super) fn run_page(doc: &Document, page_id: ObjectId, fonts: &mut FontCache) -> PageContent {
    let mut out = PageContent::default();
    let Ok(page) = doc.get_dictionary(page_id) else {
        out.failed = true;
        return out;
    };
    let mbox = inherited(doc, page, b"CropBox")
        .or_else(|| inherited(doc, page, b"MediaBox"))
        .and_then(|o| rect(doc, o))
        .unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let [llx, lly, urx, ury] = mbox;
    let rotate = inherited(doc, page, b"Rotate")
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0)
        .rem_euclid(360);
    let (w, h) = (urx - llx, ury - lly);
    let flip: Matrix = match rotate {
        90 => [0.0, 1.0, 1.0, 0.0, -lly, -llx],
        180 => [-1.0, 0.0, 0.0, 1.0, urx, -lly],
        270 => [0.0, -1.0, -1.0, 0.0, ury, urx],
        _ => [1.0, 0.0, 0.0, -1.0, -llx, ury],
    };
    (out.width, out.height) = if rotate == 90 || rotate == 270 {
        (h, w)
    } else {
        (w, h)
    };
    let resources = page_resources(doc, page_id);
    let content = doc.get_page_content(page_id);
    let mut it = Interp {
        doc,
        fonts,
        out: &mut out,
        flip,
        tm: IDENTITY,
        tlm: IDENTITY,
        stack: Vec::new(),
        state: State {
            ctm: IDENTITY,
            font: None,
            size: 0.0,
            tc: 0.0,
            tw: 0.0,
            th: 1.0,
            tl: 0.0,
            rise: 0.0,
        },
        marked: Vec::new(),
        ops: 0,
    };
    if !it.run(&content, &resources, 0) {
        it.out.failed = true;
    }
    out
}

fn inherited<'a>(doc: &'a Document, page: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    let mut node = page;
    for _ in 0..64 {
        if let Ok(v) = node.get(key) {
            return doc.dereference(v).ok().map(|(_, o)| o);
        }
        node = node
            .get(b"Parent")
            .ok()
            .and_then(|p| doc.dereference(p).ok())
            .and_then(|(_, o)| o.as_dict().ok())?;
    }
    None
}

fn number(doc: &Document, o: &Object) -> Option<f32> {
    match doc.dereference(o).ok()?.1 {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(r) => Some(*r),
        _ => None,
    }
}

fn rect(doc: &Document, o: &Object) -> Option<[f32; 4]> {
    let a = o.as_array().ok()?;
    if a.len() != 4 {
        return None;
    }
    let v: Vec<f32> = a.iter().filter_map(|x| number(doc, x)).collect();
    (v.len() == 4).then(|| {
        [
            v[0].min(v[2]),
            v[1].min(v[3]),
            v[0].max(v[2]),
            v[1].max(v[3]),
        ]
    })
}

/// The page's resource dictionaries, nearest first (inherited ones after).
fn page_resources(doc: &Document, page_id: ObjectId) -> Vec<&Dictionary> {
    let mut out = Vec::new();
    if let Ok((inline, ids)) = doc.get_page_resources(page_id) {
        out.extend(inline);
        for id in ids {
            if let Ok(d) = doc.get_dictionary(id) {
                out.push(d);
            }
        }
    }
    out
}

/// `resources[category][name]`, dereferenced, searching each dictionary.
fn lookup<'a>(
    doc: &'a Document,
    resources: &[&'a Dictionary],
    category: &[u8],
    name: &[u8],
) -> Option<&'a Object> {
    resources.iter().find_map(|r| {
        let cat = r.get(category).ok()?;
        let cat = doc.dereference(cat).ok()?.1.as_dict().ok()?;
        cat.get(name).ok()
    })
}

struct Interp<'a, 'b> {
    doc: &'a Document,
    fonts: &'b mut FontCache,
    out: &'b mut PageContent,
    flip: Matrix,
    tm: Matrix,
    tlm: Matrix,
    stack: Vec<State>,
    state: State,
    marked: Vec<Marked>,
    ops: usize,
}

fn operand(ops: &[Object], i: usize) -> f32 {
    ops.get(i)
        .and_then(|o| match o {
            Object::Integer(v) => Some(*v as f32),
            Object::Real(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(0.0)
}

fn matrix_operands(ops: &[Object]) -> Matrix {
    [
        operand(ops, 0),
        operand(ops, 1),
        operand(ops, 2),
        operand(ops, 3),
        operand(ops, 4),
        operand(ops, 5),
    ]
}

impl<'a> Interp<'a, '_> {
    /// Runs a content stream; false when it could not be parsed.
    fn run(&mut self, content: &[u8], resources: &[&'a Dictionary], depth: u8) -> bool {
        let Ok(content) = Content::decode(content) else {
            return false;
        };
        for op in &content.operations {
            self.ops += 1;
            if self.ops > MAX_OPS {
                return true;
            }
            self.op(&op.operator, &op.operands, resources, depth);
        }
        true
    }

    fn op(&mut self, operator: &str, ops: &[Object], resources: &[&'a Dictionary], depth: u8) {
        let s = &mut self.state;
        match operator {
            "q" => {
                if self.stack.len() < 512 {
                    self.stack.push(*s);
                }
            }
            "Q" => {
                if let Some(p) = self.stack.pop() {
                    *s = p;
                }
            }
            "cm" => s.ctm = mul(&matrix_operands(ops), &s.ctm),
            "BT" | "ET" => {
                self.tm = IDENTITY;
                self.tlm = IDENTITY;
            }
            "Tf" => {
                s.size = operand(ops, 1);
                let name = ops.first().and_then(|o| o.as_name().ok()).unwrap_or(b"");
                s.font = Some(match lookup(self.doc, resources, b"Font", name) {
                    Some(f) => self.fonts.index(self.doc, f),
                    None => self.fonts.fallback(),
                });
            }
            "Tc" => s.tc = operand(ops, 0),
            "Tw" => s.tw = operand(ops, 0),
            "Tz" => s.th = operand(ops, 0) / 100.0,
            "TL" => s.tl = operand(ops, 0),
            "Ts" => s.rise = operand(ops, 0),
            "Td" => self.move_line(operand(ops, 0), operand(ops, 1)),
            "TD" => {
                s.tl = -operand(ops, 1);
                self.move_line(operand(ops, 0), operand(ops, 1));
            }
            "Tm" => {
                self.tlm = matrix_operands(ops);
                self.tm = self.tlm;
            }
            "T*" => {
                let tl = s.tl;
                self.move_line(0.0, -tl);
            }
            "Tj" => {
                if let Some(Ok(b)) = ops.first().map(Object::as_str) {
                    self.show(b);
                }
            }
            "'" => {
                let tl = s.tl;
                self.move_line(0.0, -tl);
                if let Some(Ok(b)) = ops.first().map(Object::as_str) {
                    self.show(b);
                }
            }
            "\"" => {
                s.tw = operand(ops, 0);
                s.tc = operand(ops, 1);
                let tl = s.tl;
                self.move_line(0.0, -tl);
                if let Some(Ok(b)) = ops.get(2).map(Object::as_str) {
                    self.show(b);
                }
            }
            "TJ" => {
                let Some(Ok(items)) = ops.first().map(Object::as_array) else {
                    return;
                };
                for item in items {
                    match item {
                        Object::String(b, _) => self.show(b),
                        Object::Integer(_) | Object::Real(_) => {
                            let n = operand(std::slice::from_ref(item), 0);
                            let tx = -n / 1000.0 * self.state.size * self.state.th;
                            self.tm = mul(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &self.tm);
                        }
                        _ => {}
                    }
                }
            }
            "Do" => self.draw(ops, resources, depth),
            "BMC" | "BDC" => self.begin_marked(ops, resources),
            "EMC" => self.end_marked(),
            _ => {}
        }
    }

    fn move_line(&mut self, tx: f32, ty: f32) {
        self.tlm = mul(&[1.0, 0.0, 0.0, 1.0, tx, ty], &self.tlm);
        self.tm = self.tlm;
    }

    fn show(&mut self, bytes: &[u8]) {
        let fi = match self.state.font {
            Some(f) => f,
            None => self.fonts.fallback(),
        };
        let st = self.state;
        let artifact = self.marked.iter().any(|m| m.artifact);
        let mcid = self
            .marked
            .iter()
            .rev()
            .find(|m| m.mcid != NO_MCID)
            .map_or(NO_MCID, |m| m.mcid);
        let Interp {
            fonts,
            out,
            tm,
            flip,
            ..
        } = self;
        let Some(font) = fonts.fonts.get_mut(fi) else {
            return;
        };
        let style = font.style;
        font.for_each_code(bytes, |code| {
            let full = mul(&mul(tm, &st.ctm), flip);
            let adv = (code.width * st.size + st.tc + if code.is_space_code { st.tw } else { 0.0 })
                * st.th;
            let horizontal = full[1].abs() <= 0.18 * full[0].abs() && full[0] > 0.0;
            if !artifact && !code.text.is_empty() && horizontal {
                let (x, y) = apply(&full, 0.0, st.rise);
                let size = st.size.abs() * full[2].hypot(full[3]);
                let w = adv * full[0].hypot(full[1]);
                let start = out.text.len() as u32;
                out.text.push_str(&code.text);
                out.glyphs.push(Glyph {
                    x,
                    y,
                    w,
                    size,
                    start,
                    len: code.text.len() as u32,
                    style,
                    mcid,
                });
            }
            *tm = mul(&[1.0, 0.0, 0.0, 1.0, adv, 0.0], tm);
        });
    }

    fn draw(&mut self, ops: &[Object], resources: &[&'a Dictionary], depth: u8) {
        let Some(name) = ops.first().and_then(|o| o.as_name().ok()) else {
            return;
        };
        let Some(obj) = lookup(self.doc, resources, b"XObject", name) else {
            return;
        };
        let Ok((_, Object::Stream(stream))) = self.doc.dereference(obj) else {
            return;
        };
        let subtype = stream
            .dict
            .get(b"Subtype")
            .and_then(Object::as_name)
            .unwrap_or(b"");
        match subtype {
            b"Form" if depth < MAX_DEPTH => {
                let m = stream
                    .dict
                    .get(b"Matrix")
                    .ok()
                    .and_then(|o| o.as_array().ok())
                    .map_or(IDENTITY, |a| matrix_operands(a));
                let own = stream
                    .dict
                    .get(b"Resources")
                    .ok()
                    .and_then(|o| self.doc.dereference(o).ok())
                    .and_then(|(_, o)| o.as_dict().ok());
                let inner: Vec<&'a Dictionary> = match own {
                    Some(d) => std::iter::once(d)
                        .chain(resources.iter().copied())
                        .collect(),
                    None => resources.to_vec(),
                };
                let Ok(content) = stream.get_plain_content() else {
                    return;
                };
                let saved = (self.state, self.stack.len(), self.tm, self.tlm);
                self.state.ctm = mul(&m, &self.state.ctm);
                self.run(&content, &inner, depth + 1);
                self.stack.truncate(saved.1);
                (self.state, _, self.tm, self.tlm) = saved;
            }
            b"Image" => {
                let full = mul(&self.state.ctm, &self.flip);
                let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
                    .map(|(x, y)| apply(&full, x, y));
                let xs = corners.map(|c| c.0);
                let ys = corners.map(|c| c.1);
                let fold =
                    |v: [f32; 4], f: fn(f32, f32) -> f32| v.into_iter().reduce(f).unwrap_or(0.0);
                let alt = self.marked.iter().rev().find_map(|m| m.alt.clone());
                let mcid = self
                    .marked
                    .iter()
                    .rev()
                    .find(|m| m.mcid != NO_MCID)
                    .map_or(NO_MCID, |m| m.mcid);
                self.out.images.push(ImageBox {
                    x0: fold(xs, f32::min),
                    y0: fold(ys, f32::min),
                    x1: fold(xs, f32::max),
                    y1: fold(ys, f32::max),
                    mcid,
                    alt,
                });
            }
            _ => {}
        }
    }

    fn begin_marked(&mut self, ops: &[Object], resources: &[&'a Dictionary]) {
        let tag = ops.first().and_then(|o| o.as_name().ok()).unwrap_or(b"");
        let props = match ops.get(1) {
            Some(Object::Dictionary(d)) => Some(d),
            Some(Object::Name(n)) => lookup(self.doc, resources, b"Properties", n)
                .and_then(|o| self.doc.dereference(o).ok())
                .and_then(|(_, o)| o.as_dict().ok()),
            _ => None,
        };
        let text = |key: &[u8]| {
            props
                .and_then(|p| p.get(key).ok())
                .and_then(|o| lopdf::decode_text_string(o).ok())
        };
        let mcid = props
            .and_then(|p| p.get(b"MCID").ok())
            .and_then(|o| o.as_i64().ok())
            .and_then(|i| u32::try_from(i).ok())
            .unwrap_or(NO_MCID);
        self.marked.push(Marked {
            artifact: tag == b"Artifact",
            mcid,
            actual: text(b"ActualText"),
            alt: text(b"Alt"),
            glyph_start: self.out.glyphs.len(),
        });
    }

    fn end_marked(&mut self) {
        let Some(m) = self.marked.pop() else {
            return;
        };
        let Some(actual) = m.actual else {
            return;
        };
        let covered = &self.out.glyphs[m.glyph_start.min(self.out.glyphs.len())..];
        let Some(first) = covered.first().copied() else {
            return;
        };
        let x1 = covered.iter().map(|g| g.x + g.w).fold(first.x, f32::max);
        let size = covered.iter().map(|g| g.size).fold(0.0, f32::max);
        self.out.glyphs.truncate(m.glyph_start);
        let actual: String = actual.chars().filter(|c| !c.is_control()).collect();
        if actual.is_empty() {
            return;
        }
        let start = self.out.text.len() as u32;
        self.out.text.push_str(&actual);
        self.out.glyphs.push(Glyph {
            x: first.x,
            y: first.y,
            w: x1 - first.x,
            size,
            start,
            len: actual.len() as u32,
            style: first.style,
            mcid: first.mcid,
        });
    }
}
