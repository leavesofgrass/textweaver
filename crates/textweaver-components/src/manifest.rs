//! A source's own list of components, `components.toml`: read from the
//! components source (`[components] source`, a repository or a folder)
//! and from a mirror, when one is set.
//!
//! The format is the registry's, in TOML. It is a proposal for the
//! owner's review, not a frozen format: fields this version does not know
//! are ignored, so a list written for a later version still loads.
//!
//! ```toml
//! format = 1
//!
//! [[component]]
//! id = "example-voice-pack"          # a plain name
//! title = "Example voice pack"
//! license = "CC0-1.0"                # the license note, said before installing
//! credit = "Who made it, and where it comes from"
//! features = ["voice"]               # what needs it: dictation, ocr, reading-font, voice
//! folder = "components/example-voice-pack"   # optional; this is the default
//! version = "1.2.0"                  # optional; the files are then under <source>/<id>-<version>/
//! platform = "any"                   # optional: windows, linux, macos, or any (the default)
//! action = "place"                   # optional: place (the default), unpack, or installer
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
//! plain names under the data folder. A component for another platform
//! is left out; one whose id is already known (a built-in one, or one
//! listed earlier) is refused: a list may add components, never change a
//! known component's pins. An entry with an action this version does not
//! know is refused alone, with the reason; the rest still load.

use std::borrow::Cow;

use serde::Deserialize;

use crate::component::Component;
use crate::error::ComponentError;
use crate::pin::{Check, FilePin, is_plain_name};

/// The format this version reads.
pub const FORMAT: u32 = 1;

/// The most components a manifest may list.
pub const MAX_COMPONENTS: usize = 200;

/// The largest manifest read, in bytes.
pub const MAX_BYTES: u64 = 1_000_000;

/// The manifest's file name in a source.
pub const FILE_NAME: &str = "components.toml";

/// Which computers a component is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Windows.
    Windows,
    /// Linux.
    Linux,
    /// macOS.
    Macos,
    /// Every platform (models, fonts, voices).
    Any,
}

impl Platform {
    /// The platform this program runs on (`Any` elsewhere, such as BSD).
    pub const fn current() -> Platform {
        if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Macos
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else {
            Platform::Any
        }
    }

    /// The manifest's word for it (empty is `any`), or `None` for a word
    /// this version does not know.
    pub fn from_word(word: &str) -> Option<Platform> {
        Some(match word.trim().to_ascii_lowercase().as_str() {
            "windows" => Platform::Windows,
            "linux" => Platform::Linux,
            "macos" => Platform::Macos,
            "any" | "" => Platform::Any,
            _ => return None,
        })
    }

    /// The manifest's word for it.
    pub const fn word(self) -> &'static str {
        match self {
            Platform::Windows => "windows",
            Platform::Linux => "linux",
            Platform::Macos => "macos",
            Platform::Any => "any",
        }
    }

    /// True when a component for `self` belongs on `here`.
    pub fn fits(self, here: Platform) -> bool {
        self == Platform::Any || self == here
    }
}

/// What installing a component does with its files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// The files are placed in the component's folder as they are (models,
    /// fonts, voices, a library).
    Place,
    /// The file is an archive, unpacked into the component's folder
    /// (ffmpeg's zip, a tarball).
    Unpack,
    /// The file is an installer: checked, then launched only after the
    /// reader hears its name, version, and license note and says yes.
    Installer,
}

impl Action {
    /// The manifest's word for it (empty is `place`), or `None` for a word
    /// this version does not know.
    pub fn from_word(word: &str) -> Option<Action> {
        Some(match word.trim().to_ascii_lowercase().as_str() {
            "place" | "" => Action::Place,
            "unpack" => Action::Unpack,
            "installer" => Action::Installer,
            _ => return None,
        })
    }

    /// The manifest's word for it.
    pub const fn word(self) -> &'static str {
        match self {
            Action::Place => "place",
            Action::Unpack => "unpack",
            Action::Installer => "installer",
        }
    }
}

/// What a source's list says about a component beyond its pins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    /// Its version, as the list says it ("0.3.0"); empty when not given.
    pub version: String,
    /// The computers it is for.
    pub platform: Platform,
    /// What installing it does.
    pub action: Action,
}

// No `deny_unknown_fields`: a list written for a later version, with
// fields this one does not know, still loads.
#[derive(Deserialize)]
struct Manifest {
    format: u32,
    #[serde(default)]
    component: Vec<Entry>,
}

#[derive(Deserialize)]
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
    #[serde(default)]
    version: String,
    #[serde(default)]
    platform: String,
    #[serde(default)]
    action: String,
    file: Vec<FileEntry>,
}

#[derive(Deserialize)]
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
    /// Components to add to the registry, each with its
    /// [`Component::listing`].
    pub components: Vec<Component>,
    /// Ids refused, each with the reason.
    pub refused: Vec<(String, String)>,
}

/// Reads a manifest's text for this computer ([`Platform::current`]).
/// `known` says whether an id is already in the registry (built-in
/// components); those entries are refused. A malformed file is an error;
/// a malformed entry is refused alone.
pub fn parse(text: &str, known: &dyn Fn(&str) -> bool) -> Result<Parsed, ComponentError> {
    parse_for(text, known, Platform::current())
}

/// [`parse`] for the computer `here`: entries for another platform are
/// left out.
pub fn parse_for(
    text: &str,
    known: &dyn Fn(&str) -> bool,
    here: Platform,
) -> Result<Parsed, ComponentError> {
    let m: Manifest = toml::from_str(text).map_err(|e| ComponentError::Manifest(e.to_string()))?;
    if m.format != FORMAT {
        return Err(ComponentError::Manifest(format!(
            "format {} is not one this version reads ({FORMAT}); a newer textweaver may read it",
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
        // A platform word this version does not know is a platform it
        // does not run on: left out, like any other platform's entry.
        let Some(platform) = Platform::from_word(&e.platform).filter(|p| p.fits(here)) else {
            log::debug!("component {id} is for {}, not this computer", e.platform);
            continue;
        };
        let Some(action) = Action::from_word(&e.action) else {
            out.refused.push((
                id,
                format!("the action {} is not one this version knows", e.action),
            ));
            continue;
        };
        let version = e.version.trim().to_owned();
        if !version.is_empty() && !is_plain_name(&version) {
            out.refused
                .push((id, format!("the version {version} is not a plain name")));
            continue;
        }
        if known(&id) || out.components.iter().any(|c| c.id == id) {
            out.refused.push((
                id,
                "already known; a list cannot change its pins".to_owned(),
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
            listing: Some(Listing {
                version,
                platform,
                action,
            }),
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
        // Fields this version does not know are ignored (forward compatible).
        assert_eq!(
            parse("format = 1\nextra = 3", &|_| false).unwrap(),
            Parsed::default()
        );
        assert_eq!(parse("format = 1", &|_| false).unwrap(), Parsed::default());
    }

    const SOURCE: &str = r#"
format = 1
generator = "a later field this version ignores"

[[component]]
id = "ffmpeg"
title = "FFmpeg for Windows"
license = "LGPL-2.1-or-later"
version = "9.0.2"
platform = "windows"
action = "unpack"
[[component.file]]
name = "ffmpeg-win64.zip"
size = 3
sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
checked_by = "ignored too"

[[component]]
id = "ffmpeg"
title = "FFmpeg for Linux"
license = "LGPL-2.1-or-later"
version = "9.0.2"
platform = "linux"
action = "unpack"
[[component.file]]
name = "ffmpeg-linux64.tar.xz"
size = 3
sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

[[component]]
id = "engine-setup"
title = "An engine"
license = "proprietary, one user"
version = "0.3.0"
platform = "windows"
action = "installer"
[[component.file]]
name = "setup.exe"
size = 3
sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

[[component]]
id = "later"
title = "From a later version"
license = "none"
action = "compile"
[[component.file]]
name = "x.bin"
size = 3
sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

[[component]]
id = "elsewhere"
title = "For a platform this version does not know"
license = "none"
platform = "plan9"
[[component.file]]
name = "x.bin"
size = 3
sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
"#;

    #[test]
    fn platform_and_action_are_read_and_other_platforms_left_out() {
        let p = parse_for(SOURCE, &|_| false, Platform::Windows).unwrap();
        let ids: Vec<&str> = p.components.iter().map(|c| c.id.as_ref()).collect();
        assert_eq!(ids, ["ffmpeg", "engine-setup"]);
        let ff = p.components[0].listing.as_ref().unwrap();
        assert_eq!(ff.version, "9.0.2");
        assert_eq!(ff.platform, Platform::Windows);
        assert_eq!(ff.action, Action::Unpack);
        assert_eq!(p.components[0].files[0].name, "ffmpeg-win64.zip");
        assert_eq!(p.components[0].release(), "ffmpeg-9.0.2");
        assert_eq!(
            p.components[1].listing.as_ref().unwrap().action,
            Action::Installer
        );
        assert_eq!(p.refused.len(), 1, "{:?}", p.refused);
        assert_eq!(p.refused[0].0, "later");
        assert!(p.refused[0].1.contains("compile"));

        let linux = parse_for(SOURCE, &|_| false, Platform::Linux).unwrap();
        let ids: Vec<&str> = linux.components.iter().map(|c| c.id.as_ref()).collect();
        assert_eq!(ids, ["ffmpeg"]);
        assert_eq!(linux.components[0].files[0].name, "ffmpeg-linux64.tar.xz");
    }

    #[test]
    fn words_round_trip() {
        for p in [
            Platform::Windows,
            Platform::Linux,
            Platform::Macos,
            Platform::Any,
        ] {
            assert_eq!(Platform::from_word(p.word()), Some(p));
        }
        for a in [Action::Place, Action::Unpack, Action::Installer] {
            assert_eq!(Action::from_word(a.word()), Some(a));
        }
        assert_eq!(Platform::from_word(""), Some(Platform::Any));
        assert_eq!(Action::from_word(" Unpack "), Some(Action::Unpack));
        assert!(Platform::Any.fits(Platform::current()));
        assert!(!Platform::Linux.fits(Platform::Windows));
    }

    #[test]
    fn a_version_must_be_a_plain_name() {
        let text = SOURCE.replace("\"0.3.0\"", "\"../0.3\"");
        let p = parse_for(&text, &|_| false, Platform::Windows).unwrap();
        assert!(p.refused.iter().any(|(id, _)| id == "engine-setup"));
    }
}
