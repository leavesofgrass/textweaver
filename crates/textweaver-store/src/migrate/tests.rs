//! Migration tests against a synthetic Star configuration directory built
//! here (never a real Star installation).

use std::path::{Path, PathBuf};

use serde_json::json;
use textweaver_core::{CharPos, CharRange};

use super::*;
use crate::{Library, Settings};

/// Loads documents the way a minimal loader would: Markdown heading and
/// bullet markers dropped, lines kept (textweaver's canonical shape).
struct FakeHost;

impl MigrationHost for FakeHost {
    fn load(&self, path: &Path) -> Option<LoadedDoc> {
        let raw = std::fs::read_to_string(path).ok()?;
        let text = raw
            .lines()
            .map(|l| {
                l.strip_prefix("# ")
                    .or_else(|| l.strip_prefix("- "))
                    .unwrap_or(l)
            })
            .collect::<Vec<_>>()
            .join("\n");
        Some(LoadedDoc {
            text,
            title: None,
            format: "markdown".into(),
        })
    }

    fn keymap_entry(&self, action: &str, chord: &str) -> Option<Vec<String>> {
        (chord != "Bogus+Key").then(|| {
            let mut v = vec![chord.to_owned()];
            if action == "next_sentence" {
                v.push("b:.".to_owned());
            }
            v
        })
    }
}

const BOOK_MD: &str = "# Title\n\nFirst paragraph has several words.\nIt continues here.\n\n- item one\n- item two\n\nThe end of the story.";

/// Star's `plain_text` for BOOK_MD: lines joined and the list narrated,
/// so its offsets differ from textweaver's.
const BOOK_STAR: &str = "Title\n\nFirst paragraph has several words. It continues here.\n\nList with 2 items. item one item two\n\nThe end of the story.";

/// textweaver's text for BOOK_MD (what FakeHost produces).
fn book_tw() -> String {
    FakeHost.load_text(BOOK_MD)
}

impl FakeHost {
    fn load_text(&self, raw: &str) -> String {
        raw.lines()
            .map(|l| {
                l.strip_prefix("# ")
                    .or_else(|| l.strip_prefix("- "))
                    .unwrap_or(l)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn char_index(text: &str, needle: &str) -> usize {
    text[..text.find(needle).unwrap()].chars().count()
}

struct Fixture {
    _dir: tempfile::TempDir,
    star: PathBuf,
    lib: PathBuf,
    book: PathBuf,
    paths: Paths,
}

fn mtime(p: &Path) -> f64 {
    std::fs::metadata(p)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = resolve_path(dir.path());
    let star = root.join("star");
    let lib = root.join("lib");
    std::fs::create_dir_all(star.join("cache")).unwrap();
    std::fs::create_dir_all(lib.join("sub")).unwrap();
    std::fs::create_dir_all(lib.join(".star")).unwrap();
    let book = lib.join("book.md");
    std::fs::write(&book, BOOK_MD).unwrap();
    let other = lib.join("sub").join("b.md");
    std::fs::write(
        &other,
        "alpha beta gamma delta epsilon zeta eta theta iota kappa",
    )
    .unwrap();
    let book_s = book.to_string_lossy().into_owned();
    let missing = root.join("missing.md").to_string_lossy().into_owned();

    // Star's parse cache holds the book's plain_text.
    std::fs::write(
        star.join("cache").join("0123456789abcdef_ffffffff_v1.json"),
        json!({
            "version": 1, "path": book_s, "mtime": mtime(&book),
            "settings_fingerprint": "ffffffff",
            "data": {"plain_text": BOOK_STAR, "markdown": BOOK_MD, "title": "Title"}
        })
        .to_string(),
    )
    .unwrap();

    let at = |w: &str| char_index(BOOK_STAR, w);
    let settings = json!({
        "tts_rate": 300,
        "tts_volume": 1.0,
        "theme": "nord",
        "tts_voice": "com.example.voice",
        "qt_font_size": 18,
        "sync_conflict_policy": "highest_progress",
        "library_folders": [lib.to_string_lossy(), root.join("gone").to_string_lossy()],
        "recent_files": [book_s, missing],
        "last_path": book_s,
        "keybindings": {"Alt+.": "Alt+N", "Ctrl+Alt+E": "Ctrl+Alt+R", "Ctrl+B": "", "Ctrl+P": "Bogus+Key"},
        "reading_positions": {
            book_s.clone(): {"offset": at("end"), "pct": 80, "ts": "2026-06-27T10:00:00"},
            "__no_path__": {"offset": 3, "pct": 1, "ts": "2026-06-27T10:00:00"}
        },
        "bookmarks": {book_s.clone(): {"mark1": {"offset": at("continues"), "pct": 30, "ts": "2026-06-27T10:00:00"}}},
        "annotations": {book_s.clone(): [
            {"char_pos": 0, "word_idx": 13, "anchor": "The  end of the story.", "note": "Ending",
             "tags": ["plot", "#end"], "cite": "", "ts": "2026-06-28T09:00:00",
             "sr_state": {"interval": 4}},
            {"id": "abc12345", "char_pos": 0, "word_idx": 1, "anchor": "",
             "note": "No anchor", "tags": [], "ts": "2026-06-28T09:00:00"}
        ]},
        "user_highlights": {
            book_s.clone(): [{"start": at("several"), "end": at("several") + "several words".len(), "color": "#90ee90"}],
            "__no_path__": [{"start": 0, "end": 4, "color": "#ffff00"}]
        },
        "library": {book_s.clone(): {"title": "The Book", "format": "markdown", "added": "2026-06-01T08:00:00", "last_opened": "2026-06-27T10:00:00"}},
        "reading_stats": {book_s: {"seconds": 12.5, "words_read": 40}}
    });
    std::fs::write(star.join("settings.json"), settings.to_string()).unwrap();

    std::fs::write(
        lib.join(".star").join("progress.json"),
        json!({
            "sub/b.md": {"offset": 17, "pct": 30, "ts": "2026-06-27T10:00:00"},
            "gone.md": {"offset": 1, "pct": 1, "ts": "2026-06-27T10:00:00"},
            "_meta": {"sub/b.md": {"seconds": 5, "last_ts": "2026-06-27T10:00:00"}}
        })
        .to_string(),
    )
    .unwrap();

    let paths = Paths::under(&root.join("tw"));
    Fixture {
        _dir: dir,
        star,
        lib,
        book,
        paths,
    }
}

fn run(f: &Fixture, dry_run: bool) -> MigrationReport {
    migrate_star(
        &MigrateOptions {
            from: f.star.clone(),
            dry_run,
        },
        &f.paths,
        &FakeHost,
    )
    .unwrap()
}

fn item<'a>(r: &'a MigrationReport, kind: ItemKind, subject_part: &str) -> &'a ReportItem {
    r.items
        .iter()
        .find(|i| i.kind == kind && i.subject.contains(subject_part))
        .unwrap_or_else(|| panic!("no {kind:?} item for {subject_part}: {:#?}", r.items))
}

#[test]
fn dry_run_reports_everything_and_writes_nothing() {
    let f = fixture();
    let r = run(&f, true);
    assert!(r.dry_run);
    assert!(r.written.is_empty());
    assert!(!f.paths.config_dir.exists() && !f.paths.data_dir.exists());
    assert!(!f.lib.join(".textweaver").exists());
    assert!(r.count(Outcome::Imported) > 5);
    assert!(r.render().contains("Dry run: nothing was written."));
}

#[test]
fn imports_settings_state_library_keys_and_sidecars() {
    let f = fixture();
    let r = run(&f, false);
    let tw = book_tw();

    // Settings: changed values only; Star-only values skipped with reasons.
    let s: Settings = SettingsStore::new(f.paths.clone()).load().0;
    assert_eq!(s.speech.rate.wpm(), 300);
    assert_eq!(s.display.theme, "nord");
    assert_eq!(
        s.reading.sync_conflict_policy,
        ConflictPolicy::HighestProgress
    );
    assert_eq!(s.library.folders, vec![f.lib.clone()]);
    assert_eq!(
        item(&r, ItemKind::Setting, "tts_voice").outcome,
        Outcome::Skipped
    );
    assert!(
        item(&r, ItemKind::Other, "without a textweaver equivalent")
            .detail
            .contains("qt_font_size")
    );
    assert_eq!(
        item(&r, ItemKind::LibraryFolder, "gone").outcome,
        Outcome::Skipped
    );
    assert!(
        r.items.iter().all(|i| i.subject != "tts_volume"),
        "defaults are not listed"
    );

    // Keys: remaps become overrides; unknown shortcuts and keys are skipped.
    let keys = SettingsStore::new(f.paths.clone()).load_keymap().unwrap();
    assert_eq!(keys["next_sentence"], vec!["Alt+N", "b:."]);
    assert_eq!(keys["bold"], Vec::<String>::new(), "an empty remap unbinds");
    assert!(!keys.contains_key("next_paragraph"));
    assert_eq!(
        item(&r, ItemKind::Keybinding, "Ctrl+Alt+E").outcome,
        Outcome::Skipped
    );
    assert_eq!(
        item(&r, ItemKind::Keybinding, "Bogus").outcome,
        Outcome::Skipped
    );

    // Per-document state, mapped through Star's cached text.
    let state = StateStore::new(f.paths.state_dir())
        .load(&DocKey::for_path(&f.book))
        .unwrap();
    assert_eq!(state.position, CharPos(char_index(&tw, "end")));
    assert_eq!(
        state.ts,
        crate::time::parse_timestamp("2026-06-27T10:00:00").unwrap()
    );
    assert!(
        item(&r, ItemKind::Position, "book.md")
            .detail
            .contains("matched word for word")
    );
    assert_eq!(
        state.bookmark("mark1").unwrap().pos,
        CharPos(char_index(&tw, "continues"))
    );
    // The note is placed by its quoted text; the one without an anchor by
    // its word index ("First" is word 1 of Star's text).
    let ending = state.notes.iter().find(|n| n.note == "Ending").unwrap();
    let start = char_index(&tw, "The end");
    assert_eq!(
        ending.range,
        CharRange::new(start, start + "The end of the story.".len())
    );
    assert_eq!(ending.tags, vec!["plot", "end"]);
    assert_eq!(ending.anchor, "The end of the story.");
    assert_eq!(ending.extra["sr_state"]["interval"], 4);
    assert!(ending.id.starts_with("star-"));
    let plain = state.note("abc12345").unwrap();
    assert_eq!(plain.range, CharRange::empty(char_index(&tw, "First")));
    // The highlight covers the same words.
    let h = &state.highlights[0];
    assert_eq!(
        tw.chars()
            .skip(h.range.start.0)
            .take(h.range.len())
            .collect::<String>(),
        "several words"
    );
    assert_eq!(h.color, "#90ee90");
    assert_eq!(
        item(&r, ItemKind::Other, "__no_path__").outcome,
        Outcome::Skipped
    );

    // Recents, bookshelf, and skipped statistics.
    let recent = Recent::load(&f.paths.recent_file());
    assert_eq!(recent.entries.len(), 1);
    assert_eq!(
        item(&r, ItemKind::Recent, "missing.md").outcome,
        Outcome::Skipped
    );
    let lib = Library::load(&f.paths.library_file()).unwrap();
    let e = lib.get(&f.book).unwrap();
    assert_eq!(e.title, "The Book");
    assert_eq!(
        e.added,
        crate::time::parse_timestamp("2026-06-01T08:00:00").unwrap()
    );
    // Star's reading statistics reach stats.json.
    let stats = crate::ReadingStats::load(&f.paths).unwrap();
    let (_, d) = stats.most_read(1)[0];
    assert_eq!(d.seconds, 12.5);
    assert_eq!(d.title, "The Book");

    // The sidecar is converted next to Star's, which is left alone.
    let side = sync::read_sidecar(&f.lib);
    let entry = sync::ProgressEntry::from_value(side.get("sub/b.md").unwrap()).unwrap();
    assert_eq!(
        entry.offset,
        CharPos(17),
        "no cache: the offset is checked against the percentage"
    );
    assert_eq!(entry.ts, "2026-06-27T10:00:00Z");
    assert_eq!(side.get("_meta").unwrap()["sub/b.md"]["seconds"], 5);
    assert!(side.get("gone.md").is_none());
    assert!(f.lib.join(".star").join("progress.json").exists());
    assert_eq!(
        item(&r, ItemKind::Sidecar, "gone.md").outcome,
        Outcome::Skipped
    );

    assert_eq!(r.star_cache_documents, 1);
    assert!(!r.written.is_empty());
    let text = r.render();
    assert!(text.contains("Reading positions: 1 imported"), "{text}");
    assert!(text.contains("Skipped:"), "{text}");
}

#[test]
fn a_second_run_imports_nothing_new() {
    let f = fixture();
    run(&f, false);
    let again = run(&f, false);
    let imported: Vec<&ReportItem> = again
        .items
        .iter()
        .filter(|i| i.outcome == Outcome::Imported)
        .collect();
    assert!(imported.is_empty(), "{imported:#?}");
    assert!(again.written.is_empty(), "{:?}", again.written);
    assert!(again.render().contains("Nothing new to write."));
}

#[test]
fn newer_textweaver_state_wins() {
    let f = fixture();
    let store = StateStore::new(f.paths.state_dir());
    let key = DocKey::for_path(&f.book);
    let mut st = DocState::default();
    st.set_position(CharPos(3), 100);
    st.add_bookmark(Some("mark1"), CharPos(5), 100);
    store.save(&key, &st).unwrap();
    drop(store);
    let r = run(&f, false);
    let back = StateStore::new(f.paths.state_dir()).load(&key).unwrap();
    assert_eq!(back.position, CharPos(3));
    assert_eq!(back.bookmark("mark1").unwrap().pos, CharPos(5));
    assert_eq!(
        item(&r, ItemKind::Position, "book.md").outcome,
        Outcome::Unchanged
    );
    assert_eq!(
        item(&r, ItemKind::Bookmark, "mark1").outcome,
        Outcome::Unchanged
    );
    assert_eq!(back.notes.len(), 2, "notes still arrive");
}

#[test]
fn missing_star_settings_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = migrate_star(
        &MigrateOptions {
            from: dir.path().join("nothing"),
            dry_run: true,
        },
        &Paths::under(dir.path()),
        &FakeHost,
    );
    assert!(err.is_err());
    std::fs::write(dir.path().join("settings.json"), "[1, 2]").unwrap();
    let err = migrate_star(
        &MigrateOptions {
            from: dir.path().to_owned(),
            dry_run: true,
        },
        &Paths::under(&dir.path().join("tw")),
        &FakeHost,
    );
    assert!(err.is_err(), "a JSON array is not Star's settings");
}
