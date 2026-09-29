//! `tw summarize`: a document's most central sentences, without a model
//! (`textweaver-summary`, ADR-0037). Owner: Agent W5s.
//!
//! The sentences are printed one per line, in document order, with
//! nothing before them, so the meaning comes first on a braille line.
//! `--json` prints each sentence's text, range, line, and score, and how
//! the summary was made. How many sentences: `--sentences`, else
//! `[summary] sentences` (5 unless changed).

use std::path::PathBuf;

use serde_json::json;
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::{Paths, SettingsStore, SummarySettings};
use textweaver_app::summaries::{Options, summarize_with};

/// Arguments for `tw summarize`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The document to summarize.
    pub file: PathBuf,
    /// How many sentences, 1 to 50; the default is `[summary] sentences`.
    #[arg(long, short = 'n', value_name = "N",
          value_parser = clap::value_parser!(u16).range(1..=SummarySettings::MAX_SENTENCES as i64))]
    pub sentences: Option<u16>,
    /// Print JSON instead of one sentence per line.
    #[arg(long)]
    pub json: bool,
    /// Use the settings under this folder instead of the usual place.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Runs `tw summarize`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let paths = match &args.home {
        Some(h) => Some(Paths::under(h)),
        None => Paths::platform().ok(),
    };
    let settings = paths
        .as_ref()
        .map(|p| SettingsStore::new(p.clone()).load().0)
        .unwrap_or_default();
    let doc = super::text::load_document(&args.file)?;
    let options = Options {
        sentences: args
            .sentences
            .map_or(settings.summary.sentences, usize::from),
        ..Options::default()
    };
    let summary = summarize_with(&doc, &options);
    if summary.windowed {
        let (catalog, _) = Catalog::for_language(
            &settings.interface.language,
            paths.as_ref().map(Paths::locales_dir).as_deref(),
        );
        eprintln!(
            "{}",
            catalog.fmt(
                "summary-sampled-cli",
                &args!["read" => summary.chars_read, "total" => summary.chars],
            )
        );
    }
    let out = if args.json {
        let sentences: Vec<_> = summary
            .sentences
            .iter()
            .map(|s| {
                json!({
                    "text": s.text,
                    "start": s.range.start.0,
                    "end": s.range.end.0,
                    "line": doc.line_of(s.range.start) + 1,
                    "score": s.score,
                })
            })
            .collect();
        let v = json!({
            "sentences": sentences,
            "ranked": summary.ranked,
            "candidates": summary.candidates,
            "chars_read": summary.chars_read,
            "chars": summary.chars,
            "sampled": summary.sampled(),
        });
        format!("{}\n", serde_json::to_string_pretty(&v)?)
    } else {
        render_lines(&summary)
    };
    super::print_all(&out)
}

/// One sentence per line.
fn render_lines(summary: &textweaver_app::summaries::Summary) -> String {
    let mut s = String::new();
    for x in &summary.sentences {
        s.push_str(&x.text);
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name)
    }

    #[test]
    fn prints_the_central_sentences_one_per_line() {
        let doc = super::super::text::load_document(&fixture("s/chapters.md")).unwrap();
        let summary = summarize_with(
            &doc,
            &Options {
                sentences: 4,
                ..Options::default()
            },
        );
        let out = render_lines(&summary);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 4, "{out}");
        for l in &lines {
            assert!(
                l.chars().next().is_some_and(char::is_uppercase),
                "meaning first, no numbering: {l}"
            );
            assert!(!l.contains("fence") && !l.contains("train"), "{l}");
        }
    }

    #[test]
    fn the_sample_document_has_a_summary() {
        let doc = super::super::text::load_document(&fixture("sample.md")).unwrap();
        let summary = summarize_with(&doc, &Options::default());
        assert!(!summary.sentences.is_empty());
        for s in &summary.sentences {
            assert!(
                !s.text.starts_with("Sample Markdown"),
                "no headings: {}",
                s.text
            );
        }
    }
}
