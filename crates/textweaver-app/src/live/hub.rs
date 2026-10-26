//! The live hub: the app's side of an open live document. It owns the
//! source, the log view's entries, the view (list or table), the reader's
//! furthest entry for the unread count, the pause queue, and the JSON log
//! file. `App::live_tick` polls the source on every tick.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use textweaver_core::CharPos;
use textweaver_lexicon::args;
use textweaver_store::DocKey;

use super::json_log::{self, SavedLog};
use super::log::{LiveLog, LogLabels, LogOptions, LogView, MAX_ENTRIES, Shape};
use super::source::{LiveEvent, LivePayload, LiveSource};
use super::{LiveTag, load_markdown};
use crate::app::App;
use crate::command::Effect;
use crate::nav::ReadAfter;

/// How often a running log is written to its JSON file, at most.
pub const LOG_WRITE_INTERVAL: Duration = Duration::from_secs(10);

/// The most messages kept while updates are paused; older ones are left
/// out, and said on resuming. A whole log sent while paused replaces
/// everything queued before it, so a broker that resends the whole log
/// never fills the queue.
pub const PAUSE_QUEUE_CAP: usize = 1_000;

/// What a source is opened with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LiveOptions {
    /// The source's id (`[live.sources.<id>]`); it names the document and
    /// prefixes the JSON log's file name. May be empty.
    pub id: String,
    /// The title: the source's name.
    pub title: String,
    /// The field that identifies an entry, when the source names one.
    pub id_field: Option<String>,
    /// The view it opens in.
    pub view: LogView,
}

/// The JSON log file of a running log.
#[derive(Debug)]
struct LogFile {
    /// When the log was opened (RFC 3339, UTC).
    started: String,
    /// The local date and time for the file name, `YYYY-MM-DD-HHMM`.
    stamp: String,
    /// The file, fixed on the first write.
    path: Option<PathBuf>,
    /// Entries changed since the last write.
    dirty: bool,
    last_write: Option<Instant>,
    /// The last write failed (said once until one works).
    failing: bool,
}

/// The open live document's state (see the module).
pub(crate) struct LiveHub {
    source: Option<Box<dyn LiveSource>>,
    id: String,
    pub(crate) log: LiveLog,
    pub(crate) view: LogView,
    paused: bool,
    queue: VecDeque<Shape>,
    queue_dropped: usize,
    /// The furthest entry the reader has been on.
    furthest: Option<usize>,
    file: Option<LogFile>,
    /// The file a saved log was reopened from.
    saved_from: Option<PathBuf>,
    /// The log was full and that was said.
    full_said: bool,
}

impl std::fmt::Debug for LiveHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveHub")
            .field("id", &self.id)
            .field("receiving", &self.source.is_some())
            .field("entries", &self.log.len())
            .field("view", &self.view)
            .field("paused", &self.paused)
            .finish_non_exhaustive()
    }
}

/// Now, in UTC, for arrival times.
fn now_utc() -> String {
    textweaver_store::time::rfc3339(textweaver_store::now_ts())
}

/// The reader's entry before a change: its key, index, and the cursor's
/// offset into it.
struct Place {
    key: Option<String>,
    index: usize,
    offset: usize,
}

impl App {
    /// A waker for a live source: it rings the frontend, which then ticks
    /// the app ([`App::tick`]), and the app polls the source. Give it to
    /// the source when starting it; it is cheap, quick, and never blocks.
    pub fn live_waker(&self) -> Box<dyn Fn() + Send + Sync> {
        let slot = self.waker_slot();
        Box::new(move || slot.wake())
    }

    /// Opens a live document fed by `source`, in the log view: empty
    /// until its first message. The document before it closes, as when
    /// any document opens. Closing this one ([`App::live_close`], opening
    /// another, quitting) stops the source.
    pub fn open_live_source(
        &mut self,
        source: Box<dyn LiveSource>,
        options: LiveOptions,
    ) -> Vec<Effect> {
        let title = if options.title.trim().is_empty() {
            self.msg("live-log-title")
        } else {
            options.title.clone()
        };
        let log = LiveLog::new(LogOptions {
            title: title.clone(),
            id_field: options.id_field.clone(),
        });
        let md = log.render(options.view, &self.live_labels());
        let key = DocKey(format!("live-{}", options.id));
        let effects = self.open_live_keyed(key, &options.id, &title, &md);
        self.live_hub = Some(LiveHub {
            source: Some(source),
            id: options.id,
            log,
            view: options.view,
            paused: false,
            queue: VecDeque::new(),
            queue_dropped: 0,
            furthest: None,
            file: Some(LogFile {
                started: now_utc(),
                stamp: crate::templates::local_minute(),
                path: None,
                dirty: false,
                last_write: None,
                failing: false,
            }),
            saved_from: None,
            full_said: false,
        });
        effects
    }

    /// Opens a saved JSON log (`path`) in the log view, read only and not
    /// live. `None` when the file is not a textweaver live log.
    pub fn open_live_log(&mut self, path: &Path) -> Option<Vec<Effect>> {
        let saved = json_log::read(path)?;
        Some(self.open_saved_log(path, saved))
    }

    pub(crate) fn open_saved_log(&mut self, path: &Path, saved: SavedLog) -> Vec<Effect> {
        let SavedLog { mut log, source } = saved;
        if log.options.title.trim().is_empty() {
            log.options.title = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
        }
        let title = log.options.title.clone();
        let md = log.render(LogView::List, &self.live_labels());
        let key = DocKey(format!("live-file-{}", DocKey::for_path(path).0));
        let effects = self.open_live_keyed(key, &source, &title, &md);
        if let Some(s) = self.session.as_mut()
            && let Some(tag) = s.live.as_mut()
        {
            tag.receiving = false;
        }
        self.live_hub = Some(LiveHub {
            source: None,
            id: source,
            log,
            view: LogView::List,
            paused: false,
            queue: VecDeque::new(),
            queue_dropped: 0,
            furthest: None,
            file: None,
            saved_from: Some(path.to_owned()),
            full_said: false,
        });
        effects
    }

    /// The words a log is rendered with, in the interface's language.
    pub fn live_labels(&self) -> LogLabels {
        LogLabels {
            empty: self.msg("live-log-empty"),
            empty_entry: self.msg("live-entry-empty"),
            headers: [
                "live-field-callsign",
                "live-field-name",
                "live-field-location",
                "live-field-status",
                "live-field-traffic",
                "live-field-notes",
                "live-field-time",
            ]
            .map(|k| self.msg(k)),
            traffic: self.msg("live-traffic"),
            yes: self.msg("live-yes"),
            no: self.msg("live-no"),
        }
    }

    /// True while a live document is open and its source is running.
    pub fn live_receiving(&self) -> bool {
        self.live_hub.as_ref().is_some_and(|h| h.source.is_some())
    }

    /// The open live document's view; `None` when none is open.
    pub fn live_view(&self) -> Option<LogView> {
        self.live_hub.as_ref().map(|h| h.view)
    }

    /// The open live document's entries; `None` when none is open.
    pub fn live_log(&self) -> Option<&LiveLog> {
        self.live_hub.as_ref().map(|h| &h.log)
    }

    /// The newest entry as one line, meaning first, for a status line.
    pub fn live_newest_line(&self) -> Option<String> {
        let h = self.live_hub.as_ref()?;
        let n = h.log.len().checked_sub(1)?;
        Some(h.log.line(n, &self.live_labels()))
    }

    /// True while updates are paused.
    pub fn live_paused(&self) -> bool {
        self.live_hub.as_ref().is_some_and(|h| h.paused)
    }

    /// How many messages wait while updates are paused.
    pub fn live_waiting(&self) -> usize {
        self.live_hub.as_ref().map_or(0, |h| h.queue.len())
    }

    /// The JSON log's file, once written (or the file a saved log was
    /// opened from).
    pub fn live_log_path(&self) -> Option<PathBuf> {
        let h = self.live_hub.as_ref()?;
        h.saved_from
            .clone()
            .or_else(|| h.file.as_ref().and_then(|f| f.path.clone()))
    }

    /// The entry the cursor is on, in the open live document.
    fn live_cursor_entry(&self) -> Option<usize> {
        let h = self.live_hub.as_ref()?;
        let s = self.session.as_ref().filter(|s| s.live.is_some())?;
        LiveLog::entry_at(&s.doc, h.view, s.cursor)
    }

    /// Entries after the furthest one the reader has been on (the cursor
    /// on an entry counts as reading it); `None` with no live document.
    pub fn live_unread(&self) -> Option<usize> {
        let h = self.live_hub.as_ref()?;
        let furthest = h.furthest.max(self.live_cursor_entry());
        Some(match furthest {
            Some(i) => h.log.len().saturating_sub(i + 1),
            None => h.log.len(),
        })
    }

    /// Notes the reader's entry as read, for the unread count.
    fn live_note_reading(&mut self) {
        let at = self.live_cursor_entry();
        if let Some(h) = self.live_hub.as_mut() {
            h.furthest = h.furthest.max(at);
        }
    }

    /// The position part of the title line for a live document, in place
    /// of the line and percentage, which would change with every entry:
    /// the unread count ("3 unread"), and "updates paused" while they are;
    /// for a log no longer receiving, its entry count. `None` when the
    /// open document is not live.
    pub fn live_position(&self) -> Option<Vec<String>> {
        let h = self.live_hub.as_ref()?;
        self.session.as_ref().filter(|s| s.live.is_some())?;
        if h.source.is_none() {
            let n = h.log.len();
            return Some(vec![self.msg_args("live-entries", &args!["count" => n])]);
        }
        let unread = self.live_unread().unwrap_or_default();
        let mut parts = vec![self.msg_args("live-unread", &args!["count" => unread])];
        if h.paused {
            parts.push(self.msg("live-paused-status"));
        }
        Some(parts)
    }

    /// Where the reader is, in entries, before the log changes.
    fn live_place(&self) -> Option<Place> {
        let h = self.live_hub.as_ref()?;
        let s = self.session.as_ref()?;
        let ranges = LiveLog::entry_ranges(&s.doc, h.view);
        let index = ranges
            .iter()
            .position(|r| r.start <= s.cursor && s.cursor < r.end.max(CharPos(r.start.0 + 1)))?;
        Some(Place {
            key: h.log.key_of(index),
            index,
            offset: s.cursor.0 - ranges[index].start.0,
        })
    }

    /// Polls the live source and applies what arrived (called from
    /// [`App::tick`]); writes the JSON log when it is due.
    pub(crate) fn live_tick(&mut self, now: Instant) -> Vec<Effect> {
        if self.live_hub.is_none() {
            return Vec::new();
        }
        if self.session.as_ref().is_none_or(|s| s.live.is_none()) {
            // The document was replaced without closing: stop the source.
            self.live_leave();
            return Vec::new();
        }
        self.live_note_reading();
        let events = self
            .live_hub
            .as_mut()
            .and_then(|h| h.source.as_mut())
            .map(|s| s.poll())
            .unwrap_or_default();
        let mut effects = Vec::new();
        let mut shapes = Vec::new();
        for event in events {
            match event {
                LiveEvent::Message { topic, payload, .. } => match payload {
                    LivePayload::Json(v) => shapes.push(Shape::of(v)),
                    LivePayload::Text(t) => {
                        let mut m = serde_json::Map::new();
                        m.insert("text".into(), serde_json::Value::String(t));
                        shapes.push(Shape::One(m));
                    }
                    LivePayload::Binary { len } => {
                        let m = self.msg_args(
                            "live-binary",
                            &args!["topic" => topic.as_str(), "len" => len],
                        );
                        self.warn(&m);
                        effects.push(Effect::Redraw);
                    }
                    LivePayload::TooLarge { len } => {
                        let m = self.msg_args(
                            "live-too-large",
                            &args!["topic" => topic.as_str(), "len" => len],
                        );
                        self.warn(&m);
                        effects.push(Effect::Redraw);
                    }
                },
                other => {
                    self.live_connection_event(other);
                    effects.push(Effect::Redraw);
                }
            }
        }
        if !shapes.is_empty() {
            if self.live_paused() {
                self.live_queue(shapes);
                effects.push(Effect::Redraw);
            } else if self.live_take(shapes, None) {
                effects.push(Effect::Redraw);
            }
        }
        self.live_write_due(now);
        effects
    }

    /// Says a connection change.
    fn live_connection_event(&mut self, event: LiveEvent) {
        let name = self
            .session
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_default();
        let name = name.as_str();
        match event {
            LiveEvent::Connecting { attempt } if attempt <= 1 => {
                let m = self.msg_args("live-connecting", &args!["name" => name]);
                self.note(&m);
            }
            LiveEvent::Connecting { attempt } => {
                let m = self.msg_args(
                    "live-reconnecting",
                    &args!["name" => name, "attempt" => attempt],
                );
                self.note(&m);
            }
            LiveEvent::Connected {
                encrypted,
                tailscale,
            } => {
                let key = match (encrypted, tailscale) {
                    (true, _) => "live-connected",
                    (false, true) => "live-connected-tailscale",
                    (false, false) => "live-connected-plain",
                };
                // Polite, plain or not: only a lost connection interrupts.
                let m = self.msg_args(key, &args!["name" => name]);
                self.tell(&m);
            }
            LiveEvent::Disconnected { reason } => {
                let m = self.msg_args(
                    "live-disconnected",
                    &args!["name" => name, "reason" => reason.as_str()],
                );
                self.warn(&m);
            }
            LiveEvent::Failed { reason } => {
                let m = self.msg_args(
                    "live-failed",
                    &args!["name" => name, "reason" => reason.as_str()],
                );
                self.warn(&m);
            }
            LiveEvent::Quiet { topic, .. } => {
                let m = self.msg_args("live-quiet", &args!["topic" => topic.as_str()]);
                self.tell(&m);
            }
            LiveEvent::TopicRefused { topic } => {
                let m = self.msg_args("live-topic-refused", &args!["topic" => topic.as_str()]);
                self.tell(&m);
            }
            LiveEvent::Skipped { count } => {
                let m = self.msg_args("live-skipped", &args!["count" => count]);
                self.tell(&m);
            }
            LiveEvent::Message { .. } => {}
        }
    }

    /// Queues messages while paused. A whole log replaces what is queued.
    fn live_queue(&mut self, shapes: Vec<Shape>) {
        let Some(h) = self.live_hub.as_mut() else {
            return;
        };
        for shape in shapes {
            if matches!(shape, Shape::Whole(_)) {
                h.queue.clear();
            }
            if h.queue.len() >= PAUSE_QUEUE_CAP {
                h.queue.pop_front();
                h.queue_dropped += 1;
            }
            h.queue.push_back(shape);
        }
    }

    /// Applies messages to the log and shows the result in one change,
    /// keeping the reader on their entry. `say` replaces the announcement
    /// of the changed entry (resuming says how many updates it applied).
    /// True when the log changed.
    fn live_take(&mut self, shapes: Vec<Shape>, say: Option<String>) -> bool {
        let place = self.live_place();
        let labels = self.live_labels();
        let Some(h) = self.live_hub.as_mut() else {
            return false;
        };
        let furthest_key = h.furthest.and_then(|i| h.log.key_of(i));
        let now = now_utc();
        let mut whole = false;
        // The last entry changed stands for all of them; the rest are
        // counted.
        let mut last_line: Option<String> = None;
        let mut count = 0;
        let before = h.log.dropped;
        for shape in shapes {
            whole |= matches!(shape, Shape::Whole(_));
            let changed = h.log.apply(shape, &now);
            if let Some(&last) = changed.last() {
                last_line = Some(h.log.line(last, &labels));
                count += changed.len();
            }
        }
        let full = h.log.dropped > before && !h.full_said;
        if full {
            h.full_said = true;
        }
        let Some(line) = last_line else {
            if full {
                let m = self.msg_args("live-full", &args!["max" => MAX_ENTRIES]);
                self.warn(&m);
            }
            return false;
        };
        if let Some(f) = h.file.as_mut() {
            f.dirty = true;
        }
        if whole && let Some(k) = furthest_key {
            h.furthest = h.log.find_key(&k).or(h.furthest);
        }
        h.furthest = h.furthest.map(|i| i.min(h.log.len().saturating_sub(1)));
        let md = h.log.render(h.view, &labels);
        let view = h.view;
        let new = load_markdown(&md);
        // A whole log may reorder or drop entries: put the reader back on
        // theirs by its key, at the same offset.
        let target = place.filter(|_| whole).and_then(|p| {
            let i = p
                .key
                .as_deref()
                .and_then(|k| h.log.find_key(k))
                .unwrap_or(p.index);
            let ranges = LiveLog::entry_ranges(&new, view);
            let r = ranges.get(i).or(ranges.last())?;
            Some(CharPos((r.start.0 + p.offset).min(r.end.0.max(r.start.0))))
        });
        self.live_replace(new, target);
        match say {
            // The answer to the user's own command (resuming): said now.
            Some(text) if !text.is_empty() => self.tell(&text),
            Some(_) => {}
            // New entries: through the announcement throttle
            // (crate::live::announce), said on the tick.
            None => self.live_entries_arrived(count, line),
        }
        if full {
            let m = self.msg_args("live-full", &args!["max" => MAX_ENTRIES]);
            self.warn(&m);
        }
        true
    }

    /// Shows the log in `view`, keeping the reader on the same entry.
    pub fn live_set_view(&mut self, view: LogView) -> Vec<Effect> {
        let labels = self.live_labels();
        let Some(h) = self.live_hub.as_mut() else {
            return self.live_not_open();
        };
        let from = h.view;
        h.view = view;
        let md = h.log.render(view, &labels);
        let new = load_markdown(&md);
        let Some(s) = self.session.as_ref() else {
            return Vec::new();
        };
        let place = LiveLog::same_entry(&s.doc, from, &new, view, s.cursor);
        self.live_replace(new, place);
        let key = match view {
            LogView::List => "live-view-list",
            LogView::Table => "live-view-table",
        };
        let m = self.msg(key);
        self.tell(&m);
        vec![Effect::Redraw]
    }

    /// Switches between the list and the table.
    pub fn live_toggle_view(&mut self) -> Vec<Effect> {
        match self.live_view() {
            Some(v) => self.live_set_view(v.other()),
            None => self.live_not_open(),
        }
    }

    /// Moves to the newest entry and says it; every entry counts as read.
    pub fn live_jump_newest(&mut self) -> Vec<Effect> {
        let labels = self.live_labels();
        let Some(h) = self.live_hub.as_mut() else {
            return self.live_not_open();
        };
        let Some(last) = h.log.len().checked_sub(1) else {
            let m = self.msg("live-newest-none");
            self.tell(&m);
            return vec![Effect::Redraw];
        };
        h.furthest = Some(last);
        let line = h.log.line(last, &labels);
        let view = h.view;
        let Some(target) = self
            .session
            .as_ref()
            .and_then(|s| LiveLog::entry_start(&s.doc, view, last))
        else {
            return Vec::new();
        };
        self.jump(target, true, ReadAfter::Follow, &line);
        vec![Effect::Redraw]
    }

    /// Pauses updates (they wait, up to [`PAUSE_QUEUE_CAP`] messages), or
    /// resumes them, applying everything that waited as one change.
    pub fn live_set_paused(&mut self, paused: bool) -> Vec<Effect> {
        let Some(h) = self.live_hub.as_mut() else {
            return self.live_not_open();
        };
        if h.paused == paused {
            return Vec::new();
        }
        h.paused = paused;
        if paused {
            let m = self.msg("live-paused");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        let queued: Vec<Shape> = h.queue.drain(..).collect();
        let dropped = std::mem::take(&mut h.queue_dropped);
        let n = queued.len();
        let m = if dropped > 0 {
            self.msg_args(
                "live-resumed-dropped",
                &args!["count" => n, "dropped" => dropped],
            )
        } else {
            self.msg_args("live-resumed", &args!["count" => n])
        };
        if !self.live_take(queued, Some(m.clone())) {
            self.tell(&m);
        }
        vec![Effect::Redraw]
    }

    /// Pauses or resumes updates.
    pub fn live_toggle_pause(&mut self) -> Vec<Effect> {
        if self.live_hub.is_none() {
            return self.live_not_open();
        }
        let paused = self.live_paused();
        self.live_set_paused(!paused)
    }

    fn live_not_open(&mut self) -> Vec<Effect> {
        let m = self.msg("live-not-open");
        self.tell(&m);
        vec![Effect::Redraw]
    }

    /// Stops the live source: the log stays open, read only, and is
    /// written to its JSON file. Updates waiting while paused are applied
    /// first.
    pub fn live_close(&mut self) -> Vec<Effect> {
        let Some(h) = self.live_hub.as_mut() else {
            return Vec::new();
        };
        let Some(mut source) = h.source.take() else {
            return Vec::new();
        };
        source.stop();
        if h.paused {
            h.paused = false;
            let queued: Vec<Shape> = h.queue.drain(..).collect();
            self.live_take(queued, Some(String::new()));
        }
        if let Some(s) = self.session.as_mut()
            && let Some(tag) = s.live.as_mut()
        {
            tag.receiving = false;
        }
        self.live_write(Instant::now(), false);
        let name = self
            .session
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_default();
        let m = self.msg_args("live-stopped", &args!["name" => name]);
        self.note(&m);
        vec![Effect::Redraw]
    }

    /// The live document is closing (another opens, or quitting): stop
    /// the source, write the log, and forget it.
    pub(crate) fn live_leave(&mut self) {
        if let Some(mut source) = self.live_hub.as_mut().and_then(|h| h.source.take()) {
            source.stop();
            if let Some(h) = self.live_hub.as_mut() {
                // What waited while paused is kept in the file, if not
                // shown.
                let queued: Vec<Shape> = h.queue.drain(..).collect();
                let now = now_utc();
                for shape in queued {
                    if !h.log.apply(shape, &now).is_empty()
                        && let Some(f) = h.file.as_mut()
                    {
                        f.dirty = true;
                    }
                }
            }
            self.live_write(Instant::now(), false);
        }
        self.live_hub = None;
    }

    /// Save on a live document: writes the JSON log now and says where.
    pub fn live_save_log(&mut self) -> Vec<Effect> {
        let Some(h) = self.live_hub.as_ref() else {
            return self.live_not_open();
        };
        if let Some(path) = h.saved_from.clone() {
            let m = self.msg_args(
                "live-log-saved-already",
                &args!["path" => path.display().to_string()],
            );
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        if h.log.is_empty() {
            let m = self.msg("live-log-empty-not-saved");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        if self.live_log_folder().is_none() {
            let m = self.msg("live-log-nowhere");
            self.error(&m);
            return vec![Effect::Redraw];
        }
        self.live_write(Instant::now(), true);
        vec![Effect::Redraw]
    }

    /// The folder JSON logs go in: the `logs` folder in textweaver's data
    /// folder, created on the first save; `None` without one (tests that
    /// keep nothing).
    pub fn live_log_folder(&self) -> Option<PathBuf> {
        self.paths.as_ref().map(|p| p.data_dir.join("logs"))
    }

    /// Writes the JSON log when it changed and the last write was
    /// [`LOG_WRITE_INTERVAL`] ago.
    fn live_write_due(&mut self, now: Instant) {
        let due = self
            .live_hub
            .as_ref()
            .and_then(|h| h.file.as_ref())
            .is_some_and(|f| {
                f.dirty
                    && f.last_write
                        .is_none_or(|t| now.saturating_duration_since(t) >= LOG_WRITE_INTERVAL)
            });
        if due {
            self.live_write(now, false);
        }
    }

    /// Sends the JSON log to the writer, which writes it atomically. With
    /// `say`, the writer's report says where it went. Nothing is written
    /// before the first entry, or with nowhere to write.
    fn live_write(&mut self, now: Instant, say: bool) {
        let folder = self.live_log_folder();
        let Some(h) = self.live_hub.as_mut() else {
            return;
        };
        let Some(f) = h.file.as_mut() else {
            return;
        };
        if h.log.is_empty() || (!f.dirty && !say && f.path.is_some()) {
            return;
        }
        let Some(folder) = folder else {
            return;
        };
        let path = f
            .path
            .get_or_insert_with(|| {
                json_log::free_path(&folder, &json_log::file_name(&h.id, &f.stamp))
            })
            .clone();
        let bytes = json_log::to_json(&h.log, &h.id, &f.started);
        f.dirty = false;
        f.last_write = Some(now);
        self.writer
            .send(crate::writer::Job::LiveLog { path, bytes, say });
    }

    /// The writer wrote the JSON log, or could not.
    pub(crate) fn live_log_written(&mut self, path: &Path, result: Result<(), String>, say: bool) {
        let failing = self
            .live_hub
            .as_ref()
            .and_then(|h| h.file.as_ref())
            .is_some_and(|f| f.failing);
        match result {
            Ok(()) => {
                if let Some(f) = self.live_hub.as_mut().and_then(|h| h.file.as_mut()) {
                    f.failing = false;
                }
                if say {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let folder = path
                        .parent()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    let m =
                        self.msg_args("live-log-saved", &args!["name" => name, "folder" => folder]);
                    self.tell(&m);
                }
            }
            Err(e) => {
                if let Some(f) = self.live_hub.as_mut().and_then(|h| h.file.as_mut()) {
                    f.failing = true;
                    // Try again at the next interval.
                    f.dirty = true;
                }
                if say || !failing {
                    let m = self.msg_args(
                        "live-log-save-failed",
                        &args!["path" => path.display().to_string(), "error" => e],
                    );
                    self.error(&m);
                }
            }
        }
    }

    /// True when the open document is a live document (receiving or a
    /// saved log reopened).
    pub(crate) fn session_is_live(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.live.is_some())
    }

    /// Refuses edit mode on a live document, in words. True when refused.
    pub(crate) fn live_refuse_edit(&mut self) -> bool {
        if !self.session_is_live() {
            return false;
        }
        let m = self.msg("live-read-only");
        self.tell(&m);
        true
    }

    /// Makes `md` the open document, marked live, under `key`.
    pub(crate) fn open_live_keyed(
        &mut self,
        key: DocKey,
        source: &str,
        title: &str,
        md: &str,
    ) -> Vec<Effect> {
        let doc = load_markdown(md);
        let effects = self.open_document(doc, key, title.to_owned());
        if let Some(s) = self.session.as_mut() {
            s.live = Some(LiveTag {
                source: source.to_owned(),
                receiving: true,
            });
        }
        effects
    }
}
