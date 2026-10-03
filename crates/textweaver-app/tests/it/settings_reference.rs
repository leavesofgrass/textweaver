//! The generated settings reference, `docs/settings-reference.md`.
//!
//! The reference is rendered from [`SettingsSchema::generate`], so every
//! key in `settings.toml` is listed with its default, label, help, and
//! range or choices, and it can never drift from the code. This test
//! compares the committed file with a fresh rendering and fails with the
//! first line that differs. With `TEXTWEAVER_UPDATE_DOCS=1` it writes the
//! file instead: run `cargo xtask settings-doc` after adding or changing a
//! setting.
//!
//! The file is meant to be read with a screen reader and a Braille
//! display: one list item per setting, the key first, then the default,
//! the label, and the help, in words, with no tables.

use std::fmt::Write as _;
use std::path::PathBuf;

use serde_json::Value;
use textweaver_app::{Setting, SettingKind, SettingsSchema};

/// `docs/settings-reference.md` in the repository.
fn target() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("settings-reference.md")
}

/// A number as it reads: no trailing zeros ("1.5", "300").
fn number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// A value as it is written in `settings.toml`.
fn toml_value(v: &Value) -> String {
    match v {
        Value::Null => "not set".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), number),
        Value::String(s) => format!("\"{s}\""),
        Value::Array(items) if items.is_empty() => "an empty list".to_owned(),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(toml_value).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(m) if m.is_empty() => "an empty table".to_owned(),
        Value::Object(m) => match m.len() {
            1 => "a table of 1 entry".to_owned(),
            n => format!("a table of {n} entries"),
        },
    }
}

/// "default 265 words per minute", "default on (`true`)", "default
/// automatic (`"auto"`)": the default as textweaver says it, with the value
/// as written in the file when the two differ.
fn default_text(s: &Setting) -> String {
    let said = s.describe(&s.default);
    let written = toml_value(&s.default);
    let plain_number = matches!(s.default, Value::Number(_))
        && (said == written || said.starts_with(&format!("{written} ")));
    if plain_number || said == written || matches!(s.default, Value::Null | Value::Object(_)) {
        format!("default {said}")
    } else if said == written.trim_matches('"') {
        format!("default `{written}`")
    } else if matches!(&s.default, Value::Array(a) if a.is_empty()) {
        "default an empty list".to_owned()
    } else {
        format!("default {said} (`{written}`)")
    }
}

/// The range, the choices, or how the value is written, in words.
fn values_text(s: &Setting) -> Option<String> {
    match &s.kind {
        SettingKind::Toggle => Some("On or off: `true` or `false`.".to_owned()),
        SettingKind::Number {
            min,
            max,
            step,
            unit,
        } => {
            if *min <= f64::MIN || *max >= f64::MAX {
                return None;
            }
            // The unit as said after the maximum: "From 0 to 10 rows",
            // "From 0 to 1 second".
            let to = if unit.is_empty() {
                number(*max)
            } else {
                s.describe(&serde_json::json!(*max))
            };
            Some(format!(
                "From {} to {to}, in steps of {}.",
                number(*min),
                number(*step)
            ))
        }
        SettingKind::Choice { choices, open } => {
            let list: Vec<String> = choices
                .iter()
                .map(|c| {
                    let written = toml_value(&c.value);
                    let bare = written.trim_matches('"');
                    if bare == c.label {
                        format!("`{written}`")
                    } else {
                        format!("`{written}` ({})", c.label)
                    }
                })
                .collect();
            // An open list with no fixed choices (the theme) says only
            // that any value may be written.
            if list.is_empty() {
                return open.then(|| "Any value may be written.".to_owned());
            }
            let mut out = format!("Choices: {}.", list.join(", "));
            if *open {
                out.push_str(" Other values may be written too.");
            }
            Some(out)
        }
        SettingKind::Text { optional: true } => Some("Text; empty means not set.".to_owned()),
        SettingKind::Text { optional: false } => Some("Text.".to_owned()),
        SettingKind::List => Some("A list of texts, such as `[\"a\", \"b\"]`.".to_owned()),
        SettingKind::Table => Some("A table of names and values, edited in the file.".to_owned()),
    }
}

/// Ends `text` with a full stop unless it already ends a sentence.
fn sentence(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() || t.ends_with(['.', '?', '!', ':']) {
        t.to_owned()
    } else {
        format!("{t}.")
    }
}

fn item(out: &mut String, s: &Setting) {
    let mut line = format!("- `{}`: {}. {}", s.path, default_text(s), sentence(s.label));
    if !s.help.is_empty() {
        line.push(' ');
        line.push_str(&sentence(s.help));
    }
    if let Some(values) = values_text(s) {
        line.push(' ');
        line.push_str(&values);
    }
    line.push(' ');
    line.push_str(sync_text(&s.path));
    let _ = writeln!(out, "{line}");
}

/// Whether the setting syncs between computers (ADR-0049): portable
/// settings do, with the settings group or a group of their own; machine
/// settings never do.
fn sync_text(path: &str) -> &'static str {
    match textweaver_app::store::sync_scope::sync_group_of(path) {
        Some("settings") => "Syncs between computers.",
        Some("favorite_voices") => "Syncs between computers, with favorite voices.",
        Some("glossary") => "Syncs between computers, with the glossary.",
        Some(_) => "Syncs between computers.",
        None => "Stays on this computer.",
    }
}

/// The whole reference.
fn render(schema: &SettingsSchema) -> String {
    let mut out = String::new();
    out.push_str(
        "# Settings reference\n\n\
         Every setting in `settings.toml`, section by section, in the order the settings screen lists them. \
         Each item gives the key, its default, its label on the settings screen, what it does, \
         and the values it takes, then whether it syncs between computers \
         ([Syncing between computers](sync.md)). [Settings](settings.md) explains where the file lives, \
         and how to export, import, and reset it.\n\n\
         This page is generated from the settings schema by `cargo xtask settings-doc`. \
         Do not edit it by hand; CI checks that it is current.\n",
    );
    let mut section: Option<&str> = None;
    for s in schema.settings.iter().filter(|s| !s.internal) {
        let top = s.path.split('.').next().unwrap_or_default();
        if section != Some(top) {
            section = Some(top);
            let _ = write!(out, "\n## {}: the `[{top}]` section\n\n", s.section);
        }
        item(&mut out, s);
    }
    let internal: Vec<&Setting> = schema.settings.iter().filter(|s| s.internal).collect();
    if !internal.is_empty() {
        out.push_str(
            "\n## Kept by textweaver\n\n\
             textweaver writes these itself, such as a question already asked. \
             They are in the file, but not on the settings screen.\n\n",
        );
        for s in internal {
            item(&mut out, s);
        }
    }
    out.push_str(
        "\n## See also\n\n\
         - [Settings](settings.md): where settings live, and export, import, and reset.\n\
         - [JSON-RPC](json-rpc.md): `settings_schema`, `get_setting`, and `set_setting`.\n\
         - [Documentation index](README.md)\n",
    );
    out
}

/// Line endings do not matter (Git may check the file out with CRLF).
fn normalized(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn settings_reference_is_current() {
    let path = target();
    let fresh = render(&SettingsSchema::generate());
    if std::env::var_os("TEXTWEAVER_UPDATE_DOCS").is_some_and(|v| v == "1") {
        std::fs::write(&path, &fresh).expect("docs/settings-reference.md is writable");
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let current = normalized(&current);
    if current == fresh {
        return;
    }
    let (n, (old, new)) = current
        .lines()
        .chain(std::iter::repeat(""))
        .zip(fresh.lines().chain(std::iter::repeat("")))
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .unwrap_or((0, ("", "")));
    panic!(
        "docs/settings-reference.md is out of date; run `cargo xtask settings-doc`.\n\
         First difference, line {}:\n  file:   {old}\n  schema: {new}",
        n + 1
    );
}

#[test]
fn every_setting_is_listed_once() {
    let schema = SettingsSchema::generate();
    let text = render(&schema);
    for s in &schema.settings {
        let needle = format!("- `{}`: ", s.path);
        assert_eq!(text.matches(&needle).count(), 1, "{}", s.path);
    }
    assert!(!text.contains('|'), "no tables");
}
