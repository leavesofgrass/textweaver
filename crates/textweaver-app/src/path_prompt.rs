//! Prompts that ask for a file or a folder, and the choosers that answer
//! them (W8a-f).
//!
//! The owner met open and export prompts that wanted a full path typed
//! out. Every prompt for a path now has a chooser, and the typed prompt
//! stays for whoever prefers it:
//!
//! - **The GUI** shows the system's own file chooser for each of these
//!   prompts ([`App::path_prompt_spec`] gives its title, filters, the name
//!   offered and the folder it starts in), and the system's folder chooser
//!   for the commands that choose a folder ([`App::folder_choice`]: audio
//!   export's "another folder", batch conversion's folders, the sync
//!   folder). The folder chosen comes back as [`Command::PathChosen`]. A
//!   chooser that cannot be shown gives way to the typed prompt, or to the
//!   file browser for a folder.
//! - **The terminal reader** (and a typed prompt in the GUI) has the
//!   browse key, [`browse_key`] (F4), in each of these prompts: it opens
//!   the shared file browser on the places, and the path chosen fills the
//!   prompt, where Enter confirms it. Escape in the browser goes back to
//!   the prompt as it was, so nothing is lost and focus returns where it
//!   left (WCAG 2.1.2 and 2.4.3).
//!
//! The answers are the prompts' own: a chooser only supplies the text.
//!
//! [`Command::PathChosen`]: crate::Command::PathChosen

use std::path::{Path, PathBuf};

use textweaver_keymap::{Key, KeyChord, Modifiers};
use textweaver_lexicon::args;

use crate::app::{App, Mode};
use crate::command::{Effect, PromptPurpose};

/// Images Insert Image offers.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp"];

/// Reference files Import References reads: BibTeX, RIS and CSL-JSON.
pub const REFERENCE_EXTENSIONS: &[&str] = &["bib", "ris", "json"];

/// Settings and profile files: TOML and JSON.
pub const SETTINGS_EXTENSIONS: &[&str] = &["toml", "json"];

/// The name an exported settings file is offered under: TOML, which reads
/// and edits like `settings.toml`.
pub const SETTINGS_FILE_NAME: &str = "textweaver-settings.toml";

/// The name exported profiles are offered under.
pub const PROFILES_FILE_NAME: &str = "textweaver-profiles.toml";

/// The browse key in a prompt for a path: F4, free in every prompt (F4 is
/// Find Previous only while reading in the terminal), and not a screen
/// reader key in NVDA or JAWS.
pub fn browse_key() -> KeyChord {
    KeyChord::new(Key::F(4), Modifiers::empty())
}

/// What a prompt for a path asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathKind {
    /// A file that exists, to read, listed by these extensions (lowercase,
    /// no dot); none lists the documents textweaver reads.
    Read(&'static [&'static str]),
    /// A new file, to write: a folder and a name. These extensions are
    /// offered; none offers the suggested name's own.
    Write(&'static [&'static str]),
}

impl PromptPurpose {
    /// What a prompt for a path asks for; `None` for every other prompt.
    pub fn path_kind(self) -> Option<PathKind> {
        use PromptPurpose as P;
        Some(match self {
            P::Open => PathKind::Read(&[]),
            P::ImagePath => PathKind::Read(IMAGE_EXTENSIONS),
            P::ImportReferences => PathKind::Read(REFERENCE_EXTENSIONS),
            P::ImportSettings | P::ImportProfiles => PathKind::Read(SETTINGS_EXTENSIONS),
            P::SaveAs => PathKind::Write(&[]),
            P::ExportSettings | P::ExportProfiles => PathKind::Write(SETTINGS_EXTENSIONS),
            _ => return None,
        })
    }

    /// True for a prompt whose answer is a path.
    pub fn is_path(self) -> bool {
        self.path_kind().is_some()
    }
}

/// The system chooser for a prompt for a path: what the GUI shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathPromptSpec {
    /// The prompt it answers.
    pub purpose: PromptPurpose,
    /// A file to read or one to write.
    pub kind: PathKind,
    /// The chooser's title, in the interface's language.
    pub title: String,
    /// The name of its first filter ("Images"); empty when
    /// `extensions` is empty.
    pub filter_name: String,
    /// The first filter's extensions, lowercase, no dot; empty for the
    /// documents textweaver reads (the format registry's list).
    pub extensions: Vec<String>,
    /// The file name offered (a save dialog).
    pub file_name: Option<String>,
    /// The folder it starts in.
    pub folder: Option<PathBuf>,
}

/// A folder chooser a command is waiting on (audio export, batch
/// conversion, sync): what the GUI shows instead of the file browser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderChoice {
    /// The chooser's title: what the folder is for ("Choose the folder
    /// to convert").
    pub title: String,
    /// The folder it starts in.
    pub folder: Option<PathBuf>,
}

/// A prompt set aside while the file browser chooses its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SavedPrompt {
    pub(crate) label: String,
    pub(crate) purpose: PromptPurpose,
    pub(crate) text: String,
}

/// A file's name from a typed answer, when it names one.
fn typed_name(text: &str) -> Option<String> {
    let t = text.trim().trim_matches('"');
    if t.is_empty() || t.ends_with(['/', '\\']) {
        return None;
    }
    Path::new(t)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}

impl App {
    /// Tells the app the key that browses for a path in a prompt, or none:
    /// a frontend that sends [`PromptKey::Browse`](crate::PromptKey::Browse)
    /// sets it, and the key is then named when a prompt for a path opens
    /// ("Image file. F4 to browse."). Off by default (JSON-RPC has none).
    pub fn set_prompt_browse_key(&mut self, key: Option<KeyChord>) {
        self.browse.prompt_key = key;
    }

    /// What is said as a prompt for a path opens: its label, then the
    /// browse key when the frontend has one.
    pub(crate) fn path_prompt_said(&self, purpose: PromptPurpose, label: &str) -> String {
        match (&self.browse.prompt_key, purpose.is_path()) {
            (Some(key), true) => self.msg_args(
                "prompt-browse-hint",
                &args![
                    "label" => label.trim_end_matches('.'),
                    "key" => crate::help::mark_chord(self.cat(), key)
                ],
            ),
            _ => label.to_owned(),
        }
    }

    /// The system chooser for the prompt `purpose`, or `None` when it does
    /// not ask for a path. Save As offers the suggested name in the
    /// suggested folder (the document's own); Insert Image and Import
    /// References start in the document's folder.
    pub fn path_prompt_spec(&self, purpose: PromptPurpose) -> Option<PathPromptSpec> {
        let kind = purpose.path_kind()?;
        let c = self.cat();
        let doc_folder = self.document_folder();
        let mut extensions: Vec<String> = match kind {
            PathKind::Read(e) | PathKind::Write(e) => e.iter().map(|&e| e.to_owned()).collect(),
        };
        let (title, filter, file_name, folder) = match purpose {
            PromptPurpose::SaveAs => {
                let suggested = self.suggested_path.as_deref();
                let ext = suggested
                    .and_then(Path::extension)
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .filter(|e| !e.is_empty());
                let filter = ext.as_ref().map_or_else(String::new, |e| {
                    c.fmt("chooser-type-files", &args!["type" => e.to_uppercase()])
                });
                extensions = ext.into_iter().collect();
                let name = suggested
                    .and_then(Path::file_name)
                    .map(|n| n.to_string_lossy().into_owned());
                let folder = suggested
                    .and_then(Path::parent)
                    .filter(|p| p.is_dir())
                    .map(Path::to_path_buf)
                    .or(doc_folder);
                (c.tr("prompt-save-as"), filter, name, folder)
            }
            PromptPurpose::Open => (c.tr("gui-open-title"), String::new(), None, doc_folder),
            PromptPurpose::ImagePath => (
                c.tr("chooser-image-title"),
                c.tr("chooser-images"),
                None,
                doc_folder,
            ),
            PromptPurpose::ImportReferences => (
                c.tr("chooser-references-title"),
                c.tr("chooser-reference-files"),
                None,
                doc_folder,
            ),
            PromptPurpose::ImportSettings => (
                c.tr("gui-settings-import-title"),
                c.tr("gui-settings-files"),
                None,
                None,
            ),
            PromptPurpose::ExportSettings => (
                c.tr("gui-settings-export-title"),
                c.tr("gui-settings-files"),
                Some(SETTINGS_FILE_NAME.to_owned()),
                None,
            ),
            PromptPurpose::ImportProfiles => (
                c.tr("chooser-profiles-import-title"),
                c.tr("chooser-profile-files"),
                None,
                None,
            ),
            PromptPurpose::ExportProfiles => (
                c.tr("chooser-profiles-export-title"),
                c.tr("chooser-profile-files"),
                Some(PROFILES_FILE_NAME.to_owned()),
                None,
            ),
            _ => return None,
        };
        Some(PathPromptSpec {
            purpose,
            kind,
            title,
            filter_name: if extensions.is_empty() {
                String::new()
            } else {
                filter
            },
            extensions,
            file_name,
            folder,
        })
    }

    /// The browse key in a prompt for a path: the prompt is set aside and
    /// the file browser opens, to choose a file (or, for a file to write,
    /// its folder). Anywhere else nothing happens.
    pub(crate) fn browse_for_prompt(&mut self) -> Vec<Effect> {
        let Some(model) = self.prompt_model.as_ref() else {
            return vec![Effect::Redraw];
        };
        let purpose = model.purpose;
        let Some(kind) = purpose.path_kind() else {
            return vec![Effect::Redraw];
        };
        let saved = SavedPrompt {
            label: model.label.clone(),
            purpose,
            text: model.text(),
        };
        self.prompt_model = None;
        self.leave_prompt();
        let id = match kind {
            PathKind::Read(_) => "prompt-browse-file",
            PathKind::Write(_) => "prompt-browse-folder",
        };
        let what = self.msg_args(id, &args!["label" => saved.label.trim_end_matches('.')]);
        let effects = match kind {
            PathKind::Read(extensions) => {
                self.choose_file(&what, extensions, Self::prompt_file_chosen)
            }
            PathKind::Write(_) => self.choose_folder(&what, Self::prompt_folder_chosen),
        };
        self.browse.prompt = Some(saved);
        effects
    }

    /// A file chosen for the prompt set aside: it fills the prompt.
    fn prompt_file_chosen(app: &mut App, path: PathBuf) -> Vec<Effect> {
        match app.browse.prompt.take() {
            Some(saved) => app.restore_prompt(saved, Some(path)),
            None => vec![Effect::Redraw],
        }
    }

    /// A folder chosen for a prompt for a file to write: the name typed
    /// (or the one offered) in that folder fills the prompt.
    fn prompt_folder_chosen(app: &mut App, folder: PathBuf) -> Vec<Effect> {
        let Some(saved) = app.browse.prompt.take() else {
            return vec![Effect::Redraw];
        };
        let name = typed_name(&saved.text).or_else(|| {
            app.path_prompt_spec(saved.purpose)
                .and_then(|s| s.file_name)
        });
        let path = match name {
            Some(n) => folder.join(n),
            None => folder,
        };
        app.restore_prompt(saved, Some(path))
    }

    /// Opens the prompt set aside again: with `path` in it when one was
    /// chosen (said by name, with what Enter does), else as it was.
    pub(crate) fn restore_prompt(
        &mut self,
        saved: SavedPrompt,
        path: Option<PathBuf>,
    ) -> Vec<Effect> {
        if !self.mode.is_prompt() {
            self.return_mode = self.mode;
        }
        self.mode = Mode::for_prompt(saved.purpose);
        self.prompt_purpose = saved.purpose;
        let said = match &path {
            Some(p) => {
                let name = p.file_name().map_or_else(
                    || p.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                self.msg_args("prompt-browse-filled", &args!["name" => name])
            }
            None => saved.label.clone(),
        };
        let text = path.map_or(saved.text, |p| p.display().to_string());
        self.pending_prompt_text = Some(text);
        self.browse.prompt_filled = true;
        self.say_dialog(&said);
        vec![Effect::Prompt {
            label: saved.label,
            purpose: saved.purpose,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_path_prompt_has_a_kind_and_others_none() {
        use PromptPurpose as P;
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
            assert!(p.is_path(), "{p:?}");
        }
        for p in [
            P::Find,
            P::GoTo,
            P::CommandPalette,
            P::NoteText,
            P::ProfileName,
        ] {
            assert!(!p.is_path(), "{p:?}");
        }
        assert_eq!(P::SaveAs.path_kind(), Some(PathKind::Write(&[])));
        assert_eq!(
            P::ImagePath.path_kind(),
            Some(PathKind::Read(IMAGE_EXTENSIONS))
        );
    }

    #[test]
    fn a_typed_name_is_kept_and_a_folder_is_not_a_name() {
        assert_eq!(typed_name("notes.md").as_deref(), Some("notes.md"));
        assert_eq!(typed_name("\"D:/x/notes.md\"").as_deref(), Some("notes.md"));
        assert_eq!(typed_name("  "), None);
        assert_eq!(typed_name("D:/x/"), None);
    }

    #[test]
    fn the_browse_key_is_plain_f4() {
        assert_eq!(browse_key().to_string(), "F4");
    }
}
