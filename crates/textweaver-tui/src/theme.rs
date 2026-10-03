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
    /// The status line while it shows an error: the theme's error color,
    /// bold, on the page. The message itself begins with "Error:", so the
    /// color is never the only mark.
    pub status_error: Style,
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
    /// Code blocks and their tokens.
    pub code: CodeStyles,
    /// The reader's colors for marks no theme role holds (`[colors]`):
    /// set with [`with_marks`](Self::with_marks).
    pub marks: MarkStyles,
}

/// The reader's own colors for the reading aids' marks (`[colors]`), at
/// the terminal's color level. With 16 colors or none they are unset, and
/// the marks keep their attributes alone (an underline, the gutter mark).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkStyles {
    /// The band of the reading ruler and the current line.
    pub ruler: Option<Color>,
    /// The underline of difficult words.
    pub difficult: Option<Color>,
    /// The middle dots between syllables.
    pub syllables: Option<Color>,
}

/// Styles for code blocks (Agent W4g): the theme's code colors, and for
/// each token kind a color from another role plus, for most, an attribute,
/// so no kind is told apart by color alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeStyles {
    /// Code on the code background (`code`, `code_background`).
    pub plain: Style,
    /// Comments: dim text, italic.
    pub comment: Style,
    /// Keywords: heading 2 color, bold.
    pub keyword: Style,
    /// Strings: the quote color (strings keep their quote marks).
    pub string: Style,
    /// Numbers and constants: heading 4 color.
    pub number: Style,
    /// Function names: heading 1 color.
    pub function: Style,
    /// Type names: heading 3 color.
    pub kind: Style,
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
            status_error: on_page(t.color(ColorRole::Error)).add_modifier(Modifier::BOLD),
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
            code: CodeStyles::from_terminal(&t),
            marks: MarkStyles::default(),
        }
    }

    /// These styles with the reader's mark colors (`[colors]`), at this
    /// theme's color level.
    pub fn with_marks(mut self, marks: &textweaver_app::MarkColors) -> Self {
        let to = |c: Option<textweaver_theme::Rgb>| {
            c.and_then(|rgb| textweaver_theme::term_color(rgb, self.support))
                .map(color)
        };
        self.marks = MarkStyles {
            ruler: to(marks.ruler),
            difficult: to(marks.difficult_words),
            syllables: to(marks.syllables),
        };
        self
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

impl CodeStyles {
    /// The code styles of a resolved theme.
    fn from_terminal(t: &TerminalTheme) -> Self {
        let plain = style(t.color(ColorRole::CodeBackground));
        // A role's foreground over the code background, with an attribute.
        let fg = |role: ColorRole, m: Modifier| {
            let mut s = plain.add_modifier(m);
            if let Some(c) = t.color(role).fg {
                s = s.fg(color(c));
            }
            s
        };
        CodeStyles {
            plain,
            comment: fg(ColorRole::DimText, Modifier::ITALIC),
            keyword: fg(ColorRole::Heading2, Modifier::BOLD),
            string: fg(ColorRole::Quote, Modifier::empty()).remove_modifier(Modifier::ITALIC),
            number: fg(ColorRole::Heading4, Modifier::empty()),
            function: fg(ColorRole::Heading1, Modifier::empty()),
            kind: fg(ColorRole::Heading3, Modifier::empty()),
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
