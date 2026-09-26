//! Announcements through the screen reader: the `live-region` crate raises a
//! UI Automation notification on Windows, an accessibility announcement on
//! macOS, and an ATK notification on Linux, from a hidden label in the frame.

use std::cell::Cell;
use std::rc::Rc;

use live_region::Priority as LivePriority;
use textweaver_app::a11y::{Announcer, Priority};
use wxdragon::prelude::StaticText;

/// Sends the app's announcements to the screen reader.
///
/// `muted` silences it while the GUI moves the app's cursor to follow the
/// native caret (a bookkeeping dispatch the user should not hear). With
/// `log`, every announcement is also written to the diagnostic log, which
/// the automated checks read.
pub struct LiveRegionAnnouncer {
    label: StaticText,
    muted: Rc<Cell<bool>>,
    log: bool,
}

impl LiveRegionAnnouncer {
    /// Announces through `label`, which must have been passed to
    /// `live_region::set_live_region`.
    pub fn new(label: StaticText, muted: Rc<Cell<bool>>, log: bool) -> Self {
        LiveRegionAnnouncer { label, muted, log }
    }
}

/// Polite announcements let the current utterance finish (NVDA: after the
/// current speech, superseding staler queued ones); assertive ones interrupt.
fn live_priority(priority: Priority) -> LivePriority {
    match priority {
        Priority::Polite => LivePriority::Medium,
        Priority::Assertive => LivePriority::High,
    }
}

impl Announcer for LiveRegionAnnouncer {
    fn announce(&mut self, text: &str, priority: Priority) {
        let muted = self.muted.get();
        if self.log {
            let state = if muted { " (muted)" } else { "" };
            crate::log::line(&format!("announce {priority:?}{state}: {text}"));
        }
        if !muted {
            live_region::announce_with_priority(self.label, text, live_priority(priority));
        }
    }
}
