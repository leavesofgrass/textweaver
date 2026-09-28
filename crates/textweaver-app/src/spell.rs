//! Spell checking on the built-in SCOWL word list (`textweaver-aids`,
//! sizes 35 to 80, 225,038 words) plus a personal word list.
//!
//! - Alt+M and Alt+Shift+M move to the next and previous misspelled word,
//!   saying it and then spelling it ("recieve. r e c i e v e."). In edit
//!   mode the word is selected, so typing replaces it.
//! - Alt+J lists suggestions (by edit distance, commoner words first), then
//!   "Add … to your word list" and "Leave it as it is". Choosing a
//!   suggestion in edit mode replaces the word (one undo step).
//! - The personal word list is `words.txt` in the data folder, one word per
//!   line.
//! - Saving says how many possible misspellings the document has.
//!
//! What is not checked: code (spans and blocks), math, links' addresses,
//! URLs and e-mail addresses, citation keys (`[@doe2020]`), raw HTML and
//! front matter in Markdown source, words with digits, words in capitals
//! (acronyms such as `NASA`), mixed-case names (`iPhone`), and single
//! letters.

use std::collections::BTreeSet;
use std::path::PathBuf;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use textweaver_aids::ScowlList;
use textweaver_core::{CharPos, CharRange, Direction, MarkerKind};
use textweaver_editor::Selection;
use textweaver_speech::Earcon;
use textweaver_text::Document;

use crate::app::App;
use crate::authoring_state::{AuthoringList, SpellChoice};
use crate::command::Effect;
use crate::structure::{ByteToChar, parser_options};
use crate::text_util;

/// Most suggestions offered.
const MAX_SUGGESTIONS: usize = 7;

/// The char ranges of Markdown `source` whose text is prose: text outside
/// code, math, raw HTML, front matter, and link addresses.
pub(crate) fn prose_ranges_markdown(source: &str) -> Vec<CharRange> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut skip = 0usize;
    for (event, range) in Parser::new_ext(source, parser_options()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::MetadataBlock(_) | Tag::HtmlBlock) => skip += 1,
            Event::End(TagEnd::CodeBlock | TagEnd::MetadataBlock(_) | TagEnd::HtmlBlock) => {
                skip = skip.saturating_sub(1);
            }
            Event::Text(_) if skip == 0 => out.push((range.start, range.end)),
            _ => {}
        }
    }
    let conv = ByteToChar::new(source, out.iter().flat_map(|&(a, b)| [a, b]).collect());
    out.into_iter()
        .map(|(a, b)| CharRange::new(conv.get(a), conv.get(b)))
        .collect()
}

/// The char ranges of a canonical document that are prose: everything but
/// code and math.
fn prose_ranges_canonical(doc: &Document) -> Vec<CharRange> {
    let mut skip: Vec<CharRange> = doc
        .markers()
        .iter()
        .filter(|m| matches!(m.kind, MarkerKind::Code | MarkerKind::Math))
        .map(|m| m.range)
        .collect();
    skip.sort_by_key(|r| r.start);
    let mut out = Vec::new();
    let mut at = CharPos::ZERO;
    for r in skip {
        if r.start > at {
            out.push(CharRange::new(at, r.start));
        }
        at = at.max(r.end);
    }
    if at.0 < doc.len_chars() {
        out.push(CharRange::new(at, doc.end()));
    }
    out
}

/// A word to check: its char range and text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Word {
    pub(crate) range: CharRange,
    pub(crate) text: String,
}

/// True for an apostrophe inside a word.
fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

/// The words worth checking in `text`, which starts at char `base`.
pub(crate) fn words_in(text: &str, base: usize) -> Vec<Word> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        if !chars[i].is_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        while i < n
            && (chars[i].is_alphabetic()
                || (is_apostrophe(chars[i]) && i + 1 < n && chars[i + 1].is_alphabetic()))
        {
            i += 1;
        }
        let end = i;
        // The whitespace-separated chunk around the word: URLs, e-mail
        // addresses, citation keys, and paths are skipped whole.
        let mut a = start;
        while a > 0 && !chars[a - 1].is_whitespace() {
            a -= 1;
        }
        let mut b = end;
        while b < n && !chars[b].is_whitespace() {
            b += 1;
        }
        let chunk: String = chars[a..b].iter().collect();
        let before = start.checked_sub(1).map(|k| chars[k]);
        let after = chars.get(end).copied();
        let joined_to = |c: Option<char>| {
            c.is_some_and(|c| c.is_ascii_digit() || c == '_' || c == '@' || c == '\\')
        };
        let word: String = chars[start..end].iter().collect();
        let letters = word.chars().filter(|c| c.is_alphabetic()).count();
        let upper = word.chars().filter(|c| c.is_uppercase()).count();
        let skip = letters < 2
            || chunk.contains("://")
            || chunk.starts_with("www.")
            || chunk.contains('@')
            || chunk.contains('/')
            || joined_to(before)
            || joined_to(after)
            || upper == letters
            || word.chars().skip(1).any(char::is_uppercase);
        if !skip {
            out.push(Word {
                range: CharRange::new(base + start, base + end),
                text: word,
            });
        }
    }
    out
}

/// Restricted Damerau-Levenshtein distance, giving up above `limit`.
pub(crate) fn distance(a: &[char], b: &[char], limit: usize) -> Option<usize> {
    if a.len().abs_diff(b.len()) > limit {
        return None;
    }
    let w = b.len() + 1;
    let mut prev2 = vec![0usize; w];
    let mut prev: Vec<usize> = (0..w).collect();
    let mut cur = vec![0usize; w];
    for i in 1..=a.len() {
        cur[0] = i;
        let mut best = cur[0];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(prev2[j - 2] + 1);
            }
            cur[j] = v;
            best = best.min(v);
        }
        if best > limit {
            return None;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    let d = prev[b.len()];
    (d <= limit).then_some(d)
}

/// `word`'s capitalization applied to `suggestion`.
fn match_case(word: &str, suggestion: &str) -> String {
    if word.chars().next().is_some_and(char::is_uppercase) {
        let mut c = suggestion.chars();
        c.next()
            .map(|f| f.to_uppercase().chain(c).collect())
            .unwrap_or_default()
    } else {
        suggestion.to_owned()
    }
}

/// Suggestions for `word` from `list`: within two edits, closest first,
/// then commoner (smaller SCOWL size), then alphabetical.
pub(crate) fn suggestions(list: &ScowlList, word: &str) -> Vec<String> {
    let lower: Vec<char> = word.to_lowercase().chars().collect();
    let limit = if lower.len() <= 4 { 1 } else { 2 };
    let mut found: Vec<(usize, u8, &str)> = list
        .words()
        .filter_map(|(w, level)| {
            let wc: Vec<char> = w.chars().collect();
            distance(&lower, &wc, limit).map(|d| (d, level, w))
        })
        .collect();
    found.sort();
    found
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, _, w)| match_case(word, w))
        .collect()
}

/// "r e c i e v e": a word's letters, for spelling aloud.
pub(crate) fn spelled(word: &str) -> String {
    word.chars()
        .map(|c| {
            if is_apostrophe(c) {
                "apostrophe".to_owned()
            } else {
                c.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl App {
    fn words_file(&self) -> Option<PathBuf> {
        self.paths.as_ref().map(|p| p.data_dir.join("words.txt"))
    }

    /// The personal word list, loaded on first use.
    fn personal_words(&mut self) -> &BTreeSet<String> {
        if self.authoring.words.is_none() {
            let words = self
                .words_file()
                .and_then(|f| std::fs::read_to_string(f).ok())
                .map(|t| {
                    t.lines()
                        .map(|l| l.trim().to_lowercase())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect()
                })
                .unwrap_or_default();
            self.authoring.words = Some(words);
        }
        self.authoring.words.get_or_insert_with(BTreeSet::new)
    }

    /// True when `word` is spelled correctly as far as the lists know.
    fn known(&mut self, list: &ScowlList, word: &str) -> bool {
        let personal = self.personal_words();
        known_in(list, personal, word)
    }

    /// True when the document's prose is its Markdown source (edit mode on
    /// a Markdown file).
    fn prose_is_markdown(&self) -> bool {
        self.edit.is_some() && self.authoring.structure.markdown
    }

    /// Every misspelled word of the document, in order.
    pub(crate) fn misspellings(&mut self) -> Vec<Word> {
        let Some(list) = ScowlList::builtin() else {
            return Vec::new();
        };
        let markdown = self.prose_is_markdown();
        let personal = self.personal_words().clone();
        let Some(s) = self.session.as_ref() else {
            return Vec::new();
        };
        misspelled_words(&s.doc, markdown, list, &personal)
    }

    /// Alt+M and Alt+Shift+M: the next or previous misspelled word.
    pub(crate) fn misspelling_step(&mut self, dir: Direction) {
        if ScowlList::builtin().is_none() {
            self.tell("Spell checking is not available: this build has no word list.");
            return;
        }
        let Some(pos) = self.session.as_ref().map(|s| s.cursor) else {
            return;
        };
        let all = self.misspellings();
        let found = match dir {
            Direction::Forward => all.iter().find(|w| w.range.start > pos),
            Direction::Backward => all.iter().rev().find(|w| w.range.start < pos),
        }
        .cloned();
        let Some(w) = found else {
            self.speech.earcon(Earcon::Boundary);
            let msg = if all.is_empty() {
                "No misspellings found.".to_owned()
            } else {
                format!(
                    "No {} misspelling. {} in all.",
                    if dir == Direction::Forward {
                        "more"
                    } else {
                        "earlier"
                    },
                    count_phrase(all.len())
                )
            };
            self.tell(&msg);
            return;
        };
        self.stop_speech();
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(Selection::new(w.range.start, w.range.end));
            self.after_edit(&ropey::Rope::new(), &[]);
        } else if let Some(s) = self.session.as_mut() {
            s.cursor = w.range.start;
            s.selection = Some(w.range);
            s.selection_anchor = Some(w.range.start);
        }
        self.scroll_to_cursor();
        let line = self
            .session
            .as_ref()
            .map_or(0, |s| text_util::line_of(&s.doc, w.range.start) + 1);
        let mut msg = format!("{}. {}.", w.text, spelled(&w.text));
        if self.settings.speech.verbosity >= textweaver_a11y::Verbosity::High {
            msg.push_str(&format!(" Line {line}."));
        }
        self.tell(&msg);
    }

    /// The misspelled word at the cursor (or the selection), if any.
    fn misspelled_here(&mut self) -> Option<Word> {
        let s = self.session.as_ref()?;
        let pos = s.cursor;
        let sel = s.selection;
        let list = ScowlList::builtin()?;
        let line = text_util::line_range(&s.doc, text_util::line_of(&s.doc, pos));
        let words = words_in(&s.doc.slice(line), line.start.0);
        let w = words.into_iter().find(|w| {
            sel.is_some_and(|r| r == w.range) || (w.range.start <= pos && pos <= w.range.end)
        })?;
        (!self.known(list, &w.text)).then_some(w)
    }

    /// Alt+J: suggestions for the misspelled word at the cursor.
    pub(crate) fn spelling_suggestions(&mut self) -> Vec<Effect> {
        let Some(list) = ScowlList::builtin() else {
            self.tell("Spell checking is not available: this build has no word list.");
            return vec![Effect::Redraw];
        };
        let Some(w) = self.misspelled_here() else {
            self.tell("No misspelled word at the cursor.");
            return vec![Effect::Redraw];
        };
        let found = suggestions(list, &w.text);
        let mut choices: Vec<SpellChoice> =
            found.iter().cloned().map(SpellChoice::Replace).collect();
        choices.push(SpellChoice::Add);
        choices.push(SpellChoice::Ignore);
        let intro = if found.is_empty() {
            format!("{}: no suggestions.", w.text)
        } else {
            format!(
                "{}: {} {}.",
                w.text,
                found.len(),
                crate::lists::plural(found.len(), "suggestion", "suggestions")
            )
        };
        let tail = if self.edit.is_some() {
            " Enter replaces the word."
        } else {
            ""
        };
        self.tell(&format!("{intro}{tail}"));
        self.show_authoring_list(AuthoringList::Spelling {
            word: w.text,
            range: w.range,
            choices,
        })
    }

    /// A choice from the spelling list.
    pub(crate) fn spelling_chosen(
        &mut self,
        word: &str,
        range: CharRange,
        choice: SpellChoice,
    ) -> Vec<Effect> {
        match choice {
            SpellChoice::Replace(new) => {
                if self.edit.is_none() {
                    let k = self.keys(textweaver_keymap::ActionId::ToggleEditMode);
                    self.tell(&format!(
                        "{new}. Turn on edit mode with {k} to change the text."
                    ));
                    return vec![Effect::Redraw];
                }
                let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
                    return vec![Effect::Redraw];
                };
                let before = ed.text().clone();
                ed.set_selection(Selection::new(range.start, range.end));
                match ed.insert_text(&new) {
                    Ok(o) => {
                        self.after_edit(&before, &[o]);
                        self.tell(&format!("Replaced with {new}."));
                    }
                    Err(e) => self.error(&format!("Could not replace: {e}")),
                }
            }
            SpellChoice::Add => self.add_word(word),
            SpellChoice::Ignore => self.note("Left as it is."),
        }
        vec![Effect::Redraw]
    }

    /// Adds `word` to the personal word list and saves it.
    fn add_word(&mut self, word: &str) {
        let lower = word.to_lowercase().replace('\u{2019}', "'");
        self.personal_words();
        if let Some(words) = self.authoring.words.as_mut() {
            words.insert(lower);
        }
        let Some(file) = self.words_file() else {
            self.tell(&format!("Added {word} to your word list for this session."));
            return;
        };
        let text: String = self
            .authoring
            .words
            .iter()
            .flatten()
            .map(|w| format!("{w}\n"))
            .collect();
        let written = textweaver_store::atomic_write(&file, text.as_bytes());
        match written {
            Ok(()) => self.tell(&format!("Added {word} to your word list.")),
            Err(e) => self.error(&format!("Could not save your word list: {e}")),
        }
    }

    /// After a save: counts the possible misspellings on a helper thread
    /// (Wave 3: 0.6 s on 10 MB, which the keyboard used to wait for) and
    /// says the count when it is ready ([`spell_count_tick`]). Nothing when
    /// the check is not available.
    ///
    /// [`spell_count_tick`]: Self::spell_count_tick
    pub(crate) fn count_misspellings_in_background(&mut self) {
        let Some(list) = ScowlList::builtin() else {
            return;
        };
        let markdown = self.prose_is_markdown();
        let personal = self.personal_words().clone();
        let Some(doc) = self.session.as_ref().map(|s| s.doc.clone()) else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("textweaver-spell-count".into())
            .spawn(move || {
                let n = misspelled_words(&doc, markdown, list, &personal).len();
                let _ = tx.send(n);
                wake.wake();
            });
        match spawned {
            Ok(_) => self.spell_count = Some(rx),
            Err(e) => log::warn!("cannot count misspellings in the background: {e}"),
        }
    }

    /// From [`App::tick`](crate::App::tick): says the misspelling count
    /// once it is ready ("3 possible misspellings."; "No misspellings."
    /// only at high verbosity, since silence means none).
    pub(crate) fn spell_count_tick(&mut self) -> Vec<Effect> {
        let Some(rx) = &self.spell_count else {
            return Vec::new();
        };
        let n = match rx.try_recv() {
            Ok(n) => n,
            Err(std::sync::mpsc::TryRecvError::Empty) => return Vec::new(),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.spell_count = None;
                return Vec::new();
            }
        };
        self.spell_count = None;
        if n == 0 && self.settings.speech.verbosity < textweaver_a11y::Verbosity::High {
            return Vec::new();
        }
        let summary = match n {
            0 => "No misspellings.".to_owned(),
            n => format!("{}.", count_phrase(n)),
        };
        self.announce_queued(&summary, textweaver_a11y::Priority::Polite);
        vec![Effect::Redraw]
    }

    /// Waits until the misspelling count after a save is said (or
    /// `timeout` passes); for tests. True when none is left.
    pub fn wait_for_spell_count(&mut self, timeout: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while self.spell_count.is_some() {
            if std::time::Instant::now() >= deadline {
                return false;
            }
            let _ = self.spell_count_tick();
            if self.spell_count.is_some() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        true
    }
}

/// True when `word` is in the word list or the personal list (a
/// possessive of a personal word counts).
fn known_in(list: &ScowlList, personal: &BTreeSet<String>, word: &str) -> bool {
    if list.level(word).is_some() {
        return true;
    }
    let lower = word.to_lowercase().replace('\u{2019}', "'");
    personal.contains(&lower)
        || lower
            .strip_suffix("'s")
            .is_some_and(|w| personal.contains(w))
}

/// Every misspelled word of `doc`'s prose, in order: its Markdown source's
/// prose when `markdown`, else everything but code and math.
fn misspelled_words(
    doc: &Document,
    markdown: bool,
    list: &ScowlList,
    personal: &BTreeSet<String>,
) -> Vec<Word> {
    let ranges = if markdown {
        prose_ranges_markdown(&doc.text().to_string())
    } else {
        prose_ranges_canonical(doc)
    };
    ranges
        .iter()
        .flat_map(|r| words_in(&doc.slice(*r), r.start.0))
        .filter(|w| !known_in(list, personal, &w.text))
        .collect()
}

/// "1 possible misspelling", "3 possible misspellings".
fn count_phrase(n: usize) -> String {
    if n == 1 {
        "1 possible misspelling".to_owned()
    } else {
        format!(
            "{} possible misspellings",
            textweaver_editor::echo::thousands(n)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(words: &[Word]) -> Vec<&str> {
        words.iter().map(|w| w.text.as_str()).collect()
    }

    #[test]
    fn words_skip_urls_codes_and_names() {
        let w = words_in(
            "Teh cat's NASA iPhone x2 see https://example.org/page and a@b.org [@doe2020, p. 12] don't",
            0,
        );
        assert_eq!(texts(&w), ["Teh", "cat's", "see", "and", "don't"]);
        assert_eq!(w[0].range, CharRange::new(0, 3));
    }

    #[test]
    fn markdown_prose_leaves_out_code_and_addresses() {
        let src = "---\ntitle: Tset\n---\n\nSome txet [link](notes/chaptr.md) and `cdoe`.\n\n```\nfnord\n```\n";
        let ranges = prose_ranges_markdown(src);
        let prose: Vec<String> = ranges
            .iter()
            .map(|r| src.chars().skip(r.start.0).take(r.len()).collect())
            .collect();
        let joined = prose.join("|");
        assert!(joined.contains("Some txet"), "{joined}");
        assert!(joined.contains("link"), "{joined}");
        for hidden in ["Tset", "chaptr", "cdoe", "fnord"] {
            assert!(!joined.contains(hidden), "{hidden} in {joined}");
        }
    }

    #[test]
    fn distances_and_suggestions() {
        let c = |s: &str| s.chars().collect::<Vec<_>>();
        assert_eq!(distance(&c("recieve"), &c("receive"), 2), Some(1));
        assert_eq!(distance(&c("kitten"), &c("sitting"), 2), None);
        assert_eq!(distance(&c("abc"), &c("abc"), 0), Some(0));
        assert_eq!(spelled("don't"), "d o n apostrophe t");
        assert_eq!(match_case("Teh", "the"), "The");
        if let Some(list) = ScowlList::builtin() {
            let s = suggestions(list, "recieve");
            assert_eq!(s.first().map(String::as_str), Some("receive"), "{s:?}");
            let s = suggestions(list, "Teh");
            assert!(s.iter().any(|w| w == "The"), "{s:?}");
        }
    }
}
