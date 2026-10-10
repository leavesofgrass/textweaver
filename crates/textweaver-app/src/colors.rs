//! Colors for the reading aids and each part of the screen (Wave 6, W6u):
//! `[colors]`, with `[highlight] color` and `sentence_color`, laid over
//! the theme, and the Colors view of the settings screen.
//!
//! Each color is a name from a short list, said in words ("dark blue",
//! "orange"; blue and orange first, safe for red-green color blindness),
//! or typed as `#rrggbb`. The settings screen shows and says its contrast
//! as a ratio and a word ("4.8 to 1, good"); a color under 3 to 1 is
//! allowed and warned about, never refused silently. Color never carries
//! meaning alone: every mark keeps its underline, bold, symbol, or spoken
//! word, whatever the color.

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_theme::reading::parse_setting;
use textweaver_theme::{ColorRole, Rgb, StyleRole, Theme, contrast_ratio};

use crate::app::App;
use crate::command::Effect;

/// The color settings, in the order the Colors view lists them.
pub const COLOR_SETTINGS: [&str; 15] = [
    "highlight.color",
    "highlight.sentence_color",
    "colors.ruler",
    "colors.difficult_words",
    "colors.syllables",
    "colors.misspellings",
    "colors.lint",
    "colors.find_match",
    "colors.selection",
    "colors.focus",
    "colors.links",
    "colors.headings",
    "colors.status_bar",
    "colors.notes",
    "colors.bookmarks",
];

/// The named colors offered, first the pair that is safe with red-green
/// color blindness. Values are what `parse_setting` takes; labels are in
/// words (`color-name-*` in the catalog).
pub const COLOR_CHOICES: &[(&str, &str)] = &[
    ("theme", "the theme's color"),
    ("blue", "blue"),
    ("orange", "orange"),
    ("navy", "dark blue"),
    ("skyblue", "sky blue"),
    ("teal", "teal"),
    ("gold", "gold"),
    ("yellow", "yellow"),
    ("purple", "purple"),
    ("pink", "pink"),
    ("brown", "brown"),
    ("gray", "gray"),
    ("black", "black"),
    ("white", "white"),
];

/// True for a setting that holds a color.
pub fn is_color_setting(path: &str) -> bool {
    COLOR_SETTINGS.contains(&path)
}

/// What a color setting paints, which decides what its contrast is
/// measured against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Paint {
    /// A band behind text: the text on it must read.
    Band(StyleRole),
    /// Text on the page.
    Text(&'static [ColorRole]),
    /// A mark on the page (an underline, a separator, a ruler band the
    /// terminal draws with attributes): against the page. Sets the
    /// theme's derived role of the same purpose.
    Mark(ColorRole),
}

fn paint(path: &str) -> Paint {
    const HEADINGS: &[ColorRole] = &[
        ColorRole::Heading1,
        ColorRole::Heading2,
        ColorRole::Heading3,
        ColorRole::Heading4,
        ColorRole::Heading5,
        ColorRole::Heading6,
    ];
    match path {
        "highlight.color" => Paint::Band(StyleRole::SpokenWord),
        "highlight.sentence_color" => Paint::Band(StyleRole::SpokenSentence),
        "colors.find_match" => Paint::Band(StyleRole::FindHit),
        "colors.selection" => Paint::Band(StyleRole::Selection),
        "colors.focus" => Paint::Band(StyleRole::Focus),
        "colors.status_bar" => Paint::Band(StyleRole::StatusBar),
        "colors.notes" => Paint::Band(StyleRole::Note),
        "colors.bookmarks" => Paint::Band(StyleRole::Bookmark),
        "colors.links" => Paint::Text(&[ColorRole::Link]),
        "colors.headings" => Paint::Text(HEADINGS),
        "colors.ruler" => Paint::Mark(ColorRole::Ruler),
        "colors.difficult_words" => Paint::Mark(ColorRole::DifficultWord),
        "colors.syllables" => Paint::Mark(ColorRole::SyllableMark),
        "colors.misspellings" => Paint::Mark(ColorRole::Misspelling),
        _ => Paint::Mark(ColorRole::Lint),
    }
}

/// The contrast a color reaches where `path` paints it on a `page` with
/// `text`: for a band, the better of the text and page colors on it (the
/// one drawn there); for text and marks, against the page. The reading
/// ruler is a band in the window (its bar is the cue), so its row
/// measures the text on it, like the other bands.
fn contrast_with(page: Rgb, text: Rgb, path: &str, color: Rgb) -> f64 {
    match paint(path) {
        Paint::Band(_) | Paint::Mark(ColorRole::Ruler) => {
            contrast_ratio(text, color).max(contrast_ratio(page, color))
        }
        Paint::Text(_) | Paint::Mark(_) => contrast_ratio(color, page),
    }
}

/// The contrast in words: at least 4.5 to 1 is good, at least 3 to 1 is
/// fair, and less is low (and warned about).
pub fn contrast_word(c: &Catalog, ratio: f64) -> String {
    c.tr(if ratio >= 4.5 {
        "colors-contrast-good"
    } else if ratio >= 3.0 {
        "colors-contrast-fair"
    } else {
        "colors-contrast-low"
    })
}

/// A ratio as said: "4.8", in the catalog's number form.
fn ratio_text(c: &Catalog, ratio: f64) -> String {
    let n = (ratio * 10.0).round() / 10.0;
    crate::words::decimal(c, n)
}

impl App {
    /// Lays `[colors]` over `theme`'s roles: bands behind find matches,
    /// the selection, focus, the status bar, notes, and bookmarks; the
    /// text color of links and headings; and the reading aids' marks (the
    /// ruler, difficult words, syllables, misspellings, lint) on the
    /// derived roles that hold them. A band gets the more readable of
    /// the theme's text and page colors on it, and every style keeps its
    /// attributes, so nothing is shown by color alone.
    pub(crate) fn apply_color_settings(&self, theme: &mut Theme) {
        let c = &self.settings.colors;
        let values = [
            ("colors.find_match", &c.find_match),
            ("colors.selection", &c.selection),
            ("colors.focus", &c.focus),
            ("colors.status_bar", &c.status_bar),
            ("colors.notes", &c.notes),
            ("colors.bookmarks", &c.bookmarks),
            ("colors.links", &c.links),
            ("colors.headings", &c.headings),
            ("colors.ruler", &c.ruler),
            ("colors.difficult_words", &c.difficult_words),
            ("colors.syllables", &c.syllables),
            ("colors.misspellings", &c.misspellings),
            ("colors.lint", &c.lint),
        ];
        let text = theme.color(ColorRole::Text);
        let page = theme.color(ColorRole::Background);
        for (path, value) in values {
            let Ok(Some(rgb)) = parse_setting(value) else {
                continue;
            };
            match paint(path) {
                Paint::Band(role) => {
                    let fg = if contrast_ratio(text, rgb) >= contrast_ratio(page, rgb) {
                        text
                    } else {
                        page
                    };
                    let style = theme.style_mut(role);
                    style.foreground = fg;
                    style.background = Some(rgb);
                    style.attributes.reverse = false;
                    if style.attributes.is_empty() {
                        style.attributes = role.no_color_attributes();
                        style.attributes.reverse = false;
                    }
                }
                Paint::Text(roles) => {
                    for &r in roles {
                        theme.set_color(r, rgb);
                    }
                }
                Paint::Mark(role) => theme.set_color(role, rgb),
            }
        }
    }

    /// What decides the colors laid over the theme ([`App::reading_theme`]).
    pub(crate) fn color_settings_key(&self) -> Vec<String> {
        let c = &self.settings.colors;
        [
            &c.ruler,
            &c.difficult_words,
            &c.syllables,
            &c.misspellings,
            &c.lint,
            &c.find_match,
            &c.selection,
            &c.focus,
            &c.links,
            &c.headings,
            &c.status_bar,
            &c.notes,
            &c.bookmarks,
        ]
        .iter()
        .map(|s| (*s).clone())
        .collect()
    }

    /// The contrast of the color setting at `path`, on the current theme:
    /// the ratio, and "good", "fair", or "low"; `None` for the theme's own
    /// color or a value that is not a color.
    pub fn color_contrast(&self, path: &str) -> Option<(f64, String)> {
        let value = self.setting_value(path)?;
        let text = value.as_str()?;
        let rgb = parse_setting(text).ok()??;
        let theme = self.current_theme();
        let (page, text) = self.drawn_colors.unwrap_or((
            theme.color(ColorRole::Background),
            theme.color(ColorRole::Text),
        ));
        let ratio = contrast_with(page, text, path, rgb);
        Some((ratio, contrast_word(self.cat(), ratio)))
    }

    /// The page and text colors a frontend draws with when they are not
    /// the current theme's: a theme its command line chose, or the
    /// system's high contrast colors. The Colors view then measures each
    /// color against what is on the screen, not the saved theme. `None`
    /// goes back to the theme's.
    pub fn set_drawn_colors(&mut self, colors: Option<(Rgb, Rgb)>) {
        self.drawn_colors = colors;
    }

    /// A color setting's row on the settings screen: its label, value,
    /// and contrast ("Links: blue, contrast 4.8 to 1, good").
    pub(crate) fn color_row(&self, c: &Catalog, label: &str, value: &str, path: &str) -> String {
        match self.color_contrast(path) {
            Some((ratio, word)) => c.fmt(
                "colors-item",
                &args![
                    "label" => label,
                    "value" => value,
                    "ratio" => ratio_text(c, ratio),
                    "verdict" => word
                ],
            ),
            None => c.fmt("settings-item", &args!["label" => label, "value" => value]),
        }
    }

    /// Said after a color setting changes: its contrast, and a warning
    /// under 3 to 1. Empty for the theme's own color.
    pub(crate) fn color_change_note(&self, path: &str) -> String {
        let c = self.cat();
        let Some((ratio, word)) = self.color_contrast(path) else {
            return String::new();
        };
        let mut s = c.fmt(
            "colors-contrast",
            &args!["ratio" => ratio_text(c, ratio), "verdict" => word],
        );
        if ratio < 3.0 {
            s.push(' ');
            s.push_str(&c.tr("colors-contrast-warning"));
        }
        s
    }

    /// View, Colors: the settings screen showing only the colors.
    pub(crate) fn open_color_settings(&mut self) -> Vec<Effect> {
        self.open_settings_screen_with(Some(is_color_setting), "colors-intro")
    }

    /// Asks before every color goes back to the theme's (the window's
    /// Reset all colors): "Reset every color to the theme's? y or n". A
    /// yes resets them and says so; a no keeps them.
    pub fn ask_reset_colors(&mut self) {
        self.pending_colors_reset = true;
        let question = self.msg("colors-reset-question");
        self.ask(&question);
    }

    /// The answer to [`ask_reset_colors`](Self::ask_reset_colors).
    pub(crate) fn confirm_colors_reset(&mut self, answer: crate::command::Confirm) -> Vec<Effect> {
        use crate::command::Confirm;
        match answer {
            Confirm::Repeat => {
                let question = self.msg("colors-reset-question");
                self.ask(&question);
            }
            Confirm::No => {
                self.pending_colors_reset = false;
                let msg = self.msg("common-kept");
                self.tell(&msg);
            }
            Confirm::Yes => {
                self.pending_colors_reset = false;
                let mut failed = None;
                for path in COLOR_SETTINGS {
                    if let Err(e) = self.set_setting(path, serde_json::Value::Null) {
                        failed = Some(e);
                    }
                }
                match failed {
                    Some(e) => self.error(&e),
                    None => {
                        let msg = self.msg("gui-colors-reset-done");
                        self.tell(&msg);
                    }
                }
            }
        }
        vec![Effect::Redraw]
    }
}

#[cfg(test)]
mod tests {
    use textweaver_keymap::ActionId;

    use crate::{AppConfig, Command};

    use super::*;

    /// The reading aids' mark colors reach the theme's derived roles.
    #[test]
    fn mark_colors_set_the_derived_roles() {
        let mut app = App::new(AppConfig::for_tests());
        app.settings.colors.lint = "orange".into();
        app.settings.colors.ruler = "navy".into();
        let t = app.reading_theme();
        let get = |v: &str| parse_setting(v).unwrap().unwrap();
        assert_eq!(t.color(ColorRole::Lint), get("orange"));
        assert_eq!(t.color(ColorRole::Ruler), get("navy"));
        assert!(!t.is_derived(ColorRole::Lint));
    }

    #[test]
    fn every_color_setting_is_in_the_schema_with_the_named_colors() {
        let schema = crate::settings_schema::SettingsSchema::generate();
        for path in COLOR_SETTINGS {
            let s = schema.get(path).unwrap_or_else(|| panic!("{path}"));
            assert!(
                matches!(s.kind, crate::SettingKind::Choice { open: true, .. }),
                "{path}"
            );
        }
        // Blue and orange come first, and red and green are not offered.
        assert_eq!(COLOR_CHOICES[1].0, "blue");
        assert_eq!(COLOR_CHOICES[2].0, "orange");
        assert!(
            !COLOR_CHOICES
                .iter()
                .any(|(v, _)| *v == "red" || *v == "green")
        );
        for (v, _) in COLOR_CHOICES {
            assert!(parse_setting(v).is_ok(), "{v}");
        }
    }

    #[test]
    fn colors_reach_the_theme_and_keep_their_cues() {
        let mut config = AppConfig::for_tests();
        config.settings.colors.links = "orange".into();
        config.settings.colors.find_match = "blue".into();
        let app = App::new(config);
        let theme = app.reading_theme();
        assert_eq!(
            theme.color(ColorRole::Link),
            parse_setting("orange").unwrap().unwrap()
        );
        let hit = theme.style(StyleRole::FindHit);
        assert_eq!(hit.background, parse_setting("blue").ok().flatten());
        assert!(
            !hit.attributes.is_empty(),
            "a find match keeps its underline"
        );
        assert_eq!(
            ColorRole::Link.attributes(),
            textweaver_theme::Attrs::UNDERLINE
        );
    }

    #[test]
    fn a_low_contrast_color_is_applied_and_warned() {
        let mut app = App::new(AppConfig::for_tests());
        // Galaxy is dark: navy links are hard to read on it.
        let said = app
            .set_setting("colors.links", serde_json::Value::String("navy".into()))
            .unwrap();
        assert!(said.contains("Contrast"), "{said}");
        assert!(said.contains("low"), "{said}");
        assert_eq!(app.settings().colors.links, "navy");
        let (ratio, word) = app.color_contrast("colors.links").unwrap();
        assert!(ratio < 3.0);
        assert_eq!(word, "low");
        // The theme's own color has no contrast to report.
        assert_eq!(app.color_contrast("colors.ruler"), None);
    }

    #[test]
    fn the_colors_view_lists_only_colors() {
        let mut app = App::new(AppConfig::for_tests());
        app.dispatch(Command::Action(ActionId::ColorSettings));
        let list = app.list_model().unwrap();
        assert_eq!(list.items.len(), COLOR_SETTINGS.len(), "{:?}", list.items);
        assert!(
            list.items[0].starts_with("Word highlight color"),
            "{:?}",
            list.items
        );
        assert!(
            app.status_text().starts_with("Colors"),
            "{}",
            app.status_text()
        );
    }

    /// Reset all colors asks first; no keeps every color, a stray key asks
    /// again, and yes puts the theme's back and says so.
    #[test]
    fn reset_all_colors_asks_first() {
        use crate::command::Confirm;
        let mut config = AppConfig::for_tests();
        config.settings.colors.links = "orange".into();
        let mut app = App::new(config);
        app.ask_reset_colors();
        assert!(app.confirmation_pending());
        assert_eq!(
            app.status_text(),
            "Reset every color to the theme's? y or n"
        );
        app.dispatch(Command::Confirm(Confirm::No));
        assert!(!app.confirmation_pending());
        assert_eq!(app.settings().colors.links, "orange");
        app.ask_reset_colors();
        app.dispatch(Command::Confirm(Confirm::Repeat));
        assert!(app.confirmation_pending());
        app.dispatch(Command::Confirm(Confirm::Yes));
        assert!(!app.confirmation_pending());
        assert_eq!(app.settings().colors.links, "theme");
        assert_eq!(app.status_text(), "Every color is the theme's again.");
    }
}
