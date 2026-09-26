//! The shared pass over parser events: inline extensions, math, heading
//! ids and the table of contents, Obsidian block ids and embeds, and
//! footnotes gathered into an accessible endnotes section; then the HTML
//! writer.

use std::collections::HashMap;

use pulldown_cmark::{CowStr, Event, Tag, TagEnd};

use crate::inline::{self, Ctx, Inline, Seg};
use crate::slug::{Slugger, slugify};
use crate::{EmbedMode, Flavor, RenderOptions, Resolver, TocEntry, escape_html, math};

/// What the pass learns about the document besides its HTML.
#[derive(Debug, Default)]
pub struct Facts {
    /// Headings with ids, in order.
    pub toc: Vec<TocEntry>,
    /// `#tags` found.
    pub tags: Vec<String>,
    /// True when any formula was rendered.
    pub has_math: bool,
    /// Text of the first level-1 heading.
    pub first_h1: Option<String>,
}

/// Deepest nesting of inlined embeds (a note embedding itself stops here).
const MAX_EMBED_DEPTH: usize = 4;

/// Runs the pass and writes HTML.
pub fn to_html(
    events: Vec<Event<'_>>,
    opts: &RenderOptions,
    resolver: Option<&dyn Resolver>,
    depth: usize,
) -> (String, Facts) {
    let inline = inline_for(opts);
    // Merged runs matter only to the text extensions (and the block ids,
    // embeds, and heading attributes that come with them).
    let events = if inline.any() {
        merge_text(events)
    } else {
        events
    };
    let mut pass = Pass {
        opts,
        resolver,
        depth,
        inline,
        slugger: Slugger::default(),
        facts: Facts::default(),
        out: Vec::with_capacity(events.len()),
        note: None,
        notes: Vec::new(),
        note_numbers: HashMap::new(),
        note_refs: HashMap::new(),
        link_depth: 0,
        code_depth: 0,
        para_html: Vec::new(),
    };
    // The endnotes heading's id; a heading with the same text gets a suffix.
    pass.slugger.reserve("footnote-label");
    pass.run(events);
    let mut html = String::with_capacity(pass.out.len() * 16);
    let out = std::mem::take(&mut pass.out);
    pulldown_cmark::html::push_html(&mut html, out.into_iter());
    pass.write_footnotes(&mut html);
    (html, pass.facts)
}

/// The inline extensions a flavor turns on.
fn inline_for(opts: &RenderOptions) -> Inline {
    let pulldown = opts.engine == crate::Engine::PulldownCmark;
    match opts.flavor {
        Flavor::CommonMark => Inline::default(),
        Flavor::Gfm => Inline {
            autolinks: pulldown,
            ..Inline::default()
        },
        Flavor::Obsidian => Inline {
            autolinks: pulldown,
            wikilinks: true,
            tags: true,
            highlights: true,
            ..Inline::default()
        },
        Flavor::Pandoc => Inline {
            citations: true,
            spans: true,
            subsup: true,
            ..Inline::default()
        },
    }
}

/// Joins adjacent text events (parsers split text at special characters),
/// in one pass: a run of k pieces is copied once, not k times.
fn merge_text(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut out: Vec<Event<'_>> = Vec::with_capacity(events.len());
    let mut pending: Option<String> = None;
    for e in events {
        match e {
            Event::Text(t) => {
                if let Some(buf) = pending.as_mut() {
                    buf.push_str(&t);
                } else if let Some(Event::Text(prev)) = out.last() {
                    let mut buf = String::with_capacity(prev.len() + t.len() + 32);
                    buf.push_str(prev);
                    buf.push_str(&t);
                    out.pop();
                    pending = Some(buf);
                } else {
                    out.push(Event::Text(t));
                }
            }
            other => {
                if let Some(buf) = pending.take() {
                    out.push(Event::Text(CowStr::from(buf)));
                }
                out.push(other);
            }
        }
    }
    if let Some(buf) = pending {
        out.push(Event::Text(CowStr::from(buf)));
    }
    out
}

struct Pass<'o, 'a> {
    opts: &'o RenderOptions,
    resolver: Option<&'o dyn Resolver>,
    depth: usize,
    inline: Inline,
    slugger: Slugger,
    facts: Facts,
    out: Vec<Event<'a>>,
    /// The footnote definition being collected: its name and events.
    note: Option<(String, Vec<Event<'a>>)>,
    /// Collected definitions in document order.
    notes: Vec<(String, Vec<Event<'a>>)>,
    /// Footnote numbers, assigned at first reference.
    note_numbers: HashMap<String, usize>,
    /// How many times each footnote has been referenced.
    note_refs: HashMap<String, usize>,
    link_depth: usize,
    code_depth: usize,
    /// For each open paragraph: true when its start was written as HTML
    /// (it carries an id) and its end must be too.
    para_html: Vec<bool>,
}

impl<'a> Pass<'_, 'a> {
    fn emit(&mut self, e: Event<'a>) {
        match &mut self.note {
            Some((_, v)) => v.push(e),
            None => self.out.push(e),
        }
    }

    fn run(&mut self, mut ev: Vec<Event<'a>>) {
        let mut i = 0;
        while i < ev.len() {
            let e = std::mem::replace(&mut ev[i], Event::SoftBreak);
            match e {
                Event::Start(Tag::Heading {
                    level,
                    id,
                    classes,
                    attrs,
                }) => {
                    let end = find_end(&ev, i, |t| matches!(t, TagEnd::Heading(_)));
                    let (id, classes) = self.heading_id(&mut ev[i + 1..end], id, classes);
                    let text = plain_text(&ev[i + 1..end]);
                    if level == pulldown_cmark::HeadingLevel::H1 && self.facts.first_h1.is_none() {
                        self.facts.first_h1 = Some(text.clone());
                    }
                    if let Some(id) = &id
                        && self.note.is_none()
                    {
                        self.facts.toc.push(TocEntry {
                            level: level as u8,
                            id: id.to_string(),
                            text,
                        });
                    }
                    self.emit(Event::Start(Tag::Heading {
                        level,
                        id,
                        classes,
                        attrs,
                    }));
                }
                Event::Start(Tag::Paragraph) => {
                    let end = find_end(&ev, i, |t| matches!(t, TagEnd::Paragraph));
                    if self.inline.wikilinks {
                        if let Some(html) = self.block_embed(&ev[i + 1..end]) {
                            self.emit(Event::Html(CowStr::from(html)));
                            i = end + 1;
                            continue;
                        }
                        if let Some(id) = take_block_id(&mut ev[i + 1..end]) {
                            self.emit(Event::Html(CowStr::from(format!(
                                "<p id=\"block-{}\">",
                                escape_html(&id)
                            ))));
                            self.para_html.push(true);
                            i += 1;
                            continue;
                        }
                    }
                    self.para_html.push(false);
                    self.emit(Event::Start(Tag::Paragraph));
                }
                Event::End(TagEnd::Paragraph) => {
                    if self.para_html.pop() == Some(true) {
                        self.emit(Event::Html(CowStr::Borrowed("</p>\n")));
                    } else {
                        self.emit(Event::End(TagEnd::Paragraph));
                    }
                }
                Event::Start(tag @ (Tag::Link { .. } | Tag::Image { .. })) => {
                    self.link_depth += 1;
                    self.emit(Event::Start(tag));
                }
                Event::End(end @ (TagEnd::Link | TagEnd::Image)) => {
                    self.link_depth = self.link_depth.saturating_sub(1);
                    self.emit(Event::End(end));
                }
                Event::Start(tag @ Tag::CodeBlock(_)) => {
                    self.code_depth += 1;
                    self.emit(Event::Start(tag));
                }
                Event::End(TagEnd::CodeBlock) => {
                    self.code_depth = self.code_depth.saturating_sub(1);
                    self.emit(Event::End(TagEnd::CodeBlock));
                }
                Event::Start(Tag::FootnoteDefinition(name)) => {
                    self.note = Some((name.to_string(), Vec::new()));
                }
                Event::End(TagEnd::FootnoteDefinition) => {
                    if let Some(note) = self.note.take() {
                        self.notes.push(note);
                    }
                }
                Event::FootnoteReference(name) => {
                    let html = self.footnote_ref(&name);
                    self.emit(Event::InlineHtml(CowStr::from(html)));
                }
                Event::InlineMath(src) => {
                    let html = self.math(&src, false);
                    self.emit(Event::InlineHtml(CowStr::from(html)));
                }
                Event::DisplayMath(src) => {
                    let html = self.math(&src, true);
                    self.emit(Event::InlineHtml(CowStr::from(html)));
                }
                Event::Text(text) if self.code_depth == 0 && self.link_depth == 0 => {
                    let mut ctx = Ctx {
                        link_ext: &self.opts.wikilink_extension,
                        tags: &mut self.facts.tags,
                    };
                    match inline::transform(&text, self.inline, &mut ctx) {
                        None => self.emit(Event::Text(text)),
                        Some(segs) => {
                            for s in segs {
                                self.emit(match s {
                                    Seg::Text(t) => Event::Text(CowStr::from(t)),
                                    Seg::Html(h) => Event::InlineHtml(CowStr::from(h)),
                                });
                            }
                        }
                    }
                }
                other => self.emit(other),
            }
            i += 1;
        }
        if let Some(note) = self.note.take() {
            self.notes.push(note);
        }
    }

    /// The heading's id: an explicit one (pulldown's `{#id}` or a trailing
    /// Pandoc attribute block left in the text by comrak), else a slug.
    fn heading_id(
        &mut self,
        inner: &mut [Event<'a>],
        id: Option<CowStr<'a>>,
        mut classes: Vec<CowStr<'a>>,
    ) -> (Option<CowStr<'a>>, Vec<CowStr<'a>>) {
        let mut explicit = id.map(|i| i.to_string());
        if explicit.is_none()
            && self.opts.flavor == Flavor::Pandoc
            && let Some(Event::Text(last)) = inner.last_mut()
            && let Some(open) = last.rfind(" {")
            && last.trim_end().ends_with('}')
        {
            let attrs = last[open + 2..last.trim_end().len() - 1].to_owned();
            for token in attrs.split_whitespace() {
                if let Some(i) = token.strip_prefix('#') {
                    explicit = Some(i.to_owned());
                } else if let Some(c) = token.strip_prefix('.') {
                    classes.push(CowStr::from(c.to_owned()));
                }
            }
            let kept = last[..open].to_owned();
            *last = CowStr::from(kept);
        }
        if let Some(id) = explicit {
            return (Some(CowStr::from(self.slugger.reserve(&id))), classes);
        }
        if !self.opts.heading_ids {
            return (None, classes);
        }
        let text = plain_text(inner);
        (Some(CowStr::from(self.slugger.unique(&text))), classes)
    }

    fn math(&mut self, src: &str, display: bool) -> String {
        if self.opts.math {
            self.facts.has_math = true;
            math::to_mathml(src, display)
        } else {
            let delim = if display { "$$" } else { "$" };
            format!("{delim}{}{delim}", escape_html(src))
        }
    }

    fn footnote_ref(&mut self, name: &str) -> String {
        let next = self.note_numbers.len() + 1;
        let n = *self.note_numbers.entry(name.to_owned()).or_insert(next);
        let count = self.note_refs.entry(name.to_owned()).or_insert(0);
        *count += 1;
        let slug = slugify(name);
        let suffix = if *count > 1 {
            format!("-{count}")
        } else {
            String::new()
        };
        format!(
            "<sup class=\"footnote-ref\"><a href=\"#fn-{slug}\" id=\"fnref-{slug}{suffix}\" role=\"doc-noteref\" aria-describedby=\"footnote-label\">{n}</a></sup>"
        )
    }

    /// Footnote definitions as one endnotes section, numbered by first
    /// reference (unreferenced notes follow, in document order), each with
    /// links back to its references.
    fn write_footnotes(&mut self, html: &mut String) {
        if self.notes.is_empty() {
            return;
        }
        let mut notes = std::mem::take(&mut self.notes);
        let unnumbered = self.note_numbers.len() + 1;
        notes.sort_by_key(|(name, _)| self.note_numbers.get(name).copied().unwrap_or(unnumbered));
        html.push_str("<section class=\"footnotes\" role=\"doc-endnotes\" aria-labelledby=\"footnote-label\">\n<h2 id=\"footnote-label\">Footnotes</h2>\n<ol>\n");
        for (name, events) in notes {
            let slug = slugify(&name);
            let mut inner = String::new();
            pulldown_cmark::html::push_html(&mut inner, events.into_iter());
            let refs = self.note_refs.get(&name).copied().unwrap_or(0);
            let n = self.note_numbers.get(&name).copied().unwrap_or(0);
            let mut back = String::new();
            for k in 1..=refs {
                let suffix = if k > 1 {
                    format!("-{k}")
                } else {
                    String::new()
                };
                let label = if refs > 1 {
                    format!("Back to reference {n}, occurrence {k}")
                } else {
                    format!("Back to reference {n}")
                };
                back.push_str(&format!(
                    " <a href=\"#fnref-{slug}{suffix}\" class=\"footnote-backref\" role=\"doc-backlink\" aria-label=\"{label}\">\u{21a9}</a>"
                ));
            }
            let inner = inner.trim_end();
            html.push_str(&format!("<li id=\"fn-{slug}\">\n"));
            match inner.strip_suffix("</p>") {
                Some(body) => {
                    html.push_str(body);
                    html.push_str(&back);
                    html.push_str("</p>");
                }
                None => {
                    html.push_str(inner);
                    if !back.is_empty() {
                        html.push_str(&format!("<p>{}</p>", back.trim_start()));
                    }
                }
            }
            html.push_str("\n</li>\n");
        }
        html.push_str("</ol>\n</section>\n");
    }

    /// A paragraph that is only `![[Note]]` becomes the note's rendered
    /// content when embeds are inlined and the note can be found.
    fn block_embed(&mut self, inner: &[Event<'a>]) -> Option<String> {
        if self.opts.embeds != EmbedMode::Inline || self.depth >= MAX_EMBED_DEPTH {
            return None;
        }
        let [Event::Text(text)] = inner else {
            return None;
        };
        let target = text.trim().strip_prefix("![[")?.strip_suffix("]]")?;
        let target = target.split('|').next().unwrap_or(target).trim();
        let (page, fragment) = match target.split_once('#') {
            Some((p, f)) => (p.trim(), Some(f.trim())),
            None => (target, None),
        };
        if page.is_empty() || page.rsplit_once('.').is_some_and(|(_, e)| e != "md") {
            return None;
        }
        let source = self.resolver?.resolve(page)?;
        let section = match fragment {
            Some(f) => extract_section(&source, f)?,
            None => source,
        };
        let (_, body) = crate::frontmatter::split(&section, false);
        let rendered = crate::render_at(body, self.opts, self.resolver, self.depth + 1);
        Some(format!(
            "<div class=\"embed\" role=\"region\" aria-label=\"Embedded note: {}\">\n{}</div>\n",
            escape_html(page),
            rendered.html
        ))
    }
}

/// Index of the end event matching the start at `start`.
fn find_end(ev: &[Event<'_>], start: usize, is_end: impl Fn(&TagEnd) -> bool) -> usize {
    ev[start + 1..]
        .iter()
        .position(|e| matches!(e, Event::End(t) if is_end(t)))
        .map_or(ev.len(), |p| start + 1 + p)
}

/// The text a heading or paragraph reads as, without markup.
fn plain_text(ev: &[Event<'_>]) -> String {
    let mut s = String::new();
    for e in ev {
        match e {
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) | Event::DisplayMath(t) => {
                s.push_str(t)
            }
            Event::SoftBreak | Event::HardBreak => s.push(' '),
            _ => {}
        }
    }
    s
}

/// Removes a trailing Obsidian block id (` ^abc-1`) from a paragraph and
/// returns it.
fn take_block_id(inner: &mut [Event<'_>]) -> Option<String> {
    let Some(Event::Text(last)) = inner.last_mut() else {
        return None;
    };
    let trimmed = last.trim_end();
    let caret = trimmed.rfind('^')?;
    let id = &trimmed[caret + 1..];
    let before = &trimmed[..caret];
    if id.is_empty()
        || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        || !(before.is_empty() || before.ends_with(char::is_whitespace))
    {
        return None;
    }
    let id = slugify(id);
    let kept = before.trim_end().to_owned();
    *last = CowStr::from(kept);
    Some(id)
}

/// The part of a note a fragment names: `Heading` (the heading and
/// everything up to the next heading of the same or a higher level) or
/// `^id` (the paragraph carrying that block id).
pub fn extract_section(md: &str, fragment: &str) -> Option<String> {
    if let Some(id) = fragment.strip_prefix('^') {
        let marker = format!("^{id}");
        for para in md.split("\n\n") {
            if para.trim_end().ends_with(&marker) {
                return Some(para.trim().to_owned());
            }
        }
        return None;
    }
    let want = slugify(fragment);
    let lines: Vec<&str> = md.lines().collect();
    let level_of = |l: &str| {
        let hashes = l.bytes().take_while(|&b| b == b'#').count();
        (1..=6).contains(&hashes) && l[hashes..].starts_with(' ')
    };
    let mut start = None;
    let mut level = 0;
    for (n, l) in lines.iter().enumerate() {
        if !level_of(l) {
            continue;
        }
        let hashes = l.bytes().take_while(|&b| b == b'#').count();
        match start {
            None if slugify(l[hashes..].trim()) == want => {
                start = Some(n);
                level = hashes;
            }
            Some(s) if hashes <= level => return Some(lines[s..n].join("\n")),
            _ => {}
        }
    }
    start.map(|s| lines[s..].join("\n"))
}
