//! `tw study due [FILE] [--json] [--home DIR]`: how many study cards are
//! due today (B1-f2). With a document, its cards; without one, every
//! document in the library that has cards, with the total first. Counts
//! only, never a score: due, new, and when the next card falls due. The
//! schedule is SM-2 from each card's grades
//! (`textweaver_store::schedule`). Reads only; never writes state.
//! Owner: B1-f2.

use std::path::PathBuf;

use serde::Serialize;
use textweaver_app::store::schedule::{DueCounts, due_counts};
use textweaver_app::store::{CardStore, DocKey, Library, Paths};

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
    }
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
