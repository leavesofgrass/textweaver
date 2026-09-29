//! Obsidian's Markdown in the reader (ADR-0044): what the Markdown loader
//! needs beyond CommonMark to read a note from a vault.
//!
//! - **Embeds** (`![[note]]`, `![[note#Heading]]`, `![[note#^id]]`): the
//!   other note is read in place, found inside the embedding note's own
//!   folder or below it, never outside; two levels deep at most, a cycle
//!   refused, and each note and the total bounded in size.
//! - **Inline marks**: tags (`#physics/waves`, read "tag physics slash
//!   waves"), highlights (`==text==`), and comments (`%%hidden%%`, not
//!   read).
//! - **Block ids** (`^id` ending a paragraph or list item, or alone on the
//!   line after a table or list): not read, and kept in the document's
//!   properties so a link to `note#^id` can find the block
//!   ([`block_position`]).

use std::path::{Component, Path, PathBuf};

use textweaver_core::CharPos;
use textweaver_text::DocumentMeta;

use crate::encoding;

/// Deepest embed read: a note embedded in a note embedded in the document.
pub const MAX_EMBED_DEPTH: usize = 2;

/// Largest note read as an embed.
pub const MAX_EMBED_BYTES: usize = 4 << 20;

/// Most bytes read from embedded notes in one document.
pub const MAX_EMBED_TOTAL: usize = 16 << 20;

/// Most embeds read in one document; later ones are links.
pub const MAX_EMBEDS: usize = 64;

/// How deep below the note's folder the search for an embedded note by its
/// name goes, and how many folder entries it looks at in all.
const SEARCH_DEPTH: usize = 8;
const SEARCH_ENTRIES: usize = 20_000;

/// The `DocumentMeta::properties` key holding the block ids a note
/// defines: one `id offset` pair per line, the offset in characters.
pub const BLOCK_IDS_PROPERTY: &str = "textweaver.block_ids";

/// Extensions read as pictures when embedded.
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "avif", "tif", "tiff",
];

/// What an embed names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EmbedTarget {
    /// A picture: read as a graphic by its file name.
    Image,
    /// Another note, with a heading or block id to read only that part.
    Note {
        /// The note's name or path as written, without `.md`.
        name: String,
        /// `Heading`, `^id`, or nothing for the whole note.
        part: Option<String>,
    },
    /// Anything else (a PDF, a sound): a link.
    Other,
}

/// Reads an embed's target (`note#Heading`, `image.png`).
pub(crate) fn embed_target(target: &str) -> EmbedTarget {
    let (file, part) = match target.split_once('#') {
        Some((f, p)) => (
            f.trim(),
            Some(p.trim().to_owned()).filter(|p| !p.is_empty()),
        ),
        None => (target.trim(), None),
    };
    let ext = Path::new(file)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase());
    match ext.as_deref() {
        Some(e) if IMAGE_EXTENSIONS.contains(&e) => EmbedTarget::Image,
        None | Some("md") => EmbedTarget::Note {
            name: file.trim_end_matches(".md").to_owned(),
            part,
        },
        // `note.v2` is a note whose name has a dot.
        Some(e) if !e.chars().all(|c| c.is_ascii_alphanumeric()) || e.len() > 5 => {
            EmbedTarget::Note {
                name: file.to_owned(),
                part,
            }
        }
        Some(_) => EmbedTarget::Other,
    }
}

/// The embeds read so far in one document, and where they may come from.
#[derive(Clone, Debug)]
pub(crate) struct Embeds {
    /// The embedding document's folder, resolved: embeds come from here or
    /// below.
    root: PathBuf,
    /// The notes being read, outermost first (the document itself too), to
    /// refuse a cycle.
    chain: Vec<PathBuf>,
    /// Bytes and embeds read in the whole document.
    used: std::rc::Rc<std::cell::Cell<(usize, usize)>>,
}

/// Why an embed was not read in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    /// No such note inside the folder.
    NotFound,
    /// It is already being read (a cycle).
    Cycle,
    /// Two levels deep already.
    TooDeep,
    /// Too large, too many embeds, or not text.
    TooLarge,
}

impl Refused {
    /// Said after the note's name, in the canonical text.
    pub(crate) fn words(self) -> &'static str {
        match self {
            Refused::NotFound => "(embedded note not found)",
            Refused::Cycle => "(embedded above, not repeated)",
            Refused::TooDeep => "(embedded too deeply to read here)",
            Refused::TooLarge => "(embedded note too large to read here)",
        }
    }
}

impl Embeds {
    /// Embeds for a document in `folder`, itself at `path` when known.
    pub(crate) fn new(folder: &Path, path: Option<&Path>) -> Option<Self> {
        let root = folder.canonicalize().ok()?;
        let chain = path
            .and_then(|p| p.canonicalize().ok())
            .into_iter()
            .collect();
        Some(Embeds {
            root,
            chain,
            used: std::rc::Rc::default(),
        })
    }

    /// The embeds for reading `note` inside this one.
    pub(crate) fn child(&self, note: PathBuf) -> Self {
        let mut chain = self.chain.clone();
        if chain.is_empty() {
            // The document had no path of its own: count it anyway.
            chain.push(self.root.clone());
        }
        chain.push(note);
        Embeds {
            root: self.root.clone(),
            chain,
            used: self.used.clone(),
        }
    }

    /// Reads the note `name` for embedding: its resolved path and text.
    pub(crate) fn read(&self, name: &str) -> Result<(PathBuf, String), Refused> {
        let base = self.chain.len().max(1);
        if base > MAX_EMBED_DEPTH {
            return Err(Refused::TooDeep);
        }
        let path = self.resolve(name).ok_or(Refused::NotFound)?;
        if self.chain.contains(&path) {
            return Err(Refused::Cycle);
        }
        let (bytes_used, count) = self.used.get();
        if count >= MAX_EMBEDS {
            return Err(Refused::TooLarge);
        }
        let len = std::fs::metadata(&path)
            .map_err(|_| Refused::NotFound)?
            .len();
        let len = usize::try_from(len).unwrap_or(usize::MAX);
        if len > MAX_EMBED_BYTES || bytes_used.saturating_add(len) > MAX_EMBED_TOTAL {
            return Err(Refused::TooLarge);
        }
        let bytes = std::fs::read(&path).map_err(|_| Refused::NotFound)?;
        if encoding::binary_kind(&bytes[..bytes.len().min(encoding::SNIFF_BYTES)]).is_some() {
            return Err(Refused::TooLarge);
        }
        self.used.set((bytes_used + bytes.len(), count + 1));
        Ok((path, crate::decode_bytes(&bytes, None).text))
    }

    /// The note `name` names, inside the root folder: the path as written
    /// (with `.md` added), relative to the root, else a note of that file
    /// name anywhere below the root, as Obsidian finds notes by name.
    fn resolve(&self, name: &str) -> Option<PathBuf> {
        let rel = PathBuf::from(name.replace('\\', "/"));
        if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            return None;
        }
        let file_name = rel.file_name()?.to_string_lossy().to_lowercase();
        let with_md = if file_name.ends_with(".md") {
            file_name.clone()
        } else {
            format!("{file_name}.md")
        };
        let inside = |p: PathBuf| -> Option<PathBuf> {
            let real = p.canonicalize().ok()?;
            (real.starts_with(&self.root) && real.is_file()).then_some(real)
        };
        let mut direct = self.root.join(&rel);
        if !direct.to_string_lossy().to_lowercase().ends_with(".md") {
            direct.as_mut_os_string().push(".md");
        }
        if let Some(found) = inside(direct) {
            return Some(found);
        }
        let mut seen = 0usize;
        find_by_name(&self.root, &with_md, SEARCH_DEPTH, &mut seen).and_then(inside)
    }
}

/// A file named `wanted` (lowercase) under `dir`, nearest first, skipping
/// hidden folders (`.obsidian`, `.git`).
fn find_by_name(dir: &Path, wanted: &str, depth: usize, seen: &mut usize) -> Option<PathBuf> {
    let mut dirs = Vec::new();
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        *seen += 1;
        if *seen > SEARCH_ENTRIES {
            return None;
        }
        let name = e.file_name().to_string_lossy().to_lowercase();
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_file() && name == wanted {
            return Some(e.path());
        }
        if kind.is_dir() && !name.starts_with('.') {
            dirs.push(e.path());
        }
    }
    if depth == 0 {
        return None;
    }
    dirs.sort();
    dirs.iter()
        .find_map(|d| find_by_name(d, wanted, depth - 1, seen))
}

/// The lines of `text` outside fenced code blocks, with their byte offsets.
fn lines_outside_code(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        let ch = t.chars().next();
        if let Some(c @ ('`' | '~')) = ch {
            let n = t.chars().take_while(|&x| x == c).count();
            if n >= 3 {
                match fence {
                    None => fence = Some((c, n)),
                    Some((fc, fnum)) if fc == c && n >= fnum => fence = None,
                    _ => {}
                }
                at += line.len();
                continue;
            }
        }
        if fence.is_none() {
            out.push((at, line));
        }
        at += line.len();
    }
    out
}

/// An ATX heading line's level and text (`## Methods ##`).
fn atx(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_start();
    if line.len() - t.len() > 3 {
        return None;
    }
    let level = t.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &t[level..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t', '\n', '\r']) {
        return None;
    }
    Some((level, rest.trim().trim_end_matches('#').trim()))
}

/// The part of a note under `heading` (its text, ignoring case, or its
/// slug), up to the next heading of the same or a higher level; the
/// heading line itself included.
pub(crate) fn heading_section<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let want = heading.trim();
    let want_slug = textweaver_text::slug::slugify(want);
    let lines = lines_outside_code(text);
    let (i, level) = lines.iter().enumerate().find_map(|(i, (_, l))| {
        let (level, h) = atx(l)?;
        (h.eq_ignore_ascii_case(want) || textweaver_text::slug::slugify(h) == want_slug)
            .then_some((i, level))
    })?;
    let start = lines[i].0;
    let end = lines[i + 1..]
        .iter()
        .find(|(_, l)| atx(l).is_some_and(|(lv, _)| lv <= level))
        .map_or(text.len(), |(at, _)| *at);
    text.get(start..end)
}

/// The block a block id names: the paragraph or list item whose last line
/// ends `^id`, or the block before a line that is only `^id`. The id itself
/// is left out.
pub(crate) fn block_section(text: &str, id: &str) -> Option<String> {
    let lines = lines_outside_code(text);
    let is_id_line = |l: &str| {
        let t = l.trim_end();
        t.strip_suffix(id)
            .and_then(|r| r.strip_suffix('^'))
            .is_some_and(|before| before.is_empty() || before.ends_with(char::is_whitespace))
    };
    let i = lines.iter().position(|(_, l)| is_id_line(l))?;
    let line = lines[i].1.trim_end();
    let alone = line.trim() == format!("^{id}");
    let strip = |l: &str| -> String {
        let t = l.trim_end();
        t.strip_suffix(id)
            .and_then(|r| r.strip_suffix('^'))
            .unwrap_or(t)
            .trim_end()
            .to_owned()
    };
    let is_blank = |l: &str| l.trim().is_empty();
    let list_item = |l: &str| {
        let t = l.trim_start();
        t.starts_with(['-', '*', '+'])
            || t.split_once(['.', ')'])
                .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    };
    // The block ends at line `last` and starts after a blank line.
    let last = if alone {
        // Skip blank lines back to the block before the id.
        let mut j = i;
        while j > 0 && is_blank(lines[j - 1].1) {
            j -= 1;
        }
        j.checked_sub(1)?
    } else {
        i
    };
    if !alone && list_item(lines[last].1) {
        return Some(strip(lines[last].1));
    }
    let mut first = last;
    while first > 0
        && !is_blank(lines[first - 1].1)
        && atx(lines[first].1).is_none()
        && atx(lines[first - 1].1).is_none()
    {
        first -= 1;
    }
    let mut out: Vec<String> = lines[first..=last]
        .iter()
        .map(|(_, l)| l.trim_end().to_owned())
        .collect();
    if !alone && let Some(l) = out.last_mut() {
        *l = strip(l);
    }
    Some(out.join("\n"))
}

/// A trailing block id in a run of text (`Some text ^abc-1`): the text
/// before it, and the id.
pub(crate) fn trailing_block_id(text: &str) -> Option<(&str, &str)> {
    let t = text.trim_end();
    let caret = t.rfind('^')?;
    let id = &t[caret + 1..];
    let before = &t[..caret];
    let ok_id = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    (ok_id && (before.is_empty() || before.ends_with(char::is_whitespace)))
        .then_some((before.trim_end(), id))
}

/// A tag's words: `physics/waves` is "tag physics slash waves". Tags are
/// letters, digits, `_`, `-`, and `/`, and not only digits (`#1` is a
/// number).
pub(crate) fn tag_words(tag: &str) -> Option<String> {
    let tag = tag.trim_end_matches('/');
    if tag.is_empty() || tag.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let parts: Vec<&str> = tag.split('/').filter(|p| !p.is_empty()).collect();
    Some(format!("tag {}", parts.join(" slash ")))
}

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}

/// A piece of a text run, as Obsidian's inline marks cut it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Piece<'a> {
    /// Plain text.
    Text(&'a str),
    /// A tag's words ("tag physics slash waves").
    Tag(String),
    /// `==`: a highlight starts or ends.
    Mark,
    /// `%%`: a comment starts or ends.
    Comment,
}

/// Cuts a run of text at tags, `==`, and `%%`. `boundary` says whether
/// the run starts after white space or at a line start, where a `#` can
/// start a tag.
pub(crate) fn pieces(s: &str, boundary: bool) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut from = 0;
    let mut prev_space = boundary;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let next = s[i + c.len_utf8()..].chars().next();
        match c {
            '=' | '%' if next == Some(c) => {
                if from < i {
                    out.push(Piece::Text(&s[from..i]));
                }
                out.push(if c == '=' {
                    Piece::Mark
                } else {
                    Piece::Comment
                });
                chars.next();
                from = i + 2;
                prev_space = false;
                continue;
            }
            '#' if prev_space => {
                let rest = &s[i + 1..];
                let len = rest.find(|c: char| !is_tag_char(c)).unwrap_or(rest.len());
                if let Some(words) = tag_words(&rest[..len]) {
                    if from < i {
                        out.push(Piece::Text(&s[from..i]));
                    }
                    out.push(Piece::Tag(words));
                    from = i + 1 + len;
                    while chars.peek().is_some_and(|&(j, _)| j < from) {
                        chars.next();
                    }
                    prev_space = false;
                    continue;
                }
            }
            _ => {}
        }
        prev_space = c.is_whitespace();
    }
    if from < s.len() {
        out.push(Piece::Text(&s[from..]));
    }
    out
}

/// Records block ids in `meta`: `id offset` lines.
pub(crate) fn set_block_ids(meta: &mut DocumentMeta, ids: &[(String, usize)]) {
    if ids.is_empty() {
        return;
    }
    let lines: Vec<String> = ids.iter().map(|(id, at)| format!("{id} {at}")).collect();
    meta.properties
        .insert(BLOCK_IDS_PROPERTY.to_owned(), lines.join("\n"));
}

/// Where the block with id `id` (with or without its `^`) starts, in a
/// document the Markdown loader read: for following a link to
/// `note#^id`.
pub fn block_position(meta: &DocumentMeta, id: &str) -> Option<CharPos> {
    let id = id.trim().trim_start_matches('#').trim_start_matches('^');
    meta.properties
        .get(BLOCK_IDS_PROPERTY)?
        .lines()
        .find_map(|l| {
            let (k, at) = l.split_once(' ')?;
            (k == id).then(|| at.parse().ok().map(CharPos)).flatten()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets() {
        assert_eq!(embed_target("pic.PNG"), EmbedTarget::Image);
        assert_eq!(
            embed_target("Notes/Idea#Part two"),
            EmbedTarget::Note {
                name: "Notes/Idea".into(),
                part: Some("Part two".into())
            }
        );
        assert_eq!(
            embed_target("idea.md"),
            EmbedTarget::Note {
                name: "idea".into(),
                part: None
            }
        );
        assert_eq!(embed_target("slides.pdf"), EmbedTarget::Other);
    }

    #[test]
    fn sections_by_heading_and_block() {
        let text = "# A\n\none\n\n## B\n\ntwo\n\n```\n# not a heading\n```\n\n## C\nthree ^c1\n\n- item ^li\n- other\n\n| t |\n|---|\n| 1 |\n\n^tbl\n";
        assert_eq!(
            heading_section(text, "b"),
            Some("## B\n\ntwo\n\n```\n# not a heading\n```\n\n")
        );
        assert_eq!(heading_section(text, "missing"), None);
        assert_eq!(block_section(text, "c1").as_deref(), Some("three"));
        assert_eq!(block_section(text, "li").as_deref(), Some("- item"));
        assert_eq!(
            block_section(text, "tbl").as_deref(),
            Some("| t |\n|---|\n| 1 |")
        );
    }

    #[test]
    fn inline_pieces() {
        assert_eq!(
            pieces("see #physics/waves and #1 then ==hi== %%x%%", true),
            vec![
                Piece::Text("see "),
                Piece::Tag("tag physics slash waves".into()),
                Piece::Text(" and #1 then "),
                Piece::Mark,
                Piece::Text("hi"),
                Piece::Mark,
                Piece::Text(" "),
                Piece::Comment,
                Piece::Text("x"),
                Piece::Comment,
            ]
        );
        assert_eq!(pieces("C#sharp", true), vec![Piece::Text("C#sharp")]);
        assert_eq!(pieces("#tag", false), vec![Piece::Text("#tag")]);
        assert_eq!(
            trailing_block_id("Some text ^abc-1"),
            Some(("Some text", "abc-1"))
        );
        assert_eq!(trailing_block_id("x^2"), None);
    }
}
