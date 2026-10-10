//! Offline documentation and searchable help (beta 1, B1-hp).
//!
//! - **Help, Documentation** opens the documentation index packaged with
//!   textweaver as a document: `docs/README.md` beside the program or one
//!   folder up (every package stages the complete user guides there), and
//!   in a development build the repository's own `docs` folder. Its links
//!   to the guides are followed in place with Follow link, and History
//!   back (Backspace or Alt+Left) returns ([`crate::links`]). A guide the
//!   index lists under "For users" that the package lacks is named in a
//!   warning, and the rest still opens: a missing guide warns, never
//!   fails. **Online documentation** shows the web address and asks
//!   before a browser opens ([`crate::about`]).
//! - **Search help** is one search over the command names, their keys and
//!   descriptions, the settings and their help, and the guides' headings
//!   and text. Typing in F1's help starts it with what was typed. It
//!   filters as you type, like the keyboard shortcuts list and Settings
//!   ([`App::list_filter`]), and says how many match ("8 matches.").
//!   Enter on a command says what it does and keeps the list; on a
//!   setting it opens Settings at that setting; on a guide section it
//!   opens the guide at that heading. F1 on a row says the topic's help
//!   or the section's first words.
//! - The guides are split into sections on first use and kept in the
//!   cache folder (`help-index.json`), built again when a guide changes.
//!   Commands and settings are searched in place, so they follow the
//!   interface language and the user's own keys.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;

use crate::app::{App, ListKind};
use crate::command::Effect;
use crate::list_model::ListKey;

/// The documentation index's name in the docs folder.
const INDEX_FILE: &str = "README.md";
/// The heading of the user section in the index: the guides it links
/// are the ones every package ships (`cargo xtask dist`).
const USER_SECTION: &str = "## For users";
/// The guide index's name in the cache folder.
const CACHE_FILE: &str = "help-index.json";
/// Words a question carries that say nothing about the topic ("how do I
/// export audio" searches for "export audio").
///
/// shortcut: English words only; a question in another interface
/// language keeps every word, so it may find less. Upgrade to a list per
/// catalog if users ask in other languages.
const QUESTION_WORDS: &[&str] = &[
    "a", "an", "and", "are", "can", "do", "does", "for", "how", "i", "in", "is", "it", "me", "my",
    "of", "on", "or", "the", "to", "what", "when", "where", "which", "why", "with",
];
/// At most this many characters of a section are said by F1 on its row.
const PREVIEW_CHARS: usize = 200;

/// One section of a guide: the text under one heading, up to the next.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GuideSection {
    /// The guide, relative to the docs folder with forward slashes
    /// (`reading.md`, `../scripts/README.md`).
    pub file: String,
    /// The guide's title: its first heading, else its file name.
    pub guide: String,
    /// The section's heading.
    pub heading: String,
    /// The section's text, without Markdown.
    pub text: String,
}

/// The guide index as kept in the cache folder.
#[derive(Serialize, Deserialize)]
struct CachedIndex {
    /// The docs folder, the version, and each guide's size and time; a
    /// change to any of them builds the index again.
    fingerprint: String,
    sections: Vec<GuideSection>,
}

/// The guides, split into sections, ready to search.
#[derive(Clone, Debug, Default)]
pub(crate) struct HelpIndex {
    /// The docs folder found, if any.
    pub docs: Option<PathBuf>,
    /// Every guide's sections, in the index's order.
    pub sections: Vec<GuideSection>,
    /// Each section's heading and text in lowercase, for matching.
    hay: Vec<String>,
    /// The guides the index lists that are not in the docs folder.
    pub missing: Vec<String>,
}

/// One thing Search help found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HelpTopic {
    /// A command: its name, keys, and description matched.
    Command(ActionId),
    /// A setting, by its path (`speech.rate`).
    Setting(String),
    /// A guide section, by its place in [`HelpIndex::sections`].
    Guide(usize),
}

/// The packaged docs folder: `docs` beside the program, or one folder up
/// (a program installed in a `bin` folder), holding the index.
pub(crate) fn packaged_docs(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    [Some(dir), dir.parent()]
        .into_iter()
        .flatten()
        .map(|d| d.join("docs"))
        .find(|d| d.join(INDEX_FILE).is_file())
}

/// The docs folder this textweaver reads: the packaged one, else the
/// repository's `docs` folder it was built from (a development build;
/// absent on other computers).
fn find_docs() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| packaged_docs(&exe))
        .or_else(|| {
            let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)?
                .join("docs");
            repo.join(INDEX_FILE).is_file().then_some(repo)
        })
}

/// The guides linked under "For users" in the index, relative to the docs
/// folder (`reading.md`, `../scripts/README.md`). Only local Markdown
/// links count; anchors are dropped. The same rule as the packages'
/// `stage_user_docs` in xtask.
pub(crate) fn user_guides(index: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut in_section = false;
    for line in index.lines() {
        if line.starts_with("## ") {
            in_section = line.trim_end() == USER_SECTION;
            continue;
        }
        if !in_section {
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(')') else {
                break;
            };
            let target = after[..end].split('#').next().unwrap_or("");
            let local = target.ends_with(".md") && !target.contains("://");
            if local && !out.iter().any(|t| t == target) {
                out.push(target.to_owned());
            }
            rest = &after[end..];
        }
    }
    out
}

/// Splits a guide into its sections, one per heading. Text before the
/// first heading belongs to no section and is left out.
pub(crate) fn sections_of(file: &str, markdown: &str) -> Vec<GuideSection> {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let mut out: Vec<GuideSection> = Vec::new();
    let mut in_heading = false;
    let mut heading = String::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
                heading.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                out.push(GuideSection {
                    file: file.to_owned(),
                    guide: String::new(),
                    heading: heading.trim().to_owned(),
                    text: String::new(),
                });
            }
            Event::Text(t) | Event::Code(t) => {
                if in_heading {
                    heading.push_str(&t);
                } else if let Some(s) = out.last_mut() {
                    s.text.push_str(&t);
                }
            }
            Event::SoftBreak
            | Event::HardBreak
            | Event::End(
                TagEnd::Paragraph
                | TagEnd::Item
                | TagEnd::CodeBlock
                | TagEnd::TableCell
                | TagEnd::BlockQuote(_),
            ) if !in_heading => {
                if let Some(s) = out.last_mut() {
                    s.text.push(' ');
                }
            }
            _ => {}
        }
    }
    let guide = out.first().map_or_else(
        || file.rsplit('/').next().unwrap_or(file).to_owned(),
        |s| s.heading.clone(),
    );
    for s in &mut out {
        s.guide.clone_from(&guide);
        s.text = s.text.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    out
}

/// The words of `query` that say what it is about: question words are
/// dropped, unless nothing else is left.
fn topic_words(query: &str) -> String {
    let kept: Vec<&str> = query
        .split_whitespace()
        .filter(|w| !QUESTION_WORDS.contains(&w.to_lowercase().as_str()))
        .collect();
    if kept.is_empty() {
        query.trim().to_owned()
    } else {
        kept.join(" ")
    }
}

/// The docs folder's guides, their sizes and times, and the version, as
/// one string: the cache is used only while it is the same.
fn fingerprint(docs: &Path, guides: &[String]) -> String {
    let mut out = format!("{}\n{}", env!("CARGO_PKG_VERSION"), docs.display());
    for g in std::iter::once(INDEX_FILE).chain(guides.iter().map(String::as_str)) {
        let meta = std::fs::metadata(docs.join(g)).ok();
        let len = meta.as_ref().map_or(0, std::fs::Metadata::len);
        let time = meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        out.push_str(&format!("\n{g} {len} {time}"));
    }
    out
}

/// Builds the guide index for `docs` (empty without a folder or without
/// its index file), reading
/// the cache in `cache` when it is current and writing it when it was
/// not. A cache that cannot be read is rebuilt; one that cannot be
/// written is logged and skipped.
pub(crate) fn build_index(docs: Option<PathBuf>, cache: Option<&Path>) -> HelpIndex {
    let Some(dir) = docs.filter(|d| d.join(INDEX_FILE).is_file()) else {
        return HelpIndex::default();
    };
    let guides = std::fs::read_to_string(dir.join(INDEX_FILE))
        .map(|index| user_guides(&index))
        .unwrap_or_default();
    let missing: Vec<String> = guides
        .iter()
        .filter(|g| !dir.join(g).is_file())
        .cloned()
        .collect();
    let print = fingerprint(&dir, &guides);
    let cache_file = cache.map(|c| c.join(CACHE_FILE));
    let cached = cache_file
        .as_ref()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str::<CachedIndex>(&s).ok())
        .filter(|c| c.fingerprint == print);
    let sections = match cached {
        Some(c) => c.sections,
        None => {
            let sections: Vec<GuideSection> = guides
                .iter()
                .filter_map(|g| {
                    let text = std::fs::read_to_string(dir.join(g)).ok()?;
                    Some(sections_of(g, &text))
                })
                .flatten()
                .collect();
            if let Some(f) = &cache_file {
                keep_in_cache(f, print, &sections);
            }
            sections
        }
    };
    let hay = sections
        .iter()
        .map(|s| format!("{}\n{}", s.heading, s.text).to_lowercase())
        .collect();
    HelpIndex {
        docs: Some(dir),
        sections,
        hay,
        missing,
    }
}

/// Writes the guide index to the cache file `f`; a failure is logged.
fn keep_in_cache(f: &Path, fingerprint: String, sections: &[GuideSection]) {
    let kept = CachedIndex {
        fingerprint,
        sections: sections.to_vec(),
    };
    let written = serde_json::to_string(&kept)
        .map_err(std::io::Error::other)
        .and_then(|json| {
            if let Some(dir) = f.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(f, json)
        });
    if let Err(e) = written {
        log::warn!("help index not kept in the cache: {e}");
    }
}

impl App {
    /// The docs folder: the one a test set, else the packaged or the
    /// development one.
    fn docs_folder(&self) -> Option<PathBuf> {
        self.docs_override.clone().or_else(find_docs)
    }

    /// The guide index, built on first use.
    fn help_index(&mut self) -> &HelpIndex {
        if self.help_index.is_none() {
            let cache = self.paths.as_ref().map(|p| p.cache_dir.clone());
            self.help_index = Some(build_index(self.docs_folder(), cache.as_deref()));
        }
        self.help_index.get_or_insert_with(HelpIndex::default)
    }

    /// Help, Documentation: the documentation index opens as a document,
    /// and any guide it lists that is missing is named. Without a docs
    /// folder, the online documentation is offered instead.
    pub(crate) fn documentation(&mut self) -> Vec<Effect> {
        let (docs, missing) = {
            let index = self.help_index();
            (index.docs.clone(), index.missing.clone())
        };
        let Some(docs) = docs else {
            let q = self.msg_args(
                "docs-not-found",
                &args!["address" => crate::about::DOCS_ADDRESS],
            );
            self.offer_open(crate::about::DOCS_ADDRESS.to_owned(), &q);
            return vec![Effect::Redraw];
        };
        let effects = self.open_command(docs.join(INDEX_FILE));
        if !missing.is_empty() {
            let msg = self.msg_args(
                "docs-guide-missing",
                &args!["n" => missing.len(), "guides" => missing.join(", ")],
            );
            self.warn(&msg);
        }
        effects
    }

    /// Help, Online documentation: the address, and a question before a
    /// browser opens.
    pub(crate) fn online_documentation(&mut self) -> Vec<Effect> {
        let q = self.msg_args(
            "about-docs-question",
            &args!["address" => crate::about::DOCS_ADDRESS],
        );
        self.offer_open(crate::about::DOCS_ADDRESS.to_owned(), &q);
        vec![Effect::Redraw]
    }

    /// The topics matching every topic word of `query`: commands, then
    /// settings, then guide sections (those whose heading matches first).
    /// An empty query matches every topic.
    pub(crate) fn help_topics(&mut self, query: &str) -> Vec<HelpTopic> {
        let words = topic_words(query);
        let c = self.catalog();
        let en = textweaver_lexicon::i18n::Catalog::english();
        let mut out: Vec<HelpTopic> = Vec::new();
        for a in crate::help::help_order() {
            let mut hay = vec![
                crate::menu::action_name(&c, a),
                crate::menu::action_name(&en, a),
                crate::help::action_help(&c, a),
                a.help().to_owned(),
                crate::help::category_title(&c, a.category()),
            ];
            hay.extend(self.keymap.chords_for(a).iter().map(ToString::to_string));
            if crate::lists::matches(&hay.join("\n"), &words) {
                out.push(HelpTopic::Command(a));
            }
        }
        let window = self.keymap.frontend() == textweaver_keymap::Frontend::Gui;
        for s in &self.settings_schema().settings {
            let here = if window {
                s.frontend.in_window()
            } else {
                s.frontend.in_terminal()
            };
            if !s.internal && here && s.matches(&c, &words) {
                out.push(HelpTopic::Setting(s.path.clone()));
            }
        }
        let index = self.help_index();
        let lower = words.to_lowercase();
        let (mut titled, mut within): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
        for (i, hay) in index.hay.iter().enumerate() {
            if !crate::lists::matches(hay, &lower) {
                continue;
            }
            if crate::lists::matches(&index.sections[i].heading, &lower) {
                titled.push(i);
            } else {
                within.push(i);
            }
        }
        out.extend(titled.into_iter().chain(within).map(HelpTopic::Guide));
        out
    }

    /// A topic's row: what it is first, then what kind and where ("Export
    /// audio, command", "Rate, setting in Speech", "Subtitles, in Audio
    /// export").
    fn help_topic_row(&self, t: &HelpTopic) -> String {
        let c = self.cat();
        match t {
            HelpTopic::Command(a) => {
                let key = crate::help::main_chord(&self.keymap, *a)
                    .map(|ch| crate::help::mark_chord(c, &ch));
                let row = crate::help::palette_line(c, *a, key, false);
                c.fmt("helpsearch-command", &args!["row" => row])
            }
            HelpTopic::Setting(path) => match crate::settings_schema::base_schema().get(path) {
                Some(s) => c.fmt(
                    "helpsearch-setting",
                    &args!["label" => s.label_in(c), "section" => s.section_in(c)],
                ),
                None => path.clone(),
            },
            HelpTopic::Guide(i) => {
                let Some(s) = self.help_index.as_ref().and_then(|x| x.sections.get(*i)) else {
                    return String::new();
                };
                c.fmt(
                    "helpsearch-guide",
                    &args!["heading" => s.heading.as_str(), "guide" => s.guide.as_str()],
                )
            }
        }
    }

    /// The Search help command: every topic, filtering as you type.
    pub(crate) fn search_help(&mut self) -> Vec<Effect> {
        self.help_filter.clear();
        let effects = self.show_help_search();
        let n = self.help_search_count();
        let msg = self.msg_args("helpsearch-intro", &args!["n" => n]);
        self.tell(&msg);
        effects
    }

    /// The number of topics in the list shown.
    fn help_search_count(&self) -> usize {
        match &self.list {
            Some(ListKind::HelpSearch(t)) => t.len(),
            _ => 0,
        }
    }

    /// The search list as the filter leaves it.
    fn show_help_search(&mut self) -> Vec<Effect> {
        let filter = self.help_filter.clone();
        let topics = self.help_topics(&filter);
        let items = topics.iter().map(|t| self.help_topic_row(t)).collect();
        let title = if filter.trim().is_empty() {
            self.msg("helpsearch-title")
        } else {
            self.msg_args(
                "helpsearch-title-matching",
                &args!["filter" => filter.trim()],
            )
        };
        self.list = Some(ListKind::HelpSearch(topics));
        vec![Effect::ShowList { title, items }]
    }

    /// The search's filter, while its list or F1's help is shown: typing
    /// in F1's help starts a search.
    pub(crate) fn help_filter(&self) -> Option<&str> {
        match self.list {
            Some(ListKind::HelpSearch(_)) => Some(&self.help_filter),
            Some(ListKind::Help) => Some(""),
            _ => None,
        }
    }

    /// The search's filter changed to `query`: the topics that match, and
    /// how many ("8 matches.").
    pub(crate) fn filter_help(&mut self, query: String) -> Vec<Effect> {
        self.help_filter = query;
        let effects = self.show_help_search();
        let n = self.help_search_count();
        let query = self.help_filter.trim().to_owned();
        let msg = if query.is_empty() {
            self.msg_args("helpsearch-cleared", &args!["n" => n])
        } else if n == 0 {
            self.msg_args("helpsearch-none", &args!["query" => query])
        } else {
            self.msg_args("helpsearch-match", &args!["n" => n])
        };
        self.tell(&msg);
        effects
    }

    /// Enter on topic `n`: a command's help (the list stays), a setting in
    /// Settings, or a guide at its heading.
    pub(crate) fn choose_help_topic(&mut self, topics: &[HelpTopic], n: usize) -> Vec<Effect> {
        match topics.get(n).cloned() {
            Some(HelpTopic::Command(a)) => {
                let help = crate::help::marked_entry(self.cat(), &self.keymap, a);
                self.pending_list_focus = Some(n);
                let effects = self.show_help_search();
                self.tell(&help);
                effects
            }
            Some(HelpTopic::Setting(path)) => {
                self.open_settings_screen();
                self.filter_settings(path)
            }
            Some(HelpTopic::Guide(i)) => {
                let found = self.help_index.as_ref().and_then(|x| {
                    let s = x.sections.get(i)?;
                    Some((x.docs.as_ref()?.join(&s.file), s.heading.clone()))
                });
                match found {
                    Some((path, heading)) => self.open_guide_at(path, &heading),
                    None => vec![Effect::Redraw],
                }
            }
            None => vec![Effect::Redraw],
        }
    }

    /// Opens a guide and puts the cursor on `heading`.
    fn open_guide_at(&mut self, path: PathBuf, heading: &str) -> Vec<Effect> {
        let effects = self.open_command(path.clone());
        let found = self
            .session
            .as_ref()
            .filter(|s| s.doc.meta.path.as_ref() == Some(&path))
            .and_then(|s| crate::links::heading_for_anchor(&s.doc, heading));
        if let Some(p) = found {
            if let Some(s) = self.session.as_mut() {
                s.cursor = p;
            }
            self.scroll_to_cursor();
            let msg = self.msg_args("helpsearch-at-heading", &args!["heading" => heading]);
            self.tell(&msg);
        }
        effects
    }

    /// Keys the search list handles itself: F1 says the focused topic's
    /// help, or a guide section's first words. `None` leaves the key to
    /// the list.
    pub(crate) fn help_search_key(&mut self, key: ListKey) -> Option<Vec<Effect>> {
        let Some(ListKind::HelpSearch(topics)) = &self.list else {
            return None;
        };
        if key != ListKey::Introduce {
            return None;
        }
        let n = self.list_model.as_ref().map_or(0, |l| l.selected);
        let c = self.cat();
        let text = match topics.get(n)? {
            HelpTopic::Command(a) => crate::help::marked_entry(c, &self.keymap, *a),
            HelpTopic::Setting(path) => {
                let s = crate::settings_schema::base_schema().get(path)?;
                format!("{}: {}", s.label_in(c), s.help_in(c))
            }
            HelpTopic::Guide(i) => {
                let s = self.help_index.as_ref()?.sections.get(*i)?;
                let words = crate::lists::one_line(&s.text, PREVIEW_CHARS);
                format!("{}: {words}", s.heading)
            }
        };
        self.tell(&text);
        Some(vec![Effect::Redraw])
    }
}

#[cfg(test)]
mod tests;
