//! Publishing templates (ADR-0041): Star's accessible templates as
//! textweaver's, for EPUB, DOCX, and PDF.
//!
//! A [`Template`] is a named look plus the parts a kind of document needs:
//!
//! - **APA student paper** (APA 7): a title page (title, author,
//!   affiliation, course, instructor, date), double spacing, first-line
//!   indents, APA heading levels, page numbers at the top right, and
//!   references with hanging indents.
//! - **AMA manuscript** (AMA 11): a title page (title, author, affiliation,
//!   corresponding author, word count), double spacing, and page numbers.
//! - **Large print**, **dyslexia-friendly**, and **high contrast**: reading
//!   looks, a stylesheet in EPUB and matching styles in Word and PDF.
//! - **Manuscript**: standard manuscript format, a title page with a word
//!   count, double spacing, a running head, and centered scene breaks.
//!
//! Every template keeps the structure screen readers use: real heading
//! styles with outline levels in Word, `h1` to `h6` in EPUB, tagged
//! headings in PDF, and real Word footnotes. A template changes how the
//! document looks, never what it says or how it is navigated.
//!
//! Title page fields come from the document's front matter: `title`,
//! `author`, `affiliation` (or `institution`, `university`), `course`,
//! `instructor` (or `professor`), `date` (or `due-date`), and
//! `corresponding` (or `corresponding-author`).

use serde::{Deserialize, Serialize};
use textweaver_text::Document;

use crate::WriteOptions;
use crate::model::{self, Block, Facts};

/// A publishing template.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Template {
    /// An APA 7 student paper.
    #[serde(rename = "apa")]
    ApaStudentPaper,
    /// An AMA 11 manuscript.
    #[serde(rename = "ama")]
    AmaManuscript,
    /// Large print.
    #[serde(rename = "large-print")]
    LargePrint,
    /// Dyslexia-friendly.
    #[serde(rename = "dyslexia-friendly")]
    DyslexiaFriendly,
    /// High contrast.
    #[serde(rename = "high-contrast")]
    HighContrast,
    /// Standard manuscript format.
    #[serde(rename = "manuscript")]
    Manuscript,
}

impl Template {
    /// Every template, in the order lists show them.
    pub const ALL: [Template; 6] = [
        Template::ApaStudentPaper,
        Template::AmaManuscript,
        Template::LargePrint,
        Template::DyslexiaFriendly,
        Template::HighContrast,
        Template::Manuscript,
    ];

    /// The name `tw convert --template` takes (`"apa"`, `"large-print"`).
    pub fn name(self) -> &'static str {
        match self {
            Template::ApaStudentPaper => "apa",
            Template::AmaManuscript => "ama",
            Template::LargePrint => "large-print",
            Template::DyslexiaFriendly => "dyslexia-friendly",
            Template::HighContrast => "high-contrast",
            Template::Manuscript => "manuscript",
        }
    }

    /// The name as it is said ("APA student paper").
    pub fn spoken_name(self) -> &'static str {
        match self {
            Template::ApaStudentPaper => "APA student paper",
            Template::AmaManuscript => "AMA manuscript",
            Template::LargePrint => "Large print",
            Template::DyslexiaFriendly => "Dyslexia-friendly",
            Template::HighContrast => "High contrast",
            Template::Manuscript => "Manuscript",
        }
    }

    /// One sentence on what the template does, meaning first.
    pub fn description(self) -> &'static str {
        match self {
            Template::ApaStudentPaper => {
                "APA 7 student paper: title page, double spacing, APA headings, page numbers, hanging references."
            }
            Template::AmaManuscript => {
                "AMA 11 manuscript: title page with word count, double spacing, page numbers."
            }
            Template::LargePrint => {
                "Large print: 18-point sans-serif text, wide spacing, bold for emphasis."
            }
            Template::DyslexiaFriendly => {
                "Dyslexia-friendly: sans-serif text, wider letter and line spacing, cream background."
            }
            Template::HighContrast => {
                "High contrast: light text on black in EPUB, heavy black text in print."
            }
            Template::Manuscript => {
                "Manuscript: standard manuscript format, double spacing, running head, word count."
            }
        }
    }

    /// The template for a name, ignoring case, spaces, and underscores:
    /// `apa`, `apa-student-paper`, `ama`, `ama-manuscript`, `large-print`,
    /// `dyslexia`, `dyslexia-friendly`, `high-contrast`, `contrast`,
    /// `manuscript`.
    pub fn parse(name: &str) -> Option<Template> {
        let key: String = name
            .trim()
            .to_ascii_lowercase()
            .chars()
            .map(|c| if c == '_' || c == ' ' { '-' } else { c })
            .collect();
        match key.as_str() {
            "apa" | "apa-student-paper" | "apa-student" | "apa7" | "apa-7" => {
                Some(Template::ApaStudentPaper)
            }
            "ama" | "ama-manuscript" | "ama11" | "ama-11" => Some(Template::AmaManuscript),
            "large-print" | "largeprint" | "large" => Some(Template::LargePrint),
            "dyslexia" | "dyslexia-friendly" | "dyslexic" => Some(Template::DyslexiaFriendly),
            "high-contrast" | "highcontrast" | "contrast" => Some(Template::HighContrast),
            "manuscript" | "standard-manuscript" => Some(Template::Manuscript),
            _ => None,
        }
    }

    /// The template's EPUB stylesheet, added after the book's own rules.
    pub fn stylesheet(self) -> &'static str {
        match self {
            Template::LargePrint => include_str!("../templates/large-print.css"),
            Template::DyslexiaFriendly => include_str!("../templates/dyslexia-friendly.css"),
            Template::HighContrast => include_str!("../templates/high-contrast.css"),
            Template::ApaStudentPaper | Template::AmaManuscript | Template::Manuscript => {
                include_str!("../templates/manuscript.css")
            }
        }
    }

    /// True when the template starts with a title page (Word and PDF).
    pub fn has_title_page(self) -> bool {
        matches!(
            self,
            Template::ApaStudentPaper | Template::AmaManuscript | Template::Manuscript
        )
    }

    /// Sets `options` to the template's defaults: the template itself, a
    /// cover for EPUB, and the PDF layout (spacing, size, title page).
    /// Options set afterwards override these, so `tw convert` applies the
    /// template first and the layout options on top.
    pub fn apply(self, options: &mut WriteOptions) {
        options.template = Some(self);
        options.epub.cover = true;
        let pdf = &mut options.pdf;
        match self {
            Template::ApaStudentPaper | Template::AmaManuscript | Template::Manuscript => {
                pdf.font_size = 12.0;
                pdf.line_spacing = 2.0;
                pdf.margin = 72.0;
                pdf.title_page = true;
            }
            Template::LargePrint => {
                let preset = crate::PdfOptions::large_print();
                pdf.large_print = true;
                pdf.font_size = preset.font_size;
                pdf.line_spacing = preset.line_spacing;
                pdf.margin = preset.margin;
            }
            Template::DyslexiaFriendly => {
                pdf.font_size = 14.0;
                pdf.line_spacing = 1.6;
            }
            Template::HighContrast => {
                pdf.font_size = 14.0;
                pdf.line_spacing = 1.5;
            }
        }
    }

    /// The Word look.
    pub(crate) fn docx(self) -> DocxLook {
        DocxLook::of(Some(self))
    }
}

impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// How one heading level looks in Word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HeadingLook {
    /// Size in half-points.
    pub size: u32,
    pub center: bool,
    pub italic: bool,
    /// Indent in twips (APA levels 4 and 5).
    pub indent: u32,
    /// Space before, in twips.
    pub before: u32,
    /// Space after, in twips.
    pub after: u32,
}

/// A Word look: the styles a template sets. `DocxLook::of(None)` is the
/// look textweaver's DOCX had before templates, unchanged.
#[derive(Clone, Debug)]
pub(crate) struct DocxLook {
    pub font: &'static str,
    /// Body text size in half-points.
    pub size: u32,
    /// Line spacing in 240ths of a line (480 is double).
    pub line: u32,
    /// Space after a paragraph, in twips.
    pub after: u32,
    /// First-line indent of body paragraphs, in twips (0: none, and body
    /// paragraphs keep the Normal style).
    pub first_line: u32,
    /// Heading levels 1 to 6 as the template numbers them.
    pub headings: [HeadingLook; 6],
    /// Explicit text color (`"000000"`), when the template sets one.
    pub color: Option<&'static str>,
    /// Hyperlink color.
    pub link_color: &'static str,
    /// Character spacing in twentieths of a point.
    pub letter_spacing: u32,
    /// Page color, shown on screen (Word does not print it).
    pub background: Option<&'static str>,
    /// Italic and plain underline written as bold.
    pub no_italic: bool,
    /// Page numbers at the top right.
    pub page_numbers: bool,
    /// Running head text before the page number (manuscript).
    pub running_head: bool,
    /// References after a "References" heading get hanging indents.
    pub hanging_references: bool,
    /// Rules become a centered "#" (manuscript scene breaks).
    pub scene_breaks: bool,
}

const fn h(
    size: u32,
    center: bool,
    italic: bool,
    indent: u32,
    before: u32,
    after: u32,
) -> HeadingLook {
    HeadingLook {
        size,
        center,
        italic,
        indent,
        before,
        after,
    }
}

impl DocxLook {
    pub(crate) fn of(template: Option<Template>) -> DocxLook {
        let standard = DocxLook {
            font: "Calibri",
            size: 24,
            line: 360,
            after: 160,
            first_line: 0,
            headings: [
                h(36, false, false, 0, 360, 120),
                h(32, false, false, 0, 360, 120),
                h(28, false, false, 0, 240, 120),
                h(26, false, false, 0, 240, 120),
                h(24, false, false, 0, 240, 120),
                h(24, false, true, 0, 240, 120),
            ],
            color: None,
            link_color: "0563C1",
            letter_spacing: 0,
            background: None,
            no_italic: false,
            page_numbers: false,
            running_head: false,
            hanging_references: false,
            scene_breaks: false,
        };
        let Some(t) = template else {
            return standard;
        };
        // Double-spaced papers: no space between paragraphs, headings in
        // the text size set apart by weight, alignment, and slant.
        let paper = DocxLook {
            font: "Times New Roman",
            size: 24,
            line: 480,
            after: 0,
            first_line: 720,
            page_numbers: true,
            ..standard.clone()
        };
        match t {
            // APA 7, section 2.27: level 1 centered bold; 2 flush left
            // bold; 3 flush left bold italic; 4 indented bold; 5 indented
            // bold italic.
            Template::ApaStudentPaper => DocxLook {
                headings: [
                    h(24, true, false, 0, 0, 0),
                    h(24, false, false, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                    h(24, false, false, 720, 0, 0),
                    h(24, false, true, 720, 0, 0),
                    h(24, false, true, 720, 0, 0),
                ],
                hanging_references: true,
                ..paper
            },
            // AMA 11 and the JAMA Network instructions: headings flush
            // left, bold, then bold italic, then italic.
            Template::AmaManuscript => DocxLook {
                headings: [
                    h(24, false, false, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                ],
                ..paper
            },
            Template::Manuscript => DocxLook {
                headings: [
                    h(24, true, false, 0, 480, 0),
                    h(24, false, false, 0, 480, 0),
                    h(24, false, true, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                    h(24, false, true, 0, 0, 0),
                ],
                running_head: true,
                scene_breaks: true,
                ..paper
            },
            // ACB and APH large print: 18 points, sans-serif, bold for
            // emphasis, headings larger again.
            Template::LargePrint => DocxLook {
                font: "Verdana",
                size: 36,
                line: 360,
                after: 240,
                headings: [
                    h(48, false, false, 0, 480, 240),
                    h(44, false, false, 0, 480, 240),
                    h(40, false, false, 0, 360, 200),
                    h(38, false, false, 0, 360, 200),
                    h(36, false, false, 0, 360, 200),
                    h(36, false, false, 0, 360, 200),
                ],
                color: Some("000000"),
                no_italic: true,
                ..standard
            },
            // The British Dyslexia Association's style guide: 12 to 14
            // points sans-serif, wider letter spacing, 1.5 line spacing,
            // bold rather than italic or underline, a cream background.
            Template::DyslexiaFriendly => DocxLook {
                font: "Verdana",
                size: 28,
                line: 360,
                after: 240,
                headings: [
                    h(40, false, false, 0, 480, 200),
                    h(36, false, false, 0, 480, 200),
                    h(32, false, false, 0, 360, 160),
                    h(30, false, false, 0, 360, 160),
                    h(28, false, false, 0, 360, 160),
                    h(28, false, false, 0, 360, 160),
                ],
                color: Some("1A1A1A"),
                link_color: "1A3F8F",
                letter_spacing: 12,
                background: Some("FBF8EE"),
                no_italic: true,
                ..standard
            },
            // Black text on white, the strongest contrast on paper; a page
            // color would not print, and white text would vanish.
            Template::HighContrast => DocxLook {
                font: "Arial",
                size: 28,
                line: 360,
                after: 200,
                headings: [
                    h(44, false, false, 0, 480, 200),
                    h(38, false, false, 0, 480, 200),
                    h(34, false, false, 0, 360, 160),
                    h(32, false, false, 0, 360, 160),
                    h(30, false, false, 0, 360, 160),
                    h(28, false, false, 0, 360, 160),
                ],
                color: Some("000000"),
                link_color: "0000CC",
                ..standard
            },
        }
    }
}

/// A title page: the title, then lines in order (empty strings are blank
/// lines).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TitlePage {
    pub title: String,
    pub lines: Vec<String>,
}

/// The first non-empty front matter value among `keys`.
fn property(doc: &Document, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| {
        doc.meta
            .properties
            .get(*k)
            .map(|v| model::collapse_ws(v))
            .filter(|v| !v.is_empty())
    })
}

/// The date for a title page: the option, else the front matter's.
pub(crate) fn title_date(doc: &Document, options: &WriteOptions) -> Option<String> {
    options
        .pdf
        .date
        .as_deref()
        .map(model::collapse_ws)
        .filter(|d| !d.is_empty())
        .or_else(|| property(doc, &["date", "due-date", "due_date", "due"]))
}

/// The title page of `template`, from the document's facts and front
/// matter; `None` for templates without one.
pub(crate) fn title_page(
    template: Template,
    doc: &Document,
    options: &WriteOptions,
    facts: &Facts,
    blocks: &[Block],
) -> Option<TitlePage> {
    let affiliation = || property(doc, &["affiliation", "institution", "university"]);
    let mut lines = Vec::new();
    match template {
        Template::ApaStudentPaper => {
            // APA 7, section 2.3: a blank line after the title, then the
            // author, affiliation, course, instructor, and due date.
            lines.push(String::new());
            lines.extend(facts.author.clone());
            lines.extend(affiliation());
            lines.extend(property(doc, &["course"]));
            lines.extend(property(doc, &["instructor", "professor", "teacher"]));
            lines.extend(title_date(doc, options));
        }
        Template::AmaManuscript => {
            lines.push(String::new());
            lines.extend(facts.author.clone());
            lines.extend(affiliation());
            if let Some(c) = property(doc, &["corresponding", "corresponding-author"]) {
                lines.push(String::new());
                lines.push(format!("Corresponding author: {c}"));
            }
            lines.push(String::new());
            lines.push(format!("Word count: {}", model::word_count(blocks)));
            lines.extend(title_date(doc, options));
        }
        Template::Manuscript => {
            lines.extend(facts.author.as_ref().map(|a| format!("by {a}")));
            lines.push(String::new());
            // Manuscripts give a round count: to the nearest hundred.
            let n = model::word_count(blocks);
            let about = if n < 100 { n } else { (n + 50) / 100 * 100 };
            lines.push(format!("About {about} words"));
        }
        _ => return None,
    }
    Some(TitlePage {
        title: facts.title.clone(),
        lines,
    })
}

/// The text of the EPUB cover's alternative text: "Cover: Title, by
/// Author."
pub(crate) fn cover_alt(facts: &Facts) -> String {
    match &facts.author {
        Some(a) => format!("Cover: {}, by {a}.", facts.title),
        None => format!("Cover: {}.", facts.title),
    }
}

/// The EPUB cover as SVG: the title and author on a plain field, in the
/// template's colors, from `templates/cover.svg`.
pub(crate) fn cover_svg(template: Option<Template>, facts: &Facts) -> String {
    // (background, foreground, accent, font); every pair above 7 to 1.
    let (bg, fg, accent, font) = match template {
        Some(Template::HighContrast) => (
            "#000000",
            "#ffffff",
            "#ffff00",
            "Verdana, Arial, sans-serif",
        ),
        Some(Template::DyslexiaFriendly) => (
            "#fbf8ee",
            "#1a1a1a",
            "#1a3f8f",
            "Verdana, Arial, sans-serif",
        ),
        Some(Template::LargePrint) => (
            "#ffffff",
            "#000000",
            "#1a3f8f",
            "Verdana, Arial, sans-serif",
        ),
        Some(Template::ApaStudentPaper | Template::AmaManuscript | Template::Manuscript) => (
            "#ffffff",
            "#000000",
            "#000000",
            "Times New Roman, Times, serif",
        ),
        None => (
            "#1a3f8f",
            "#ffffff",
            "#ffffff",
            "Verdana, Arial, sans-serif",
        ),
    };
    let title_lines = wrap(&facts.title, 18, 6);
    let size = if title_lines.len() > 4 { 80 } else { 100 };
    let lead = size * 6 / 5;
    let block = lead * title_lines.len() as u32;
    let top = 700u32.saturating_sub(block / 2).max(200);
    let mut titles = String::new();
    for (n, line) in title_lines.iter().enumerate() {
        titles.push_str(&format!(
            "<text x=\"600\" y=\"{}\" font-size=\"{size}\" font-weight=\"bold\">{}</text>\n",
            top + lead * (n as u32 + 1),
            crate::xml::text(line)
        ));
    }
    let mut authors = String::new();
    if let Some(a) = &facts.author {
        for (n, line) in wrap(a, 28, 3).iter().enumerate() {
            authors.push_str(&format!(
                "<text x=\"600\" y=\"{}\" font-size=\"64\">{}</text>\n",
                1450 + 80 * n as u32,
                crate::xml::text(line)
            ));
        }
    }
    include_str!("../templates/cover.svg")
        .replace("{{alt}}", &crate::xml::text(&cover_alt(facts)))
        .replace("{{background}}", bg)
        .replace("{{foreground}}", fg)
        .replace("{{accent}}", accent)
        .replace("{{font}}", font)
        .replace("{{title_lines}}\n", &titles)
        .replace("{{author_lines}}\n", &authors)
}

/// `text` in lines of at most about `width` characters, at most `max`
/// lines (the last one ends with an ellipsis when text is left over).
fn wrap(text: &str, width: usize, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.len() > max {
        lines.truncate(max);
        if let Some(last) = lines.last_mut() {
            last.push('\u{2026}');
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_and_round_trip() {
        for t in Template::ALL {
            assert_eq!(Template::parse(t.name()), Some(t));
            let json = format!("\"{}\"", t.name());
            assert_eq!(serde_json::from_str::<Template>(&json).unwrap(), t);
            assert!(
                t.description()
                    .starts_with(t.spoken_name().split(' ').next().unwrap())
            );
            // A description fits two lines of a 40-cell display at most
            // with its meaning in the first line.
            assert!(t.description().len() <= 100, "{t}");
        }
        assert_eq!(
            Template::parse("APA Student Paper"),
            Some(Template::ApaStudentPaper)
        );
        assert_eq!(
            Template::parse("dyslexia"),
            Some(Template::DyslexiaFriendly)
        );
        assert_eq!(Template::parse("default"), None);
    }

    #[test]
    fn apply_sets_the_pdf_layout_and_the_cover() {
        let mut o = WriteOptions::default();
        Template::ApaStudentPaper.apply(&mut o);
        assert_eq!(o.template, Some(Template::ApaStudentPaper));
        assert!(o.epub.cover && o.pdf.title_page);
        assert_eq!(o.pdf.line_spacing, 2.0);
        let mut o = WriteOptions::default();
        Template::LargePrint.apply(&mut o);
        assert!(o.pdf.large_print && o.pdf.font_size >= crate::LARGE_PRINT_MIN_SIZE);
        assert!(!o.pdf.title_page);
    }

    #[test]
    fn wrapping_keeps_words_whole() {
        assert_eq!(wrap("A short title", 18, 6), ["A short title"]);
        assert_eq!(
            wrap(
                "Reading aloud with a screen reader and a braille display",
                18,
                6
            ),
            [
                "Reading aloud with",
                "a screen reader",
                "and a braille",
                "display"
            ]
        );
        let long = wrap(&"word ".repeat(100), 18, 2);
        assert_eq!(long.len(), 2);
        assert!(long[1].ends_with('\u{2026}'));
    }

    #[test]
    fn cover_is_well_formed_and_labelled() {
        let facts = Facts {
            title: "Tea & <Biscuits>".to_owned(),
            language: "en".to_owned(),
            author: Some("Ada Example".to_owned()),
        };
        for t in Template::ALL.map(Some).into_iter().chain([None]) {
            let svg = cover_svg(t, &facts);
            let doc = roxmltree::Document::parse(&svg).expect("the cover parses");
            let title = doc
                .descendants()
                .find(|n| n.tag_name().name() == "title")
                .and_then(|n| n.text())
                .unwrap();
            assert_eq!(title, "Cover: Tea & <Biscuits>, by Ada Example.");
            assert!(!svg.contains("{{"), "every placeholder is filled");
        }
    }
}
