//! CSS output: a snapshot of the default light/dark/high-contrast
//! stylesheet, and the properties every template can rely on.

use textweaver_theme::css::{SchemeSet, stylesheet, variables};
use textweaver_theme::{ColorRole, StyleRole, builtin};

fn default_set() -> String {
    stylesheet(SchemeSet {
        light: builtin::get("galaxy-light").unwrap(),
        dark: builtin::get("galaxy").unwrap(),
        high_contrast: Some(builtin::get("high-contrast").unwrap()),
    })
}

#[test]
fn default_stylesheet_snapshot() {
    insta::assert_snapshot!("galaxy_pair_with_high_contrast", default_set());
}

#[test]
fn sepia_variables_snapshot() {
    insta::assert_snapshot!("sepia_variables", variables(builtin::get("sepia").unwrap()));
}

#[test]
fn every_role_has_a_property_in_every_block() {
    let css = default_set();
    for &r in ColorRole::ALL {
        let p = format!("--tw-{}:", r.key().replace('_', "-"));
        // light, dark, more contrast, forced colors
        assert_eq!(css.matches(&p).count(), 4, "{p}");
    }
    for &r in StyleRole::ALL {
        let p = format!("--tw-{}-fg:", r.key().replace('_', "-"));
        assert_eq!(css.matches(&p).count(), 4, "{p}");
    }
    assert!(css.contains("@media (prefers-color-scheme: dark)"));
    assert!(css.contains("@media (prefers-contrast: more)"));
    assert!(css.contains("@media (forced-colors: active)"));
    assert!(css.contains("a { color: var(--tw-link); text-decoration: underline; }"));
    assert!(css.contains(".tw-highlight-yellow {"));
    assert!(css.contains(":focus-visible"));
    assert!(!css.contains("outline: none"));
}
