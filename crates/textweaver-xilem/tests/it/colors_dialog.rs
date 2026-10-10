//! View, Colors (W6a6): every color setting in one form, in Masonry's test
//! harness. What a screen reader finds (the color and its contrast in
//! words, never the sample alone), Enter typing a color, and the Reset all
//! colors button beside Close.

use std::cell::Cell;
use std::rc::Rc;

use masonry::accesskit::Role;
use masonry::core::TextEvent;
use masonry::core::keyboard::{Key, NamedKey};
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::App;
use textweaver_app::a11y::LogAnnouncer;
use textweaver_xilem::gui::{self, FORM, ROOT};
use textweaver_xilem::settings_dialog::{self, FormAction, FormChange, SettingsForm};
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::Root;

fn app(home: &std::path::Path) -> App {
    let opts = Options {
        no_speech: true,
        home: Some(home.to_path_buf()),
        ..Options::default()
    };
    setup::build_app(&opts, Box::new(LogAnnouncer::default())).0
}

/// The window with the Colors dialog open; whether it has a Reset all
/// colors button.
fn harness(app: &App) -> (TestHarness<Root>, bool) {
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
    let form = SettingsForm::colors(app.settings_schema());
    let d = gui::settings_dialog(&p, &form, app, 0, 0);
    let reset = d.reset.is_some();
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(d.modal)));
    h.focus_on(Some(d.form));
    let _ = h.redraw();
    (h, reset)
}

/// Every label of `role` in the tree.
fn names_of(h: &TestHarness<Root>, role: Role) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        if n.role() == role {
            out.push(n.label().unwrap_or_default());
        }
        stack.extend(n.children());
    }
    out
}

#[test]
fn every_color_setting_is_in_the_form_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let form = SettingsForm::colors(app.settings_schema());
    assert!(form.is_colors());
    assert_eq!(form.sections.len(), 1);
    let paths: Vec<&str> = form
        .settings_in(0)
        .iter()
        .map(|s| s.path.as_str())
        .collect();
    assert_eq!(paths, textweaver_app::COLOR_SETTINGS.to_vec());
    assert_eq!(
        form.section_title(0, &app.catalog()),
        app.catalog().tr("section-colors")
    );
}

/// A color shows a sample and says its contrast in words; the theme's own
/// color shows no sample.
#[test]
fn a_color_says_its_contrast_and_shows_a_sample() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let form = SettingsForm::colors(app.settings_schema());
    let row = form
        .settings_in(0)
        .iter()
        .position(|s| s.path == "colors.links")
        .unwrap();
    assert_eq!(form.rows(0, &app)[row].swatch, None, "the theme's color");
    let setting = form.setting(0, row).unwrap().clone();
    let said = settings_dialog::apply(&mut app, &setting, FormChange::Text("blue".into()));
    assert!(said.is_ok(), "{said:?}");
    let after = form.rows(0, &app);
    assert!(after[row].swatch.is_some(), "a sample of blue");
    let (ratio, verdict) = app.color_contrast("colors.links").expect("a contrast");
    assert!(ratio > 1.0);
    assert!(
        after[row].value_text.contains(&verdict),
        "{} says {verdict}",
        after[row].value_text
    );
    assert!(after[row].is_typed(), "Enter types a color");
    // The full settings dialog's Colors section shows the same rows.
    let full = SettingsForm::new(app.settings_schema());
    let (sec, r) = full.find("colors.links").unwrap();
    assert_eq!(full.rows(sec, &app)[r], after[row]);
}

/// The dialog: one form named Colors (no section list), a Reset all
/// colors button beside Close, and Enter on a color asks for a name or
/// value.
#[test]
fn the_dialog_has_the_form_and_its_buttons() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let (mut h, reset) = harness(&app);
    assert!(reset);
    let c = app.catalog();
    let form = h.access_node(h.get_widget(FORM).id()).expect("the form");
    assert_eq!(
        form.label().as_deref(),
        Some(c.tr("section-colors").as_str())
    );
    assert!(form.children().count() >= textweaver_app::COLOR_SETTINGS.len());
    let buttons = names_of(&h, Role::Button);
    for name in [c.tr("gui-colors-reset-all"), c.tr("gui-button-close")] {
        assert!(buttons.contains(&name), "{name} in {buttons:?}");
    }
    let lists = names_of(&h, Role::ListBox);
    assert!(
        !lists.contains(&c.tr("gui-settings-sections")),
        "no section list: {lists:?}"
    );
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Enter)));
    let (action, _) = h.pop_action::<FormAction>().expect("Enter edits");
    assert!(
        matches!(action, FormAction::Edit { row: 0, .. }),
        "{action:?}"
    );
}

/// Every row is measured, the theme's own color too, and the highlight
/// palette's entries end the form (B1-g2c): each says its name first,
/// then its color, shape and contrast with the text and the page in
/// words, with a sample; a low one has the contrast warning as its
/// description; and each belongs to the palette's table, read only.
#[test]
fn every_row_is_measured_and_the_palette_ends_the_form() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let c = app.catalog();
    let form = SettingsForm::colors(app.settings_schema());
    let rows = form.rows(0, &app);
    let n = textweaver_app::COLOR_SETTINGS.len();
    assert_eq!(rows.len(), n + app.highlight_palette().len());
    for r in &rows[..n] {
        assert!(
            r.value_text.contains("contrast"),
            "{}: {}",
            r.label,
            r.value_text
        );
    }
    let first = &rows[n];
    assert_eq!(first.label, "Highlight name important");
    assert!(
        first.value_text.starts_with("yellow, underline, contrast"),
        "{}",
        first.value_text
    );
    assert!(
        first.value_text.contains("with the page"),
        "{}",
        first.value_text
    );
    assert!(first.swatch.is_some(), "a sample beside the words");
    assert_eq!(first.kind, settings_dialog::RowKind::Table);
    assert_eq!(first.help, c.tr("colors-contrast-warning"), "low on Galaxy");
    assert_eq!(
        form.setting(0, n).map(|s| s.path.as_str()),
        Some("highlight.palette")
    );
    assert_eq!(
        form.setting(0, rows.len() + 3).map(|s| s.path.as_str()),
        Some("highlight.palette")
    );
    // A screen reader finds the entry by its name, and its words.
    let (h, _) = harness(&app);
    let mut found = None;
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(node) = stack.pop() {
        if node.label().as_deref() == Some("Highlight name important") {
            found = Some((node.value(), node.description()));
        }
        stack.extend(node.children());
    }
    let (value, description) = found.expect("the entry is in the tree");
    assert!(value.unwrap_or_default().contains("with the text"));
    assert_eq!(description, Some(c.tr("colors-contrast-warning")));
}

/// While the system's high contrast colors are drawn, they win: the help
/// says so, no row shows a sample or a contrast of a color not on the
/// screen, and the highlight names still differ by shape.
#[test]
fn the_system_high_contrast_colors_win() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    app.set_setting("colors.links", serde_json::Value::String("navy".into()))
        .unwrap();
    app.set_system_colors_win(true);
    let c = app.catalog();
    let form = SettingsForm::colors(app.settings_schema());
    let rows = form.rows(0, &app);
    let n = textweaver_app::COLOR_SETTINGS.len();
    assert!(rows.iter().all(|r| r.swatch.is_none()));
    for r in &rows[..n] {
        assert!(
            r.value_text.contains("not drawn"),
            "{}: {}",
            r.label,
            r.value_text
        );
        assert!(!r.value_text.contains(" to 1"), "{}", r.value_text);
    }
    let mut shapes = Vec::new();
    for (r, e) in rows[n..].iter().zip(app.highlight_palette()) {
        assert!(
            r.value_text.contains("high contrast colors are drawn"),
            "{}",
            r.value_text
        );
        shapes.push(e.shape);
    }
    let count = shapes.len();
    shapes.sort_by_key(|s| format!("{s:?}"));
    shapes.dedup();
    assert_eq!(shapes.len(), count, "the shapes still differ");
    let (h, _) = harness(&app);
    let grid = h.access_node(h.get_widget(FORM).id()).expect("the form");
    assert_eq!(grid.description(), Some(c.tr("gui-colors-help-system")));
}

/// The Colors dialog measures each color against the page the window
/// draws (a theme from the command line, or the system's contrast
/// colors), not the saved theme: navy links are low on Galaxy's dark page
/// and good on a white one.
#[test]
fn the_colors_dialog_reads_the_theme_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    app.set_setting("colors.links", serde_json::Value::String("navy".into()))
        .unwrap();
    let (saved, _) = app.color_contrast("colors.links").unwrap();
    assert!(saved < 3.0, "navy on Galaxy: {saved}");
    let light = Palette::named("galaxy-light");
    app.set_drawn_colors(Some((light.background, light.text)));
    let (drawn, word) = app.color_contrast("colors.links").unwrap();
    assert!(drawn > 4.5, "navy on Galaxy Light: {drawn}");
    let form = SettingsForm::colors(app.settings_schema());
    let row = form
        .settings_in(0)
        .iter()
        .position(|s| s.path == "colors.links")
        .unwrap();
    assert!(form.rows(0, &app)[row].value_text.contains(&word));
    // The reading ruler is a band in the window: its row measures the
    // text on it.
    app.set_drawn_colors(None);
    app.set_setting("colors.ruler", serde_json::Value::String("navy".into()))
        .unwrap();
    let (ruler, _) = app.color_contrast("colors.ruler").unwrap();
    assert!(ruler > 4.5, "Galaxy's text on a navy band: {ruler}");
}
