//! Tesseract, run as a separate program: the fallback for languages ocrs
//! cannot read.
//!
//! Tesseract is found through `TEXTWEAVER_TESSERACT` (a path to the
//! program), then on `PATH`, then where its installers put it (on Windows,
//! `Tesseract-OCR` under Program Files or the user's programs folder; on
//! macOS, Homebrew's folders). The page is written to a temporary PNG and
//! read back as TSV, which gives each word with its box. The program is
//! stopped when recognition is cancelled or takes longer than the timeout.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::image::GrayImage;
use crate::{OcrError, OcrLine, OcrPage, OcrWord, Rect};

/// How long one page may take before Tesseract is stopped.
pub const PAGE_TIMEOUT: Duration = Duration::from_secs(120);

/// The most TSV output read for one page (a page has a few thousand
/// words; this is far beyond that).
const MAX_OUTPUT: u64 = 32 * 1024 * 1024;

fn program_name() -> &'static str {
    if cfg!(windows) {
        "tesseract.exe"
    } else {
        "tesseract"
    }
}

/// Places Tesseract's installers use, besides `PATH`.
fn install_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
            if let Some(p) = std::env::var_os(var) {
                dirs.push(PathBuf::from(p).join("Tesseract-OCR"));
            }
        }
        if let Some(p) = std::env::var_os("LOCALAPPDATA") {
            let p = PathBuf::from(p);
            dirs.push(p.join("Programs").join("Tesseract-OCR"));
            dirs.push(p.join("Tesseract-OCR"));
        }
    } else {
        for d in [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/opt/local/bin",
        ] {
            dirs.push(PathBuf::from(d));
        }
    }
    dirs
}

/// Looks for the Tesseract program now (see the module docs).
pub fn find_uncached() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("TEXTWEAVER_TESSERACT").filter(|p| !p.is_empty()) {
        let p = PathBuf::from(p);
        return p.is_file().then_some(p);
    }
    if let Some(p) = textweaver_core::process::find_program("tesseract") {
        return Some(p);
    }
    let name = program_name();
    install_dirs()
        .into_iter()
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// The Tesseract program. Once found it is kept while it is there; when it
/// is not found, it is looked for again after a few seconds, so one
/// installed while textweaver runs is seen without a restart.
pub fn find() -> Option<PathBuf> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static FOUND: Mutex<Option<(Option<PathBuf>, Instant)>> = Mutex::new(None);
    const RETRY: Duration = Duration::from_secs(5);
    let mut cached = FOUND.lock().unwrap_or_else(|p| p.into_inner());
    match &*cached {
        Some((Some(p), _)) if p.is_file() => return Some(p.clone()),
        Some((None, at)) if at.elapsed() < RETRY => return None,
        _ => {}
    }
    let found = find_uncached();
    *cached = Some((found.clone(), Instant::now()));
    found
}

fn command(exe: &Path) -> Command {
    let mut c = Command::new(exe);
    c.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console window flashes up per page.
        c.creation_flags(0x0800_0000);
    }
    c
}

/// The languages Tesseract has data for (`eng`, `fra`, ...).
pub fn languages(exe: &Path) -> Vec<String> {
    let Ok(out) = command(exe)
        .arg("--list-langs")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    else {
        return Vec::new();
    };
    // The list goes to stdout in Tesseract 4 and 5 (stderr in some builds).
    let text = [out.stdout, out.stderr]
        .iter()
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains(' ') && !l.ends_with(':'))
        .map(str::to_owned)
        .collect()
}

/// Recognizes `image` with Tesseract in the given languages (Tesseract
/// codes joined by `+`).
pub fn recognize(
    exe: &Path,
    image: &GrayImage,
    langs: &str,
    cancel: &AtomicBool,
) -> Result<OcrPage, OcrError> {
    let png = image.to_png()?;
    let dir = std::env::temp_dir();
    let input = dir.join(format!(
        "textweaver-ocr-{}-{}.png",
        std::process::id(),
        unique()
    ));
    std::fs::write(&input, &png).map_err(|e| OcrError::Io(input.clone(), e))?;
    let result = run(exe, &input, langs, cancel);
    let _ = std::fs::remove_file(&input);
    let tsv = result?;
    Ok(parse_tsv(&tsv, image.width, image.height))
}

fn unique() -> u64 {
    use std::sync::atomic::AtomicU64;
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

fn run(exe: &Path, input: &Path, langs: &str, cancel: &AtomicBool) -> Result<String, OcrError> {
    let mut child = command(exe)
        .arg(input)
        .arg("stdout")
        .arg("-l")
        .arg(langs)
        .arg("--psm")
        .arg("3")
        .arg("tsv")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| OcrError::Tesseract(format!("cannot start Tesseract: {e}")))?;
    // Read both pipes on threads, so a full pipe never stalls the program.
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(o) = stdout.as_mut() {
            let _ = o.take(MAX_OUTPUT).read_to_end(&mut s);
        }
        s
    });
    let err = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(e) = stderr.as_mut() {
            let _ = e.take(64 * 1024).read_to_end(&mut s);
        }
        s
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => return Err(OcrError::Tesseract(e.to_string())),
        }
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(OcrError::Cancelled);
        }
        if started.elapsed() > PAGE_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(OcrError::Tesseract(format!(
                "Tesseract took longer than {} seconds on one page and was stopped",
                PAGE_TIMEOUT.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(15));
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    if !status.success() {
        let msg = String::from_utf8_lossy(&stderr);
        let first = msg
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("it failed");
        return Err(OcrError::Tesseract(format!(
            "Tesseract could not read the page: {}",
            first.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

/// Words from Tesseract's TSV output, grouped into lines.
pub(crate) fn parse_tsv(tsv: &str, width: u32, height: u32) -> OcrPage {
    let mut page = OcrPage {
        width,
        height,
        lines: Vec::new(),
        turned: 0,
    };
    let mut current: Option<((u32, u32, u32), OcrLine)> = None;
    for row in tsv.lines().skip(1) {
        let cols: Vec<&str> = row.split('\t').collect();
        if cols.len() < 12 || cols[0] != "5" {
            continue;
        }
        let num = |i: usize| cols[i].trim().parse::<i64>().unwrap_or(0);
        let text = cols[11..].join("\t");
        let text = text.trim();
        let conf = cols[10].trim().parse::<f32>().unwrap_or(-1.0);
        if text.is_empty() || conf < 0.0 {
            continue;
        }
        let key = (
            u32::try_from(num(2)).unwrap_or(0),
            u32::try_from(num(3)).unwrap_or(0),
            u32::try_from(num(4)).unwrap_or(0),
        );
        let (l, t, w, h) = (num(6) as f32, num(7) as f32, num(8) as f32, num(9) as f32);
        let word = OcrWord {
            text: text.to_owned(),
            rect: Rect {
                x0: l,
                y0: t,
                x1: l + w,
                y1: t + h,
            },
            confidence: Some(conf / 100.0),
        };
        match current.as_mut() {
            Some((k, line)) if *k == key => line.push(word),
            _ => {
                if let Some((_, line)) = current.take() {
                    page.lines.push(line);
                }
                let mut line = OcrLine::default();
                line.push(word);
                current = Some((key, line));
            }
        }
    }
    if let Some((_, line)) = current {
        page.lines.push(line);
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_words_group_into_lines() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
            1\t1\t0\t0\t0\t0\t0\t0\t100\t100\t-1\t\n\
            5\t1\t1\t1\t1\t1\t10\t10\t30\t12\t96.5\tHello\n\
            5\t1\t1\t1\t1\t2\t45\t10\t30\t12\t91\tworld\n\
            5\t1\t1\t1\t2\t1\t10\t30\t20\t12\t88\tBye\n\
            5\t1\t1\t1\t2\t2\t40\t30\t20\t12\t95\t \n";
        let page = parse_tsv(tsv, 100, 100);
        assert_eq!(page.lines.len(), 2);
        assert_eq!(page.lines[0].text(), "Hello world");
        assert_eq!(page.lines[0].rect.x1, 75.0);
        assert_eq!(page.lines[1].text(), "Bye");
        assert!((page.lines[0].words[0].confidence.unwrap_or(0.0) - 0.965).abs() < 1e-4);
    }
}
