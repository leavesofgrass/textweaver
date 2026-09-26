//! Bounding XML nesting before `roxmltree` sees it.
//!
//! `roxmltree`'s tokenizer recurses once per nested element, so a package
//! part with tens of thousands of nested elements overflows the stack
//! before any loader code runs. [`limit_depth`] rewrites such a part so
//! nothing is nested deeper than a limit: below the element at the limit,
//! the elements that contain other elements are dropped (their text is
//! kept), and the leaf elements (a DOCX `w:t` run text, `w:tab`, `w:br`)
//! are kept in document order inside one [`FLAT_ELEMENT`], which the
//! loaders read as plain text. Well-nested parts come back unchanged
//! without being copied.

/// The element wrapping flattened content.
pub(crate) const FLAT_ELEMENT: &str = "textweaver-flat";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Text,
    Start { empty: bool },
    End,
    Other,
}

/// Splits XML into tokens (byte ranges). Unterminated markup ends the
/// scan with the rest as text, which the parser then rejects as usual.
fn tokens(text: &str) -> Vec<(Kind, usize, usize)> {
    let b = text.as_bytes();
    let find = |from: usize, pat: &[u8]| -> Option<usize> {
        b.get(from..)?
            .windows(pat.len())
            .position(|w| w == pat)
            .map(|i| from + i + pat.len())
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'<' {
            let end = b[i..]
                .iter()
                .position(|&c| c == b'<')
                .map_or(b.len(), |p| i + p);
            out.push((Kind::Text, i, end));
            i = end;
            continue;
        }
        let rest = &b[i..];
        let end = if rest.starts_with(b"<!--") {
            find(i + 4, b"-->").map(|e| (Kind::Other, e))
        } else if rest.starts_with(b"<![CDATA[") {
            find(i + 9, b"]]>").map(|e| (Kind::Text, e))
        } else if rest.starts_with(b"<?") {
            find(i + 2, b"?>").map(|e| (Kind::Other, e))
        } else if rest.starts_with(b"<!") {
            // A DOCTYPE, maybe with an internal subset in brackets.
            let mut depth = 0usize;
            rest.iter()
                .enumerate()
                .find(|&(_, &c)| {
                    match c {
                        b'[' => depth += 1,
                        b']' => depth = depth.saturating_sub(1),
                        b'>' if depth == 0 => return true,
                        _ => {}
                    }
                    false
                })
                .map(|(p, _)| (Kind::Other, i + p + 1))
        } else if rest.starts_with(b"</") {
            find(i + 2, b">").map(|e| (Kind::End, e))
        } else {
            let mut quote = None;
            rest.iter()
                .enumerate()
                .skip(1)
                .find(|&(_, &c)| {
                    match (quote, c) {
                        (None, b'"' | b'\'') => quote = Some(c),
                        (Some(q), c) if c == q => quote = None,
                        (None, b'>') => return true,
                        _ => {}
                    }
                    false
                })
                .map(|(p, _)| {
                    let empty = p > 0 && rest[p - 1] == b'/';
                    (Kind::Start { empty }, i + p + 1)
                })
        };
        match end {
            Some((kind, e)) => {
                out.push((kind, i, e));
                i = e;
            }
            None => {
                out.push((Kind::Text, i, b.len()));
                break;
            }
        }
    }
    out
}

/// `text` rewritten so no element is nested more than `limit` deep (plus
/// the flat wrapper and its leaves), or `None` when it already is.
pub(crate) fn limit_depth(text: &str, limit: usize) -> Option<String> {
    let toks = tokens(text);
    // Pass 1: the deepest nesting, and which elements contain elements.
    let mut has_child = vec![false; toks.len()];
    let mut stack: Vec<usize> = Vec::new();
    let mut max = 0usize;
    for (i, &(kind, ..)) in toks.iter().enumerate() {
        match kind {
            Kind::Start { empty } => {
                if let Some(&parent) = stack.last() {
                    has_child[parent] = true;
                }
                if !empty {
                    stack.push(i);
                    max = max.max(stack.len());
                }
            }
            Kind::End => {
                stack.pop();
            }
            Kind::Text | Kind::Other => {}
        }
    }
    if max <= limit {
        return None;
    }
    // Pass 2: copy, dropping the tags of containers below the limit.
    let mut out = String::with_capacity(text.len());
    // For each open element: whether its tags were copied.
    let mut open: Vec<bool> = Vec::new();
    let mut flat_open = false;
    for (i, &(kind, a, e)) in toks.iter().enumerate() {
        let tok = &text[a..e];
        match kind {
            Kind::Start { empty } => {
                let depth = open.len();
                let keep = if depth < limit {
                    true
                } else {
                    if !flat_open {
                        out.push('<');
                        out.push_str(FLAT_ELEMENT);
                        out.push('>');
                        flat_open = true;
                    }
                    // Only leaves: they contain no elements, so kept ones
                    // never nest.
                    !has_child[i]
                };
                if keep {
                    out.push_str(tok);
                } else {
                    // Words in different dropped elements stay apart.
                    out.push(' ');
                }
                if !empty {
                    open.push(keep);
                }
            }
            Kind::End => {
                // The level of the element this tag closes.
                let level = open.len();
                let kept = open.pop().unwrap_or(true);
                if level == limit && flat_open {
                    out.push_str("</");
                    out.push_str(FLAT_ELEMENT);
                    out.push('>');
                    flat_open = false;
                }
                if kept {
                    out.push_str(tok);
                } else {
                    out.push(' ');
                }
            }
            Kind::Text | Kind::Other => out.push_str(tok),
        }
    }
    if flat_open {
        out.push_str("</");
        out.push_str(FLAT_ELEMENT);
        out.push('>');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nested(depth: usize, inner: &str) -> String {
        let mut s = String::from("<?xml version=\"1.0\"?><!-- c --><root>");
        for _ in 0..depth {
            s.push_str("<b x=\"a>b\">");
        }
        s.push_str(inner);
        for _ in 0..depth {
            s.push_str("</b>");
        }
        s.push_str("</root>");
        s
    }

    #[test]
    fn shallow_xml_is_untouched() {
        assert_eq!(limit_depth(&nested(5, "<t>x</t>"), 8), None);
    }

    #[test]
    fn deep_xml_is_flattened_keeping_leaves() {
        let src = nested(50_000, "<t>deep</t><tab/><t><![CDATA[a<b]]></t>");
        let out = limit_depth(&src, 10).unwrap();
        let doc = roxmltree::Document::parse(&out).unwrap();
        let flat: Vec<_> = doc
            .descendants()
            .filter(|n| n.tag_name().name() == FLAT_ELEMENT)
            .collect();
        assert_eq!(flat.len(), 1);
        let depth = flat[0].ancestors().count();
        assert!(depth <= 12, "{depth}");
        let leaves: Vec<_> = flat[0]
            .children()
            .filter(|n| n.is_element())
            .map(|n| {
                (
                    n.tag_name().name().to_owned(),
                    n.text().unwrap_or("").to_owned(),
                )
            })
            .collect();
        assert_eq!(
            leaves,
            vec![
                ("t".to_owned(), "deep".to_owned()),
                ("tab".to_owned(), String::new()),
                ("t".to_owned(), "a<b".to_owned())
            ]
        );
    }

    #[test]
    fn unterminated_markup_does_not_panic() {
        for s in [
            "<a",
            "<a b='>",
            "<!-- x",
            "<![CDATA[",
            "<!DOCTYPE [",
            "</",
            "<?x",
        ] {
            let _ = limit_depth(s, 0);
            let _ = limit_depth(s, 3);
        }
    }
}
