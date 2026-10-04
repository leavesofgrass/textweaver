//! `cargo xtask fuzz-seed [--target NAME]`: copies the fixtures that suit
//! each cargo-fuzz target into its corpus folder, `fuzz/corpus/NAME/`, so
//! the fuzzer starts from real files (`fuzz/README.md`). The nightly
//! workflow calls it for each target before fuzzing.
//!
//! It only creates folders and copies files; it never removes anything. A
//! file already in the corpus with the same name is overwritten with the
//! fixture's current contents. A target with no fixtures gets an empty
//! folder and starts from nothing.

use std::path::{Path, PathBuf};

use anyhow::Context;

/// Where a target's seeds come from: a folder under the repository root,
/// the file extensions to take (lower case, without the dot), and whether
/// to look in subfolders too.
struct Seeds {
    dir: &'static str,
    extensions: &'static [&'static str],
    recursive: bool,
}

const fn seeds(dir: &'static str, extensions: &'static [&'static str], recursive: bool) -> Seeds {
    Seeds {
        dir,
        extensions,
        recursive,
    }
}

/// Every fuzz target, with its seeds. A target listed with no seeds starts
/// from an empty corpus. Keep this in step with `fuzz/Cargo.toml` and the
/// matrix in `.github/workflows/nightly.yml`.
const TARGETS: &[(&str, &[Seeds])] = &[
    ("markdown", &[seeds("fixtures", &["md"], false)]),
    (
        "html",
        &[
            seeds("fixtures", &["html"], false),
            // star's empty-document case: an unclosed `<meta charset>`.
            seeds("fixtures/o", &["html"], false),
        ],
    ),
    ("epub", &[seeds("fixtures/a", &["epub"], false)]),
    ("docx", &[seeds("fixtures/a", &["docx"], false)]),
    ("pdf", &[seeds("fixtures/a", &["pdf"], false)]),
    // W6c5: links, comments, and form fields.
    ("pdf_annots", &[seeds("fixtures/c5", &["pdf"], false)]),
    ("settings", &[]),
    ("keymap", &[]),
    ("state", &[]),
    ("frame", &[]),
    // daisy, pptx, and sheet have no small fixtures yet.
    ("daisy", &[]),
    ("pptx", &[]),
    ("sheet", &[]),
    ("archive", &[seeds("fixtures/w3d", &["7z"], false)]),
    (
        "image",
        &[
            seeds("fixtures/w3d", &["png"], false),
            seeds("fixtures/m", &["png"], false),
        ],
    ),
    ("web", &[seeds("fixtures", &["html"], false)]),
    // W4c2's documents.
    ("rtf", &[seeds("fixtures/c2", &["rtf"], false)]),
    ("odt", &[seeds("fixtures/c2", &["odt"], false)]),
    (
        "docx_revisions",
        &[
            seeds("fixtures/c2", &["docx"], false),
            seeds("fixtures/a", &["docx"], false),
        ],
    ),
    (
        "latex_math",
        &[seeds("fixtures/cloud/latex_math", &["txt"], false)],
    ),
    (
        "asciimath",
        &[seeds("fixtures/cloud/asciimath", &["txt"], false)],
    ),
    ("bibtex", &[seeds("fixtures/p", &["bib"], false)]),
    ("ris", &[seeds("fixtures/p", &["ris"], false)]),
    (
        "csl_json",
        &[
            seeds("fixtures/p", &["json"], false),
            seeds("fixtures/p/recorded", &["json"], false),
        ],
    ),
    (
        "theme",
        &[seeds("crates/textweaver-theme/themes", &["toml"], false)],
    ),
    // lexicon patches a small valid file it builds itself.
    ("lexicon", &[]),
    (
        "vault_import",
        &[
            seeds("fixtures/l/vault", &["md"], true),
            seeds("fixtures/l/flavors", &["md"], false),
        ],
    ),
    ("rpc", &[seeds("fixtures/cloud/rpc", &["jsonl"], false)]),
    // W5c3's documents.
    ("latex", &[seeds("fixtures/c3", &["tex"], false)]),
    ("eml", &[seeds("fixtures/c3", &["eml", "mhtml"], false)]),
    // W6o's formats (ADR-0044).
    (
        "json",
        &[seeds("fixtures/o", &["json", "jsonl", "ipynb"], false)],
    ),
    ("svg", &[seeds("fixtures/o", &["svg", "mml"], false)]),
    ("obsidian", &[seeds("fixtures/o/vault", &["md"], true)]),
    // S1: a sync record (ADR-0049).
    // W7f: the nightly's crashing inputs, kept as seeds.
    (
        "sync_record",
        &[
            seeds("fixtures/s1", &["json"], false),
            seeds("fixtures/w7f/sync_record", &["json"], false),
        ],
    ),
    // W7s: the sync group files.
    (
        "sync_group",
        &[
            seeds("fixtures/w7s", &["json"], false),
            seeds("fixtures/w7f/sync_group", &["json"], false),
        ],
    ),
];

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let only = match args.as_slice() {
        [] => None,
        [flag, name] if flag == "--target" => Some(name.as_str()),
        _ => anyhow::bail!("usage: cargo xtask fuzz-seed [--target NAME]"),
    };
    let root = root();
    let chosen: Vec<&(&str, &[Seeds])> = TARGETS
        .iter()
        .filter(|(name, _)| only.is_none_or(|o| o == *name))
        .collect();
    if let Some(name) = only {
        anyhow::ensure!(
            !chosen.is_empty(),
            "no fuzz target named {name}; the targets are: {}",
            TARGETS
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    for (name, sources) in chosen {
        let corpus = root.join("fuzz").join("corpus").join(name);
        std::fs::create_dir_all(&corpus)
            .with_context(|| format!("creating {}", corpus.display()))?;
        let mut copied = 0usize;
        for source in *sources {
            for file in files(&root, source)? {
                let dest = corpus.join(seed_name(&root, &file));
                std::fs::copy(&file, &dest)
                    .with_context(|| format!("copying {} to {}", file.display(), dest.display()))?;
                copied += 1;
            }
        }
        println!("{name}: {copied} seed files in fuzz/corpus/{name}");
    }
    Ok(())
}

/// The fixture files `seeds` names, sorted by path. A missing folder has
/// none.
fn files(root: &Path, seeds: &Seeds) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut dirs = vec![root.join(seeds.dir)];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let entry = entry.with_context(|| format!("listing {}", dir.display()))?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                if seeds.recursive {
                    dirs.push(path);
                }
            } else if kind.is_file() && wanted(&path, seeds.extensions) {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn wanted(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// The corpus file name for a fixture: its path under the repository root
/// with every separator and space turned into `_`, so two fixtures with
/// the same file name in different folders stay apart.
fn seed_name(root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("_")
        .replace(' ', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every target in `fuzz/Cargo.toml` has an entry here, and the other
    /// way round.
    #[test]
    fn targets_match_the_fuzz_manifest() {
        let manifest = std::fs::read_to_string(root().join("fuzz").join("Cargo.toml"))
            .expect("fuzz/Cargo.toml exists");
        let mut bins: Vec<&str> = manifest
            .lines()
            .filter_map(|l| l.strip_prefix("name = \""))
            .filter_map(|l| l.strip_suffix('"'))
            .filter(|n| *n != "textweaver-fuzz")
            .collect();
        let mut ours: Vec<&str> = TARGETS.iter().map(|(n, _)| *n).collect();
        bins.sort_unstable();
        ours.sort_unstable();
        assert_eq!(bins, ours);
    }

    /// The nightly workflow fuzzes every target: its matrix line names
    /// exactly the targets in `fuzz/Cargo.toml`, so a new target cannot be
    /// left out of the nightly run.
    #[test]
    fn the_nightly_workflow_fuzzes_every_target() {
        let workflow =
            std::fs::read_to_string(root().join(".github").join("workflows").join("nightly.yml"))
                .expect(".github/workflows/nightly.yml exists");
        let line = workflow
            .lines()
            .map(str::trim)
            .find_map(|l| l.strip_prefix("target: ["))
            .and_then(|l| l.strip_suffix(']'))
            .expect("nightly.yml has a `target: [...]` matrix line");
        let mut matrix: Vec<&str> = line.split(',').map(str::trim).collect();
        let mut ours: Vec<&str> = TARGETS.iter().map(|(n, _)| *n).collect();
        matrix.sort_unstable();
        ours.sort_unstable();
        let missing: Vec<&&str> = ours.iter().filter(|t| !matrix.contains(t)).collect();
        let extra: Vec<&&str> = matrix.iter().filter(|t| !ours.contains(t)).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "nightly.yml's fuzz matrix: missing {missing:?}, not a target {extra:?}"
        );
    }

    /// Every target with seeds finds at least one file, so a moved fixture
    /// folder is noticed.
    #[test]
    fn every_seed_folder_has_files() {
        let root = root();
        for (name, sources) in TARGETS {
            for s in *sources {
                let found = files(&root, s).expect("fixtures list");
                assert!(!found.is_empty(), "{name}: no seeds in {}", s.dir);
            }
        }
    }

    #[test]
    fn seed_names_keep_folders_apart() {
        let root = Path::new("/repo");
        assert_eq!(
            seed_name(root, Path::new("/repo/fixtures/l/vault/sub/Two Words.md")),
            "fixtures_l_vault_sub_Two_Words.md"
        );
    }
}
