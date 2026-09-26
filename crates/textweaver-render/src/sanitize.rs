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
}
