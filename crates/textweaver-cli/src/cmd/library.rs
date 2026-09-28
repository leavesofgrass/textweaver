//! `tw library`: the library's folders and documents, adding and removing
//! folders, and searching titles, authors, DOIs, ISBNs, and text. Owner:
//! Agent C; search by metadata, Wave 5 (W5y).
//!
//! A document's author, DOI, and ISBN come from the bookshelf (recorded
//! when it opens), from a DOI or ISBN near the start of its indexed text,
//! and from `tw cite`'s record of the same work: a reference in the
//! personal library or a library folder's `references.json` with the same
//! DOI, ISBN, or title fills what the document lacks.

use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_app::store::fulltext::{FullTextIndex, SearchHit, SimpleIndex};
use textweaver_app::store::library::{self, DocMetadata, LibraryItem, ScannedDoc};
use textweaver_app::store::{
    DocKey, Library, Paths, Recent, Settings, SettingsStore, StateStore, sync::SidecarStore,
};
use textweaver_cite::Reference;

/// Arguments for `tw library`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Search the library: titles, paths, authors, DOIs, ISBNs, and
    /// document text.
    #[arg(long)]
    pub search: Option<String>,
    /// Add a folder to the library.
    #[arg(long)]
    pub add: Option<PathBuf>,
    /// Remove a folder from the library (its files are not touched).
    #[arg(long)]
    pub remove: Option<PathBuf>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
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
    let sidecars = SidecarStore::new(settings.reading.sync_conflict_policy);
    let states = StateStore::new(paths.state_dir());
    let local = |p: &Path| {
        states
            .load(&DocKey::for_path(p))
            .filter(|s| s.has_position())
            .map(|s| s.pct)
    };
    let mut items = library::library_view(&scanned, &lib, &recent, &sidecars, &local);
    let refs = references(paths, &settings.library.folders);
    enrich(
        &mut items,
        &SimpleIndex::load(&paths.fulltext_file()),
        &refs,
    );
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
    // A DOI or ISBN in text indexed just now counts too.
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

fn render_listing(l: &Listing) -> String {
    let mut out = String::new();
    if l.folders.is_empty() {
        out.push_str("No library folders. Add one with tw library --add FOLDER.\n");
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

fn run_with(args: &Args, paths: &Paths) -> anyhow::Result<String> {
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
    if let Some(q) = args.search.as_deref().map(str::trim) {
        anyhow::ensure!(!q.is_empty(), "the search text is empty");
        let r = search(paths, &settings, q);
        out.push_str(&render_search(&r));
        json.insert("search".into(), serde_json::to_value(&r)?);
    } else if args.add.is_none() && args.remove.is_none() {
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

/// Runs `tw library`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let paths = Paths::platform()?;
    print!("{}", run_with(&args, &paths)?);
    Ok(())
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
            json: false,
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
