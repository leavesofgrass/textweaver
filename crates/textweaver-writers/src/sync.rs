//! An HTML5 body with sync spans: the document's blocks, as the EPUB
//! writer draws them, with every spoken sentence and word wrapped in a span
//! that a read-along page (and later EPUB media overlays) can point at.
//!
//! Sentence `n` is drawn as one or more `<span class="tw-s" data-s="n">`
//! fragments (a sentence can run across bold text, a link, or a list item
//! boundary), the first with `id="s{n}"`. Word `k` (numbered over all the
//! sentences' words in order) is a `<span class="tw-w" data-w="k">` inside
//! its sentence, the first fragment with `id="w{k}"`. Text the speech did
//! not cover (a code block skipped, alt text) has no spans.
//!
//! The block tree drops character ranges, so each text run is found again
//! in the document's canonical text, moving forward from the last run
//! found; whitespace may differ (runs are collapsed in the tree).

use std::collections::HashSet;

use textweaver_core::CharRange;
use textweaver_text::Document;

use crate::math::Formula;
use crate::model::{self, Block, Image, Inline, List, Style, Table};
use crate::xml;

/// One sentence as heard, for [`body`]: its document range and the
/// document ranges of its words, in order. `None` for speech with no
/// document text; it still takes its number.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncSentence {
    /// The document chars the sentence covers.
    pub range: Option<CharRange>,
    /// The document chars of each word.
    pub words: Vec<Option<CharRange>>,
}

/// The document's blocks as an HTML5 body fragment with sync spans for
/// `sentences` (see the module docs for the ids). Images are drawn as
/// their descriptions, so the page needs no other file.
pub fn body(doc: &Document, sentences: &[SyncSentence]) -> String {
    let mut sents = Vec::new();
    let mut words = Vec::new();
    let mut k = 0usize;
    for (n, s) in sentences.iter().enumerate() {
        if let Some(r) = s.range.filter(|r| !r.is_empty()) {
            sents.push((r.start.0, r.end.0, n));
        }
        for w in &s.words {
            if let Some(r) = w.filter(|r| !r.is_empty()) {
                words.push((r.start.0, r.end.0, k));
            }
            k += 1;
        }
    }
    sents.sort_unstable();
    words.sort_unstable();
    let mut b = Body {
        doc: doc.text().chars().collect(),
        cursor: 0,
        out: String::new(),
        sents,
        words,
        seen_s: HashSet::new(),
        seen_w: HashSet::new(),
        headings: 0,
    };
    b.blocks(&model::blocks(doc));
    b.out
}

struct Body {
    doc: Vec<char>,
    /// Where the next text run is looked for.
    cursor: usize,
    out: String,
    /// (start, end, sentence number), by start.
    sents: Vec<(usize, usize, usize)>,
    /// (start, end, word number), by start.
    words: Vec<(usize, usize, usize)>,
    seen_s: HashSet<usize>,
    seen_w: HashSet<usize>,
    headings: usize,
}

/// The entry of `table` holding `pos`, if any.
fn holding(table: &[(usize, usize, usize)], pos: usize) -> Option<usize> {
    let i = table.partition_point(|&(start, _, _)| start <= pos);
    let &(_, end, n) = table.get(i.checked_sub(1)?)?;
    (pos < end).then_some(n)
}

impl Body {
    fn blocks(&mut self, blocks: &[Block]) {
        for b in blocks {
            self.block(b);
        }
    }

    fn block(&mut self, b: &Block) {
        match b {
            Block::Heading { level, content } => {
                self.headings += 1;
                let l = (*level).clamp(1, 6);
                self.out
                    .push_str(&format!("<h{l} id=\"h-{}\">", self.headings));
                self.inlines(content);
                self.out.push_str(&format!("</h{l}>\n"));
            }
            Block::Paragraph(content) => {
                self.out.push_str("<p>");
                self.inlines(content);
                self.out.push_str("</p>\n");
            }
            Block::List(list) => self.list(list),
            Block::Table(table) => self.table(table),
            Block::Code { text, .. } => {
                self.out.push_str("<pre><code>");
                self.text(text);
                self.out.push_str("</code></pre>\n");
            }
            Block::Quote(inner) => {
                self.out.push_str("<blockquote>\n");
                self.blocks(inner);
                self.out.push_str("</blockquote>\n");
            }
            Block::Figure(img) => {
                if !img.alt.is_empty() {
                    self.out.push_str("<p>");
                    self.image(img);
                    self.out.push_str("</p>\n");
                }
            }
            Block::Footnote { id, content } => {
                self.out.push_str(&format!(
                    "<aside role=\"doc-footnote\" id=\"{}\"><p>",
                    xml::id("fn-", id)
                ));
                self.inlines(content);
                self.out.push_str("</p></aside>\n");
            }
            Block::SectionBreak { .. } | Block::Rule => self.out.push_str("<hr>\n"),
            Block::PageBreak { label } => {
                if let Some(l) = label.as_deref().filter(|l| !l.trim().is_empty()) {
                    self.out.push_str(&format!(
                        "<span role=\"doc-pagebreak\" aria-label=\"{}\"></span>\n",
                        xml::attr(l)
                    ));
                }
            }
        }
    }

    fn list(&mut self, list: &List) {
        let tag = if list.ordered { "ol" } else { "ul" };
        if list.ordered && list.start != 1 {
            self.out
                .push_str(&format!("<ol start=\"{}\">\n", list.start));
        } else {
            self.out.push_str(&format!("<{tag}>\n"));
        }
        for item in &list.items {
            self.out.push_str("<li>");
            let mut rest: &[Block] = &item.blocks;
            if let Some(Block::Paragraph(content)) = rest.first()
                && rest
                    .iter()
                    .filter(|b| matches!(b, Block::Paragraph(_)))
                    .count()
                    <= 1
            {
                self.inlines(content);
                rest = &rest[1..];
            }
            self.blocks(rest);
            self.out.push_str("</li>\n");
        }
        self.out.push_str(&format!("</{tag}>\n"));
    }

    fn table(&mut self, table: &Table) {
        self.out.push_str("<table>\n");
        if let Some(c) = &table.caption {
            self.out.push_str("<caption>");
            self.text(c);
            self.out.push_str("</caption>\n");
        }
        for row in &table.rows {
            self.out.push_str("<tr>");
            for cell in &row.cells {
                let open = if row.header {
                    "<th scope=\"col\">"
                } else {
                    "<td>"
                };
                self.out.push_str(open);
                self.inlines(cell);
                self.out
                    .push_str(if row.header { "</th>" } else { "</td>" });
            }
            self.out.push_str("</tr>\n");
        }
        self.out.push_str("</table>\n");
    }

    fn image(&mut self, img: &Image) {
        self.out.push_str(&format!(
            "<span role=\"img\" aria-label=\"{}\">{}</span>",
            xml::attr(&img.alt),
            xml::text(&img.alt)
        ));
    }

    fn inlines(&mut self, inlines: &[Inline]) {
        for i in inlines {
            match i {
                Inline::Text(t) => self.text(t),
                Inline::LineBreak => self.out.push_str("<br>"),
                Inline::Span(style, children) => self.span(style, children),
            }
        }
    }

    fn span(&mut self, style: &Style, children: &[Inline]) {
        let wrap = |me: &mut Self, open: &str, close: &str| {
            me.out.push_str(open);
            me.inlines(children);
            me.out.push_str(close);
        };
        match style {
            Style::Bold => wrap(self, "<strong>", "</strong>"),
            Style::Italic => wrap(self, "<em>", "</em>"),
            Style::Underline => wrap(self, "<u>", "</u>"),
            Style::Code => wrap(self, "<code>", "</code>"),
            Style::Strikethrough => wrap(self, "<del>", "</del>"),
            Style::Math { display } => {
                let f = Formula::from_marked(&Inline::plain(children), *display);
                self.out.push_str(&f.mathml());
            }
            Style::Link(target) => {
                let t = target.trim();
                if t.starts_with("http://") || t.starts_with("https://") {
                    let open = format!("<a href=\"{}\">", xml::attr(t));
                    wrap(self, &open, "</a>");
                } else {
                    self.inlines(children);
                }
            }
            Style::FootnoteRef(id) => {
                let open = format!("<a role=\"doc-noteref\" href=\"#{}\">", xml::id("fn-", id));
                wrap(self, &open, "</a>");
            }
            Style::Image(_) => {
                let alt = model::collapse_ws(&Inline::plain(children));
                if !alt.is_empty() {
                    self.image(&Image { alt, src: None });
                }
            }
        }
    }

    /// Where each char of `t` is in the document, if found.
    fn locate(&mut self, t: &[char]) -> Vec<Option<usize>> {
        let mut at = vec![None; t.len()];
        let first: Vec<char> = t
            .iter()
            .copied()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| !c.is_whitespace())
            .collect();
        if first.is_empty() || first.len() > self.doc.len() {
            return at;
        }
        let Some(mut c) = (self.cursor..=self.doc.len() - first.len())
            .find(|&i| self.doc[i..i + first.len()] == first[..])
        else {
            return at;
        };
        let lead = t.iter().take_while(|c| c.is_whitespace()).count();
        for (i, &ch) in t.iter().enumerate().skip(lead) {
            if ch.is_whitespace() {
                if self.doc.get(c).is_some_and(|d| d.is_whitespace()) {
                    at[i] = Some(c);
                    while self.doc.get(c).is_some_and(|d| d.is_whitespace()) {
                        c += 1;
                    }
                }
                continue;
            }
            let mut d = c;
            while self.doc.get(d).is_some_and(|x| x.is_whitespace()) {
                d += 1;
            }
            if self.doc.get(d) == Some(&ch) {
                at[i] = Some(d);
                c = d + 1;
            }
        }
        self.cursor = c;
        at
    }

    /// Writes a text run with its sync spans.
    fn text(&mut self, t: &str) {
        let chars: Vec<char> = t.chars().collect();
        let at = self.locate(&chars);
        let mut open: (Option<usize>, Option<usize>) = (None, None);
        let mut run = String::new();
        for (ch, pos) in chars.iter().zip(&at) {
            let s = pos.and_then(|p| holding(&self.sents, p));
            let w = s.and(pos.and_then(|p| holding(&self.words, p)));
            if (s, w) != open {
                self.out.push_str(&xml::text(&run));
                run.clear();
                if open.1.is_some() {
                    self.out.push_str("</span>");
                }
                if open.0 != s {
                    if open.0.is_some() {
                        self.out.push_str("</span>");
                    }
                    if let Some(n) = s {
                        let id = if self.seen_s.insert(n) {
                            format!(" id=\"s{n}\"")
                        } else {
                            String::new()
                        };
                        self.out
                            .push_str(&format!("<span class=\"tw-s\" data-s=\"{n}\"{id}>"));
                    }
                }
                if let Some(k) = w {
                    let id = if self.seen_w.insert(k) {
                        format!(" id=\"w{k}\"")
                    } else {
                        String::new()
                    };
                    self.out
                        .push_str(&format!("<span class=\"tw-w\" data-w=\"{k}\"{id}>"));
                }
                open = (s, w);
            }
            run.push(*ch);
        }
        self.out.push_str(&xml::text(&run));
        if open.1.is_some() {
            self.out.push_str("</span>");
        }
        if open.0.is_some() {
            self.out.push_str("</span>");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_core::CharPos;

    fn r(a: usize, b: usize) -> Option<CharRange> {
        Some(CharRange::new(CharPos(a), CharPos(b)))
    }

    #[test]
    fn sentences_and_words_get_spans_inside_the_blocks() {
        let doc = textweaver_text::Document::from_plain_text("Plants use light. Roots drink.");
        let sentences = [
            SyncSentence {
                range: r(0, 17),
                words: vec![r(0, 6), r(7, 10), r(11, 16)],
            },
            SyncSentence {
                range: r(18, 30),
                words: vec![r(18, 23), r(24, 29)],
            },
        ];
        let html = body(&doc, &sentences);
        assert!(html.starts_with("<p><span class=\"tw-s\" data-s=\"0\" id=\"s0\">"));
        for k in 0..5 {
            assert!(html.contains(&format!("id=\"w{k}\"")), "w{k}: {html}");
        }
        assert!(html.contains("<span class=\"tw-w\" data-w=\"2\" id=\"w2\">light</span>."));
        assert!(html.contains("id=\"s1\""));
    }

    #[test]
    fn a_sentence_across_bold_text_is_split_into_fragments() {
        use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};
        let doc = MarkdownLoader
            .load(
                &Source::Bytes {
                    data: b"Plants **use light** today.".to_vec(),
                    hint: "md".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        let n = doc.len_chars();
        let html = body(
            &doc,
            &[SyncSentence {
                range: r(0, n),
                words: vec![],
            }],
        );
        assert_eq!(html.matches("data-s=\"0\"").count(), 3, "{html}");
        assert_eq!(html.matches("id=\"s0\"").count(), 1, "{html}");
        assert!(html.contains("<strong>"), "{html}");
    }
}
