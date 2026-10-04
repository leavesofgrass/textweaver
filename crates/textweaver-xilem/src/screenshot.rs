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
use textweaver_app::store::GuiSidebar;
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
    /// Draw the settings dialog, on the speech rate.
    pub settings: bool,
    /// Keep settings under this directory.
    pub home: Option<PathBuf>,
    /// Turn the reading aids on first (bionic reading, difficult words,
    /// the ruler with its band, WCAG's text spacing, and RSVP). They are
    /// saved like any setting, so this needs `home`.
    pub aids: bool,
    /// Draw the Colors dialog (W6a6), with a few colors chosen so their
    /// samples and contrast show. Saved like any setting, so this needs
    /// `home`.
    pub colors: bool,
    /// Draw the voice manager (W7v) with a few sample voices of several
    /// engines (no engine is started for it).
    pub voices: bool,
    /// Start editing first, as the Edit button does (W8c-x).
    pub edit: bool,
    /// Show this panel beside the document (W8c-x). The Notes panel gets
    /// two sample notes. Saved like any setting, so this needs `home`.
    pub panel: Option<GuiSidebar>,
    /// Turn on only the reading ruler with its band, so the band's bar
    /// shows on its own (W8c-x). Saved like any setting, so this needs
    /// `home`.
    pub ruler: bool,
    /// Draw the Reading settings dialog (W9b-d) instead of Settings.
    pub reading: bool,
    /// With `settings`: this filter typed in the form (W9b-d), so the first
    /// section is the settings it matches.
    pub settings_filter: Option<String>,
}

/// A few colors chosen for the Colors dialog's review: the blue and
/// orange pair first, and one that is hard to see, to show the warning.
fn choose_review_colors(app: &mut textweaver_app::App) -> Result<(), String> {
    for (path, value) in [
        ("highlight.color", "blue"),
        ("highlight.sentence_color", "orange"),
        ("colors.links", "skyblue"),
        ("colors.headings", "gold"),
        ("colors.difficult_words", "navy"),
    ] {
        app.set_setting(path, serde_json::json!(value))?;
    }
    Ok(())
}

/// Every reading aid on, for review: bionic reading, difficult words, the
/// ruler band, WCAG 1.4.12's text spacing, and RSVP on the cursor's word.
pub(crate) fn turn_on_aids(app: &mut textweaver_app::App) -> Result<(), String> {
    use textweaver_app::Command;
    use textweaver_app::keymap::ActionId;
    use textweaver_app::store::reading_aids::{RulerMode, TextSpacing};
    app.update_settings(|s| {
        let a = &mut s.reading_aids;
        a.bionic = true;
        a.difficult_words = true;
        a.syllables = true;
        a.ruler.mode = RulerMode::Ruler;
        a.spacing = TextSpacing {
            line_height: 1.5,
            paragraph_spacing: 2.0,
            letter_spacing: 0.12,
            word_spacing: 0.16,
        };
    })
    .map_err(|e| format!("cannot save the reading aids: {e}"))?;
    let _ = app.dispatch(Command::Action(ActionId::RsvpToggle));
    Ok(())
}

/// Two sample notes for the Notes panel's review: one on the table's
/// heading and one on the block quote.
fn add_review_notes(app: &mut textweaver_app::App) -> Result<(), String> {
    use textweaver_app::{Command, NoteCommand, Panel};
    // The notes are saved, so a second shot in the same folder finds them.
    if !app.panel_entries(Panel::Notes).is_empty() {
        return Ok(());
    }
    let text = app
        .session()
        .map(|s| s.doc.text().to_string())
        .ok_or("the Notes panel screenshot needs a document")?;
    for (needle, note) in [
        ("A Table", "Check the totals in the score column"),
        ("A block quote", "Ask about the source of this quote"),
    ] {
        let Some(byte) = text.find(needle) else {
            continue;
        };
        let at = CharPos(text[..byte].chars().count());
        let _ = app.dispatch(Command::SetCursor(at));
        let _ = app.dispatch(Command::Notes(NoteCommand::Add));
        let _ = app.dispatch(Command::Answer(note.into()));
    }
    let _ = app.dispatch(Command::SetCursor(CharPos(0)));
    Ok(())
}

/// Draws the window into `opts.path`.
pub fn screenshot(opts: &ShotOptions) -> Result<(), String> {
    harness(opts)?
        .render()
        .save(&opts.path)
        .map_err(|e| format!("cannot write {}: {e}", opts.path.display()))
}

/// The window as [`screenshot`] draws it, in Masonry's test harness, ready
/// to render.
fn harness(opts: &ShotOptions) -> Result<TestHarness<Root>, String> {
    let app_opts = Options {
        no_speech: true,
        home: opts.home.clone(),
        ..Options::default()
    };
    if (opts.aids || opts.colors || opts.ruler || opts.panel.is_some()) && opts.home.is_none() {
        return Err("the reading aids, ruler, colors and panel screenshots need a home folder, so their settings are not saved into yours".into());
    }
    let (mut app, _) = setup::build_app(&app_opts, Box::new(LogAnnouncer::default()));
    if let Some(file) = &opts.file {
        app.open(file)
            .map_err(|e| format!("cannot open {}: {e}", file.display()))?;
    }
    if opts.highlight_at.is_some() && opts.home.is_some() {
        // The review draws the sentence's underline too: `[highlight]
        // granularity = "both"` (the default, "word", draws the word only).
        app.set_setting("highlight.granularity", serde_json::json!("both"))?;
    }
    if opts.aids {
        turn_on_aids(&mut app)?;
    }
    if opts.colors {
        choose_review_colors(&mut app)?;
    }
    if opts.ruler {
        use textweaver_app::store::reading_aids::RulerMode;
        app.update_settings(|s| s.reading_aids.ruler.mode = RulerMode::Ruler)
            .map_err(|e| format!("cannot save the ruler: {e}"))?;
    }
    if let Some(panel) = opts.panel {
        if panel == GuiSidebar::Notes {
            add_review_notes(&mut app)?;
        }
        app.update_settings(|s| s.gui.sidebar = panel)
            .map_err(|e| format!("cannot save the panel: {e}"))?;
    }
    if opts.edit {
        use textweaver_app::Command;
        use textweaver_app::keymap::ActionId;
        let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    }
    if let Some(at) = opts.highlight_at {
        // The app's cursor on the reading position, so the status bar's
        // line and the view's scroll agree with the highlight drawn
        // (a resumed position or a note's place no longer wins).
        app.set_cursor(CharPos(at));
    }
    let palette = match opts.theme.as_deref() {
        // Windows High Contrast's own colors, as the window follows them.
        Some(crate::system_colors::NIGHT_SKY_NAME) => {
            crate::system_colors::palette(&crate::system_colors::NIGHT_SKY)
        }
        Some(name) => Palette::named(name),
        None => Palette::from_theme(&app.reading_theme()),
    };
    // The Colors dialog measures against the palette drawn (GUI audit
    // QW9), not the saved theme.
    if opts.theme.is_some() {
        app.set_drawn_colors(Some((palette.background, palette.text)));
    }
    let font = crate::fonts::doc_font(&app.settings().reading_aids.font);
    let tree = gui::build_tree(
        &palette,
        font,
        Some(&app),
        Rc::new(Cell::new(0)),
        Default::default(),
    );
    build(&app, tree.root, &palette, opts)
}

fn build(
    app: &textweaver_app::App,
    root: NewWidget<Root>,
    palette: &Palette,
    opts: &ShotOptions,
) -> Result<TestHarness<Root>, String> {
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
    // The panel beside the document, as the window shows it from the
    // setting; it leaves the focus alone.
    let _ = crate::sidebar::sync(
        app,
        palette,
        &mut crate::sidebar::SidebarShown::default(),
        &mut harness,
    );
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
        // What the highlight draws follows `[highlight]` from the app's
        // spoken word (W9a-v), which no speech confirmed here: draw it
        // from this position instead, as the window would while reading.
        let shown = gui::highlight_shown_for(app, (state.spoken, state.sentence));
        harness.edit_widget(DOC, |mut d| {
            DocumentView::set_state(&mut d, state);
            DocumentView::set_highlight_shown(&mut d, shown);
        });
    }
    if let Some((title, items)) = &opts.list {
        let (modal, list_id) = gui::list_dialog(
            palette,
            &textweaver_app::lexicon::i18n::Catalog::english(),
            title,
            items.clone(),
            1,
            false,
        );
        harness.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        harness.focus_on(Some(list_id));
    }
    if opts.settings {
        let mut form = crate::settings_dialog::SettingsForm::new(app.settings_schema());
        let (section, row) = match &opts.settings_filter {
            Some(query) => {
                form.set_filter(query, &app.catalog());
                (0, 0)
            }
            None => form.find("speech.rate").unwrap_or((0, 0)),
        };
        let d = gui::settings_dialog(palette, &form, app, section, row);
        harness.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(d.modal)));
        harness.focus_on(Some(d.form));
    }
    if opts.reading {
        let form = crate::settings_dialog::SettingsForm::reading(app.settings_schema());
        let d = gui::settings_dialog(palette, &form, app, 0, 0);
        harness.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(d.modal)));
        harness.focus_on(Some(d.form));
    }
    if opts.colors {
        let form = crate::settings_dialog::SettingsForm::colors(app.settings_schema());
        let d = gui::settings_dialog(palette, &form, app, 0, 0);
        harness.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(d.modal)));
        harness.focus_on(Some(d.form));
    }
    if opts.voices {
        let items = [
            "Reed, en-US, Eloquence, current",
            "Microsoft Zira (OneCore), en-US, SAPI 5, OneCore, favorite",
            "Joe (medium), en-US, Piper, medium",
            "Paul, en-US, DECtalk",
            "English (America), en-US, eSpeak NG",
            "Amy (low), en-US, Piper, low, download 63 MB, non-commercial",
            "eci:shelley, favorite, not on this computer",
        ];
        let d = crate::voices::voice_dialog(
            palette,
            app,
            &app.catalog().tr("voice-list-title"),
            items.iter().map(|s| (*s).to_owned()).collect(),
            1,
        );
        harness.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(d.modal)));
        harness.focus_on(Some(d.list));
    }
    Ok(harness)
}

/// The screenshots for review: Galaxy, Galaxy Light, high contrast, and
/// Windows High Contrast's own colors at 100% and 200% with a spoken word
/// (bold, its sentence underlined), a list dialog, the settings dialog,
/// the Colors dialog, the voice manager, edit mode, the window with no
/// document, the reading ruler's band, the Contents and Notes panels, the
/// Reading settings dialog, Settings with a filter, Lamplight, and the
/// window at four small sizes (960 by 540, 780 by 540 with the bars
/// folded, 683 by 384 at 200%, and 420 by 320). Returns the files.
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
        settings: false,
        home: Some(dir.join("home")),
        aids: false,
        colors: false,
        voices: false,
        edit: false,
        panel: None,
        ruler: false,
        reading: false,
        settings_filter: None,
    };
    // What each shot shows over the window: nothing, a list, or settings.
    const WINDOW: u8 = 0;
    const LIST: u8 = 1;
    const SETTINGS: u8 = 2;
    // The reading aids on, with RSVP under the document.
    const AIDS: u8 = 3;
    // The Colors dialog, with a few colors chosen (W6a6).
    const COLORS: u8 = 4;
    // The voice manager, with sample voices (W7v).
    const VOICES: u8 = 5;
    // Edit mode, with the caret in the first paragraph (W8c-x).
    const EDIT: u8 = 6;
    // The window with no document open (W8c-x).
    const EMPTY: u8 = 7;
    // The Contents panel beside the document (W8c-x).
    const CONTENTS: u8 = 8;
    // The Notes panel, with two sample notes (W8c-x).
    const NOTES: u8 = 9;
    // Only the reading ruler, so its band's bar shows (W8c-x).
    const RULER: u8 = 10;
    // The Reading settings dialog (W9b-d).
    const READING: u8 = 11;
    // Settings with a filter typed (W9b-d).
    const FILTER: u8 = 12;
    // The usual review size, and the small ones (W8c-x): half of a 1920 by
    // 1080 screen, a 1366 by 768 laptop at 200% (683 by 384 logical), and
    // the smallest window that should still read.
    const FULL: (u32, u32) = (1100, 780);
    const HALF: (u32, u32) = (960, 540);
    const LAPTOP: (u32, u32) = (683, 384);
    const TINY: (u32, u32) = (420, 320);
    const FOLD: (u32, u32) = (780, 540);
    const NIGHT_SKY: &str = crate::system_colors::NIGHT_SKY_NAME;
    let shots = [
        ("galaxy-100.png", "galaxy", 1.0, WINDOW),
        ("galaxy-200.png", "galaxy", 2.0, WINDOW),
        ("galaxy-light-100.png", "galaxy-light", 1.0, WINDOW),
        ("galaxy-light-200.png", "galaxy-light", 2.0, WINDOW),
        ("high-contrast-100.png", "high-contrast", 1.0, WINDOW),
        ("high-contrast-200.png", "high-contrast", 2.0, WINDOW),
        ("galaxy-dialog-100.png", "galaxy", 1.0, LIST),
        ("galaxy-dialog-200.png", "galaxy", 2.0, LIST),
        ("galaxy-settings-100.png", "galaxy", 1.0, SETTINGS),
        ("galaxy-settings-200.png", "galaxy", 2.0, SETTINGS),
        (
            "galaxy-light-settings-100.png",
            "galaxy-light",
            1.0,
            SETTINGS,
        ),
        (
            "high-contrast-settings-100.png",
            "high-contrast",
            1.0,
            SETTINGS,
        ),
        ("galaxy-aids-100.png", "galaxy", 1.0, AIDS),
        ("galaxy-aids-200.png", "galaxy", 2.0, AIDS),
        ("galaxy-light-aids-100.png", "galaxy-light", 1.0, AIDS),
        ("high-contrast-aids-100.png", "high-contrast", 1.0, AIDS),
        // Windows High Contrast ("Night sky"): the system's colors.
        ("system-contrast-100.png", NIGHT_SKY, 1.0, WINDOW),
        ("system-contrast-200.png", NIGHT_SKY, 2.0, WINDOW),
        ("system-contrast-aids-100.png", NIGHT_SKY, 1.0, AIDS),
        ("galaxy-colors-100.png", "galaxy", 1.0, COLORS),
        ("galaxy-colors-200.png", "galaxy", 2.0, COLORS),
        ("galaxy-light-colors-100.png", "galaxy-light", 1.0, COLORS),
        ("high-contrast-colors-100.png", "high-contrast", 1.0, COLORS),
        ("galaxy-voices-100.png", "galaxy", 1.0, VOICES),
        ("galaxy-voices-200.png", "galaxy", 2.0, VOICES),
        ("high-contrast-voices-100.png", "high-contrast", 1.0, VOICES),
        ("galaxy-edit-100.png", "galaxy", 1.0, EDIT),
        ("galaxy-edit-200.png", "galaxy", 2.0, EDIT),
        ("galaxy-light-edit-100.png", "galaxy-light", 1.0, EDIT),
        ("galaxy-empty-100.png", "galaxy", 1.0, EMPTY),
        ("galaxy-empty-200.png", "galaxy", 2.0, EMPTY),
        ("high-contrast-empty-100.png", "high-contrast", 1.0, EMPTY),
        ("galaxy-ruler-100.png", "galaxy", 1.0, RULER),
        ("galaxy-ruler-200.png", "galaxy", 2.0, RULER),
        ("galaxy-light-ruler-100.png", "galaxy-light", 1.0, RULER),
        ("high-contrast-ruler-100.png", "high-contrast", 1.0, RULER),
        ("galaxy-contents-100.png", "galaxy", 1.0, CONTENTS),
        ("galaxy-contents-200.png", "galaxy", 2.0, CONTENTS),
        (
            "galaxy-light-contents-100.png",
            "galaxy-light",
            1.0,
            CONTENTS,
        ),
        (
            "high-contrast-contents-100.png",
            "high-contrast",
            1.0,
            CONTENTS,
        ),
        ("system-contrast-contents-100.png", NIGHT_SKY, 1.0, CONTENTS),
        ("galaxy-notes-100.png", "galaxy", 1.0, NOTES),
        ("galaxy-notes-200.png", "galaxy", 2.0, NOTES),
        // Wave 9: the Reading settings dialog, Settings with a filter,
        // and Lamplight, the soft dark theme.
        ("galaxy-reading-100.png", "galaxy", 1.0, READING),
        ("galaxy-reading-200.png", "galaxy", 2.0, READING),
        (
            "high-contrast-reading-100.png",
            "high-contrast",
            1.0,
            READING,
        ),
        ("galaxy-filter-100.png", "galaxy", 1.0, FILTER),
        ("lamplight-100.png", "lamplight", 1.0, WINDOW),
        ("lamplight-200.png", "lamplight", 2.0, WINDOW),
        ("lamplight-edit-100.png", "lamplight", 1.0, EDIT),
        ("lamplight-settings-100.png", "lamplight", 1.0, SETTINGS),
    ]
    .map(|(name, theme, scale, over)| (name, theme, scale, over, FULL));
    // The small windows: the window, its panel, and edit mode at each.
    let small = [
        ("galaxy-960x540-100.png", "galaxy", 1.0, WINDOW, HALF),
        (
            "galaxy-contents-960x540-100.png",
            "galaxy",
            1.0,
            CONTENTS,
            HALF,
        ),
        ("galaxy-edit-960x540-100.png", "galaxy", 1.0, EDIT, HALF),
        ("galaxy-683x384-200.png", "galaxy", 2.0, WINDOW, LAPTOP),
        (
            "galaxy-contents-683x384-200.png",
            "galaxy",
            2.0,
            CONTENTS,
            LAPTOP,
        ),
        ("galaxy-edit-683x384-200.png", "galaxy", 2.0, EDIT, LAPTOP),
        ("galaxy-420x320-100.png", "galaxy", 1.0, WINDOW, TINY),
        (
            "galaxy-contents-420x320-100.png",
            "galaxy",
            1.0,
            CONTENTS,
            TINY,
        ),
        ("galaxy-empty-420x320-100.png", "galaxy", 1.0, EMPTY, TINY),
        // Just under the width where the bars fold into one (W9a-w).
        ("galaxy-780x540-100.png", "galaxy", 1.0, WINDOW, FOLD),
        ("galaxy-edit-780x540-100.png", "galaxy", 1.0, EDIT, FOLD),
    ];
    for (name, theme, scale, over, size) in shots.into_iter().chain(small) {
        let mut o = base.clone();
        o.path = dir.join(name);
        o.theme = Some(theme.into());
        o.scale = scale;
        o.size = size;
        o.settings = over == SETTINGS || over == FILTER;
        o.settings_filter = (over == FILTER).then(|| "voice".to_owned());
        o.reading = over == READING;
        o.aids = over == AIDS;
        o.colors = over == COLORS;
        o.voices = over == VOICES;
        o.edit = over == EDIT;
        o.ruler = over == RULER;
        o.panel = match over {
            CONTENTS => Some(GuiSidebar::Contents),
            NOTES => Some(GuiSidebar::Notes),
            _ => None,
        };
        if over == EMPTY {
            o.file = None;
            o.highlight_at = None;
        }
        if over == EDIT {
            // A caret, not a spoken word: editing is not reading.
            o.highlight_at = None;
        }
        // Each shot starts from the defaults; the shots that change a
        // setting save theirs in their own folder.
        o.home = Some(dir.join(match over {
            AIDS => "home-aids",
            COLORS => "home-colors",
            RULER => "home-ruler",
            CONTENTS => "home-contents",
            NOTES => "home-notes",
            _ => "home",
        }));
        if over == LIST {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::PaintStep;

    fn options(home: &Path) -> ShotOptions {
        ShotOptions {
            path: PathBuf::new(),
            file: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md")),
            size: (1100, 780),
            scale: 1.0,
            theme: Some("galaxy".into()),
            highlight_at: Some(120),
            list: None,
            settings: false,
            home: Some(home.to_path_buf()),
            aids: false,
            colors: false,
            voices: false,
            edit: false,
            panel: None,
            ruler: false,
            reading: false,
            settings_filter: None,
        }
    }

    /// The review pictures' reading position draws the spoken word's band
    /// and its sentence's underline (the alpha.9 review found neither).
    #[test]
    fn review_reading_position_draws_word_and_sentence() {
        let home = tempfile::tempdir().expect("temp dir");
        let mut h = harness(&options(home.path())).expect("harness");
        let _ = h.render();
        let painted = h.get_widget(DOC).inner().painted().to_vec();
        assert!(
            painted.iter().any(|s| matches!(s, PaintStep::WordBand(_))),
            "no word band in {painted:?}"
        );
        assert!(
            painted
                .iter()
                .any(|s| matches!(s, PaintStep::SentenceUnderline(..))),
            "no sentence underline in {painted:?}"
        );
    }
}
