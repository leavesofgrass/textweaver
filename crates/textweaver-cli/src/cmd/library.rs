//! `tw library`: the library's folders and documents, adding and removing
//! folders, searching titles, authors, DOIs, ISBNs, and text, and
//! "Continue reading" (`--continue`). Owner: Agent C; search by metadata,
//! Wave 5 (W5y); sync, the sync wave (S6).
//!
//! With sync on, the details other computers published (ADR-0049) fill in
//! a document's author, DOI, and ISBN here too, so a search finds a
//! document by a DOI only another computer knows.
//!
//! A document's author, DOI, and ISBN come from the bookshelf (recorded
//! when it opens), from a DOI or ISBN near the start of its indexed text,
//! and from `tw cite`'s record of the same work: a reference in the
//! personal library or a library folder's `references.json` with the same
//! DOI, ISBN, or title fills what the document lacks.
//!
//! `tw library edit FILE --title ... --author ... --doi ... --isbn ...`
//! (Wave 7, W7m) sets a document's details by hand: they win over the
//! document's own, and with sync on they reach the other computers, newest
//! wins per field. An empty value (`--author ""`) clears the hand edit, so
//! the document's own value shows again.
//!
//! `tw library queue` (beta 3) lists the reading queue, and `add FILE`,
//! `remove FILE`, and `next` change and read it; the queue is the reader's,
//! kept in `library.json`. `tw library review` prints the review list: the
//! documents read before, the one read longest ago first.

use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::fulltext::{FullTextIndex, SearchHit, SimpleIndex};
use textweaver_app::store::library::{
    self, DetailField, DocMetadata, EditedDetails, LibraryItem, ScannedDoc,
};
use textweaver_app::store::{
    DocKey, Library, Paths, Recent, Settings, SettingsStore, StateStore, sync::SidecarStore,
};
use textweaver_app::synced_library::{ContinueItem, SyncedLibrary, continue_reading};
use textweaver_cite::Reference;

/// Arguments for `tw library`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The old spelling of `tw library search TEXT`, kept hidden through
    /// beta 1.
    #[arg(long, hide = true)]
    pub search: Option<String>,
    /// The old spelling of `tw library add FOLDER`, kept hidden through
    /// beta 1.
    #[arg(long, hide = true)]
    pub add: Option<PathBuf>,
    /// The old spelling of `tw library remove FOLDER`, kept hidden through
    /// beta 1.
    #[arg(long, hide = true)]
    pub remove: Option<PathBuf>,
    /// The old spelling of `tw library continue`, kept hidden through
    /// beta 1.
    #[arg(long = "continue", hide = true)]
    pub continue_reading: bool,
    /// Print JSON.
    #[arg(long, global = true)]
    pub json: bool,
    /// Use the files under this folder instead of the usual place.
    #[arg(long, global = true, value_name = "DIR")]
    pub home: Option<PathBuf>,
    /// What to do instead of listing.
    #[command(subcommand)]
    pub command: Option<LibraryCommand>,
}

/// `tw library` commands.
#[derive(clap::Subcommand, Debug)]
pub enum LibraryCommand {
    /// List the library's folders and documents (what `tw library` alone
    /// prints).
    List,
    /// Add a folder to the library.
    Add {
        /// The folder.
        folder: PathBuf,
    },
    /// Remove a folder from the library (its files are not touched).
    Remove {
        /// The folder.
        folder: PathBuf,
    },
    /// Search the library: titles, paths, authors, DOIs, ISBNs, and
    /// document text.
    Search {
        /// The text to find.
        text: String,
    },
    /// Continue reading: the documents on this computer with a place saved
    /// here or on another computer (with sync on), newest first.
    Continue,
    /// Edit a document's details by hand: its title, author, DOI, and ISBN.
    /// An empty value clears your edit, so the document's own shows again.
    Edit(EditArgs),
    /// The reading queue: the documents to read next, in order. Without a
    /// command, lists it.
    Queue {
        /// What to do instead of listing.
        #[command(subcommand)]
        command: Option<QueueCommand>,
    },
    /// The review list: the documents you have read, the one read longest
    /// ago first, with how long ago, how far, and how many notes. A
    /// convenience for coming back to old readings, not a study method.
    Review,
}

/// `tw library queue` commands.
#[derive(clap::Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum QueueCommand {
    /// List the queue, in order (what `tw library queue` alone prints).
    List,
    /// Add a document to the end of the queue.
    Add {
        /// The document.
        file: PathBuf,
    },
    /// Take a document out of the queue (the file is not touched).
    Remove {
        /// The document.
        file: PathBuf,
    },
    /// Print the next document to read: the first in the queue whose file
    /// is there.
    Next,
}

/// Arguments for `tw library edit`.
#[derive(clap::Args, Debug, Default)]
pub struct EditArgs {
    /// The document.
    pub file: PathBuf,
    /// The title.
    #[arg(long)]
    pub title: Option<String>,
    /// The author or authors.
    #[arg(long)]
    pub author: Option<String>,
    /// The DOI, such as 10.1000/xyz or a doi.org link.
    #[arg(long)]
    pub doi: Option<String>,
    /// The ISBN, with or without hyphens.
    #[arg(long)]
    pub isbn: Option<String>,
}

/// Most text matches listed.
const SEARCH_LIMIT: usize = 50;

/// Extensions textweaver can open.
fn supported() -> impl Fn(&str) -> bool {
    let exts = textweaver_app::formats::Registry::with_builtins().extensions();
    move |ext: &str| exts.contains(&ext)
}

/// A folder change.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct FolderChange {
    folder: PathBuf,
    changed: bool,
    documents: usize,
    message: String,
}

fn add_folder(paths: &Paths, folder: &Path) -> anyhow::Result<FolderChange> {
    anyhow::ensure!(folder.is_dir(), "{} is not a folder", folder.display());
    let store = SettingsStore::new(paths.clone());
    let (mut settings, _) = store.load();
    let (stored, added) = settings.library.add_folder(folder);
    if added {
        store.save(&settings)?;
    }
    let documents =
        library::scan_folder(&stored, true, library::MAX_SCAN_FILES, &supported()).len();
    let name = stored.file_name().map_or_else(
        || stored.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let message = if added {
        let a = textweaver_app::a11y::Announcement::LibraryFolderAdded { name, documents };
        a.text(textweaver_app::a11y::Verbosity::Normal)
            .unwrap_or_default()
    } else {
        format!("{name} is already in the library")
    };
    Ok(FolderChange {
        folder: stored,
        changed: added,
        documents,
        message,
    })
}

fn remove_folder(paths: &Paths, folder: &Path) -> anyhow::Result<FolderChange> {
    let store = SettingsStore::new(paths.clone());
    let (mut settings, _) = store.load();
    let removed = settings.library.remove_folder(folder);
    if removed {
        store.save(&settings)?;
    }
    let shown = library::resolve_path(folder);
    Ok(FolderChange {
        message: if removed {
            format!(
                "Removed {} from the library; its files are unchanged",
                shown.display()
            )
        } else {
            format!("{} is not a library folder", shown.display())
        },
        folder: shown,
        changed: removed,
        documents: 0,
    })
}

/// The library as listed and searched.
#[derive(Debug, Serialize)]
struct Listing {
    folders: Vec<PathBuf>,
    items: Vec<LibraryItem>,
}

fn listing(paths: &Paths, settings: &Settings) -> (Listing, Vec<ScannedDoc>) {
    let scanned = library::scan_library(&settings.library.folders, &supported());
    let lib = Library::load(&paths.library_file()).unwrap_or_default();
    let recent = Recent::load(&paths.recent_file());
    let sidecars = SidecarStore::new(settings.sync.position_policy.conflict_policy());
    let states = StateStore::new(paths.state_dir());
    let local = |p: &Path| {
        states
            .load(&DocKey::for_path(p))
            .filter(|s| s.has_position())
            .map(|s| s.pct)
    };
    let mut items = library::library_view(&scanned, &lib, &recent, &sidecars, &local);
    let refs = references(paths, &settings.library.folders);
    let index = SimpleIndex::load(&paths.fulltext_file());
    // What other computers know first (S6): an author, DOI, or ISBN found
    // only there makes the search find the document here too.
    enrich_synced(&mut items, paths, settings, &index);
    enrich(&mut items, &index, &refs);
    (
        Listing {
            folders: settings.library.folders.clone(),
            items,
        },
        scanned,
    )
}

/// What `--search` found.
#[derive(Debug, Serialize)]
struct SearchReport {
    query: String,
    titles: Vec<LibraryItem>,
    text: Vec<SearchHit>,
    indexed: usize,
    unreadable: Vec<PathBuf>,
}

fn search(paths: &Paths, settings: &Settings, query: &str) -> SearchReport {
    let (mut list, scanned) = listing(paths, settings);
    // Index the folder documents and the recent ones that still exist.
    let mut docs = scanned;
    for item in &list.items {
        if item.source == library::ItemSource::Recent
            && let Ok(meta) = std::fs::metadata(&item.path)
            && meta.is_file()
        {
            docs.push(ScannedDoc {
                path: item.path.clone(),
                rel: String::new(),
                title: item.title.clone(),
                ext: String::new(),
                size: meta.len(),
                mtime: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0)),
                folder: PathBuf::new(),
            });
        }
    }
    let cache = paths.fulltext_file();
    let mut index = SimpleIndex::load(&cache);
    let refresh = index.refresh(&docs, &mut |p: &Path| {
        textweaver_app::formats::load_path(p)
            .ok()
            .map(|d| d.text().to_string())
    });
    if refresh.changed() {
        // Only a cache: a failed write costs a slower next search.
        let _ = index.save(&cache);
    }
    // A DOI or ISBN in text indexed just now counts too, and so does a
    // document another computer knows by the same text.
    enrich_synced(&mut list.items, paths, settings, &index);
    enrich(&mut list.items, &index, &[]);
    let titles = library::filter_items(&list.items, query)
        .into_iter()
        .cloned()
        .collect();
    SearchReport {
        query: query.to_owned(),
        titles,
        text: index.search(query, SEARCH_LIMIT),
        indexed: index.len(),
        unreadable: refresh.failed,
    }
}

/// The references `tw cite` keeps: the personal library, then each
/// library folder's `references.json`. Unreadable files are skipped.
fn references(paths: &Paths, folders: &[PathBuf]) -> Vec<Reference> {
    std::iter::once(textweaver_cite::user_library_path(&paths.data_dir))
        .chain(
            folders
                .iter()
                .map(|f| textweaver_cite::folder_library_path(f)),
        )
        .filter_map(|p| textweaver_cite::Library::load(&p).ok())
        .flat_map(|l| l.references().to_vec())
        .collect()
}

/// A title reduced to lowercase letters and digits, for matching a
/// document to its reference.
fn title_key(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Titles shorter than this (in letters and digits) are too common to
/// match a document to a reference by title alone ("Notes", "Chapter 1").
const TITLE_MATCH_MIN: usize = 12;

/// What a reference says: its authors, DOI, and ISBN.
fn reference_meta(r: &Reference) -> DocMetadata {
    let authors: Vec<String> = r
        .author
        .iter()
        .map(|n| n.natural_form())
        .filter(|n| !n.is_empty())
        .collect();
    DocMetadata {
        author: (!authors.is_empty()).then(|| authors.join(", ")),
        doi: r.doi.as_deref().and_then(library::normalize_doi),
        isbn: r.isbn.as_deref().and_then(library::normalize_isbn),
    }
}

/// True when `r` is the work `item` is: the same DOI or ISBN, or the same
/// title when it is long enough to be distinctive.
fn same_work(item: &LibraryItem, r: &Reference, meta: &DocMetadata) -> bool {
    if let (Some(a), Some(b)) = (&item.meta.doi, &meta.doi) {
        return a == b;
    }
    if let (Some(a), Some(b)) = (&item.meta.isbn, &meta.isbn) {
        return library::isbn13(a) == library::isbn13(b);
    }
    let mine = title_key(&item.title);
    mine.len() >= TITLE_MATCH_MIN
        && r.title
            .as_deref()
            .is_some_and(|t| title_key(&textweaver_cite::text::plain_title(t)) == mine)
}

/// Fills each item's missing author, DOI, and ISBN: first a DOI or ISBN
/// near the start of its indexed text, then `tw cite`'s record of the same
/// work.
fn enrich(items: &mut [LibraryItem], index: &SimpleIndex, refs: &[Reference]) {
    let metas: Vec<DocMetadata> = refs.iter().map(reference_meta).collect();
    for item in items {
        if let Some(e) = index.entries.get(&item.path) {
            let head: String = e.text.chars().take(library::METADATA_SCAN_CHARS).collect();
            item.meta.fill_from(&DocMetadata::from_document(
                None,
                &Default::default(),
                &head,
            ));
        }
        if let Some((_, m)) = refs.iter().zip(&metas).find(|(r, m)| same_work(item, r, m)) {
            item.meta.fill_from(m);
        }
    }
}

/// Adds the library details other computers published (with sync on).
fn enrich_synced(
    items: &mut [LibraryItem],
    paths: &Paths,
    settings: &Settings,
    index: &SimpleIndex,
) {
    if let Some(s) = SyncedLibrary::load(paths, settings) {
        s.enrich(items, &|p| index.entries.get(p).map(|e| e.text.clone()));
    }
}

/// `--continue`: the documents found here, each at its newest place from
/// any computer, newest first.
fn continue_list(paths: &Paths, settings: &Settings) -> Vec<ContinueItem> {
    let (l, _) = listing(paths, settings);
    let index = SimpleIndex::load(&paths.fulltext_file());
    let synced = SyncedLibrary::load(paths, settings);
    continue_reading(&l.items, Some(&paths.state_dir()), synced.as_ref(), &|p| {
        index.entries.get(p).map(|e| e.text.clone())
    })
}

fn render_continue(items: &[ContinueItem], c: &Catalog) -> String {
    if items.is_empty() {
        return format!("{}\n", c.tr("continue-empty"));
    }
    let now = textweaver_app::sync_folder::wall_ms();
    let mut out = format!("{}\n", c.fmt("continue-intro", &args!["n" => items.len()]));
    for i in items {
        out.push_str(&format!(
            "  {}\n    {}\n",
            i.describe(c, now),
            i.path.display()
        ));
    }
    out
}

fn render_listing(l: &Listing) -> String {
    let mut out = String::new();
    if l.folders.is_empty() {
        out.push_str("No library folders. Add one with tw library add FOLDER.\n");
    } else {
        out.push_str(&format!(
            "{} library folder{}:\n",
            l.folders.len(),
            if l.folders.len() == 1 { "" } else { "s" }
        ));
        for f in &l.folders {
            out.push_str(&format!("  {}\n", f.display()));
        }
    }
    if l.items.is_empty() {
        out.push_str("No documents yet.\n");
    } else {
        out.push_str(&format!("{}:\n", documents(l.items.len())));
        for i in &l.items {
            out.push_str(&format!("  {}\n    {}\n", i.describe(), i.path.display()));
        }
    }
    out
}

/// "1 document", "2 documents".
fn documents(n: usize) -> String {
    if n == 1 {
        "1 document".to_owned()
    } else {
        format!("{n} documents")
    }
}

fn render_search(r: &SearchReport) -> String {
    let mut out = String::new();
    if r.titles.is_empty() {
        out.push_str(&format!(
            "No titles, authors, DOIs, or ISBNs match {}.\n",
            r.query
        ));
    } else {
        out.push_str(&format!(
            "{} matching {} by title, author, DOI, or ISBN:\n",
            documents(r.titles.len()),
            r.query
        ));
        for i in &r.titles {
            out.push_str(&format!("  {}\n    {}\n", i.describe(), i.path.display()));
        }
    }
    if r.text.is_empty() {
        out.push_str(&format!(
            "No text matches for {} in {}.\n",
            r.query,
            documents(r.indexed)
        ));
    } else {
        out.push_str(&format!(
            "Text matches in {} of {}:\n",
            r.text.len(),
            documents(r.indexed)
        ));
        for h in &r.text {
            out.push_str(&format!("  {}\n    {}\n", h.describe(), h.path.display()));
        }
    }
    if !r.unreadable.is_empty() {
        out.push_str(&format!(
            "{} could not be read for searching.\n",
            documents(r.unreadable.len())
        ));
    }
    out
}

/// What `tw library edit` did.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct EditReport {
    /// The document, as the bookshelf keeps it.
    path: PathBuf,
    /// The bookshelf changed.
    changed: bool,
    /// The fields given, by name.
    fields: Vec<&'static str>,
    /// The edit was written to the sync folder (sync on).
    published: bool,
    /// Every hand-edited detail of the document now (this computer's).
    edited: EditedDetails,
    /// What was said.
    message: String,
}

fn edit(paths: &Paths, settings: &Settings, a: &EditArgs) -> anyhow::Result<EditReport> {
    use textweaver_app::sync_engine::{EngineConfig, SyncEngine};
    use textweaver_app::sync_folder::{DetailEdit, Identify, docid::Details};

    anyhow::ensure!(a.file.is_file(), "{} is not a file", a.file.display());
    let (c, _) = Catalog::for_language(&settings.interface.language, Some(&paths.locales_dir()));
    let mut edits = Vec::new();
    for (field, value) in [
        (DetailField::Title, &a.title),
        (DetailField::Author, &a.author),
        (DetailField::Doi, &a.doi),
        (DetailField::Isbn, &a.isbn),
    ] {
        let Some(value) = value else {
            continue;
        };
        match field.clean(value) {
            Ok(v) => edits.push((field, v)),
            Err(bad) => {
                let id = if field == DetailField::Doi {
                    "details-not-a-doi"
                } else {
                    "details-not-an-isbn"
                };
                anyhow::bail!("{}", c.fmt(id, &args!["text" => bad]));
            }
        }
    }
    anyhow::ensure!(
        !edits.is_empty(),
        "nothing to edit: give --title, --author, --doi, or --isbn"
    );
    let file = paths.library_file();
    let mut lib = Library::load(&file)?;
    let changed = lib.record_edits(&a.file, &edits, textweaver_app::sync_folder::wall_ms());
    if changed {
        lib.save(&file)?;
    }
    let mut published = false;
    if let Some(config) = EngineConfig::from_settings(&settings.sync, paths) {
        let mut engine = SyncEngine::new();
        engine.configure(Some(config));
        let identify = Identify {
            ids_file: paths.sync_ids_file(),
            path: a.file.clone(),
            library_folders: settings.library.folders.clone(),
            details: Details::default(),
        };
        let sync_edits: Vec<DetailEdit> = edits
            .iter()
            .map(|(f, v)| DetailEdit {
                name: f.name(),
                value: v.clone(),
            })
            .collect();
        published = engine.edit_details(&identify, &sync_edits);
    }
    let entry = lib.get(&a.file);
    let mut edited = entry.map(|e| e.edited.clone()).unwrap_or_default();
    edited.at_ms = 0;
    let message = if changed || published {
        let labels: Vec<String> = edits
            .iter()
            .map(|(f, _)| c.tr(&format!("details-label-{}", f.name())))
            .collect();
        c.fmt("details-saved", &args!["fields" => labels.join(", ")])
    } else {
        c.tr("details-unchanged")
    };
    Ok(EditReport {
        path: library::resolve_path(&a.file),
        changed,
        fields: edits.iter().map(|(f, _)| f.name()).collect(),
        published,
        edited,
        message,
    })
}

/// One document in the reading queue.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct QueuedDoc {
    /// The document here; absent for one another computer queued that is
    /// not found here.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    title: String,
    /// The file is there.
    found: bool,
}

fn queued_docs(lib: &Library) -> Vec<QueuedDoc> {
    lib.queue
        .iter()
        .map(|q| QueuedDoc {
            path: q.path.clone(),
            title: lib.queue_title(q),
            found: q.path.as_deref().is_some_and(Path::is_file),
        })
        .collect()
}

fn render_queue(docs: &[QueuedDoc]) -> String {
    if docs.is_empty() {
        return "The reading queue is empty. Add a document with tw library queue add FILE.\n"
            .to_owned();
    }
    let mut out = format!("Reading queue, {}, in order:\n", documents(docs.len()));
    for d in docs {
        let Some(path) = &d.path else {
            let title = if d.title.is_empty() {
                "A document from another computer"
            } else {
                d.title.as_str()
            };
            out.push_str(&format!("  {title}, not on this computer\n"));
            continue;
        };
        let missing = if d.found { "" } else { ", not found" };
        out.push_str(&format!("  {}{missing}\n    {}\n", d.title, path.display()));
    }
    out
}

/// `tw library queue`: lists the queue, or changes it.
fn queue(
    paths: &Paths,
    settings: &Settings,
    command: Option<&QueueCommand>,
    json: bool,
) -> anyhow::Result<String> {
    let file = paths.library_file();
    let mut lib = Library::load(&file)?;
    let (c, _) = Catalog::for_language(&settings.interface.language, Some(&paths.locales_dir()));
    let (text, value) = match command {
        None | Some(QueueCommand::List) => {
            let docs = queued_docs(&lib);
            (render_queue(&docs), serde_json::json!({ "queue": docs }))
        }
        Some(QueueCommand::Add { file: doc }) => {
            anyhow::ensure!(doc.is_file(), "{} is not a file", doc.display());
            let added = lib.queue_add(doc);
            if added {
                lib.save(&file)?;
            }
            let title = lib.title_of(doc);
            let msg = if added {
                c.fmt(
                    "queue-added",
                    &args!["title" => title.as_str(), "n" => lib.queue.len()],
                )
            } else {
                format!("{title} is in the reading queue already.")
            };
            let value = serde_json::json!({
                "added": { "path": library::resolve_path(doc), "changed": added, "message": msg },
                "queue": queued_docs(&lib),
            });
            (format!("{msg}\n"), value)
        }
        Some(QueueCommand::Remove { file: doc }) => {
            let removed = lib.queue_remove(doc);
            if removed {
                lib.save(&file)?;
            }
            let title = lib.title_of(doc);
            let msg = if removed {
                c.fmt("queue-removed", &args!["title" => title.as_str()])
            } else {
                format!("{title} is not in the reading queue.")
            };
            let value = serde_json::json!({
                "removed": { "path": library::resolve_path(doc), "changed": removed, "message": msg },
                "queue": queued_docs(&lib),
            });
            (format!("{msg}\n"), value)
        }
        Some(QueueCommand::Next) => {
            let next = lib.queue_next(None).map(|p| QueuedDoc {
                path: Some(p.to_owned()),
                title: lib.title_of(p),
                found: true,
            });
            let text = match next.as_ref().and_then(|d| d.path.as_ref()) {
                Some(p) => format!("{}\n", p.display()),
                None => "The reading queue has nothing to read next.\n".to_owned(),
            };
            (text, serde_json::json!({ "next": next }))
        }
    };
    Ok(if json {
        format!("{}\n", serde_json::to_string_pretty(&value)?)
    } else {
        text
    })
}

/// `tw library review`: the review list.
fn review(paths: &Paths, settings: &Settings, json: bool) -> anyhow::Result<String> {
    use textweaver_app::store::ReadingStats;
    let (c, _) = Catalog::for_language(&settings.interface.language, Some(&paths.locales_dir()));
    let stats = ReadingStats::load(paths).unwrap_or_default();
    let states = StateStore::new(paths.state_dir());
    let rows = stats.review(textweaver_app::store::now_ts(), &Path::is_file, &|key| {
        states
            .load(&DocKey(key.to_owned()))
            .map_or(0, |s| s.notes.len())
    });
    if json {
        return Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({ "review": rows }))?
        ));
    }
    if rows.is_empty() {
        let id = if settings.stats.enabled {
            "review-empty"
        } else {
            "review-stats-off"
        };
        return Ok(format!("{}\n", c.tr(id)));
    }
    let mut out = format!(
        "Review list, {}, read longest ago first:\n",
        documents(rows.len())
    );
    for r in &rows {
        let line = c.fmt(
            "review-item",
            &args![
                "title" => r.title.as_str(),
                "days" => c.fmt("review-days", &args!["n" => r.days]),
                "pct" => r.percent,
                "notes" => c.fmt("review-notes", &args!["n" => r.notes])
            ],
        );
        out.push_str(&format!("  {line}\n    {}\n", r.path.display()));
    }
    Ok(out)
}

fn run_with(args: &Args, paths: &Paths) -> anyhow::Result<String> {
    match &args.command {
        Some(LibraryCommand::Queue { command }) => {
            let settings = SettingsStore::new(paths.clone()).load().0;
            return queue(paths, &settings, command.as_ref(), args.json);
        }
        Some(LibraryCommand::Review) => {
            let settings = SettingsStore::new(paths.clone()).load().0;
            return review(paths, &settings, args.json);
        }
        _ => {}
    }
    if let Some(LibraryCommand::Edit(a)) = &args.command {
        let settings = SettingsStore::new(paths.clone()).load().0;
        let r = edit(paths, &settings, a)?;
        return Ok(if args.json {
            format!(
                "{}
",
                serde_json::to_string_pretty(&serde_json::json!({ "edit": r }))?
            )
        } else {
            format!(
                "{}
",
                r.message
            )
        });
    }
    let mut out = String::new();
    let mut json = serde_json::Map::new();
    if let Some(folder) = &args.add {
        let c = add_folder(paths, folder)?;
        out.push_str(&format!("{}\n", c.message));
        json.insert("added".into(), serde_json::to_value(&c)?);
    }
    if let Some(folder) = &args.remove {
        let c = remove_folder(paths, folder)?;
        out.push_str(&format!("{}\n", c.message));
        json.insert("removed".into(), serde_json::to_value(&c)?);
    }
    let settings = SettingsStore::new(paths.clone()).load().0;
    if args.continue_reading {
        let (c, _) =
            Catalog::for_language(&settings.interface.language, Some(&paths.locales_dir()));
        let items = continue_list(paths, &settings);
        out.push_str(&render_continue(&items, &c));
        json.insert("continue".into(), serde_json::to_value(&items)?);
    }
    if let Some(q) = args.search.as_deref().map(str::trim) {
        anyhow::ensure!(!q.is_empty(), "the search text is empty");
        let r = search(paths, &settings, q);
        out.push_str(&render_search(&r));
        json.insert("search".into(), serde_json::to_value(&r)?);
    } else if args.add.is_none() && args.remove.is_none() && !args.continue_reading {
        let (l, _) = listing(paths, &settings);
        out.push_str(&render_listing(&l));
        json.insert("library".into(), serde_json::to_value(&l)?);
    }
    if args.json {
        Ok(format!("{}\n", serde_json::to_string_pretty(&json)?))
    } else {
        Ok(out)
    }
}

/// Turns a verb (`tw library add FOLDER`) into the fields its old flag
/// spelling (`--add FOLDER`) set, so both run the same way.
fn normalize(mut args: Args) -> Args {
    match args.command.take() {
        Some(LibraryCommand::Add { folder }) => args.add = Some(folder),
        Some(LibraryCommand::Remove { folder }) => args.remove = Some(folder),
        Some(LibraryCommand::Search { text }) => args.search = Some(text),
        Some(LibraryCommand::Continue) => args.continue_reading = true,
        Some(LibraryCommand::List) | None => {}
        own @ Some(
            LibraryCommand::Edit(_) | LibraryCommand::Queue { .. } | LibraryCommand::Review,
        ) => args.command = own,
    }
    args
}

/// Runs `tw library`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let args = normalize(args);
    let paths = super::paths(args.home.as_deref())?;
    super::print_all(&run_with(&args, &paths)?)
}

#[cfg(test)]
mod tests {
    use textweaver_app::core::CharPos;
    use textweaver_app::store::DocState;

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let p = std::env::temp_dir()
                .join(format!("tw-library-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn args() -> Args {
        Args {
            search: None,
            add: None,
            remove: None,
            continue_reading: false,
            json: false,
            home: None,
            command: None,
        }
    }

    #[test]
    fn one_document_is_singular() {
        assert_eq!(documents(1), "1 document");
        assert_eq!(documents(0), "0 documents");
        assert_eq!(documents(2), "2 documents");
    }

    #[test]
    fn add_list_search_and_remove() {
        let dir = TempDir::new("flow");
        let paths = Paths::under(&dir.0.join("tw"));
        let lib = dir.0.join("Readings");
        std::fs::create_dir_all(lib.join("week1")).unwrap();
        std::fs::write(
            lib.join("cells.md"),
            "# Cells\n\nThe mitochondria makes energy.\n",
        )
        .unwrap();
        std::fs::write(
            lib.join("week1").join("notes.txt"),
            "Energy, energy, and more energy.",
        )
        .unwrap();
        std::fs::write(lib.join("picture.png"), "not a document").unwrap();

        let out = run_with(
            &Args {
                add: Some(lib.clone()),
                ..args()
            },
            &paths,
        )
        .unwrap();
        assert_eq!(out, "Added folder Readings with 2 documents\n");
        let again = run_with(
            &Args {
                add: Some(lib.clone()),
                ..args()
            },
            &paths,
        )
        .unwrap();
        assert!(again.contains("already in the library"), "{again}");

        // A local reading position shows as progress.
        let mut st = DocState::default();
        st.set_position(CharPos(10), 40);
        StateStore::new(paths.state_dir())
            .save(&DocKey::for_path(&lib.join("cells.md")), &st)
            .unwrap();
        let listed = run_with(&args(), &paths).unwrap();
        assert!(listed.contains("1 library folder:"), "{listed}");
        assert!(listed.contains("2 documents:"), "{listed}");
        assert!(
            listed.contains("cells, 25 percent, in Readings"),
            "{listed}"
        );
        assert!(listed.contains("notes, in Readings"), "{listed}");

        let found = run_with(
            &Args {
                search: Some("ENERGY".into()),
                ..args()
            },
            &paths,
        )
        .unwrap();
        assert!(
            found.contains("No titles, authors, DOIs, or ISBNs match ENERGY."),
            "{found}"
        );
        assert!(
            found.contains("Text matches in 2 of 2 documents:"),
            "{found}"
        );
        let notes_line = found.find("notes, 3 matches").unwrap();
        let cells_line = found.find("cells, 1 match").unwrap();
        assert!(notes_line < cells_line, "most matches first: {found}");
        assert!(paths.fulltext_file().exists());

        let json = run_with(
            &Args {
                search: Some("week1".into()),
                json: true,
                ..args()
            },
            &paths,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            v["search"]["titles"].as_array().unwrap().len(),
            1,
            "paths match too"
        );

        let removed = run_with(
            &Args {
                remove: Some(lib.clone()),
                ..args()
            },
            &paths,
        )
        .unwrap();
        assert!(removed.starts_with("Removed "), "{removed}");
        assert!(lib.join("cells.md").exists());
        let empty = run_with(&args(), &paths).unwrap();
        assert!(empty.contains("No library folders."), "{empty}");
    }

    /// Wave 5 (W5y): search by DOI, ISBN, and author, from the text and
    /// from `tw cite`'s record of the same work.
    #[test]
    fn search_by_doi_isbn_and_author() {
        let dir = TempDir::new("meta");
        let paths = Paths::under(&dir.0.join("tw"));
        let lib = dir.0.join("Papers");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(
            lib.join("paper.md"),
            "# Cells and Energy\n\nPublished as https://doi.org/10.1000/XYZ.\n",
        )
        .unwrap();
        std::fs::write(
            lib.join("book.txt"),
            "A Long Book About Reading\n\nISBN 978-0-306-40615-7\n",
        )
        .unwrap();
        // tw cite's record of the book, by its ISBN.
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        std::fs::write(
            textweaver_cite::user_library_path(&paths.data_dir),
            r#"[{"id": "example2020", "type": "book", "title": "A Long Book About Reading",
                "author": [{"family": "Example", "given": "Ada"}], "ISBN": "0-306-40615-2"}]"#,
        )
        .unwrap();
        run_with(
            &Args {
                add: Some(lib.clone()),
                ..args()
            },
            &paths,
        )
        .unwrap();
        let search = |q: &str| {
            run_with(
                &Args {
                    search: Some(q.into()),
                    ..args()
                },
                &paths,
            )
            .unwrap()
        };
        let found = search("10.1000/xyz");
        assert!(
            found.contains("1 document matching 10.1000/xyz by title, author, DOI, or ISBN:"),
            "{found}"
        );
        assert!(found.contains("paper"), "{found}");
        let found = search("doi:10.1000/XYZ");
        assert!(found.contains("1 document matching"), "{found}");
        let found = search("9780306406157");
        assert!(found.contains("book, by Ada Example"), "{found}");
        let found = search("Ada Example");
        assert!(found.contains("1 document matching"), "{found}");
        assert!(found.contains("book, by Ada Example"), "{found}");
        let listed = run_with(&args(), &paths).unwrap();
        assert!(
            listed.contains("book, by Ada Example, in Papers"),
            "{listed}"
        );
        let json = run_with(
            &Args {
                search: Some("0306406152".into()),
                json: true,
                ..args()
            },
            &paths,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["search"]["titles"][0]["isbn"], "9780306406157");
        assert_eq!(v["search"]["titles"][0]["author"], "Ada Example");
    }

    /// The sync wave (S6): another computer (the "laptop") published a
    /// paper's DOI, author, and title, and its place and reading, into the
    /// sync folder. This home (the "lab") has the same library folder,
    /// synced with its id file, and never opened the paper.
    fn laptop_published_a_paper(dir: &Path) -> (Paths, PathBuf) {
        use textweaver_app::sync_folder::docid::{self, Details, Fingerprint};
        use textweaver_app::sync_folder::record::Place;
        use textweaver_app::sync_folder::{Clock, DocRecord, Identity, SyncFolder, SyncId};

        let folder = dir.join("sync");
        std::fs::create_dir_all(&folder).unwrap();
        let lib_a = dir.join("laptop").join("Papers");
        std::fs::create_dir_all(&lib_a).unwrap();
        std::fs::write(lib_a.join("paper.md"), "The mitochondria makes energy.\n").unwrap();
        let lib_id = docid::library_id(&lib_a).unwrap();
        let laptop = Paths::under(&dir.join("laptop-home"));
        let (mut identity, _) = Identity::load_or_create(&laptop.data_dir).unwrap();
        let sync = SyncFolder::open(&folder, &mut identity, "laptop", "test")
            .unwrap()
            .folder;
        let mut clock = Clock::new(sync.device());
        let mut record = DocRecord::new(SyncId::random());
        record.publish_identity(
            clock.tick(),
            &Fingerprint {
                library_key: Some(docid::library_key(lib_id, "paper.md")),
                ..Fingerprint::default()
            },
            &Details {
                title: Some("Cell Energy".into()),
                doi: Some("10.1000/xyz".into()),
                author: Some("Ada Example".into()),
                ..Details::default()
            },
        );
        record.set_place(
            clock.tick(),
            Place {
                pos: CharPos(10),
                pct: 42,
                anchor: None,
            },
        );
        record.stats.seconds.add(sync.device(), 120);
        record.stats.sessions.add(sync.device(), 2);
        sync.write_doc(&record).unwrap();

        // The lab: the same folder, arrived with its id file.
        let lib_b = dir.join("lab").join("Course papers");
        for rel in ["paper.md", ".textweaver/library-id.json"] {
            let to = lib_b.join(rel);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(lib_a.join(rel), to).unwrap();
        }
        let lab = Paths::under(&dir.join("lab-home"));
        let store = SettingsStore::new(lab.clone());
        let (mut settings, _) = store.load();
        settings.library.add_folder(&lib_b);
        settings.sync.enabled = true;
        settings.sync.folder = Some(folder);
        settings.sync.device_name = "lab".into();
        store.save(&settings).unwrap();
        (lab, lib_b.join("paper.md"))
    }

    #[test]
    fn a_doi_known_on_another_computer_is_found_and_continue_lists_it() {
        let dir = TempDir::new("synced");
        let (lab, paper) = laptop_published_a_paper(&dir.0);
        let found = run_with(
            &Args {
                search: Some("doi:10.1000/XYZ".into()),
                ..args()
            },
            &lab,
        )
        .unwrap();
        assert!(
            found.contains("1 document matching doi:10.1000/XYZ by title, author, DOI, or ISBN:"),
            "{found}"
        );
        assert!(
            found.contains("Cell Energy, by Ada Example, in Course papers"),
            "{found}"
        );

        let cont = run_with(
            &Args {
                continue_reading: true,
                ..args()
            },
            &lab,
        )
        .unwrap();
        assert!(
            cont.starts_with("Continue reading: 1 document, newest first.\n"),
            "{cont}"
        );
        assert!(
            cont.contains("  Cell Energy, 42 percent, laptop, just now\n"),
            "{cont}"
        );
        // Paths are shown resolved (long names on Windows, symbolic links
        // followed), as the library stores them.
        let shown = library::resolve_path(&paper);
        assert!(cont.contains(&shown.display().to_string()), "{cont}");
        let json = run_with(
            &Args {
                continue_reading: true,
                json: true,
                ..args()
            },
            &lab,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["continue"][0]["pct"], 42);
        assert_eq!(v["continue"][0]["device"], "laptop");
        assert_eq!(v["continue"][0]["this_computer"], false);

        // With sync off, nothing from the laptop is used.
        let store = SettingsStore::new(lab.clone());
        let (mut settings, _) = store.load();
        settings.sync.enabled = false;
        store.save(&settings).unwrap();
        let found = run_with(
            &Args {
                search: Some("10.1000/xyz".into()),
                ..args()
            },
            &lab,
        )
        .unwrap();
        assert!(
            found.contains("No titles, authors, DOIs, or ISBNs match"),
            "{found}"
        );
        let cont = run_with(
            &Args {
                continue_reading: true,
                ..args()
            },
            &lab,
        )
        .unwrap();
        assert_eq!(cont, "No places yet. Reading saves your place.\n");
    }

    fn edit_args(file: &Path) -> EditArgs {
        EditArgs {
            file: file.to_owned(),
            ..EditArgs::default()
        }
    }

    fn run_edit(paths: &Paths, a: EditArgs, json: bool) -> anyhow::Result<String> {
        run_with(
            &Args {
                json,
                command: Some(LibraryCommand::Edit(a)),
                ..args()
            },
            paths,
        )
    }

    /// Wave 7 (W7m): `tw library edit` sets a document's details by hand;
    /// they survive, win over the document's own, and are searched; a bad
    /// DOI is refused; an empty value clears the edit.
    #[test]
    fn edit_sets_details_by_hand() {
        let dir = TempDir::new("edit");
        let paths = Paths::under(&dir.0.join("tw"));
        let lib = dir.0.join("Readings");
        std::fs::create_dir_all(&lib).unwrap();
        let doc = lib.join("scan0042.txt");
        std::fs::write(&doc, "Cells divide by mitosis.\n").unwrap();
        run_with(
            &Args {
                add: Some(lib.clone()),
                ..args()
            },
            &paths,
        )
        .unwrap();

        let out = run_edit(
            &paths,
            EditArgs {
                title: Some("Cell Biology, Chapter 3".into()),
                author: Some("Ada  Example".into()),
                doi: Some("https://doi.org/10.1000/CELLS".into()),
                ..edit_args(&doc)
            },
            false,
        )
        .unwrap();
        assert_eq!(out, "Details saved: Title, Author, DOI.\n");
        let again = run_edit(
            &paths,
            EditArgs {
                title: Some("Cell Biology, Chapter 3".into()),
                ..edit_args(&doc)
            },
            false,
        )
        .unwrap();
        assert_eq!(again, "Details not changed.\n");

        let listed = run_with(&args(), &paths).unwrap();
        assert!(
            listed.contains("Cell Biology, Chapter 3, by Ada Example, in Readings"),
            "{listed}"
        );
        for q in ["chapter 3", "doi:10.1000/cells"] {
            let found = run_with(
                &Args {
                    search: Some(q.into()),
                    ..args()
                },
                &paths,
            )
            .unwrap();
            assert!(found.contains("1 document matching"), "{q}: {found}");
        }

        let err = run_edit(
            &paths,
            EditArgs {
                isbn: Some("12345".into()),
                ..edit_args(&doc)
            },
            false,
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "Not an ISBN: 12345. Fix it or clear it.");
        assert!(
            run_edit(&paths, edit_args(&doc), false).is_err(),
            "nothing given"
        );
        assert!(run_edit(&paths, edit_args(&dir.0.join("nope.txt")), false).is_err());

        let json = run_edit(
            &paths,
            EditArgs {
                author: Some(String::new()),
                ..edit_args(&doc)
            },
            true,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["edit"]["changed"], true);
        assert_eq!(v["edit"]["fields"][0], "author");
        assert_eq!(v["edit"]["published"], false, "sync is off");
        assert_eq!(v["edit"]["edited"]["title"], "Cell Biology, Chapter 3");
        assert!(v["edit"]["edited"].get("author").is_none(), "cleared");
        let listed = run_with(&args(), &paths).unwrap();
        assert!(
            listed.contains("Cell Biology, Chapter 3, in Readings"),
            "{listed}"
        );
    }

    /// With sync on, the edit reaches the other computers, where it wins
    /// over the details the document published.
    #[test]
    fn an_edit_is_published_to_the_sync_folder() {
        use textweaver_app::sync_folder::FolderView;
        use textweaver_app::sync_folder::record::detail;

        let dir = TempDir::new("edit-sync");
        let (lab, paper) = laptop_published_a_paper(&dir.0);
        let json = run_edit(
            &lab,
            EditArgs {
                title: Some("Cell Energy, revised".into()),
                ..edit_args(&paper)
            },
            true,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["edit"]["published"], true, "{json}");
        let view = FolderView::read(&dir.0.join("sync")).unwrap();
        let shown: Vec<_> = view
            .docs
            .values()
            .filter_map(|r| r.identity.shown_detail(detail::TITLE))
            .collect();
        assert_eq!(shown, vec!["Cell Energy, revised"], "one record, edited");
        let found = run_with(
            &Args {
                search: Some("revised".into()),
                ..args()
            },
            &lab,
        )
        .unwrap();
        assert!(
            found.contains("Cell Energy, revised, by Ada Example, in Course papers"),
            "{found}"
        );
    }

    fn queue_cmd(paths: &Paths, command: Option<QueueCommand>, json: bool) -> String {
        run_with(
            &Args {
                json,
                command: Some(LibraryCommand::Queue { command }),
                ..args()
            },
            paths,
        )
        .unwrap()
    }

    /// Beta 3 (B1-s3): `tw library queue` adds, lists, names the next
    /// document, and removes; a file is never touched.
    #[test]
    fn the_queue_adds_lists_names_the_next_and_removes() {
        let dir = TempDir::new("queue");
        let paths = Paths::under(&dir.0.join("tw"));
        let week3 = dir.0.join("week3.md");
        let week4 = dir.0.join("week4.md");
        std::fs::write(&week3, "# Week 3\n").unwrap();
        std::fs::write(&week4, "# Week 4\n").unwrap();

        assert_eq!(
            queue_cmd(&paths, None, false),
            "The reading queue is empty. Add a document with tw library queue add FILE.\n"
        );
        assert_eq!(
            queue_cmd(&paths, Some(QueueCommand::Next), false),
            "The reading queue has nothing to read next.\n"
        );
        let added = queue_cmd(
            &paths,
            Some(QueueCommand::Add {
                file: week3.clone(),
            }),
            false,
        );
        assert_eq!(
            added,
            "Added to the reading queue: week3. 1 document in the queue.\n"
        );
        queue_cmd(
            &paths,
            Some(QueueCommand::Add {
                file: week4.clone(),
            }),
            false,
        );
        let again = queue_cmd(
            &paths,
            Some(QueueCommand::Add {
                file: week4.clone(),
            }),
            false,
        );
        assert_eq!(again, "week4 is in the reading queue already.\n");
        let listed = queue_cmd(&paths, Some(QueueCommand::List), false);
        assert!(
            listed.starts_with("Reading queue, 2 documents, in order:\n  week3\n"),
            "{listed}"
        );
        let next = queue_cmd(&paths, Some(QueueCommand::Next), false);
        assert_eq!(
            next.trim_end(),
            library::resolve_path(&week3).display().to_string()
        );
        let json: serde_json::Value = serde_json::from_str(&queue_cmd(&paths, None, true)).unwrap();
        assert_eq!(json["queue"][1]["title"], "week4");
        assert_eq!(json["queue"][1]["found"], true);

        let removed = queue_cmd(
            &paths,
            Some(QueueCommand::Remove {
                file: week3.clone(),
            }),
            false,
        );
        assert_eq!(removed, "Removed from the reading queue: week3.\n");
        assert!(week3.is_file(), "the file is not touched");
        let gone = queue_cmd(&paths, Some(QueueCommand::Remove { file: week3 }), false);
        assert_eq!(gone, "week3 is not in the reading queue.\n");
        let lib = Library::load(&paths.library_file()).unwrap();
        assert_eq!(
            lib.queue_paths().collect::<Vec<_>>(),
            [library::resolve_path(&week4).as_path()]
        );

        let missing = run_with(
            &Args {
                command: Some(LibraryCommand::Queue {
                    command: Some(QueueCommand::Add {
                        file: dir.0.join("nope.md"),
                    }),
                }),
                ..args()
            },
            &paths,
        );
        assert!(missing.is_err());
    }

    /// Beta 3 (B1-s3): `tw library review` lists documents read before,
    /// the one read longest ago first.
    #[test]
    fn review_lists_the_oldest_reading_first() {
        use textweaver_app::store::{ReadingStats, StatsDelta};
        let dir = TempDir::new("review");
        let paths = Paths::under(&dir.0.join("tw"));
        let review_args = |json| Args {
            json,
            command: Some(LibraryCommand::Review),
            ..args()
        };
        let empty = run_with(&review_args(false), &paths).unwrap();
        assert!(empty.starts_with("Nothing to review yet."), "{empty}");
        let doc = dir.0.join("pharm2.md");
        std::fs::write(&doc, "# Pharmacology 2\n").unwrap();
        let mut st = DocState::default();
        for _ in 0..3 {
            st.add_note(textweaver_app::core::CharRange::new(0, 1), "P", "n", "");
        }
        StateStore::new(paths.state_dir())
            .save(&DocKey::for_path(&doc), &st)
            .unwrap();
        let now = textweaver_app::store::now_ts();
        ReadingStats::add_to_file(
            &paths,
            &[StatsDelta {
                key: DocKey::for_path(&doc).0,
                title: "Pharmacology 2".into(),
                path: Some(doc.clone()),
                seconds: 30.0,
                new_session: true,
                furthest_percent: 80,
                furthest_char: 10,
                at: now - 21 * 86_400 - 60,
            }],
        )
        .unwrap();
        let out = run_with(&review_args(false), &paths).unwrap();
        assert!(
            out.starts_with(
                "Review list, 1 document, read longest ago first:\n  Pharmacology 2, 21 days, 80 percent, 3 notes\n"
            ),
            "{out}"
        );
        let json: serde_json::Value =
            serde_json::from_str(&run_with(&review_args(true), &paths).unwrap()).unwrap();
        assert_eq!(json["review"][0]["days"], 21);
        assert_eq!(json["review"][0]["notes"], 3);
    }

    #[test]
    fn adding_a_missing_folder_fails() {
        let dir = TempDir::new("missing");
        let paths = Paths::under(&dir.0);
        let err = run_with(
            &Args {
                add: Some(dir.0.join("nope")),
                ..args()
            },
            &paths,
        );
        assert!(err.is_err());
        assert!(!paths.config_dir.exists());
    }
}
