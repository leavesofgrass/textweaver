//! Authoring and reading quick wins (roadmap Phase 1, Agent P1b): word
//! count, the link address at the cursor, table cells in edit mode, the
//! typing echo switch, and copying to the clipboard; and from Phase 2
//! (Agent P2b) select all, deleting a word, paste, and cycling verbosity
//! and punctuation while running.
//!
//! # Clipboard
//!
//! Copy and Cut put the text in [`App::take_clipboard`]; the terminal
//! frontend sends it to the terminal as an OSC 52 sequence
//! ([`osc52`]), which Windows Terminal, iTerm2, kitty, WezTerm, foot,
//! Alacritty, and xterm (when allowed) pass to the system clipboard, over
//! SSH too, as Star did. Where the terminal cannot take OSC 52 (the old
//! Windows console, macOS Terminal.app, VTE terminals), the terminal
//! frontend puts the text on the system clipboard itself with `arboard`
//! (its `clipboard` feature, Agent W4g), and says so once.

use textweaver_a11y::Verbosity;
use textweaver_core::{CharPos, CharRange, Direction, MarkerKind, PunctuationLevel, Unit};
use textweaver_editor::Selection;
use textweaver_editor::echo::thousands;
use textweaver_lexicon::args;
use textweaver_speech::Earcon;
use textweaver_text::units::unit_at;

use crate::app::App;
use crate::command::Effect;
use crate::mdline;
use crate::text_util;

/// Words in `text`: runs of non-space characters with a letter or digit
/// (Markdown markup such as `##` or `-` is not a word).
pub(crate) fn count_words(chars: impl Iterator<Item = char>) -> usize {
    let mut n = 0;
    let mut in_token = false;
    let mut has_alnum = false;
    for c in chars {
        if c.is_whitespace() {
            if in_token && has_alnum {
                n += 1;
            }
            in_token = false;
            has_alnum = false;
        } else {
            in_token = true;
            has_alnum |= c.is_alphanumeric();
        }
    }
    if in_token && has_alnum {
        n += 1;
    }
    n
}

/// The OSC 52 sequence that asks the terminal to put `text` on the system
/// clipboard (`ESC ] 52 ; c ; base64 BEL`).
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(T[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// What a line of a code block is called on caret and Speech Cursor moves
/// (Agent W4g): "code, Python" on the block's first line (`line`) when the
/// block names its language, "code" otherwise.
pub(crate) fn code_structure(
    c: &textweaver_lexicon::i18n::Catalog,
    block: &textweaver_text::Marker,
    line: CharRange,
) -> String {
    match block
        .label
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        Some(lang) if line.contains(block.range.start) || line.start == block.range.start => c.fmt(
            "nav-line-code-language",
            &textweaver_lexicon::args!["language" => language_name(c, lang)],
        ),
        _ => crate::words::kind_name(c, textweaver_core::MarkerKind::Code),
    }
}

/// A code block language as it is said: the usual name for common fence
/// words (`py` is "Python", `sh` is "shell"), otherwise the word itself.
/// Proper names stay as they are; names made of ordinary words come from
/// the catalog.
pub(crate) fn language_name(c: &textweaver_lexicon::i18n::Catalog, fence: &str) -> String {
    let name = match fence.to_ascii_lowercase().as_str() {
        "py" | "python" | "python3" => "Python",
        "rs" | "rust" => "Rust",
        "js" | "javascript" | "mjs" => "JavaScript",
        "ts" | "typescript" => "TypeScript",
        "jsx" => return c.tr("authoring-language-jsx"),
        "tsx" => return c.tr("authoring-language-tsx"),
        "sh" | "bash" | "shell" | "zsh" | "console" => return c.tr("authoring-language-shell"),
        "ps1" | "powershell" | "pwsh" => "PowerShell",
        "bat" | "cmd" | "batch" => return c.tr("authoring-language-batch"),
        "c" => "C",
        "h" => return c.tr("authoring-language-c-header"),
        "cpp" | "c++" | "cc" | "cxx" | "hpp" => return c.tr("authoring-language-cpp"),
        "cs" | "csharp" | "c#" => return c.tr("authoring-language-csharp"),
        "java" => "Java",
        "kt" | "kotlin" => "Kotlin",
        "go" | "golang" => "Go",
        "rb" | "ruby" => "Ruby",
        "php" => "PHP",
        "swift" => "Swift",
        "r" => "R",
        "sql" => "SQL",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "xml" => "XML",
        "md" | "markdown" => "Markdown",
        "tex" | "latex" => "LaTeX",
        "diff" | "patch" => return c.tr("authoring-language-diff"),
        "text" | "txt" | "plain" | "plaintext" => return c.tr("authoring-language-plain-text"),
        "hs" | "haskell" => "Haskell",
        "lua" => "Lua",
        "pl" | "perl" => "Perl",
        "scala" => "Scala",
        "m" | "matlab" => "MATLAB",
        "jl" | "julia" => "Julia",
        "dockerfile" | "docker" => "Dockerfile",
        "makefile" | "make" => "Makefile",
        "ini" => "INI",
        _ => return fence.to_owned(),
    };
    name.to_owned()
}

/// A grammar fix as the fixes list says it: the new words, or "Remove the
/// words" for a fix that removes them.
pub(crate) fn grammar_fix_label(c: &textweaver_lexicon::i18n::Catalog, fix: &str) -> String {
    if fix.trim().is_empty() {
        c.tr("grammar-remove-the-words")
    } else {
        fix.to_owned()
    }
}

impl App {
    /// Ctrl+F7 and Ctrl+Shift+F7: grammar problems (the `grammar`
    /// feature, [`crate::grammar`]).
    pub(crate) fn grammar_action(&mut self, dir: Direction) {
        #[cfg(feature = "grammar")]
        self.grammar_step(dir);
        #[cfg(not(feature = "grammar"))]
        {
            let _ = dir;
            let msg = self.msg("authoring-grammar-not-in-build");
            self.tell(&msg);
        }
    }

    /// A choice from the grammar fixes list.
    pub(crate) fn grammar_fix_action(
        &mut self,
        range: CharRange,
        fixes: &[String],
        n: usize,
    ) -> Vec<Effect> {
        #[cfg(feature = "grammar")]
        {
            self.grammar_fix_chosen(range, fixes, n)
        }
        #[cfg(not(feature = "grammar"))]
        {
            let _ = (range, fixes, n);
            vec![Effect::Redraw]
        }
    }
}

/// The typing echo settings in cycle order: characters and words,
/// characters, words, none; with the key `authoring-typing-echo` chooses
/// its words by.
const ECHO_CYCLE: [(bool, bool, &str); 4] = [
    (true, true, "characters-and-words"),
    (true, false, "characters"),
    (false, true, "words"),
    (false, false, "none"),
];

impl App {
    /// Says how many words the selection has, else the document.
    pub(crate) fn word_count(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let (range, what) = match s.selection.filter(|r| !r.is_empty()) {
            Some(r) => (r, "authoring-word-count-selection"),
            None => (
                CharRange::new(0, s.doc.len_chars()),
                "authoring-word-count-document",
            ),
        };
        let n = count_words(s.doc.text().slice(range.to_range()).chars());
        let msg = self.msg_args(what, &args!["n" => n, "count" => thousands(n)]);
        self.tell(&msg);
    }

    /// The number of the word at `pos` (from 1) and the words in the
    /// document, for "say position".
    pub(crate) fn word_position(&self, pos: CharPos) -> Option<(usize, usize)> {
        let s = self.session.as_ref()?;
        let rope = s.doc.text();
        let pos = pos.0.min(rope.len_chars());
        let before = count_words(rope.slice(..pos).chars());
        // Inside a word: it is counted from its end.
        let inside =
            text_util::word_containing(&s.doc, CharPos(pos)).is_some_and(|w| w.start.0 < pos);
        let total = count_words(rope.chars());
        let here = if inside { before } else { before + 1 };
        Some((here.min(total).max(1), total))
    }

    /// Says the address of the link at the cursor: the link marker's
    /// target in reading, the Markdown link on the line in edit mode.
    pub(crate) fn link_address(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let pos = s.cursor;
        let found = if self.edit.is_some() {
            let line = text_util::line_of(&s.doc, pos);
            let r = text_util::line_range(&s.doc, line);
            let text = s.doc.slice(r);
            let col = text
                .char_indices()
                .nth(pos.0.saturating_sub(r.start.0))
                .map_or(text.len(), |(b, _)| b);
            mdline::link_at(&text, col)
        } else {
            let index = s.doc.marker_index();
            index
                .enclosing(MarkerKind::Link, pos)
                .or_else(|| {
                    // The cursor on the space just before a link.
                    unit_at(&s.doc, pos, Unit::Word)
                        .and_then(|w| index.enclosing(MarkerKind::Link, w.start))
                })
                .and_then(|m| m.reference.clone().map(|url| (s.doc.slice(m.range), url)))
        };
        let cite = if found.is_none() {
            self.citation_description_at(pos)
        } else {
            None
        };
        match found {
            Some((text, url)) if url.is_empty() => {
                let msg = self.msg_args("authoring-link-no-address", &args!["text" => text]);
                self.tell(&msg);
            }
            Some((text, url)) if text.trim() == url => {
                let msg = self.msg_args("authoring-link-address", &args!["url" => url]);
                self.tell(&msg);
            }
            Some((text, url)) => {
                let msg = self.msg_args(
                    "authoring-link-named-address",
                    &args!["text" => text.trim(), "url" => url],
                );
                self.tell(&msg);
            }
            // A citation is said in words instead ("Citation: Doe, 2020").
            None if cite.is_some() => {
                if let Some(d) = cite {
                    self.tell(&d);
                }
            }
            None => {
                self.speech.earcon(Earcon::Boundary);
                let msg = self.msg("authoring-no-link");
                self.tell(&msg);
            }
        }
    }

    /// Cycles the typing echo (characters and words, characters, words,
    /// none) and saves it (`[editing] echo_characters` and `echo_words`).
    pub(crate) fn cycle_typing_echo(&mut self) {
        let e = &self.settings.editing;
        let now = (e.echo_characters, e.echo_words);
        let i = ECHO_CYCLE
            .iter()
            .position(|(c, w, _)| (*c, *w) == now)
            .unwrap_or(ECHO_CYCLE.len() - 1);
        let (c, w, name) = ECHO_CYCLE[(i + 1) % ECHO_CYCLE.len()];
        self.settings.editing.echo_characters = c;
        self.settings.editing.echo_words = w;
        self.settings_dirty = true;
        let msg = self.msg_args("authoring-typing-echo", &args!["echo" => name]);
        self.tell(&msg);
    }

    /// The text Copy takes: the selection, else the sentence at the cursor
    /// (the editor's selection while editing).
    fn copy_source(&self) -> Option<(CharRange, bool)> {
        let s = self.session.as_ref()?;
        if let Some(r) = s.selection.filter(|r| !r.is_empty()) {
            return Some((r, true));
        }
        if self.edit.is_some() {
            return None;
        }
        unit_at(&s.doc, s.cursor, Unit::Sentence)
            .filter(|r| !r.is_empty())
            .map(|r| (r, false))
    }

    /// Copy: the selection (or, reading, the sentence at the cursor) goes
    /// to the clipboard, and what was copied is said.
    pub(crate) fn copy(&mut self) -> Vec<Effect> {
        let Some((range, selected)) = self.copy_source() else {
            let msg = self.msg("authoring-nothing-to-copy");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let text = s.doc.slice(range);
        let id = if selected {
            "authoring-copied"
        } else {
            "authoring-copied-sentence"
        };
        let spoken = text_util::summary_text(self.cat(), &text, "copied").unwrap_or_else(|| {
            self.msg_args(id, &args!["text" => text_util::preview(&s.doc, range, 8)])
        });
        self.authoring.copied = Some(text.clone());
        self.clipboard = Some(text);
        self.tell(&spoken);
        vec![Effect::Redraw]
    }

    /// Cut (edit mode): the selection goes to the clipboard and is deleted
    /// as one undo step.
    pub(crate) fn cut(&mut self) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("cut-text");
        }
        let Some(range) = self
            .session
            .as_ref()
            .and_then(|s| s.selection)
            .filter(|r| !r.is_empty())
        else {
            let msg = self.msg("authoring-nothing-to-cut");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let text = s.doc.slice(range);
        let spoken = text_util::summary_text(self.cat(), &text, "cut").unwrap_or_else(|| {
            self.msg_args(
                "authoring-cut",
                &args!["text" => text_util::preview(&s.doc, range, 8)],
            )
        });
        self.authoring.copied = Some(text.clone());
        self.clipboard = Some(text);
        self.delete_quietly();
        self.tell(&spoken);
        vec![Effect::Redraw]
    }

    /// Text copied or cut since the last call, for the frontend to put on
    /// the system clipboard (the terminal sends [`osc52`]).
    pub fn take_clipboard(&mut self) -> Option<String> {
        self.clipboard.take()
    }

    /// Tab and Shift+Tab in edit mode: in a table row, to the next or
    /// previous cell (across rows, skipping the delimiter row), saying the
    /// column header and the cell; outside a table, Tab types a tab.
    pub(crate) fn table_cell(&mut self, dir: Direction) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("move-cells");
        }
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let doc = &s.doc;
        let line = text_util::line_of(doc, s.cursor);
        let text = text_util::line_text(doc, line);
        if !mdline::is_table_row(&text) {
            return match dir {
                Direction::Forward => self.insert("\t"),
                Direction::Backward => {
                    let msg = self.msg("authoring-not-in-table");
                    self.tell(&msg);
                    vec![Effect::Redraw]
                }
            };
        }
        // The table's lines: contiguous table rows around this one.
        let count = text_util::line_count(doc);
        let mut first = line;
        while first > 0 && mdline::is_table_row(&text_util::line_text(doc, first - 1)) {
            first -= 1;
        }
        let mut last = line;
        while last + 1 < count && mdline::is_table_row(&text_util::line_text(doc, last + 1)) {
            last += 1;
        }
        // Every cell in order: (line, char range in the document).
        let mut cells: Vec<(usize, CharRange)> = Vec::new();
        for l in first..=last {
            let t = text_util::line_text(doc, l);
            if mdline::is_table_delimiter(&t) {
                continue;
            }
            let start = text_util::line_range(doc, l).start.0;
            for r in mdline::table_cells(&t) {
                let a = start + t[..r.start].chars().count();
                let b = start + t[..r.end].chars().count();
                cells.push((l, CharRange::new(a, b)));
            }
        }
        let cursor = s.cursor;
        // The cell the cursor is in: the last one starting at or before it.
        let here = cells
            .iter()
            .rposition(|(l, r)| *l == line && r.start <= cursor)
            .or_else(|| cells.iter().position(|(l, _)| *l == line));
        let target = match (here, dir) {
            (Some(i), Direction::Forward) if cursor < cells[i].1.start => Some(i),
            (Some(i), Direction::Forward) => (i + 1 < cells.len()).then_some(i + 1),
            (Some(i), Direction::Backward) => i.checked_sub(1),
            (None, _) => None,
        };
        let Some(t) = target else {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg_args(
                "authoring-table-edge",
                &args!["dir" => crate::words::dir_key(dir)],
            );
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let (tline, trange) = cells[t];
        let header_cells: Vec<String> = {
            let t0 = text_util::line_text(doc, first);
            mdline::table_cells(&t0)
                .into_iter()
                .map(|r| t0[r].to_owned())
                .collect()
        };
        let column = cells[..t].iter().filter(|(l, _)| *l == tline).count();
        let header = header_cells
            .get(column)
            .filter(|h| !h.is_empty())
            .cloned()
            .unwrap_or_else(|| self.msg_args("authoring-table-column", &args!["n" => column + 1]));
        let content = doc.slice(trange);
        let content = if content.trim().is_empty() {
            self.msg("nav-blank")
        } else {
            content
        };
        // Rows counted as in reading: the header is row 1.
        let row = (first..=tline)
            .filter(|&l| !mdline::is_table_delimiter(&text_util::line_text(doc, l)))
            .count();
        let new_row = here.is_none_or(|h| cells[h].0 != tline);
        let msg = if new_row {
            self.msg_args(
                "authoring-table-cell-row",
                &args!["row" => row, "header" => header, "content" => content],
            )
        } else {
            format!("{header}: {content}")
        };
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            // Select the cell's text, so typing replaces it.
            ed.set_selection(if trange.is_empty() {
                Selection::caret(trange.start)
            } else {
                Selection::new(trange.start, trange.end)
            });
        }
        self.after_edit(&ropey::Rope::new(), &[]);
        self.tell(&msg);
        vec![Effect::Redraw]
    }
}

/// Verbosity levels in cycle order.
const VERBOSITY_CYCLE: [(Verbosity, &str); 3] = [
    (Verbosity::Low, "low"),
    (Verbosity::Normal, "normal"),
    (Verbosity::High, "high"),
];

/// Punctuation levels in cycle order.
const PUNCTUATION_CYCLE: [(PunctuationLevel, &str); 3] = [
    (PunctuationLevel::None, "none"),
    (PunctuationLevel::Some, "some"),
    (PunctuationLevel::All, "all"),
];

impl App {
    /// Select all (Ctrl+A in edit mode; from the palette while reading).
    pub(crate) fn select_all(&mut self) {
        let Some(len) = self.session.as_ref().map(|s| s.doc.len_chars()) else {
            return;
        };
        if len == 0 {
            let msg = self.msg("authoring-nothing-to-select");
            self.tell(&msg);
            return;
        }
        let words = self
            .session
            .as_ref()
            .map_or(0, |s| count_words(s.doc.text().chars()));
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(Selection::new(0, len));
            self.after_edit(&ropey::Rope::new(), &[]);
        } else {
            self.select(CharRange::new(0, len));
        }
        let msg = self.msg_args(
            "authoring-selected-all",
            &args!["n" => words, "count" => thousands(words)],
        );
        self.tell(&msg);
    }

    /// Deletes the word before or after the caret (edit mode), or the
    /// selection when there is one; says what went.
    pub(crate) fn delete_word(&mut self, dir: Direction) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("delete-words");
        }
        self.stop_speech();
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let Some(ed) = self.edit.as_ref().and_then(|e| e.session.editor()) else {
            return vec![Effect::Redraw];
        };
        let doc = &s.doc;
        let sel = ed.selection();
        let head = sel.head;
        let range = if !sel.is_caret() {
            sel.range()
        } else {
            match dir {
                Direction::Backward => {
                    let start = text_util::word_containing(doc, head)
                        .filter(|w| w.start < head)
                        .map(|w| w.start)
                        .or_else(|| {
                            textweaver_text::navigate(
                                doc,
                                head,
                                Unit::Word,
                                Direction::Backward,
                                textweaver_text::NavOptions::default(),
                            )
                            .map(|t| t.range.start)
                        })
                        .unwrap_or(CharPos::ZERO);
                    CharRange::new(start, head)
                }
                Direction::Forward => {
                    let end = textweaver_text::navigate(
                        doc,
                        head,
                        Unit::Word,
                        Direction::Forward,
                        textweaver_text::NavOptions::default(),
                    )
                    .map_or(doc.end(), |t| t.range.start);
                    CharRange::new(head, end.max(head))
                }
            }
        };
        if range.is_empty() {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg(match dir {
                Direction::Forward => "nav-end-of-document-stop",
                Direction::Backward => "nav-top-of-document-stop",
            });
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let removed = doc.slice(range);
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(Selection::new(range.start, range.end));
        }
        self.delete_quietly();
        let said = text_util::summary_text(self.cat(), &removed, "deleted").unwrap_or_else(|| {
            let t = removed.trim();
            if t.is_empty() {
                self.msg("authoring-space-deleted")
            } else {
                self.msg_args("authoring-deleted", &args!["text" => t])
            }
        });
        self.show(&said);
        if self.settings.editing.echo_deletions && self.route(textweaver_a11y::Channel::Echo).speak
        {
            self.speech.say(said, textweaver_speech::SayMode::Interrupt);
        }
        vec![Effect::Redraw]
    }

    /// Paste (Ctrl+V in edit mode): the text last copied or cut in
    /// textweaver. Most terminals paste the system clipboard themselves
    /// when Ctrl+V or Ctrl+Shift+V is pressed; this is for the times they
    /// pass the key on instead.
    pub(crate) fn paste(&mut self) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("paste");
        }
        match self.authoring.copied.clone() {
            Some(text) if !text.is_empty() => self.insert(&text),
            _ => {
                // The terminal's own paste key, not one of textweaver's.
                let msg = self.msg_args(
                    "authoring-nothing-copied",
                    &args!["key" => "Control Shift V"],
                );
                self.tell(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// Cycles how much is announced (low, normal, high) and saves it.
    pub(crate) fn cycle_verbosity(&mut self) {
        let now = self.settings.speech.verbosity;
        let i = VERBOSITY_CYCLE
            .iter()
            .position(|(v, _)| *v == now)
            .unwrap_or(1);
        let (next, name) = VERBOSITY_CYCLE[(i + 1) % VERBOSITY_CYCLE.len()];
        self.settings.speech.verbosity = next;
        self.settings_dirty = true;
        let msg = self.msg_args("authoring-verbosity", &args!["level" => name]);
        self.tell(&msg);
    }

    /// Cycles how much punctuation is spoken (none, some, all), applies it
    /// to the voice at once, and saves it.
    pub(crate) fn cycle_punctuation(&mut self) {
        let now = self.settings.speech.punctuation;
        let i = PUNCTUATION_CYCLE
            .iter()
            .position(|(p, _)| *p == now)
            .unwrap_or(1);
        let (next, name) = PUNCTUATION_CYCLE[(i + 1) % PUNCTUATION_CYCLE.len()];
        self.settings.speech.punctuation = next;
        self.settings_dirty = true;
        self.speech.set_punctuation(next);
        let msg = self.msg_args("authoring-punctuation", &args!["level" => name]);
        self.tell(&msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_counted_without_markup() {
        assert_eq!(count_words("## Methods and results\n- one\n".chars()), 4);
        assert_eq!(count_words("".chars()), 0);
        assert_eq!(count_words("  don't stop -- now ".chars()), 3);
        let c = textweaver_lexicon::i18n::Catalog::english();
        let said = |n: usize| {
            c.fmt(
                "authoring-selected-all",
                &args!["n" => n, "count" => thousands(n)],
            )
        };
        assert_eq!(said(1), "Selected all, 1 word.");
        assert_eq!(said(3412), "Selected all, 3,412 words.");
    }

    #[test]
    fn osc52_is_base64() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64("héllo".as_bytes()), "aMOpbGxv");
        assert_eq!(osc52("hi"), "\x1b]52;c;aGk=\x07");
    }
}
