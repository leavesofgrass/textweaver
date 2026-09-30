//! A list only a frontend knows, on the app's list model (Wave 6, W6a6):
//! the window's font families, which it reads from the system. Shown
//! through [`App::show_frontend_list`], the list gets everything the app's
//! lists have (type to filter, Home and End, the Help key repeating its
//! introduction, the same announcements in every frontend); the choice
//! comes back through [`App::take_frontend_choice`], and the frontend acts
//! on it.

use textweaver_a11y::Importance;

use crate::app::{App, ListKind};
use crate::command::Effect;

impl App {
    /// Shows `items` as the app's list titled `title`, starting on
    /// `selected`, and says `intro` (the list's name and size, a result of
    /// the command that opened it). Enter chooses an item; the frontend
    /// reads which with [`take_frontend_choice`](Self::take_frontend_choice).
    pub fn show_frontend_list(
        &mut self,
        title: &str,
        intro: &str,
        items: Vec<String>,
        selected: usize,
    ) -> Vec<Effect> {
        self.entry(|app| {
            app.list = Some(ListKind::Frontend);
            app.frontend_choice = None;
            app.pending_list_focus = Some(selected.min(items.len().saturating_sub(1)));
            app.announce_as(intro, textweaver_a11y::Priority::Polite, Importance::Result);
            vec![Effect::ShowList {
                title: title.to_owned(),
                items,
            }]
        })
    }

    /// The item chosen in the list [`show_frontend_list`](Self::show_frontend_list)
    /// showed, once: `None` until one is chosen (or when it was closed).
    pub fn take_frontend_choice(&mut self) -> Option<usize> {
        self.frontend_choice.take()
    }

    /// Enter on an item of a frontend's list: kept for the frontend, and
    /// the list closes.
    pub(crate) fn choose_frontend_item(&mut self, n: usize) {
        self.frontend_choice = Some(n);
    }
}

#[cfg(test)]
mod tests {
    use crate::{AppConfig, Command, ListKey};

    use super::*;

    #[test]
    fn a_frontend_list_gives_back_its_choice() {
        let mut app = App::new(AppConfig::for_tests());
        let items = vec!["Atkinson Hyperlegible".into(), "OpenDyslexic".into()];
        let _ = app.show_frontend_list("Font", "Font, 2 families.", items, 1);
        let model = app.list_model().expect("the list is shown");
        assert_eq!(model.selected, 1);
        assert_eq!(app.take_frontend_choice(), None);
        let _ = app.dispatch(Command::ListKey(ListKey::Enter));
        assert!(app.list_model().is_none(), "the list closes");
        assert_eq!(app.take_frontend_choice(), Some(1));
        assert_eq!(app.take_frontend_choice(), None, "taken once");
        // Escape closes it with no choice.
        let _ = app.show_frontend_list("Font", "Font, 1 family.", vec!["A".into()], 0);
        let _ = app.dispatch(Command::ListKey(ListKey::Escape));
        assert!(app.list_model().is_none());
        assert_eq!(app.take_frontend_choice(), None);
    }
}
