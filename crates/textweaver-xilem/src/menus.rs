//! The window's menus, from the app's one menu model (ADR-0043, ADR-0046).
//!
//! The app core owns the menus: File, Edit, View, Reading, Speech, Tools,
//! and Help, each a list of commands, submenus, and separators, labelled in
//! the interface's language with each command's key read from the live
//! keymap (`App::menu_bar`, `App::menu_view`). This module never keeps a
//! second list of commands; it only turns the model into what each system
//! shows:
//!
//! - **Windows:** a native menu bar (a Win32 `HMENU`, through `muda`). UI
//!   Automation exposes it as MenuBar, Menu, and MenuItem, with the access
//!   key from the `&` letter and the shortcut as `AcceleratorKey` from the
//!   text after a tab. Alt, F10, and Alt with a menu's letter enter it, as
//!   in any Windows program. The shortcut is only shown: the keymap stays
//!   the one place keys are handled, so no accelerator table duplicates it.
//! - **macOS:** the application's menu bar (an `NSMenu`), with each key as
//!   the item's key equivalent, taken from the same keymap.
//! - **Linux, and wherever no native menu could be attached:** the app's
//!   list menu (F10), shown in the window's list dialog, with the keys the
//!   terminal uses: Enter or Right opens, a letter moves to its access key,
//!   Left or Backspace goes up, Escape closes.
//!
//! [`tree`] is the part every system shares, and what the tests check:
//! every item's text, key, and check state come from the model, and every
//! key shown is a key the keymap binds to that command.

use std::path::PathBuf;

use textweaver_app::App;
use textweaver_app::keymap::{ActionId, KeyChord};
use textweaver_app::lexicon::args;
use textweaver_app::menu::{MenuId, MenuItem, MenuItemKind, menu_chord};

/// How deep submenus go: the model nests two levels (View, Reading aids,
/// RSVP); a deeper one would be a mistake in the model, not drawn.
const MAX_DEPTH: usize = 4;

/// What a menu item does when it is chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Runs a command (`Command::RunCommand`, so it joins the recent
    /// commands).
    Command(ActionId),
    /// Opens a recent document.
    Document(PathBuf),
}

impl Pick {
    /// The id the native menu gives this item, and gives back when it is
    /// chosen: `a:open`, or `d:` and the document's path.
    pub fn id(&self) -> String {
        match self {
            Pick::Command(a) => format!("a:{}", a.id()),
            Pick::Document(p) => format!("d:{}", p.display()),
        }
    }

    /// The item an id names ([`Pick::id`]).
    pub fn from_id(id: &str) -> Option<Pick> {
        if let Some(a) = id.strip_prefix("a:") {
            return ActionId::from_id(a).map(Pick::Command);
        }
        id.strip_prefix("d:")
            .filter(|p| !p.is_empty())
            .map(|p| Pick::Document(PathBuf::from(p)))
    }
}

/// One entry of a native menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// An item that does something.
    Item {
        /// What it does.
        pick: Pick,
        /// Its label, with the access key marked `&` (a literal `&` is
        /// doubled), and a choice's value after a colon.
        label: String,
        /// The key, written as the platform writes it ("Ctrl+O").
        keys: Option<String>,
        /// The key as a chord (for macOS's key equivalents).
        chord: Option<KeyChord>,
        /// For a toggle, whether it is on.
        checked: Option<bool>,
    },
    /// A line of text that does nothing ("No recent documents"), shown
    /// dimmed.
    Note(String),
    /// A submenu.
    Submenu {
        /// Its label, access key marked.
        label: String,
        /// Its entries.
        entries: Vec<Entry>,
    },
    /// A line between groups.
    Separator,
}

impl Entry {
    /// The text a Windows menu item takes: the label, then a tab and the
    /// key, which UI Automation gives screen readers as the item's
    /// `AcceleratorKey` and draws at the right of the menu.
    pub fn windows_text(&self) -> String {
        match self {
            Entry::Item {
                label,
                keys: Some(k),
                ..
            } => format!("{label}\t{k}"),
            Entry::Item { label, .. } | Entry::Submenu { label, .. } => label.clone(),
            Entry::Note(text) => text.replace('&', "&&"),
            Entry::Separator => String::new(),
        }
    }
}

/// A menu on the menu bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopMenu {
    /// Which menu.
    pub id: MenuId,
    /// Its title, access key marked (`&File`).
    pub title: String,
    /// Its entries, submenus filled in.
    pub entries: Vec<Entry>,
}

/// The whole menu bar as the native menus show it, from the app's model in
/// the interface's language, with keys from the live keymap and each
/// toggle's state. Two calls give equal trees until something the menus
/// show has changed, which is how the window knows to rebuild them.
pub fn tree(app: &App) -> Vec<TopMenu> {
    app.menu_bar()
        .into_iter()
        .map(|view| TopMenu {
            id: view.id,
            title: view.marked_title(),
            entries: entries(app, &view.items, 0),
        })
        .collect()
}

fn entries(app: &App, items: &[MenuItem], depth: usize) -> Vec<Entry> {
    let c = app.catalog();
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let label = || match &item.value {
            // "Reading ruler: current line": the access key stays on the
            // name, and a `&` in the value is not taken as one.
            Some(v) => c.fmt(
                "menu-value",
                &args!["name" => item.marked_label(), "value" => v.replace('&', "&&")],
            ),
            None => item.marked_label(),
        };
        match &item.kind {
            // A command only the terminal has (line numbers, scrolling by
            // lines) is left out of the window's menus.
            MenuItemKind::Action(a) if !in_window(*a) => {}
            MenuItemKind::Action(a) => out.push(Entry::Item {
                pick: Pick::Command(*a),
                label: label(),
                keys: item.keys.clone(),
                chord: menu_chord(app.keymap(), *a),
                checked: item.checked,
            }),
            MenuItemKind::Document(p) => out.push(Entry::Item {
                pick: Pick::Document(p.clone()),
                label: label(),
                keys: None,
                chord: None,
                checked: None,
            }),
            MenuItemKind::Submenu(m) if depth < MAX_DEPTH => {
                let view = app.menu_view(*m);
                out.push(Entry::Submenu {
                    label: label(),
                    entries: entries(app, &view.items, depth + 1),
                });
            }
            MenuItemKind::Submenu(_) => {}
            MenuItemKind::Separator => out.push(Entry::Separator),
            MenuItemKind::Empty => out.push(Entry::Note(item.label.clone())),
        }
    }
    // No separator first, last, or twice in a row (a command left out can
    // leave one alone), as the model keeps them.
    let mut tidy: Vec<Entry> = Vec::with_capacity(out.len());
    for e in out {
        if e == Entry::Separator && tidy.last().is_none_or(|l| *l == Entry::Separator) {
            continue;
        }
        tidy.push(e);
    }
    if tidy.last() == Some(&Entry::Separator) {
        tidy.pop();
    }
    tidy
}

/// True when the window runs `a` (every command but the few only the
/// terminal has, [`crate::parity`]).
pub fn in_window(a: ActionId) -> bool {
    !matches!(
        crate::parity::support(a),
        crate::parity::Support::TerminalOnly(_)
    )
}

/// What the menus show depends on: the settings (toggles, choices, the
/// language, the theme), the keymap, the live modes, the document open
/// (the recent documents), and which pending commands have their module.
/// Building the tree takes a few milliseconds (each toggle reads the
/// settings schema); comparing this takes microseconds, so the window
/// builds the tree again only when this changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Fingerprint {
    settings: textweaver_app::store::Settings,
    keymap: textweaver_app::keymap::Keymap,
    mode: textweaver_app::Mode,
    editing: bool,
    rsvp: bool,
    document: Option<textweaver_app::store::DocKey>,
    available: Vec<bool>,
}

impl Fingerprint {
    /// The fingerprint of what `app`'s menus show now.
    pub fn of(app: &App) -> Fingerprint {
        Fingerprint {
            settings: app.settings().clone(),
            keymap: app.keymap().clone(),
            mode: app.mode(),
            editing: app.is_editing(),
            rsvp: app.rsvp().is_some(),
            document: app.session().map(|s| s.key.clone()),
            available: textweaver_app::menu::PENDING
                .iter()
                .map(|a| app.is_available(*a))
                .collect(),
        }
    }
}

/// Every command a tree offers, depth first.
pub fn commands(tree: &[TopMenu]) -> Vec<ActionId> {
    fn walk(entries: &[Entry], out: &mut Vec<ActionId>) {
        for e in entries {
            match e {
                Entry::Item {
                    pick: Pick::Command(a),
                    ..
                } => out.push(*a),
                Entry::Submenu { entries, .. } => walk(entries, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for m in tree {
        walk(&m.entries, &mut out);
    }
    out
}

/// The command on row `row` of the list menu, while the app shows the
/// menus as a list (the terminal's F10 list, and the window's on Linux)
/// and that row runs a command. The rows are the menu's items without its
/// separators, as the app lists them. The window runs its own commands
/// (the text size, the font, the settings and colors dialogs) itself.
pub fn list_row_command(app: &App, row: usize) -> Option<ActionId> {
    let menu = *app.menu_path()?.last()?;
    app.menu_view(menu)
        .items
        .into_iter()
        .filter(|i| i.kind != MenuItemKind::Separator)
        .nth(row)
        .and_then(|i| match i.kind {
            MenuItemKind::Action(a) => Some(a),
            _ => None,
        })
}

/// A menu item was chosen: the item's id ([`Pick::id`]), posted to the
/// event loop from the native menu's handler.
#[derive(Debug)]
pub struct MenuPicked(pub String);

/// True when this system shows the menus natively (Windows and macOS);
/// elsewhere the menu key opens the list menu.
pub const NATIVE: bool = cfg!(any(windows, target_os = "macos"));

pub use native::{Native, listen};

/// The letters that open the top menus (their access keys), lowercase.
pub fn access_letters(tree: &[TopMenu]) -> Vec<char> {
    tree.iter()
        .filter_map(|m| textweaver_app::menu::split_access(&m.title).1)
        .collect()
}

/// Windows: enters the menu bar as a key would, through the window's own
/// menu handling (`WM_SYSCOMMAND` with `SC_KEYMENU`): `key` 0 selects the
/// bar (as F10 does), a menu's letter opens that menu (as Alt with it
/// does), and a space opens the system menu (as Alt+Space does). winit
/// keeps the character of an Alt chord from Windows, so the window posts
/// it itself for the chords the keymap leaves free.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn enter_menu_bar(hwnd: isize, key: char) -> bool {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, SC_KEYMENU, WM_SYSCOMMAND};
    if hwnd == 0 {
        return false;
    }
    // SAFETY: posting a message to this process's own window.
    unsafe {
        PostMessageW(
            Some(HWND(hwnd as *mut core::ffi::c_void)),
            WM_SYSCOMMAND,
            WPARAM(SC_KEYMENU as usize),
            LPARAM(key as isize),
        )
    }
    .is_ok()
}

/// Elsewhere there is no menu bar to enter.
#[cfg(not(windows))]
pub fn enter_menu_bar(_hwnd: isize, _key: char) -> bool {
    false
}

#[cfg(not(any(windows, target_os = "macos")))]
mod native {
    use masonry_winit::app::{EventLoopProxy, WindowId};
    use masonry_winit::winit::window::Window as WinitWindow;

    use super::TopMenu;

    /// No native menus here: the menu key opens the list menu.
    pub struct Native;

    /// Nothing to listen to.
    pub fn listen(_proxy: EventLoopProxy, _window_id: WindowId) {}

    impl Native {
        /// Always fails: this system shows the list menu.
        pub fn attach(_window: &WinitWindow, _tree: Vec<TopMenu>) -> Result<Native, String> {
            Err("native menus are only on Windows and macOS".into())
        }

        /// Never called (no `Native` exists).
        pub fn update(&mut self, _tree: Vec<TopMenu>) -> Result<bool, String> {
            Ok(false)
        }

        /// Nothing shown.
        pub fn shown(&self) -> &[TopMenu] {
            &[]
        }

        /// Nothing to read back.
        pub fn dump(&self) -> Vec<String> {
            Vec::new()
        }
    }
}

#[cfg(any(windows, target_os = "macos"))]
mod native {
    use std::sync::Mutex;

    use masonry_winit::app::{EventLoopProxy, MasonryUserEvent, WindowId};
    use masonry_winit::winit::window::Window as WinitWindow;
    use muda::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

    use super::{Entry, MenuPicked, TopMenu};

    /// The native menu bar, attached to the window.
    pub struct Native {
        menu: Menu,
        shown: Vec<TopMenu>,
        #[cfg(windows)]
        hwnd: isize,
    }

    /// Sends chosen items to the event loop as [`MenuPicked`]. Once per
    /// process: muda keeps one handler.
    pub fn listen(proxy: EventLoopProxy, window_id: WindowId) {
        let proxy = Mutex::new(proxy);
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Ok(p) = proxy.lock() {
                let picked = MenuPicked(e.id().0.clone());
                let _ = p.send_event(MasonryUserEvent::AsyncAction(window_id, Box::new(picked)));
            }
        }));
    }

    impl Native {
        /// Builds the menus from `tree` and attaches them to `window`.
        #[allow(unsafe_code)]
        pub fn attach(window: &WinitWindow, tree: Vec<TopMenu>) -> Result<Native, String> {
            let menu = build(&tree)?;
            #[cfg(windows)]
            {
                let hwnd = crate::gui::hwnd_of(window);
                if hwnd == 0 {
                    return Err("the window has no handle".into());
                }
                // SAFETY: `hwnd` is this process's live top-level window,
                // and `menu` is kept in `Native` for as long as it is
                // attached (muda's window subclass points at it).
                unsafe { menu.init_for_hwnd(hwnd) }.map_err(|e| e.to_string())?;
                Ok(Native {
                    menu,
                    shown: tree,
                    hwnd,
                })
            }
            #[cfg(target_os = "macos")]
            {
                let _ = window;
                menu.init_for_nsapp();
                Ok(Native { menu, shown: tree })
            }
        }

        /// Shows `tree` if it differs from what is shown: the menus are
        /// built again and put in place of the old ones. Returns whether
        /// they were.
        #[allow(unsafe_code)]
        pub fn update(&mut self, tree: Vec<TopMenu>) -> Result<bool, String> {
            if tree == self.shown {
                return Ok(false);
            }
            let menu = build(&tree)?;
            #[cfg(windows)]
            {
                // SAFETY: as in `attach`; the old menu is detached before it
                // is dropped, and the new one is kept.
                unsafe {
                    let _ = self.menu.remove_for_hwnd(self.hwnd);
                    menu.init_for_hwnd(self.hwnd).map_err(|e| e.to_string())?;
                }
            }
            #[cfg(target_os = "macos")]
            menu.init_for_nsapp();
            self.menu = menu;
            self.shown = tree;
            Ok(true)
        }

        /// What the menus show now.
        pub fn shown(&self) -> &[TopMenu] {
            &self.shown
        }

        /// The menus as the system holds them, one line per item:
        /// "&File > &Open...", a tab, and the key. Read back from the
        /// window's `HMENU`, which is what UI Automation reads. Empty on
        /// macOS.
        pub fn dump(&self) -> Vec<String> {
            #[cfg(windows)]
            {
                read_back(self.hwnd)
            }
            #[cfg(target_os = "macos")]
            {
                Vec::new()
            }
        }
    }

    #[cfg(windows)]
    #[allow(unsafe_code)]
    fn read_back(hwnd: isize) -> Vec<String> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetMenu, GetMenuItemCount, GetMenuStringW, GetSubMenu, HMENU, MF_BYPOSITION,
        };
        fn walk(menu: HMENU, path: &str, out: &mut Vec<String>) {
            // SAFETY: `menu` is a live menu of this process's window, read
            // on the thread that owns it.
            let n = unsafe { GetMenuItemCount(Some(menu)) };
            for i in 0..n.max(0) {
                let mut buf = [0u16; 512];
                // SAFETY: as above; the buffer's length bounds the copy.
                let len = unsafe { GetMenuStringW(menu, i as u32, Some(&mut buf), MF_BYPOSITION) };
                let text = String::from_utf16_lossy(&buf[..len.max(0) as usize]);
                // SAFETY: as above.
                let sub = unsafe { GetSubMenu(menu, i) };
                if !sub.is_invalid() {
                    walk(sub, &format!("{path}{text} > "), out);
                } else if !text.is_empty() {
                    out.push(format!("{path}{text}"));
                }
            }
        }
        let mut out = Vec::new();
        // SAFETY: this process's own window.
        let bar = unsafe { GetMenu(HWND(hwnd as *mut core::ffi::c_void)) };
        if !bar.is_invalid() {
            walk(bar, "", &mut out);
        }
        out
    }

    fn build(tree: &[TopMenu]) -> Result<Menu, String> {
        let menu = Menu::new();
        #[cfg(target_os = "macos")]
        {
            // macOS puts the first menu under the application's name.
            let app = Submenu::new("textweaver", true);
            let items: Vec<Box<dyn IsMenuItem>> = vec![
                Box::new(PredefinedMenuItem::hide(None)),
                Box::new(PredefinedMenuItem::hide_others(None)),
                Box::new(PredefinedMenuItem::show_all(None)),
            ];
            for i in &items {
                app.append(i.as_ref()).map_err(|e| e.to_string())?;
            }
            menu.append(&app).map_err(|e| e.to_string())?;
        }
        for top in tree {
            let sub = Submenu::new(title_text(&top.title), true);
            fill(&sub, &top.entries)?;
            menu.append(&sub).map_err(|e| e.to_string())?;
        }
        Ok(menu)
    }

    fn fill(sub: &Submenu, entries: &[Entry]) -> Result<(), String> {
        for e in entries {
            let item: Box<dyn IsMenuItem> = match e {
                Entry::Item {
                    pick,
                    checked: Some(on),
                    ..
                } => {
                    let item = CheckMenuItem::with_id(pick.id(), text(e), true, *on, None);
                    #[cfg(target_os = "macos")]
                    let _ = item.set_key_accelerator(key_equivalent(e));
                    Box::new(item)
                }
                Entry::Item { pick, .. } => {
                    let item = MenuItem::with_id(pick.id(), text(e), true, None);
                    #[cfg(target_os = "macos")]
                    let _ = item.set_key_accelerator(key_equivalent(e));
                    Box::new(item)
                }
                Entry::Note(_) => Box::new(MenuItem::new(text(e), false, None)),
                Entry::Submenu { entries, .. } => {
                    let s = Submenu::new(text(e), true);
                    fill(&s, entries)?;
                    Box::new(s)
                }
                Entry::Separator => Box::new(PredefinedMenuItem::separator()),
            };
            sub.append(item.as_ref()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// An entry's text: on Windows the label, a tab, and the key (the
    /// accelerator column and UI Automation's `AcceleratorKey`); on macOS
    /// the label without its `&` marks (the key is the key equivalent).
    fn text(e: &Entry) -> String {
        #[cfg(windows)]
        {
            e.windows_text()
        }
        #[cfg(target_os = "macos")]
        {
            match e {
                Entry::Item { label, .. } | Entry::Submenu { label, .. } => title_text(label),
                Entry::Note(t) => t.clone(),
                Entry::Separator => String::new(),
            }
        }
    }

    /// A menu title: Windows keeps the `&` mark (the access key); macOS
    /// has no access keys, so the marks go.
    fn title_text(title: &str) -> String {
        #[cfg(windows)]
        {
            title.to_owned()
        }
        #[cfg(target_os = "macos")]
        {
            textweaver_app::menu::split_access(title).0
        }
    }

    /// macOS: the item's key as its key equivalent, when it has a
    /// modifier or is a function key (a bare letter or arrow would take
    /// keys from typing and the caret).
    #[cfg(target_os = "macos")]
    fn key_equivalent(e: &Entry) -> Option<muda::accelerator::KeyAccelerator> {
        use muda::accelerator::{Key, KeyAccelerator, Modifiers as M, NamedKey};
        use textweaver_app::keymap::{Key as K, Modifiers};
        let Entry::Item {
            chord: Some(chord), ..
        } = e
        else {
            return None;
        };
        let function = matches!(chord.key, K::F(_));
        let mods = chord.mods - Modifiers::SHIFT;
        if mods.is_empty() && !function {
            return None;
        }
        let key = match chord.key {
            K::Char(c) => Key::Character(c.to_lowercase().to_string()),
            K::F(n) => Key::Named(match n {
                1 => NamedKey::F1,
                2 => NamedKey::F2,
                3 => NamedKey::F3,
                4 => NamedKey::F4,
                5 => NamedKey::F5,
                6 => NamedKey::F6,
                7 => NamedKey::F7,
                8 => NamedKey::F8,
                9 => NamedKey::F9,
                10 => NamedKey::F10,
                11 => NamedKey::F11,
                12 => NamedKey::F12,
                _ => return None,
            }),
            K::Space => Key::Character(" ".into()),
            K::Enter => Key::Named(NamedKey::Enter),
            K::Escape => Key::Named(NamedKey::Escape),
            K::Tab => Key::Named(NamedKey::Tab),
            K::Backspace => Key::Named(NamedKey::Backspace),
            K::Delete => Key::Named(NamedKey::Delete),
            K::Insert => Key::Named(NamedKey::Insert),
            K::Home => Key::Named(NamedKey::Home),
            K::End => Key::Named(NamedKey::End),
            K::PageUp => Key::Named(NamedKey::PageUp),
            K::PageDown => Key::Named(NamedKey::PageDown),
            K::Up => Key::Named(NamedKey::ArrowUp),
            K::Down => Key::Named(NamedKey::ArrowDown),
            K::Left => Key::Named(NamedKey::ArrowLeft),
            K::Right => Key::Named(NamedKey::ArrowRight),
        };
        let mut m = M::empty();
        m.set(M::CONTROL, chord.mods.contains(Modifiers::CTRL));
        m.set(M::ALT, chord.mods.contains(Modifiers::ALT));
        m.set(M::META, chord.mods.contains(Modifiers::META));
        let upper = matches!(chord.key, K::Char(c) if c.is_uppercase());
        m.set(M::SHIFT, chord.mods.contains(Modifiers::SHIFT) || upper);
        Some(KeyAccelerator::new(m, key))
    }
}

#[cfg(test)]
mod tests {
    use textweaver_app::AppConfig;
    use textweaver_app::keymap::{Layer, Platform};

    use super::*;

    /// An app with the window's keymap for this system, as the GUI builds
    /// it (`setup::build_app`).
    fn app() -> App {
        let mut config = AppConfig::for_tests();
        config.keymap = textweaver_app::keymap::Keymap::defaults(
            Platform::current(),
            textweaver_app::keymap::Frontend::Gui,
        );
        App::new(config)
    }

    /// Opens the list menu and walks to the menu holding `a`, as the keys
    /// would (focus a row, Right); returns `a`'s row there.
    fn walk_to(app: &mut App, a: ActionId) -> usize {
        use textweaver_app::{Command, ListKey};
        let path = textweaver_app::menu::find_path(a).expect("the command is in a menu");
        let _ = app.dispatch(Command::Action(ActionId::Menu));
        assert_eq!(app.menu_path(), Some(&[][..]), "the menu key shows the bar");
        let mut rows: Vec<MenuItemKind> = MenuId::TOP
            .iter()
            .map(|m| MenuItemKind::Submenu(*m))
            .collect();
        for m in &path {
            let row = rows
                .iter()
                .position(|k| *k == MenuItemKind::Submenu(*m))
                .expect("the submenu is a row");
            let _ = app.dispatch(Command::ListFocus(row));
            let _ = app.dispatch(Command::ListKey(ListKey::Right));
            rows = app
                .menu_view(*m)
                .items
                .into_iter()
                .filter(|i| i.kind != MenuItemKind::Separator)
                .map(|i| i.kind)
                .collect();
        }
        assert_eq!(app.menu_path(), Some(path.as_slice()));
        rows.iter()
            .position(|k| *k == MenuItemKind::Action(a))
            .expect("the command is a row")
    }

    /// The list menu (Linux, and wherever no native menu attaches): the
    /// menu key opens it, and the window knows which command a row runs,
    /// so it runs its own (the font, the text size) itself.
    #[test]
    fn the_list_menu_names_the_command_on_each_row() {
        let mut app = app();
        assert!(!app.keymap().chords_for(ActionId::Menu).is_empty());
        for a in [ActionId::ChooseFont, ActionId::TextLarger, ActionId::Open] {
            let row = walk_to(&mut app, a);
            assert_eq!(list_row_command(&app, row), Some(a));
            let _ = app.dispatch(textweaver_app::Command::Cancel);
        }
        assert_eq!(list_row_command(&app, 0), None, "no menu shown");
    }

    fn items(entries: &[Entry]) -> Vec<&Entry> {
        let mut out = Vec::new();
        for e in entries {
            out.push(e);
            if let Entry::Submenu { entries, .. } = e {
                out.extend(items(entries));
            }
        }
        out
    }

    #[test]
    fn the_seven_menus_come_from_the_model() {
        let app = app();
        let t = tree(&app);
        let ids: Vec<MenuId> = t.iter().map(|m| m.id).collect();
        assert_eq!(ids, MenuId::TOP.to_vec());
        for m in &t {
            assert!(m.title.contains('&'), "{} has an access key", m.title);
            assert!(!m.entries.is_empty(), "{} has items", m.title);
        }
        // Every command the model's menus offer, and no other.
        let mut ours = commands(&t);
        ours.sort_by_key(|a| a.id());
        ours.dedup();
        let mut model: Vec<ActionId> = textweaver_app::menu::actions_in_menus()
            .into_iter()
            .filter(|a| app.is_available(*a) && in_window(*a))
            .collect();
        model.sort_by_key(|a| a.id());
        model.dedup();
        assert_eq!(ours, model);
    }

    /// Every key a menu shows is bound to that command in the keymap the
    /// window uses (star's lesson: a shortcut shown is a shortcut bound).
    #[test]
    fn every_key_shown_is_bound() {
        let app = app();
        let keymap = app.keymap();
        for m in tree(&app) {
            for e in items(&m.entries) {
                let Entry::Item {
                    pick: Pick::Command(a),
                    keys,
                    chord,
                    ..
                } = e
                else {
                    continue;
                };
                let Some(chord) = chord else {
                    assert!(keys.is_none(), "{a:?} shows a key it has no chord for");
                    continue;
                };
                assert_eq!(keys.as_deref(), Some(chord.to_string().as_str()));
                assert!(
                    keymap.chords_for(*a).contains(chord),
                    "{a:?} shows {chord}, which is not its key"
                );
                let bound = Layer::ALL
                    .iter()
                    .any(|l| keymap.lookup(chord, *l) == Some(*a));
                assert!(bound, "{chord} does not run {a:?}");
            }
        }
    }

    #[test]
    fn windows_text_puts_the_key_after_a_tab() {
        let app = app();
        let t = tree(&app);
        let file = &t[0];
        let open = items(&file.entries)
            .into_iter()
            .find(|e| {
                matches!(
                    e,
                    Entry::Item {
                        pick: Pick::Command(ActionId::Open),
                        ..
                    }
                )
            })
            .expect("File has Open");
        let key = menu_chord(app.keymap(), ActionId::Open)
            .expect("Open has a key")
            .to_string();
        let text = open.windows_text();
        let (label, keys) = text.split_once('\t').expect("a tab before the key");
        assert!(label.contains('&'), "{label} marks its access key");
        assert_eq!(keys, key);
        // Labels never hold a tab of their own.
        for e in items(&file.entries) {
            assert!(e.windows_text().matches('\t').count() <= 1);
        }
    }

    #[test]
    fn toggles_show_their_state() {
        let mut app = app();
        let find = |app: &App| {
            items(&tree(app)[2].entries)
                .into_iter()
                .find_map(|e| match e {
                    Entry::Item {
                        pick: Pick::Command(ActionId::BionicToggle),
                        checked,
                        ..
                    } => Some(*checked),
                    _ => None,
                })
                .expect("View has bionic reading")
        };
        assert_eq!(find(&app), Some(false));
        let before = tree(&app);
        let _ = app.dispatch(textweaver_app::Command::Action(ActionId::BionicToggle));
        assert_eq!(find(&app), Some(true));
        assert_ne!(tree(&app), before, "a changed toggle rebuilds the menus");
        assert_eq!(tree(&app), tree(&app), "an unchanged model is equal");
    }

    /// The fingerprint changes whenever the tree does, so a command that
    /// leaves it alone can skip building the tree.
    #[test]
    fn the_fingerprint_follows_the_tree() {
        use textweaver_app::Command;
        let mut app = app();
        let commands = [
            ActionId::NextSentence,
            ActionId::BionicToggle,
            ActionId::RulerCycle,
            ActionId::ToggleEditMode,
            ActionId::ToggleEditMode,
            ActionId::NextTheme,
            ActionId::CycleInterfaceAnnouncements,
            ActionId::ToggleCharacterKeys,
            ActionId::SpeechCursorToggle,
        ];
        for a in commands {
            let (key, shown) = (Fingerprint::of(&app), tree(&app));
            let _ = app.dispatch(Command::Action(a));
            if Fingerprint::of(&app) == key {
                assert_eq!(tree(&app), shown, "{a:?} changed the menus, not the key");
            }
        }
    }

    /// Commands whose modules have merged appear in the window's menus as
    /// they register their handlers: browse files (W6f), and dictation
    /// (W6d) in a build with it. None is ever shown before it works.
    #[test]
    fn registered_modules_join_the_menus() {
        let app = app();
        let offered = commands(&tree(&app));
        for &a in textweaver_app::menu::PENDING {
            assert_eq!(
                offered.contains(&a),
                app.is_available(a),
                "{a:?} is in the menus exactly when it works"
            );
        }
        assert!(app.is_available(ActionId::BrowseFiles), "W6f registers it");
        let file = &tree(&app)[0];
        assert!(commands(std::slice::from_ref(file)).contains(&ActionId::BrowseFiles));
    }

    #[test]
    fn picks_round_trip_through_their_ids() {
        for p in [
            Pick::Command(ActionId::Open),
            Pick::Command(ActionId::ToggleEditMode),
            Pick::Document(PathBuf::from("D:/docs/essay one.md")),
        ] {
            assert_eq!(Pick::from_id(&p.id()), Some(p.clone()));
        }
        assert_eq!(Pick::from_id("a:no_such_command"), None);
        assert_eq!(Pick::from_id("d:"), None);
        assert_eq!(Pick::from_id("x"), None);
    }

    #[test]
    fn the_menus_follow_the_language() {
        let mut app = app();
        let english = tree(&app);
        let said = app.set_setting("interface.language", serde_json::json!("de"));
        assert!(said.is_ok(), "{said:?}");
        let german = tree(&app);
        assert_ne!(english[0].title, german[0].title);
        assert_eq!(english.len(), german.len());
        let _ = Platform::current();
    }
}
