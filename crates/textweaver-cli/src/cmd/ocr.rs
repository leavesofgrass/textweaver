//! `tw ocr`: text recognition for scanned pages (ADR-0026). Owner: Agent
//! W3d.
//!
//! - `tw ocr status` says which engines can run: the ocrs models (and the
//!   experimental PaddleOCR Latin model), and Tesseract with its languages.
//! - `tw ocr download [ocrs|paddle-latin]` downloads a model set after
//!   saying what it is (size, licence, source) and asking; `--yes` answers
//!   for you. It goes through the components path (W9a-c), as `tw
//!   components download ocr-ocrs` does: the mirror is tried first, and
//!   each file is checked by SHA-256 before it is kept.
//! - `--home DIR` on `status` and `download` uses the models under that
//!   data folder.
//! - `tw ocr read FILE` recognizes a scanned PDF or a picture and prints
//!   the text, with progress on standard error.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use textweaver_app::components::{Status, component_dir};
use textweaver_app::formats::{
    LoadOptions, OcrEngineChoice, OcrOptions, Progress, Registry, Source, warnings,
};
use textweaver_app::store::SettingsStore;
use textweaver_ocr::models;

/// Arguments for `tw ocr`.
#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: Sub,
}

#[derive(clap::Subcommand, Debug)]
enum Sub {
    /// Say which OCR engines can run.
    Status {
        /// Print the status as JSON.
        #[arg(long)]
        json: bool,
        /// Use the models under this data folder instead of the usual place.
        #[arg(long, value_name = "DIR")]
        home: Option<PathBuf>,
    },
    /// Download a model set: ocrs (English, the default) or paddle-latin.
    Download {
        /// The set: ocrs or paddle-latin.
        #[arg(default_value = "ocrs")]
        set: String,
        /// Download without asking.
        #[arg(long, short)]
        yes: bool,
        /// Use the models under this data folder instead of the usual place.
        #[arg(long, value_name = "DIR")]
        home: Option<PathBuf>,
    },
    /// Recognize a scanned PDF or a picture and print its text.
    Read {
        /// The file.
        file: PathBuf,
        /// The text's language: Tesseract codes (eng, fra+eng) or tags (fr).
        #[arg(long)]
        lang: Option<String>,
        /// The engine: auto, ocrs, tesseract, or paddle.
        #[arg(long, default_value = "auto")]
        engine: String,
    },
}

/// Runs `tw ocr`.
pub fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        Sub::Status { json, home } => {
            let data = super::paths(home.as_deref())?.data_dir;
            if json {
                super::print_all(&format!("{:#}\n", status_json(&data)))
            } else {
                super::print_all(&status(&data))
            }
        }
        Sub::Download { set, yes, home } => download(&set, yes, home.as_deref()),
        Sub::Read { file, lang, engine } => read(&file, lang, &engine),
    }
}

fn set_named(name: &str) -> anyhow::Result<models::ModelSet> {
    models::ALL
        .into_iter()
        .find(|s| s.id.eq_ignore_ascii_case(name.trim()))
        .with_context(|| format!("{name} is not a model set; the sets are ocrs and paddle-latin"))
}

/// A model set's state in the data folder `data`, through the components
/// path, as `tw components list` sees it.
fn set_status(set: &models::ModelSet, data: &Path) -> Status {
    let c = set.component();
    c.status_in(&component_dir(&c, data))
}

/// The folder every model set lives under, for the data folder `data`.
fn models_folder(data: &Path) -> PathBuf {
    models::flat_dir().unwrap_or_else(|| data.join("models").join("ocr"))
}

/// What `tw ocr status` prints, for the data folder `data`.
fn status(data: &Path) -> String {
    let mut out = String::new();
    for set in models::ALL {
        let state = match set_status(&set, data) {
            Status::Installed => "downloaded".to_owned(),
            Status::NotInstalled | Status::Partial(_) => format!(
                "not downloaded ({}, {}); run tw ocr download {}",
                set.size_text(),
                set.licence,
                set.id
            ),
            Status::Damaged(f) => {
                format!("damaged ({f}); run tw ocr download {} again", set.id)
            }
        };
        out.push_str(&format!("{}: {state}.\n", capital(set.title)));
    }
    match textweaver_ocr::tesseract::find() {
        Some(exe) => {
            let langs = textweaver_ocr::tesseract::languages(&exe);
            out.push_str(&format!(
                "Tesseract: installed at {}, with {} languages.\n",
                exe.display(),
                langs.len()
            ));
        }
        None => out.push_str(
            "Tesseract: not installed. It reads languages other than English; install it with its language data.\n",
        ),
    }
    out.push_str(&format!(
        "Models folder: {}\n",
        models_folder(data).display()
    ));
    out
}

/// `tw ocr status --json`: the model sets and Tesseract. Keys are English
/// and never translated.
fn status_json(data: &Path) -> serde_json::Value {
    let sets: Vec<serde_json::Value> = models::ALL
        .into_iter()
        .map(|set| {
            let (status, detail) = match set_status(&set, data) {
                Status::Installed => ("downloaded", None),
                Status::NotInstalled | Status::Partial(_) => ("not-downloaded", None),
                Status::Damaged(f) => ("damaged", Some(f)),
            };
            serde_json::json!({
                "id": set.id,
                "title": set.title,
                "status": status,
                "damaged": detail,
                "size": set.size_text(),
                "license": set.licence,
            })
        })
        .collect();
    let tesseract = textweaver_ocr::tesseract::find().map(|exe| {
        let languages = textweaver_ocr::tesseract::languages(&exe);
        serde_json::json!({ "path": exe, "languages": languages })
    });
    serde_json::json!({
        "models": sets,
        "tesseract": tesseract,
        "models_folder": models_folder(data),
    })
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// `tw ocr download SET`: the set's component, downloaded through the
/// components path (the mirror first, every file checked), as `tw
/// components download ocr-SET` does.
fn download(name: &str, yes: bool, home: Option<&Path>) -> anyhow::Result<()> {
    let set = set_named(name)?;
    let paths = super::paths(home)?;
    let settings = SettingsStore::new(paths.clone()).load().0;
    let c = set.component();
    let dir = component_dir(&c, &paths.data_dir);
    super::components::download(&c, &dir, &settings, yes)?;
    if c.status_in(&dir) == Status::Installed {
        eprintln!("Scanned pages can now be read.");
    }
    Ok(())
}

fn read(file: &Path, lang: Option<String>, engine: &str) -> anyhow::Result<()> {
    let Some(engine) = OcrEngineChoice::parse(engine) else {
        bail!("{engine} is not an engine; use auto, ocrs, tesseract, or paddle");
    };
    let options = LoadOptions {
        ocr: OcrOptions {
            enabled: true,
            lang: lang.unwrap_or_default(),
            engine,
        },
        progress: Progress::new(|p| eprintln!("{}", p.message)),
        ..LoadOptions::default()
    };
    let doc = Registry::with_builtins()
        .load(&Source::Path(file.to_owned()), &options)
        .with_context(|| format!("cannot open {}", file.display()))?;
    for w in warnings(&doc.meta) {
        eprintln!("{w}");
    }
    crate::cmd::outln!("{}", doc.text());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_are_named_and_status_reads_well() {
        assert_eq!(set_named("OCRS").unwrap().id, "ocrs");
        assert!(set_named("gpt").is_err());
        let dir = tempfile::tempdir().unwrap();
        let s = status(dir.path());
        assert!(
            s.starts_with("The ocrs text recognition models for English: "),
            "{s}"
        );
        assert!(s.contains("Tesseract: "));
    }
}
