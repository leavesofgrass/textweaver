//! CSS custom properties for HTML output (Agent L's templates and exports).
//!
//! [`variables`] gives one theme's properties; [`stylesheet`] pairs a light
//! and a dark theme under `prefers-color-scheme`, adds a high-contrast theme
//! under `prefers-contrast: more`, maps everything to system colors under
//! `forced-colors: active` (so Windows contrast themes win, as they should),
//! and appends element rules that use the properties. Properties are named
//! `--tw-<key>` with underscores as hyphens: `--tw-dim-text`,
//! `--tw-spoken-word-bg`, `--tw-highlight-yellow-fg`.
//!
//! Highlights carry their attributes as properties too
//! (`--tw-spoken-word-weight`, `-decoration`, `-style`), so HTML never marks
//! them by color alone. Reverse video is resolved to plain colors.

use std::fmt::Write as _;

use crate::model::{Attrs, ColorRole, Resolved, StyleRole, Theme};

fn prop(key: &str) -> String {
    format!("--tw-{}", key.replace('_', "-"))
}

/// A user-highlight name made safe for a CSS identifier.
pub fn css_ident(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    if s.is_empty() { "x".into() } else { s }
}

fn attrs_props(out: &mut String, indent: &str, key: &str, a: Attrs) {
    let p = prop(key);
    let _ = writeln!(
        out,
        "{indent}{p}-weight: {};",
        if a.bold { "bold" } else { "inherit" }
    );
    let _ = writeln!(
        out,
        "{indent}{p}-decoration: {};",
        if a.underline { "underline" } else { "none" }
    );
    let _ = writeln!(
        out,
        "{indent}{p}-style: {};",
        if a.italic { "italic" } else { "inherit" }
    );
}

fn style_props(out: &mut String, indent: &str, key: &str, s: Resolved) {
    let p = prop(key);
    let _ = writeln!(out, "{indent}{p}-fg: {};", s.foreground);
    let bg = if s.has_band {
        s.background.hex()
    } else {
        "transparent".to_owned()
    };
    let _ = writeln!(out, "{indent}{p}-bg: {bg};");
    attrs_props(out, indent, key, s.attributes);
}

/// The declarations for one theme (no selector), one per line, each line
/// starting with `indent`.
pub fn declarations(theme: &Theme, indent: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{indent}color-scheme: {};",
        if theme.is_dark() { "dark" } else { "light" }
    );
    for &r in ColorRole::ALL {
        let _ = writeln!(out, "{indent}{}: {};", prop(r.key()), theme.color(r));
    }
    for &r in StyleRole::ALL {
        style_props(&mut out, indent, r.key(), theme.resolve_style(r));
    }
    for h in &theme.user_highlights {
        let key = format!("highlight-{}", css_ident(&h.name));
        style_props(&mut out, indent, &key, theme.resolve(&h.style));
    }
    out
}

/// One theme's properties on `:root`.
pub fn variables(theme: &Theme) -> String {
    format!(
        "/* textweaver theme: {} */\n:root {{\n{}}}\n",
        theme.meta.display_name,
        declarations(theme, "  ")
    )
}

/// The themes a stylesheet is built from.
#[derive(Clone, Copy, Debug)]
pub struct SchemeSet<'a> {
    /// Used by default and under `prefers-color-scheme: light`.
    pub light: &'a Theme,
    /// Used under `prefers-color-scheme: dark`.
    pub dark: &'a Theme,
    /// Used under `prefers-contrast: more`, when given.
    pub high_contrast: Option<&'a Theme>,
}

fn forced_colors(theme: &Theme) -> String {
    // System colors: the platform's contrast theme decides.
    let mut out = String::new();
    let i = "    ";
    let _ = writeln!(out, "{i}color-scheme: light dark;");
    for &r in ColorRole::ALL {
        let v = match r {
            ColorRole::Background | ColorRole::Surface | ColorRole::CodeBackground => "Canvas",
            ColorRole::Link => "LinkText",
            ColorRole::DimText => "GrayText",
            _ => "CanvasText",
        };
        let _ = writeln!(out, "{i}{}: {v};", prop(r.key()));
    }
    for &r in StyleRole::ALL {
        let p = prop(r.key());
        let (fg, bg) = match r {
            StyleRole::Selection
            | StyleRole::SpokenWord
            | StyleRole::CurrentFindHit
            | StyleRole::Focus => ("HighlightText", "Highlight"),
            StyleRole::StatusBar => ("ButtonText", "ButtonFace"),
            _ => ("CanvasText", "Canvas"),
        };
        let _ = writeln!(out, "{i}{p}-fg: {fg};");
        let _ = writeln!(out, "{i}{p}-bg: {bg};");
    }
    for h in &theme.user_highlights {
        let p = prop(&format!("highlight-{}", css_ident(&h.name)));
        let _ = writeln!(out, "{i}{p}-fg: HighlightText;");
        let _ = writeln!(out, "{i}{p}-bg: Highlight;");
    }
    out
}

/// Element rules using the properties: body, headings, links (always
/// underlined), code, quotes, rules, tables, selection, focus rings, and a
/// class per highlight (`.tw-spoken-word`, `.tw-find-hit`,
/// `.tw-highlight-yellow`, …).
pub fn rules(highlight_names: &[&str]) -> String {
    let mut out = String::from(
        "body { background: var(--tw-background); color: var(--tw-text); }\n\
         h1 { color: var(--tw-heading1); }\n\
         h2 { color: var(--tw-heading2); }\n\
         h3 { color: var(--tw-heading3); }\n\
         h4 { color: var(--tw-heading4); }\n\
         h5 { color: var(--tw-heading5); }\n\
         h6 { color: var(--tw-heading6); }\n\
         a { color: var(--tw-link); text-decoration: underline; }\n\
         code { color: var(--tw-code); background: var(--tw-code-background); }\n\
         pre { color: var(--tw-text); background: var(--tw-code-background); }\n\
         pre code { background: transparent; }\n\
         blockquote { color: var(--tw-quote); border-left: 3px solid var(--tw-dim-text); padding-left: 0.75em; }\n\
         hr { border: 0; border-top: 1px solid var(--tw-dim-text); }\n\
         th, td { border: 1px solid var(--tw-dim-text); }\n\
         .tw-dim { color: var(--tw-dim-text); }\n\
         .tw-error { color: var(--tw-error); font-weight: bold; }\n\
         .tw-panel { background: var(--tw-surface); color: var(--tw-text); }\n\
         ::selection { color: var(--tw-selection-fg); background: var(--tw-selection-bg); }\n\
         :focus-visible { outline: 3px solid var(--tw-focus-bg); outline-offset: 2px; }\n",
    );
    let class = |out: &mut String, cls: &str, key: &str| {
        let p = prop(key);
        let _ = writeln!(
            out,
            ".{cls} {{ color: var({p}-fg); background: var({p}-bg); font-weight: var({p}-weight); \
             text-decoration: var({p}-decoration); font-style: var({p}-style); }}"
        );
    };
    for &r in StyleRole::ALL {
        if r == StyleRole::Selection {
            continue;
        }
        class(&mut out, &prop(r.key())[2..], r.key());
    }
    for n in highlight_names {
        let key = format!("highlight-{}", css_ident(n));
        class(&mut out, &prop(&key)[2..], &key);
    }
    out
}

/// A complete stylesheet: light properties on `:root`, dark ones under
/// `prefers-color-scheme: dark`, high contrast under `prefers-contrast:
/// more`, system colors under `forced-colors: active`, then [`rules`].
pub fn stylesheet(set: SchemeSet<'_>) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "/* textweaver themes: {} (light), {} (dark){} */",
        set.light.meta.display_name,
        set.dark.meta.display_name,
        set.high_contrast
            .map(|t| format!(", {} (more contrast)", t.meta.display_name))
            .unwrap_or_default()
    );
    let _ = write!(out, ":root {{\n{}}}\n", declarations(set.light, "  "));
    let _ = write!(
        out,
        "@media (prefers-color-scheme: dark) {{\n  :root {{\n{}  }}\n}}\n",
        declarations(set.dark, "    ")
    );
    if let Some(hc) = set.high_contrast {
        let _ = write!(
            out,
            "@media (prefers-contrast: more) {{\n  :root {{\n{}  }}\n}}\n",
            declarations(hc, "    ")
        );
    }
    let _ = write!(
        out,
        "@media (forced-colors: active) {{\n  :root {{\n{}  }}\n}}\n",
        forced_colors(set.light)
    );
    let mut names: Vec<&str> = Vec::new();
    for t in [Some(set.light), Some(set.dark), set.high_contrast]
        .into_iter()
        .flatten()
    {
        for h in &t.user_highlights {
            if !names.iter().any(|n| n.eq_ignore_ascii_case(&h.name)) {
                names.push(&h.name);
            }
        }
    }
    out.push_str(&rules(&names));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idents() {
        assert_eq!(css_ident("Sky Blue"), "sky-blue");
        assert_eq!(css_ident(""), "x");
        assert_eq!(prop("dim_text"), "--tw-dim-text");
    }
}
