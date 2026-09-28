//! Hearing the status again (Wave 4, Agent W4h): the title line's parts,
//! the "say status" and "repeat message" actions.
//!
//! A screen reader user can read the status line and the title line again
//! with the screen reader's own keys; a self-voicing user had nothing
//! (usability pass, item 3). Two actions, in both frontends and the
//! command palette:
//!
//! - **Repeat message** says the last message again, as the status line
//!   shows it ("Opened essay. Ctrl+Q quits."), keys spoken by name.
//! - **Say status** says the last message, then the title line's parts:
//!   the mode, whether the document is modified, the reading state, the
//!   position, the accessibility mode, the rate, and the speech engine.
//!   In an open list it says the list's introduction instead.
//!
//! In a list, [`ListKey::Introduce`] (F1 and the Say Status key in the
//! terminal) repeats the list's introduction: its title, how many items it
//! has, and the keys it takes, then the focused item ("3 of 12"). The
//! introduction is the message said when the list was shown; a list shown
//! without one gets "Title, 12 items." and the keys every list takes.
//!
//! [`ListKey::Introduce`]: crate::ListKey::Introduce
//!
//! Both are assertive, so they are heard over the reading, which then goes
//! on, as a question is.

use textweaver_a11y::{AccessMode, Priority, Verbosity};

use crate::app::{App, Mode};
use crate::command::Effect;

impl App {
    /// The position the title line shows: "line 3 of 40, 7%"; `None`
    /// without a document.
    pub fn title_position(&self) -> Option<String> {
        let s = self.session.as_ref()?;
        Some(format!(
            "line {} of {}, {}%",
            s.line() + 1,
            crate::text_util::line_count(&s.doc),
            s.percent()
        ))
    }

    /// The parts of the title line after the document's name, most
    /// important first: the mode (unless browse), "modified", the reading
    /// state ([`App::reading_state`]), `position`, the accessibility mode
    /// (unless self-voicing), the rate, and the speech engine. Frontends
    /// draw them joined with commas and drop trailing parts when narrow;
    /// "say status" speaks them.
    pub fn title_parts(&self, position: Option<&str>) -> Vec<String> {
        self.status_parts(position, false)
    }

    /// The title line's parts; `spoken` names every mode (browse and
    /// self-voicing too) and says the rate in words.
    fn status_parts(&self, position: Option<&str>, spoken: bool) -> Vec<String> {
        let mut parts = Vec::new();
        if spoken {
            parts.push(format!("{} mode", self.mode.name()));
        } else if self.mode != Mode::Browse {
            parts.push(self.mode.name().to_owned());
        }
        if self.is_dirty() {
            parts.push("modified".to_owned());
        }
        parts.push(self.reading_state().to_owned());
        if let Some(p) = position {
            parts.push(p.to_owned());
        }
        match self.access_mode {
            AccessMode::SelfVoicing if spoken => parts.push("self-voicing".to_owned()),
            AccessMode::SelfVoicing => {}
            AccessMode::Hybrid => parts.push("hybrid".to_owned()),
            AccessMode::ScreenReader => parts.push("screen reader mode".to_owned()),
        }
        let wpm = self.settings.speech.rate.wpm();
        parts.push(if spoken {
            format!("{wpm} words per minute")
        } else {
            format!("{wpm} wpm")
        });
        parts.push(self.backend_name.clone());
        parts
    }

    /// The Say Status action: the last message, then the title line's
    /// parts. In a list, the list's introduction and the focused item.
    pub(crate) fn say_status(&mut self) -> Vec<Effect> {
        if self.list_model.is_some() {
            return self.repeat_list_introduction();
        }
        let position = self.title_position();
        let title = self
            .session
            .as_ref()
            .map_or("No document", |s| s.title.as_str())
            .to_owned();
        let parts = self.status_parts(position.as_deref(), true);
        let status = format!("{title}: {}.", parts.join(", "));
        let msg = match self.last_message.as_deref() {
            Some(last) if !last.trim().is_empty() => format!("{} {status}", with_stop(last)),
            _ => status,
        };
        self.say_unremembered(&msg);
        vec![Effect::Redraw]
    }

    /// The Repeat Message action: the last message again.
    pub(crate) fn repeat_message(&mut self) -> Vec<Effect> {
        match self.last_message.clone().filter(|m| !m.trim().is_empty()) {
            Some(last) => self.say_unremembered(&last),
            None => self.say_unremembered("No message yet."),
        }
        vec![Effect::Redraw]
    }

    /// Says `text` assertively (heard over the reading) at any verbosity,
    /// keeping the last message as it was, so "say status" twice does not
    /// say the status twice over, and "repeat message" after it repeats
    /// the message, not the status.
    pub(crate) fn say_unremembered(&mut self, text: &str) {
        let last = self.last_message.take();
        self.say_at(text, Verbosity::Low, Priority::Assertive);
        self.last_message = last;
    }

    /// The list's introduction, then its focused item, heard over the
    /// reading.
    pub(crate) fn repeat_list_introduction(&mut self) -> Vec<Effect> {
        let Some(list) = self.list_model.as_ref() else {
            return vec![Effect::Redraw];
        };
        let intro = self.list_intro.clone().unwrap_or_else(|| {
            let n = list.items.len();
            let items = if n == 1 { "item" } else { "items" };
            format!(
                "{}, {n} {items}. Up and Down move, Enter chooses, Escape closes.",
                list.title
            )
        });
        let msg = match list.spoken_item() {
            Some(item) => format!("{} {item}.", with_stop(&intro)),
            None => with_stop(&intro),
        };
        self.say_unremembered(&msg);
        vec![Effect::Redraw]
    }

    /// Remembers `text` as the last message ([`App::repeat_message`]);
    /// `queued` adds it after the one before, as the status line shows a
    /// queued announcement.
    pub(crate) fn remember_message(&mut self, text: &str, queued: bool) {
        self.messages_said = self.messages_said.wrapping_add(1);
        self.last_message = Some(match self.last_message.take() {
            Some(before) if queued && !before.is_empty() => format!("{before} {text}"),
            _ => text.to_owned(),
        });
    }
}

/// `text` ending with a full stop, so the status after it is a new
/// sentence when spoken.
fn with_stop(text: &str) -> String {
    let t = text.trim_end();
    if t.ends_with(['.', '?', '!']) {
        t.to_owned()
    } else {
        format!("{t}.")
    }
}

#[cfg(test)]
mod tests {
    use crate::{App, AppConfig, ListModel};

    /// A list shown without an introduction still has one to repeat: its
    /// title, count, and the keys every list takes.
    #[test]
    fn a_list_without_an_introduction_gets_a_plain_one() {
        let mut app = App::new(AppConfig::for_tests());
        app.list_model = Some(ListModel::new("Things", vec!["a".into(), "b".into()]));
        app.list_intro = None;
        app.repeat_list_introduction();
        assert_eq!(
            app.status_text(),
            "Things, 2 items. Up and Down move, Enter chooses, Escape closes. a, 1 of 2."
        );
        // Repeating it is not a new message to repeat.
        assert_eq!(app.last_message, None);
    }

    #[test]
    fn a_message_ends_with_a_stop() {
        assert_eq!(super::with_stop("Opened essay"), "Opened essay.");
        assert_eq!(super::with_stop("Quit? y or n?"), "Quit? y or n?");
        assert_eq!(super::with_stop("Paused. "), "Paused.");
    }
}
