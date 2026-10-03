//! The logic behind `tw cite`, independent of the terminal so it can be
//! tested with a temporary library and a recorded HTTP client.
//!
//! Each function returns the text to print on standard output. Messages
//! are complete sentences for speech; data (exports, `--json`) is printed
//! bare so it can be redirected to a file.

use std::path::{Path, PathBuf};

use crate::error::{CiteError, Result};
use crate::formats::{self, Format};
use crate::library::{Layered, Library, ReferenceSource, plural};
use crate::lookup::{Cache, HttpClient, Lookup};
use crate::pandoc::{Citation, CiteItem};
use crate::render::{Formatter, OutputFormat};
use crate::style::{CitationStyle, builtin_styles};

/// Where the commands read and write.
pub struct Context<'a> {
    /// The library commands change (`add`, `import`, `remove`) and list.
    pub library: PathBuf,
    /// More libraries consulted, in order, when resolving keys for
    /// `format` and `check` (the user library behind a folder library).
    pub fallback: Vec<PathBuf>,
    /// Lookup cache folder.
    pub cache_dir: Option<PathBuf>,
    /// HTTP client for lookups.
    pub client: &'a dyn HttpClient,
}

/// What `format` prints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    /// Each bibliography entry followed by its in-text citation.
    #[default]
    Both,
    /// Only the in-text citation of all keys together.
    Citation,
    /// Only the bibliography entries.
    Bibliography,
}

impl Context<'_> {
    fn load(&self) -> Result<Library> {
        Library::load(&self.library)
    }

    fn load_all(&self) -> Result<Vec<Library>> {
        let mut libs = vec![self.load()?];
        for p in &self.fallback {
            if p != &self.library {
                libs.push(Library::load(p)?);
            }
        }
        Ok(libs)
    }
}

/// `tw cite add DOI|ISBN`: looks the identifier up and adds it.
pub fn add(ctx: &Context<'_>, identifier: &str) -> Result<String> {
    let mut lookup = Lookup::new(ctx.client);
    if let Some(dir) = &ctx.cache_dir {
        lookup = lookup.with_cache(Cache::new(dir));
    }
    let reference = lookup.lookup(identifier)?;
    let mut lib = ctx.load()?;
    let outcome = lib.add(reference);
    lib.save()?;
    let label = lib
        .get(outcome.key())
        .map(|r| r.label())
        .unwrap_or_default();
    Ok(format!("{} {label}", outcome.announcement()))
}

/// `tw cite import FILE`: merges a BibTeX, RIS, or CSL-JSON file.
pub fn import(ctx: &Context<'_>, file: &Path) -> Result<String> {
    let refs = formats::read_file(file)?;
    let mut lib = ctx.load()?;
    let report = lib.merge(refs);
    lib.save()?;
    Ok(report.announcement())
}

/// `tw cite export --to FORMAT [KEY...]`: the library (or the given keys)
/// in a format.
pub fn export(ctx: &Context<'_>, format: Format, keys: &[String]) -> Result<String> {
    let lib = ctx.load()?;
    let refs: Vec<_> = if keys.is_empty() {
        lib.sorted().into_iter().cloned().collect()
    } else {
        keys.iter()
            .map(|k| {
                lib.get(k)
                    .cloned()
                    .ok_or_else(|| CiteError::MissingKey { key: k.clone() })
            })
            .collect::<Result<_>>()?
    };
    formats::write(&refs, format)
}

/// `tw cite format KEY... --style STYLE`: formatted citations and entries.
/// No keys means every reference in the library.
pub fn format(
    ctx: &Context<'_>,
    keys: &[String],
    style: &str,
    output: OutputFormat,
    show: Show,
) -> Result<String> {
    let style = CitationStyle::resolve(style)?;
    let libs = ctx.load_all()?;
    let layers: Vec<&Library> = libs.iter().collect();
    let source = Layered { layers: &layers };
    let keys: Vec<String> = if keys.is_empty() {
        libs[0].sorted().into_iter().map(|r| r.id.clone()).collect()
    } else {
        keys.to_vec()
    };
    if keys.is_empty() {
        return Ok(empty_message(&ctx.library));
    }
    let refs = keys
        .iter()
        .map(|k| {
            source
                .get(k)
                .ok_or_else(|| CiteError::MissingKey { key: k.clone() })
        })
        .collect::<Result<Vec<_>>>()?;
    let fmt = Formatter::new(&style, output);
    let separator = if output == OutputFormat::Plain {
        "\n"
    } else {
        "\n\n"
    };
    let out = match show {
        Show::Citation => {
            let cite = Citation {
                range: 0..0,
                chars: Default::default(),
                narrative: false,
                items: keys
                    .iter()
                    .map(|k| CiteItem {
                        key: k.clone(),
                        ..CiteItem::default()
                    })
                    .collect(),
            };
            fmt.citation(&cite, &source)?
        }
        Show::Bibliography => fmt
            .bibliography(&refs)?
            .into_iter()
            .map(|e| e.text)
            .collect::<Vec<_>>()
            .join(separator),
        Show::Both => {
            let mut blocks = Vec::new();
            for r in &refs {
                let entry = fmt.entry(r)?;
                let cite = fmt.cite(r)?;
                let label = if style.is_note_style() {
                    "Note"
                } else {
                    "In text"
                };
                blocks.push(if entry.is_empty() {
                    format!("{label}: {cite}")
                } else {
                    format!("{entry}\n{label}: {cite}")
                });
            }
            blocks.join(separator)
        }
    };
    Ok(out)
}

/// `tw cite list [--json]`.
pub fn list(ctx: &Context<'_>, json: bool) -> Result<String> {
    let lib = ctx.load()?;
    if json {
        let refs: Vec<_> = lib.sorted().into_iter().cloned().collect();
        return crate::csljson::write(&refs);
    }
    if lib.is_empty() {
        return Ok(empty_message(&ctx.library));
    }
    let n = lib.len();
    let mut out = format!(
        "{n} {} in {}:\n",
        plural(n, "reference", "references"),
        ctx.library.display()
    );
    for r in lib.sorted() {
        out.push_str(&r.label());
        out.push('\n');
    }
    Ok(out.trim_end().to_owned())
}

/// `tw cite remove KEY`.
pub fn remove(ctx: &Context<'_>, key: &str) -> Result<String> {
    let mut lib = ctx.load()?;
    match lib.remove(key) {
        Some(r) => {
            lib.save()?;
            Ok(format!("Removed reference {}: {}", key, r.label()))
        }
        None => Err(CiteError::MissingKey {
            key: key.to_owned(),
        }),
    }
}

/// `tw cite styles`: the built-in styles, featured ones first.
pub fn styles() -> String {
    builtin_styles()
        .into_iter()
        .map(|s| format!("{}: {}", s.name, s.title))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `tw cite check FILE`: every citation in a document, and the keys the
/// libraries do not have.
pub fn check(ctx: &Context<'_>, text: &str) -> Result<String> {
    Ok(check_keys(ctx, text)?.message)
}

/// What [`check_keys`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The sentence `tw cite check` prints.
    pub message: String,
    /// The cited keys no library has, in the order they are first cited.
    pub missing: Vec<String>,
}

/// [`check`], with the missing keys apart, so `tw cite check` can exit
/// with status 1 when there are any.
pub fn check_keys(ctx: &Context<'_>, text: &str) -> Result<Checked> {
    let libs = ctx.load_all()?;
    let layers: Vec<&Library> = libs.iter().collect();
    let source = Layered { layers: &layers };
    let cites = crate::pandoc::find_citations(text);
    if cites.is_empty() {
        return Ok(Checked {
            message: "The document has no citations.".to_owned(),
            missing: Vec::new(),
        });
    }
    let mut missing: Vec<&str> = Vec::new();
    for item in cites.iter().flat_map(|c| &c.items) {
        if source.get(&item.key).is_none() && !missing.contains(&item.key.as_str()) {
            missing.push(&item.key);
        }
    }
    let n = cites.len();
    let mut out = format!("{n} {} found.", plural(n, "citation", "citations"));
    if missing.is_empty() {
        out.push_str(" Every key is in the library.");
    } else {
        out.push_str(&format!(
            " {} not in the library: {}.",
            if missing.len() == 1 {
                "One key is"
            } else {
                "These keys are"
            },
            missing.join(", ")
        ));
        out.push_str(" Add references with tw cite add or tw cite import.");
    }
    Ok(Checked {
        message: out,
        missing: missing.into_iter().map(str::to_owned).collect(),
    })
}

fn empty_message(path: &Path) -> String {
    format!(
        "The reference library at {} is empty. Add references with tw cite add or tw cite import.",
        path.display()
    )
}
