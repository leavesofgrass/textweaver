//! Optional HTML sanitization with `ammonia`, for Markdown from untrusted
//! sources. Scripts, event handlers, and `javascript:` URLs are removed;
//! everything textweaver itself produces survives: MathML, callouts
//! (`details`, `summary`, `role="note"`), footnote roles, heading ids,
//! classes, `lang`, and `aria-*` attributes.

use std::sync::OnceLock;

use ammonia::Builder;

/// MathML Core elements (a superset of what `textweaver-math` writes).
const MATHML: &[&str] = &[
    "math",
    "semantics",
    "annotation",
    "mrow",
    "mi",
    "mn",
    "mo",
    "ms",
    "mtext",
    "mspace",
    "msup",
    "msub",
    "msubsup",
    "mfrac",
    "msqrt",
    "mroot",
    "mstyle",
    "mpadded",
    "mphantom",
    "menclose",
    "mover",
    "munder",
    "munderover",
    "mtable",
    "mtr",
    "mtd",
    "mmultiscripts",
    "mprescripts",
    "none",
    "merror",
];

const EXTRA_TAGS: &[&str] = &[
    "details",
    "summary",
    "section",
    "nav",
    "aside",
    "figure",
    "figcaption",
    "mark",
    "input",
    "span",
    "div",
    "dl",
    "dt",
    "dd",
    "sup",
    "sub",
    "del",
    "ins",
    "u",
];

const MATH_ATTRS: &[&str] = &[
    "alttext",
    "display",
    "form",
    "xmlns",
    "encoding",
    "mathvariant",
    "stretchy",
    "fence",
    "separator",
    "lspace",
    "rspace",
    "movablelimits",
    "largeop",
    "symmetric",
    "minsize",
    "maxsize",
    "accent",
    "accentunder",
    "linethickness",
    "scriptlevel",
    "displaystyle",
    "columnalign",
    "rowalign",
    "columnspacing",
    "rowspacing",
    "width",
    "height",
    "depth",
    "voffset",
    "notation",
    "style",
];

fn builder() -> &'static Builder<'static> {
    static BUILDER: OnceLock<Builder<'static>> = OnceLock::new();
    BUILDER.get_or_init(|| {
        let mut b = Builder::default();
        b.add_tags(MATHML.iter().copied())
            .add_tags(EXTRA_TAGS.iter().copied())
            .add_generic_attributes(["id", "class", "role", "lang", "dir", "title", "open"])
            .add_generic_attribute_prefixes(["aria-", "data-"])
            .add_tag_attributes("input", ["type", "checked", "disabled"])
            .add_tag_attributes("ol", ["start", "reversed", "type"])
            .add_tag_attributes("th", ["scope", "style"])
            .add_tag_attributes("td", ["style"])
            .add_tag_attributes("img", ["src", "alt", "width", "height", "loading"]);
        for tag in MATHML {
            b.add_tag_attributes(tag, MATH_ATTRS.iter().copied());
        }
        b
    })
}

/// Cleans `html`, keeping the markup textweaver renders.
pub fn sanitize(html: &str) -> String {
    builder().clean(html).to_string()
}

/// True when cleaning would leave out some of the raw HTML written in the
/// Markdown `src`: a tag (a script, an iframe) or an attribute (an event
/// handler, a `javascript:` address) that [`sanitize`] removes. Raw HTML
/// it keeps whole (`<b>`, `<br>`, a `<details>` block) and HTML comments
/// do not count, so a reader is told only when something is missing.
///
/// The raw HTML is found with pulldown-cmark under `opts.flavor`, whatever
/// the engine, so markup textweaver writes itself never counts.
pub fn raw_html_removed(src: &str, opts: &crate::RenderOptions) -> bool {
    use pulldown_cmark::{Event, Parser};
    let mut raw = String::new();
    for event in Parser::new_ext(src, crate::engine::pulldown_options(opts.flavor, false)) {
        if let Event::Html(h) | Event::InlineHtml(h) = event {
            raw.push_str(&h);
        }
    }
    if raw.trim().is_empty() {
        return false;
    }
    let before = tag_counts(&raw);
    let after = tag_counts(&sanitize(&raw));
    before
        .iter()
        .any(|(k, n)| after.get(k).copied().unwrap_or(0) < *n)
}

/// How many times each opening tag (`b`) and each attribute on a tag
/// (`a href`) appears in `html`. Closing tags, comments, and
/// declarations are skipped.
fn tag_counts(html: &str) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    let bytes = html.as_bytes();
    let mut i = 0;
    while let Some(lt) = html[i..].find('<') {
        let start = i + lt + 1;
        i = start;
        let name_len = bytes[start..]
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric() || **b == b'-')
            .count();
        if name_len == 0 || !bytes[start].is_ascii_alphabetic() {
            continue;
        }
        let tag = html[start..start + name_len].to_ascii_lowercase();
        *counts.entry(tag.clone()).or_insert(0) += 1;
        let mut j = start + name_len;
        // Attributes, up to the end of the tag.
        while j < bytes.len() && bytes[j] != b'>' {
            let b = bytes[j];
            if b.is_ascii_whitespace() || b == b'/' {
                j += 1;
            } else if b == b'=' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                match bytes.get(j) {
                    Some(&q @ (b'"' | b'\'')) => {
                        j += 1;
                        while j < bytes.len() && bytes[j] != q {
                            j += 1;
                        }
                        j += 1;
                    }
                    _ => {
                        while j < bytes.len() && !bytes[j].is_ascii_whitespace() && bytes[j] != b'>'
                        {
                            j += 1;
                        }
                    }
                }
            } else {
                let a = j;
                while j < bytes.len()
                    && !bytes[j].is_ascii_whitespace()
                    && !matches!(bytes[j], b'=' | b'>' | b'/')
                {
                    j += 1;
                }
                let attr = html[a..j].to_ascii_lowercase();
                *counts.entry(format!("{tag} {attr}")).or_insert(0) += 1;
            }
        }
        i = j.min(bytes.len());
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_scripts_keeps_structure() {
        let out = sanitize(
            "<h2 id=\"a\">T</h2><script>alert(1)</script><p onclick=\"x()\">p</p><a href=\"javascript:x()\">l</a><div class=\"callout\" role=\"note\">c</div><math display=\"block\"><mi>x</mi></math>",
        );
        assert!(out.contains("<h2 id=\"a\">T</h2>"), "{out}");
        assert!(!out.contains("script"), "{out}");
        assert!(!out.contains("onclick"), "{out}");
        assert!(!out.contains("javascript"), "{out}");
        assert!(out.contains("role=\"note\""), "{out}");
        assert!(
            out.contains("<math display=\"block\"><mi>x</mi></math>"),
            "{out}"
        );
    }

    #[test]
    fn says_when_raw_html_loses_something() {
        let opts = crate::RenderOptions::default();
        // Nothing raw, raw HTML kept whole, a comment, or code: nothing to
        // say. A script, an event handler, a javascript: address, or an
        // iframe is left out.
        let cases = [
            (false, "# Title\n\nText with *emphasis*.\n"),
            (false, "A <b>bold</b> word<br>\n\n<!-- a comment -->\n"),
            (
                false,
                "<details>\n<summary>More</summary>\n\nHidden.\n\n</details>\n",
            ),
            (true, "Text\n\n<script>fetch('x')</script>\n"),
            (true, "A <span onclick=\"x()\">word</span>.\n"),
            (true, "<a href='javascript:x()'>link</a>\n"),
            (true, "<iframe src=\"https://example.com\"></iframe>\n"),
            (false, "```\n<script>x</script>\n```\n"),
        ];
        for (want, src) in cases {
            assert_eq!(raw_html_removed(src, &opts), want, "{src:?}");
        }
    }
}
