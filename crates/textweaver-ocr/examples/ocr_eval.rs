//! Measures OCR quality and speed on test pages (Agent W3d's evaluation).
//!
//! ```text
//! TEXTWEAVER_OCR_MODELS=.cache/models cargo run --release -p textweaver-ocr \
//!     --example ocr_eval -- ENGINE LANG FILE...
//! ```
//!
//! ENGINE is `ocrs`, `tesseract`, or `paddle`; LANG is a Tesseract code
//! (`eng`, `fra`). Each FILE is a PDF or a PNG/JPEG image, with its ground
//! truth in a `.txt` file of the same name. For each page this prints the
//! time taken and the word and character error rates (edit distance over
//! the truth's length; lower is better).

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use textweaver_ocr::pdf::PdfPages;
use textweaver_ocr::{EngineChoice, GrayImage, plan, recognize};

fn edit_distance<T: PartialEq>(a: &[T], b: &[T]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn rates(truth: &str, got: &str) -> (f64, f64) {
    let tw: Vec<&str> = truth.split_whitespace().collect();
    let gw: Vec<&str> = got.split_whitespace().collect();
    let tc: Vec<char> = tw.join(" ").chars().collect();
    let gc: Vec<char> = gw.join(" ").chars().collect();
    (
        edit_distance(&tw, &gw) as f64 / tw.len().max(1) as f64,
        edit_distance(&tc, &gc) as f64 / tc.len().max(1) as f64,
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [engine, lang, files @ ..] = args.as_slice() else {
        eprintln!("usage: ocr_eval ENGINE LANG FILE...");
        std::process::exit(2);
    };
    let choice = EngineChoice::parse(engine).unwrap_or(EngineChoice::Auto);
    let p = match plan(choice, lang) {
        Ok(p) => p,
        Err(u) => {
            eprintln!("{}", u.message);
            std::process::exit(1);
        }
    };
    let t0 = Instant::now();
    if let Err(e) = textweaver_ocr::warm_up(&p) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    println!(
        "engine {} ({}), models loaded in {:.2} s",
        p.engine.name(),
        p.langs,
        t0.elapsed().as_secs_f64()
    );
    let cancel = AtomicBool::new(false);
    for file in files {
        let path = Path::new(file);
        let bytes = std::fs::read(path).expect("read");
        let images: Vec<(GrayImage, String)> =
            if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
                let pdf = PdfPages::open(bytes).expect("pdf");
                (0..pdf.len())
                    .map(|i| {
                        let t = Instant::now();
                        let (img, kind) = pdf.page_image(i).expect("page");
                        let how = format!(
                            "page {} {:?} {}x{} in {:.3} s",
                            i + 1,
                            kind,
                            img.width,
                            img.height,
                            t.elapsed().as_secs_f64()
                        );
                        (img, how)
                    })
                    .collect()
            } else {
                vec![(GrayImage::decode(&bytes).expect("image"), "image".into())]
            };
        let mut text = String::new();
        let mut secs = 0.0;
        for (n, (img, how)) in images.iter().enumerate() {
            if let Some(dir) = std::env::var_os("OCR_EVAL_DUMP") {
                let out = Path::new(&dir).join(format!(
                    "{}-{n}.png",
                    path.file_stem().unwrap_or_default().to_string_lossy()
                ));
                std::fs::write(out, img.to_png().expect("png")).expect("dump");
            }
            let t = Instant::now();
            let page = recognize(&p, img, &cancel).expect("recognize");
            let s = t.elapsed().as_secs_f64();
            secs += s;
            println!("{file}: {how}; recognized in {s:.2} s");
            text.push_str(&page.text());
            text.push('\n');
        }
        let truth = std::fs::read_to_string(path.with_extension("txt")).unwrap_or_default();
        let (wer, cer) = rates(&truth, &text);
        println!(
            "{file}: {} pages in {secs:.2} s; word errors {:.1}%, character errors {:.1}%",
            images.len(),
            wer * 100.0,
            cer * 100.0
        );
        if std::env::var_os("OCR_EVAL_SHOW").is_some() {
            println!("---\n{text}---");
        }
    }
}
