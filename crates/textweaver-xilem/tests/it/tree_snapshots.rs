//! Accessibility tree snapshots of every dialog and form in the
//! textweaver app (beta 6, the hearing suite): what a screen reader
//! finds in each dialog, written as text, one node per line, indented by
//! depth, in the tree's order (the order Tab and a screen reader's review
//! go through it). Each line has the role, the name in quotes, then the
//! value, the description and the states in words.
//!
//! Each dialog has its own file in `snapshots/`, compared on every run,
//! so a renamed label, a lost name or a control that moved shows as one
//! changed line in review. To accept a change on purpose, run the tests
//! with `TEXTWEAVER_UPDATE_SNAPSHOTS=1` and review the diff.
//!
//! The lines follow `tools/a11y/tree_report.py`, the CI tree dump
//! (ADR-0039), in spirit: role, name, value, description, states, and
//! nothing that changes from run to run (ids, bounds). Two differences:
//! text is never clipped, so every word is reviewed, and text runs (the
//! parts of a field's text, which the platform adapters do not show as
//! elements) are left out.
//!
//! A password field never shows its value in a snapshot. That a masked
//! field is a password field, and that its secret is nowhere in the tree,
//! is asserted on its own, so updating a snapshot can never hide a leak.
//!
//! Deterministic and the same on every system: a temporary home, no
//! speech, no audio. The live forms that need the app's own broker start
//! it on the loopback address with a port from the system (`:0`); the
//! port is written as "PORT", and the join link's code, new for every
//! broker, as "CODE". The settings sections follow the app's engine and
//! system, but the only settings tied to a system are those of the
//! Eloquence, SAPI and Apple engines, which never apply with speech off,
//! so the sections are the same on Windows, macOS and Linux.

use std::cell::Cell;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use accesskit_consumer::FullNodeId;
use masonry::accesskit::{Live, Role, Toggled};
use masonry::core::{NewWidget, Widget, WidgetId};
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::keymap::ActionId;
use textweaver_app::{App, Command, PromptKey};
use textweaver_xilem::find_panel::{self, FindTexts};
use textweaver_xilem::gui;
use textweaver_xilem::settings_dialog::SettingsForm;
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::Root;

/// The variable that writes the snapshots instead of comparing them.
const UPDATE: &str = "TEXTWEAVER_UPDATE_SNAPSHOTS";

/// The secret typed into the join code field: it must never reach the
/// tree.
#[cfg(feature = "mqtt")]
const SECRET: &str = "Zq7secret";

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("it")
        .join("snapshots")
        .join(format!("{name}.txt"))
}

fn app(home: &Path) -> App {
    let opts = Options {
        no_speech: true,
        home: Some(home.to_path_buf()),
        ..Options::default()
    };
    setup::build_app(&opts, Box::new(LogAnnouncer::default())).0
}

/// The window at the usual review size, scale 1.
fn window(app: &App) -> TestHarness<Root> {
    let p = Palette::galaxy();
    let tree = gui::build_tree(
        &p,
        Default::default(),
        Some(app),
        Rc::new(Cell::new(0)),
        Default::default(),
    );
    let mut params = TestHarnessParams::default();
    params.window_size = (1100, 780).into();
    let mut h = TestHarness::create_with(theme::default_properties(&p), tree.root, params);
    gui::refresh_for_tests(app, &mut h);
    let _ = h.redraw();
    h
}

/// A note's links as the app lists them (as in `dialogs.rs`).
const LINK_ROWS: [&str; 4] = [
    "supports: Chapter note",
    "cites: Week four note, in Pharmacology 2",
    "What links here: 1 note",
    "Add a link",
];

/// The changes and comments as the app lists them (as in `dialogs.rs`).
const CHANGE_ROWS: [&str; 4] = [
    "Inserted: 'renal', by Ada Example, Tuesday, March 3, 2026",
    "Moved away: 'Check the labs first.', by Ada Example, Thursday, March 5, 2026",
    "Comment by Bo Example: check this date, 1 reply, resolved",
    "Inserted: 'Repeat tomorrow.', by Bo Example, date not recorded",
];

/// The dialog kinds `dialogs.rs` opens, built the same way.
const KINDS: [&str; 11] = [
    "list", "prompt", "palette", "question", "settings", "colors", "reading", "voices", "find",
    "links", "changes",
];

fn build(kind: &str, app: &App) -> (NewWidget<dyn Widget>, WidgetId) {
    let p = Palette::galaxy();
    let c = app.catalog();
    let items: Vec<String> = (1..=30).map(|i| format!("Bookmark {i}")).collect();
    match kind {
        "list" => gui::list_dialog(&p, &c, "Bookmarks", items, 0, true),
        "prompt" => gui::prompt_dialog(
            &p,
            &c,
            "Go to line",
            "A line number, or a percentage such as 50%.",
            "",
            false,
            false,
        ),
        "palette" => gui::palette_dialog(&p, &c, "Commands", items),
        "question" => {
            let q = gui::question_dialog(&p, &c, "Remove the voice? y or n", None);
            (q.modal, q.focus)
        }
        "settings" | "colors" | "reading" => {
            let schema = app.settings_schema();
            let form = match kind {
                "colors" => SettingsForm::colors(schema),
                "reading" => SettingsForm::reading(schema),
                _ => SettingsForm::for_window(app),
            };
            let d = gui::settings_dialog(&p, &form, app, 0, 0);
            (d.modal, d.form)
        }
        "voices" => {
            let d = textweaver_xilem::voices::voice_dialog_fit(&p, app, "Voices", items, 0, false);
            (d.modal, d.list)
        }
        "find" => {
            let d = find_panel::find_dialog(&p, app, &FindTexts::default(), false);
            (d.modal, d.find)
        }
        "links" => {
            let rows = LINK_ROWS.iter().map(|r| (*r).to_owned()).collect();
            gui::list_dialog(&p, &c, "Links of: Energy note", rows, 0, true)
        }
        "changes" => {
            let rows = CHANGE_ROWS.iter().map(|r| (*r).to_owned()).collect();
            gui::list_dialog(&p, &c, "Changes and comments", rows, 0, true)
        }
        _ => unreachable!("{kind}"),
    }
}

/// Quotes `s` on one line: inner quotes and line breaks escaped.
fn quoted(s: &str) -> String {
    format!("{s:?}")
}

/// One node as one line: role, name, value, description, then states.
/// A password field's value is never written.
fn describe(n: &accesskit_consumer::NodeRef<'_>, focus: Option<FullNodeId>) -> String {
    let mut line = format!("{:?}", n.role());
    if let Some(name) = n.label().filter(|s: &String| !s.is_empty()) {
        let _ = write!(line, " {}", quoted(&name));
    }
    if n.role() == Role::PasswordInput {
        line.push_str(" value hidden");
    } else if let Some(v) = n.value().filter(|s: &String| !s.is_empty()) {
        let _ = write!(line, " value={}", quoted(&v));
    } else if let Some(v) = n.numeric_value() {
        let _ = write!(line, " value={v}");
    }
    if let Some(d) = n.description().filter(|s: &String| !s.is_empty()) {
        let _ = write!(line, " description={}", quoted(&d));
    }
    let mut states = Vec::new();
    if n.is_focused() || Some(n.id()) == focus {
        states.push("focused".to_owned());
    }
    if n.is_disabled() {
        states.push("disabled".to_owned());
    }
    if n.is_hidden() {
        states.push("hidden".to_owned());
    }
    if n.is_read_only() && n.is_read_only_supported() {
        states.push("read-only".to_owned());
    }
    if n.is_modal() {
        states.push("modal".to_owned());
    }
    if n.is_required() {
        states.push("required".to_owned());
    }
    match n.toggled() {
        Some(Toggled::True) => states.push("checked".to_owned()),
        Some(Toggled::False) => states.push("not checked".to_owned()),
        Some(Toggled::Mixed) => states.push("partly checked".to_owned()),
        None => {}
    }
    if n.is_selected() == Some(true) {
        states.push("selected".to_owned());
    }
    match n.live() {
        Live::Polite => states.push("live polite".to_owned()),
        Live::Assertive => states.push("live assertive".to_owned()),
        Live::Off => {}
    }
    if let Some(a) = n.active_descendant() {
        let name = a.label().unwrap_or_default();
        states.push(format!("active item {}", quoted(&name)));
    }
    if !states.is_empty() {
        let _ = write!(line, " [{}]", states.join(", "));
    }
    line
}

/// `n` and everything under it, one line per node, indented two spaces
/// per level. Text runs are left out.
fn walk(
    n: accesskit_consumer::NodeRef<'_>,
    focus: Option<FullNodeId>,
    depth: usize,
    out: &mut String,
) {
    if n.role() == Role::TextRun {
        return;
    }
    let _ = writeln!(out, "{}- {}", "  ".repeat(depth), describe(&n, focus));
    for c in n.children() {
        walk(c, focus, depth + 1, out);
    }
}

/// The open dialog's subtree, as text.
fn dialog_tree(h: &TestHarness<Root>) -> String {
    // The widget with the focus, as the harness has it.
    let focus = h
        .focused_widget_id()
        .and_then(|w| h.access_node(w))
        .map(|n| n.id());
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        if n.role() == Role::Dialog || n.role() == Role::AlertDialog {
            let mut out = String::new();
            walk(n, focus, 0, &mut out);
            return out;
        }
        stack.extend(n.children().rev());
    }
    panic!("no dialog is open");
}

/// Every text in the whole tree, text runs included: names, values,
/// descriptions.
#[cfg(feature = "mqtt")]
fn all_text(h: &TestHarness<Root>) -> String {
    let mut out = String::new();
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        stack.extend(n.children());
        for t in [n.label(), n.value(), n.description()]
            .into_iter()
            .flatten()
        {
            out.push_str(&t);
            out.push('\n');
        }
    }
    out
}

/// What differs between runs, written as words: the temporary home, and
/// the loopback port the system gave the broker.
fn normalize(text: &str, home: &Path) -> String {
    let mut s = text.replace("\r\n", "\n");
    for form in [
        home.display().to_string(),
        home.display().to_string().replace('\\', "/"),
    ] {
        s = s.replace(&form, "HOME");
    }
    let mut out = String::with_capacity(s.len());
    let needle = "127.0.0.1:";
    let mut rest = s.as_str();
    while let Some(at) = rest.find(needle) {
        let after = &rest[at + needle.len()..];
        let digits = after.chars().take_while(char::is_ascii_digit).count();
        out.push_str(&rest[..at + needle.len()]);
        if digits > 0 {
            out.push_str("PORT");
        }
        rest = &after[digits..];
        // A join link's code is new for every broker: "/CODE".
        if let Some(code) = rest.strip_prefix('/') {
            let len = code
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .count();
            if len > 0 {
                out.push_str("/CODE");
                rest = &code[len..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Compares `fresh` with the snapshot `name`, or writes it when
/// [`UPDATE`] is 1; fails with the first line that differs.
fn compare(name: &str, fresh: &str) {
    let path = snapshot_path(name);
    if std::env::var_os(UPDATE).is_some_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, fresh).expect("the snapshot is writable");
        return;
    }
    let current = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    if current == fresh {
        return;
    }
    let (n, (old, new)) = current
        .lines()
        .chain(std::iter::repeat(""))
        .zip(fresh.lines().chain(std::iter::repeat("")))
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .unwrap_or((0, ("", "")));
    panic!(
        "the tree snapshot {name} differs; if the change is meant, run with \
         {UPDATE}=1 and review the diff.\nFirst difference, line {}:\n  \
         snapshot: {old}\n  now:      {new}",
        n + 1
    );
}

/// A snapshot's first lines: what it is, and how to update it.
fn header(what: &str) -> String {
    format!(
        "Accessibility tree in the textweaver app: {what}. One node per line.\n\
         Written by tests/it/tree_snapshots.rs; update with TEXTWEAVER_UPDATE_SNAPSHOTS=1.\n\n"
    )
}

/// The eleven dialog kinds of the dialog contract, one file each.
#[test]
fn every_dialog_kind_matches_its_tree_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    for kind in KINDS {
        let mut h = window(&app);
        let (modal, focus) = build(kind, &app);
        let mut back = None;
        gui::open_dialog_in(&mut h, modal, focus, &mut back);
        let _ = h.redraw();
        let mut text = header(&format!("the {kind} dialog"));
        text.push_str(&dialog_tree(&h));
        compare(&format!("dialog_{kind}"), &normalize(&text, dir.path()));
    }
}

/// The settings dialog's sections the owner meets first, plus the last,
/// All settings: one file each, named by the section's key.
#[test]
fn settings_sections_match_their_tree_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let p = Palette::galaxy();
    let form = SettingsForm::for_window(&app);
    let wanted = ["reading", "display", "braille"];
    let mut done = 0;
    for (i, key) in form.sections.iter().enumerate() {
        if !wanted.contains(key) {
            continue;
        }
        let mut h = window(&app);
        let d = gui::settings_dialog(&p, &form, &app, i, 0);
        let mut back = None;
        gui::open_dialog_in(&mut h, d.modal, d.form, &mut back);
        let _ = h.redraw();
        let mut text = header(&format!("the settings dialog, section {key}"));
        text.push_str(&dialog_tree(&h));
        compare(&format!("settings_{key}"), &normalize(&text, dir.path()));
        done += 1;
    }
    assert_eq!(done, wanted.len(), "sections: {:?}", form.sections);
}

/// Opens the app's current prompt as the window does, in `h`, and returns
/// its tree. A masked field is masked as the window masks it.
fn prompt_tree(h: &mut TestHarness<Root>, app: &App) -> String {
    let p = Palette::galaxy();
    let c = app.catalog();
    let model = app.prompt_model().expect("a prompt is open").clone();
    let mut back = None;
    let (modal, field) = if model.is_masked() {
        let show = c.tr(model.show_message_ids().0);
        gui::secret_prompt_dialog(
            &p,
            &c,
            &model.label,
            "",
            &model.shown_text(),
            model.purpose.is_form(),
            &show,
        )
    } else {
        gui::prompt_dialog(
            &p,
            &c,
            &model.label,
            "",
            &model.shown_text(),
            false,
            model.purpose.is_form(),
        )
    };
    gui::open_dialog_in(h, modal, field, &mut back);
    if model.is_masked() {
        // As the window does after opening a secret prompt; the harness
        // has no Show box to find in a plain prompt.
        gui::show_secret_in(h, &model);
    }
    let _ = h.redraw();
    let tree = dialog_tree(h);
    gui::close_dialog_in(h, &mut back);
    tree
}

/// Set up live source: the form's seven fields in order, each the dialog
/// the window opens for it, in one file.
#[test]
fn the_live_source_form_matches_its_tree_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let mut h = window(&app);
    let _ = app.dispatch(Command::Action(ActionId::SetUpLiveSource));
    let mut text = header("the live source form, field by field");
    for i in 1..=7 {
        let _ = writeln!(text, "Field {i}:");
        text.push_str(&prompt_tree(&mut h, &app));
        let _ = app.dispatch(Command::PromptKey(PromptKey::Tab));
    }
    let _ = app.dispatch(Command::PromptKey(PromptKey::Escape));
    compare("live_source_form", &normalize(&text, dir.path()));
}

/// The live source form with an encryption that asks for a join code:
/// the join code field, typed in and masked.
#[cfg(feature = "mqtt")]
fn join_code_field(app: &mut App) {
    let _ = app.dispatch(Command::Action(ActionId::SetUpLiveSource));
    let _ = app.dispatch(Command::PromptKey(PromptKey::SetText("Club".into())));
    let _ = app.dispatch(Command::PromptKey(PromptKey::Tab));
    let _ = app.dispatch(Command::PromptKey(PromptKey::Tab));
    for _ in 0..4 {
        let _ = app.dispatch(Command::PromptKey(PromptKey::Down));
    }
    let _ = app.dispatch(Command::PromptKey(PromptKey::Tab));
    let _ = app.dispatch(Command::PromptKey(PromptKey::SetText(SECRET.into())));
    let model = app.prompt_model().expect("the join code field");
    assert!(model.label.starts_with("Join code"), "{}", model.label);
    assert!(model.is_masked());
}

/// The join code field: a password field whose value is never written.
#[cfg(feature = "mqtt")]
#[test]
fn the_join_code_field_matches_its_tree_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let mut h = window(&app);
    join_code_field(&mut app);
    let mut text = header("the join code field of the live source form");
    text.push_str(&prompt_tree(&mut h, &app));
    let _ = app.dispatch(Command::PromptKey(PromptKey::Escape));
    compare("live_join_code_field", &normalize(&text, dir.path()));
}

/// Apart from any snapshot: the masked join code field is a password
/// field, and the secret typed into it is nowhere in the tree, text runs
/// included, nor in any snapshot line written for it.
#[cfg(feature = "mqtt")]
#[test]
fn a_masked_field_is_a_password_field_and_its_secret_never_reaches_the_tree() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let mut h = window(&app);
    join_code_field(&mut app);
    let p = Palette::galaxy();
    let c = app.catalog();
    let model = app.prompt_model().unwrap().clone();
    let show = c.tr(model.show_message_ids().0);
    let mut back = None;
    let (modal, field) =
        gui::secret_prompt_dialog(&p, &c, &model.label, "", &model.shown_text(), true, &show);
    gui::open_dialog_in(&mut h, modal, field, &mut back);
    gui::show_secret_in(&mut h, &model);
    let _ = h.redraw();
    let node = h.access_node(field).expect("the field's node");
    assert_eq!(node.role(), Role::PasswordInput);
    let everything = all_text(&h);
    assert!(!everything.contains(SECRET), "the secret is in the tree");
    let lines = dialog_tree(&h);
    assert!(!lines.contains(SECRET), "{lines}");
    assert!(
        lines.contains("PasswordInput") && lines.contains("value hidden"),
        "{lines}"
    );
    gui::close_dialog_in(&mut h, &mut back);
    let _ = app.dispatch(Command::PromptKey(PromptKey::Escape));
}

/// Publish a check-in: the form's three fields, each the dialog the
/// window opens for it, in one file. The app's own broker runs on the
/// loopback address.
#[cfg(feature = "mqtt")]
#[test]
fn the_check_in_form_matches_its_tree_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let mut h = window(&app);
    let _ = app.update_settings(|s| s.live.broker.listen = vec!["127.0.0.1:0".into()]);
    let _ = app.dispatch(Command::Action(ActionId::StartBroker));
    assert!(app.live_broker_running());
    let _ = app.dispatch(Command::Action(ActionId::SendCheckIn));
    let mut text = header("the check-in form, field by field");
    for i in 1..=3 {
        let _ = writeln!(text, "Field {i}:");
        text.push_str(&prompt_tree(&mut h, &app));
        let _ = app.dispatch(Command::PromptKey(PromptKey::Tab));
    }
    let _ = app.dispatch(Command::PromptKey(PromptKey::Escape));
    let _ = app.dispatch(Command::Action(ActionId::StopBroker));
    app.shutdown();
    compare("live_check_in_form", &normalize(&text, dir.path()));
}

/// Show the join QR code: the warning, the image, the link field and the
/// buttons. The broker's port is written as "PORT".
#[cfg(feature = "mqtt")]
#[test]
fn the_join_qr_dialog_matches_its_tree_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let mut h = window(&app);
    let _ = app.update_settings(|s| s.live.broker.listen = vec!["127.0.0.1:0".into()]);
    let _ = app.dispatch(Command::Action(ActionId::StartBroker));
    let _ = app.dispatch(Command::Action(ActionId::ShowJoinQr));
    let model = app.list_model().expect("the view").clone();
    let view = app.live_join_qr().expect("the code").clone();
    let p = Palette::galaxy();
    let c = app.catalog();
    let d = gui::join_qr_dialog(&p, &c, &model.title, &model.items, &view).expect("built");
    let mut back = None;
    gui::open_dialog_in(&mut h, d.modal, d.focus, &mut back);
    let _ = h.redraw();
    let mut text = header("the join QR code dialog");
    text.push_str(&dialog_tree(&h));
    gui::close_dialog_in(&mut h, &mut back);
    let _ = app.dispatch(Command::Cancel);
    let _ = app.dispatch(Command::Action(ActionId::StopBroker));
    app.shutdown();
    compare("live_join_qr", &normalize(&text, dir.path()));
}

/// The trees are the same twice in one run: nothing in them depends on
/// the clock, ids or the order tests run in.
#[test]
fn a_dialog_tree_is_the_same_twice() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let tree = || {
        let mut h = window(&app);
        let (modal, focus) = build("settings", &app);
        let mut back = None;
        gui::open_dialog_in(&mut h, modal, focus, &mut back);
        let _ = h.redraw();
        normalize(&dialog_tree(&h), dir.path())
    };
    assert_eq!(tree(), tree());
}
