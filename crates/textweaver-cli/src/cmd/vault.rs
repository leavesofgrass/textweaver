//! `tw vault`: export documents' notes and highlights to an Obsidian vault,
//! or import a vault's notes. Owner: Agent J (wave 2).
//!
//! Beta 4 adds the vault index's questions, which only read: `backlinks`
//! (the notes that link to a note), `unresolved` (links to notes that
//! don't exist, of a note or a whole vault), `tags` (every tag with its
//! count, or with `--tag` the notes carrying one), and `properties` (a
//! note's front matter and its Dataview `key::` fields).

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;
use textweaver_app::store::{Paths, StateStore};
use textweaver_vault::{
    AnnotationStore, ExportDocument, ExportOptions, ImportMode, ImportOptions, LibraryEntry,
    LinkScope, RelationType, StateStoreAnnotations, VaultIndex, apply, export_documents,
    inline_fields, read_vault, save_library, split_front_matter,
};

/// Arguments for `tw vault`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// `import` or `export`; or, reading only, `backlinks`, `unresolved`,
    /// `tags`, or `properties`.
    #[arg(value_parser = ["import", "export", "backlinks", "unresolved", "tags", "properties"])]
    pub action: String,
    /// The Obsidian vault folder; for `backlinks` and `properties`, a note
    /// in it; for `unresolved` and `tags`, a note or a folder.
    pub vault: PathBuf,
    /// The tag whose notes to list (tags only), with or without `#`.
    #[arg(long)]
    pub tag: Option<String>,
    /// Document whose notes and highlights to export (export only; repeat
    /// for several documents).
    #[arg(long)]
    pub document: Vec<PathBuf>,
    /// Folder inside the vault for new notes (export only).
    #[arg(long)]
    pub folder: Option<PathBuf>,
    /// Do not write a document note per document (export only).
    #[arg(long)]
    pub no_document_notes: bool,
    /// What to import: `graph` (documents, notes, links) or `library`
    /// (documents only).
    #[arg(long, default_value = "graph", value_parser = ["graph", "library"])]
    pub mode: String,
    /// The relation for plain `[[links]]` (import only), for example
    /// `SEE_ALSO` or `supports`.
    #[arg(long)]
    pub link_relation: Option<String>,
    /// Read the vault and report, without storing anything (import only).
    #[arg(long)]
    pub dry_run: bool,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
    /// Use the files under this folder instead of the usual place.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// What `tw vault export` reports per document.
#[derive(Debug, Serialize)]
struct DocSummary {
    path: PathBuf,
    title: String,
    notes: usize,
    highlights: usize,
    loaded: bool,
}

/// A document with its annotations and text, owned for export.
struct Loaded {
    path: PathBuf,
    title: String,
    text: Option<String>,
    annotations: textweaver_vault::DocAnnotations,
}

fn load_document(path: &Path, store: &mut StateStoreAnnotations) -> anyhow::Result<Loaded> {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    let annotations = store.load(&path)?;
    let doc = textweaver_app::formats::load_path(&path).ok();
    let title = doc
        .as_ref()
        .and_then(|d| d.meta.title.clone())
        .filter(|t| !t.trim().is_empty())
        // Some loaders fall back to the file name; the stem reads better.
        .filter(|t| {
            path.file_name()
                .is_none_or(|n| n.to_string_lossy() != t.as_str())
        })
        .unwrap_or_else(|| {
            path.file_stem().map_or_else(
                || path.display().to_string(),
                |s| s.to_string_lossy().into_owned(),
            )
        });
    let text = doc.map(|d| d.text().to_string());
    Ok(Loaded {
        path,
        title,
        text,
        annotations,
    })
}

fn export(args: &Args, paths: &Paths) -> anyhow::Result<()> {
    if args.document.is_empty() {
        anyhow::bail!("Name the documents to export with --document FILE");
    }
    let mut store = StateStoreAnnotations::new(StateStore::new(paths.state_dir()));
    let mut loaded = Vec::new();
    let mut summaries = Vec::new();
    for d in &args.document {
        let l = load_document(d, &mut store)?;
        summaries.push(DocSummary {
            path: l.path.clone(),
            title: l.title.clone(),
            notes: l.annotations.notes.len(),
            highlights: l.annotations.highlights.len(),
            loaded: l.text.is_some(),
        });
        loaded.push(l);
    }
    let with_notes: Vec<&Loaded> = loaded
        .iter()
        .filter(|l| !l.annotations.is_empty())
        .collect();
    let docs: Vec<ExportDocument<'_>> = with_notes
        .iter()
        .map(|l| ExportDocument {
            path: &l.path,
            title: &l.title,
            text: l.text.as_deref(),
            annotations: &l.annotations,
        })
        .collect();
    let options = ExportOptions {
        folder: args.folder.clone(),
        document_notes: !args.no_document_notes,
    };
    let report = if docs.is_empty() {
        None
    } else {
        Some(export_documents(&args.vault, &docs, &options)?)
    };
    if args.json {
        #[derive(Serialize)]
        struct Out<'a> {
            documents: &'a [DocSummary],
            #[serde(skip_serializing_if = "Option::is_none")]
            export: Option<&'a textweaver_vault::ExportReport>,
        }
        crate::cmd::outln!(
            "{}",
            serde_json::to_string_pretty(&Out {
                documents: &summaries,
                export: report.as_ref(),
            })?
        );
        return Ok(());
    }
    for s in &summaries {
        if s.notes == 0 && s.highlights == 0 {
            crate::cmd::outln!("{} has no notes or highlights.", s.title);
        } else if !s.loaded {
            crate::cmd::outln!(
                "{} could not be opened, so highlights are exported without their text.",
                s.title
            );
        }
    }
    match report {
        Some(r) => crate::cmd::outln!("{}", r.summary()),
        None => crate::cmd::outln!("Nothing to export."),
    }
    Ok(())
}

fn import(args: &Args, paths: &Paths) -> anyhow::Result<()> {
    let link_relation = match &args.link_relation {
        Some(name) => Some(RelationType::parse(name).with_context(|| {
            let names: Vec<&str> = RelationType::ALL.iter().map(|r| r.as_str()).collect();
            format!(
                "Unknown relation {name:?}; choose one of {}",
                names.join(", ")
            )
        })?),
        None => None,
    };
    let options = ImportOptions {
        mode: ImportMode::parse(&args.mode).unwrap_or_default(),
        link_relation,
    };
    let read = read_vault(&args.vault, &options)?;
    if args.dry_run {
        if args.json {
            crate::cmd::outln!("{}", serde_json::to_string_pretty(&read)?);
        } else {
            let mut store = textweaver_vault::MemoryStore::new();
            let report = apply(&read, &mut store)?;
            crate::cmd::outln!("Dry run, nothing stored. {}", report.summary());
        }
        return Ok(());
    }
    let mut store = StateStoreAnnotations::new(StateStore::new(paths.state_dir()));
    let report = apply(&read, &mut store)?;
    let library: Vec<LibraryEntry> = store.library().to_vec();
    // The documents the import registered go into the library (both
    // modes), so they are on the bookshelf and in library searches.
    save_library(&library, &paths.library_file()).with_context(|| {
        format!(
            "The notes were imported, but the library {} could not be updated",
            paths.library_file().display()
        )
    })?;
    if args.json {
        #[derive(Serialize)]
        struct Out<'a> {
            import: &'a textweaver_vault::ImportReport,
            library: &'a [LibraryEntry],
        }
        crate::cmd::outln!(
            "{}",
            serde_json::to_string_pretty(&Out {
                import: &report,
                library: &library,
            })?
        );
    } else {
        crate::cmd::outln!("{}", report.summary());
        for (path, why) in &read.unreadable {
            crate::cmd::outln!("Could not read {}: {why}", path.display());
        }
    }
    Ok(())
}

/// The index of the vault holding `path` (a note or a folder), or of
/// the folder itself when no `.obsidian` is above it.
fn index_for(path: &Path) -> anyhow::Result<VaultIndex> {
    if !path.exists() {
        anyhow::bail!("{} was not found", path.display());
    }
    let scope = if path.is_dir() {
        LinkScope::for_folder(path)
    } else {
        LinkScope::for_note(path)
    };
    Ok(VaultIndex::build(scope))
}

/// Said after a list when the index hit a size limit.
const PARTIAL: &str = "Only part of the vault was searched; it is very large.";

/// A note for text output: its title, then its folder in the vault.
fn note_line(index: &VaultIndex, path: &Path) -> String {
    let rel = index.scope().relative(path);
    let folder = rel.rsplit_once('/').map_or("top folder", |(f, _)| f);
    format!("{}, in {folder}", index.title_of(path))
}

/// Checks the path names a note, for the commands about one note.
fn note_arg(args: &Args) -> anyhow::Result<()> {
    if !args.vault.is_file() {
        anyhow::bail!("Name a note file, not a folder: {}", args.vault.display());
    }
    Ok(())
}

/// "1 note" or "2 notes".
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// `tw vault backlinks NOTE`.
fn backlinks(args: &Args) -> anyhow::Result<()> {
    note_arg(args)?;
    let index = index_for(&args.vault)?;
    let back = index.backlinks(&args.vault);
    if args.json {
        #[derive(Serialize)]
        struct Row {
            path: PathBuf,
            title: String,
            links: usize,
        }
        #[derive(Serialize)]
        struct Out<'a> {
            note: &'a Path,
            vault: &'a Path,
            partial: bool,
            backlinks: Vec<Row>,
        }
        let rows = back
            .iter()
            .map(|b| Row {
                path: b.from.clone(),
                title: index.title_of(&b.from),
                links: b.links,
            })
            .collect();
        crate::cmd::outln!(
            "{}",
            serde_json::to_string_pretty(&Out {
                note: &args.vault,
                vault: index.scope().root(),
                partial: index.is_partial(),
                backlinks: rows,
            })?
        );
        return Ok(());
    }
    let name = index.title_of(&args.vault);
    if back.is_empty() {
        crate::cmd::outln!("No notes link to {name}.");
    } else {
        let n = count(back.len(), "note links", "notes link");
        crate::cmd::outln!("{n} to {name}:");
        for b in &back {
            let links = count(b.links, "link", "links");
            crate::cmd::outln!("{}, {links}", note_line(&index, &b.from));
        }
    }
    if index.is_partial() {
        crate::cmd::outln!("{PARTIAL}");
    }
    Ok(())
}

/// `tw vault unresolved PATH`: a note's links, or a whole vault's.
fn unresolved(args: &Args) -> anyhow::Result<()> {
    let index = index_for(&args.vault)?;
    let note = args.vault.is_file().then_some(args.vault.as_path());
    let missing = index.unresolved(note);
    if args.json {
        #[derive(Serialize)]
        struct Out<'a> {
            vault: &'a Path,
            partial: bool,
            unresolved: &'a [textweaver_vault::Unresolved],
        }
        crate::cmd::outln!(
            "{}",
            serde_json::to_string_pretty(&Out {
                vault: index.scope().root(),
                partial: index.is_partial(),
                unresolved: &missing,
            })?
        );
        return Ok(());
    }
    if missing.is_empty() {
        crate::cmd::outln!("Every link leads to a note.");
    } else {
        let n = count(missing.len(), "link", "links");
        crate::cmd::outln!("{n} to notes that don't exist:");
        for u in &missing {
            crate::cmd::outln!("{}, from {}", u.target, note_line(&index, &u.from));
        }
    }
    if index.is_partial() {
        crate::cmd::outln!("{PARTIAL}");
    }
    Ok(())
}

/// `tw vault tags PATH [--tag NAME]`.
fn tags(args: &Args) -> anyhow::Result<()> {
    let index = index_for(&args.vault)?;
    if let Some(tag) = &args.tag {
        let notes = index.notes_with_tag(tag);
        if args.json {
            crate::cmd::outln!("{}", serde_json::to_string_pretty(&notes)?);
            return Ok(());
        }
        let tag = tag.trim().trim_start_matches('#');
        if notes.is_empty() {
            crate::cmd::outln!("No notes carry the tag {tag}.");
        } else {
            crate::cmd::outln!("Tag {tag}, {}:", count(notes.len(), "note", "notes"));
            for p in &notes {
                crate::cmd::outln!("{}", note_line(&index, p));
            }
        }
    } else {
        let tags = index.tags();
        if args.json {
            crate::cmd::outln!("{}", serde_json::to_string_pretty(&tags)?);
            return Ok(());
        }
        if tags.is_empty() {
            crate::cmd::outln!("No tags in this vault.");
        }
        for t in &tags {
            crate::cmd::outln!("{}, {}", t.tag, count(t.notes, "note", "notes"));
        }
    }
    if index.is_partial() {
        crate::cmd::outln!("{PARTIAL}");
    }
    Ok(())
}

/// `tw vault properties NOTE`: front matter, then Dataview fields.
fn properties(args: &Args) -> anyhow::Result<()> {
    note_arg(args)?;
    let bytes = std::fs::read(&args.vault)
        .with_context(|| format!("Could not read {}", args.vault.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let (fm, body) = split_front_matter(&text);
    let props = fm.properties();
    let fields = inline_fields(body);
    if args.json {
        #[derive(Serialize)]
        struct Out<'a> {
            properties: &'a [(String, String)],
            fields: &'a [(String, String)],
        }
        crate::cmd::outln!(
            "{}",
            serde_json::to_string_pretty(&Out {
                properties: &props,
                fields: &fields,
            })?
        );
        return Ok(());
    }
    if props.is_empty() && fields.is_empty() {
        crate::cmd::outln!("This note has no properties.");
        return Ok(());
    }
    if !props.is_empty() {
        crate::cmd::outln!("Properties: {}", props.len());
        for (k, v) in &props {
            crate::cmd::outln!("{k}: {v}");
        }
    }
    if !fields.is_empty() {
        crate::cmd::outln!("Dataview fields: {}", fields.len());
        for (k, v) in &fields {
            crate::cmd::outln!("{k}: {v}");
        }
    }
    Ok(())
}

/// Runs `tw vault`.
pub fn run(args: Args) -> anyhow::Result<()> {
    match args.action.as_str() {
        "backlinks" => return backlinks(&args),
        "unresolved" => return unresolved(&args),
        "tags" => return tags(&args),
        "properties" => return properties(&args),
        _ => {}
    }
    let paths = super::paths(args.home.as_deref())?;
    match args.action.as_str() {
        "export" => export(&args, &paths),
        _ => import(&args, &paths),
    }
}
