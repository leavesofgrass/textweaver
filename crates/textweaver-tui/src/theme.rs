//! Terminal styles from the shared themes (ADR-0020, `textweaver-theme`):
//! each [`TerminalTheme`] role resolved at the terminal's detected color
//! level (truecolor, 256, 16, or none) and turned into ratatui styles.
//! Every highlight carries a text attribute (bold, underline, italic, or
//! reverse), so no state is shown by color alone, even with color off.

use ratatui::style::{Color, Modifier, Style};
use textweaver_app::HighlightKind;
use textweaver_theme::{
    Attrs, ColorRole, ColorSupport, Registry, StyleRole, TermColor, TermStyle, TerminalTheme,
};

/// Styles for every part of the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Theme name as stored in settings (`galaxy`).
    pub name: String,
    /// Name read aloud (`Galaxy`).
    pub display_name: String,
    /// The color level the styles were resolved for.
    pub support: ColorSupport,
    /// Document text on the page.
    pub text: Style,
    /// Title line.
    pub title: Style,
    /// Status line.
    pub status: Style,
    /// Key hints line.
    pub hints: Style,
    /// Minibuffer (prompt) line.
    pub minibuffer: Style,
    /// Line numbers.
    pub gutter: Style,
    /// Word being spoken.
    pub spoken_word: Style,
    /// Sentence being spoken.
    pub spoken_sentence: Style,
    /// Search matches.
    pub find_hit: Style,
    /// The search match at the cursor.
    pub current_hit: Style,
    /// Bookmarked words.
    pub bookmark: Style,
    /// The selection.
    pub selection: Style,
    /// Ranges the user highlighted (the theme's first reader highlight,
    /// yellow in the built-ins).
    pub user_highlight: Style,
    /// Ranges with notes.
    pub note: Style,
    /// List overlay body.
    pub list: Style,
    /// The focused list item.
    pub list_selected: Style,
}

fn color(c: TermColor) -> Color {
    match c {
        TermColor::Rgb(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        TermColor::Indexed(i) => Color::Indexed(i),
    }
}

fn modifiers(a: Attrs) -> Modifier {
    let mut m = Modifier::empty();
    if a.bold {
        m |= Modifier::BOLD;
    }
    if a.italic {
        m |= Modifier::ITALIC;
    }
    if a.underline {
        m |= Modifier::UNDERLINED;
    }
    if a.reverse {
        m |= Modifier::REVERSED;
    }
    m
}

/// A theme style as a ratatui style: colors left unset stay as drawn
/// underneath, attributes add.
pub fn style(t: TermStyle) -> Style {
    let mut s = Style::new().add_modifier(modifiers(t.attrs));
    if let Some(fg) = t.fg {
        s = s.fg(color(fg));
    }
    if let Some(bg) = t.bg {
        s = s.bg(color(bg));
    }
    s
}

impl Theme {
    /// The styles of `theme` at color level `support`.
    pub fn from_theme(theme: &textweaver_theme::Theme, support: ColorSupport) -> Self {
        let t = TerminalTheme::new(theme, support);
        let page = t.page();
        let on_page = |s: TermStyle| style(page.patch(s));
        let status = style(t.style(StyleRole::StatusBar));
        let panel = style(t.color(ColorRole::Surface));
        Theme {
            name: theme.meta.name.clone(),
            display_name: theme.meta.display_name.clone(),
            support,
            text: style(page),
            title: status.add_modifier(Modifier::BOLD),
            status,
            hints: on_page(t.color(ColorRole::DimText)),
            minibuffer: panel,
            gutter: on_page(t.color(ColorRole::DimText)),
            spoken_word: style(t.style(StyleRole::SpokenWord)),
            spoken_sentence: style(t.style(StyleRole::SpokenSentence)),
            find_hit: style(t.style(StyleRole::FindHit)),
            current_hit: style(t.style(StyleRole::CurrentFindHit)),
            bookmark: style(t.style(StyleRole::Bookmark)),
            selection: style(t.style(StyleRole::Selection)),
            user_highlight: t
                .user_highlight(0)
                .map_or_else(|| style(t.style(StyleRole::Selection)), style),
            note: style(t.style(StyleRole::Note)),
            list: panel,
            list_selected: style(t.style(StyleRole::Focus)),
        }
    }

    /// The built-in theme with this name (Star's old names accepted), or
    /// Galaxy, at the detected color level.
    pub fn named(name: &str) -> Self {
        let registry = Registry::builtin();
        Theme::from_theme(registry.resolve(name).0, ColorSupport::detect())
    }

    /// Galaxy, the default theme.
    pub fn galaxy() -> Self {
        Theme::named(textweaver_theme::DEFAULT_THEME)
    }

    /// The style patched onto text for a highlight.
    pub fn highlight(&self, kind: HighlightKind) -> Style {
        match kind {
            HighlightKind::UserHighlight => self.user_highlight,
            HighlightKind::Note => self.note,
            HighlightKind::Bookmark => self.bookmark,
            HighlightKind::FindHit => self.find_hit,
            HighlightKind::Selection => self.selection,
            HighlightKind::SpokenSentence => self.spoken_sentence,
            HighlightKind::CurrentFindHit => self.current_hit,
            HighlightKind::SpokenWord => self.spoken_word,
        }
    }
}

/// The `--theme` help text: every built-in theme, Galaxy first.
pub fn theme_help() -> String {
    format!(
        "Color theme for this run (not saved): {}; or the name of a theme in your themes folder",
        Registry::builtin().names().join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_themes_and_fallback() {
        assert_eq!(Theme::named("galaxy-light").name, "galaxy-light");
        assert_eq!(Theme::named("High_Contrast").name, "high-contrast");
        assert_eq!(Theme::named("nonsense").name, "galaxy");
        assert_eq!(Theme::galaxy().display_name, "Galaxy");
    }

    #[test]
    fn highlights_never_rely_on_color_alone_at_any_level() {
        let registry = Registry::builtin();
        for support in [
            ColorSupport::TrueColor,
            ColorSupport::Ansi256,
            ColorSupport::Ansi16,
            ColorSupport::NoColor,
        ] {
            for theme in registry.themes() {
                let t = Theme::from_theme(theme, support);
                for k in [
                    HighlightKind::SpokenWord,
                    HighlightKind::SpokenSentence,
                    HighlightKind::FindHit,
                    HighlightKind::CurrentFindHit,
                    HighlightKind::Bookmark,
                    HighlightKind::Note,
                ] {
                    assert!(
                        !t.highlight(k).add_modifier.is_empty(),
                        "{} {k:?} {support:?}",
                        t.name
                    );
                }
                // The spoken word and its sentence differ by attribute.
                assert_ne!(
                    t.spoken_word.add_modifier, t.spoken_sentence.add_modifier,
                    "{}",
                    t.name
                );
            }
        }
    }

    #[test]
    fn truecolor_galaxy_is_stars_palette() {
        let t = Theme::from_theme(
            Registry::builtin().resolve("galaxy").0,
            ColorSupport::TrueColor,
        );
        assert_eq!(t.text.bg, Some(Color::Rgb(0x1e, 0x1e, 0x1e)));
        assert_eq!(t.text.fg, Some(Color::Rgb(0xda, 0xda, 0xda)));
        let none = Theme::from_theme(
            Registry::builtin().resolve("galaxy").0,
            ColorSupport::NoColor,
        );
        assert_eq!(none.text.bg, None);
    }

    #[test]
    fn help_lists_every_builtin_theme() {
        let help = theme_help();
        for name in Registry::builtin().names() {
            assert!(help.contains(name), "{name}");
        }
        assert!(help.contains("galaxy, galaxy-light"));
    }
}
