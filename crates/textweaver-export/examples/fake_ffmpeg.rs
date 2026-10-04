//! A stand-in for ffmpeg in the video tests (never a real encoder).
//!
//! The tests copy this program into a temporary folder and pass it as the
//! ffmpeg to use. It writes what it was asked into `record.txt` beside
//! itself: every argument, the contents of the caption and metadata files
//! it was given, and how many 1280 by 720 RGBA frames arrived on standard
//! input. It creates the output file first, as ffmpeg does, so a test can
//! check that a stopped export removes it. With `fail` beside it, it fails
//! the way ffmpeg does, with a message on standard error.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::io::{Read, Write};
use std::path::PathBuf;

const FRAME: u64 = 1280 * 720 * 4;

fn main() {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
    else {
        return;
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut record = String::new();
    if args.iter().any(|a| a == "-encoders") {
        println!(" V....D libx264              fake H.264");
        println!(" A....D aac                  fake AAC");
        append(&dir, "encoders asked\n");
        return;
    }
    // Run directly (as `cargo test` may), it does nothing.
    let Some(out) = args.last().filter(|_| args.len() > 1) else {
        return;
    };
    let _ = std::fs::write(out, b"partial");
    for a in &args {
        record.push_str("arg: ");
        record.push_str(a);
        record.push('\n');
    }
    for pair in args.windows(2) {
        if pair[0] == "-i" && (pair[1].ends_with(".vtt") || pair[1].ends_with(".txt")) {
            let text = std::fs::read_to_string(&pair[1]).unwrap_or_default();
            record.push_str(&format!("file {}:\n{text}\nend of file\n", pair[1]));
        }
    }
    let mut bytes = 0u64;
    let mut buf = vec![0u8; 1 << 20];
    let mut stdin = std::io::stdin().lock();
    while let Ok(n) = stdin.read(&mut buf) {
        if n == 0 {
            break;
        }
        bytes += n as u64;
    }
    record.push_str(&format!(
        "frames: {}\nleft over: {}\n",
        bytes / FRAME,
        bytes % FRAME
    ));
    append(&dir, &record);
    if dir.join("fail").exists() {
        eprintln!("fake failure: no such encoder");
        std::process::exit(1);
    }
    let _ = std::fs::write(out, b"fake mp4");
}

fn append(dir: &std::path::Path, text: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("record.txt"))
    {
        let _ = f.write_all(text.as_bytes());
    }
}
