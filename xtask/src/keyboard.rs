//! `cargo xtask keyboard`: writes `docs/keyboard.md` from the keymap.
//! `cargo xtask keyboard --check` fails when the committed file is stale.
//! Owner: Agent C.

use std::path::PathBuf;

use anyhow::Context;

/// `docs/keyboard.md` in the workspace.
fn target() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|root| root.join("docs").join("keyboard.md"))
        .unwrap_or_else(|| PathBuf::from("docs/keyboard.md"))
}

/// Line endings do not matter (Git may check the file out with CRLF).
fn normalized(s: &str) -> String {
    s.replace("\r\n", "\n")
}

pub fn run() -> anyhow::Result<()> {
    let path = target();
    let text = textweaver_keymap::keyboard_markdown();
    let check = std::env::args().skip(2).any(|a| a == "--check");
    if check {
        let current = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        anyhow::ensure!(
            normalized(&current) == text,
            "{} is out of date; run `cargo xtask keyboard`",
            path.display()
        );
        println!("{} is up to date", path.display());
        return Ok(());
    }
    std::fs::write(&path, &text).with_context(|| format!("writing {}", path.display()))?;
    println!(
        "wrote {} ({} actions)",
        path.display(),
        textweaver_keymap::ActionId::ALL.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed `docs/keyboard.md` matches the keymap. Regenerate with
    /// `cargo xtask keyboard` after changing an action or a default.
    #[test]
    fn keyboard_md_is_current() {
        let current = std::fs::read_to_string(target()).expect("docs/keyboard.md exists");
        assert!(
            normalized(&current) == textweaver_keymap::keyboard_markdown(),
            "docs/keyboard.md is stale; run `cargo xtask keyboard`"
        );
    }
}
