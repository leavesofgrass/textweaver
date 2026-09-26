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
//!   workspace crates, and otherwise only on serde-level crates. There are
//!   no exceptions: the reading-aid settings types are store's own.
//! - `textweaver-tui` (the reader, `textweaver`) built without its default
//!   features never reaches the conversion and citation stack: not the
//!   convert, render, writers, or cite crates, and not their big
//!   dependencies (hayagriva, comrak, minijinja, krilla, ammonia, ureq), so
//!   a lean reader stays small and starts fast (docs/roadmap.md, "Binary
//!   size"). In-reader export, preview, and citations are the `publish`
//!   feature (on by default and in releases); what the reader reaches only
//!   through a feature is reported, not refused. `tw` has them all.
//!
//! Dev-dependencies are ignored except for core, since tests do not change
//! what a crate links. Optional dependencies count only when a feature
//! turns them on: the reader rule resolves features the way Cargo does
//! (`dep:x`, `x/feature`, `x?/feature`, default features, and the features
//! a dependency asks for).
//!
//! The rules read `cargo metadata --no-deps`, so the check needs no network
//! and no build.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use anyhow::{Context, bail};
use serde_json::Value;

/// One dependency of a workspace crate, as `cargo metadata` lists it.
#[derive(Debug, Clone)]
struct Dep {
    /// The package name.
    name: String,
    /// The name features use for it (its `rename`, else its name).
    key: String,
    /// A workspace crate.
    internal: bool,
    /// A dev-dependency.
    dev: bool,
    /// Turned on only by a feature.
    optional: bool,
    /// Its default features are on.
    default_features: bool,
    /// Features it asks for.
    features: Vec<String>,
}

/// One workspace crate's dependencies and features.
#[derive(Debug, Default, Clone)]
struct Crate {
    deps: Vec<Dep>,
    /// `[features]`: name to what it turns on.
    features: BTreeMap<String, Vec<String>>,
}

impl Crate {
    /// Workspace crates it depends on: normal and build dependencies,
    /// optional ones included.
    fn internal(&self) -> impl Iterator<Item = &str> {
        self.deps
            .iter()
            .filter(|d| d.internal && !d.dev)
            .map(|d| d.name.as_str())
    }

    /// Workspace crates it depends on for tests only.
    fn internal_dev(&self) -> impl Iterator<Item = &str> {
        self.deps
            .iter()
            .filter(|d| d.internal && d.dev)
            .map(|d| d.name.as_str())
    }

    /// Other crates it depends on: normal and build dependencies, optional
    /// ones included.
    fn external(&self) -> impl Iterator<Item = &str> {
        self.deps
            .iter()
            .filter(|d| !d.internal && !d.dev)
            .map(|d| d.name.as_str())
    }
}

/// The workspace graph: crate name to its dependencies.
type Graph = BTreeMap<String, Crate>;

const CORE: &str = "textweaver-core";
const SPEECH: &str = "textweaver-speech";
const STORE: &str = "textweaver-store";

/// Crates speech must never reach.
const SPEECH_FORBIDDEN: [&str; 2] = ["textweaver-text", "textweaver-formats"];

/// The reader.
const READER: &str = "textweaver-tui";

/// Workspace crates the reader reaches only through the `publish` feature
/// (in-reader export, preview, and citations).
const READER_FORBIDDEN: [&str; 4] = [
    "textweaver-convert",
    "textweaver-render",
    "textweaver-writers",
    "textweaver-cite",
];

/// Outside crates of the conversion and citation stack, which no crate the
/// lean reader reaches may depend on.
const READER_FORBIDDEN_EXTERNAL: [&str; 7] = [
    "hayagriva",
    "biblatex",
    "comrak",
    "minijinja",
    "krilla",
    "ammonia",
    "ureq",
];

/// Workspace crates store may use. No exceptions: the reading-aid
/// settings types are store's own since Wave 3 (Agent W3c), and
/// `textweaver-aids` converts them.
const STORE_INTERNAL: [&str; 1] = [CORE];

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
            let deps: Vec<&str> = c.internal().collect();
            println!("{name}: {}", deps.join(", "));
        }
        return Ok(());
    }
    let (errors, notes) = violations(&graph);
    for n in &notes {
        println!("allowed: {n}");
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
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str().map(str::to_owned))
            .collect()
    };
    let mut graph = Graph::new();
    for p in packages {
        let name = p["name"].as_str().context("a package has no name")?;
        let mut c = Crate::default();
        for d in p["dependencies"].as_array().into_iter().flatten() {
            let Some(dep) = d["name"].as_str() else {
                continue;
            };
            c.deps.push(Dep {
                name: dep.to_owned(),
                key: d["rename"].as_str().unwrap_or(dep).to_owned(),
                internal: members.contains(dep),
                dev: d["kind"].as_str() == Some("dev"),
                optional: d["optional"].as_bool().unwrap_or(false),
                default_features: d["uses_default_features"].as_bool().unwrap_or(true),
                features: strings(&d["features"]),
            });
        }
        if let Some(f) = p["features"].as_object() {
            c.features = f.iter().map(|(k, v)| (k.clone(), strings(v))).collect();
        }
        graph.insert(name.to_owned(), c);
    }
    Ok(graph)
}

/// The workspace crates `name` reaches through normal and build
/// dependencies, optional ones included, with the path to each (for the
/// message).
fn reachable(graph: &Graph, name: &str) -> BTreeMap<String, Vec<String>> {
    let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut stack = vec![(name.to_owned(), vec![name.to_owned()])];
    while let Some((at, path)) = stack.pop() {
        let Some(c) = graph.get(&at) else {
            continue;
        };
        for dep in c.internal() {
            if !seen.contains_key(dep) {
                let mut p = path.clone();
                p.push(dep.to_owned());
                seen.insert(dep.to_owned(), p.clone());
                stack.push((dep.to_owned(), p));
            }
        }
    }
    seen
}

/// What building `root` with `features` links, feature resolution
/// included: each workspace crate reached, with the path to it, and each
/// outside crate reached, with the path through the crate that depends on
/// it.
#[derive(Debug, Default)]
struct Build {
    internal: BTreeMap<String, Vec<String>>,
    external: BTreeMap<String, Vec<String>>,
}

/// Resolves features from `root` with `features` on, the way Cargo does.
fn build(graph: &Graph, root: &str, features: &[&str]) -> Build {
    // Features on per workspace crate, and how each crate was reached.
    let mut on: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    on.insert(
        root.to_owned(),
        features.iter().map(|f| (*f).to_owned()).collect(),
    );
    paths.insert(root.to_owned(), vec![root.to_owned()]);
    let mut out = Build::default();
    let mut visited: BTreeSet<String> = BTreeSet::from([root.to_owned()]);
    let mut queue = vec![root.to_owned()];
    while let Some(name) = queue.pop() {
        let Some(c) = graph.get(&name) else {
            continue;
        };
        // The crate's own features, closed over; the optional dependencies
        // they turn on; and features they ask of dependencies.
        let mut feats: BTreeSet<String> = on.get(&name).cloned().unwrap_or_default();
        let mut optional_on: BTreeSet<String> = BTreeSet::new();
        // (dependency key, feature): `x/f` and `x?/f` both ask `x` for `f`; only
        // `x/f` also turns `x` on.
        let mut asks: Vec<(String, String)> = Vec::new();
        let mut todo: Vec<String> = feats.iter().cloned().collect();
        while let Some(f) = todo.pop() {
            let Some(items) = c.features.get(&f) else {
                // An optional dependency's implicit feature.
                if c.deps.iter().any(|d| d.optional && d.key == f) {
                    optional_on.insert(f);
                }
                continue;
            };
            for item in items {
                if let Some(dep) = item.strip_prefix("dep:") {
                    optional_on.insert(dep.to_owned());
                } else if let Some((dep, feat)) = item.split_once('/') {
                    match dep.strip_suffix('?') {
                        Some(weak) => asks.push((weak.to_owned(), feat.to_owned())),
                        None => {
                            optional_on.insert(dep.to_owned());
                            asks.push((dep.to_owned(), feat.to_owned()));
                        }
                    }
                } else if feats.insert(item.clone()) {
                    todo.push(item.clone());
                }
            }
        }
        let active: Vec<&Dep> = c
            .deps
            .iter()
            .filter(|d| !d.dev && (!d.optional || optional_on.contains(&d.key)))
            .collect();
        let path = paths.get(&name).cloned().unwrap_or_default();
        for d in active {
            let mut p = path.clone();
            p.push(d.name.clone());
            if !d.internal {
                out.external.entry(d.name.clone()).or_insert(p);
                continue;
            }
            let mut want: BTreeSet<String> = d.features.iter().cloned().collect();
            if d.default_features {
                want.insert("default".into());
            }
            for (key, feat) in &asks {
                // Only dependencies that are on are here, so a weak request
                // counts exactly when it should.
                if *key == d.key {
                    want.insert(feat.clone());
                }
            }
            paths.entry(d.name.clone()).or_insert_with(|| p.clone());
            out.internal.entry(d.name.clone()).or_insert(p);
            let have = on.entry(d.name.clone()).or_default();
            let before = have.len();
            have.extend(want);
            let changed = have.len() != before;
            if visited.insert(d.name.clone()) || changed {
                queue.push(d.name.clone());
            }
        }
    }
    out
}

/// The forbidden edges (errors) and the edges allowed through a feature
/// (notes).
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
        for dep in core.internal().chain(core.internal_dev()) {
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

    if graph.contains_key(READER) {
        let lean = build(graph, READER, &[]);
        let full = build(graph, READER, &["default"]);
        for bad in READER_FORBIDDEN {
            if let Some(path) = lean.internal.get(bad) {
                errors.push(format!(
                    "{READER} reaches {bad} without its default features ({}); in-reader export and citations belong behind the app's `publish` feature",
                    path.join(" -> ")
                ));
            } else if let Some(path) = full.internal.get(bad) {
                notes.push(format!(
                    "{READER} reaches {bad} through the `publish` feature ({})",
                    path.join(" -> ")
                ));
            }
        }
        for bad in READER_FORBIDDEN_EXTERNAL {
            if let Some(path) = lean.external.get(bad) {
                errors.push(format!(
                    "{READER} reaches {bad} without its default features ({})",
                    path.join(" -> ")
                ));
            } else if let Some(path) = full.external.get(bad) {
                notes.push(format!(
                    "{READER} reaches {bad} through the `publish` feature ({})",
                    path.join(" -> ")
                ));
            }
        }
    }

    if let Some(store) = graph.get(STORE) {
        for dep in store.internal() {
            if !STORE_INTERNAL.contains(&dep) {
                errors.push(format!(
                    "{STORE} -> {dep} (store depends only on {CORE} among workspace crates)"
                ));
            }
        }
        for dep in store.external() {
            if !STORE_EXTERNAL.contains(&dep) {
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

    fn dep(name: &str, internal: bool) -> Dep {
        Dep {
            name: name.to_owned(),
            key: name.to_owned(),
            internal,
            dev: false,
            optional: false,
            default_features: true,
            features: Vec::new(),
        }
    }

    fn krate(internal: &[&str], external: &[&str]) -> Crate {
        Crate {
            deps: internal
                .iter()
                .map(|n| dep(n, true))
                .chain(external.iter().map(|n| dep(n, false)))
                .collect(),
            features: BTreeMap::new(),
        }
    }

    fn add(g: &mut Graph, from: &str, d: Dep) {
        g.get_mut(from).unwrap().deps.push(d);
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
        add(
            &mut g,
            CORE,
            Dep {
                dev: true,
                ..dep("textweaver-text", true)
            },
        );
        // Through another crate: speech -> math -> text.
        add(&mut g, "textweaver-math", dep("textweaver-text", true));
        add(&mut g, STORE, dep(SPEECH, true));
        add(&mut g, STORE, dep("regex", false));
        let (errors, _) = violations(&g);
        assert_eq!(errors.len(), 4, "{errors:#?}");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("textweaver-speech -> textweaver-math -> textweaver-text"))
        );
    }

    /// A reader over an app whose `publish` feature (on by default) turns
    /// on the citation crate, as in the workspace.
    fn reader_graph(reader_asks_default: bool) -> Graph {
        let mut g = good();
        g.insert("textweaver-cite".into(), krate(&[CORE], &["hayagriva"]));
        let mut app = krate(&[CORE], &[]);
        app.deps.push(Dep {
            optional: true,
            ..dep("textweaver-cite", true)
        });
        app.features = BTreeMap::from([
            ("default".into(), vec!["publish".into()]),
            ("publish".into(), vec!["dep:textweaver-cite".into()]),
        ]);
        g.insert("textweaver-app".into(), app);
        let mut reader = krate(&[], &["ratatui"]);
        reader.deps.push(Dep {
            default_features: reader_asks_default,
            ..dep("textweaver-app", true)
        });
        reader.features = BTreeMap::from([
            ("default".into(), vec!["publish".into()]),
            ("publish".into(), vec!["textweaver-app/publish".into()]),
        ]);
        g.insert(READER.into(), reader);
        g
    }

    #[test]
    fn the_reader_reaching_conversion_or_citations_through_the_feature_is_reported() {
        let (errors, notes) = violations(&reader_graph(false));
        assert!(errors.is_empty(), "{errors:#?}");
        assert!(notes.iter().any(|e| e.contains(
            "textweaver-tui reaches textweaver-cite through the `publish` feature (textweaver-tui -> textweaver-app -> textweaver-cite)"
        )), "{notes:#?}");
        assert!(notes.iter().any(|e| e.contains("hayagriva")));
    }

    #[test]
    fn a_lean_reader_that_still_reaches_citations_is_refused() {
        // The reader takes the app's default features, so it cannot turn
        // `publish` off.
        let (errors, _) = violations(&reader_graph(true));
        assert!(
            errors.iter().any(|e| e
                .contains("textweaver-tui reaches textweaver-cite without its default features")),
            "{errors:#?}"
        );
        assert!(errors.iter().any(|e| e.contains("hayagriva")));
        // So is a plain dependency on a conversion crate.
        let mut g = reader_graph(false);
        add(&mut g, "textweaver-app", dep("textweaver-render", true));
        g.insert("textweaver-render".into(), krate(&[CORE], &[]));
        let (errors, _) = violations(&g);
        assert_eq!(errors.len(), 1, "{errors:#?}");
    }

    #[test]
    fn features_resolve_like_cargo() {
        let mut g = reader_graph(false);
        // A weak request does not turn a dependency on; a strong one does.
        g.get_mut("textweaver-app")
            .unwrap()
            .features
            .insert("weak".into(), vec!["textweaver-cite?/x".into()]);
        let b = build(&g, "textweaver-app", &["weak"]);
        assert!(!b.internal.contains_key("textweaver-cite"));
        g.get_mut("textweaver-app")
            .unwrap()
            .features
            .insert("strong".into(), vec!["textweaver-cite/x".into()]);
        let b = build(&g, "textweaver-app", &["strong"]);
        assert!(b.internal.contains_key("textweaver-cite"));
        assert!(b.external.contains_key("hayagriva"));
        // An optional dependency's implicit feature.
        let b = build(&g, "textweaver-app", &["textweaver-cite"]);
        assert!(b.internal.contains_key("textweaver-cite"));
    }

    #[test]
    fn store_on_aids_is_refused() {
        let mut g = good();
        add(&mut g, STORE, dep("textweaver-aids", true));
        let (errors, notes) = violations(&g);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("textweaver-store -> textweaver-aids"));
        assert!(notes.is_empty());
    }

    #[test]
    fn the_workspace_passes() {
        let (errors, _) = violations(&graph().unwrap());
        assert!(errors.is_empty(), "{errors:#?}");
    }
}
