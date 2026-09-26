//! Export and preview in a build without the `publish` feature: the
//! commands say they are not in this build. The full module is
//! `src/publish.rs`; saving still announces possible misspellings.

// Some of these stand in for items only the full build uses.
#![allow(dead_code)]

use textweaver_core::CharRange;
use textweaver_speech::ReadingGeneration;

use crate::app::App;
use crate::authoring_state::{ExportDone, ExportKind};
use crate::command::Effect;

/// What the export and preview commands say in this build.
pub(crate) const NOT_IN_BUILD: &str = "Export and preview are not in this build of textweaver. It was built without the publish feature; tw convert still converts.";

/// The formats the reader exports to (the full build's is
/// `textweaver_convert::OutputFormat`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    /// HTML.
    Html,
    /// Tagged PDF.
    Pdf,
    /// Word.
    Docx,
    /// EPUB 3.
    Epub,
    /// Braille.
    Brf,
}

impl App {
    /// `export_html`, `export_pdf`, and the rest.
    pub(crate) fn export_to(&mut self, _to: OutputFormat) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// `preview_in_browser`.
    pub(crate) fn preview_in_browser(&mut self) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// Progress of an export (none start in this build).
    pub(crate) fn export_progress(&mut self, _kind: ExportKind, _what: &str, _secs: u64) {}

    /// Nothing to close: no preview runs in this build.
    pub(crate) fn close_preview(&mut self) {
        self.preview_server = None;
        self.authoring.preview = None;
    }

    /// `toggle_preview_auto_reload`.
    pub(crate) fn toggle_preview_auto_reload(&mut self) {
        self.tell(NOT_IN_BUILD);
    }

    /// `toggle_preview_live`.
    pub(crate) fn toggle_preview_live(&mut self) {
        self.tell(NOT_IN_BUILD);
    }

    /// No live preview in this build.
    pub(crate) fn live_preview_tick(&mut self, _now: std::time::Instant) {}

    /// A finished export (none start in this build).
    pub(crate) fn export_finished(
        &mut self,
        _what: &str,
        _kind: ExportKind,
        _result: Result<ExportDone, String>,
    ) {
    }

    /// After a successful save in edit mode: says how many possible
    /// misspellings the document has.
    pub(crate) fn on_saved(&mut self) {
        self.count_misspellings_in_background();
    }

    /// `listen_rendered`.
    pub(crate) fn listen_rendered(&mut self) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// Spoken ranges are in the document itself in this build.
    pub(crate) fn map_listened(&self, _generation: ReadingGeneration, r: CharRange) -> CharRange {
        r
    }
}
