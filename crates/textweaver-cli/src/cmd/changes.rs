//! `tw changes`: a document's tracked changes and comment threads, one per
//! line, meaning first ("Inserted: 'renal', by Ada Example, Tuesday, March
//! 3, 2026"), from the same list the reader shows (`textweaver_app::changes`).
//! `--json` prints the changes and comments as the loaders recorded them.
//! `--accept-all` or `--reject-all` with `--out FILE` writes the document
//! with every change decided: Markdown (`.md`), plain text (`.txt`), HTML,
//! or a writer's format (`.docx`, `.epub`, `.pdf`, `.brf`). The original
//! file is never changed. Owner: task B1-t1.

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
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The document whose changes and comments to list.
    pub file: PathBuf,
    /// Print JSON instead of one row per line.
    #[arg(long)]
    pub json: bool,
    /// Accept every tracked change; needs --out.
    #[arg(long, requires = "output", conflicts_with_all = ["reject_all", "json"])]
    pub accept_all: bool,
    /// Reject every tracked change; needs --out.
    #[arg(long, requires = "output", conflicts_with = "json")]
    pub reject_all: bool,
    /// With --accept-all or --reject-all: the file to write; its extension
    /// names the format (md, txt, html, docx, epub, pdf, brf).
    #[arg(long = "out", short = 'o', value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Use the settings under this folder instead of the usual place.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Runs `tw changes`.
pub fn run(args: Args) -> anyhow::Result<()> {
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
    let mut doc = super::text::load_document(&args.file)?;
    if args.accept_all || args.reject_all {
        let Some(out) = args.output.as_deref() else {
            anyhow::bail!("--out is needed with --accept-all or --reject-all");
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
            file: fixture(),
            json: false,
            accept_all: true,
            reject_all: false,
            output: Some(out.clone()),
            home: Some(dir.path().to_owned()),
        })
        .unwrap();
        let md = std::fs::read_to_string(&out).unwrap();
        assert!(md.contains("acute renal failure"), "{md}");
        assert!(!md.contains("rarely"), "{md}");
        assert_eq!(std::fs::read(fixture()).unwrap(), before);
        let reloaded = formats::load_path(&out).unwrap();
        assert!(changes(&reloaded.meta).is_empty());
    }
}
