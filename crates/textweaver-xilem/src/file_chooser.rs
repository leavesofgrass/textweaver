//! Open with the system's own file chooser (the owner's session 2,
//! ADR-0033): the common item dialog (`IFileOpenDialog`) on Windows, which
//! NVDA and JAWS know; the XDG desktop portal on Linux, over D-Bus loaded
//! at run time (no GTK); the open panel on macOS. Through the `rfd` crate.
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

/// The dialog before it is shown: built on the window's thread, where the
/// window's handle is read, and shown on its own thread.
pub struct Chooser {
    dialog: rfd::FileDialog,
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
        Chooser { dialog }
    }

    /// Shows the dialog on its own thread and calls `done` with the answer
    /// there, once it closes.
    pub fn show(self, done: impl FnOnce(FileChosen) + Send + 'static) {
        let dialog = self.dialog;
        std::thread::spawn(move || {
            let started = Instant::now();
            let path = dialog.pick_file();
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
