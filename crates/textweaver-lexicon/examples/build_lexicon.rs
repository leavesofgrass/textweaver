//! Builds the define-word data file from Open English WordNet (WNDB) and
//! CMUdict, then times lookups in it. `tools/build_lexicon.py` downloads
//! and checks the sources and runs this; see `third_party/lexicon/README.md`.
//!
//! ```text
//! cargo run --release -p textweaver-lexicon --example build_lexicon -- \
//!     --wndb DIR --cmudict FILE --out FILE \
//!     --source "name|version|licence|url|sha256" ...
//! ```

use std::path::PathBuf;
use std::time::Instant;

use textweaver_lexicon::sources::{WordNet, read_cmudict};
use textweaver_lexicon::{Lexicon, SourceInfo, build};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut wndb = None;
    let mut cmudict = None;
    let mut out = None;
    let mut sources = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--wndb" => wndb = Some(PathBuf::from(value()?)),
            "--cmudict" => cmudict = Some(PathBuf::from(value()?)),
            "--out" => out = Some(PathBuf::from(value()?)),
            "--source" => {
                let v = value()?;
                let f: Vec<&str> = v.split('|').collect();
                let [name, version, licence, url, sha256] = f.as_slice() else {
                    return Err(format!("--source needs five fields: {v}").into());
                };
                sources.push(SourceInfo {
                    name: (*name).into(),
                    version: (*version).into(),
                    licence: (*licence).into(),
                    url: (*url).into(),
                    sha256: (*sha256).into(),
                });
            }
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let (Some(wndb), Some(cmudict), Some(out)) = (wndb, cmudict, out) else {
        return Err(
            "usage: build_lexicon --wndb DIR --cmudict FILE --out FILE [--source ...]".into(),
        );
    };
    let t = Instant::now();
    let wn = WordNet::read_dir(&wndb)?;
    let cmu = read_cmudict(&String::from_utf8_lossy(&std::fs::read(&cmudict)?));
    println!(
        "read {} synsets, {} lemmas, {} exceptions, {} CMUdict words in {:.2?}",
        wn.synsets.len(),
        wn.index.len(),
        wn.exceptions.len(),
        cmu.len(),
        t.elapsed()
    );
    let t = Instant::now();
    let bytes = build(&wn, &cmu, sources)?;
    println!("built {} bytes in {:.2?}", bytes.len(), t.elapsed());
    std::fs::write(&out, &bytes)?;

    // Timings, on the file as written.
    let t = Instant::now();
    let lex = Lexicon::open(&out)?;
    println!(
        "opened in {:.2?}: {} headwords, {} synsets, {} pronunciations",
        t.elapsed(),
        lex.info().headwords,
        lex.info().synsets,
        lex.info().pronunciations
    );
    for word in ["run", "running", "geese", "dog", "photosynthesis", "the"] {
        let t = Instant::now();
        let d = lex.define(word)?;
        let first = t.elapsed();
        let t = Instant::now();
        let _ = lex.define(word)?;
        println!(
            "define {word}: {} senses, first {first:.2?}, again {:.2?}",
            d.map_or(0, |d| d.sense_count()),
            t.elapsed()
        );
    }
    let words: Vec<String> = wn.index.keys().step_by(97).cloned().collect();
    let t = Instant::now();
    let mut found = 0;
    for w in &words {
        if lex.define(w)?.is_some() {
            found += 1;
        }
    }
    let each = t.elapsed() / words.len().max(1) as u32;
    println!(
        "{} lookups spread over the alphabet: {found} found, {each:.2?} each on average",
        words.len()
    );
    Ok(())
}
