//! Export and preview from inside the reader, and listening to the text as
//! it will render.
//!
//! - **Export** (palette commands `export_html`, `export_pdf`,
//!   `export_docx`, `export_epub`, `export_brf`): the document goes through
//!   `textweaver-convert`, as `tw convert` would convert it, into a file of
//!   the same name next to it (`essay.md` gives `essay.pdf`). In edit mode
//!   the live text is exported, saved or not. Citations are formatted and a
//!   References section added, from the bibliography the front matter
//!   names, the folder's `references.json`, and your library. The work runs
//!   on another thread; when it is done textweaver says where the file went
//!   and asks "Open it? y or n". HTML first asks "Theme for the HTML
//!   page?": the themes, this session's last answer or else the reading
//!   theme first and selected; Escape cancels. The page carries that
//!   theme's colors alone, not Galaxy's light and dark pair.
//! - **Preview in browser** writes the page, with math as MathML, to the
//!   `preview` folder of the cache folder and opens the default browser,
//!   after the same theme question; rewrites keep the answer. The first
//!   preview of a session says in one sentence what will happen ("Preview
//!   opens in your browser. Press F5 there after each save.").
//!   Each save while editing writes it again, and textweaver says "Preview
//!   updated. Press F5 in the browser." With `[preview] follow` ("Browser
//!   preview follows", View menu and palette) set to save, the
//!   page is served by a small server on this computer only
//!   ([`crate::preview_server`]) and reloads itself after each save,
//!   landing on the heading nearest the caret; set to typing, also when
//!   typing pauses for `[preview] pane_delay_ms` (the preview pane's pause).
//! - **Progress.** An export says it started; one that takes more than two
//!   seconds says "Still exporting to PDF, 2 seconds." and then every ten
//!   seconds, never more often.
//! - **Listen to the rendered text** reads, from the caret, what a reader of
//!   the exported document hears: no Markdown marks, without leaving edit
//!   mode. Citations follow `[reading] citations`, as in continuous
//!   reading: skipped, or said in words. The highlight follows in the
//!   source.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

pub(crate) use textweaver_convert::OutputFormat;
use textweaver_convert::{ConvertOptions, Converter, Job as ConvertJob, ReportFormat, Status};
use textweaver_core::{CharRange, Utterance};
use textweaver_formats::{Loader, MarkdownLoader, Source};
use textweaver_lexicon::args;
use textweaver_speech::ReadingGeneration;
use textweaver_store::PreviewFollow;

use crate::app::App;
use crate::authoring_state::{
    ExportDone, ExportKind, Job, Listening, Progress, ThemeFor, file_name,
};
use crate::command::Effect;

/// How much text "listen to the rendered text" plans at once, in chars.
const LISTEN_LIMIT: usize = 400_000;

/// `[preview] follow`'s value as the catalogs' selector: off, save, or
/// typing.
fn follow_name(follow: PreviewFollow) -> &'static str {
    match follow {
        PreviewFollow::Off => "off",
        PreviewFollow::Save => "save",
        PreviewFollow::Typing => "typing",
    }
}

/// The headings of an HTML page: each one's id and its text without tags.
fn page_headings(html: &str) -> Vec<(String, String)> {
    let ids = crate::preview_server::heading_ids(html);
    let lower = html.to_ascii_lowercase();
    let mut texts = Vec::new();
    let mut at = 0;
    while let Some(i) = lower[at..].find("<h") {
        let start = at + i;
        at = start + 2;
        let Some(level) = lower.as_bytes().get(start + 2).copied() else {
            break;
        };
        if !(b'1'..=b'6').contains(&level) {
            continue;
        }
        let close = format!("</h{}>", char::from(level));
        let Some(open_end) = lower[start..].find('>') else {
            break;
        };
        let body_start = start + open_end + 1;
        let Some(len) = lower[body_start..].find(&close) else {
            break;
        };
        texts.push(heading_key(&strip_tags(
            &html[body_start..body_start + len],
        )));
        at = body_start + len;
    }
    ids.into_iter().zip(texts).collect()
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Heading text compared loosely: letters and digits, lowercase.
fn heading_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Numbers the temporary folders of exports in this process.
static EXPORT_SEQ: AtomicU32 = AtomicU32::new(0);

/// What an export reads.
struct ExportSource {
    /// The file to convert.
    file: PathBuf,
    /// A temporary folder to delete afterwards.
    temp: Option<PathBuf>,
    /// The document's folder: exports go here, images resolve here.
    folder: PathBuf,
    /// The base name for exports.
    stem: String,
    /// A bibliography to use, when the live text cannot be found beside
    /// the document.
    bibliography: Option<PathBuf>,
}

/// A `file:` URL for a folder, ending in `/`.
fn folder_url(folder: &Path) -> String {
    let mut s = folder.display().to_string().replace('\\', "/");
    if !s.ends_with('/') {
        s.push('/');
    }
    let encoded: String = s
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_owned(),
            '#' => "%23".to_owned(),
            '%' => "%25".to_owned(),
            '?' => "%3F".to_owned(),
            c => c.to_string(),
        })
        .collect();
    if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

/// The reader's typography for an HTML page, read from the settings at
/// export time (the owner's choice: a page looks like the reader, and
/// themes keep colors only): `[reading_aids.font]` family, size, and
/// weight, `[reading_aids.spacing]`, and `[display] measure`, each
/// clamped to its range. `tw convert`, export, the batch, and the browser
/// preview all use it.
pub fn page_typography(settings: &textweaver_store::Settings) -> textweaver_render::Typography {
    let aids = &settings.reading_aids;
    let font = textweaver_aids::fonts::from_store(&aids.font).clamped();
    let spacing = textweaver_aids::TextSpacing::from(&aids.spacing).clamped();
    textweaver_render::Typography {
        font_family: font.css_font_family(),
        size_pt: font.size_pt,
        weight: font.weight,
        line_height: spacing.line_height,
        paragraph_spacing: spacing.paragraph_spacing,
        letter_spacing: spacing.letter_spacing,
        word_spacing: spacing.word_spacing,
        measure: settings.display.measure,
    }
}

/// `html` with `<base href="…">` after `<head>`, so relative images and
/// links in a page written elsewhere resolve beside the document.
pub(crate) fn with_base(html: &str, href: &str) -> String {
    let lower = html.to_ascii_lowercase();
    if lower.contains("<base ") {
        return html.to_owned();
    }
    match lower.find("<head>") {
        Some(i) => {
            let at = i + "<head>".len();
            format!("{}\n<base href=\"{href}\">{}", &html[..at], &html[at..])
        }
        None => html.to_owned(),
    }
}

/// The bibliography file named in Markdown front matter
/// (`bibliography: refs.bib`), relative to `folder`.
fn front_matter_bibliography(text: &str, folder: &Path) -> Option<PathBuf> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    rest[..end].lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == "bibliography").then(|| {
            let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
            folder.join(v)
        })
    })
}

/// `[braille] table_format` for the BRF writer.
pub(crate) fn braille_tables(
    settings: &textweaver_store::Settings,
) -> textweaver_convert::BrailleTableFormat {
    use textweaver_convert::BrailleTableFormat as Out;
    use textweaver_store::BrailleTableFormat as In;
    match settings.braille.table_format {
        In::Linear => Out::Linear,
        In::Listed => Out::Listed,
        In::Stairstep => Out::Stairstep,
    }
}

impl App {
    /// The document's folder and base name.
    fn doc_place(&self) -> (Option<PathBuf>, PathBuf, String) {
        let path = self
            .edit
            .as_ref()
            .and_then(|e| e.session.doc().path.clone())
            .or_else(|| self.session.as_ref().and_then(|s| s.doc.meta.path.clone()));
        let folder = path
            .as_ref()
            .and_then(|p| p.parent().map(Path::to_owned))
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| self.loose_folder());
        let stem = path
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| {
                let title = self
                    .session
                    .as_ref()
                    .map_or_else(|| "untitled".to_owned(), |s| s.title.clone());
                let slug = textweaver_render::slug::slugify(&title);
                if slug.is_empty() {
                    "untitled".to_owned()
                } else {
                    slug
                }
            });
        (path, folder, stem)
    }

    /// What to export: the file itself while reading it, else the live
    /// text written to a temporary file.
    fn export_source(&self) -> Result<ExportSource, String> {
        let s = self
            .session
            .as_ref()
            .ok_or_else(|| self.msg("common-no-document"))?;
        let (path, folder, stem) = self.doc_place();
        let editing = self.edit.is_some();
        if !editing && let Some(p) = path.as_ref().filter(|p| p.is_file()) {
            return Ok(ExportSource {
                file: p.clone(),
                temp: None,
                folder,
                stem,
                bibliography: None,
            });
        }
        let text = match self.edit.as_ref().and_then(|e| e.session.editor()) {
            Some(ed) => ed.text().to_string(),
            None => textweaver_formats::to_markdown(&s.doc),
        };
        let cache = self
            .paths
            .as_ref()
            .map_or_else(std::env::temp_dir, |p| p.cache_dir.clone());
        let n = EXPORT_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = cache
            .join("export")
            .join(format!("{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| {
            self.msg_args(
                "publish-cannot-write-to",
                &args!["path" => dir.display().to_string(), "error" => e.to_string()],
            )
        })?;
        let file = dir.join(format!("{stem}.md"));
        std::fs::write(&file, &text).map_err(|e| {
            self.msg_args(
                "publish-cannot-write",
                &args!["path" => file.display().to_string(), "error" => e.to_string()],
            )
        })?;
        let bibliography = front_matter_bibliography(&text, &folder)
            .filter(|p| p.is_file())
            .or_else(|| {
                let p = textweaver_cite::folder_library_path(&folder);
                p.is_file().then_some(p)
            });
        Ok(ExportSource {
            file,
            temp: Some(dir),
            folder,
            stem,
            bibliography,
        })
    }

    /// Conversion settings for `to` from `src`.
    fn convert_options(&self, to: OutputFormat, src: &ExportSource) -> ConvertOptions {
        let mut o = ConvertOptions {
            to,
            force: true,
            jobs: Some(1),
            pandoc: false,
            load: self.load_options(),
            ..ConvertOptions::default()
        };
        o.write.resource_dir = Some(src.folder.clone());
        o.write.braille.math_code = match self.settings.braille.math_code {
            textweaver_store::MathBrailleCode::Nemeth => textweaver_convert::MathCode::Nemeth,
            textweaver_store::MathBrailleCode::Ueb => textweaver_convert::MathCode::Ueb,
        };
        o.write.braille.table_format = braille_tables(&self.settings);
        // The reading font, a downloaded Lexend too, as `tw convert
        // --font` gives it (W8a); a font missing here keeps the defaults.
        let fonts = self.export_fonts();
        o.write.pdf.font_family = fonts.pdf;
        o.write.epub.font = fonts.epub;
        o.citations.user_library = self
            .paths
            .as_ref()
            .map(|p| textweaver_cite::user_library_path(&p.data_dir));
        o.citations.bibliography = src.bibliography.clone();
        if to == OutputFormat::Html {
            o.theme_css = Some(self.html_theme_css());
            o.typography = Some(page_typography(&self.settings));
        }
        o
    }

    /// Converts on another thread; the result arrives in [`App::tick`].
    fn spawn_export(
        &mut self,
        what: String,
        kind: ExportKind,
        src: ExportSource,
        options: ConvertOptions,
        output: PathBuf,
        base: Option<String>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel::<Result<ExportDone, String>>();
        let job = ConvertJob {
            source: src.file.clone(),
            output: output.clone(),
            root: src.folder.clone(),
        };
        let temp = src.temp.clone();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("tw-export".into())
            .spawn(move || {
                let result = (|| {
                    if let Some(dir) = output.parent() {
                        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    }
                    // Previews are rewritten on every save: no audit.
                    let options = ConvertOptions {
                        audit: kind == ExportKind::Export,
                        ..options
                    };
                    let conv = Converter::new(options).map_err(|e| e.to_string())?;
                    let mut r = conv.convert_job(&job);
                    // An export gets its conversion report beside it; a
                    // preview, in the cache folder, does not.
                    let report = (kind == ExportKind::Export
                        && matches!(r.status, Status::Converted))
                    .then(|| {
                        textweaver_convert::report::write_file_report(
                            &r,
                            Some(conv.options().to),
                            ReportFormat::Markdown,
                            std::time::SystemTime::now(),
                        )
                    });
                    let issues = r.issues.len() + r.issues_omitted;
                    match std::mem::replace(&mut r.status, Status::Skipped) {
                        Status::Failed(reason) => Err(reason),
                        Status::Converted | Status::Skipped => {
                            let mut headings = Vec::new();
                            if let Some(href) = base {
                                let html =
                                    std::fs::read_to_string(&output).map_err(|e| e.to_string())?;
                                headings = page_headings(&html);
                                textweaver_convert::write_atomic(
                                    &output,
                                    with_base(&html, &href).as_bytes(),
                                )
                                .map_err(|e| e.to_string())?;
                            }
                            Ok(ExportDone {
                                path: output,
                                warnings: r.warnings,
                                headings,
                                report,
                                issues,
                            })
                        }
                    }
                })();
                if let Some(dir) = temp {
                    let _ = std::fs::remove_dir_all(dir);
                }
                let _ = tx.send(result);
                wake.wake();
            });
        match spawned {
            Ok(_) => self.authoring.jobs.push(Job::Export {
                what,
                kind,
                rx,
                progress: Progress::new(std::time::Instant::now()),
            }),
            Err(e) => {
                let msg = self.msg_args("publish-start-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
    }

    /// The theme names for "Theme for the HTML page?": this session's last
    /// answer, else the reading theme, first; then every other theme in
    /// cycle order.
    pub(crate) fn html_theme_names(&self) -> Vec<String> {
        let first = self
            .authoring
            .html_theme
            .as_deref()
            .and_then(|n| self.themes.get(n))
            .unwrap_or_else(|| self.current_theme())
            .meta
            .name
            .clone();
        let mut names = vec![first.clone()];
        names.extend(
            self.themes
                .names()
                .into_iter()
                .filter(|n| *n != first)
                .map(str::to_owned),
        );
        names
    }

    /// Asks "Theme for the HTML page?" with the themes, the first selected.
    /// Escape cancels.
    pub(crate) fn ask_html_theme(&mut self, purpose: ThemeFor) -> Vec<Effect> {
        let names = self.html_theme_names();
        let items: Vec<String> = names
            .iter()
            .filter_map(|n| self.themes.get(n))
            .map(|t| t.meta.display_name.clone())
            .collect();
        let first = items.first().cloned().unwrap_or_default();
        let msg = self.msg_args(
            "publish-theme-intro",
            &args!["n" => items.len(), "first" => first],
        );
        self.list = Some(crate::app::ListKind::HtmlTheme(purpose, names));
        self.tell(&msg);
        vec![Effect::ShowList {
            title: self.msg("publish-theme-title"),
            items,
        }]
    }

    /// Enter in the theme list: remembered for the session, then the
    /// export, preview, or batch goes on.
    pub(crate) fn choose_html_theme(
        &mut self,
        purpose: ThemeFor,
        names: &[String],
        n: usize,
    ) -> Vec<Effect> {
        let Some(name) = names.get(n) else {
            return vec![Effect::Redraw];
        };
        self.authoring.html_theme = Some(name.clone());
        match purpose {
            ThemeFor::Export => self.export_now(OutputFormat::Html),
            ThemeFor::Preview => self.preview_now(),
            ThemeFor::Batch => self.batch_theme_chosen(),
        }
    }

    /// The CSS for an HTML page: the theme chosen this session, else the
    /// reading theme; the reading theme keeps the reader's highlight and
    /// `[colors]` settings.
    pub(crate) fn html_theme_css(&self) -> String {
        let current = self.current_theme().meta.name.clone();
        let chosen = self
            .authoring
            .html_theme
            .as_deref()
            .and_then(|n| self.themes.get(n))
            .filter(|t| t.meta.name != current);
        match chosen {
            Some(t) => textweaver_theme::css::single_stylesheet(t),
            None => textweaver_theme::css::single_stylesheet(&self.reading_theme()),
        }
    }

    /// Exports the document to `to`, next to it; HTML asks for the theme
    /// first.
    pub(crate) fn export_to(&mut self, to: OutputFormat) -> Vec<Effect> {
        if self.session.is_none() {
            let open = self.key(textweaver_keymap::ActionId::Open);
            let msg = self.msg_args("app-no-document-open", &args!["key" => open]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if to == OutputFormat::Html {
            return self.ask_html_theme(ThemeFor::Export);
        }
        self.export_now(to)
    }

    /// Exports the document to `to`, next to it, without asking.
    fn export_now(&mut self, to: OutputFormat) -> Vec<Effect> {
        let src = match self.export_source() {
            Ok(s) => s,
            Err(e) => {
                let msg = self.msg_args("publish-export-error", &args!["error" => e]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
        };
        let mut output = src.folder.join(format!("{}.{}", src.stem, to.extension()));
        if output == src.file {
            output = src
                .folder
                .join(format!("{}-export.{}", src.stem, to.extension()));
        }
        let options = self.convert_options(to, &src);
        let label = to.label();
        let msg = self.msg_args("publish-exporting", &args!["format" => label]);
        self.tell(&msg);
        self.spawn_export(
            label.to_owned(),
            ExportKind::Export,
            src,
            options,
            output,
            None,
        );
        vec![Effect::Redraw]
    }

    /// Writes the HTML preview and opens it in the browser (through the
    /// reload server when `[preview] follow` is not off).
    pub(crate) fn preview_in_browser(&mut self) -> Vec<Effect> {
        if self.session.is_none() {
            let open = self.key(textweaver_keymap::ActionId::Open);
            let msg = self.msg_args("app-no-document-open", &args!["key" => open]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.ask_html_theme(ThemeFor::Preview)
    }

    /// Writes the preview and opens it, the theme chosen. The first time in
    /// a session it says what the browser will do, from `[preview] follow`.
    fn preview_now(&mut self) -> Vec<Effect> {
        let msg = if std::mem::replace(&mut self.authoring.preview_explained, true) {
            self.msg("publish-writing-preview")
        } else {
            let follow = follow_name(self.settings.preview.follow);
            self.msg_args("publish-preview-first", &args!["follow" => follow])
        };
        self.tell(&msg);
        self.write_preview(ExportKind::PreviewOpen);
        vec![Effect::Redraw]
    }

    fn write_preview(&mut self, kind: ExportKind) {
        let src = match self.export_source() {
            Ok(s) => s,
            Err(e) => {
                let msg = self.msg_args("publish-preview-error", &args!["error" => e]);
                self.error(&msg);
                return;
            }
        };
        let cache = self
            .paths
            .as_ref()
            .map_or_else(std::env::temp_dir, |p| p.cache_dir.clone());
        let output = cache.join("preview").join(format!("{}.html", src.stem));
        let options = self.convert_options(OutputFormat::Html, &src);
        let base = Some(folder_url(&src.folder));
        self.authoring.preview = Some(output.clone());
        self.authoring.preview_version = self.authoring.structure.version;
        if let Some(server) = &self.preview_server {
            server.set_page(output.clone(), src.folder.clone());
        }
        self.authoring.preview_folder = Some(src.folder.clone());
        self.spawn_export(String::new(), kind, src, options, output, base);
    }

    /// Says how a long export is getting on (at most every ten seconds).
    pub(crate) fn export_progress(&mut self, kind: ExportKind, what: &str, secs: u64) {
        // `what` is the export's format label ("PDF").
        let msg = match kind {
            ExportKind::Export => self.msg_args(
                "publish-still-exporting",
                &args!["format" => what, "secs" => secs],
            ),
            ExportKind::PreviewOpen => {
                self.msg_args("publish-still-previewing", &args!["secs" => secs])
            }
            // Rewriting the preview is quiet.
            ExportKind::PreviewRefresh | ExportKind::PreviewLive => return,
        };
        self.note(&msg);
    }

    /// Stops the preview's reload server, if one runs, and forgets the
    /// preview: the document closed or textweaver is quitting.
    pub(crate) fn close_preview(&mut self) {
        if let Some(mut server) = self.preview_server.take() {
            server.stop();
        }
        self.authoring.preview = None;
    }

    /// The id of the preview heading nearest the caret: the last heading at
    /// or before it, matched by its text (and by its order among headings
    /// with the same text).
    fn preview_anchor(&self, headings: &[(String, String)]) -> Option<String> {
        let s = self.session.as_ref()?;
        let caret = s.cursor;
        let doc_headings: Vec<String> = s
            .doc
            .markers()
            .iter()
            .filter(|m| m.kind == textweaver_core::MarkerKind::Heading && m.range.start <= caret)
            .map(|m| heading_key(s.doc.slice(m.range).trim_start_matches('#')))
            .collect();
        let last = doc_headings.last()?;
        let nth = doc_headings.iter().filter(|h| *h == last).count();
        headings
            .iter()
            .filter(|(_, text)| text == last)
            .nth(nth.saturating_sub(1))
            .or_else(|| headings.get(doc_headings.len().saturating_sub(1)))
            .map(|(id, _)| id.clone())
            .filter(|id| !id.is_empty())
    }

    /// `cycle_preview_follow`, "Browser preview follows": off, each save,
    /// your typing, then off again. Saved.
    pub(crate) fn cycle_preview_follow(&mut self) {
        let follow = match self.settings.preview.follow {
            PreviewFollow::Off => PreviewFollow::Save,
            PreviewFollow::Save => PreviewFollow::Typing,
            PreviewFollow::Typing => PreviewFollow::Off,
        };
        self.settings.preview.follow = follow;
        self.settings_dirty = true;
        if follow == PreviewFollow::Off {
            if let Some(mut server) = self.preview_server.take() {
                server.stop();
            }
            let msg = self.msg("publish-follow-off");
            self.tell(&msg);
            return;
        }
        // A preview opened as a plain file reloads by itself only once it
        // is opened again, through the server.
        let again = if self.authoring.preview.is_some() && self.preview_server.is_none() {
            "yes"
        } else {
            "no"
        };
        let key = match follow {
            PreviewFollow::Typing => "publish-follow-typing",
            _ => "publish-follow-save",
        };
        let msg = self.msg_args(key, &args!["again" => again]);
        self.tell(&msg);
    }

    /// `[preview] follow` set to typing: rewrites the preview once typing
    /// has paused for `[preview] pane_delay_ms`.
    pub(crate) fn live_preview_tick(&mut self, now: std::time::Instant) {
        let p = &self.settings.preview;
        if p.follow != PreviewFollow::Typing || self.preview_server.is_none() || self.edit.is_none()
        {
            return;
        }
        let pause = std::time::Duration::from_millis(u64::from(p.pane_delay_ms));
        let st = &self.authoring.structure;
        let changed = st.version != self.authoring.preview_version;
        let paused = st
            .last_edit
            .is_some_and(|t| now.saturating_duration_since(t) >= pause);
        let busy = self.authoring.jobs.iter().any(|j| {
            matches!(
                j,
                Job::Export {
                    kind: ExportKind::PreviewLive | ExportKind::PreviewRefresh,
                    ..
                }
            )
        });
        if changed && paused && !busy {
            self.write_preview(ExportKind::PreviewLive);
        }
    }

    /// An export or preview finished on its thread.
    pub(crate) fn export_finished(
        &mut self,
        what: &str,
        kind: ExportKind,
        result: Result<ExportDone, String>,
    ) {
        let ExportDone {
            path,
            warnings,
            headings,
            report,
            issues,
        } = match result {
            Ok(x) => x,
            Err(e) => {
                self.speech.earcon(textweaver_speech::Earcon::Error);
                // `what` is the export's format label ("PDF"), empty for a
                // preview.
                let msg = if kind == ExportKind::Export {
                    self.msg_args(
                        "publish-export-failed",
                        &args!["format" => what, "error" => e],
                    )
                } else {
                    self.msg_args("publish-preview-failed", &args!["error" => e])
                };
                self.error(&msg);
                return;
            }
        };
        // A sentence of its own, with a space before it, or nothing.
        let warned = match warnings.first() {
            None => String::new(),
            Some(first) => format!(
                " {}",
                self.msg_args(
                    "publish-warnings",
                    &args!["n" => warnings.len(), "first" => first.as_str()],
                )
            ),
        };
        match kind {
            ExportKind::Export => {
                // What could not be made accessible, and where the report
                // is, in words, after the warnings.
                let mut warned = warned;
                if issues > 0 {
                    warned.push(' ');
                    warned.push_str(&self.msg_args("publish-report-issues", &args!["n" => issues]));
                }
                match &report {
                    Some(Ok(p)) => {
                        warned.push(' ');
                        warned.push_str(
                            &self.msg_args("publish-report", &args!["file" => file_name(p)]),
                        );
                    }
                    Some(Err(e)) => {
                        warned.push(' ');
                        warned.push_str(
                            &self.msg_args("publish-report-failed", &args!["error" => e.clone()]),
                        );
                    }
                    None => {}
                }
                let folder = path
                    .parent()
                    .map_or_else(String::new, |p| p.display().to_string());
                let target = path.display().to_string();
                let question = self.msg_args(
                    "publish-exported",
                    &args![
                        "format" => what,
                        "file" => file_name(&path),
                        "folder" => folder,
                        "warned" => warned
                    ],
                );
                self.offer_open(target, &question);
            }
            ExportKind::PreviewOpen => {
                let served = if self.settings.preview.follow != PreviewFollow::Off {
                    self.serve_preview(&path)
                } else {
                    None
                };
                match served {
                    Some(url) => {
                        let msg = self
                            .msg_args("publish-preview-written-served", &args!["warned" => warned]);
                        self.tell(&msg);
                        self.launch(&url);
                    }
                    None => {
                        let msg =
                            self.msg_args("publish-preview-written", &args!["warned" => warned]);
                        self.tell(&msg);
                        let target = path.display().to_string();
                        self.launch(&target);
                    }
                }
            }
            ExportKind::PreviewRefresh | ExportKind::PreviewLive => {
                let anchor = self.preview_anchor(&headings);
                match &self.preview_server {
                    Some(server) => {
                        server.reload(anchor.as_deref());
                        let msg = self.msg("publish-preview-updated");
                        if kind == ExportKind::PreviewRefresh {
                            self.tell(&msg);
                        } else {
                            self.show(&msg);
                        }
                    }
                    None => {
                        let msg = self.msg("publish-preview-updated-press-f5");
                        self.tell(&msg);
                    }
                }
            }
        }
    }

    /// Starts the reload server for the preview at `page` (or points the
    /// running one at it); its address, or `None` when it cannot start.
    fn serve_preview(&mut self, page: &Path) -> Option<String> {
        let folder = self
            .authoring
            .preview_folder
            .clone()
            .unwrap_or_else(|| self.loose_folder());
        if let Some(server) = &self.preview_server {
            server.set_page(page.to_owned(), folder);
            return Some(server.url());
        }
        match crate::preview_server::PreviewServer::start(page.to_owned(), folder) {
            Ok(server) => {
                server.set_catalog(self.catalog());
                let url = server.url();
                self.preview_server = Some(server);
                Some(url)
            }
            Err(e) => {
                let msg = self.msg_args("publish-server-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                None
            }
        }
    }

    /// After a successful save in edit mode: rewrites the preview and says
    /// how many possible misspellings the document has.
    pub(crate) fn on_saved(&mut self) {
        if self.authoring.preview.is_some() {
            self.write_preview(ExportKind::PreviewRefresh);
        }
        // The count follows on a helper thread (Wave 3).
        self.count_misspellings_in_background();
    }

    /// Reads the document as it will render, from the caret, without
    /// leaving edit mode. Outside edit mode this is ordinary reading.
    pub(crate) fn listen_rendered(&mut self) -> Vec<Effect> {
        if self.edit.is_none() {
            self.stop_speech();
            self.read_from_cursor();
            return vec![Effect::Redraw];
        }
        self.refresh_structure(true);
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let text = s.doc.text().to_string();
        let caret = s.cursor;
        let loaded = MarkdownLoader.load(
            &Source::Bytes {
                data: text.into_bytes(),
                hint: "md".into(),
            },
            &self.load_options(),
        );
        let rendered = match loaded {
            Ok(d) => d,
            Err(e) => {
                let msg = self.msg_args("publish-render-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
        };
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let map = crate::structure::SourceMap::build(&rendered, &s.doc);
        let start = if map.is_empty() {
            textweaver_core::CharPos::ZERO
        } else {
            map.to_canonical(&rendered, &s.doc, caret)
        };
        let start = crate::text_util::word_start(&rendered, start);
        // Without textweaver's voice (screen-reader mode), the rendered
        // paragraph goes on the status line for the screen reader instead.
        if !self.route(textweaver_a11y::Channel::Reading).speak {
            let para =
                textweaver_text::units::unit_at(&rendered, start, textweaver_core::Unit::Paragraph)
                    .unwrap_or_else(|| CharRange::new(start, rendered.end()));
            let text = crate::lists::one_line(
                &rendered.slice(CharRange::new(start, para.end.max(start))),
                crate::access::STATUS_TEXT_LIMIT,
            );
            let text = self.screen_text(&text);
            self.show_as(textweaver_a11y::Channel::Reading, &text);
            return vec![Effect::Redraw];
        }
        let end = (start.0 + LISTEN_LIMIT).min(rendered.len_chars());
        let range = CharRange::new(start, end);
        let policy = self.narration_policy();
        let citations = self.citation_speech_of(&rendered, range);
        let utterances: Vec<Utterance> =
            textweaver_text::plan_with(&rendered, range, &policy, &citations);
        let Some(generation) = self.read_planned(utterances) else {
            let msg = self.msg("publish-nothing-after-caret");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        self.authoring.listening = Some(Listening {
            generation,
            rendered,
            map,
        });
        let msg = self.msg("publish-listening");
        self.show(&msg);
        vec![Effect::Redraw]
    }

    /// A spoken range of the reading `generation`, in the edited source
    /// when it is a "listen to the rendered text" reading.
    pub(crate) fn map_listened(&self, generation: ReadingGeneration, r: CharRange) -> CharRange {
        let (Some(l), Some(s)) = (self.authoring.listening.as_ref(), self.session.as_ref()) else {
            return r;
        };
        if l.generation != generation || l.map.is_empty() {
            return r;
        }
        let a = l.map.to_source(&l.rendered, &s.doc, r.start);
        // The end is after the range's last char, not before the next one
        // (which may come after markup: `bold**`).
        let b = match r.end.0.checked_sub(1).filter(|&e| e >= r.start.0) {
            Some(last) => l
                .map
                .to_source(&l.rendered, &s.doc, textweaver_core::CharPos(last))
                .saturating_add(1),
            None => a,
        };
        CharRange::new(a, b.max(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_pages_get_a_base() {
        let html = "<!doctype html><html><head><title>x</title></head><body></body></html>";
        let out = with_base(html, "file:///D:/notes/");
        assert!(out.contains("<head>\n<base href=\"file:///D:/notes/\"><title>"));
        assert_eq!(with_base(&out, "file:///elsewhere/"), out);
        assert_eq!(
            folder_url(Path::new("/home/ada/my notes")),
            "file:///home/ada/my%20notes/"
        );
        assert_eq!(folder_url(Path::new("D:\\notes")), "file:///D:/notes/");
    }

    /// The reading font reaches the PDF and EPUB writers' options (W8a), a
    /// downloaded Lexend too; an installed font only the PDF's; a generic
    /// family or a font missing here leaves the writers' defaults.
    #[test]
    fn export_uses_the_reading_font_when_it_is_here() {
        let home = tempfile::tempdir().unwrap();
        let paths = textweaver_store::Paths::under(home.path());
        let mut app = App::new(crate::AppConfig {
            paths: Some(paths.clone()),
            ..crate::AppConfig::for_tests()
        });
        let installed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = installed.clone();
        app.set_installed_fonts_check(std::sync::Arc::new(move |name: &str| {
            flag.load(std::sync::atomic::Ordering::SeqCst) && name == "Lexend"
        }));
        let src = ExportSource {
            file: home.path().join("a.md"),
            temp: None,
            folder: home.path().to_owned(),
            stem: "a".into(),
            bibliography: None,
        };
        let fonts = |app: &App| {
            let o = app.convert_options(OutputFormat::Pdf, &src);
            (o.write.pdf.font_family, o.write.epub.font)
        };
        // A generic family: the writers' defaults.
        app.settings.reading_aids.font.family = "sans".into();
        assert_eq!(fonts(&app), (None, None));
        // Lexend, neither downloaded nor installed: the defaults, no error.
        app.settings.reading_aids.font.family = "lexend".into();
        assert_eq!(fonts(&app), (None, None));
        // Installed on this computer: the PDF's font only.
        installed.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(fonts(&app), (Some("Lexend".into()), None));
        installed.store(false, std::sync::atomic::Ordering::SeqCst);
        // Downloaded into the data folder: both, by its key.
        let dir = textweaver_fonts::downloaded::LEXEND.dir_in(&crate::fonts_folder(&paths));
        std::fs::create_dir_all(&dir).unwrap();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/w7l");
        for f in textweaver_fonts::downloaded::LEXEND.files {
            std::fs::copy(fixtures.join(f.file_name), dir.join(f.file_name)).unwrap();
        }
        assert_eq!(fonts(&app), (Some("lexend".into()), Some("lexend".into())));
        // A bundled reading font, when this build bundles fonts.
        app.settings.reading_aids.font.family = "opendyslexic".into();
        match textweaver_fonts::bundled::family("opendyslexic") {
            Some(b) => assert_eq!(fonts(&app), (Some(b.name.into()), Some(b.name.into()))),
            None => assert_eq!(fonts(&app), (None, None)),
        }
    }

    /// Item 4 of Agent P2e: an export says it started (the command's own
    /// "Exporting to PDF."), then, when it takes long, how long so far:
    /// after 2 seconds, then every 10, never more often.
    #[test]
    fn long_exports_say_progress_without_flooding() {
        use std::time::{Duration, Instant};
        let mut app = App::new(crate::AppConfig::for_tests());
        let (tx, rx) = std::sync::mpsc::channel();
        let t0 = Instant::now();
        app.authoring.jobs.push(Job::Export {
            what: "PDF".into(),
            kind: ExportKind::Export,
            rx,
            progress: Progress::new(t0),
        });
        app.authoring_tick(t0 + Duration::from_millis(1500));
        assert!(
            !app.status_text().starts_with("Still"),
            "{}",
            app.status_text()
        );
        app.authoring_tick(t0 + Duration::from_millis(2100));
        assert_eq!(app.status_text(), "Still exporting to PDF, 2 seconds.");
        app.tell("Something else.");
        for ms in [2500, 5000, 9000, 12000] {
            app.authoring_tick(t0 + Duration::from_millis(ms));
            assert_eq!(app.status_text(), "Something else.", "at {ms} ms");
        }
        app.authoring_tick(t0 + Duration::from_millis(12200));
        assert_eq!(app.status_text(), "Still exporting to PDF, 12 seconds.");
        // A preview being rewritten stays quiet.
        let (_tx2, rx2) = std::sync::mpsc::channel();
        app.authoring.jobs.push(Job::Export {
            what: "Preview".into(),
            kind: ExportKind::PreviewRefresh,
            rx: rx2,
            progress: Progress::new(t0),
        });
        app.tell("Quiet.");
        app.authoring_tick(t0 + Duration::from_millis(30000));
        assert!(app.status_text().starts_with("Still exporting to PDF, 30"));
        drop(tx);
    }

    /// Export to HTML asks "Theme for the HTML page?": the reading theme
    /// first; Escape cancels with nothing written; the chosen theme's CSS is
    /// in the page, and the answer comes first next time this session.
    #[test]
    fn html_export_asks_for_the_theme() {
        use crate::command::Command;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("essay.md");
        std::fs::write(&file, "# Essay\n\nText.\n").unwrap();
        let mut config = crate::AppConfig::for_tests();
        config.settings.display.theme = "nord".into();
        let mut app = App::new(config);
        app.dispatch(Command::Open(file));
        let ask = |app: &mut App| {
            let effects = app.dispatch(Command::Action(textweaver_keymap::ActionId::ExportHtml));
            effects
                .iter()
                .find_map(|e| match e {
                    Effect::ShowList { title, items } => Some((title.clone(), items.clone())),
                    _ => None,
                })
                .expect("a list")
        };
        let (title, items) = ask(&mut app);
        assert_eq!(title, "Theme for the HTML page");
        assert_eq!(items[0], "Nord");
        assert!(
            app.status_text().starts_with(&format!(
                "Theme for the HTML page? Nord first, {} choices.",
                items.len()
            )),
            "{}",
            app.status_text()
        );
        // Escape: nothing is exported.
        app.dispatch(Command::Cancel);
        assert!(app.authoring.jobs.is_empty());
        let out = dir.path().join("essay.html");
        assert!(!out.exists());
        // Sepia is chosen: its colors are the page's.
        let (_, items) = ask(&mut app);
        let n = items.iter().position(|i| i == "Sepia").unwrap();
        app.dispatch(Command::Choose(n));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !app.authoring.jobs.is_empty() && std::time::Instant::now() < deadline {
            let _ = app.tick(std::time::Instant::now());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let page = std::fs::read_to_string(&out).unwrap();
        assert!(page.contains("/* textweaver theme: Sepia */"));
        assert!(!page.contains("textweaver theme: Nord"));
        // Remembered for the session: Sepia first now.
        app.dispatch(Command::Confirm(crate::command::Confirm::No));
        let (_, items) = ask(&mut app);
        assert_eq!(items[0], "Sepia");
        assert_eq!(items.iter().filter(|i| *i == "Nord").count(), 1);
    }

    #[test]
    fn page_headings_have_ids_and_texts() {
        let html =
            "<h1 id=\"intro\">Intro <em>here</em></h1><p>x</p><h2 id=\"methods-1\">Methods</h2>";
        assert_eq!(
            page_headings(html),
            vec![
                ("intro".to_owned(), "introhere".to_owned()),
                ("methods-1".to_owned(), "methods".to_owned())
            ]
        );
    }

    #[test]
    fn front_matter_names_a_bibliography() {
        let folder = Path::new("/docs");
        assert_eq!(
            front_matter_bibliography(
                "---\ntitle: X\nbibliography: \"refs.bib\"\n---\n\nText",
                folder
            ),
            Some(folder.join("refs.bib"))
        );
        assert_eq!(front_matter_bibliography("No front matter", folder), None);
    }
}
