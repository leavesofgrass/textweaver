//! Inline extensions applied to runs of plain text (never to code, raw HTML,
//! or text inside links), the same way for both engines:
//!
//! - GFM autolink literals (`https://…`, `www.…`) for the pulldown engine,
//!   which lacks them (comrak parses them itself);
//! - Obsidian `[[wikilinks]]` with aliases, headings (`#Heading`) and block
//!   references (`#^id`), `![[embeds]]`, `#tags`, and `==highlights==`;
//! - Pandoc citations (`[@key]`, `[see @a, p. 3; -@b]`, bare `@key`) rendered
//!   as links to `#ref-key`, and bracketed spans `[text]{.class #id k=v}`.

use crate::escape_html;
use crate::slug::slugify;

/// Which inline extensions are on.
#[derive(Clone, Copy, Debug, Default)]
pub struct Inline {
    /// GFM autolink literals.
    pub autolinks: bool,
    /// Obsidian wikilinks and embeds.
    pub wikilinks: bool,
    /// Obsidian `#tags`.
    pub tags: bool,
    /// Obsidian `==highlight==`.
    pub highlights: bool,
    /// Pandoc citations.
    pub citations: bool,
    /// Pandoc bracketed spans.
    pub spans: bool,
    /// Pandoc `H~2~O` and `2^10^` inside words (engines only handle them at
    /// word boundaries).
    pub subsup: bool,
}

impl Inline {
    fn any(self) -> bool {
        self.autolinks
            || self.wikilinks
            || self.tags
            || self.highlights
            || self.citations
            || self.spans
            || self.subsup
    }
}

/// A piece of transformed text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seg {
    /// Plain text, still to be escaped by the HTML writer.
    Text(String),
    /// Finished inline HTML.
    Html(String),
}

/// Settings the extensions need beyond on and off.
pub struct Ctx<'a> {
    /// Extension appended to wikilink targets without one (".html").
    pub link_ext: &'a str,
    /// Tags found, in order of first appearance.
    pub tags: &'a mut Vec<String>,
}

/// Image extensions that `![[embeds]]` show as images.
const IMAGE_EXTS: [&str; 9] = [
    "png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "avif", "tiff",
];

/// Transforms one text run. Returns `None` when nothing matched, so the
/// caller keeps the original event.
pub fn transform(text: &str, on: Inline, ctx: &mut Ctx<'_>) -> Option<Vec<Seg>> {
    if !on.any() || !has_trigger(text, on) {
        return None;
    }
    let bytes = text.as_bytes();
    let mut segs: Vec<Seg> = Vec::new();
    let mut plain_start = 0;
    let mut i = 0;
    while i < bytes.len() {
        let found: Option<(usize, String)> = match bytes[i] {
            b'!' if on.wikilinks && text[i..].starts_with("![[") => {
                wikilink(&text[i + 1..], ctx.link_ext, true).map(|(n, h)| (n + 1, h))
            }
            b'[' if on.wikilinks && text[i..].starts_with("[[") => {
                wikilink(&text[i..], ctx.link_ext, false)
            }
            b'[' if on.citations || on.spans => bracket(&text[i..], on),
            b'#' if on.tags && boundary_before(text, i) => tag(&text[i..], ctx),
            b'=' if on.highlights && text[i..].starts_with("==") => highlight(&text[i..]),
            b'@' if on.citations && boundary_before(text, i) => bare_citation(&text[i..]),
            b'~' if on.subsup && !text[i..].starts_with("~~") => sub_sup(&text[i..], '~'),
            b'^' if on.subsup => sub_sup(&text[i..], '^'),
            b'h' | b'w' | b'H' | b'W' if on.autolinks && autolink_boundary(text, i) => {
                autolink(&text[i..])
            }
            _ => None,
        };
        match found {
            Some((len, html)) => {
                if plain_start < i {
                    segs.push(Seg::Text(text[plain_start..i].to_owned()));
                }
                segs.push(Seg::Html(html));
                i += len;
                plain_start = i;
            }
            None => {
                i += text[i..].chars().next().map_or(1, char::len_utf8);
            }
        }
    }
    if segs.is_empty() {
        return None;
    }
    if plain_start < text.len() {
        segs.push(Seg::Text(text[plain_start..].to_owned()));
    }
    Some(segs)
}

fn has_trigger(text: &str, on: Inline) -> bool {
    text.bytes().any(|b| match b {
        b'[' => on.wikilinks || on.citations || on.spans,
        b'#' => on.tags,
        b'=' => on.highlights,
        b'@' => on.citations,
        b':' | b'w' | b'W' => on.autolinks,
        b'~' | b'^' => on.subsup,
        _ => false,
    })
}

/// True at the start of the text or after whitespace or an opening bracket.
fn boundary_before(text: &str, i: usize) -> bool {
    text[..i]
        .chars()
        .next_back()
        .is_none_or(|c| c.is_whitespace() || matches!(c, '(' | '[' | '{' | '"' | '\''))
}

fn autolink_boundary(text: &str, i: usize) -> bool {
    text[..i]
        .chars()
        .next_back()
        .is_none_or(|c| c.is_whitespace() || matches!(c, '*' | '_' | '~' | '('))
}

/// Parses `[[target|alias]]` (with `embed`, the text after `!`).
fn wikilink(s: &str, ext: &str, embed: bool) -> Option<(usize, String)> {
    let inner_end = s[2..].find("]]")? + 2;
    let inner = &s[2..inner_end];
    if inner.is_empty() || inner.contains('\n') || inner.contains('[') {
        return None;
    }
    let (target, alias) = match inner.split_once('|') {
        Some((t, a)) => (t.trim(), Some(a.trim())),
        None => (inner.trim(), None),
    };
    let (page, fragment) = match target.split_once('#') {
        Some((p, f)) => (p.trim(), Some(f.trim())),
        None => (target, None),
    };
    let href = wiki_href(page, fragment, ext);
    let len = inner_end + 2;
    if embed {
        let lower = page.to_lowercase();
        let is_image = lower
            .rsplit_once('.')
            .is_some_and(|(_, e)| IMAGE_EXTS.contains(&e));
        if is_image {
            // `|300` or `|300x200` is a size in Obsidian, not alt text.
            let alt = alias
                .filter(|a| !a.chars().all(|c| c.is_ascii_digit() || c == 'x'))
                .map(str::to_owned)
                .unwrap_or_else(|| file_stem(page));
            return Some((
                len,
                format!(
                    "<img src=\"{}\" alt=\"{}\">",
                    escape_html(&encode_path(page)),
                    escape_html(&alt)
                ),
            ));
        }
        let text = alias.map_or_else(|| display_text(page, fragment), str::to_owned);
        return Some((
            len,
            format!(
                "<a class=\"embed\" href=\"{}\">{}</a>",
                escape_html(&href),
                escape_html(&text)
            ),
        ));
    }
    let text = alias.map_or_else(|| display_text(page, fragment), str::to_owned);
    Some((
        len,
        format!(
            "<a class=\"wikilink\" href=\"{}\">{}</a>",
            escape_html(&href),
            escape_html(&text)
        ),
    ))
}

/// The visible text of a wikilink without an alias.
fn display_text(page: &str, fragment: Option<&str>) -> String {
    match fragment {
        Some(f) => {
            let f = f.trim_start_matches('^');
            if page.is_empty() {
                f.to_owned()
            } else {
                format!("{page}: {f}")
            }
        }
        None => page.to_owned(),
    }
}

/// The href of a wikilink target: the page with `ext` added when it has no
/// extension, and a heading slug or `block-<id>` fragment.
pub fn wiki_href(page: &str, fragment: Option<&str>, ext: &str) -> String {
    let mut href = String::new();
    if !page.is_empty() {
        href.push_str(&encode_path(page));
        let has_ext = page
            .rsplit_once('.')
            .is_some_and(|(_, e)| !e.is_empty() && e.len() <= 5 && !e.contains(['/', ' ']));
        if !has_ext {
            href.push_str(ext);
        }
    }
    if let Some(f) = fragment.filter(|f| !f.is_empty()) {
        href.push('#');
        match f.strip_prefix('^') {
            Some(block) => {
                href.push_str("block-");
                href.push_str(&slugify(block));
            }
            None => href.push_str(&slugify(f)),
        }
    }
    href
}

fn file_stem(page: &str) -> String {
    let name = page.rsplit(['/', '\\']).next().unwrap_or(page);
    name.rsplit_once('.').map_or(name, |(s, _)| s).to_owned()
}

/// Percent-encodes the characters a path in an href cannot carry.
pub fn encode_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    for c in p.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '"' => out.push_str("%22"),
            '#' => out.push_str("%23"),
            '%' => out.push_str("%25"),
            '?' => out.push_str("%3F"),
            '\\' => out.push('/'),
            c => out.push(c),
        }
    }
    out
}

/// `[...]` as a citation group or a bracketed span.
fn bracket(s: &str, on: Inline) -> Option<(usize, String)> {
    let close = matching_bracket(s)?;
    let inner = &s[1..close];
    if on.spans && s[close + 1..].starts_with('{') {
        let attr_end = s[close + 1..].find('}')? + close + 1;
        let attrs = attributes_html(&s[close + 2..attr_end]);
        return Some((
            attr_end + 1,
            format!("<span{attrs}>{}</span>", escape_html(inner)),
        ));
    }
    if on.citations && inner.contains('@') {
        let html = citation_group(inner)?;
        return Some((close + 1, html));
    }
    None
}

fn matching_bracket(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            '\n' => return None,
            _ => {}
        }
    }
    None
}

/// `see @doe99, p. 33; -@smith04` → a parenthesized citation.
fn citation_group(inner: &str) -> Option<String> {
    let mut keys = Vec::new();
    let mut parts = Vec::new();
    for item in inner.split(';') {
        let at = item.find('@')?;
        let (prefix, rest) = item.split_at(at);
        let prefix = prefix.trim_end();
        let prefix = prefix.strip_suffix('-').unwrap_or(prefix).trim_end();
        let (key, key_len) = citation_key(&rest[1..])?;
        let suffix = &rest[1 + key_len..];
        let mut part = String::new();
        if !prefix.is_empty() {
            part.push_str(&escape_html(prefix));
            part.push(' ');
        }
        part.push_str(&format!(
            "<a href=\"#ref-{}\">{}</a>",
            escape_html(key),
            escape_html(key)
        ));
        part.push_str(&escape_html(suffix.trim_end()));
        parts.push(part);
        keys.push(key);
    }
    if keys.is_empty() {
        return None;
    }
    Some(format!(
        "<span class=\"citation\" data-cites=\"{}\">({})</span>",
        escape_html(&keys.join(" ")),
        parts.join("; ")
    ))
}

/// A citation key at the start of `s` (after `@`) and its byte length.
fn citation_key(s: &str) -> Option<(&str, usize)> {
    let first = s.chars().next()?;
    if !(first.is_alphanumeric() || first == '_') {
        return None;
    }
    let mut end = 0;
    for (i, c) in s.char_indices() {
        if c.is_alphanumeric() || "_:.#$%&-+?<>~/".contains(c) {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    // Trailing punctuation belongs to the sentence.
    let key = s[..end].trim_end_matches(|c: char| ":.#$%&-+?<>~/".contains(c));
    (!key.is_empty()).then_some((key, key.len()))
}

fn bare_citation(s: &str) -> Option<(usize, String)> {
    let (key, len) = citation_key(&s[1..])?;
    Some((
        len + 1,
        format!(
            "<span class=\"citation\" data-cites=\"{0}\"><a href=\"#ref-{0}\">{0}</a></span>",
            escape_html(key)
        ),
    ))
}

fn tag(s: &str, ctx: &mut Ctx<'_>) -> Option<(usize, String)> {
    let body = &s[1..];
    let end = body
        .char_indices()
        .find(|&(_, c)| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '/')))
        .map_or(body.len(), |(i, _)| i);
    let name = body[..end].trim_end_matches(['/', '-']);
    if name.is_empty() || name.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !ctx.tags.iter().any(|t| t == name) {
        ctx.tags.push(name.to_owned());
    }
    Some((
        name.len() + 1,
        format!("<span class=\"tag\">#{}</span>", escape_html(name)),
    ))
}

/// `~sub~` or `^sup^`: no whitespace inside, not empty.
fn sub_sup(s: &str, delim: char) -> Option<(usize, String)> {
    let end = s[1..].find(delim)? + 1;
    let inner = &s[1..end];
    if inner.is_empty() || inner.chars().any(char::is_whitespace) {
        return None;
    }
    let tag = if delim == '~' { "sub" } else { "sup" };
    Some((end + 1, format!("<{tag}>{}</{tag}>", escape_html(inner))))
}

fn highlight(s: &str) -> Option<(usize, String)> {
    let end = s[2..].find("==")? + 2;
    let inner = &s[2..end];
    if inner.is_empty() || inner.starts_with(' ') || inner.ends_with(' ') {
        return None;
    }
    Some((end + 2, format!("<mark>{}</mark>", escape_html(inner))))
}

/// A GFM autolink literal at the start of `s`.
fn autolink(s: &str) -> Option<(usize, String)> {
    let lower_start: String = s.chars().take(8).collect::<String>().to_lowercase();
    let scheme = if lower_start.starts_with("https://") || lower_start.starts_with("http://") {
        ""
    } else if lower_start.starts_with("www.") {
        "http://"
    } else {
        return None;
    };
    let mut end = s
        .char_indices()
        .find(|&(_, c)| c.is_whitespace() || c == '<')
        .map_or(s.len(), |(i, _)| i);
    // Trailing punctuation and unbalanced closing parentheses are not part
    // of the link (GFM 6.9).
    loop {
        let candidate = &s[..end];
        let Some(last) = candidate.chars().next_back() else {
            return None;
        };
        if "?!.,:*_~'\"".contains(last) {
            end -= last.len_utf8();
        } else if last == ')' && candidate.matches(')').count() > candidate.matches('(').count() {
            end -= 1;
        } else if last == ';'
            && let Some(amp) = candidate.rfind('&')
            && candidate[amp + 1..end - 1].chars().all(|c| c.is_ascii_alphanumeric())
        {
            end = amp;
        } else {
            break;
        }
    }
    let url = &s[..end];
    let host = url.split("://").nth(1).unwrap_or(url);
    let domain = host.split(['/', '?', '#']).next().unwrap_or("");
    if !domain.contains('.') || domain.ends_with('.') || domain.contains('_') {
        return None;
    }
    Some((
        end,
        format!(
            "<a href=\"{}{}\">{}</a>",
            scheme,
            escape_html(url),
            escape_html(url)
        ),
    ))
}

/// Pandoc attributes (`#id .class key=value key="quoted value"`) as HTML
/// attributes with a leading space.
pub fn attributes_html(inner: &str) -> String {
    let mut id = None;
    let mut classes: Vec<&str> = Vec::new();
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut rest = inner.trim();
    while !rest.is_empty() {
        let (token, after) = next_token(rest);
        rest = after.trim_start();
        if let Some(i) = token.strip_prefix('#') {
            id = Some(i.to_owned());
        } else if let Some(c) = token.strip_prefix('.') {
            classes.push(c);
        } else if let Some((k, v)) = token.split_once('=') {
            let v = v.trim_matches('"');
            if k.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') && !k.is_empty() {
                pairs.push((k.to_owned(), v.to_owned()));
            }
        }
    }
    let mut out = String::new();
    if let Some(id) = id {
        out.push_str(&format!(" id=\"{}\"", escape_html(&id)));
    }
    if !classes.is_empty() {
        out.push_str(&format!(" class=\"{}\"", escape_html(&classes.join(" "))));
    }
    for (k, v) in pairs {
        // Unknown keys become data- attributes, as Pandoc does for HTML.
        let known = matches!(k.as_str(), "lang" | "dir" | "title" | "role") || k.starts_with("aria-");
        let name = if known || k.starts_with("data-") {
            k
        } else {
            format!("data-{k}")
        };
        out.push_str(&format!(" {}=\"{}\"", name, escape_html(&v)));
    }
    out
}

/// One attribute token, honoring double quotes.
fn next_token(s: &str) -> (&str, &str) {
    let mut quoted = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => return (&s[..i], &s[i..]),
            _ => {}
        }
    }
    (s, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, on: Inline) -> String {
        let mut tags = Vec::new();
        let mut ctx = Ctx {
            link_ext: ".html",
            tags: &mut tags,
        };
        match transform(text, on, &mut ctx) {
            None => format!("UNCHANGED:{text}"),
            Some(segs) => segs
                .into_iter()
                .map(|s| match s {
                    Seg::Text(t) => t,
                    Seg::Html(h) => h,
                })
                .collect(),
        }
    }

    const OBS: Inline = Inline {
        autolinks: false,
        wikilinks: true,
        tags: true,
        highlights: true,
        citations: false,
        spans: false,
        subsup: false,
    };
    const PANDOC: Inline = Inline {
        autolinks: false,
        wikilinks: false,
        tags: false,
        highlights: false,
        citations: true,
        spans: true,
        subsup: true,
    };

    #[test]
    fn wikilinks() {
        assert_eq!(
            run("See [[My Note]] now", OBS),
            "See <a class=\"wikilink\" href=\"My%20Note.html\">My Note</a> now"
        );
        assert_eq!(
            run("[[Note#Some Heading|there]]", OBS),
            "<a class=\"wikilink\" href=\"Note.html#some-heading\">there</a>"
        );
        assert_eq!(
            run("[[Note#^abc1]]", OBS),
            "<a class=\"wikilink\" href=\"Note.html#block-abc1\">Note: abc1</a>"
        );
        assert_eq!(
            run("[[#Local]]", OBS),
            "<a class=\"wikilink\" href=\"#local\">Local</a>"
        );
    }

    #[test]
    fn embeds() {
        assert_eq!(
            run("![[diagram.png|A flow chart]]", OBS),
            "<img src=\"diagram.png\" alt=\"A flow chart\">"
        );
        assert_eq!(
            run("![[photo.jpg|300]]", OBS),
            "<img src=\"photo.jpg\" alt=\"photo\">"
        );
        assert_eq!(
            run("![[Other]]", OBS),
            "<a class=\"embed\" href=\"Other.html\">Other</a>"
        );
    }

    #[test]
    fn tags_and_highlights() {
        assert_eq!(
            run("a #tag/sub and #42 and ==bright==", OBS),
            "a <span class=\"tag\">#tag/sub</span> and #42 and <mark>bright</mark>"
        );
        assert_eq!(run("issue#3", OBS), "UNCHANGED:issue#3");
    }

    #[test]
    fn citations_and_spans() {
        assert_eq!(
            run("As shown [see @doe99, p. 33; -@roe04].", PANDOC),
            "As shown <span class=\"citation\" data-cites=\"doe99 roe04\">(see <a href=\"#ref-doe99\">doe99</a>, p. 33; <a href=\"#ref-roe04\">roe04</a>)</span>."
        );
        assert_eq!(
            run("@doe99 says.", PANDOC),
            "<span class=\"citation\" data-cites=\"doe99\"><a href=\"#ref-doe99\">doe99</a></span> says."
        );
        assert_eq!(run("mail me@example.org", PANDOC), "UNCHANGED:mail me@example.org");
        assert_eq!(
            run("[small caps]{.smallcaps lang=fr}", PANDOC),
            "<span class=\"smallcaps\" lang=\"fr\">small caps</span>"
        );
        assert_eq!(run("H~2~O, 2^10^, a ~ b", PANDOC), "H<sub>2</sub>O, 2<sup>10</sup>, a ~ b");
    }

    #[test]
    fn gfm_autolinks() {
        let on = Inline {
            autolinks: true,
            ..Inline::default()
        };
        assert_eq!(
            run("Visit www.example.com/a_(b). Or https://x.org?q=1!", on),
            "Visit <a href=\"http://www.example.com/a_(b)\">www.example.com/a_(b)</a>. Or <a href=\"https://x.org?q=1\">https://x.org?q=1</a>!"
        );
        assert_eq!(run("not a link: www.", on), "UNCHANGED:not a link: www.");
    }
}
