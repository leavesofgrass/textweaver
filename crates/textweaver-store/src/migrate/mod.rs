//! `tw migrate-star`: importing a Star installation.
//!
//! Reads Star's configuration directory (never writes to it) and imports,
//! into textweaver's own files:
//!
//! - settings with a textweaver equivalent, when they differ from Star's
//!   defaults ([`star::apply_settings`]);
//! - reading positions, bookmarks, notes, and highlights, each mapped onto
//!   textweaver's text by word alignment ([`align`], ADR-0002);
//! - recent files, the bookshelf (`library.json`), and library folders;
//! - GUI keybinding remaps, as `keymap.toml` overrides;
//! - each library folder's `.star/progress.json` sidecar, converted into
//!   `.textweaver/progress.json` and merged with any textweaver sidecar.
//!
//! Every item lands in a [`MigrationReport`] as imported, unchanged, or
//! skipped with a reason. Running the migration twice imports nothing new:
//! existing textweaver data wins over Star's where both exist (a newer
//! textweaver position, a bookmark name already used, a note id already
//! present), and imported notes without a Star id get a stable one.
//!
//! Loading documents and composing keymap entries belong to other crates,
//! so the caller supplies them through [`MigrationHost`].

pub mod align;
pub mod star;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use textweaver_core::{CharPos, CharRange};

use crate::library::{Library, resolve_path};
use crate::notes::{self, Highlight, Note, Relation};
use crate::sync::{self, ConflictPolicy, Prefer, SidecarMap};
use crate::{
    Bookmark, DocKey, DocState, Paths, Recent, RecentEntry, SettingsStore, StateStore, StoreError,
    percent,
};

use self::align::{MapMethod, PositionMapper};
use self::star::{SettingOutcome, StarCache};

/// A document as textweaver loads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedDoc {
    /// The canonical text.
    pub text: String,
    /// The title, when the document has one.
    pub title: Option<String>,
    /// The loader id (`markdown`, `text`, `html`, ...).
    pub format: String,
}

/// What the migration needs from the rest of textweaver.
pub trait MigrationHost {
    /// Loads a document, or `None` when it cannot be opened.
    fn load(&self, path: &Path) -> Option<LoadedDoc>;

    /// The `keymap.toml` value binding `chord` (a Star/Qt shortcut such as
    /// `Ctrl+Shift+P`) to `action`, keeping the action's other default
    /// keys; `None` when the chord cannot be read.
    fn keymap_entry(&self, action: &str, chord: &str) -> Option<Vec<String>>;
}

/// What an item of the report is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// A setting.
    Setting,
    /// A library folder.
    LibraryFolder,
    /// A keybinding remap.
    Keybinding,
    /// A reading position.
    Position,
    /// A bookmark.
    Bookmark,
    /// A note.
    Note,
    /// A highlight.
    Highlight,
    /// A recent file.
    Recent,
    /// A bookshelf entry.
    Library,
    /// A library folder sidecar entry.
    Sidecar,
    /// Anything else (reading statistics, settings with no equivalent).
    Other,
}

impl ItemKind {
    /// Plural name for the summary.
    pub fn plural(self) -> &'static str {
        match self {
            ItemKind::Setting => "Settings",
            ItemKind::LibraryFolder => "Library folders",
            ItemKind::Keybinding => "Keybindings",
            ItemKind::Position => "Reading positions",
            ItemKind::Bookmark => "Bookmarks",
            ItemKind::Note => "Notes",
            ItemKind::Highlight => "Highlights",
            ItemKind::Recent => "Recent files",
            ItemKind::Library => "Bookshelf entries",
            ItemKind::Sidecar => "Synced positions",
            ItemKind::Other => "Other",
        }
    }
}

/// What happened to an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Imported (or, in a dry run, would be).
    Imported,
    /// Already present in textweaver, or equal to the default.
    Unchanged,
    /// Not imported; the detail says why.
    Skipped,
}

/// One line of the report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportItem {
    /// What it is.
    pub kind: ItemKind,
    /// Which one: a settings key, a document path, a bookmark name.
    pub subject: String,
    /// What happened.
    pub outcome: Outcome,
    /// Details or the reason it was skipped.
    pub detail: String,
}

/// Everything the migration did or would do.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReport {
    /// Star's configuration directory.
    pub from: PathBuf,
    /// True when nothing was written.
    pub dry_run: bool,
    /// Documents with Star's own text in its parse cache (exact mapping).
    pub star_cache_documents: usize,
    /// Files written (empty in a dry run).
    pub written: Vec<PathBuf>,
    /// Every item.
    pub items: Vec<ReportItem>,
}

impl MigrationReport {
    fn push(
        &mut self,
        kind: ItemKind,
        subject: impl Into<String>,
        outcome: Outcome,
        detail: impl Into<String>,
    ) {
        self.items.push(ReportItem {
            kind,
            subject: subject.into(),
            outcome,
            detail: detail.into(),
        });
    }

    /// Imported, unchanged, and skipped counts per kind.
    pub fn counts(&self) -> BTreeMap<ItemKind, (usize, usize, usize)> {
        let mut m: BTreeMap<ItemKind, (usize, usize, usize)> = BTreeMap::new();
        for i in &self.items {
            let e = m.entry(i.kind).or_default();
            match i.outcome {
                Outcome::Imported => e.0 += 1,
                Outcome::Unchanged => e.1 += 1,
                Outcome::Skipped => e.2 += 1,
            }
        }
        m
    }

    /// Number of items with `outcome`.
    pub fn count(&self, outcome: Outcome) -> usize {
        self.items.iter().filter(|i| i.outcome == outcome).count()
    }

    /// The report as plain text for the terminal and screen readers: a
    /// summary line per kind, then what was imported, then what was
    /// skipped and why.
    pub fn render(&self) -> String {
        let mut out = format!("Star configuration: {}\n", self.from.display());
        if self.dry_run {
            out.push_str("Dry run: nothing was written.\n");
        } else if self.written.is_empty() {
            out.push_str("Nothing new to write.\n");
        } else {
            let n = self.written.len();
            out.push_str(&format!(
                "Wrote {n} {}.\n",
                if n == 1 { "file" } else { "files" }
            ));
        }
        if self.star_cache_documents > 0 {
            out.push_str(&format!(
                "Star's cached text was available for {} {}.\n",
                self.star_cache_documents,
                if self.star_cache_documents == 1 {
                    "document"
                } else {
                    "documents"
                }
            ));
        }
        out.push_str("\nSummary:\n");
        for (kind, (imp, unch, skip)) in self.counts() {
            let mut parts = vec![format!("{imp} imported")];
            if unch > 0 {
                parts.push(format!("{unch} already present"));
            }
            if skip > 0 {
                parts.push(format!("{skip} skipped"));
            }
            out.push_str(&format!("  {}: {}\n", kind.plural(), parts.join(", ")));
        }
        for (title, outcome) in [
            ("Imported", Outcome::Imported),
            ("Skipped", Outcome::Skipped),
        ] {
            let items: Vec<&ReportItem> =
                self.items.iter().filter(|i| i.outcome == outcome).collect();
            if items.is_empty() {
                continue;
            }
            out.push_str(&format!("\n{title}:\n"));
            for i in items {
                out.push_str(&format!("  {}: {}", i.kind.plural(), i.subject));
                if !i.detail.is_empty() {
                    out.push_str(&format!(": {}", i.detail));
                }
                out.push('\n');
            }
        }
        out
    }
}

/// Migration options.
#[derive(Clone, Debug)]
pub struct MigrateOptions {
    /// Star's configuration directory.
    pub from: PathBuf,
    /// Report without writing anything.
    pub dry_run: bool,
}

/// One loaded document and its mapper, shared by every item of that
/// document.
struct Target {
    chars: Vec<char>,
    mapper: PositionMapper,
}

impl Target {
    fn len(&self) -> usize {
        self.chars.len()
    }

    fn slice(&self, r: CharRange) -> String {
        let r = r.clamp_to(self.len());
        self.chars[r.start.0..r.end.0].iter().collect()
    }
}

struct Run<'a> {
    opts: &'a MigrateOptions,
    paths: &'a Paths,
    host: &'a dyn MigrationHost,
    cache: StarCache,
    targets: HashMap<PathBuf, Option<Target>>,
    report: MigrationReport,
}

/// Why a Star document key cannot be used.
fn unusable(key: &str) -> Option<String> {
    if key.starts_with("http://") || key.starts_with("https://") {
        return Some("a web page, not a file".to_owned());
    }
    let p = Path::new(key);
    if key == "__no_path__" || !p.is_absolute() {
        return Some("not a saved file (an untitled document or the welcome page)".to_owned());
    }
    if !p.is_file() {
        return Some("the file no longer exists".to_owned());
    }
    None
}

fn obj(v: Option<&Value>) -> Option<&Map<String, Value>> {
    v.and_then(Value::as_object)
}

impl Run<'_> {
    fn write(
        &mut self,
        path: &Path,
        f: impl FnOnce() -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        if self.opts.dry_run {
            return Ok(());
        }
        f()?;
        self.report.written.push(path.to_owned());
        Ok(())
    }

    /// The document at `key`, loaded once.
    fn target(&mut self, path: &Path) -> Option<&Target> {
        let key = resolve_path(path);
        if !self.targets.contains_key(&key) {
            let t = self.host.load(path).map(|doc| {
                let star_text = self.cache.plain_text(path);
                Target {
                    mapper: PositionMapper::new(&doc.text, star_text.as_deref()),
                    chars: doc.text.chars().collect(),
                }
            });
            self.targets.insert(key.clone(), t);
        }
        self.targets.get(&key).and_then(Option::as_ref)
    }

    fn settings(&mut self, star: &Map<String, Value>) -> Result<(), StoreError> {
        let store = SettingsStore::new(self.paths.clone());
        let loaded = store.load_detailed();
        if let Some(err) = loaded.error {
            self.report.push(
                ItemKind::Setting,
                "settings.toml",
                Outcome::Skipped,
                format!(
                    "textweaver's own settings file could not be read, so it is left alone: {err}"
                ),
            );
            return Ok(());
        }
        let before = loaded.settings;
        let mut settings = before.clone();
        let (outcomes, unknown) = star::apply_settings(star, &mut settings);
        for (key, o) in outcomes {
            match o {
                SettingOutcome::Imported(d) => {
                    self.report
                        .push(ItemKind::Setting, key, Outcome::Imported, d)
                }
                SettingOutcome::Default => {}
                SettingOutcome::Unchanged => {
                    self.report
                        .push(ItemKind::Setting, key, Outcome::Unchanged, "already set")
                }
                SettingOutcome::Skipped(why) => {
                    self.report
                        .push(ItemKind::Setting, key, Outcome::Skipped, why)
                }
            }
        }
        if !unknown.is_empty() {
            self.report.push(
                ItemKind::Other,
                "settings without a textweaver equivalent",
                Outcome::Skipped,
                unknown.join(", "),
            );
        }
        for folder in star
            .get("library_folders")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|f| !f.is_empty())
        {
            let p = Path::new(folder);
            if !p.is_dir() {
                self.report.push(
                    ItemKind::LibraryFolder,
                    folder,
                    Outcome::Skipped,
                    "the folder no longer exists",
                );
                continue;
            }
            let (stored, added) = settings.library.add_folder(p);
            self.report.push(
                ItemKind::LibraryFolder,
                stored.display().to_string(),
                if added {
                    Outcome::Imported
                } else {
                    Outcome::Unchanged
                },
                "",
            );
        }
        for w in settings.validate() {
            self.report
                .push(ItemKind::Setting, "value adjusted", Outcome::Imported, w);
        }
        if settings != before {
            let file = self.paths.settings_file();
            self.write(&file, || store.save(&settings))?;
        }
        Ok(())
    }

    fn keybindings(&mut self, star: &Map<String, Value>) -> Result<(), StoreError> {
        let Some(remaps) = obj(star.get("keybindings")) else {
            return Ok(());
        };
        if remaps.is_empty() {
            return Ok(());
        }
        let store = SettingsStore::new(self.paths.clone());
        let mut overrides = match store.load_keymap() {
            Ok(o) => o,
            Err(e) => {
                self.report.push(
                    ItemKind::Keybinding,
                    "keymap.toml",
                    Outcome::Skipped,
                    format!("textweaver's keymap file could not be read, so it is left alone: {e}"),
                );
                return Ok(());
            }
        };
        let before = overrides.clone();
        for (shortcut, custom) in remaps {
            let custom = custom.as_str().unwrap_or("").trim();
            let subject = format!(
                "{shortcut} to {}",
                if custom.is_empty() { "nothing" } else { custom }
            );
            let Some(action) = star::action_for_star_shortcut(shortcut) else {
                self.report.push(
                    ItemKind::Keybinding,
                    subject,
                    Outcome::Skipped,
                    format!("textweaver has no command for Star's {shortcut}"),
                );
                continue;
            };
            if overrides.contains_key(action) {
                self.report.push(
                    ItemKind::Keybinding,
                    subject,
                    Outcome::Unchanged,
                    format!("{action} already has keys in keymap.toml"),
                );
                continue;
            }
            let entry = if custom.is_empty() {
                Some(Vec::new())
            } else {
                self.host.keymap_entry(action, custom)
            };
            match entry {
                Some(keys) => {
                    self.report.push(
                        ItemKind::Keybinding,
                        subject,
                        Outcome::Imported,
                        format!(
                            "{action} = [{}]",
                            keys.iter()
                                .map(|k| format!("\"{k}\""))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                    overrides.insert(action.to_owned(), keys);
                }
                None => self.report.push(
                    ItemKind::Keybinding,
                    subject,
                    Outcome::Skipped,
                    format!("textweaver cannot read the key {custom}"),
                ),
            }
        }
        if overrides != before {
            let file = self.paths.keymap_file();
            self.write(&file, || store.save_keymap(&overrides))?;
        }
        Ok(())
    }

    fn documents(&mut self, star: &Map<String, Value>) -> Result<(), StoreError> {
        // Every document key any per-document store mentions, in a stable
        // order.
        let mut keys: Vec<String> = Vec::new();
        for store in [
            "reading_positions",
            "bookmarks",
            "annotations",
            "user_highlights",
        ] {
            if let Some(m) = obj(star.get(store)) {
                for k in m.keys() {
                    if !keys.contains(k) {
                        keys.push(k.clone());
                    }
                }
            }
        }
        let state_store =
            StateStore::with_debounce(self.paths.state_dir(), std::time::Duration::ZERO);
        let mut states: BTreeMap<DocKey, (PathBuf, DocState, DocState)> = BTreeMap::new();
        for key in keys {
            let position = obj(star.get("reading_positions")).and_then(|m| m.get(&key));
            let bookmarks = obj(star.get("bookmarks")).and_then(|m| obj(m.get(&key)));
            let annotations = obj(star.get("annotations"))
                .and_then(|m| m.get(&key))
                .and_then(Value::as_array);
            let highlights = obj(star.get("user_highlights"))
                .and_then(|m| m.get(&key))
                .and_then(Value::as_array);
            if let Some(why) = unusable(&key) {
                let n = usize::from(position.is_some())
                    + bookmarks.map_or(0, Map::len)
                    + annotations.map_or(0, Vec::len)
                    + highlights.map_or(0, Vec::len);
                if n > 0 {
                    self.report.push(
                        ItemKind::Other,
                        key.clone(),
                        Outcome::Skipped,
                        format!("{n} saved items not imported: {why}"),
                    );
                }
                continue;
            }
            let path = PathBuf::from(&key);
            if self.target(&path).is_none() {
                self.report.push(
                    ItemKind::Other,
                    key.clone(),
                    Outcome::Skipped,
                    "textweaver could not open this document",
                );
                continue;
            }
            let dk = DocKey::for_path(&path);
            let entry = states.entry(dk.clone()).or_insert_with(|| {
                let st = state_store.load(&dk).unwrap_or_default();
                (path.clone(), st.clone(), st)
            });
            let mut state = std::mem::take(&mut entry.1);
            let Some(target) = self.target(&path) else {
                continue;
            };
            let mut items = Vec::new();
            if let Some(p) = position {
                import_position(&key, p, target, &mut state, &mut items);
            }
            if let Some(b) = bookmarks {
                import_bookmarks(&key, b, target, &mut state, &mut items);
            }
            if let Some(a) = annotations {
                import_notes(&key, a, target, &mut state, &mut items);
            }
            if let Some(h) = highlights {
                import_highlights(&key, h, target, &mut state, &mut items);
            }
            self.report.items.extend(items);
            if let Some(e) = states.get_mut(&dk) {
                e.1 = state;
            }
        }
        for (dk, (_, state, before)) in states {
            if state != before {
                let file = state_store.dir().join(format!("{}.json", dk.as_str()));
                self.write(&file, || state_store.save(&dk, &state))?;
            }
        }
        Ok(())
    }

    fn recents(&mut self, star: &Map<String, Value>) -> Result<(), StoreError> {
        let mut candidates: Vec<String> = Vec::new();
        if let Some(last) = star
            .get("last_path")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            candidates.push(last.to_owned());
        }
        for p in star
            .get("recent_files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !candidates.iter().any(|c| c == p) {
                candidates.push(p.to_owned());
            }
        }
        if candidates.is_empty() {
            return Ok(());
        }
        let limit = SettingsStore::new(self.paths.clone())
            .load()
            .0
            .library
            .recent_limit;
        let file = self.paths.recent_file();
        let mut recent = Recent::load(&file);
        let before = recent.clone();
        for c in candidates {
            if let Some(why) = unusable(&c) {
                self.report.push(ItemKind::Recent, c, Outcome::Skipped, why);
                continue;
            }
            let p = PathBuf::from(&c);
            if recent
                .entries
                .iter()
                .any(|e| resolve_path(&e.path) == resolve_path(&p))
            {
                self.report
                    .push(ItemKind::Recent, c, Outcome::Unchanged, "");
                continue;
            }
            if recent.entries.len() >= limit {
                self.report.push(
                    ItemKind::Recent,
                    c,
                    Outcome::Skipped,
                    format!("the recent list is full ({limit})"),
                );
                continue;
            }
            recent.entries.push(RecentEntry {
                path: p,
                title: None,
                opened: 0,
            });
            self.report.push(ItemKind::Recent, c, Outcome::Imported, "");
        }
        if recent != before {
            self.write(&file, || recent.save(&file))?;
        }
        Ok(())
    }

    fn bookshelf(&mut self, star: &Map<String, Value>) -> Result<(), StoreError> {
        let Some(entries) = obj(star.get("library")) else {
            return Ok(());
        };
        let file = self.paths.library_file();
        let mut lib = match Library::load(&file) {
            Ok(l) => l,
            Err(e) => {
                self.report.push(
                    ItemKind::Library,
                    "library.json",
                    Outcome::Skipped,
                    format!("textweaver's bookshelf could not be read, so it is left alone: {e}"),
                );
                return Ok(());
            }
        };
        let before = lib.clone();
        for (path, e) in entries {
            if let Some(why) = unusable(path) {
                self.report
                    .push(ItemKind::Library, path.clone(), Outcome::Skipped, why);
                continue;
            }
            let p = Path::new(path);
            if lib.get(p).is_some() {
                self.report
                    .push(ItemKind::Library, path.clone(), Outcome::Unchanged, "");
                continue;
            }
            let title = e.get("title").and_then(Value::as_str).unwrap_or("");
            let format = e.get("format").and_then(Value::as_str).unwrap_or("");
            let last = star::star_ts(e.get("last_opened"));
            let added = star::star_ts(e.get("added"));
            lib.record_open_at(p, title, format, last);
            let key = resolve_path(p);
            if let Some(entry) = lib.entries.iter_mut().find(|x| x.path == key)
                && added > 0
            {
                entry.added = added;
            }
            self.report.push(
                ItemKind::Library,
                path.clone(),
                Outcome::Imported,
                title.to_owned(),
            );
        }
        if lib != before {
            self.write(&file, || lib.save(&file))?;
        }
        Ok(())
    }

    fn sidecars(&mut self, star: &Map<String, Value>) -> Result<(), StoreError> {
        let folders: Vec<PathBuf> = star
            .get("library_folders")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .collect();
        for folder in folders {
            let star_file = folder.join(".star").join("progress.json");
            let Some(star_side) = std::fs::read_to_string(&star_file)
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            else {
                continue;
            };
            let star_map = SidecarMap::from_value(&star_side);
            let existing = sync::read_sidecar(&folder);
            let mut converted = SidecarMap::new();
            for (rel, entry) in star_map.iter() {
                if rel == sync::META_KEY {
                    converted.insert(rel, entry.clone());
                    continue;
                }
                let subject = format!("{rel} in {}", folder.display());
                let doc = folder.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
                let Some(m) = entry.as_object() else {
                    self.report.push(
                        ItemKind::Sidecar,
                        subject,
                        Outcome::Skipped,
                        "not a position entry",
                    );
                    continue;
                };
                let ts = star::star_ts(m.get("ts"));
                let pct =
                    star::as_usize(m.get("pct")).map(|p| u8::try_from(p.min(100)).unwrap_or(100));
                let offset = star::as_usize(m.get("offset"));
                if !doc.is_file() {
                    self.report.push(
                        ItemKind::Sidecar,
                        subject,
                        Outcome::Skipped,
                        "the file no longer exists",
                    );
                    continue;
                }
                let Some(target) = self.target(&doc) else {
                    self.report.push(
                        ItemKind::Sidecar,
                        subject,
                        Outcome::Skipped,
                        "textweaver could not open this document",
                    );
                    continue;
                };
                let mapped = match (offset, pct) {
                    (Some(o), _) => target.mapper.map_offset(o, pct),
                    // Only a percentage was saved.
                    (None, Some(p)) => target.mapper.map_percentage(p),
                    (None, None) => {
                        self.report.push(
                            ItemKind::Sidecar,
                            subject,
                            Outcome::Skipped,
                            "no saved position",
                        );
                        continue;
                    }
                };
                let new_pct = percent(mapped.pos, target.len());
                let mut e = sync::ProgressEntry::new(mapped.pos, new_pct, ts).to_value();
                if ts == 0
                    && let Some(obj) = e.as_object_mut()
                {
                    obj.insert("ts".to_owned(), Value::String(String::new()));
                }
                let (outcome, detail) = match existing.get(rel) {
                    Some(x) if sync::json_eq(x, &e) => {
                        (Outcome::Unchanged, "already imported".to_owned())
                    }
                    Some(x) => {
                        let (_, c) = sync::resolve_entry(
                            rel,
                            Some(x),
                            Some(&e),
                            ConflictPolicy::Newest,
                            Prefer::Local,
                        );
                        match c.map(|c| c.resolution) {
                            Some(sync::Resolution::Local) => (
                                Outcome::Unchanged,
                                "textweaver's synced position is newer; kept it".to_owned(),
                            ),
                            _ => (Outcome::Imported, describe_mapped(new_pct, mapped.method)),
                        }
                    }
                    None => (Outcome::Imported, describe_mapped(new_pct, mapped.method)),
                };
                converted.insert(rel, e);
                self.report
                    .push(ItemKind::Sidecar, subject, outcome, detail);
            }
            if converted.is_empty() {
                continue;
            }
            let (merged, _) =
                sync::merge_maps(&existing, &converted, ConflictPolicy::Newest, Prefer::Local);
            if merged != existing {
                let file = sync::sidecar_file(&folder);
                self.write(&file, || sync::write_sidecar(&folder, &merged))?;
            }
        }
        Ok(())
    }
}

fn describe_mapped(pct: u8, method: MapMethod) -> String {
    format!("{pct} percent, {}", method.describe())
}

fn import_position(
    key: &str,
    p: &Value,
    t: &Target,
    state: &mut DocState,
    items: &mut Vec<ReportItem>,
) {
    let item = |outcome, detail: String| ReportItem {
        kind: ItemKind::Position,
        subject: key.to_owned(),
        outcome,
        detail,
    };
    let Some(m) = p.as_object() else {
        items.push(item(Outcome::Skipped, "not a position entry".to_owned()));
        return;
    };
    let pct = star::as_usize(m.get("pct")).map(|p| u8::try_from(p.min(100)).unwrap_or(100));
    let ts = star::star_ts(m.get("ts")).max(1);
    let Some(offset) = star::as_usize(m.get("offset")) else {
        items.push(item(Outcome::Skipped, "no saved offset".to_owned()));
        return;
    };
    if state.has_position() && state.ts >= ts {
        items.push(item(
            Outcome::Unchanged,
            "textweaver's saved position is newer".to_owned(),
        ));
        return;
    }
    let mapped = t.mapper.map_offset(offset, pct);
    state.position = mapped.pos;
    state.pct = percent(mapped.pos, t.len());
    state.ts = ts;
    items.push(item(
        Outcome::Imported,
        describe_mapped(state.pct, mapped.method),
    ));
}

fn import_bookmarks(
    key: &str,
    b: &Map<String, Value>,
    t: &Target,
    state: &mut DocState,
    items: &mut Vec<ReportItem>,
) {
    for (name, v) in b {
        let subject = format!("{name} in {key}");
        let item = |outcome, detail: String| ReportItem {
            kind: ItemKind::Bookmark,
            subject: subject.clone(),
            outcome,
            detail,
        };
        if state.bookmark(name).is_some() {
            items.push(item(
                Outcome::Unchanged,
                "a bookmark with this name exists".to_owned(),
            ));
            continue;
        }
        let Some(offset) = star::as_usize(v.get("offset")) else {
            items.push(item(Outcome::Skipped, "no saved offset".to_owned()));
            continue;
        };
        let pct = star::as_usize(v.get("pct")).map(|p| u8::try_from(p.min(100)).unwrap_or(100));
        let mapped = t.mapper.map_offset(offset, pct);
        let bm = Bookmark {
            name: name.clone(),
            pos: mapped.pos,
            pct: percent(mapped.pos, t.len()),
            ts: star::star_ts(v.get("ts")),
        };
        items.push(item(
            Outcome::Imported,
            describe_mapped(bm.pct, mapped.method),
        ));
        let at = state.bookmarks.partition_point(|x| x.pos <= bm.pos);
        state.bookmarks.insert(at, bm);
    }
}

/// Star annotation fields textweaver stores in named fields; the others
/// (`sr_state`, ...) are kept in [`Note::extra`].
const NOTE_FIELDS: [&str; 10] = [
    "id",
    "char_pos",
    "word_idx",
    "anchor",
    "note",
    "tags",
    "cite",
    "ts",
    "relations",
    "color",
];

fn import_notes(
    key: &str,
    list: &[Value],
    t: &Target,
    state: &mut DocState,
    items: &mut Vec<ReportItem>,
) {
    for a in list {
        let Some(m) = a.as_object() else {
            items.push(ReportItem {
                kind: ItemKind::Note,
                subject: key.to_owned(),
                outcome: Outcome::Skipped,
                detail: "not a note".to_owned(),
            });
            continue;
        };
        let text = m
            .get("note")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_owned();
        let anchor = m.get("anchor").and_then(Value::as_str).unwrap_or("");
        let ts = star::star_ts(m.get("ts"));
        let word_idx = star::as_usize(m.get("word_idx"));
        let char_pos = star::as_usize(m.get("char_pos")).filter(|p| *p > 0);
        let id = sync::ann_id(a).unwrap_or_else(|| {
            notes::stable_id(
                "star-",
                &[
                    anchor,
                    &text,
                    &word_idx.map(|w| w.to_string()).unwrap_or_default(),
                    m.get("ts").and_then(Value::as_str).unwrap_or(""),
                ],
            )
        });
        let short: String = notes::collapse(if text.is_empty() { anchor } else { &text }, 40);
        let subject = format!("\u{201c}{short}\u{201d} in {key}");
        if state.note(&id).is_some() {
            items.push(ReportItem {
                kind: ItemKind::Note,
                subject,
                outcome: Outcome::Unchanged,
                detail: "already imported".to_owned(),
            });
            continue;
        }
        // Star's word index is exact where Star's text is known; the GUI's
        // char position is a rendered-editor offset, close to it.
        let estimate = match (word_idx, char_pos) {
            (Some(w), _) => t.mapper.map_word(w, None),
            (None, Some(c)) => t.mapper.map_offset(c, None),
            (None, None) => align::Mapped {
                pos: CharPos(0),
                method: MapMethod::Percentage,
            },
        };
        let (range, method) = match t.mapper.find_anchor(anchor, estimate.pos) {
            Some(r) => (r, MapMethod::Anchor),
            None => (CharRange::empty(estimate.pos), estimate.method),
        };
        let tags = match m.get("tags") {
            Some(Value::Array(v)) => v
                .iter()
                .filter_map(|t| t.as_str().map(|s| s.trim_start_matches('#').to_owned()))
                .filter(|s| !s.is_empty())
                .collect(),
            Some(Value::String(s)) => notes::parse_tags(s),
            _ => Vec::new(),
        };
        let relations = m
            .get("relations")
            .and_then(Value::as_array)
            .map(|v| {
                v.iter()
                    .filter_map(|r| serde_json::from_value::<Relation>(r.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        let extra: Map<String, Value> = m
            .iter()
            .filter(|(k, _)| !NOTE_FIELDS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let note = Note {
            id,
            range,
            anchor: notes::collapse(anchor, notes::ANCHOR_MAX_CHARS),
            note: text,
            tags,
            cite: m
                .get("cite")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            color: m.get("color").and_then(Value::as_str).map(str::to_owned),
            relations,
            created: ts,
            ts,
            extra,
        };
        items.push(ReportItem {
            kind: ItemKind::Note,
            subject,
            outcome: Outcome::Imported,
            detail: describe_mapped(percent(range.start, t.len()), method),
        });
        state.insert_note(note);
    }
}

fn import_highlights(
    key: &str,
    list: &[Value],
    t: &Target,
    state: &mut DocState,
    items: &mut Vec<ReportItem>,
) {
    for h in list {
        let item = |outcome, detail: String| ReportItem {
            kind: ItemKind::Highlight,
            subject: key.to_owned(),
            outcome,
            detail,
        };
        let (Some(start), Some(end)) =
            (star::as_usize(h.get("start")), star::as_usize(h.get("end")))
        else {
            items.push(item(Outcome::Skipped, "not a highlight".to_owned()));
            continue;
        };
        if start >= end {
            items.push(item(Outcome::Skipped, "an empty highlight".to_owned()));
            continue;
        }
        let color = h
            .get("color")
            .and_then(Value::as_str)
            .filter(|c| !c.is_empty())
            .unwrap_or(notes::DEFAULT_HIGHLIGHT_COLOR)
            .to_owned();
        let id = notes::stable_id("star-h-", &[&start.to_string(), &end.to_string(), &color]);
        if state.highlight(&id).is_some() {
            items.push(item(Outcome::Unchanged, "already imported".to_owned()));
            continue;
        }
        let s = t.mapper.map_offset(start, None);
        let e = t.mapper.map_end(end, None);
        if e <= s.pos {
            items.push(item(
                Outcome::Skipped,
                "its text could not be found".to_owned(),
            ));
            continue;
        }
        let range = CharRange::new(s.pos, e);
        let text = t.slice(range);
        let pct = percent(range.start, t.len());
        items.push(item(
            Outcome::Imported,
            format!(
                "{} highlight at {pct} percent, \u{201c}{}\u{201d}, approximate: Star stored screen offsets",
                notes::color_name(&color),
                notes::collapse(&text, 40)
            ),
        ));
        state.insert_highlight(Highlight {
            id,
            range,
            color,
            text: notes::collapse(&text, notes::HIGHLIGHT_TEXT_MAX_CHARS),
            ts: 0,
            extra: Map::new(),
        });
    }
}

/// Imports a Star installation into textweaver's files under `paths`
/// (see the module docs). Star's directory is only read. Fails when Star's
/// `settings.json` cannot be read; problems with single items are reported
/// in the [`MigrationReport`] instead.
pub fn migrate_star(
    opts: &MigrateOptions,
    paths: &Paths,
    host: &dyn MigrationHost,
) -> Result<MigrationReport, StoreError> {
    let star = star::read_settings(&opts.from)?;
    let cache = StarCache::index(&opts.from);
    let mut run = Run {
        opts,
        paths,
        host,
        cache,
        targets: HashMap::new(),
        report: MigrationReport {
            from: opts.from.clone(),
            dry_run: opts.dry_run,
            ..MigrationReport::default()
        },
    };
    run.settings(&star)?;
    run.keybindings(&star)?;
    run.documents(&star)?;
    run.recents(&star)?;
    run.bookshelf(&star)?;
    run.sidecars(&star)?;
    if let Some(stats) = obj(star.get("reading_stats")).filter(|m| !m.is_empty()) {
        run.report.push(
            ItemKind::Other,
            "reading statistics",
            Outcome::Skipped,
            format!(
                "{} reading time and progress; textweaver does not keep reading statistics yet",
                if stats.len() == 1 {
                    "1 document's".to_owned()
                } else {
                    format!("{} documents'", stats.len())
                }
            ),
        );
    }
    if let Some(presets) = obj(star.get("annotation_filter_presets")).filter(|m| !m.is_empty()) {
        run.report.push(
            ItemKind::Other,
            "note filter presets",
            Outcome::Skipped,
            format!(
                "{} saved note searches; textweaver has no saved searches yet",
                presets.len()
            ),
        );
    }
    run.report.star_cache_documents = run
        .targets
        .values()
        .flatten()
        .filter(|t| t.mapper.exact())
        .count();
    Ok(run.report)
}

#[cfg(test)]
mod tests;
