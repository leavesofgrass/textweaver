//! `tw study due [FILE] [--json] [--home DIR]`: how many study cards are
//! due today (B1-f2). With a document, its cards; without one, every
//! document in the library that has cards, with the total first. Counts
//! only, never a score: due, new, and when the next card falls due. The
//! schedule is SM-2 from each card's grades
//! (`textweaver_store::schedule`). Reads only; never writes state.
//!
//! `tw study export --anki OUT.apkg [FILE] [--deck NAME]` writes the cards
//! as an Anki package, one deck per document named after it (or all in
//! `--deck`); `tw study import PACKAGE.apkg [FILE]` adds a package's cards
//! to a document's, matching each Anki deck to the library document of
//! that title when no document is named. Importing twice adds no card
//! twice (B1-f3, `textweaver_app::anki`).
//! Owner: B1-f2, B1-f3.

use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_app::anki::{ExportDeck, Package, export_apkg, read_apkg};
use textweaver_app::store::schedule::{DueCounts, due_counts};
use textweaver_app::store::{CardDeck, CardStore, DocKey, Library, Paths};

/// Arguments for `tw study`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// `tw study` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// How many study cards are due today, and how many are new.
    Due(DueArgs),
    /// Export study cards as an Anki package (.apkg).
    Export(ExportArgs),
    /// Import study cards from an Anki package (.apkg).
    Import(ImportArgs),
}

/// Arguments for `tw study export`.
#[derive(clap::Args, Debug)]
pub struct ExportArgs {
    /// Write an Anki package (.apkg) to this file.
    #[arg(long, value_name = "FILE")]
    pub anki: PathBuf,
    /// Document whose cards to export; without one, every document in the
    /// library that has cards, one deck each.
    pub file: Option<PathBuf>,
    /// Put every card in one deck of this name, instead of one deck per
    /// document named after it.
    #[arg(long, value_name = "NAME")]
    pub deck: Option<String>,
    /// Read the state under this directory (like `TEXTWEAVER_HOME`).
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Arguments for `tw study import`.
#[derive(clap::Args, Debug)]
pub struct ImportArgs {
    /// The Anki package (.apkg) to import.
    pub package: PathBuf,
    /// Document to add the cards to; without one, each Anki deck goes to
    /// the library document of the same title.
    pub file: Option<PathBuf>,
    /// Write the state under this directory (like `TEXTWEAVER_HOME`).
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Arguments for `tw study due`.
#[derive(clap::Args, Debug)]
pub struct DueArgs {
    /// Document whose cards to count; without one, every document in the
    /// library that has cards.
    pub file: Option<PathBuf>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
    /// Read the state under this directory (like `TEXTWEAVER_HOME`).
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// One document's counts.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct DocDue {
    /// The document's title, or its path when it is not in the library.
    document: String,
    cards: usize,
    due: usize,
    new: usize,
    /// Days until the next card not yet due falls due.
    #[serde(skip_serializing_if = "Option::is_none")]
    next_in_days: Option<i64>,
}

impl DocDue {
    fn new(document: String, c: DueCounts) -> Self {
        DocDue {
            document,
            cards: c.cards,
            due: c.due,
            new: c.new,
            next_in_days: c.next_in_days,
        }
    }
}

/// The whole report.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct Report {
    due: usize,
    new: usize,
    documents: Vec<DocDue>,
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// When the next card falls due, in words.
fn next_in(days: i64) -> String {
    if days <= 1 {
        "next tomorrow".to_owned()
    } else {
        format!("next in {days} days")
    }
}

/// One line per document, meaning first: "Due today: 3 cards, 1 new.
/// Pharmacology." The total comes first when there is more than one.
fn render(r: &Report) -> String {
    if r.documents.is_empty() {
        return "No study cards. Make them in the reader from notes and highlights.\n".to_owned();
    }
    let line = |d: &DocDue| {
        if d.due == 0 && d.new == 0 {
            let when = d
                .next_in_days
                .map(|n| format!(", {}", next_in(n)))
                .unwrap_or_default();
            return format!("Nothing due today{when}. {}", d.document);
        }
        format!(
            "Due today: {}, {} new. {}",
            plural(d.due, "card", "cards"),
            d.new,
            d.document
        )
    };
    let mut lines = Vec::new();
    if r.documents.len() > 1 {
        lines.push(format!(
            "Due today: {}, {} new, in {}.",
            plural(r.due, "card", "cards"),
            r.new,
            plural(r.documents.len(), "document", "documents")
        ));
    }
    lines.extend(r.documents.iter().map(line));
    lines.push(String::new());
    lines.join("\n")
}

/// Runs `tw study`.
pub fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        Command::Due(a) => due(a),
        Command::Export(a) => export(a),
        Command::Import(a) => import(a),
    }
}

fn paths_for(home: Option<&PathBuf>) -> anyhow::Result<Paths> {
    Ok(match home {
        Some(home) => Paths::under(home),
        None => Paths::platform()?,
    })
}

/// A document's title: the library's, else the file's name without its
/// extension.
fn title_of(library: &Library, file: &Path) -> String {
    library.get(file).map_or_else(
        || {
            file.file_stem().map_or_else(
                || file.display().to_string(),
                |s| s.to_string_lossy().into_owned(),
            )
        },
        |e| e.shown_title().to_owned(),
    )
}

fn export(args: ExportArgs) -> anyhow::Result<()> {
    let paths = paths_for(args.home.as_ref())?;
    let store = CardStore::new(paths.cards_dir());
    let library = Library::load(&paths.library_file()).unwrap_or_default();
    let files: Vec<PathBuf> = match &args.file {
        Some(file) => vec![file.clone()],
        None => library.entries.iter().map(|e| e.path.clone()).collect(),
    };
    let decks: Vec<(String, CardDeck)> = files
        .iter()
        .map(|f| (title_of(&library, f), store.load(&DocKey::for_path(f))))
        .filter(|(_, d)| !d.cards.is_empty())
        .collect();
    if decks.is_empty() {
        anyhow::bail!(
            "no study cards to export; make them in the reader from notes and highlights"
        );
    }
    let export: Vec<ExportDeck<'_>> = decks
        .iter()
        .map(|(title, deck)| ExportDeck {
            name: args.deck.clone().unwrap_or_else(|| title.clone()),
            document: title.clone(),
            cards: &deck.cards,
        })
        .collect();
    let bytes = export_apkg(&export, textweaver_app::store::now_ts())?;
    textweaver_app::store::atomic_write(&args.anki, &bytes)?;
    let cards: usize = decks.iter().map(|(_, d)| d.cards.len()).sum();
    let graded: usize = decks
        .iter()
        .flat_map(|(_, d)| d.cards.iter())
        .filter(|c| !c.reviews.is_empty())
        .count();
    crate::cmd::outln!(
        "Exported {} from {} to {}. {} graded, with {} schedule; {} new.",
        plural(cards, "card", "cards"),
        plural(decks.len(), "document", "documents"),
        args.anki.display(),
        graded,
        if graded == 1 { "its" } else { "their" },
        cards - graded
    );
    Ok(())
}

/// What an import did, meaning first: "Imported 120 cards. Skipped 4
/// cloze notes."
fn import_summary(pkg: &Package, added: usize, already: usize) -> String {
    let mut parts = vec![format!("Imported {}.", plural(added, "card", "cards"))];
    if already > 0 {
        parts.push(if already == 1 {
            "1 card was here already; its grades were merged.".to_owned()
        } else {
            format!("{already} cards were here already; their grades were merged.")
        });
    }
    for (n, one, many) in [
        (pkg.skipped_cloze, "cloze note", "cloze notes"),
        (
            pkg.skipped_media,
            "note that is only pictures or sound",
            "notes that are only pictures or sound",
        ),
        (
            pkg.skipped_other,
            "note with no text or with more than two cards",
            "notes with no text or with more than two cards",
        ),
    ] {
        if n > 0 {
            parts.push(format!("Skipped {}.", plural(n, one, many)));
        }
    }
    parts.join(" ")
}

/// The library document an Anki deck belongs to: the one whose title is
/// the deck's name, or its last level after `::`.
fn document_for_deck<'a>(library: &'a Library, deck: &str) -> Option<&'a Path> {
    let last = deck.rsplit("::").next().unwrap_or(deck);
    library
        .entries
        .iter()
        .find(|e| e.shown_title() == deck)
        .or_else(|| library.entries.iter().find(|e| e.shown_title() == last))
        .map(|e| e.path.as_path())
}

fn import(args: ImportArgs) -> anyhow::Result<()> {
    let paths = paths_for(args.home.as_ref())?;
    let store = CardStore::new(paths.cards_dir());
    let bytes = std::fs::read(&args.package)
        .map_err(|e| anyhow::anyhow!("{}: {e}", args.package.display()))?;
    let pkg = read_apkg(&bytes, textweaver_app::store::now_ts())?;
    // Where each card goes, decided before anything is written.
    let mut targets: Vec<(PathBuf, Vec<textweaver_app::store::Card>)> = Vec::new();
    let library = Library::load(&paths.library_file()).unwrap_or_default();
    for c in &pkg.cards {
        let file = match &args.file {
            Some(f) => f.clone(),
            None => match document_for_deck(&library, &c.deck) {
                Some(p) => p.to_owned(),
                None => anyhow::bail!(
                    "no document in the library is titled \"{}\", the Anki deck's name; \
                     name the document: tw study import {} FILE",
                    c.deck,
                    args.package.display()
                ),
            },
        };
        match targets.iter_mut().find(|(f, _)| *f == file) {
            Some((_, cards)) => cards.push(c.card.clone()),
            None => targets.push((file, vec![c.card.clone()])),
        }
    }
    let (mut added, mut already) = (0, 0);
    for (file, cards) in targets {
        let key = DocKey::for_path(&file);
        let mut deck = store.load(&key);
        let r = deck.add_imported(cards);
        added += r.added;
        already += r.already;
        store.save(&key, &deck)?;
    }
    crate::cmd::outln!("{}", import_summary(&pkg, added, already));
    Ok(())
}

fn due(args: DueArgs) -> anyhow::Result<()> {
    let paths = match &args.home {
        Some(home) => Paths::under(home),
        None => Paths::platform()?,
    };
    let store = CardStore::new(paths.cards_dir());
    let now = textweaver_app::store::now_ts();
    let library = Library::load(&paths.library_file()).unwrap_or_default();
    let mut documents = Vec::new();
    match &args.file {
        Some(file) => {
            let deck = store.load(&DocKey::for_path(file));
            let name = library.get(file).map_or_else(
                || file.display().to_string(),
                |e| e.shown_title().to_owned(),
            );
            if !deck.cards.is_empty() {
                documents.push(DocDue::new(name, due_counts(&deck.cards, now)));
            }
        }
        // shortcut: only documents in the library are counted; a deck
        // whose document left the library is not, as its title is gone.
        None => {
            for e in &library.entries {
                let deck = store.load(&DocKey::for_path(&e.path));
                if !deck.cards.is_empty() {
                    let name = e.shown_title().to_owned();
                    documents.push(DocDue::new(name, due_counts(&deck.cards, now)));
                }
            }
        }
    }
    let report = Report {
        due: documents.iter().map(|d| d.due).sum(),
        new: documents.iter().map(|d| d.new).sum(),
        documents,
    };
    if args.json {
        crate::cmd::outln!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        crate::cmd::out!("{}", render(&report));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(name: &str, due: usize, new: usize, next: Option<i64>) -> DocDue {
        DocDue {
            document: name.into(),
            cards: due + new + 1,
            due,
            new,
            next_in_days: next,
        }
    }

    #[test]
    fn import_says_the_count_then_what_was_skipped() {
        let pkg = Package {
            skipped_cloze: 4,
            skipped_media: 1,
            ..Package::default()
        };
        assert_eq!(
            import_summary(&pkg, 120, 0),
            "Imported 120 cards. Skipped 4 cloze notes. \
             Skipped 1 note that is only pictures or sound."
        );
        assert_eq!(
            import_summary(&Package::default(), 0, 2),
            "Imported 0 cards. 2 cards were here already; their grades were merged."
        );
    }

    #[test]
    fn lines_put_the_count_first() {
        let r = Report {
            due: 3,
            new: 1,
            documents: vec![
                doc("Pharmacology", 3, 1, None),
                doc("Anatomy", 0, 0, Some(3)),
            ],
        };
        assert_eq!(
            render(&r),
            "Due today: 3 cards, 1 new, in 2 documents.\n\
             Due today: 3 cards, 1 new. Pharmacology\n\
             Nothing due today, next in 3 days. Anatomy\n"
        );
        let one = Report {
            due: 1,
            new: 0,
            documents: vec![doc("Week 4", 1, 0, None)],
        };
        assert_eq!(render(&one), "Due today: 1 card, 0 new. Week 4\n");
        let none = Report {
            due: 0,
            new: 0,
            documents: Vec::new(),
        };
        assert!(render(&none).starts_with("No study cards."));
    }
}
