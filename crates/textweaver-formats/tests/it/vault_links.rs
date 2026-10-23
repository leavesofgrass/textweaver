//! Obsidian vaults in the reader (beta 4, obsidian-fixes 1 and 2): embeds
//! resolve anywhere in the vault (the folder holding `.obsidian`) and never
//! outside it, by name, path, title, or alias; a note outside any vault
//! keeps its own folder; properties are read by the vault crate's parser.
//! Small vaults of placeholder notes are built in temporary folders.

use std::path::{Path, PathBuf};

use textweaver_formats::properties::front_matter_properties;
use textweaver_formats::{LoadOptions, Source};
use textweaver_text::Document;

fn write(root: &Path, rel: &str, text: &str) -> PathBuf {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, text).unwrap();
    p
}

fn load(path: &Path) -> Document {
    textweaver_formats::Registry::with_builtins()
        .load(&Source::Path(path.to_owned()), &LoadOptions::default())
        .unwrap()
}

/// A vault with notes in sibling and nested folders, two notes named
/// `Dup`, and a note outside the vault.
fn vault() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("vault");
    std::fs::create_dir_all(root.join(".obsidian")).unwrap();
    write(
        &root,
        "Projects/Plan.md",
        "---\ntitle: The Plan\naliases:\n  - Roadmap\n---\n# Plan\n\nPlan text.\n\nKey point here. ^key-point\n",
    );
    write(&root, "Projects/Dup.md", "Near dup.\n");
    write(&root, "Archive/old/Dup.md", "Far dup.\n");
    write(&root, "Notes/deep/Leaf.md", "Leaf text.\n");
    write(tmp.path(), "Secret.md", "secret words\n");
    (tmp, root)
}

#[test]
fn embeds_reach_sibling_folders_by_name_path_and_alias() {
    let (_tmp, root) = vault();
    let main = write(
        &root,
        "Notes/Main.md",
        "# Main\n\n![[Plan#^key-point]]\n\n![[Leaf]]\n\n![[Roadmap]]\n\n![[Projects/Dup]]\n",
    );
    let text = load(&main).text().to_string();
    for e in [
        "Embedded from Plan, key-point\n\nKey point here.",
        "Embedded from Leaf\n\nLeaf text.",
        "Embedded from Roadmap\n\nPlan\n\nPlan text.",
        "Embedded from Projects/Dup\n\nNear dup.",
    ] {
        assert!(text.contains(e), "{e:?} missing from {text:?}");
    }
}

#[test]
fn duplicate_embeds_pick_the_nearest() {
    let (_tmp, root) = vault();
    let near = write(&root, "Archive/old/Main.md", "![[Dup]]\n");
    assert!(load(&near).text().to_string().contains("Far dup."));
    let far = write(&root, "Projects/Main.md", "![[Dup]]\n");
    assert!(load(&far).text().to_string().contains("Near dup."));
}

#[test]
fn embeds_never_leave_the_vault() {
    let (tmp, root) = vault();
    let abs = tmp
        .path()
        .join("Secret")
        .to_string_lossy()
        .replace('\\', "/");
    let main = write(
        &root,
        "Notes/Main.md",
        &format!("![[Secret]]\n\n![[../../Secret]]\n\n![[{abs}]]\n"),
    );
    let text = load(&main).text().to_string();
    assert!(!text.contains("secret words"), "{text}");
    assert_eq!(
        text.matches("(embedded note not found)").count(),
        3,
        "{text}"
    );
}

#[test]
fn without_a_vault_embeds_stay_in_the_note_folder() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "docs/sub/Deep.md", "Deep text.\n");
    write(tmp.path(), "other/Side.md", "Side text.\n");
    let main = write(tmp.path(), "docs/Main.md", "![[Deep]]\n\n![[Side]]\n");
    let text = load(&main).text().to_string();
    assert!(text.contains("Deep text."), "{text}");
    assert!(text.contains("Side (embedded note not found)"), "{text}");
}

#[test]
fn properties_are_listed_in_order_with_lists_joined() {
    let (_tmp, root) = vault();
    let note = write(
        &root,
        "Notes/Props.md",
        "---\ntitle: Props\ntags:\n  - exam\n  - review\naliases: [Properties note]\ndone: true\nnested:\n  a: 1\n---\nBody.\n",
    );
    let d = load(&note);
    assert_eq!(d.text().to_string(), "Body.");
    assert_eq!(
        d.meta.properties.get("aliases").map(String::as_str),
        Some("Properties note")
    );
    assert_eq!(
        front_matter_properties(&d.meta),
        vec![
            ("title".to_owned(), "Props".to_owned()),
            ("tags".to_owned(), "exam, review".to_owned()),
            ("aliases".to_owned(), "Properties note".to_owned()),
            ("done".to_owned(), "true".to_owned()),
        ]
    );
}
