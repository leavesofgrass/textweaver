//! The library (`open_library`, Alt+L in the terminal): documents in the
//! library folders and recently opened files, as one list; Enter opens one.
//!
//! Opening any document records it on the bookshelf (`library.json`,
//! [`Library::record_open`]) and the recent list. Documents inside a library
//! folder also sync their reading position through the folder's sidecar
//! (`<folder>/.textweaver/progress.json`, [`LibrarySync`]): the position is
//! mirrored there whenever it is saved, the sidecars are flushed on
//! document switch and quit, and on open the local and synced positions are
//! chosen between by `reading.sync_conflict_policy` (C2's rule: newest,
//! highest progress, or manual, which keeps this device's position and
//! says that another device differs).

use std::path::{Path, PathBuf};

use textweaver_store::library::{self, LibraryItem, ResumeSource};
use textweaver_store::sync::Resolution;
use textweaver_store::{DocKey, DocState, Library, LibrarySync, Recent};

use crate::app::{App, ListKind};
use crate::command::Effect;

/// Where a document resumes, and how to say so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResumePoint {
    /// Char offset to resume at.
    pub(crate) pos: textweaver_core::CharPos,
    /// The position came from another device (the folder's sidecar).
    pub(crate) synced: bool,
    /// Another device's position differs and nothing was chosen (the
    /// `manual` policy): this device's is used.
    pub(crate) unresolved: bool,
}

impl App {
    /// A library sync for the configured folders and conflict policy.
    pub(crate) fn make_library_sync(settings: &textweaver_store::Settings) -> LibrarySync {
        LibrarySync::new(
            &settings.library.folders,
            settings.reading.sync_conflict_policy,
        )
    }

    /// Where `path` should resume: the local saved position or the one
    /// synced through its library folder, by the conflict policy.
    pub(crate) fn resume_point(
        &self,
        path: &Path,
        local: Option<&DocState>,
    ) -> Option<ResumePoint> {
        let local_pos = local
            .map(|s| s.position)
            .filter(|p| *p > textweaver_core::CharPos::ZERO);
        match self.library_sync.resume(path, local) {
            Some(r) => Some(ResumePoint {
                pos: match r.source {
                    ResumeSource::Sidecar => r.pos,
                    // A local state saved without a timestamp still counts.
                    ResumeSource::Local => local_pos.unwrap_or(r.pos),
                },
                synced: r.source == ResumeSource::Sidecar,
                unresolved: r
                    .conflict
                    .is_some_and(|c| c.resolution == Resolution::Unresolved),
            }),
            None => local_pos.map(|pos| ResumePoint {
                pos,
                synced: false,
                unresolved: false,
            }),
        }
    }

    /// Records an opened document on the bookshelf (`library.json`).
    pub(crate) fn record_library_open(&self, path: &Path, title: &str, format: &str) {
        let Some(paths) = &self.paths else {
            return;
        };
        let file = paths.library_file();
        match Library::load(&file) {
            Ok(mut lib) => {
                lib.record_open(path, title, format);
                if let Err(e) = lib.save(&file) {
                    log::warn!("cannot save the library: {e}");
                }
            }
            // An unreadable library is left alone rather than overwritten.
            Err(e) => log::warn!("cannot read the library: {e}"),
        }
    }

    /// Mirrors a saved state into the library folder's sidecar, when the
    /// document is in a library folder.
    pub(crate) fn sync_position(&self, path: &Path, state: &DocState) {
        if let Err(e) = self.library_sync.record(path, state, None) {
            log::warn!("cannot sync the reading position: {e}");
        }
    }

    /// Writes pending sidecars (document switch and quit).
    pub(crate) fn flush_library_sync(&self) {
        match self.library_sync.flush() {
            Ok(conflicts) if !conflicts.is_empty() => {
                log::info!("{} sidecar entries differed on write", conflicts.len());
            }
            Ok(_) => {}
            Err(e) => log::warn!("cannot write the library sidecar: {e}"),
        }
    }

    /// The library's documents: every document in the library folders,
    /// then recently opened ones, newest first.
    pub fn library_items(&self) -> Vec<LibraryItem> {
        let exts = self.registry.extensions();
        let supported = |ext: &str| exts.contains(&ext);
        let scanned = library::scan_library(&self.settings.library.folders, &supported);
        let (lib, recent) = match &self.paths {
            Some(p) => (
                Library::load(&p.library_file()).unwrap_or_default(),
                Recent::load(&p.recent_file()),
            ),
            None => (Library::default(), Recent::default()),
        };
        let states = self.state_store();
        let local = |p: &Path| {
            states
                .as_ref()?
                .load(&DocKey::for_path(p))
                .filter(DocState::has_position)
                .map(|s| s.pct)
        };
        library::library_view(
            &scanned,
            &lib,
            &recent,
            self.library_sync.sidecars(),
            &local,
        )
    }

    /// Shows the library list (`open_library`).
    pub(crate) fn open_library(&mut self) -> Vec<Effect> {
        let items = self.library_items();
        if items.is_empty() {
            self.tell(
                "The library is empty. Add a folder with tw library --add, or open a file with Control O.",
            );
            return vec![Effect::Redraw];
        }
        let n = items.len();
        let paths: Vec<PathBuf> = items.iter().map(|i| i.path.clone()).collect();
        let lines: Vec<String> = items.iter().map(LibraryItem::describe).collect();
        self.list = Some(ListKind::Library(paths));
        self.tell(&format!(
            "Library, {n} {}. Enter opens one.",
            if n == 1 { "document" } else { "documents" }
        ));
        vec![Effect::ShowList {
            title: "Library".into(),
            items: lines,
        }]
    }
}
