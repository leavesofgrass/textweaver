//! The title bar and menus follow the theme (W8a-m): one rule, the
//! effective palette's background, with the system's high contrast mode
//! winning; and `gui.auto_hide_menu`, the setting that hides the menu bar
//! until Alt or F10.

use textweaver_app::SettingKind;
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::store::sync_scope::{SettingScope, setting_scope};
use textweaver_theme::ThemeKind;
use textweaver_xilem::dark_mode::{self, Chrome};
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::system_colors;
use textweaver_xilem::theme::Palette;

/// Galaxy, the default, is dark: so are its title bar and menus.
#[test]
fn galaxy_gets_a_dark_frame() {
    let p = Palette::galaxy();
    assert_eq!(dark_mode::chrome(p.background, false), Chrome::Dark);
    assert_eq!(
        Chrome::Dark.window_theme(),
        Some(masonry_winit::winit::window::Theme::Dark)
    );
}

/// Every built-in theme: a dark theme gets a dark frame, a light theme a
/// light one, and a high-contrast theme whichever its page is.
#[test]
fn every_theme_follows_its_page() {
    for theme in textweaver_theme::builtin::all() {
        let p = Palette::from_theme(theme);
        let frame = dark_mode::chrome(p.background, false);
        let want = match theme.kind() {
            ThemeKind::Dark => Chrome::Dark,
            ThemeKind::Light => Chrome::Light,
            ThemeKind::HighContrast if p.background.is_dark() => Chrome::Dark,
            ThemeKind::HighContrast => Chrome::Light,
        };
        assert_eq!(frame, want, "{}", theme.name());
    }
}

/// The system's high contrast mode always wins: the system draws the
/// frame in the person's colors, whatever textweaver's theme.
#[test]
fn high_contrast_always_wins() {
    for theme in textweaver_theme::builtin::all() {
        let p = Palette::from_theme(theme);
        assert_eq!(
            dark_mode::chrome(p.background, true),
            Chrome::System,
            "{}",
            theme.name()
        );
    }
    let night = system_colors::palette(&system_colors::NIGHT_SKY);
    assert_eq!(dark_mode::chrome(night.background, true), Chrome::System);
    assert_eq!(Chrome::System.window_theme(), None);
}

/// `gui.auto_hide_menu`: off by default, a toggle in the Window section
/// with its help in English, kept on this computer, and set like any
/// other setting.
#[test]
fn auto_hide_menu_is_a_machine_toggle() {
    let home = tempfile::tempdir().expect("a temporary folder");
    let opts = Options {
        no_speech: true,
        home: Some(home.path().to_path_buf()),
        ..Options::default()
    };
    let mut app = setup::build_app(&opts, Box::new(LogAnnouncer::default())).0;
    assert!(!app.settings().gui.auto_hide_menu, "off by default");
    let schema = app.settings_schema();
    let s = schema.get("gui.auto_hide_menu").expect("in the schema");
    assert_eq!(s.kind, SettingKind::Toggle);
    assert!(!s.internal, "on the settings screen");
    assert_eq!(s.default, serde_json::json!(false));
    let c = app.catalog();
    assert_eq!(s.label_in(&c), "Hide the menu bar");
    assert!(s.help_in(&c).contains("Alt or F10"), "{}", s.help_in(&c));
    assert!(s.help_in(&c).contains("macOS"), "{}", s.help_in(&c));
    assert_eq!(
        setting_scope("gui.auto_hide_menu"),
        Some(SettingScope::Machine)
    );
    let said = app.set_setting("gui.auto_hide_menu", serde_json::json!(true));
    assert!(said.is_ok(), "{said:?}");
    assert!(app.settings().gui.auto_hide_menu);
}
