//! The interface language (Wave 4, W4d; ADR-0030): choosing it, and
//! changing it while textweaver runs.
//!
//! - **Live.** The catalog is a value ([`App::catalog`]); a change of
//!   `[interface] language` swaps it at once, and what is said next, the
//!   title line, and every list shown after it are in the new language.
//!   The change is confirmed in the new language, followed by the title
//!   line, so the user hears that it worked.
//! - **A voice for the language.** The voice named for the language in
//!   `[speech] voices_by_language`, else the current voice when it speaks
//!   the language, else the engine's preferred voice for it
//!   ([`textweaver_engines::voice_for_language`]). **When the engine has
//!   no voice for the language, the current voice stays and textweaver
//!   says so: it never goes silent.** The pseudo-locales keep the voice.
//! - **The first run** offers a list of the languages, the system's first
//!   ([`App::language_list`]); the terminal reader shows it when there are
//!   no settings yet.

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::{self, Catalog, LANGUAGES, PSEUDO_ACCENTED, PSEUDO_RTL};
use textweaver_store::Paths;

use crate::app::{App, ListKind};
use crate::command::Effect;

/// The languages offered, as `(tag, name)`: the built-in ones by their
/// names in themselves, with `system` (the system's language, when built
/// in) first.
pub fn language_choices(system: Option<&str>) -> Vec<(String, String)> {
    let first = system.and_then(i18n::language).map(|l| l.tag);
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(tag) = first
        && let Some(l) = LANGUAGES.iter().find(|l| l.tag == tag)
    {
        out.push((l.tag.to_owned(), l.name.to_owned()));
    }
    for l in LANGUAGES {
        if Some(l.tag) != first {
            out.push((l.tag.to_owned(), l.name.to_owned()));
        }
    }
    out
}

impl App {
    /// Puts `[interface] language` into effect: the catalog for it, and a
    /// voice that speaks it (see the module notes). What happened to the
    /// voice is kept in `language_note`, said with the change.
    pub(crate) fn apply_interface_language(&mut self) {
        let lang = self.settings.interface.language.clone();
        let locales = self.paths.as_ref().map(Paths::locales_dir);
        let (catalog, warning) = Catalog::for_language(&lang, locales.as_deref());
        self.study.catalog = catalog;
        let mut notes: Vec<String> = Vec::new();
        if let Some(w) = warning {
            log::warn!("{w}");
            notes.push(w);
        }
        if let Some(n) = self.follow_language_voice(&lang) {
            notes.push(n);
        }
        self.language_note = (!notes.is_empty()).then(|| notes.join(" "));
    }

    /// Changes the voice to one for `lang`, when the engine has one and
    /// the current voice does not speak it. Returns what to say: the new
    /// voice, or that the current voice stays. `None` when nothing needs
    /// saying (the voice already speaks it, or the list is not ready).
    fn follow_language_voice(&mut self, lang: &str) -> Option<String> {
        if lang.eq_ignore_ascii_case(PSEUDO_ACCENTED) || lang.eq_ignore_ascii_case(PSEUDO_RTL) {
            return None;
        }
        // Most engines list their voices in milliseconds; SAPI can take
        // longer, in the background. A language change is rare, so it may
        // wait a moment for the list; if it is still coming, the voice
        // stays and that is said.
        let list = self
            .speech
            .voice_cache()
            .wait(std::time::Duration::from_millis(1500));
        if list.is_loading() {
            return Some(self.msg("language-voices-loading"));
        }
        let voices = list.into_result().ok()?;
        if voices.is_empty() {
            return None;
        }
        let current = self.settings.speech.voice.clone();
        let language = i18n::language(lang).map_or_else(|| lang.to_owned(), |l| l.name.to_owned());
        match textweaver_engines::voice_for_language(
            &voices,
            lang,
            &self.settings,
            current.as_deref(),
        ) {
            Some(v) if current.as_deref() == Some(v.id.as_str()) => None,
            Some(v) => {
                let (id, name) = (v.id.clone(), v.name.clone());
                self.settings.speech.voice = Some(id.clone());
                self.settings_dirty = true;
                self.speech.set_voice(Some(id));
                Some(self.msg_args(
                    "language-voice-changed",
                    &args!["voice" => name, "language" => language],
                ))
            }
            None => {
                let name = current
                    .as_deref()
                    .and_then(|id| voices.iter().find(|v| v.id == id))
                    .map_or_else(|| self.backend_name.clone(), |v| v.name.clone());
                Some(self.msg_args(
                    "language-voice-kept",
                    &args!["voice" => name, "language" => language],
                ))
            }
        }
    }

    /// The list of interface languages, the system's first, with the one
    /// in use focused ("Español"); Enter changes to the one chosen, at
    /// once. Frontends show it on the first run.
    pub fn language_list(&mut self) -> Vec<Effect> {
        let system = crate::words::system_language();
        let choices = language_choices(system.as_deref());
        let (tags, items): (Vec<String>, Vec<String>) = choices.into_iter().unzip();
        let n = items.len();
        self.list = Some(ListKind::Languages(tags));
        let intro = self.msg_args("language-list-intro", &args!["n" => n]);
        self.tell(&intro);
        vec![Effect::ShowList {
            title: self.msg("language-list-title"),
            items,
        }]
    }

    /// Enter in the language list.
    pub(crate) fn choose_language(&mut self, tags: &[String], n: usize) -> Vec<Effect> {
        match tags.get(n) {
            Some(tag) => self.set_interface_language(tag),
            None => vec![Effect::Redraw],
        }
    }

    /// Changes the interface language to `tag` at once, saves it, and
    /// says so in the new language, then the title line.
    pub fn set_interface_language(&mut self, tag: &str) -> Vec<Effect> {
        self.set_setting_command("interface.language", serde_json::Value::String(tag.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_language_comes_first() {
        let c = language_choices(Some("fr_CA.UTF-8"));
        assert_eq!(c[0], ("fr".to_owned(), "Français".to_owned()));
        assert_eq!(c.len(), LANGUAGES.len());
        assert_eq!(c[1].0, "en");
        let c = language_choices(Some("ja-JP"));
        assert_eq!(c[0].0, "en");
        assert_eq!(language_choices(None)[0].1, "English");
    }
}
