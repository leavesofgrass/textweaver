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
//!   Each save while editing writes it again; refresh the browser (F5) to
//!   see the change.
//! - **Listen to the rendered text** reads, from the caret, what a reader of
//!   the exported document hears: no Markdown marks, citations formatted,
//!   without leaving edit mode. The highlight follows in the source.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use textweaver_convert::{ConvertOptions, Converter, Job as ConvertJob, OutputFormat, Status};
use textweaver_core::{CharRange, Utterance};
use textweaver_formats::{Loader, MarkdownLoader, Source};
use textweaver_speech::ReadingGeneration;

use crate::app::App;
use crate::authoring_state::{ExportKind, Job, Listening, file_name};
use crate::command::Effect;

/// How much text "listen to the rendered text" plans at once, in chars.
const LISTEN_LIMIT: usize = 400_000;

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
        let (tx, rx) = std::sync::mpsc::channel();
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
                            if let Some(href) = base {
                                let html =
                                    std::fs::read_to_string(&output).map_err(|e| e.to_string())?;
                                textweaver_convert::write_atomic(
                                    &output,
                                    with_base(&html, &href).as_bytes(),
                                )
                                .map_err(|e| e.to_string())?;
                            }
                            Ok((output, r.warnings))
                        }
                    }
                })();
                if let Some(dir) = temp {
                    let _ = std::fs::remove_dir_all(dir);
                }
                let _ = tx.send(result);
            });
        match spawned {
            Ok(_) => self.authoring.jobs.push(Job::Export { what, kind, rx }),
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

    /// Writes the HTML preview and opens it in the browser.
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
        self.spawn_export("Preview".into(), kind, src, options, output, base);
    }

    /// An export or preview finished on its thread.
    pub(crate) fn export_finished(
        &mut self,
        what: &str,
        kind: ExportKind,
        result: Result<(PathBuf, Vec<String>), String>,
    ) {
        let (path, warnings) = match result {
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
                self.tell(&format!(
                    "Preview written. Opening it in the browser. Saving writes it again; refresh the browser to see it.{warned}"
                ));
                let target = path.display().to_string();
                self.launch(&target);
            }
            ExportKind::PreviewRefresh => {
                self.note("Preview updated.");
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
        let rendered_md = self.with_formatted_citations(&text);
        let loaded = MarkdownLoader.load(
            &Source::Bytes {
                data: rendered_md.into_bytes(),
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
        let end = (start.0 + LISTEN_LIMIT).min(rendered.len_chars());
        let policy = self.narration_policy();
        let utterances: Vec<Utterance> =
            textweaver_text::plan(&rendered, CharRange::new(start, end), &policy);
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
