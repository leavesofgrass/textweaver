//! Formatting citations and bibliographies with a CSL style (hayagriva).
//!
//! References are CSL-JSON; hayagriva formats its own `Entry` type (its
//! CSL-JSON input is behind a cargo feature this workspace does not enable,
//! see ADR-0019), so each reference is converted into hayagriva's data
//! model: a journal article becomes an `article` with a `periodical`
//! parent, a chapter a `chapter` with a `book` parent, and so on. Fields
//! hayagriva rejects (a malformed URL or language tag) are dropped one by
//! one with a log message rather than failing the whole reference.
//!
//! Output is plain text, Markdown, or HTML. None of them relies on visual
//! formatting alone: superscript citation numbers (AMA, Nature) are written
//! in brackets, `[1]`, so they are not read as part of the preceding word
//! ("reported1"), and markup in titles is removed before formatting.

use std::collections::HashMap;

use hayagriva::citationberg::{FontStyle, FontVariant, FontWeight, TextDecoration, VerticalAlign};
use hayagriva::{
    BibliographyDriver, BibliographyRequest, CitationItem, CitationRequest, CitePurpose, ElemChild,
    ElemChildren, ElemMeta, Entry, Formatting, LocatorPayload, SpecificLocator,
};
use serde_json::{Map, Value, json};

use crate::error::{CiteError, Result};
use crate::library::ReferenceSource;
use crate::pandoc::{Citation, CiteItem};
use crate::reference::{Name, Reference};
use crate::style::{CitationStyle, locales};
use crate::text::{escape_html, escape_markdown, to_format_string};

/// The markup of formatted output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Plain text: no markup, one line per entry.
    #[default]
    Plain,
    /// Markdown (CommonMark): `*italic*`, `**bold**`, links.
    Markdown,
    /// HTML fragments: `<i>`, `<b>`, `<a href>`.
    Html,
}

impl OutputFormat {
    /// The format for a name: `plain`/`text`, `markdown`/`md`, `html`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "plain" | "text" | "txt" => Some(OutputFormat::Plain),
            "markdown" | "md" => Some(OutputFormat::Markdown),
            "html" => Some(OutputFormat::Html),
            _ => None,
        }
    }
}

/// One formatted bibliography entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormattedEntry {
    /// The reference's key.
    pub key: String,
    /// The entry in the requested output format.
    pub text: String,
}

/// Every citation of a document, formatted together so numbering,
/// disambiguation ("2020a"), and "ibid." come out right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedDocument {
    /// One formatted citation per input citation, in the same order. For a
    /// note style these are the texts of the footnotes.
    pub citations: Vec<String>,
    /// The bibliography (references list) of the cited works.
    pub bibliography: Vec<FormattedEntry>,
    /// Keys cited but not found, in order of first use, without repeats.
    pub missing: Vec<String>,
    /// Whether the style puts citations in footnotes.
    pub note_style: bool,
}

/// Formats references with one style into one output format.
#[derive(Clone, Copy, Debug)]
pub struct Formatter<'s> {
    style: &'s CitationStyle,
    format: OutputFormat,
}

enum Plan {
    /// All items in one request.
    Whole(usize),
    /// Assembled from per-item parts.
    Parts(Vec<Part>),
}

enum Part {
    Rendered {
        request: usize,
        prefix: String,
        suffix: String,
    },
    Missing(String),
}

/// A flattened piece of rendered output.
enum Leaf<'a> {
    Text(&'a str, Formatting),
    Link(&'a str, Formatting, &'a str),
    BlockStart,
    BlockEnd,
}

impl Leaf<'_> {
    fn raised(&self) -> Option<VerticalAlign> {
        match self {
            Leaf::Text(_, f) | Leaf::Link(_, f, _) => {
                matches!(f.vertical_align, VerticalAlign::Sup | VerticalAlign::Sub)
                    .then_some(f.vertical_align)
            }
            _ => None,
        }
    }

    fn text(&self) -> &str {
        match self {
            Leaf::Text(t, _) | Leaf::Link(t, _, _) => t,
            _ => "",
        }
    }
}

fn flatten<'a>(children: &'a [ElemChild], out: &mut Vec<Leaf<'a>>) {
    for c in children {
        match c {
            ElemChild::Text(t) => out.push(Leaf::Text(&t.text, t.formatting)),
            ElemChild::Markup(m) => out.push(Leaf::Text(m, Formatting::default())),
            ElemChild::Link { text, url } => out.push(Leaf::Link(&text.text, text.formatting, url)),
            ElemChild::Elem(e) => {
                let block = e.display.is_some();
                if block {
                    out.push(Leaf::BlockStart);
                }
                flatten(&e.children.0, out);
                if block {
                    out.push(Leaf::BlockEnd);
                }
            }
            ElemChild::Transparent { .. } => {}
        }
    }
}

/// Removes the first names element (the author) from rendered output, for
/// `-@key` (suppress author).
fn remove_first_names(children: &mut Vec<ElemChild>) -> bool {
    for i in 0..children.len() {
        if let ElemChild::Elem(e) = &mut children[i] {
            if matches!(e.meta, Some(ElemMeta::Names(..))) {
                children.remove(i);
                return true;
            }
            if remove_first_names(&mut e.children.0) {
                return true;
            }
        }
    }
    false
}

impl<'s> Formatter<'s> {
    /// A formatter.
    pub fn new(style: &'s CitationStyle, format: OutputFormat) -> Self {
        Formatter { style, format }
    }

    /// The output format.
    pub fn format(&self) -> OutputFormat {
        self.format
    }

    /// Formats references as a bibliography, sorted and numbered as the
    /// style says (a numeric style numbers them in the given order).
    pub fn bibliography(&self, refs: &[&Reference]) -> Result<Vec<FormattedEntry>> {
        let entries: Vec<Entry> = refs.iter().map(|r| to_entry(r)).collect::<Result<_>>()?;
        let mut driver = BibliographyDriver::new();
        for e in &entries {
            let item = CitationItem::new(e, None, None, true, None);
            driver.citation(CitationRequest::new(
                vec![item],
                &self.style.csl,
                self.style.locale.clone(),
                locales(),
                None,
            ));
        }
        let rendered = driver.finish(BibliographyRequest::new(
            &self.style.csl,
            self.style.locale.clone(),
            locales(),
        ));
        Ok(self.bibliography_entries(rendered.bibliography))
    }

    /// One reference's bibliography entry.
    pub fn entry(&self, r: &Reference) -> Result<String> {
        let mut entries = self.bibliography(&[r])?;
        Ok(entries.pop().map(|e| e.text).unwrap_or_default())
    }

    /// One reference's in-text citation, as if cited alone: "(Doe &
    /// Roe, 2020)" in APA, "[1]" in IEEE, the full note in a note style.
    pub fn cite(&self, r: &Reference) -> Result<String> {
        let entry = to_entry(r)?;
        let mut driver = BibliographyDriver::new();
        driver.citation(CitationRequest::new(
            vec![CitationItem::with_entry(&entry)],
            &self.style.csl,
            self.style.locale.clone(),
            locales(),
            Some(1),
        ));
        let rendered = driver.finish(BibliographyRequest::new(
            &self.style.csl,
            self.style.locale.clone(),
            locales(),
        ));
        Ok(rendered
            .citations
            .first()
            .map(|c| self.children(&c.citation))
            .unwrap_or_default())
    }

    /// Formats one citation found in text, on its own.
    pub fn citation(&self, cite: &Citation, source: &dyn ReferenceSource) -> Result<String> {
        let doc = self.document(std::slice::from_ref(cite), source)?;
        Ok(doc.citations.into_iter().next().unwrap_or_default())
    }

    /// Formats every citation of a document and its bibliography.
    ///
    /// In a note style (Chicago notes), in-text (`@key`) and author-less
    /// (`-@key`) citations are formatted as ordinary notes, as Pandoc does:
    /// the note carries the full reference either way.
    pub fn document(
        &self,
        cites: &[Citation],
        source: &dyn ReferenceSource,
    ) -> Result<RenderedDocument> {
        // Convert each cited reference once.
        let mut entries: HashMap<String, Entry> = HashMap::new();
        let mut missing: Vec<String> = Vec::new();
        for item in cites.iter().flat_map(|c| &c.items) {
            if entries.contains_key(&item.key) || missing.contains(&item.key) {
                continue;
            }
            match source.get(&item.key) {
                Some(r) => {
                    entries.insert(item.key.clone(), to_entry(r)?);
                }
                None => missing.push(item.key.clone()),
            }
        }

        let style = &self.style.csl;
        let locale = self.style.locale.clone();
        let note = self.style.is_note_style();
        let mut driver = BibliographyDriver::new();
        let mut requests = 0usize;
        let mut suppressed: Vec<bool> = Vec::new();
        let mut plans: Vec<Plan> = Vec::with_capacity(cites.len());
        for (n, cite) in cites.iter().enumerate() {
            let note_number = note.then_some(n + 1);
            let narrative = cite.narrative && !note;
            let simple = cite.items.iter().all(|i| {
                i.prefix.is_empty()
                    && i.suffix.is_empty()
                    && (note || !i.suppress_author)
                    && entries.contains_key(&i.key)
            });
            if simple {
                let items = cite
                    .items
                    .iter()
                    .filter_map(|i| entries.get(&i.key).map(|e| citation_item(e, i, narrative)))
                    .collect();
                driver.citation(CitationRequest::new(
                    items,
                    style,
                    locale.clone(),
                    locales(),
                    note_number,
                ));
                plans.push(Plan::Whole(requests));
                suppressed.push(false);
                requests += 1;
                continue;
            }
            let mut parts = Vec::new();
            for i in &cite.items {
                let Some(e) = entries.get(&i.key) else {
                    parts.push(Part::Missing(i.key.clone()));
                    continue;
                };
                let suppress = i.suppress_author && !note;
                driver.citation(CitationRequest::new(
                    vec![citation_item(e, i, narrative || suppress)],
                    style,
                    locale.clone(),
                    locales(),
                    note_number,
                ));
                parts.push(Part::Rendered {
                    request: requests,
                    prefix: i.prefix.clone(),
                    suffix: i.suffix.clone(),
                });
                suppressed.push(suppress);
                requests += 1;
            }
            plans.push(Plan::Parts(parts));
        }
        let rendered = driver.finish(BibliographyRequest::new(style, locale, locales()));
        let texts: Vec<String> = rendered
            .citations
            .iter()
            .zip(&suppressed)
            .map(|(c, &suppress)| {
                if suppress {
                    // A prose citation ("Doe (2020)") without its names.
                    let mut children = c.citation.clone();
                    remove_first_names(&mut children.0);
                    self.children(&children)
                } else {
                    self.children(&c.citation)
                }
            })
            .collect();

        let layout = &style.citation.layout;
        let open = layout.prefix.clone().unwrap_or_default();
        let close = layout.suffix.clone().unwrap_or_default();
        let delimiter = layout.delimiter.clone().unwrap_or_else(|| "; ".to_owned());
        let citations = plans
            .into_iter()
            .map(|plan| match plan {
                Plan::Whole(i) => texts.get(i).cloned().unwrap_or_default(),
                Plan::Parts(parts) => {
                    let mut out = String::new();
                    for p in parts {
                        let (piece, has_prefix) = match p {
                            Part::Missing(key) => (self.missing_text(&key), false),
                            Part::Rendered {
                                request,
                                prefix,
                                suffix,
                            } => {
                                let t = texts.get(request).map(String::as_str).unwrap_or_default();
                                let t = strip_affixes(t, &open, &close).trim();
                                let mut piece = String::new();
                                if !prefix.is_empty() {
                                    piece.push_str(&self.escape(&prefix));
                                    if !t.is_empty() {
                                        piece.push(' ');
                                    }
                                }
                                piece.push_str(t);
                                if !suffix.is_empty() {
                                    if !suffix.starts_with([',', '.', ';', ':', ')'])
                                        && !piece.is_empty()
                                    {
                                        piece.push(' ');
                                    }
                                    piece.push_str(&self.escape(&suffix));
                                }
                                (piece, !prefix.is_empty())
                            }
                        };
                        if piece.trim().is_empty() {
                            continue;
                        }
                        if !out.is_empty() {
                            out.push_str(&delimiter);
                            // "(see 3, also 4)", not "(see 3,also 4)".
                            if has_prefix && !delimiter.ends_with(' ') {
                                out.push(' ');
                            }
                        }
                        out.push_str(&piece);
                    }
                    if out.is_empty() {
                        String::new()
                    } else {
                        format!("{open}{out}{close}")
                    }
                }
            })
            .collect();

        Ok(RenderedDocument {
            citations,
            bibliography: self.bibliography_entries(rendered.bibliography),
            missing,
            note_style: note,
        })
    }

    fn missing_text(&self, key: &str) -> String {
        let t = format!("missing reference {key}");
        match self.format {
            OutputFormat::Plain => t,
            OutputFormat::Markdown => format!("**{}**", escape_markdown(&t)),
            OutputFormat::Html => format!("<strong>{}</strong>", escape_html(&t)),
        }
    }

    fn escape(&self, s: &str) -> String {
        match self.format {
            OutputFormat::Plain => s.to_owned(),
            OutputFormat::Markdown => escape_markdown(s),
            OutputFormat::Html => escape_html(s),
        }
    }

    fn bibliography_entries(
        &self,
        bib: Option<hayagriva::RenderedBibliography>,
    ) -> Vec<FormattedEntry> {
        let Some(bib) = bib else {
            return Vec::new();
        };
        bib.items
            .into_iter()
            .map(|item| {
                let mut text = String::new();
                if let Some(first) = &item.first_field {
                    text.push_str(&self.children(&ElemChildren(vec![first.clone()])));
                    text.push(' ');
                }
                text.push_str(&self.children(&item.content));
                FormattedEntry {
                    key: item.key,
                    text: tidy(&text, self.format),
                }
            })
            .collect()
    }

    fn children(&self, children: &ElemChildren) -> String {
        let mut leaves = Vec::new();
        flatten(&children.0, &mut leaves);
        let mut out = String::new();
        let mut i = 0;
        while i < leaves.len() {
            // A run of raised (superscript or subscript) text is rendered as
            // one unit; numbers in it are bracketed so they are not heard as
            // part of the word before them ("reported [1]", not "reported1").
            if let Some(align) = leaves[i].raised() {
                let start = i;
                while i < leaves.len() && leaves[i].raised() == Some(align) {
                    i += 1;
                }
                let run: String = leaves[start..i].iter().map(Leaf::text).collect();
                self.raised_run(&run, align, &mut out);
                continue;
            }
            match &leaves[i] {
                Leaf::Text(t, f) => self.formatted(t, *f, &mut out),
                Leaf::Link(t, f, url) => self.link(t, *f, url, &mut out),
                Leaf::BlockStart => match self.format {
                    OutputFormat::Html => out.push_str("<div class=\"csl-block\">"),
                    _ => out.push(' '),
                },
                Leaf::BlockEnd => match self.format {
                    OutputFormat::Html => out.push_str("</div>"),
                    _ => out.push(' '),
                },
            }
            i += 1;
        }
        tidy(&out, self.format)
    }

    fn raised_run(&self, run: &str, align: VerticalAlign, out: &mut String) {
        let core = run.trim();
        if core.is_empty() {
            return;
        }
        let bracketed = core.chars().any(|c| c.is_ascii_digit());
        let body = if bracketed {
            format!("[{core}]")
        } else {
            core.to_owned()
        };
        match self.format {
            OutputFormat::Plain => out.push_str(&body),
            OutputFormat::Markdown => out.push_str(&escape_markdown(&body)),
            OutputFormat::Html => {
                let tag = if align == VerticalAlign::Sub {
                    "sub"
                } else {
                    "sup"
                };
                out.push_str(&format!("<{tag}>{}</{tag}>", escape_html(&body)));
            }
        }
    }

    fn link(&self, text: &str, f: Formatting, url: &str, out: &mut String) {
        match self.format {
            OutputFormat::Plain => self.formatted(text, f, out),
            OutputFormat::Markdown => {
                if text == url && !url.contains(['<', '>', ' ']) {
                    out.push('<');
                    out.push_str(url);
                    out.push('>');
                } else {
                    out.push('[');
                    self.formatted(text, f, out);
                    out.push_str("](");
                    out.push_str(&url.replace(' ', "%20").replace(')', "%29"));
                    out.push(')');
                }
            }
            OutputFormat::Html => {
                out.push_str("<a href=\"");
                out.push_str(&escape_html(url));
                out.push_str("\">");
                self.formatted(text, f, out);
                out.push_str("</a>");
            }
        }
    }

    fn formatted(&self, text: &str, f: Formatting, out: &mut String) {
        if text.is_empty() {
            return;
        }
        match self.format {
            OutputFormat::Plain => out.push_str(text),
            OutputFormat::Markdown => {
                let lead = &text[..text.len() - text.trim_start().len()];
                let trail = &text[text.trim_end().len()..];
                let core = text.trim();
                let mut marks = String::new();
                if matches!(f.font_weight, FontWeight::Bold) {
                    marks.push_str("**");
                }
                if matches!(f.font_style, FontStyle::Italic) {
                    marks.push('*');
                }
                out.push_str(lead);
                if !core.is_empty() {
                    out.push_str(&marks);
                    out.push_str(&escape_markdown(core));
                    out.extend(marks.chars().rev());
                }
                out.push_str(trail);
            }
            OutputFormat::Html => {
                let mut close: Vec<&str> = Vec::new();
                if matches!(f.font_style, FontStyle::Italic) {
                    out.push_str("<i>");
                    close.push("</i>");
                }
                if matches!(f.font_weight, FontWeight::Bold) {
                    out.push_str("<b>");
                    close.push("</b>");
                }
                if matches!(f.font_variant, FontVariant::SmallCaps) {
                    out.push_str("<span style=\"font-variant: small-caps\">");
                    close.push("</span>");
                }
                if matches!(f.text_decoration, TextDecoration::Underline) {
                    out.push_str("<u>");
                    close.push("</u>");
                }
                out.push_str(&escape_html(text));
                for c in close.into_iter().rev() {
                    out.push_str(c);
                }
            }
        }
    }
}

fn strip_affixes<'a>(t: &'a str, open: &str, close: &str) -> &'a str {
    let t = if open.is_empty() {
        t
    } else {
        t.strip_prefix(open).unwrap_or(t)
    };
    if close.is_empty() {
        t
    } else {
        t.strip_suffix(close).unwrap_or(t)
    }
}

/// Collapses runs of whitespace (block breaks become spaces in plain text
/// and Markdown) and trims.
fn tidy(s: &str, format: OutputFormat) -> String {
    let t = crate::text::collapse_whitespace(s);
    match format {
        OutputFormat::Html => t.replace("\"> ", "\">").replace(" </div>", "</div>"),
        _ => t,
    }
}

fn citation_item<'a>(e: &'a Entry, item: &'a CiteItem, prose: bool) -> CitationItem<'a, Entry> {
    let locator = item
        .locator
        .as_ref()
        .map(|l| SpecificLocator(l.label.csl(), LocatorPayload::Str(&l.value)));
    let purpose = prose.then_some(CitePurpose::Prose);
    CitationItem::new(e, locator, None, false, purpose)
}

// ---- CSL-JSON to hayagriva ----------------------------------------------------

/// hayagriva's entry type and container type for a CSL type.
fn hayagriva_types(r: &Reference) -> (&'static str, Option<&'static str>) {
    let container = r.container_title.is_some();
    match r.kind.as_str() {
        "article-journal" | "article-magazine" | "review" | "review-book" => {
            ("article", Some("periodical"))
        }
        "article-newspaper" => ("article", Some("newspaper")),
        "article" => ("article", container.then_some("periodical")),
        "paper-conference" => ("chapter", Some("book")),
        "chapter" => ("chapter", Some("book")),
        "entry-encyclopedia" | "entry-dictionary" | "entry" => ("chapter", Some("reference")),
        "book" | "classic" => ("book", None),
        "collection" => ("anthology", None),
        "periodical" => ("periodical", None),
        "report" => ("report", None),
        "thesis" => ("thesis", None),
        "webpage" => ("web", container.then_some("web")),
        "post-weblog" => ("article", Some("blog")),
        "post" => ("post", None),
        "manuscript" => ("manuscript", None),
        "software" => ("repository", None),
        "motion_picture" => ("video", None),
        "broadcast" => ("video", container.then_some("video")),
        "song" => ("audio", None),
        "patent" => ("patent", None),
        "legal_case" => ("case", None),
        "legislation" | "bill" => ("legislation", None),
        "graphic" => ("artwork", None),
        _ => ("misc", None),
    }
}

fn person(n: &Name) -> Value {
    if let Some(lit) = &n.literal {
        return json!({ "name": lit });
    }
    let mut family = n.family.clone().unwrap_or_default();
    if let Some(p) = &n.non_dropping_particle {
        family = format!("{p} {family}");
    }
    let mut m = Map::new();
    m.insert("name".into(), Value::String(family.trim().to_owned()));
    if let Some(g) = &n.given {
        m.insert("given-name".into(), Value::String(g.clone()));
    }
    if let Some(p) = &n.dropping_particle {
        m.insert("prefix".into(), Value::String(p.clone()));
    }
    if let Some(s) = &n.suffix {
        m.insert("suffix".into(), Value::String(s.clone()));
    }
    Value::Object(m)
}

fn persons(names: &[Name]) -> Value {
    Value::Array(names.iter().filter(|n| !n.is_empty()).map(person).collect())
}

fn fs(s: &str) -> Value {
    Value::String(to_format_string(s))
}

fn fs_short(long: &str, short: Option<&String>) -> Value {
    match short {
        Some(s) => json!({ "value": to_format_string(long), "short": to_format_string(s) }),
        None => fs(long),
    }
}

/// The hayagriva YAML/JSON form of a reference: the entry and its parent.
fn entry_value(r: &Reference) -> Map<String, Value> {
    let (ty, parent_ty) = hayagriva_types(r);
    let mut e = Map::new();
    let mut parent = Map::new();
    e.insert("type".into(), Value::String(ty.into()));
    if let Some(p) = parent_ty {
        parent.insert("type".into(), Value::String(p.into()));
    }
    let periodical = matches!(parent_ty, Some("periodical" | "newspaper"));
    let editors_on_parent = matches!(parent_ty, Some("book" | "proceedings" | "reference"));

    if let Some(t) = &r.title {
        e.insert("title".into(), fs_short(t, r.title_short.as_ref()));
    }
    if !r.author.is_empty() {
        e.insert("author".into(), persons(&r.author));
    }
    if !r.editor.is_empty() {
        let target = if editors_on_parent {
            &mut parent
        } else {
            &mut e
        };
        target.insert("editor".into(), persons(&r.editor));
    }
    if !r.translator.is_empty() {
        e.insert(
            "affiliated".into(),
            json!([{ "role": "translator", "names": persons(&r.translator) }]),
        );
    }
    if let Some(d) = r.issued.as_ref().and_then(|d| d.iso()) {
        e.insert("date".into(), Value::String(d));
    }
    if let Some(p) = &r.publisher {
        let mut m = Map::new();
        m.insert("name".into(), fs(p));
        if let Some(loc) = &r.publisher_place {
            m.insert("location".into(), fs(loc));
        }
        e.insert("publisher".into(), Value::Object(m));
    }
    for (name, value) in [("volume", &r.volume), ("issue", &r.issue)] {
        if let Some(v) = value {
            let target = if periodical { &mut parent } else { &mut e };
            target.insert(name.into(), Value::String(v.clone()));
        }
    }
    if let Some(v) = &r.edition {
        e.insert("edition".into(), Value::String(v.clone()));
    }
    if let Some(p) = &r.page {
        e.insert("page-range".into(), Value::String(p.clone()));
    }
    if let Some(u) = &r.url {
        let value = match r.accessed.as_ref().and_then(|d| d.iso()) {
            Some(date) => json!({ "value": u, "date": date }),
            None => Value::String(u.clone()),
        };
        e.insert("url".into(), value);
    }
    let mut serial = Map::new();
    for (name, value) in [
        ("doi", &r.doi),
        ("isbn", &r.isbn),
        ("issn", &r.issn),
        ("serial", &r.number),
    ] {
        if let Some(v) = value {
            serial.insert(name.into(), Value::String(v.clone()));
        }
    }
    if !serial.is_empty() {
        e.insert("serial-number".into(), Value::Object(serial));
    }
    for (name, value) in [
        ("language", &r.language),
        ("note", &r.note),
        ("abstract", &r.abstract_text),
        ("genre", &r.genre),
    ] {
        if let Some(v) = value {
            let v = if name == "language" {
                Value::String(v.clone())
            } else {
                fs(v)
            };
            e.insert(name.into(), v);
        }
    }
    if let Some(c) = &r.container_title {
        if parent_ty.is_some() {
            parent.insert(
                "title".into(),
                fs_short(c, r.container_title_short.as_ref()),
            );
        } else {
            // No container in hayagriva's model for this type: keep the
            // information where styles print it.
            e.entry("note").or_insert_with(|| fs(c));
        }
    }
    if let Some(series) = &r.collection_title {
        let series_value = json!({ "type": "book", "title": to_format_string(series) });
        let target = if parent_ty.is_some() {
            &mut parent
        } else {
            &mut e
        };
        target.insert("parent".into(), series_value);
    }
    if parent_ty.is_some() {
        e.insert("parent".into(), Value::Object(parent));
    }
    e
}

fn build(key: &str, e: &Map<String, Value>) -> std::result::Result<Entry, String> {
    let mut lib = Map::new();
    lib.insert(key.to_owned(), Value::Object(e.clone()));
    let lib: hayagriva::Library =
        serde_json::from_value(Value::Object(lib)).map_err(|err| err.to_string())?;
    lib.get(key)
        .cloned()
        .ok_or_else(|| "the entry was lost in conversion".to_owned())
}

/// Converts a reference into a hayagriva entry, dropping fields hayagriva
/// cannot parse.
pub(crate) fn to_entry(r: &Reference) -> Result<Entry> {
    let key = if r.id.is_empty() {
        "ref"
    } else {
        r.id.as_str()
    };
    let mut e = entry_value(r);
    if let Ok(entry) = build(key, &e) {
        return Ok(entry);
    }
    // Find and drop the fields hayagriva rejects.
    let ty = e
        .get("type")
        .cloned()
        .unwrap_or(Value::String("misc".into()));
    let names: Vec<String> = e.keys().filter(|k| *k != "type").cloned().collect();
    for name in names {
        let Some(value) = e.get(&name).cloned() else {
            continue;
        };
        if let (Value::Object(parent), "parent") = (&value, name.as_str()) {
            let mut clean = parent.clone();
            let ptype = parent
                .get("type")
                .cloned()
                .unwrap_or(Value::String("misc".into()));
            for pname in parent.keys().filter(|k| *k != "type") {
                let mut probe = Map::new();
                probe.insert("type".into(), ptype.clone());
                probe.insert(pname.clone(), parent[pname].clone());
                let mut probe_entry = Map::new();
                probe_entry.insert("type".into(), ty.clone());
                probe_entry.insert("parent".into(), Value::Object(probe));
                if let Err(err) = build(key, &probe_entry) {
                    log::warn!(
                        "reference {key}: dropping the container's {pname}, which the formatter rejects: {err}"
                    );
                    clean.remove(pname);
                }
            }
            e.insert(name, Value::Object(clean));
            continue;
        }
        let mut probe = Map::new();
        probe.insert("type".into(), ty.clone());
        probe.insert(name.clone(), value);
        if let Err(err) = build(key, &probe) {
            log::warn!("reference {key}: dropping {name}, which the formatter rejects: {err}");
            e.remove(&name);
        }
    }
    build(key, &e).map_err(|message| CiteError::Format {
        key: key.to_owned(),
        message,
    })
}
