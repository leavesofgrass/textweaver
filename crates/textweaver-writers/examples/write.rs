//! Converts a document to every native output format.
//!
//! ```text
//! cargo run -p textweaver-writers --example write -- INPUT OUT_DIR [epub|docx|brf|pdf ...]
//! ```
//!
//! Loads INPUT with the built-in loaders and writes `OUT_DIR/<stem>.<ext>`
//! for each format asked for (all four by default), printing each file's
//! size and any warnings.

use std::path::PathBuf;
use std::process::ExitCode;

use textweaver_writers::{Format, WriteOptions, write_to_vec};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(input), Some(out_dir)) = (args.first(), args.get(1)) else {
        eprintln!("usage: write INPUT OUT_DIR [epub|docx|brf|pdf ...]");
        return ExitCode::from(2);
    };
    let formats: Vec<Format> = if args.len() > 2 {
        args[2..]
            .iter()
            .filter_map(|a| Format::from_name(a))
            .collect()
    } else {
        Format::ALL.to_vec()
    };
    let doc = match textweaver_formats::load_path(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let stem = PathBuf::from(input).file_stem().map_or_else(
        || "document".to_owned(),
        |s| s.to_string_lossy().into_owned(),
    );
    let mut failed = false;
    for format in formats {
        let path = PathBuf::from(out_dir).join(format!("{stem}.{}", format.extension()));
        let started = std::time::Instant::now();
        match write_to_vec(&doc, format, &WriteOptions::default()) {
            Ok((bytes, report)) => {
                if let Err(e) = std::fs::write(&path, &bytes) {
                    eprintln!("{}: {e}", path.display());
                    failed = true;
                    continue;
                }
                println!(
                    "{}: {} bytes in {} ms",
                    path.display(),
                    bytes.len(),
                    started.elapsed().as_millis()
                );
                for w in report.warnings {
                    println!("  warning: {w}");
                }
            }
            Err(e) => {
                eprintln!("{}: {e}", format.spoken_name());
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
