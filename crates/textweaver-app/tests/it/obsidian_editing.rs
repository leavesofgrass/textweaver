//! Obsidian in edit mode and canvases (beta 4, obsidian-fixes 7 and 9):
//! inserting a wiki link chosen from the vault, toggling and cycling a
//! task, inserting a callout, and following a canvas's file cards. Each
//! test builds a small vault of placeholder notes in a temporary folder.

use std::path::{Path, PathBuf};

use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::Paths;
use textweaver_app::{App, AppConfig, Command, Effect};

struct Rig {
    app: App,
    vault: PathBuf,
    _tmp: tempfile::TempDir,
}

const MITO: &str =
    "# Mitochondria\n\nIntro.\n\n## Energy\n\nPower.\n\nCells make ATP here. ^atp-fact\n";

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
        let rig = Rig {
            app,
            vault,
            _tmp: tmp,
        };
        rig.write("Projects/Mitochondria.md", MITO);
        rig.write("Notes/Plain.md", "No headings here.\n");
        rig.write("Notes/Dup.md", "Near.\n");
        rig.write("Archive/Dup.md", "Far.\n");
        rig
    }

    fn write(&self, rel: &str, text: &str) -> PathBuf {
        let p = self.vault.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        p
    }

    /// Opens `rel`, puts the cursor at char `at`, and turns on edit mode.
    fn edit(&mut self, rel: &str, at: usize) {
        let p = self.vault.join(rel);
        self.app.open(&p).unwrap();
        self.act(ActionId::ToggleEditMode);
        self.send(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(at))));
        assert_eq!(self.cursor(), at);
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

    fn text(&self) -> String {
        self.app.session().unwrap().doc.text().to_string()
    }

    fn cursor(&self) -> usize {
        self.app.session().unwrap().cursor.0
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
        .unwrap_or_else(|| panic!("no list in {effects:?}"))
}

fn row(items: &[String], starts: &str) -> usize {
    items
        .iter()
        .position(|i| i.starts_with(starts))
        .unwrap_or_else(|| panic!("{starts:?} in {items:?}"))
}

fn same_file(a: &Path, b: &Path) -> bool {
    std::fs::canonicalize(a).ok() == std::fs::canonicalize(b).ok()
}

#[test]
fn a_wiki_link_to_a_note_heading_or_block_is_chosen_from_the_vault() {
    let mut r = Rig::new();
    r.write("Notes/Today.md", "See  today.\n");
    r.edit("Notes/Today.md", 4);
    let (title, items) = list(&r.act(ActionId::InsertWikiLink));
    assert_eq!(title, "Link to a note");
    assert!(
        r.status().starts_with("Link to a note, 5 notes."),
        "{}",
        r.status()
    );
    assert!(
        items.contains(&"Mitochondria, in Projects".to_owned()),
        "{items:?}"
    );
    // The note's headings and block.
    let (title, parts) = list(&r.send(Command::Choose(row(&items, "Mitochondria"))));
    assert_eq!(title, "Link to part of Mitochondria");
    assert_eq!(
        parts,
        [
            "The whole note",
            "Heading 1: Mitochondria",
            "Heading 2: Energy",
            "Block atp-fact: Cells make ATP here.",
        ]
    );
    r.send(Command::Choose(2));
    assert_eq!(r.text(), "See [[Mitochondria#Energy]] today.\n");
    assert_eq!(r.status(), "Inserted a link to Mitochondria#Energy.");
    assert_eq!(r.cursor(), 4 + "[[Mitochondria#Energy]]".len());
    // One undo step.
    r.act(ActionId::Undo);
    assert_eq!(r.text(), "See  today.\n");
}

#[test]
fn a_note_without_parts_links_at_once_and_duplicates_by_path() {
    let mut r = Rig::new();
    r.write("Notes/Today.md", "x\n");
    r.edit("Notes/Today.md", 0);
    let (_, items) = list(&r.act(ActionId::InsertWikiLink));
    r.send(Command::Choose(row(&items, "Plain")));
    assert_eq!(r.text(), "[[Plain]]x\n");
    // Two notes named Dup: the path in the vault tells them apart.
    let (_, items) = list(&r.act(ActionId::InsertWikiLink));
    r.send(Command::Choose(row(&items, "Dup, in Archive")));
    assert!(
        r.text().starts_with("[[Plain]][[Archive/Dup]]"),
        "{}",
        r.text()
    );
}

#[test]
fn a_link_into_the_open_note_names_only_the_heading() {
    let mut r = Rig::new();
    r.edit("Projects/Mitochondria.md", 0);
    let (_, items) = list(&r.act(ActionId::InsertWikiLink));
    let (_, parts) = list(&r.send(Command::Choose(row(&items, "Mitochondria"))));
    r.send(Command::Choose(row(&parts, "Block atp-fact")));
    assert!(
        r.text().starts_with("[[#^atp-fact]]# Mitochondria"),
        "{}",
        r.text()
    );
}

#[test]
fn tasks_toggle_and_cycle_with_their_status_said() {
    let mut r = Rig::new();
    r.write("Notes/List.md", "- [ ] milk\nbread\n");
    r.edit("Notes/List.md", 3);
    r.act(ActionId::ToggleTask);
    assert_eq!(r.text(), "- [x] milk\nbread\n");
    assert_eq!(r.status(), "Checked.");
    assert_eq!(r.cursor(), 3);
    r.act(ActionId::ToggleTask);
    assert_eq!(r.status(), "Not checked.");
    r.act(ActionId::CycleTaskStatus);
    assert_eq!(r.text(), "- [/] milk\nbread\n");
    assert_eq!(r.status(), "In progress.");
    r.act(ActionId::CycleTaskStatus);
    r.act(ActionId::CycleTaskStatus);
    assert_eq!(r.text(), "- [-] milk\nbread\n");
    assert_eq!(r.status(), "Cancelled.");
    // A plain line becomes a task; the caret moves with its text.
    r.send(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(13))));
    r.act(ActionId::ToggleTask);
    assert_eq!(r.text(), "- [-] milk\n- [ ] bread\n");
    assert_eq!(r.status(), "New task, not checked.");
    assert_eq!(r.cursor(), 19);
    r.act(ActionId::Undo);
    assert_eq!(r.text(), "- [-] milk\nbread\n");
}

#[test]
fn callouts_are_chosen_by_type() {
    let mut r = Rig::new();
    r.write("Notes/Tips.md", "Remember this.\n");
    r.edit("Notes/Tips.md", 0);
    let (title, items) = list(&r.act(ActionId::InsertCallout));
    assert_eq!(title, "Callout type");
    assert_eq!(items.len(), 13);
    assert!(
        r.status()
            .starts_with("Callout types, 13. Enter inserts one."),
        "{}",
        r.status()
    );
    r.send(Command::Choose(row(&items, "tip")));
    assert_eq!(r.text(), "> [!tip]\n> Remember this.\n");
    assert_eq!(r.status(), "Inserted a tip callout. Type its text.");
    assert_eq!(r.cursor(), "> [!tip]\n> Remember this.".chars().count());
}

#[test]
fn the_aids_need_edit_mode() {
    let mut r = Rig::new();
    r.app
        .open(&r.vault.join("Projects/Mitochondria.md"))
        .unwrap();
    r.act(ActionId::ToggleTask);
    assert!(
        r.status().starts_with("Turn on edit mode"),
        "{}",
        r.status()
    );
    r.act(ActionId::InsertWikiLink);
    assert!(
        r.status().contains("to insert into the text"),
        "{}",
        r.status()
    );
}

#[test]
fn a_canvas_reads_card_by_card_and_its_file_cards_open_notes() {
    let mut r = Rig::new();
    let board = r.write(
        "Boards/plan.canvas",
        r##"{"nodes":[
            {"id":"t","type":"text","x":0,"y":0,"width":200,"height":100,"text":"Start here\n\nRead the notes."},
            {"id":"f","type":"file","x":300,"y":0,"width":200,"height":100,"file":"Projects/Mitochondria.md","subpath":"#Energy"}
        ],"edges":[{"id":"e","fromNode":"t","toNode":"f","label":"then"}]}"##,
    );
    r.app.open(&board).unwrap();
    let text = r.text();
    assert!(
        text.starts_with("Canvas: 2 cards, 0 groups, 1 connection."),
        "{text}"
    );
    assert!(
        text.contains("From Start here to Mitochondria, Energy: then"),
        "{text}"
    );
    let at = text.find("Mitochondria, Energy, note").unwrap();
    r.send(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(
        text[..at].chars().count(),
    ))));
    r.act(ActionId::FollowLink);
    assert!(same_file(
        &r.path(),
        &r.vault.join("Projects/Mitochondria.md")
    ));
    let s = r.app.session().unwrap();
    let here = s.doc.slice(textweaver_app::core::CharRange::new(
        s.cursor,
        s.cursor.saturating_add(6),
    ));
    assert_eq!(here, "Energy");
}
