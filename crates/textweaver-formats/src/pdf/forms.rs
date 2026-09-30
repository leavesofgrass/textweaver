//! Form fields (AcroForm), read with their labels and values (ADR-0048).
//!
//! A filled form's answers are not in the page's text: each field is a
//! widget annotation whose value lives in the field dictionary, so before
//! this the reader heard the questions and none of the answers. Each field
//! now becomes one line where the field is on the page, its label first
//! and then its value: "Name: Ada Example", "I agree to the terms:
//! checked", "Payment: Credit card", "Signature: not signed". An empty
//! field says "empty"; a required one says so after its label.
//!
//! The label is, in order of trust: the field's own accessible name
//! (`/TU`, what a screen reader says in a PDF viewer); the words printed
//! just before the field on its line (or just after a check box), or on the
//! line just above it; and last the field's internal name (`/T`) with its
//! underscores as spaces. Printed words used as a label are taken out of
//! the page's text, so the label is not heard twice, and so are the
//! underscores or dots drawn as a line to write on.
//!
//! Radio buttons are one field: the group's name, then the chosen option,
//! named by the words printed beside its button when there are some. The
//! options' own words stay in the text. Push buttons ("Submit", "Print")
//! do nothing in a reader and are left out. A form made only in XFA, which
//! is not PDF form data, is named in a warning.
//!
//! Hostile forms are bounded: field trees are followed at most 32 levels
//! deep and 10,000 fields in all, each object once, and labels and values
//! are cut to a few hundred characters.

use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Object, ObjectId};

use super::interp::PageContent;
use super::locate::Area;
use crate::annotations::clean_text;

/// Most fields read from one form.
pub const MAX_FIELDS: usize = 10_000;

/// Longest label or value kept, in characters.
const MAX_CHARS: usize = 300;

/// A field placed on a page, ready for the layout.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FieldBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    /// "Label: value".
    pub text: String,
}

/// What kind of field.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Text,
    Check,
    Radio,
    Push,
    Choice,
    Signature,
}

/// A terminal field and its widgets.
struct Field {
    /// `/T` of each level, joined by dots.
    name: String,
    /// `/TU`.
    tooltip: String,
    kind: Kind,
    flags: i64,
    value: Option<Object>,
    options: Vec<(String, String)>,
    /// Widget dictionaries with their object ids.
    widgets: Vec<(Option<ObjectId>, Dictionary)>,
}

const FF_READ_ONLY: i64 = 1;
const FF_REQUIRED: i64 = 1 << 1;
const FF_PASSWORD: i64 = 1 << 13;
const FF_RADIO: i64 = 1 << 15;
const FF_PUSH: i64 = 1 << 16;

fn cut(s: &str) -> String {
    let s = clean_text(s);
    if s.chars().count() <= MAX_CHARS {
        s
    } else {
        let mut t: String = s.chars().take(MAX_CHARS).collect();
        t.push_str("...");
        t
    }
}

fn text_value(pdf: &lopdf::Document, o: &Object) -> Option<String> {
    let (_, o) = pdf.dereference(o).ok()?;
    let t = match o {
        Object::Name(n) => String::from_utf8_lossy(n).into_owned(),
        other => lopdf::decode_text_string(other).ok()?,
    };
    let t = cut(&t);
    (!t.is_empty()).then_some(t)
}

/// A field's internal name made readable: "first_name" is "first name".
fn humanize(name: &str) -> String {
    let last = name.rsplit('.').next().unwrap_or(name);
    let words = cut(&last.replace(['_', '-'], " "));
    let mut c = words.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => words,
    }
}

/// Walks the field tree.
struct Walker<'a> {
    pdf: &'a lopdf::Document,
    seen: HashSet<ObjectId>,
    fields: Vec<Field>,
    visits: usize,
}

/// Values a field inherits from its parents.
#[derive(Clone, Default)]
struct Inherited {
    name: String,
    ft: Vec<u8>,
    flags: i64,
    value: Option<Object>,
    options: Option<Object>,
}

impl Walker<'_> {
    fn walk(&mut self, node: &Object, parent: &Inherited, depth: usize) {
        self.visits += 1;
        if depth > 32 || self.visits > MAX_FIELDS * 4 || self.fields.len() >= MAX_FIELDS {
            return;
        }
        let id = node.as_reference().ok();
        if let Some(id) = id
            && !self.seen.insert(id)
        {
            return;
        }
        let Some(d) = self
            .pdf
            .dereference(node)
            .ok()
            .and_then(|(_, o)| o.as_dict().ok())
        else {
            return;
        };
        let mut here = parent.clone();
        if let Some(t) = d.get(b"T").ok().and_then(|o| text_value(self.pdf, o)) {
            here.name = if here.name.is_empty() {
                t
            } else {
                format!("{}.{t}", here.name)
            };
        }
        if let Ok(ft) = d.get(b"FT").and_then(Object::as_name) {
            here.ft = ft.to_vec();
        }
        if let Ok(f) = d.get(b"Ff").and_then(Object::as_i64) {
            here.flags = f;
        }
        if let Ok(v) = d.get(b"V") {
            here.value = Some(v.clone());
        }
        if let Ok(o) = d.get(b"Opt") {
            here.options = Some(o.clone());
        }
        // Kids with a name are fields; kids without are this field's
        // widgets.
        let kids: Vec<Object> = d
            .get(b"Kids")
            .ok()
            .and_then(|o| self.pdf.dereference(o).ok())
            .and_then(|(_, o)| o.as_array().ok())
            .map(|a| a.iter().take(MAX_FIELDS).cloned().collect())
            .unwrap_or_default();
        let is_field = |o: &Object| {
            self.pdf
                .dereference(o)
                .ok()
                .and_then(|(_, o)| o.as_dict().ok())
                .is_some_and(|k| k.get(b"T").is_ok())
        };
        let (fields, widgets): (Vec<Object>, Vec<Object>) = kids.into_iter().partition(is_field);
        for f in &fields {
            self.walk(f, &here, depth + 1);
        }
        if !fields.is_empty() && widgets.is_empty() {
            return;
        }
        let mut ws = Vec::new();
        if d.get(b"Rect").is_ok() {
            ws.push((id, d.clone()));
        }
        for w in widgets {
            let wid = w.as_reference().ok();
            if let Some(dict) = self
                .pdf
                .dereference(&w)
                .ok()
                .and_then(|(_, o)| o.as_dict().ok())
            {
                ws.push((wid, dict.clone()));
            }
        }
        let kind = match here.ft.as_slice() {
            b"Tx" => Kind::Text,
            b"Btn" if here.flags & FF_PUSH != 0 => Kind::Push,
            b"Btn" if here.flags & FF_RADIO != 0 => Kind::Radio,
            b"Btn" => Kind::Check,
            b"Ch" => Kind::Choice,
            b"Sig" => Kind::Signature,
            _ => return,
        };
        let options = here
            .options
            .as_ref()
            .map(|o| self.options(o))
            .unwrap_or_default();
        self.fields.push(Field {
            name: here.name.clone(),
            tooltip: d
                .get(b"TU")
                .ok()
                .and_then(|o| text_value(self.pdf, o))
                .unwrap_or_default(),
            kind,
            flags: here.flags,
            value: here.value.clone(),
            options,
            widgets: ws,
        });
    }

    /// `/Opt`: export value and shown text for each option.
    fn options(&self, o: &Object) -> Vec<(String, String)> {
        let Some(a) = self
            .pdf
            .dereference(o)
            .ok()
            .and_then(|(_, o)| o.as_array().ok())
        else {
            return Vec::new();
        };
        a.iter()
            .take(1_000)
            .filter_map(|item| {
                let (_, item) = self.pdf.dereference(item).ok()?;
                match item {
                    Object::Array(pair) => {
                        let export = text_value(self.pdf, pair.first()?)?;
                        let shown = pair
                            .get(1)
                            .and_then(|s| text_value(self.pdf, s))
                            .unwrap_or_else(|| export.clone());
                        Some((export, shown))
                    }
                    other => text_value(self.pdf, other).map(|t| (t.clone(), t)),
                }
            })
            .collect()
    }
}

fn widget_rect(pdf: &lopdf::Document, d: &Dictionary) -> Option<[f32; 4]> {
    let a = pdf
        .dereference(d.get(b"Rect").ok()?)
        .ok()?
        .1
        .as_array()
        .ok()?;
    let v: Vec<f32> = a
        .iter()
        .filter_map(|o| match pdf.dereference(o).ok()?.1 {
            Object::Integer(i) => Some(*i as f32),
            Object::Real(r) => Some(*r),
            _ => None,
        })
        .filter(|v| v.is_finite())
        .collect();
    (v.len() == 4).then(|| {
        [
            v[0].min(v[2]),
            v[1].min(v[3]),
            v[0].max(v[2]),
            v[1].max(v[3]),
        ]
    })
}

/// A field on its page, with its widgets there.
type Placed<'a> = (usize, &'a Field, Vec<(Area, State)>);

/// A check box or radio widget's state.
#[derive(Clone, Debug, Default)]
struct State {
    /// Shown on (`/AS` other than `Off`).
    on: bool,
    /// The name it has when on.
    export: Option<Vec<u8>>,
}

/// The on state of a check box or radio widget: its `/AS`, when not `Off`.
fn on_state(d: &Dictionary) -> Option<Vec<u8>> {
    d.get(b"AS")
        .and_then(Object::as_name)
        .ok()
        .filter(|n| *n != b"Off")
        .map(<[u8]>::to_vec)
}

/// The export name a check box or radio widget has when on (the key of its
/// normal appearance other than `Off`).
fn export_name(pdf: &lopdf::Document, d: &Dictionary) -> Option<Vec<u8>> {
    let ap = pdf.dereference(d.get(b"AP").ok()?).ok()?.1.as_dict().ok()?;
    let n = pdf.dereference(ap.get(b"N").ok()?).ok()?.1.as_dict().ok()?;
    n.iter()
        .map(|(k, _)| k.clone())
        .find(|k| k.as_slice() != b"Off")
}

/// Where each annotation is: its object id to its page.
fn widget_pages(pdf: &lopdf::Document, page_ids: &[ObjectId]) -> HashMap<ObjectId, usize> {
    let mut out = HashMap::new();
    for (i, &id) in page_ids.iter().enumerate() {
        if let Some(list) = pdf
            .get_dictionary(id)
            .ok()
            .and_then(|p| p.get(b"Annots").ok())
            .and_then(|o| pdf.dereference(o).ok())
            .and_then(|(_, o)| o.as_array().ok())
        {
            for a in list.iter().take(super::annots::MAX_ANNOTS) {
                if let Ok(r) = a.as_reference() {
                    out.entry(r).or_insert(i);
                }
            }
        }
    }
    out
}

/// Reads the form, takes each field's printed label out of its page's
/// glyphs, and adds the fields to the pages. Returns a warning to add, if
/// any.
pub(super) fn apply(
    pdf: &lopdf::Document,
    page_ids: &[ObjectId],
    contents: &mut [PageContent],
) -> Option<String> {
    let form = pdf
        .catalog()
        .ok()?
        .get(b"AcroForm")
        .ok()
        .and_then(|o| pdf.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())?;
    let roots: Vec<Object> = form
        .get(b"Fields")
        .ok()
        .and_then(|o| pdf.dereference(o).ok())
        .and_then(|(_, o)| o.as_array().ok())
        .map(|a| a.iter().take(MAX_FIELDS).cloned().collect())
        .unwrap_or_default();
    let mut w = Walker {
        pdf,
        seen: HashSet::new(),
        fields: Vec::new(),
        visits: 0,
    };
    for r in &roots {
        w.walk(r, &Inherited::default(), 0);
    }
    if w.fields.is_empty() {
        return form.get(b"XFA").is_ok().then(|| {
            "This PDF's form is an XFA form, which textweaver cannot read, so its fields are not read.".to_owned()
        });
    }
    let pages = widget_pages(pdf, page_ids);
    let index: HashMap<ObjectId, usize> = page_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();
    let page_of = |id: Option<ObjectId>, d: &Dictionary| {
        id.and_then(|id| pages.get(&id).copied()).or_else(|| {
            d.get(b"P")
                .and_then(Object::as_reference)
                .ok()
                .and_then(|p| index.get(&p).copied())
        })
    };
    // Every widget's area, per page, so labels stop at the next field.
    let mut areas: HashMap<usize, Vec<Area>> = HashMap::new();
    let mut placed: Vec<Placed<'_>> = Vec::new();
    for f in &w.fields {
        if f.kind == Kind::Push {
            continue;
        }
        let mut by_page: Vec<(usize, Area, State)> = Vec::new();
        for (id, d) in &f.widgets {
            let flags = d.get(b"F").and_then(Object::as_i64).unwrap_or(0);
            if flags & 2 != 0 {
                continue;
            }
            let (Some(page), Some(r)) = (page_of(*id, d), widget_rect(pdf, d)) else {
                continue;
            };
            let Some(a) = contents.get(page).and_then(|c| c.rect_on_page(r)) else {
                continue;
            };
            areas.entry(page).or_default().push(a);
            let state = State {
                on: on_state(d).is_some(),
                export: export_name(pdf, d),
            };
            by_page.push((page, a, state));
        }
        let Some(&(page, ..)) = by_page.first() else {
            continue;
        };
        let widgets = by_page
            .into_iter()
            .filter(|(p, ..)| *p == page)
            .map(|(_, a, s)| (a, s))
            .collect();
        placed.push((page, f, widgets));
    }
    for (page, f, widgets) in placed {
        let others = areas.get(&page).cloned().unwrap_or_default();
        let Some(content) = contents.get_mut(page) else {
            continue;
        };
        if let Some(b) = field_box(pdf, f, &widgets, &others, content) {
            content.fields.push(b);
        }
    }
    None
}

/// The chosen value of a field, as said.
fn value_text(
    pdf: &lopdf::Document,
    f: &Field,
    widgets_on: bool,
    chosen_label: Option<String>,
) -> String {
    let v = f
        .value
        .as_ref()
        .and_then(|v| pdf.dereference(v).ok())
        .map(|(_, v)| v);
    let shown = |export: &str| {
        f.options
            .iter()
            .find(|(e, _)| e == export)
            .map_or_else(|| export.to_owned(), |(_, s)| s.clone())
    };
    match f.kind {
        Kind::Text if f.flags & FF_PASSWORD != 0 => "hidden".into(),
        Kind::Text => v
            .and_then(|v| text_value(pdf, v))
            .unwrap_or_else(|| "empty".into()),
        Kind::Check => {
            // The value, else (a form that only set the appearance) the
            // widget's state.
            let on = match v.and_then(|v| v.as_name().ok()) {
                Some(n) => n != b"Off",
                None => widgets_on,
            };
            let said = if on { "checked" } else { "not checked" };
            said.into()
        }
        Kind::Radio => {
            let on = v
                .and_then(|v| v.as_name().ok())
                .filter(|n| *n != b"Off")
                .map(|n| String::from_utf8_lossy(n).into_owned());
            match (on, chosen_label) {
                (Some(_), Some(l)) => l,
                (Some(n), None) => shown(&cut(&n)),
                (None, _) => "none chosen".into(),
            }
        }
        Kind::Choice => {
            let chosen: Vec<String> = match v {
                Some(Object::Array(a)) => a
                    .iter()
                    .take(100)
                    .filter_map(|o| text_value(pdf, o))
                    .map(|e| shown(&e))
                    .collect(),
                Some(o) => text_value(pdf, o).map(|e| shown(&e)).into_iter().collect(),
                None => Vec::new(),
            };
            if chosen.is_empty() {
                "none chosen".into()
            } else {
                chosen.join(", ")
            }
        }
        Kind::Signature => {
            let said = if v.is_some() { "signed" } else { "not signed" };
            said.into()
        }
        Kind::Push => String::new(),
    }
}

/// The glyphs of the words printed next to `area` on its line: before it
/// (`after = false`) or after it. Stops at a wide gap or another field.
fn run_beside(content: &PageContent, area: Area, after: bool, others: &[Area]) -> Vec<usize> {
    let (x0, y0, x1, y1) = area;
    let height = (y1 - y0).max(1.0);
    let mut row: Vec<usize> = (0..content.glyphs.len())
        .filter(|&i| {
            let g = &content.glyphs[i];
            let cy = g.y - 0.3 * g.size;
            cy >= y0 - 0.3 * height.max(g.size)
                && cy <= y1 + 0.3 * height.max(g.size)
                && if after {
                    g.x >= x1 - 1.0
                } else {
                    g.x + g.w <= x0 + 1.0
                }
        })
        .collect();
    row.sort_by(|&a, &b| {
        let (ga, gb) = (&content.glyphs[a], &content.glyphs[b]);
        if after {
            ga.x.total_cmp(&gb.x)
        } else {
            (gb.x + gb.w).total_cmp(&(ga.x + ga.w))
        }
    });
    let mut out = Vec::new();
    let mut edge = if after { x1 } else { x0 };
    let mut chars = 0;
    for i in row {
        let g = &content.glyphs[i];
        let gap = if after {
            g.x - edge
        } else {
            edge - (g.x + g.w)
        };
        let limit = if out.is_empty() { 3.0 } else { 1.5 } * g.size.max(4.0);
        if gap > limit {
            break;
        }
        // Another field between: its label is its own.
        let (lo, hi) = if after {
            (edge, g.x)
        } else {
            (g.x + g.w, edge)
        };
        let blocked = others.iter().any(|o| {
            *o != area && o.0 < hi && o.2 > lo && o.1 < y1 && o.3 > y0 && (o.2 - o.0) > 1.0
        });
        if blocked {
            break;
        }
        edge = if after { g.x + g.w } else { g.x };
        chars += content.glyph_text(g).chars().count();
        out.push(i);
        if chars > 80 {
            break;
        }
    }
    if !after {
        out.reverse();
    }
    out
}

/// The glyphs of a short line just above `area`, over it.
fn line_above(content: &PageContent, area: Area) -> Vec<usize> {
    let (x0, y0, x1, _) = area;
    let above: Vec<usize> = (0..content.glyphs.len())
        .filter(|&i| {
            let g = &content.glyphs[i];
            g.y <= y0 + 1.0 && g.y >= y0 - 2.5 * g.size && g.x < x1 && g.x + g.w > x0 - 2.0
        })
        .collect();
    let Some(base) = above
        .iter()
        .map(|&i| content.glyphs[i].y)
        .max_by(f32::total_cmp)
    else {
        return Vec::new();
    };
    let mut line: Vec<usize> = above
        .into_iter()
        .filter(|&i| (content.glyphs[i].y - base).abs() <= 0.3 * content.glyphs[i].size)
        .collect();
    line.sort_by(|&a, &b| content.glyphs[a].x.total_cmp(&content.glyphs[b].x));
    let chars: usize = line
        .iter()
        .map(|&i| content.glyph_text(&content.glyphs[i]).chars().count())
        .sum();
    if chars > 60 { Vec::new() } else { line }
}

/// The text of glyphs, with spaces where the gaps are.
fn words(content: &PageContent, glyphs: &[usize]) -> String {
    let mut out = String::new();
    let mut last: Option<(f32, f32)> = None;
    for &i in glyphs {
        let g = &content.glyphs[i];
        if let Some((end, size)) = last
            && g.x - end > 0.15 * size.max(g.size)
            && !out.ends_with(' ')
        {
            out.push(' ');
        }
        out.push_str(content.glyph_text(g));
        last = Some((g.x + g.w, g.size));
    }
    out
}

/// A printed label cleaned: the colon, stars, and writing lines at its end
/// removed.
fn clean_label(s: &str) -> String {
    let t = s
        .trim()
        .trim_end_matches(|c: char| {
            c == ':' || c == '*' || c == '_' || c == '.' || c.is_whitespace()
        })
        .trim_start_matches(|c: char| c == '_' || c.is_whitespace());
    cut(t)
}

/// A glyph that only draws a line to write on.
fn is_rule_glyph(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| matches!(c, '_' | '.' | '\u{2026}' | ' '))
}

/// The field's line: label, then value, with the printed label taken from
/// the page.
fn field_box(
    pdf: &lopdf::Document,
    f: &Field,
    widgets: &[(Area, State)],
    others: &[Area],
    content: &mut PageContent,
) -> Option<FieldBox> {
    let &(area, _) = widgets.first()?;
    let boxy = matches!(f.kind, Kind::Check | Kind::Radio);
    let mut used: Vec<usize> = Vec::new();
    let mut chosen_label = None;
    let printed = if f.kind == Kind::Radio {
        // The chosen option's words, left in the text.
        let on = f
            .value
            .as_ref()
            .and_then(|v| pdf.dereference(v).ok())
            .and_then(|(_, v)| v.as_name().ok().map(<[u8]>::to_vec));
        if let Some((a, _)) = widgets
            .iter()
            .find(|(_, s)| s.export.is_some() && s.export.as_deref() == on.as_deref())
        {
            let run = run_beside(content, *a, true, others);
            let l = clean_label(&words(content, &run));
            if !l.is_empty() {
                chosen_label = Some(l);
            }
        }
        String::new()
    } else {
        let mut run = if boxy {
            run_beside(content, area, true, others)
        } else {
            Vec::new()
        };
        if run.is_empty() {
            run = run_beside(content, area, false, others);
        }
        if run.is_empty() && !boxy {
            run = line_above(content, area);
        }
        let l = clean_label(&words(content, &run));
        if !l.is_empty() {
            used = run;
        }
        l
    };
    let label = if !f.tooltip.is_empty() {
        f.tooltip.clone()
    } else if !printed.is_empty() {
        printed
    } else {
        humanize(&f.name)
    };
    let label = if label.is_empty() {
        "Field".to_owned()
    } else {
        label
    };
    let value = value_text(pdf, f, widgets.iter().any(|(_, s)| s.on), chosen_label);
    let required = if f.flags & FF_REQUIRED != 0 && f.flags & FF_READ_ONLY == 0 {
        ", required"
    } else {
        ""
    };
    // Lines to write on, drawn as text inside the field.
    let (x0, y0, x1, y1) = area;
    for (i, g) in content.glyphs.iter().enumerate() {
        let cx = g.x + g.w / 2.0;
        let cy = g.y - 0.3 * g.size;
        if cx >= x0
            && cx <= x1
            && cy >= y0 - g.size
            && cy <= y1 + g.size / 2.0
            && is_rule_glyph(content.glyph_text(g))
        {
            used.push(i);
        }
    }
    let mut b = FieldBox {
        x0,
        y0,
        x1,
        y1,
        text: format!("{label}{required}: {value}"),
    };
    for &i in &used {
        let g = &content.glyphs[i];
        b.x0 = b.x0.min(g.x);
        b.x1 = b.x1.max(g.x + g.w);
        b.y0 = b.y0.min(g.y - g.size * 0.8);
        b.y1 = b.y1.max(g.y + g.size * 0.25);
    }
    let used: HashSet<usize> = used.into_iter().collect();
    let mut k = 0;
    content.glyphs.retain(|_| {
        let keep = !used.contains(&k);
        k += 1;
        keep
    });
    Some(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_cleaned_and_names_humanized() {
        assert_eq!(clean_label("  Name: ____"), "Name");
        assert_eq!(clean_label("Date of birth *"), "Date of birth");
        assert_eq!(humanize("form.page1.first_name"), "First name");
        assert!(is_rule_glyph("___"));
        assert!(is_rule_glyph("\u{2026}"));
        assert!(!is_rule_glyph("A"));
    }
}
