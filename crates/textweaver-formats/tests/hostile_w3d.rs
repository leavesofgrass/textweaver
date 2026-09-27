//! Malformed and hostile input for the Wave 3 loaders (Agent W3d):
//! archives, DAISY and DTBook, PowerPoint, spreadsheets, pictures, and
//! web responses (feature `url`).
//! Every case loads, or fails with a clear error, quickly and without
//! exhausting memory.

use std::io::{Cursor, Write};

use textweaver_core::MarkerKind;
use textweaver_formats::{
    LoadError, LoadOptions, NESTING_WARNING, OcrOptions, Registry, Source, archive, warnings,
};
use textweaver_text::Document;
use zip::write::SimpleFileOptions;

fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in files {
        z.start_file(*name, SimpleFileOptions::default()).unwrap();
        z.write_all(body).unwrap();
    }
    z.finish().unwrap().into_inner()
}

/// Loading options with OCR off, so no engine runs on hostile pictures.
fn no_ocr() -> LoadOptions {
    LoadOptions {
        ocr: OcrOptions {
            enabled: false,
            ..OcrOptions::default()
        },
        ..LoadOptions::default()
    }
}

fn load(data: Vec<u8>, hint: &str) -> Result<Document, LoadError> {
    Registry::with_builtins().load(
        &Source::Bytes {
            data,
            hint: hint.into(),
        },
        &no_ocr(),
    )
}

#[test]
fn archives_nested_too_deeply_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut inner = zip(&[("end.txt", b"deep")]);
    let mut path = String::new();
    for i in 0..6 {
        let name = format!("level{i}.zip");
        inner = zip(&[(name.as_str(), &inner)]);
        path = format!("{name}!{path}");
    }
    let top = dir.path().join("top.zip");
    std::fs::write(&top, &inner).unwrap();
    let member = dir.path().join(format!("top.zip!{path}end.txt"));
    let err = archive::read_path(&member).unwrap_err().to_string();
    assert!(err.contains("nested too deeply"), "{err}");
}

#[test]
fn huge_archive_listings_are_cut() {
    let names: Vec<String> = (0..archive::MAX_LISTED + 50)
        .map(|i| format!("f{i}.txt"))
        .collect();
    let files: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x"[..])).collect();
    let doc = load(zip(&files), "zip").unwrap();
    let links = doc.marker_index().iter(MarkerKind::Link, None).count();
    assert_eq!(links, archive::MAX_LISTED);
    assert!(
        doc.text()
            .to_string()
            .ends_with("Only the first 10,000 files are listed.")
    );
}

#[test]
fn broken_archives_fail_clearly() {
    for (bytes, hint) in [
        (b"PK\x03\x04garbage".to_vec(), "zip"),
        (
            vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0, 4, 1, 2, 3],
            "7z",
        ),
        (vec![0x1F, 0x8B, 8, 0, 0, 0, 0, 0, 0, 3, 1, 2, 3], "gz"),
        (b"not an archive at all".to_vec(), "tar"),
    ] {
        let err = load(bytes, hint).unwrap_err();
        assert!(
            matches!(err, LoadError::Parse(_) | LoadError::Io(..)),
            "{hint}: {err}"
        );
    }
}

#[test]
fn archive_member_names_cannot_climb_out() {
    let dir = tempfile::tempdir().unwrap();
    let z = dir.path().join("a.zip");
    std::fs::write(&z, zip(&[("../../secret.txt", b"inside")])).unwrap();
    // The name is normalized inside the archive; nothing is written anywhere.
    let doc = Registry::with_builtins()
        .load(&Source::Path(z), &no_ocr())
        .unwrap();
    let links: Vec<String> = doc
        .marker_index()
        .iter(MarkerKind::Link, None)
        .filter_map(|m| m.reference.clone())
        .collect();
    assert_eq!(links, ["a.zip!secret.txt"]);
    let member = dir.path().join("a.zip!secret.txt");
    assert_eq!(archive::read_path(&member).unwrap(), b"inside");
}

#[test]
fn deep_dtbook_is_flattened() {
    let deep = 20_000;
    let mut xml = String::from("<dtbook><book><bodymatter>");
    for _ in 0..deep {
        xml.push_str("<level>");
    }
    xml.push_str("<p>bottom</p>");
    for _ in 0..deep {
        xml.push_str("</level>");
    }
    xml.push_str("</bodymatter></book></dtbook>");
    let doc = load(xml.into_bytes(), "xml").unwrap();
    assert!(doc.text().to_string().contains("bottom"));
    assert!(warnings(&doc.meta).iter().any(|w| w == NESTING_WARNING));
}

#[test]
fn broken_powerpoint_fails_clearly_and_missing_slides_are_skipped() {
    let err = load(zip(&[("x.xml", b"<x/>")]), "pptx").unwrap_err();
    assert!(err.to_string().contains("no ppt/presentation.xml"), "{err}");
    let pres = br#"<p:presentation xmlns:p="p" xmlns:r="r"><p:sldIdLst><p:sldId r:id="rId9"/><p:sldId r:id="rId1"/></p:sldIdLst></p:presentation>"#;
    let rels = br#"<Relationships><Relationship Id="rId1" Type="x/slide" Target="slides/gone.xml"/></Relationships>"#;
    let doc = load(
        zip(&[
            ("ppt/presentation.xml", pres),
            ("ppt/_rels/presentation.xml.rels", rels),
        ]),
        "pptx",
    )
    .unwrap();
    assert_eq!(doc.text().to_string(), "");
}

/// A minimal workbook whose one sheet holds `cells` (`r="A1"` and so on).
#[cfg(feature = "spreadsheets")]
fn xlsx(cells: &[(&str, &str)]) -> Vec<u8> {
    let sheet: String = cells
        .iter()
        .map(|(r, v)| {
            format!(
                r#"<row r="{}"><c r="{r}" t="inlineStr"><is><t>{v}</t></is></c></row>"#,
                r.trim_start_matches(char::is_alphabetic)
            )
        })
        .collect();
    let sheet = format!(
        r#"<?xml version="1.0"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:XFD1048576"/><sheetData>{sheet}</sheetData></worksheet>"#
    );
    zip(&[
        (
            "[Content_Types].xml",
            br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            br#"<?xml version="1.0"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", sheet.as_bytes()),
    ])
}

#[cfg(feature = "spreadsheets")]
#[test]
fn a_workbook_claiming_every_cell_is_read_cell_by_cell() {
    let started = std::time::Instant::now();
    let doc = load(
        xlsx(&[("A1", "Name"), ("B2", "Ann"), ("XFD1048576", "far")]),
        "xlsx",
    )
    .unwrap();
    assert!(started.elapsed().as_secs() < 30);
    let text = doc.text().to_string();
    assert!(text.starts_with("Data\n\nName | \n | Ann"), "{text}");
    // The far cell is past the column limit: cut, with a warning.
    assert!(!text.contains("far"));
    assert_eq!(warnings(&doc.meta).len(), 1);
}

#[test]
fn unterminated_csv_quotes_load() {
    let mut data = b"a,\"".to_vec();
    data.extend(std::iter::repeat_n(b'x', 2_000_000));
    let doc = load(data, "csv").unwrap();
    assert_eq!(
        doc.marker_index().iter(MarkerKind::TableCell, None).count(),
        2
    );
}

/// CRC-32 (IEEE), for hand-made PNG chunks.
#[cfg(feature = "ocr")]
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(feature = "ocr")]
#[test]
fn pictures_claiming_huge_sizes_are_refused() {
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = b"IHDR".to_vec();
    ihdr.extend(100_000u32.to_be_bytes());
    ihdr.extend(100_000u32.to_be_bytes());
    ihdr.extend([8, 0, 0, 0, 0]);
    png.extend(13u32.to_be_bytes());
    png.extend(&ihdr);
    png.extend(crc32(&ihdr).to_be_bytes());
    let err = load(png, "png").unwrap_err().to_string();
    assert!(
        err.contains("too large") || err.contains("not a readable"),
        "{err}"
    );
    let err = load(b"\xff\xd8\xff\xe0 broken".to_vec(), "jpg")
        .unwrap_err()
        .to_string();
    assert!(err.contains("JPEG"), "{err}");
}

/// The web loader's response reader, on random content types and bodies
/// (the `web` fuzz target's input, in small doses): it never panics, and a
/// document it returns keeps its markers inside the text, in order.
#[cfg(feature = "url")]
mod web_responses {
    use proptest::prelude::*;
    use textweaver_formats::web::read_response;

    const TYPES: &[&str] = &[
        "",
        "text/html",
        "text/html; charset=windows-1252",
        "text/html; charset=\"no-such-charset\"",
        "application/pdf",
        "application/zip",
        "application/epub+zip",
        "text/csv",
        "text/markdown",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ";;;=charset",
    ];

    const PREFIXES: &[&[u8]] = &[
        b"",
        b"%PDF-1.7\n",
        b"PK\x03\x04",
        b"<html><meta charset=utf-16>",
    ];

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(96))]
        #[test]
        fn never_panic(
            pick in 0..TYPES.len(),
            prefix in 0..PREFIXES.len(),
            body in prop::collection::vec(any::<u8>(), 0..512),
        ) {
            let mut data = PREFIXES[prefix].to_vec();
            data.extend(body);
            for url in ["https://example.org/page", "https://example.org/f.xlsx?x=1#y"] {
                if let Ok(doc) = read_response(url, TYPES[pick], data.clone(), &super::no_ocr()) {
                    let len = doc.len_chars();
                    let mut last = 0;
                    for m in doc.markers() {
                        prop_assert!(m.range.start <= m.range.end);
                        prop_assert!(m.range.end.0 <= len);
                        prop_assert!(m.range.start.0 >= last);
                        last = m.range.start.0;
                    }
                }
            }
        }
    }
}
