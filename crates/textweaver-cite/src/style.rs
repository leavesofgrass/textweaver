//! Citation styles: the CSL styles bundled with hayagriva, or a `.csl`
//! file.
//!
//! Short names cover the styles students are most often asked for:
//! `apa`, `mla`, `chicago` (author-date), `chicago-notes`, `ieee`,
//! `vancouver`, `harvard`, `ama`, `nature`. Any name hayagriva's archive
//! knows (its roughly eighty styles) also works, and so does the path of
//! any `.csl` file from the Zotero style repository. A dependent style
//! (one that only points at a parent) is resolved through the archive.

use std::path::Path;
use std::sync::OnceLock;

use hayagriva::archive::{self, ArchivedStyle};
use hayagriva::citationberg::{
    CitationFormat, IndependentStyle, Locale, LocaleCode, Style, StyleCategory, StyleClass,
};

use crate::error::{CiteError, Result};

/// A loaded citation style.
#[derive(Clone, Debug)]
pub struct CitationStyle {
    pub(crate) csl: IndependentStyle,
    pub(crate) locale: Option<LocaleCode>,
    name: String,
}

/// A built-in style, for menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuiltinStyle {
    /// The short name to type (`apa`).
    pub name: &'static str,
    /// The style's full title, for menus and speech.
    pub title: &'static str,
}

/// The styles offered first, in menu order: `(short name, archive name)`.
const FEATURED: &[(&str, &str)] = &[
    ("apa", "apa"),
    ("mla", "modern-language-association"),
    ("chicago", "chicago-author-date"),
    ("chicago-notes", "chicago-notes"),
    ("harvard", "harvard-cite-them-right"),
    ("ieee", "ieee"),
    ("vancouver", "vancouver"),
    ("ama", "american-medical-association"),
    ("nature", "nature"),
];

/// Built-in styles: the featured short names first, then every other
/// archived style under its first archive name.
pub fn builtin_styles() -> Vec<BuiltinStyle> {
    let mut out: Vec<BuiltinStyle> = FEATURED
        .iter()
        .filter_map(|(short, archived)| {
            ArchivedStyle::by_name(archived).map(|s| BuiltinStyle {
                name: short,
                title: s.display_name(),
            })
        })
        .collect();
    let featured: Vec<ArchivedStyle> = FEATURED
        .iter()
        .filter_map(|(_, a)| ArchivedStyle::by_name(a))
        .collect();
    for s in ArchivedStyle::all() {
        if featured.contains(s) {
            continue;
        }
        if let Some(name) = s.names().first() {
            out.push(BuiltinStyle {
                name,
                title: s.display_name(),
            });
        }
    }
    out
}

/// Every CSL locale bundled with hayagriva, parsed once.
pub(crate) fn locales() -> &'static [Locale] {
    static LOCALES: OnceLock<Vec<Locale>> = OnceLock::new();
    LOCALES.get_or_init(archive::locales)
}

impl CitationStyle {
    /// A built-in style by short name (`apa`, `mla`, `chicago`, ...), by
    /// archive name (`chicago-author-date`), or by CSL id (a zotero.org
    /// style URL). Case and surrounding spaces are ignored.
    pub fn builtin(name: &str) -> Result<Self> {
        let wanted = name.trim().to_ascii_lowercase();
        let archived = FEATURED
            .iter()
            .find(|(short, _)| *short == wanted)
            .and_then(|(_, a)| ArchivedStyle::by_name(a))
            .or_else(|| {
                let alias = match wanted.as_str() {
                    "chicago-author-date" | "chicago-ad" => "chicago-author-date",
                    "chicago-note" | "chicago-notes-bibliography" => "chicago-notes",
                    "modern-language-association" => "modern-language-association",
                    "american-medical-association" => "american-medical-association",
                    other => other,
                };
                ArchivedStyle::by_name(alias)
            })
            .or_else(|| ArchivedStyle::by_id(name.trim()))
            .ok_or_else(|| CiteError::UnknownStyle {
                name: name.trim().to_owned(),
            })?;
        Self::from_style(archived.get(), Some(archived.display_name()))
    }

    /// A style from a `.csl` file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let xml = std::fs::read_to_string(path).map_err(|e| CiteError::io("read", path, e))?;
        Self::from_xml(&xml)
    }

    /// A style from CSL XML.
    pub fn from_xml(xml: &str) -> Result<Self> {
        let style =
            Style::from_xml(xml.trim_start_matches('\u{feff}')).map_err(|e| CiteError::Style {
                message: format!("the CSL file is not valid: {e}"),
            })?;
        Self::from_style(style, None)
    }

    /// A style by name or, when `spec` ends in `.csl` or names an existing
    /// file, from that file.
    pub fn resolve(spec: &str) -> Result<Self> {
        let path = Path::new(spec.trim());
        let is_file = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("csl"))
            || path.is_file();
        if is_file {
            Self::from_file(path)
        } else {
            Self::builtin(spec)
        }
    }

    fn from_style(style: Style, display: Option<&str>) -> Result<Self> {
        match style {
            Style::Independent(csl) => {
                let name = display
                    .map(str::to_owned)
                    .unwrap_or_else(|| csl.info.title.value.clone());
                Ok(CitationStyle {
                    locale: None,
                    name,
                    csl,
                })
            }
            Style::Dependent(dep) => {
                let parent = ArchivedStyle::by_id(&dep.parent_link.href).ok_or_else(|| CiteError::Style {
                    message: format!(
                        "{} depends on the style {}, which is not built in; download that parent style's .csl file and use it instead",
                        dep.info.title.value, dep.parent_link.href
                    ),
                })?;
                let Style::Independent(csl) = parent.get() else {
                    return Err(CiteError::Style {
                        message: "the parent style is itself dependent".to_owned(),
                    });
                };
                Ok(CitationStyle {
                    locale: dep.default_locale,
                    name: dep.info.title.value,
                    csl,
                })
            }
        }
    }

    /// Uses a locale for terms and dates (`en-GB`, `de-DE`) instead of the
    /// style's default.
    pub fn with_locale(mut self, locale: &str) -> Self {
        self.locale = Some(LocaleCode(locale.to_owned()));
        self
    }

    /// The style's title ("American Psychological Association 7th edition").
    pub fn title(&self) -> &str {
        &self.name
    }

    /// Whether citations are footnotes (Chicago notes) rather than in the
    /// text.
    pub fn is_note_style(&self) -> bool {
        self.csl.settings.class == StyleClass::Note
    }

    /// Whether citations are numbers (IEEE, Vancouver).
    pub fn is_numeric(&self) -> bool {
        self.csl.info.category.iter().any(|c| {
            matches!(
                c,
                StyleCategory::CitationFormat {
                    format: CitationFormat::Numeric
                }
            )
        })
    }

    /// Whether the style defines a bibliography.
    pub fn has_bibliography(&self) -> bool {
        self.csl.bibliography.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn featured_styles_load() {
        for (short, _) in FEATURED {
            let s = CitationStyle::builtin(short).unwrap_or_else(|e| panic!("{short}: {e}"));
            assert!(!s.title().is_empty());
        }
        assert!(CitationStyle::builtin("IEEE").unwrap().is_numeric());
        assert!(
            CitationStyle::builtin("chicago-notes")
                .unwrap()
                .is_note_style()
        );
        assert!(!CitationStyle::builtin("apa").unwrap().is_note_style());
    }

    #[test]
    fn unknown_style_names_the_choices() {
        let err = CitationStyle::builtin("no-such-style")
            .unwrap_err()
            .to_string();
        assert!(err.contains("apa, mla, chicago"), "{err}");
    }

    #[test]
    fn builtin_list_starts_with_featured() {
        let list = builtin_styles();
        assert_eq!(list[0].name, "apa");
        assert!(list.len() > 50);
        assert!(list.iter().any(|s| s.name == "ieee"));
    }
}
