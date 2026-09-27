//! PowerPoint loader (`.pptx`, `.pptm`, `.ppsx`): slides in order, each
//! with its speaker notes.
//!
//! - Each slide is a level-1 `SectionBreak` (label "Slide 3: Title") and a
//!   `PageBreak` labeled with its number, so going to page 3 goes to slide
//!   3.
//! - The slide's title becomes a level-1 heading ("Slide 3" when it has
//!   none), then the other shapes follow in the slide's order: text boxes as
//!   paragraphs; body placeholders and bulleted or numbered paragraphs as
//!   lists, nested by their outline level; tables as tables; pictures by
//!   their alternative text (pictures without any are decorative and left
//!   out). Group shapes are read through.
//! - Slide numbers, dates, and footers are left out.
//! - Speaker notes follow the slide under a level-2 heading, "Speaker
//!   notes".
//! - Title and author come from `docProps/core.xml`.

use std::collections::HashMap;

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, HEADER_ROW_LABEL, Marker};

use crate::builder::Builder;
use crate::package::{Package, dir_of, parse_xml, resolve};
use crate::{LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// Most slides read.
pub const MAX_SLIDES: usize = 5_000;

/// Deepest group-shape nesting followed.
const MAX_GROUP_DEPTH: usize = 32;

/// Loads PowerPoint presentations.
#[derive(Clone, Copy, Debug, Default)]
pub struct PptxLoader;

impl Loader for PptxLoader {
    fn id(&self) -> &'static str {
        "pptx"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["pptx", "pptm", "ppsx", "potx"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, _options: &LoadOptions) -> Result<Document, LoadError> {
        let mut pkg = Package::open(source.read()?, "PowerPoint")?;
        let mut meta = meta_for(source, self.id());
        read_core(&mut pkg, &mut meta)?;
        let slides = slide_paths(&mut pkg)?;
        let mut b = Builder::new();
        for (i, path) in slides.iter().enumerate().take(MAX_SLIDES) {
            let Some(xml) = pkg.read_text(path)? else {
                continue;
            };
            let Ok(doc) = parse_xml(&xml) else {
                crate::add_warning(
                    &mut meta,
                    &format!("Slide {} could not be read and was skipped.", i + 1),
                );
                continue;
            };
            let notes = notes_path(&mut pkg, path)?;
            let notes_xml = match &notes {
                Some(p) => pkg.read_text(p)?,
                None => None,
            };
            slide(&mut b, i + 1, &doc, notes_xml.as_deref());
        }
        if slides.len() > MAX_SLIDES {
            crate::add_warning(
                &mut meta,
                &format!("Only the first {MAX_SLIDES} slides were read."),
            );
        }
        if pkg.flattened() {
            crate::add_warning(&mut meta, crate::NESTING_WARNING);
        }
        meta.properties
            .insert("slides".into(), slides.len().min(MAX_SLIDES).to_string());
        if meta.title.is_none() {
            meta.title = title_from_path(source);
        }
        let (text, markers) = b.finish();
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn read_core(pkg: &mut Package, meta: &mut textweaver_text::DocumentMeta) -> Result<(), LoadError> {
    let Some(core) = pkg.read_text("docProps/core.xml")? else {
        return Ok(());
    };
    let Ok(xml) = parse_xml(&core) else {
        return Ok(());
    };
    for n in xml.descendants().filter(|n| n.is_element()) {
        let t = collapse(n.text().unwrap_or_default());
        if t.is_empty() {
            continue;
        }
        match n.tag_name().name() {
            "title" => meta.title = Some(t),
            "creator" => meta.author = Some(t),
            "language" => meta.language = Some(t),
            _ => {}
        }
    }
    Ok(())
}

/// A part's relationships: id → target path (resolved).
fn rels(pkg: &mut Package, part: &str) -> Result<HashMap<String, (String, String)>, LoadError> {
    let dir = dir_of(part);
    let name = &part[dir.len()..];
    let rels_path = format!("{dir}_rels/{name}.rels");
    let mut out = HashMap::new();
    let Some(text) = pkg.read_text(&rels_path)? else {
        return Ok(out);
    };
    let Ok(xml) = parse_xml(&text) else {
        return Ok(out);
    };
    for r in xml
        .descendants()
        .filter(|n| n.tag_name().name() == "Relationship")
    {
        let (Some(id), Some(target)) = (r.attribute("Id"), r.attribute("Target")) else {
            continue;
        };
        if r.attribute("TargetMode") == Some("External") {
            continue;
        }
        let ty = r.attribute("Type").unwrap_or("");
        let kind = ty.rsplit('/').next().unwrap_or("").to_owned();
        out.insert(id.to_owned(), (resolve(dir, target).0, kind));
    }
    Ok(out)
}

/// The slides in presentation order.
fn slide_paths(pkg: &mut Package) -> Result<Vec<String>, LoadError> {
    let part = "ppt/presentation.xml";
    let text = pkg
        .read_text(part)?
        .ok_or_else(|| LoadError::Parse("not a PowerPoint file: no ppt/presentation.xml".into()))?;
    let xml = parse_xml(&text)?;
    let rels = rels(pkg, part)?;
    Ok(xml
        .descendants()
        .filter(|n| n.tag_name().name() == "sldId")
        .filter_map(|n| {
            n.attributes()
                .find(|a| a.name() == "id" && a.namespace().is_some())
                .and_then(|a| rels.get(a.value()))
                .map(|(p, _)| p.clone())
        })
        .collect())
}

fn notes_path(pkg: &mut Package, slide: &str) -> Result<Option<String>, LoadError> {
    Ok(rels(pkg, slide)?
        .into_values()
        .find(|(_, kind)| kind == "notesSlide")
        .map(|(p, _)| p))
}

fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

/// A shape's placeholder type (`title`, `body`, `sldNum`, ...), `""` for
/// a placeholder without one, `None` for a plain shape.
fn placeholder(shape: roxmltree::Node<'_, '_>) -> Option<String> {
    shape
        .descendants()
        .find(|n| n.tag_name().name() == "ph")
        .map(|ph| ph.attribute("type").unwrap_or("").to_owned())
}

/// One paragraph of a text body.
struct Para {
    text: String,
    level: u8,
    /// `Some(numbered)` when the paragraph is a list item.
    bullet: Option<bool>,
}

fn paragraphs(body: roxmltree::Node<'_, '_>, bulleted_body: bool) -> Vec<Para> {
    let mut out = Vec::new();
    for p in body.children().filter(|n| n.tag_name().name() == "p") {
        let mut text = String::new();
        for n in p.descendants() {
            match n.tag_name().name() {
                "t" if n.is_element() => text.push_str(n.text().unwrap_or("")),
                "br" => text.push('\n'),
                _ => {}
            }
        }
        let ppr = child(p, "pPr");
        let level = ppr
            .and_then(|p| p.attribute("lvl"))
            .and_then(|l| l.parse::<u8>().ok())
            .unwrap_or(0)
            .min(8);
        let has = |name: &str| ppr.is_some_and(|p| child(p, name).is_some());
        let bullet = if has("buNone") {
            None
        } else if has("buAutoNum") {
            Some(true)
        } else if has("buChar") || has("buBlip") || bulleted_body {
            Some(false)
        } else {
            None
        };
        if text.trim().is_empty() {
            continue;
        }
        out.push(Para {
            text,
            level,
            bullet,
        });
    }
    out
}

fn marker(kind: MarkerKind) -> Marker {
    Marker::new(kind, CharRange::empty(0))
}

/// Writes paragraphs: runs of list items as lists, the rest as paragraphs.
fn write_paragraphs(b: &mut Builder, paras: &[Para]) {
    let mut i = 0;
    while i < paras.len() {
        if paras[i].bullet.is_none() {
            b.paragraph_break();
            let id = b.open(marker(MarkerKind::Paragraph));
            for (n, line) in paras[i].text.split('\n').enumerate() {
                if n > 0 {
                    b.line_break();
                }
                b.text(line);
            }
            b.close(id);
            i += 1;
            continue;
        }
        b.paragraph_break();
        let list = b.open(marker(MarkerKind::List).with_level(1));
        let mut counters = [0u32; 9];
        while i < paras.len()
            && let Some(numbered) = paras[i].bullet
        {
            let p = &paras[i];
            let lvl = usize::from(p.level);
            counters[lvl] += 1;
            for c in counters.iter_mut().skip(lvl + 1) {
                *c = 0;
            }
            b.line_break();
            let mut m = marker(MarkerKind::ListItem).with_level(p.level + 1);
            if numbered {
                m = m.with_label(format!("{}.", counters[lvl]));
            }
            let item = b.open(m);
            b.text(&p.text.replace('\n', " "));
            b.close(item);
            i += 1;
        }
        b.close(list);
    }
}

fn table(b: &mut Builder, tbl: roxmltree::Node<'_, '_>) {
    b.paragraph_break();
    let t = b.open(marker(MarkerKind::Table));
    for (r, tr) in tbl
        .children()
        .filter(|n| n.tag_name().name() == "tr")
        .enumerate()
    {
        b.line_break();
        let mut rm = marker(MarkerKind::TableRow);
        if r == 0 {
            rm = rm.with_label(HEADER_ROW_LABEL);
        }
        let row = b.open(rm);
        for (c, tc) in tr
            .children()
            .filter(|n| n.tag_name().name() == "tc")
            .enumerate()
        {
            if c > 0 {
                b.separator(crate::CELL_SEPARATOR);
            }
            let cell = b.open_here(marker(MarkerKind::TableCell));
            let text: Vec<String> = tc
                .descendants()
                .filter(|n| n.tag_name().name() == "t" && n.is_element())
                .map(|n| n.text().unwrap_or("").to_owned())
                .collect();
            b.text(&text.join(" "));
            b.close(cell);
        }
        b.close(row);
    }
    b.close(t);
    b.paragraph_break();
}

/// The shapes of a tree (a slide's `spTree` or a group), in order.
fn shapes(b: &mut Builder, tree: roxmltree::Node<'_, '_>, depth: usize) {
    if depth > MAX_GROUP_DEPTH {
        return;
    }
    for shape in tree.children().filter(|n| n.is_element()) {
        match shape.tag_name().name() {
            "sp" => {
                let ph = placeholder(shape);
                if matches!(
                    ph.as_deref(),
                    Some("title" | "ctrTitle" | "sldNum" | "dt" | "ftr" | "hdr")
                ) {
                    continue;
                }
                let bulleted = matches!(ph.as_deref(), Some("body" | "obj" | ""));
                if let Some(body) = child(shape, "txBody") {
                    write_paragraphs(b, &paragraphs(body, bulleted));
                }
            }
            "pic" => {
                let props = shape.descendants().find(|n| n.tag_name().name() == "cNvPr");
                let alt = props
                    .and_then(|p| p.attribute("descr").or_else(|| p.attribute("title")))
                    .map(collapse)
                    .filter(|a| !a.is_empty());
                if let Some(alt) = alt {
                    b.paragraph_break();
                    let id = b.open(marker(MarkerKind::Image));
                    b.text(&alt);
                    b.close(id);
                }
            }
            "graphicFrame" => {
                if let Some(tbl) = shape.descendants().find(|n| n.tag_name().name() == "tbl") {
                    table(b, tbl);
                }
            }
            "grpSp" => shapes(b, shape, depth + 1),
            _ => {}
        }
    }
}

/// The text of a slide's title placeholder.
fn title_of(tree: roxmltree::Node<'_, '_>) -> Option<String> {
    tree.descendants()
        .filter(|n| n.tag_name().name() == "sp")
        .find(|sp| matches!(placeholder(*sp).as_deref(), Some("title" | "ctrTitle")))
        .and_then(|sp| child(sp, "txBody"))
        .map(|body| {
            paragraphs(body, false)
                .iter()
                .map(|p| collapse(&p.text))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|t| !t.is_empty())
}

fn slide(b: &mut Builder, n: usize, xml: &roxmltree::Document<'_>, notes: Option<&str>) {
    let tree = xml.descendants().find(|n| n.tag_name().name() == "spTree");
    let title = tree.and_then(title_of);
    let label = match &title {
        Some(t) => format!("Slide {n}: {t}"),
        None => format!("Slide {n}"),
    };
    b.paragraph_break();
    let section = b.open(
        marker(MarkerKind::SectionBreak)
            .with_level(1)
            .with_label(label),
    );
    let page = b.open(marker(MarkerKind::PageBreak).with_label(n.to_string()));
    let h = b.open(marker(MarkerKind::Heading).with_level(1));
    b.text(title.as_deref().unwrap_or(&format!("Slide {n}")));
    b.close(h);
    if let Some(tree) = tree {
        shapes(b, tree, 0);
    }
    if let Some(notes) = notes
        && let Ok(doc) = parse_xml(notes)
    {
        let paras: Vec<Para> = doc
            .descendants()
            .filter(|n| n.tag_name().name() == "sp")
            .filter(|sp| placeholder(*sp).as_deref() == Some("body"))
            .filter_map(|sp| child(sp, "txBody"))
            .flat_map(|body| paragraphs(body, false))
            .collect();
        if !paras.is_empty() {
            b.paragraph_break();
            let h = b.open(marker(MarkerKind::Heading).with_level(2));
            b.text("Speaker notes");
            b.close(h);
            write_paragraphs(b, &paras);
        }
    }
    b.close(page);
    b.close(section);
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use zip::write::SimpleFileOptions;

    use super::*;

    const P: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

    fn sp(ph: Option<&str>, paras: &str) -> String {
        let ph = ph.map_or(String::new(), |t| format!(r#"<p:ph type="{t}"/>"#));
        format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="s"/><p:cNvSpPr/><p:nvPr>{ph}</p:nvPr></p:nvSpPr><p:txBody>{paras}</p:txBody></p:sp>"#
        )
    }

    fn para(text: &str, extra: &str) -> String {
        format!(r#"<a:p>{extra}<a:r><a:t>{text}</a:t></a:r></a:p>"#)
    }

    pub(crate) fn pptx() -> Vec<u8> {
        let rel = |id: &str, ty: &str, target: &str| {
            format!(
                r#"<Relationship Id="{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/{ty}" Target="{target}"/>"#
            )
        };
        let rels = |items: String| {
            format!(
                r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{items}</Relationships>"#
            )
        };
        let slide1 = format!(
            r#"<p:sld {P}><p:cSld><p:spTree>{}{}{}<p:pic><p:nvPicPr><p:cNvPr id="4" name="p" descr="A bar chart of rainfall"/></p:nvPicPr></p:pic><p:pic><p:nvPicPr><p:cNvPr id="5" name="deco"/></p:nvPicPr></p:pic>{}</p:spTree></p:cSld></p:sld>"#,
            sp(Some("title"), &para("Weather", "")),
            sp(
                Some("body"),
                &[
                    para("Rain", ""),
                    para("Drizzle", r#"<a:pPr lvl="1"/>"#),
                    para("Sun", "")
                ]
                .concat()
            ),
            sp(None, &para("A plain text box.", "")),
            sp(Some("sldNum"), &para("1", "")),
        );
        let slide2 = format!(
            r#"<p:sld {P}><p:cSld><p:spTree>{}<p:graphicFrame><a:graphic><a:graphicData><a:tbl><a:tr><a:tc><a:txBody><a:p><a:r><a:t>City</a:t></a:r></a:p></a:txBody></a:tc><a:tc><a:txBody><a:p><a:r><a:t>mm</a:t></a:r></a:p></a:txBody></a:tc></a:tr><a:tr><a:tc><a:txBody><a:p><a:r><a:t>Oslo</a:t></a:r></a:p></a:txBody></a:tc><a:tc><a:txBody><a:p><a:r><a:t>763</a:t></a:r></a:p></a:txBody></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame></p:spTree></p:cSld></p:sld>"#,
            sp(
                None,
                &[
                    para("Steps", ""),
                    para(
                        "First",
                        r#"<a:pPr><a:buAutoNum type="arabicPeriod"/></a:pPr>"#
                    ),
                    para(
                        "Second",
                        r#"<a:pPr><a:buAutoNum type="arabicPeriod"/></a:pPr>"#
                    )
                ]
                .concat()
            ),
        );
        let notes1 = format!(
            r#"<p:notes {P}><p:cSld><p:spTree>{}{}</p:spTree></p:cSld></p:notes>"#,
            sp(Some("sldImg"), ""),
            sp(Some("body"), &para("Mention the flood of 2019.", "")),
        );
        let pres = format!(
            r#"<p:presentation {P}><p:sldIdLst><p:sldId id="257" r:id="rId3"/><p:sldId id="256" r:id="rId2"/></p:sldIdLst></p:presentation>"#
        );
        let files: Vec<(String, String)> = vec![
            ("ppt/presentation.xml".into(), pres),
            (
                "ppt/_rels/presentation.xml.rels".into(),
                rels(rel("rId2", "slide", "slides/slide2.xml") + &rel("rId3", "slide", "slides/slide1.xml")),
            ),
            ("ppt/slides/slide1.xml".into(), slide1),
            ("ppt/slides/slide2.xml".into(), slide2),
            (
                "ppt/slides/_rels/slide1.xml.rels".into(),
                rels(rel("rId1", "notesSlide", "../notesSlides/notesSlide1.xml")),
            ),
            ("ppt/notesSlides/notesSlide1.xml".into(), notes1),
            (
                "docProps/core.xml".into(),
                r#"<cp:coreProperties xmlns:cp="c" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Climate</dc:title><dc:creator>T. Teacher</dc:creator></cp:coreProperties>"#.into(),
            ),
        ];
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, body) in &files {
            z.start_file(name.as_str(), SimpleFileOptions::default())
                .unwrap();
            z.write_all(body.as_bytes()).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn slides_titles_lists_tables_and_notes() {
        let d = PptxLoader
            .load(
                &Source::Bytes {
                    data: pptx(),
                    hint: "pptx".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        assert_eq!(d.meta.title.as_deref(), Some("Climate"));
        assert_eq!(d.meta.author.as_deref(), Some("T. Teacher"));
        let text = d.text().to_string();
        assert_eq!(
            text,
            "Weather\n\nRain\nDrizzle\nSun\n\nA plain text box.\n\nA bar chart of rainfall\n\nSpeaker notes\n\nMention the flood of 2019.\n\nSlide 2\n\nSteps\n\nFirst\nSecond\n\nCity | mm\nOslo | 763"
        );
        let sections: Vec<String> = d
            .marker_index()
            .iter(MarkerKind::SectionBreak, None)
            .filter_map(|m| m.label.clone())
            .collect();
        assert_eq!(sections, ["Slide 1: Weather", "Slide 2"]);
        let pages: Vec<String> = d
            .marker_index()
            .iter(MarkerKind::PageBreak, None)
            .filter_map(|m| m.label.clone())
            .collect();
        assert_eq!(pages, ["1", "2"]);
        let items: Vec<(u8, Option<String>)> = d
            .marker_index()
            .iter(MarkerKind::ListItem, None)
            .map(|m| (m.level, m.label.clone()))
            .collect();
        assert_eq!(
            items,
            [
                (1, None),
                (2, None),
                (1, None),
                (1, Some("1.".into())),
                (1, Some("2.".into()))
            ]
        );
        assert_eq!(d.marker_index().iter(MarkerKind::Table, None).count(), 1);
        let headings: Vec<u8> = d
            .marker_index()
            .iter(MarkerKind::Heading, None)
            .map(|m| m.level)
            .collect();
        assert_eq!(headings, [1, 2, 1]);
    }
}
