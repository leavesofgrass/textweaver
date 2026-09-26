//! Citations while writing and reading, on `textweaver-cite` (ADR-0019).
//!
//! - **Alt+C** (edit mode) inserts a citation: a list of the references in
//!   the folder's `references.json` and your library, filtered as you type;
//!   then a page or other locator ("12", "pp. 3-5", "chapter 2", or Enter
//!   for none); then `[@key, p. 12]` goes in at the caret, or joins the
//!   citation the caret is in (`[@a; @b]`). textweaver says what went in:
//!   "Inserted citation of Doe and Roe, 2020, page 12."
//! - **Alt+Shift+D** adds a reference by DOI or ISBN. The lookup runs on
//!   another thread (doi.org, Open Library; cached in the cache folder);
//!   the reference goes into the folder's library when the document's
//!   folder has one, else into yours.
//! - Palette: `insert_bibliography` (the formatted entries of every work
//!   cited, APA unless the front matter names a `csl` style),
//!   `check_citations` (`tw cite check`), and `import_references` (BibTeX,
//!   RIS, or CSL-JSON into your library).
//! - Reading: moving onto a citation by word, or asking for the link
//!   address there (Alt+Shift+K), says it in words: "Citation: Doe and
//!   Roe, 2020, On X, page 12."

use std::path::{Path, PathBuf};
use std::time::Duration;

use textweaver_cite::insert::{
    add_to_citation, announce_inserted, describe_citation, insertion_text, parse_locator,
    picker_entries,
};
use textweaver_cite::pandoc::{Citation, CiteItem, citation_at, find_citations, write_citation};
use textweaver_cite::{
    CitationStyle, Formatter, Layered, Library, OutputFormat as CiteFormat, Reference,
    folder_library_path, user_library_path,
};
use textweaver_core::{CharPos, CharRange};
use textweaver_editor::Selection;

use crate::app::App;
use crate::authoring_state::{AuthoringList, Job};
use crate::command::{Effect, PromptPurpose};
use crate::text_util;

/// How long a DOI or ISBN lookup may take.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(15);

/// The style named in Markdown front matter (`csl:` or
/// `citation-style:`), else APA.
fn front_matter_style(text: &str) -> String {
    let named = text.strip_prefix("---\n").and_then(|rest| {
        let end = rest.find("\n---")?;
        rest[..end].lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            matches!(k.trim(), "csl" | "citation-style")
                .then(|| v.trim().trim_matches(|c| c == '"' || c == '\'').to_owned())
        })
    });
    named
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| textweaver_convert::citations::DEFAULT_STYLE.to_owned())
}

impl App {
    /// The document's folder, for its `references.json`.
    fn doc_folder(&self) -> Option<PathBuf> {
        let path = self
            .edit
            .as_ref()
            .and_then(|e| e.session.doc().path.clone())
            .or_else(|| self.session.as_ref().and_then(|s| s.doc.meta.path.clone()))?;
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_owned)
    }

    /// The user library's file, when state is kept.
    fn user_library_file(&self) -> Option<PathBuf> {
        self.paths.as_ref().map(|p| user_library_path(&p.data_dir))
    }

    /// The folder library (when the document's folder has one) and the
    /// user library.
    fn libraries(&self) -> (Option<Library>, Library) {
        let folder = self
            .doc_folder()
            .map(|f| folder_library_path(&f))
            .filter(|p| p.is_file())
            .and_then(|p| match Library::load(&p) {
                Ok(l) => Some(l),
                Err(e) => {
                    log::warn!("cannot read {}: {e}", p.display());
                    None
                }
            });
        let user = self
            .user_library_file()
            .and_then(|p| Library::load(&p).ok())
            .unwrap_or_default();
        (folder, user)
    }

    /// Alt+C: the citation picker.
    pub(crate) fn insert_citation(&mut self) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("insert a citation");
        }
        let (folder, user) = self.libraries();
        let entries = picker_entries(folder.as_ref(), &user);
        if entries.is_empty() {
            let add =
                crate::help::chords_text(&self.keymap, textweaver_keymap::ActionId::AddReference);
            self.tell(&format!(
                "Your reference library is empty. Add a reference by DOI or ISBN with {add}, or run import references from the command palette."
            ));
            return vec![Effect::Redraw];
        }
        let n = entries.len();
        self.authoring.filter.clear();
        self.tell(&format!(
            "Insert citation, {n} {}. Type to filter, Enter chooses, Escape cancels.",
            crate::lists::plural(n, "reference", "references")
        ));
        let shown = (0..n).collect();
        self.show_authoring_list(AuthoringList::Citations { entries, shown })
    }

    /// A reference was picked: ask for the locator.
    pub(crate) fn citation_chosen(&mut self, key: String) -> Vec<Effect> {
        self.authoring.citing = Some(key);
        self.prompt(PromptPurpose::CitationLocator)
    }

    /// The locator prompt's answer: inserts the citation.
    pub(crate) fn answer_locator(&mut self, text: &str) -> Vec<Effect> {
        let Some(key) = self.authoring.citing.take() else {
            return vec![Effect::Redraw];
        };
        let locator = if text.trim().is_empty() {
            None
        } else {
            match parse_locator(text) {
                Some(l) => Some(l),
                None => {
                    self.error(&format!(
                        "Could not read the locator {}. Type a page such as 12, pages such as 3-5, or chapter 2; Enter alone for none.",
                        text.trim()
                    ));
                    self.authoring.citing = Some(key);
                    let mut e = self.prompt(PromptPurpose::CitationLocator);
                    e.push(Effect::Redraw);
                    return e;
                }
            }
        };
        let Some(ed) = self.edit.as_ref().and_then(|e| e.session.editor()) else {
            return vec![Effect::Redraw];
        };
        let rope = ed.text();
        let caret = ed.selection().head;
        let line = rope.char_to_line(caret.0.min(rope.len_chars()));
        let line_start = rope.line_to_char(line);
        let line_text = rope.line(line).to_string();
        let existing = citation_at(&line_text, caret.0 - line_start);
        let (range, insert) = match existing {
            Some(c) if !c.narrative => {
                let r = CharRange::new(line_start + c.chars.start.0, line_start + c.chars.end.0);
                let text = match &locator {
                    None => add_to_citation(&c, &[key.as_str()]),
                    Some(l) => {
                        let mut items = c.items.clone();
                        if !items.iter().any(|i| i.key == key) {
                            items.push(CiteItem {
                                key: key.clone(),
                                locator: Some(l.clone()),
                                ..CiteItem::default()
                            });
                        }
                        write_citation(&items, false)
                    }
                };
                (r, text)
            }
            _ => {
                let before = (caret.0 > 0).then(|| rope.char(caret.0 - 1));
                let space = before.is_some_and(|c| !c.is_whitespace() && c != '(' && c != '[');
                let text = insertion_text(&[key.as_str()], locator.clone(), false);
                let text = if space { format!(" {text}") } else { text };
                (CharRange::empty(caret), text)
            }
        };
        let (folder, user) = self.libraries();
        let layers: Vec<&Library> = folder.iter().chain(std::iter::once(&user)).collect();
        let said = announce_inserted(
            &[key.as_str()],
            locator.as_ref(),
            &Layered { layers: &layers },
        );
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let before = ed.text().clone();
        ed.set_selection(Selection::new(range.start, range.end));
        match ed.insert_text(&insert) {
            Ok(o) => {
                self.after_edit(&before, &[o]);
                self.tell(&said);
            }
            Err(e) => self.error(&format!("Could not insert the citation: {e}")),
        }
        vec![Effect::Redraw]
    }

    /// Sets how reference lookups reach the network (tests give a
    /// recorded client).
    pub fn set_citation_client(&mut self, factory: crate::authoring_state::ClientFactory) {
        self.authoring.client = Some(factory);
    }

    /// The DOI or ISBN prompt's answer: looks it up on another thread.
    pub(crate) fn answer_identifier(&mut self, text: &str) -> Vec<Effect> {
        let input = text.trim().to_owned();
        if input.is_empty() {
            self.note("Cancelled.");
            return vec![Effect::Redraw];
        }
        let id = match textweaver_cite::Identifier::parse(&input) {
            Ok(id) => id,
            Err(e) => {
                self.error(&format!("{e}"));
                return vec![Effect::Redraw];
            }
        };
        let factory = self.authoring.client.clone();
        let cache = self.paths.as_ref().map(|p| p.cache_dir.join("cite"));
        let (tx, rx) = std::sync::mpsc::channel();
        let what = id.describe();
        let spawned = std::thread::Builder::new()
            .name("tw-cite-lookup".into())
            .spawn(move || {
                let client: Box<dyn textweaver_cite::HttpClient> = match factory {
                    Some(f) => f(),
                    None => Box::new(textweaver_cite::UreqClient::new(LOOKUP_TIMEOUT)),
                };
                let mut lookup = textweaver_cite::Lookup::new(client.as_ref());
                if let Some(dir) = cache {
                    lookup = lookup.with_cache(textweaver_cite::Cache::new(dir));
                }
                let _ = tx.send(lookup.identifier(&id).map_err(|e| e.to_string()));
            });
        match spawned {
            Ok(_) => {
                self.authoring.jobs.push(Job::Lookup {
                    input: input.clone(),
                    rx,
                });
                self.tell(&format!("Looking up {what}."));
            }
            Err(e) => self.error(&format!("Could not start the lookup: {e}")),
        }
        vec![Effect::Redraw]
    }

    /// A DOI or ISBN lookup finished on its thread: adds the reference.
    pub(crate) fn lookup_finished(&mut self, input: &str, result: Result<Reference, String>) {
        let reference = match result {
            Ok(r) => r,
            Err(e) => {
                self.speech.earcon(textweaver_speech::Earcon::Error);
                self.error(&format!("Could not look up {input}: {e}"));
                return;
            }
        };
        let (folder, user) = self.libraries();
        let mut lib = match folder {
            Some(f) => f,
            None => {
                match self.user_library_file() {
                    Some(p) => {
                        let mut u = user;
                        u.set_path(p);
                        u
                    }
                    None => {
                        self.error("There is no library to add to: textweaver keeps no files in this session.");
                        return;
                    }
                }
            }
        };
        let outcome = lib.add(reference);
        let label = lib
            .get(outcome.key())
            .map(Reference::label)
            .unwrap_or_default();
        if let Some(dir) = lib.path().and_then(Path::parent) {
            let _ = std::fs::create_dir_all(dir);
        }
        match lib.save() {
            Ok(()) => self.tell(&format!("{} {label}", outcome.announcement())),
            Err(e) => self.error(&format!("Could not save the library: {e}")),
        }
    }

    /// The text to check or format: the live text while editing, else the
    /// document's.
    fn citation_text(&self) -> Option<String> {
        match self.edit.as_ref().and_then(|e| e.session.editor()) {
            Some(ed) => Some(ed.text().to_string()),
            None => self.session.as_ref().map(|s| s.doc.text().to_string()),
        }
    }

    /// The palette's `check_citations`.
    pub(crate) fn check_citations(&mut self) {
        let Some(text) = self.citation_text() else {
            return;
        };
        let user = self.user_library_file();
        let folder = self
            .doc_folder()
            .map(|f| folder_library_path(&f))
            .filter(|p| p.is_file());
        let Some(library) = folder.clone().or(user.clone()) else {
            let n = find_citations(&text).len();
            self.tell(&format!(
                "{n} {} found. textweaver keeps no library in this session.",
                crate::lists::plural(n, "citation", "citations")
            ));
            return;
        };
        let offline = textweaver_cite::RecordedClient::new();
        let ctx = textweaver_cite::commands::Context {
            library,
            fallback: user.into_iter().collect(),
            cache_dir: None,
            client: &offline,
        };
        match textweaver_cite::commands::check(&ctx, &text) {
            Ok(msg) => self.tell(&msg),
            Err(e) => self.error(&format!("Could not check the citations: {e}")),
        }
    }

    /// The import prompt's answer: merges the file into the user library.
    pub(crate) fn answer_import_references(&mut self, text: &str) -> Vec<Effect> {
        let file = text.trim().trim_matches('"');
        if file.is_empty() {
            self.note("Cancelled.");
            return vec![Effect::Redraw];
        }
        let Some(library) = self.user_library_file() else {
            self.error(
                "There is no library to import into: textweaver keeps no files in this session.",
            );
            return vec![Effect::Redraw];
        };
        let mut path = PathBuf::from(file);
        if path.is_relative()
            && let Some(folder) = self.doc_folder()
            && folder.join(&path).is_file()
        {
            path = folder.join(path);
        }
        if let Some(dir) = library.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let offline = textweaver_cite::RecordedClient::new();
        let ctx = textweaver_cite::commands::Context {
            library,
            fallback: Vec::new(),
            cache_dir: None,
            client: &offline,
        };
        match textweaver_cite::commands::import(&ctx, &path) {
            Ok(msg) => self.tell(&msg),
            Err(e) => self.error(&format!("Could not import {}: {e}", path.display())),
        }
        vec![Effect::Redraw]
    }

    /// Formats the citations of `text` with `style`; `None` when it has
    /// none or the style cannot be used.
    fn format_citations(
        &mut self,
        text: &str,
        format: CiteFormat,
    ) -> Option<(Vec<Citation>, textweaver_cite::RenderedDocument, String)> {
        let cites = find_citations(text);
        if cites.is_empty() {
            return None;
        }
        let style_name = front_matter_style(text);
        let style = match CitationStyle::resolve(&style_name) {
            Ok(s) => s,
            Err(e) => {
                self.error(&format!("Cannot use the citation style {style_name}: {e}"));
                return None;
            }
        };
        let (folder, user) = self.libraries();
        let layers: Vec<&Library> = folder.iter().chain(std::iter::once(&user)).collect();
        let fmt = Formatter::new(&style, format);
        match fmt.document(&cites, &Layered { layers: &layers }) {
            Ok(doc) => Some((cites, doc, style_name)),
            Err(e) => {
                self.error(&format!("Could not format the citations: {e}"));
                None
            }
        }
    }

    /// The palette's `insert_bibliography`: the entries of every work cited,
    /// formatted, at the caret.
    pub(crate) fn insert_bibliography(&mut self) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("insert a bibliography");
        }
        let Some(text) = self.citation_text() else {
            return vec![Effect::Redraw];
        };
        let Some((_, doc, style)) = self.format_citations(&text, CiteFormat::Markdown) else {
            if find_citations(&text).is_empty() {
                self.tell("The document has no citations yet. Insert one with Alt+C.");
            }
            return vec![Effect::Redraw];
        };
        if doc.bibliography.is_empty() {
            self.tell("None of the cited works is in your library, so there is nothing to list.");
            return vec![Effect::Redraw];
        }
        let entries: Vec<String> = doc.bibliography.iter().map(|e| e.text.clone()).collect();
        let n = entries.len();
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let caret = ed.selection().head;
        let rope = ed.text();
        let at_line_start = caret.0 == 0 || rope.char(caret.0 - 1) == '\n';
        let lead = if at_line_start { "" } else { "\n\n" };
        let block = format!("{lead}{}\n", entries.join("\n\n"));
        let before = ed.text().clone();
        match ed.insert_text(&block) {
            Ok(o) => {
                self.after_edit(&before, &[o]);
                let missing = if doc.missing.is_empty() {
                    String::new()
                } else {
                    format!(" Not in the library: {}.", doc.missing.join(", "))
                };
                self.tell(&format!(
                    "Inserted the bibliography, {n} {}, {style} style.{missing}",
                    crate::lists::plural(n, "entry", "entries")
                ));
            }
            Err(e) => self.error(&format!("Could not insert the bibliography: {e}")),
        }
        vec![Effect::Redraw]
    }

    /// `text` with each citation replaced by its formatted in-text form
    /// ("(Doe & Roe, 2020, p. 12)"), for listening to the rendered text.
    pub(crate) fn with_formatted_citations(&mut self, text: &str) -> String {
        let Some((cites, doc, _)) = self.format_citations(text, CiteFormat::Plain) else {
            return text.to_owned();
        };
        let mut out = text.to_owned();
        for (c, formatted) in cites.iter().zip(doc.citations.iter()).rev() {
            if c.range.end <= out.len() {
                out.replace_range(c.range.clone(), formatted);
            }
        }
        out
    }

    /// The citation at `pos`, in words, when there is one:
    /// "Citation: Doe and Roe, 2020, On X, page 12."
    pub(crate) fn citation_description_at(&self, pos: CharPos) -> Option<String> {
        let s = self.session.as_ref()?;
        let line = text_util::line_range(&s.doc, text_util::line_of(&s.doc, pos));
        let text = s.doc.slice(line);
        if !text.contains('@') {
            return None;
        }
        let c = citation_at(&text, pos.0.saturating_sub(line.start.0))?;
        let (folder, user) = self.libraries();
        let layers: Vec<&Library> = folder.iter().chain(std::iter::once(&user)).collect();
        Some(describe_citation(&c, &Layered { layers: &layers }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_come_from_front_matter() {
        assert_eq!(front_matter_style("---\ncsl: ieee\n---\n\nText"), "ieee");
        assert_eq!(
            front_matter_style("---\ncitation-style: \"mla\"\n---\n"),
            "mla"
        );
        assert_eq!(front_matter_style("No front matter [@a]."), "apa");
    }
}
