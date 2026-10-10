//! The reading settings (W9b-d's reading form): the settings a reader
//! changes most while reading, in one short form, and the two text-spacing
//! presets (WCAG and Generous) that set the four spacings at once.
//!
//! The window shows them as its Reading settings dialog, with a button to
//! the voice manager beside the rate; the terminal shows them as a view of
//! its settings screen, as it shows the colors.

use serde_json::Value;
use textweaver_aids::TextSpacing;

use crate::app::App;
use crate::command::Effect;

/// The reading settings, in the order the form lists them.
pub const READING_SETTINGS: [&str; 17] = [
    "speech.rate",
    "reading_aids.font.family",
    "reading_aids.font.size_pt",
    "reading_aids.font.weight",
    "reading_aids.spacing.line_height",
    "reading_aids.spacing.paragraph_spacing",
    "reading_aids.spacing.word_spacing",
    "reading_aids.spacing.letter_spacing",
    "display.measure",
    "display.theme",
    "highlight.granularity",
    "highlight.color",
    "highlight.sentence_color",
    "reading_aids.ruler.mode",
    "reading_aids.ruler.mask_outside",
    "reading_aids.bionic",
    "reading_aids.syllables",
];

/// True for a setting the reading form lists.
pub fn is_reading_setting(path: &str) -> bool {
    READING_SETTINGS.contains(&path)
}

/// A text-spacing preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpacingPreset {
    /// Exactly the values WCAG 1.4.12 names (`TextSpacing::wcag`).
    Wcag,
    /// More room than WCAG's values (`TextSpacing::generous`).
    Generous,
}

impl SpacingPreset {
    /// The four spacings of this preset.
    pub fn spacing(self) -> TextSpacing {
        match self {
            SpacingPreset::Wcag => TextSpacing::wcag(),
            SpacingPreset::Generous => TextSpacing::generous(),
        }
    }
}

/// A spacing as the settings store it: two decimals, so 0.12 stays 0.12.
fn number(v: f32) -> Value {
    let v = (f64::from(v) * 100.0).round() / 100.0;
    serde_json::Number::from_f64(v).map_or(Value::Null, Value::Number)
}

impl App {
    /// View, Reading settings in the terminal: the settings screen showing
    /// only the reading settings.
    pub(crate) fn open_reading_form(&mut self) -> Vec<Effect> {
        self.open_settings_screen_with(Some(is_reading_setting), "reading-form-intro")
    }

    /// Sets the four text spacings to `preset`'s and saves them. Returns
    /// what to say ("Spacing set to WCAG text spacing."), or why a value
    /// was refused.
    pub fn apply_spacing_preset(&mut self, preset: SpacingPreset) -> Result<String, String> {
        let s = preset.spacing();
        for (path, v) in [
            ("reading_aids.spacing.line_height", s.line_height),
            (
                "reading_aids.spacing.paragraph_spacing",
                s.paragraph_spacing,
            ),
            ("reading_aids.spacing.word_spacing", s.word_spacing),
            ("reading_aids.spacing.letter_spacing", s.letter_spacing),
        ] {
            self.set_setting(path, number(v))?;
        }
        Ok(self.msg(match preset {
            SpacingPreset::Wcag => "reading-form-spacing-wcag-done",
            SpacingPreset::Generous => "reading-form-spacing-generous-done",
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppConfig;
    use crate::command::Command;
    use crate::keymap::ActionId;

    #[test]
    fn every_reading_setting_is_in_the_schema() {
        let app = App::new(AppConfig::for_tests());
        let schema = app.settings_schema();
        for path in READING_SETTINGS {
            assert!(schema.get(path).is_some(), "{path}");
        }
    }

    #[test]
    fn the_reading_view_lists_only_reading_settings() {
        let mut app = App::new(AppConfig::for_tests());
        app.dispatch(Command::Action(ActionId::ReadingForm));
        let list = app.list_model().unwrap();
        // The line length is the window's own (`display.measure`).
        let shown = READING_SETTINGS
            .iter()
            .filter(|p| !crate::settings_schema::WINDOW_ONLY.contains(p))
            .count();
        assert_eq!(list.items.len(), shown, "{:?}", list.items);
        assert!(list.items[0].starts_with("Rate"), "{:?}", list.items);
    }

    #[test]
    fn the_presets_set_the_four_spacings() {
        let mut app = App::new(AppConfig::for_tests());
        let said = app.apply_spacing_preset(SpacingPreset::Generous).unwrap();
        assert!(said.contains("Generous"), "{said}");
        let s = &app.settings().reading_aids.spacing;
        assert!((s.line_height - 2.0).abs() < 1e-3);
        assert!((s.letter_spacing - 0.15).abs() < 1e-3);
        app.apply_spacing_preset(SpacingPreset::Wcag).unwrap();
        let s = &app.settings().reading_aids.spacing;
        assert!((s.line_height - 1.5).abs() < 1e-3);
        assert!((s.word_spacing - 0.16).abs() < 1e-3);
    }

    #[test]
    fn letters_wider_than_words_get_one_hint() {
        let mut app = App::new(AppConfig::for_tests());
        let hint = "Raise word spacing with letter spacing.";
        let said = app
            .set_setting("reading_aids.spacing.word_spacing", serde_json::json!(0.0))
            .unwrap();
        assert!(!said.contains(hint), "{said}");
        let said = app
            .set_setting(
                "reading_aids.spacing.letter_spacing",
                serde_json::json!(0.2),
            )
            .unwrap();
        assert!(said.ends_with(hint), "{said}");
        let said = app
            .set_setting("reading_aids.spacing.word_spacing", serde_json::json!(0.3))
            .unwrap();
        assert!(!said.contains(hint), "{said}");
    }
}
