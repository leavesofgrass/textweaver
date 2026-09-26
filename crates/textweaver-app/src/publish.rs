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
//!   and asks "Open it? y or n".
//! - **Preview in browser** writes the page, with math as MathML, to the
//!   `preview` folder of the cache folder and opens the default browser.
//!   Each save while editing writes it again, and textweaver says "Preview
//!   updated. Press F5 in the browser." With `[preview] auto_reload`
//!   (palette: `toggle preview auto reload`), the page is served by a small
//!   server on this computer only ([`crate::preview_server`]) and reloads
//!   itself after each save, landing on the heading nearest the caret; with
//!   `[preview] live` too, also when typing pauses for a second.
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

use textweaver_convert::{ConvertOptions, Converter, Job as ConvertJob, OutputFormat, Status};
use textweaver_core::{CharRange, Utterance};
use textweaver_formats::{Loader, MarkdownLoader, Source};
use textweaver_speech::ReadingGeneration;

use crate::app::App;
use crate::authoring_state::{ExportDone, ExportKind, Job, Listening, Progress, file_name};
use crate::command::Effect;

/// How much text "listen to the rendered text" plans at once, in chars.
const LISTEN_LIMIT: usize = 400_000;

/// How long typing must pause before a live preview reloads.
const LIVE_PAUSE: std::time::Duration = std::time::Duration::from_secs(1);

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

impl App {
    /// Where files made from a document that has no file yet go (exports,
    /// study sheets): the folder textweaver was started in, as Save As
    /// suggests; in a session that keeps no files (tests), the system's
    /// temporary folder.
    pub(crate) fn loose_folder(&self) -> PathBuf {
        if self.paths.is_some() {
            std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
        } else {
            std::env::temp_dir()
        }
    }

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
        let s = self.session.as_ref().ok_or("No document is open.")?;
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
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot write to {}: {e}", dir.display()))?;
        let file = dir.join(format!("{stem}.md"));
        std::fs::write(&file, &text)
            .map_err(|e| format!("cannot write {}: {e}", file.display()))?;
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
        o.citations.user_library = self
            .paths
            .as_ref()
            .map(|p| textweaver_cite::user_library_path(&p.data_dir));
        o.citations.bibliography = src.bibliography.clone();
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
        let spawned = std::thread::Builder::new()
            .name("tw-export".into())
            .spawn(move || {
                let result = (|| {
                    if let Some(dir) = output.parent() {
                        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    }
                    let conv = Converter::new(options).map_err(|e| e.to_string())?;
                    let r = conv.convert_job(&job);
                    match r.status {
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
                            })
                        }
                    }
                })();
                if let Some(dir) = temp {
                    let _ = std::fs::remove_dir_all(dir);
                }
                let _ = tx.send(result);
            });
        match spawned {
            Ok(_) => self.authoring.jobs.push(Job::Export {
                what,
                kind,
                rx,
                progress: Progress::new(std::time::Instant::now()),
            }),
            Err(e) => self.error(&format!("Could not start the export: {e}")),
        }
    }

    /// Exports the document to `to`, next to it.
    pub(crate) fn export_to(&mut self, to: OutputFormat) -> Vec<Effect> {
        if self.session.is_none() {
            self.tell("No document is open. Press Control O to open one.");
            return vec![Effect::Redraw];
        }
        let src = match self.export_source() {
            Ok(s) => s,
            Err(e) => {
                self.error(&format!("Could not export: {e}"));
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
        self.tell(&format!("Exporting to {label}."));
        self.spawn_export(
            format!("Exported to {label}"),
            ExportKind::Export,
            src,
            options,
            output,
            None,
        );
        vec![Effect::Redraw]
    }

    /// Writes the HTML preview and opens it in the browser (through the
    /// reload server with `[preview] auto_reload`).
    pub(crate) fn preview_in_browser(&mut self) -> Vec<Effect> {
        if self.session.is_none() {
            self.tell("No document is open. Press Control O to open one.");
            return vec![Effect::Redraw];
        }
        self.tell("Writing the preview.");
        self.write_preview(ExportKind::PreviewOpen);
        vec![Effect::Redraw]
    }

    fn write_preview(&mut self, kind: ExportKind) {
        let src = match self.export_source() {
            Ok(s) => s,
            Err(e) => {
                self.error(&format!("Could not write the preview: {e}"));
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
        self.spawn_export("Preview".into(), kind, src, options, output, base);
    }

    /// Says how a long export is getting on (at most every ten seconds).
    pub(crate) fn export_progress(&mut self, kind: ExportKind, what: &str, secs: u64) {
        let doing = match kind {
            ExportKind::Export => what.replacen("Exported", "exporting", 1),
            ExportKind::PreviewOpen => "writing the preview".to_owned(),
            // Rewriting the preview is quiet.
            ExportKind::PreviewRefresh | ExportKind::PreviewLive => return,
        };
        let unit = if secs == 1 { "second" } else { "seconds" };
        self.note(&format!("Still {doing}, {secs} {unit}."));
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

    /// `toggle_preview_auto_reload`: saved.
    pub(crate) fn toggle_preview_auto_reload(&mut self) {
        let on = !self.settings.preview.auto_reload;
        self.settings.preview.auto_reload = on;
        self.settings_dirty = true;
        if on {
            let more = if self.authoring.preview.is_some() {
                " Run preview in browser again to use it."
            } else {
                ""
            };
            self.tell(&format!(
                "Automatic preview reloading on: after each save the browser reloads the page by itself.{more}"
            ));
        } else {
            if let Some(mut server) = self.preview_server.take() {
                server.stop();
            }
            self.tell("Automatic preview reloading off: press F5 in the browser after a save.");
        }
    }

    /// `toggle_preview_live`: saved.
    pub(crate) fn toggle_preview_live(&mut self) {
        let on = !self.settings.preview.live;
        self.settings.preview.live = on;
        self.settings_dirty = true;
        self.tell(match (on, self.settings.preview.auto_reload) {
            (true, true) => "Live preview on: the preview also reloads when typing pauses.",
            (true, false) => {
                "Live preview on. It works with automatic reloading, which is off; turn it on with toggle preview auto reload."
            }
            (false, _) => "Live preview off: the preview reloads after saves only.",
        });
    }

    /// `[preview] live`: rewrites the preview a second after typing stops.
    pub(crate) fn live_preview_tick(&mut self, now: std::time::Instant) {
        let p = &self.settings.preview;
        if !(p.live && p.auto_reload) || self.preview_server.is_none() || self.edit.is_none() {
            return;
        }
        let st = &self.authoring.structure;
        let changed = st.version != self.authoring.preview_version;
        let paused = st
            .last_edit
            .is_some_and(|t| now.saturating_duration_since(t) >= LIVE_PAUSE);
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
        } = match result {
            Ok(x) => x,
            Err(e) => {
                self.speech.earcon(textweaver_speech::Earcon::Error);
                self.error(&format!("{what} failed: {e}"));
                return;
            }
        };
        let warned = match warnings.len() {
            0 => String::new(),
            1 => format!(" 1 warning: {}", warnings[0]),
            n => format!(" {n} warnings; the first: {}", warnings[0]),
        };
        match kind {
            ExportKind::Export => {
                let folder = path
                    .parent()
                    .map_or_else(String::new, |p| p.display().to_string());
                let target = path.display().to_string();
                self.offer_open(
                    target,
                    &format!(
                        "{what}: {} in {folder}.{warned} Open it? y or n.",
                        file_name(&path)
                    ),
                );
            }
            ExportKind::PreviewOpen => {
                let served = if self.settings.preview.auto_reload {
                    self.serve_preview(&path)
                } else {
                    None
                };
                match served {
                    Some(url) => {
                        self.tell(&format!(
                            "Preview written. Opening it in the browser. It reloads by itself after each save.{warned}"
                        ));
                        self.launch(&url);
                    }
                    None => {
                        self.tell(&format!(
                            "Preview written. Opening it in the browser. Saving writes it again; then press F5 in the browser.{warned}"
                        ));
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
                        if kind == ExportKind::PreviewRefresh {
                            self.tell("Preview updated.");
                        } else {
                            self.show("Preview updated.");
                        }
                    }
                    None => self.tell("Preview updated. Press F5 in the browser."),
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
                let url = server.url();
                self.preview_server = Some(server);
                Some(url)
            }
            Err(e) => {
                self.error(&format!(
                    "Could not start the preview's reload server ({e}); opening the file instead."
                ));
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
        if let Some(summary) = self.misspelling_summary() {
            // "No misspellings." only at high verbosity: silence means none.
            let quiet = summary.starts_with("No ")
                && self.settings.speech.verbosity < textweaver_a11y::Verbosity::High;
            if !quiet {
                self.announce_queued(&summary, textweaver_a11y::Priority::Polite);
            }
        }
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
                self.error(&format!("Could not render the text: {e}"));
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
            self.tell("Nothing to read after the caret.");
            return vec![Effect::Redraw];
        };
        self.authoring.listening = Some(Listening {
            generation,
            rendered,
            map,
        });
        self.show("Listening to the rendered text.");
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
            folder_url(Path::new("/home/jon/my notes")),
            "file:///home/jon/my%20notes/"
        );
        assert_eq!(folder_url(Path::new("D:\\notes")), "file:///D:/notes/");
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
            what: "Exported to PDF".into(),
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
