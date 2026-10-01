//! Authoring extras (Agent W4g), driven through `App::dispatch` without a
//! terminal: Markdown lint in edit mode.

use std::path::PathBuf;

use textweaver_app::keymap::ActionId;
use textweaver_app::{App, AppConfig, Command, Effect};

/// A silent app and a folder for the document.
struct Rig {
    app: App,
    dir: PathBuf,
    _tmp: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        Rig {
            app: App::new(AppConfig::for_tests()),
            dir: tmp.path().to_owned(),
            _tmp: tmp,
        }
    }

    fn open(&mut self, name: &str, text: &str) {
        let path = self.dir.join(name);
        std::fs::write(&path, text).unwrap();
        self.app.open(&path).unwrap();
    }

    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        let effects = self.app.dispatch(Command::Action(a));
        self.app.wait_for_writes();
        effects
    }

    fn status(&self) -> String {
        self.app.status_text().to_owned()
    }

    /// The text the editor has selected.
    #[cfg_attr(not(any(feature = "lint", feature = "grammar")), expect(dead_code))]
    fn selected(&self) -> String {
        let sel = self
            .app
            .edit_session()
            .unwrap()
            .editor()
            .unwrap()
            .selection();
        let s = self.app.session().unwrap();
        s.doc.slice(sel.range())
    }
}

#[cfg(feature = "lint")]
#[test]
fn lint_problems_are_found_selected_and_said_in_edit_mode() {
    let mut r = Rig::new();
    r.open(
        "lint.md",
        "# Title\n\n### Too deep\n\nSee https://example.org today. \n\n- one\n* two\n",
    );
    r.act(ActionId::NextLintProblem);
    assert!(
        r.status()
            .starts_with("Lint checks the Markdown you write."),
        "{}",
        r.status()
    );
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::DocumentStart);
    r.act(ActionId::NextLintProblem);
    assert_eq!(
        r.status(),
        "Lint: heading level 3 after level 1; use level 2."
    );
    assert_eq!(r.selected(), "### Too deep");
    r.act(ActionId::NextLintProblem);
    assert_eq!(
        r.status(),
        "Lint: bare web address; put it in angle brackets or make it a link with a name."
    );
    assert_eq!(r.selected(), "https://example.org");
    r.act(ActionId::NextLintProblem);
    assert_eq!(r.status(), "Lint: 1 space at the end of the line.");
    r.act(ActionId::NextLintProblem);
    assert_eq!(r.status(), "Lint: list marker star; this list uses dash.");
    r.act(ActionId::NextLintProblem);
    assert_eq!(r.status(), "No more lint problem. 4 lint problems in all.");
    r.act(ActionId::PreviousLintProblem);
    assert_eq!(r.status(), "Lint: 1 space at the end of the line.");
}

#[cfg(feature = "lint")]
#[test]
fn lint_says_so_for_plain_text() {
    let mut r = Rig::new();
    r.open("plain.txt", "Just text.  \n");
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::NextLintProblem);
    assert_eq!(
        r.status(),
        "Lint checks Markdown, and this document is not Markdown."
    );
}

/// Entering a code block names its language; its other lines say "code".
#[test]
fn a_code_block_names_its_language_on_its_first_line() {
    let mut r = Rig::new();
    r.open(
        "code.md",
        "Before the code.\n\n```py\nprint(1)\nprint(2)\n```\n\nAfter.\n",
    );
    r.act(ActionId::DocumentStart);
    r.act(ActionId::CaretNextLine);
    let mut said = r.status();
    // A blank line may come first.
    if !said.starts_with("code") {
        r.act(ActionId::CaretNextLine);
        said = r.status();
    }
    assert_eq!(said, "code, Python, print(1)");
    r.act(ActionId::CaretNextLine);
    assert_eq!(r.status(), "code, print(2)");
}

/// The items of the list an effect shows.
#[cfg(feature = "grammar")]
fn list_items(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { items, .. } => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[cfg(feature = "grammar")]
#[test]
fn grammar_problems_are_said_with_a_fix_and_fixed() {
    let mut r = Rig::new();
    r.open("grammar.md", "# Notes\n\nShe ate a apple after class.\n");
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::DocumentStart);
    r.act(ActionId::NextGrammarProblem);
    let said = r.status();
    assert!(said.starts_with("Grammar: "), "{said}");
    assert!(said.contains("The words: "), "{said}");
    assert!(said.contains(" Fix: "), "{said}");
    assert!(said.contains("lists fixes."), "{said}");
    let selected = r.selected();
    assert!(selected.starts_with('a'), "{selected}");
    let effects = r.act(ActionId::SpellingSuggestions);
    let items = list_items(&effects);
    assert_eq!(
        items.last().map(String::as_str),
        Some("Leave it as it is"),
        "{items:?}"
    );
    let i = items.iter().position(|x| x.starts_with("an")).unwrap();
    r.app.dispatch(Command::Choose(i));
    let text = r.app.session().unwrap().doc.text().to_string();
    assert!(text.contains("She ate an apple"), "{text}");
    assert!(r.status().starts_with("Changed to an"), "{}", r.status());
    r.act(ActionId::DocumentStart);
    r.act(ActionId::NextGrammarProblem);
    assert_eq!(r.status(), "No grammar problems found.");
}
