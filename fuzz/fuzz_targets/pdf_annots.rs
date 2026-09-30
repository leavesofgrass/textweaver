//! Fuzz target: PDF links, comments, and form fields (ADR-0048). A PDF is
//! loaded as it is. Other input is split at NUL bytes into a page's content
//! stream, its `/Annots` array, the catalog's `/AcroForm` and `/Names`
//! dictionaries, and then extra objects numbered from 10, all written
//! as PDF syntax into a one-page file with a correct cross-reference table,
//! so the fuzzer spends its time on annotations and fields rather than on
//! the file's framing. Every comment must lie inside the text, every link
//! must have a reference, and a page anchor must find its page or nothing.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_core::MarkerKind;
use textweaver_formats::{Registry, Source, comments};

/// A one-page PDF around the parts, with a cross-reference table.
fn wrap(data: &[u8]) -> Vec<u8> {
    let mut parts = data.split(|&b| b == 0);
    let content = parts.next().unwrap_or_default();
    let annots = parts.next().unwrap_or_default();
    let form = parts.next().unwrap_or_default();
    let names = parts.next().unwrap_or_default();
    let or = |part: &[u8], default: &[u8]| {
        if part.is_empty() {
            default.to_vec()
        } else {
            part.to_vec()
        }
    };
    let mut objects: Vec<(u32, Vec<u8>)> = vec![
        (
            1,
            [
                b"<< /Type /Catalog /Pages 2 0 R /AcroForm ".as_slice(),
                &or(form, b"<< /Fields [] >>"),
                b" /Names ",
                &or(names, b"<< >>"),
                b" >>",
            ]
            .concat(),
        ),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
        (
            3,
            [
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] ".as_slice(),
                b"/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots ",
                &or(annots, b"[]"),
                b" >>",
            ]
            .concat(),
        ),
        (
            4,
            [
                format!("<< /Length {} >>\nstream\n", content.len()).as_bytes(),
                content,
                b"\nendstream",
            ]
            .concat(),
        ),
        (
            5,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        ),
    ];
    for (i, part) in parts.take(64).enumerate() {
        objects.push((10 + i as u32, part.to_vec()));
    }
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (n, body) in &objects {
        offsets.push((*n, out.len()));
        out.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let size = objects.iter().map(|(n, _)| n + 1).max().unwrap_or(1);
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for n in 1..size {
        match offsets.iter().find(|(k, _)| *k == n) {
            Some((_, at)) => out.extend_from_slice(format!("{at:010} 00000 n \n").as_bytes()),
            None => out.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    out
}

fuzz_target!(|data: &[u8]| {
    let pdf = if data.starts_with(b"%PDF") {
        data.to_vec()
    } else {
        wrap(data)
    };
    let source = Source::Bytes {
        data: pdf,
        hint: "pdf".into(),
    };
    let Ok(doc) = Registry::with_builtins().load(&source, &textweaver_fuzz::options()) else {
        return;
    };
    textweaver_fuzz::check(&doc);
    let len = doc.len_chars();
    for c in comments(&doc.meta) {
        assert!(c.range.start <= c.range.end, "{c:?} runs backwards");
        assert!(c.range.end.0 <= len, "{c:?} past the end ({len})");
        assert!(!c.text.is_empty(), "{c:?} says nothing");
    }
    for m in doc.marker_index().iter(MarkerKind::Link, None) {
        let r = m.reference.as_deref().unwrap_or_default();
        assert!(!r.is_empty(), "{m:?} has no reference");
        assert!(!r.chars().any(char::is_control), "{m:?} has a control character");
        if textweaver_formats::pdf::is_page_anchor(r)
            && let Some(at) = textweaver_formats::pdf::page_anchor(&doc, r)
        {
            assert!(at.0 <= len, "{r} found past the end");
        }
    }
});
