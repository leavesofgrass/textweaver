//! Block extensions neither engine has, rewritten at the source level into
//! raw HTML blocks that both engines pass through while still parsing the
//! Markdown inside them:
//!
//! - callouts and alerts: `> [!note] Title` (Obsidian, any type, optional
//!   `+`/`-` fold marker) and `> [!NOTE]` (GitHub's five alert types) become
//!   `<div class="callout callout-note" role="note">` with a title paragraph,
//!   or a `<details>` element when foldable;
//! - Pandoc fenced divs: `::: warning` or `::: {#id .class key="value"}` …
//!   `:::` become `<div>`s with those attributes; they nest.
//!
//! Lines inside fenced code blocks are never touched.

use textweaver_formats::callout::{self, CalloutHead, Fold};

use crate::escape_html;

/// Which rewrites to apply.
#[derive(Clone, Copy, Debug, Default)]
pub struct Blocks {
    /// Obsidian callouts: any type, custom titles, folding.
    pub callouts: bool,
    /// GitHub alerts only: the five fixed types, no custom title.
    pub alerts_only: bool,
    /// Pandoc fenced divs.
    pub fenced_divs: bool,
}

/// Applies the enabled rewrites. Returns `None` when nothing changed, so
/// the caller keeps borrowing the original text.
pub fn rewrite(src: &str, blocks: Blocks) -> Option<String> {
    let want_callouts = (blocks.callouts || blocks.alerts_only) && src.contains("[!");
    let want_divs = blocks.fenced_divs && src.contains(":::");
    if !want_callouts && !want_divs {
        return None;
    }
    let lines: Vec<&str> = src.split_inclusive('\n').collect();
    let mut out = String::with_capacity(src.len() + 256);
    let changed = process(&lines, blocks, &mut out, 0);
    changed.then_some(out)
}

/// An open code fence: its character and length.
#[derive(Clone, Copy)]
struct Fence {
    ch: u8,
    len: usize,
}

fn fence_of(line: &str) -> Option<Fence> {
    let t = line.trim_start();
    if line.len() - t.len() > 3 {
        return None;
    }
    let ch = *t.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = t.bytes().take_while(|&b| b == ch).count();
    (len >= 3).then_some(Fence { ch, len })
}

/// Processes `lines` into `out`; true when any rewrite happened.
fn process(lines: &[&str], blocks: Blocks, out: &mut String, depth: usize) -> bool {
    let mut changed = false;
    let mut fence: Option<Fence> = None;
    // Open fenced divs: the colon count of each.
    let mut divs: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(f) = fence {
            out.push_str(line);
            if let Some(close) = fence_of(line)
                && close.ch == f.ch
                && close.len >= f.len
                && line.trim_start()[close.len..].trim().is_empty()
            {
                fence = None;
            }
            i += 1;
            continue;
        }
        if let Some(f) = fence_of(line) {
            fence = Some(f);
            out.push_str(line);
            i += 1;
            continue;
        }
        if blocks.fenced_divs
            && let Some(div) = fenced_div(line)
        {
            match div {
                Div::Open { colons, attrs } => {
                    out.push_str("\n<div");
                    out.push_str(&attrs);
                    out.push_str(">\n\n");
                    divs.push(colons);
                    changed = true;
                    i += 1;
                    continue;
                }
                Div::Close { colons } if divs.last().is_some_and(|&c| colons >= c) => {
                    divs.pop();
                    out.push_str("\n</div>\n\n");
                    changed = true;
                    i += 1;
                    continue;
                }
                Div::Close { .. } => {}
            }
        }
        if (blocks.callouts || blocks.alerts_only)
            && depth < 8
            && let Some(head) = callout_head(line, blocks)
            && (i == 0 || !is_quote_line(lines[i - 1], head.indent))
        {
            // The callout body: following quote lines at the same indent,
            // with one level of `>` removed.
            let mut body: Vec<String> = Vec::new();
            let mut j = i + 1;
            while j < lines.len() && is_quote_line(lines[j], head.indent) {
                body.push(strip_quote(lines[j], head.indent));
                j += 1;
            }
            let pad = " ".repeat(head.indent);
            // The type word comes first ("Tip: Remember"), so a reader
            // hears what kind of note it is before its title; a title that
            // already starts with the word is kept as written.
            let word = head.type_word();
            let custom = head.custom_title.trim();
            let title =
                if custom.is_empty() || custom.to_lowercase().starts_with(&word.to_lowercase()) {
                    escape_html(&head.title())
                } else {
                    escape_html(&format!("{word}: {custom}"))
                };
            let class = escape_html(&head.kind);
            if head.fold.is_some() {
                let open = if head.fold == Some(Fold::Expanded) {
                    " open"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "\n{pad}<details class=\"callout callout-{class}\"{open}>\n{pad}<summary class=\"callout-title\">{title}</summary>\n\n"
                ));
            } else {
                out.push_str(&format!(
                    "\n{pad}<div class=\"callout callout-{class}\" role=\"note\">\n{pad}<p class=\"callout-title\"><strong>{title}</strong></p>\n\n"
                ));
            }
            let body_refs: Vec<&str> = body.iter().map(String::as_str).collect();
            let mut inner = String::new();
            process(&body_refs, blocks, &mut inner, depth + 1);
            for l in inner.split_inclusive('\n') {
                if l.trim().is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(&pad);
                    out.push_str(l);
                }
            }
            if !out.ends_with('\n') {
                out.push('\n');
            }
            let close = if head.fold.is_some() {
                "</details>"
            } else {
                "</div>"
            };
            out.push_str(&format!("\n{pad}{close}\n\n"));
            changed = true;
            i = j;
            continue;
        }
        out.push_str(line);
        i += 1;
    }
    changed
}

enum Div {
    Open { colons: usize, attrs: String },
    Close { colons: usize },
}

/// A fenced div line: `:::` (three or more colons) alone closes; followed
/// by a class word or an attribute block it opens.
fn fenced_div(line: &str) -> Option<Div> {
    let t = line.trim_start();
    if line.len() - t.len() > 3 || !t.starts_with(":::") {
        return None;
    }
    let colons = t.bytes().take_while(|&b| b == b':').count();
    let rest = t[colons..].trim();
    let rest = rest.trim_end_matches(':').trim();
    if rest.is_empty() {
        return Some(Div::Close { colons });
    }
    let attrs = if let Some(inner) = rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        crate::inline::attributes_html(inner)
    } else if rest.split_whitespace().count() == 1 {
        format!(" class=\"{}\"", escape_html(rest))
    } else {
        return None;
    };
    Some(Div::Open { colons, attrs })
}

/// `> [!type]± Title`, by the rules the reader shares ([`callout::head`]):
/// any type with a title and folding for Obsidian, GitHub's five alerts
/// otherwise.
fn callout_head(line: &str, blocks: Blocks) -> Option<CalloutHead> {
    callout::head(line, blocks.callouts)
}

fn is_quote_line(line: &str, indent: usize) -> bool {
    let t = line.trim_start();
    line.len() - t.len() == indent && t.starts_with('>')
}

fn strip_quote(line: &str, indent: usize) -> String {
    let t = &line[indent..];
    let t = t.strip_prefix('>').unwrap_or(t);
    let t = t.strip_prefix(' ').unwrap_or(t);
    if t.is_empty() || t == "\n" || t == "\r\n" {
        "\n".to_owned()
    } else {
        t.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBSIDIAN: Blocks = Blocks {
        callouts: true,
        alerts_only: false,
        fenced_divs: false,
    };

    #[test]
    fn callout_becomes_div_with_title() {
        let out = rewrite("> [!tip] Remember\n> Body *text*.\n\nAfter\n", OBSIDIAN).unwrap();
        assert!(out.contains("<div class=\"callout callout-tip\" role=\"note\">"));
        assert!(out.contains("<strong>Tip: Remember</strong>"));
        let plain = rewrite("> [!tip]\n> x\n", OBSIDIAN).unwrap();
        assert!(plain.contains("<strong>Tip</strong>"), "{plain}");
        let kept = rewrite("> [!tip] Tip of the day\n> x\n", OBSIDIAN).unwrap();
        assert!(kept.contains("<strong>Tip of the day</strong>"), "{kept}");
        assert!(out.contains("\nBody *text*.\n"));
        assert!(out.contains("</div>"));
        assert!(out.ends_with("After\n"));
    }

    #[test]
    fn foldable_callout_is_details() {
        let out = rewrite("> [!faq]- Why?\n> Because.\n", OBSIDIAN).unwrap();
        assert!(out.contains("<details class=\"callout callout-faq\">"));
        assert!(out.contains("<summary class=\"callout-title\">FAQ: Why?</summary>"));
    }

    #[test]
    fn gfm_alerts_only_fixed_types() {
        let gfm = Blocks {
            callouts: false,
            alerts_only: true,
            fenced_divs: false,
        };
        assert!(rewrite("> [!NOTE]\n> x\n", gfm).is_some());
        assert!(rewrite("> [!custom]\n> x\n", gfm).is_none());
    }

    #[test]
    fn code_fences_are_untouched() {
        assert!(rewrite("```\n> [!note]\n```\n", OBSIDIAN).is_none());
    }

    #[test]
    fn fenced_divs_nest() {
        let pandoc = Blocks {
            callouts: false,
            alerts_only: false,
            fenced_divs: true,
        };
        let out = rewrite(
            ":::: {#outer .box}\n::: warning\nInner\n:::\n::::\n",
            pandoc,
        )
        .unwrap();
        assert!(out.contains("<div id=\"outer\" class=\"box\">"));
        assert!(out.contains("<div class=\"warning\">"));
        assert_eq!(out.matches("</div>").count(), 2);
    }
}
