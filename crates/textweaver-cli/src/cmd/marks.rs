//! `tw marks`: a document's saved reading position, bookmarks, notes,
//! highlights, and synced sidecar position. Reads only; never writes state.
//! With `--export FORMAT`, the notes and highlights as reference records
//! instead: BibTeX, BibLaTeX, RIS, or CSL-JSON (Agent W4g), to the terminal
//! or to `--output FILE`.
//! Owner: Agent C.

use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_app::cite::Format;
use textweaver_app::core::CharPos;
use textweaver_app::store::notes::{Annotation, color_name};
use textweaver_app::store::sync::{self, ProgressEntry, SidecarStore};
use textweaver_app::store::{DocKey, Paths, SettingsStore, StateStore, time};
use textweaver_app::{NotesRecords, export_notes};

/// Arguments for `tw marks`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document whose marks to list.
    pub file: PathBuf,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
    /// Read the state under this directory (like `TEXTWEAVER_HOME`).
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
    /// Write the notes and highlights as reference records: bibtex,
    /// biblatex, ris, or json (CSL-JSON).
    #[arg(long, value_name = "FORMAT", conflicts_with = "json")]
    pub export: Option<String>,
    /// With --export: write to this file instead of the terminal.
    #[arg(long, value_name = "FILE", requires = "export")]
    pub output: Option<PathBuf>,
}

/// One position with where it falls in the document.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct Mark {
    /// Bookmark name; absent for the reading position.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Char offset.
    offset: usize,
    /// Percentage through the document.
    pct: u8,
    /// When it was saved, RFC 3339 UTC.
    #[serde(skip_serializing_if = "Option::is_none")]
    saved: Option<String>,
    /// Line number from 1, when the document could be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    /// The text of that line, shortened, when the document could be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
}

/// The position recorded in a library folder's sidecar.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct SidecarMark {
    folder: PathBuf,
    rel: String,
    #[serde(flatten)]
    mark: Mark,
}

/// A note with where it falls.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct NoteMark {
    id: String,
    note: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    anchor: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    /// End of the noted text (the start is the mark's offset).
    end: usize,
    /// The note as it is spoken.
    spoken: String,
    #[serde(flatten)]
    mark: Mark,
}

/// A highlight with where it falls.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct HighlightMark {
    id: String,
    color: String,
    text: String,
    end: usize,
    spoken: String,
    #[serde(flatten)]
    mark: Mark,
}

/// Everything `tw marks` reports.
#[derive(Debug, Serialize)]
struct Report {
    file: PathBuf,
    key: String,
    position: Option<Mark>,
    bookmarks: Vec<Mark>,
    notes: Vec<NoteMark>,
    highlights: Vec<HighlightMark>,
    history: Vec<usize>,
    sidecar: Option<SidecarMark>,
}

/// Line number and a short context for `offset`, from the loaded document.
fn locate(text: Option<&DocText>, offset: usize) -> (Option<usize>, Option<String>) {
    let Some(text) = text else {
        return (None, None);
    };
    let (line, content) = text.line_at(offset);
    let mut context: String = content.trim().chars().take(60).collect();
    if content.trim().chars().count() > 60 {
        context.push_str("...");
    }
    (Some(line + 1), (!context.is_empty()).then_some(context))
}

/// The loaded document, used only to place marks on lines.
struct DocText(textweaver_app::text::Document);

impl DocText {
    /// The zero-based line holding `offset` and that line's text.
    fn line_at(&self, offset: usize) -> (usize, String) {
        let rope = self.0.text();
        let n = rope.len_chars();
        let line = rope.char_to_line(offset.min(n));
        let content = rope.line(line).to_string();
        (line, content.trim_end_matches(['\n', '\r']).to_owned())
    }
}

fn saved_text(ts: i64) -> Option<String> {
    (ts > 0).then(|| time::rfc3339(ts))
}

fn build(file: &Path, paths: &Paths) -> Report {
    let settings = SettingsStore::new(paths.clone()).load().0;
    let key = DocKey::for_path(file);
    let store = StateStore::new(paths.state_dir());
    let state = store.load(&key).unwrap_or_default();
    let text = textweaver_app::formats::load_path(file).ok().map(DocText);
    let mark = |name: Option<String>, offset: usize, pct: u8, ts: i64| {
        let (line, context) = locate(text.as_ref(), offset);
        Mark {
            name,
            offset,
            pct,
            saved: saved_text(ts),
            line,
            context,
        }
    };
    let position = state
        .has_position()
        .then(|| mark(None, state.position.0, state.pct, state.ts));
    let bookmarks = state
        .sorted_bookmarks()
        .into_iter()
        .map(|b| mark(Some(b.name.clone()), b.pos.0, b.pct, b.ts))
        .collect();
    let len = text.as_ref().map(|t| t.0.len_chars());
    let pct_of = |pos: usize| len.map_or(0, |l| textweaver_app::store::percent(CharPos(pos), l));
    let notes = state
        .notes
        .iter()
        .map(|n| NoteMark {
            id: n.id.clone(),
            note: n.note.clone(),
            anchor: n.anchor.clone(),
            tags: n.tags.clone(),
            end: n.range.end.0,
            spoken: Annotation::Note(n).spoken(),
            mark: mark(None, n.range.start.0, pct_of(n.range.start.0), n.ts),
        })
        .collect();
    let highlights = state
        .highlights
        .iter()
        .map(|h| HighlightMark {
            id: h.id.clone(),
            color: color_name(&h.color),
            text: h.text.clone(),
            end: h.range.end.0,
            spoken: Annotation::Highlight(h).spoken(),
            mark: mark(None, h.range.start.0, pct_of(h.range.start.0), h.ts),
        })
        .collect();
    let sidecar = sync::folder_for(&settings.library.folders, file).and_then(|(folder, rel)| {
        let side = SidecarStore::new(settings.sync.position_policy.conflict_policy());
        let entry = ProgressEntry::from_value(&side.progress_for(&folder, &rel)?)?;
        let (line, context) = locate(text.as_ref(), entry.offset.0);
        Some(SidecarMark {
            folder,
            rel,
            mark: Mark {
                name: None,
                offset: entry.offset.0,
                pct: entry.pct,
                saved: (!entry.ts.is_empty()).then(|| entry.ts.clone()),
                line,
                context,
            },
        })
    });
    Report {
        file: file.to_owned(),
        key: key.0,
        position,
        bookmarks,
        notes,
        highlights,
        history: state.history.iter().map(|p| p.0).collect(),
        sidecar,
    }
}

fn describe(m: &Mark) -> String {
    let mut s = format!("{}% (character {}", m.pct, m.offset);
    if let Some(line) = m.line {
        s.push_str(&format!(", line {line}"));
    }
    s.push(')');
    if let Some(saved) = m
        .saved
        .as_deref()
        .and_then(time::parse_timestamp)
        .map(time::human)
        .or_else(|| m.saved.clone())
    {
        s.push_str(&format!(", saved {saved}"));
    }
    if let Some(c) = &m.context {
        s.push_str(&format!("\n      {c}"));
    }
    s
}

fn render(r: &Report) -> String {
    let mut out = format!("{}\n", r.file.display());
    match &r.position {
        Some(p) => out.push_str(&format!("  Reading position: {}\n", describe(p))),
        None => out.push_str("  No saved reading position.\n"),
    }
    if let Some(s) = &r.sidecar {
        out.push_str(&format!(
            "  Synced position in {} ({}): {}\n",
            s.folder.display(),
            s.rel,
            describe(&s.mark)
        ));
    }
    if r.bookmarks.is_empty() {
        out.push_str("  No bookmarks.\n");
    } else {
        let n = r.bookmarks.len();
        out.push_str(&format!(
            "  {n} bookmark{}:\n",
            if n == 1 { "" } else { "s" }
        ));
        for b in &r.bookmarks {
            out.push_str(&format!(
                "    {}: {}\n",
                b.name.as_deref().unwrap_or(""),
                describe(b)
            ));
        }
    }
    let count =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    if !r.notes.is_empty() {
        out.push_str(&format!("  {}:\n", count(r.notes.len(), "note", "notes")));
        for n in &r.notes {
            out.push_str(&format!("    {}; {}\n", n.spoken, describe(&n.mark)));
        }
    }
    if !r.highlights.is_empty() {
        out.push_str(&format!(
            "  {}:\n",
            count(r.highlights.len(), "highlight", "highlights")
        ));
        for h in &r.highlights {
            out.push_str(&format!("    {}; {}\n", h.spoken, describe(&h.mark)));
        }
    }
    out
}

/// The notes and highlights of `file` as reference records in `format`,
/// and how many there were.
fn export(file: &Path, paths: &Paths, format: Format) -> anyhow::Result<(String, usize)> {
    let state = StateStore::new(paths.state_dir())
        .load(&DocKey::for_path(file))
        .unwrap_or_default();
    let doc = textweaver_app::formats::load_path(file).ok();
    let meta = doc.as_ref().map(|d| &d.meta);
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let title = meta
        .and_then(|m| m.title.clone())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| stem.clone());
    let opts = NotesRecords {
        title: &title,
        author: meta.and_then(|m| m.author.as_deref()),
        key: &stem,
        url: None,
        doc_len: doc.as_ref().map(|d| d.len_chars()),
    };
    let text = export_notes(&state.notes, &state.highlights, &opts, format)?;
    Ok((text, state.notes.len() + state.highlights.len()))
}

/// Runs `tw marks`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let paths = match &args.home {
        Some(home) => Paths::under(home),
        None => Paths::platform()?,
    };
    if let Some(name) = &args.export {
        let Some(format) = Format::from_name(name) else {
            anyhow::bail!("Unknown export format {name}. Use bibtex, biblatex, ris, or json.");
        };
        let (text, n) = export(&args.file, &paths, format)?;
        match &args.output {
            Some(out) => {
                std::fs::write(out, text.as_bytes())
                    .map_err(|e| anyhow::anyhow!("Could not write {}: {e}", out.display()))?;
                let what = if n == 1 {
                    "1 note or highlight".to_owned()
                } else {
                    format!("{n} notes and highlights")
                };
                println!(
                    "Wrote {what} as {} to {}.",
                    format.display_name(),
                    out.display()
                );
            }
            None => super::print_all(&text)?,
        }
        return Ok(());
    }
    let report = build(&args.file, &paths);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render(&report));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use textweaver_app::core::CharRange;
    use textweaver_app::store::DocState;

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let p =
                std::env::temp_dir().join(format!("tw-marks-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reports_position_bookmarks_and_context() {
        let dir = TempDir::new("report");
        let paths = Paths::under(&dir.0);
        let doc = dir.0.join("book.txt");
        std::fs::write(&doc, "first line\nsecond line here\nthird\n").unwrap();
        let mut st = DocState::default();
        st.set_position(CharPos(11), 34);
        st.add_bookmark(Some("intro"), CharPos(0), 34);
        st.add_bookmark(None, CharPos(29), 34);
        st.add_note(CharRange::new(11, 17), "second", "Look here", "#exam");
        st.add_highlight(CharRange::new(28, 33), "yellow", "third");
        StateStore::new(paths.state_dir())
            .save(&DocKey::for_path(&doc), &st)
            .unwrap();

        let r = build(&doc, &paths);
        let p = r.position.as_ref().unwrap();
        assert_eq!((p.offset, p.pct), (11, 32));
        assert!(p.saved.is_some());
        // Line numbers and context come from the loaded document when the
        // loader keeps line structure.
        if let Some(line) = p.line {
            assert_eq!(line, 2);
            assert_eq!(p.context.as_deref(), Some("second line here"));
        }
        let names: Vec<_> = r
            .bookmarks
            .iter()
            .map(|b| b.name.clone().unwrap())
            .collect();
        assert_eq!(names, vec!["intro", "mark1"]);
        let text = render(&r);
        assert!(
            text.contains("Reading position: 32% (character 11"),
            "{text}"
        );
        assert!(text.contains("2 bookmarks:"), "{text}");
        assert!(text.contains("    intro: 0% (character 0"), "{text}");
        assert!(
            text.contains(
                "  1 note:\n    Note: Look here, on \u{201c}second\u{201d}, tagged exam; "
            ),
            "{text}"
        );
        assert!(
            text.contains("  1 highlight:\n    Yellow highlight: third; "),
            "{text}"
        );
        let json: serde_json::Value = serde_json::to_value(&r).unwrap();
        assert_eq!(json["position"]["offset"], 11);
        assert_eq!(json["bookmarks"][1]["name"], "mark1");
        assert_eq!(json["notes"][0]["offset"], 11);
        assert_eq!(json["notes"][0]["end"], 17);
        assert_eq!(json["highlights"][0]["color"], "yellow");
        assert!(json["sidecar"].is_null());
    }

    #[test]
    fn exports_notes_as_reference_records() {
        let dir = TempDir::new("export");
        let paths = Paths::under(&dir.0);
        let doc = dir.0.join("cell biology.txt");
        std::fs::write(&doc, "first line\nsecond line here\nthird\n").unwrap();
        let mut st = DocState::default();
        st.add_note(CharRange::new(11, 17), "second", "Look here", "#exam");
        st.add_highlight(CharRange::new(28, 33), "yellow", "third");
        StateStore::new(paths.state_dir())
            .save(&DocKey::for_path(&doc), &st)
            .unwrap();
        let (bib, n) = export(&doc, &paths, Format::BibTex).unwrap();
        assert_eq!(n, 2);
        assert!(bib.contains("@misc{cell-biology-note-1,"), "{bib}");
        assert!(bib.contains("@misc{cell-biology-highlight-1,"), "{bib}");
        let (ris, _) = export(&doc, &paths, Format::Ris).unwrap();
        assert!(ris.contains("KW  - exam"), "{ris}");
        let (json, _) = export(&doc, &paths, Format::CslJson).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v[0]["note"], "Look here");
        assert_eq!(v[1]["abstract"], "third");
        // Nothing saved: no records, and nothing written.
        let other = dir.0.join("none.txt");
        let (json, n) = export(&other, &paths, Format::CslJson).unwrap();
        assert_eq!((json.trim(), n), ("[]", 0));
    }

    #[test]
    fn reports_nothing_saved_and_never_writes() {
        let dir = TempDir::new("empty");
        let paths = Paths::under(&dir.0);
        let doc = dir.0.join("missing.md");
        let r = build(&doc, &paths);
        assert!(r.position.is_none() && r.bookmarks.is_empty());
        let text = render(&r);
        assert!(text.contains("No saved reading position."));
        assert!(text.contains("No bookmarks."));
        assert!(!paths.config_dir.exists() && !paths.data_dir.exists());
    }

    #[test]
    fn reports_the_sidecar_position() {
        let dir = TempDir::new("sidecar");
        let paths = Paths::under(&dir.0);
        let lib = dir.0.join("lib");
        std::fs::create_dir_all(lib.join("sub")).unwrap();
        let doc = lib.join("sub").join("b.md");
        std::fs::write(&doc, "text").unwrap();
        std::fs::create_dir_all(&paths.config_dir).unwrap();
        let lib_toml = lib.display().to_string().replace('\\', "\\\\");
        std::fs::write(
            paths.settings_file(),
            format!("[library]\nfolders = [\"{lib_toml}\"]\n"),
        )
        .unwrap();
        SidecarStore::default()
            .record_progress(
                &lib,
                "sub/b.md",
                ProgressEntry::new(CharPos(2), 50, 1_790_344_987).to_value(),
                None,
            )
            .unwrap();
        let r = build(&doc, &paths);
        let s = r.sidecar.unwrap();
        assert_eq!(s.rel, "sub/b.md");
        assert_eq!((s.mark.offset, s.mark.pct), (2, 50));
        assert_eq!(s.mark.saved.as_deref(), Some("2026-09-25T14:03:07Z"));
    }
}
