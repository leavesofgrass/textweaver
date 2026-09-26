//! `cargo xtask listen`: sample WAV files from every real speech engine
//! on this machine, for the listening checklist in `docs/releasing.md`.
//!
//! For each engine (Eloquence, SAPI 5, DECtalk, Piper, eSpeak NG) it runs
//! `tw export-audio` on a short sample (a sentence, a heading, numbers,
//! an abbreviation, and a question) and writes
//! `target/listen/<engine>.wav` (and `<engine>-fast.wav` at 400 words per
//! minute, and `<engine>-low.wav` 4 semitones down), with word-level
//! subtitles beside each, so the highlight timing can be checked by eye.
//! An engine that is not available is reported and skipped; nothing falls
//! back to another engine under its name.
//!
//! It never plays anything: listening is for a person, afterwards.
//!
//! Options: `--engine ID` (repeatable) for only those engines; `--text
//! FILE` for your own sample; `--out DIR` instead of `target/listen`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

/// The engines tried, in order.
pub(crate) const ENGINES: [&str; 5] = ["eci", "sapi", "dectalk", "piper", "espeak"];

/// The sample read by every engine.
pub(crate) const SAMPLE: &str = "# Chapter 3: The Library\n\n\
The library opens at 9:00 a.m. on weekdays, and it closes at 6.\n\n\
Dr. Okafor said the fine is $1.50 per day; is that right? \
It adds up quickly: 12 books, 3 days late.\n";

/// Parsed options.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Args {
    pub engines: Vec<String>,
    pub text: Option<PathBuf>,
    pub out: Option<PathBuf>,
}

pub(crate) fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut a = Args::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .with_context(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--engine" => a.engines.push(value()?),
            "--text" => a.text = Some(PathBuf::from(value()?)),
            "--out" => a.out = Some(PathBuf::from(value()?)),
            other => bail!("unknown option {other}"),
        }
    }
    Ok(a)
}

/// The variants written per engine: file suffix and extra arguments.
pub(crate) fn variants() -> [(&'static str, &'static [&'static str]); 3] {
    [
        ("", &[]),
        ("-fast", &["--rate", "400"]),
        ("-low", &["--pitch", "-4"]),
    ]
}

/// Runs `cargo xtask listen`.
pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let args = parse(&args)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("workspace root")?
        .to_owned();
    let out = args
        .out
        .unwrap_or_else(|| root.join("target").join("listen"));
    std::fs::create_dir_all(&out)?;
    let sample = match args.text {
        Some(t) => t,
        None => {
            let p = out.join("sample.md");
            std::fs::write(&p, SAMPLE)?;
            p
        }
    };
    let engines: Vec<String> = if args.engines.is_empty() {
        ENGINES.iter().map(|s| (*s).to_owned()).collect()
    } else {
        args.engines
    };
    println!("Building tw.");
    let status = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args(["build", "-p", "textweaver-cli", "--features", "espeak"])
        .status()?;
    if !status.success() {
        bail!("building tw failed");
    }
    let tw = root
        .join("target")
        .join("debug")
        .join(format!("tw{}", std::env::consts::EXE_SUFFIX));
    let mut written = 0;
    for engine in &engines {
        for (suffix, extra) in variants() {
            let wav = out.join(format!("{engine}{suffix}.wav"));
            let srt = out.join(format!("{engine}{suffix}.srt"));
            let output = Command::new(&tw)
                .arg("export-audio")
                .arg(&sample)
                .arg("--out")
                .arg(&wav)
                .arg("--subtitles")
                .arg(&srt)
                .args(["--word-level", "--quiet", "--json", "--backend", engine])
                .args(extra)
                .output()?;
            if !output.status.success() {
                println!(
                    "{engine}{suffix}: failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
                break;
            }
            let report: serde_json::Value =
                serde_json::from_slice(&output.stdout).unwrap_or_default();
            if report["backend"]["fell_back"].as_bool() == Some(true) {
                let _ = std::fs::remove_file(&wav);
                let _ = std::fs::remove_file(&srt);
                println!("{engine}: not available on this machine; skipped.");
                break;
            }
            println!("{engine}{suffix}: wrote {}", wav.display());
            written += 1;
        }
    }
    if written == 0 {
        bail!("no engine wrote a sample");
    }
    println!(
        "Wrote {written} files. Listen to them, then follow the checklist in docs/releasing.md."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse() {
        let a = parse(&[
            "--engine".into(),
            "piper".into(),
            "--engine".into(),
            "eci".into(),
            "--out".into(),
            "x".into(),
        ])
        .unwrap();
        assert_eq!(a.engines, vec!["piper", "eci"]);
        assert_eq!(a.out, Some(PathBuf::from("x")));
        assert!(parse(&["--bogus".into()]).is_err());
        assert!(parse(&["--text".into()]).is_err());
    }

    #[test]
    fn the_sample_exercises_normalization() {
        for needle in ["9:00", "$1.50", "Dr.", "?", "# Chapter"] {
            assert!(SAMPLE.contains(needle), "{needle}");
        }
        assert_eq!(variants().len(), 3);
        assert!(ENGINES.contains(&"piper"));
    }
}
