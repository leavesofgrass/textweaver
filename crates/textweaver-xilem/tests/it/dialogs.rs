//! The dialog contract (W9b-d): every kind of dialog the window builds
//! has a Close (a mouse user can always leave it), Escape closes it, the
//! focus goes back where it was, and nothing leaves a small window.

use std::cell::Cell;
use std::rc::Rc;

use masonry::accesskit::Role;
use masonry::core::keyboard::{Key, NamedKey};
use masonry::core::{NewWidget, TextEvent, Widget, WidgetId};
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::App;
use textweaver_app::a11y::LogAnnouncer;
use textweaver_xilem::dialog::DialogAction;
use textweaver_xilem::gui;
use textweaver_xilem::settings_dialog::SettingsForm;
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

/// The window at `size` (logical pixels) and `scale`.
fn window(app: &App, size: (u32, u32), scale: f64) -> TestHarness<Root> {
    let p = Palette::galaxy();
    let tree = gui::build_tree(
        &p,
        Default::default(),
        Some(app),
        Rc::new(Cell::new(0)),
        Default::default(),
    );
    let mut params = TestHarnessParams::default();
    params.window_size = (
        (f64::from(size.0) * scale) as u32,
        (f64::from(size.1) * scale) as u32,
    )
        .into();
    params.scale_factor = scale;
    let mut h = TestHarness::create_with(theme::default_properties(&p), tree.root, params);
    gui::refresh_for_tests(app, &mut h);
    let _ = h.redraw();
    h
}

/// Every kind of dialog, by name, with the widget that takes the focus.
const KINDS: [&str; 8] = [
    "list", "prompt", "palette", "question", "settings", "colors", "reading", "voices",
];

fn build(kind: &str, app: &App) -> (NewWidget<dyn Widget>, WidgetId) {
    build_for(kind, app, 780)
}

/// [`build`] for a window `height` logical pixels high (the voice manager
/// is compact in a short one).
fn build_for(kind: &str, app: &App, height: u32) -> (NewWidget<dyn Widget>, WidgetId) {
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
            use textweaver_xilem::voices;
            let short = f64::from(height) < voices::SHORT_HEIGHT;
            let d = voices::voice_dialog_fit(&p, app, "Voices", items, 0, short);
            (d.modal, d.list)
        }
        _ => unreachable!("{kind}"),
    }
}

/// The names of the buttons inside the open dialog.
fn dialog_buttons(h: &TestHarness<Root>) -> Vec<String> {
    let mut stack = vec![h.access_tree().state().root()];
    let mut dialog = None;
    while let Some(n) = stack.pop() {
        if n.role() == Role::Dialog {
            dialog = Some(n);
            break;
        }
        stack.extend(n.children());
    }
    let mut names = Vec::new();
    let mut stack = vec![dialog.expect("a dialog is open")];
    while let Some(n) = stack.pop() {
        if n.role() == Role::Button {
            names.push(n.label().unwrap_or_default());
        }
        stack.extend(n.children());
    }
    names
}

#[test]
fn every_dialog_has_a_close_escape_closes_and_focus_returns() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    for kind in KINDS {
        let mut h = window(&app, (1100, 780), 1.0);
        // The focus starts on Play, as after a click on it.
        let play = h.get_widget(gui::PLAY).id();
        h.focus_on(Some(play));
        let (modal, focus) = build(kind, &app);
        let mut back = None;
        gui::open_dialog_in(&mut h, modal, focus, &mut back);
        let _ = h.redraw();
        assert_eq!(h.focused_widget_id(), Some(focus), "{kind}: focus moves in");
        // A Close, or for a question its No, which closes it.
        let buttons = dialog_buttons(&h);
        let leave = if kind == "question" { "No" } else { "Close" };
        assert!(
            buttons.iter().any(|b| b == leave),
            "{kind}: no {leave} in {buttons:?}"
        );
        // Escape closes: the dialog says Cancel (an app list hands Escape
        // to the app's list model, which closes it).
        h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Escape)));
        let mut closed = false;
        while let Some((a, _)) = h.pop_action_erased() {
            // A prompt's field says Cancelled, which the driver takes as
            // Escape.
            closed |= match a.downcast::<DialogAction>() {
                Ok(a) => matches!(
                    *a,
                    DialogAction::Cancel | DialogAction::Key(textweaver_app::ListKey::Escape)
                ),
                Err(a) => a
                    .downcast::<masonry::widgets::TextAction>()
                    .is_ok_and(|t| matches!(*t, masonry::widgets::TextAction::Cancelled)),
            };
        }
        assert!(closed, "{kind}: Escape does not close it");
        // Closing puts the focus back where it was.
        gui::close_dialog_in(&mut h, &mut back);
        let _ = h.redraw();
        assert_eq!(
            h.focused_widget_id(),
            Some(play),
            "{kind}: the focus goes back to Play"
        );
    }
}

/// View, Reading settings: the reading settings in one form, the rate
/// first, then Voices, the two spacing presets, and Close.
#[test]
fn the_reading_form_lists_the_reading_settings_and_its_buttons() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let form = SettingsForm::reading(app.settings_schema());
    assert!(form.is_reading());
    let rows = form.rows(0, &app);
    assert_eq!(rows.len(), textweaver_app::READING_SETTINGS.len());
    assert_eq!(rows[0].label, "Rate");
    let mut h = window(&app, (1100, 780), 1.0);
    let (modal, focus) = build("reading", &app);
    let mut back = None;
    gui::open_dialog_in(&mut h, modal, focus, &mut back);
    let _ = h.redraw();
    let mut buttons = dialog_buttons(&h);
    buttons.sort();
    assert_eq!(
        buttons,
        ["Close", "Generous spacing", "Voices", "WCAG spacing"],
        "{buttons:?}"
    );
}

/// The dialog half of the bounds test (W9b-n's
/// `no_control_leaves_the_window_at_any_review_size`): at the smallest
/// window and a 1366 by 768 laptop at 200 percent, every button of every
/// dialog is inside the window and at least 24 by 24.
#[test]
fn no_dialog_control_leaves_a_small_window() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    for (size, scale) in [((420, 320), 1.0), ((683, 384), 2.0)] {
        for kind in KINDS {
            let mut h = window(&app, size, scale);
            let (modal, focus) = build_for(kind, &app, size.1);
            let mut back = None;
            gui::open_dialog_in(&mut h, modal, focus, &mut back);
            let _ = h.redraw();
            let _ = h.redraw();
            let at = format!("{kind} at {}x{} at {}%", size.0, size.1, scale * 100.0);
            let window = masonry::kurbo::Rect::new(
                0.0,
                0.0,
                f64::from(size.0) * scale,
                f64::from(size.1) * scale,
            );
            let mut stack = vec![h.access_tree().state().root()];
            let mut seen = 0;
            while let Some(n) = stack.pop() {
                // A list's options and a form's settings scroll inside
                // it; the list or form itself must fit.
                if !matches!(n.role(), Role::ListBox | Role::Group) {
                    stack.extend(n.children());
                }
                if !matches!(
                    n.role(),
                    Role::Button | Role::ListBox | Role::TextInput | Role::Group
                ) {
                    continue;
                }
                let Some(b) = n.bounding_box() else {
                    continue;
                };
                seen += 1;
                let name = n.label().unwrap_or_default();
                assert!(
                    b.x0 >= window.x0 - 0.5
                        && b.y0 >= window.y0 - 0.5
                        && b.x1 <= window.x1 + 0.5
                        && b.y1 <= window.y1 + 0.5,
                    "{at}: {name:?} at {b:?} leaves the window {window:?}"
                );
                if n.role() == Role::Button {
                    assert!(
                        b.width() >= 24.0 * scale - 0.5 && b.height() >= 24.0 * scale - 0.5,
                        "{at}: {name:?} is smaller than 24 by 24: {b:?}"
                    );
                }
            }
            assert!(seen > 0, "{at}: no controls found");
        }
    }
}

/// The title a reader hears for each kind, as [`build`] opens it; the
/// settings dialog takes its title from the catalog, and the colors and
/// voice dialogs are only checked for a name.
fn title_of(kind: &str, app: &App) -> String {
    match kind {
        "list" => "Bookmarks".into(),
        "prompt" => "Go to line".into(),
        "palette" => "Commands".into(),
        "question" => "Remove the voice?".into(),
        "settings" => app.catalog().tr("settings-title"),
        "colors" | "voices" | "reading" => String::new(),
        _ => unreachable!("{kind}"),
    }
}

/// True when the node of `id`, or a parent of it, is hidden from readers.
fn hidden_from_readers(h: &TestHarness<Root>, id: WidgetId) -> bool {
    let mut n = h.access_node(id);
    while let Some(node) = n {
        if node.is_hidden() {
            return true;
        }
        n = node.parent();
    }
    false
}

/// The rest of the dialog contract (checklist row C1, W9e-c): every kind
/// is one modal dialog, named by its title or question; while it is open,
/// the window behind it is hidden from readers and disabled, and after it
/// closes the window is back.
#[test]
fn every_dialog_is_modal_named_and_hides_the_window_behind() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    for kind in KINDS {
        let mut h = window(&app, (1100, 780), 1.0);
        let play = h.get_widget(gui::PLAY).id();
        let (modal, focus) = build(kind, &app);
        let mut back = None;
        gui::open_dialog_in(&mut h, modal, focus, &mut back);
        let _ = h.redraw();
        let mut dialogs = Vec::new();
        let mut stack = vec![h.access_tree().state().root()];
        while let Some(n) = stack.pop() {
            stack.extend(n.children());
            if n.role() == Role::Dialog {
                dialogs.push((n.label().unwrap_or_default(), n.is_modal()));
            }
        }
        assert_eq!(dialogs.len(), 1, "{kind}: one dialog: {dialogs:?}");
        let (name, modal) = &dialogs[0];
        assert!(*modal, "{kind}: the dialog is modal");
        assert!(!name.trim().is_empty(), "{kind}: the dialog has a name");
        let title = title_of(kind, &app);
        assert!(
            name.starts_with(&title),
            "{kind}: named {name:?}, not by its title {title:?}"
        );
        // The window behind: Play is disabled and hidden from readers.
        assert!(
            h.get_widget(gui::PLAY).ctx().is_disabled(),
            "{kind}: Play is still enabled behind the dialog"
        );
        assert!(
            hidden_from_readers(&h, play),
            "{kind}: the window behind is not hidden"
        );
        gui::close_dialog_in(&mut h, &mut back);
        let _ = h.redraw();
        assert!(
            !hidden_from_readers(&h, play),
            "{kind}: the window stays hidden after closing"
        );
        assert!(
            !h.get_widget(gui::PLAY).ctx().is_disabled(),
            "{kind}: Play stays disabled after closing"
        );
    }
}

/// Every action button under `w`.
fn action_buttons<'a>(
    w: masonry::core::WidgetRef<'a, dyn Widget>,
    out: &mut Vec<masonry::core::WidgetRef<'a, textweaver_xilem::widgets::ActionButton>>,
) {
    if let Some(b) = w.downcast::<textweaver_xilem::widgets::ActionButton>() {
        out.push(b);
    }
    for c in w.children() {
        action_buttons(c, out);
    }
}

/// Label in name inside every dialog (WCAG 2.5.3, checklist row C4):
/// each button's text on screen starts with its accessible name, the name
/// holds no key ("Close", not "Close (Escape)"), and the tree carries the
/// same name.
#[test]
fn every_dialog_button_shows_its_name_and_keeps_its_key_out_of_it() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    for kind in KINDS {
        let mut h = window(&app, (1100, 780), 1.0);
        let (modal, focus) = build(kind, &app);
        let mut back = None;
        gui::open_dialog_in(&mut h, modal, focus, &mut back);
        let _ = h.redraw();
        let mut found = Vec::new();
        action_buttons(h.root_widget().as_dyn(), &mut found);
        // The header's five and the toolbar's six, then the dialog's own.
        assert!(found.len() > 11, "{kind}: no dialog buttons");
        for b in found {
            let w = b.inner();
            let name = w.name();
            let shown = w.shown_text();
            let key = w.shortcut();
            assert!(
                shown.starts_with(&name),
                "{kind}: shows {shown:?}, named {name:?}"
            );
            // A one-letter answer key ("Y" for Yes) is the name's own letter.
            if key.chars().count() > 1 {
                assert!(
                    !name.contains(key),
                    "{kind}: the key {key:?} is inside the name {name:?}"
                );
            }
            if let Some(node) = h.access_node(b.id()) {
                assert_eq!(
                    node.label().unwrap_or_default(),
                    name,
                    "{kind}: the tree's name differs from the button's"
                );
            }
        }
    }
}

/// The voice manager at the smallest window, 420 by 320, in its compact
/// layout: every control stays inside the window (checklist row V3).
#[test]
fn the_voice_manager_fits_the_smallest_window() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path());
    let mut h = window(&app, (420, 320), 1.0);
    // Built for the window's height, as the window does: compact below 480.
    let (modal, focus) = build_for("voices", &app, 320);
    let mut back = None;
    gui::open_dialog_in(&mut h, modal, focus, &mut back);
    let _ = h.redraw();
    let _ = h.redraw();
    let window = masonry::kurbo::Rect::new(0.0, 0.0, 420.0, 320.0);
    let mut stack = vec![h.access_tree().state().root()];
    let mut outside = Vec::new();
    while let Some(n) = stack.pop() {
        if !matches!(n.role(), Role::ListBox | Role::Group) {
            stack.extend(n.children());
        }
        if !matches!(n.role(), Role::Button | Role::ListBox | Role::TextInput) {
            continue;
        }
        let Some(b) = n.bounding_box() else {
            continue;
        };
        if b.x0 < -0.5 || b.y0 < -0.5 || b.x1 > window.x1 + 0.5 || b.y1 > window.y1 + 0.5 {
            outside.push(format!("{:?} at {b:?}", n.label().unwrap_or_default()));
        }
    }
    assert!(outside.is_empty(), "outside the window: {outside:#?}");
}
