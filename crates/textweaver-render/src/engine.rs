//! The two parsers behind one event stream.
//!
//! Both engines produce `pulldown_cmark::Event`s; everything after parsing
//! (flavor extensions, math, heading ids, footnotes, the HTML writer) is
//! shared, so the choice of engine changes parsing fidelity and speed, never
//! the shape of the output. pulldown-cmark streams straight from the
//! source; comrak builds a full AST in an arena, which this module walks
//! into the same events.

use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment};
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, Options,
    Parser, Tag, TagEnd,
};

use crate::{Engine, Flavor};

/// Parses `src` with `engine` under `flavor`.
pub fn parse(src: &str, engine: Engine, flavor: Flavor, smart: bool) -> Vec<Event<'_>> {
    match engine {
        Engine::PulldownCmark => Parser::new_ext(src, pulldown_options(flavor, smart)).collect(),
        Engine::Comrak => comrak_events(src, flavor, smart),
    }
}

/// pulldown-cmark options for a flavor.
pub fn pulldown_options(flavor: Flavor, smart: bool) -> Options {
    let mut o = Options::empty();
    match flavor {
        Flavor::CommonMark => {}
        Flavor::Gfm | Flavor::Obsidian => {
            o |= Options::ENABLE_TABLES
                | Options::ENABLE_FOOTNOTES
                | Options::ENABLE_STRIKETHROUGH
                | Options::ENABLE_TASKLISTS
                | Options::ENABLE_MATH;
        }
        Flavor::Pandoc => {
            o |= Options::ENABLE_TABLES
                | Options::ENABLE_FOOTNOTES
                | Options::ENABLE_STRIKETHROUGH
                | Options::ENABLE_TASKLISTS
                | Options::ENABLE_MATH
                | Options::ENABLE_DEFINITION_LIST
                | Options::ENABLE_HEADING_ATTRIBUTES
                | Options::ENABLE_SUPERSCRIPT
                | Options::ENABLE_SUBSCRIPT;
        }
    }
    if smart {
        o |= Options::ENABLE_SMART_PUNCTUATION;
    }
    o
}

fn comrak_options(flavor: Flavor, smart: bool) -> comrak::Options<'static> {
    let mut o = comrak::Options::default();
    let e = &mut o.extension;
    match flavor {
        Flavor::CommonMark => {}
        Flavor::Gfm | Flavor::Obsidian => {
            e.table = true;
            e.strikethrough = true;
            e.autolink = true;
            e.tasklist = true;
            e.footnotes = true;
            e.math_dollars = true;
            if flavor == Flavor::Obsidian {
                e.highlight = true;
            }
        }
        Flavor::Pandoc => {
            e.table = true;
            e.strikethrough = true;
            e.tasklist = true;
            e.footnotes = true;
            e.superscript = true;
            e.subscript = true;
            e.description_lists = true;
            e.math_dollars = true;
        }
    }
    o.parse.smart = smart;
    // Raw HTML passes through, as with pulldown-cmark; sanitization is a
    // separate, explicit option.
    o.render.r#unsafe = true;
    o
}

fn comrak_events(src: &str, flavor: Flavor, smart: bool) -> Vec<Event<'static>> {
    let arena = comrak::Arena::new();
    let options = comrak_options(flavor, smart);
    let root = comrak::parse_document(&arena, src, &options);
    let mut w = Walker { out: Vec::new() };
    w.children(root, false);
    w.out
}

struct Walker {
    out: Vec<Event<'static>>,
}

fn owned(s: &str) -> CowStr<'static> {
    CowStr::from(s.to_owned())
}

fn heading_level(level: u8) -> HeadingLevel {
    match level {
        1 => HeadingLevel::H1,
        2 => HeadingLevel::H2,
        3 => HeadingLevel::H3,
        4 => HeadingLevel::H4,
        5 => HeadingLevel::H5,
        _ => HeadingLevel::H6,
    }
}

impl Walker {
    fn children<'a>(&mut self, node: &'a AstNode<'a>, tight: bool) {
        for child in node.children() {
            self.node(child, tight);
        }
    }

    fn wrap<'a>(&mut self, node: &'a AstNode<'a>, tag: Tag<'static>, tight: bool) {
        let end = tag.to_end();
        self.out.push(Event::Start(tag));
        self.children(node, tight);
        self.out.push(Event::End(end));
    }

    fn inline_html<'a>(&mut self, node: &'a AstNode<'a>, open: &'static str, close: &'static str) {
        self.out.push(Event::InlineHtml(CowStr::Borrowed(open)));
        self.children(node, false);
        self.out.push(Event::InlineHtml(CowStr::Borrowed(close)));
    }

    /// `tight`: this node is a direct child of an item in a tight list,
    /// whose paragraphs pulldown-cmark leaves unwrapped.
    fn node<'a>(&mut self, node: &'a AstNode<'a>, tight: bool) {
        let ast = node.data();
        match &ast.value {
            NodeValue::Document | NodeValue::Escaped => self.children(node, false),
            NodeValue::FrontMatter(_) => {}
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => {
                self.wrap(node, Tag::BlockQuote(None), false)
            }
            NodeValue::Alert(a) => {
                let kind = match a.alert_type {
                    comrak::nodes::AlertType::Note => BlockQuoteKind::Note,
                    comrak::nodes::AlertType::Tip => BlockQuoteKind::Tip,
                    comrak::nodes::AlertType::Important => BlockQuoteKind::Important,
                    comrak::nodes::AlertType::Warning => BlockQuoteKind::Warning,
                    comrak::nodes::AlertType::Caution => BlockQuoteKind::Caution,
                };
                self.wrap(node, Tag::BlockQuote(Some(kind)), false)
            }
            NodeValue::List(l) => {
                let start = match l.list_type {
                    ListType::Bullet => None,
                    ListType::Ordered => Some(l.start as u64),
                };
                let tag = Tag::List(start);
                let end = tag.to_end();
                self.out.push(Event::Start(tag));
                for item in node.children() {
                    self.node(item, l.tight);
                }
                self.out.push(Event::End(end));
            }
            NodeValue::Item(_) => self.wrap(node, Tag::Item, tight),
            NodeValue::TaskItem(t) => {
                self.out.push(Event::Start(Tag::Item));
                let checked = t.symbol.is_some_and(|c| c != ' ');
                self.out.push(Event::TaskListMarker(checked));
                self.children(node, tight);
                self.out.push(Event::End(TagEnd::Item));
            }
            NodeValue::DescriptionList => self.wrap(node, Tag::DefinitionList, false),
            NodeValue::DescriptionItem(item) => {
                let t = item.tight;
                self.children(node, t)
            }
            NodeValue::DescriptionTerm => self.wrap(node, Tag::DefinitionListTitle, true),
            NodeValue::DescriptionDetails => {
                self.wrap(node, Tag::DefinitionListDefinition, tight)
            }
            NodeValue::CodeBlock(cb) => {
                let kind = if cb.fenced {
                    CodeBlockKind::Fenced(owned(&cb.info))
                } else {
                    CodeBlockKind::Indented
                };
                self.out.push(Event::Start(Tag::CodeBlock(kind)));
                if !cb.literal.is_empty() {
                    self.out.push(Event::Text(owned(&cb.literal)));
                }
                self.out.push(Event::End(TagEnd::CodeBlock));
            }
            NodeValue::HtmlBlock(h) => {
                self.out.push(Event::Start(Tag::HtmlBlock));
                self.out.push(Event::Html(owned(&h.literal)));
                self.out.push(Event::End(TagEnd::HtmlBlock));
            }
            NodeValue::Raw(r) => self.out.push(Event::Html(owned(r))),
            NodeValue::Paragraph => {
                if tight {
                    self.children(node, false);
                } else {
                    self.wrap(node, Tag::Paragraph, false);
                }
            }
            NodeValue::Heading(h) => self.wrap(
                node,
                Tag::Heading {
                    level: heading_level(h.level),
                    id: None,
                    classes: Vec::new(),
                    attrs: Vec::new(),
                },
                false,
            ),
            NodeValue::ThematicBreak => self.out.push(Event::Rule),
            NodeValue::FootnoteDefinition(f) => {
                self.wrap(node, Tag::FootnoteDefinition(owned(&f.name)), false)
            }
            NodeValue::Table(t) => {
                let aligns = t
                    .alignments
                    .iter()
                    .map(|a| match a {
                        TableAlignment::None => Alignment::None,
                        TableAlignment::Left => Alignment::Left,
                        TableAlignment::Center => Alignment::Center,
                        TableAlignment::Right => Alignment::Right,
                    })
                    .collect();
                self.out.push(Event::Start(Tag::Table(aligns)));
                for row in node.children() {
                    let header = matches!(row.data().value, NodeValue::TableRow(true));
                    let tag = if header { Tag::TableHead } else { Tag::TableRow };
                    self.wrap(row, tag, false);
                }
                self.out.push(Event::End(TagEnd::Table));
            }
            NodeValue::TableRow(_) => self.wrap(node, Tag::TableRow, false),
            NodeValue::TableCell => self.wrap(node, Tag::TableCell, false),
            NodeValue::Text(t) => self.out.push(Event::Text(owned(t))),
            NodeValue::SoftBreak => self.out.push(Event::SoftBreak),
            NodeValue::LineBreak => self.out.push(Event::HardBreak),
            NodeValue::Code(c) => self.out.push(Event::Code(owned(&c.literal))),
            NodeValue::HtmlInline(h) => self.out.push(Event::InlineHtml(owned(h))),
            NodeValue::Emph => self.wrap(node, Tag::Emphasis, false),
            NodeValue::Strong => self.wrap(node, Tag::Strong, false),
            NodeValue::Strikethrough => self.wrap(node, Tag::Strikethrough, false),
            NodeValue::Superscript => self.wrap(node, Tag::Superscript, false),
            NodeValue::Subscript => self.wrap(node, Tag::Subscript, false),
            NodeValue::Underline => self.inline_html(node, "<u>", "</u>"),
            NodeValue::Highlight => self.inline_html(node, "<mark>", "</mark>"),
            NodeValue::Insert => self.inline_html(node, "<ins>", "</ins>"),
            NodeValue::SpoileredText => {
                self.inline_html(node, "<span class=\"spoiler\">", "</span>")
            }
            NodeValue::Subtext => self.inline_html(node, "<small>", "</small>"),
            NodeValue::Link(l) => self.wrap(
                node,
                Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url: owned(&l.url),
                    title: owned(&l.title),
                    id: CowStr::Borrowed(""),
                },
                false,
            ),
            NodeValue::Image(l) => self.wrap(
                node,
                Tag::Image {
                    link_type: LinkType::Inline,
                    dest_url: owned(&l.url),
                    title: owned(&l.title),
                    id: CowStr::Borrowed(""),
                },
                false,
            ),
            NodeValue::WikiLink(l) => self.wrap(
                node,
                Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url: owned(&l.url),
                    title: CowStr::Borrowed(""),
                    id: CowStr::Borrowed(""),
                },
                false,
            ),
            NodeValue::FootnoteReference(r) => {
                self.out.push(Event::FootnoteReference(owned(&r.name)))
            }
            NodeValue::Math(m) => {
                let lit = owned(&m.literal);
                self.out.push(if m.display_math {
                    Event::DisplayMath(lit)
                } else {
                    Event::InlineMath(lit)
                });
            }
            NodeValue::EscapedTag(t) => self.out.push(Event::Text(owned(t))),
            NodeValue::BlockDirective(d) => {
                self.out.push(Event::Html(CowStr::from(format!(
                    "<div class=\"{}\">\n",
                    crate::escape_html(d.info.trim())
                ))));
                self.children(node, false);
                self.out.push(Event::Html(CowStr::Borrowed("</div>\n")));
            }
        }
    }
}
