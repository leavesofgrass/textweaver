//! Open with the system's own file chooser (the owner's session 2,
//! ADR-0033), and choose the file for exporting or importing settings
//! (W6a6: a save dialog for export, an open dialog for import, both for
//! TOML and JSON): the common item dialog (`IFileOpenDialog`) on Windows, which
//! NVDA and JAWS know; the XDG desktop portal on Linux, over D-Bus loaded
//! at run time (no GTK); the open panel on macOS. Through the `rfd` crate.
//!
//! Every prompt for a path has one (W8a-f): Save As (a save dialog with
//! the document's name in its folder), Insert Image, Import References,
//! and profile import and export, from the app's
//! [`PathPromptSpec`](textweaver_app::path_prompt::PathPromptSpec); and the
//! commands that choose a folder (audio export, batch conversion, sync)
//! get the system's folder chooser ([`Chooser::folders`]).
//!
//! The dialog runs on its own thread, modal to the window, so the window's
//! event loop never runs inside it. Its answer comes back to the driver as
//! a [`FileChosen`] action. The typed path stays as a fallback: the
//! Open Path key opens the one-line prompt, and so does a dialog that could
//! not be shown at all ([`Outcome::Failed`]).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The chooser's answer, posted to the driver.
#[derive(Debug)]
pub struct FileChosen {
    /// The file picked, or `None` (cancelled, or no dialog).
    pub path: Option<PathBuf>,
    /// How long the dialog was up.
    pub elapsed: Duration,
}

/// What the answer means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A file was picked.
    Chosen(PathBuf),
    /// The dialog was closed without a file.
    Cancelled,
    /// No dialog was shown (no portal on Linux, say): the typed prompt
    /// takes over.
    Failed,
}

/// A dialog that closes sooner than this with no file was never shown:
/// nobody cancels a file chooser that fast.
pub const NOT_SHOWN: Duration = Duration::from_millis(250);

/// What `path` and `elapsed` mean ([`FileChosen`]).
pub fn outcome(path: Option<PathBuf>, elapsed: Duration) -> Outcome {
    match path {
        Some(p) => Outcome::Chosen(p),
        None if elapsed < NOT_SHOWN => Outcome::Failed,
        None => Outcome::Cancelled,
    }
}

/// A file type filter: its name, and extensions without the dot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filter {
    /// The name the dialog shows and reads.
    pub name: String,
    /// Extensions, lowercase, without the dot; `*` is every file.
    pub extensions: Vec<String>,
}

/// The filters: the documents textweaver reads first (the default), then
/// every file. `extensions` is the format registry's list.
pub fn filters(extensions: &[&str], documents: &str, all_files: &str) -> Vec<Filter> {
    let mut exts: Vec<String> = extensions.iter().map(|e| e.to_lowercase()).collect();
    exts.sort();
    exts.dedup();
    let mut out = Vec::new();
    if !exts.is_empty() {
        out.push(Filter {
            name: documents.to_owned(),
            extensions: exts,
        });
    }
    out.push(Filter {
        name: all_files.to_owned(),
        extensions: vec!["*".to_owned()],
    });
    out
}

/// The folder the chooser starts in: the open document's, when it is a
/// file that exists.
pub fn start_folder(document_key: Option<&str>) -> Option<PathBuf> {
    let path = Path::new(document_key?);
    path.is_file()
        .then(|| path.parent().map(Path::to_path_buf))
        .flatten()
}

/// True when the app's prompt `purpose` is answered with the system's
/// chooser: every prompt for a path, except Open after Open Path asked
/// for typing (`typed_open`), and a prompt the file browser has just
/// filled (`from_browser`), which waits for Enter as typed.
pub fn uses_chooser(
    purpose: textweaver_app::PromptPurpose,
    typed_open: bool,
    from_browser: bool,
) -> bool {
    use textweaver_app::PromptPurpose as P;
    purpose.is_path() && !from_browser && !(typed_open && purpose == P::Open)
}

/// The filters a prompt's chooser shows: its own first (the documents
/// textweaver reads, `extensions`, when the spec names none), then every
/// file.
pub fn spec_filters(
    spec: &textweaver_app::path_prompt::PathPromptSpec,
    extensions: &[&str],
    documents: &str,
    all_files: &str,
) -> Vec<Filter> {
    if spec.extensions.is_empty() {
        if matches!(spec.kind, textweaver_app::path_prompt::PathKind::Write(_)) {
            // A file to write with no type of its own: every file.
            return filters(&[], documents, all_files);
        }
        return filters(extensions, documents, all_files);
    }
    let exts: Vec<&str> = spec.extensions.iter().map(String::as_str).collect();
    filters(&exts, &spec.filter_name, all_files)
}

/// The filters for a settings file (W6a6): TOML and JSON, the two
/// `export_settings` writes and `import_settings` reads, then every file.
pub fn settings_filters(settings_files: &str, all_files: &str) -> Vec<Filter> {
    vec![
        Filter {
            name: settings_files.to_owned(),
            extensions: vec!["toml".to_owned(), "json".to_owned()],
        },
        Filter {
            name: all_files.to_owned(),
            extensions: vec!["*".to_owned()],
        },
    ]
}

/// The name an exported settings file is offered under: TOML, which reads
/// and edits like `settings.toml`.
pub const SETTINGS_FILE_NAME: &str = textweaver_app::path_prompt::SETTINGS_FILE_NAME;

/// What the dialog chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// A file that exists (`IFileOpenDialog`).
    Open,
    /// A new file's name (`IFileSaveDialog`).
    Save,
    /// A folder (the open dialog in folder mode on Windows).
    Folder,
}

/// The dialog before it is shown: built on the window's thread, where the
/// window's handle is read, and shown on its own thread.
pub struct Chooser {
    dialog: rfd::FileDialog,
    /// A file to open, a file to save, or a folder.
    mode: Mode,
}

impl Chooser {
    /// A chooser titled `title`, modal to `parent`.
    pub fn new(
        parent: &masonry_winit::winit::window::Window,
        title: &str,
        filters: &[Filter],
        folder: Option<&Path>,
    ) -> Chooser {
        let mut dialog = rfd::FileDialog::new().set_title(title).set_parent(parent);
        for f in filters {
            dialog = dialog.add_filter(&f.name, &f.extensions);
        }
        if let Some(folder) = folder {
            dialog = dialog.set_directory(folder);
        }
        Chooser {
            dialog,
            mode: Mode::Open,
        }
    }

    /// A folder chooser instead: the system's own (the common item dialog
    /// in folder mode on Windows, the portal on Linux, the open panel on
    /// macOS). Filters are not shown.
    pub fn folders(mut self) -> Chooser {
        self.mode = Mode::Folder;
        self
    }

    /// A save dialog instead (`IFileSaveDialog` on Windows), offering
    /// `file_name`; the system asks before replacing a file.
    pub fn saving(mut self, file_name: &str) -> Chooser {
        self.dialog = self.dialog.set_file_name(file_name);
        self.mode = Mode::Save;
        self
    }

    /// Shows the dialog on its own thread and calls `done` with the answer
    /// there, once it closes.
    pub fn show(self, done: impl FnOnce(FileChosen) + Send + 'static) {
        let (dialog, mode) = (self.dialog, self.mode);
        std::thread::spawn(move || {
            let started = Instant::now();
            let path = match mode {
                Mode::Open => dialog.pick_file(),
                Mode::Save => dialog.save_file(),
                Mode::Folder => dialog.pick_folder(),
            };
            done(FileChosen {
                path,
                elapsed: started.elapsed(),
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_read_by_path_and_time() {
        let p = PathBuf::from("notes.md");
        assert_eq!(
            outcome(Some(p.clone()), Duration::from_millis(5)),
            Outcome::Chosen(p)
        );
        assert_eq!(outcome(None, Duration::from_secs(3)), Outcome::Cancelled);
        assert_eq!(outcome(None, Duration::from_millis(20)), Outcome::Failed);
    }

    #[test]
    fn documents_come_first_then_every_file() {
        let f = filters(&["md", "PDF", "md", "docx"], "Documents", "All files");
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].name, "Documents");
        assert_eq!(f[0].extensions, vec!["docx", "md", "pdf"]);
        assert_eq!(f[1].extensions, vec!["*"]);
        // With no known formats, only every file.
        assert_eq!(filters(&[], "Documents", "All files").len(), 1);
    }

    #[test]
    fn every_prompt_for_a_path_routes_to_the_chooser() {
        use textweaver_app::PromptPurpose as P;
        for p in [
            P::Open,
            P::SaveAs,
            P::ImagePath,
            P::ImportReferences,
            P::ImportSettings,
            P::ExportSettings,
            P::ImportProfiles,
            P::ExportProfiles,
        ] {
            assert!(uses_chooser(p, false, false), "{p:?}");
            // Filled by the file browser: shown as typed, for Enter.
            assert!(!uses_chooser(p, false, true), "{p:?}");
        }
        // Open Path asks for typing; the other prompts still choose.
        assert!(!uses_chooser(P::Open, true, false));
        assert!(uses_chooser(P::SaveAs, true, false));
        for p in [P::Find, P::GoTo, P::CommandPalette, P::SyncComputerName] {
            assert!(!uses_chooser(p, false, false), "{p:?}");
        }
    }

    #[test]
    fn a_prompts_filters_come_from_its_spec() {
        use textweaver_app::path_prompt::{IMAGE_EXTENSIONS, PathKind, PathPromptSpec};
        let spec = |kind, exts: &[&str], name: &str| PathPromptSpec {
            purpose: textweaver_app::PromptPurpose::ImagePath,
            kind,
            title: "Insert an image".into(),
            filter_name: name.into(),
            extensions: exts.iter().map(|e| (*e).to_owned()).collect(),
            file_name: None,
            folder: None,
        };
        let images = spec_filters(
            &spec(PathKind::Read(IMAGE_EXTENSIONS), IMAGE_EXTENSIONS, "Images"),
            &["md"],
            "Documents",
            "All files",
        );
        assert_eq!(images[0].name, "Images");
        assert!(images[0].extensions.contains(&"png".to_owned()));
        assert_eq!(images[1].extensions, vec!["*"]);
        // Open: the documents textweaver reads.
        let open = spec_filters(
            &spec(PathKind::Read(&[]), &[], ""),
            &["md", "txt"],
            "Documents",
            "All files",
        );
        assert_eq!(open[0].name, "Documents");
        assert_eq!(open[0].extensions, vec!["md", "txt"]);
        // A file to write with no type of its own: every file.
        let save = spec_filters(
            &spec(PathKind::Write(&[]), &[], ""),
            &["md"],
            "Documents",
            "All files",
        );
        assert_eq!(save.len(), 1);
        assert_eq!(save[0].extensions, vec!["*"]);
    }

    #[test]
    fn settings_files_are_toml_and_json() {
        let f = settings_filters("Settings files", "All files");
        assert_eq!(f[0].extensions, vec!["toml", "json"]);
        assert_eq!(f[1].extensions, vec!["*"]);
        assert!(SETTINGS_FILE_NAME.ends_with(".toml"));
    }

    #[test]
    fn the_registry_offers_the_common_formats() {
        let exts = textweaver_app::formats::Registry::with_builtins().extensions();
        for e in ["md", "txt", "docx", "epub", "html", "odt", "rtf"] {
            assert!(exts.contains(&e), "{e} missing from {exts:?}");
        }
    }

    #[test]
    fn the_start_folder_is_the_documents_own() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("Ada Example.md");
        std::fs::write(&file, "# Notes\n").expect("write");
        let key = file.to_string_lossy().into_owned();
        assert_eq!(start_folder(Some(&key)).as_deref(), Some(dir.path()));
        assert_eq!(start_folder(Some("no such file.md")), None);
        assert_eq!(start_folder(None), None);
    }
}
