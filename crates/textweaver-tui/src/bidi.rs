//! Right-to-left text on screen (Wave 4, W4d; ADR-0030).
//!
//! The document, speech, and the screen reader always have text in logical
//! order, the order it is read in. Many terminals draw each cell left to
//! right as they get it, so an Arabic or Hebrew line shows backwards; the
//! reader can reorder such lines for display with the Unicode
//! Bidirectional Algorithm (`unicode-bidi`), `[interface] rtl`:
//!
//! - `auto` (the default) reorders only where it helps: not in terminals
//!   that reorder on their own (VTE ones such as GNOME Terminal, Konsole,
//!   mlterm, macOS Terminal), not on Windows (Windows Terminal and the
//!   console have no right-to-left support, so textweaver leaves the text
//!   as it is there), and not with a screen reader (hybrid or screen
//!   reader mode): a screen reader reads the terminal's cells, and
//!   reordered cells would reach it backwards;
//! - `on` always reorders, `off` never.
//!
//! Lines with no right-to-left letters are drawn as they are. Direction
//! marks and isolates, which the catalog puts around values, are used by
//! the algorithm and then left out of what is drawn, since terminals
//! show them as boxes or gaps. The typing line of a prompt is not
//! reordered: its caret follows the logical text.

use std::borrow::Cow;

use ratatui::style::Style;
use ratatui::text::Span;
use textweaver_app::store::RtlDisplay;
use unicode_bidi::{BidiClass, BidiInfo, bidi_class};

/// True when right-to-left text should be reordered for display, for the
/// setting and the terminal this runs in.
pub fn reorders(setting: RtlDisplay, screen_reader: bool) -> bool {
    reorders_in(setting, screen_reader, terminal_reorders())
}

/// [`reorders`] with the terminal's own reordering already probed
/// ([`crate::terminal_info::TerminalInfo`]), so drawing reads no
/// environment variables.
pub fn reorders_in(setting: RtlDisplay, screen_reader: bool, terminal_reorders: bool) -> bool {
    match setting {
        RtlDisplay::On => true,
        RtlDisplay::Off => false,
        RtlDisplay::Auto => !screen_reader && !cfg!(windows) && !terminal_reorders,
    }
}

/// True in terminals known to reorder right-to-left text themselves.
pub fn terminal_reorders() -> bool {
    let set = |v: &str| std::env::var_os(v).is_some_and(|x| !x.is_empty());
    set("VTE_VERSION")
        || set("KONSOLE_VERSION")
        || set("MLTERM")
        || std::env::var("TERM_PROGRAM").is_ok_and(|t| t == "Apple_Terminal")
        || std::env::var("TERM").is_ok_and(|t| t.starts_with("mlterm"))
}

/// True for a character the algorithm treats as right to left (Hebrew,
/// Arabic, Syriac, Thaana, N'Ko, their numbers) or that opens a right-to-left
/// embedding, override, or isolate.
fn is_rtl(c: char) -> bool {
    matches!(
        bidi_class(c),
        BidiClass::R | BidiClass::AL | BidiClass::RLE | BidiClass::RLO | BidiClass::RLI
    )
}

/// Direction marks, embeddings, overrides, and isolates: used by the
/// algorithm, not drawn.
fn is_control(c: char) -> bool {
    matches!(c, '\u{200e}' | '\u{200f}' | '\u{061c}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// `text` in the order a terminal that draws left to right should show
/// it: unchanged without right-to-left letters, else each paragraph
/// reordered, without its direction controls.
pub fn visual(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_rtl) {
        return Cow::Borrowed(text);
    }
    let info = BidiInfo::new(text, None);
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for para in &info.paragraphs {
        // Separators between paragraphs (new lines) stay where they are.
        out.push_str(&text[last..para.range.start]);
        let line = para.range.clone();
        let body_end = text[line.clone()].trim_end_matches(['\n', '\r']).len() + line.start;
        out.extend(
            info.reorder_line(para, line.start..body_end)
                .chars()
                .filter(|c| !is_control(*c)),
        );
        out.push_str(&text[body_end..line.end]);
        last = line.end;
    }
    out.push_str(&text[last..]);
    Cow::Owned(out)
}

/// One styled row in the order to draw it, and for each character of the
/// row as given (logical order, controls included) where it went: its
/// index among the drawn characters, or `None` for a control that is not
/// drawn. `None` for the whole map when nothing moved (no right-to-left
/// letters in the row).
pub fn visual_spans(spans: Vec<Span<'static>>) -> (Vec<Span<'static>>, Option<Vec<Option<usize>>>) {
    let chars: Vec<(char, Style)> = spans
        .iter()
        .flat_map(|s| s.content.chars().map(move |c| (c, s.style)))
        .collect();
    if !chars.iter().any(|(c, _)| is_rtl(*c)) {
        return (spans, None);
    }
    let text: String = chars.iter().map(|(c, _)| *c).collect();
    // Byte offset of each char, to map the runs back to char indexes.
    let starts: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    let info = BidiInfo::new(&text, None);
    let mut order: Vec<usize> = Vec::with_capacity(chars.len());
    for para in &info.paragraphs {
        let (levels, runs) = info.visual_runs(para, para.range.clone());
        for run in runs {
            let from = starts.partition_point(|&b| b < run.start);
            let to = starts.partition_point(|&b| b < run.end);
            if levels[run.start].is_rtl() {
                order.extend((from..to).rev());
            } else {
                order.extend(from..to);
            }
        }
    }
    let mut map: Vec<Option<usize>> = vec![None; chars.len()];
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut drawn = 0usize;
    for i in order {
        let (c, style) = chars[i];
        if is_control(c) {
            continue;
        }
        map[i] = Some(drawn);
        drawn += 1;
        match out.last_mut() {
            Some(last) if last.style == style => last.content.to_mut().push(c),
            _ => out.push(Span::styled(c.to_string(), style)),
        }
    }
    (out, Some(map))
}

/// The cell column where the character at cell column `col` of a row, as
/// given, is drawn after [`visual_spans`] reordered it; `col` unchanged
/// without a map.
pub fn visual_column(logical: &[Span<'_>], map: Option<&[Option<usize>]>, col: usize) -> usize {
    let Some(map) = map else {
        return col;
    };
    let chars: Vec<char> = logical.iter().flat_map(|s| s.content.chars()).collect();
    let width = |c: char| unicode_width(c);
    // The logical character at `col`.
    let mut at = 0usize;
    let mut index = chars.len();
    for (i, c) in chars.iter().enumerate() {
        if at >= col {
            index = i;
            break;
        }
        at += width(*c);
    }
    let Some(Some(target)) = map.get(index) else {
        return col;
    };
    // The drawn characters before it, in drawn order.
    let mut drawn: Vec<(usize, char)> = map
        .iter()
        .zip(&chars)
        .filter_map(|(m, c)| m.map(|v| (v, *c)))
        .collect();
    drawn.sort_unstable_by_key(|(v, _)| *v);
    drawn
        .iter()
        .take_while(|(v, _)| v < target)
        .map(|(_, c)| width(*c))
        .sum()
}

fn unicode_width(c: char) -> usize {
    ratatui::text::Span::raw(c.to_string()).width()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_to_right_text_is_left_alone() {
        assert!(matches!(visual("Hello, world"), Cow::Borrowed(_)));
        let spans = vec![Span::raw("plain")];
        let (out, map) = visual_spans(spans.clone());
        assert_eq!(out, spans);
        assert!(map.is_none());
    }

    #[test]
    fn right_to_left_text_is_reversed_for_display() {
        // "Shalom" in Hebrew, logical order: shin, lamed, vav, mem.
        let logical = "\u{05e9}\u{05dc}\u{05d5}\u{05dd}";
        let shown = visual(logical);
        assert_eq!(shown, "\u{05dd}\u{05d5}\u{05dc}\u{05e9}");
        // Numbers inside keep their order; controls are not drawn.
        let s = "\u{202e}\u{05d0} 12\u{202c}";
        assert!(!visual(s).contains('\u{202e}'));
        // A left-to-right word in right-to-left text stays readable.
        let mixed = "\u{05d0}\u{05d1} abc";
        assert!(visual(mixed).contains("abc"));
    }

    #[test]
    fn styled_rows_keep_their_styles_and_map_the_cursor() {
        let bold = Style::new().add_modifier(ratatui::style::Modifier::BOLD);
        let spans = vec![
            Span::styled("\u{05d0}\u{05d1}", bold),
            Span::raw("\u{05d2}"),
        ];
        let (out, map) = visual_spans(spans.clone());
        let text: String = out.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "\u{05d2}\u{05d1}\u{05d0}");
        assert_eq!(out[0].style, Style::default());
        assert_eq!(out[1].style, bold);
        let map = map.unwrap();
        assert_eq!(map, vec![Some(2), Some(1), Some(0)]);
        // The first logical letter is drawn last, at column 2.
        assert_eq!(visual_column(&spans, Some(&map), 0), 2);
        assert_eq!(visual_column(&spans, Some(&map), 2), 0);
    }

    #[test]
    fn the_setting_decides() {
        assert!(reorders(RtlDisplay::On, true));
        assert!(!reorders(RtlDisplay::Off, false));
        assert!(!reorders(RtlDisplay::Auto, true));
    }
}
