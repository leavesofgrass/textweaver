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
    ElemChildren, Entry, Formatting, LocatorPayload, SpecificLocator,
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
        /// For `-@key`: the author text to strip from a prose rendering.
        strip_author: Option<String>,
    },
    Missing(String),
}

impl<'s> Formatter<'s> {
    /// A formatter.
    pub fn new(style: &'s CitationStyle, format: OutputFormat) -> Self {
        Formatter { style, format }
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
    /// Roe, 2020)" in APA, "[1]" in IEEE.
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
        let mut driver = BibliographyDriver::new();
        let mut requests = 0usize;
        let mut plans: Vec<Plan> = Vec::with_capacity(cites.len());
        let note = self.style.is_note_style();
        for (n, cite) in cites.iter().enumerate() {
            let note_number = note.then_some(n + 1);
            let simple = cite.items.iter().all(|i| {
                i.prefix.is_empty()
                    && i.suffix.is_empty()
                    && !i.suppress_author
                    && entries.contains_key(&i.key)
            });
            if simple {
                let items = cite
                    .items
                    .iter()
                    .filter_map(|i| {
                        entries
                            .get(&i.key)
                            .map(|e| citation_item(e, i, cite.narrative))
                    })
                    .collect();
                driver.citation(CitationRequest::new(
                    items,
                    style,
                    locale.clone(),
                    locales(),
                    note_number,
                ));
                plans.push(Plan::Whole(requests));
                requests += 1;
                continue;
            }
            let mut parts = Vec::new();
            for i in &cite.items {
                let Some(e) = entries.get(&i.key) else {
                    parts.push(Part::Missing(i.key.clone()));
                    continue;
                };
                let strip_author = if i.suppress_author {
                    Some(self.standalone_author(e))
                } else {
                    None
                };
                let prose = cite.narrative || i.suppress_author;
                driver.citation(CitationRequest::new(
                    vec![citation_item(e, i, prose)],
                    style,
                    locale.clone(),
                    locales(),
                    note_number,
                ));
                parts.push(Part::Rendered {
                    request: requests,
                    prefix: i.prefix.clone(),
                    suffix: i.suffix.clone(),
                    strip_author,
                });
                requests += 1;
            }
            plans.push(Plan::Parts(parts));
        }
        let rendered = driver.finish(BibliographyRequest::new(style, locale, locales()));
        let texts: Vec<String> = rendered
            .citations
            .iter()
            .map(|c| self.children(&c.citation))
            .collect();

        let layout = &style.citation.layout;
        let (open, close) = (
            layout.prefix.clone().unwrap_or_default(),
            layout.suffix.clone().unwrap_or_default(),
        );
        let delimiter = layout.delimiter.clone().unwrap_or_else(|| "; ".to_owned());
        let citations = plans
            .into_iter()
            .map(|plan| match plan {
                Plan::Whole(i) => texts.get(i).cloned().unwrap_or_default(),
                Plan::Parts(parts) => {
                    let pieces: Vec<String> = parts
                        .into_iter()
                        .map(|p| match p {
                            Part::Missing(key) => self.missing_text(&key),
                            Part::Rendered {
                                request,
                                prefix,
                                suffix,
                                strip_author,
                            } => {
                                let mut t = texts.get(request).cloned().unwrap_or_default();
                                if let Some(author) = strip_author {
                                    t = t
                                        .strip_prefix(author.as_str())
                                        .map(str::trim_start)
                                        .map(str::to_owned)
                                        .unwrap_or(t);
                                }
                                let t = strip_affixes(&t, &open, &close);
                                let mut piece = String::new();
                                if !prefix.is_empty() {
                                    piece.push_str(&self.escape(&prefix));
                                    piece.push(' ');
                                }
                                piece.push_str(t);
                                if !suffix.is_empty() {
                                    if !suffix.starts_with([',', '.', ';', ':', ')']) {
                                        piece.push(' ');
                                    }
                                    piece.push_str(&self.escape(&suffix));
                                }
                                piece
                            }
                        })
                        .collect();
                    format!("{open}{}{close}", pieces.join(&delimiter))
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

    fn standalone_author(&self, e: &Entry) -> String {
        let req = CitationRequest::new(
            vec![CitationItem::with_entry(e).kind(CitePurpose::Author)],
            &self.style.csl,
            self.style.locale.clone(),
            locales(),
            None,
        );
        self.children(&hayagriva::standalone_citation(req))
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
                    let mut f = String::new();
                    self.child(first, &mut f);
                    text.push_str(f.trim());
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
        let mut out = String::new();
        for c in &children.0 {
            self.child(c, &mut out);
        }
        tidy(&out, self.format)
    }

    fn child(&self, child: &ElemChild, out: &mut String) {
        match child {
            ElemChild::Text(t) => self.formatted(&t.text, t.formatting, out),
            ElemChild::Markup(m) => out.push_str(&self.escape(m)),
            ElemChild::Elem(e) => {
                let block = e.display.is_some();
                if block {
                    match self.format {
                        OutputFormat::Html => out.push_str("<div class=\"csl-block\">"),
                        _ => out.push(' '),
                    }
                }
                for c in &e.children.0 {
                    self.child(c, out);
                }
                if block {
                    match self.format {
                        OutputFormat::Html => out.push_str("</div>"),
                        _ => out.push(' '),
                    }
                }
            }
            ElemChild::Link { text, url } => match self.format {
                OutputFormat::Plain => self.formatted(&text.text, text.formatting, out),
                OutputFormat::Markdown => {
                    if text.text == *url && !url.contains(['<', '>', ' ']) {
                        out.push('<');
                        out.push_str(url);
                        out.push('>');
                    } else {
                        out.push('[');
                        self.formatted(&text.text, text.formatting, out);
                        out.push_str("](");
                        out.push_str(&url.replace(' ', "%20").replace(')', "%29"));
                        out.push(')');
                    }
                }
                OutputFormat::Html => {
                    out.push_str("<a href=\"");
                    out.push_str(&escape_html(url));
                    out.push_str("\">");
                    self.formatted(&text.text, text.formatting, out);
                    out.push_str("</a>");
                }
            },
            ElemChild::Transparent { .. } => {}
        }
    }

    fn formatted(&self, text: &str, f: Formatting, out: &mut String) {
        if text.is_empty() {
            return;
        }
        // Superscript and subscript numbers are bracketed so they are not
        // heard as part of the word before them.
        let numeric = text
            .trim()
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ',' | '-' | '–' | ' '));
        let raised = !matches!(
            f.vertical_align,
            VerticalAlign::None | VerticalAlign::Baseline
        );
        let body: String = if raised && numeric {
            format!("[{}]", text.trim())
        } else {
            text.to_owned()
        };
        match self.format {
            OutputFormat::Plain => out.push_str(&body),
            OutputFormat::Markdown => {
                let lead = &body[..body.len() - body.trim_start().len()];
                let trail = &body[body.trim_end().len()..];
                let core = body.trim();
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
                match f.vertical_align {
                    VerticalAlign::Sup => {
                        out.push_str("<sup>");
                        close.push("</sup>");
                    }
                    VerticalAlign::Sub => {
                        out.push_str("<sub>");
                        close.push("</sub>");
                    }
                    _ => {}
                }
                out.push_str(&escape_html(&body));
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
        "paper-conference" => ("article", Some("proceedings")),
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
