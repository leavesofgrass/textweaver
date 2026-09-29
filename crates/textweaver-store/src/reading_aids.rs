//! `[reading_aids]` settings: RSVP, bionic reading, text spacing, fonts,
//! the reading ruler, and syllable splitting (ADR-0022).
//!
//! These are the saved form only: plain data with defaults, serialized
//! exactly as the settings file spells it. The reading aids themselves live
//! in `textweaver-aids`, which has a working type for each of these (with
//! its checks, CSS, and layout) and converts to and from them, and font
//! resolution lives in `textweaver-fonts`. Keeping the saved form here lets
//! the store depend on nothing but `textweaver-core` (ADR-0001), and gives
//! the settings schema one place to describe every setting.
//!
//! Ranges the store enforces when it loads a file are the font's size and
//! weight ([`FontSettings::range_problem`]); every other value is clamped
//! where it is used.

use serde::{Deserialize, Serialize};

/// `[reading_aids.rsvp] pacing`: what moves the RSVP word forward.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pacing {
    /// Words-per-minute timing (silent reading).
    #[default]
    Timer,
    /// Speech word events (Star's behaviour).
    External,
}

/// `[reading_aids.rsvp] position`: one of Star's nine RSVP positions. The
/// default is top centre, as in Star.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RsvpPosition {
    /// Top left.
    TopLeft,
    /// Top centre (Star's default).
    #[default]
    TopCenter,
    /// Top right.
    TopRight,
    /// Middle left.
    CenterLeft,
    /// Middle of the view.
    Center,
    /// Middle right.
    CenterRight,
    /// Bottom left.
    BottomLeft,
    /// Bottom centre.
    BottomCenter,
    /// Bottom right.
    BottomRight,
}

/// `[reading_aids.rsvp]`: rate, pauses, context words, and position.
/// Missing keys take the defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RsvpSettings {
    /// Words per minute, 50 to 1500.
    pub wpm: u32,
    /// Timer or speech-driven.
    pub pacing: Pacing,
    /// Extra time after a word followed by a comma, semicolon, colon, dash,
    /// bracket, or closing quote, in percent of one word's time.
    pub clause_pause: u32,
    /// Extra time at the end of a sentence, in percent.
    pub sentence_pause: u32,
    /// Extra time at the end of a paragraph, in percent (used instead of
    /// the sentence pause there).
    pub paragraph_pause: u32,
    /// Words longer than this many letters get extra time.
    pub long_word_len: u32,
    /// Extra time per letter beyond `long_word_len`, in percent.
    pub long_word_step: u32,
    /// Most extra time a long word gets, in percent.
    pub long_word_max: u32,
    /// Show the previous word (Star: `qt_rsvp_show_prev`).
    pub show_previous: bool,
    /// Show the next word (Star: `qt_rsvp_show_next`).
    pub show_next: bool,
    /// Where the word appears (Star: `qt_rsvp_position`).
    pub position: RsvpPosition,
    /// Size of the word in the GUI, in points (Star: `qt_rsvp_font_size`).
    pub font_size_pt: u16,
    /// With speech pacing, show this many words ahead of (positive) or
    /// behind (negative) the spoken word (Star: `highlight_lead_words`).
    pub lead_words: i32,
}

impl Default for RsvpSettings {
    fn default() -> Self {
        RsvpSettings {
            wpm: 300,
            pacing: Pacing::Timer,
            clause_pause: 50,
            sentence_pause: 100,
            paragraph_pause: 150,
            long_word_len: 8,
            long_word_step: 10,
            long_word_max: 80,
            show_previous: true,
            show_next: true,
            position: RsvpPosition::TopCenter,
            font_size_pt: 48,
            lead_words: 0,
        }
    }
}

/// `[reading_aids.bionic_options]`: how much of each word to embolden, and
/// what to skip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BionicOptions {
    /// Share of each word to embolden, from 0.1 to 0.9. Star used 0.4.
    pub ratio: f32,
    /// Words shorter than this (in graphemes) are left alone. Star used 2.
    pub min_word_len: usize,
    /// Skip words that contain a digit.
    pub skip_numbers: bool,
    /// Skip URLs and email addresses.
    pub skip_urls: bool,
    /// Skip code: `Code` markers, inline code, and code-like tokens.
    pub skip_code: bool,
}

impl Default for BionicOptions {
    /// Star's settings: 40 percent, words of two letters or more.
    fn default() -> Self {
        BionicOptions {
            ratio: 0.4,
            min_word_len: 2,
            skip_numbers: true,
            skip_urls: true,
            skip_code: true,
        }
    }
}

/// `[reading_aids.spacing]`: text spacing (WCAG 2.2, 1.4.12), all in
/// multiples of the font size (CSS `em`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextSpacing {
    /// Line height, 1.0 to 3.0. Star's default and WCAG's value: 1.5.
    pub line_height: f32,
    /// Space after each paragraph, 0 to 4.
    pub paragraph_spacing: f32,
    /// Extra space between letters, 0 to 0.5.
    pub letter_spacing: f32,
    /// Extra space between words, 0 to 1.
    pub word_spacing: f32,
}

impl Default for TextSpacing {
    /// Star's defaults: line height 1.5, no extra letter or word spacing,
    /// and a modest paragraph gap (Star had none).
    fn default() -> Self {
        TextSpacing {
            line_height: 1.5,
            paragraph_spacing: 1.0,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        }
    }
}

/// Smallest font size the settings accept, in points.
pub const MIN_FONT_PT: f32 = 6.0;
/// Largest font size the settings accept, in points.
pub const MAX_FONT_PT: f32 = 144.0;

/// `[reading_aids.font]`: the reading font's family, size, and weight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontSettings {
    /// The family: `system-ui`, `sans`, `serif`, `monospace`, a reading
    /// font key (`opendyslexic`, `atkinson`, `lexend`), or a family name.
    /// `textweaver-fonts` parses it (`FontFamily`).
    pub family: String,
    /// Size in points (Star's `qt_font_size`, default 14).
    pub size_pt: f32,
    /// Weight, 100 (thin) to 900 (black); 400 is regular, 700 bold.
    pub weight: u16,
}

impl Default for FontSettings {
    fn default() -> Self {
        FontSettings {
            family: "sans".into(),
            size_pt: 14.0,
            weight: 400,
        }
    }
}

impl FontSettings {
    /// What is out of range, if anything: the size (6 to 144 points), the
    /// weight (100 to 900), or an empty family.
    pub fn range_problem(&self) -> Option<String> {
        if !self.size_pt.is_finite() || !(MIN_FONT_PT..=MAX_FONT_PT).contains(&self.size_pt) {
            return Some(format!(
                "font size must be between {MIN_FONT_PT} and {MAX_FONT_PT} points, not {}",
                self.size_pt
            ));
        }
        if !(100..=900).contains(&self.weight) {
            return Some(format!(
                "font weight must be between 100 and 900, not {}",
                self.weight
            ));
        }
        if self.family.trim().is_empty() {
            return Some("the font family name is empty".into());
        }
        None
    }

    /// The settings with size and weight clamped into range, and an empty
    /// family replaced by `sans`.
    pub fn clamped(&self) -> Self {
        FontSettings {
            family: if self.family.trim().is_empty() {
                "sans".into()
            } else {
                self.family.clone()
            },
            size_pt: if self.size_pt.is_finite() {
                self.size_pt.clamp(MIN_FONT_PT, MAX_FONT_PT)
            } else {
                FontSettings::default().size_pt
            },
            weight: self.weight.clamp(100, 900),
        }
    }
}

/// `[reading_aids.ruler] mode`: which aid is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulerMode {
    /// No marks.
    #[default]
    Off,
    /// Mark only the focus rows (Star's current-line highlight).
    CurrentLine,
    /// Mark the focus rows and a band of rows around them (Star's reading
    /// ruler).
    Ruler,
}

/// `[reading_aids.ruler] scope`: what counts as "the line" for the focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulerScope {
    /// The one screen row holding the position.
    Row,
    /// Every wrapped row of the logical line holding the position.
    #[default]
    Line,
}

/// `[reading_aids.ruler]`: the reading ruler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct RulerSettings {
    /// Off, current line, or ruler.
    pub mode: RulerMode,
    /// Row or whole line.
    pub scope: RulerScope,
    /// Band rows above the focus (ruler mode).
    pub rows_above: u8,
    /// Band rows below the focus (ruler mode).
    pub rows_below: u8,
    /// Mask the rows outside the band (a typoscope). Off by default.
    pub mask_outside: bool,
}

impl Default for RulerSettings {
    fn default() -> Self {
        RulerSettings {
            mode: RulerMode::Off,
            scope: RulerScope::Line,
            rows_above: 1,
            rows_below: 1,
            mask_outside: false,
        }
    }
}

/// `[reading_aids.syllable_options]`: how words are split for display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyllableOptions {
    /// Shown between syllables. Star's default is a middle dot; a hyphen or
    /// a thin space also work.
    pub separator: String,
    /// Fewest letters before the first break (Pyphen's default, 2).
    pub left_min: usize,
    /// Fewest letters after the last break (Pyphen's default, 2).
    pub right_min: usize,
    /// Words shorter than this (in chars) are never split.
    pub min_word_len: usize,
    /// Leave URLs and email addresses alone.
    pub skip_urls: bool,
    /// Leave code alone: `Code` markers, inline code, code-like tokens.
    pub skip_code: bool,
}

impl Default for SyllableOptions {
    fn default() -> Self {
        SyllableOptions {
            separator: "\u{b7}".to_owned(),
            left_min: 2,
            right_min: 2,
            min_word_len: 4,
            skip_urls: true,
            skip_code: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_forms_are_as_the_settings_file_spells_them() {
        let r = RsvpSettings {
            pacing: Pacing::External,
            position: RsvpPosition::BottomRight,
            ..RsvpSettings::default()
        };
        let t = toml::to_string(&r).unwrap();
        assert!(t.contains("pacing = \"external\""), "{t}");
        assert!(t.contains("position = \"bottom-right\""), "{t}");
        assert_eq!(toml::from_str::<RsvpSettings>(&t).unwrap(), r);
        let ruler: RulerSettings = toml::from_str("mode = \"current_line\"").unwrap();
        assert_eq!(ruler.mode, RulerMode::CurrentLine);
        assert_eq!(ruler.scope, RulerScope::Line);
        let f: FontSettings = toml::from_str("family = \"lexend\"").unwrap();
        assert_eq!(f.family, "lexend");
        assert_eq!(f.size_pt, 14.0);
    }

    #[test]
    fn font_ranges() {
        assert_eq!(FontSettings::default().range_problem(), None);
        let big = FontSettings {
            size_pt: 500.0,
            ..FontSettings::default()
        };
        assert_eq!(
            big.range_problem().unwrap(),
            "font size must be between 6 and 144 points, not 500"
        );
        assert_eq!(big.clamped().size_pt, MAX_FONT_PT);
        let empty = FontSettings {
            family: " ".into(),
            weight: 50,
            ..FontSettings::default()
        };
        assert!(empty.range_problem().unwrap().contains("weight"));
        let c = empty.clamped();
        assert_eq!((c.family.as_str(), c.weight), ("sans", 100));
        assert_eq!(c.range_problem(), None);
    }
}
