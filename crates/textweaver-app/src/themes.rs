//! Themes (ADR-0020, `textweaver-theme`): the built-in palettes (Star's 23,
//! Galaxy first and the default) plus the user's own from the config
//! folder's `themes/`, the `next_theme` cycle, and following the system's
//! light, dark, or high-contrast setting at startup.

use textweaver_lexicon::args;
use textweaver_theme::os::{self, OsScheme};
use textweaver_theme::{Registry, Theme};

use crate::app::App;

impl App {
    /// Loads user themes and checks the configured name. Problems with user
    /// theme files are spoken ([`LoadReport::summary`]); an unknown theme
    /// name falls back to Galaxy and says so (Star fell back silently).
    ///
    /// [`LoadReport::summary`]: textweaver_theme::LoadReport::summary
    pub(crate) fn load_themes(&mut self) {
        if let Some(dir) = self.paths.as_ref().map(|p| p.themes_dir())
            && dir.is_dir()
        {
            let report = self.themes.load_dir(&dir);
            if !report.is_clean() {
                self.tell(&report.summary());
            }
        }
        self.check_theme_name();
        self.check_reading_colors();
    }

    /// Says so when the configured theme does not exist (Galaxy is used).
    pub(crate) fn check_theme_name(&mut self) {
        let name = self.settings.display.theme.clone();
        let (theme, fell_back) = self.themes.resolve(&name);
        if fell_back {
            let used = theme.meta.display_name.clone();
            let msg = self.msg_args(
                "themes-unknown",
                &args!["name" => name.as_str(), "used" => used.as_str()],
            );
            self.tell(&msg);
        }
    }

    /// Every theme: built-ins in Star's order, then the user's.
    pub fn theme_registry(&self) -> &Registry {
        &self.themes
    }

    /// The theme in effect: the configured one, or Galaxy when there is no
    /// theme by that name, as stored (without the reader's highlight
    /// colours; see [`reading_theme`](Self::reading_theme)).
    pub fn current_theme(&self) -> &Theme {
        self.themes.resolve(&self.settings.display.theme).0
    }

    /// The theme to draw with: [`current_theme`](Self::current_theme) with
    /// `[highlight] color` and `sentence_color` laid over its spoken-word
    /// and spoken-sentence styles ([`textweaver_theme::reading`]).
    pub fn reading_theme(&self) -> Theme {
        let mut theme = self.current_theme().clone();
        let h = &self.settings.highlight;
        textweaver_theme::reading::apply_reading_colors(
            &mut theme,
            &h.color,
            h.sentence_color.as_deref(),
        );
        // `[colors]` over the rest (crate::colors).
        self.apply_color_settings(&mut theme);
        theme
    }

    /// What decides [`reading_theme`](Self::reading_theme): the theme name,
    /// the two highlight colors, and `[colors]`. Frontends rebuild their
    /// styles when it changes.
    pub fn reading_theme_key(&self) -> (String, String, Option<String>, Vec<String>) {
        let h = &self.settings.highlight;
        (
            self.current_theme().meta.name.clone(),
            h.color.clone(),
            h.sentence_color.clone(),
            self.color_settings_key(),
        )
    }

    /// Says what is wrong with the highlight colours on the current theme:
    /// an unknown colour, or too little contrast.
    pub(crate) fn check_reading_colors(&mut self) {
        let mut theme = self.current_theme().clone();
        let h = &self.settings.highlight;
        let report = textweaver_theme::reading::apply_reading_colors(
            &mut theme,
            &h.color,
            h.sentence_color.as_deref(),
        );
        for w in report.warnings {
            self.tell(&w);
        }
    }

    /// At startup, follows the system's scheme when `display.follow_os_theme`
    /// is on and no theme was chosen explicitly (Star's rule). The switch is
    /// not an explicit choice and is not saved on its own, so the next
    /// launch follows again. Returns the new theme's name, if it changed.
    pub fn apply_startup_theme(&mut self, scheme: OsScheme) -> Option<String> {
        let d = &self.settings.display;
        let name = os::startup_theme(
            self.themes.facts(&d.theme),
            d.follow_os_theme,
            d.theme_explicit,
            scheme,
        )?;
        let theme = self.themes.get(&name)?.meta.name.clone();
        if theme == self.settings.display.theme {
            return None;
        }
        self.settings.display.theme = theme.clone();
        Some(theme)
    }

    /// F5: the next theme in the cycle. Picking a theme is an explicit
    /// choice (it stops following the system) and is saved.
    pub(crate) fn next_theme(&mut self) {
        let next = self.themes.next(&self.settings.display.theme);
        let (name, spoken) = (next.meta.name.clone(), next.meta.display_name.clone());
        self.settings.display.theme = name;
        self.settings.display.theme_explicit = true;
        self.settings_dirty = true;
        let msg = self.msg_args("themes-next", &args!["theme" => spoken.as_str()]);
        self.tell(&msg);
        self.check_reading_colors();
    }
}

#[cfg(test)]
mod tests {
    use crate::AppConfig;

    use super::*;

    #[test]
    fn galaxy_by_default_and_unknown_names_fall_back_with_a_notice() {
        let app = App::new(AppConfig::for_tests());
        assert_eq!(app.current_theme().meta.name, "galaxy");
        assert_eq!(app.theme_registry().names().len(), 23);

        let mut config = AppConfig::for_tests();
        config.settings.display.theme = "neon".into();
        let app = App::new(config);
        assert_eq!(app.current_theme().meta.name, "galaxy");
        assert_eq!(
            app.status_text(),
            "There is no theme called neon; using Galaxy."
        );
    }

    /// `[highlight] color` and `sentence_color` reach the drawn theme, and a
    /// colour with poor contrast is reported when the app starts.
    #[test]
    fn highlight_colours_are_laid_over_the_theme() {
        use textweaver_theme::{Rgb, StyleRole};
        let app = App::new(AppConfig::for_tests());
        assert_eq!(app.reading_theme(), *app.current_theme(), "theme default");

        let mut config = AppConfig::for_tests();
        config.settings.highlight.color = "yellow".into();
        config.settings.highlight.sentence_color = Some("#003355".into());
        let app = App::new(config);
        let t = app.reading_theme();
        assert_eq!(
            t.style(StyleRole::SpokenWord).background,
            Some(Rgb::from_u32(0xffff00))
        );
        assert_eq!(
            t.style(StyleRole::SpokenSentence).background,
            Some(Rgb::from_u32(0x003355))
        );
        assert_eq!(app.reading_theme_key().1, "yellow");
        assert!(
            !app.status_text().contains("contrast"),
            "{}",
            app.status_text()
        );

        let mut config = AppConfig::for_tests();
        config.settings.highlight.color = "#777777".into();
        let app = App::new(config);
        assert!(
            app.status_text().contains("leaves its text at"),
            "{}",
            app.status_text()
        );
    }

    #[test]
    fn startup_follows_the_system_unless_a_theme_was_picked() {
        let mut app = App::new(AppConfig::for_tests());
        assert_eq!(
            app.apply_startup_theme(OsScheme::Light).as_deref(),
            Some("galaxy-light")
        );
        assert_eq!(app.apply_startup_theme(OsScheme::Light), None, "no change");
        assert_eq!(
            app.apply_startup_theme(OsScheme::HighContrast).as_deref(),
            Some("high-contrast")
        );

        let mut config = AppConfig::for_tests();
        config.settings.display.theme_explicit = true;
        let mut app = App::new(config);
        assert_eq!(app.apply_startup_theme(OsScheme::Light), None);
        let mut config = AppConfig::for_tests();
        config.settings.display.follow_os_theme = false;
        let mut app = App::new(config);
        assert_eq!(app.apply_startup_theme(OsScheme::Light), None);
        assert_eq!(app.current_theme().meta.name, "galaxy");
    }

    /// One F5 stops following the system, and turning "Follow the system
    /// theme" off and on again follows it once more (walkthroughs QW5).
    #[test]
    fn following_again_undoes_a_picked_theme() {
        let mut app = App::new(AppConfig::for_tests());
        app.next_theme();
        assert!(app.settings().display.theme_explicit);
        let picked = app.settings().display.theme.clone();
        assert_eq!(app.apply_startup_theme(OsScheme::HighContrast), None);
        app.set_setting("display.follow_os_theme", serde_json::json!(false))
            .unwrap();
        assert!(app.settings().display.theme_explicit, "off keeps the pick");
        app.set_setting("display.follow_os_theme", serde_json::json!(true))
            .unwrap();
        assert!(!app.settings().display.theme_explicit);
        assert_eq!(
            app.settings().display.theme,
            picked,
            "no change until startup"
        );
        assert_eq!(
            app.apply_startup_theme(OsScheme::HighContrast).as_deref(),
            Some("high-contrast")
        );
    }

    #[test]
    fn user_themes_load_and_problems_are_spoken() {
        let home = tempfile::tempdir().unwrap();
        let paths = textweaver_store::Paths::under(home.path());
        std::fs::create_dir_all(paths.themes_dir()).unwrap();
        std::fs::write(
            paths.themes_dir().join("ocean.toml"),
            "[theme]\nname = \"ocean\"\ndisplay_name = \"Ocean\"\ninherits = \"galaxy\"\n",
        )
        .unwrap();
        std::fs::write(paths.themes_dir().join("broken.toml"), "not = [toml").unwrap();
        let mut config = AppConfig::for_tests();
        config.paths = Some(paths);
        config.settings.display.theme = "ocean".into();
        let app = App::new(config);
        assert!(
            app.theme_registry().get("ocean").is_some(),
            "{}",
            app.status_text()
        );
        assert_eq!(app.current_theme().meta.name, "ocean");
        assert!(
            app.status_text().contains("broken.toml was not loaded"),
            "{}",
            app.status_text()
        );
    }
}
