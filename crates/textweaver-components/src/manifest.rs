//! A mirror's own list of components: `manifest/components.toml` under the
//! mirror's address, read only when a mirror is set.
//!
//! The format is the registry's, in TOML:
//!
//! ```toml
//! format = 1
//!
//! [[component]]
//! id = "example-voice-pack"          # a plain name; the files are under <mirror>/<id>/
//! title = "Example voice pack"
//! license = "CC0-1.0"
//! credit = "Who made it, and where it comes from"
//! features = ["voice"]               # what needs it: dictation, ocr, reading-font, voice
//! folder = "components/example-voice-pack"   # optional; this is the default
//!
//! [[component.file]]
//! name = "voice.onnx"
//! size = 1234
//! sha256 = "64 hex digits"
//! url = "https://example.org/voice.onnx"     # optional public address
//! ```
//!
//! Every name must be plain (letters, digits, `-`, `_`, `.`), every size
//! more than zero, every hash 64 hex digits, and the folder a path of
//! plain names under the data folder. A component whose id is already
//! known (a built-in one) is refused: a mirror may add components, never
//! change a built-in component's pins.

use std::borrow::Cow;

use serde::Deserialize;

use crate::component::Component;
use crate::error::ComponentError;
use crate::pin::{Check, FilePin};

/// The format this version reads.
pub const FORMAT: u32 = 1;

/// The most components a manifest may list.
pub const MAX_COMPONENTS: usize = 200;

/// The largest manifest read, in bytes.
pub const MAX_BYTES: u64 = 1_000_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: u32,
    #[serde(default)]
    component: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    title: String,
    license: String,
    #[serde(default)]
    credit: String,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    folder: Option<String>,
    file: Vec<FileEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileEntry {
    name: String,
    size: u64,
    sha256: String,
    #[serde(default)]
    url: String,
}

/// What a manifest gave: the components to add, and the ones refused with
/// the reason.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Components to add to the registry.
    pub components: Vec<Component>,
    /// Ids refused, each with the reason.
    pub refused: Vec<(String, String)>,
}

/// Reads a manifest's text. `known` says whether an id is already in the
/// registry (built-in components); those entries are refused. A
/// malformed file is an error; a malformed entry is refused alone.
pub fn parse(text: &str, known: &dyn Fn(&str) -> bool) -> Result<Parsed, ComponentError> {
    let m: Manifest = toml::from_str(text).map_err(|e| ComponentError::Manifest(e.to_string()))?;
    if m.format != FORMAT {
        return Err(ComponentError::Manifest(format!(
            "format {} is not one this version reads ({FORMAT})",
            m.format
        )));
    }
    if m.component.len() > MAX_COMPONENTS {
        return Err(ComponentError::Manifest(format!(
            "more than {MAX_COMPONENTS} components"
        )));
    }
    let mut out = Parsed::default();
    for e in m.component {
        let id = e.id.clone();
        if known(&id) || out.components.iter().any(|c| c.id == id) {
            out.refused.push((
                id,
                "already known; a mirror cannot change its pins".to_owned(),
            ));
            continue;
        }
        let folder = e.folder.unwrap_or_else(|| format!("components/{}", e.id));
        let c = Component {
            id: Cow::Owned(e.id),
            title: Cow::Owned(e.title),
            license: Cow::Owned(e.license),
            credit: Cow::Owned(e.credit),
            features: Cow::Owned(e.features.into_iter().map(Cow::Owned).collect()),
            folder: Cow::Owned(folder),
            files: Cow::Owned(
                e.file
                    .into_iter()
                    .map(|f| FilePin {
                        name: Cow::Owned(f.name),
                        url: Cow::Owned(f.url),
                        size: f.size,
                        check: Check::Sha256(Cow::Owned(f.sha256.to_ascii_lowercase())),
                    })
                    .collect(),
            ),
            notice: None,
        };
        let public_ok = c
            .files
            .iter()
            .all(|f| f.url.is_empty() || f.url.starts_with("https://"));
        match c.check_names() {
            Ok(()) if public_ok => out.components.push(c),
            Ok(()) => out
                .refused
                .push((id, "a public address that is not https".to_owned())),
            Err(err) => out.refused.push((id, err.to_string())),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
format = 1

[[component]]
id = "example-voice-pack"
title = "Example voice pack"
license = "CC0-1.0"
features = ["voice"]

[[component.file]]
name = "voice.onnx"
size = 3
sha256 = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"

[[component]]
id = "lexend"
title = "Not Lexend"
license = "none"
[[component.file]]
name = "x.ttf"
size = 1
sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

[[component]]
id = "sneaky"
title = "Sneaky"
license = "none"
folder = "../../outside"
[[component.file]]
name = "x.bin"
size = 1
sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
"#;

    #[test]
    fn a_manifest_adds_but_never_overrides() {
        let p = parse(GOOD, &|id| id == "lexend").unwrap();
        assert_eq!(p.components.len(), 1);
        let c = &p.components[0];
        assert_eq!(c.id, "example-voice-pack");
        assert_eq!(c.folder, "components/example-voice-pack");
        assert_eq!(
            c.files[0].check.expected(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let refused: Vec<&str> = p.refused.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(refused, ["lexend", "sneaky"]);
        assert!(
            p.refused[1].1.contains("not a plain name"),
            "{:?}",
            p.refused
        );
    }

    #[test]
    fn a_wrong_format_or_garbage_is_an_error() {
        assert!(parse("format = 2", &|_| false).is_err());
        assert!(parse("not toml [", &|_| false).is_err());
        assert!(parse("format = 1\nextra = 3", &|_| false).is_err());
        assert_eq!(parse("format = 1", &|_| false).unwrap(), Parsed::default());
    }
}
