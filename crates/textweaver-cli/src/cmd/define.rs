//! `tw define`: define words offline, as the reader's define word does: the
//! glossary first, then Open English WordNet, with CMUdict pronunciations
//! (`textweaver-lexicon`). Owner: Agent W3e.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::lexicon::{self, Dictionary, Glossary, args};
use textweaver_app::store::{Paths, SettingsStore};

/// Arguments for `tw define`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The words to define, each on its own (quote a phrase: "ice cream").
    #[arg(required = true)]
    pub words: Vec<String>,
    /// Print JSON instead of Markdown.
    #[arg(long)]
    pub json: bool,
    /// List headwords that start with each word instead (at most 20).
    #[arg(long)]
    pub complete: bool,
    /// A glossary to look in first, instead of `[lexicon] glossary`.
    #[arg(long, value_name = "FILE")]
    pub glossary: Option<PathBuf>,
    /// The dictionary data file, instead of `[lexicon] data_file` and the
    /// usual places.
    #[arg(long, value_name = "FILE")]
    pub data: Option<PathBuf>,
    /// Use the settings under this folder instead of the usual place.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Runs `tw define`. Fails when no word was found.
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
    let data_file = args.data.clone().or(settings.lexicon.data_file.clone());
    let found = lexicon::find_lexicon(
        data_file.as_deref(),
        paths.as_ref().map(|p| p.data_dir.as_path()),
    )
    .context("opening the dictionary")?;
    let glossary_path = args
        .glossary
        .clone()
        .or(settings.lexicon.glossary.clone())
        .or_else(|| paths.as_ref().and_then(Paths::default_glossary));
    let glossary = match &glossary_path {
        Some(p) => Some(Arc::new(Glossary::load(p).context("reading the glossary")?)),
        None => None,
    };
    let lexicon = found.map(|(_, l)| Arc::new(l));
    if lexicon.is_none() {
        eprintln!("{}", catalog.tr("define-no-dictionary"));
    }
    let dict = Dictionary { glossary, lexicon };
    let mut out = std::io::stdout().lock();
    if args.complete {
        let Some(l) = &dict.lexicon else {
            anyhow::bail!("no dictionary file");
        };
        for w in &args.words {
            for c in l.complete(w, 20) {
                writeln!(out, "{c}")?;
            }
        }
        return Ok(());
    }
    let mut results = Vec::new();
    let mut missing = 0;
    for w in &args.words {
        match dict.define(w)? {
            Some(d) => results.push(d),
            None => {
                missing += 1;
                eprintln!(
                    "{}",
                    catalog.fmt("define-not-found", &args!["word" => lexicon::normalize(w)])
                );
            }
        }
    }
    if args.json {
        serde_json::to_writer_pretty(&mut out, &results)?;
        writeln!(out)?;
    } else {
        for (i, d) in results.iter().enumerate() {
            if i > 0 {
                writeln!(out)?;
            }
            write!(out, "{}", lexicon::to_markdown(&catalog, d))?;
        }
    }
    if results.is_empty() && missing > 0 {
        // "No definition found for WORD." was printed above; exit 1 without
        // saying it a second time as an error.
        out.flush()?;
        return Err(super::NothingFound.into());
    }
    Ok(())
}
