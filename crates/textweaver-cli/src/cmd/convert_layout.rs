//! `tw convert` options for the look of PDF and EPUB output: fonts, page
//! size, margins, spacing, large print, page numbers, a title page, a table
//! of contents, and the document language. Owner: Agent W.
//!
//! Kept apart from `convert.rs` so the conversion command itself stays
//! small; `convert.rs` flattens [`LayoutArgs`] into its arguments and calls
//! [`LayoutArgs::apply`].

use textweaver_convert::{PageSize, WriteOptions};
use textweaver_writers::parse_length;

/// Layout arguments for `tw convert`.
#[derive(clap::Args, Debug, Default, Clone, PartialEq)]
pub struct LayoutArgs {
    /// PDF and EPUB: the text font, by name. Bundled: "Atkinson
    /// Hyperlegible Next" (the PDF default), "Atkinson Hyperlegible Mono",
    /// "OpenDyslexic". PDF also takes any installed family or a font file.
    /// EPUB embeds bundled fonts only, with their licence.
    #[arg(long, value_name = "NAME|FILE")]
    pub font: Option<String>,
    /// PDF and EPUB: the code font, by name or file, as for --font
    /// (PDF default: the bundled Atkinson Hyperlegible Mono).
    #[arg(long, value_name = "NAME|FILE")]
    pub code_font: Option<String>,
    /// PDF: text size in points (default 12).
    #[arg(long, value_name = "POINTS", value_parser = parse_points)]
    pub font_size: Option<f32>,
    /// PDF: page size: letter (default), a4, a5, legal, or WIDTHxHEIGHT
    /// with in, mm, cm, or pt (for example 6x9in).
    #[arg(long, value_name = "SIZE", value_parser = PageSize::parse)]
    pub page_size: Option<PageSize>,
    /// PDF: margin on every side, such as 1in, 20mm, or 54pt (default
    /// 1in).
    #[arg(long, value_name = "LENGTH", value_parser = parse_length)]
    pub margin: Option<f32>,
    /// PDF: line spacing as a multiple of the text size (default 1.5).
    #[arg(long, value_name = "MULTIPLE", value_parser = parse_spacing)]
    pub line_spacing: Option<f32>,
    /// PDF: large print: text of 18 points or more, 1.6 line spacing, and
    /// three-quarter-inch margins (other options still apply on top).
    #[arg(long)]
    pub large_print: bool,
    /// PDF: leave out "Page N of M" in the footer.
    #[arg(long)]
    pub no_page_numbers: bool,
    /// PDF: start with a title page (title, author, and date).
    #[arg(long)]
    pub title_page: bool,
    /// PDF: the date for the title page, as it should be printed (default:
    /// the document's own date, if it has one).
    #[arg(long, value_name = "TEXT")]
    pub date: Option<String>,
    /// PDF: add a table of contents whose entries link to their headings.
    #[arg(long)]
    pub contents: bool,
    /// PDF: heading levels the table of contents lists (default 3).
    #[arg(long, value_name = "LEVELS", value_parser = clap::value_parser!(u8).range(1..=6))]
    pub contents_depth: Option<u8>,
    /// PDF and EPUB: the document language (BCP 47, such as en-GB or fr),
    /// replacing the one the source declares.
    #[arg(long, value_name = "TAG")]
    pub lang: Option<String>,
}

fn parse_points(s: &str) -> Result<f32, String> {
    let v = parse_length(s)?;
    if (6.0..=72.0).contains(&v) {
        Ok(v)
    } else {
        Err(format!("{s:?} is outside 6 to 72 points"))
    }
}

fn parse_spacing(s: &str) -> Result<f32, String> {
    match s.trim().parse::<f32>() {
        Ok(v) if (1.0..=3.0).contains(&v) => Ok(v),
        _ => Err(format!("{s:?} is not a line spacing from 1 to 3")),
    }
}

impl LayoutArgs {
    /// `options` with these arguments applied. `--large-print` starts from
    /// the large-print preset; the other arguments then override it.
    pub fn apply(&self, mut options: WriteOptions) -> WriteOptions {
        let pdf = &mut options.pdf;
        if self.large_print {
            let preset = textweaver_convert::PdfOptions::large_print();
            pdf.large_print = true;
            pdf.font_size = preset.font_size;
            pdf.line_spacing = preset.line_spacing;
            pdf.margin = preset.margin;
        }
        if let Some(f) = &self.font {
            pdf.font_family = Some(f.clone());
            options.epub.font = Some(f.clone());
        }
        if let Some(f) = &self.code_font {
            pdf.code_font_family = Some(f.clone());
            options.epub.code_font = Some(f.clone());
        }
        let pdf = &mut options.pdf;
        if let Some(v) = self.font_size {
            pdf.font_size = v;
        }
        if let Some(v) = self.page_size {
            pdf.page_size = v;
        }
        if let Some(v) = self.margin {
            pdf.margin = v;
        }
        if let Some(v) = self.line_spacing {
            pdf.line_spacing = v;
        }
        if self.no_page_numbers {
            pdf.page_numbers = false;
        }
        pdf.title_page |= self.title_page;
        if let Some(d) = self
            .date
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
        {
            pdf.date = Some(d.to_owned());
        }
        pdf.toc |= self.contents;
        if let Some(d) = self.contents_depth {
            pdf.toc_depth = d;
        }
        if let Some(l) = self
            .lang
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
        {
            options.language = Some(l.to_owned());
        }
        options
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        layout: LayoutArgs,
    }

    fn parse(args: &[&str]) -> Result<LayoutArgs, clap::Error> {
        Cli::try_parse_from(std::iter::once("tw").chain(args.iter().copied())).map(|c| c.layout)
    }

    #[test]
    fn defaults_change_nothing() {
        let o = parse(&[]).unwrap().apply(WriteOptions::default());
        assert_eq!(o, WriteOptions::default());
    }

    #[test]
    fn every_option_reaches_the_writers() {
        let a = parse(&[
            "--font",
            "OpenDyslexic",
            "--code-font",
            "Atkinson Hyperlegible Mono",
            "--font-size",
            "14",
            "--page-size",
            "a4",
            "--margin",
            "20mm",
            "--line-spacing",
            "1.8",
            "--no-page-numbers",
            "--title-page",
            "--date",
            "Spring term 2027",
            "--contents",
            "--contents-depth",
            "2",
            "--lang",
            "en-GB",
        ])
        .unwrap();
        let o = a.apply(WriteOptions::default());
        assert_eq!(o.pdf.font_family.as_deref(), Some("OpenDyslexic"));
        assert_eq!(o.epub.font.as_deref(), Some("OpenDyslexic"));
        assert_eq!(
            o.pdf.code_font_family.as_deref(),
            Some("Atkinson Hyperlegible Mono")
        );
        assert_eq!(o.pdf.font_size, 14.0);
        assert_eq!(o.pdf.page_size, PageSize::A4);
        assert!((o.pdf.margin - 56.69).abs() < 0.01);
        assert_eq!(o.pdf.line_spacing, 1.8);
        assert!(!o.pdf.page_numbers && o.pdf.title_page && o.pdf.toc);
        assert_eq!(o.pdf.toc_depth, 2);
        assert_eq!(o.pdf.date.as_deref(), Some("Spring term 2027"));
        assert_eq!(o.language.as_deref(), Some("en-GB"));
    }

    #[test]
    fn large_print_is_a_preset_that_other_options_override() {
        let o = parse(&["--large-print"])
            .unwrap()
            .apply(WriteOptions::default());
        assert!(o.pdf.large_print);
        assert_eq!(o.pdf.font_size, 18.0);
        assert_eq!(o.pdf.margin, 54.0);
        let o = parse(&["--large-print", "--font-size", "24", "--margin", "1in"])
            .unwrap()
            .apply(WriteOptions::default());
        assert_eq!(o.pdf.font_size, 24.0);
        assert_eq!(o.pdf.margin, 72.0);
    }

    #[test]
    fn bad_values_are_refused_with_a_reason() {
        for bad in [
            &["--font-size", "200"][..],
            &["--page-size", "huge"],
            &["--margin", "wide"],
            &["--line-spacing", "9"],
            &["--contents-depth", "7"],
        ] {
            let e = parse(bad).unwrap_err().to_string();
            assert!(e.contains("invalid value"), "{bad:?}: {e}");
        }
    }
}
