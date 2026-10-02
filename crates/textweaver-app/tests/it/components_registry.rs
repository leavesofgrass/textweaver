//! The components registry is enforced, as Star's optional-dependency
//! registry was (W8a-d): two halves.
//!
//! - **Completeness:** every download goes through the shared downloader
//!   with a registered component. No crate but the shared one opens HTTP
//!   for files (the citation lookups and opening a web page are requests,
//!   not components), the downloader is called only from the known sites,
//!   and each site's components are in the registry with the same pins.
//! - **Consistency:** what is reported installed really checks out, and
//!   with an empty data folder nothing is reported installed (the suite
//!   passes with no component installed).

use std::path::{Path, PathBuf};

use textweaver_app::components::{FileState, Registry, Status, component_dir};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file under each crate's `src`.
fn sources() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let crates = workspace().join("crates");
    let mut stack: Vec<PathBuf> = std::fs::read_dir(&crates)
        .unwrap()
        .map(|e| e.unwrap().path().join("src"))
        .filter(|p| p.is_dir())
        .collect();
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let rel = p
                    .strip_prefix(&crates)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    out
}

/// The only files that may use an HTTP client, and why.
const HTTP_ALLOWED: &[(&str, &str)] = &[
    (
        "textweaver-components/src/fetch.rs",
        "the shared downloader's fetcher",
    ),
    (
        "textweaver-cite/src/lookup.rs",
        "citation lookups by DOI or ISBN: metadata, not a component",
    ),
    (
        "textweaver-formats/src/web.rs",
        "opening a web page the reader asked for: a document, not a component",
    ),
];

/// The only files that call the shared downloader or install from a
/// file, and where their components come from.
const DOWNLOAD_SITES: &[(&str, &str)] = &[
    (
        "textweaver-components/src/download.rs",
        "the downloader itself",
    ),
    (
        "textweaver-components/src/install.rs",
        "install from a file itself",
    ),
    (
        "textweaver-ocr/src/models.rs",
        "an OCR model set: ModelSet::component, each in the registry",
    ),
    (
        "textweaver-fonts/src/downloaded.rs",
        "a downloadable font: DownloadableFont::component, each in the registry",
    ),
    (
        "textweaver-piper/src/download.rs",
        "a Piper voice: the registry's piper- family, from the catalogue",
    ),
    (
        "textweaver-app/src/components.rs",
        "the registry's own components",
    ),
    (
        "textweaver-cli/src/cmd/components.rs",
        "the registry's own components (tw components and tw dictate download)",
    ),
];

#[test]
fn every_download_goes_through_a_registered_component() {
    let calls = [
        "textweaver_components::download(",
        "download_component(",
        "textweaver_components::install_from(",
        "install_component(",
        "pub fn download(",
        "pub fn install_from(",
    ];
    let mut bad = Vec::new();
    for (file, text) in sources() {
        if text.contains("ureq::") && !HTTP_ALLOWED.iter().any(|(f, _)| *f == file) {
            bad.push(format!(
                "{file} uses ureq; downloads go through textweaver-components"
            ));
        }
        let calls_downloader = calls.iter().any(|c| text.contains(c))
            && (text.contains("textweaver_components")
                || file.starts_with("textweaver-components/"));
        if calls_downloader && !DOWNLOAD_SITES.iter().any(|(f, _)| *f == file) {
            bad.push(format!(
                "{file} calls the shared downloader; add its components to the registry and to DOWNLOAD_SITES"
            ));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
    // Each site's components are registered, with the same pins.
    let registry = Registry::builtin();
    for set in textweaver_app::components::ocr_sets() {
        let c = set.component();
        assert_eq!(registry.get(&c.id), Some(&c), "{}", c.id);
    }
    for font in textweaver_app::fonts::downloaded::DOWNLOADABLE {
        let c = font.component();
        assert_eq!(registry.get(&c.id), Some(&c), "{}", c.id);
    }
    // Ids are unique.
    let mut ids: Vec<&str> = registry
        .components()
        .iter()
        .map(|c| c.id.as_ref())
        .collect();
    ids.sort_unstable();
    let n = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), n);
}

#[test]
fn with_an_empty_data_folder_nothing_is_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::builtin();
    assert_eq!(
        registry.installed_count(tmp.path()),
        (0, registry.components().len())
    );
    for c in registry.components() {
        let dir = component_dir(c, tmp.path());
        assert!(
            c.verify_in(&dir)
                .iter()
                .all(|(_, s)| *s == FileState::Missing),
            "{}",
            c.id
        );
    }
}

/// Reported installed means it checks out: a file with the right size but
/// the wrong bytes is installed by size, and Verify says it does not
/// match, so the manager offers to download it again.
#[test]
fn installed_by_size_is_checked_by_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::builtin();
    let lexend = registry.get("lexend").unwrap();
    let dir = component_dir(lexend, tmp.path());
    std::fs::create_dir_all(&dir).unwrap();
    for f in lexend.files.iter() {
        let size = usize::try_from(f.size).unwrap();
        std::fs::write(dir.join(f.name.as_ref()), vec![0u8; size]).unwrap();
    }
    assert_eq!(lexend.status_in(&dir), Status::Installed);
    assert!(
        lexend
            .verify_in(&dir)
            .iter()
            .all(|(_, s)| *s == FileState::WrongHash)
    );
    // And the font code, which checks the hash, does not load it.
    let fonts = tmp.path().join("fonts");
    assert!(
        textweaver_app::fonts::downloaded::LEXEND
            .load_from(&fonts)
            .is_none()
    );
}
