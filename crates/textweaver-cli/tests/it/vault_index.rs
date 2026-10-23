//! `tw vault backlinks`, `unresolved`, `tags`, and `properties` (beta 4):
//! the vault index's answers, as text and JSON, over a small vault of
//! placeholder notes built in a temporary folder. They only read.

use std::path::Path;
use std::process::{Command, Stdio};

fn tw(home: &Path, args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_tw"))
        .args(args)
        .env("TEXTWEAVER_HOME", home)
        .env("TEXTWEAVER_ESPEAK_OUTPUT", "virtual")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text.replace("\r\n", "\n"))
}

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

#[test]
fn backlinks_unresolved_tags_and_properties() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let vault = tmp.path().join("vault");
    std::fs::create_dir_all(vault.join(".obsidian")).unwrap();
    write(
        &vault,
        "Notes/Cell.md",
        "---\ntitle: The Cell\naliases: [Cytology]\ntags: [biology]\n---\nSee [[Ghost]].\n\nstatus:: draft\n- supports:: [[Mitochondria]]\n",
    );
    write(
        &vault,
        "Projects/Mitochondria.md",
        "About [[Cytology]] and [[Cell]]. #biology/organelles\n",
    );
    let cell = vault.join("Notes/Cell.md");
    let cell = cell.to_str().unwrap();
    let v = vault.to_str().unwrap();

    let (ok, out) = tw(&home, &["vault", "backlinks", cell]);
    assert!(ok, "{out}");
    assert_eq!(
        out,
        "1 note links to The Cell:\nMitochondria, in Projects, 2 links\n"
    );
    let (ok, out) = tw(&home, &["vault", "backlinks", cell, "--json"]);
    assert!(ok, "{out}");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["backlinks"][0]["links"], 2);
    assert_eq!(json["backlinks"][0]["title"], "Mitochondria");
    assert_eq!(json["partial"], false);

    let (ok, out) = tw(&home, &["vault", "unresolved", v]);
    assert!(ok, "{out}");
    assert_eq!(
        out,
        "1 link to notes that don't exist:\nGhost, from The Cell, in Notes\n"
    );

    let (ok, out) = tw(&home, &["vault", "tags", v]);
    assert!(ok, "{out}");
    assert_eq!(out, "biology, 1 note\nbiology/organelles, 1 note\n");
    let (ok, out) = tw(&home, &["vault", "tags", v, "--tag", "#biology"]);
    assert!(ok, "{out}");
    assert_eq!(
        out,
        "Tag biology, 2 notes:\nThe Cell, in Notes\nMitochondria, in Projects\n"
    );

    let (ok, out) = tw(&home, &["vault", "properties", cell]);
    assert!(ok, "{out}");
    assert_eq!(
        out,
        "Properties: 3\ntitle: The Cell\naliases: Cytology\ntags: biology\n\
         Dataview fields: 2\nstatus: draft\nsupports: [[Mitochondria]]\n"
    );
    // A folder where a note is needed is a usage mistake, said plainly.
    let (ok, out) = tw(&home, &["vault", "backlinks", v]);
    assert!(!ok);
    assert!(out.contains("Name a note file, not a folder"), "{out}");
    // Nothing was written into the vault.
    let mut names: Vec<String> = std::fs::read_dir(&vault)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, [".obsidian", "Notes", "Projects"]);
}
