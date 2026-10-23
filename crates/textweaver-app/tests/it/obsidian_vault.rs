//! Obsidian vaults in the reader (beta 4, obsidian-fixes 1, 2, 5 and 10):
//! links that reach across the vault and never leave it, heading and block
//! links, aliases, duplicate names, properties, backlinks, unresolved
//! links, and tags. Each test builds a small vault of placeholder notes in
//! a temporary folder.

use std::path::{Path, PathBuf};

use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::Paths;
use textweaver_app::{App, AppConfig, Command, Effect};

struct Rig {
    app: App,
    vault: PathBuf,
    outside: PathBuf,
    _tmp: tempfile::TempDir,
}

const CELL: &str = "---\ntitle: The Cell\naliases:\n  - Cytology\ntags: [biology, exam]\ndone: true\n---\n# Cell\n\nSee [[Mitochondria]], [[Mitochondria#Energy]], [[Mitochondria#Parts#Matrix]], [[Mitochondria#^atp-fact]], [[Cytology]], [[Dup]], [[Ghost]], and [[Outside]].\n\n#exam\n";

const MITO: &str = "# Mitochondria\n\nIntro.\n\n## Energy\n\nPower.\n\n## Parts\n\n### Matrix\n\nInside.\n\nCells make ATP here. ^atp-fact\n\nBack to [[The Cell]]. #biology/organelles\n";

impl Rig {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::under(&tmp.path().join("home"));
        let app = App::new(AppConfig {
            paths: Some(paths),
            ..AppConfig::for_tests()
        });
        let vault = tmp.path().join("vault");
        std::fs::create_dir_all(vault.join(".obsidian")).unwrap();
        let outside = tmp.path().join("Outside.md");
        std::fs::write(&outside, "Outside the vault.\n").unwrap();
        let rig = Rig {
            app,
            vault,
            outside,
            _tmp: tmp,
        };
        rig.write("Notes/Cell.md", CELL);
        rig.write("Projects/Mitochondria.md", MITO);
        rig.write("Notes/Dup.md", "Near dup.\n");
        rig.write("Archive/Dup.md", "Far dup.\n");
        rig.write("Daily/today.md", "Read ![[Cell]] today. #Exam\n");
        rig
    }

    fn write(&self, rel: &str, text: &str) -> PathBuf {
        let p = self.vault.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        p
    }

    fn open(&mut self, rel: &str) {
        let p = self.vault.join(rel);
        self.app.open(&p).unwrap();
    }

    fn send(&mut self, cmd: Command) -> Vec<Effect> {
        let effects = self.app.dispatch(cmd);
        self.app.wait_for_writes();
        effects
    }

    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        self.send(Command::Action(a))
    }

    fn status(&self) -> String {
        self.app.status_text().to_owned()
    }

    fn title(&self) -> String {
        self.app.session().unwrap().title.clone()
    }

    fn text(&self) -> String {
        self.app.session().unwrap().doc.text().to_string()
    }

    fn at_cursor(&self, n: usize) -> String {
        let s = self.app.session().unwrap();
        s.doc
            .slice(CharRange::new(s.cursor, s.cursor.saturating_add(n)))
    }

    /// Puts the cursor on the `nth` (from 0) occurrence of `needle`.
    fn go(&mut self, needle: &str, nth: usize) {
        let t = self.text();
        let at = t
            .match_indices(needle)
            .nth(nth)
            .unwrap_or_else(|| panic!("{needle:?} {nth} in {t:?}"))
            .0;
        let pos = CharPos(t[..at].chars().count());
        self.send(Command::GoTo(textweaver_app::text::GoTo::Char(pos)));
    }

    /// Follows the `nth` link whose text is `needle`, from Cell.
    fn follow_from_cell(&mut self, needle: &str, nth: usize) {
        self.open("Notes/Cell.md");
        self.go(needle, nth);
        self.act(ActionId::FollowLink);
    }

    fn path(&self) -> PathBuf {
        self.app.session().unwrap().doc.meta.path.clone().unwrap()
    }
}

fn list(effects: &[Effect]) -> (String, Vec<String>) {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { title, items } => Some((title.clone(), items.clone())),
            _ => None,
        })
        .unwrap_or_default()
}

fn same_file(a: &Path, b: &Path) -> bool {
    std::fs::canonicalize(a).ok() == std::fs::canonicalize(b).ok()
}

#[test]
fn links_reach_another_folder_of_the_vault_and_its_headings() {
    let mut r = Rig::new();
    r.follow_from_cell("Mitochondria", 0);
    assert_eq!(r.title(), "Mitochondria");
    assert!(
        r.status().starts_with("Opened Mitochondria. Back:"),
        "{}",
        r.status()
    );
    // A heading, and a nested heading path (the last heading).
    r.follow_from_cell("Mitochondria", 1);
    assert_eq!(r.at_cursor(6), "Energy");
    r.follow_from_cell("Mitochondria", 2);
    assert_eq!(r.at_cursor(6), "Matrix");
}

#[test]
fn block_links_open_at_the_block() {
    let mut r = Rig::new();
    // Today a block link said "No heading called ^atp-fact".
    r.follow_from_cell("Mitochondria", 3);
    assert_eq!(r.title(), "Mitochondria");
    assert_eq!(r.at_cursor(15), "Cells make ATP ");
    // A block link within the same note.
    r.write(
        "Projects/Self.md",
        "[[#^here|jump]]\n\nFiller.\n\nThe target block. ^here\n",
    );
    r.open("Projects/Self.md");
    r.go("jump", 0);
    r.act(ActionId::FollowLink);
    assert_eq!(r.at_cursor(10), "The target");
    // A block that does not exist says so in words.
    r.write("Projects/Bad.md", "[[#^nope|bad]]\n");
    r.open("Projects/Bad.md");
    r.go("bad", 0);
    r.act(ActionId::FollowLink);
    assert_eq!(r.status(), "No block nope in this document.");
}

#[test]
fn aliases_titles_and_duplicates() {
    let mut r = Rig::new();
    // `[[The Cell]]` names Cell.md by its front matter title.
    r.open("Projects/Mitochondria.md");
    r.go("The Cell", 0);
    r.act(ActionId::FollowLink);
    assert!(same_file(&r.path(), &r.vault.join("Notes/Cell.md")));
    // `[[Cytology]]` by an alias (a block list in the front matter).
    r.follow_from_cell("Cytology", 0);
    assert!(same_file(&r.path(), &r.vault.join("Notes/Cell.md")));
    // Two notes named Dup: the nearest opens, said once.
    r.follow_from_cell("Dup", 0);
    assert_eq!(r.text().trim(), "Near dup.");
    assert!(
        r.status()
            .contains("2 notes named Dup; the nearest opened."),
        "{}",
        r.status()
    );
    r.follow_from_cell("Dup", 0);
    assert!(!r.status().contains("notes named"), "{}", r.status());
}

#[test]
fn links_never_leave_the_vault() {
    let mut r = Rig::new();
    assert!(r.outside.is_file());
    r.follow_from_cell("Outside", 0);
    assert_eq!(r.status(), "Link to a note that doesn't exist: Outside.");
    assert!(same_file(&r.path(), &r.vault.join("Notes/Cell.md")));
    r.follow_from_cell("Ghost", 0);
    assert_eq!(r.status(), "Link to a note that doesn't exist: Ghost.");
}

#[test]
fn properties_are_counted_on_opening_and_listed_on_f() {
    let mut r = Rig::new();
    r.open("Notes/Cell.md");
    assert_eq!(r.status(), "Opened The Cell. Properties: 4.");
    let (title, items) = list(&r.act(ActionId::NoteProperties));
    assert_eq!(title, "Properties");
    assert_eq!(
        items,
        [
            "title: The Cell",
            "aliases: Cytology",
            "tags: biology, exam",
            "done: checked"
        ]
    );
    // A note without front matter says nothing about properties.
    r.open("Daily/today.md");
    assert_eq!(r.status(), "Opened today.md.");
    r.act(ActionId::NoteProperties);
    assert_eq!(r.status(), "This document has no properties.");
}

#[test]
fn backlinks_list_the_notes_that_link_here_and_open_one() {
    let mut r = Rig::new();
    r.open("Notes/Cell.md");
    let effects = r.act(ActionId::Backlinks);
    let (title, items) = list(&effects);
    assert_eq!(title, "Notes that link here");
    assert_eq!(
        items,
        [
            "today, 1 link, in Daily",
            "Mitochondria, 1 link, in Projects"
        ]
    );
    assert!(
        r.status().starts_with("Notes that link here, 2 notes."),
        "{}",
        r.status()
    );
    r.send(Command::Choose(1));
    assert_eq!(r.title(), "Mitochondria");
    // A new note linking here is seen at once: the index refreshes.
    r.write("New.md", "About [[Cytology]] and [[The Cell]].\n");
    r.open("Notes/Cell.md");
    let (_, items) = list(&r.act(ActionId::Backlinks));
    assert_eq!(items[1], "New, 2 links, in top folder");
    // A note nobody links to.
    r.open("Daily/today.md");
    r.act(ActionId::Backlinks);
    assert_eq!(r.status(), "No notes link here.");
}

#[test]
fn unresolved_links_are_listed() {
    let mut r = Rig::new();
    r.open("Notes/Cell.md");
    let (title, items) = list(&r.act(ActionId::UnresolvedLinks));
    assert_eq!(title, "Links to notes that don't exist");
    assert_eq!(items, ["Ghost", "Outside"]);
    r.open("Projects/Mitochondria.md");
    r.act(ActionId::UnresolvedLinks);
    assert_eq!(r.status(), "Every link in this note leads to a note.");
}

#[test]
fn tags_in_the_note_and_the_vault_lead_to_notes() {
    let mut r = Rig::new();
    r.open("Notes/Cell.md");
    let (title, items) = list(&r.act(ActionId::NoteTags));
    assert_eq!(title, "Tags in this note");
    assert_eq!(items, ["biology", "exam"]);
    // Enter on "exam": the notes with it, any case.
    let (title, items) = list(&r.send(Command::Choose(1)));
    assert_eq!(title, "Notes with a tag");
    assert_eq!(items, ["today, in Daily", "The Cell, in Notes"]);
    let (title, items) = list(&r.act(ActionId::VaultTags));
    assert_eq!(title, "Tags in the vault");
    assert_eq!(
        items,
        [
            "biology, 1 note",
            "biology/organelles, 1 note",
            "Exam, 2 notes"
        ]
    );
    // Enter on "biology" finds the nested tag too; Enter opens a note.
    let (_, items) = list(&r.send(Command::Choose(0)));
    assert_eq!(items, ["The Cell, in Notes", "Mitochondria, in Projects"]);
    r.send(Command::Choose(1));
    assert_eq!(r.title(), "Mitochondria");
}

#[test]
fn a_folder_without_obsidian_keeps_its_own_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(docs.join("sub")).unwrap();
    std::fs::create_dir_all(tmp.path().join("other")).unwrap();
    std::fs::write(docs.join("sub/Deep.md"), "# Deep\n").unwrap();
    std::fs::write(tmp.path().join("other/Side.md"), "# Side\n").unwrap();
    std::fs::write(docs.join("a.md"), "[[Deep]] and [[Side]].\n").unwrap();
    let mut app = App::new(AppConfig {
        paths: Some(Paths::under(&tmp.path().join("home"))),
        ..AppConfig::for_tests()
    });
    app.open(&docs.join("a.md")).unwrap();
    app.dispatch(Command::Action(ActionId::FollowLink));
    assert_eq!(app.session().unwrap().title, "Deep");
    app.open(&docs.join("a.md")).unwrap();
    let t = app.session().unwrap().doc.text().to_string();
    let at = t.find("Side").unwrap();
    app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(
        t[..at].chars().count(),
    ))));
    app.dispatch(Command::Action(ActionId::FollowLink));
    assert_eq!(
        app.status_text(),
        "Link to a note that doesn't exist: Side."
    );
}
