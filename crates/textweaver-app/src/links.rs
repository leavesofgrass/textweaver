//! Following links and footnotes (Alt+Shift+F).
//!
//! - On a footnote reference, the cursor goes to its note; on the note, back
//!   to the (first) reference. Both are jumps, so Alt+Left returns too.
//! - On a link to a heading in the document (`#methods`), the cursor goes
//!   to the heading whose id or text matches.
//! - On a link to a local file (`chapter-2.md`, `notes/a.md#part`, an
//!   Obsidian `[[wiki link]]`), the file opens (asking about unsaved edits
//!   first), at the heading when the link names one. Alt+Left, before any
//!   other jump there, goes back to the link.
//! - A web or mail link is said, with an offer to open it in the browser.

use std::path::{Path, PathBuf};

use textweaver_core::{CharPos, MarkerKind, Unit};
use textweaver_lexicon::args;
use textweaver_speech::Earcon;
use textweaver_text::Document;
use textweaver_text::units::unit_at;

use crate::app::App;
use crate::authoring_state::{LinkBack, file_name};
use crate::command::Effect;
use crate::nav::ReadAfter;
use crate::text_util;

/// Decodes `%20` and the like in a link target.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// True for a link target with a scheme (`https:`, `mailto:`), not a
/// Windows drive letter.
fn has_scheme(target: &str) -> bool {
    match target.split_once(':') {
        Some((scheme, _)) => {
            scheme.len() > 1
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        }
        None => false,
    }
}

/// The heading `anchor` names in `doc`: by its id (`methods`,
/// `data-and-methods`) or its text, ignoring case.
pub(crate) fn heading_for_anchor(doc: &Document, anchor: &str) -> Option<CharPos> {
    let want = anchor.trim().trim_start_matches('#');
    if want.is_empty() {
        return None;
    }
    let want_slug = textweaver_text::slug::slugify(want);
    doc.marker_index()
        .iter(MarkerKind::Heading, None)
        .find(|m| {
            let text = doc.slice(m.range);
            let text = text.trim();
            text.eq_ignore_ascii_case(want) || textweaver_text::slug::slugify(text) == want_slug
        })
        .map(|m| m.range.start)
}

/// The local file a link target names, relative to `folder`, if it exists:
/// the path as written, else with `.md` added (wiki links), else a file of
/// that name anywhere under the folder (Obsidian resolves by name).
pub(crate) fn resolve_local(folder: &Path, target: &str) -> Option<PathBuf> {
    let direct = folder.join(target);
    // A file, or a file inside an archive (`book.zip!chapter.xml`, the links
    // an archive's listing holds).
    if direct.is_file() || textweaver_formats::archive::exists(&direct) {
        return Some(direct);
    }
    let with_md = folder.join(format!("{target}.md"));
    if Path::new(target).extension().is_none() && with_md.is_file() {
        return Some(with_md);
    }
    let name = Path::new(target)
        .file_name()?
        .to_string_lossy()
        .to_lowercase();
    let wanted = [name.clone(), format!("{name}.md")];
    find_by_name(folder, &wanted, 4)
}

fn find_by_name(dir: &Path, wanted: &[String], depth: usize) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut dirs = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_lowercase();
        if path.is_file() && wanted.contains(&name) {
            return Some(path);
        }
        if path.is_dir() && !name.starts_with('.') {
            dirs.push(path);
        }
    }
    if depth == 0 {
        return None;
    }
    dirs.sort();
    dirs.iter().find_map(|d| find_by_name(d, wanted, depth - 1))
}

impl App {
    /// Alt+Shift+F: follows the link or footnote at the cursor.
    pub(crate) fn follow_link(&mut self) -> Vec<Effect> {
        self.refresh_structure(false);
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let pos = s.cursor;
        if let Some(effects) = self.follow_footnote(pos) {
            return effects;
        }
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let index = s.doc.marker_index();
        let link = index
            .enclosing(MarkerKind::Link, pos)
            .or_else(|| {
                unit_at(&s.doc, pos, Unit::Word)
                    .and_then(|w| index.enclosing(MarkerKind::Link, w.start))
            })
            .and_then(|m| m.reference.clone().map(|r| (s.doc.slice(m.range), r)));
        let Some((text, target)) = link else {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg("links-none-here");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let target = target.trim().to_owned();
        if target.is_empty() {
            let msg = self.msg_args("links-no-address", &args!["text" => text.trim()]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if let Some(anchor) = target.strip_prefix('#') {
            self.go_to_anchor(&percent_decode(anchor));
            return vec![Effect::Redraw];
        }
        if has_scheme(&target) {
            let lower = target.to_ascii_lowercase();
            if lower.starts_with("file:") {
                let path = percent_decode(
                    target
                        .trim_start_matches("file://")
                        .trim_start_matches("file:"),
                );
                return self.follow_local(&path);
            }
            let kind = if lower.starts_with("mailto:") {
                "mail"
            } else {
                "web"
            };
            let question = self.msg_args(
                "links-open-question",
                &args!["kind" => kind, "target" => target.as_str()],
            );
            self.offer_open(target.clone(), &question);
            return vec![Effect::Redraw];
        }
        self.follow_local(&percent_decode(&target))
    }

    /// Goes to the heading `anchor` names in this document.
    fn go_to_anchor(&mut self, anchor: &str) {
        let found = self
            .session
            .as_ref()
            .and_then(|s| heading_for_anchor(&s.doc, anchor));
        match found {
            Some(pos) => {
                let text = self
                    .session
                    .as_ref()
                    .map(|s| {
                        text_util::preview(
                            &s.doc,
                            text_util::line_range(&s.doc, text_util::line_of(&s.doc, pos)),
                            10,
                        )
                    })
                    .unwrap_or_default();
                let label = self.msg("links-heading-label");
                let msg = self.nav_message(Some(&label), pos, &text);
                self.jump(pos, true, ReadAfter::Follow, &msg);
            }
            None => {
                self.speech.earcon(Earcon::Error);
                let msg = self.msg_args("links-no-heading", &args!["anchor" => anchor]);
                self.tell(&msg);
            }
        }
    }

    /// Opens the local file a link names (and goes to its heading).
    fn follow_local(&mut self, target: &str) -> Vec<Effect> {
        let (file, anchor) = match target.split_once('#') {
            Some((f, a)) => (f, Some(a)),
            None => (target, None),
        };
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let here = s.doc.meta.path.clone();
        let folder = here
            .as_ref()
            .and_then(|p| p.parent().map(Path::to_owned))
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        if file.is_empty() {
            if let Some(a) = anchor {
                self.go_to_anchor(a);
            }
            return vec![Effect::Redraw];
        }
        let Some(path) = resolve_local(&folder, file) else {
            self.speech.earcon(Earcon::Error);
            let msg = self.msg_args("links-file-not-found", &args!["file" => file]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let same = here
            .as_ref()
            .is_some_and(|h| std::fs::canonicalize(h).ok() == std::fs::canonicalize(&path).ok());
        if same {
            if let Some(a) = anchor {
                self.go_to_anchor(a);
            }
            return vec![Effect::Redraw];
        }
        let from = here.clone();
        let pos = self.reading_position().unwrap_or(CharPos::ZERO);
        let effects = self.open_command(path.clone());
        // The file opened now (not waiting on a question about unsaved
        // edits): go to the heading and remember the way back.
        let opened = self
            .session
            .as_ref()
            .and_then(|s| s.doc.meta.path.clone())
            .is_some_and(|p| p == path);
        if opened {
            if let Some(a) = anchor.filter(|a| !a.is_empty()) {
                let found = self
                    .session
                    .as_ref()
                    .and_then(|s| heading_for_anchor(&s.doc, a));
                if let Some(p) = found
                    && let Some(s) = self.session.as_mut()
                {
                    s.cursor = p;
                    self.scroll_to_cursor();
                }
            }
            if let Some(from) = from {
                let history_len = self
                    .session
                    .as_ref()
                    .map_or(0, |s| s.history.entries().len());
                self.authoring.link_back.push(LinkBack {
                    from,
                    pos,
                    to: path.clone(),
                    history_len,
                });
                let back = self.keys(textweaver_keymap::ActionId::HistoryBack);
                let msg = self.msg_args(
                    "links-followed",
                    &args!["file" => file_name(&path), "key" => back],
                );
                self.tell(&msg);
            }
        }
        effects
    }

    /// History back to the file a link was followed from, when nothing was
    /// jumped to since arriving. True when it went back.
    pub(crate) fn link_history_back(&mut self) -> bool {
        let Some(top) = self.authoring.link_back.last().cloned() else {
            return false;
        };
        let Some(s) = self.session.as_ref() else {
            return false;
        };
        let here = s.doc.meta.path.clone();
        if here.as_ref() != Some(&top.to) || s.history.entries().len() > top.history_len {
            return false;
        }
        self.authoring.link_back.pop();
        self.open_command(top.from.clone());
        let back = self
            .session
            .as_ref()
            .and_then(|s| s.doc.meta.path.clone())
            .is_some_and(|p| p == top.from);
        if back {
            if let Some(s) = self.session.as_mut() {
                s.cursor = top.pos.clamp_to(s.doc.len_chars());
            }
            self.scroll_to_cursor();
            let msg = self.msg_args("links-back-in", &args!["file" => file_name(&top.from)]);
            self.tell(&msg);
        }
        true
    }

    /// On a footnote reference, goes to its note; on the note, back to the
    /// reference. `None` when the cursor is on neither.
    fn follow_footnote(&mut self, pos: CharPos) -> Option<Vec<Effect>> {
        let s = self.session.as_ref()?;
        let doc = &s.doc;
        let notes: Vec<&textweaver_text::Marker> = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Footnote)
            .collect();
        let here: Vec<&&textweaver_text::Marker> = notes
            .iter()
            .filter(|m| m.range.start <= pos && pos <= m.range.end)
            .collect();
        if here.is_empty() {
            return None;
        }
        let body = here.iter().find(|m| m.level == 1);
        let reference = here.iter().find(|m| m.level == 0);
        let label = body
            .or(reference)
            .and_then(|m| m.reference.clone())
            .unwrap_or_default();
        let in_body = |p: CharPos| {
            notes
                .iter()
                .any(|b| b.level == 1 && b.range.contains(p) && b.range.start < p)
        };
        let (target, msg) = if let Some(b) = body {
            // On the note: back to the first reference outside any note.
            let back = notes
                .iter()
                .find(|m| {
                    m.level == 0
                        && m.reference.as_deref() == Some(label.as_str())
                        && !in_body(m.range.start)
                        && !(b.range.contains(m.range.start))
                })
                .map(|m| m.range.start);
            let inline = !notes
                .iter()
                .any(|m| m.level == 0 && b.range.contains_range(m.range));
            match back {
                Some(p) => {
                    let line = text_util::line_of(doc, p) + 1;
                    (
                        p,
                        self.msg_args(
                            "links-back-to-footnote-reference",
                            &args!["label" => label.as_str(), "line" => line],
                        ),
                    )
                }
                // Read in place: the note is here; say it.
                None if inline => {
                    let text = text_util::preview(doc, b.range, 40);
                    let msg = self.msg_args(
                        "links-footnote",
                        &args!["label" => label.as_str(), "text" => text],
                    );
                    self.tell(&msg);
                    return Some(vec![Effect::Redraw]);
                }
                None => {
                    let msg = self.msg_args(
                        "links-footnote-unreferenced",
                        &args!["label" => label.as_str()],
                    );
                    self.tell(&msg);
                    return Some(vec![Effect::Redraw]);
                }
            }
        } else {
            let note = notes
                .iter()
                .find(|m| m.level == 1 && m.reference.as_deref() == Some(label.as_str()));
            match note {
                Some(m) => {
                    let text = text_util::preview(doc, m.range, 20);
                    (
                        m.range.start,
                        self.msg_args(
                            "links-footnote",
                            &args!["label" => label.as_str(), "text" => text],
                        ),
                    )
                }
                None => {
                    let msg =
                        self.msg_args("links-footnote-no-note", &args!["label" => label.as_str()]);
                    self.tell(&msg);
                    return Some(vec![Effect::Redraw]);
                }
            }
        };
        self.jump(target, true, ReadAfter::Follow, &msg);
        Some(vec![Effect::Redraw])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_decode_and_schemes_are_told_from_paths() {
        assert_eq!(percent_decode("my%20notes.md"), "my notes.md");
        assert_eq!(percent_decode("100%"), "100%");
        assert!(has_scheme("https://example.org"));
        assert!(has_scheme("mailto:a@b.c"));
        assert!(!has_scheme("C:\\notes\\a.md"));
        assert!(!has_scheme("notes/a.md"));
    }
}
