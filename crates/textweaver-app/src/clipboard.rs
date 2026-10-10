//! Pasting from the system clipboard (beta 1, B1-cm).
//!
//! A frontend gives the app its clipboard with [`App::set_clipboard`]:
//! anything that implements [`Clipboard`] and reads the forms textweaver
//! understands ([`ClipboardContents`]: plain text, HTML, and RTF). Paste
//! (Ctrl+V) reads it then:
//!
//! - **Formatted text pastes as Markdown** (the owner's decision): HTML
//!   from a browser and RTF from a word processor go through textweaver's
//!   own loaders ([`HtmlLoader`], [`RtfLoader`]) and Markdown writer
//!   ([`textweaver_formats::to_markdown`]), so headings, lists, emphasis,
//!   links, tables, and code survive, and the app says what came in:
//!   "Pasted as Markdown: 1 heading, 3 paragraphs, 1 list".
//! - **Paste as plain text** inserts only the clipboard's plain text.
//! - **Large formatted pastes** (over [`LARGE_PASTE`] bytes) are converted
//!   on a worker thread, so the keys keep working; the text goes in when
//!   the conversion is done ([`App::tick`]).
//! - Every paste is **one undo step**.
//!
//! Without a clipboard (the terminal over SSH, where the system clipboard
//! is not the listener's), Paste inserts the text last copied or cut in
//! textweaver, and the terminal's own paste arrives as typed text
//! ([`App::paste_contents`]).
//!
//! [`FakeClipboard`] holds whatever a test puts on it.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use textweaver_core::MarkerKind;
use textweaver_formats::{HtmlLoader, LoadOptions, Loader, RtfLoader, Source};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::App;
use crate::command::Effect;

/// Formatted text longer than this many bytes is converted on a worker
/// thread.
pub const LARGE_PASTE: usize = 64 * 1024;

/// What the system clipboard holds, in the forms textweaver reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClipboardContents {
    /// The plain text.
    pub text: Option<String>,
    /// HTML (a browser's or an editor's copy). A Windows `HTML Format`
    /// header, if the frontend leaves one, is skipped.
    pub html: Option<String>,
    /// Rich Text Format (a word processor's copy).
    pub rtf: Option<String>,
}

impl ClipboardContents {
    /// Contents with plain text only.
    pub fn text(text: impl Into<String>) -> Self {
        ClipboardContents {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    /// True when no form holds anything.
    pub fn is_empty(&self) -> bool {
        [&self.text, &self.html, &self.rtf]
            .iter()
            .all(|f| f.as_deref().is_none_or(str::is_empty))
    }

    fn formatted_len(&self) -> usize {
        self.html.as_ref().map_or(0, String::len) + self.rtf.as_ref().map_or(0, String::len)
    }
}

/// A clipboard the app reads when pasting: the frontend's system
/// clipboard, or [`FakeClipboard`] in tests.
pub trait Clipboard: Send {
    /// What the clipboard holds now. Forms it cannot read are `None`.
    fn read(&mut self) -> ClipboardContents;
}

/// A clipboard for tests: it holds what [`set`](Self::set) put on it.
/// Clones share the contents, so a test keeps one and gives the app the
/// other.
#[derive(Clone, Debug, Default)]
pub struct FakeClipboard(Arc<Mutex<ClipboardContents>>);

impl FakeClipboard {
    /// An empty clipboard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the contents.
    pub fn set(&self, contents: ClipboardContents) {
        if let Ok(mut c) = self.0.lock() {
            *c = contents;
        }
    }
}

impl Clipboard for FakeClipboard {
    fn read(&mut self) -> ClipboardContents {
        self.0.lock().map(|c| c.clone()).unwrap_or_default()
    }
}

/// How much structure a Markdown paste brought in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    headings: usize,
    paragraphs: usize,
    lists: usize,
    tables: usize,
    code: usize,
    quotes: usize,
    links: usize,
}

impl Counts {
    /// "1 heading, 3 paragraphs, 1 list", or `None` when there is nothing
    /// to count (a few words).
    fn describe(&self, c: &Catalog) -> Option<String> {
        let parts: Vec<String> = [
            ("heading", self.headings),
            ("paragraph", self.paragraphs),
            ("list", self.lists),
            ("table", self.tables),
            ("code", self.code),
            ("quote", self.quotes),
            ("link", self.links),
        ]
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|(kind, n)| c.fmt(&format!("paste-part-{kind}"), &args!["n" => n]))
        .collect();
        (!parts.is_empty()).then(|| parts.join(", "))
    }
}

/// Text ready to insert, and the structure it carries when it was
/// converted to Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Prepared {
    pub(crate) text: String,
    pub(crate) markdown: Option<Counts>,
}

/// What a paste of `contents` inserts: Markdown from the HTML or RTF
/// unless `plain`, else the plain text (or, with no plain text, the
/// formatted text's words). `None` when there is nothing to insert.
pub(crate) fn prepare(contents: ClipboardContents, plain: bool) -> Option<Prepared> {
    let html = contents.html.filter(|h| !h.trim().is_empty());
    let rtf = contents.rtf.filter(|r| !r.trim().is_empty());
    let text = contents
        .text
        .map(|t| t.replace("\r\n", "\n").replace('\r', "\n"))
        .filter(|t| !t.is_empty());
    let formatted = || {
        html.as_deref()
            .and_then(|h| load(&HtmlLoader, skip_html_header(h), "html"))
            .or_else(|| rtf.as_deref().and_then(|r| load(&RtfLoader, r, "rtf")))
    };
    if !plain && let Some(doc) = formatted() {
        let md = textweaver_formats::to_markdown(&doc);
        let md = md.trim_end_matches(['\n', ' ']);
        if !md.trim().is_empty() {
            return Some(Prepared {
                text: md.to_owned(),
                markdown: Some(count(&doc)),
            });
        }
    }
    if let Some(text) = text {
        return Some(Prepared {
            text,
            markdown: None,
        });
    }
    // No plain text on the clipboard: the formatted text's words.
    let doc = formatted()?;
    let words = doc.text().to_string();
    let words = words.trim_end_matches('\n');
    (!words.trim().is_empty()).then(|| Prepared {
        text: words.to_owned(),
        markdown: None,
    })
}

/// Loads `data` with `loader`; `None` when it cannot be read.
fn load(loader: &dyn Loader, data: &str, hint: &str) -> Option<textweaver_text::Document> {
    let source = Source::Bytes {
        data: data.as_bytes().to_vec(),
        hint: hint.into(),
    };
    loader.load(&source, &LoadOptions::default()).ok()
}

/// The HTML after the Windows clipboard's `Version:0.9 StartHTML:...`
/// header, when one is there.
fn skip_html_header(html: &str) -> &str {
    if html.starts_with("Version:") {
        html.find('<').map_or(html, |i| &html[i..])
    } else {
        html
    }
}

fn count(doc: &textweaver_text::Document) -> Counts {
    let mut n = Counts::default();
    for m in doc.markers() {
        match m.kind {
            MarkerKind::Heading => n.headings += 1,
            MarkerKind::Paragraph => n.paragraphs += 1,
            MarkerKind::List if m.level <= 1 => n.lists += 1,
            MarkerKind::Table => n.tables += 1,
            MarkerKind::Code if m.level == 1 => n.code += 1,
            MarkerKind::Quote => n.quotes += 1,
            MarkerKind::Link => n.links += 1,
            _ => {}
        }
    }
    n
}

/// A formatted paste being converted on a worker thread.
pub(crate) struct PasteJob {
    rx: Receiver<Option<Prepared>>,
}

impl App {
    /// Gives the app the system clipboard that Paste reads (the frontend's
    /// own, or a [`FakeClipboard`]). Without one, Paste inserts the text
    /// last copied or cut in textweaver.
    pub fn set_clipboard(&mut self, clipboard: Box<dyn Clipboard>) {
        self.system_clipboard = Some(clipboard);
    }

    /// Paste and Paste as plain text: reads the clipboard and inserts it
    /// at the cursor ([`paste_contents`](Self::paste_contents)).
    pub(crate) fn paste_from_clipboard(&mut self, plain: bool) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("paste");
        }
        let contents = self
            .system_clipboard
            .as_mut()
            .map(|c| c.read())
            .unwrap_or_default();
        if !contents.is_empty() {
            return self.paste_contents(contents, plain);
        }
        match self.authoring.copied.clone().filter(|t| !t.is_empty()) {
            Some(text) => self.paste_contents(ClipboardContents::text(text), true),
            None if self.system_clipboard.is_some() => {
                let msg = self.msg("paste-empty");
                self.tell(&msg);
                vec![Effect::Redraw]
            }
            None => {
                // The key of the terminal itself, not one of textweaver's.
                let msg = self.msg_args(
                    "authoring-nothing-copied",
                    &args!["key" => "Control Shift V"],
                );
                self.tell(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// Pastes `contents` at the cursor in edit mode, as one undo step:
    /// formatted text becomes Markdown unless `plain`. A frontend that is
    /// handed pasted data (the bracketed paste of a terminal, the paste
    /// event of a window) calls this; Paste calls it with the contents of
    /// the clipboard. Large formatted text is converted on a worker thread
    /// and inserted from [`App::tick`].
    pub fn paste_contents(&mut self, contents: ClipboardContents, plain: bool) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("paste");
        }
        if !plain && contents.formatted_len() > LARGE_PASTE {
            let (tx, rx) = mpsc::channel();
            let wake = self.waker_slot();
            let spawned = std::thread::Builder::new()
                .name("textweaver-paste".into())
                .spawn(move || {
                    let _ = tx.send(prepare(contents, false));
                    wake.wake();
                });
            if let Err(e) = spawned {
                let msg = self.msg_args("paste-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
            self.paste_job = Some(PasteJob { rx });
            let msg = self.msg("paste-converting");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        match prepare(contents, plain) {
            Some(p) => self.insert_prepared(p),
            None => {
                let msg = self.msg("paste-empty");
                self.tell(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// Whether a formatted paste is still being converted.
    pub fn pasting(&self) -> bool {
        self.paste_job.is_some()
    }

    /// Inserts a converted paste once its worker is done (from
    /// [`App::tick`]).
    pub(crate) fn paste_tick(&mut self) -> Vec<Effect> {
        let Some(job) = &self.paste_job else {
            return Vec::new();
        };
        let prepared = match job.rx.try_recv() {
            Ok(p) => p,
            Err(TryRecvError::Empty) => return Vec::new(),
            Err(TryRecvError::Disconnected) => None,
        };
        self.paste_job = None;
        match prepared {
            // Edit mode may have ended while the text was converted.
            Some(p) if self.edit.is_some() => self.insert_prepared(p),
            Some(_) => self.not_editing("paste"),
            None => {
                let msg = self.msg("paste-empty");
                self.tell(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    fn insert_prepared(&mut self, p: Prepared) -> Vec<Effect> {
        // shortcut: the text goes in at the cursor as it is, so blocks
        // pasted in the middle of a line join that line; start them on a
        // line of their own if listeners find that confusing.
        let said = p
            .markdown
            .and_then(|n| n.describe(self.cat()))
            .map(|parts| self.msg_args("paste-markdown", &args!["parts" => parts]));
        self.insert_said(&p.text, said)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_becomes_markdown_with_its_structure() {
        let html = "<h2>Results</h2><p>One <strong>bold</strong> and \
                    <a href=\"https://example.org/\">a link</a>.</p>\
                    <ul><li>first</li><li>second</li></ul>";
        let p = prepare(
            ClipboardContents {
                text: Some("Results One bold".into()),
                html: Some(html.into()),
                rtf: None,
            },
            false,
        )
        .unwrap();
        assert!(p.text.starts_with("## Results\n"), "{}", p.text);
        assert!(p.text.contains("**bold**"), "{}", p.text);
        assert!(
            p.text.contains("[a link](https://example.org/)"),
            "{}",
            p.text
        );
        assert!(p.text.contains("- first\n- second"), "{}", p.text);
        let n = p.markdown.unwrap();
        assert_eq!((n.headings, n.lists, n.links), (1, 1, 1));
    }

    #[test]
    fn plain_keeps_only_the_plain_text() {
        let p = prepare(
            ClipboardContents {
                text: Some("Results\r\nOne bold".into()),
                html: Some("<h2>Results</h2>".into()),
                rtf: None,
            },
            true,
        )
        .unwrap();
        assert_eq!(p.text, "Results\nOne bold");
        assert_eq!(p.markdown, None);
    }

    #[test]
    fn the_windows_html_header_is_skipped() {
        let h = "Version:0.9\r\nStartHTML:0000000105\r\n<html><body><em>x</em></body></html>";
        assert!(skip_html_header(h).starts_with("<html>"));
        let p = prepare(
            ClipboardContents {
                html: Some(h.into()),
                ..Default::default()
            },
            false,
        )
        .unwrap();
        assert_eq!(p.text, "*x*");
    }

    #[test]
    fn empty_contents_paste_nothing() {
        assert!(ClipboardContents::default().is_empty());
        assert!(ClipboardContents::text("").is_empty());
        assert_eq!(prepare(ClipboardContents::default(), false), None);
    }
}
