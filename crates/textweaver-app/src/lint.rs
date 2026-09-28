//! Markdown lint for a blind author (Agent W4g, ADR-0032): problems a
//! sighted author sees at a glance and a listener does not.
//!
//! Ctrl+F8 and Ctrl+Shift+F8 in edit mode move to the next and previous
//! problem, select it, and say it: "Lint: heading level 3 after level 1;
//! use level 2." The rules:
//!
//! - **Heading levels** (`heading-level`): a heading more than one level
//!   below the heading before it skips a level, which screen readers'
//!   heading lists and the outline show as a gap.
//! - **List markers** (`list-marker`): a bullet list that changes its
//!   marker (`-`, `*`, `+`) from one item to the next; Markdown starts a
//!   new list there.
//! - **Trailing spaces** (`trailing-space`): spaces or tabs at the end of
//!   a line, except exactly two before more text, which is Markdown's line
//!   break.
//! - **Link references** (`link-reference`): `[text][ref]` or `[ref][]`
//!   with no `[ref]: address` anywhere, which shows as plain brackets.
//! - **Bare web addresses** (`bare-url`): an address in running text,
//!   which stays plain text in some renderers and is long to hear; put it
//!   in angle brackets (`<https://…>`) or a link.
//!
//! Code blocks, inline code, math, front matter, and raw HTML are not
//! checked, except for trailing spaces outside code blocks.
//!
//! These are textweaver's own rules on the Markdown parser the app already
//! uses (pulldown-cmark), not rumdl: rumdl 0.2.77 as a library brings 189
//! crates (tokio, rayon, and a configuration system) for five rules, and
//! its 0.2 series releases several times a week with no API promise
//! (ADR-0032).

use pulldown_cmark::{BrokenLink, Event, LinkType, Parser, Tag, TagEnd};
use textweaver_core::{CharRange, Direction};
use textweaver_editor::Selection;
use textweaver_speech::Earcon;

use crate::app::App;
use crate::structure::{ByteToChar, parser_options};
use crate::text_util;

/// One lint problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintProblem {
    /// The rule's name (`heading-level`).
    pub rule: &'static str,
    /// Where, in chars of the source.
    pub range: CharRange,
    /// What is wrong and what to do, as said after "Lint: ".
    pub message: String,
}

/// A problem found in bytes, before conversion to chars.
struct Found {
    rule: &'static str,
    start: usize,
    end: usize,
    message: String,
}

/// Every lint problem of Markdown `source`, in order.
pub fn lint_markdown(source: &str) -> Vec<LintProblem> {
    let mut found: Vec<Found> = Vec::new();
    let mut broken: Vec<(usize, usize, String)> = Vec::new();
    // Byte ranges of code blocks, math, front matter, and HTML blocks.
    let mut code: Vec<(usize, usize)> = Vec::new();
    {
        let mut callback = |link: BrokenLink<'_>| {
            if matches!(link.link_type, LinkType::Reference | LinkType::Collapsed) {
                broken.push((link.span.start, link.span.end, link.reference.to_string()));
            }
            None
        };
        let parser =
            Parser::new_with_broken_link_callback(source, parser_options(), Some(&mut callback));
        let mut last_heading: Option<u8> = None;
        let mut in_link = 0usize;
        let mut skip = 0usize;
        for (event, range) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    let level = level as u8;
                    if let Some(prev) = last_heading
                        && level > prev + 1
                    {
                        found.push(Found {
                            rule: "heading-level",
                            start: range.start,
                            end: heading_line_end(source, range.start, range.end),
                            message: format!(
                                "heading level {level} after level {prev}; use level {}.",
                                prev + 1
                            ),
                        });
                    }
                    last_heading = Some(level);
                }
                Event::Start(Tag::CodeBlock(_) | Tag::MetadataBlock(_) | Tag::HtmlBlock) => {
                    skip += 1;
                    code.push((range.start, range.end));
                }
                Event::End(TagEnd::CodeBlock | TagEnd::MetadataBlock(_) | TagEnd::HtmlBlock) => {
                    skip = skip.saturating_sub(1);
                }
                Event::Start(Tag::Link { .. } | Tag::Image { .. }) => in_link += 1,
                Event::End(TagEnd::Link | TagEnd::Image) => in_link = in_link.saturating_sub(1),
                Event::DisplayMath(_) => code.push((range.start, range.end)),
                Event::Text(text) if skip == 0 && in_link == 0 => {
                    bare_urls(&text, range.start, source, &mut found);
                }
                _ => {}
            }
        }
    }
    for (start, end, reference) in broken {
        found.push(Found {
            rule: "link-reference",
            start,
            end,
            message: format!("link reference {reference} has no definition."),
        });
    }
    list_markers(source, &code, &mut found);
    trailing_spaces(source, &code, &mut found);
    found.sort_by_key(|f| (f.start, f.end));
    found.dedup_by(|b, a| a.start == b.start && a.rule == b.rule);
    let conv = ByteToChar::new(
        source,
        found.iter().flat_map(|f| [f.start, f.end]).collect(),
    );
    found
        .into_iter()
        .map(|f| LintProblem {
            rule: f.rule,
            range: CharRange::new(conv.get(f.start), conv.get(f.end)),
            message: f.message,
        })
        .collect()
}

/// The end of a heading's first line: the heading's text without a setext
/// underline.
fn heading_line_end(source: &str, start: usize, end: usize) -> usize {
    source[start..end]
        .find('\n')
        .map_or(end, |i| start + i)
        .max(start)
}

/// True when byte `at` is inside one of `ranges`.
fn inside(ranges: &[(usize, usize)], at: usize) -> bool {
    ranges.iter().any(|&(a, b)| a <= at && at < b)
}

/// Bare web addresses in a text event starting at byte `base`. The event's
/// text is the source's own when it matches; otherwise (entities, escapes)
/// nothing is reported, since its offsets would not line up.
fn bare_urls(text: &str, base: usize, source: &str, out: &mut Vec<Found>) {
    if source.get(base..base + text.len()) != Some(text) {
        return;
    }
    let mut from = 0;
    while let Some(i) = ["https://", "http://", "www."]
        .iter()
        .filter_map(|p| text[from..].find(p))
        .min()
    {
        let start = from + i;
        // Only at a word's start: not `xwww.` or inside another address.
        if start > 0
            && text[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '/' || c == '<')
        {
            from = start + 4;
            continue;
        }
        let len = text[start..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == ')')
            .unwrap_or(text.len() - start);
        let end = start + len;
        let shown = text[start..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
        let end = start + shown.len();
        if shown.len() > 4 {
            out.push(Found {
                rule: "bare-url",
                start: base + start,
                end: base + end,
                message:
                    "bare web address; put it in angle brackets or make it a link with a name."
                        .to_owned(),
            });
        }
        from = end.max(start + 1);
    }
}

/// The bullet of a list item line (`-`, `*`, `+`), its indent, and the
/// byte where the marker is.
fn bullet(line: &str) -> Option<(char, usize)> {
    let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
    let rest = &line[indent..];
    let mut chars = rest.chars();
    let marker = chars.next()?;
    if !matches!(marker, '-' | '*' | '+') {
        return None;
    }
    // A thematic break (`***`, `- - -`) is not an item.
    if rest
        .chars()
        .filter(|c| !c.is_whitespace())
        .all(|c| c == marker)
        && rest.chars().filter(|&c| c == marker).count() >= 3
    {
        return None;
    }
    matches!(chars.next(), Some(' ' | '\t') | None).then_some((marker, indent))
}

/// Bullet items that change their marker within one list: consecutive
/// item lines at the same indent, with only blank lines, deeper lines, or
/// continuation text between them.
fn list_markers(source: &str, code: &[(usize, usize)], out: &mut Vec<Found>) {
    // (indent, marker) of the current item at each indent.
    let mut open: Vec<(usize, char)> = Vec::new();
    let mut at = 0usize;
    for line in source.split_inclusive('\n') {
        let start = at;
        at += line.len();
        if inside(code, start) {
            continue;
        }
        let text = line.trim_end_matches(['\n', '\r']);
        if text.trim().is_empty() {
            continue;
        }
        match bullet(text) {
            Some((marker, indent)) => {
                open.retain(|&(i, _)| i <= indent);
                match open.last() {
                    Some(&(i, m)) if i == indent && m != marker => {
                        out.push(Found {
                            rule: "list-marker",
                            start: start + indent,
                            end: start + indent + 1,
                            message: format!(
                                "list marker {}; this list uses {}.",
                                marker_name(marker),
                                marker_name(m)
                            ),
                        });
                    }
                    Some(&(i, _)) if i == indent => {}
                    _ => open.push((indent, marker)),
                }
            }
            None => {
                // Text at the left edge ends every list.
                let indent = text.len() - text.trim_start().len();
                if indent == 0 && !text.trim_start().starts_with(|c: char| c.is_ascii_digit()) {
                    open.clear();
                }
            }
        }
    }
}

/// A bullet's spoken name.
fn marker_name(c: char) -> &'static str {
    match c {
        '-' => "dash",
        '*' => "star",
        '+' => "plus",
        _ => "other",
    }
}

/// Spaces or tabs at the end of lines, outside code blocks. Exactly two
/// spaces before a line of text is a Markdown line break and is left
/// alone.
fn trailing_spaces(source: &str, code: &[(usize, usize)], out: &mut Vec<Found>) {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut at = 0usize;
    for (i, line) in lines.iter().enumerate() {
        let start = at;
        at += line.len();
        if inside(code, start) {
            continue;
        }
        let text = line.trim_end_matches(['\n', '\r']);
        let trimmed = text.trim_end_matches([' ', '\t']);
        let n = text.len() - trimmed.len();
        if n == 0 {
            continue;
        }
        let spaces = &text[trimmed.len()..];
        let next_has_text = lines.get(i + 1).is_some_and(|l| !l.trim().is_empty());
        if spaces == "  " && !trimmed.trim().is_empty() && next_has_text {
            continue;
        }
        let what = match (spaces.contains('\t'), n) {
            (true, _) => "tabs or spaces".to_owned(),
            (false, 1) => "1 space".to_owned(),
            (false, n) => format!("{n} spaces"),
        };
        let message = if trimmed.trim().is_empty() {
            format!("{what} on an empty line.")
        } else {
            format!("{what} at the end of the line.")
        };
        out.push(Found {
            rule: "trailing-space",
            start: start + trimmed.len(),
            end: start + text.len(),
            message,
        });
    }
}

impl App {
    /// Every lint problem of the Markdown source being edited.
    fn lint_problems(&self) -> Vec<LintProblem> {
        self.session
            .as_ref()
            .map(|s| lint_markdown(&s.doc.text().to_string()))
            .unwrap_or_default()
    }

    /// Ctrl+F8 and Ctrl+Shift+F8: the next or previous lint problem, in
    /// edit mode on Markdown; selects it and says it.
    pub(crate) fn lint_step(&mut self, dir: Direction) {
        if self.edit.is_none() {
            let k = self.keys(textweaver_keymap::ActionId::ToggleEditMode);
            self.tell(&format!(
                "Lint checks the Markdown you write. Turn on edit mode with {k} first."
            ));
            return;
        }
        if !self.authoring.structure.markdown {
            self.tell("Lint checks Markdown, and this document is not Markdown.");
            return;
        }
        let Some(pos) = self.session.as_ref().map(|s| s.cursor) else {
            return;
        };
        // Going back starts before the problem that is selected now.
        let selected = self
            .edit
            .as_ref()
            .and_then(|e| e.session.editor())
            .map(|ed| ed.selection().range());
        let from = selected.map_or(pos, |r| r.start.min(pos));
        let all = self.lint_problems();
        let found = match dir {
            // A problem right at the caret counts (the document's start).
            Direction::Forward => all.iter().find(|p| {
                p.range.start > pos || (p.range.start == pos && selected != Some(p.range))
            }),
            Direction::Backward => all.iter().rev().find(|p| p.range.start < from),
        }
        .cloned();
        let Some(p) = found else {
            self.speech.earcon(Earcon::Boundary);
            let msg = if all.is_empty() {
                "No lint problems.".to_owned()
            } else {
                format!(
                    "No {} lint problem. {} in all.",
                    if dir == Direction::Forward {
                        "more"
                    } else {
                        "earlier"
                    },
                    problems_phrase(all.len())
                )
            };
            self.tell(&msg);
            return;
        };
        self.stop_speech();
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            let sel = if p.range.is_empty() {
                Selection::caret(p.range.start)
            } else {
                Selection::new(p.range.start, p.range.end)
            };
            ed.set_selection(sel);
            self.after_edit(&ropey::Rope::new(), &[]);
        }
        self.scroll_to_cursor();
        let line = self
            .session
            .as_ref()
            .map_or(0, |s| text_util::line_of(&s.doc, p.range.start) + 1);
        let mut msg = format!("Lint: {}", p.message);
        if self.settings.speech.verbosity >= textweaver_a11y::Verbosity::High {
            msg.push_str(&format!(" Line {line}."));
        }
        self.tell(&msg);
    }
}

/// "1 lint problem", "3 lint problems".
fn problems_phrase(n: usize) -> String {
    if n == 1 {
        "1 lint problem".to_owned()
    } else {
        format!("{} lint problems", textweaver_editor::echo::thousands(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_core::CharPos;

    fn rules(src: &str) -> Vec<(&'static str, String)> {
        lint_markdown(src)
            .into_iter()
            .map(|p| {
                let text: String = src
                    .chars()
                    .skip(p.range.start.0)
                    .take(p.range.len())
                    .collect();
                (p.rule, text)
            })
            .collect()
    }

    #[test]
    fn heading_levels_that_skip() {
        let src = "# Title\n\n### Too deep\n\n## Fine\n\n#### Too deep again\n";
        let found = lint_markdown(src);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(
            found[0].message,
            "heading level 3 after level 1; use level 2."
        );
        assert_eq!(rules(src)[0].1, "### Too deep");
        // Going back up is fine.
        assert!(lint_markdown("## A\n\n# B\n\n## C\n").is_empty());
    }

    #[test]
    fn list_markers_that_change() {
        let src = "- one\n- two\n* three\n\n  - nested\n  - nested two\n\nText.\n\n* new list\n";
        let found = rules(src);
        assert_eq!(found, [("list-marker", "*".to_owned())], "{found:?}");
        let p = &lint_markdown(src)[0];
        assert_eq!(p.message, "list marker star; this list uses dash.");
        // A thematic break is not an item.
        assert!(lint_markdown("- a\n\n***\n").is_empty());
    }

    #[test]
    fn trailing_spaces_but_not_line_breaks() {
        let src = "One \nTwo  \nThree\nFour\t\n   \n```\ncode   \n```\n";
        let found = rules(src);
        assert_eq!(
            found,
            [
                ("trailing-space", " ".to_owned()),
                ("trailing-space", "\t".to_owned()),
                ("trailing-space", "   ".to_owned()),
            ],
            "{found:?}"
        );
        let msgs: Vec<String> = lint_markdown(src).into_iter().map(|p| p.message).collect();
        assert_eq!(msgs[0], "1 space at the end of the line.");
        assert_eq!(msgs[2], "3 spaces on an empty line.");
    }

    #[test]
    fn link_references_without_definitions() {
        let src =
            "See [the intro][intro] and [notes][] and [fine][ok].\n\n[ok]: https://example.org\n";
        let found = lint_markdown(src);
        let refs: Vec<&str> = found
            .iter()
            .filter(|p| p.rule == "link-reference")
            .map(|p| p.message.as_str())
            .collect();
        assert_eq!(
            refs,
            [
                "link reference intro has no definition.",
                "link reference notes has no definition."
            ]
        );
        // A shortcut `[word]` may be plain brackets or a citation: not
        // reported.
        assert!(lint_markdown("A [note] and [@doe2020].\n").is_empty());
    }

    #[test]
    fn bare_urls_but_not_links_or_code() {
        let src = "Go to https://example.org/page. Or www.example.com, or <https://ok.org>, [a](https://ok.org), `https://code.org`.\n";
        let found = rules(src);
        assert_eq!(
            found,
            [
                ("bare-url", "https://example.org/page".to_owned()),
                ("bare-url", "www.example.com".to_owned()),
            ],
            "{found:?}"
        );
    }

    #[test]
    fn positions_are_chars() {
        let src = "# Café\n\n### Été\n";
        let p = &lint_markdown(src)[0];
        assert_eq!(p.range.start, CharPos(8));
    }

    #[test]
    fn the_practice_fixture_has_one_of_each() {
        let src = include_str!("../../../fixtures/g/lint.md");
        let found: Vec<&str> = lint_markdown(src).iter().map(|p| p.rule).collect();
        assert_eq!(
            found,
            [
                "heading-level",
                "trailing-space",
                "bare-url",
                "list-marker",
                "link-reference"
            ]
        );
    }

    #[test]
    fn a_clean_document_has_none() {
        let src = "# Title\n\nSome text with a [link](https://example.org).\n\n## Part\n\n- one\n- two\n\nA line  \nbreak.\n";
        assert!(lint_markdown(src).is_empty(), "{:?}", lint_markdown(src));
    }
}
