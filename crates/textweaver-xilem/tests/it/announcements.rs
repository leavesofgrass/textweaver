//! Every message the window says has a level (Wave 6, W6a6; ADR-0043):
//! the window's own messages go through `App::announce_as` with an
//! `Importance`, so `[accessibility] interface_announcements` decides
//! whether each is heard, as it does for the app's.
//!
//! The check reads the window's sources, as the app's own check does: a
//! call to the plain `announce`, or to `announce_queued` or `echo` outside
//! the few places that gate them by hand, fails.

use std::path::Path;

#[test]
fn every_window_announcement_has_a_level() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    // Calls with no level of their own.
    let sinks = [".announce(", ".announce_queued(", ".echo("];
    // File, function, and sink allowed there, with the reason.
    let allowed: &[(&str, &str, &str)] = &[
        // Startup warnings (never silenced) and the first run's welcome,
        // which is said only when `interface_allows(Importance::Tip)`.
        ("gui.rs", "fn start", ".announce_queued("),
        // Caret and selection echo in the self-voicing mode: reading the
        // text the user moved over, not an interface message.
        ("gui.rs", "fn echo_caret", ".echo("),
        // The window taking the focus, said only when
        // `interface_allows(Importance::Dialog)`.
        ("gui.rs", "fn window_focused", ".echo("),
    ];
    let mut bad = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("the sources") {
        let path = entry.expect("an entry").path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = std::fs::read_to_string(&path).expect("a source file");
        let mut current_fn = String::new();
        for (n, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            if let Some(i) = t.find("fn ")
                && (t.starts_with("fn ") || t.starts_with("pub"))
            {
                current_fn = t[i..]
                    .split(['(', '<'])
                    .next()
                    .unwrap_or_default()
                    .to_owned();
            }
            for s in sinks {
                let called = t.match_indices(s).any(|(i, _)| !t[..i].ends_with('"'));
                // The live region's own `say` and the announcer trait's
                // `announce` (the queue the app writes to) are not calls
                // from the window's code.
                let trait_impl = s == ".announce(" && t.contains("fn announce(");
                if called
                    && !trait_impl
                    && !allowed
                        .iter()
                        .any(|(f, func, sink)| *f == name && current_fn == *func && *sink == s)
                {
                    bad.push(format!("{name}:{}: {s} in {current_fn}", n + 1));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "window announcements without a level: {bad:#?}"
    );
}
