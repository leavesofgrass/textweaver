//! `tw vault`: export documents' notes and highlights to an Obsidian vault,
//! or import a vault's notes. Owner: Agent J (wave 2).

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;
use textweaver_app::store::{Paths, StateStore};
use textweaver_vault::{
    AnnotationStore, ExportDocument, ExportOptions, ImportMode, ImportOptions, LibraryEntry,
    RelationType, StateStoreAnnotations, apply, export_documents, read_vault, save_library,
};

/// Arguments for `tw vault`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// `import` or `export`.
    #[arg(value_parser = ["import", "export"])]
    pub action: String,
    /// The Obsidian vault folder.
    pub vault: PathBuf,
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

/// Runs `tw vault`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let paths = super::paths(args.home.as_deref())?;
    match args.action.as_str() {
        "export" => export(&args, &paths),
        _ => import(&args, &paths),
    }
}
