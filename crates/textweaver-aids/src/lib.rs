//! Reading aids for textweaver, carried forward from star.
//!
//! Everything here is pure data in, data out: no threads, no clocks, no
//! terminal or GUI types. The terminal UI and the GUI render the results;
//! the tests drive them deterministically.
//!
//! - [`rsvp`]: rapid serial visual presentation. One word at a time, with an
//!   optimal recognition point, words-per-minute timing with pauses at
//!   punctuation and long words, context words, star's nine screen positions,
//!   and seeking by word, sentence, and paragraph. A state machine driven by
//!   a clock value the caller passes in, or by speech word events.
//! - [`bionic`]: which leading letters of each word to embolden.
//! - [`syllables`]: `read·a·bil·i·ty` for display, with an
//!   [`OffsetMap`](textweaver_core::OffsetMap) back to canonical positions
//!   (ADR-0005) so highlights stay exact.
//! - [`level`]: Flesch-Kincaid grade and Flesch reading ease.
//! - [`spacing`]: WCAG 1.4.12 text spacing settings, checks, and CSS.
//! - [`fonts`]: font family, size, and weight, with the reading fonts star
//!   offered (OpenDyslexic, Atkinson Hyperlegible, Lexend), fallbacks, and CSS.
//!   Choosing and resolving a family is `textweaver-fonts`' job; this crate
//!   re-exports those types and adds text spacing.
//! - [`ruler`]: the reading ruler and current-line band as row marks.
//! - [`difficult`]: rare-word marking, with SCOWL's word levels built in
//!   (no download) or any word-frequency list.
//! - [`definitions`]: a difficult word's first definition, short enough to
//!   say after it, from any dictionary the caller passes (ADR-0037).
//! - [`html`]: wraps ranges of text in HTML tags, escaping the rest.
//! - [`settings`]: conversions from and to the saved `[reading_aids]`
//!   settings (`textweaver_store::reading_aids`).
//!
//! Positions are canonical [`CharPos`](textweaver_core::CharPos) values
//! (ADR-0002). Words come from `textweaver-text`'s word units, so every aid
//! agrees with navigation and speech about where a word starts and ends.
//! See `docs/adr/0022-reading-aids.md` and `docs/reading-aids.md`.

pub mod bionic;
pub mod definitions;
pub mod difficult;
pub mod fonts;
pub mod html;
pub mod level;
pub mod rsvp;
pub mod ruler;
pub mod settings;
pub mod spacing;
pub mod syllables;
mod util;

pub use bionic::{BionicOptions, bionic_range, bionic_text, fixation_len};
pub use definitions::{DEFINITION_MAX_CHARS, Definitions, difficult_definition, short_definition};
pub use difficult::{
    Commonness, DifficultOptions, FrequencyList, ScowlList, WordList, difficult_range,
    difficult_text,
};
pub use fonts::{
    FontDescription, FontFamily, FontSettings, Platform, READING_FONTS, ReadingFont, ReadingFontId,
    describe as describe_font, to_css as font_css,
};
pub use level::{GradeBand, ReadingLevel, count_syllables, reading_level, reading_level_text};
pub use rsvp::{
    Millis, Pacing, Rsvp, RsvpEvent, RsvpFrame, RsvpPosition, RsvpSettings, Tick, WordTrack,
    optimal_recognition_point,
};
pub use ruler::{RowMark, RulerMode, RulerScope, RulerSettings, TermStyle, ViewRow, ruler_rows};
pub use spacing::{SpacingError, SpacingIssue, TerminalSpacing, TextSpacing};
pub use syllables::{SplitText, SyllableOptions, split_range, split_text, split_word};
