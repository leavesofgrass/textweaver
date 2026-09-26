//! A fake Whisper program for tests. With `-f` it behaves like
//! whisper.cpp's `whisper-cli`; otherwise like the openai-whisper and
//! whisper-ctranslate2 command line.
//!
//! The "audio" file is a script: each line is a segment's text, two
//! seconds each, except directives:
//! - `!fail MESSAGE`: print MESSAGE to stderr and exit with status 3;
//! - `!sleep MS`: pause before the next segment;
//! - `!stderr MESSAGE`: print MESSAGE to stderr;
//! - `!nojson`: print segments but write no JSON transcript.
//!
//! A real WAV file (starting with `RIFF`) transcribes as
//! `Captured N samples at R hertz.`
//!
//! When `TW_FAKE_WHISPER_ARGS` names a file, the arguments are written to
//! it, one per line.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn value_after(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// The sample count and rate of a WAV file.
fn wav_info(bytes: &[u8]) -> Option<(usize, u32)> {
    let rate = u32::from_le_bytes(bytes.get(24..28)?.try_into().ok()?);
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().ok()?) as usize;
        if id == b"data" {
            return Some((len / 2, rate));
        }
        i += 8 + len + (len % 2);
    }
    None
}

fn clock(ms: u64, cpp: bool) -> String {
    let (h, m, s, milli) = (
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000,
    );
    if cpp {
        format!("{h:02}:{m:02}:{s:02}.{milli:03}")
    } else {
        format!("{m:02}:{s:02}.{milli:03}")
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Ok(record) = std::env::var("TW_FAKE_WHISPER_ARGS") {
        let _ = std::fs::write(record, args.join("\n"));
    }
    let cpp = args.iter().any(|a| a == "-f");
    let input: Option<PathBuf> = if cpp {
        value_after(&args, "-f").map(PathBuf::from)
    } else {
        // The first argument that is neither a flag nor a flag's value.
        let mut it = args.iter();
        let mut found = None;
        while let Some(a) = it.next() {
            if a.starts_with("--") {
                it.next();
            } else {
                found = Some(PathBuf::from(a));
                break;
            }
        }
        found
    };
    let Some(input) = input else {
        eprintln!("error: no input file");
        std::process::exit(2);
    };
    if cpp {
        let model = value_after(&args, "-m").unwrap_or_default();
        if !Path::new(&model).is_file() {
            eprintln!("error: failed to open model '{model}'");
            std::process::exit(1);
        }
    }
    let bytes = match std::fs::read(&input) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", input.display());
            std::process::exit(2);
        }
    };
    let mut segments: Vec<(u64, u64, String)> = Vec::new();
    let mut write_json = true;
    let mut t = 0u64;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut emit = |text: String, t: &mut u64, segments: &mut Vec<(u64, u64, String)>| {
        let (start, end) = (*t, *t + 2000);
        *t = end;
        let _ = writeln!(
            out,
            "[{} --> {}]  {text}",
            clock(start, cpp),
            clock(end, cpp)
        );
        let _ = out.flush();
        segments.push((start, end, text));
    };
    if bytes.starts_with(b"RIFF") {
        let (n, rate) = wav_info(&bytes).unwrap_or((0, 0));
        emit(
            format!("Captured {n} samples at {rate} hertz."),
            &mut t,
            &mut segments,
        );
    } else {
        for line in String::from_utf8_lossy(&bytes).lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(msg) = line.strip_prefix("!fail") {
                eprintln!("whisper: loading model");
                eprintln!("{}", msg.trim());
                std::process::exit(3);
            } else if let Some(ms) = line.strip_prefix("!sleep") {
                std::thread::sleep(Duration::from_millis(ms.trim().parse().unwrap_or(0)));
            } else if let Some(msg) = line.strip_prefix("!stderr") {
                eprintln!("{}", msg.trim());
            } else if line == "!nojson" {
                write_json = false;
            } else {
                emit(line.to_owned(), &mut t, &mut segments);
            }
        }
    }
    if !write_json {
        return;
    }
    let (json_path, body) = if cpp {
        let base = value_after(&args, "-of").unwrap_or_else(|| "out".into());
        let items: Vec<String> = segments
            .iter()
            .map(|(s, e, text)| {
                format!(
                    "{{\"timestamps\":{{\"from\":\"{}\",\"to\":\"{}\"}},\"offsets\":{{\"from\":{s},\"to\":{e}}},\"text\":\" {}\"}}",
                    clock(*s, true),
                    clock(*e, true),
                    json_escape(text)
                )
            })
            .collect();
        (
            PathBuf::from(format!("{base}.json")),
            format!(
                "{{\"systeminfo\":\"fake\",\"transcription\":[{}]}}",
                items.join(",")
            ),
        )
    } else {
        let dir = value_after(&args, "--output_dir").unwrap_or_else(|| ".".into());
        let stem = input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let items: Vec<String> = segments
            .iter()
            .map(|(s, e, text)| {
                format!(
                    "{{\"start\":{},\"end\":{},\"text\":\" {}\"}}",
                    *s as f64 / 1000.0,
                    *e as f64 / 1000.0,
                    json_escape(text)
                )
            })
            .collect();
        let all: Vec<&str> = segments.iter().map(|(_, _, t)| t.as_str()).collect();
        (
            Path::new(&dir).join(format!("{stem}.json")),
            format!(
                "{{\"text\":\" {}\",\"segments\":[{}],\"language\":\"en\"}}",
                json_escape(&all.join(" ")),
                items.join(",")
            ),
        )
    };
    if let Err(e) = std::fs::write(&json_path, body) {
        eprintln!("error: cannot write {}: {e}", json_path.display());
        std::process::exit(4);
    }
}
