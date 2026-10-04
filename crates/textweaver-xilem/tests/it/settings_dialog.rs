//! The settings dialog, built from the app's schema, in Masonry's test
//! harness: what a screen reader finds, the keys, and changes reaching the
//! app.

use std::cell::Cell;
use std::rc::Rc;

use masonry::accesskit::{Action, ActionRequest, Role, Toggled, TreeId};
use masonry::core::TextEvent;
use masonry::core::keyboard::{Key, NamedKey};
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::{App, SettingKind};
use textweaver_xilem::gui::{self, FORM, ROOT, SECTIONS};
use textweaver_xilem::settings_dialog::{
    self, FormAction, FormChange, RowKind, SettingsForm, SettingsGrid,
};
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

/// The window with the settings dialog open on its first section.
fn harness_with_dialog(app: &App) -> (TestHarness<Root>, SettingsForm) {
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
    let form = SettingsForm::new(app.settings_schema());
    let d = gui::settings_dialog(&p, &form, app, 0, 0);
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(d.modal)));
    h.focus_on(Some(d.form));
    let _ = h.redraw();
    (h, form)
}

/// The setting a screen reader is on (the form's active descendant): its
/// number and its check state.
fn focused_setting(h: &TestHarness<Root>) -> (Option<f64>, Option<Toggled>) {
    let node = h
        .access_node(h.get_widget(FORM).id())
        .and_then(|g| g.active_descendant())
        .expect("the form has an active descendant");
    (node.numeric_value(), node.toggled())
}

#[test]
fn every_visible_setting_is_in_one_section() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let schema = app.settings_schema();
    let form = SettingsForm::new(schema.clone());
    assert!(form.sections.len() > 3, "{:?}", form.sections);
    let shown: usize = (0..form.sections.len())
        .map(|i| form.settings_in(i).len())
        .sum();
    // Every setting that does something in the window; none of the
    // terminal's own.
    assert_eq!(shown, schema.in_window().count());
    assert_eq!(
        schema.visible().count() - shown,
        textweaver_app::TERMINAL_ONLY.len()
    );
    let rate = form.find("speech.rate").expect("the rate is in the form");
    assert_eq!(
        form.setting(rate.0, rate.1).map(|s| s.path.as_str()),
        Some("speech.rate")
    );
}

/// Settings below the fold, and sections below the list's edge, stay in
/// the tree a screen reader gets: AccessKit's own filter (the one the
/// platform adapters use) keeps every row. Before W4s the UI Automation
/// report found 13 of 15 sections.
#[test]
fn settings_and_sections_below_the_fold_stay_in_the_tree() {
    use accesskit_consumer::common_filter;
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let (h, _form) = harness_with_dialog(&app);
    for (name, id) in [
        ("form", h.get_widget(FORM).id()),
        ("sections", h.get_widget(SECTIONS).id()),
    ] {
        let node = h.access_node(id).unwrap();
        let all = node.children().count();
        let kept = node.filtered_children(common_filter).count();
        assert!(all > 12, "the {name} has more rows than fit: {all}");
        assert_eq!(
            kept, all,
            "AccessKit's filter keeps every row of the {name}"
        );
        assert!(!node.clips_children(), "the {name} claims to clip");
        // "1 of 20": zero-based positions, the size on the container.
        let first = node.children().next().unwrap();
        assert_eq!(first.position_in_set(), Some(0), "the {name}");
        assert_eq!(
            first.size_of_set_from_container(&common_filter),
            Some(all),
            "the {name}"
        );
    }
}

/// Settings scrolled into view are drawn with their text, not blank.
#[test]
fn settings_scrolled_into_view_are_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let (mut h, _form) = harness_with_dialog(&app);
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::End)));
    let _ = h.redraw();
    let grid = h.access_node(h.get_widget(FORM).id()).unwrap();
    let n = grid.children().count();
    let row = grid
        .children()
        .nth(n - 2)
        .and_then(|o| o.bounding_box())
        .expect("the row before the last has bounds");
    let img = h.render();
    let mut colors = std::collections::HashSet::new();
    for y in row.y0.ceil() as u32..row.y1.floor() as u32 {
        for x in row.x0.ceil() as u32..(row.x0 + 160.0) as u32 {
            if x < img.width() && y < img.height() {
                colors.insert(img.get_pixel(x, y).0);
            }
        }
    }
    assert!(
        colors.len() > 4,
        "the setting before the last is drawn with its text after End ({} colors)",
        colors.len()
    );
}

#[test]
fn the_dialog_names_its_sections_and_settings_with_their_roles() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let (h, form) = harness_with_dialog(&app);
    let tree = h.access_tree().state();
    let mut stack = vec![tree.root()];
    let (mut dialog, mut sections, mut group) = (None, None, None);
    while let Some(n) = stack.pop() {
        match n.role() {
            Role::Dialog => dialog = n.label(),
            Role::ListBox => sections = n.label(),
            Role::Group => group = Some(n.id()),
            _ => {}
        }
        stack.extend(n.children());
    }
    assert_eq!(dialog.as_deref(), Some("Settings"));
    assert_eq!(sections.as_deref(), Some("Sections"));
    let group = tree.node_by_id(group.expect("the form")).unwrap();
    assert_eq!(
        group.label().as_deref(),
        Some(format!("{} settings", form.sections[0]).as_str())
    );
    // One node per setting, each with the role its kind needs, a name, and
    // (for numbers) the range.
    let settings = form.settings_in(0);
    let rows: Vec<_> = group.children().collect();
    assert_eq!(rows.len(), settings.len());
    for (node, s) in rows.iter().zip(&settings) {
        assert_eq!(node.label().as_deref(), Some(s.label), "{}", s.path);
        let want = match s.kind {
            SettingKind::Toggle => Role::CheckBox,
            SettingKind::Number { .. } => Role::Slider,
            SettingKind::Choice { .. } => Role::ComboBox,
            SettingKind::Text { .. } | SettingKind::List => Role::TextInput,
            SettingKind::Table => Role::Label,
        };
        assert_eq!(node.role(), want, "{}", s.path);
        if let SettingKind::Number { min, max, .. } = s.kind {
            assert_eq!(node.min_numeric_value(), Some(min), "{}", s.path);
            assert_eq!(node.max_numeric_value(), Some(max), "{}", s.path);
            assert!(node.numeric_value().is_some(), "{}", s.path);
        }
        if let SettingKind::Toggle = s.kind {
            assert!(node.toggled().is_some(), "{}", s.path);
        }
    }
    // The focus is the form's first setting (its active descendant).
    let focus = group.active_descendant().expect("an active descendant");
    assert_eq!(focus.label().as_deref(), Some(settings[0].label));
}

#[test]
fn keys_move_and_change_settings_and_the_app_takes_the_change() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let (mut h, form) = harness_with_dialog(&app);
    let (section, row) = form.find("speech.rate").unwrap();
    assert_eq!(section, 0, "the rate is in the first section");
    // Down to the rate, then Right.
    for _ in 0..row {
        h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowDown)));
    }
    assert_eq!(h.get_widget(FORM).inner().selected(), row);
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowRight)));
    let (action, _) = h.pop_action::<FormAction>().expect("a change");
    assert_eq!(
        action,
        FormAction::Change {
            row,
            change: FormChange::Step(true)
        }
    );
    let before = app.setting_value("speech.rate").unwrap().as_f64().unwrap();
    let setting = form.setting(section, row).unwrap().clone();
    let said = settings_dialog::apply(&mut app, &setting, FormChange::Step(true)).unwrap();
    let after = app.setting_value("speech.rate").unwrap().as_f64().unwrap();
    assert!(after > before, "{before} -> {after}");
    assert!(said.starts_with("Rate, "), "{said}");
    // The form shows the new value; nothing extra to announce.
    let rows = form.rows(section, &app);
    assert_eq!(settings_dialog::extra_note(&said, &rows[row]), None);
    h.edit_widget(FORM, |mut g| {
        SettingsGrid::update_rows(&mut g, rows.clone())
    });
    let _ = h.redraw();
    assert_eq!(focused_setting(&h).0, Some(after));
    // A typed value out of range is refused, saying the range; the value
    // stays.
    let why =
        settings_dialog::apply(&mut app, &setting, FormChange::Text("100000".into())).unwrap_err();
    assert!(why.contains("outside"), "{why}");
    assert_eq!(
        app.setting_value("speech.rate").unwrap().as_f64(),
        Some(after)
    );
    // Delete asks for the default.
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Delete)));
    let (action, _) = h.pop_action::<FormAction>().expect("a reset");
    assert_eq!(
        action,
        FormAction::Change {
            row,
            change: FormChange::Reset
        }
    );
    settings_dialog::apply(&mut app, &setting, FormChange::Reset).unwrap();
    assert_eq!(
        app.setting_value("speech.rate"),
        Some(setting.default.clone())
    );
}

#[test]
fn a_switch_toggles_and_enter_types_a_number() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let (mut h, form) = harness_with_dialog(&app);
    let rows = form.rows(0, &app);
    let toggle = rows
        .iter()
        .position(|r| matches!(r.kind, RowKind::Toggle(_)))
        .expect("a switch in the first section");
    let number = rows
        .iter()
        .position(|r| matches!(r.kind, RowKind::Number { .. }))
        .expect("a number in the first section");
    h.edit_widget(FORM, |mut g| {
        SettingsGrid::set_section(&mut g, form.sections[0], rows.clone(), toggle);
    });
    h.process_text_event(TextEvent::key_down(Key::Character(" ".into())));
    let (action, _) = h.pop_action::<FormAction>().expect("Space toggles");
    assert_eq!(
        action,
        FormAction::Change {
            row: toggle,
            change: FormChange::Step(true)
        }
    );
    let setting = form.setting(0, toggle).unwrap().clone();
    let was = app.setting_value(&setting.path).unwrap().as_bool().unwrap();
    settings_dialog::apply(&mut app, &setting, FormChange::Step(true)).unwrap();
    assert_eq!(
        app.setting_value(&setting.path).unwrap().as_bool(),
        Some(!was)
    );
    let rows = form.rows(0, &app);
    h.edit_widget(FORM, |mut g| {
        SettingsGrid::update_rows(&mut g, rows.clone())
    });
    let _ = h.redraw();
    let want = if was { Toggled::False } else { Toggled::True };
    assert_eq!(focused_setting(&h).1, Some(want));
    // Enter on a number asks for it, starting with the current value.
    h.edit_widget(FORM, |mut g| {
        SettingsGrid::set_section(&mut g, form.sections[0], rows.clone(), number);
    });
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Enter)));
    let (action, _) = h.pop_action::<FormAction>().expect("Enter edits");
    let FormAction::Edit { row, text } = action else {
        panic!("not an edit: {action:?}");
    };
    assert_eq!(row, number);
    assert!(text.parse::<f64>().is_ok(), "{text:?}");
    // Control Page Down asks for the next section.
    let mut k = masonry::core::KeyboardEvent {
        state: masonry::core::keyboard::KeyState::Down,
        key: Key::Named(NamedKey::PageDown),
        ..Default::default()
    };
    k.modifiers = masonry::core::Modifiers::CONTROL;
    h.process_text_event(TextEvent::Keyboard(k));
    let (action, _) = h.pop_action::<FormAction>().expect("next section");
    assert_eq!(action, FormAction::Section(1));
}

#[test]
fn screen_reader_actions_on_a_setting_reach_the_form() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let (mut h, form) = harness_with_dialog(&app);
    let (_, row) = form.find("speech.rate").unwrap();
    let group = h.access_node(h.get_widget(FORM).id()).unwrap();
    let node = group
        .children()
        .nth(row)
        .expect("the rate's node")
        .locate()
        .0;
    // Increment on the setting's own node (not a widget) reaches the form,
    // which asks for a step on that row.
    h.process_access_event(ActionRequest {
        action: Action::Increment,
        target_tree: TreeId::ROOT,
        target_node: node,
        data: None,
    });
    let (action, _) = h.pop_action::<FormAction>().expect("routed to the form");
    assert_eq!(
        action,
        FormAction::Change {
            row,
            change: FormChange::Step(true)
        }
    );
    // Focus on a section's option moves the section list there.
    let sections = h.access_node(h.get_widget(SECTIONS).id()).unwrap();
    let second = sections
        .children()
        .nth(1)
        .expect("a second section")
        .locate()
        .0;
    h.process_access_event(ActionRequest {
        action: Action::Focus,
        target_tree: TreeId::ROOT,
        target_node: second,
        data: None,
    });
    assert_eq!(h.get_widget(SECTIONS).inner().selected(), 1);
}

/// In Spanish (`[interface] language = "es"`), the window's drawn labels,
/// the settings dialog's sections, labels, and values are Spanish, and a
/// change is announced once: the form shows the new value, so the app's
/// "Velocidad, ..." adds nothing to say (W4d's catalog, W5a4).
#[test]
fn the_window_and_the_settings_dialog_speak_spanish() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    app.set_setting("interface.language", serde_json::json!("es"))
        .expect("Spanish");
    assert_eq!(app.catalog().lang(), "es");
    let (mut h, form) = harness_with_dialog(&app);
    let mut buttons = Vec::new();
    let mut toolbars = Vec::new();
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        stack.extend(n.children());
        match n.role() {
            Role::Button => buttons.push(n.label().unwrap_or_default()),
            Role::Toolbar => toolbars.push(n.label().unwrap_or_default()),
            _ => {}
        }
    }
    for name in ["Abrir", "Reproducir", "Detener", "Más rápido", "Cerrar"] {
        assert!(buttons.iter().any(|b| b == name), "{name}: {buttons:?}");
    }
    assert!(toolbars.iter().any(|t| t == "Lectura"), "{toolbars:?}");
    let c = app.catalog();
    let items = form.section_items(&c);
    assert!(items[0].starts_with("Voz, "), "{items:?}");
    assert!(items[0].ends_with(" ajustes"), "{items:?}");
    assert_eq!(form.form_label(0, &c), "Configuración: Voz");
    let (section, row) = form.find("speech.rate").unwrap();
    let rows = form.rows(section, &app);
    assert_eq!(rows[row].label, "Velocidad");
    let setting = form.setting(section, row).unwrap().clone();
    let said = settings_dialog::apply(&mut app, &setting, FormChange::Step(true)).unwrap();
    assert!(said.starts_with("Velocidad, "), "{said}");
    let rows = form.rows(section, &app);
    assert_eq!(
        settings_dialog::extra_note(&said, &rows[row]),
        None,
        "{said}"
    );
    h.edit_widget(FORM, |mut g| SettingsGrid::update_rows(&mut g, rows));
    let _ = h.redraw();
    let why =
        settings_dialog::apply(&mut app, &setting, FormChange::Text("100000".into())).unwrap_err();
    assert!(why.contains("fuera de"), "{why}");
}

/// The dialog starts on a section's first setting that is not a table, so
/// a screen reader does not start on "4 entries" (CI's NVDA run, Wave 5).
#[test]
fn the_form_starts_on_a_plain_setting() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let form = SettingsForm::new(app.settings_schema());
    for section in 0..form.sections.len() {
        let row = form.first_plain_row(section);
        let settings = form.settings_in(section);
        if settings
            .iter()
            .any(|s| !matches!(s.kind, SettingKind::Table))
        {
            assert!(
                !matches!(settings[row].kind, SettingKind::Table),
                "section {section} starts on a table"
            );
        }
    }
}

/// The chevrons beside a number or a choice do what they show: a click on
/// the left one steps back, on the right one forward (GUI audit QW8,
/// W8c-w). A switch's value is in words beside it.
#[test]
fn the_chevrons_step_the_way_they_point() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let (mut h, _) = harness_with_dialog(&app);
    let rows = h.get_widget(FORM).inner().rows().to_vec();
    let row = rows
        .iter()
        .take(4)
        .position(|r| matches!(r.kind, RowKind::Number { .. } | RowKind::Choice))
        .expect("a number or a choice near the top of the first section");
    for t in rows.iter().filter(|r| matches!(r.kind, RowKind::Toggle(_))) {
        assert!(
            !t.value_text.is_empty(),
            "{}: a switch says its value",
            t.label
        );
    }
    let box_ = h.get_widget(FORM).ctx().bounding_box();
    // The x ranges a click steps back and forward in.
    let (back, forward): (Vec<f64>, Vec<f64>) = {
        let grid = h.get_widget(FORM).inner();
        let xs = (0..(box_.width() as usize)).map(|x| x as f64);
        (
            xs.clone()
                .filter(|&x| grid.chevron_step(row, x) == Some(false))
                .collect(),
            xs.filter(|&x| grid.chevron_step(row, x) == Some(true))
                .collect(),
        )
    };
    assert!(!back.is_empty() && !forward.is_empty());
    assert!(back[back.len() - 1] < forward[0], "back is left of forward");
    let y = box_.y0 + 46.0 * row as f64 + 23.0;
    for (xs, forward) in [(back, false), (forward, true)] {
        let x = box_.x0 + xs[xs.len() / 2];
        h.mouse_move((x, y));
        h.mouse_button_press(None);
        h.mouse_button_release(None);
        let mut got = None;
        while let Some((a, _)) = h.pop_action::<FormAction>() {
            got = Some(a);
        }
        assert_eq!(
            got,
            Some(FormAction::Change {
                row,
                change: FormChange::Step(forward),
            }),
            "a click at {x} on row {row}"
        );
    }
}

/// Typing in the form filters every section's settings (W9b-d): the
/// first section becomes the matches, the count is known, and the keys
/// reach the driver as filter actions; F1 asks for the focused help.
#[test]
fn typing_filters_the_settings_and_f1_asks_for_help() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let c = app.catalog();
    let mut form = SettingsForm::for_window(&app);
    form.set_filter("rate", &c);
    assert!(form.special_section(0));
    assert_eq!(form.section_title(0, &c), "Matching rate");
    let paths: Vec<&str> = form
        .settings_in(0)
        .iter()
        .map(|s| s.path.as_str())
        .collect();
    assert!(paths.contains(&"speech.rate"), "{paths:?}");
    assert_eq!(form.match_count(), paths.len());
    // Terminal-only settings never match in the window.
    form.set_filter("scroll margin", &c);
    assert_eq!(form.match_count(), 0);
    assert!(form.find("display.wrap_width").is_none());
    assert!(app.settings_schema().get("display.wrap_width").is_some());
    // Cleared: the schema's sections again.
    form.set_filter("", &c);
    assert!(!form.special_section(0));
    // The keys in the form.
    let (mut h, _form) = harness_with_dialog(&app);
    h.process_text_event(TextEvent::key_down(Key::Character("r".into())));
    let (a, _) = h.pop_action::<FormAction>().expect("a filter key");
    assert_eq!(a, FormAction::Type("r".into()));
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Backspace)));
    let (a, _) = h.pop_action::<FormAction>().expect("Backspace");
    assert_eq!(a, FormAction::Backspace);
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::F1)));
    let (a, _) = h.pop_action::<FormAction>().expect("F1");
    assert_eq!(a, FormAction::Help(0));
}

/// The settings changed last come first, so the dialog opens on the
/// last-changed setting (W9b-d).
#[test]
fn the_dialog_opens_on_the_last_changed_setting() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    assert!(!SettingsForm::for_window(&app).special_section(0));
    let rate = app.settings_schema().get("speech.rate").unwrap().clone();
    settings_dialog::apply(&mut app, &rate, FormChange::Step(true)).unwrap();
    let form = SettingsForm::for_window(&app);
    let c = app.catalog();
    assert_eq!(form.section_title(0, &c), "Recently changed");
    assert_eq!(
        form.setting(0, 0).map(|s| s.path.as_str()),
        Some("speech.rate")
    );
}
