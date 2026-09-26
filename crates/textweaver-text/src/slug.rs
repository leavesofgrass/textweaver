//! Heading anchors: GitHub-style slugs.
//!
//! One rule for every place that names a heading by its anchor: the ids
//! `textweaver-render` gives headings in HTML, and following a link such
//! as `[see](#data-and-methods)` in the reader.

/// A GitHub-style slug: lowercase letters and digits kept (any script),
/// spaces turned into `-`, `-` and `_` kept, everything else dropped.
/// An empty result becomes `"section"`.
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if c == ' ' || c == '-' {
            out.push('-');
        } else if c == '_' {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("section");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_like_github() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("  Ünïcode — ok_1 "), "ünïcode--ok_1");
        assert_eq!(slugify("?!"), "section");
    }
}
