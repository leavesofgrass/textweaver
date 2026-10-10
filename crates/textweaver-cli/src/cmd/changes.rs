//! `tw changes`: a document's tracked changes and comment threads, one per
//! line, meaning first ("Inserted: 'renal', by Ada Example, Tuesday, March
//! 3, 2026"), from the same list the reader shows (`textweaver_app::changes`).
//! `--json` prints the changes and comments as the loaders recorded them.
//! `tw changes accept FILE` or `tw changes reject FILE` with `--out FILE`
//! writes the document with every change decided (the older spellings,
//! `--accept-all` and `--reject-all`, still work, hidden, through beta 1): Markdown (`.md`), plain text (`.txt`), HTML,
//! or a writer's format (`.docx`, `.epub`, `.pdf`, `.brf`), leaving the
//! original alone. With `--in-place` instead, a Word file is changed
//! itself (`textweaver_writers::docx_update`): a copy of the original is
//! kept beside it first (`report-original.docx`), and everything else in
//! the package stays as Word wrote it. Owners: tasks B1-t1 and B1-t2.

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use serde_json::json;
use textweaver_app::changes::{change_row, comment_row, resolve_all};
use textweaver_app::formats::{self, changes, comments};
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::{Paths, SettingsStore};
use textweaver_app::text::Document;

/// Arguments for `tw changes`.
#[derive(clap::Args, Debug, Default)]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
pub struct Args {
    /// The document whose changes and comments to list.
    #[arg(required = true)]
    pub file: Option<PathBuf>,
    /// Print JSON instead of one row per line.
    #[arg(long)]
    pub json: bool,
    /// The old spelling of `tw changes accept FILE`, kept hidden through
    /// beta 1.
    #[arg(long, hide = true, conflicts_with_all = ["reject_all", "json"])]
    pub accept_all: bool,
    /// The old spelling of `tw changes reject FILE`, kept hidden through
    /// beta 1.
    #[arg(long, hide = true, conflicts_with = "json")]
    pub reject_all: bool,
    /// With the old --accept-all or --reject-all, kept hidden through
    /// beta 1: the file to write.
    #[arg(
        long = "out",
        short = 'o',
        value_name = "FILE",
        conflicts_with = "in_place",
        hide = true
    )]
    pub output: Option<PathBuf>,
    /// With the old --accept-all or --reject-all, kept hidden through
    /// beta 1: change the Word file itself.
    #[arg(long, hide = true)]
    pub in_place: bool,
    /// Use the settings under this folder instead of the usual place.
    #[arg(long, global = true, value_name = "DIR")]
    pub home: Option<PathBuf>,
    /// What to do instead of listing.
    #[command(subcommand)]
    pub command: Option<ChangesCommand>,
}

/// `tw changes` commands.
#[derive(clap::Subcommand, Debug)]
pub enum ChangesCommand {
    /// List the changes and comments (what `tw changes FILE` prints).
    List {
        /// The document whose changes and comments to list.
        file: PathBuf,
        /// Print JSON instead of one row per line.
        #[arg(long)]
        json: bool,
    },
    /// Write the document with every tracked change accepted; give -o FILE
    /// or --in-place.
    Accept(Decide),
    /// Write the document with every tracked change rejected; give -o FILE
    /// or --in-place.
    Reject(Decide),
}

/// Arguments for `tw changes accept` and `tw changes reject`.
#[derive(clap::Args, Debug)]
pub struct Decide {
    /// The document whose changes to decide.
    pub file: PathBuf,
    /// The file to write; its extension names the format (md, txt, html,
    /// docx, epub, pdf, brf).
    #[arg(
        long = "out",
        short = 'o',
        value_name = "FILE",
        conflicts_with = "in_place"
    )]
    pub output: Option<PathBuf>,
    /// Change the Word file (.docx) itself, after keeping a copy of the
    /// original beside it.
    #[arg(long)]
    pub in_place: bool,
}

impl Args {
    /// The subcommand folded into the flags it stands for, so one path
    /// runs both spellings.
    fn folded(mut self) -> Self {
        match self.command.take() {
            Some(ChangesCommand::List { file, json }) => {
                self.file = Some(file);
                self.json |= json;
            }
            Some(ChangesCommand::Accept(d)) => self.decide(d, true),
            Some(ChangesCommand::Reject(d)) => self.decide(d, false),
            None => {}
        }
        self
    }

    fn decide(&mut self, d: Decide, accept: bool) {
        self.file = Some(d.file);
        self.output = d.output;
        self.in_place = d.in_place;
        self.accept_all = accept;
        self.reject_all = !accept;
    }
}

/// Runs `tw changes`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let args = args.folded();
    let Some(file) = args.file.clone() else {
        anyhow::bail!("give the document: tw changes FILE");
    };
    let paths = match &args.home {
        Some(h) => Some(Paths::under(h)),
        None => Paths::platform().ok(),
    };
    let settings = paths
        .as_ref()
        .map(|p| SettingsStore::new(p.clone()).load().0)
        .unwrap_or_default();
    let (catalog, _) = Catalog::for_language(
        &settings.interface.language,
        paths.as_ref().map(Paths::locales_dir).as_deref(),
    );
    let mut doc = super::text::load_document(&file)?;
    if args.in_place && !(args.accept_all || args.reject_all) {
        anyhow::bail!("--in-place goes with tw changes accept or tw changes reject");
    }
    if args.in_place {
        return in_place(&catalog, &file, &doc, args.accept_all);
    }
    if args.accept_all || args.reject_all {
        let Some(out) = args.output.as_deref() else {
            anyhow::bail!(
                "nothing to write to: give -o FILE, or --in-place to change the Word file itself"
            );
        };
        let n = resolve_all(&mut doc, args.accept_all, None).len();
        write(&doc, out)?;
        let id = if args.accept_all {
            "changes-written-accepted"
        } else {
            "changes-written-rejected"
        };
        let path = out.display().to_string();
        return super::print_all(&format!(
            "{}\n",
            catalog.fmt(id, &args!["path" => path, "n" => n])
        ));
    }
    let out = if args.json {
        let v = json!({
            "changes": changes(&doc.meta),
            "comments": comments(&doc.meta),
        });
        format!("{}\n", serde_json::to_string_pretty(&v)?)
    } else {
        rows(&catalog, &doc)
    };
    super::print_all(&out)
}

/// The list's rows, one per line, in document order.
fn rows(c: &Catalog, doc: &Document) -> String {
    let mut rows: Vec<(usize, bool, String)> = changes(&doc.meta)
        .iter()
        .map(|x| (x.range.start.0, false, change_row(c, x)))
        .chain(
            comments(&doc.meta)
                .iter()
                .map(|x| (x.range.start.0, true, comment_row(c, x))),
        )
        .collect();
    rows.sort_by_key(|(at, comment, _)| (*at, *comment));
    let mut out = String::new();
    for (_, _, row) in rows {
        out.push_str(&row);
        out.push('\n');
    }
    if out.is_empty() {
        out = format!("{}\n", c.tr("changes-none"));
    }
    out
}

/// `--in-place`: every change in the Word file `file` accepted (`accept`)
/// or rejected, in the file itself, after a copy of the original is kept.
fn in_place(c: &Catalog, file: &Path, doc: &Document, accept: bool) -> anyhow::Result<()> {
    use textweaver_writers::docx_update::{DocxUpdate, backup, update_docx};
    let docx = file
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("docx"));
    if !docx {
        anyhow::bail!(
            "--in-place changes only Word files (.docx); use --out to write {} in another format",
            file.display()
        );
    }
    let n = changes(&doc.meta).len();
    let update = DocxUpdate {
        rest: Some(accept),
        ..DocxUpdate::default()
    };
    let bytes =
        std::fs::read(file).with_context(|| format!("could not read {}", file.display()))?;
    let (new, _) = update_docx(&bytes, &update)
        .with_context(|| format!("could not change {}", file.display()))?;
    let kept = backup(file).with_context(|| {
        format!(
            "could not keep a copy of {}; nothing was changed",
            file.display()
        )
    })?;
    textweaver_convert::write_atomic(file, &new)
        .with_context(|| format!("could not write {}", file.display()))?;
    let id = if accept {
        "changes-in-place-accepted"
    } else {
        "changes-in-place-rejected"
    };
    let backup_name = kept
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    super::print_all(&format!(
        "{}\n",
        c.fmt(
            id,
            &args!["path" => file.display().to_string(), "backup" => backup_name, "n" => n]
        )
    ))
}

/// Writes `doc` to `out` in the format its extension names.
fn write(doc: &Document, out: &Path) -> anyhow::Result<()> {
    let ext = out
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let bytes = match ext.as_str() {
        "md" | "markdown" => formats::to_markdown(doc).into_bytes(),
        "txt" | "text" => doc.text().to_string().into_bytes(),
        "html" | "htm" => formats::to_html(
            doc,
            &formats::HtmlOptions {
                standalone: true,
                ..formats::HtmlOptions::default()
            },
        )
        .into_bytes(),
        other => {
            let Some(format) = textweaver_writers::Format::from_name(other) else {
                anyhow::bail!(
                    "cannot write {}: the extension is not one textweaver writes. Use md, txt, html, docx, epub, pdf, or brf.",
                    out.display()
                );
            };
            textweaver_writers::write_to_vec(
                doc,
                format,
                &textweaver_writers::WriteOptions::default(),
            )
            .with_context(|| format!("could not write {}", out.display()))?
            .0
        }
    };
    std::fs::write(out, bytes).with_context(|| format!("could not write {}", out.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/t1/changes.docx")
    }

    #[test]
    fn lists_one_row_per_change_and_comment() {
        let doc = super::super::text::load_document(&fixture()).unwrap();
        let out = rows(&Catalog::english(), &doc);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 7, "{out}");
        assert_eq!(
            lines[0],
            "Inserted: 'renal', by Ada Example, Tuesday, March 3, 2026"
        );
        assert_eq!(lines[6], "Comment by Ada Example: Who repeats it?");
    }

    #[test]
    fn the_json_listing_round_trips() {
        let doc = super::super::text::load_document(&fixture()).unwrap();
        let v = json!({ "changes": changes(&doc.meta), "comments": comments(&doc.meta) });
        let text = serde_json::to_string(&v).unwrap();
        let back: serde_json::Value = serde_json::from_str(&text).unwrap();
        let listed: Vec<formats::DocumentChange> =
            serde_json::from_value(back["changes"].clone()).unwrap();
        assert_eq!(listed, changes(&doc.meta));
        assert_eq!(listed.len(), 5);
    }

    #[test]
    fn accept_all_writes_markdown_and_leaves_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("final.md");
        let before = std::fs::read(fixture()).unwrap();
        run(Args {
            home: Some(dir.path().to_owned()),
            command: Some(ChangesCommand::Accept(Decide {
                file: fixture(),
                output: Some(out.clone()),
                in_place: false,
            })),
            ..Args::default()
        })
        .unwrap();
        let md = std::fs::read_to_string(&out).unwrap();
        assert!(md.contains("acute renal failure"), "{md}");
        assert!(!md.contains("rarely"), "{md}");
        assert_eq!(std::fs::read(fixture()).unwrap(), before);
        let reloaded = formats::load_path(&out).unwrap();
        assert!(changes(&reloaded.meta).is_empty());
    }

    #[test]
    fn in_place_changes_the_word_file_after_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("draft.docx");
        std::fs::copy(fixture(), &file).unwrap();
        let before = std::fs::read(&file).unwrap();
        run(Args {
            home: Some(dir.path().to_owned()),
            command: Some(ChangesCommand::Reject(Decide {
                file: file.clone(),
                output: None,
                in_place: true,
            })),
            ..Args::default()
        })
        .unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("draft-original.docx")).unwrap(),
            before
        );
        let doc = formats::load_path(&file).unwrap();
        assert!(changes(&doc.meta).is_empty());
        assert!(doc.text().to_string().contains("Fluids were rarely given"));
        assert_eq!(comments(&doc.meta).len(), 2, "the comments stay");
    }

    #[test]
    fn in_place_refuses_a_file_that_is_not_word() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "Plain notes.").unwrap();
        // The old spelling, --accept-all --in-place, takes the same path.
        let err = run(Args {
            file: Some(file),
            accept_all: true,
            in_place: true,
            home: Some(dir.path().to_owned()),
            ..Args::default()
        })
        .unwrap_err();
        assert!(err.to_string().contains("only Word files"), "{err}");
    }

    /// `tw changes FILE`, `tw changes list FILE`, and the subcommands
    /// parse; the old flags still do, hidden.
    #[test]
    fn verbs_are_subcommands_and_the_old_flags_still_parse() {
        use clap::Parser as _;
        #[derive(clap::Parser)]
        struct Tw {
            #[command(flatten)]
            args: Args,
        }
        let parse = |argv: &[&str]| Tw::try_parse_from(argv).map(|t| t.args.folded());
        let a = parse(&["tw", "a.docx", "--json"]).unwrap();
        assert!(a.json && a.file.is_some() && !a.accept_all);
        let a = parse(&["tw", "list", "a.docx", "--json"]).unwrap();
        assert!(a.json && a.file.is_some());
        let a = parse(&["tw", "accept", "a.docx", "-o", "b.md"]).unwrap();
        assert!(a.accept_all && !a.reject_all);
        assert_eq!(a.output.as_deref(), Some(Path::new("b.md")));
        let a = parse(&["tw", "reject", "a.docx", "--in-place", "--home", "h"]).unwrap();
        assert!(a.reject_all && a.in_place && a.home.is_some());
        let a = parse(&["tw", "a.docx", "--accept-all", "-o", "b.md"]).unwrap();
        assert!(a.accept_all && a.output.is_some());
        assert!(parse(&["tw"]).is_err(), "the document is required");
    }
}
