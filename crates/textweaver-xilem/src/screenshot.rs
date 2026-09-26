//! `--screenshot PATH`: the window drawn with Vello's CPU renderer, with no
//! window on screen, at any scale, for design review.
//!
//! It builds the same widget tree as the real window, in Masonry's test
//! harness, with the bundled fonts, and optionally shows a spoken-word
//! highlight or an open dialog.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use masonry::core::NewWidget;
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::core::{CharPos, Unit};
use textweaver_app::text::units::unit_at;

use crate::document::{DocState, DocumentView};
use crate::gui::{self, DOC, ROOT};
use crate::setup::{self, Options};
use crate::theme::{self, Palette};
use crate::widgets::Root;

/// What to draw.
#[derive(Clone, Debug)]
pub struct ShotOptions {
    /// Where to write the PNG.
    pub path: PathBuf,
    /// The document.
    pub file: Option<PathBuf>,
    /// Logical window size.
    pub size: (u32, u32),
    /// Scale factor (1.0 is 100%, 2.0 is 200%).
    pub scale: f64,
    /// Theme name (the saved theme otherwise).
    pub theme: Option<String>,
    /// Show a spoken word and sentence at this char.
    pub highlight_at: Option<usize>,
    /// Draw an open list dialog with these items.
    pub list: Option<(String, Vec<String>)>,
    /// Keep settings under this directory.
    pub home: Option<PathBuf>,
}

/// Draws the window into `opts.path`.
pub fn screenshot(opts: &ShotOptions) -> Result<(), String> {
    let app_opts = Options {
        no_speech: true,
        home: opts.home.clone(),
        ..Options::default()
    };
    let (mut app, _) = setup::build_app(&app_opts, Box::new(LogAnnouncer::default()));
    if let Some(file) = &opts.file {
        app.open(file)
            .map_err(|e| format!("cannot open {}: {e}", file.display()))?;
    }
    let palette = match &opts.theme {
        Some(name) => Palette::named(name),
        None => Palette::from_theme(app.current_theme()),
    };
    let font = crate::fonts::doc_font(&app.settings().reading_aids.font);
    let tree = gui::build_tree(&palette, font, Some(&app), Rc::new(Cell::new(0)), false);
    render(&app, tree.root, &palette, opts)
}

fn render(
    app: &textweaver_app::App,
    root: NewWidget<Root>,
    palette: &Palette,
    opts: &ShotOptions,
) -> Result<(), String> {
    let (w, h) = opts.size;
    let scale = opts.scale.clamp(0.5, 4.0);
    let mut params = TestHarnessParams::default();
    params.window_size = ((f64::from(w) * scale) as u32, (f64::from(h) * scale) as u32).into();
    params.scale_factor = scale;
    params.background_color = theme::color(palette.background);
    params.root_padding = 0;
    params.max_screenshot_size = u32::MAX;
    let mut harness = TestHarness::create_with(theme::default_properties(palette), root, params);
    for blob in crate::fonts::bundled_blobs() {
        harness.register_fonts(blob);
    }
    gui::refresh_for_tests(app, &mut harness);
    let doc_id = harness.get_widget(DOC).id();
    harness.focus_on(Some(doc_id));
    if let (Some(at), Some(s)) = (opts.highlight_at, app.session()) {
        let pos = CharPos(at.min(s.doc.len_chars()));
        let state = DocState {
            caret: pos,
            anchor: None,
            spoken: unit_at(&s.doc, pos, Unit::Word),
            sentence: unit_at(&s.doc, pos, Unit::Sentence),
            reading: true,
        };
        harness.edit_widget(DOC, |mut d| DocumentView::set_state(&mut d, state));
    }
    if let Some((title, items)) = &opts.list {
        let (modal, list_id) = gui::list_dialog(palette, title, items.clone(), 1);
        harness.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        harness.focus_on(Some(list_id));
    }
    harness
        .render()
        .save(&opts.path)
        .map_err(|e| format!("cannot write {}: {e}", opts.path.display()))
}

/// The screenshots for review: Galaxy, Galaxy Light, and high contrast at
/// 100% and 200%, a spoken word, and a list dialog. Returns the files.
pub fn review_set(dir: &Path, file: &Path) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let mut out = Vec::new();
    let base = ShotOptions {
        path: PathBuf::new(),
        file: Some(file.to_path_buf()),
        size: (1100, 780),
        scale: 1.0,
        theme: Some("galaxy".into()),
        highlight_at: Some(120),
        list: None,
        home: Some(dir.join("home")),
    };
    let shots = [
        ("galaxy-100.png", "galaxy", 1.0, false),
        ("galaxy-200.png", "galaxy", 2.0, false),
        ("galaxy-light-100.png", "galaxy-light", 1.0, false),
        ("high-contrast-100.png", "high-contrast", 1.0, false),
        ("galaxy-dialog-100.png", "galaxy", 1.0, true),
        ("galaxy-dialog-200.png", "galaxy", 2.0, true),
    ];
    for (name, theme, scale, list) in shots {
        let mut o = base.clone();
        o.path = dir.join(name);
        o.theme = Some(theme.into());
        o.scale = scale;
        if list {
            o.list = Some((
                "Bookmarks".into(),
                vec![
                    "Introduction, line 1".into(),
                    "Lists, line 9".into(),
                    "Emphasis, line 14".into(),
                    "Links, line 21".into(),
                    "Conclusion, line 38".into(),
                ],
            ));
        }
        screenshot(&o)?;
        out.push(o.path);
    }
    Ok(out)
}
