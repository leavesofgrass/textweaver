//! Times the shared block tree ([`model::blocks`]) and each native writer
//! on large documents, and prints a fingerprint of the tree so two builds
//! can be checked to give the same output.
//!
//! ```text
//! cargo run --release -p textweaver-writers --example bench_blocks -- \
//!     FILE... [--runs 3] [--write epub,docx,brf,pdf]
//! ```
//!
//! Each file is loaded once with the built-in loaders. The block tree is
//! built `--runs` times and the fastest run is reported; each format in
//! `--write` is written once. The fingerprint is a hash of the tree's
//! debug form: it changes only when the tree does. The corpora of
//! `cargo xtask bench` (`target/bench-corpus`) are the usual input.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use textweaver_writers::{Format, WriteOptions, model, write_to_vec};

fn main() -> ExitCode {
    let mut files = Vec::new();
    let mut runs = 3usize;
    let mut formats: Vec<Format> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--runs" => match args.next().and_then(|v| v.parse().ok()) {
                Some(n) if n > 0 => runs = n,
                _ => {
                    eprintln!("--runs needs a number above zero");
                    return ExitCode::from(2);
                }
            },
            "--write" => {
                let Some(list) = args.next() else {
                    eprintln!("--write needs a list, such as epub,pdf");
                    return ExitCode::from(2);
                };
                for name in list.split(',') {
                    match Format::from_name(name.trim()) {
                        Some(f) => formats.push(f),
                        None => {
                            eprintln!("unknown format {name}");
                            return ExitCode::from(2);
                        }
                    }
                }
            }
            _ => files.push(a),
        }
    }
    if files.is_empty() {
        eprintln!("usage: bench_blocks FILE... [--runs N] [--write epub,docx,brf,pdf]");
        return ExitCode::from(2);
    }
    for file in &files {
        let started = Instant::now();
        let doc = match textweaver_formats::load_path(file) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{file}: {e}");
                return ExitCode::FAILURE;
            }
        };
        println!(
            "{file}: loaded in {} ms, {} chars, {} markers",
            started.elapsed().as_millis(),
            doc.len_chars(),
            doc.markers().len()
        );
        let mut best = Duration::MAX;
        let mut fingerprint = 0;
        let mut count = 0;
        for _ in 0..runs {
            let t = Instant::now();
            let tree = model::blocks(&doc);
            best = best.min(t.elapsed());
            let mut h = DefaultHasher::new();
            format!("{tree:?}").hash(&mut h);
            fingerprint = h.finish();
            count = tree.len();
        }
        println!(
            "  blocks: {} ms (fastest of {runs}), {count} top-level blocks, tree {fingerprint:016x}",
            best.as_millis()
        );
        for &format in &formats {
            let t = Instant::now();
            match write_to_vec(&doc, format, &WriteOptions::default()) {
                Ok((bytes, _)) => println!(
                    "  {}: {} ms, {} bytes",
                    format.extension(),
                    t.elapsed().as_millis(),
                    bytes.len()
                ),
                Err(e) => {
                    eprintln!("  {}: {e}", format.extension());
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    ExitCode::SUCCESS
}
