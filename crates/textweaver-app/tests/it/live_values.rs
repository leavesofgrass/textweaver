//! Live documents, beta 5 phase 2: the values view, message key order in
//! both views, a wrapper's heading line, and the log folder setting.

use std::path::Path;
use std::time::{Duration, Instant};

use textweaver_app::a11y::{LogAnnouncer, Shared};
use textweaver_app::core::CharPos;
use textweaver_app::live::{LiveOptions, LiveSource, ScriptHandle, ScriptedSource};
use textweaver_app::store::{LiveField, LiveNotice, LiveSourceSettings, LiveView, Paths, Settings};
use textweaver_app::{App, AppConfig, Command};

struct Rig {
    app: App,
    said: Shared<LogAnnouncer>,
    feed: ScriptHandle,
    start: Instant,
}

/// A weather source shown in the values view, mapped as a user would.
fn weather(settings: &mut Settings) {
    settings.live.sources.insert(
        "wx".into(),
        LiveSourceSettings {
            name: "Weather".into(),
            host: "broker.example".into(),
            topics: vec!["station/wx".into()],
            view: LiveView::Values,
            stale_after_seconds: Some(120),
            fields: vec![LiveField {
                path: "temp_c".into(),
                label: "Temperature".into(),
                unit: "C".into(),
                decimals: Some(1),
                ..LiveField::default()
            }],
            ..LiveSourceSettings::default()
        },
    );
}

fn rig_with(paths: Option<Paths>, id: &str, settings: impl FnOnce(&mut Settings)) -> Rig {
    let said = Shared::new(LogAnnouncer::default());
    let mut config = AppConfig {
        announcer: Box::new(said.clone()),
        paths,
        ..AppConfig::for_tests()
    };
    settings(&mut config.settings);
    let mut app = App::new(config);
    let (source, feed) = ScriptedSource::new();
    let source: Box<dyn LiveSource> = Box::new(source);
    app.open_live_source(
        source,
        LiveOptions {
            id: id.into(),
            title: String::new(),
            ..LiveOptions::default()
        },
    );
    Rig {
        app,
        said,
        feed,
        start: Instant::now(),
    }
}

impl Rig {
    fn send_at(&mut self, topic: &str, json: &str, secs: u64) {
        self.feed.push_json(topic, json);
        let _ = self.app.tick(self.start + Duration::from_secs(secs));
    }

    fn text(&self) -> String {
        self.app.session().unwrap().doc.text().to_string()
    }

    fn at(&self, needle: &str) -> CharPos {
        let t = self.text();
        let b = t
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} in {t:?}"));
        CharPos(t[..b].chars().count())
    }

    fn go(&mut self, needle: &str) {
        let p = self.at(needle);
        let _ = self.app.dispatch(Command::SetCursor(p));
    }

    fn word(&self) -> String {
        let s = self.app.session().unwrap();
        let end = (s.cursor.0 + 8).min(s.doc.len_chars());
        s.doc
            .slice(textweaver_app::core::CharRange::new(s.cursor.0, end))
    }

    fn last_said(&self) -> String {
        self.said
            .lock()
            .log
            .last()
            .map(|l| l.0.clone())
            .unwrap_or_default()
    }
}

#[test]
fn the_values_view_updates_in_place_and_keeps_the_place() {
    let mut r = rig_with(None, "wx", weather);
    assert!(r.app.live_shows_values());
    assert!(r.text().contains("No values yet."), "{}", r.text());
    r.send_at(
        "station/wx",
        r#"{"wind_kmh": 12, "temp_c": 21.44, "humidity": 55}"#,
        0,
    );
    // Mapped first, then the rest in message order.
    let text = r.text();
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines,
        [
            "Weather",
            "Temperature: 21.4 C",
            "Wind kmh: 12",
            "Humidity: 55"
        ],
        "{text}"
    );
    let first = r.app.live_take_field_changes();
    assert_eq!(first.len(), 3, "{first:?}");
    // The reader on Humidity stays there while the others change.
    r.go("Humidity");
    let cursor = r.app.session().unwrap().cursor;
    r.send_at(
        "station/wx",
        r#"{"wind_kmh": 18, "temp_c": 21.61, "humidity": 55}"#,
        5,
    );
    assert!(r.text().contains("Temperature: 21.6 C"), "{}", r.text());
    assert!(r.text().contains("Wind kmh: 18"), "{}", r.text());
    assert_eq!(r.app.session().unwrap().cursor, cursor);
    assert_eq!(r.word(), "Humidity");
    let changes = r.app.live_take_field_changes();
    let labels: Vec<&str> = changes.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["Temperature", "Wind kmh"]);
    assert_eq!(changes[0].old.as_deref(), Some("21.4 C"));
    assert_eq!(changes[0].new.as_deref(), Some("21.6 C"));
    assert_eq!(changes[0].line, "Temperature: 21.6 C");
    assert!(changes[0].show && !changes[0].speak);
    assert_eq!(changes[0].min_interval, Duration::from_secs(10));
    // Nothing is announced by the view itself: that is the announcements'
    // choice.
    assert!(!r.last_said().contains("21.6"), "{}", r.last_said());
    // A field that disappears reads no value; the newest is the last
    // changed.
    r.send_at("station/wx", r#"{"temp_c": 21.61, "humidity": 56}"#, 6);
    assert!(r.text().contains("Wind kmh: no value"), "{}", r.text());
    assert_eq!(
        r.app.live_newest_line().as_deref(),
        Some("Wind kmh: no value"),
        "{:?}",
        r.app.live_take_field_changes()
    );
    assert_eq!(r.app.title_position(), Some(vec!["3 fields".to_owned()]));
    // The table and Save are for logs: said in words.
    let _ = r.app.live_toggle_view();
    assert!(r.last_said().contains("no table"), "{}", r.last_said());
}

#[test]
fn stale_values_say_so_on_a_virtual_clock() {
    let mut r = rig_with(None, "wx", weather);
    r.send_at("station/wx", r#"{"temp_c": 20, "rain": 0}"#, 0);
    let _ = r.app.tick(r.start + Duration::from_secs(119));
    assert!(r.text().contains("Temperature: 20.0 C\n"), "{}", r.text());
    let _ = r.app.tick(r.start + Duration::from_secs(120));
    assert!(
        r.text().contains("Temperature: 20.0 C, no recent value\n"),
        "{}",
        r.text()
    );
    assert!(r.text().contains("Rain: 0, no recent value\n"));
    r.send_at("station/wx", r#"{"temp_c": 20, "rain": 0}"#, 121);
    assert!(r.text().contains("Temperature: 20.0 C\n"), "{}", r.text());
}

#[test]
fn paused_values_change_unseen_and_resume_in_one_step() {
    let mut r = rig_with(None, "wx", weather);
    r.send_at("station/wx", r#"{"temp_c": 20, "rain": 0}"#, 0);
    let _ = r.app.live_take_field_changes();
    let _ = r.app.live_set_paused(true);
    r.send_at("station/wx", r#"{"temp_c": 21, "rain": 0}"#, 1);
    r.send_at("station/wx", r#"{"temp_c": 22, "rain": 1}"#, 2);
    assert!(r.text().contains("Temperature: 20.0 C"), "{}", r.text());
    let _ = r.app.live_set_paused(false);
    assert!(r.text().contains("Temperature: 22.0 C"), "{}", r.text());
    assert_eq!(r.last_said(), "Updates resumed, 2 fields changed.");
    assert!(
        r.app.live_take_field_changes().is_empty(),
        "nothing from the pause is read"
    );
}

#[test]
fn a_fields_choices_are_saved_to_the_source_and_it_stays_put() {
    let mut r = rig_with(None, "wx", weather);
    r.send_at(
        "station/wx",
        r#"{"temp_c": 20, "battery": {"volts": 12.4}}"#,
        0,
    );
    let before = r.text();
    assert!(before.contains("Battery\n\nVolts: 12.4"), "{before}");
    r.go("Volts");
    let f = r.app.live_field_at_cursor().expect("a field");
    assert_eq!(
        (f.path.as_str(), f.label.as_str()),
        ("/battery/volts", "Volts")
    );
    assert_eq!(f.speak, LiveNotice::Off);
    let _ = r.app.live_toggle_field_speak();
    assert_eq!(r.last_said(), "Changes of Volts are spoken.");
    let fields = &r.app.settings().live.sources["wx"].fields;
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[1].path, "battery.volts");
    assert_eq!(fields[1].speak, LiveNotice::OnChange);
    let _ = r.app.live_take_field_changes();
    r.send_at(
        "station/wx",
        r#"{"temp_c": 20, "battery": {"volts": 12.1}}"#,
        1,
    );
    let c = r.app.live_take_field_changes();
    assert_eq!(c.len(), 1);
    assert!(c[0].speak);
    assert_eq!(r.text(), before.replace("12.4", "12.1"));
    // The temperature's choice, by path.
    let _ = r
        .app
        .live_set_field_notice("/temp_c", Some(LiveNotice::Off), None);
    assert_eq!(
        r.app.settings().live.sources["wx"].fields[0].show,
        LiveNotice::Off
    );
}

#[test]
fn several_topics_are_grouped_by_topic() {
    let mut r = rig_with(None, "wx", weather);
    r.send_at("station/wx", r#"{"temp_c": 20}"#, 0);
    r.send_at("station/door", "1", 0);
    let text = r.text();
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines,
        [
            "Weather",
            "station/wx",
            "Temperature: 20.0 C",
            "station/door",
            "Door: 1"
        ],
        "{text}"
    );
}

#[test]
fn the_log_view_keeps_the_messages_key_order() {
    let mut r = rig_with(None, "club", |_| {});
    r.send_at(
        "club/log",
        r#"{"call": "N0CALL", "zone": "3", "band": "2m", "antenna": "vertical"}"#,
        0,
    );
    assert!(
        r.text()
            .contains("N0CALL. zone: 3. band: 2m. antenna: vertical."),
        "{}",
        r.text()
    );
    let _ = r.app.live_toggle_view();
    // The table's columns, in the same order.
    let t = r.text();
    let (c, z, b, a) = (
        t.find("Callsign").unwrap(),
        t.find("zone").unwrap(),
        t.find("band").unwrap(),
        t.find("antenna").unwrap(),
    );
    assert!(c < z && z < b && b < a, "{t}");
}

#[test]
fn a_wrappers_plain_values_are_the_logs_heading() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let mut r = rig_with(Some(paths.clone()), "club", |_| {});
    r.send_at(
        "club/log",
        r#"{"net": "Practice net", "frequency": "147.040", "checkins": [{"call": "N0CALL"}, {"call": "N1TEST"}]}"#,
        0,
    );
    let text = r.text();
    let head = text.find("Practice net. frequency: 147.040.").expect(&text);
    let first = text.find("N0CALL").unwrap();
    assert!(head < first, "{text}");
    // Saved and reopened, the heading comes back.
    r.app.shutdown();
    let path = std::fs::read_dir(paths.live_logs_dir())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    drop(r);
    let mut app = App::new(AppConfig {
        paths: Some(paths),
        ..AppConfig::for_tests()
    });
    let _ = app.dispatch(Command::Open(path));
    let text = app.session().unwrap().doc.text().to_string();
    assert!(text.contains("Practice net. frequency: 147.040."), "{text}");
}

fn files_in(dir: &Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default()
}

#[test]
fn the_log_folder_setting_is_honored() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let elsewhere = dir.path().join("net-logs");
    let folder = elsewhere.to_string_lossy().into_owned();
    let mut r = rig_with(Some(paths.clone()), "club", move |s| {
        s.live.log_folder = folder;
    });
    assert_eq!(
        r.app.live_log_folder().as_deref(),
        Some(elsewhere.as_path())
    );
    r.send_at("club/log", r#"{"call": "N0CALL"}"#, 0);
    let _ = r.app.wait_for_writes();
    assert_eq!(files_in(&elsewhere).len(), 1, "{:?}", files_in(&elsewhere));
    assert!(files_in(&paths.live_logs_dir()).is_empty());
}
