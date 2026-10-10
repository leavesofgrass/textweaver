//! Braille (BRF) files read as print (`textweaver_formats::brf`): the
//! original braille of the page at the cursor, as a list of its lines, and
//! the message when liblouis is missing and the file shows as braille.

use textweaver_core::MarkerKind;
use textweaver_formats::Source;
use textweaver_formats::brf::{TRANSLATION_PROPERTY, UNTRANSLATED, original_pages};
use textweaver_lexicon::args;
use textweaver_text::Document;

use crate::app::{App, ListKind};
use crate::command::Effect;

/// True when `doc` is a BRF file shown as braille because liblouis could
/// not run.
pub(crate) fn untranslated_braille(doc: &Document) -> bool {
    doc.meta.format == "brf"
        && doc
            .meta
            .properties
            .get(TRANSLATION_PROPERTY)
            .map(String::as_str)
            == Some(UNTRANSLATED)
}

impl App {
    /// After opening a BRF file without liblouis: one question to fetch
    /// liblouis, after which the file opens again as print, when this
    /// computer can have it; else what happened and how to read it as
    /// print (with the package command to copy, when it is known), once.
    pub(crate) fn say_braille_untranslated(&mut self, path: &std::path::Path) {
        if self.offer_liblouis_for(path.to_owned()) {
            return;
        }
        let msg = match self.helper_command_text(crate::components::Helper::Liblouis) {
            Some(command) => format!("{} {command}", self.msg("brf-no-liblouis-short")),
            None => self.msg("brf-no-liblouis"),
        };
        self.tell(&msg);
    }

    /// `show_original_braille`: the braille page at the cursor, one line per
    /// item, in Unicode braille, read again from the file. A braille
    /// display shows the cells as they are; Escape goes back.
    pub(crate) fn show_original_braille(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let Some(path) = s
            .doc
            .meta
            .path
            .clone()
            .filter(|_| s.doc.meta.format == "brf")
        else {
            let msg = self.msg("brf-original-not-brf");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        // The braille page the cursor is on: the last page marker that
        // starts at or before it, else the first page.
        let cursor = s.cursor;
        let pages: Vec<_> = s
            .doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::PageBreak)
            .collect();
        let marker = pages
            .iter()
            .filter(|m| m.range.start <= cursor)
            .max_by_key(|m| m.range.start)
            .or(pages.first());
        let index = marker
            .and_then(|m| m.reference.as_deref())
            .and_then(|r| r.parse::<usize>().ok())
            .map_or(0, |n| n.saturating_sub(1));
        let label = marker
            .and_then(|m| m.label.clone())
            .unwrap_or_else(|| (index + 1).to_string());
        let bytes = match Source::Path(path).read() {
            Ok(b) => b,
            Err(e) => {
                let msg =
                    self.msg_args("brf-original-unreadable", &args!["reason" => e.to_string()]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
        };
        let Some(all) = original_pages(&bytes) else {
            let msg = self.msg("brf-original-not-brf");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let items: Vec<String> = all
            .get(index)
            .map(|lines| {
                lines
                    .iter()
                    .filter(|l| !l.trim().is_empty())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let title = self.msg_args("brf-original-title", &args!["page" => label.as_str()]);
        let intro = self.msg_args(
            "brf-original-intro",
            &args!["page" => label.as_str(), "n" => items.len()],
        );
        self.list = Some(ListKind::Info);
        self.tell(&intro);
        vec![Effect::ShowList { title, items }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppConfig, Command};
    use textweaver_keymap::ActionId;

    fn brf_fixture() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/r5/river-ueb.brf")
    }

    #[test]
    fn the_original_braille_of_the_cursor_page_is_listed() {
        let mut app = App::new(AppConfig::for_tests());
        let _ = app.open_now(&brf_fixture());
        let doc = &app.session().unwrap().doc;
        let second = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::PageBreak)
            .nth(1)
            .unwrap()
            .range
            .start;
        app.set_cursor(second);
        let effects = app.dispatch(Command::Action(ActionId::ShowOriginalBraille));
        let (title, items) = effects
            .into_iter()
            .find_map(|e| match e {
                Effect::ShowList { title, items } => Some((title, items)),
                _ => None,
            })
            .unwrap();
        assert_eq!(title, "Original Braille, page 2");
        // The page's text lines and its braille page number, as cells.
        assert_eq!(items.len(), 5, "{items:?}");
        assert!(items[0].trim_start().starts_with('\u{2820}'), "{items:?}");
    }

    #[test]
    fn other_documents_say_it_needs_a_braille_file() {
        let mut app = App::new(AppConfig::for_tests());
        let mut doc = Document::from_plain_text("Hello.");
        doc.meta.format = "text".into();
        let _ = app.open_document(doc, textweaver_store::DocKey("x".into()), "x".into());
        let effects = app.dispatch(Command::Action(ActionId::ShowOriginalBraille));
        assert!(
            !effects.iter().any(|e| matches!(e, Effect::ShowList { .. })),
            "{effects:?}"
        );
    }
}
