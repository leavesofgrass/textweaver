//! Text spacing (WCAG 2.2 success criterion 1.4.12).
//!
//! 1.4.12 asks that nothing is lost when a reader sets line height to 1.5
//! times the font size, paragraph spacing to 2 times, letter spacing to
//! 0.12 times, and word spacing to 0.16 times. star offered line height,
//! letter spacing (in percent), and word spacing (in pixels), but no
//! paragraph spacing (its audit's one gap), and its word spacing in pixels
//! did not grow with the font. Here all four are multiples of the font
//! size (`em`), so they scale with it, and each can be checked against the
//! criterion's values.
//!
//! Outputs: CSS for HTML views ([`TextSpacing::to_css`]), and the nearest a
//! terminal can do ([`TextSpacing::terminal`]): blank rows and extra
//! spaces.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

/// Line height WCAG 1.4.12 names, in multiples of the font size.
pub const WCAG_LINE_HEIGHT: f32 = 1.5;
/// Paragraph spacing WCAG 1.4.12 names, in multiples of the font size.
pub const WCAG_PARAGRAPH_SPACING: f32 = 2.0;
/// Letter spacing WCAG 1.4.12 names, in multiples of the font size.
pub const WCAG_LETTER_SPACING: f32 = 0.12;
/// Word spacing WCAG 1.4.12 names, in multiples of the font size.
pub const WCAG_WORD_SPACING: f32 = 0.16;

/// Spacing settings, all in multiples of the font size (CSS `em`).
///
/// Line height is the full line box (1.0 is solid, 1.5 is WCAG's value).
/// Letter, word, and paragraph spacing are *extra* space added to the
/// font's own.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextSpacing {
    /// Line height, 1.0 to 3.0. star's default and WCAG's value: 1.5.
    pub line_height: f32,
    /// Space after each paragraph, 0 to 4.
    pub paragraph_spacing: f32,
    /// Extra space between letters, 0 to 0.5.
    pub letter_spacing: f32,
    /// Extra space between words, 0 to 1.
    pub word_spacing: f32,
}

impl Default for TextSpacing {
    /// star's defaults: line height 1.5, no extra letter or word spacing,
    /// and a modest paragraph gap (star had none).
    fn default() -> Self {
        TextSpacing {
            line_height: 1.5,
            paragraph_spacing: 1.0,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        }
    }
}

/// A setting outside the range textweaver supports.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum SpacingError {
    /// The value is not a finite number.
    #[error("{name} must be a number")]
    NotANumber {
        /// Which setting.
        name: &'static str,
    },
    /// The value is outside `min..=max`.
    #[error("{name} must be between {min} and {max}, not {value}")]
    OutOfRange {
        /// Which setting.
        name: &'static str,
        /// The value given.
        value: f32,
        /// Smallest allowed.
        min: f32,
        /// Largest allowed.
        max: f32,
    },
}

/// A setting below the value WCAG 1.4.12 names. Not an error: 1.4.12 asks
/// that content *survives* these values, and a reader may prefer less. It
/// tells the settings screen what to say.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpacingIssue {
    /// Which setting, as spoken ("line height").
    pub name: &'static str,
    /// The current value.
    pub value: f32,
    /// The WCAG 1.4.12 value.
    pub wcag: f32,
}

impl SpacingIssue {
    /// A sentence to show and speak.
    pub fn message(&self) -> String {
        format!(
            "{} is {} times the font size; WCAG text spacing uses {}.",
            capitalize(self.name),
            trim(self.value),
            trim(self.wcag)
        )
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// A number without trailing zeros: 1.5, 2, 0.12.
fn trim(v: f32) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

/// What a terminal can do for spacing: whole blank rows and extra spaces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TerminalSpacing {
    /// Blank rows between wrapped lines (1 for double spacing).
    pub rows_between_lines: u16,
    /// Blank rows between paragraphs (terminal paragraphs already have
    /// one blank line in canonical text; this is the total wanted).
    pub rows_between_paragraphs: u16,
    /// Extra spaces after each space between words.
    pub extra_word_spaces: u16,
}

const RANGES: [(&str, f32, f32); 4] = [
    ("line height", 1.0, 3.0),
    ("paragraph spacing", 0.0, 4.0),
    ("letter spacing", 0.0, 0.5),
    ("word spacing", 0.0, 1.0),
];

impl TextSpacing {
    /// Exactly the values WCAG 1.4.12 names.
    pub fn wcag() -> Self {
        TextSpacing {
            line_height: WCAG_LINE_HEIGHT,
            paragraph_spacing: WCAG_PARAGRAPH_SPACING,
            letter_spacing: WCAG_LETTER_SPACING,
            word_spacing: WCAG_WORD_SPACING,
        }
    }

    /// More room than WCAG's values. Wider line and word spacing helped
    /// readers with macular disease in one study, and wider letter spacing
    /// helped some children with dyslexia; neither is a promise for any
    /// one reader.
    pub fn generous() -> Self {
        TextSpacing {
            line_height: 2.0,
            paragraph_spacing: 2.5,
            letter_spacing: 0.15,
            word_spacing: 0.3,
        }
    }

    /// Converts star's settings: `qt_line_height` (a multiple),
    /// `qt_letter_spacing` (extra percent of the font size), and
    /// `qt_word_spacing` (extra pixels), with the font size in pixels.
    pub fn from_star(line_height: f32, letter_percent: f32, word_px: f32, font_px: f32) -> Self {
        let font = if font_px.is_finite() && font_px > 0.0 {
            font_px
        } else {
            16.0
        };
        let finite = |v: f32, d: f32| if v.is_finite() { v } else { d };
        TextSpacing {
            line_height: finite(line_height, 1.5).clamp(1.0, 3.0),
            paragraph_spacing: TextSpacing::default().paragraph_spacing,
            letter_spacing: (finite(letter_percent, 0.0) / 100.0).clamp(0.0, 0.5),
            word_spacing: (finite(word_px, 0.0) / font).clamp(0.0, 1.0),
        }
    }

    fn values(&self) -> [f32; 4] {
        [
            self.line_height,
            self.paragraph_spacing,
            self.letter_spacing,
            self.word_spacing,
        ]
    }

    /// Checks every value is a number in textweaver's supported range.
    pub fn validate(&self) -> Result<(), SpacingError> {
        for ((name, min, max), value) in RANGES.into_iter().zip(self.values()) {
            if !value.is_finite() {
                return Err(SpacingError::NotANumber { name });
            }
            if !(min..=max).contains(&value) {
                return Err(SpacingError::OutOfRange {
                    name,
                    value,
                    min,
                    max,
                });
            }
        }
        Ok(())
    }

    /// The settings with every value clamped into range (non-numbers take
    /// the default).
    pub fn clamped(&self) -> Self {
        let d = TextSpacing::default().values();
        let v = self.values();
        let c = |i: usize| {
            let (_, min, max) = RANGES[i];
            if v[i].is_finite() {
                v[i].clamp(min, max)
            } else {
                d[i]
            }
        };
        TextSpacing {
            line_height: c(0),
            paragraph_spacing: c(1),
            letter_spacing: c(2),
            word_spacing: c(3),
        }
    }

    /// The values below WCAG 1.4.12's, in the order line height, paragraph
    /// spacing, letter spacing, word spacing. Empty when all meet it.
    pub fn below_wcag(&self) -> Vec<SpacingIssue> {
        let wcag = TextSpacing::wcag().values();
        RANGES
            .iter()
            .zip(self.values())
            .zip(wcag)
            .filter(|((_, v), w)| *v + 1e-4 < *w)
            .map(|(((name, _, _), value), wcag)| SpacingIssue { name, value, wcag })
            .collect()
    }

    /// True when every value is at least WCAG 1.4.12's.
    pub fn meets_wcag(&self) -> bool {
        self.below_wcag().is_empty()
    }

    /// True when letters are spaced wider than words (letter spacing above
    /// zero and above word spacing): words then run together, so word
    /// spacing should rise with letter spacing. WCAG 1.4.12 names word
    /// spacing above letter spacing (0.16 and 0.12).
    pub fn letter_exceeds_word(&self) -> bool {
        self.letter_spacing > 1e-4 && self.letter_spacing > self.word_spacing + 1e-4
    }

    /// CSS declarations for `selector` (for example `body` or `.reader`),
    /// with paragraph spacing on `p`, list items, and blockquotes inside
    /// it. Values are clamped into range first.
    ///
    /// ```
    /// # use textweaver_aids::TextSpacing;
    /// let css = TextSpacing::wcag().to_css("main");
    /// assert!(css.contains("line-height: 1.5;"));
    /// assert!(css.contains("letter-spacing: 0.12em;"));
    /// ```
    pub fn to_css(&self, selector: &str) -> String {
        let s = self.clamped();
        let mut out = String::new();
        let _ = writeln!(out, "{selector} {{");
        let _ = writeln!(out, "  line-height: {};", trim(s.line_height));
        let _ = writeln!(out, "  letter-spacing: {}em;", trim(s.letter_spacing));
        let _ = writeln!(out, "  word-spacing: {}em;", trim(s.word_spacing));
        let _ = writeln!(out, "}}");
        let _ = writeln!(
            out,
            "{selector} p, {selector} li, {selector} blockquote, {selector} dd {{"
        );
        let _ = writeln!(out, "  margin-block-end: {}em;", trim(s.paragraph_spacing));
        let _ = writeln!(out, "}}");
        out
    }

    /// CSS custom properties (`--tw-line-height` and so on) for a template
    /// that applies them itself.
    pub fn to_css_variables(&self) -> String {
        let s = self.clamped();
        format!(
            "--tw-line-height: {}; --tw-paragraph-spacing: {}em; \
             --tw-letter-spacing: {}em; --tw-word-spacing: {}em;",
            trim(s.line_height),
            trim(s.paragraph_spacing),
            trim(s.letter_spacing),
            trim(s.word_spacing)
        )
    }

    /// The nearest terminal equivalent. A terminal cannot space letters or
    /// change line height, so: line height of 2 or more becomes one blank
    /// row between lines; paragraph spacing rounds to whole blank rows
    /// (at least the one canonical text has); word spacing of a quarter
    /// em or more adds a space.
    pub fn terminal(&self) -> TerminalSpacing {
        let s = self.clamped();
        TerminalSpacing {
            rows_between_lines: u16::from(s.line_height >= 2.0 - 1e-4),
            rows_between_paragraphs: (s.paragraph_spacing.round() as u16).max(1),
            extra_word_spaces: (s.word_spacing * 4.0).floor() as u16,
        }
    }

    /// A summary to show and speak: "Line height 1.5, paragraph spacing
    /// 2, letter spacing 0.12, word spacing 0.16 times the font size."
    pub fn summary(&self) -> String {
        format!(
            "Line height {}, paragraph spacing {}, letter spacing {}, word spacing {} times the font size.",
            trim(self.line_height),
            trim(self.paragraph_spacing),
            trim(self.letter_spacing),
            trim(self.word_spacing)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wcag_values_meet_wcag() {
        assert!(TextSpacing::wcag().meets_wcag());
        assert!(TextSpacing::generous().meets_wcag());
        assert!(TextSpacing::wcag().validate().is_ok());
        let d = TextSpacing::default();
        assert!(d.validate().is_ok());
        let issues = d.below_wcag();
        let names: Vec<&str> = issues.iter().map(|i| i.name).collect();
        assert_eq!(
            names,
            ["paragraph spacing", "letter spacing", "word spacing"]
        );
        assert_eq!(
            issues[0].message(),
            "Paragraph spacing is 1 times the font size; WCAG text spacing uses 2."
        );
    }

    #[test]
    fn letters_wider_than_words_are_flagged() {
        assert!(!TextSpacing::default().letter_exceeds_word());
        assert!(!TextSpacing::wcag().letter_exceeds_word());
        assert!(!TextSpacing::generous().letter_exceeds_word());
        let wide = TextSpacing {
            letter_spacing: 0.2,
            word_spacing: 0.0,
            ..TextSpacing::default()
        };
        assert!(wide.letter_exceeds_word());
    }

    #[test]
    fn validation_rejects_bad_values() {
        let bad = TextSpacing {
            line_height: 0.5,
            ..TextSpacing::default()
        };
        assert_eq!(
            bad.validate(),
            Err(SpacingError::OutOfRange {
                name: "line height",
                value: 0.5,
                min: 1.0,
                max: 3.0
            })
        );
        let nan = TextSpacing {
            word_spacing: f32::NAN,
            ..TextSpacing::default()
        };
        assert_eq!(
            nan.validate(),
            Err(SpacingError::NotANumber {
                name: "word spacing"
            })
        );
        let c = nan.clamped();
        assert_eq!(c.word_spacing, 0.0);
        assert_eq!(bad.clamped().line_height, 1.0);
        assert_eq!(
            bad.validate().unwrap_err().to_string(),
            "line height must be between 1 and 3, not 0.5"
        );
    }

    #[test]
    fn css_output() {
        let css = TextSpacing::wcag().to_css(".reader");
        assert_eq!(
            css,
            ".reader {\n  line-height: 1.5;\n  letter-spacing: 0.12em;\n  word-spacing: 0.16em;\n}\n\
             .reader p, .reader li, .reader blockquote, .reader dd {\n  margin-block-end: 2em;\n}\n"
        );
        let vars = TextSpacing::wcag().to_css_variables();
        assert!(vars.contains("--tw-word-spacing: 0.16em;"));
    }

    #[test]
    fn terminal_equivalents() {
        assert_eq!(
            TextSpacing::default().terminal(),
            TerminalSpacing {
                rows_between_lines: 0,
                rows_between_paragraphs: 1,
                extra_word_spaces: 0
            }
        );
        assert_eq!(
            TextSpacing::generous().terminal(),
            TerminalSpacing {
                rows_between_lines: 1,
                rows_between_paragraphs: 3,
                extra_word_spaces: 1
            }
        );
    }

    #[test]
    fn from_star_settings() {
        // star: 1.5x, +12 %, +3 px at a 16 px font.
        let s = TextSpacing::from_star(1.5, 12.0, 3.0, 16.0);
        assert_eq!(s.line_height, 1.5);
        assert!((s.letter_spacing - 0.12).abs() < 1e-6);
        assert!((s.word_spacing - 0.1875).abs() < 1e-6);
        let weird = TextSpacing::from_star(f32::NAN, -5.0, 3.0, 0.0);
        assert_eq!(weird.line_height, 1.5);
        assert_eq!(weird.letter_spacing, 0.0);
        assert!(weird.validate().is_ok());
    }

    #[test]
    fn serde_and_summary() {
        let s = TextSpacing::wcag();
        let t = toml::to_string(&s).unwrap();
        assert_eq!(toml::from_str::<TextSpacing>(&t).unwrap(), s);
        let partial: TextSpacing = toml::from_str("line_height = 2.0").unwrap();
        assert_eq!(partial.line_height, 2.0);
        assert_eq!(partial.word_spacing, 0.0);
        assert_eq!(
            s.summary(),
            "Line height 1.5, paragraph spacing 2, letter spacing 0.12, word spacing 0.16 times the font size."
        );
    }
}
