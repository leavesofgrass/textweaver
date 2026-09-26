//! Wikilinks, Dataview relation fields, and inline tags in a note body,
//! ported from `star/obsidian.py` (`_WIKILINK`, `_INLINE_REL`,
//! `_INLINE_TAG`, `_extract_links`, `_first_line`).
//!
//! Differences from Star, all deliberate:
//! - Links and tags inside code (fenced blocks and inline code spans) and
//!   inside Obsidian comments (`%% ... %%`) are ignored, as Obsidian itself
//!   ignores them. Star counted them.
//! - A purely numeric `#123` is not a tag, as in Obsidian.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::RelationType;

/// `[[Target]]`, `[[Target#Heading]]`, `[[Target|Alias]]`. Group 1 is the
/// target, group 2 the alias (when there is one).
static WIKILINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[([^\]\|#]+)(?:#[^\]\|]*)?(?:\|([^\]]*))?\]\]").expect("valid regex")
});

/// A wikilink or an embed, for [`strip_link_syntax`].
static ANY_WIKILINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"!?\[\[([^\]\|#]+)(?:#[^\]\|]*)?(?:\|([^\]]*))?\]\]").expect("valid regex")
});

/// A Dataview inline field whose value is a wikilink: `rel:: [[Target]]`.
static INLINE_REL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([A-Za-z][\w -]*?)\s*::\s*\[\[([^\]\|#]+)(?:[#\|][^\]]*)?\]\]")
        .expect("valid regex")
});

/// `#tag` at the start or after whitespace.
static INLINE_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|\s)#([A-Za-z0-9_][A-Za-z0-9_/-]*)").expect("valid regex"));

/// One link found in a note body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// The relation named by a Dataview field (`supports:: [[X]]`); `None`
    /// for a plain wikilink.
    pub rel_type: Option<RelationType>,
    /// The linked note's name, as written (heading and alias removed).
    pub target: String,
    /// For a typed field, the rest of its line after the link (a comment
    /// on the relation, `- supports:: [[X]] - because ...`); else empty.
    pub note: String,
}

/// Blanks out code and comments, keeping byte offsets, so the regexes see
/// only prose. Fenced blocks (```` ``` ```` or `~~~`), inline code spans,
/// and `%% ... %%` comments become spaces.
pub fn mask_code(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut fence: Option<(char, usize)> = None;
    let mut in_comment = false;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let fence_run = |c: char| trimmed.chars().take_while(|&x| x == c).count();
        if let Some((c, n)) = fence {
            out.push_str(&blank(line));
            if fence_run(c) >= n && trimmed[fence_run(c)..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if !in_comment {
            let (tick, tilde) = (fence_run('`'), fence_run('~'));
            if tick >= 3 || tilde >= 3 {
                fence = Some(if tick >= 3 { ('`', tick) } else { ('~', tilde) });
                out.push_str(&blank(line));
                continue;
            }
        }
        out.push_str(&mask_line(line, &mut in_comment));
    }
    out
}

/// Masks inline code spans and comments within one line.
fn mask_line(line: &str, in_comment: &mut bool) -> String {
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    let bytes = line.as_bytes();
    while i < line.len() {
        if *in_comment {
            if line[i..].starts_with("%%") {
                out.push_str("  ");
                i += 2;
                *in_comment = false;
            } else {
                let c = next_char(line, i);
                out.push_str(&blank(&line[i..i + c]));
                i += c;
            }
            continue;
        }
        if line[i..].starts_with("%%") {
            out.push_str("  ");
            i += 2;
            *in_comment = true;
            continue;
        }
        if bytes[i] == b'`' {
            let run = line[i..].bytes().take_while(|&b| b == b'`').count();
            let ticks = &line[i..i + run];
            if let Some(close) = find_closing_ticks(&line[i + run..], run) {
                let end = i + run + close + run;
                out.push_str(&blank(&line[i..end]));
                i = end;
            } else {
                out.push_str(ticks);
                i += run;
            }
            continue;
        }
        let c = next_char(line, i);
        out.push_str(&line[i..i + c]);
        i += c;
    }
    out
}

/// The offset of a backtick run of exactly `run` ticks in `s`.
fn find_closing_ticks(s: &str, run: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let n = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            if n == run {
                return Some(i);
            }
            i += n;
        } else {
            i += 1;
        }
    }
    None
}

fn next_char(s: &str, i: usize) -> usize {
    s[i..].chars().next().map_or(1, char::len_utf8)
}

/// Spaces for every char of `s`, keeping newlines and byte length.
fn blank(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\n' || c == '\r' {
                c.to_string()
            } else {
                " ".repeat(c.len_utf8())
            }
        })
        .collect()
}

/// The start of the byte before `at` is `!` (an embed, `![[...]]`).
fn is_embed(text: &str, at: usize) -> bool {
    at > 0 && text.as_bytes()[at - 1] == b'!'
}

/// The relation a Dataview field name names. The regex's field name may
/// begin earlier in the sentence (`Some text supports:: [[X]]` captures
/// `Some text supports`), so the name's word suffixes are tried longest
/// first. Star tried only the whole capture and lost such relations.
fn field_relation(key: &str) -> Option<RelationType> {
    let words: Vec<&str> = key.split_whitespace().collect();
    (0..words.len()).find_map(|i| RelationType::parse(&words[i..].join(" ")))
}

/// Every link in `body`, in order: typed Dataview fields first as they
/// appear, then plain wikilinks not already consumed by a field (Star's
/// `_extract_links`). Embeds (`![[...]]`) are skipped.
pub fn extract_links(body: &str) -> Vec<Link> {
    let text = mask_code(body);
    let mut links = Vec::new();
    let mut consumed: Vec<(usize, usize)> = Vec::new();
    for m in INLINE_REL.captures_iter(&text) {
        let (Some(whole), Some(key), Some(target)) = (m.get(0), m.get(1), m.get(2)) else {
            continue;
        };
        if let Some(rt) = field_relation(key.as_str()) {
            links.push(Link {
                rel_type: Some(rt),
                target: target.as_str().trim().to_owned(),
                note: if starts_line(&text, whole.start()) {
                    rest_of_line(&text, whole.end())
                } else {
                    String::new()
                },
            });
            consumed.push((whole.start(), whole.end()));
        }
    }
    for m in WIKILINK.captures_iter(&text) {
        let (Some(whole), Some(target)) = (m.get(0), m.get(1)) else {
            continue;
        };
        if is_embed(&text, whole.start()) {
            continue;
        }
        if consumed
            .iter()
            .any(|&(s, e)| s <= whole.start() && whole.start() < e)
        {
            continue;
        }
        links.push(Link {
            rel_type: None,
            target: target.as_str().trim().to_owned(),
            note: String::new(),
        });
    }
    links
}

/// True when only a list marker and spaces precede byte `at` on its line.
fn starts_line(text: &str, at: usize) -> bool {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    text[line_start..at]
        .trim_start()
        .trim_start_matches(['-', '*', '+'])
        .trim()
        .is_empty()
}

/// The rest of the line after byte `at`, without leading separators.
fn rest_of_line(text: &str, at: usize) -> String {
    let rest = &text[at..];
    let line = rest.split(['\n', '\r']).next().unwrap_or("");
    line.trim()
        .trim_start_matches(['-', '\u{2013}', '\u{2014}', ':', ' '])
        .trim()
        .to_owned()
}

/// Inline `#tags` in `body`, in order, without duplicates.
pub fn inline_tags(body: &str) -> Vec<String> {
    let text = mask_code(body);
    let mut tags: Vec<String> = Vec::new();
    for m in INLINE_TAG.captures_iter(&text) {
        let Some(t) = m.get(1) else { continue };
        let t = t.as_str();
        if t.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if !tags.iter().any(|x| x == t) {
            tags.push(t.to_owned());
        }
    }
    tags
}

/// `text` with link syntax reduced to what it reads as: `[[T|Alias]]`
/// becomes `Alias`, `[[T]]` and `![[T]]` become `T`, and a Dataview field
/// `rel:: [[T]]` becomes `T`. The result reads well aloud and never
/// re-introduces an edge when exported again.
pub fn strip_link_syntax(text: &str) -> String {
    let no_fields = INLINE_REL.replace_all(text, |c: &regex::Captures<'_>| {
        match c.get(1).and_then(|k| field_relation(k.as_str())) {
            Some(_) => c
                .get(2)
                .map_or(String::new(), |t| t.as_str().trim().to_owned()),
            None => c.get(0).map_or(String::new(), |m| m.as_str().to_owned()),
        }
    });
    ANY_WIKILINK
        .replace_all(&no_fields, |c: &regex::Captures<'_>| {
            match c
                .get(2)
                .map(|a| a.as_str().trim())
                .filter(|a| !a.is_empty())
            {
                Some(alias) => alias.to_owned(),
                None => c
                    .get(1)
                    .map_or(String::new(), |t| t.as_str().trim().to_owned()),
            }
        })
        .into_owned()
}

/// The note's first line of content as a one-line summary: heading marks
/// and link syntax removed, at most 200 chars (Star's `_first_line`).
pub fn first_line(body: &str) -> String {
    for line in body.lines() {
        let s = line.trim().trim_start_matches('#').trim();
        if s.is_empty() || s.starts_with("%%") {
            continue;
        }
        let s = strip_link_syntax(s);
        let s = s.trim_matches([' ', '-']);
        if !s.is_empty() {
            return s.chars().take(200).collect();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(rel: Option<RelationType>, t: &str) -> Link {
        Link {
            rel_type: rel,
            target: t.to_owned(),
            note: String::new(),
        }
    }

    #[test]
    fn typed_fields_keep_the_rest_of_the_line_as_a_comment() {
        let links = extract_links("- cites:: [[Source]] - page 12\n- supports:: [[B]]\n");
        assert_eq!(links[0].note, "page 12");
        assert_eq!(links[1].note, "");
    }

    #[test]
    fn plain_typed_and_embedded_links() {
        let body = "See [[Alpha]] and [[Beta#Part|the beta]].\n![[picture.png]]\n- supports:: [[Gamma]]\n- related:: [[Delta]]\n";
        assert_eq!(
            extract_links(body),
            vec![
                link(Some(RelationType::Supports), "Gamma"),
                link(None, "Alpha"),
                link(None, "Beta"),
                // An unknown field name leaves the link untyped (Star).
                link(None, "Delta"),
            ]
        );
    }

    #[test]
    fn field_names_after_prose_still_type_the_link() {
        assert_eq!(
            extract_links("This note clearly see also:: [[X]]"),
            vec![link(Some(RelationType::SeeAlso), "X")]
        );
    }

    #[test]
    fn links_in_code_and_comments_are_ignored() {
        let body = "Real [[One]].\n```\n[[NotALink]]\n```\nInline `[[Nope]]` here.\n%% hidden [[Hidden]] %%\n~~~md\n[[Also]]\n~~~\nEnd [[Two]]";
        assert_eq!(
            extract_links(body),
            vec![link(None, "One"), link(None, "Two")]
        );
    }

    #[test]
    fn multi_line_comments_are_masked() {
        let body = "a %% start\n[[X]]\nend %% [[Y]]";
        assert_eq!(extract_links(body), vec![link(None, "Y")]);
    }

    #[test]
    fn inline_tags_skip_headings_numbers_and_code() {
        let body = "# Heading\nText #exam and #reading/ch-1, #123 `#code` #exam";
        assert_eq!(inline_tags(body), vec!["exam", "reading/ch-1"]);
    }

    #[test]
    fn first_line_strips_link_syntax() {
        let body = "\n## [[Alpha]] supports:: [[Beta]] and [[Gamma|G]]\nmore";
        assert_eq!(first_line(body), "Alpha Beta and G");
        assert_eq!(first_line("\n\n"), "");
        let long = "x".repeat(300);
        assert_eq!(first_line(&long).chars().count(), 200);
    }

    #[test]
    fn strip_keeps_unknown_fields() {
        assert_eq!(strip_link_syntax("related:: [[X]]"), "related:: X");
        assert_eq!(strip_link_syntax("see ![[img.png]]"), "see img.png");
    }

    #[test]
    fn mask_keeps_multibyte_offsets() {
        let s = "é `ü` [[Ω]]";
        assert_eq!(mask_code(s).len(), s.len());
        assert_eq!(extract_links(s), vec![link(None, "Ω")]);
    }
}
