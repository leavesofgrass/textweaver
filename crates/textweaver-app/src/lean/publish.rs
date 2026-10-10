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
        let msg = self.msg("lean-publish-not-in-build");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// The Export as prompt is never opened in this build.
    pub(crate) fn answer_export_as(&mut self, _text: &str) -> Vec<Effect> {
        vec![Effect::Redraw]
    }

    /// The replace question is never asked in this build.
    pub(crate) fn export_replace_confirmed(&mut self, _out: std::path::PathBuf) -> Vec<Effect> {
        vec![Effect::Redraw]
    }

    /// The replace question is never asked in this build.
    pub(crate) fn export_not_replaced(&mut self, _out: std::path::PathBuf) -> Vec<Effect> {
        vec![Effect::Redraw]
    }

    /// The theme question is never asked in this build.
    pub(crate) fn choose_html_theme(
        &mut self,
        _purpose: crate::authoring_state::ThemeFor,
        _names: &[String],
        _n: usize,
    ) -> Vec<Effect> {
        vec![Effect::Redraw]
    }

    /// `preview_in_browser`.
    pub(crate) fn preview_in_browser(&mut self) -> Vec<Effect> {
        let msg = self.msg("lean-publish-not-in-build");
        self.tell(&msg);
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
        let msg = self.msg("lean-publish-not-in-build");
        self.tell(&msg);
    }

    /// `toggle_preview_live`.
    pub(crate) fn toggle_preview_live(&mut self) {
        let msg = self.msg("lean-publish-not-in-build");
        self.tell(&msg);
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
        let msg = self.msg("lean-publish-not-in-build");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Spoken ranges are in the document itself in this build.
    pub(crate) fn map_listened(&self, _generation: ReadingGeneration, r: CharRange) -> CharRange {
        r
    }
}
