//! The file browser and chooser (Wave 6, W6f; ADR-0045).
//!
//! One list, shared by every frontend through the app's list model, that
//! enters and leaves folders and archives:
//!
//! - **Places first.** File, Browse files opens on the places: the open
//!   document's folder (focused), the folder textweaver started in, the
//!   library's folders, and the drives on Windows (the root folder
//!   elsewhere).
//! - **Rows say the meaning first,** so the first cells of a Braille line
//!   hold the name and the kind: "notes.md, Markdown, 12 KB", "Week 1,
//!   folder, 12 items", "course.zip, zip archive, 3.4 MB". The position
//!   comes last when a row is said ("notes.md, Markdown, 12 KB, 3 of 40").
//! - **Enter** opens a document, or enters a folder or an archive; inside
//!   an archive, folders and archives in it are entered the same way, and
//!   a member is addressed as `course.zip!week1/notes.md`, the form that
//!   keys its notes and reading position (`textweaver_formats::archive`).
//!   **Backspace** goes up; out of an archive it lands on the archive's
//!   row. Right enters and Left goes up, as in the menus. Typing filters by
//!   name; Backspace takes the filter back first.
//! - **The Say Status key** says a preview of the focused row: a
//!   document's title and first sentence, loaded on a helper thread with
//!   the loaders' own limits (text recognition off), so the keyboard never
//!   waits; a folder's or an archive's first names.
//! - **Readable files only** by default; the Show All key shows every
//!   file, hidden ones included. The Sort key cycles by name, by date
//!   (newest first), and by size (largest first); folders stay first.
//! - **Choosing:** [`App::choose_folder`] and [`App::choose_file`] open
//!   the browser for another command (batch conversion, audio export,
//!   settings import) and hand it the path chosen. The Choose Folder key
//!   (Ctrl+Enter, or Cmd+Enter in the Mac window) takes the focused folder
//!   or the one shown; a "Choose this folder" row does the same for a
//!   terminal that cannot send Ctrl+Enter.
//!
//! The browser only opens and chooses. It never copies, moves, renames or
//! deletes anything, and nothing in an archive is extracted: members are
//! read into memory within the archive module's limits. Names are shown
//! with control and direction characters replaced, so a hostile name can
//! neither drive the terminal nor pretend to be another file; archive
//! members are normalized so none climbs out of its archive; junk entries
//! (`__MACOSX/`, `.DS_Store`) are left out; archives nested too deeply,
//! damaged, or too large are refused with a sentence.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant, SystemTime};

use textweaver_a11y::{Importance, Priority};
use textweaver_core::Unit;
use textweaver_formats::{Progress, Registry, Source, archive};
use textweaver_keymap::{ActionId, Key, KeyChord, Modifiers};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::{App, ListKind};
use crate::command::{Command, Effect};
use crate::list_model::ListKey;

/// Most entries listed in one folder or archive.
pub const MAX_ENTRIES: usize = archive::MAX_LISTED;

/// Folders whose items are counted for their rows; past this many, a
/// folder's row leaves its count out, so a huge folder lists quickly.
const MAX_COUNTED_FOLDERS: usize = 500;

/// Longest first sentence said in a preview, in characters.
const PREVIEW_CHARS: usize = 240;

/// How many names a folder's or an archive's preview says.
const PREVIEW_NAMES: usize = 3;

/// A command another module runs with the path chosen in the browser.
pub type Chosen = fn(&mut App, PathBuf) -> Vec<Effect>;

/// The browser's own keys, besides the list's: they work only while the
/// browser is shown, so they take no key from the keymap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrowseKey {
    /// Chooses the focused folder, or the folder shown (Ctrl+Enter).
    ChooseFolder,
    /// Cycles the order: name, date, size (Ctrl+R).
    Sort,
    /// Shows every file, or readable ones only (Ctrl+A; in the Mac window
    /// Cmd+Shift+Period, as in the Mac's own file dialogs).
    ShowAll,
}

impl BrowseKey {
    /// Every browser key.
    pub const ALL: [BrowseKey; 3] = [BrowseKey::ChooseFolder, BrowseKey::Sort, BrowseKey::ShowAll];

    /// Its chord where commands use `command` (Ctrl; Cmd in the Mac
    /// window, [`command_modifier`]).
    pub fn chord(self, command: Modifiers) -> KeyChord {
        let mac = command.contains(Modifiers::META);
        match self {
            BrowseKey::ChooseFolder => KeyChord::new(Key::Enter, command),
            BrowseKey::Sort => KeyChord::new(Key::Char('r'), command),
            BrowseKey::ShowAll if mac => KeyChord::new(Key::Char('.'), command | Modifiers::SHIFT),
            BrowseKey::ShowAll => KeyChord::new(Key::Char('a'), command),
        }
    }

    /// The list key a frontend sends for it.
    pub fn list_key(self) -> ListKey {
        match self {
            BrowseKey::ChooseFolder => ListKey::ChooseHere,
            BrowseKey::Sort => ListKey::Sort,
            BrowseKey::ShowAll => ListKey::ShowAll,
        }
    }
}

/// The modifier `keymap` runs commands with: Open's (Ctrl, or Cmd in the
/// Mac window). The browser's keys follow it, so they are Cmd keys where
/// the other commands are.
pub fn command_modifier(keymap: &textweaver_keymap::Keymap) -> Modifiers {
    keymap
        .chords_for(ActionId::Open)
        .iter()
        .map(|c| c.mods & (Modifiers::CTRL | Modifiers::META))
        .find(|m| !m.is_empty())
        .unwrap_or(Modifiers::CTRL)
}

/// What the browser is open for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Browsing: Enter opens a document.
    Open,
    /// Choosing a folder for a command.
    Folder,
    /// Choosing a file for a command.
    File,
}

/// The order of a folder's rows. Folders always come first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortBy {
    /// By name, numbers in order ("Week 2" before "Week 10").
    #[default]
    Name,
    /// Newest first.
    Date,
    /// Largest first.
    Size,
}

impl SortBy {
    fn next(self) -> SortBy {
        match self {
            SortBy::Name => SortBy::Date,
            SortBy::Date => SortBy::Size,
            SortBy::Size => SortBy::Name,
        }
    }

    fn id(self) -> &'static str {
        match self {
            SortBy::Name => "name",
            SortBy::Date => "date",
            SortBy::Size => "size",
        }
    }
}

/// Where the browser is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    /// The places.
    Places,
    /// A folder on disk.
    Folder(PathBuf),
    /// A folder inside an archive: the archive (a file, or a member path
    /// for an archive in an archive) and the folder's name inside it,
    /// empty or ending with `/`.
    Archive {
        /// The archive.
        path: PathBuf,
        /// The folder inside it: `""` or `week1/`.
        prefix: String,
    },
}

/// What kind of place a place row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaceKind {
    DocumentFolder,
    StartFolder,
    LibraryFolder,
    /// A drive, by its Windows drive type (Windows only).
    #[cfg_attr(not(windows), allow(dead_code))]
    Drive(u32),
    /// The root folder (not on Windows, which lists its drives).
    #[cfg_attr(windows, allow(dead_code))]
    Root,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RowKind {
    /// "Choose this folder" (while choosing a folder).
    ChooseHere,
    Place(PlaceKind),
    /// A folder; inside an archive its name inside it, ending with `/`.
    Folder {
        items: Option<usize>,
        prefix: Option<String>,
    },
    /// An archive, with its kind's name ("zip").
    Archive(String),
    /// A readable document, by its loader's id.
    Document(&'static str),
    /// Any other file (shown with Show All, or chosen by extension).
    File,
}

#[derive(Clone, Debug)]
struct Row {
    /// The name as the file system or archive has it.
    name: String,
    kind: RowKind,
    /// Its path: on disk, or a member path.
    path: PathBuf,
    size: Option<u64>,
    modified: Option<SystemTime>,
    hidden: bool,
}

/// A preview loading on a helper thread.
struct PreviewJob {
    rx: Receiver<String>,
    generation: u64,
    progress: Progress,
}

/// The browser's state, kept by the app.
pub(crate) struct BrowseState {
    location: Option<Location>,
    rows: Vec<Row>,
    /// Rows shown, in order, as indexes into `rows`.
    shown: Vec<usize>,
    /// Rows left out because they are not readable or are hidden.
    hidden: usize,
    /// More entries than [`MAX_ENTRIES`] were there.
    cut: bool,
    filter: String,
    show_all: bool,
    sort: SortBy,
    pick: Pick,
    /// Extensions a file chooser shows; empty for readable documents.
    extensions: &'static [&'static str],
    then: Option<Chosen>,
    /// The row last chosen (for a frontend whose Choose closed the list).
    chose: usize,
    /// What the chooser is for, said when it opens.
    purpose: Option<String>,
    /// The listing of the archive last entered.
    archive: Option<(PathBuf, Vec<archive::Entry>)>,
    /// The folder textweaver started in.
    start: Option<PathBuf>,
    preview: Option<PreviewJob>,
    /// A prompt for a path set aside while the browser chooses it (the
    /// browse key, crate::path_prompt): Escape opens it again.
    pub(crate) prompt: Option<crate::path_prompt::SavedPrompt>,
    /// The frontend's browse key, named when a prompt for a path opens.
    pub(crate) prompt_key: Option<KeyChord>,
    /// The next prompt shown was filled by the browser.
    pub(crate) prompt_filled: bool,
    /// A frontend took this folder choice to its own chooser
    /// ([`App::take_folder_choice`]): the browser is its fallback.
    native_taken: bool,
}

impl BrowseState {
    /// Nothing shown; the start folder is the current folder now.
    pub(crate) fn new() -> Self {
        BrowseState {
            location: None,
            rows: Vec::new(),
            shown: Vec::new(),
            hidden: 0,
            cut: false,
            filter: String::new(),
            show_all: false,
            sort: SortBy::Name,
            pick: Pick::Open,
            extensions: &[],
            then: None,
            chose: 0,
            purpose: None,
            archive: None,
            start: std::env::current_dir().ok(),
            preview: None,
            prompt: None,
            prompt_key: None,
            prompt_filled: false,
            native_taken: false,
        }
    }
}

/// Registers the browser's command with the menus and the palette.
pub(crate) fn register(app: &mut App) {
    app.register_handler(ActionId::BrowseFiles, |app| app.browse_files());
}

/// `name` with control, format and direction characters replaced by a
/// question mark: a file name cannot drive the terminal, and cannot
/// reorder itself to look like another name.
fn shown_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            let direction = matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}');
            if c.is_control() || direction {
                '?'
            } else {
                c
            }
        })
        .collect()
}

/// The last part of a path, or the whole of it (a drive, the root).
fn path_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The lowercase extension of a name.
fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// The kind of archive `name` is, as said ("zip", "tar.gz"), when this
/// build opens archives of its extension.
fn archive_kind(registry: &Registry, name: &str) -> Option<String> {
    let ext = extension(name);
    let opens = registry
        .loader_by_id("archive")
        .is_some_and(|l| l.extensions().contains(&ext.as_str()));
    if !opens {
        return None;
    }
    Some(
        if ext == "tgz" || name.to_lowercase().ends_with(".tar.gz") {
            "tar.gz".to_owned()
        } else if ext == "gz" {
            "gzip".to_owned()
        } else {
            ext
        },
    )
}

/// What kind of file `name` is: an archive, a readable document, or
/// another file.
fn classify(registry: &Registry, name: &str) -> RowKind {
    if let Some(kind) = archive_kind(registry, name) {
        return RowKind::Archive(kind);
    }
    let hint = Source::Bytes {
        data: Vec::new(),
        hint: extension(name),
    };
    match registry.loader_for(&hint) {
        Some(l) => RowKind::Document(l.id()),
        None => RowKind::File,
    }
}

/// True when a row of `kind` is listed without Show All: folders,
/// archives, and readable documents (or, choosing a file by extension,
/// folders and files with one of them).
fn listed(kind: &RowKind, name: &str, extensions: &[&str]) -> bool {
    match kind {
        RowKind::ChooseHere | RowKind::Place(_) | RowKind::Folder { .. } => true,
        _ if !extensions.is_empty() => extensions.contains(&extension(name).as_str()),
        RowKind::File => false,
        RowKind::Archive(_) | RowKind::Document(_) => true,
    }
}

/// True for a hidden file: a dot file, or one Windows marks hidden or as
/// part of the system.
fn is_hidden(name: &str, meta: Option<&std::fs::Metadata>) -> bool {
    if name.starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const HIDDEN: u32 = 0x2;
        const SYSTEM: u32 = 0x4;
        if meta.is_some_and(|m| m.file_attributes() & (HIDDEN | SYSTEM) != 0) {
            return true;
        }
    }
    #[cfg(not(windows))]
    let _ = meta;
    false
}

/// Compares names as people read them: case aside, and runs of digits by
/// their value ("Week 2" before "Week 10").
fn natural(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(p), Some(q)) if p.is_ascii_digit() && q.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars<'_>>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (m, n) = (take(&mut x), take(&mut y));
                let (m, n) = (m.trim_start_matches('0'), n.trim_start_matches('0'));
                let o = m.len().cmp(&n.len()).then_with(|| m.cmp(n));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(p), Some(q)) => {
                let o = p.to_lowercase().cmp(q.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// A size in words: "812 bytes", "12 KB", "3.4 MB", "1.2 GB".
fn size_text(c: &Catalog, bytes: u64) -> String {
    const K: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        let n = i64::try_from(bytes).unwrap_or(i64::MAX);
        return c.fmt(
            "browse-size-bytes",
            &args!["n" => n, "count" => crate::words::grouped(c, bytes as usize)],
        );
    }
    let (id, value) = if b < K * K {
        ("browse-size-kb", b / K)
    } else if b < K * K * K {
        ("browse-size-mb", b / (K * K))
    } else {
        ("browse-size-gb", b / (K * K * K))
    };
    let amount = if value < 10.0 {
        crate::words::decimal(c, value)
    } else {
        crate::words::grouped(c, value.round() as usize)
    };
    c.fmt(id, &args!["size" => amount])
}

/// The words for a document kind, by loader id ("Markdown"); the
/// extension for a loader without a name of its own.
fn kind_text(c: &Catalog, id: &str, name: &str) -> String {
    let key = format!("browse-kind-{id}");
    if c.has(&key) {
        c.tr(&key)
    } else {
        extension(name).to_uppercase()
    }
}

/// The drives Windows has, with their types.
#[cfg(windows)]
#[allow(unsafe_code)]
fn drives() -> Vec<(PathBuf, u32)> {
    use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    // SAFETY: GetLogicalDrives takes no arguments and only returns a mask.
    let mask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();
    for i in 0..26u8 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let root = format!("{}:\\", char::from(b'A' + i));
        let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives
        // the call; GetDriveTypeW only reads it and touches no disk.
        let kind = unsafe { GetDriveTypeW(windows::core::PCWSTR(wide.as_ptr())) };
        out.push((PathBuf::from(root), kind));
    }
    out
}

/// The row text for a drive's type (DRIVE_REMOVABLE is 2, DRIVE_FIXED 3,
/// DRIVE_REMOTE 4, DRIVE_CDROM 5, DRIVE_RAMDISK 6).
fn drive_id(kind: u32) -> &'static str {
    match kind {
        2 => "browse-place-removable",
        4 => "browse-place-network",
        5 => "browse-place-cd",
        _ => "browse-place-disk",
    }
}

impl Row {
    /// The row as shown and said, meaning first.
    fn text(&self, c: &Catalog, shown_folder: &str) -> String {
        let name = shown_name(&self.name);
        let size = || self.size.map(|s| size_text(c, s));
        let mut text = match &self.kind {
            RowKind::ChooseHere => c.fmt(
                "browse-choose-here",
                &args!["name" => shown_name(shown_folder)],
            ),
            RowKind::Place(p) => {
                let id = match p {
                    PlaceKind::DocumentFolder => "browse-place-document",
                    PlaceKind::StartFolder => "browse-place-start",
                    PlaceKind::LibraryFolder => "browse-place-library",
                    PlaceKind::Root => "browse-place-root",
                    PlaceKind::Drive(k) => drive_id(*k),
                };
                c.fmt(id, &args!["name" => name])
            }
            RowKind::Folder { items: Some(n), .. } => c.fmt(
                "browse-row-folder-items",
                &args!["name" => name, "n" => *n, "count" => crate::words::grouped(c, *n)],
            ),
            RowKind::Folder { items: None, .. } => {
                c.fmt("browse-row-folder", &args!["name" => name])
            }
            RowKind::Archive(kind) => {
                let kind = c.fmt("browse-kind-archive", &args!["kind" => kind.as_str()]);
                match size() {
                    Some(s) => c.fmt(
                        "browse-row-file",
                        &args!["name" => name, "kind" => kind, "size" => s],
                    ),
                    None => c.fmt("browse-row-kind", &args!["name" => name, "kind" => kind]),
                }
            }
            RowKind::Document(id) => {
                let kind = kind_text(c, id, &self.name);
                match size() {
                    Some(s) => c.fmt(
                        "browse-row-file",
                        &args!["name" => name, "kind" => kind, "size" => s],
                    ),
                    None => c.fmt("browse-row-kind", &args!["name" => name, "kind" => kind]),
                }
            }
            RowKind::File => {
                let kind = c.tr("browse-kind-file");
                match size() {
                    Some(s) => c.fmt(
                        "browse-row-file",
                        &args!["name" => name, "kind" => kind, "size" => s],
                    ),
                    None => c.fmt("browse-row-kind", &args!["name" => name, "kind" => kind]),
                }
            }
        };
        if self.hidden {
            text = c.fmt("browse-row-hidden", &args!["row" => text]);
        }
        text
    }

    fn is_folder(&self) -> bool {
        matches!(self.kind, RowKind::Folder { .. } | RowKind::Place(_))
    }
}

/// How rows compare in `sort`: the Choose row first, then folders, then
/// files.
fn compare(a: &Row, b: &Row, sort: SortBy) -> Ordering {
    let rank = |r: &Row| match r.kind {
        RowKind::ChooseHere => 0,
        RowKind::Place(_) | RowKind::Folder { .. } => 1,
        _ => 2,
    };
    rank(a).cmp(&rank(b)).then_with(|| {
        let by_name = || natural(&a.name, &b.name);
        match sort {
            SortBy::Name => by_name(),
            SortBy::Date => b.modified.cmp(&a.modified).then_with(by_name),
            SortBy::Size if !a.is_folder() => b.size.cmp(&a.size).then_with(by_name),
            SortBy::Size => by_name(),
        }
    })
}

/// How many entries of `dir` a row would list.
fn count_items(
    dir: &Path,
    registry: &Registry,
    show_all: bool,
    extensions: &[&str],
) -> Option<usize> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut n = 0;
    for e in entries.flatten().take(MAX_ENTRIES) {
        let name = e.file_name().to_string_lossy().into_owned();
        let meta = e.metadata().ok();
        let is_dir = e.file_type().is_ok_and(|t| t.is_dir())
            || (e.file_type().is_ok_and(|t| t.is_symlink()) && e.path().is_dir());
        if !show_all && is_hidden(&name, meta.as_ref()) {
            continue;
        }
        if is_dir || show_all || listed(&classify(registry, &name), &name, extensions) {
            n += 1;
        }
    }
    Some(n)
}

/// A folder's rows, every entry (the filter comes later), and whether
/// there were more than [`MAX_ENTRIES`].
fn read_folder(
    dir: &Path,
    registry: &Registry,
    show_all: bool,
    extensions: &[&str],
) -> std::io::Result<(Vec<Row>, bool)> {
    let mut rows = Vec::new();
    let mut counted = 0;
    let mut cut = false;
    for (i, e) in std::fs::read_dir(dir)?.flatten().enumerate() {
        if i >= MAX_ENTRIES {
            cut = true;
            break;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        // A link is listed as what it points at.
        let meta = std::fs::metadata(&path).ok().or_else(|| e.metadata().ok());
        let hidden = is_hidden(&name, e.metadata().ok().as_ref());
        let modified = meta.as_ref().and_then(|m| m.modified().ok());
        if meta.as_ref().is_some_and(std::fs::Metadata::is_dir) {
            let items = (counted < MAX_COUNTED_FOLDERS)
                .then(|| {
                    counted += 1;
                    count_items(&path, registry, show_all, extensions)
                })
                .flatten();
            rows.push(Row {
                name,
                kind: RowKind::Folder {
                    items,
                    prefix: None,
                },
                path,
                size: None,
                modified,
                hidden,
            });
        } else {
            rows.push(Row {
                kind: classify(registry, &name),
                name,
                path,
                size: meta.as_ref().map(std::fs::Metadata::len),
                modified,
                hidden,
            });
        }
    }
    Ok((rows, cut))
}

/// The rows of folder `prefix` in an archive listed as `entries`.
fn archive_rows(
    archive_path: &Path,
    entries: &[archive::Entry],
    prefix: &str,
    registry: &Registry,
    show_all: bool,
    extensions: &[&str],
) -> Vec<Row> {
    use std::collections::BTreeMap;
    // A folder's name, the names under it, and whether it is hidden.
    let mut folders: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    let mut rows = Vec::new();
    for e in entries.iter().take(MAX_ENTRIES) {
        if archive::is_junk(&e.name) {
            continue;
        }
        let Some(rest) = e.name.strip_prefix(prefix) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        match rest.split_once('/') {
            Some((dir, below)) => {
                let child = below.split('/').next().unwrap_or(below).to_owned();
                let set = folders.entry(dir.to_owned()).or_default();
                let hidden_child = child.starts_with('.');
                let kind = classify(registry, &child);
                let is_dir = below.contains('/');
                if !child.is_empty()
                    && (show_all || !hidden_child)
                    && (show_all || is_dir || listed(&kind, &child, extensions))
                {
                    set.insert(child);
                }
            }
            None => rows.push(Row {
                kind: classify(registry, rest),
                name: rest.to_owned(),
                path: archive::member_path(archive_path, &e.name),
                size: Some(e.size),
                modified: None,
                hidden: rest.starts_with('.'),
            }),
        }
    }
    for (dir, children) in folders {
        let inner = format!("{prefix}{dir}/");
        rows.push(Row {
            hidden: dir.starts_with('.'),
            path: archive::member_path(archive_path, &inner),
            name: dir,
            kind: RowKind::Folder {
                items: Some(children.len()),
                prefix: Some(inner),
            },
            size: None,
            modified: None,
        });
    }
    rows
}

/// The location above an archive's root: the folder holding it, or for an
/// archive in an archive, the folder inside the outer archive.
fn archive_parent(path: &Path) -> Location {
    if path.is_file() {
        return path
            .parent()
            .map_or(Location::Places, |p| Location::Folder(p.to_owned()));
    }
    let Some((file, inner)) = archive::split_member(path) else {
        return Location::Places;
    };
    let (outer, name) = match inner.rfind(archive::SEPARATOR) {
        Some(i) => (
            archive::member_path(&file, &inner[..i]),
            inner[i + 1..].to_owned(),
        ),
        None => (file, inner),
    };
    let prefix = match name.rfind('/') {
        Some(i) => name[..=i].to_owned(),
        None => String::new(),
    };
    Location::Archive {
        path: outer,
        prefix,
    }
}

/// A preview's words, made on the helper thread.
fn preview_text(c: &Catalog, row: &Row, options: textweaver_formats::LoadOptions) -> String {
    let name = shown_name(&row.name);
    match &row.kind {
        RowKind::Document(_) => {
            let registry = Registry::with_builtins();
            match registry.load(&Source::Path(row.path.clone()), &options) {
                Ok(doc) => {
                    let title = doc
                        .meta
                        .title
                        .clone()
                        .filter(|t| !t.trim().is_empty())
                        .unwrap_or_else(|| name.clone());
                    match first_sentence(&doc, &title) {
                        Some(s) => c.fmt(
                            "browse-preview-document",
                            &args!["title" => shown_name(&title), "sentence" => shown_name(&s)],
                        ),
                        None => c.fmt(
                            "browse-preview-no-text",
                            &args!["title" => shown_name(&title)],
                        ),
                    }
                }
                Err(e) => c.fmt(
                    "browse-preview-failed",
                    &args![
                        "name" => name,
                        "reason" => crate::opening::open_failure_reason_in(c, &row.path, &e)
                    ],
                ),
            }
        }
        RowKind::Archive(_) => match archive::list_path(&row.path) {
            Ok(entries) => {
                let registry = Registry::with_builtins();
                let files: Vec<&archive::Entry> = entries
                    .iter()
                    .filter(|e| !archive::is_junk(&e.name))
                    .collect();
                let readable = files
                    .iter()
                    .filter(|e| !matches!(classify(&registry, &e.name), RowKind::File))
                    .count();
                let mut names: Vec<String> = files.iter().map(|e| shown_name(&e.name)).collect();
                names.sort_by(|a, b| natural(a, b));
                names.truncate(PREVIEW_NAMES);
                c.fmt(
                    "browse-preview-archive",
                    &args![
                        "name" => name,
                        "n" => files.len(),
                        "count" => crate::words::grouped(c, files.len()),
                        "readable" => crate::words::grouped(c, readable),
                        "names" => names.join(", ")
                    ],
                )
            }
            Err(e) => archive_refusal(c, &row.name, &e),
        },
        RowKind::Folder { prefix: None, .. } | RowKind::Place(_) => {
            let registry = Registry::with_builtins();
            let mut names: Vec<String> = std::fs::read_dir(&row.path)
                .map(|rd| {
                    rd.flatten()
                        .take(MAX_ENTRIES)
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .filter(|n| {
                            !is_hidden(n, None) && !matches!(classify(&registry, n), RowKind::File)
                        })
                        .collect()
                })
                .unwrap_or_default();
            names.sort_by(|a, b| natural(a, b));
            let path = shown_name(&row.path.display().to_string());
            if names.is_empty() {
                return c.fmt("browse-preview-folder-empty", &args!["path" => path]);
            }
            let names: Vec<String> = names
                .iter()
                .take(PREVIEW_NAMES)
                .map(|n| shown_name(n))
                .collect();
            c.fmt(
                "browse-preview-folder",
                &args!["path" => path, "names" => names.join(", ")],
            )
        }
        RowKind::Folder { .. } | RowKind::ChooseHere | RowKind::File => String::new(),
    }
}

/// The first sentence of `doc` that is not its title, whitespace
/// collapsed, at most [`PREVIEW_CHARS`] characters (cut at a word).
fn first_sentence(doc: &textweaver_text::Document, title: &str) -> Option<String> {
    let mut range = textweaver_text::first_unit(doc, Unit::Sentence)?;
    for _ in 0..8 {
        let text: String = doc
            .slice(range)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() && text.trim_end_matches(['.', ':']) != title.trim() {
            if text.chars().count() <= PREVIEW_CHARS {
                return Some(text);
            }
            let cut: String = text.chars().take(PREVIEW_CHARS).collect();
            let at = cut.rfind(' ').unwrap_or(cut.len());
            return Some(cut[..at].to_owned());
        }
        range = textweaver_text::next_unit(doc, range.end, Unit::Sentence)?;
    }
    None
}

/// Why an archive cannot be listed, in a sentence.
fn archive_refusal(c: &Catalog, name: &str, e: &std::io::Error) -> String {
    let detail = e.to_string();
    let id = if detail.contains("nested too deeply") {
        "browse-archive-too-deep"
    } else if detail.contains("too large") || detail.contains("too much data") {
        "browse-archive-too-large"
    } else {
        "browse-archive-unreadable"
    };
    c.fmt(id, &args!["name" => shown_name(name)])
}

impl App {
    /// File, Browse files: the browser, on the places, to open a document.
    pub fn browse_files(&mut self) -> Vec<Effect> {
        self.start_browser(Pick::Open, &[], None, None)
    }

    /// Opens the browser to choose a folder for another command (batch
    /// conversion, audio export): `purpose` is said as it opens ("Choose
    /// the folder to convert"), and `then` runs with the folder chosen.
    /// A folder inside an archive cannot be chosen. Escape closes the
    /// browser without calling `then`.
    ///
    /// Call it from the command's handler ([`App::register_handler`]) and
    /// return its effects, so the app shows the list it returns:
    ///
    /// ```ignore
    /// fn convert(app: &mut App) -> Vec<Effect> {
    ///     // In the interface's language, from the catalog.
    ///     let purpose = String::from("Choose the folder to convert");
    ///     app.choose_folder(&purpose, |app, folder| app.convert_folder(folder))
    /// }
    /// ```
    pub fn choose_folder(&mut self, purpose: &str, then: Chosen) -> Vec<Effect> {
        self.start_browser(Pick::Folder, &[], Some(purpose.to_owned()), Some(then))
    }

    /// Opens the browser to choose a file for another command (settings
    /// import): files with one of `extensions` are listed (lowercase, no
    /// dot; empty lists readable documents), and `then` runs with the
    /// file chosen. Archives are listed as files, not entered.
    pub fn choose_file(
        &mut self,
        purpose: &str,
        extensions: &'static [&'static str],
        then: Chosen,
    ) -> Vec<Effect> {
        self.start_browser(Pick::File, extensions, Some(purpose.to_owned()), Some(then))
    }

    /// The folder chooser a command is waiting on, just opened on the
    /// places ([`App::choose_folder`]): a GUI shows the system's folder
    /// chooser instead of the browser, and answers with
    /// [`Command::PathChosen`]. `None` once a frontend took it
    /// ([`App::take_folder_choice`]) or the browser is in use, and for a
    /// browser opened from a prompt with the browse key.
    pub fn folder_choice(&self) -> Option<crate::path_prompt::FolderChoice> {
        let waiting = self.list == Some(ListKind::Browse)
            && self.browse.pick == Pick::Folder
            && self.browse.then.is_some()
            && self.browse.prompt.is_none()
            && !self.browse.native_taken
            && self.browse.location == Some(Location::Places);
        if !waiting {
            return None;
        }
        Some(crate::path_prompt::FolderChoice {
            title: self.browse.purpose.clone().unwrap_or_default(),
            folder: self.document_folder(),
        })
    }

    /// [`App::folder_choice`], taken by a frontend that shows its own
    /// chooser: asked again, it is `None`, so the browser shown after a
    /// chooser that could not open is not taken over.
    pub fn take_folder_choice(&mut self) -> Option<crate::path_prompt::FolderChoice> {
        let choice = self.folder_choice()?;
        self.browse.native_taken = true;
        Some(choice)
    }

    /// [`Command::PathChosen`]: the path chosen in a system chooser hands
    /// it to the command waiting, as the browser would; `None` (the
    /// chooser was closed) cancels the choice.
    pub(crate) fn path_chosen(&mut self, path: Option<PathBuf>) -> Vec<Effect> {
        if self.list != Some(ListKind::Browse) || self.browse.then.is_none() {
            return vec![Effect::Redraw];
        }
        match path {
            Some(p) => self.chosen(p),
            None => {
                let _ = self.close_browser(false);
                let msg = self.msg("common-cancelled");
                self.note(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// Where the browser is, while it is shown.
    pub fn browse_location(&self) -> Option<&Location> {
        self.browse.location.as_ref()
    }

    /// The browser's order.
    pub fn browse_sort(&self) -> SortBy {
        self.browse.sort
    }

    /// The list key for `chord` while the browser is shown: the browser's
    /// own keys ([`BrowseKey`]), in the form this keymap's commands use.
    pub fn browse_list_key_for(&self, chord: &KeyChord) -> Option<ListKey> {
        if self.list != Some(ListKind::Browse) {
            return None;
        }
        let command = command_modifier(&self.keymap);
        BrowseKey::ALL
            .into_iter()
            .find(|k| k.chord(command) == *chord)
            .map(BrowseKey::list_key)
    }

    /// Waits until a preview has been said (or `timeout` passes); for
    /// tests. True when none is waiting.
    pub fn wait_for_preview(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let _ = self.tick(Instant::now());
            if self.browse.preview.is_none() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn start_browser(
        &mut self,
        pick: Pick,
        extensions: &'static [&'static str],
        purpose: Option<String>,
        then: Option<Chosen>,
    ) -> Vec<Effect> {
        self.leave_menu();
        self.browse.prompt = None;
        self.browse.native_taken = false;
        self.browse.pick = pick;
        self.browse.extensions = extensions;
        self.browse.purpose = purpose;
        self.browse.then = then;
        self.browse.filter.clear();
        let doc_folder = self.document_folder();
        self.show_places(doc_folder.as_deref())
    }

    /// The open document's folder (an archive member's: the archive's).
    pub(crate) fn document_folder(&self) -> Option<PathBuf> {
        let path = self.session.as_ref()?.doc.meta.path.clone()?;
        let file = archive::split_member(&path).map_or(path, |(file, _)| file);
        file.parent().filter(|p| p.is_dir()).map(Path::to_owned)
    }

    /// The places, focused on `focus` if it is one.
    fn show_places(&mut self, focus: Option<&Path>) -> Vec<Effect> {
        let mut rows = Vec::new();
        let push = |rows: &mut Vec<Row>, path: PathBuf, kind: PlaceKind| {
            if rows.iter().any(|r: &Row| r.path == path) || !path.is_dir() {
                return;
            }
            rows.push(Row {
                name: match kind {
                    PlaceKind::Drive(_) | PlaceKind::Root => path.display().to_string(),
                    _ => path_name(&path),
                },
                kind: RowKind::Place(kind),
                path,
                size: None,
                modified: None,
                hidden: false,
            });
        };
        if let Some(d) = self.document_folder() {
            push(&mut rows, d, PlaceKind::DocumentFolder);
        }
        if let Some(s) = self.browse.start.clone() {
            push(&mut rows, s, PlaceKind::StartFolder);
        }
        for f in self.settings.library.folders.clone() {
            push(&mut rows, f, PlaceKind::LibraryFolder);
        }
        #[cfg(windows)]
        for (root, kind) in drives() {
            // Listed without touching the drive: a disconnected network
            // drive or an empty card reader must not stall the list.
            if !rows.iter().any(|r| r.path == root) {
                rows.push(Row {
                    name: root.display().to_string(),
                    kind: RowKind::Place(PlaceKind::Drive(kind)),
                    path: root,
                    size: None,
                    modified: None,
                    hidden: false,
                });
            }
        }
        #[cfg(not(windows))]
        push(&mut rows, PathBuf::from("/"), PlaceKind::Root);
        self.browse.location = Some(Location::Places);
        self.browse.rows = rows;
        self.browse.cut = false;
        self.browse.hidden = 0;
        self.browse.shown = (0..self.browse.rows.len()).collect();
        let focus = focus
            .and_then(|f| self.browse.rows.iter().position(|r| r.path == f))
            .unwrap_or(0);
        let n = self.browse.rows.len();
        let c = self.catalog();
        let mut intro = c.fmt("browse-places-intro", &args!["n" => n]);
        if let Some(p) = &self.browse.purpose {
            intro = c.fmt(
                "browse-choosing",
                &args!["purpose" => p.as_str(), "intro" => intro],
            );
        }
        self.present(focus, Some(&intro))
    }

    /// Shows the rows of the location, focused on shown row `focus`, and
    /// says `intro` first.
    fn present(&mut self, focus: usize, intro: Option<&str>) -> Vec<Effect> {
        let c = self.catalog();
        let folder = self.location_name();
        let items: Vec<String> = self
            .browse
            .shown
            .iter()
            .map(|&i| self.browse.rows[i].text(&c, &folder))
            .collect();
        self.list = Some(ListKind::Browse);
        self.pending_list_focus = Some(focus.min(items.len().saturating_sub(1)));
        if let Some(intro) = intro {
            self.say_result(intro);
        }
        vec![Effect::ShowList {
            title: self.location_title(),
            items,
        }]
    }

    /// The browser stays as it was after a key that changed nothing; a
    /// frontend whose Choose closed the list gets it back, on the row it
    /// chose.
    fn stay(&mut self) -> Vec<Effect> {
        self.list = Some(ListKind::Browse);
        if self.list_model.is_some() || self.browse.location.is_none() {
            return vec![Effect::Redraw];
        }
        let focus = self.browse.chose;
        self.present(focus, None)
    }

    /// The list's title: the place's full path, so each is its own list.
    fn location_title(&self) -> String {
        match &self.browse.location {
            Some(Location::Folder(p)) => shown_name(&p.display().to_string()),
            Some(Location::Archive { path, prefix }) => {
                shown_name(&format!("{}{}{prefix}", path.display(), archive::SEPARATOR))
            }
            _ => self.msg("browse-places-title"),
        }
    }

    /// The name of the folder shown: its last part.
    fn location_name(&self) -> String {
        match &self.browse.location {
            Some(Location::Folder(p)) => path_name(p),
            Some(Location::Archive { path, prefix }) if prefix.is_empty() => {
                let s = path.to_string_lossy();
                s.rsplit(['/', '\\', archive::SEPARATOR])
                    .next()
                    .unwrap_or(&s)
                    .to_owned()
            }
            Some(Location::Archive { prefix, .. }) => prefix
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(prefix)
                .to_owned(),
            _ => self.msg("browse-places-title"),
        }
    }

    /// Enters folder `dir`, focused on the row for `focus` if given.
    fn enter_folder(&mut self, dir: &Path, focus: Option<&Path>) -> Vec<Effect> {
        let (rows, cut) = match read_folder(
            dir,
            &self.registry,
            self.browse.show_all,
            self.browse.extensions,
        ) {
            Ok(r) => r,
            Err(e) => {
                let reason = match e.kind() {
                    std::io::ErrorKind::PermissionDenied => self.msg("opening-no-permission"),
                    _ => e
                        .to_string()
                        .split(" (os error")
                        .next()
                        .unwrap_or_default()
                        .to_owned(),
                };
                let msg = self.msg_args(
                    "browse-folder-unreadable",
                    &args!["name" => shown_name(&path_name(dir)), "reason" => reason],
                );
                self.error(&msg);
                return self.stay();
            }
        };
        self.browse.location = Some(Location::Folder(dir.to_owned()));
        self.browse.rows = rows;
        self.browse.cut = cut;
        self.browse.filter.clear();
        self.relist(focus)
    }

    /// Enters folder `prefix` of the archive at `path`, focused on the row
    /// for `focus` if given.
    fn enter_archive(&mut self, path: &Path, prefix: &str, focus: Option<&Path>) -> Vec<Effect> {
        let cached = self.browse.archive.as_ref().is_some_and(|(p, _)| p == path);
        if !cached {
            match archive::list_path(path) {
                Ok(entries) => self.browse.archive = Some((path.to_owned(), entries)),
                Err(e) => {
                    let c = self.catalog();
                    let msg = archive_refusal(&c, &path_name(path), &e);
                    log::warn!("{}: {e}", path.display());
                    self.error(&msg);
                    return self.stay();
                }
            }
        }
        let entries = self
            .browse
            .archive
            .as_ref()
            .map(|(_, e)| e.clone())
            .unwrap_or_default();
        self.browse.cut = entries.len() > MAX_ENTRIES;
        self.browse.rows = archive_rows(
            path,
            &entries,
            prefix,
            &self.registry,
            self.browse.show_all,
            self.browse.extensions,
        );
        self.browse.location = Some(Location::Archive {
            path: path.to_owned(),
            prefix: prefix.to_owned(),
        });
        self.browse.filter.clear();
        self.relist(focus)
    }

    /// Lists the location's rows (entered, or filtered), focused on the row
    /// for `focus`, and says the location's introduction.
    fn relist(&mut self, focus: Option<&Path>) -> Vec<Effect> {
        self.relist_with(focus, true)
    }

    /// [`relist`](Self::relist); `intro` false leaves the introduction
    /// out (a new order or Show All says its own result, then the row).
    fn relist_with(&mut self, focus: Option<&Path>, intro: bool) -> Vec<Effect> {
        let pick = self.browse.pick;
        let in_folder = matches!(self.browse.location, Some(Location::Folder(_)));
        self.browse.rows.retain(|r| r.kind != RowKind::ChooseHere);
        if pick == Pick::Folder && in_folder {
            let here = match &self.browse.location {
                Some(Location::Folder(p)) => p.clone(),
                _ => PathBuf::new(),
            };
            self.browse.rows.push(Row {
                name: path_name(&here),
                kind: RowKind::ChooseHere,
                path: here,
                size: None,
                modified: None,
                hidden: false,
            });
        }
        let sort = self.browse.sort;
        let (show_all, extensions) = (self.browse.show_all, self.browse.extensions);
        let filter = self.browse.filter.to_lowercase();
        let mut hidden = 0;
        let mut shown: Vec<usize> = Vec::new();
        for (i, r) in self.browse.rows.iter().enumerate() {
            if !show_all && (!listed(&r.kind, &r.name, extensions) || r.hidden) {
                hidden += 1;
                continue;
            }
            if !filter.is_empty()
                && r.kind != RowKind::ChooseHere
                && !r.name.to_lowercase().contains(&filter)
            {
                continue;
            }
            shown.push(i);
        }
        let rows = &self.browse.rows;
        shown.sort_by(|&a, &b| compare(&rows[a], &rows[b], sort));
        self.browse.hidden = hidden;
        let focus = focus
            .and_then(|f| shown.iter().position(|&i| rows[i].path == f))
            .unwrap_or(0);
        self.browse.shown = shown;
        if !intro {
            return self.present(focus, None);
        }
        let intro = self.location_intro();
        self.present(focus, Some(&intro))
    }

    /// "Week 1, 12 items, 3 hidden." and its variants.
    fn location_intro(&self) -> String {
        let c = self.cat();
        let name = shown_name(&self.location_name());
        let n = self
            .browse
            .shown
            .iter()
            .filter(|&&i| self.browse.rows[i].kind != RowKind::ChooseHere)
            .count();
        let mut intro = if !self.browse.filter.is_empty() {
            c.fmt(
                "browse-intro-filtered",
                &args![
                    "name" => name,
                    "n" => n,
                    "count" => crate::words::grouped(c, n),
                    "filter" => shown_name(&self.browse.filter)
                ],
            )
        } else if n == 0 {
            c.fmt("browse-intro-empty", &args!["name" => name])
        } else {
            c.fmt(
                "browse-intro",
                &args!["name" => name, "n" => n, "count" => crate::words::grouped(c, n)],
            )
        };
        if let Some(Location::Archive { path, prefix }) = &self.browse.location
            && !prefix.is_empty()
        {
            let s = path.to_string_lossy();
            let archive = s
                .rsplit(['/', '\\', archive::SEPARATOR])
                .next()
                .unwrap_or(&s);
            intro = c.fmt(
                "browse-intro-in-archive",
                &args!["intro" => intro, "archive" => shown_name(archive)],
            );
        }
        if self.browse.hidden > 0 && self.browse.filter.is_empty() {
            intro = c.fmt(
                "browse-intro-hidden",
                &args![
                    "intro" => intro,
                    "n" => self.browse.hidden,
                    "count" => crate::words::grouped(c, self.browse.hidden)
                ],
            );
        }
        if self.browse.cut {
            intro = c.fmt(
                "browse-intro-cut",
                &args!["intro" => intro, "max" => crate::words::grouped(c, MAX_ENTRIES)],
            );
        }
        intro
    }

    /// The row shown at list position `n`.
    fn shown_row(&self, n: usize) -> Option<&Row> {
        self.browse.shown.get(n).map(|&i| &self.browse.rows[i])
    }

    /// The focused row's list position.
    fn browse_focus(&self) -> usize {
        self.list_model.as_ref().map_or(0, |l| l.selected)
    }

    /// Keys the browser handles itself; `None` leaves the key to the list
    /// (moving the focus).
    pub(crate) fn browse_list_key(&mut self, key: ListKey) -> Option<Vec<Effect>> {
        if self.list != Some(ListKind::Browse) {
            return None;
        }
        let n = self.browse_focus();
        Some(match key {
            ListKey::Enter => self.browse_choose(n),
            ListKey::Right => match self.shown_row(n).map(|r| r.kind.clone()) {
                Some(RowKind::Place(_) | RowKind::Folder { .. }) => self.browse_choose(n),
                Some(RowKind::Archive(_)) if self.browse.pick == Pick::Open => {
                    self.browse_choose(n)
                }
                _ => vec![Effect::Redraw],
            },
            ListKey::Backspace if !self.browse.filter.is_empty() => {
                self.browse.filter.pop();
                let focus = self.shown_row(n).map(|r| r.path.clone());
                self.relist(focus.as_deref())
            }
            ListKey::Backspace | ListKey::Left => self.browse_up(),
            ListKey::Escape => self.close_browser(true),
            ListKey::Char(c) if !c.is_control() => {
                self.browse.filter.push(c);
                self.relist(None)
            }
            ListKey::Details => self.browse_preview(n),
            ListKey::Introduce => self.browse_keys_help(),
            ListKey::ChooseHere => self.browse_choose_folder(n),
            ListKey::Sort => {
                self.browse.sort = self.browse.sort.next();
                let focus = self.shown_row(n).map(|r| r.path.clone());
                let effects = self.refresh(focus.as_deref());
                let msg = self.msg(&format!("browse-sorted-{}", self.browse.sort.id()));
                self.say_result(&msg);
                effects
            }
            ListKey::ShowAll => {
                self.browse.show_all = !self.browse.show_all;
                let focus = self.shown_row(n).map(|r| r.path.clone());
                let effects = self.refresh(focus.as_deref());
                let msg = self.msg(if self.browse.show_all {
                    "browse-showing-all"
                } else {
                    "browse-showing-readable"
                });
                self.say_result(&msg);
                effects
            }
            ListKey::Delete | ListKey::Rename => {
                let msg = self.msg("browse-read-only");
                self.tell(&msg);
                self.stay()
            }
            _ => return None,
        })
    }

    /// The location read again, with Show All or the order changed.
    fn refresh(&mut self, focus: Option<&Path>) -> Vec<Effect> {
        match self.browse.location.clone() {
            Some(Location::Folder(dir)) => {
                // Read again: a folder's count depends on Show All.
                match read_folder(
                    &dir,
                    &self.registry,
                    self.browse.show_all,
                    self.browse.extensions,
                ) {
                    Ok((rows, cut)) => {
                        self.browse.rows = rows;
                        self.browse.cut = cut;
                    }
                    Err(e) => log::warn!("{}: {e}", dir.display()),
                }
                self.relist_with(focus, false)
            }
            Some(Location::Archive { path, prefix }) => {
                let entries = self
                    .browse
                    .archive
                    .as_ref()
                    .map(|(_, e)| e.clone())
                    .unwrap_or_default();
                self.browse.rows = archive_rows(
                    &path,
                    &entries,
                    &prefix,
                    &self.registry,
                    self.browse.show_all,
                    self.browse.extensions,
                );
                self.relist_with(focus, false)
            }
            _ => self.stay(),
        }
    }

    /// Enter on shown row `n` (also a frontend's or JSON-RPC's
    /// [`Command::Choose`]).
    pub(crate) fn browse_choose(&mut self, n: usize) -> Vec<Effect> {
        self.browse.chose = n;
        let Some(row) = self.shown_row(n).cloned() else {
            return self.stay();
        };
        let pick = self.browse.pick;
        match row.kind {
            RowKind::ChooseHere => self.chosen(row.path),
            RowKind::Place(_) | RowKind::Folder { prefix: None, .. } => {
                self.enter_folder(&row.path, None)
            }
            RowKind::Folder {
                prefix: Some(prefix),
                ..
            } => match self.browse.location.clone() {
                Some(Location::Archive { path, .. }) => self.enter_archive(&path, &prefix, None),
                _ => vec![Effect::Redraw],
            },
            RowKind::Archive(_) if pick == Pick::Open => self.enter_archive(&row.path, "", None),
            RowKind::Document(_) | RowKind::File | RowKind::Archive(_) => match pick {
                Pick::File => self.chosen(row.path),
                Pick::Folder => {
                    let msg = self.msg_args(
                        "browse-choose-a-folder",
                        &args!["key" => self.browse_key_named(BrowseKey::ChooseFolder)],
                    );
                    self.tell(&msg);
                    self.stay()
                }
                Pick::Open if matches!(row.kind, RowKind::File) => {
                    let msg = self.msg_args(
                        "browse-cannot-read",
                        &args!["name" => shown_name(&row.name)],
                    );
                    self.tell(&msg);
                    self.stay()
                }
                Pick::Open => {
                    self.close_browser(false);
                    self.dispatch_inner(Command::Open(row.path))
                }
            },
        }
    }

    /// Backspace: up one folder, landing on the row left.
    fn browse_up(&mut self) -> Vec<Effect> {
        match self.browse.location.clone() {
            None | Some(Location::Places) => self.close_browser(true),
            Some(Location::Folder(dir)) => match dir.parent() {
                Some(parent) => self.enter_folder(parent, Some(&dir)),
                None => self.show_places(Some(&dir)),
            },
            Some(Location::Archive { path, prefix }) if prefix.is_empty() => {
                match archive_parent(&path) {
                    Location::Folder(dir) => self.enter_folder(&dir, Some(&path)),
                    Location::Archive {
                        path: outer,
                        prefix,
                    } => self.enter_archive(&outer, &prefix, Some(&path)),
                    Location::Places => self.show_places(None),
                }
            }
            Some(Location::Archive { path, prefix }) => {
                let trimmed = prefix.trim_end_matches('/');
                let up = match trimmed.rfind('/') {
                    Some(i) => trimmed[..=i].to_owned(),
                    None => String::new(),
                };
                let here = archive::member_path(&path, &prefix);
                self.enter_archive(&path, &up, Some(&here))
            }
        }
    }

    /// The Choose Folder key on shown row `n`.
    fn browse_choose_folder(&mut self, n: usize) -> Vec<Effect> {
        let row = self.shown_row(n).cloned();
        let target = match (&self.browse.location, &row) {
            (Some(Location::Archive { .. }), _) => None,
            (_, Some(r))
                if matches!(
                    r.kind,
                    RowKind::Place(_) | RowKind::Folder { prefix: None, .. } | RowKind::ChooseHere
                ) =>
            {
                Some(r.path.clone())
            }
            (Some(Location::Folder(dir)), _) => Some(dir.clone()),
            _ => None,
        };
        let refusal = match (self.browse.pick, &target) {
            (Pick::Folder, Some(_)) => None,
            (Pick::Folder, None) => Some(self.msg("browse-no-archive-folder")),
            (Pick::File, _) => Some(self.msg("browse-choose-a-file")),
            (Pick::Open, _) => Some(self.msg("browse-nothing-waiting")),
        };
        if let Some(msg) = refusal {
            self.tell(&msg);
            return self.stay();
        }
        match target {
            Some(t) => self.chosen(t),
            None => vec![Effect::Redraw],
        }
    }

    /// Hands `path` to the command that asked for it, closing the browser.
    fn chosen(&mut self, path: PathBuf) -> Vec<Effect> {
        let then = self.browse.then.take();
        self.close_browser(false);
        match then {
            Some(f) => f(self, path),
            None => vec![Effect::Redraw],
        }
    }

    /// Escape, or Backspace on the places: the browser closes (said when
    /// `say`), and the reader is where it was.
    pub(crate) fn close_browser(&mut self, say: bool) -> Vec<Effect> {
        if let Some(p) = self.browse.preview.take() {
            p.progress.cancel();
        }
        self.browse.location = None;
        self.browse.rows.clear();
        self.browse.shown.clear();
        self.browse.archive = None;
        self.browse.then = None;
        self.browse.purpose = None;
        self.browse.filter.clear();
        if self.list == Some(ListKind::Browse) {
            self.list = None;
        }
        self.list_model = None;
        if say {
            // Opened from a prompt for a path: back to the prompt, as it
            // was (crate::path_prompt).
            if let Some(saved) = self.browse.prompt.take() {
                return self.restore_prompt(saved, None);
            }
            let msg = self.msg("browse-closed");
            self.say_dialog(&msg);
        }
        vec![Effect::Redraw]
    }

    /// Escape in a frontend's own list dialog ([`Command::Cancel`]).
    pub(crate) fn browse_cancelled(&mut self) {
        let _ = self.close_browser(true);
    }

    /// A browser key named for a message, in both forms.
    fn browse_key_named(&self, key: BrowseKey) -> String {
        crate::help::mark_chord(self.cat(), &key.chord(command_modifier(&self.keymap)))
    }

    /// F1 in the browser: where it is, its keys, and the focused row.
    fn browse_keys_help(&mut self) -> Vec<Effect> {
        let intro = match self.browse.location {
            Some(Location::Places) => self.msg_args(
                "browse-places-intro",
                &args!["n" => self.browse.shown.len()],
            ),
            _ => self.location_intro(),
        };
        let item = self
            .list_model
            .as_ref()
            .and_then(|l| l.spoken_item_text(self.cat()))
            .unwrap_or_default();
        let msg = self.msg_args(
            "browse-keys",
            &args![
                "intro" => intro,
                "preview" => self.key(ActionId::SayStatus),
                "choose" => self.browse_key_named(BrowseKey::ChooseFolder),
                "sort" => self.browse_key_named(BrowseKey::Sort),
                "all" => self.browse_key_named(BrowseKey::ShowAll),
                "item" => item
            ],
        );
        self.tell(&msg);
        self.stay()
    }

    /// The Say Status key on shown row `n`: its preview, made on a helper
    /// thread when it reads a file, said when ready.
    fn browse_preview(&mut self, n: usize) -> Vec<Effect> {
        self.list = Some(ListKind::Browse);
        let Some(row) = self.shown_row(n).cloned() else {
            return vec![Effect::Redraw];
        };
        let c = self.catalog();
        let at_once = match &row.kind {
            RowKind::ChooseHere => Some(c.fmt(
                "browse-preview-path",
                &args!["path" => shown_name(&row.path.display().to_string())],
            )),
            RowKind::File => Some(c.fmt(
                "browse-preview-other",
                &args![
                    "name" => shown_name(&row.name),
                    "size" => row.size.map(|s| size_text(&c, s)).unwrap_or_default()
                ],
            )),
            RowKind::Folder { prefix: Some(_), items } => Some(c.fmt(
                "browse-preview-archive-folder",
                &args!["name" => shown_name(&row.name), "n" => items.unwrap_or(0), "count" => crate::words::grouped(&c, items.unwrap_or(0))],
            )),
            // A place has no preview: say the row and its place, as the
            // list says it.
            RowKind::Place(_) => Some(
                self.list_model
                    .as_ref()
                    .and_then(|l| l.spoken_item_text(&c))
                    .unwrap_or_else(|| row.text(&c, &self.location_name())),
            ),
            _ => None,
        };
        if let Some(text) = at_once {
            self.tell(&text);
            return vec![Effect::Redraw];
        }
        if let Some(old) = self.browse.preview.take() {
            old.progress.cancel();
        }
        let (tx, rx) = channel::<String>();
        let progress = Progress::default();
        let mut options = self.load_options();
        options.ocr.enabled = false;
        options.progress = progress.clone();
        let wake = self.waker_slot();
        let cat: Arc<Catalog> = self.catalog();
        let spawned = std::thread::Builder::new()
            .name("textweaver-preview".into())
            .spawn(move || {
                let text = preview_text(&cat, &row, options);
                let _ = tx.send(text);
                wake.wake();
            });
        match spawned {
            Ok(_) => {
                self.browse.preview = Some(PreviewJob {
                    rx,
                    generation: self.dialog_generation(),
                    progress,
                });
            }
            Err(e) => log::warn!("cannot start a preview: {e}"),
        }
        vec![Effect::Redraw]
    }

    /// From [`App::tick`]: says a preview that is ready, unless the list it
    /// was asked in has closed or changed.
    pub(crate) fn browse_tick(&mut self) -> Vec<Effect> {
        let Some(job) = self.browse.preview.as_ref() else {
            return Vec::new();
        };
        let text = match job.rx.try_recv() {
            Ok(text) => text,
            Err(TryRecvError::Empty) => return Vec::new(),
            Err(TryRecvError::Disconnected) => {
                self.browse.preview = None;
                return Vec::new();
            }
        };
        let generation = job.generation;
        self.browse.preview = None;
        if !text.is_empty() {
            self.announce_for(&text, Priority::Polite, Importance::Answer, generation);
        }
        vec![Effect::Redraw]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_sort_as_people_read_them() {
        let mut v = vec!["Week 10", "week 2", "Week 1", "appendix", "Week 02b"];
        v.sort_by(|a, b| natural(a, b));
        assert_eq!(v, ["appendix", "Week 1", "week 2", "Week 02b", "Week 10"]);
    }

    #[test]
    fn hostile_names_are_shown_harmlessly() {
        assert_eq!(shown_name("a\u{1b}[2Jb.md"), "a?[2Jb.md");
        assert_eq!(shown_name("photo\u{202e}gpj.exe"), "photo?gpj.exe");
        assert_eq!(shown_name("Notes, week 1.md"), "Notes, week 1.md");
    }

    #[test]
    fn sizes_read_plainly() {
        let c = Catalog::english();
        assert_eq!(size_text(&c, 1), "1 byte");
        assert_eq!(size_text(&c, 812), "812 bytes");
        assert_eq!(size_text(&c, 12 * 1024 + 100), "12 KB");
        assert_eq!(size_text(&c, 3_565_158), "3.4 MB");
        assert_eq!(size_text(&c, 5 * 1024 * 1024 * 1024), "5 GB");
    }

    #[test]
    fn the_parent_of_an_archive_in_an_archive_is_inside_the_outer_one() {
        let dir = std::env::temp_dir();
        let outer = dir.join("w6f-no-such-outer.zip");
        assert_eq!(
            archive_parent(&outer),
            Location::Places,
            "a missing archive has no parent inside"
        );
    }

    #[test]
    fn keys_follow_the_command_modifier() {
        let ctrl = BrowseKey::ChooseFolder.chord(Modifiers::CTRL);
        assert_eq!(ctrl, KeyChord::new(Key::Enter, Modifiers::CTRL));
        let cmd = BrowseKey::ShowAll.chord(Modifiers::META);
        assert_eq!(
            cmd,
            KeyChord::new(Key::Char('.'), Modifiers::META | Modifiers::SHIFT)
        );
    }
}
