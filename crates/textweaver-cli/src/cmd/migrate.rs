//! `tw migrate-star`: imports a Star installation's settings, reading
//! positions, bookmarks, notes, highlights, recent files, library, key
//! remaps, and library-folder sidecars into textweaver, and prints a report
//! of everything imported or skipped. Star's own files are only read.
//! Owner: Agent C.

use std::path::{Path, PathBuf};

use anyhow::Context;
use textweaver_app::keymap::{ActionId, Frontend, KeyChord, Keymap, Layer, Platform};
use textweaver_app::store::Paths;
use textweaver_app::store::migrate::{
    self, LoadedDoc, MigrateOptions, MigrationHost, MigrationReport,
};

/// Arguments for `tw migrate-star`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Star configuration directory (default: the platform location).
    #[arg(long)]
    pub from: Option<PathBuf>,
    /// Show what would be imported without writing anything.
    #[arg(long)]
    pub dry_run: bool,
    /// Print the report as JSON.
    #[arg(long)]
    pub json: bool,
    /// Use the files under this folder instead of the usual place.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Loads documents with textweaver's loaders and composes keymap entries
/// from the keymap's defaults.
struct Host;

impl MigrationHost for Host {
    fn load(&self, path: &Path) -> Option<LoadedDoc> {
        let doc = textweaver_app::formats::load_path(path).ok()?;
        Some(LoadedDoc {
            text: doc.text().to_string(),
            title: doc.meta.title.clone(),
            format: doc.meta.format.clone(),
        })
    }

    fn keymap_entry(&self, action: &str, chord: &str) -> Option<Vec<String>> {
        keymap_entry(action, chord)
    }
}

/// The override for binding a Star shortcut `chord` to `action`: the chord
/// in the action's usual layer, plus the action's single-key browse and
/// Speech Cursor defaults, which an override would otherwise remove.
fn keymap_entry(action: &str, chord: &str) -> Option<Vec<String>> {
    let action = ActionId::from_id(action)?;
    let chord: KeyChord = chord.parse().ok()?;
    let mut keys = vec![chord.to_string()];
    let defaults = Keymap::defaults(Platform::current(), Frontend::Terminal);
    for b in defaults.bindings_for(action) {
        if matches!(b.layer, Layer::Browse | Layer::SpeechCursor) {
            let k = format!("{}:{}", b.layer.prefix(), b.chord);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
    }
    Some(keys)
}

/// Runs the migration from `from` into `paths`.
fn migrate(from: &Path, dry_run: bool, paths: &Paths) -> anyhow::Result<MigrationReport> {
    migrate::migrate_star(
        &MigrateOptions {
            from: from.to_owned(),
            dry_run,
        },
        paths,
        &Host,
    )
    .with_context(|| format!("reading Star's settings in {}", from.display()))
}

/// Runs `tw migrate-star`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let from = match args.from {
        Some(f) => f,
        None => migrate::star::default_star_dir()
            .context("no configuration directory is known on this system; use --from DIR")?,
    };
    anyhow::ensure!(
        from.join("settings.json").is_file(),
        "No Star settings found in {}. Use --from with Star's configuration directory.",
        from.display()
    );
    let paths = super::paths(args.home.as_deref())?;
    let report = migrate(&from, args.dry_run, &paths)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", report.render());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use textweaver_app::core::CharPos;
    use textweaver_app::store::migrate::Outcome;
    use textweaver_app::store::migrate::align::{MapMethod, PositionMapper};
    use textweaver_app::store::{DocKey, StateStore};

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let p = std::env::temp_dir()
                .join(format!("tw-migrate-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    /// ADR-0002 on real Star data: every word Star exported for the parity
    /// fixtures maps onto the same word in textweaver's text, or onto the
    /// nearest one when textweaver's text has no such word.
    #[test]
    fn star_word_offsets_map_onto_the_same_words() {
        for name in ["sample.md", "sample.txt", "sample.html"] {
            let json: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(
                    fixtures().join("star-parity").join(format!("{name}.json")),
                )
                .unwrap(),
            )
            .unwrap();
            let star_text = json["plain_text"].as_str().unwrap();
            let doc = Host.load(&fixtures().join(name)).unwrap();
            let tw: Vec<char> = doc.text.chars().collect();
            let mapper = PositionMapper::new(&doc.text, Some(star_text));
            assert!(
                // sample.md: 0.80 (Star speaks the front matter and narrates
                // the table); sample.txt: 1.00; sample.html: 0.98.
                mapper.aligned_share() > 0.75,
                "{name}: {:.2} aligned",
                mapper.aligned_share()
            );
            let mut aligned = 0;
            for t in json["word_tokens"].as_array().unwrap() {
                let (start, word) = (t[0].as_u64().unwrap() as usize, t[2].as_str().unwrap());
                let m = mapper.map_offset(start, None);
                if m.method == MapMethod::Aligned {
                    let got: String = tw[m.pos.0..].iter().take(word.chars().count()).collect();
                    assert_eq!(got, word, "{name} at {start}");
                    aligned += 1;
                } else {
                    assert!(m.pos.0 <= tw.len());
                }
            }
            assert!(aligned > 0, "{name}");
        }
    }

    #[test]
    fn keymap_entries_keep_browse_keys() {
        let e = keymap_entry("next_paragraph", "Alt+N").unwrap();
        assert_eq!(e[0], "Alt+N");
        assert!(e.contains(&"b:p".to_owned()), "{e:?}");
        assert!(e.contains(&"s:PageDown".to_owned()), "{e:?}");
        assert!(keymap_entry("next_sentence", "Nonsense+++").is_none());
        assert!(keymap_entry("no_such_action", "F9").is_none());
    }

    /// A synthetic Star directory: one Markdown document with a saved
    /// position, a dry run, then the real run.
    #[test]
    fn migrates_a_synthetic_star_directory() {
        let dir = TempDir::new("run");
        let star = dir.0.join("star");
        std::fs::create_dir_all(&star).unwrap();
        let doc = dir.0.join("book.md");
        std::fs::write(&doc, "# Book\n\nOne two three.\n\nFour five six.\n").unwrap();
        let doc_s = doc.to_string_lossy().into_owned();
        // Star's plain_text would be "Book\n\nOne two three.\n\nFour five
        // six."; "Four" is at 21. No parse cache: textweaver's text stands
        // in, checked against the saved percentage.
        std::fs::write(
            star.join("settings.json"),
            serde_json::json!({
                "tts_rate": 320,
                "reading_positions": {doc_s.clone(): {"offset": 21, "pct": 60, "ts": "2026-06-27T10:00:00"}},
                "bookmarks": {doc_s.clone(): {"b1": {"offset": 6, "pct": 17, "ts": "2026-06-27T10:00:00"}}},
                "keybindings": {"Alt+.": "Alt+N"},
                "recent_files": [doc_s]
            })
            .to_string(),
        )
        .unwrap();
        let paths = Paths::under(&dir.0.join("tw"));
        let dry = migrate(&star, true, &paths).unwrap();
        assert!(dry.written.is_empty());
        assert!(!paths.config_dir.exists());

        let report = migrate(&star, false, &paths).unwrap();
        assert_eq!(report.count(Outcome::Skipped), 0, "{}", report.render());
        let state = StateStore::new(paths.state_dir())
            .load(&DocKey::for_path(&doc))
            .unwrap();
        let text = Host.load(&doc).unwrap().text;
        let at = |w: &str| CharPos(text[..text.find(w).unwrap()].chars().count());
        assert_eq!(state.position, at("Four"));
        assert_eq!(state.bookmark("b1").unwrap().pos, at("One"));
        let keys = textweaver_app::store::SettingsStore::new(paths.clone())
            .load_keymap()
            .unwrap();
        assert_eq!(keys["next_sentence"][0], "Alt+N");
        let rendered = report.render();
        assert!(
            rendered.contains("Reading positions: 1 imported"),
            "{rendered}"
        );
    }
}
