//! Page templates with MiniJinja.
//!
//! Built-in templates:
//!
//! | Name | Use |
//! |---|---|
//! | `default` | An accessible standalone page: `lang`, a skip link, a `main` landmark, an optional table of contents in a labelled `nav`, and a readable stylesheet that follows `prefers-color-scheme`, `prefers-contrast`, `prefers-reduced-motion`, `forced-colors`, and print; the reader's [`Typography`] when given. |
//! | `print` | A page for paper or PDF: black on white, serif body unless the reader's [`Typography`] says otherwise, link targets printed after links, no page breaks inside tables, code, or callouts. |
//! | `fragment` | The rendered HTML alone, for embedding. |
//!
//! User templates are HTML files with MiniJinja syntax, loaded from a
//! folder (the name is the file stem) or given as a path; they may extend
//! the built-ins (`{% extends "default" %}`). Every template sees these
//! variables:
//!
//! | Variable | Meaning |
//! |---|---|
//! | `content` | The rendered document body (HTML, not escaped) |
//! | `title` | Front matter `title`, else the first level-1 heading, else the file name |
//! | `lang`, `dir` | Front matter `lang` (or `language`, default `en`) and `dir` |
//! | `author`, `date`, `description`, `keywords` | From front matter, as text |
//! | `meta` | All front matter, as an object |
//! | `toc` | The table of contents as a nested list (HTML), or empty |
//! | `toc_entries` | The headings: `level`, `id`, `text` |
//! | `tags` | Obsidian `#tags` found |
//! | `has_h1`, `has_math` | Whether the body has a level-1 heading or formulas |
//! | `stylesheet`, `print_stylesheet` | The built-in CSS, after the theme's colors and the reader's [`Typography`] |
//! | `generator` | `textweaver` and its version |

use std::path::{Path, PathBuf};

use minijinja::{AutoEscape, Environment, Value, context};

use crate::frontmatter::value_text;
use crate::{RenderError, Rendered};

const DEFAULT: &str = include_str!("../templates/default.html");
const PRINT: &str = include_str!("../templates/print.html");
const FRAGMENT: &str = include_str!("../templates/fragment.html");
/// The built-in screen rules. They read the theme's `--tw-*` properties,
/// which [`stylesheet`] puts in front of them.
pub const STYLESHEET: &str = include_str!("../templates/style.css");

/// The screen stylesheet templates receive: the theme's properties
/// (Galaxy, Galaxy Light when the system asks for light, High Contrast
/// when it asks for more contrast, system colors under forced colors),
/// then [`STYLESHEET`].
pub fn stylesheet() -> &'static str {
    static CSS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CSS.get_or_init(|| {
        format!(
            "{}{STYLESHEET}",
            textweaver_theme::css::default_stylesheet()
        )
    })
}
/// The built-in print stylesheet.
pub const PRINT_STYLESHEET: &str = include_str!("../templates/print.css");

/// Names of the built-in templates.
pub const BUILTIN: [&str; 3] = ["default", "print", "fragment"];

/// Extensions of files loaded as templates from a folder.
const TEMPLATE_EXTS: [&str; 5] = ["html", "htm", "jinja", "j2", "xhtml"];

/// Which template a page uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplateChoice {
    /// A template by name: a built-in, or one loaded from a folder.
    Named(String),
    /// A template file.
    File(PathBuf),
}

impl Default for TemplateChoice {
    fn default() -> Self {
        TemplateChoice::Named("default".to_owned())
    }
}

impl TemplateChoice {
    /// A name or a path, as given on the command line: an existing file or
    /// anything with a path separator or extension is a file.
    pub fn parse(s: &str) -> Self {
        let p = Path::new(s);
        if p.is_file() || s.contains(['/', '\\']) || p.extension().is_some() {
            TemplateChoice::File(p.to_owned())
        } else {
            TemplateChoice::Named(s.to_owned())
        }
    }
}

/// Per-page settings.
#[derive(Clone, Debug)]
pub struct PageOptions {
    /// Title when the document gives none (usually the file stem).
    pub fallback_title: Option<String>,
    /// Language when front matter gives none.
    pub default_lang: String,
    /// Include the table of contents (when there are at least two headings).
    pub toc: bool,
    /// The theme's properties to put in front of [`STYLESHEET`], such as
    /// `textweaver_theme::css::single_stylesheet` of the reader's chosen
    /// theme; `None` uses [`stylesheet`] (Galaxy).
    pub theme_css: Option<String>,
    /// The reader's font, spacing, and line length, from the reading
    /// settings at export time; `None` keeps the stylesheets' own
    /// (Atkinson Hyperlegible or the system font, 70 characters).
    pub typography: Option<Typography>,
}

impl Default for PageOptions {
    fn default() -> Self {
        PageOptions {
            fallback_title: None,
            default_lang: "en".to_owned(),
            toc: true,
            theme_css: None,
            typography: None,
        }
    }
}

/// The reader's typography for a page: the owner's choice is that a page
/// looks like the reader, so the font, size, weight, spacing, and line
/// length come from the reading settings (`[reading_aids.font]`,
/// `[reading_aids.spacing]`, `[display] measure`) at export time, and
/// themes keep colors only. [`Typography::css`] sets the `--tw-type-*`
/// properties that `style.css` and `print.css` read, each with a fallback,
/// so a page without them (and every user template) still works.
#[derive(Clone, Debug, PartialEq)]
pub struct Typography {
    /// A CSS `font-family` value: the family and its fallbacks, quoted,
    /// then a generic family (`textweaver_fonts`' `css_font_family`).
    pub font_family: String,
    /// Body size in points; the screen page scales it from the browser's
    /// own default size (12 points), so the reader's browser zoom and
    /// minimum font size still apply.
    pub size_pt: f32,
    /// Weight, 100 to 900 (400 regular).
    pub weight: u16,
    /// Line height, a multiple of the font size.
    pub line_height: f32,
    /// Space after a paragraph, a multiple of the font size.
    pub paragraph_spacing: f32,
    /// Extra space between letters, a multiple of the font size.
    pub letter_spacing: f32,
    /// Extra space between words, a multiple of the font size.
    pub word_spacing: f32,
    /// Line length in characters, 25 to 90; 0 fills the window.
    pub measure: u16,
}

/// A number for CSS: at most three decimals, without trailing zeros.
fn css_number(v: f32) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

impl Typography {
    /// The `--tw-type-*` properties on `:root`, with every value clamped to
    /// the settings' ranges and the family cleaned of anything that could
    /// end the declaration or the style element.
    pub fn css(&self) -> String {
        let finite = |v: f32, lo: f32, hi: f32, fallback: f32| {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                fallback
            }
        };
        let size = finite(self.size_pt, 6.0, 144.0, 14.0);
        let family: String = self
            .font_family
            .chars()
            .filter(|c| !matches!(c, '{' | '}' | ';' | '<' | '>' | '\\') && !c.is_control())
            .collect();
        let family = if family.trim().is_empty() {
            "system-ui, sans-serif".to_owned()
        } else {
            family
        };
        let measure = match self.measure {
            0 => "none".to_owned(),
            m => format!("{}ch", m.clamp(25, 90)),
        };
        format!(
            "/* textweaver reading settings */\n:root {{\n  --tw-type-family: {family};\n  --tw-type-size: {}%;\n  --tw-type-print-size: {}pt;\n  --tw-type-weight: {};\n  --tw-type-line-height: {};\n  --tw-type-paragraph-spacing: {}em;\n  --tw-type-letter-spacing: {}em;\n  --tw-type-word-spacing: {}em;\n  --tw-type-measure: {measure};\n}}\n",
            css_number(size / 12.0 * 100.0),
            css_number(size),
            self.weight.clamp(100, 900),
            css_number(finite(self.line_height, 1.0, 3.0, 1.5)),
            css_number(finite(self.paragraph_spacing, 0.0, 4.0, 1.0)),
            css_number(finite(self.letter_spacing, 0.0, 0.5, 0.0)),
            css_number(finite(self.word_spacing, 0.0, 1.0, 0.0)),
        )
    }
}

/// A set of templates, shared by all conversion workers.
pub struct Templates {
    env: Environment<'static>,
}

impl std::fmt::Debug for Templates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Templates")
            .field("names", &self.names())
            .finish()
    }
}

impl Default for Templates {
    fn default() -> Self {
        Self::builtin()
    }
}

impl Templates {
    /// The built-in templates.
    pub fn builtin() -> Self {
        let mut env = Environment::new();
        env.set_auto_escape_callback(|_| AutoEscape::Html);
        env.set_keep_trailing_newline(true);
        for (name, src) in [
            ("default", DEFAULT),
            ("print", PRINT),
            ("fragment", FRAGMENT),
        ] {
            // The built-ins are tested; a failure here is a build defect.
            if let Err(e) = env.add_template(name, src) {
                log::error!("built-in template {name}: {e}");
            }
        }
        Templates { env }
    }

    /// Adds every template file in `dir` (not recursive), named by file
    /// stem; a user template may replace a built-in. Returns the names.
    pub fn load_dir(&mut self, dir: &Path) -> Result<Vec<String>, RenderError> {
        let entries = std::fs::read_dir(dir).map_err(|e| RenderError::Io(dir.to_owned(), e))?;
        let mut names = Vec::new();
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| TEMPLATE_EXTS.contains(&e.to_lowercase().as_str()))
            })
            .collect();
        paths.sort();
        for p in paths {
            names.push(self.add_file(&p)?);
        }
        Ok(names)
    }

    /// Adds one template file, named by its stem; returns the name.
    pub fn add_file(&mut self, path: &Path) -> Result<String, RenderError> {
        let src = std::fs::read_to_string(path).map_err(|e| RenderError::Io(path.to_owned(), e))?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "template".to_owned());
        self.add(&name, src)?;
        Ok(name)
    }

    /// Adds a template from source.
    pub fn add(&mut self, name: &str, src: String) -> Result<(), RenderError> {
        self.env
            .add_template_owned(name.to_owned(), src)
            .map_err(|e| RenderError::Template(format!("{name}: {e}")))
    }

    /// Makes `choice` available and returns the name to render with.
    pub fn resolve(&mut self, choice: &TemplateChoice) -> Result<String, RenderError> {
        match choice {
            TemplateChoice::Named(n) => {
                if self.env.get_template(n).is_ok() {
                    Ok(n.clone())
                } else {
                    Err(RenderError::Template(format!(
                        "no template named {n:?}; available: {}",
                        self.names().join(", ")
                    )))
                }
            }
            TemplateChoice::File(p) => self.add_file(p),
        }
    }

    /// Names of the available templates, sorted.
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.env.templates().map(|(n, _)| n.to_owned()).collect();
        v.sort();
        v
    }

    /// Renders `doc` into the page template `name`.
    pub fn render(
        &self,
        name: &str,
        doc: &Rendered,
        page: &PageOptions,
    ) -> Result<String, RenderError> {
        let tmpl = self
            .env
            .get_template(name)
            .map_err(|e| RenderError::Template(format!("{name}: {e}")))?;
        let text = |k: &str| doc.meta.get(k).map(value_text).filter(|s| !s.is_empty());
        let title = doc
            .title()
            .or_else(|| page.fallback_title.clone())
            .unwrap_or_else(|| "Untitled".to_owned());
        let lang = text("lang")
            .or_else(|| text("language"))
            .unwrap_or_else(|| page.default_lang.clone());
        let keywords =
            text("keywords").or_else(|| (!doc.tags.is_empty()).then(|| doc.tags.join(", ")));
        let toc = if page.toc && doc.toc.len() >= 2 {
            doc.toc_html()
        } else {
            String::new()
        };
        let typography = page
            .typography
            .as_ref()
            .map(Typography::css)
            .unwrap_or_default();
        let ctx = context! {
            content => Value::from_safe_string(doc.html.clone()),
            title => title,
            lang => lang,
            dir => text("dir"),
            author => text("author"),
            date => text("date"),
            description => text("description").or_else(|| text("abstract")),
            keywords => keywords,
            meta => Value::from_serialize(&doc.meta),
            toc => Value::from_safe_string(toc),
            toc_entries => Value::from_serialize(&doc.toc),
            tags => doc.tags.clone(),
            has_h1 => doc.has_h1,
            has_math => doc.has_math,
            stylesheet => Value::from_safe_string(match &page.theme_css {
                Some(theme) => format!("{theme}{typography}{STYLESHEET}"),
                None if typography.is_empty() => stylesheet().to_owned(),
                None => format!(
                    "{}{typography}{STYLESHEET}",
                    textweaver_theme::css::default_stylesheet()
                ),
            }),
            print_stylesheet => Value::from_safe_string(format!("{typography}{PRINT_STYLESHEET}")),
            generator => concat!("textweaver ", env!("CARGO_PKG_VERSION")),
        };
        tmpl.render(ctx)
            .map_err(|e| RenderError::Template(format!("{name}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `var(--tw-…)` the page rules read is a property the theme's
    /// stylesheet defines, and the theme comes first; every typography
    /// property (`--tw-type-*`) has a fallback and is one that
    /// [`Typography::css`] sets.
    #[test]
    fn every_theme_variable_the_rules_read_exists() {
        let theme = textweaver_theme::css::default_stylesheet();
        let typography = sample().css();
        let mut used = 0;
        for css in [STYLESHEET, PRINT_STYLESHEET] {
            for rest in css.split("var(").skip(1) {
                let name = rest.split([')', ',']).next().unwrap_or("").trim();
                assert!(name.starts_with("--tw-"), "{name} is not a theme property");
                if name.starts_with("--tw-type-") {
                    let after = rest[rest.find(name).unwrap_or(0) + name.len()..].trim_start();
                    assert!(after.starts_with(','), "{name} has no fallback");
                    assert!(
                        typography.contains(&format!("{name}:")),
                        "{name} is never set"
                    );
                } else {
                    assert!(
                        theme.contains(&format!("{name}:")),
                        "{name} is never defined"
                    );
                }
                used += 1;
            }
        }
        assert!(used > 10, "{used}");
        assert!(stylesheet().starts_with(&theme));
    }

    fn sample() -> Typography {
        Typography {
            font_family: "\"Atkinson Hyperlegible\", sans-serif".into(),
            size_pt: 15.0,
            weight: 400,
            line_height: 1.5,
            paragraph_spacing: 1.0,
            letter_spacing: 0.12,
            word_spacing: 0.16,
            measure: 66,
        }
    }

    #[test]
    fn typography_is_clamped_and_cannot_escape_the_style() {
        let css = sample().css();
        assert!(css.contains("--tw-type-size: 125%;"), "{css}");
        assert!(css.contains("--tw-type-print-size: 15pt;"), "{css}");
        assert!(css.contains("--tw-type-measure: 66ch;"), "{css}");
        assert!(css.contains("--tw-type-letter-spacing: 0.12em;"), "{css}");
        let wild = Typography {
            font_family: "x; } </style><script>".into(),
            size_pt: f32::NAN,
            weight: 5000,
            measure: 0,
            line_height: 99.0,
            ..sample()
        }
        .css();
        assert!(!wild.contains("</style"), "{wild}");
        assert!(!wild.contains("; }"), "{wild}");
        assert!(wild.contains("--tw-type-size: 116.667%;"), "{wild}");
        assert!(wild.contains("--tw-type-weight: 900;"), "{wild}");
        assert!(wild.contains("--tw-type-line-height: 3;"), "{wild}");
        assert!(wild.contains("--tw-type-measure: none;"), "{wild}");
    }

    #[test]
    fn typography_reaches_both_stylesheets() {
        let doc = crate::render("# T\n\nBody.\n", &crate::RenderOptions::default());
        let page = PageOptions {
            typography: Some(sample()),
            ..PageOptions::default()
        };
        let t = Templates::builtin();
        for name in ["default", "print"] {
            let html = t.render(name, &doc, &page).unwrap();
            assert!(html.contains("--tw-type-size: 125%;"), "{name}");
        }
        let plain = t.render("default", &doc, &PageOptions::default()).unwrap();
        assert!(!plain.contains("--tw-type-size:"));
    }
}
