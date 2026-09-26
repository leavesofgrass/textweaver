//! Star's authoring tests, ported (docs/star-parity.md Part 3 §4.6): 31
//! from `tests/test_authoring.py` (`a01`..`a31`) and 25 from
//! `tests/test_authoring_depth.py` (`d01`..`d25`). Tests of Qt widgets with
//! no counterpart outside a GUI are listed here with the reason, and where
//! the same guarantee can be checked without the widget it is.
//!
//! | Star test | Here |
//! |---|---|
//! | a01 new document opens blank in edit mode | `a01` |
//! | a02 new document, cancel keeps current edits | `a02` |
//! | a03 new document, discard starts blank | `a03` |
//! | a04 stale background load does not clobber the new document | `a04` |
//! | a05 formatting toolbar exists and follows edit mode | `a05` (actions exist; formatting only works in edit mode); the edit-layer-only binding check is in `textweaver-keymap` (`formatting_is_bound_only_in_edit_mode`) |
//! | a06 bold wraps the selection | `a06` |
//! | a07 wrap with no selection inserts a selected placeholder | `a07` |
//! | a08 heading prefixes the current line | `a08` |
//! | a09 numbered list renumbers selected lines | `a09` |
//! | a10 bullet list prefixes each selected line | `a10` |
//! | a11 link wraps the selection as link text | `a11` |
//! | a12 horizontal rule | `a12`, fixed: a blank line before `---` so it is not a setext heading |
//! | a13 Ctrl+B owned by bold, not bookmark | `textweaver-keymap` `authoring_chords_have_one_owner`; the bold behavior is `a13` |
//! | a14 Ctrl+I, Ctrl+K, Ctrl+M have one owner | `textweaver-keymap` `authoring_chords_have_one_owner` |
//! | a15 underline wraps in `<u>` | `a15`; the Ctrl+U owner check is in `textweaver-keymap` |
//! | a16 menu bar padding and menu order | not applicable (Qt stylesheet); help category order is fixed by `textweaver-keymap` `every_category_has_actions_and_actions_are_grouped` |
//! | a17 formatting is a no-op outside edit mode | `a17` |
//! | a18 save keeps the user in edit mode | `a18` |
//! | a19 formatting refreshes the preview immediately | not applicable (textweaver has no preview pane) |
//! | a20 preview refresh is a no-op when hidden | not applicable (no preview pane) |
//! | a21 finish editing, cancel keeps editing | `a21` |
//! | a22 finish editing, discard exits without saving | `a22` |
//! | a23 finish editing, save persists then exits | `a23` |
//! | a24 finish editing when clean just exits | `a24` |
//! | a25 confirm leave, cancel aborts | `a25` |
//! | a26 confirm leave, discard tears down | `a26` |
//! | a27 loading a document mid-edit tears edit mode down | `a27` |
//! | a28 converted document: Save As adopts the path, no re-prompt | `a28` |
//! | a29 word-map rebuild only after a real change | `a29` |
//! | a30 loading a document clears the stale-maps flag | `a30` |
//! | a31 word-map worker bails when the window is closing | not applicable (Qt worker thread; the app owns rebuilds) |
//! | d01 replace one replaces the current match | `d01`, fixed: the match at the caret, not the one after it |
//! | d02 replace all, one undo | `d02` |
//! | d03 replace is a no-op outside edit mode | `d03` |
//! | d04 new document opens in edit mode with preview | `d04` (edit mode; no preview, and no setting is changed) |
//! | d05 reset editor formatting clears a leaked heading style | not applicable (Qt rich-text formats; the editor holds plain text) |
//! | d06 equalize the edit split | not applicable (Qt splitter) |
//! | d07 table skeleton shape | `d07` |
//! | d08 insert table writes Markdown | `d08` |
//! | d09 add table row matches the columns | `d09` |
//! | d10 insert image uses a relative path when saved | `d10` |
//! | d11 live Markdown reflects unsaved edits | `d11` |
//! | d12 export Markdown writes the editor buffer | `d12` |
//! | d13 snapshot key stable for a path | `d13` (key derivation is `DocKey` in `textweaver-store`, tested there; here the snapshot file follows the key) |
//! | d14 write and scan a snapshot | `d14` |
//! | d15 scan skips an already-saved snapshot | `d15` |
//! | d16 autosave tick writes only when dirty | `d16` |
//! | d17 autosave opt-out disables snapshots | `d17` |
//! | d18 startup recovery offer loads the snapshot | `d18` |
//! | d19 notes pane hidden at launch | not applicable (notes UI is wave 2) |
//! | d20 notes pane auto-shows for a document with notes | not applicable (wave 2) |
//! | d21 notes pane manual toggle is transient | not applicable (wave 2) |
//! | d22 contents pane visibility matches headings at launch | not applicable (Qt dock) |
//! | d23 contents pane auto-shows for a document with headings | not applicable (Qt dock) |
//! | d24 contents pane manual toggle is transient | not applicable (Qt dock) |
//! | d25 startup recovery declined drops the snapshot | `d25` |

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use textweaver_core::CharPos;
use textweaver_editor::autosave::{self, AutosavePolicy, RecoverySnapshot};
use textweaver_editor::markdown::{self, MarkdownOp};
use textweaver_editor::session::resolve_recovery;
use textweaver_editor::{
    Choice, DocInfo, EditSession, FindOptions, LeaveOutcome, SaveOutcome, Selection, SessionError,
};

/// Star's fixture: document `/tmp/d.md` with Markdown `"# D\n\nhi"`.
fn fixture() -> EditSession {
    EditSession::new(
        DocInfo {
            key: "d.md-0".into(),
            path: Some(PathBuf::from("/tmp/d.md")),
            loader_id: "markdown".into(),
            title: "d".into(),
        },
        "# D\n\nhi",
    )
}

fn doc_at(path: &Path, loader: &str) -> DocInfo {
    DocInfo {
        key: format!(
            "{}-k",
            path.file_name().unwrap_or_default().to_string_lossy()
        ),
        path: Some(path.to_owned()),
        loader_id: loader.into(),
        title: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    }
}

/// An editing session over `text` with the whole editor replaced.
fn editing(text: &str) -> EditSession {
    let mut s = fixture();
    s.enter_edit();
    let ed = s.editor_mut().unwrap();
    ed.set_text(text);
    s
}

/// Types into the session so it becomes dirty.
fn make_dirty(s: &mut EditSession, text: &str) {
    let ed = s.editor_mut().unwrap();
    ed.set_selection(Selection::new(0, ed.text().len_chars()));
    ed.insert_text(text).unwrap();
    assert!(s.is_dirty());
}

fn text(s: &EditSession) -> String {
    s.live_text()
}

fn format(text: &str, sel: Selection, op: MarkdownOp) -> (String, Selection) {
    let mut s = editing(text);
    s.select(sel).unwrap();
    s.format(op).unwrap();
    let ed = s.editor().unwrap();
    (ed.text().to_string(), ed.selection())
}

// ---- tests/test_authoring.py ----

#[test]
fn a01_new_document_opens_blank_in_edit_mode() {
    let mut s = fixture();
    let out = s.new_document("untitled-1", None, None).unwrap();
    assert_eq!(out, LeaveOutcome::Left { rebuild: false });
    assert!(s.is_editing());
    assert_eq!(s.doc().path, None);
    assert_eq!(s.doc().loader_id, "markdown");
    assert_eq!(text(&s), "");
}

#[test]
fn a02_new_document_cancel_keeps_current_edits() {
    let mut s = fixture();
    s.enter_edit();
    make_dirty(&mut s, "precious work");
    assert_eq!(
        s.new_document("untitled-1", None, None).unwrap(),
        LeaveOutcome::NeedsChoice
    );
    assert_eq!(
        s.new_document("untitled-1", Some(Choice::Cancel), None)
            .unwrap(),
        LeaveOutcome::Stayed
    );
    assert_eq!(text(&s), "precious work");
    assert!(s.is_editing());
}

#[test]
fn a03_new_document_discard_starts_blank() {
    let mut s = fixture();
    s.enter_edit();
    make_dirty(&mut s, "throwaway draft");
    s.new_document("untitled-1", Some(Choice::Discard), None)
        .unwrap();
    assert!(s.is_editing());
    assert_eq!(s.doc().path, None);
    assert_eq!(text(&s), "");
}

#[test]
fn a04_stale_background_load_does_not_clobber_new_document() {
    let mut s = fixture();
    let welcome = s.begin_load();
    s.new_document("untitled-1", None, None).unwrap();
    let applied = s.finish_load(
        welcome,
        doc_at(Path::new("welcome.md"), "markdown"),
        "welcome",
    );
    assert!(!applied);
    assert_eq!(s.doc().path, None);
    assert_eq!(text(&s), "");
    let current = s.begin_load();
    assert!(s.finish_load(current, doc_at(Path::new("real.md"), "markdown"), "real"));
    assert_eq!(s.doc().path.as_deref(), Some(Path::new("real.md")));
}

#[test]
fn a05_formatting_commands_exist_and_follow_edit_mode() {
    let ops = [
        MarkdownOp::Bold,
        MarkdownOp::Italic,
        MarkdownOp::Heading(1),
        MarkdownOp::BulletList,
        MarkdownOp::NumberedList,
        MarkdownOp::Quote,
        MarkdownOp::InlineCode,
        MarkdownOp::Link,
        MarkdownOp::HorizontalRule,
    ];
    let mut s = fixture();
    for op in ops {
        assert!(matches!(s.format(op), Err(SessionError::NotEditing { .. })));
    }
    s.enter_edit();
    for op in ops {
        assert!(s.format(op).is_ok(), "{op:?}");
    }
    s.finish_editing(Some(Choice::Discard), None).unwrap();
    assert!(!s.is_editing());
    assert!(matches!(
        s.format(MarkdownOp::Bold),
        Err(SessionError::NotEditing { .. })
    ));
}

#[test]
fn a06_bold_wraps_the_selection() {
    let (t, _) = format("make me bold", Selection::new(8, 12), MarkdownOp::Bold);
    assert_eq!(t, "make me **bold**");
}

#[test]
fn a07_wrap_with_no_selection_inserts_a_placeholder() {
    let (t, sel) = format("", Selection::caret(CharPos(0)), MarkdownOp::Italic);
    assert_eq!(t, "*italic text*");
    let r = sel.range();
    assert_eq!(&t[r.start.0..r.end.0], "italic text");
}

#[test]
fn a08_heading_prefixes_the_current_line() {
    let (t, _) = format(
        "Title\nbody",
        Selection::caret(CharPos(0)),
        MarkdownOp::Heading(1),
    );
    assert_eq!(t, "# Title\nbody");
}

#[test]
fn a09_numbered_list_renumbers_selected_lines() {
    let (t, _) = format(
        "one\ntwo\nthree",
        Selection::new(0, 13),
        MarkdownOp::NumberedList,
    );
    assert_eq!(t, "1. one\n2. two\n3. three");
}

#[test]
fn a10_bullet_list_prefixes_each_selected_line() {
    let (t, _) = format("a\nb", Selection::new(0, 3), MarkdownOp::BulletList);
    assert_eq!(t, "- a\n- b");
}

#[test]
fn a11_link_wraps_selection_as_link_text() {
    let (t, _) = format("click here", Selection::new(0, 10), MarkdownOp::Link);
    assert_eq!(t, "[click here](https://)");
}

/// Star inserted `"\n---\n"`, giving `"above\n---\n"`, which renders as a
/// setext heading (Part 3 §7 item 32). Fixed: a blank line comes first.
#[test]
fn a12_horizontal_rule_inserts_a_rule() {
    let (t, sel) = format(
        "above",
        Selection::caret(CharPos(5)),
        MarkdownOp::HorizontalRule,
    );
    assert_eq!(t, "above\n\n---\n");
    assert_eq!(sel, Selection::caret(CharPos(11)));
}

#[test]
fn a13_bold_command_on_a_word() {
    let (t, _) = format("word", Selection::new(0, 4), MarkdownOp::Bold);
    assert_eq!(t, "**word**");
}

#[test]
fn a15_underline_wraps_selection_in_html() {
    let (t, _) = format("word", Selection::new(0, 4), MarkdownOp::Underline);
    assert_eq!(t, "<u>word</u>");
}

#[test]
fn a17_formatting_is_a_no_op_outside_edit_mode() {
    let mut s = fixture();
    let before = text(&s);
    let err = s.format(MarkdownOp::Bold).unwrap_err();
    assert!(err.to_string().contains("format"), "{err}");
    s.format(MarkdownOp::BulletList).unwrap_err();
    assert_eq!(text(&s), before);
}

#[test]
fn a18_save_keeps_the_user_in_edit_mode() {
    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("note.md");
    std::fs::write(&note, "# Old\n").unwrap();
    let mut s = EditSession::new(doc_at(&note, "markdown"), "# Old\n");
    s.enter_edit();
    make_dirty(&mut s, "# New heading\n\nBody.\n");
    let out = s.save(None).unwrap();
    assert_eq!(
        out,
        SaveOutcome::Saved {
            path: note.clone(),
            adopted: false
        }
    );
    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        "# New heading\n\nBody.\n"
    );
    assert!(s.is_editing());
    assert!(!s.is_dirty());
    assert_eq!(text(&s), "# New heading\n\nBody.\n");
}

#[test]
fn a21_finish_editing_cancel_keeps_editing() {
    let mut s = fixture();
    s.enter_edit();
    make_dirty(&mut s, "kept");
    assert_eq!(
        s.finish_editing(None, None).unwrap(),
        LeaveOutcome::NeedsChoice
    );
    assert_eq!(
        s.finish_editing(Some(Choice::Cancel), None).unwrap(),
        LeaveOutcome::Stayed
    );
    assert!(s.is_editing());
    assert_eq!(text(&s), "kept");
}

#[test]
fn a22_finish_editing_discard_exits_without_saving() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.md");
    std::fs::write(&file, "original\n").unwrap();
    let mut s = EditSession::new(doc_at(&file, "markdown"), "original\n");
    s.enter_edit();
    make_dirty(&mut s, "throwaway");
    assert_eq!(
        s.finish_editing(Some(Choice::Discard), None).unwrap(),
        LeaveOutcome::Left { rebuild: false }
    );
    assert!(!s.is_editing());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "original\n");
    assert_eq!(text(&s), "original\n");
}

#[test]
fn a23_finish_editing_save_persists_then_exits() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.md");
    std::fs::write(&file, "original\n").unwrap();
    let mut s = EditSession::new(doc_at(&file, "markdown"), "original\n");
    s.enter_edit();
    make_dirty(&mut s, "kept edit\n");
    assert_eq!(
        s.finish_editing(Some(Choice::Save), None).unwrap(),
        LeaveOutcome::Left { rebuild: true }
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "kept edit\n");
    assert!(!s.is_editing());
}

#[test]
fn a24_finish_editing_when_clean_just_exits() {
    let mut s = fixture();
    s.enter_edit();
    assert_eq!(
        s.finish_editing(None, None).unwrap(),
        LeaveOutcome::Left { rebuild: false }
    );
    assert!(!s.is_editing());
}

#[test]
fn a25_confirm_leave_edit_cancel_aborts() {
    let mut s = fixture();
    s.enter_edit();
    make_dirty(&mut s, "x");
    assert_eq!(
        s.confirm_leave(Some(Choice::Cancel), None).unwrap(),
        LeaveOutcome::Stayed
    );
    assert!(s.is_editing());
}

#[test]
fn a26_confirm_leave_edit_discard_tears_down() {
    let mut s = fixture();
    s.enter_edit();
    make_dirty(&mut s, "x");
    assert!(matches!(
        s.confirm_leave(Some(Choice::Discard), None).unwrap(),
        LeaveOutcome::Left { .. }
    ));
    assert!(!s.is_editing());
}

#[test]
fn a27_loading_a_document_mid_edit_tears_edit_mode_down() {
    let dir = tempfile::tempdir().unwrap();
    let b = dir.path().join("b.md");
    std::fs::write(&b, "bee").unwrap();
    let mut s = fixture();
    s.enter_edit();
    make_dirty(&mut s, "unsaved");
    s.load(doc_at(&b, "markdown"), "bee");
    assert!(!s.is_editing());
    assert_eq!(std::fs::read_to_string(&b).unwrap(), "bee");
    assert_eq!(text(&s), "bee");
}

#[test]
fn a28_converted_doc_save_as_adopts_path_no_reprompt() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("report.pdf");
    std::fs::write(&pdf, b"%PDF").unwrap();
    let mut s = EditSession::new(doc_at(&pdf, "pdf"), "Report text\n");
    s.enter_edit();
    make_dirty(&mut s, "Report, edited\n");
    let suggested = match s.save(None).unwrap() {
        SaveOutcome::NeedsPath { suggested } => suggested,
        other => panic!("expected a Save As prompt, got {other:?}"),
    };
    assert_eq!(suggested, dir.path().join("report.md"));
    let out = s.save(Some(&suggested)).unwrap();
    assert_eq!(
        out,
        SaveOutcome::Saved {
            path: suggested.clone(),
            adopted: true
        }
    );
    assert_eq!(
        std::fs::read_to_string(&suggested).unwrap(),
        "Report, edited\n"
    );
    assert_eq!(std::fs::read(&pdf).unwrap(), b"%PDF", "source untouched");
    assert_eq!(s.doc().path.as_deref(), Some(suggested.as_path()));
    assert!(s.is_editing());
    // The second save writes in place without asking.
    s.editor_mut().unwrap().type_text("more").unwrap();
    assert_eq!(
        s.save(None).unwrap(),
        SaveOutcome::Saved {
            path: suggested.clone(),
            adopted: false
        }
    );
}

#[test]
fn a29_word_map_rebuild_only_after_a_real_change() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("m.md");
    std::fs::write(&file, "text").unwrap();
    let mut s = EditSession::new(doc_at(&file, "markdown"), "text");
    s.enter_edit();
    assert_eq!(
        s.finish_editing(None, None).unwrap(),
        LeaveOutcome::Left { rebuild: false }
    );
    assert!(!s.maps_stale());
    s.enter_edit();
    make_dirty(&mut s, "changed");
    s.save(None).unwrap();
    assert!(s.maps_stale());
    assert_eq!(
        s.finish_editing(None, None).unwrap(),
        LeaveOutcome::Left { rebuild: true }
    );
    assert!(!s.maps_stale());
}

#[test]
fn a30_loading_a_document_clears_the_stale_maps_flag() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("m.md");
    let mut s = EditSession::new(doc_at(&file, "markdown"), "");
    s.enter_edit();
    make_dirty(&mut s, "x");
    s.save(None).unwrap();
    assert!(s.maps_stale());
    s.load(doc_at(Path::new("other.md"), "markdown"), "y");
    assert!(!s.maps_stale());
}

// ---- tests/test_authoring_depth.py ----

/// Star replaced the second `alpha` because re-running a search skipped the
/// match under the caret (Part 3 §7 item 35). Star's assertions (two
/// `alpha` left, `X` present) hold; textweaver replaces the match at the
/// caret, the first.
#[test]
fn d01_replace_one_replaces_current_match() {
    let mut s = editing("alpha beta alpha gamma alpha");
    assert!(s.replace_one("alpha", "X", FindOptions::default()).unwrap());
    let t = text(&s);
    assert_eq!(t.matches("alpha").count(), 2);
    assert!(t.contains('X'));
    assert_eq!(t, "X beta alpha gamma alpha");
}

#[test]
fn d02_replace_all_replaces_every_match_one_undo() {
    let mut s = editing("cat cat cat dog cat");
    assert_eq!(
        s.replace_all("cat", "fish", FindOptions::default())
            .unwrap(),
        4
    );
    let t = text(&s);
    assert!(!t.contains("cat"));
    assert_eq!(t.matches("fish").count(), 4);
    assert!(s.undo().unwrap());
    assert_eq!(text(&s).matches("cat").count(), 4);
}

#[test]
fn d03_replace_is_noop_outside_edit_mode() {
    let mut s = fixture();
    let before = text(&s);
    assert!(matches!(
        s.replace_all("hi", "yo", FindOptions::default()),
        Err(SessionError::NotEditing { .. })
    ));
    assert_eq!(text(&s), before);
}

/// Star also forced its preview setting on (Part 3 §7 item 34); a session
/// has no settings to change.
#[test]
fn d04_new_document_opens_in_edit_mode() {
    let mut s = fixture();
    s.new_document("untitled-2", None, None).unwrap();
    assert!(s.is_editing());
    assert!(!s.is_dirty());
}

#[test]
fn d07_table_skeleton_shape() {
    let sk = markdown::table_skeleton(2, 3);
    let lines: Vec<&str> = sk.lines().collect();
    assert_eq!(lines[0].matches('|').count(), 4);
    assert!(
        lines[1]
            .chars()
            .filter(|c| !matches!(c, '|' | ' '))
            .all(|c| c == '-')
    );
    assert_eq!(lines.len(), 4);
    assert!(sk.ends_with('\n'));
    assert_eq!(
        markdown::table_skeleton(0, 0),
        "| Column 1 |\n| --- |\n|   |\n"
    );
}

#[test]
fn d08_insert_table_writes_markdown() {
    let (t, sel) = format(
        "",
        Selection::caret(CharPos(0)),
        MarkdownOp::InsertTable { rows: 2, cols: 2 },
    );
    assert!(t.contains("| Column 1 | Column 2 |"));
    assert!(t.contains("| --- | --- |"));
    assert_eq!(
        t,
        "\n| Column 1 | Column 2 |\n| --- | --- |\n|   |   |\n|   |   |\n\n"
    );
    // Fixed (Part 3 §7 item 33): the first header cell is selected.
    let r = sel.range();
    assert_eq!(&t[r.start.0..r.end.0], "Column 1");
    // Mid-line, a leading newline.
    let (t, _) = format(
        "ab",
        Selection::caret(CharPos(1)),
        MarkdownOp::InsertTable { rows: 1, cols: 1 },
    );
    assert_eq!(t, "a\n\n| Column 1 |\n| --- |\n|   |\n\nb");
}

#[test]
fn d09_add_table_row_matches_columns() {
    let src = "| a | b | c |\n| --- | --- | --- |\n| 1 | 2 | 3 |";
    let (t, _) = format(
        src,
        Selection::caret(CharPos(src.chars().count())),
        MarkdownOp::AddTableRow,
    );
    let last = t.lines().last().unwrap();
    assert_eq!(last.matches('|').count(), 4);
    assert_eq!(last, "|   |   |   |");
    // Outside a table, Star's message.
    let mut s = editing("plain");
    let err = s.format(MarkdownOp::AddTableRow).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Put the cursor inside a table row to add a row"
    );
}

#[test]
fn d10_insert_image_uses_relative_path_when_saved() {
    let dir = tempfile::tempdir().unwrap();
    let note = dir.path().join("note.md");
    let pic = dir.path().join("pic.png");
    std::fs::write(&pic, b"png").unwrap();
    let mut s = EditSession::new(doc_at(&note, "markdown"), "");
    s.enter_edit();
    s.insert_image(&pic).unwrap();
    assert!(text(&s).contains("![pic](pic.png)"));
    // Fixed (item 33): the alt text is selected for describing the image.
    let r = s.editor().unwrap().selection().range();
    assert_eq!(&text(&s)[r.start.0..r.end.0], "pic");
}

#[test]
fn d11_live_markdown_reflects_unsaved_edits() {
    let mut s = fixture();
    s.enter_edit();
    make_dirty(&mut s, "# Draft heading\n\nlive body");
    assert_eq!(s.live_text(), "# Draft heading\n\nlive body");
    assert!(s.live_text().contains("live body"));
    assert_eq!(s.document_text(), "# D\n\nhi");
}

#[test]
fn d12_export_markdown_writes_editor_buffer() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("export.md");
    let mut s = fixture();
    s.enter_edit();
    make_dirty(&mut s, "edited but unsaved");
    s.export(&out).unwrap();
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "edited but unsaved");
    assert!(s.is_dirty(), "export does not save the document");
}

fn autosaving(dir: &Path, key: &str, path: Option<PathBuf>) -> EditSession {
    let doc = DocInfo {
        key: key.into(),
        path,
        loader_id: "markdown".into(),
        title: "Untitled".into(),
    };
    EditSession::new(doc, "").with_autosave(AutosavePolicy::default(), dir)
}

#[test]
fn d13_snapshot_file_follows_the_document_key() {
    let dir = tempfile::tempdir().unwrap();
    let a = autosaving(dir.path(), "a.md-1", Some("/tmp/a.md".into()));
    let a2 = autosaving(dir.path(), "a.md-1", Some("/tmp/a.md".into()));
    assert_eq!(a.snapshot_path(), a2.snapshot_path());
    let u1 = autosaving(dir.path(), "untitled-1", None);
    let u2 = autosaving(dir.path(), "untitled-2", None);
    assert_ne!(u1.snapshot_path(), u2.snapshot_path());
}

#[test]
fn d14_write_and_scan_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let snap = RecoverySnapshot {
        doc_key: "untitled-abc".into(),
        path: None,
        text: "unsaved text".into(),
        ts: 1,
        title: Some("Untitled".into()),
    };
    let file = autosave::write_snapshot(dir.path(), &snap).unwrap();
    assert!(file.ends_with("untitled-abc.json"));
    let found = autosave::scan_snapshots(dir.path());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.text, "unsaved text");
    // Only the snapshot is left: no temp files.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn d15_scan_skips_already_saved() {
    let dir = tempfile::tempdir().unwrap();
    let saved = dir.path().join("saved.md");
    std::fs::write(&saved, "same text").unwrap();
    let rec = dir.path().join("recovery");
    let snap = RecoverySnapshot {
        doc_key: "saved.md-1".into(),
        path: Some(saved),
        text: "same text".into(),
        ts: 1,
        title: None,
    };
    let file = autosave::write_snapshot(&rec, &snap).unwrap();
    assert!(autosave::scan_snapshots(&rec).is_empty());
    assert!(!file.exists());
}

#[test]
fn d16_autosave_tick_writes_only_when_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = autosaving(dir.path(), "untitled-1", None);
    s.enter_edit();
    s.editor_mut().unwrap().type_text("typing…").unwrap();
    let t0 = Instant::now();
    assert!(s.autosave_tick(t0).unwrap());
    let file = s.snapshot_path().unwrap();
    let snap: RecoverySnapshot =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(snap.text, "typing…");
    std::fs::remove_file(&file).unwrap();
    s.editor_mut().unwrap().mark_saved();
    assert!(!s.autosave_tick(t0 + Duration::from_secs(60)).unwrap());
    assert!(!file.exists());
    // Dirty again, but inside the interval: no snapshot yet.
    s.editor_mut().unwrap().type_text("x").unwrap();
    assert!(!s.autosave_tick(t0 + Duration::from_secs(5)).unwrap());
    assert!(s.autosave_tick(t0 + Duration::from_secs(20)).unwrap());
}

#[test]
fn d17_autosave_opt_out_setting_disables_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let doc = DocInfo::untitled("untitled-1");
    let mut s = EditSession::new(doc, "").with_autosave(AutosavePolicy::new(false, 20), dir.path());
    s.enter_edit();
    s.editor_mut().unwrap().type_text("typing").unwrap();
    assert!(!s.autosave_tick(Instant::now()).unwrap());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn d18_startup_recovery_offer_loads_the_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let snap = RecoverySnapshot {
        doc_key: "untitled-crashed".into(),
        path: None,
        text: "recovered body".into(),
        ts: 1,
        title: None,
    };
    autosave::write_snapshot(dir.path(), &snap).unwrap();
    let offers = autosave::scan_snapshots(dir.path());
    assert_eq!(offers.len(), 1);
    let (file, snap) = &offers[0];
    let s = resolve_recovery(file, snap, true).unwrap().unwrap();
    assert!(s.is_editing());
    assert!(s.is_dirty());
    assert_eq!(s.live_text(), "recovered body");
    assert_eq!(s.doc().title, "Recovered document");
    assert!(!file.exists());
}

#[test]
fn d25_startup_recovery_declined_drops_the_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let snap = RecoverySnapshot {
        doc_key: "untitled-crashed".into(),
        path: None,
        text: "recovered body".into(),
        ts: 1,
        title: None,
    };
    let file = autosave::write_snapshot(dir.path(), &snap).unwrap();
    assert!(resolve_recovery(&file, &snap, false).unwrap().is_none());
    assert!(!file.exists());
}

// ---- Behavior around the ported tests ----

/// Leaving edit mode cleanly deletes the snapshot (Star's
/// `_autosave_stop(clear=True)`); a successful save does too.
#[test]
fn snapshots_are_cleared_on_save_and_on_leaving() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("n.md");
    let doc = DocInfo {
        key: "n.md-1".into(),
        path: Some(file.clone()),
        loader_id: "markdown".into(),
        title: "n".into(),
    };
    let rec = dir.path().join("recovery");
    let mut s = EditSession::new(doc, "").with_autosave(AutosavePolicy::default(), &rec);
    s.enter_edit();
    s.editor_mut().unwrap().type_text("draft").unwrap();
    assert!(s.autosave_tick(Instant::now()).unwrap());
    let snap = s.snapshot_path().unwrap();
    assert!(snap.exists());
    s.save(None).unwrap();
    assert!(!snap.exists());
    s.editor_mut().unwrap().type_text("more").unwrap();
    s.autosave_tick(Instant::now()).unwrap();
    assert!(snap.exists());
    assert!(s.needs_save_prompt());
    s.finish_editing(Some(Choice::Discard), None).unwrap();
    assert!(!snap.exists());
}

/// A new document asks for a path on its first save; a converted source is
/// never overwritten even when the loader was the text loader.
#[test]
fn new_documents_and_converted_sources_need_a_path() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = fixture();
    s.new_document("untitled-1", None, None).unwrap();
    s.editor_mut().unwrap().type_text("hello").unwrap();
    assert_eq!(
        s.finish_editing(Some(Choice::Save), None).unwrap(),
        LeaveOutcome::NeedsPath {
            suggested: PathBuf::from("document.md")
        }
    );
    assert!(s.is_editing());
    let dest = dir.path().join("hello");
    assert_eq!(
        s.finish_editing(Some(Choice::Save), Some(&dest)).unwrap(),
        LeaveOutcome::Left { rebuild: true }
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("hello.md")).unwrap(),
        "hello"
    );

    let rst = dir.path().join("guide.rst");
    std::fs::write(&rst, "Guide\n=====\n").unwrap();
    let mut s = EditSession::new(doc_at(&rst, "text"), "Guide\n=====\n");
    s.enter_edit();
    s.editor_mut().unwrap().type_text("x").unwrap();
    assert!(matches!(
        s.save(None).unwrap(),
        SaveOutcome::NeedsPath { .. }
    ));
    s.save(Some(&rst)).unwrap();
    assert_eq!(std::fs::read_to_string(&rst).unwrap(), "Guide\n=====\n");
    assert!(dir.path().join("guide.md").exists());
}

#[test]
fn undo_outside_edit_mode_and_on_empty_history() {
    let mut s = fixture();
    let err = s.undo().unwrap_err();
    assert_eq!(err.to_string(), "Turn on edit mode to undo");
    s.enter_edit();
    assert!(!s.undo().unwrap());
    assert!(!s.redo().unwrap());
}

#[test]
fn formatting_is_one_undo_step_each() {
    let mut s = editing("a\nb\nc");
    s.select(Selection::new(0, 5)).unwrap();
    s.format(MarkdownOp::NumberedList).unwrap();
    s.format(MarkdownOp::Bold).unwrap();
    assert_eq!(s.editor().unwrap().undo_depth(), 2);
    s.undo().unwrap();
    assert_eq!(text(&s), "1. a\n2. b\n3. c");
    s.undo().unwrap();
    assert_eq!(text(&s), "a\nb\nc");
}
