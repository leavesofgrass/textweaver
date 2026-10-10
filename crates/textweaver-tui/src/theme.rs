//! Terminal styles from the shared themes (ADR-0020, `textweaver-theme`):
//! each [`TerminalTheme`] role resolved at the terminal's detected color
//! level (truecolor, 256, 16, or none) and turned into ratatui styles.
//! Every highlight carries a text attribute (bold, underline, italic, or
//! reverse), so no state is shown by color alone, even with color off.

use ratatui::style::{Color, Modifier, Style};
use textweaver_app::HighlightKind;
use textweaver_app::palette::PaletteLook;
use textweaver_app::store::HighlightShape;
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
    /// The colors of the reading aids' marks, from the theme's roles.
    pub marks: MarkStyles,
    /// The foreground of rows the reading ruler masks: the text color
    /// blended [`MASK_TOWARD_PAGE`] of the way toward the page, as the
    /// window's overlay draws it. Unset at 16 colors and with no color,
    /// where the mask stays the dim attribute.
    pub masked: Option<Color>,
}

/// How far the ruler's mask moves masked text toward the page color: the
/// window's overlay strength, so both frontends mask alike.
pub const MASK_TOWARD_PAGE: f64 = 0.72;

/// The colors of the reading aids' marks, at the terminal's color level:
/// set only where the theme file or the reader's `[colors]` gives the
/// role, so a role worked out at load leaves the mark to its attributes
/// alone (an underline, the gutter mark), as with no color.
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

/// A highlight shape as terminal attributes: a different set for each.
fn shape_modifiers(shape: HighlightShape) -> Modifier {
    match shape {
        HighlightShape::Underline => Modifier::UNDERLINED,
        HighlightShape::DoubleUnderline => Modifier::UNDERLINED | Modifier::BOLD,
        HighlightShape::Bold => Modifier::BOLD,
        HighlightShape::Dotted => Modifier::UNDERLINED | Modifier::ITALIC,
        HighlightShape::Brackets => Modifier::ITALIC,
        HighlightShape::Symbol => Modifier::ITALIC | Modifier::BOLD,
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
        let panel = style(t.color(ColorRole::Surface));
        // Quiet chrome: with colors to spare, the title and status lines
        // sit on the panel color, not in the spoken word's band, so the
        // reading highlight is the one strong color on screen. At 16
        // colors and with none they keep the status bar's reverse video.
        let rich = matches!(support, ColorSupport::TrueColor | ColorSupport::Ansi256);
        let status = if rich {
            panel
        } else {
            style(t.style(StyleRole::StatusBar))
        };
        let masked = if rich {
            let ink = theme.color(ColorRole::Text);
            let paper = theme.color(ColorRole::Background);
            textweaver_theme::term_color(ink.mix(paper, MASK_TOWARD_PAGE), support).map(color)
        } else {
            None
        };
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
            marks: MarkStyles::from_theme(theme, support),
            masked,
        }
    }

    /// The style a span on a masked ruler row gets, patched over `s`: the
    /// blended foreground where this theme has one (spans on a colored
    /// band keep theirs, so the band stays readable), else the dim
    /// attribute.
    pub fn mask(&self, s: Style) -> Style {
        match self.masked {
            Some(fg) if s.bg.is_none() || s.bg == self.text.bg => s.fg(fg),
            Some(_) => s,
            None => s.add_modifier(Modifier::DIM),
        }
    }

    /// The built-in theme with this name (star's old names accepted), or
    /// Galaxy, at the detected color level.
    pub fn named(name: &str) -> Self {
        let registry = Registry::builtin();
        Theme::from_theme(registry.resolve(name).0, ColorSupport::detect())
    }

    /// Galaxy, the default theme.
    pub fn galaxy() -> Self {
        Theme::named(textweaver_theme::DEFAULT_THEME)
    }

    /// The style patched onto text for highlight `h`: a user highlight's
    /// palette color and shape ([`textweaver_app::palette::resolve`]), the
    /// theme's style for the rest.
    pub fn mark(&self, h: &textweaver_app::Highlight) -> Style {
        match (h.kind, h.look) {
            (HighlightKind::UserHighlight, Some(look)) => self.palette_mark(look),
            _ => self.highlight(h.kind),
        }
    }

    /// A palette entry's style: its color as the band, with black or
    /// white text, whichever reads better on it, where the terminal shows
    /// colors (the theme's highlight band where it does not), and its
    /// shape's own attributes, so no two entries differ by color alone.
    ///
    /// shortcut: a terminal has no double or dotted underline and no room
    /// for brackets or a symbol without moving the text, so each shape is
    /// a distinct set of attributes here; the window draws the shapes
    /// themselves.
    pub fn palette_mark(&self, look: PaletteLook) -> Style {
        let band = look.color.and_then(|rgb| {
            let bg = textweaver_theme::term_color(rgb, self.support)?;
            let on_black = textweaver_theme::contrast_ratio(textweaver_theme::Rgb::BLACK, rgb);
            let on_white = textweaver_theme::contrast_ratio(textweaver_theme::Rgb::WHITE, rgb);
            let ink = if on_black >= on_white {
                textweaver_theme::Rgb::BLACK
            } else {
                textweaver_theme::Rgb::WHITE
            };
            let fg = textweaver_theme::term_color(ink, self.support)?;
            Some(Style::new().bg(color(bg)).fg(color(fg)))
        });
        let base = band.unwrap_or(self.user_highlight);
        Style {
            add_modifier: (base.add_modifier & Modifier::REVERSED) | shape_modifiers(look.shape),
            sub_modifier: Modifier::empty(),
            ..base
        }
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

impl MarkStyles {
    /// The mark colors `theme` gives (its file or the reader's
    /// `[colors]`), at color level `support`.
    fn from_theme(theme: &textweaver_theme::Theme, support: ColorSupport) -> Self {
        let given = |role: ColorRole| {
            (!theme.is_derived(role))
                .then(|| textweaver_theme::term_color(theme.color(role), support))
                .flatten()
                .map(color)
        };
        MarkStyles {
            ruler: given(ColorRole::Ruler),
            difficult: given(ColorRole::DifficultWord),
            syllables: given(ColorRole::SyllableMark),
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

    /// Every palette shape is its own set of attributes, at every color
    /// level, so two entries of one color, or any entries with no color,
    /// still read apart; with colors, the band is the entry's color.
    #[test]
    fn palette_entries_differ_by_attributes_not_color_alone() {
        let registry = Registry::builtin();
        let theme = registry.resolve("galaxy").0;
        for support in [ColorSupport::TrueColor, ColorSupport::NoColor] {
            let t = Theme::from_theme(theme, support);
            let mut seen = Vec::new();
            for shape in HighlightShape::ALL {
                let look = PaletteLook {
                    entry: Some(0),
                    color: Some(textweaver_theme::Rgb::from_u32(0xffff00)),
                    shape,
                };
                let m = t.palette_mark(look).add_modifier;
                assert!(!m.is_empty(), "{shape:?}");
                assert!(!seen.contains(&m), "{shape:?} {support:?}");
                seen.push(m);
            }
        }
        let t = Theme::from_theme(theme, ColorSupport::TrueColor);
        let look = PaletteLook {
            entry: Some(0),
            color: Some(textweaver_theme::Rgb::from_u32(0xffff00)),
            shape: HighlightShape::Underline,
        };
        assert_eq!(t.palette_mark(look).bg, Some(Color::Rgb(0xff, 0xff, 0)));
        assert_eq!(t.palette_mark(look).fg, Some(Color::Rgb(0, 0, 0)));
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
    fn the_ruler_mask_is_a_color_with_colors_and_dim_without() {
        let registry = Registry::builtin();
        let galaxy = registry.resolve("galaxy").0;
        let t = Theme::from_theme(galaxy, ColorSupport::TrueColor);
        let masked = t.mask(t.text);
        assert!(!masked.add_modifier.contains(Modifier::DIM));
        let (Some(Color::Rgb(r, ..)), Some(Color::Rgb(ink, ..)), Some(Color::Rgb(paper, ..))) =
            (masked.fg, t.text.fg, t.text.bg)
        else {
            panic!("truecolor styles are RGB");
        };
        // Between the text and the page, nearer the page.
        let (lo, hi) = (ink.min(paper), ink.max(paper));
        assert!(lo < r && r < hi, "{r} between {lo} and {hi}");
        assert!(r.abs_diff(paper) < r.abs_diff(ink));
        assert!(
            Theme::from_theme(galaxy, ColorSupport::Ansi256)
                .masked
                .is_some()
        );
        for support in [ColorSupport::Ansi16, ColorSupport::NoColor] {
            let t = Theme::from_theme(galaxy, support);
            assert!(
                t.mask(t.text).add_modifier.contains(Modifier::DIM),
                "{support:?}"
            );
        }
    }

    #[test]
    fn quiet_chrome_off_the_spoken_words_band_reverse_kept_at_16() {
        for theme in Registry::builtin().themes() {
            let t = Theme::from_theme(theme, ColorSupport::TrueColor);
            assert_ne!(t.status.bg, t.spoken_word.bg, "{}", t.name);
            assert!(t.title.add_modifier.contains(Modifier::BOLD));
        }
        let registry = Registry::builtin();
        let galaxy = registry.resolve("galaxy").0;
        for support in [ColorSupport::Ansi16, ColorSupport::NoColor] {
            let t = Theme::from_theme(galaxy, support);
            let bar = style(TerminalTheme::new(galaxy, support).style(StyleRole::StatusBar));
            assert_eq!(t.status, bar, "{support:?}");
        }
    }

    /// The marks take their colors from the theme's roles: a reader's
    /// `[colors]` value shows; a role worked out at load leaves the mark
    /// to its attributes.
    #[test]
    fn marks_come_from_the_theme_roles() {
        let mut config = textweaver_app::AppConfig::for_tests();
        config.settings.colors.ruler = "#336699".into();
        let app = textweaver_app::App::new(config);
        let t = Theme::from_theme(&app.reading_theme(), ColorSupport::TrueColor);
        assert_eq!(t.marks.ruler, Some(Color::Rgb(0x33, 0x66, 0x99)));
        assert_eq!(t.marks.difficult, None);
        assert_eq!(t.marks.syllables, None);
        let none = Theme::from_theme(&app.reading_theme(), ColorSupport::NoColor);
        assert_eq!(none.marks, MarkStyles::default());
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
