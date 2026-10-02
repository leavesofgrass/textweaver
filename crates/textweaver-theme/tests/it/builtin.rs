//! The built-in themes: Star's 23 palettes, in step with their generator,
//! round-tripping through TOML, and passing every contrast check.

use textweaver_theme::{
    ColorRole, Repair, StyleRole, Theme, ThemeFile, ThemeKind, builtin, check, resolve::resolve,
    star,
};

#[test]
fn all_23_star_palettes_are_built_in_in_star_order() {
    let names: Vec<&str> = builtin::all().iter().map(|t| t.name()).collect();
    let star: Vec<&str> = star::PALETTES.iter().map(|p| p.name).collect();
    assert_eq!(names.len(), 23);
    assert_eq!(names, star);
    assert_eq!(builtin::NAMES.to_vec(), star);
    assert_eq!(builtin::default_theme().name(), "galaxy");
}

#[test]
fn shipped_files_match_the_generator() {
    for p in &star::PALETTES {
        let expected = star::generated_file(p).unwrap();
        let shipped = builtin::source(p.name).unwrap().replace("\r\n", "\n");
        assert!(
            shipped == expected,
            "themes/{}.toml is stale; run `cargo run -p textweaver-theme --example generate_builtin_themes`",
            p.name
        );
    }
}

#[test]
fn the_required_themes_pass_every_check() {
    // The owner's policy: only Galaxy, Galaxy Light, and the high-contrast themes
    // must meet WCAG AA; the others keep Star's colors and are labelled.
    let mut failures = Vec::new();
    for t in builtin::all()
        .iter()
        .filter(|t| star::must_meet_aa(t.name()))
    {
        let r = check(t);
        assert!(r.checks.len() >= 40, "{}", t.name());
        if !r.passed() {
            failures.push(r.summary());
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The two-tone highlight: in every built-in theme the spoken word's band
/// stands 3 to 1 above the page and above the sentence's band. The
/// sentence's band itself is a soft tint with no check of its own (the
/// underline marks the sentence), so no theme fails for it.
#[test]
fn every_theme_steps_the_word_band_up_from_page_and_sentence() {
    for t in builtin::all() {
        let r = check(t);
        let word: Vec<_> = r
            .checks
            .iter()
            .filter(|c| c.subject.starts_with("spoken word band against"))
            .collect();
        assert_eq!(word.len(), 2, "{}", t.name());
        for c in word {
            assert!(c.passed, "{}: {}", t.name(), c.describe());
            assert!(c.ratio.unwrap() >= 3.0, "{}: {}", t.name(), c.describe());
        }
        assert!(
            !r.checks
                .iter()
                .any(|c| c.subject.starts_with("spoken sentence band against")),
            "{}: no sentence band check",
            t.name()
        );
    }
}

#[test]
fn high_contrast_themes_reach_seven_to_one() {
    for name in ["contrast", "high-contrast"] {
        let t = builtin::get(name).unwrap();
        assert_eq!(t.kind(), ThemeKind::HighContrast);
        for c in check(t).checks {
            if c.requirement == textweaver_theme::Requirement::Text {
                assert!(c.ratio.unwrap() >= 7.0, "{name}: {}", c.describe());
            }
        }
    }
}

#[test]
fn star_colors_are_kept_where_they_passed() {
    // Only the adjusted keys differ from Star.
    for p in &star::PALETTES {
        let (_, adj) = star::port(p).unwrap();
        let t = builtin::get(p.name).unwrap();
        let changed = |key: &str| adj.iter().any(|a| a.key == key);
        for (role, star_color) in [
            (ColorRole::Background, p.bg),
            (ColorRole::Text, p.fg),
            (ColorRole::Heading1, p.h1),
            (ColorRole::Heading2, p.h2),
            (ColorRole::Heading3, p.h3),
            (ColorRole::Heading4, p.h4),
            (ColorRole::Link, p.link),
            (ColorRole::Code, p.code),
            (ColorRole::CodeBackground, p.code_bg),
            (ColorRole::DimText, p.muted),
        ] {
            let key = format!("colors.{}", role.key());
            if !changed(&key) {
                assert_eq!(t.color(role), star_color, "{} {key}", p.name);
            }
        }
        if !changed("styles.selection.background") {
            assert_eq!(
                t.style(StyleRole::Selection).background,
                Some(p.sel),
                "{}",
                p.name
            );
        }
        for a in &adj {
            assert!(
                a.ratio_before < a.minimum && a.ratio_after >= a.minimum,
                "{a:?}"
            );
            // Minimal: the change stays near the floor.
            assert!(a.ratio_after < a.minimum + 0.15, "{a:?}");
        }
    }
}

#[test]
fn every_theme_round_trips_through_toml() {
    for t in builtin::all() {
        let text = t.to_toml_string();
        let file = ThemeFile::parse(&text).unwrap();
        assert_eq!(ThemeFile::parse(&file.to_toml_string()).unwrap(), file);
        let (again, adj) = resolve(&file, None, None, Repair::DerivedOnly).unwrap();
        assert!(adj.is_empty());
        assert_eq!(&again, t, "{}", t.name());
    }
}

#[test]
fn counterparts_pair_up() {
    for t in builtin::all() {
        if let Some(c) = &t.meta.counterpart {
            let other: &Theme = builtin::get(c).unwrap();
            assert_eq!(other.meta.counterpart.as_deref(), Some(t.name()));
            assert_ne!(other.is_dark(), t.is_dark());
        }
    }
}

#[test]
fn names_and_aliases() {
    assert_eq!(builtin::get("obsidian").unwrap().name(), "galaxy");
    assert_eq!(builtin::get("zed-one-light").unwrap().name(), "one-light");
    assert_eq!(
        builtin::get(" HIGH_CONTRAST ").unwrap().name(),
        "high-contrast"
    );
    assert_eq!(builtin::get("Sepia").unwrap().name(), "sepia");
    assert!(builtin::get("solarized").is_none());
    assert!(builtin::source("nord").unwrap().contains("[colors]"));
}

#[test]
fn highlights_that_appear_together_differ_without_color() {
    for t in builtin::all() {
        let a = |r: StyleRole| t.style(r).attributes;
        assert_ne!(
            a(StyleRole::SpokenWord),
            a(StyleRole::SpokenSentence),
            "{}",
            t.name()
        );
        assert_ne!(
            a(StyleRole::FindHit),
            a(StyleRole::CurrentFindHit),
            "{}",
            t.name()
        );
        for &r in StyleRole::ALL {
            if r.is_highlight() {
                assert!(!a(r).is_empty(), "{} {r}", t.name());
            }
        }
    }
}

#[test]
fn gui_rgb_table_covers_every_role() {
    let t = builtin::get("sepia").unwrap();
    let table = t.rgb_table();
    assert_eq!(
        table.len(),
        ColorRole::COUNT + 2 * StyleRole::COUNT + 2 * t.user_highlights.len()
    );
    let get = |k: &str| table.iter().find(|(n, _)| n == k).map(|(_, c)| *c);
    assert_eq!(get("background"), Some(t.color(ColorRole::Background)));
    // Selection is reverse video: the table has the colors as shown.
    assert_eq!(get("selection.background"), Some(t.color(ColorRole::Text)));
    assert_eq!(
        get("bookmark.background"),
        Some(t.color(ColorRole::Background))
    );
    assert!(get("highlight.yellow.background").is_some());
}

#[test]
fn adjustments_table_lists_every_change() {
    let table = star::adjustments_table().unwrap();
    let rows = table.lines().count() - 2;
    let total: usize = star::PALETTES
        .iter()
        .map(|p| star::port(p).unwrap().1.len())
        .sum();
    assert_eq!(rows, total);
    assert!(table.contains("| galaxy | `colors.dim_text` | `#7d7d7d` |"));
}

#[test]
fn galaxy_is_the_default_and_faithful_to_star() {
    // The owner's theme: Star's default, modeled on Obsidian's dark palette.
    let g = builtin::default_theme();
    assert_eq!(g.name(), "galaxy");
    assert_eq!(textweaver_theme::DEFAULT_THEME, "galaxy");
    assert_eq!(builtin::NAMES[0], "galaxy");
    assert_eq!(g.kind(), ThemeKind::Dark);
    assert_eq!(g.meta.counterpart.as_deref(), Some("galaxy-light"));
    // Every Star value is kept except muted, which moved from 4.05 to 4.52
    // to 1, toward Obsidian's own muted gray.
    let (_, adj) = star::port(star::palette("galaxy").unwrap()).unwrap();
    assert_eq!(adj.len(), 1);
    assert_eq!(adj[0].key, "colors.dim_text");
    let hex = |r: ColorRole| g.color(r).hex();
    assert_eq!(hex(ColorRole::Background), "#1e1e1e");
    assert_eq!(hex(ColorRole::Text), "#dadada");
    assert_eq!(hex(ColorRole::Heading1), "#c9b6ff");
    assert_eq!(hex(ColorRole::Link), "#a882ff");
    assert_eq!(hex(ColorRole::DimText), "#858585");
    // The light pair passes too and points back.
    let gl = builtin::get("galaxy-light").unwrap();
    assert!(check(gl).passed());
    assert_eq!(gl.meta.counterpart.as_deref(), Some("galaxy"));
}

#[test]
fn themes_outside_the_required_set_keep_star_colors_exactly() {
    for p in star::PALETTES
        .iter()
        .filter(|p| !star::must_meet_aa(p.name))
    {
        let (_, adj) = star::port(p).unwrap();
        let explicit: Vec<_> = adj
            .iter()
            .filter(|a| {
                [
                    "colors.background",
                    "colors.text",
                    "colors.heading1",
                    "colors.heading2",
                    "colors.heading3",
                    "colors.heading4",
                    "colors.link",
                    "colors.code",
                    "colors.code_background",
                    "colors.dim_text",
                    "colors.error",
                ]
                .contains(&a.key.as_str())
            })
            .collect();
        assert!(explicit.is_empty(), "{}: {explicit:?}", p.name);
        let t = builtin::get(p.name).unwrap();
        assert_eq!(t.color(ColorRole::DimText), p.muted, "{}", p.name);
        assert_eq!(t.color(ColorRole::Text), p.fg, "{}", p.name);
    }
}
