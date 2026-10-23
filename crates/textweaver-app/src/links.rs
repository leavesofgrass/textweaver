//! Following links and footnotes (Enter on a link in reading mode, or
//! Alt+Shift+F anywhere).
//!
//! - On a footnote reference, the cursor goes to its note; on the note, back
//!   to the (first) reference. Both are jumps, so Alt+Left returns too.
//! - On a link to a heading in the document (`#methods`), the cursor goes
//!   to the heading whose id or text matches.
//! - On a link to a local file (`chapter-2.md`, `notes/a.md#part`, an
//!   Obsidian `[[wiki link]]`), the file opens (asking about unsaved edits
//!   first), at the heading or block when the link names one
//!   (`[[Note#Heading]]`, `[[Note#A#B]]`, `[[Note#^id]]`), or at the start.
//!   Wiki links resolve anywhere in the note's Obsidian vault and never
//!   outside it, by path, name, title, or alias (`textweaver_vault`); when
//!   two notes share the name, the nearest opens and "2 notes named X" is
//!   said once.
//!   Back (Alt+Left) goes back to the link once nothing earlier is left in
//!   the new document's history, or at once when nothing was jumped to
//!   there; Forward (Alt+Right) then returns. The documents form one
//!   history stack, as pages do in a web browser.
//! - A web or mail link is said, with an offer to open it in the browser.

use std::path::{Path, PathBuf};

use textweaver_core::{CharPos, CharRange, MarkerKind, Unit};
use textweaver_lexicon::args;
use textweaver_speech::Earcon;
use textweaver_text::Document;
use textweaver_text::units::unit_at;

use crate::app::App;
use crate::authoring_state::{LinkBack, LinkForward};
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
        .or_else(|| textweaver_formats::pdf::page_anchor(doc, want))
}

/// Where an anchor of `doc` points: a block (`^id`, from the Markdown
/// loader's block ids), a heading by id or text (the whole anchor, then
/// the last heading of a nested path `A#B`), or a PDF page.
pub(crate) fn anchor_position(doc: &Document, anchor: &str) -> Option<CharPos> {
    let a = anchor.trim().trim_start_matches('#').trim();
    if let Some(id) = a.strip_prefix('^') {
        return textweaver_formats::obsidian::block_position(&doc.meta, id);
    }
    textweaver_vault::Anchor::Heading(a.to_owned())
        .heading_candidates()
        .into_iter()
        .find_map(|h| heading_for_anchor(doc, h))
}

/// The block id an anchor names (`^id`), if it names one.
fn block_id(anchor: &str) -> Option<&str> {
    anchor.trim().trim_start_matches('#').strip_prefix('^')
}

/// The local file a link target names, for a document in `folder`, if
/// it exists: a file inside an archive (`book.zip!chapter.xml`), else the
/// vault resolver's answer (a path from the folder, then from the vault
/// root, then by name, nearest first), with how many files share the name.
pub(crate) fn resolve_local(folder: &Path, target: &str) -> Option<(PathBuf, usize)> {
    let direct = folder.join(target);
    if !direct.is_file() && textweaver_formats::archive::exists(&direct) {
        return Some((direct, 1));
    }
    let scope = textweaver_vault::LinkScope::for_folder(folder);
    let files = scope.files();
    textweaver_vault::resolve(&scope, &files, folder, target).map(|r| (r.path, r.same_name))
}

/// Where a document opening because of a link, Back, or Forward puts the
/// cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ArriveAt {
    /// The heading a link names, or the start when it names none.
    Anchor(Option<String>),
    /// The reading position the document was left at.
    Pos(CharPos),
}

/// Why the document is opening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arrival {
    Follow,
    Back,
    Forward,
}

/// A document opening because of a link, Back, or Forward. It is applied
/// when the document opens, which may be after the question about unsaved
/// edits or after a large file loads in the background.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkArrival {
    pub(crate) to: PathBuf,
    pub(crate) at: ArriveAt,
    pub(crate) kind: Arrival,
    /// The document left, and the reading position in it.
    pub(crate) leaving: Option<(PathBuf, CharPos)>,
    /// Said after the arrival: "2 notes named Dup; the nearest opened."
    pub(crate) note: Option<String>,
}

/// True when `a` and `b` name the same file.
fn same_file(a: &Path, b: &Path) -> bool {
    a == b
        || matches!(
            (std::fs::canonicalize(a), std::fs::canonicalize(b)),
            (Ok(x), Ok(y)) if x == y
        )
}

impl App {
    /// The link (its text and address) at the cursor, or on its word.
    fn link_at_cursor(&self) -> Option<(String, String)> {
        let s = self.session.as_ref()?;
        let pos = s.cursor;
        let index = s.doc.marker_index();
        index
            .enclosing(MarkerKind::Link, pos)
            .or_else(|| {
                unit_at(&s.doc, pos, Unit::Word)
                    .and_then(|w| index.enclosing(MarkerKind::Link, w.start))
            })
            .and_then(|m| m.reference.clone().map(|r| (s.doc.slice(m.range), r)))
    }

    /// True when the cursor is on a link or a footnote (Enter follows it).
    pub(crate) fn link_or_footnote_here(&mut self) -> bool {
        self.refresh_structure(false);
        let Some(s) = self.session.as_ref() else {
            return false;
        };
        let pos = s.cursor;
        let on_note =
            s.doc.markers().iter().any(|m| {
                m.kind == MarkerKind::Footnote && m.range.start <= pos && pos <= m.range.end
            });
        on_note || self.link_at_cursor().is_some()
    }

    /// Alt+Shift+F, or Enter on a link: follows the link or footnote at
    /// the cursor.
    pub(crate) fn follow_link(&mut self) -> Vec<Effect> {
        self.refresh_structure(false);
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let pos = s.cursor;
        if let Some(effects) = self.follow_footnote(pos) {
            return effects;
        }
        let Some((text, target)) = self.link_at_cursor() else {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg("links-none-here");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let target = target.trim().to_owned();
        if target.is_empty() {
            let msg = self.msg_args("common-link-no-address", &args!["text" => text.trim()]);
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
            // Only web and mail addresses are offered (W8a): another
            // scheme (ms-msdt:, javascript:) is refused before the question.
            if let Err(refused) = crate::opener::classify(&target) {
                self.speech.earcon(Earcon::Error);
                let msg = self.refused_message(&refused);
                self.tell(&msg);
                return vec![Effect::Redraw];
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

    /// Goes to the heading or block `anchor` names in this document.
    fn go_to_anchor(&mut self, anchor: &str) {
        let found = self
            .session
            .as_ref()
            .and_then(|s| anchor_position(&s.doc, anchor));
        let block = block_id(anchor);
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
                let label = match textweaver_formats::pdf::page_anchor_label(anchor) {
                    Some(page) => {
                        self.msg_args("links-page-label", &args!["page" => page.as_str()])
                    }
                    None if block.is_some() => self.msg("links-block-label"),
                    None => self.msg("links-heading-label"),
                };
                let msg = self.nav_message(Some(&label), pos, &text);
                self.jump(pos, true, ReadAfter::Follow, &msg);
            }
            None => {
                self.speech.earcon(Earcon::Error);
                let msg = match block {
                    Some(id) => self.msg_args("links-no-block", &args!["id" => id]),
                    None => self.msg_args("links-no-heading", &args!["anchor" => anchor]),
                };
                self.tell(&msg);
            }
        }
    }

    /// Opens the local file a link names, at its heading when the link
    /// names one. A link to this same file moves to the heading.
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
        let found = resolve_local(&folder, file).or_else(|| {
            // A note's title or alias, from the vault index.
            let from = here.clone().unwrap_or_else(|| folder.join("_"));
            self.vault_resolve(&from, file)
                .map(|r| (r.path, r.same_name))
        });
        let Some((path, same_name)) = found else {
            self.speech.earcon(Earcon::Error);
            // A note named without its extension, as wikilinks name them;
            // a file named whole keeps the file wording.
            let note = Path::new(file).extension().is_none();
            let msg = if note {
                self.msg_args("links-note-not-found", &args!["name" => file])
            } else {
                self.msg_args("links-file-not-found", &args!["file" => file])
            };
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let note = self.same_name_once(file, same_name);
        if here.as_ref().is_some_and(|h| same_file(h, &path)) {
            if let Some(a) = anchor {
                self.go_to_anchor(a);
            }
            return vec![Effect::Redraw];
        }
        let pos = self.reading_position().unwrap_or(CharPos::ZERO);
        self.authoring.link_pending = Some(LinkArrival {
            to: path.clone(),
            at: ArriveAt::Anchor(anchor.filter(|a| !a.is_empty()).map(str::to_owned)),
            kind: Arrival::Follow,
            leaving: here.map(|h| (h, pos)),
            note,
        });
        self.open_command(path)
    }

    /// Back to the document a link was followed from, at the link. Unless
    /// `past_history`, only when nothing was jumped to since arriving.
    /// `None` when there is no document to go back to from here.
    pub(crate) fn link_history_back(&mut self, past_history: bool) -> Option<Vec<Effect>> {
        let top = self.authoring.link_back.last().cloned()?;
        let s = self.session.as_ref()?;
        let here = s.doc.meta.path.clone()?;
        if !same_file(&here, &top.to)
            || (!past_history && s.history.entries().len() > top.history_len)
        {
            return None;
        }
        let pos = self.reading_position().unwrap_or(CharPos::ZERO);
        self.authoring.link_pending = Some(LinkArrival {
            to: top.from.clone(),
            at: ArriveAt::Pos(top.pos),
            kind: Arrival::Back,
            leaving: Some((here, pos)),
            note: None,
        });
        Some(self.open_command(top.from))
    }

    /// Forward to the document going back left, where it was left. `None`
    /// when there is none to go forward to from here.
    pub(crate) fn link_history_forward(&mut self) -> Option<Vec<Effect>> {
        let top = self.authoring.link_forward.last().cloned()?;
        let here = self.session.as_ref()?.doc.meta.path.clone()?;
        if !same_file(&here, &top.here) {
            return None;
        }
        let pos = self.reading_position().unwrap_or(CharPos::ZERO);
        self.authoring.link_pending = Some(LinkArrival {
            to: top.to.clone(),
            at: ArriveAt::Pos(top.pos),
            kind: Arrival::Forward,
            leaving: Some((here, pos)),
            note: None,
        });
        Some(self.open_command(top.to))
    }

    /// Called as a document opens: when a link, Back, or Forward opened
    /// it, puts the cursor at the heading or the position left, keeps the
    /// way back and forward, and returns what to say in place of "Opened".
    pub(crate) fn link_arrival(&mut self) -> Option<String> {
        let arrival = self.authoring.link_pending.take()?;
        let s = self.session.as_ref()?;
        let path = s.doc.meta.path.clone()?;
        if !same_file(&path, &arrival.to) {
            return None;
        }
        let mut missing = None;
        let cursor = match &arrival.at {
            ArriveAt::Pos(p) => p.clamp_to(s.doc.len_chars()),
            ArriveAt::Anchor(None) => CharPos::ZERO,
            ArriveAt::Anchor(Some(a)) => anchor_position(&s.doc, a).unwrap_or_else(|| {
                missing = Some(a.clone());
                CharPos::ZERO
            }),
        };
        let history_len = s.history.entries().len();
        let title = s.title.clone();
        let line = text_util::line_of(&s.doc, cursor);
        let end = text_util::line_range(&s.doc, line).end;
        let text = text_util::preview(&s.doc, CharRange::new(cursor, end), 10);
        if let Some(s) = self.session.as_mut() {
            s.cursor = cursor;
        }
        self.scroll_to_cursor();
        match (arrival.kind, arrival.leaving) {
            (Arrival::Follow | Arrival::Forward, Some((from, pos))) => {
                if arrival.kind == Arrival::Follow {
                    self.authoring.link_forward.clear();
                } else {
                    self.authoring.link_forward.pop();
                }
                self.authoring.link_back.push(LinkBack {
                    from,
                    pos,
                    to: path.clone(),
                    history_len,
                });
            }
            (Arrival::Back, Some((left, pos))) => {
                self.authoring.link_back.pop();
                self.authoring.link_forward.push(LinkForward {
                    here: path.clone(),
                    to: left,
                    pos,
                });
            }
            (_, None) => {}
        }
        let title = title.as_str();
        let text = text.as_str();
        let said = match arrival.kind {
            Arrival::Back => {
                self.msg_args("links-back-in", &args!["title" => title, "text" => text])
            }
            Arrival::Forward => {
                self.msg_args("links-forward-in", &args!["title" => title, "text" => text])
            }
            Arrival::Follow => {
                let back = self.keys(textweaver_keymap::ActionId::HistoryBack);
                match (&arrival.at, missing) {
                    (_, Some(anchor)) if block_id(&anchor).is_some() => self.msg_args(
                        "links-opened-no-block",
                        &args![
                            "title" => title,
                            "id" => block_id(&anchor).unwrap_or_default(),
                            "key" => back
                        ],
                    ),
                    (_, Some(anchor)) => self.msg_args(
                        "links-opened-no-heading",
                        &args!["title" => title, "anchor" => anchor, "key" => back],
                    ),
                    (ArriveAt::Anchor(Some(_)), None) => self.msg_args(
                        "links-opened-at",
                        &args!["title" => title, "heading" => text, "key" => back],
                    ),
                    _ => self.msg_args("links-opened", &args!["title" => title, "key" => back]),
                }
            }
        };
        Some(match arrival.note {
            Some(note) => format!("{said} {note}"),
            None => said,
        })
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
