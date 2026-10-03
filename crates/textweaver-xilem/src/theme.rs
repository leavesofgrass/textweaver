//! textweaver's themes on Masonry's widgets.
//!
//! The layout, spacing, and corner radii follow Masonry's default look (the
//! one Xilem's `to_do_mvc` example shows, which the owner liked); the colors come
//! from a textweaver theme ([`Theme::rgb_table`]'s roles), Galaxy by
//! default. Every derived color is checked: text 4.5 to 1 on its surface
//! (7 to 1 in high-contrast themes) and the focus ring 3 to 1 against both
//! the page and the panels.

use masonry::core::{DefaultProperties, PropertyStack, Selector};
use masonry::layout::{AsUnit, Length};
use masonry::peniko::Color;
use masonry::properties::{
    Background, BorderColor, BorderWidth, BoxShadow, CaretColor, CheckmarkColor,
    CheckmarkStrokeWidth, ContentColor, CornerRadius, Gap, Padding, PlaceholderColor,
    SelectionColor,
};
use masonry::widgets::{Button, Checkbox, Divider, Flex, Label, TextArea, TextInput};
use textweaver_theme::color::{adjust_away, contrast_ratio};
use textweaver_theme::{Attrs, ColorRole, Rgb, StyleRole, Theme, ThemeKind};

/// Space between related controls.
pub const GAP: f64 = 8.0;
/// Space around panels.
pub const PAD: f64 = 16.0;
/// Corner radius of buttons and fields.
pub const RADIUS: f64 = 6.0;
/// Corner radius of panels and dialogs.
pub const PANEL_RADIUS: f64 = 10.0;
/// Width of the focus ring.
pub const FOCUS_WIDTH: f64 = 2.0;
/// Interface text size, in logical pixels.
pub const UI_TEXT: f32 = 15.0;
/// The document's default text size (before the reader's font setting).
pub const DOC_TEXT: f32 = 20.0;

/// The colors the GUI draws with, derived from one theme.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// The theme's name.
    pub name: String,
    /// Dark, light, or high contrast.
    pub kind: ThemeKind,
    /// The page (window background and the document).
    pub background: Rgb,
    /// Panels: the header, the status bar, dialogs.
    pub surface: Rgb,
    /// Buttons and fields, a step up from the panel.
    pub raised: Rgb,
    /// Hairline borders.
    pub border: Rgb,
    /// Borders under the pointer.
    pub border_hover: Rgb,
    /// Body text.
    pub text: Rgb,
    /// Secondary text.
    pub dim_text: Rgb,
    /// The text of a disabled control (everything behind an open dialog):
    /// at least 3 to 1 on buttons and panels.
    pub disabled_text: Rgb,
    /// Heading colors, levels 1 to 6.
    pub headings: [Rgb; 6],
    /// Links.
    pub link: Rgb,
    /// Code.
    pub code: Rgb,
    /// Behind code.
    pub code_background: Rgb,
    /// Quotes.
    pub quote: Rgb,
    /// Errors.
    pub error: Rgb,
    /// The focus ring.
    pub focus: Rgb,
    /// The accent: the primary button's fill.
    pub accent: Rgb,
    /// Text on the accent.
    pub on_accent: Rgb,
    /// The spoken word: text and band.
    pub spoken_word: (Rgb, Rgb),
    /// The spoken word's attributes besides color: how it is marked
    /// without color (bold in every bundled theme). The window draws bold
    /// without reflowing the line, and italic and underline as well.
    pub spoken_word_attrs: Attrs,
    /// The band behind the spoken sentence.
    pub spoken_sentence: Rgb,
    /// The spoken sentence's attributes besides color (underline in every
    /// bundled theme). The window underlines the sentence in every palette
    /// whatever these say, because its band is too faint to be the cue.
    pub spoken_sentence_attrs: Attrs,
    /// The color of the line under the spoken sentence: the sentence's own
    /// text color, at least 3 to 1 against its band.
    pub sentence_line: Rgb,
    /// The selection: text and band.
    pub selection: (Rgb, Rgb),
    /// The selection band in a text field, under the field's own text
    /// color (a field keeps its text color on the band, so the reversed
    /// band the document draws would leave the text unreadable).
    pub field_selection: Rgb,
    /// A search match: text and band.
    pub find_hit: (Rgb, Rgb),
    /// The search match at the caret: its band.
    pub current_find_hit: Rgb,
    /// Text with a note: its band.
    pub note: Rgb,
    /// A bookmarked word: its band.
    pub bookmark: Rgb,
    /// A highlight the reader made: its band (the theme's first highlight
    /// color, or the selection's).
    pub user_highlight: Rgb,
    /// The caret.
    pub caret: Rgb,
    /// The reading ruler's band on the reading line (a tint of the focus
    /// color the text stays readable on; a bar at the line's start marks
    /// it too, so color is not the only cue).
    pub ruler_focus: Rgb,
    /// The ruler's band on the lines around the reading line.
    pub ruler_band: Rgb,
}

/// `c` as a Masonry color.
pub fn color(c: Rgb) -> Color {
    Color::from_rgb8(c.r, c.g, c.b)
}

/// `c` with `alpha` (0 to 1).
pub fn with_alpha(c: Rgb, alpha: f32) -> Color {
    color(c).with_alpha(alpha)
}

/// The smallest change to `c` that reaches `min` contrast against every
/// color in `against`, or `c` if it already does (or nothing can).
fn ensure(c: Rgb, against: &[Rgb], min: f64) -> Rgb {
    let mut c = c;
    for &bg in against {
        if contrast_ratio(c, bg) < min
            && let Some(better) = adjust_away(c, bg, min)
        {
            c = better;
        }
    }
    c
}

impl Palette {
    /// The palette for `theme`.
    pub fn from_theme(theme: &Theme) -> Palette {
        let background = theme.color(ColorRole::Background);
        let surface = theme.color(ColorRole::Surface);
        let text = theme.color(ColorRole::Text);
        let dark = background.is_dark();
        let hc = theme.kind() == ThemeKind::HighContrast;
        let text_min = if hc { 7.0 } else { 4.5 };
        // A step from the panel towards the text: subtle elevation.
        let raised = if hc { surface } else { surface.mix(text, 0.07) };
        let border = if hc {
            text
        } else {
            background.mix(text, if dark { 0.20 } else { 0.28 })
        };
        let border_hover = if hc { text } else { background.mix(text, 0.40) };
        let focus_style = theme.resolve_style(StyleRole::Focus);
        // The focus ring: the focus style's band, at least 3 to 1 against
        // the page, the panels, and the buttons.
        let focus = ensure(focus_style.background, &[background, surface, raised], 3.0);
        let status = theme.resolve_style(StyleRole::StatusBar);
        let accent = status.background;
        let on_accent = ensure(status.foreground, &[accent], 4.5);
        let spoken = theme.resolve_style(StyleRole::SpokenWord);
        let sentence = theme.resolve_style(StyleRole::SpokenSentence);
        let selection = theme.resolve_style(StyleRole::Selection);
        // A text field keeps its text on the selection band: the style's
        // own band (unreversed), at the text floor against the text.
        let field_selection = {
            let style = theme.style(StyleRole::Selection);
            let band = if style.attributes.reverse {
                style.background.unwrap_or(background)
            } else {
                selection.background
            };
            ensure(
                band,
                &[ensure(text, &[background, surface, raised], text_min)],
                text_min,
            )
        };
        let find = theme.resolve_style(StyleRole::FindHit);
        let current_find = theme.resolve_style(StyleRole::CurrentFindHit);
        let note = theme.resolve_style(StyleRole::Note);
        let bookmark = theme.resolve_style(StyleRole::Bookmark);
        let user_highlight = theme
            .user_highlights
            .first()
            .map_or(selection.background, |h| theme.resolve(&h.style).background);
        let headings = [1u8, 2, 3, 4, 5, 6].map(|l| {
            ColorRole::heading(l).map_or(text, |r| ensure(theme.color(r), &[background], text_min))
        });
        Palette {
            name: theme.name().to_owned(),
            kind: theme.kind(),
            background,
            surface,
            raised,
            border,
            border_hover,
            text: ensure(text, &[background, surface, raised], text_min),
            dim_text: ensure(theme.color(ColorRole::DimText), &[background, surface], 4.5),
            // The secondary text, a step toward the button fill, but never
            // under 3 to 1 on a button or a panel.
            disabled_text: ensure(
                ensure(
                    theme.color(ColorRole::DimText).mix(raised, 0.2),
                    &[raised],
                    3.0,
                ),
                &[surface],
                3.0,
            ),
            headings,
            link: ensure(theme.color(ColorRole::Link), &[background], text_min),
            code: theme.color(ColorRole::Code),
            code_background: theme.color(ColorRole::CodeBackground),
            quote: theme.color(ColorRole::Quote),
            error: ensure(theme.color(ColorRole::Error), &[background, surface], 4.5),
            focus,
            accent,
            on_accent,
            spoken_word: (spoken.foreground, spoken.background),
            spoken_word_attrs: spoken.attributes,
            spoken_sentence: sentence.background,
            spoken_sentence_attrs: sentence.attributes,
            sentence_line: ensure(sentence.foreground, &[sentence.background], 3.0),
            selection: (selection.foreground, selection.background),
            field_selection,
            find_hit: (find.foreground, find.background),
            current_find_hit: current_find.background,
            note: note.background,
            bookmark: bookmark.background,
            user_highlight,
            caret: text,
            ruler_focus: ensure(background.mix(focus, 0.22), &[text], text_min),
            ruler_band: ensure(background.mix(focus, 0.10), &[text], text_min),
        }
    }

    /// The palette of the built-in theme `name`, or Galaxy.
    pub fn named(name: &str) -> Palette {
        let theme = textweaver_theme::builtin::get(name)
            .unwrap_or_else(textweaver_theme::builtin::default_theme);
        Palette::from_theme(theme)
    }

    /// Galaxy, the default.
    pub fn galaxy() -> Palette {
        Palette::from_theme(textweaver_theme::builtin::default_theme())
    }

    /// The color of heading level `level` (1 to 6).
    pub fn heading(&self, level: u8) -> Rgb {
        self.headings[usize::from(level.clamp(1, 6) - 1)]
    }
}

/// Masonry's default properties, recolored with `p`.
pub fn default_properties(p: &Palette) -> DefaultProperties {
    let mut props = masonry::theme::default_property_set();
    let hc = p.kind == ThemeKind::HighContrast;
    let border_w = if hc { 2.px() } else { 1.px() };
    let text = color(p.text);
    let focus = color(p.focus);

    // Buttons: the to_do_mvc shape, recolored, with a 2 px focus ring.
    button_props::<Button>(&mut props, p);
    button_props::<crate::widgets::ActionButton>(&mut props, p);

    // Checkboxes.
    props.insert::<Checkbox, _>(Background::Color(color(p.raised)));
    props.insert::<Checkbox, _>(BorderColor {
        color: color(p.border_hover),
    });
    props.insert::<Checkbox, _>(CheckmarkColor { color: text });
    props.insert::<Checkbox, _>(CheckmarkStrokeWidth { width: 2.0 });
    {
        let mut stack = PropertyStack::new();
        stack.push_layer(
            Selector::new().with_focused(true),
            (
                BorderColor { color: focus },
                BorderWidth {
                    width: Length::px(FOCUS_WIDTH),
                },
            ),
        );
        props.insert_stack::<Checkbox>(stack);
    }

    // Text fields.
    props.insert::<TextInput, _>(Padding::from_vh(8.px(), 12.px()));
    props.insert::<TextInput, _>(CornerRadius {
        radius: Length::px(RADIUS),
    });
    props.insert::<TextInput, _>(BorderWidth { width: border_w });
    props.insert::<TextInput, _>(BorderColor {
        color: color(p.border_hover),
    });
    props.insert::<TextInput, _>(Background::Color(color(p.background)));
    props.insert::<TextInput, _>(PlaceholderColor::new(color(p.dim_text)));
    props.insert::<TextInput, _>(CaretColor { color: text });
    props.insert::<TextInput, _>(SelectionColor {
        color: color(p.field_selection),
    });
    {
        let mut stack = PropertyStack::new();
        stack.push_layer(
            Selector::new().with_focused(true),
            (
                BorderColor { color: focus },
                BorderWidth {
                    width: Length::px(FOCUS_WIDTH),
                },
            ),
        );
        props.insert_stack::<TextInput>(stack);
    }

    // The field inside a text input (the prompt), which Masonry would draw
    // in its own near-white whatever the theme.
    props.insert::<TextArea<true>, _>(ContentColor::new(text));
    props.insert::<TextArea<true>, _>(CaretColor { color: text });
    props.insert::<TextArea<true>, _>(SelectionColor {
        color: color(p.field_selection),
    });
    props.insert::<TextArea<false>, _>(ContentColor::new(text));
    props.insert::<TextArea<false>, _>(SelectionColor {
        color: color(p.field_selection),
    });

    props.insert::<Label, _>(ContentColor::new(text));
    {
        // Masonry's own disabled label color (a fixed light gray, 2.06 to 1
        // on Galaxy Light's buttons) would win over the base color; the
        // palette's disabled text keeps 3 to 1 on buttons and panels
        // (design system QW3).
        let mut stack = PropertyStack::new();
        stack.push_layer(
            Selector::classes(&[crate::widgets::ACCENT_TEXT_CLASS]),
            ContentColor::new(color(p.on_accent)),
        );
        // Last, so it wins: a disabled Play's text is not left in the
        // text-on-accent color on a plain button.
        stack.push_layer(
            Selector::new().with_disabled(true),
            ContentColor::new(color(p.disabled_text)),
        );
        props.insert_stack::<Label>(stack);
    }
    props.insert::<Divider, _>(ContentColor::new(color(p.border)));
    props.insert::<Flex, _>(Gap::new(Length::px(GAP)));
    props
}

/// The button look for widget type `W`: Masonry's `Button` shape with the
/// palette's colors; the `primary` class fills with the accent.
fn button_props<W: masonry::core::Widget>(props: &mut DefaultProperties, p: &Palette) {
    let hc = p.kind == ThemeKind::HighContrast;
    let border_w = if hc { 2.px() } else { 1.px() };
    let focus = color(p.focus);
    props.insert::<W, _>(Padding::from_vh(8.px(), 18.px()));
    props.insert::<W, _>(CornerRadius {
        radius: Length::px(RADIUS),
    });
    props.insert::<W, _>(BorderWidth { width: border_w });
    props.insert::<W, _>(Background::Color(color(p.raised)));
    props.insert::<W, _>(BorderColor {
        color: color(p.border),
    });
    let ring = (
        BorderColor { color: focus },
        BorderWidth {
            width: Length::px(FOCUS_WIDTH),
        },
    );
    let mut stack = PropertyStack::new();
    stack.push_layer(
        Selector::new().with_hovered(true),
        BorderColor {
            color: color(p.border_hover),
        },
    );
    stack.push_layer(Selector::new().with_focused(true), ring);
    stack.push_layer(
        Selector::new().with_active(true),
        Background::Color(color(p.raised.mix(p.text, 0.10))),
    );
    stack.push_layer(
        Selector::classes(&["primary"]),
        (
            Background::Color(color(p.accent)),
            BorderColor {
                color: color(p.accent),
            },
        ),
    );
    stack.push_layer(
        Selector::classes(&["primary"]).with_hovered(true),
        Background::Color(color(p.accent.mix(p.text, 0.15))),
    );
    // On the accent fill, the ring is drawn in the page color, which
    // stands out from the fill (at least 3 to 1, tested) where the text
    // color can be close to it.
    stack.push_layer(
        Selector::classes(&["primary"]).with_focused(true),
        (
            BorderColor {
                color: color(p.background),
            },
            BorderWidth {
                width: Length::px(FOCUS_WIDTH),
            },
        ),
    );
    // Disabled last, since later layers win: a disabled Play loses its
    // accent fill and border like any other button (design system QW3).
    stack.push_layer(
        Selector::new().with_disabled(true),
        (
            Background::Color(color(p.surface)),
            BorderColor {
                color: color(p.border),
            },
        ),
    );
    props.insert_stack::<W>(stack);
}

/// A panel's look: the surface, a hairline border, rounded corners, and a
/// soft shadow for elevation (none in high contrast, where a solid border
/// shows the edge instead).
pub fn panel_props(
    p: &Palette,
) -> (
    Background,
    BorderColor,
    BorderWidth,
    CornerRadius,
    BoxShadow,
) {
    let hc = p.kind == ThemeKind::HighContrast;
    let shadow_alpha = if hc || !p.background.is_dark() {
        0.0
    } else {
        0.35
    };
    (
        Background::Color(color(p.surface)),
        BorderColor {
            color: color(p.border),
        },
        BorderWidth {
            width: Length::px(if hc { 2.0 } else { 1.0 }),
        },
        CornerRadius {
            radius: Length::px(PANEL_RADIUS),
        },
        BoxShadow::new(Color::BLACK.with_alpha(shadow_alpha), (0.0, 2.0)).blur(Length::px(8.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIRED: [&str; 4] = ["galaxy", "galaxy-light", "contrast", "high-contrast"];

    #[test]
    fn required_themes_meet_the_contrast_floor() {
        for name in REQUIRED {
            let p = Palette::named(name);
            assert_eq!(p.name, name);
            let min = if p.kind == ThemeKind::HighContrast {
                7.0
            } else {
                4.5
            };
            for (what, fg, bg) in [
                ("text on page", p.text, p.background),
                ("text on panel", p.text, p.surface),
                ("text on button", p.text, p.raised),
                ("link", p.link, p.background),
            ] {
                let r = contrast_ratio(fg, bg);
                assert!(r >= min - 0.01, "{name}: {what} is {r:.2} to 1");
            }
            for (what, bg) in [
                ("page", p.background),
                ("panel", p.surface),
                ("button", p.raised),
            ] {
                let r = contrast_ratio(p.focus, bg);
                assert!(
                    r >= 3.0 - 0.01,
                    "{name}: focus ring on {what} is {r:.2} to 1"
                );
            }
            assert!(
                contrast_ratio(p.on_accent, p.accent) >= 4.5 - 0.01,
                "{name}: primary button"
            );
            // Play's focus ring, in the page color on the accent fill.
            let r = contrast_ratio(p.background, p.accent);
            assert!(r >= 3.0 - 0.01, "{name}: Play's focus ring is {r:.2} to 1");
            // Typed text on a text field's selection band.
            let r = contrast_ratio(p.text, p.field_selection);
            assert!(
                r >= min - 0.01,
                "{name}: selected field text is {r:.2} to 1"
            );
            for l in 1..=6 {
                let r = contrast_ratio(p.heading(l), p.background);
                assert!(r >= min - 0.01, "{name}: heading {l} is {r:.2} to 1");
            }
        }
    }

    /// Disabled text (the window behind an open dialog) stays readable:
    /// at least 3 to 1 on a button and on a panel, in the four required
    /// themes and every built-in one (design system QW3, W8c-w).
    #[test]
    fn disabled_text_reaches_three_to_one() {
        for t in textweaver_theme::builtin::all() {
            let p = Palette::from_theme(t);
            for (what, bg) in [("button", p.raised), ("panel", p.surface)] {
                let r = contrast_ratio(p.disabled_text, bg);
                assert!(
                    r >= 3.0 - 0.01,
                    "{}: disabled text on {what} is {r:.2} to 1",
                    p.name
                );
            }
        }
        for name in REQUIRED {
            let p = Palette::named(name);
            assert!(contrast_ratio(p.disabled_text, p.raised) >= 3.0 - 0.01);
        }
    }

    #[test]
    fn galaxy_is_the_default_and_dark() {
        let p = Palette::named("no such theme");
        assert_eq!(p.name, "galaxy");
        assert!(p.background.is_dark());
        assert_eq!(p.background, Rgb::parse("#1e1e1e").unwrap());
    }

    #[test]
    fn every_builtin_theme_builds_properties() {
        for t in textweaver_theme::builtin::all() {
            let p = Palette::from_theme(t);
            let _ = default_properties(&p);
        }
    }
}
