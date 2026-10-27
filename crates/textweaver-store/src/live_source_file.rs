//! Live source files (`.twlive`, beta 5): one live source, to send to
//! someone or keep beside a broker's notes.
//!
//! The file is TOML holding the keys of one `[live.sources.<id>]` table
//! at the top level, with no id (the reader's textweaver makes one from
//! the name):
//!
//! ```toml
//! name = "Practice log"
//! host = "logger.example.ts.net"
//! topics = ["log/checkins"]
//! username = "reader"
//! view = "log"
//!
//! [[fields]]
//! path = "call"
//! label = "Callsign"
//! ```
//!
//! The file comes from someone else, so it is read as untrusted input
//! ([`read_live_source_file`]): at most [`LIVE_SOURCE_FILE_MAX`] bytes;
//! keys textweaver does not know are named and left out; values that do
//! not fit are left out in words, as in `settings.toml`; every string has
//! control, escape and direction characters removed and is cut to
//! [`LIVE_SOURCE_TEXT_MAX`] characters; lists are cut to a few hundred
//! items. A `password` key is refused and never read: a password is only
//! ever kept in the system credential store. A relative `ca_file` is
//! resolved against the file's folder.
//!
//! [`live_source_file_text`] writes a source back as such a file, never
//! with a password and without the keys only this computer knows about.

use std::path::Path;

use crate::live_settings::{LiveField, LiveSourceSettings, LiveTls, LiveView, lenient};

/// The extension of a live source file, without the dot.
pub const LIVE_SOURCE_FILE_EXTENSION: &str = "twlive";

/// The most bytes a live source file may hold: 64 KiB, far more than any
/// real source needs.
pub const LIVE_SOURCE_FILE_MAX: usize = 64 * 1024;

/// The most characters any one string in a live source file keeps.
pub const LIVE_SOURCE_TEXT_MAX: usize = 1024;

/// The most topics a live source file keeps.
pub const LIVE_SOURCE_TOPICS_MAX: usize = 64;

/// The most fields a live source file keeps.
pub const LIVE_SOURCE_FIELDS_MAX: usize = 256;

/// The most value words one field keeps.
pub const LIVE_SOURCE_VALUES_MAX: usize = 256;

/// The keys a live source file may hold at its top level.
const SOURCE_KEYS: &[&str] = &[
    "name",
    "host",
    "port",
    "tls",
    "ca_file",
    "username",
    "topics",
    "view",
    "id_field",
    "stale_after_seconds",
    "fields",
];

/// The keys a field (`[[fields]]`) may hold.
const FIELD_KEYS: &[&str] = &[
    "path", "label", "unit", "decimals", "values", "show", "speak", "order",
];

/// Something about a live source file that the reader should hear: what
/// was left out, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveFileNote {
    /// A key textweaver does not know (dotted for a field's key, as
    /// `fields entry 2.colour`), left out.
    UnknownKey(String),
    /// A `password` key: refused, never read or kept.
    Password,
    /// A value that does not fit its key, left out, in English words as
    /// `settings.toml`'s warnings are ("port \"x\" is not a port number;
    /// using the default").
    BadValue(String),
    /// A list longer than textweaver keeps: the first `max` items were
    /// kept.
    TooMany {
        /// The list's key (`topics`, `fields`, `values`).
        key: String,
        /// How many were kept.
        max: usize,
    },
}

/// A live source read from a file, and what was left out of it.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveSourceFile {
    /// The source, with no password and only the keys textweaver knows.
    pub source: LiveSourceSettings,
    /// What was left out, in the file's order.
    pub notes: Vec<LiveFileNote>,
}

/// Why a live source file could not be read at all.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum LiveFileError {
    /// The file is larger than [`LIVE_SOURCE_FILE_MAX`].
    #[error("the file is {len} bytes; a live source file holds at most {max}")]
    TooLarge {
        /// Its size, in bytes.
        len: usize,
        /// The most allowed.
        max: usize,
    },
    /// The file is not UTF-8 text.
    #[error("the file is not text")]
    NotText,
    /// The file is not TOML; `line` is where reading stopped, from 1.
    #[error("the file is not valid TOML at line {line}")]
    NotToml {
        /// The line, from 1.
        line: usize,
    },
}

/// `s` with control, escape and direction characters removed, white space
/// runs made one space, trimmed, and at most [`LIVE_SOURCE_TEXT_MAX`]
/// characters.
pub fn clean_live_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(LIVE_SOURCE_TEXT_MAX));
    let mut space = false;
    let mut n = 0;
    for c in s.chars() {
        let direction = matches!(
            c,
            '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
                | '\u{061c}'
                | '\u{feff}'
        );
        if direction {
            continue;
        }
        if c.is_whitespace() || c.is_control() {
            space = !out.is_empty();
            continue;
        }
        if n >= LIVE_SOURCE_TEXT_MAX {
            break;
        }
        if space {
            out.push(' ');
            space = false;
            n += 1;
        }
        out.push(c);
        n += 1;
    }
    out
}

/// Every string in `value` cleaned ([`clean_live_text`]), table keys
/// included; tables and arrays are walked, other values kept.
fn clean_value(value: &mut toml::Value) {
    match value {
        toml::Value::String(s) => *s = clean_live_text(s),
        toml::Value::Array(items) => items.iter_mut().for_each(clean_value),
        toml::Value::Table(t) => {
            let old = std::mem::take(t);
            for (k, mut v) in old {
                clean_value(&mut v);
                t.insert(clean_live_text(&k), v);
            }
        }
        _ => {}
    }
}

/// Keeps the first `max` items of the list `key` in `table`, noting it
/// when there were more.
fn cap_list(table: &mut toml::Table, key: &str, max: usize, notes: &mut Vec<LiveFileNote>) {
    if let Some(toml::Value::Array(items)) = table.get_mut(key)
        && items.len() > max
    {
        items.truncate(max);
        notes.push(LiveFileNote::TooMany {
            key: key.to_owned(),
            max,
        });
    }
}

/// Removes from `table` the keys not in `known`, noting each (with
/// `prefix` before its name); a `password` key, anywhere, is refused.
fn keep_known(
    table: &mut toml::Table,
    known: &[&str],
    prefix: &str,
    notes: &mut Vec<LiveFileNote>,
) {
    let unknown: Vec<String> = table
        .keys()
        .filter(|k| !known.contains(&k.as_str()))
        .cloned()
        .collect();
    for key in unknown {
        table.remove(&key);
        if key.eq_ignore_ascii_case("password") {
            if !notes.contains(&LiveFileNote::Password) {
                notes.push(LiveFileNote::Password);
            }
        } else {
            notes.push(LiveFileNote::UnknownKey(format!("{prefix}{key}")));
        }
    }
}

/// The line of byte `offset` in `text`, from 1.
fn line_of(text: &str, offset: usize) -> usize {
    let end = offset.min(text.len());
    let end = (0..=end)
        .rev()
        .find(|i| text.is_char_boundary(*i))
        .unwrap_or(0);
    text[..end].matches('\n').count() + 1
}

/// Reads a live source file's bytes. `folder` is the file's folder: a
/// relative `ca_file` is resolved against it. Never panics, whatever the
/// bytes.
pub fn read_live_source_file(
    bytes: &[u8],
    folder: Option<&Path>,
) -> Result<LiveSourceFile, LiveFileError> {
    if bytes.len() > LIVE_SOURCE_FILE_MAX {
        return Err(LiveFileError::TooLarge {
            len: bytes.len(),
            max: LIVE_SOURCE_FILE_MAX,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| LiveFileError::NotText)?;
    let mut table: toml::Table =
        text.parse()
            .map_err(|e: toml::de::Error| LiveFileError::NotToml {
                line: e.span().map_or(1, |s| line_of(text, s.start)),
            })?;
    let mut notes = Vec::new();
    keep_known(&mut table, SOURCE_KEYS, "", &mut notes);
    // One topic written as a string, not a list, is taken as a list.
    if let Some(toml::Value::String(t)) = table.get("topics") {
        let one = toml::Value::Array(vec![toml::Value::String(t.clone())]);
        table.insert("topics".into(), one);
    }
    cap_list(&mut table, "topics", LIVE_SOURCE_TOPICS_MAX, &mut notes);
    cap_list(&mut table, "fields", LIVE_SOURCE_FIELDS_MAX, &mut notes);
    for (_, v) in table.iter_mut() {
        clean_value(v);
    }
    let fields = table.remove("fields");
    let mut warnings = Vec::new();
    let mut source: LiveSourceSettings = lenient("source", table, &mut warnings);
    source.extra = toml::Table::new();
    match fields {
        None => {}
        Some(toml::Value::Array(items)) => {
            for (i, item) in items.into_iter().enumerate() {
                let entry = format!("fields entry {}", i + 1);
                let toml::Value::Table(mut t) = item else {
                    warnings.push(format!("source.{entry} is not a table"));
                    continue;
                };
                keep_known(&mut t, FIELD_KEYS, &format!("{entry}."), &mut notes);
                if let Some(toml::Value::Table(values)) = t.get_mut("values")
                    && values.len() > LIVE_SOURCE_VALUES_MAX
                {
                    let kept: toml::Table = std::mem::take(values)
                        .into_iter()
                        .take(LIVE_SOURCE_VALUES_MAX)
                        .collect();
                    *values = kept;
                    notes.push(LiveFileNote::TooMany {
                        key: format!("{entry}.values"),
                        max: LIVE_SOURCE_VALUES_MAX,
                    });
                }
                let mut field: LiveField = lenient(&format!("source.{entry}"), t, &mut warnings);
                field.extra = toml::Table::new();
                source.fields.push(field);
            }
        }
        Some(_) => warnings.push("source.fields is not a list of tables".to_owned()),
    }
    notes.extend(warnings.into_iter().map(|w| {
        LiveFileNote::BadValue(w.strip_prefix("source.").map(str::to_owned).unwrap_or(w))
    }));
    source.topics.retain(|t| !t.is_empty());
    let ca = source.ca_file.trim();
    if !ca.is_empty()
        && let Some(folder) = folder
        && Path::new(ca).is_relative()
    {
        source.ca_file = folder.join(ca).display().to_string();
    }
    Ok(LiveSourceFile { source, notes })
}

/// `source` as a live source file: the keys a reader needs, in a fixed
/// order, without empty ones, keys only this computer knows about, or any
/// password (none is ever in the settings). A `ca_file` inside `folder`
/// (the file's own folder) is written relative to it, so the two can be
/// sent together.
pub fn live_source_file_text(source: &LiveSourceSettings, folder: Option<&Path>) -> String {
    let mut t = toml::Table::new();
    let mut put = |key: &str, value: &str| {
        let value = value.trim();
        if !value.is_empty() {
            t.insert(key.to_owned(), toml::Value::String(value.to_owned()));
        }
    };
    put("name", &source.name);
    put("host", &source.host);
    if let Some(port) = source.port {
        t.insert("port".into(), toml::Value::Integer(i64::from(port)));
    }
    if source.tls != LiveTls::Off {
        t.insert("tls".into(), source.tls.as_str().into());
    }
    let ca = source.ca_file.trim();
    if !ca.is_empty() {
        let path = Path::new(ca);
        let relative = folder
            .and_then(|f| path.strip_prefix(f).ok())
            .filter(|r| !r.as_os_str().is_empty())
            .map(|r| r.to_string_lossy().replace('\\', "/"));
        let shown = relative.unwrap_or_else(|| ca.to_owned());
        t.insert("ca_file".into(), shown.into());
    }
    let user = source.username.trim();
    if !user.is_empty() {
        t.insert("username".into(), user.into());
    }
    let topics: Vec<toml::Value> = source
        .topics
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(Into::into)
        .collect();
    t.insert("topics".into(), toml::Value::Array(topics));
    if source.view != LiveView::Log {
        t.insert("view".into(), source.view.as_str().into());
    }
    let id_field = source.id_field.trim();
    if !id_field.is_empty() {
        t.insert("id_field".into(), id_field.into());
    }
    if let Some(s) = source.stale_after_seconds {
        t.insert(
            "stale_after_seconds".into(),
            toml::Value::Integer(i64::from(s)),
        );
    }
    let fields: Vec<toml::Value> = source
        .fields
        .iter()
        .filter_map(|f| {
            let mut f = f.clone();
            f.extra = toml::Table::new();
            let toml::Value::Table(mut ft) = toml::Value::try_from(&f).ok()? else {
                return None;
            };
            for key in ["label", "unit"] {
                if ft.get(key).and_then(toml::Value::as_str) == Some("") {
                    ft.remove(key);
                }
            }
            if ft
                .get("values")
                .and_then(toml::Value::as_table)
                .is_some_and(toml::Table::is_empty)
            {
                ft.remove("values");
            }
            Some(toml::Value::Table(ft))
        })
        .collect();
    if !fields.is_empty() {
        t.insert("fields".into(), toml::Value::Array(fields));
    }
    let body = toml::to_string(&t).unwrap_or_default();
    format!(
        "# A textweaver live source. Open this file in textweaver to add it.\n\
         # It holds no password: textweaver asks for one and keeps it in the\n\
         # system credential store.\n\n{body}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_settings::LiveNotice;

    fn read(text: &str) -> LiveSourceFile {
        read_live_source_file(text.as_bytes(), None).expect("a live source file")
    }

    const FULL: &str = r#"
name = "Practice log"
host = "logger.example.ts.net"
port = 1884
tls = "off"
username = "reader"
topics = ["log/checkins", "log/#"]
view = "values"
id_field = "serial"

[[fields]]
path = "call"
label = "Callsign"

[[fields]]
path = "/sensor/temp"
label = "Temperature"
unit = "C"
decimals = 1
speak = "on_change"
"#;

    #[test]
    fn a_full_file_reads_as_its_source() {
        let f = read(FULL);
        assert!(f.notes.is_empty(), "{:?}", f.notes);
        let s = f.source;
        assert_eq!(s.name, "Practice log");
        assert_eq!(s.host, "logger.example.ts.net");
        assert_eq!(s.port, Some(1884));
        assert_eq!(s.username, "reader");
        assert_eq!(s.topics, ["log/checkins", "log/#"]);
        assert_eq!(s.view, LiveView::Values);
        assert_eq!(s.id_field, "serial");
        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.fields[1].unit, "C");
        assert_eq!(s.fields[1].speak, LiveNotice::OnChange);
        assert!(s.problems("practice").is_empty());
    }

    #[test]
    fn a_password_is_refused_and_never_kept() {
        let f = read(
            "name = \"Practice log\"\nhost = \"192.0.2.1\"\ntopics = [\"a\"]\npassword = \"example only\"\n\
             [[fields]]\npath = \"a\"\nPassword = \"also example\"\n",
        );
        assert_eq!(f.notes, [LiveFileNote::Password]);
        let written = live_source_file_text(&f.source, None);
        assert!(!written.contains("example only"), "{written}");
        assert!(!format!("{:?}", f.source).contains("example"));
    }

    #[test]
    fn unknown_keys_are_named_and_bad_values_said_in_words() {
        let f = read(
            "name = \"X\"\ncolour = \"blue\"\nport = \"high\"\ntopics = \"log/one\"\n\
             [[fields]]\npath = \"a\"\nshade = 3\ndecimals = 300\n",
        );
        assert_eq!(f.source.topics, ["log/one"], "one topic as a string");
        assert!(f.notes.contains(&LiveFileNote::UnknownKey("colour".into())));
        assert!(
            f.notes
                .contains(&LiveFileNote::UnknownKey("fields entry 1.shade".into()))
        );
        let bad: Vec<&String> = f
            .notes
            .iter()
            .filter_map(|n| match n {
                LiveFileNote::BadValue(w) => Some(w),
                _ => None,
            })
            .collect();
        assert!(
            bad.iter()
                .any(|w| w.starts_with("port \"high\" is not a port number")),
            "{bad:?}"
        );
        assert!(
            bad.iter().any(|w| w.starts_with("fields entry 1.decimals")),
            "{bad:?}"
        );
        assert_eq!(f.source.port, None);
        assert!(f.source.extra.is_empty());
        assert!(f.source.fields[0].extra.is_empty());
    }

    #[test]
    fn text_is_cleaned_of_control_and_direction_characters() {
        let f = read(
            "name = \"Practice\\u001b[31m log\\u202e\"\nhost = \" logger.example \"\n\
             topics = [\"log/\\u0007a\", \"\\u200f\"]\nusername = \"r\\u0000eader\"\n",
        );
        assert_eq!(f.source.name, "Practice [31m log");
        assert_eq!(f.source.host, "logger.example");
        assert_eq!(f.source.topics, ["log/ a"], "an empty topic is dropped");
        assert_eq!(f.source.username, "r eader");
        let long = format!("name = \"{}\"\n", "a".repeat(5000));
        assert_eq!(
            read(&long).source.name.chars().count(),
            LIVE_SOURCE_TEXT_MAX
        );
    }

    #[test]
    fn big_broken_and_deep_files_are_refused_or_cut_without_a_panic() {
        let big = vec![b'#'; LIVE_SOURCE_FILE_MAX + 1];
        assert!(matches!(
            read_live_source_file(&big, None),
            Err(LiveFileError::TooLarge { .. })
        ));
        assert_eq!(
            read_live_source_file(&[0xff, 0xfe, 0x00], None),
            Err(LiveFileError::NotText)
        );
        assert_eq!(
            read_live_source_file(b"name = \"ok\"\n\nhost = = 3\n", None),
            Err(LiveFileError::NotToml { line: 3 })
        );
        // Deep nesting within the size cap, on a small stack (the GUI's
        // main thread has 1 MiB on Windows).
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let open = "[".repeat(30_000);
                let close = "]".repeat(30_000);
                let _ = read_live_source_file(format!("x = {open}{close}").as_bytes(), None);
                let open = "{a=".repeat(12_000);
                let close = "}".repeat(12_000);
                let _ = read_live_source_file(format!("x = {open}1{close}").as_bytes(), None);
                let text = format!("[[fields]]\nvalues = {open}1{close}");
                let _ = read_live_source_file(text.as_bytes(), None);
            })
            .unwrap()
            .join()
            .unwrap();
        // Long lists are cut.
        let topics: Vec<String> = (0..500).map(|i| format!("\"t{i}\"")).collect();
        let f = read(&format!("topics = [{}]\n", topics.join(",")));
        assert_eq!(f.source.topics.len(), LIVE_SOURCE_TOPICS_MAX);
        assert!(f.notes.contains(&LiveFileNote::TooMany {
            key: "topics".into(),
            max: LIVE_SOURCE_TOPICS_MAX
        }));
        // Wrong shapes.
        let f = read("fields = 3\n");
        assert!(f.source.fields.is_empty());
        let f = read("fields = [1, \"two\"]\n");
        assert!(f.source.fields.is_empty());
        assert_eq!(f.notes.len(), 2, "{:?}", f.notes);
    }

    #[test]
    fn a_relative_ca_file_is_resolved_against_the_files_folder() {
        let folder = Path::new("shared").join("broker");
        let f = read_live_source_file(b"tls = \"ca_file\"\nca_file = \"ca.pem\"\n", Some(&folder))
            .unwrap();
        assert_eq!(
            f.source.ca_file,
            folder.join("ca.pem").display().to_string()
        );
        assert_eq!(f.source.tls, LiveTls::CaFile);
        // Written back beside it, it is relative again.
        let text = live_source_file_text(&f.source, Some(&folder));
        assert!(text.contains("ca_file = \"ca.pem\""), "{text}");
    }

    /// The fuzz seeds (`fixtures/b5/twlive`): each reads or is refused in
    /// words, a password is never kept, and what reads writes back the
    /// same, as the `twlive` fuzz target checks.
    #[test]
    fn the_seed_files_read_or_are_refused_and_round_trip() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/b5/twlive");
        let mut read_ok = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            let Ok(file) = read_live_source_file(&bytes, None) else {
                continue;
            };
            read_ok += 1;
            let text = live_source_file_text(&file.source, None);
            assert!(!text.contains("not a real password"), "{path:?}");
            let again = read_live_source_file(text.as_bytes(), None).unwrap();
            assert!(again.notes.is_empty(), "{path:?}: {:?}", again.notes);
            assert_eq!(again.source, file.source, "{path:?}");
        }
        assert!(read_ok >= 8, "{read_ok}");
    }

    #[test]
    fn a_source_written_as_a_file_reads_back_the_same() {
        let mut s = read(FULL).source;
        s.extra.insert("local_only".into(), true.into());
        let text = live_source_file_text(&s, None);
        assert!(text.starts_with("# A textweaver live source."), "{text}");
        assert!(!text.contains("local_only"), "{text}");
        assert!(!text.contains("ca_file"), "no empty keys: {text}");
        assert!(!text.to_lowercase().contains("password ="), "{text}");
        let again = read(&text);
        assert!(again.notes.is_empty(), "{:?}", again.notes);
        s.extra.clear();
        assert_eq!(again.source, s);
    }
}
