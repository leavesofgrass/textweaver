//! Pathological inputs for `cargo xtask bench` (W8b-i, from star's
//! 34-second hard wrap of one 5 MB token): one generated file per loader,
//! each holding the shapes that turn a linear loader quadratic:
//!
//! - a very long token (512 KB with no space or break in it);
//! - deeply nested lists (200 levels, or the format's nearest thing:
//!   nested RTF groups, JSON arrays, SVG groups, MathML rows);
//! - a huge table (2,000 rows of 10 cells);
//! - a 1 MB line (one paragraph with no line break).
//!
//! The bench loads each file and plans its narration, and fails when one
//! takes longer than a ceiling (10 seconds by default, `--ceiling-s`).
//! The fuzz targets catch crashes, not slowness; this catches slowness.
//!
//! Every file is built here from constants, so it is the same on every
//! machine. The text is plain ASCII words, so no format needs escapes.
//! Loaders not covered: images (OCR), DAISY, PowerPoint, and archives,
//! whose text comes through the loaders covered here.

// Without the feature only the tests use it.
#![cfg_attr(not(feature = "bench"), allow(dead_code))]

use std::io::Write as _;

/// Length of the long token, in bytes.
pub(crate) const TOKEN_BYTES: usize = 512 * 1024;
/// Length of the long line, in bytes.
pub(crate) const LINE_BYTES: usize = 1024 * 1024;
/// How deep the lists nest.
pub(crate) const DEPTH: usize = 200;
/// Rows and cells of the table.
pub(crate) const ROWS: usize = 2_000;
pub(crate) const COLS: usize = 10;

/// One generated file.
pub(crate) struct Input {
    /// The file name (plain: letters, digits, `-` and `.`).
    pub name: &'static str,
    /// The loader it is meant for, as the report names it.
    pub loader: &'static str,
    pub bytes: Vec<u8>,
}

const ASCII_WORDS: &[&str] = &[
    "the",
    "reader",
    "speech",
    "highlight",
    "document",
    "students",
    "voice",
    "sentence",
    "paragraph",
    "keyboard",
    "screen",
    "quickly",
    "position",
    "library",
    "chapter",
    "notes",
    "and",
    "of",
    "to",
    "in",
    "is",
    "for",
    "with",
    "every",
    "word",
    "never",
    "drifts",
];

/// A token of `n` letters with nothing to break on.
pub(crate) fn token(n: usize) -> String {
    (0..n)
        .map(|i| char::from(b'a' + u8::try_from(i % 26).unwrap_or(0)))
        .collect()
}

/// About `n` bytes of words and sentences on one line, no line break.
pub(crate) fn line(n: usize) -> String {
    let mut r = crate::bench::Lcg(11);
    let mut out = String::with_capacity(n + 32);
    let mut in_sentence = 0;
    while out.len() < n {
        let w = ASCII_WORDS[r.below(ASCII_WORDS.len())];
        if in_sentence == 0 {
            let mut c = w.chars();
            if let Some(f) = c.next() {
                out.extend(f.to_uppercase());
                out.push_str(c.as_str());
            }
        } else {
            out.push_str(w);
        }
        in_sentence += 1;
        if in_sentence > 6 + r.below(14) {
            out.push_str(". ");
            in_sentence = 0;
        } else {
            out.push(' ');
        }
    }
    out.truncate(out.trim_end().len());
    out.push('.');
    out
}

/// The cell at `row`, `col`.
fn cell(row: usize, col: usize) -> String {
    format!(
        "r{row}c{col} {}",
        ASCII_WORDS[(row * 7 + col) % ASCII_WORDS.len()]
    )
}

/// The parts every file is made of.
struct Parts {
    token: String,
    line: String,
}

impl Parts {
    fn new() -> Self {
        Parts {
            token: token(TOKEN_BYTES),
            line: line(LINE_BYTES),
        }
    }
}

fn markdown(p: &Parts) -> String {
    let mut s = String::from("# Pathological input\n\n");
    s.push_str(&p.token);
    s.push_str("\n\n");
    for d in 0..DEPTH {
        s.push_str(&"  ".repeat(d));
        s.push_str(&format!("- level {d}\n"));
    }
    s.push('\n');
    s.push('|');
    for c in 0..COLS {
        s.push_str(&format!(" h{c} |"));
    }
    s.push_str("\n|");
    s.push_str(&"---|".repeat(COLS));
    s.push('\n');
    for r in 0..ROWS {
        s.push('|');
        for c in 0..COLS {
            s.push(' ');
            s.push_str(&cell(r, c));
            s.push_str(" |");
        }
        s.push('\n');
    }
    s.push('\n');
    s.push_str(&p.line);
    s.push('\n');
    s
}

fn text(p: &Parts) -> String {
    let mut s = String::from("Pathological input\n\n");
    s.push_str(&p.token);
    s.push_str("\n\n");
    for d in 0..DEPTH {
        s.push_str(&"  ".repeat(d));
        s.push_str(&format!("* level {d}\n"));
    }
    s.push('\n');
    for r in 0..ROWS {
        let row: Vec<String> = (0..COLS).map(|c| cell(r, c)).collect();
        s.push_str(&row.join("\t"));
        s.push('\n');
    }
    s.push('\n');
    s.push_str(&p.line);
    s.push('\n');
    s
}

/// The body of the HTML-like files: token, nested lists, table, line.
fn html_body(p: &Parts) -> String {
    let mut s = String::from("<h1>Pathological input</h1>\n<p>");
    s.push_str(&p.token);
    s.push_str("</p>\n");
    for d in 0..DEPTH {
        s.push_str(&format!("<ul><li>level {d}"));
    }
    for _ in 0..DEPTH {
        s.push_str("</li></ul>");
    }
    s.push_str("\n<table>\n<tr>");
    for c in 0..COLS {
        s.push_str(&format!("<th>h{c}</th>"));
    }
    s.push_str("</tr>\n");
    for r in 0..ROWS {
        s.push_str("<tr>");
        for c in 0..COLS {
            s.push_str("<td>");
            s.push_str(&cell(r, c));
            s.push_str("</td>");
        }
        s.push_str("</tr>\n");
    }
    s.push_str("</table>\n<p>");
    s.push_str(&p.line);
    s.push_str("</p>\n");
    s
}

fn html(p: &Parts) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>Pathological input</title></head><body>\n{}</body></html>\n",
        html_body(p)
    )
}

fn xhtml(p: &Parts) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"en\"><head><title>Pathological input</title></head><body>\n{}</body></html>\n",
        html_body(p)
    )
}

fn latex(p: &Parts) -> String {
    let mut s = String::from(
        "\\documentclass{article}\n\\begin{document}\n\\section{Pathological input}\n\n",
    );
    s.push_str(&p.token);
    s.push_str("\n\n");
    for d in 0..DEPTH {
        s.push_str(&format!("\\begin{{itemize}}\n\\item level {d}\n"));
    }
    for _ in 0..DEPTH {
        s.push_str("\\end{itemize}\n");
    }
    s.push_str(&format!("\n\\begin{{tabular}}{{{}}}\n", "l".repeat(COLS)));
    for r in 0..ROWS {
        let row: Vec<String> = (0..COLS).map(|c| cell(r, c)).collect();
        s.push_str(&row.join(" & "));
        s.push_str(" \\\\\n");
    }
    s.push_str("\\end{tabular}\n\n");
    s.push_str(&p.line);
    s.push_str("\n\n\\end{document}\n");
    s
}

fn rtf(p: &Parts) -> String {
    let mut s = String::from("{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Times New Roman;}}\n");
    s.push_str("\\pard Pathological input\\par\n\\pard ");
    s.push_str(&p.token);
    s.push_str("\\par\n");
    // Nested groups, the nearest thing RTF has to nested lists.
    s.push_str(&"{".repeat(DEPTH));
    s.push_str("\\pard deep inside\\par");
    s.push_str(&"}".repeat(DEPTH));
    s.push('\n');
    for d in 0..DEPTH {
        s.push_str(&format!("\\pard\\li{} level {d}\\par\n", (d + 1) * 20));
    }
    for r in 0..ROWS {
        s.push_str("\\trowd");
        for c in 0..COLS {
            s.push_str(&format!("\\cellx{}", (c + 1) * 900));
        }
        for c in 0..COLS {
            s.push_str("\\pard\\intbl ");
            s.push_str(&cell(r, c));
            s.push_str("\\cell");
        }
        s.push_str("\\row\n");
    }
    s.push_str("\\pard ");
    s.push_str(&p.line);
    s.push_str("\\par\n}\n");
    s
}

fn eml(p: &Parts) -> String {
    format!(
        "From: Ada Example <ada@example.org>\r\nTo: Grace Example <grace@example.org>\r\nSubject: Pathological input\r\nDate: Thu, 01 Oct 2026 12:00:00 +0000\r\nMIME-Version: 1.0\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{}",
        html(p)
    )
}

fn mhtml(p: &Parts) -> String {
    format!(
        "From: <saved by textweaver bench>\r\nSubject: Pathological input\r\nMIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=\"tw-bench\"; type=\"text/html\"\r\n\r\n--tw-bench\r\nContent-Type: text/html; charset=utf-8\r\nContent-Location: https://example.org/pathological.html\r\n\r\n{}\r\n--tw-bench--\r\n",
        html(p)
    )
}

fn csv(p: &Parts) -> String {
    let mut s = String::new();
    let head: Vec<String> = (0..COLS).map(|c| format!("h{c}")).collect();
    s.push_str(&head.join(","));
    s.push('\n');
    for r in 0..ROWS {
        let row: Vec<String> = (0..COLS).map(|c| cell(r, c)).collect();
        s.push_str(&row.join(","));
        s.push('\n');
    }
    s.push_str(&format!("\"{}\"{}\n", p.token, ",".repeat(COLS - 1)));
    s.push_str(&format!("\"{}\"{}\n", p.line, ",".repeat(COLS - 1)));
    s
}

fn json(p: &Parts) -> String {
    let mut s = String::from("{\"title\": \"Pathological input\", \"token\": \"");
    s.push_str(&p.token);
    s.push_str("\", \"nested\": ");
    s.push_str(&"[".repeat(DEPTH));
    s.push_str("\"deep inside\"");
    s.push_str(&"]".repeat(DEPTH));
    s.push_str(", \"table\": [\n");
    for r in 0..ROWS {
        let row: Vec<String> = (0..COLS)
            .map(|c| format!("\"h{c}\": \"{}\"", cell(r, c)))
            .collect();
        s.push('{');
        s.push_str(&row.join(", "));
        s.push('}');
        if r + 1 < ROWS {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("], \"line\": \"");
    s.push_str(&p.line);
    s.push_str("\"}\n");
    s
}

fn notebook(p: &Parts) -> String {
    let source = serde_json::Value::String(markdown(p));
    let nb = serde_json::json!({
        "cells": [{"cell_type": "markdown", "metadata": {}, "source": source}],
        "metadata": {},
        "nbformat": 4,
        "nbformat_minor": 5,
    });
    nb.to_string()
}

fn fodt(p: &Parts) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" office:version=\"1.3\" office:mimetype=\"application/vnd.oasis.opendocument.text\"><office:body><office:text>\n<text:h text:outline-level=\"1\">Pathological input</text:h>\n<text:p>",
    );
    s.push_str(&p.token);
    s.push_str("</text:p>\n");
    for d in 0..DEPTH {
        s.push_str(&format!(
            "<text:list><text:list-item><text:p>level {d}</text:p>"
        ));
    }
    for _ in 0..DEPTH {
        s.push_str("</text:list-item></text:list>");
    }
    s.push_str("\n<table:table table:name=\"Big\">\n");
    for r in 0..ROWS {
        s.push_str("<table:table-row>");
        for c in 0..COLS {
            s.push_str("<table:table-cell><text:p>");
            s.push_str(&cell(r, c));
            s.push_str("</text:p></table:table-cell>");
        }
        s.push_str("</table:table-row>\n");
    }
    s.push_str("</table:table>\n<text:p>");
    s.push_str(&p.line);
    s.push_str("</text:p>\n</office:text></office:body></office:document>\n");
    s
}

fn svg(p: &Parts) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"800\" height=\"600\"><title>Pathological input</title>\n",
    );
    s.push_str(&"<g>".repeat(DEPTH));
    s.push_str("<text x=\"10\" y=\"20\">");
    s.push_str(&p.token);
    s.push_str("</text>");
    s.push_str(&"</g>".repeat(DEPTH));
    for r in 0..ROWS {
        s.push_str(&format!(
            "<text x=\"10\" y=\"{}\">{}</text>\n",
            40 + r,
            cell(r, 0)
        ));
    }
    s.push_str("<text x=\"10\" y=\"30\">");
    s.push_str(&p.line);
    s.push_str("</text>\n</svg>\n");
    s
}

fn mathml(p: &Parts) -> String {
    // The token sits inside the 200 nested rows, past the loader's depth
    // limit, where it is read letter by letter. That took 189 s for 512 KB
    // until W8b-ml made the LaTeX builder linear; the ceiling guards it.
    let mut s = String::from("<math xmlns=\"http://www.w3.org/1998/Math/MathML\">\n");
    s.push_str(&"<mrow>".repeat(DEPTH));
    s.push_str("<mi>");
    s.push_str(&p.token);
    s.push_str("</mi>");
    s.push_str(&"</mrow>".repeat(DEPTH));
    s.push_str("\n<mtable>\n");
    for r in 0..ROWS / 4 {
        s.push_str("<mtr>");
        for c in 0..COLS {
            s.push_str(&format!("<mtd><mn>{}</mn></mtd>", r * COLS + c));
        }
        s.push_str("</mtr>\n");
    }
    s.push_str("</mtable>\n<mtext>");
    s.push_str(&p.line);
    s.push_str("</mtext>\n</math>\n");
    s
}

/// A zip archive of `(name, contents)`, the first entry stored (EPUB's
/// `mimetype` must be), the rest deflated.
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut out);
        for (i, (name, data)) in entries.iter().enumerate() {
            let method = if i == 0 {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            };
            let options = zip::write::SimpleFileOptions::default().compression_method(method);
            // Writing to memory cannot fail but for a bug; an empty file
            // then fails to load and says so.
            if z.start_file(*name, options).is_err() || z.write_all(data).is_err() {
                return Vec::new();
            }
        }
        if z.finish().is_err() {
            return Vec::new();
        }
    }
    out.into_inner()
}

fn docx(p: &Parts) -> Vec<u8> {
    let para = |t: &str| format!("<w:p><w:r><w:t xml:space=\"preserve\">{t}</w:t></w:r></w:p>");
    let mut body = String::new();
    body.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Pathological input</w:t></w:r></w:p>");
    body.push_str(&para(&p.token));
    for d in 0..DEPTH {
        body.push_str(&format!(
            "<w:p><w:pPr><w:numPr><w:ilvl w:val=\"{}\"/><w:numId w:val=\"1\"/></w:numPr><w:ind w:left=\"{}\"/></w:pPr><w:r><w:t>level {d}</w:t></w:r></w:p>",
            d % 9,
            (d + 1) * 20
        ));
    }
    body.push_str("<w:tbl>");
    for r in 0..ROWS {
        body.push_str("<w:tr>");
        for c in 0..COLS {
            body.push_str("<w:tc>");
            body.push_str(&para(&cell(r, c)));
            body.push_str("</w:tc>");
        }
        body.push_str("</w:tr>");
    }
    body.push_str("</w:tbl>");
    body.push_str(&para(&p.line));
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}</w:body></w:document>"
    );
    let types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
    zip(&[
        ("[Content_Types].xml", types.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("word/document.xml", document.as_bytes()),
    ])
}

fn epub(p: &Parts) -> Vec<u8> {
    let container = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>";
    let opf = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"id\"><metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:identifier id=\"id\">urn:uuid:00000000-0000-4000-8000-000000000000</dc:identifier><dc:title>Pathological input</dc:title><dc:language>en</dc:language><meta property=\"dcterms:modified\">2026-10-01T00:00:00Z</meta></metadata><manifest><item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/><item id=\"ch1\" href=\"ch1.xhtml\" media-type=\"application/xhtml+xml\"/></manifest><spine><itemref idref=\"ch1\"/></spine></package>";
    let nav = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\"><head><title>Contents</title></head><body><nav epub:type=\"toc\"><ol><li><a href=\"ch1.xhtml\">Pathological input</a></li></ol></nav></body></html>";
    let chapter = xhtml(p);
    zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container.as_bytes()),
        ("OEBPS/content.opf", opf.as_bytes()),
        ("OEBPS/nav.xhtml", nav.as_bytes()),
        ("OEBPS/ch1.xhtml", chapter.as_bytes()),
    ])
}

/// A PDF of three pages: the token in one text operator, 2,000 table rows
/// placed line by line, and the 1 MB line in one text operator.
fn pdf(p: &Parts) -> Vec<u8> {
    let mut table = String::from("BT /F1 6 Tf 20 780 Td 7 TL\n");
    for r in 0..ROWS {
        let row: Vec<String> = (0..COLS).map(|c| cell(r, c)).collect();
        table.push_str(&format!("({}) '\n", row.join("  ")));
    }
    table.push_str("ET\n");
    let streams = [
        format!("BT /F1 12 Tf 72 720 Td ({}) Tj ET\n", p.token),
        table,
        format!("BT /F1 12 Tf 72 720 Td ({}) Tj ET\n", p.line),
    ];
    let pages = streams.len();
    // Objects: 1 catalog, 2 pages, 3 font, then a page and its contents
    // for each stream.
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {pages} >>",
            (0..pages)
                .map(|i| format!("{} 0 R", 4 + 2 * i))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
    ];
    for (i, s) in streams.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            5 + 2 * i
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{s}endstream", s.len()));
    }
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Every generated input, one per loader.
pub(crate) fn inputs() -> Vec<Input> {
    let p = Parts::new();
    let text_input = |name, loader, s: String| Input {
        name,
        loader,
        bytes: s.into_bytes(),
    };
    vec![
        text_input("patho.txt", "text", text(&p)),
        text_input("patho.md", "markdown", markdown(&p)),
        text_input("patho.html", "html", html(&p)),
        text_input("patho.tex", "latex", latex(&p)),
        text_input("patho.rtf", "rtf", rtf(&p)),
        text_input("patho.eml", "eml", eml(&p)),
        text_input("patho.mhtml", "mhtml", mhtml(&p)),
        text_input("patho.csv", "sheet", csv(&p)),
        text_input("patho.json", "json", json(&p)),
        text_input("patho.ipynb", "notebook", notebook(&p)),
        text_input("patho.fodt", "odt", fodt(&p)),
        text_input("patho.svg", "svg", svg(&p)),
        text_input("patho.mml", "mathml", mathml(&p)),
        Input {
            name: "patho.docx",
            loader: "docx",
            bytes: docx(&p),
        },
        Input {
            name: "patho.epub",
            loader: "epub",
            bytes: epub(&p),
        },
        Input {
            name: "patho.pdf",
            loader: "pdf",
            bytes: pdf(&p),
        },
    ]
}

/// The default time ceiling per file (load plus narration plan).
pub(crate) const DEFAULT_CEILING_S: f64 = 10.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shapes_have_their_sizes() {
        let t = token(1000);
        assert_eq!(t.len(), 1000);
        assert!(!t.contains(char::is_whitespace));
        let l = line(50_000);
        assert!(l.len() >= 50_000 && l.len() < 50_100);
        assert!(!l.contains('\n'));
        assert!(l.is_ascii());
        assert_eq!(l, line(50_000), "deterministic");
    }

    #[test]
    fn every_input_is_built_with_plain_names() {
        let all = inputs();
        assert_eq!(all.len(), 16);
        let mut loaders: Vec<&str> = all.iter().map(|i| i.loader).collect();
        loaders.sort_unstable();
        loaders.dedup();
        assert_eq!(loaders.len(), all.len(), "one file per loader");
        for i in &all {
            assert!(
                i.name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.'),
                "{}",
                i.name
            );
            // Zipped inputs are compressed; the rest hold the line as is.
            let zipped = i.bytes.starts_with(b"PK");
            assert!(
                zipped || i.bytes.len() > LINE_BYTES,
                "{} holds the long line",
                i.name
            );
            assert!(i.bytes.len() > 10_000, "{} is not empty", i.name);
        }
        let pdf = &all.iter().find(|i| i.loader == "pdf").map(|i| &i.bytes);
        assert!(pdf.is_some_and(|b| b.starts_with(b"%PDF-1.7") && b.ends_with(b"%%EOF\n")));
        let epub = all.iter().find(|i| i.loader == "epub").map(|i| &i.bytes);
        assert!(epub.is_some_and(|b| b.starts_with(b"PK")));
    }
}
