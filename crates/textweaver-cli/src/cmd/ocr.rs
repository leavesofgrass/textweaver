//! `tw ocr`: text recognition for scanned pages (ADR-0026). Owner: Agent
//! W3d.
//!
//! - `tw ocr status` says which engines can run: the ocrs models (and the
//!   experimental PaddleOCR Latin model), and Tesseract with its languages.
//! - `tw ocr download [ocrs|paddle-latin]` downloads a model set after
//!   saying what it is (size, licence, source) and asking; `--yes` answers
//!   for you. Each file is checked by SHA-256 before it is kept.
//! - `tw ocr read FILE` recognizes a scanned PDF or a picture and prints
//!   the text, with progress on standard error.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use anyhow::{Context, bail};
use textweaver_app::formats::{
    LoadOptions, OcrEngineChoice, OcrOptions, Progress, Registry, Source, warnings,
};
use textweaver_ocr::{ModelStatus, models};

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
    },
    /// Download a model set: ocrs (English, the default) or paddle-latin.
    Download {
        /// The set: ocrs or paddle-latin.
        #[arg(default_value = "ocrs")]
        set: String,
        /// Download without asking.
        #[arg(long, short)]
        yes: bool,
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
        Sub::Status { json: false } => super::print_all(&status()),
        Sub::Status { json: true } => super::print_all(&format!("{:#}\n", status_json())),
        Sub::Download { set, yes } => download(&set, yes),
        Sub::Read { file, lang, engine } => read(&file, lang, &engine),
    }
}

fn set_named(name: &str) -> anyhow::Result<models::ModelSet> {
    models::ALL
        .into_iter()
        .find(|s| s.id.eq_ignore_ascii_case(name.trim()))
        .with_context(|| format!("{name} is not a model set; the sets are ocrs and paddle-latin"))
}

/// What `tw ocr status` prints.
fn status() -> String {
    let mut out = String::new();
    for set in models::ALL {
        let state = match set.status() {
            ModelStatus::Present => "downloaded".to_owned(),
            ModelStatus::Missing(_) => format!(
                "not downloaded ({}, {}); run tw ocr download {}",
                set.size_text(),
                set.licence,
                set.id
            ),
            ModelStatus::Damaged(f) => {
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
    if let Some(dir) = models::root_dir() {
        out.push_str(&format!("Models folder: {}\n", dir.display()));
    }
    out
}

/// `tw ocr status --json`: the model sets and Tesseract. Keys are English
/// and never translated.
fn status_json() -> serde_json::Value {
    let sets: Vec<serde_json::Value> = models::ALL
        .into_iter()
        .map(|set| {
            let (status, detail) = match set.status() {
                ModelStatus::Present => ("downloaded", None),
                ModelStatus::Missing(_) => ("not-downloaded", None),
                ModelStatus::Damaged(f) => ("damaged", Some(f.to_string())),
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
        "models_folder": models::root_dir(),
    })
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

fn download(name: &str, yes: bool) -> anyhow::Result<()> {
    let set = set_named(name)?;
    if set.status() == ModelStatus::Present {
        println!("{} are already downloaded.", capital(set.title));
        return Ok(());
    }
    let host = set
        .files
        .first()
        .and_then(|f| f.url.split('/').nth(2))
        .unwrap_or("the internet");
    eprintln!(
        "This downloads {} from {host}: {}, under the {} license. {}",
        set.title,
        set.size_text(),
        set.licence,
        set.credit
    );
    let mut input = std::io::stdin().lock();
    if !yes
        && !super::confirm(
            "Download them? y or n:",
            &mut input,
            super::stdin_is_terminal(),
            "--yes",
        )?
    {
        eprintln!("Nothing was downloaded.");
        return Ok(());
    }
    let cancel = AtomicBool::new(false);
    let mut last = u64::MAX;
    models::download(
        &set,
        &mut |p| {
            let percent = p.done * 100 / p.total.max(1);
            if percent / 10 != last / 10 {
                last = percent;
                eprintln!("{percent} percent");
            }
        },
        &cancel,
    )?;
    println!(
        "{} are downloaded and checked. Scanned pages can now be read.",
        capital(set.title)
    );
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
    println!("{}", doc.text());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_are_named_and_status_reads_well() {
        assert_eq!(set_named("OCRS").unwrap().id, "ocrs");
        assert!(set_named("gpt").is_err());
        let s = status();
        assert!(
            s.starts_with("The ocrs text recognition models for English: "),
            "{s}"
        );
        assert!(s.contains("Tesseract: "));
    }
}
