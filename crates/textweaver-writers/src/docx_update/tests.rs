use std::io::{Cursor, Read, Write};

use super::*;

const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;
const STYLES: &str = r#"<?xml version="1.0"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:styleId="Normal"/></w:styles>"#;

fn package(body: &str, extra: &[(&str, &str)]) -> Vec<u8> {
    let mut z = ZipWriter::new(Cursor::new(Vec::new()));
    let o = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let doc =
        format!(r#"<?xml version="1.0"?><w:document {NS}><w:body>{body}</w:body></w:document>"#);
    let types = r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>"#;
    let mut all = vec![
        (TYPES, types),
        (DOCUMENT, doc.as_str()),
        ("word/styles.xml", STYLES),
    ];
    all.extend_from_slice(extra);
    for (name, text) in all {
        z.start_file(name, o).unwrap();
        z.write_all(text.as_bytes()).unwrap();
    }
    z.finish().unwrap().into_inner()
}

fn part(pkg: &[u8], name: &str) -> Option<String> {
    let mut z = ZipArchive::new(Cursor::new(pkg)).unwrap();
    let mut f = z.by_name(name).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    Some(s)
}

/// Each paragraph's text (`w:t` only), joined by " | ".
fn text(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    doc.descendants()
        .filter(|n| is_w_named(*n, "p"))
        .map(|p| {
            p.descendants()
                .filter(|n| is_w_named(*n, "t"))
                .filter_map(|n| n.text())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn r(t: &str) -> String {
    format!(r#"<w:r><w:t xml:space="preserve">{t}</w:t></w:r>"#)
}

fn d(t: &str) -> String {
    format!(r#"<w:r><w:delText xml:space="preserve">{t}</w:delText></w:r>"#)
}

/// A paragraph with each kind of change, a deleted paragraph, a move with
/// named ranges, and a formatting change.
fn review_body() -> String {
    let a = r#"w:author="Ada Example" w:date="2026-03-03T09:00:00Z""#;
    format!(
        concat!(
            "<w:p>{one}<w:ins w:id=\"1\" {a}>{ins}</w:ins>{two}<w:del w:id=\"2\" {a}>{del}</w:del>{three}</w:p>",
            "<w:p><w:pPr><w:rPr><w:del w:id=\"3\" {a}/></w:rPr></w:pPr><w:del w:id=\"4\" {a}>{gone}</w:del></w:p>",
            "<w:p>{keep}</w:p>",
            "<w:p><w:moveFromRangeStart w:id=\"5\" w:name=\"move1\" {a}/><w:moveFrom w:id=\"6\" {a}>{moved}</w:moveFrom><w:moveFromRangeEnd w:id=\"5\"/>{rest}</w:p>",
            "<w:p>{then}<w:moveToRangeStart w:id=\"7\" w:name=\"move1\" {a}/><w:moveTo w:id=\"8\" {a}>{moved}</w:moveTo><w:moveToRangeEnd w:id=\"7\"/></w:p>",
            "<w:p><w:r><w:rPr><w:b/><w:rPrChange w:id=\"9\" {a}><w:rPr><w:i/></w:rPr></w:rPrChange></w:rPr><w:t>Bold</w:t></w:r></w:p>",
        ),
        a = a,
        one = r("The cat "),
        ins = r("quietly "),
        two = r("sat "),
        del = d("down "),
        three = r("there."),
        gone = d("A paragraph deleted."),
        keep = r("Kept."),
        moved = r("Moved."),
        rest = r(" Stays."),
        then = r("Then "),
    )
}

#[test]
fn accepting_all_leaves_no_revision_and_other_parts_alone() {
    let pkg = package(&review_body(), &[]);
    let update = DocxUpdate {
        rest: Some(true),
        ..DocxUpdate::default()
    };
    let (out, _) = update_docx(&pkg, &update).unwrap();
    let doc = part(&out, DOCUMENT).unwrap();
    for mark in ["<w:ins", "<w:del", "<w:move", "rPrChange", "delText"] {
        assert!(!doc.contains(mark), "{mark} left in {doc}");
    }
    assert_eq!(
        text(&doc),
        "The cat quietly sat there. | Kept. |  Stays. | Then Moved. | Bold"
    );
    assert!(doc.contains("<w:b/>") && !doc.contains("<w:i/>"));
    assert_eq!(part(&out, "word/styles.xml").unwrap(), STYLES);
    // The untouched part's compressed bytes are copied too.
    let raw = |p: &[u8]| {
        let mut z = ZipArchive::new(Cursor::new(p.to_vec())).unwrap();
        let f = z.by_name("word/styles.xml").unwrap();
        (f.compressed_size(), f.crc32())
    };
    assert_eq!(raw(&out), raw(&pkg));
}

#[test]
fn rejecting_all_gives_the_original_text() {
    let pkg = package(&review_body(), &[]);
    let update = DocxUpdate {
        rest: Some(false),
        ..DocxUpdate::default()
    };
    let (out, _) = update_docx(&pkg, &update).unwrap();
    let doc = part(&out, DOCUMENT).unwrap();
    for mark in ["<w:ins", "<w:del", "<w:move", "rPrChange", "delText"] {
        assert!(!doc.contains(mark), "{mark} left in {doc}");
    }
    assert_eq!(
        text(&doc),
        "The cat sat down there. | A paragraph deleted. | Kept. | Moved. Stays. | Then  | Bold"
    );
    assert!(doc.contains("<w:rPr><w:i/></w:rPr>"), "{doc}");
}

#[test]
fn one_half_of_a_move_decides_the_other_and_others_stay() {
    let pkg = package(&review_body(), &[]);
    let update = DocxUpdate {
        decisions: HashMap::from([("8".to_owned(), true), ("2".to_owned(), false)]),
        ..DocxUpdate::default()
    };
    let (out, _) = update_docx(&pkg, &update).unwrap();
    let doc = part(&out, DOCUMENT).unwrap();
    assert!(
        !doc.contains("moveFrom") && !doc.contains("moveTo"),
        "{doc}"
    );
    assert!(doc.contains("<w:ins w:id=\"1\""), "undecided stays");
    assert!(text(&doc).contains("sat down there."), "{doc}");
    assert!(text(&doc).contains("Then Moved."));
    assert!(!text(&doc).contains("Moved. Stays."));
}

#[test]
fn nothing_decided_changes_nothing() {
    let pkg = package(&review_body(), &[]);
    let update = DocxUpdate {
        decisions: HashMap::from([("99".to_owned(), true)]),
        ..DocxUpdate::default()
    };
    let (out, _) = update_docx(&pkg, &update).unwrap();
    assert_eq!(part(&out, DOCUMENT), part(&pkg, DOCUMENT));
}

#[test]
fn a_deleted_paragraph_mark_follows_its_deleted_text() {
    let pkg = package(&review_body(), &[]);
    let update = DocxUpdate {
        decisions: HashMap::from([("4".to_owned(), true)]),
        ..DocxUpdate::default()
    };
    let (out, _) = update_docx(&pkg, &update).unwrap();
    let doc = part(&out, DOCUMENT).unwrap();
    assert!(!doc.contains("w:id=\"3\""), "{doc}");
    assert!(text(&doc).contains("there. | Kept."), "{doc}");
}

#[test]
fn the_backup_name_says_what_it_is() {
    let dir = std::env::temp_dir().join("tw-docx-update-backup-name");
    let p = dir.join("report.docx");
    assert_eq!(backup_path(&p), dir.join("report-original.docx"));
}

const COMMENTS_XML: &str = concat!(
    r#"<?xml version="1.0"?><w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml">"#,
    r#"<w:comment w:id="0" w:author="Bo Example"><w:p w14:paraId="2B000001"><w:r><w:t>check this</w:t></w:r></w:p></w:comment>"#,
    r#"<w:comment w:id="1" w:author="Ada Example"><w:p w14:paraId="2B000002"><w:r><w:t>Done.</w:t></w:r></w:p></w:comment>"#,
    r#"<w:comment w:id="2" w:author="Ada Example"><w:p w14:paraId="2B000003"><w:r><w:t>Who?</w:t></w:r></w:p></w:comment>"#,
    "</w:comments>"
);
const EXTENDED_XML: &str = concat!(
    r#"<?xml version="1.0"?><w15:commentsEx xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml">"#,
    r#"<w15:commentEx w15:paraId="2B000001" w15:done="1"/>"#,
    r#"<w15:commentEx w15:paraId="2B000002" w15:paraIdParent="2B000001" w15:done="0"/>"#,
    r#"<w15:commentEx w15:paraId="2B000003" w15:done="0"/>"#,
    "</w15:commentsEx>"
);

fn commented_body() -> String {
    format!(
        concat!(
            "<w:p><w:commentRangeStart w:id=\"0\"/>{a}<w:commentRangeEnd w:id=\"0\"/>",
            "<w:r><w:commentReference w:id=\"0\"/></w:r></w:p>",
            "<w:p>{b}<w:r><w:commentReference w:id=\"2\"/></w:r></w:p>",
            "<w:p>{c}{d}</w:p>"
        ),
        a = r("Call the family."),
        b = r("Repeat tomorrow."),
        c = r("See the "),
        d = r("chart today."),
    )
}

fn thread(id: &str, resolved: bool, replies: usize) -> CommentThread {
    CommentThread {
        id: id.into(),
        resolved,
        replies: (0..replies)
            .map(|i| ThreadReply {
                author: "Ada Example".into(),
                date: "2026-10-09T10:00:00Z".into(),
                text: format!("Reply {i}."),
            })
            .collect(),
        ..CommentThread::default()
    }
}

fn load(pkg: &[u8]) -> textweaver_text::Document {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.docx");
    std::fs::write(&path, pkg).unwrap();
    textweaver_formats::load_path(&path).unwrap()
}

#[test]
fn replies_resolved_marks_and_new_comments_are_written() {
    let pkg = package(
        &commented_body(),
        &[(COMMENTS, COMMENTS_XML), (EXTENDED, EXTENDED_XML)],
    );
    let new = CommentThread {
        id: "7".into(),
        author: String::new(),
        date: "2026-10-09T10:00:00Z".into(),
        text: "Which chart?".into(),
        anchor: "the chart".into(),
        ..CommentThread::default()
    };
    let update = DocxUpdate {
        threads: vec![thread("0", false, 2), thread("2", true, 0), new],
        ..DocxUpdate::default()
    };
    let (out, report) = update_docx(&pkg, &update).unwrap();
    assert_eq!(
        report.new_comment_ids,
        vec![("7".to_owned(), "4".to_owned())]
    );
    assert_eq!(report.unplaced, 0);
    let doc = load(&out);
    let cs = textweaver_formats::comments(&doc.meta);
    assert_eq!(cs.len(), 3, "{cs:?}");
    let first = cs.iter().find(|c| c.id == "0").unwrap();
    assert!(!first.resolved);
    assert_eq!(first.replies.len(), 2);
    assert_eq!(first.replies[1].text, "Reply 1.");
    assert!(cs.iter().find(|c| c.id == "2").unwrap().resolved);
    let added = cs.iter().find(|c| c.id == "4").unwrap();
    assert_eq!(added.author, "textweaver", "an empty author is textweaver");
    assert_eq!(doc.slice(added.range).to_string(), "See the chart today.");
    assert_eq!(part(&out, "word/styles.xml").unwrap(), STYLES);
}

#[test]
fn a_deleted_thread_goes_with_its_replies_and_marks() {
    let pkg = package(
        &commented_body(),
        &[(COMMENTS, COMMENTS_XML), (EXTENDED, EXTENDED_XML)],
    );
    let update = DocxUpdate {
        deleted_comments: vec!["0".into()],
        ..DocxUpdate::default()
    };
    let (out, _) = update_docx(&pkg, &update).unwrap();
    let comments = part(&out, COMMENTS).unwrap();
    assert!(!comments.contains("w:id=\"0\"") && !comments.contains("w:id=\"1\""));
    assert!(comments.contains("w:id=\"2\""));
    let doc = part(&out, DOCUMENT).unwrap();
    assert!(!doc.contains("w:id=\"0\""), "{doc}");
    assert!(doc.contains("Call the family."));
    assert!(!part(&out, EXTENDED).unwrap().contains("2B000002"));
}

#[test]
fn comments_are_added_to_a_package_without_comment_parts() {
    let rels = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="x" Target="styles.xml"/></Relationships>"#;
    let pkg = package(&commented_body(), &[(DOC_RELS, rels)]);
    let new = CommentThread {
        id: "a".into(),
        author: "Ada Example".into(),
        text: "First.".into(),
        resolved: true,
        anchor: "Repeat tomorrow.".into(),
        ..CommentThread::default()
    };
    let update = DocxUpdate {
        threads: vec![new],
        ..DocxUpdate::default()
    };
    let (out, _) = update_docx(&pkg, &update).unwrap();
    let types = part(&out, TYPES).unwrap();
    assert!(types.contains("/word/comments.xml") && types.contains("/word/commentsExtended.xml"));
    let rels = part(&out, DOC_RELS).unwrap();
    assert!(rels.contains("Id=\"rId2\"") && rels.contains("Target=\"comments.xml\""));
    let cs = textweaver_formats::comments(&load(&out).meta);
    assert_eq!(cs.len(), 1);
    assert!(cs[0].resolved);
    assert_eq!(cs[0].text, "First.");
}
