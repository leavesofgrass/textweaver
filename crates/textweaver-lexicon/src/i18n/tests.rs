use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::*;
use crate::model::Pos;

#[test]
fn english_catalog_parses() {
    let text = ENGLISH;
    let c = Catalog::parse("en", text).unwrap_or_else(|e| panic!("en.ftl: {e:?}"));
    assert!(c.ids().len() > 10);
}

#[test]
fn formats_values_and_plurals() {
    let c = Catalog::parse(
        "en",
        "-brand = textweaver\n\
         hello = Hello, { $name }, from { -brand }.\n\
         count =\n    { $n ->\n        [0] none\n        [one] one { $what }\n       *[other] { $n } { $what }s\n    }\n\
         nested = Say { hello }\n\
         brace = a { \"{\" } b\n\
         multi =\n    line one\n    line two\n",
    )
    .unwrap();
    assert_eq!(
        c.fmt("hello", &args!["name" => "Ada"]),
        "Hello, Ada, from textweaver."
    );
    assert_eq!(c.fmt("count", &args!["n" => 0, "what" => "cat"]), "none");
    assert_eq!(c.fmt("count", &args!["n" => 1, "what" => "cat"]), "one cat");
    assert_eq!(c.fmt("count", &args!["n" => 5, "what" => "cat"]), "5 cats");
    assert_eq!(c.fmt("count", &[]), "{$n} {$what}s");
    assert_eq!(
        c.fmt("nested", &args!["name" => "Ann"]),
        "Say Hello, Ann, from textweaver."
    );
    assert_eq!(c.tr("brace"), "a { b");
    assert_eq!(c.tr("multi"), "line one\nline two");
    assert_eq!(c.tr("missing-id"), "missing-id");
    assert_eq!(
        c.variables("count"),
        ["n", "what"].iter().map(|s| (*s).to_owned()).collect()
    );
}

#[test]
fn reports_errors_with_lines() {
    let errors = Catalog::parse(
        "en",
        "ok = fine\nno equals sign\n9bad = x\nsel = { $n -> [one] x }\nopen = { $x\ndup = a\ndup = b\nattr = x\n    .title = y\n",
    )
    .unwrap_err();
    let lines: Vec<usize> = errors.iter().map(|e| e.line).collect();
    assert_eq!(lines, [2, 3, 4, 5, 7, 9]);
}

#[test]
fn languages_fall_back_to_english() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("it.ftl"), "pos-noun = sostantivo\n").unwrap();
    std::fs::write(dir.path().join("xx.ftl"), "broken = {\n").unwrap();
    let (it, warn) = Catalog::for_language("it", Some(dir.path()));
    assert!(warn.is_none());
    assert_eq!(it.lang(), "it");
    assert_eq!(it.tr("pos-noun"), "sostantivo");
    assert_eq!(it.tr("pos-verb"), "verb");
    let (xx, warn) = Catalog::for_language("xx", Some(dir.path()));
    assert_eq!(xx.lang(), "en");
    assert!(warn.unwrap().contains("problems"));
    let (ja, warn) = Catalog::for_language("ja", None);
    assert_eq!(ja.lang(), "en");
    assert!(warn.unwrap().contains("No translation for ja"));
    assert!(Catalog::for_language("en-GB", None).1.is_none());
    assert!(Catalog::for_language("C", None).1.is_none());
}

#[test]
fn built_in_languages_by_tag_and_region() {
    for l in LANGUAGES {
        let (c, warn) = Catalog::for_language(l.tag, None);
        assert!(warn.is_none(), "{}: {warn:?}", l.tag);
        assert_eq!(c.lang(), l.tag);
    }
    assert_eq!(Catalog::for_language("es-MX", None).0.lang(), "es");
    assert_eq!(Catalog::for_language("pt_BR.UTF-8", None).0.lang(), "pt");
    assert_eq!(Catalog::for_language("fr_CA@euro", None).0.lang(), "fr");
    assert_eq!(normalize_tag(" pt_br.UTF-8 "), "pt-BR");
    assert_eq!(normalize_tag("zh-Hant-tw"), "zh-Hant-TW");
    assert_eq!(normalize_tag("POSIX"), "en");
    assert_eq!(language("de-AT").map(|l| l.tag), Some("de"));
    assert!(language("ja").is_none());
    assert_eq!(
        Catalog::for_language("ar", None).0.direction(),
        Direction::RightToLeft
    );
}

/// A file in the settings folder goes over the built-in translation, which
/// goes over English.
#[test]
fn a_user_file_goes_over_the_built_in_translation() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("fr.ftl"), "pos-noun = NOM\n").unwrap();
    let (fr, warn) = Catalog::for_language("fr", Some(dir.path()));
    assert!(warn.is_none());
    assert_eq!(fr.tr("pos-noun"), "NOM");
    let builtin = Catalog::builtin("fr").unwrap();
    assert_eq!(fr.tr("pos-verb"), builtin.tr("pos-verb"));
    std::fs::write(dir.path().join("fr.ftl"), "broken = {\n").unwrap();
    let (fr, warn) = Catalog::for_language("fr", Some(dir.path()));
    assert_eq!(fr.lang(), "fr");
    assert!(warn.unwrap().contains("the built-in translation"));
}

/// Every built-in translation is valid Fluent in the subset, has only
/// messages English has, uses only the values English's message is given,
/// and chooses variants by keys its language can produce.
#[test]
fn built_in_translations_are_valid() {
    let en = Catalog::english();
    for (tag, text) in BUILTIN {
        let c = Catalog::parse(tag, text).unwrap_or_else(|e| panic!("{tag}.ftl: {e:?}"));
        for id in c.ids() {
            assert!(en.has(id), "{tag}.ftl has {id}, which en.ftl does not");
            // `$unit` is the key of the noun a message names (heading,
            // sentence), which the code passes beside it so a language can
            // make words agree with it; English does not need it.
            let extra: Vec<String> = c
                .variables(id)
                .difference(&en.variables(id))
                .filter(|v| v.as_str() != "unit" || !en_passes_unit(id))
                .cloned()
                .collect();
            assert!(
                extra.is_empty(),
                "{tag}.ftl: {id} uses {extra:?}, which the code does not give it"
            );
        }
        let allowed: BTreeSet<&str> = (0..200).map(|n| plural::category(tag, n).name()).collect();
        for (id, parts) in &c.messages {
            check_keys(tag, id, parts, &allowed);
        }
    }
}

/// Messages the code gives `$unit` though English does not use it: the
/// ones that name a unit or a kind of structure through `$what`.
fn en_passes_unit(id: &str) -> bool {
    matches!(
        id,
        "nav-no-unit" | "nav-nothing-to-read" | "playback-no-unit-here" | "unit-with-level"
    )
}

/// A variant keyed by a plural category the language never produces (a
/// `[few]` in Spanish, a `[zero]` in English: a count of 0 is `[0]`) is
/// never chosen. Keys that are words (`[next]`, `[sentence]`) choose by a
/// text value and are fine.
fn check_keys(tag: &str, id: &str, parts: &[parse::Part], allowed: &BTreeSet<&str>) {
    const CATEGORIES: [&str; 6] = ["zero", "one", "two", "few", "many", "other"];
    for p in parts {
        if let parse::Part::Select { variants, .. } = p {
            for (key, v) in variants {
                assert!(
                    !CATEGORIES.contains(&key.as_str()) || allowed.contains(key.as_str()),
                    "{tag}.ftl: {id} has a variant [{key}] that {tag} never chooses"
                );
                check_keys(tag, id, v, allowed);
            }
        }
    }
}

/// Every built-in translation is complete: a message added to en.ftl needs
/// its translations too (ADR-0030 says how), so no one hears English in
/// the middle of their language.
#[test]
fn built_in_translations_are_complete() {
    let en = Catalog::english();
    for (tag, text) in BUILTIN {
        let c = Catalog::parse(tag, text).unwrap_or_else(|e| panic!("{tag}.ftl: {e:?}"));
        let missing: Vec<&str> = en
            .ids()
            .into_iter()
            .filter(|id| !c.messages.contains_key(*id))
            .collect();
        assert!(
            missing.is_empty(),
            "{tag}.ftl lacks {} messages: {missing:?}",
            missing.len()
        );
    }
}

/// Arabic, a built-in right-to-left language: every message closes the
/// direction marks it opens and isolates every value.
#[test]
fn arabic_isolates_values() {
    let c = Catalog::builtin("ar").unwrap();
    assert_eq!(c.direction(), Direction::RightToLeft);
    let en = Catalog::english();
    for id in en.ids() {
        let vars = en.variables(id);
        let args: Vec<(&str, Arg)> = vars
            .iter()
            .map(|v| (v.as_str(), Arg::Str("\u{e000}".into())))
            .collect();
        let s = c.fmt(id, &args);
        assert!(bidi_problems(&s).is_empty(), "{id}: {s:?}");
    }
}

#[test]
fn pseudo_locale_marks_every_message() {
    let (c, _) = Catalog::for_language(PSEUDO_ACCENTED, None);
    let en = Catalog::english();
    for id in en.ids() {
        let s = c.fmt(id, &[]);
        assert!(s.starts_with('⟦') && s.ends_with('⟧'), "{id}: {s}");
        assert!(
            s.chars().count() > en.fmt(id, &[]).chars().count(),
            "{id} is not longer in the pseudo-locale"
        );
    }
    // Values stay as they are.
    assert_eq!(
        c.fmt("define-pronounced", &args!["say" => "DAWG"]),
        "⟦Pŕöñöûñçéð ~~~~DAWG.⟧"
    );
    assert_eq!(c.direction(), Direction::LeftToRight);
}

#[test]
fn right_to_left_check() {
    assert!(is_rtl("ar") && is_rtl("he-IL") && is_rtl("fa") && is_rtl("ur_PK"));
    assert!(!is_rtl("en") && !is_rtl("es") && !is_rtl(""));
    let (c, _) = Catalog::for_language(PSEUDO_RTL, None);
    assert_eq!(c.direction(), Direction::RightToLeft);
    let en = Catalog::english();
    for id in en.ids() {
        let vars = en.variables(id);
        let args: Vec<(&str, Arg)> = vars
            .iter()
            .map(|v| (v.as_str(), Arg::Str("\u{e000}".into())))
            .collect();
        let s = c.fmt(id, &args);
        assert!(
            bidi_problems(&s).is_empty(),
            "{id}: {:?}",
            bidi_problems(&s)
        );
        // Every value is isolated.
        assert_eq!(
            s.matches('\u{e000}').count(),
            s.matches("\u{2068}\u{e000}\u{2069}").count(),
            "{id} has a value that is not isolated: {s:?}"
        );
    }
    let s = c.fmt("define-pronounced", &args!["say" => "DAWG"]);
    assert_eq!(
        s,
        "\u{202e}Pronounced \u{202c}\u{2068}DAWG\u{2069}\u{202e}.\u{202c}"
    );
    assert!(!bidi_problems("\u{202e}open").is_empty());
    assert!(!bidi_problems("close\u{2069}").is_empty());
    assert!(bidi_problems("\u{2068}\u{202b}x\u{2069}").is_empty());
}

#[test]
fn every_part_of_speech_has_a_name() {
    let en = Catalog::english();
    for p in Pos::ALL {
        assert!(en.has(&format!("pos-{}", p.name())), "{p:?}");
    }
}

/// The English catalog is complete: every message id the workspace's
/// sources ask for (`tr("id")`, `fmt("id", ...)`, `msg("id"...)`) is in
/// en.ftl, and every message in en.ftl is used somewhere.
#[test]
fn english_is_complete() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    rust_files(&root.join("crates"), &mut files);
    let call =
        regex::Regex::new(r#"\b(?:tr|fmt|msg|msg_args)\(\s*"([a-z][a-z0-9]*(?:-[a-z0-9]+)+)""#)
            .unwrap();
    let dynamic = regex::Regex::new(r#""([a-z][a-z0-9]*(?:-[a-z0-9]+)*-)\{"#).unwrap();
    // Ids chosen before the call (`let id = if .. { "a" } else { "b" }`).
    let literal = regex::Regex::new(r#""([a-z][a-z0-9]*(?:-[a-z0-9]+)+)""#).unwrap();
    let mut used = BTreeSet::new();
    let mut literals = BTreeSet::new();
    let mut prefixes = BTreeSet::new();
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        for c in call.captures_iter(&text) {
            used.insert(c[1].to_owned());
        }
        for c in dynamic.captures_iter(&text) {
            prefixes.insert(c[1].to_owned());
        }
        for c in literal.captures_iter(&text) {
            literals.insert(c[1].to_owned());
        }
    }
    let en = Catalog::english();
    let missing: Vec<&String> = used.iter().filter(|id| !en.has(id)).collect();
    assert!(
        missing.is_empty(),
        "missing from locales/en.ftl: {missing:?}"
    );
    let unused: Vec<&str> = en
        .ids()
        .into_iter()
        .filter(|id| {
            !used.contains(*id)
                && !literals.contains(*id)
                && !prefixes.iter().any(|p| id.starts_with(p.as_str()))
        })
        .collect();
    assert!(
        unused.is_empty(),
        "in locales/en.ftl but never used: {unused:?}"
    );
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if p.is_dir() {
            if name != "target" && !name.starts_with('.') {
                rust_files(&p, out);
            }
        } else if name.ends_with(".rs") && name != "tests.rs" {
            out.push(p);
        }
    }
}
