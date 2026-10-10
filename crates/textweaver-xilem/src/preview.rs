//! The preview pane (B1-p1): in edit mode, the document as the reading
//! view draws it, beside the editor.
//!
//! - **Optional, off by default.** `[preview] pane` turns it on (the View
//!   menu's Show preview, the palette's `show preview`, or Alt+F5); the
//!   choice is remembered. It shows only while editing.
//! - **textweaver's own view.** The editor's Markdown is parsed by the
//!   same loader that opens a file, and drawn by a second, read-only
//!   [`DocumentView`] (headings, lists, tables, code, callouts, math, and
//!   images with their captions, as reading draws them); no web engine.
//! - **Never slows typing.** Each new revision of the text is handed to a
//!   worker thread as the rope the editor already holds (a cheap clone);
//!   the worker waits for a pause in typing (`[preview] pane_delay_ms`,
//!   300 ms by default), parses only the newest text, and rings the
//!   window, which swaps the result in on its next frame.
//! - **Follows the caret.** The caret is carried into the preview by its
//!   words ([`Aligner`], the map edit mode already uses between the source
//!   and the reading text); the preview scrolls so that block is in view
//!   and marks it as the spoken sentence is marked: a band and a line
//!   under it, the theme's roles, never color alone.
//! - **Quiet.** The focus and the caret stay in the editor; nothing is
//!   said when the preview changes. F6 reaches it: a `Region` landmark
//!   named "Preview" around a read-only document with the same text runs
//!   as any document, so a reader who moves there can read it.
//! - **No cost when off.** [`sync`] reads one setting and returns.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use masonry::accesskit::Role;
use masonry::core::{NewWidget, WidgetTag};
use textweaver_app::align::Aligner;
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::formats::{LoadOptions, Registry, Source};
use textweaver_app::keymap::ActionId;
use textweaver_app::lexicon::args;
use textweaver_app::store::DocKey;
use textweaver_app::text::Document;
use textweaver_app::{App, DocWindow};

use crate::document::{DocAids, DocFont, DocModel, DocState, DocumentView};
use crate::gui::DOC;
use crate::sidebar::{FocusHost, SIDEBAR, Sidebar};
use crate::theme::Palette;
use crate::widgets::Region;
use crate::window;

/// The preview pane: a `Region` named "Preview".
pub const PREVIEW: WidgetTag<Region> = WidgetTag::named("tw-preview");
/// The preview's document view.
pub const PREVIEW_DOC: WidgetTag<DocumentView> = WidgetTag::named("tw-preview-doc");

/// A document and its text revision.
type Revision = (DocKey, u64);

/// A text to parse.
struct Job {
    generation: u64,
    revision: Revision,
    source: ropey::Rope,
    /// The loader's format hint (`md`, or `txt` for a plain-text file).
    hint: &'static str,
    options: LoadOptions,
    /// How long typing must pause before it is parsed.
    pause: Duration,
}

/// A text parsed, and how to carry a position in it into the preview.
pub struct Parsed {
    generation: u64,
    revision: Revision,
    doc: Document,
    to_preview: Aligner,
}

/// Parses `job`'s text as a file of its kind would be loaded. `None` when
/// the loader refuses it (the preview keeps what it showed).
fn parse(job: Job, registry: &Registry) -> Option<Parsed> {
    let data = job.source.to_string().into_bytes();
    let source = Source::Bytes {
        data,
        hint: job.hint.to_owned(),
    };
    let doc = registry.load(&source, &job.options).ok()?;
    let to_preview = Aligner::new(&job.source, doc.text());
    Some(Parsed {
        generation: job.generation,
        revision: job.revision,
        doc,
        to_preview,
    })
}

/// The thread that parses the editor's text off the window's thread.
pub struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Parsed>,
}

impl Worker {
    /// Starts the worker; `wake` is called when a parse is ready (the
    /// window posts itself a tick). `None` if no thread could start: the
    /// window then parses on its own thread, after the pause.
    pub fn spawn(wake: impl Fn() + Send + 'static) -> Option<Self> {
        let (jobs, inbox) = mpsc::channel::<Job>();
        let (outbox, done) = mpsc::channel::<Parsed>();
        std::thread::Builder::new()
            .name("tw-preview".into())
            .spawn(move || {
                let registry = Registry::with_builtins();
                while let Ok(mut job) = inbox.recv() {
                    // Wait for the pause in typing; each newer text starts
                    // it again, and only the newest is parsed.
                    loop {
                        match inbox.recv_timeout(job.pause) {
                            Ok(newer) => job = newer,
                            Err(RecvTimeoutError::Timeout) => break,
                            Err(RecvTimeoutError::Disconnected) => return,
                        }
                    }
                    let Some(parsed) = parse(job, &registry) else {
                        continue;
                    };
                    if outbox.send(parsed).is_err() {
                        return;
                    }
                    wake();
                }
            })
            .ok()?;
        Some(Worker { jobs, done })
    }
}

/// What the pane last showed, to change only what changed.
#[derive(Default)]
pub struct PreviewShown {
    /// The parsing thread; `None` parses on the caller's thread at once
    /// (tests, the frame probe).
    worker: Option<Worker>,
    /// The pane is in the window.
    open: bool,
    /// The last text handed out to be parsed.
    generation: u64,
    /// Its revision.
    sent: Option<Revision>,
    /// A parse made on this thread, waiting to be shown.
    ready: Option<Parsed>,
    /// The parse shown.
    parsed: Option<Parsed>,
    /// The part of the parsed document the view holds.
    window: Option<DocWindow>,
    /// The window's paragraphs, as the blocks the caret can be in.
    blocks: Vec<CharRange>,
    /// The block marked as being edited.
    block: Option<CharRange>,
    /// What the view was drawn with.
    palette: String,
    font: Option<textweaver_app::store::reading_aids::FontSettings>,
    aids: Option<DocAids>,
    lang: String,
}

impl PreviewShown {
    /// Parses on a worker thread, which calls `wake` when a parse is ready.
    pub fn with_worker(wake: impl Fn() + Send + 'static) -> Self {
        PreviewShown {
            worker: Worker::spawn(wake),
            ..PreviewShown::default()
        }
    }

    /// True while the pane is in the window.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The block of the preview marked as being edited.
    pub fn block(&self) -> Option<CharRange> {
        self.block
    }

    /// The preview's text, as parsed (none before the first parse).
    pub fn text(&self) -> Option<String> {
        self.parsed.as_ref().map(|p| p.doc.text().to_string())
    }
}

/// The loader's hint for the text being edited: a plain-text file is
/// parsed as text; Markdown, and every converted format (edited as
/// Markdown), as Markdown.
fn hint_for(doc: &Document) -> &'static str {
    let ext = doc
        .meta
        .path
        .as_ref()
        .and_then(|p| p.extension())
        .map(|e| e.to_string_lossy().to_lowercase());
    match ext.as_deref() {
        Some("txt" | "text") => "txt",
        _ => "md",
    }
}

/// The preview's model for `range` of `doc`, titled `title`.
fn model_of(doc: &Document, range: CharRange, title: &str) -> DocModel {
    DocModel {
        paragraphs: window::window_paragraphs(doc, range),
        spans: window::window_spans(doc, range),
        doc_len: doc.len_chars(),
        title: title.to_owned(),
        breaks: Vec::new(),
        separator: String::new(),
    }
}

/// The block (paragraph) at `pos`: the last starting at or before it.
fn block_at(blocks: &[CharRange], pos: CharPos) -> Option<CharRange> {
    let i = blocks.partition_point(|b| b.start <= pos).checked_sub(1)?;
    blocks.get(i).copied()
}

/// Brings the pane in step with `[preview] pane`, edit mode, and the
/// editor: shows or closes it, hands a new text to be parsed, shows a
/// finished parse, and marks the block at the caret. Never moves the
/// focus, except back to the editor when the pane holding it closes, and
/// says nothing. Off, it reads one setting and returns.
pub fn sync(app: &App, palette: &Palette, shown: &mut PreviewShown, host: &mut impl FocusHost) {
    let want = app.settings().preview.pane && app.is_editing() && app.session().is_some();
    if !want {
        if shown.open {
            close(shown, host);
        }
        return;
    }
    if !shown.open {
        open(app, palette, host);
        shown.open = true;
    }
    restyle(app, palette, shown, host);
    request(app, shown);
    take_done(shown);
    follow(app, shown, host);
}

/// Puts the pane in the window, beside the editor, drawn as the editor.
fn open(app: &App, palette: &Palette, host: &mut impl FocusHost) {
    let c = app.catalog();
    let name = c.tr("gui-preview");
    let (font, passes) = host.edit(DOC, |d| (d.widget.font().clone(), d.widget.full_passes()));
    let view = DocumentView::new(palette.clone(), font, passes)
        .with_preview(name.clone())
        .with_label(name.clone())
        .with_empty_hint(c.tr("gui-preview-empty"));
    let view = NewWidget::new(view).with_tag(PREVIEW_DOC);
    let doc = view.id();
    let region = NewWidget::new(Region::new(view, Role::Region, name)).with_tag(PREVIEW);
    host.edit(SIDEBAR, |mut s| Sidebar::open_preview(&mut s, region, doc));
}

/// Takes the pane out; the focus, if it was there, goes to the editor.
fn close(shown: &mut PreviewShown, host: &mut impl FocusHost) {
    let (preview, doc) = host.edit(SIDEBAR, |s| (s.widget.preview_doc_id(), s.widget.doc_id()));
    let had_focus = preview.is_some() && host.focused() == preview;
    host.edit(SIDEBAR, |mut s| Sidebar::close_preview(&mut s));
    if had_focus || host.focused().is_none() {
        host.focus(Some(doc));
    }
    let worker = shown.worker.take();
    *shown = PreviewShown {
        worker,
        ..PreviewShown::default()
    };
}

/// The theme, the reading font, text spacing, the line length, and the
/// interface language, as the editor has them (the reading ruler is the
/// reading line's, and stays off here).
fn restyle(app: &App, palette: &Palette, shown: &mut PreviewShown, host: &mut impl FocusHost) {
    if shown.palette != palette.name {
        host.edit(PREVIEW_DOC, |mut d| {
            DocumentView::set_palette(&mut d, palette.clone())
        });
        shown.palette.clone_from(&palette.name);
    }
    let settings = &app.settings().reading_aids.font;
    if shown.font.as_ref() != Some(settings) {
        let font: DocFont = crate::fonts::doc_font(settings);
        host.edit(PREVIEW_DOC, |mut d| DocumentView::set_font(&mut d, font));
        shown.font = Some(settings.clone());
    }
    let aids = DocAids {
        spacing: (&app.settings().reading_aids.spacing).into(),
        ruler: Default::default(),
        measure: app.settings().display.measure,
    };
    if shown.aids != Some(aids) {
        host.edit(PREVIEW_DOC, |mut d| DocumentView::set_aids(&mut d, aids));
        shown.aids = Some(aids);
    }
    let c = app.catalog();
    if shown.lang != c.lang() {
        shown.lang = c.lang().to_owned();
        let name = c.tr("gui-preview");
        let hint = c.tr("gui-preview-empty");
        host.edit(PREVIEW, |mut r| Region::set_label(&mut r, name.clone()));
        host.edit(PREVIEW_DOC, |mut d| {
            DocumentView::set_label(&mut d, name.clone());
            DocumentView::set_editing_word(&mut d, name);
            DocumentView::set_empty_hint(&mut d, hint);
        });
    }
}

/// Hands a new revision of the text to be parsed: to the worker, which
/// waits for the pause in typing, or, with no worker, parsed here at once.
/// The first text after the pane opens is parsed with no pause.
fn request(app: &App, shown: &mut PreviewShown) {
    let Some(s) = app.session() else {
        return;
    };
    if shown
        .sent
        .as_ref()
        .is_some_and(|(k, r)| *k == s.key && *r == s.revision)
    {
        return;
    }
    let first = shown.sent.is_none();
    shown.generation += 1;
    shown.sent = Some((s.key.clone(), s.revision));
    let pause = if first {
        Duration::ZERO
    } else {
        Duration::from_millis(u64::from(app.settings().preview.pane_delay_ms))
    };
    let job = Job {
        generation: shown.generation,
        revision: (s.key.clone(), s.revision),
        source: s.doc.text().clone(),
        hint: hint_for(&s.doc),
        options: textweaver_app::load_options(app.settings()),
        pause,
    };
    let job = match &shown.worker {
        Some(w) => match w.jobs.send(job) {
            Ok(()) => return,
            // The worker is gone: parse here.
            Err(mpsc::SendError(job)) => job,
        },
        None => job,
    };
    shown.ready = parse(job, &Registry::with_builtins());
}

/// Takes the newest finished parse, if it is newer than the one shown.
fn take_done(shown: &mut PreviewShown) {
    let mut newest = shown.ready.take();
    if let Some(w) = &shown.worker {
        while let Ok(p) = w.done.try_recv() {
            newest = Some(p);
        }
    }
    let Some(p) = newest else {
        return;
    };
    if shown
        .parsed
        .as_ref()
        .is_some_and(|old| old.generation >= p.generation)
    {
        return;
    }
    shown.parsed = Some(p);
    // A new parse: the view's text is built again below.
    shown.window = None;
}

/// Shows the part of the preview around the caret, and marks the block
/// being edited (scrolled into view, unless the reader is in the preview).
fn follow(app: &App, shown: &mut PreviewShown, host: &mut impl FocusHost) {
    let (Some(p), Some(s)) = (&shown.parsed, app.session()) else {
        return;
    };
    if p.revision.0 != s.key {
        return;
    }
    let pos = p.to_preview.map(s.cursor);
    let rebuild = shown.window.is_none_or(|w| !w.contains(&p.doc, pos));
    if rebuild {
        // The same text that stays keeps its run nodes (a slide), so a
        // reader in the preview keeps the place; a first parse is new.
        let slide = !shown.blocks.is_empty();
        let w = DocWindow::around(&p.doc, pos);
        let model = model_of(&p.doc, w.range(), &s.title);
        shown.blocks = model
            .paragraphs
            .iter()
            .map(|q| CharRange::new(q.start.0, q.start.0 + q.len_chars()))
            .collect();
        host.edit(PREVIEW_DOC, |mut d| {
            if slide {
                DocumentView::slide_model(&mut d, model);
            } else {
                DocumentView::set_model(&mut d, model);
            }
        });
        shown.window = Some(w);
        shown.block = None;
    }
    let block = block_at(&shown.blocks, pos);
    if block == shown.block {
        return;
    }
    shown.block = block;
    let preview = host.edit(SIDEBAR, |s| s.widget.preview_doc_id());
    let reading_it = preview.is_some() && host.focused() == preview;
    host.edit(PREVIEW_DOC, |mut d| {
        DocumentView::set_block(&mut d, block);
        if !reading_it {
            // The preview's own caret goes to the block, which scrolls it
            // into view; the reader's focus stays in the editor.
            let caret = block.map_or(pos, |b| b.start);
            DocumentView::set_state(
                &mut d,
                DocState {
                    caret,
                    ..DocState::default()
                },
            );
        }
    });
}

/// What the preview key did ([`toggle`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggled {
    /// The pane was turned on and shows beside the editor.
    Shown,
    /// The pane was turned on; it shows in edit mode.
    Later,
    /// The pane was turned off.
    Hidden,
    /// A narrow, short window had hidden it: the focus went to it.
    Focused,
}

/// The preview key (Alt+F5, the View menu's Show preview): turns
/// `[preview] pane` on or off, saved. With the pane on but hidden by a
/// narrow, short window, it shows the pane and goes to it instead, as the
/// panel keys do; the pane stays while it has the focus.
pub fn toggle(
    app: &mut App,
    palette: &Palette,
    shown: &mut PreviewShown,
    host: &mut impl FocusHost,
) -> Toggled {
    let (preview, on_screen) = host.edit(SIDEBAR, |s| {
        (s.widget.preview_doc_id(), s.widget.shown_preview_id())
    });
    let on = app.settings().preview.pane;
    if on && preview.is_some() && on_screen.is_none() && host.focused() != preview {
        host.edit(SIDEBAR, |mut s| Sidebar::reveal_preview(&mut s));
        host.focus(preview);
        return Toggled::Focused;
    }
    let _ = app.update_settings(|s| s.preview.pane = !on);
    sync(app, palette, shown, host);
    match (on, shown.open) {
        (true, _) => Toggled::Hidden,
        (false, true) => Toggled::Shown,
        (false, false) => Toggled::Later,
    }
}

/// What the window says for the preview key: the pane shown (and the key
/// that moves to it), hidden, or on for edit mode. Nothing when the focus
/// moved to it: the screen reader says the preview.
pub fn toggled_message(app: &App, t: Toggled) -> Option<String> {
    let c = app.catalog();
    Some(match t {
        Toggled::Shown => {
            let key = textweaver_app::named_key(app.keymap(), ActionId::NextRegion);
            let said = c.fmt("gui-preview-shown", &args!["key" => key.as_str()]);
            textweaver_app::written_text(&said).into_owned()
        }
        Toggled::Hidden => c.tr("gui-preview-hidden"),
        Toggled::Later => c.tr("gui-preview-later"),
        Toggled::Focused => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_is_the_paragraph_at_the_position() {
        let blocks = [
            CharRange::new(0, 5),
            CharRange::new(6, 20),
            CharRange::new(21, 30),
        ];
        assert_eq!(block_at(&blocks, CharPos(0)), Some(blocks[0]));
        assert_eq!(block_at(&blocks, CharPos(12)), Some(blocks[1]));
        // Between two blocks (the line break): the one before.
        assert_eq!(block_at(&blocks, CharPos(5)), Some(blocks[0]));
        assert_eq!(block_at(&blocks, CharPos(99)), Some(blocks[2]));
        assert_eq!(block_at(&[], CharPos(3)), None);
    }

    fn job(generation: u64, text: &str, pause: Duration) -> Job {
        Job {
            generation,
            revision: (DocKey::for_path(std::path::Path::new("a.md")), generation),
            source: ropey::Rope::from_str(text),
            hint: "md",
            options: LoadOptions::default(),
            pause,
        }
    }

    /// The worker waits for the pause, parses only the newest text, and
    /// rings once; the heading and the list come out as the reading view
    /// has them, and the caret is carried over by its words.
    #[test]
    fn the_worker_parses_only_the_newest_text_after_the_pause() {
        let (rang, rings) = mpsc::channel();
        let w = Worker::spawn(move || {
            let _ = rang.send(());
        })
        .expect("the worker starts");
        let pause = Duration::from_millis(150);
        w.jobs.send(job(1, "# Old\n", pause)).unwrap();
        w.jobs
            .send(job(2, "# Notes\n\n- *First* item\n", pause))
            .unwrap();
        rings
            .recv_timeout(Duration::from_secs(20))
            .expect("the worker rings");
        let p = w
            .done
            .recv_timeout(Duration::from_secs(1))
            .expect("a parse");
        assert_eq!(p.generation, 2);
        let text = p.doc.text().to_string();
        assert!(text.starts_with("Notes\n"), "{text:?}");
        assert!(text.contains("First item"), "{text:?}");
        assert!(!text.contains('#') && !text.contains('*'), "{text:?}");
        // The "t" of "item" in the source is the "t" of "item" here.
        let at = p.to_preview.map(CharPos(20));
        assert_eq!(text.chars().nth(at.0), Some('t'), "{text:?} at {at:?}");
        assert!(w.done.try_recv().is_err(), "only the newest is parsed");
    }

    #[test]
    fn a_plain_text_file_is_previewed_as_text() {
        let mut d = Document::new(Default::default(), "x".into(), Vec::new());
        assert_eq!(hint_for(&d), "md");
        d.meta.path = Some("notes.TXT".into());
        assert_eq!(hint_for(&d), "txt");
        d.meta.path = Some("paper.docx".into());
        assert_eq!(hint_for(&d), "md");
    }
}
