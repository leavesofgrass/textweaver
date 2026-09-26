//! Citations in a build without the `publish` feature: every command says
//! it is not in this build, and reading says citations as they are
//! written. The full module is `src/citations.rs`.

// Some of these stand in for items only the full build uses.
#![allow(dead_code)]

use textweaver_core::{CharPos, CharRange};
use textweaver_text::InlineSpeech;

use crate::app::App;
use crate::command::Effect;

/// What the citation commands say in this build.
pub(crate) const NOT_IN_BUILD: &str =
    "Citations are not in this build of textweaver. It was built without the publish feature.";

/// Makes HTTP requests for reference lookups (the full build's is
/// `textweaver_cite::HttpClient`); nothing uses it in this build.
pub trait HttpClient {}

/// A reference found by a lookup; never made in this build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reference;

/// One entry of the citation picker; never made in this build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PickerEntry {
    pub(crate) key: String,
    pub(crate) label: String,
}

/// Libraries loaded for reading citations aloud; never made in this build.
#[derive(Debug)]
pub(crate) struct CachedLibraries;

impl App {
    /// Citations are read as written.
    pub(crate) fn citation_speech_in(&mut self, _range: CharRange) -> Vec<InlineSpeech> {
        Vec::new()
    }

    /// `toggle_citations`.
    pub(crate) fn toggle_citations(&mut self) {
        self.tell(NOT_IN_BUILD);
    }

    /// `insert_citation`.
    pub(crate) fn insert_citation(&mut self) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// A reference chosen in the picker (never shown in this build).
    pub(crate) fn citation_chosen(&mut self, _key: String) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// The locator prompt (never asked in this build).
    pub(crate) fn answer_locator(&mut self, _text: &str) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// Sets the HTTP client for reference lookups; unused in this build.
    pub fn set_citation_client(&mut self, factory: crate::authoring_state::ClientFactory) {
        self.authoring.client = Some(factory);
    }

    /// The DOI or ISBN prompt (never asked in this build).
    pub(crate) fn answer_identifier(&mut self, _text: &str) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// A finished lookup (none start in this build).
    pub(crate) fn lookup_finished(&mut self, _input: &str, _result: Result<Reference, String>) {}

    /// `check_citations`.
    pub(crate) fn check_citations(&mut self) {
        self.tell(NOT_IN_BUILD);
    }

    /// The import prompt (never asked in this build).
    pub(crate) fn answer_import_references(&mut self, _text: &str) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// `insert_bibliography`.
    pub(crate) fn insert_bibliography(&mut self) -> Vec<Effect> {
        self.tell(NOT_IN_BUILD);
        vec![Effect::Redraw]
    }

    /// No citation is described in this build.
    pub(crate) fn citation_description_at(&self, _pos: CharPos) -> Option<String> {
        None
    }
}
