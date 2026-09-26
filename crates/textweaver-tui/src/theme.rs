//! Color themes. Every highlight also carries a text modifier (bold,
//! underline, reverse), so no state is shown by color alone.

use ratatui::style::{Color, Modifier, Style};
use textweaver_app::HighlightKind;

/// Styles for every part of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Theme name as stored in settings.
    pub name: &'static str,
    /// Document text.
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
    /// Ranges the user highlighted.
    pub user_highlight: Style,
    /// Ranges with notes.
    pub note: Style,
    /// List overlay body.
    pub list: Style,
    /// The focused list item.
    pub list_selected: Style,
}

const BOLD: Modifier = Modifier::BOLD;
const UNDERLINED: Modifier = Modifier::UNDERLINED;
const REVERSED: Modifier = Modifier::REVERSED;
const ITALIC: Modifier = Modifier::ITALIC;

impl Theme {
    /// The default dark theme.
    pub fn galaxy() -> Self {
        let bg = Color::Rgb(0x14, 0x12, 0x2e);
        let fg = Color::Rgb(0xe4, 0xe2, 0xf4);
        Theme {
            name: "galaxy",
            text: Style::new().fg(fg).bg(bg),
            title: Style::new()
                .fg(Color::White)
                .bg(Color::Rgb(0x3b, 0x2a, 0x6b))
                .add_modifier(BOLD),
            status: Style::new()
                .fg(Color::Rgb(0xf6, 0xe7, 0x8c))
                .bg(Color::Rgb(0x1e, 0x1b, 0x40)),
            hints: Style::new().fg(Color::Rgb(0xa6, 0xa4, 0xd0)).bg(bg),
            minibuffer: Style::new()
                .fg(Color::White)
                .bg(Color::Rgb(0x2a, 0x26, 0x55)),
            gutter: Style::new().fg(Color::Rgb(0x6e, 0x6a, 0x9e)).bg(bg),
            spoken_word: Style::new()
                .fg(Color::Black)
                .bg(Color::Rgb(0x4d, 0xd0, 0xe1))
                .add_modifier(BOLD),
            spoken_sentence: Style::new()
                .bg(Color::Rgb(0x2e, 0x2a, 0x5e))
                .add_modifier(UNDERLINED),
            find_hit: Style::new()
                .fg(Color::Black)
                .bg(Color::Rgb(0xb3, 0x8a, 0x2e))
                .add_modifier(UNDERLINED),
            current_hit: Style::new()
                .fg(Color::Black)
                .bg(Color::Rgb(0xff, 0xc1, 0x07))
                .add_modifier(BOLD | UNDERLINED),
            bookmark: Style::new()
                .fg(Color::Rgb(0xff, 0x8a, 0xc8))
                .add_modifier(UNDERLINED),
            selection: Style::new()
                .bg(Color::Rgb(0x44, 0x55, 0x99))
                .add_modifier(REVERSED),
            user_highlight: Style::new()
                .fg(Color::Black)
                .bg(Color::Rgb(0xe8, 0xd8, 0x5a))
                .add_modifier(ITALIC),
            note: Style::new()
                .fg(Color::Rgb(0x8c, 0xf0, 0xa8))
                .add_modifier(ITALIC | UNDERLINED),
            list: Style::new().fg(fg).bg(Color::Rgb(0x22, 0x1f, 0x4a)),
            list_selected: Style::new()
                .fg(Color::Black)
                .bg(Color::Rgb(0x4d, 0xd0, 0xe1))
                .add_modifier(BOLD),
        }
    }

    /// A light theme with the terminal's named colors.
    pub fn light() -> Self {
        Theme {
            name: "light",
            text: Style::new().fg(Color::Black).bg(Color::White),
            title: Style::new()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(BOLD),
            status: Style::new().fg(Color::Black).bg(Color::Gray),
            hints: Style::new().fg(Color::DarkGray).bg(Color::White),
            minibuffer: Style::new().fg(Color::Black).bg(Color::LightCyan),
            gutter: Style::new().fg(Color::DarkGray).bg(Color::White),
            spoken_word: Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(BOLD),
            spoken_sentence: Style::new().bg(Color::LightYellow).add_modifier(UNDERLINED),
            find_hit: Style::new()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(UNDERLINED),
            current_hit: Style::new()
                .fg(Color::White)
                .bg(Color::Green)
                .add_modifier(BOLD | UNDERLINED),
            bookmark: Style::new().fg(Color::Magenta).add_modifier(UNDERLINED),
            selection: Style::new().add_modifier(REVERSED),
            user_highlight: Style::new()
                .fg(Color::Black)
                .bg(Color::LightYellow)
                .add_modifier(ITALIC),
            note: Style::new()
                .fg(Color::Green)
                .add_modifier(ITALIC | UNDERLINED),
            list: Style::new().fg(Color::Black).bg(Color::Gray),
            list_selected: Style::new()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(BOLD),
        }
    }

    /// Black and white with yellow accents; highlights use reverse video,
    /// bold, and underline so they survive any palette.
    pub fn high_contrast() -> Self {
        Theme {
            name: "high-contrast",
            text: Style::new().fg(Color::White).bg(Color::Black),
            title: Style::new()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(BOLD),
            status: Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(BOLD),
            hints: Style::new().fg(Color::White).bg(Color::Black),
            minibuffer: Style::new()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(BOLD),
            gutter: Style::new().fg(Color::Yellow).bg(Color::Black),
            spoken_word: Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(BOLD | UNDERLINED),
            spoken_sentence: Style::new()
                .fg(Color::White)
                .add_modifier(BOLD | UNDERLINED),
            find_hit: Style::new().add_modifier(REVERSED),
            current_hit: Style::new()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(BOLD | UNDERLINED),
            bookmark: Style::new()
                .fg(Color::Yellow)
                .add_modifier(BOLD | UNDERLINED),
            selection: Style::new().add_modifier(REVERSED),
            user_highlight: Style::new()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(ITALIC),
            note: Style::new()
                .fg(Color::Cyan)
                .add_modifier(ITALIC | UNDERLINED),
            list: Style::new().fg(Color::White).bg(Color::Black),
            list_selected: Style::new()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(BOLD),
        }
    }

    /// The theme with this name, or `galaxy`.
    pub fn named(name: &str) -> Self {
        match name {
            "light" => Theme::light(),
            "high-contrast" | "high_contrast" | "contrast" => Theme::high_contrast(),
            _ => Theme::galaxy(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_themes_and_fallback() {
        assert_eq!(Theme::named("light").name, "light");
        assert_eq!(Theme::named("high-contrast").name, "high-contrast");
        assert_eq!(Theme::named("nonsense").name, "galaxy");
        for t in [Theme::galaxy(), Theme::light(), Theme::high_contrast()] {
            // Highlights never rely on color alone.
            for k in [
                HighlightKind::SpokenWord,
                HighlightKind::CurrentFindHit,
                HighlightKind::Bookmark,
                HighlightKind::UserHighlight,
                HighlightKind::Note,
            ] {
                assert!(!t.highlight(k).add_modifier.is_empty(), "{} {k:?}", t.name);
            }
        }
    }
}
