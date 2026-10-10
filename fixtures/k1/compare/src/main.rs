//! The carta spike's comparison (B1-k1): each sample in `fixtures/k1` read
//! by carta and by Pandoc (both through textweaver's HTML loader), and LaTeX
//! by the native loader too, written as canonical text with marker counts.
//! Then a probe of whether a carta reader follows includes to other files.
//!
//! A standalone package, outside the workspace. Run it from this folder:
//! `cargo run --release -- .. OUT_DIR`, then diff `OUT_DIR/FORMAT.pandoc.txt`
//! with `OUT_DIR/FORMAT.carta.txt`. Needs `pandoc` on `PATH`.

use std::path::Path;
use textweaver_core::MarkerKind;
use textweaver_formats::{LatexLoader, LoadOptions, Loader, PandocLoader, Source};
use textweaver_text::Document;

fn summary(doc: &Document) -> String {
    let idx = doc.marker_index();
    let kinds = [
        ("headings", MarkerKind::Heading),
        ("list items", MarkerKind::ListItem),
        ("tables", MarkerKind::Table),
        ("table rows", MarkerKind::TableRow),
        ("links", MarkerKind::Link),
        ("code", MarkerKind::Code),
        ("quotes", MarkerKind::Quote),
    ];
    let mut s = format!("title: {:?}\n", doc.meta.title);
    for (n, k) in kinds {
        s.push_str(&format!("{n}: {}\n", idx.count(k, None)));
    }
    s
}

fn via_carta(from: &str, bytes: &[u8]) -> Result<Document, String> {
    let text = String::from_utf8_lossy(bytes);
    let started = std::time::Instant::now();
    let (doc, media) = carta::read_document(from, text.as_bytes(), &Default::default())
        .map_err(|e| e.to_string())?;
    let title = match doc.meta.get("title") {
        Some(carta::ast::MetaValue::MetaString(t)) => t.to_string(),
        Some(carta::ast::MetaValue::MetaInlines(i)) => carta::ast::to_plain_text(i),
        _ => String::new(),
    };
    let mut w = carta::WriterOptions::default();
    w.math_method = carta::MathMethod::Mathml;
    let carta::Output::Text(body) =
        carta::render_document("html", doc, media, &w).map_err(|e| e.to_string())?
    else {
        return Err("bytes".into());
    };
    let html = if title.is_empty() {
        body
    } else {
        format!("<h1>{title}</h1>{body}")
    };
    let took = started.elapsed();
    eprintln!("carta {from}: {took:?}");
    let src = Source::Bytes {
        data: html.into_bytes(),
        hint: "html".into(),
    };
    textweaver_formats::HtmlLoader
        .load(&src, &LoadOptions::default())
        .map_err(|e| e.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let fixtures = Path::new(&args[1]);
    let out = Path::new(&args[2]);
    std::fs::create_dir_all(out).unwrap();
    let cases = [
        ("sample.org", "org", "org"),
        ("sample.rst", "rst", "rst"),
        ("sample.mediawiki", "mediawiki", "mediawiki"),
        ("sample.dokuwiki", "dokuwiki", "dokuwiki"),
        ("sample.jira", "jira", "jira"),
        ("sample.typ", "typst", "typ"),
        ("sample.tex", "latex", "tex"),
    ];
    let opts = LoadOptions::default();
    for (file, from, hint) in cases {
        let bytes = std::fs::read(fixtures.join(file)).unwrap();
        let write = |engine: &str, r: Result<Document, String>| {
            let body = match r {
                Ok(d) => format!("{}----\n{}", summary(&d), d.text()),
                Err(e) => format!("ERROR: {e}\n"),
            };
            std::fs::write(out.join(format!("{from}.{engine}.txt")), body).unwrap();
        };
        write("carta", via_carta(from, &bytes));
        // Pandoc's reader names; the loader picks by extension hint.
        let pandoc_hint = match from {
            "dokuwiki" | "jira" => None,
            _ => Some(hint),
        };
        if let Some(h) = pandoc_hint {
            let src = Source::Bytes {
                data: bytes.clone(),
                hint: h.into(),
            };
            write(
                "pandoc",
                PandocLoader::default()
                    .load(&src, &opts)
                    .map_err(|e| e.to_string()),
            );
        } else {
            // No extension: through pandoc to HTML by hand, then the HTML loader.
            let o = std::process::Command::new("pandoc")
                .args(["--from", from, "--to", "html5", "--wrap=none", "--sandbox"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut c| {
                    use std::io::Write;
                    c.stdin.take().unwrap().write_all(&bytes)?;
                    c.wait_with_output()
                })
                .unwrap();
            let src = Source::Bytes {
                data: o.stdout,
                hint: "html".into(),
            };
            write(
                "pandoc",
                textweaver_formats::HtmlLoader
                    .load(&src, &opts)
                    .map_err(|e| e.to_string()),
            );
        }
        if from == "latex" {
            let src = Source::Bytes {
                data: bytes.clone(),
                hint: "tex".into(),
            };
            write(
                "native",
                LatexLoader.load(&src, &opts).map_err(|e| e.to_string()),
            );
        }
    }
    // The include probe: does carta read a file named in the source?
    let secret = out.join("secret.txt");
    std::fs::write(&secret, "The secret word is pelican.\n").unwrap();
    let p = secret.display().to_string();
    for (from, src) in [
        ("rst", format!("Before.\n\n.. include:: {p}\n\nAfter.\n")),
        (
            "typst",
            format!("Before.\n\n#read(\"{}\")\n\nAfter.\n", p.replace('\\', "/")),
        ),
        ("org", format!("Before.\n\n#+INCLUDE: \"{p}\"\n\nAfter.\n")),
        ("latex", format!("Before.\n\n\\input{{{p}}}\n\nAfter.\n")),
    ] {
        let html =
            carta::convert_text(from, "html", &src, &Default::default(), &Default::default());
        println!(
            "include probe {from}: reads file = {}",
            html.map(|h| h.contains("pelican")).unwrap_or(false)
        );
    }
}
