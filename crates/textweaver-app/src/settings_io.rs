//! The `export_settings` and `import_settings` actions (in the command
//! palette, and on Alt+Shift+E and Alt+Shift+I): settings and key
//! overrides to and from a JSON (or TOML) file, through
//! `textweaver_store::settings_io`. Both prompt for a path; import asks
//! "Import N changed settings from FILE? y or n" and then applies the new
//! settings and keys at once.

use std::path::PathBuf;

use textweaver_keymap::{Frontend, Keymap, Platform};
use textweaver_store::{
    ExportFormat, ExportOptions, ImportMode, ImportPlan, SettingsStore, atomic_write,
};

use crate::app::App;
use crate::command::{Confirm, Effect, PromptPurpose};

/// The path typed at a prompt, without surrounding quotes; `None` if empty.
fn answer_path(text: &str) -> Option<PathBuf> {
    let t = text.trim().trim_matches('"');
    (!t.is_empty()).then(|| PathBuf::from(t))
}

/// The question asked before an import.
fn import_question(plan: &ImportPlan, name: &str) -> String {
    let n = plan.change_count().max(1);
    let noun = if n == 1 { "setting" } else { "settings" };
    format!("Import {n} changed {noun} from {name}? y or n")
}

impl App {
    /// `export_settings` or `import_settings`: asks for the file.
    pub(crate) fn settings_file_prompt(&mut self, import: bool) -> Vec<Effect> {
        self.prompt(if import {
            PromptPurpose::ImportSettings
        } else {
            PromptPurpose::ExportSettings
        })
    }

    /// The settings store, after saving any unsaved change so the files
    /// match what is in effect. `None` (announced) without persistence.
    fn settings_store_for_io(&mut self) -> Option<SettingsStore> {
        let Some(paths) = self.paths.clone() else {
            self.error(
                "Settings are not saved in this session, so they cannot be exported or imported.",
            );
            return None;
        };
        if let Err(e) = self.save_settings() {
            self.error(&format!("Could not save settings: {e}"));
            return None;
        }
        Some(SettingsStore::new(paths))
    }

    /// The answer to the export prompt: writes every setting and key
    /// override to the file, as TOML when its name ends in `.toml`.
    pub(crate) fn answer_export_settings(&mut self, text: &str) -> Vec<Effect> {
        let Some(path) = answer_path(text) else {
            self.note("Cancelled.");
            return vec![Effect::Redraw];
        };
        let Some(store) = self.settings_store_for_io() else {
            return vec![Effect::Redraw];
        };
        let options = ExportOptions {
            changed_only: false,
            format: ExportFormat::for_path(&path),
        };
        let written = store
            .export(options)
            .map_err(|e| e.to_string())
            .and_then(|t| atomic_write(&path, t.as_bytes()).map_err(|e| e.to_string()));
        match written {
            Ok(()) => self.tell(&format!("Settings exported to {}.", path.display())),
            Err(e) => self.error(&format!("Could not export settings: {e}")),
        }
        vec![Effect::Redraw]
    }

    /// The answer to the import prompt: reads and checks the file, then
    /// asks for a yes or no. Nothing changes until the answer is yes.
    pub(crate) fn answer_import_settings(&mut self, text: &str) -> Vec<Effect> {
        let Some(path) = answer_path(text) else {
            self.note("Cancelled.");
            return vec![Effect::Redraw];
        };
        let Some(store) = self.settings_store_for_io() else {
            return vec![Effect::Redraw];
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                self.error(&format!("Could not read {}: {e}.", path.display()));
                return vec![Effect::Redraw];
            }
        };
        let plan = match store.plan_import(&content, ImportMode::Merge) {
            Ok(p) => p,
            Err(e) => {
                self.error(&e.to_string());
                return vec![Effect::Redraw];
            }
        };
        if plan.is_empty() {
            self.tell("Nothing to import: your settings already match that file.");
            return vec![Effect::Redraw];
        }
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let question = import_question(&plan, &name);
        self.pending_import = Some((plan, name));
        self.tell(&question);
        vec![Effect::Redraw]
    }

    /// Answers the import question ([`App::confirmation_pending`]).
    pub(crate) fn confirm_import(&mut self, answer: Confirm) -> Vec<Effect> {
        match answer {
            Confirm::Yes => {
                if let Some((plan, _)) = self.pending_import.take() {
                    self.apply_import(plan);
                }
            }
            Confirm::No => {
                self.pending_import = None;
                self.tell("Cancelled. Nothing was changed.");
            }
            Confirm::Repeat => {
                if let Some((plan, name)) = &self.pending_import {
                    let q = import_question(plan, name);
                    self.tell(&q);
                }
            }
        }
        vec![Effect::Redraw]
    }

    /// Writes the plan (with backups) and puts the new settings and keys
    /// into effect.
    fn apply_import(&mut self, plan: ImportPlan) {
        let Some(store) = self.settings_store_for_io() else {
            return;
        };
        let applied = match store.apply(&plan) {
            Ok(a) => a,
            Err(e) => {
                self.error(&format!("Could not import settings: {e}"));
                return;
            }
        };
        self.settings = plan.settings.clone();
        self.settings_dirty = false;
        self.apply_voice_settings();
        // apply_voice_settings leaves the voice alone when none is set.
        self.speech.set_voice(self.settings.speech.voice.clone());
        let (mut keymap, key_warnings) = Keymap::with_preset_and_overrides(
            Platform::current(),
            Frontend::Terminal,
            crate::access::keymap_preset(self.settings.keyboard.preset),
            &plan.keymap,
        );
        self.access_mode =
            crate::access::access_mode_from_setting(self.settings.accessibility.mode);
        keymap.set_character_keys(self.settings.keyboard.character_keys);
        self.keymap = keymap;
        // Library folders and sync policy, and the theme name, may have
        // changed too.
        self.library_sync = Self::make_library_sync(&self.settings);
        self.check_theme_name();
        for w in plan.warnings.iter().chain(&key_warnings) {
            self.note(w);
        }
        let mut msg = format!("Settings imported. {}", plan.summary());
        if plan
            .changes
            .iter()
            .any(|c| c.path.starts_with("speech.backend"))
        {
            msg.push_str(" The new speech backend is used from the next start.");
        }
        if !applied.backups.is_empty() {
            msg.push_str(" The old settings were backed up.");
        }
        self.tell(&msg);
    }
}

#[cfg(test)]
mod tests {
    use textweaver_store::Paths;

    use super::*;
    use crate::{AppConfig, Command};

    fn app(dir: &std::path::Path) -> App {
        let mut config = AppConfig::for_tests();
        config.paths = Some(Paths::under(dir));
        App::new(config)
    }

    fn palette(app: &mut App, name: &str, answer: &str) {
        app.dispatch(Command::Action(textweaver_keymap::ActionId::CommandPalette));
        app.dispatch(Command::Answer(name.into()));
        app.dispatch(Command::Answer(answer.into()));
    }

    #[test]
    fn export_then_import_with_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mine.json");
        let mut a = app(&dir.path().join("a"));
        a.settings.speech.rate = textweaver_core::Rate::Wpm(320);
        a.settings_dirty = true;
        palette(&mut a, "export settings", file.to_str().unwrap());
        assert!(
            a.status_text().starts_with("Settings exported to "),
            "{}",
            a.status_text()
        );
        assert!(
            std::fs::read_to_string(&file)
                .unwrap()
                .contains("\"rate\": 320")
        );

        let mut b = app(&dir.path().join("b"));
        palette(&mut b, "import_settings", file.to_str().unwrap());
        assert_eq!(
            b.status_text(),
            "Import 1 changed setting from mine.json? y or n"
        );
        assert!(b.confirmation_pending());
        b.dispatch(Command::Confirm(Confirm::Repeat));
        assert_eq!(
            b.status_text(),
            "Import 1 changed setting from mine.json? y or n"
        );
        b.dispatch(Command::Confirm(Confirm::Yes));
        assert!(!b.confirmation_pending());
        assert_eq!(b.settings().speech.rate.wpm(), 320, "applied live");
        assert_eq!(b.status_text(), "Settings imported. 1 setting changes.");
        let saved = SettingsStore::new(Paths::under(&dir.path().join("b")))
            .load()
            .0;
        assert_eq!(saved.speech.rate.wpm(), 320, "and saved");
    }

    #[test]
    fn no_cancels_and_bad_files_are_announced() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("in.json");
        std::fs::write(
            &file,
            r#"{"textweaver_settings": 1, "settings": {"speech": {"rate": 300}}}"#,
        )
        .unwrap();
        let mut a = app(&dir.path().join("a"));
        palette(&mut a, "import_settings", file.to_str().unwrap());
        a.dispatch(Command::Confirm(Confirm::No));
        assert_eq!(a.status_text(), "Cancelled. Nothing was changed.");
        assert_eq!(a.settings().speech.rate.wpm(), 265);

        std::fs::write(
            &file,
            r#"{"textweaver_settings": 1, "settings": {"speech": {"rate": "fast"}}}"#,
        )
        .unwrap();
        palette(&mut a, "import_settings", file.to_str().unwrap());
        assert!(
            a.status_text().contains("settings.speech.rate"),
            "{}",
            a.status_text()
        );
        assert!(!a.confirmation_pending());
    }
}
