//! Sync in the reader (the sync wave, ADR-0049): notes, highlights,
//! bookmarks, places, and reading statistics shared with the owner's other
//! computers through a folder they choose.
//!
//! Everything that touches the sync folder runs on the background writer
//! ([`crate::sync_engine`]); this module decides when, and applies and says
//! what comes back:
//!
//! - **Publishing.** Every state save marks the open document changed; the
//!   next tick after [`PUBLISH_DEBOUNCE`] sends this computer's merged view,
//!   with its place. Leaving a document and quitting publish at once.
//! - **Receiving.** A timed scan every [`SCAN_INTERVAL`] looks at the other
//!   computers' files for the open document (their size and time only) and
//!   merges when one changed. A timed scan works the same on a USB stick, a
//!   network folder, and a sync service's folder, and adds no dependency to
//!   the lean reader. Arriving items are found again in this computer's
//!   text (`crate::relocate`) and never move the cursor.
//! - **Places.** When a document opens, `[sync] position_policy` picks
//!   another computer's place: newest or furthest resume there while
//!   nothing has moved yet; "ask" asks, naming the computer ("lab at 42
//!   percent. Go there? Y or N"). While reading, a place is only offered.
//! - **Notes.** The newest edit wins; the note it replaced goes to the local
//!   backup of replaced notes, and the owner hears which ("Cells: a note was
//!   replaced by laptop's newer edit"). Replaced notes lists them, and Enter
//!   puts one back.
//! - **Messages** go through ADR-0043's levels (arrivals routine, conflicts
//!   and replacements results, write failures errors) and are held while
//!   reading aloud, then said at the pause. Damaged files, a newer format,
//!   and a clock far ahead are said once a session.
//! - **The status line** starts with "Sync" ([`App::sync_status_line`]).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_a11y::{Importance, Priority, Verbosity};
use textweaver_core::CharPos;
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_store::{Anchor, DocKey, DocState, MarkKind, PositionPolicy};
use textweaver_sync::{DeviceId, SyncId};

use crate::app::{App, ListKind};
use crate::command::{Confirm, Effect, PromptPurpose};
use crate::nav::ReadAfter;
use crate::playback::Playback;
use crate::sync_engine::{
    AllOutcome, Applied, CycleOutcome, EngineConfig, EngineStatus, Groups, Item, Notice,
    OtherPlace, Snapshot, StatusKind, SyncEngine, SyncRequest, SyncResponse, apply_arrivals,
};
use crate::text_util;
use crate::writer::Job;

/// How long after a change the open document is published: changes made
/// together go out together.
pub const PUBLISH_DEBOUNCE: Duration = Duration::from_secs(2);

/// How often the other computers' files for the open document are looked
/// at.
pub const SCAN_INTERVAL: Duration = Duration::from_secs(3);

/// A question sync is waiting for a yes or no to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SyncQuestion {
    /// Go to another computer's place?
    Place {
        /// The document it is for.
        key: DocKey,
        /// The computer, by name.
        label: String,
        /// The place.
        pos: CharPos,
        /// Its percentage.
        pct: u8,
        /// The text at the place.
        anchor: Option<Anchor>,
    },
    /// Is this document the one another computer has (same DOI or ISBN)?
    Suggestion {
        /// The document it is for.
        key: DocKey,
        /// The other computer's sync id for it.
        sync_id: SyncId,
        /// The computer, by name.
        label: String,
        /// The title the other computer knows.
        title: String,
        /// Its notes.
        notes: usize,
    },
}

/// A sync list shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SyncList {
    /// Set up sync's groups, then "Start syncing".
    Groups,
    /// Other computers' places, by index into `SyncState::places`.
    Places,
    /// Replaced notes, by backup id.
    Replaced(Vec<String>),
}

/// Set up sync, part way.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Setup {
    folder: PathBuf,
    name: String,
    groups: Groups,
}

/// The open document's sync.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DocSync {
    key: DocKey,
    /// Known once the writer found it.
    sync_id: Option<SyncId>,
    /// Where the cursor was when the document opened: a place from another
    /// computer is resumed only while it is still there.
    opened_at: CharPos,
}

/// The app's sync state.
#[derive(Default)]
pub(crate) struct SyncState {
    /// The engine; only the writer thread locks it.
    engine: Arc<Mutex<SyncEngine>>,
    /// The configuration last sent to the engine.
    configured: Option<EngineConfig>,
    /// Whether a configuration was ever sent.
    started: bool,
    /// Requests sent and not answered yet.
    in_flight: u32,
    doc: Option<DocSync>,
    status: EngineStatus,
    /// The other computers' places in the open document, newest first.
    places: Vec<OtherPlace>,
    /// Computers whose place in the open document was offered already.
    places_said: Vec<DeviceId>,
    /// Messages held while reading aloud.
    held: Vec<(String, Importance)>,
    /// The question asked.
    pub(crate) question: Option<SyncQuestion>,
    /// Questions waiting for the one asked, or for the reading to pause.
    waiting: Vec<SyncQuestion>,
    setup: Option<Setup>,
    dirty: bool,
    last_publish: Option<Instant>,
    last_scan: Option<Instant>,
    /// Sync now is waiting for its pass over the other documents.
    all_pending: bool,
}

impl std::fmt::Debug for SyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncState").finish_non_exhaustive()
    }
}

/// The group names, in the order Set up sync lists them.
const GROUP_IDS: [&str; 5] = [
    "sync-group-places",
    "sync-group-notes",
    "sync-group-highlights",
    "sync-group-bookmarks",
    "sync-group-statistics",
];

fn group_value(g: &Groups, i: usize) -> bool {
    [g.places, g.notes, g.highlights, g.bookmarks, g.statistics][i]
}

fn flip_group(g: &mut Groups, i: usize) {
    match i {
        0 => g.places = !g.places,
        1 => g.notes = !g.notes,
        2 => g.highlights = !g.highlights,
        3 => g.bookmarks = !g.bookmarks,
        _ => g.statistics = !g.statistics,
    }
}

impl App {
    /// True when sync is set up and on.
    pub fn sync_enabled(&self) -> bool {
        self.settings.sync.enabled && self.settings.sync.folder.is_some() && self.paths.is_some()
    }

    /// The sync status line, meaning first: "Sync: up to date", "Sync:
    /// folder missing, saving here", "Sync: newer format, read only",
    /// "Sync: lab's clock is ahead".
    pub fn sync_status_line(&self) -> String {
        crate::sync_engine::status_line(
            self.cat(),
            &self.settings.sync,
            self.sync_enabled(),
            &self.sync.status,
        )
    }

    /// Says a sync message, or holds it while reading aloud: sync never
    /// speaks over the reading (ADR-0049).
    fn sync_say(&mut self, text: String, importance: Importance) {
        if matches!(self.playback, Playback::Reading) {
            self.sync.held.push((text, importance));
            return;
        }
        let priority = if importance == Importance::Error {
            Priority::Assertive
        } else {
            Priority::Polite
        };
        self.say_kind(&text, Verbosity::Low, priority, importance);
    }

    /// The label to say for a computer whose name is not known.
    fn sync_label(&self, label: Option<&str>) -> String {
        label.map_or_else(|| self.msg("sync-another-computer"), str::to_owned)
    }

    /// Sends a request to the engine, on the writer.
    fn sync_send(&mut self, request: SyncRequest) {
        self.sync.in_flight += 1;
        self.writer.send(Job::Sync {
            engine: Arc::clone(&self.sync.engine),
            request,
        });
    }

    /// Sends `[sync]`'s configuration to the engine when it changed.
    fn sync_ensure_configured(&mut self) {
        let wanted = self
            .paths
            .as_ref()
            .and_then(|p| EngineConfig::from_settings(&self.settings.sync, p));
        if self.sync.started && wanted == self.sync.configured {
            return;
        }
        let was_on = self.sync.configured.is_some();
        self.sync.started = true;
        self.sync.configured.clone_from(&wanted);
        if wanted.is_none() && !was_on {
            return;
        }
        self.sync.status = EngineStatus::default();
        self.sync.places.clear();
        self.sync_send(SyncRequest::Configure(wanted.map(Box::new)));
        // The open document is found again in the new folder, on the next
        // tick.
        self.sync.doc = None;
    }

    /// What the open document says about itself, for recognizing it.
    fn sync_details(&self) -> textweaver_sync::docid::Details {
        let Some(s) = self.session.as_ref() else {
            return textweaver_sync::docid::Details::default();
        };
        let meta = crate::library::document_metadata(&s.doc);
        textweaver_sync::docid::Details {
            title: crate::library::stated_title(&s.doc),
            doi: meta.doi,
            isbn: meta.isbn,
        }
    }

    /// A document opened: with sync on, its sync id is found and it is
    /// merged; otherwise only its id is kept (`sync-ids.json`).
    pub(crate) fn sync_document_opened(
        &mut self,
        path: &std::path::Path,
        text: Option<ropey::Rope>,
        details: textweaver_sync::docid::Details,
    ) {
        self.sync_ensure_configured();
        if self.sync_enabled() {
            self.sync_open(path, text, details);
        } else {
            self.identify_on_writer(path, text, details);
        }
    }

    fn sync_open(
        &mut self,
        path: &std::path::Path,
        text: Option<ropey::Rope>,
        details: textweaver_sync::docid::Details,
    ) {
        let Some(paths) = &self.paths else {
            return;
        };
        let identify = textweaver_sync::Identify {
            ids_file: paths.sync_ids_file(),
            path: path.to_owned(),
            library_folders: self.settings.library.folders.clone(),
            details,
        };
        // The place is not published yet: the other computers' places are
        // weighed against the one this computer saved before.
        let Some(snapshot) = self.sync_snapshot(false) else {
            return;
        };
        let opened_at = self.session.as_ref().map_or(CharPos::ZERO, |s| s.cursor);
        self.sync.doc = Some(DocSync {
            key: snapshot.key.clone(),
            sync_id: None,
            opened_at,
        });
        self.sync.places.clear();
        self.sync.places_said.clear();
        self.sync.dirty = false;
        self.sync_send(SyncRequest::Open {
            identify: Box::new(identify),
            text,
            snapshot: Box::new(snapshot),
        });
    }

    /// A document was saved: its new hashes are published (its sync id
    /// stays), or only kept when sync is off.
    pub(crate) fn sync_document_saved(&mut self, path: &std::path::Path) {
        let details = textweaver_sync::docid::Details::default();
        if !self.sync_enabled() {
            self.identify_on_writer(path, None, details);
            return;
        }
        let Some(paths) = &self.paths else {
            return;
        };
        let identify = textweaver_sync::Identify {
            ids_file: paths.sync_ids_file(),
            path: path.to_owned(),
            library_folders: self.settings.library.folders.clone(),
            details,
        };
        self.sync_send(SyncRequest::Reidentify {
            identify: Box::new(identify),
        });
    }

    /// The open document's state as the next save would write it, for the
    /// engine. `None` while editing (positions are in the source text then)
    /// or with no document.
    fn sync_snapshot(&self, publish_place: bool) -> Option<Snapshot> {
        let state = self.state_now()?;
        let key = self.session.as_ref()?.key.clone();
        Some(Snapshot {
            key,
            state,
            publish_place,
        })
    }

    /// The open document's state changed and was saved: publish it soon.
    pub(crate) fn sync_mark_changed(&mut self) {
        self.sync.dirty = true;
    }

    /// Before the open document closes (another opens, or quitting): its
    /// place and changes are published now.
    pub(crate) fn sync_leave_document(&mut self) {
        let Some(sync_id) = self.sync.doc.as_ref().and_then(|d| d.sync_id) else {
            self.sync.doc = None;
            return;
        };
        if self.sync_enabled()
            && let Some(snapshot) = self.sync_snapshot(true)
        {
            self.sync_send(SyncRequest::Cycle {
                sync_id,
                snapshot: Box::new(snapshot),
                force: false,
            });
        }
        self.sync.doc = None;
        self.sync.places.clear();
    }

    /// Sync's part of [`App::tick`]: configuration, publishing, the timed
    /// scan, and messages held while reading.
    pub(crate) fn sync_tick(&mut self, now: Instant) -> Vec<Effect> {
        self.sync_ensure_configured();
        let reading = matches!(self.playback, Playback::Reading);
        let mut effects = Vec::new();
        if !reading && !self.sync.held.is_empty() {
            for (text, importance) in std::mem::take(&mut self.sync.held) {
                self.sync_say(text, importance);
            }
            effects.push(Effect::Redraw);
        }
        if !reading && self.sync.question.is_none() && !self.sync.waiting.is_empty() {
            let q = self.sync.waiting.remove(0);
            self.sync_ask(q);
            effects.push(Effect::Redraw);
        }
        if !self.sync_enabled() || self.sync.in_flight > 0 {
            return effects;
        }
        // The open document is not merged yet (sync just turned on, or it
        // was opened while editing): find it now.
        let unmerged = self.edit.is_none()
            && self.session.as_ref().is_some_and(|s| {
                s.doc.meta.path.is_some() && self.sync.doc.as_ref().is_none_or(|d| d.key != s.key)
            });
        if unmerged && let Some(path) = self.session.as_ref().and_then(|s| s.doc.meta.path.clone())
        {
            let details = self.sync_details();
            self.sync_open(&path, None, details);
            return effects;
        }
        let Some(doc) = self.sync.doc.clone() else {
            return effects;
        };
        let (Some(sync_id), Some(s)) = (doc.sync_id, self.session.as_ref()) else {
            return effects;
        };
        if s.key != doc.key {
            return effects;
        }
        let publish = self.sync.dirty
            && self
                .sync
                .last_publish
                .is_none_or(|t| now.saturating_duration_since(t) >= PUBLISH_DEBOUNCE);
        let scan = self
            .sync
            .last_scan
            .is_none_or(|t| now.saturating_duration_since(t) >= SCAN_INTERVAL);
        if !(publish || scan) {
            return effects;
        }
        let Some(snapshot) = self.sync_snapshot(publish) else {
            return effects;
        };
        self.sync.last_scan = Some(now);
        if publish {
            self.sync.last_publish = Some(now);
            self.sync.dirty = false;
        }
        self.sync_send(SyncRequest::Cycle {
            sync_id,
            snapshot: Box::new(snapshot),
            force: false,
        });
        effects
    }

    /// Applies the engine's answer (from [`App::poll_writes`]).
    pub(crate) fn sync_response(&mut self, response: SyncResponse) -> Vec<Effect> {
        self.sync.in_flight = self.sync.in_flight.saturating_sub(1);
        match response {
            SyncResponse::Configured { status, notices } => {
                self.sync.status = status;
                self.sync_notices(notices);
                vec![Effect::Redraw]
            }
            SyncResponse::Idle { status, .. } | SyncResponse::Done { status } => {
                self.sync.status = status;
                Vec::new()
            }
            SyncResponse::Cycle(o) => self.sync_cycle_done(*o),
            SyncResponse::All(o) => self.sync_all_done(&o),
        }
    }

    fn sync_notices(&mut self, notices: Vec<Notice>) {
        for n in notices {
            let (text, importance) = crate::sync_engine::notice_text(self.cat(), &n);
            self.sync_say(text, importance);
        }
    }

    fn sync_cycle_done(&mut self, o: CycleOutcome) -> Vec<Effect> {
        self.sync.status = o.status.clone();
        self.sync_notices(o.notices.clone());
        let current = self
            .sync
            .doc
            .as_ref()
            .is_some_and(|d| Some(&d.key) == o.key.as_ref())
            && self.session.as_ref().map(|s| &s.key) == o.key.as_ref();
        if !current {
            // A document already closed: what arrived goes into its state
            // file, so the next merge starts from it.
            self.sync_apply_closed(&o);
            return Vec::new();
        }
        if let (Some(d), Some(id)) = (self.sync.doc.as_mut(), o.sync_id) {
            d.sync_id = Some(id);
        }
        let mut effects = Vec::new();
        if !o.arrivals.is_empty() {
            let applied = self.sync_apply(&o.arrivals);
            if applied.changed() {
                self.sync_say_applied(&applied);
                effects.push(Effect::Redraw);
            }
        }
        self.sync.places.clone_from(&o.places);
        if let Some(opened) = &o.opened {
            self.sync_resume(&o);
            for s in &opened.suggestions {
                if let Some(key) = o.key.clone() {
                    let label = self.sync_label(s.label.as_deref());
                    let title = s.title.clone().unwrap_or_else(|| self.msg("sync-untitled"));
                    self.sync_queue(SyncQuestion::Suggestion {
                        key,
                        sync_id: s.sync_id,
                        label,
                        title,
                        notes: s.notes,
                    });
                }
            }
        } else if o.places_changed {
            self.sync_offer_places();
        }
        effects.push(Effect::Redraw);
        effects
    }

    /// Applies arrivals to a document that is no longer open, in its state
    /// file (the replaced notes' backup included), without a word: the
    /// owner has moved on.
    fn sync_apply_closed(&mut self, o: &CycleOutcome) {
        let (Some(key), Some(store)) = (o.key.clone(), self.state_store()) else {
            return;
        };
        if o.arrivals.is_empty() {
            return;
        }
        let mut state = self
            .writer
            .queued_state(&store, &key)
            .or_else(|| store.load(&key))
            .unwrap_or_default();
        let applied = apply_arrivals(&mut state, &o.arrivals, &mut |_| true);
        if applied.changed() {
            self.writer.send(Job::State {
                store,
                key,
                state: Box::new(state),
                sync: None,
                note: crate::writer::StateNote::Quiet,
            });
        }
    }

    /// Applies arrivals to the open document, each found again in its text.
    fn sync_apply(&mut self, arrivals: &[crate::sync_engine::Arrival]) -> Applied {
        let Some(s) = self.session.as_mut() else {
            return Applied::default();
        };
        let mut st: DocState = s.saved.clone();
        st.bookmarks.clone_from(&s.bookmarks);
        st.notes.clone_from(&s.notes);
        st.highlights.clone_from(&s.highlights);
        let doc = &s.doc;
        let mut place = |item: &mut Item| relocate_item(doc, item);
        let applied = apply_arrivals(&mut st, arrivals, &mut place);
        if applied.changed() {
            s.bookmarks = st.bookmarks;
            s.notes = st.notes;
            s.highlights = st.highlights;
            s.saved.note_backups = st.note_backups;
            // Keep what arrived: the state file matches the merged view.
            self.save_state(crate::writer::StateNote::Quiet);
            // Nothing here changed: no need to publish again.
            self.sync.dirty = false;
        }
        applied
    }

    fn sync_say_applied(&mut self, a: &Applied) {
        let title = self
            .session
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_default();
        if let Some((_, label)) = a.replaced_notes.first() {
            let device = self.sync_label(label.as_deref());
            let n = a.replaced_notes.len();
            let msg = self.msg_args(
                "sync-note-replaced",
                &args!["title" => title.as_str(), "device" => device, "n" => n],
            );
            self.sync_say(msg, Importance::Result);
        }
        for (kind, id) in [
            (MarkKind::Note, "sync-restored-notes"),
            (MarkKind::Bookmark, "sync-restored-bookmarks"),
            (MarkKind::Highlight, "sync-restored-highlights"),
        ] {
            let of_kind: Vec<&Option<String>> = a
                .restored
                .iter()
                .filter(|(k, _)| *k == kind)
                .map(|(_, l)| l)
                .collect();
            if let Some(label) = of_kind.first() {
                let device = self.sync_label(label.as_deref());
                let msg = self.msg_args(
                    id,
                    &args!["title" => title.as_str(), "device" => device, "n" => of_kind.len()],
                );
                self.sync_say(msg, Importance::Result);
            }
        }
        let others = a.added.len() + a.removed.len() + a.replaced_other;
        if others > 0 {
            let label = a
                .added
                .iter()
                .chain(&a.removed)
                .map(|(_, l)| l.clone())
                .next()
                .flatten();
            let device = self.sync_label(label.as_deref());
            let msg = self.msg_args(
                "sync-arrived",
                &args!["title" => title.as_str(), "device" => device, "n" => others],
            );
            self.sync_say(msg, Importance::Routine);
        }
    }

    /// When the document opened: `position_policy` picks another
    /// computer's place. It is resumed only while the cursor is still where
    /// the document opened and nothing is being read; "ask" asks.
    fn sync_resume(&mut self, o: &CycleOutcome) {
        if !self.settings.sync.places || o.places.is_empty() {
            return;
        }
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let Some(doc) = self.sync.doc.as_ref() else {
            return;
        };
        let here = s.cursor;
        let here_pct = text_util::percent(&s.doc, here);
        let policy = self.settings.sync.position_policy;
        let candidate = match policy {
            PositionPolicy::Newest => o
                .places
                .first()
                .filter(|p| o.my_place.as_ref().is_none_or(|(mine, _)| p.stamp > *mine)),
            PositionPolicy::Furthest => o
                .places
                .iter()
                .max_by_key(|p| (p.place.pct, p.stamp))
                .filter(|p| p.place.pct > here_pct),
            PositionPolicy::Ask => o.places.first(),
        };
        let Some(p) = candidate.cloned() else {
            return;
        };
        let target = find_place(&s.doc, &p);
        let target = text_util::first_word_at_or_after(&s.doc, target);
        if text_util::word_start(&s.doc, here) == target {
            return;
        }
        let pct = text_util::percent(&s.doc, target);
        let label = self.sync_label(p.label.as_deref());
        let untouched = here == doc.opened_at
            && !matches!(self.playback, Playback::Reading)
            && self.settings.reading.auto_resume;
        if policy == PositionPolicy::Ask {
            let key = s.key.clone();
            self.sync_queue(SyncQuestion::Place {
                key,
                label,
                pos: target,
                pct,
                anchor: p.place.anchor.clone(),
            });
        } else if untouched {
            let title = s.title.clone();
            if let Some(s) = self.session.as_mut() {
                s.cursor = target;
                s.goal_column = None;
            }
            self.scroll_to_cursor();
            if let Some(d) = self.sync.doc.as_mut() {
                d.opened_at = target;
            }
            let msg = self.msg_args(
                "sync-resumed",
                &args!["title" => title, "pct" => pct, "device" => label],
            );
            self.sync_say(msg, Importance::Result);
            self.sync.places_said.push(p.device);
        } else {
            self.sync_offer_places();
        }
    }

    /// Says, once per computer, that another computer's place is there to
    /// go to: it never moves the cursor.
    fn sync_offer_places(&mut self) {
        if !self.settings.sync.places {
            return;
        }
        let Some(p) = self
            .sync
            .places
            .iter()
            .find(|p| !self.sync.places_said.contains(&p.device))
            .cloned()
        else {
            return;
        };
        self.sync.places_said.push(p.device);
        let label = self.sync_label(p.label.as_deref());
        let msg = self.msg_args(
            "sync-place-arrived",
            &args!["device" => label, "pct" => p.place.pct],
        );
        self.sync_say(msg, Importance::Routine);
    }

    /// Asks `q` now, or after the question asked, or at the pause.
    fn sync_queue(&mut self, q: SyncQuestion) {
        if self.sync.question.is_some() || matches!(self.playback, Playback::Reading) {
            self.sync.waiting.push(q);
        } else {
            self.sync_ask(q);
        }
    }

    fn sync_question_text(&self, q: &SyncQuestion) -> String {
        match q {
            SyncQuestion::Place { label, pct, .. } => self.msg_args(
                "sync-place-question",
                &args!["device" => label.as_str(), "pct" => *pct],
            ),
            SyncQuestion::Suggestion {
                label,
                title,
                notes,
                ..
            } => self.msg_args(
                "sync-suggestion-question",
                &args!["title" => title.as_str(), "device" => label.as_str(), "n" => *notes],
            ),
        }
    }

    fn sync_ask(&mut self, q: SyncQuestion) {
        let text = self.sync_question_text(&q);
        self.sync.question = Some(q);
        self.ask(&text);
    }

    /// The sync question waiting, if any (for the status line).
    pub(crate) fn sync_pending_question(&self) -> Option<String> {
        self.sync
            .question
            .as_ref()
            .map(|q| self.sync_question_text(q))
    }

    /// The answer to a sync question.
    pub(crate) fn confirm_sync(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(q) = self.sync.question.clone() else {
            return vec![Effect::Redraw];
        };
        if answer == Confirm::Repeat {
            let text = self.sync_question_text(&q);
            self.ask(&text);
            return vec![Effect::Redraw];
        }
        self.sync.question = None;
        let yes = answer == Confirm::Yes;
        let same_doc =
            |app: &App, key: &DocKey| app.session.as_ref().is_some_and(|s| s.key == *key);
        match q {
            SyncQuestion::Place {
                key,
                label,
                pos,
                pct,
                ..
            } => {
                if yes && same_doc(self, &key) {
                    let msg = self.msg_args(
                        "sync-went-to-place",
                        &args!["device" => label, "pct" => pct],
                    );
                    self.jump(pos, true, ReadAfter::Follow, &msg);
                } else {
                    let msg = self.msg("sync-kept-place");
                    self.tell(&msg);
                }
            }
            SyncQuestion::Suggestion {
                key,
                sync_id,
                label,
                ..
            } => {
                if yes && same_doc(self, &key) {
                    if let Some(snapshot) = self.sync_snapshot(true) {
                        if let Some(d) = self.sync.doc.as_mut() {
                            d.sync_id = Some(sync_id);
                        }
                        self.sync_send(SyncRequest::Adopt {
                            sync_id,
                            snapshot: Box::new(snapshot),
                        });
                    }
                    let msg = self.msg_args("sync-suggestion-accepted", &args!["device" => label]);
                    self.tell(&msg);
                } else {
                    self.sync_send(SyncRequest::Decline { key, sync_id });
                    let msg = self.msg("sync-suggestion-declined");
                    self.tell(&msg);
                }
            }
        }
        vec![Effect::Redraw]
    }

    /// The library folder's sidecar (the old place sync) found places that
    /// differed on write, or could not be written: said, not only logged
    /// (ADR-0049, problem 2).
    pub(crate) fn sidecar_reported(&mut self, result: Result<usize, String>) {
        match result {
            Ok(n) => {
                let msg = self.msg_args("sync-sidecar-differed", &args!["n" => n]);
                self.sync_say(msg, Importance::Result);
            }
            Err(error) => {
                let msg = self.msg_args("sync-sidecar-failed", &args!["error" => error]);
                self.sync_say(msg, Importance::Error);
            }
        }
    }

    /// The library sidecar's place differs and `position_policy` is "ask":
    /// ask whether to go there, instead of keeping this computer's quietly
    /// (ADR-0049, problem 1).
    pub(crate) fn sync_ask_sidecar_place(&mut self, pos: CharPos) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let target = text_util::first_word_at_or_after(&s.doc, pos.clamp_to(s.doc.len_chars()));
        if target == s.cursor {
            return;
        }
        let pct = text_util::percent(&s.doc, target);
        let key = s.key.clone();
        let label = self.msg("sync-another-computer");
        self.sync_queue(SyncQuestion::Place {
            key,
            label,
            pos: target,
            pct,
            anchor: None,
        });
    }

    // ----- Commands --------------------------------------------------------

    /// Runs a sync command.
    pub(crate) fn sync_action(&mut self, a: ActionId) -> Vec<Effect> {
        match a {
            ActionId::SyncSetup => self.sync_setup(),
            ActionId::SyncStatus => self.sync_status_command(),
            ActionId::SyncNow => self.sync_now(),
            ActionId::SyncGoToPlace => self.sync_places_list(),
            ActionId::SyncReplacedNotes => self.sync_replaced_list(),
            ActionId::SyncStop => self.sync_stop(),
            _ => vec![Effect::Redraw],
        }
    }

    fn sync_setup(&mut self) -> Vec<Effect> {
        if self.paths.is_none() {
            let msg = self.msg("sync-no-state");
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        let purpose = self.msg("sync-choose-folder");
        self.choose_folder(&purpose, Self::sync_folder_chosen)
    }

    /// Set up sync: the folder is chosen; the computer's name is next.
    fn sync_folder_chosen(app: &mut App, folder: PathBuf) -> Vec<Effect> {
        let current = app.settings.sync.device_name.trim().to_owned();
        let name = if current.is_empty() {
            let taken = crate::sync_engine::labels_elsewhere(&folder);
            textweaver_sync::default_label(taken.iter().map(String::as_str))
        } else {
            current
        };
        app.sync.setup = Some(Setup {
            folder,
            name: name.clone(),
            groups: Groups::from_settings(&app.settings.sync),
        });
        app.pending_prompt_text = Some(name);
        app.prompt(PromptPurpose::SyncComputerName)
    }

    /// Set up sync: the answer to the computer's name.
    pub(crate) fn answer_sync_name(&mut self, text: &str) -> Vec<Effect> {
        let Some(suggested) = self.sync.setup.as_ref().map(|s| s.name.clone()) else {
            return vec![Effect::Redraw];
        };
        let text = if text.trim().is_empty() {
            suggested.clone()
        } else {
            text.trim().to_owned()
        };
        match textweaver_sync::check_label(&text) {
            Ok(name) => {
                if let Some(setup) = self.sync.setup.as_mut() {
                    setup.name = name;
                }
                self.sync_groups_list(0)
            }
            Err(_) => {
                let msg = self.msg("sync-name-refused");
                self.error(&msg);
                self.pending_prompt_text = Some(suggested);
                self.prompt(PromptPurpose::SyncComputerName)
            }
        }
    }

    fn sync_groups_list(&mut self, focus: usize) -> Vec<Effect> {
        let Some(setup) = self.sync.setup.clone() else {
            return vec![Effect::Redraw];
        };
        let mut items: Vec<String> = GROUP_IDS
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let name = self.msg(id);
                let state = self.msg(if group_value(&setup.groups, i) {
                    "common-on"
                } else {
                    "common-off"
                });
                self.msg_args("sync-group-item", &args!["name" => name, "state" => state])
            })
            .collect();
        items.push(self.msg("sync-start"));
        let title = self.msg("sync-groups-title");
        if focus == 0 {
            let intro = self.msg_args(
                "sync-groups-intro",
                &args!["title" => title.as_str(), "name" => setup.name.as_str()],
            );
            self.tell(&intro);
        }
        self.pending_list_focus = Some(focus);
        self.list = Some(ListKind::Sync(SyncList::Groups));
        vec![Effect::ShowList { title, items }]
    }

    /// Enter in a sync list.
    pub(crate) fn choose_sync(&mut self, list: SyncList, n: usize) -> Vec<Effect> {
        match list {
            SyncList::Groups => {
                if n < GROUP_IDS.len() {
                    if let Some(setup) = self.sync.setup.as_mut() {
                        flip_group(&mut setup.groups, n);
                        let on = group_value(&setup.groups, n);
                        let name = self.msg(GROUP_IDS[n]);
                        let state = self.msg(if on { "common-on" } else { "common-off" });
                        let msg = self
                            .msg_args("sync-group-item", &args!["name" => name, "state" => state]);
                        self.tell(&msg);
                    }
                    return self.sync_groups_list(n.max(1));
                }
                self.sync_start()
            }
            SyncList::Places => {
                let Some(p) = self.sync.places.get(n).cloned() else {
                    return vec![Effect::Redraw];
                };
                let Some(s) = self.session.as_ref() else {
                    return vec![Effect::Redraw];
                };
                let target = text_util::first_word_at_or_after(&s.doc, find_place(&s.doc, &p));
                let pct = text_util::percent(&s.doc, target);
                let label = self.sync_label(p.label.as_deref());
                let msg = self.msg_args(
                    "sync-went-to-place",
                    &args!["device" => label, "pct" => pct],
                );
                self.jump(target, true, ReadAfter::Follow, &msg);
                vec![Effect::Redraw]
            }
            SyncList::Replaced(ids) => {
                let Some(id) = ids.get(n) else {
                    return vec![Effect::Redraw];
                };
                self.sync_restore_note(id)
            }
        }
    }

    fn sync_start(&mut self) -> Vec<Effect> {
        let Some(setup) = self.sync.setup.take() else {
            return vec![Effect::Redraw];
        };
        let y = &mut self.settings.sync;
        y.enabled = true;
        y.folder = Some(setup.folder);
        y.device_name = setup.name.clone();
        y.places = setup.groups.places;
        y.notes = setup.groups.notes;
        y.highlights = setup.groups.highlights;
        y.bookmarks = setup.groups.bookmarks;
        y.statistics = setup.groups.statistics;
        self.settings_dirty = true;
        if let Err(e) = self.save_settings() {
            log::warn!("cannot save settings: {e}");
        }
        self.list = None;
        let key = self.key(ActionId::SyncStatus);
        let msg = self.msg_args(
            "sync-started",
            &args!["name" => setup.name.as_str(), "key" => key],
        );
        self.say_result(&msg);
        self.sync_ensure_configured();
        vec![Effect::Redraw]
    }

    fn sync_status_command(&mut self) -> Vec<Effect> {
        let mut parts = vec![self.sync_status_line()];
        if self.sync_enabled() {
            let st = self.sync.status.clone();
            if !st.label.is_empty() {
                parts.push(self.msg_args(
                    "sync-status-this-computer",
                    &args!["name" => st.label.as_str()],
                ));
            }
            if st.others.is_empty() {
                parts.push(self.msg("sync-status-no-others"));
            } else {
                let names = st.others.join(", ");
                parts.push(self.msg_args("sync-status-others", &args!["names" => names]));
            }
            if let Some(e) = &st.write_error {
                parts.push(self.msg_args("sync-status-error", &args!["error" => e.as_str()]));
            }
            if let Some(e) = &st.failure {
                parts.push(self.msg_args("sync-status-error", &args!["error" => e.as_str()]));
            }
        } else if self.settings.sync.folder.is_none() {
            let msg = self.msg("sync-how-to-set-up");
            parts.push(msg);
        }
        let text = parts.join(" ");
        self.tell(&text);
        vec![Effect::Redraw]
    }

    fn sync_not_on(&mut self) -> bool {
        if self.sync_enabled() {
            return false;
        }
        let msg = self.msg("sync-how-to-set-up");
        self.tell(&msg);
        true
    }

    fn sync_now(&mut self) -> Vec<Effect> {
        if self.sync_not_on() {
            return vec![Effect::Redraw];
        }
        self.sync_ensure_configured();
        let skip = self.session.as_ref().map(|s| s.key.clone());
        if let Some(sync_id) = self.sync.doc.as_ref().and_then(|d| d.sync_id)
            && let Some(snapshot) = self.sync_snapshot(true)
        {
            self.sync.dirty = false;
            self.sync.last_publish = Some(Instant::now());
            self.sync_send(SyncRequest::Cycle {
                sync_id,
                snapshot: Box::new(snapshot),
                force: true,
            });
        }
        self.sync.all_pending = true;
        self.sync_send(SyncRequest::All { skip });
        let msg = self.msg("sync-now-started");
        self.note(&msg);
        vec![Effect::Redraw]
    }

    fn sync_all_done(&mut self, o: &AllOutcome) -> Vec<Effect> {
        self.sync.status = o.status.clone();
        self.sync_notices(o.notices.clone());
        if !std::mem::take(&mut self.sync.all_pending) {
            return Vec::new();
        }
        let msg = if o.status.kind != StatusKind::Ready && o.status.kind != StatusKind::ReadOnly {
            self.sync_status_line()
        } else if o.changed == 0 {
            self.msg_args("sync-now-done", &args!["n" => o.documents])
        } else {
            self.msg_args("sync-now-changed", &args!["n" => o.changed])
        };
        self.sync_say(msg, Importance::Result);
        vec![Effect::Redraw]
    }

    fn sync_places_list(&mut self) -> Vec<Effect> {
        if self.sync_not_on() {
            return vec![Effect::Redraw];
        }
        if self.session.is_none() {
            let open = self.key(ActionId::Open);
            let msg = self.msg_args("app-no-document-open", &args!["key" => open]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if self.sync.places.is_empty() {
            let msg = self.msg("sync-no-places");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let places = self.sync.places.clone();
        let items: Vec<String> = places
            .iter()
            .map(|p| {
                let label = self.sync_label(p.label.as_deref());
                self.msg_args(
                    "sync-place-item",
                    &args!["device" => label, "pct" => p.place.pct],
                )
            })
            .collect();
        let title = self.msg_args("sync-places-title", &args!["n" => items.len()]);
        self.say_result(&title);
        self.list = Some(ListKind::Sync(SyncList::Places));
        vec![Effect::ShowList { title, items }]
    }

    fn sync_replaced_list(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            let open = self.key(ActionId::Open);
            let msg = self.msg_args("app-no-document-open", &args!["key" => open]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let backups: Vec<(String, String, String, bool)> = s
            .saved
            .note_backups_newest_first()
            .into_iter()
            .map(|b| {
                (
                    b.id.clone(),
                    crate::notes::collapse(&b.note.note, 60),
                    b.by.clone(),
                    b.deleted,
                )
            })
            .collect();
        if backups.is_empty() {
            let msg = self.msg("sync-no-replaced");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items: Vec<String> = backups
            .iter()
            .map(|(_, text, by, deleted)| {
                let device = self.sync_label((!by.is_empty()).then_some(by.as_str()));
                let id = if *deleted {
                    "sync-replaced-item-deleted"
                } else {
                    "sync-replaced-item"
                };
                self.msg_args(id, &args!["text" => text.as_str(), "device" => device])
            })
            .collect();
        let title = self.msg_args("sync-replaced-title", &args!["n" => items.len()]);
        let intro = self.msg_args("sync-replaced-intro", &args!["title" => title.as_str()]);
        self.say_result(&intro);
        self.list = Some(ListKind::Sync(SyncList::Replaced(
            backups.into_iter().map(|b| b.0).collect(),
        )));
        vec![Effect::ShowList { title, items }]
    }

    /// Puts a replaced note's text back, as a new edit; the text it
    /// replaces is kept in turn.
    fn sync_restore_note(&mut self, backup_id: &str) -> Vec<Effect> {
        let Some(s) = self.session.as_mut() else {
            return vec![Effect::Redraw];
        };
        let mut st = s.saved.clone();
        st.notes.clone_from(&s.notes);
        let Some(note) = st.restore_note_backup(backup_id) else {
            return vec![Effect::Redraw];
        };
        s.notes = st.notes;
        let len = s.doc.len_chars();
        for n in &mut s.notes {
            n.range = n.range.clamp_to(len);
        }
        s.saved.note_backups = st.note_backups;
        self.persist_marks();
        let text = crate::notes::collapse(&note.note, 40);
        let msg = self.msg_args("sync-note-restored", &args!["text" => text]);
        self.say_result(&msg);
        self.list = None;
        vec![Effect::Redraw]
    }

    fn sync_stop(&mut self) -> Vec<Effect> {
        if !self.settings.sync.enabled {
            let msg = self.msg("sync-already-off");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.sync_leave_document();
        self.settings.sync.enabled = false;
        self.settings_dirty = true;
        if let Err(e) = self.save_settings() {
            log::warn!("cannot save settings: {e}");
        }
        self.sync_ensure_configured();
        let msg = self.msg("sync-stopped");
        self.say_result(&msg);
        vec![Effect::Redraw]
    }

    /// The sync engine, for tests that drive two computers.
    #[doc(hidden)]
    pub fn sync_idle(&self) -> bool {
        self.sync.in_flight == 0
    }
}

/// Finds another computer's place in this computer's text: by the text at
/// the place when it is still there, else by its position.
fn find_place(doc: &textweaver_text::Document, p: &OtherPlace) -> CharPos {
    let mut st = DocState {
        position: p.place.pos.clamp_to(doc.len_chars()),
        pct: p.place.pct,
        anchor: p.place.anchor.clone(),
        ..DocState::default()
    };
    let _ = crate::relocate::relocate(&mut st, doc);
    st.position.clamp_to(doc.len_chars())
}

/// Finds an arriving item again in this computer's text (`crate::relocate`,
/// the rules an outside edit uses). A highlight whose range comes out empty
/// is not kept.
fn relocate_item(doc: &textweaver_text::Document, item: &mut Item) -> bool {
    let len = doc.len_chars();
    let mut st = DocState::default();
    match item {
        Item::Bookmark(b) => st.bookmarks.push(b.clone()),
        Item::Note(n) => st.notes.push((**n).clone()),
        Item::Highlight(h) => st.highlights.push(h.clone()),
    }
    let _ = crate::relocate::relocate(&mut st, doc);
    match item {
        Item::Bookmark(b) => {
            if let Some(x) = st.bookmarks.pop() {
                *b = x;
            }
            b.pos = b.pos.clamp_to(len);
            true
        }
        Item::Note(n) => {
            if let Some(x) = st.notes.pop() {
                **n = x;
            }
            n.range = n.range.clamp_to(len);
            true
        }
        Item::Highlight(h) => {
            if let Some(x) = st.highlights.pop() {
                *h = x;
            }
            h.range = h.range.clamp_to(len);
            !h.range.is_empty()
        }
    }
}
