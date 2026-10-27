//! `[live]` and `[live.sources.<id>]`: live documents (beta 5).
//!
//! A live source is a configured MQTT connection whose messages textweaver
//! shows as a document that grows while it is read: a log view (one entry
//! per message) or a values view (one line per field, updated in place).
//!
//! ```toml
//! [live]
//! announce_new_entries = true
//!
//! [live.sources.practice]
//! name = "Practice log"
//! host = "logger.example.ts.net"
//! topics = ["log/checkins"]
//! username = "reader"
//!
//! [[live.sources.practice.fields]]
//! path = "call"
//! label = "Callsign"
//! ```
//!
//! A source's password is never here: it is kept in the system credential
//! store under the account `live:<id>`. Host names, topics and user names
//! belong to this computer and never sync (`crate::sync_scope`).
//!
//! Values that do not fit their type are dropped at load, one by one, with
//! a warning in words; a source that loads but
//! cannot be opened (no host, port 0, a field path that is neither dotted
//! nor a JSON Pointer) says why through [`LiveSourceSettings::problems`].

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Paths;

/// The longest `min_interval_seconds` and `quiet_hint_seconds`: an hour.
pub const LIVE_SECONDS_MAX: u32 = 3600;

/// The most decimals a values-view field may show.
pub const LIVE_DECIMALS_MAX: u8 = 9;

/// The MQTT port with TLS off.
pub const MQTT_PORT: u16 = 1883;

/// The MQTT port with TLS on.
pub const MQTT_TLS_PORT: u16 = 8883;

/// `[live]`: live documents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveSettings {
    /// Where a closed log is saved as JSON. Empty: the `logs` folder in
    /// textweaver's data folder, made on the first save; a relative folder
    /// is inside the data folder ([`LiveSettings::log_folder_in`]).
    pub log_folder: String,
    /// Each new entry is sent, whole, as a polite message, so the screen
    /// reader shows it on the Braille display (and speaks it, unless its
    /// speech is off). On by default.
    pub announce_new_entries: bool,
    /// textweaver's own voice reads each new entry. Off by default: the
    /// reader is often listening to something else.
    pub speak_new_entries: bool,
    /// In the values view, the fewest seconds between two announcements
    /// of one field's changes (10 by default, 0 to 3600).
    pub min_interval_seconds: u32,
    /// After this many seconds with no message on a topic, say so in
    /// words, with a hint (60 by default, 0 never, up to 3600).
    pub quiet_hint_seconds: u32,
    /// The sources, by id (`[live.sources.<id>]`). The id is plain:
    /// letters, digits, `-` and `_` ([`live_source_id_ok`]).
    pub sources: BTreeMap<String, LiveSourceSettings>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for LiveSettings {
    fn default() -> Self {
        LiveSettings {
            log_folder: String::new(),
            announce_new_entries: true,
            speak_new_entries: false,
            min_interval_seconds: 10,
            quiet_hint_seconds: 60,
            sources: BTreeMap::new(),
            extra: toml::Table::new(),
        }
    }
}

impl LiveSettings {
    /// The folder closed logs are saved in: `log_folder`, or the `logs`
    /// folder in the data folder when it is empty. A relative
    /// `log_folder` is inside the data folder, never the working folder,
    /// which changes with how textweaver was started.
    pub fn log_folder_in(&self, paths: &Paths) -> PathBuf {
        let folder = self.log_folder.trim();
        if folder.is_empty() {
            return paths.live_logs_dir();
        }
        let folder = PathBuf::from(folder);
        if folder.is_absolute() {
            folder
        } else {
            paths.data_dir.join(folder)
        }
    }

    /// Every source's problems ([`LiveSourceSettings::problems`]), source
    /// by source in id order.
    pub fn problems(&self) -> Vec<String> {
        self.sources
            .iter()
            .flat_map(|(id, s)| s.problems(id))
            .collect()
    }
}

/// How a live source's connection is encrypted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveTls {
    /// Not encrypted (port 1883). Acceptable over Tailscale, whose
    /// WireGuard link is encrypted already.
    #[default]
    Off,
    /// TLS, trusting textweaver's bundled Mozilla root certificates, not
    /// the operating system's store (port 8883).
    System,
    /// TLS, trusting the certificate authority in `ca_file` (port 8883).
    CaFile,
}

impl LiveTls {
    /// Its value in `settings.toml`.
    pub fn as_str(self) -> &'static str {
        match self {
            LiveTls::Off => "off",
            LiveTls::System => "system",
            LiveTls::CaFile => "ca_file",
        }
    }
}

/// How a live source is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveView {
    /// One entry per message, added at the end (a net log).
    #[default]
    Log,
    /// One line per field, updated in place (sensor readings).
    Values,
}

impl LiveView {
    /// Its value in `settings.toml`.
    pub fn as_str(self) -> &'static str {
        match self {
            LiveView::Log => "log",
            LiveView::Values => "values",
        }
    }
}

/// When a values-view field's change is shown or spoken.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveNotice {
    /// Never.
    #[default]
    Off,
    /// When its value changes, at most once each `min_interval_seconds`.
    OnChange,
}

impl LiveNotice {
    /// Its value in `settings.toml`.
    pub fn as_str(self) -> &'static str {
        match self {
            LiveNotice::Off => "off",
            LiveNotice::OnChange => "on_change",
        }
    }
}

/// `[live.sources.<id>]`: one live source, an MQTT connection. Its
/// password is in the system credential store, never here.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveSourceSettings {
    /// The name shown and said ("Practice log"). Empty: the id.
    pub name: String,
    /// The broker's host name or address.
    pub host: String,
    /// The broker's port. Unset: 1883 with TLS off, 8883 with it on
    /// ([`LiveSourceSettings::port_or_default`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Encryption: `off`, `system`, or `ca_file`.
    pub tls: LiveTls,
    /// The certificate authority file, with `tls = "ca_file"`.
    pub ca_file: String,
    /// The user name to sign in with. Empty: none. The password is asked
    /// once and kept in the system credential store.
    pub username: String,
    /// The topics to subscribe to; MQTT's wildcards `+` and `#` work.
    pub topics: Vec<String>,
    /// `log` or `values`.
    pub view: LiveView,
    /// The field that identifies an entry, so a later message with the
    /// same value corrects it. Empty: `id`, else the callsign.
    pub id_field: String,
    /// In the values view, a value not updated for this many seconds
    /// reads "no recent value" (0 never, at most 3600). Unset: the
    /// `[live] quiet_hint_seconds` time, when a quiet topic is named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_after_seconds: Option<u32>,
    /// How fields are labeled and shown (`[[live.sources.<id>.fields]]`).
    /// Fields not listed are still shown, after these.
    pub fields: Vec<LiveField>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl LiveSourceSettings {
    /// How long a values-view value may go without an update before it
    /// reads "no recent value": `stale_after_seconds`, else
    /// `quiet_hint_seconds` from `[live]`; `None` when that is 0.
    pub fn stale_after(&self, live: &LiveSettings) -> Option<std::time::Duration> {
        let secs = self
            .stale_after_seconds
            .unwrap_or(live.quiet_hint_seconds)
            .min(LIVE_SECONDS_MAX);
        (secs > 0).then(|| std::time::Duration::from_secs(u64::from(secs)))
    }

    /// The port to connect to: the one set, or 1883 with TLS off and
    /// 8883 with TLS on.
    pub fn port_or_default(&self) -> u16 {
        self.port.unwrap_or(match self.tls {
            LiveTls::Off => MQTT_PORT,
            LiveTls::System | LiveTls::CaFile => MQTT_TLS_PORT,
        })
    }

    /// The name shown and said: `name`, or `id` when it is empty.
    pub fn display_name<'a>(&'a self, id: &'a str) -> &'a str {
        let name = self.name.trim();
        if name.is_empty() { id } else { name }
    }

    /// Why the source `id` cannot be opened, each reason a sentence in
    /// words; empty when it can.
    pub fn problems(&self, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        if !live_source_id_ok(id) {
            out.push(format!(
                "The live source id \"{id}\" may hold only letters, digits, hyphens and underscores."
            ));
        }
        let name = self.display_name(id);
        if self.host.trim().is_empty() {
            out.push(format!("Live source {name} has no host."));
        }
        if self.port == Some(0) {
            out.push(format!("Live source {name}: port 0 is outside 1 to 65535."));
        }
        if self.topics.iter().all(|t| t.trim().is_empty()) {
            out.push(format!("Live source {name} has no topic."));
        }
        if self.tls == LiveTls::CaFile && self.ca_file.trim().is_empty() {
            out.push(format!("Live source {name} uses a CA file but names none."));
        }
        for f in &self.fields {
            if field_path_segments(&f.path).is_none() {
                out.push(format!(
                    "Live source {name}: the field path \"{}\" is neither a dotted path nor a JSON Pointer.",
                    f.path
                ));
            }
            if let Some(d) = f.decimals
                && d > LIVE_DECIMALS_MAX
            {
                out.push(format!(
                    "Live source {name}: {d} decimals for {} is more than {LIVE_DECIMALS_MAX}.",
                    f.path
                ));
            }
        }
        out
    }
}

/// `[[live.sources.<id>.fields]]`: how one field of a message is shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveField {
    /// Where the field is in a message: dotted (`sensor.temp`) or a JSON
    /// Pointer (`/sensor/temp`) ([`field_path_segments`]).
    pub path: String,
    /// What it is called ("Temperature"). Empty: the field's own name.
    pub label: String,
    /// The unit said after its value ("C"). Empty: none.
    pub unit: String,
    /// The decimals a number is shown with, 0 to 9. Unset: as sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    /// Words for raw values (`"0" = "closed"`).
    pub values: BTreeMap<String, String>,
    /// When a change is shown (values view): `off` or `on_change`
    /// (the default).
    pub show: LiveNotice,
    /// When a change is spoken in textweaver's own voice (values view):
    /// `off` (the default) or `on_change`.
    pub speak: LiveNotice,
    /// Its place among the fields, smallest first. Unset: the order
    /// listed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for LiveField {
    fn default() -> Self {
        LiveField {
            path: String::new(),
            label: String::new(),
            unit: String::new(),
            decimals: None,
            values: BTreeMap::new(),
            show: LiveNotice::OnChange,
            speak: LiveNotice::Off,
            order: None,
            extra: toml::Table::new(),
        }
    }
}

/// True when `id` can name a live source: 1 to 64 letters, digits, `-`
/// and `_` (it is also the credential store account `live:<id>`).
pub fn live_source_id_ok(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The keys of a field path, or `None` when it is neither dotted nor a
/// JSON Pointer. A path starting with `/` is a JSON Pointer (RFC 6901:
/// `~1` is `/` and `~0` is `~`, any other `~` is wrong); any other path
/// is dotted, with no empty key (`a..b`, `.a`, `a.` are wrong). An empty
/// path is wrong too.
pub fn field_path_segments(path: &str) -> Option<Vec<String>> {
    if path.is_empty() {
        return None;
    }
    if let Some(pointer) = path.strip_prefix('/') {
        return pointer.split('/').map(unescape_pointer_key).collect();
    }
    let keys: Vec<String> = path.split('.').map(str::to_owned).collect();
    keys.iter().all(|k| !k.trim().is_empty()).then_some(keys)
}

/// One JSON Pointer key with `~1` and `~0` undone, or `None` for a `~`
/// followed by anything else.
fn unescape_pointer_key(key: &str) -> Option<String> {
    let mut out = String::with_capacity(key.len());
    let mut chars = key.chars();
    while let Some(c) = chars.next() {
        if c == '~' {
            match chars.next() {
                Some('0') => out.push('~'),
                Some('1') => out.push('/'),
                _ => return None,
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// What is wrong with `value` for the key `key` of a live source or
/// field, in words, when the key is one with a fixed set or range.
fn describe(key: &str, value: &toml::Value) -> Option<String> {
    let shown = match value {
        toml::Value::String(s) => format!("\"{s}\""),
        other => other.to_string(),
    };
    Some(match key {
        "port" => match value {
            toml::Value::Integer(_) => format!("{shown} is outside 1 to 65535; using the default"),
            _ => format!("{shown} is not a port number; using the default"),
        },
        "tls" => format!("{shown} is not off, system or ca_file; using off"),
        "view" => format!("{shown} is not a view, log or values; using log"),
        "show" => format!("{shown} is not off or on_change; using on_change"),
        "speak" => format!("{shown} is not off or on_change; using off"),
        "decimals" => format!("{shown} is outside 0 to {LIVE_DECIMALS_MAX}; using as sent"),
        "order" => format!("{shown} is not a whole number from 0; using the order listed"),
        "stale_after_seconds" => format!(
            "{shown} is not a whole number of seconds from 0; using live.quiet_hint_seconds"
        ),
        _ => return None,
    })
}

/// Deserializes `table` as a `T`, key by key when it does not fit whole:
/// a key whose value does not fit is dropped and reported, in words for
/// the keys [`describe`] knows.
pub(crate) fn lenient<T>(name: &str, table: toml::Table, warnings: &mut Vec<String>) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    if let Ok(v) = toml::Value::Table(table.clone()).try_into::<T>() {
        return v;
    }
    let mut good = toml::Table::new();
    for (key, v) in table {
        let mut candidate = good.clone();
        candidate.insert(key.clone(), v.clone());
        if toml::Value::Table(candidate.clone())
            .try_into::<T>()
            .is_ok()
        {
            good = candidate;
        } else {
            warnings.push(match describe(&key, &v) {
                Some(why) => format!("{name}.{key} {why}"),
                None => format!("{name}.{key} has an invalid value"),
            });
        }
    }
    toml::Value::Table(good).try_into().unwrap_or_default()
}

/// `[live.sources]` source by source and field by field: a value that
/// does not fit is dropped and reported, the rest of the source kept.
pub(crate) fn live_sources_leniently(
    value: Option<toml::Value>,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, LiveSourceSettings> {
    let mut out = BTreeMap::new();
    let Some(value) = value else {
        return out;
    };
    let toml::Value::Table(table) = value else {
        warnings.push("[live.sources] is not a table".to_owned());
        return out;
    };
    for (id, v) in table {
        let name = format!("live.sources.{id}");
        let toml::Value::Table(mut source) = v else {
            warnings.push(format!("[{name}] is not a table"));
            continue;
        };
        let fields = source.remove("fields");
        let mut s: LiveSourceSettings = lenient(&name, source, warnings);
        match fields {
            None => {}
            Some(toml::Value::Array(items)) => {
                for (i, item) in items.into_iter().enumerate() {
                    let entry = format!("{name}.fields entry {}", i + 1);
                    match item {
                        toml::Value::Table(t) => s.fields.push(lenient(&entry, t, warnings)),
                        _ => warnings.push(format!("{entry} is not a table")),
                    }
                }
            }
            Some(_) => warnings.push(format!("{name}.fields is not a list of tables")),
        }
        out.insert(id, s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Settings;
    use crate::sync_scope::{SettingScope, portable_settings, setting_scope};

    const SAMPLE: &str = r#"
[live]
speak_new_entries = true

[live.sources.practice]
name = "Practice log"
host = "logger.example.ts.net"
username = "reader"
topics = ["log/checkins", "log/#"]
id_field = "serial"

[live.sources.weather]
host = "192.0.2.10"
tls = "ca_file"
ca_file = "certs/ca.pem"
view = "values"
topics = ["weather/#"]
stale_after_seconds = 600

[[live.sources.weather.fields]]
path = "/sensor/temp"
label = "Temperature"
unit = "C"
decimals = 1
speak = "on_change"

[[live.sources.weather.fields]]
path = "door"
values = { "0" = "closed", "1" = "open" }
show = "off"
order = 2
"#;

    fn load(text: &str) -> (Settings, Vec<String>) {
        Settings::from_table(text.parse::<toml::Table>().unwrap())
    }

    #[test]
    fn defaults_are_the_owners_choices() {
        let l = Settings::default().live;
        assert_eq!(l.log_folder, "");
        assert!(l.announce_new_entries, "entries reach the Braille display");
        assert!(!l.speak_new_entries, "textweaver's own voice is off");
        assert_eq!(l.min_interval_seconds, 10);
        assert_eq!(l.quiet_hint_seconds, 60);
        assert!(l.sources.is_empty());
        let f = LiveField::default();
        assert_eq!((f.show, f.speak), (LiveNotice::OnChange, LiveNotice::Off));
        let s = LiveSourceSettings::default();
        assert_eq!((s.tls, s.view), (LiveTls::Off, LiveView::Log));
        assert_eq!(s.port_or_default(), 1883);
        let tls = LiveSourceSettings {
            tls: LiveTls::System,
            ..s.clone()
        };
        assert_eq!(tls.port_or_default(), 8883);
        let set = LiveSourceSettings {
            port: Some(1884),
            ..tls
        };
        assert_eq!(set.port_or_default(), 1884);
        let paths = Paths::under(std::path::Path::new("root"));
        assert_eq!(l.log_folder_in(&paths), paths.data_dir.join("logs"));
        let elsewhere = LiveSettings {
            log_folder: "logs-elsewhere".into(),
            ..l
        };
        assert_eq!(
            elsewhere.log_folder_in(&paths),
            paths.data_dir.join("logs-elsewhere"),
            "a relative folder is in the data folder"
        );
        let absolute = std::env::temp_dir().join("net-logs");
        let there = LiveSettings {
            log_folder: absolute.display().to_string(),
            ..elsewhere
        };
        assert_eq!(there.log_folder_in(&paths), absolute);
    }

    #[test]
    fn sources_load_and_round_trip_through_settings_toml() {
        let (s, w) = load(SAMPLE);
        assert!(w.is_empty(), "{w:?}");
        assert!(s.live.speak_new_entries);
        let p = &s.live.sources["practice"];
        assert_eq!(p.display_name("practice"), "Practice log");
        assert_eq!(p.topics, ["log/checkins", "log/#"]);
        assert_eq!(p.port_or_default(), 1883);
        let wx = &s.live.sources["weather"];
        assert_eq!(wx.display_name("weather"), "weather");
        assert_eq!((wx.tls, wx.view), (LiveTls::CaFile, LiveView::Values));
        assert_eq!(wx.port_or_default(), 8883);
        assert_eq!(wx.fields.len(), 2);
        assert_eq!(
            wx.stale_after(&s.live),
            Some(std::time::Duration::from_secs(600))
        );
        assert_eq!(
            p.stale_after(&s.live),
            Some(std::time::Duration::from_secs(60)),
            "unset: the quiet hint time"
        );
        assert_eq!(wx.fields[0].decimals, Some(1));
        assert_eq!(wx.fields[0].speak, LiveNotice::OnChange);
        assert_eq!(wx.fields[1].values["1"], "open");
        assert_eq!(wx.fields[1].show, LiveNotice::Off);
        assert_eq!(wx.fields[1].order, Some(2));
        assert!(s.live.problems().is_empty(), "{:?}", s.live.problems());

        let text = s.to_minimal_toml().unwrap();
        assert!(text.contains("[live.sources.practice]"), "{text}");
        assert!(
            !text.contains("announce_new_entries"),
            "defaults are left out"
        );
        let (back, w) = load(&text);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(back.live, s.live);
        // And through JSON, as `tw settings export` writes it.
        let json = crate::settings_io::settings_to_json(&s).unwrap();
        let again: Settings = serde_json::from_value(serde_json::Value::Object(json)).unwrap();
        assert_eq!(again.live, s.live);
    }

    #[test]
    fn a_bad_value_is_dropped_in_words_and_the_rest_kept() {
        let (s, w) = load(
            r#"
[live]
min_interval_seconds = 99999

[live.sources.net]
host = "broker.example"
port = 70000
view = "grid"
tls = "maybe"
topics = ["a"]

[[live.sources.net.fields]]
path = "call"
show = "always"

[[live.sources.net.fields]]
path = "name"
"#,
        );
        let net = &s.live.sources["net"];
        assert_eq!(net.host, "broker.example", "the rest of the source is kept");
        assert_eq!(net.port, None);
        assert_eq!((net.view, net.tls), (LiveView::Log, LiveTls::Off));
        assert_eq!(net.fields.len(), 2);
        assert_eq!(net.fields[0].show, LiveNotice::OnChange);
        assert_eq!(s.live.min_interval_seconds, LIVE_SECONDS_MAX);
        for expected in [
            "live.sources.net.port 70000 is outside 1 to 65535; using the default",
            r#"live.sources.net.view "grid" is not a view, log or values; using log"#,
            r#"live.sources.net.tls "maybe" is not off, system or ca_file; using off"#,
            r#"live.sources.net.fields entry 1.show "always" is not off or on_change; using on_change"#,
            "live.min_interval_seconds 99999 is outside 0 to 3600 seconds; using 3600",
        ] {
            assert!(w.iter().any(|x| x == expected), "{expected} in {w:#?}");
        }
    }

    #[test]
    fn problems_are_said_in_words() {
        let source = LiveSourceSettings {
            name: "Practice log".into(),
            port: Some(0),
            tls: LiveTls::CaFile,
            fields: vec![
                LiveField {
                    path: "a..b".into(),
                    ..LiveField::default()
                },
                LiveField {
                    path: "/a/~2".into(),
                    decimals: Some(12),
                    ..LiveField::default()
                },
            ],
            ..LiveSourceSettings::default()
        };
        assert_eq!(
            source.problems("practice"),
            [
                "Live source Practice log has no host.",
                "Live source Practice log: port 0 is outside 1 to 65535.",
                "Live source Practice log has no topic.",
                "Live source Practice log uses a CA file but names none.",
                r#"Live source Practice log: the field path "a..b" is neither a dotted path nor a JSON Pointer."#,
                r#"Live source Practice log: the field path "/a/~2" is neither a dotted path nor a JSON Pointer."#,
                "Live source Practice log: 12 decimals for /a/~2 is more than 9.",
            ]
        );
        let ok = LiveSourceSettings {
            host: "h".into(),
            topics: vec!["t".into()],
            ..LiveSourceSettings::default()
        };
        assert!(ok.problems("ok").is_empty());
        assert_eq!(
            ok.problems("has space"),
            [
                r#"The live source id "has space" may hold only letters, digits, hyphens and underscores."#
            ]
        );
    }

    #[test]
    fn field_paths_are_dotted_or_json_pointers() {
        assert_eq!(field_path_segments("call"), Some(vec!["call".to_owned()]));
        assert_eq!(
            field_path_segments("sensor.temp"),
            Some(vec!["sensor".to_owned(), "temp".to_owned()])
        );
        assert_eq!(
            field_path_segments("/a~1b/c~0d/0"),
            Some(vec!["a/b".to_owned(), "c~d".to_owned(), "0".to_owned()])
        );
        for bad in ["", ".a", "a.", "a..b", "/a/~", "/a/~x"] {
            assert_eq!(field_path_segments(bad), None, "{bad}");
        }
        assert!(live_source_id_ok("net-log_2"));
        let long = "x".repeat(65);
        for bad in ["", "a b", "a.b", "live:x", long.as_str()] {
            assert!(!live_source_id_ok(bad), "{bad}");
        }
    }

    #[test]
    fn sources_and_the_log_folder_never_sync() {
        assert_eq!(setting_scope("live.sources"), Some(SettingScope::Machine));
        assert_eq!(
            setting_scope("live.log_folder"),
            Some(SettingScope::Machine)
        );
        assert_eq!(
            setting_scope("live.speak_new_entries"),
            Some(SettingScope::Portable)
        );
        let (s, _) = load(SAMPLE);
        let portable = portable_settings(&s);
        assert!(portable.contains_key("live.speak_new_entries"));
        assert!(!portable.contains_key("live.sources"));
        let all = serde_json::to_string(&portable).unwrap();
        assert!(!all.contains("logger.example"), "{all}");
        assert!(!all.contains("log/checkins"), "{all}");
        assert!(!all.contains("reader"), "{all}");
    }
}
