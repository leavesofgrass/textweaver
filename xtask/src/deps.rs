//! `cargo xtask deps [--check]`: the dependency direction between the
//! workspace's crates (ADR-0001).
//!
//! Without `--check` it prints each crate's internal dependencies. With
//! `--check` it fails when a crate has a forbidden edge:
//!
//! - `textweaver-core` depends on no other workspace crate (any kind of
//!   dependency, tests included).
//! - `textweaver-speech` never depends on `textweaver-text` or
//!   `textweaver-formats`, directly or through another workspace crate.
//! - `textweaver-store` depends only on `textweaver-core` among the
//!   workspace crates, and otherwise only on serde-level crates.
//!
//! Dev-dependencies are ignored except for core, since tests do not change
//! what a crate links.
//!
//! The rules read `cargo metadata --no-deps`, so the check needs no network
//! and no build.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use anyhow::{Context, bail};
use serde_json::Value;

/// One workspace crate's dependencies.
#[derive(Debug, Default, Clone)]
struct Crate {
    /// Workspace crates it depends on: normal and build dependencies.
    internal: BTreeSet<String>,
    /// Workspace crates it depends on for tests only.
    internal_dev: BTreeSet<String>,
    /// Other crates it depends on: normal and build dependencies.
    external: BTreeSet<String>,
}

/// The workspace graph: crate name to its dependencies.
type Graph = BTreeMap<String, Crate>;

const CORE: &str = "textweaver-core";
const SPEECH: &str = "textweaver-speech";
const STORE: &str = "textweaver-store";

/// Crates speech must never reach.
const SPEECH_FORBIDDEN: [&str; 2] = ["textweaver-text", "textweaver-formats"];

/// Workspace crates store may use.
const STORE_INTERNAL: [&str; 1] = [CORE];

/// Workspace crates store may use for now, with the reason.
///
/// TODO(roadmap, Phase 2, Architecture, "Dependency direction"): the
/// settings types move from `textweaver-aids` into `textweaver-store`; then
/// remove this allowance.
const STORE_INTERNAL_FOR_NOW: [(&str, &str); 1] = [(
    "textweaver-aids",
    "the settings types move into store (docs/roadmap.md, Phase 2, Dependency direction)",
)];

/// Serde-level crates store may use: serialization, formats, paths, errors,
/// and logging.
const STORE_EXTERNAL: [&str; 6] = [
    "serde",
    "serde_json",
    "toml",
    "directories",
    "thiserror",
    "log",
];

/// `cargo xtask deps`.
pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let check = match args.as_slice() {
        [] => false,
        [a] if a == "--check" => true,
        _ => bail!("usage: cargo xtask deps [--check]"),
    };
    let graph = graph()?;
    if !check {
        for (name, c) in &graph {
            let deps: Vec<&str> = c.internal.iter().map(String::as_str).collect();
            println!("{name}: {}", deps.join(", "));
        }
        return Ok(());
    }
    let (errors, notes) = violations(&graph);
    for n in &notes {
        println!("allowed for now: {n}");
    }
    if errors.is_empty() {
        println!("dependency direction: ok ({} crates)", graph.len());
        Ok(())
    } else {
        for e in &errors {
            eprintln!("forbidden: {e}");
        }
        bail!("{} forbidden dependency edges", errors.len())
    }
}

/// Reads the workspace graph from `cargo metadata`.
fn graph() -> anyhow::Result<Graph> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .current_dir(crate::eci::root())
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("running cargo metadata")?;
    if !out.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let meta: Value = serde_json::from_slice(&out.stdout).context("reading cargo metadata")?;
    graph_from_metadata(&meta)
}

/// Builds the graph from `cargo metadata --no-deps` output.
fn graph_from_metadata(meta: &Value) -> anyhow::Result<Graph> {
    let packages = meta["packages"]
        .as_array()
        .context("cargo metadata has no packages")?;
    let members: BTreeSet<String> = packages
        .iter()
        .filter_map(|p| p["name"].as_str().map(str::to_owned))
        .collect();
    let mut graph = Graph::new();
    for p in packages {
        let name = p["name"].as_str().context("a package has no name")?;
        let mut c = Crate::default();
        for d in p["dependencies"].as_array().into_iter().flatten() {
            let Some(dep) = d["name"].as_str() else {
                continue;
            };
            let dev = d["kind"].as_str() == Some("dev");
            match (members.contains(dep), dev) {
                (true, false) => c.internal.insert(dep.to_owned()),
                (true, true) => c.internal_dev.insert(dep.to_owned()),
                (false, false) => c.external.insert(dep.to_owned()),
                (false, true) => false,
            };
        }
        graph.insert(name.to_owned(), c);
    }
    Ok(graph)
}

/// The workspace crates `name` reaches through normal and build
/// dependencies, with the path to each (for the message).
fn reachable(graph: &Graph, name: &str) -> BTreeMap<String, Vec<String>> {
    let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut stack = vec![(name.to_owned(), vec![name.to_owned()])];
    while let Some((at, path)) = stack.pop() {
        let Some(c) = graph.get(&at) else {
            continue;
        };
        for dep in &c.internal {
            if !seen.contains_key(dep) {
                let mut p = path.clone();
                p.push(dep.clone());
                seen.insert(dep.clone(), p.clone());
                stack.push((dep.clone(), p));
            }
        }
    }
    seen
}

/// The forbidden edges (errors) and the edges allowed for now (notes).
fn violations(graph: &Graph) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut notes = Vec::new();

    for must in [CORE, SPEECH, STORE] {
        if !graph.contains_key(must) {
            errors.push(format!(
                "{must} is not in the workspace; update xtask/src/deps.rs"
            ));
        }
    }

    if let Some(core) = graph.get(CORE) {
        for dep in core.internal.iter().chain(&core.internal_dev) {
            errors.push(format!(
                "{CORE} -> {dep} (core depends on no workspace crate)"
            ));
        }
    }

    let from_speech = reachable(graph, SPEECH);
    for bad in SPEECH_FORBIDDEN {
        if let Some(path) = from_speech.get(bad) {
            errors.push(format!(
                "{SPEECH} reaches {bad} ({}); speech never depends on text or formats",
                path.join(" -> ")
            ));
        }
    }

    if let Some(store) = graph.get(STORE) {
        for dep in &store.internal {
            if STORE_INTERNAL.contains(&dep.as_str()) {
                continue;
            }
            match STORE_INTERNAL_FOR_NOW.iter().find(|(n, _)| n == dep) {
                Some((_, why)) => notes.push(format!("{STORE} -> {dep}: {why}")),
                None => errors.push(format!(
                    "{STORE} -> {dep} (store depends only on {CORE} among workspace crates)"
                )),
            }
        }
        for dep in &store.external {
            if !STORE_EXTERNAL.contains(&dep.as_str()) {
                errors.push(format!(
                    "{STORE} -> {dep} (store uses only serde-level crates: {})",
                    STORE_EXTERNAL.join(", ")
                ));
            }
        }
    }
    (errors, notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(internal: &[&str], external: &[&str]) -> Crate {
        Crate {
            internal: internal.iter().map(|s| (*s).to_owned()).collect(),
            internal_dev: BTreeSet::new(),
            external: external.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn good() -> Graph {
        let mut g = Graph::new();
        g.insert(CORE.into(), krate(&[], &["serde"]));
        g.insert(SPEECH.into(), krate(&[CORE, "textweaver-math"], &["log"]));
        g.insert("textweaver-math".into(), krate(&[CORE], &[]));
        g.insert("textweaver-text".into(), krate(&[CORE], &[]));
        g.insert(STORE.into(), krate(&[CORE], &["serde", "toml"]));
        g
    }

    #[test]
    fn a_clean_graph_passes() {
        let (errors, notes) = violations(&good());
        assert!(errors.is_empty(), "{errors:?}");
        assert!(notes.is_empty());
    }

    #[test]
    fn forbidden_edges_fail() {
        let mut g = good();
        g.get_mut(CORE)
            .unwrap()
            .internal_dev
            .insert("textweaver-text".into());
        // Through another crate: speech -> math -> text.
        g.get_mut("textweaver-math")
            .unwrap()
            .internal
            .insert("textweaver-text".into());
        g.get_mut(STORE).unwrap().internal.insert(SPEECH.into());
        g.get_mut(STORE).unwrap().external.insert("regex".into());
        let (errors, _) = violations(&g);
        assert_eq!(errors.len(), 4, "{errors:#?}");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("textweaver-speech -> textweaver-math -> textweaver-text"))
        );
    }

    #[test]
    fn store_on_aids_is_allowed_for_now() {
        let mut g = good();
        g.get_mut(STORE)
            .unwrap()
            .internal
            .insert("textweaver-aids".into());
        let (errors, notes) = violations(&g);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn the_workspace_passes() {
        let (errors, _) = violations(&graph().unwrap());
        assert!(errors.is_empty(), "{errors:#?}");
    }
}
