//! The join QR code (beta 5, phase 7c): the app's broker's join link
//! ([`textweaver_mqtt::JoinLink`]) as a QR code that another computer or
//! a phone scans, and joining from a QR code in an image.
//!
//! - **Show the join QR code** shows a view whose rows are, in order: the
//!   warning in words ("Anyone who sees this code can join."), the image
//!   (in the textweaver app; its accessible name is "QR code for joining
//!   HOST"), the link as text, then Copy the join link, Save the QR code
//!   and Close. The terminal reader has no image row; it adds Draw the QR
//!   code in characters, which shows the code in block characters only
//!   when chosen, after a one-line warning that a screen reader reads it
//!   as noise.
//! - **Save the QR code** asks where (`join-qr.png` is offered) and writes
//!   a PNG, or an SVG when the name ends in `.svg`, then says where. Only
//!   there: textweaver keeps no copy.
//! - **Join from a QR image** reads the image on the clipboard; without
//!   one, or with no join link in it, it asks for an image file (PNG or
//!   JPEG). The link read fills the live source form, as a pasted link
//!   does, and Enter joins.
//!
//! The link holds the join code. It is shown only in the view, on
//! request, and copied only by Copy: never said, logged, saved in
//! settings, or put in the status line. The view's code is held in the
//! open list only, and goes when the list closes.

use crate::app::App;

/// A row of the join QR code view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrRow {
    /// The warning, in words, first.
    Warning,
    /// The image (the textweaver app draws the code here).
    Image,
    /// The link, as text.
    Link,
    /// Copy the join link.
    Copy,
    /// Save the QR code to a file.
    Save,
    /// Draw the QR code in block characters (the terminal reader).
    Draw,
    /// Close the view.
    Close,
}

/// A QR code's modules, its quiet zone included, row by row from the top:
/// true is dark. Its `Debug` form gives the size only, since the modules
/// hold the join code.
#[derive(Clone, PartialEq, Eq)]
pub struct QrModules(Vec<Vec<bool>>);

impl std::fmt::Debug for QrModules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QrModules({} by {0})", self.side())
    }
}

impl QrModules {
    /// The number of modules on a side, the quiet zone included.
    pub fn side(&self) -> usize {
        self.0.len()
    }

    /// Whether the module at `x`, `y` (from the top left, quiet zone
    /// included) is dark. Outside the code, false.
    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        self.0
            .get(y)
            .and_then(|r| r.get(x))
            .copied()
            .unwrap_or(false)
    }

    /// The code in block characters, two rows of modules to a line: each
    /// light module is drawn, so a terminal's light text on a dark ground
    /// shows dark modules dark. Scanners read either way round.
    pub fn block_lines(&self) -> Vec<String> {
        let n = self.side();
        (0..n)
            .step_by(2)
            .map(|y| {
                (0..n)
                    .map(|x| {
                        let top = !self.is_dark(x, y);
                        // Below the last row: light, as the quiet zone.
                        let bottom = y + 1 >= n || !self.is_dark(x, y + 1);
                        match (top, bottom) {
                            (true, true) => '\u{2588}',
                            (true, false) => '\u{2580}',
                            (false, true) => '\u{2584}',
                            (false, false) => ' ',
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

/// The join QR code view while it is open: its rows, in order, the code,
/// and the image's accessible name. The rows' text is the list's items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinQrView {
    /// What each row of the list is.
    pub rows: Vec<QrRow>,
    /// The code.
    pub modules: QrModules,
    /// The image's accessible name: "QR code for joining HOST".
    pub image_name: String,
}

#[cfg(feature = "mqtt")]
mod imp {
    use std::path::PathBuf;

    use textweaver_lexicon::args;
    use textweaver_mqtt::JoinLink;
    use textweaver_qr::QrError;

    use super::{JoinQrView, QrModules, QrRow};
    use crate::app::{App, ListKind};
    use crate::clipboard_image::ClipboardImage;
    use crate::command::{Effect, PromptPurpose};
    use crate::live::wire::LiveList;

    /// The code for `link`, its quiet zone included.
    fn modules(link: &JoinLink) -> Result<QrModules, QrError> {
        let m = textweaver_qr::encode(&link.to_string())?;
        let q = textweaver_qr::QUIET_ZONE;
        let n = m.size() + 2 * q;
        let rows = (0..n)
            .map(|y| {
                (0..n)
                    .map(|x| x >= q && y >= q && x < n - q && y < n - q && m.is_dark(x - q, y - q))
                    .collect()
            })
            .collect();
        Ok(QrModules(rows))
    }

    impl App {
        /// The join QR code view while it is shown, for the textweaver
        /// app to draw: `None` otherwise.
        pub fn live_join_qr(&self) -> Option<&JoinQrView> {
            match &self.list {
                Some(ListKind::Live(LiveList::Qr(view))) => Some(view),
                _ => None,
            }
        }

        /// Show the join QR code: the view, focused on its warning; with
        /// no join code, why not, in words.
        pub fn live_show_join_qr(&mut self) -> Vec<Effect> {
            self.live_join_qr_list(0)
        }

        /// The view, focused on row `focus`.
        fn live_join_qr_list(&mut self, focus: usize) -> Vec<Effect> {
            let Some(link) = self.live_join_link() else {
                return self.live_say_no_join_code();
            };
            let modules = match modules(&link) {
                Ok(m) => m,
                Err(e) => {
                    let reason = self.live_qr_reason(&e);
                    let m = self.msg_args("live-qr-failed", &args!["reason" => reason]);
                    self.error(&m);
                    return vec![Effect::Redraw];
                }
            };
            let image_name = self.msg_args("live-qr-image", &args!["host" => link.host()]);
            let terminal = self.keymap.frontend() == textweaver_keymap::Frontend::Terminal;
            let mut rows = vec![QrRow::Warning];
            if !terminal {
                rows.push(QrRow::Image);
            }
            rows.extend([QrRow::Link, QrRow::Copy, QrRow::Save]);
            if terminal {
                rows.push(QrRow::Draw);
            }
            rows.push(QrRow::Close);
            let items: Vec<String> = rows
                .iter()
                .map(|r| match r {
                    QrRow::Warning => self.msg("live-qr-warning"),
                    QrRow::Image => image_name.clone(),
                    QrRow::Link => link.to_string(),
                    QrRow::Copy => self.msg("name-copy-join-link"),
                    QrRow::Save => self.msg("live-qr-save"),
                    QrRow::Draw => self.msg("live-qr-draw"),
                    QrRow::Close => self.msg("gui-button-close"),
                })
                .collect();
            let title = self.msg("live-qr-title");
            self.list = Some(ListKind::Live(LiveList::Qr(JoinQrView {
                rows,
                modules,
                image_name,
            })));
            self.pending_list_focus = Some(focus.min(items.len().saturating_sub(1)));
            vec![Effect::ShowList { title, items }]
        }

        /// Enter on row `n` of the view.
        pub(crate) fn live_choose_qr(&mut self, view: &JoinQrView, n: usize) -> Vec<Effect> {
            match view.rows.get(n) {
                // The link copies, like the button after it.
                Some(QrRow::Link | QrRow::Copy) => {
                    let mut effects = self.live_copy_join_link();
                    if self.live_has_join_code() {
                        // The view stays, and its row is not said again.
                        self.list_reshow_quiet = true;
                        effects = self.live_join_qr_list(n);
                    }
                    effects
                }
                Some(QrRow::Save) => self.prompt(PromptPurpose::SaveJoinQr),
                Some(QrRow::Draw) => self.live_draw_join_qr(&view.modules),
                Some(QrRow::Warning | QrRow::Image) => {
                    self.list_reshow_quiet = true;
                    self.live_join_qr_list(n)
                }
                Some(QrRow::Close) | None => vec![Effect::Redraw],
            }
        }

        /// The code in block characters, after a one-line warning.
        fn live_draw_join_qr(&mut self, modules: &QrModules) -> Vec<Effect> {
            let mut items = vec![self.msg("live-qr-drawing-warning")];
            items.extend(modules.block_lines());
            let title = self.msg("live-qr-drawing-title");
            self.list = Some(ListKind::Live(LiveList::QrDrawing));
            self.pending_list_focus = Some(0);
            vec![Effect::ShowList { title, items }]
        }

        /// The answer to Save the QR code: a PNG, or an SVG for a name
        /// ending in `.svg`, with `.png` added to a name without an
        /// extension; then says where.
        pub(crate) fn answer_save_join_qr(&mut self, text: &str) -> Vec<Effect> {
            let typed = text.trim().trim_matches('"');
            if typed.is_empty() {
                let m = self.msg("common-cancelled");
                self.note(&m);
                return vec![Effect::Redraw];
            }
            let mut path = PathBuf::from(typed);
            if path.extension().is_none() {
                path.set_extension("png");
            }
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if ext != "png" && ext != "svg" {
                let m = self.msg("live-qr-save-type");
                self.error(&m);
                return vec![Effect::Redraw];
            }
            let Some(link) = self.live_join_link() else {
                return self.live_say_no_join_code();
            };
            let made = textweaver_qr::encode(&link.to_string()).and_then(|m| {
                if ext == "svg" {
                    let title = self.msg_args("live-qr-image", &args!["host" => link.host()]);
                    Ok(m.to_svg(&title).into_bytes())
                } else {
                    m.to_png(textweaver_qr::DEFAULT_MODULE_PIXELS)
                }
            });
            let bytes = match made {
                Ok(b) => b,
                Err(e) => {
                    let reason = self.live_qr_reason(&e);
                    let m = self.msg_args("live-qr-save-failed", &args!["error" => reason]);
                    self.error(&m);
                    return vec![Effect::Redraw];
                }
            };
            let shown = std::path::absolute(&path).unwrap_or_else(|_| path.clone());
            match textweaver_store::atomic_write(&path, &bytes) {
                Ok(()) => {
                    let m = self.msg_args(
                        "live-qr-saved",
                        &args!["path" => shown.display().to_string()],
                    );
                    self.tell(&m);
                }
                Err(e) => {
                    let m = self.msg_args("live-qr-save-failed", &args!["error" => e.to_string()]);
                    self.error(&m);
                }
            }
            vec![Effect::Redraw]
        }

        /// Join from a QR image: the image on the clipboard, read into the
        /// form; else (no image, or no join link in it, said why) the
        /// prompt for an image file.
        pub fn live_join_from_qr_image(&mut self) -> Vec<Effect> {
            if let Some(image) = self.clipboard_image() {
                let read = match &image {
                    ClipboardImage::Encoded(bytes) => textweaver_qr::decode_bytes(bytes),
                    ClipboardImage::Rgba {
                        width,
                        height,
                        pixels,
                    } => textweaver_qr::decode_rgba(*width, *height, pixels),
                };
                drop(image);
                match self.live_link_from(read) {
                    Ok(link) => {
                        let intro = self.msg("live-qr-intro-clipboard");
                        return self.live_form_from_link(&link, &intro);
                    }
                    Err(reason) => {
                        let m =
                            self.msg_args("live-qr-clipboard-no-link", &args!["reason" => reason]);
                        self.tell(&m);
                    }
                }
            }
            self.prompt(PromptPurpose::JoinQrImage)
        }

        /// The answer to the image file prompt: the link in its QR code
        /// fills the form; else why not, in words.
        pub(crate) fn answer_join_qr_image(&mut self, text: &str) -> Vec<Effect> {
            let typed = text.trim().trim_matches('"');
            if typed.is_empty() {
                let m = self.msg("common-cancelled");
                self.note(&m);
                return vec![Effect::Redraw];
            }
            let path = PathBuf::from(typed);
            match self.live_link_from(textweaver_qr::decode_file(&path)) {
                Ok(link) => {
                    let file = path.file_name().map_or_else(
                        || path.display().to_string(),
                        |n| n.to_string_lossy().into_owned(),
                    );
                    let intro = self.msg_args("live-qr-intro-file", &args!["file" => file]);
                    self.live_form_from_link(&link, &intro)
                }
                Err(reason) => {
                    let m = self.msg_args("live-qr-no-link", &args!["reason" => reason]);
                    self.error(&m);
                    vec![Effect::Redraw]
                }
            }
        }

        /// The first join link among the texts read from an image, or why
        /// there is none, as a sentence that never holds a code.
        fn live_link_from(&self, read: Result<Vec<String>, QrError>) -> Result<JoinLink, String> {
            let texts = read.map_err(|e| self.live_qr_reason(&e))?;
            let mut bad = None;
            for text in &texts {
                match JoinLink::find(text) {
                    Some(Ok(link)) => return Ok(link),
                    Some(Err(reason)) => bad = Some(reason),
                    None => {}
                }
            }
            Err(match bad {
                Some(reason) => {
                    let reason = reason.strip_prefix("Join link error: ").unwrap_or(&reason);
                    let mut chars = reason.chars();
                    chars.next().map_or_else(String::new, |c| {
                        c.to_uppercase().chain(chars).collect::<String>()
                    })
                }
                None => self.msg("live-qr-other-text"),
            })
        }

        /// Why a QR code was not made or read, in words.
        fn live_qr_reason(&self, e: &QrError) -> String {
            match e {
                QrError::NotFound => self.msg("live-qr-not-found"),
                QrError::Unreadable => self.msg("live-qr-unreadable"),
                QrError::FileTooLarge => self.msg("live-qr-too-large"),
                QrError::TooLong | QrError::Empty => self.msg("live-qr-too-long"),
                QrError::Io(io) => {
                    self.msg_args("live-qr-image-error", &args!["error" => io.to_string()])
                }
                QrError::Image(m) | QrError::Write(m) => {
                    self.msg_args("live-qr-image-error", &args!["error" => m.as_str()])
                }
            }
        }
    }
}

impl App {
    /// True while the join QR code view is shown (in any build).
    pub fn live_join_qr_open(&self) -> bool {
        self.live_join_qr().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_lines_draw_light_modules_two_rows_to_a_line() {
        // 3 by 3: dark on the diagonal.
        let m = QrModules(vec![
            vec![true, false, false],
            vec![false, true, false],
            vec![false, false, true],
        ]);
        assert_eq!(
            m.block_lines(),
            ["\u{2584}\u{2580}\u{2588}", "\u{2588}\u{2588}\u{2584}"]
        );
        assert_eq!(format!("{m:?}"), "QrModules(3 by 3)");
        assert!(m.is_dark(1, 1));
        assert!(!m.is_dark(5, 5), "outside the code");
    }
}

#[cfg(all(test, feature = "mqtt"))]
mod app_tests {
    //! The broker listens on 127.0.0.1 with a port the system picks:
    //! loopback only, never a real broker, never audio, never the real
    //! clipboard (a `FakeClipboard`), and files only in a temporary folder.

    use textweaver_a11y::{LogAnnouncer, Shared};
    use textweaver_keymap::{ActionId, Frontend, Keymap, Platform};
    use textweaver_store::Paths;

    use super::*;
    use crate::clipboard::{ClipboardContents, FakeClipboard};
    use crate::clipboard_image::ClipboardImage;
    use crate::command::{Command, PromptPurpose};
    use crate::list_model::{ListKey, PromptKey};
    use crate::{App, AppConfig};

    struct Side {
        app: App,
        said: Shared<LogAnnouncer>,
        clip: FakeClipboard,
        tmp: tempfile::TempDir,
    }

    impl Side {
        fn new(frontend: Frontend) -> Side {
            textweaver_components::fake::memory_credentials();
            let tmp = tempfile::tempdir().unwrap();
            let said = Shared::new(LogAnnouncer::default());
            let mut app = App::new(AppConfig {
                announcer: Box::new(said.clone()),
                paths: Some(Paths::under(tmp.path())),
                keymap: Keymap::defaults(Platform::current(), frontend),
                ..AppConfig::for_tests()
            });
            let _ = app.update_settings(|s| {
                s.live.broker.listen = vec!["127.0.0.1:0".into()];
            });
            let clip = FakeClipboard::new();
            app.set_clipboard(Box::new(clip.clone()));
            Side {
                app,
                said,
                clip,
                tmp,
            }
        }

        fn texts(&self) -> Vec<String> {
            self.said
                .lock()
                .texts()
                .iter()
                .map(|s| s.to_string())
                .collect()
        }

        fn any(&self, needle: &str) -> bool {
            self.texts().iter().any(|t| t.contains(needle))
        }

        fn act(&mut self, a: ActionId) {
            self.app.dispatch(Command::Action(a));
        }

        fn key(&mut self, k: PromptKey) {
            self.app.dispatch(Command::PromptKey(k));
        }

        fn items(&self) -> Vec<String> {
            self.app
                .list_model()
                .map(|l| l.items.clone())
                .unwrap_or_default()
        }

        /// Enter on row `n` of the list shown, moved to quietly (as a GUI
        /// list moves its own focus), so the link row is never said here.
        fn choose(&mut self, n: usize) {
            self.app.dispatch(Command::ListFocus(n));
            self.app.dispatch(Command::ListKey(ListKey::Enter));
        }

        /// The broker's join link, from Copy the join link.
        fn link(&mut self) -> String {
            self.act(ActionId::CopyJoinLink);
            self.app.take_clipboard().unwrap()
        }

        /// No said text ever holds the link or the code.
        fn never_said(&self, secret: &str) {
            for t in self.texts() {
                assert!(!t.contains(secret), "said: {t}");
                assert!(!t.contains("twjoin"), "said: {t}");
            }
        }
    }

    /// The view's code as RGBA pixels, four to a module, read back.
    fn read_back(m: &QrModules) -> Vec<String> {
        let scale = 4;
        let side = m.side() * scale;
        let mut px = Vec::with_capacity(side * side * 4);
        for y in 0..side {
            for x in 0..side {
                let v = if m.is_dark(x / scale, y / scale) {
                    0
                } else {
                    255
                };
                px.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let side = u32::try_from(side).unwrap();
        textweaver_qr::decode_rgba(side, side, &px).unwrap()
    }

    #[test]
    fn the_view_warns_first_then_shows_the_image_the_link_and_the_buttons() {
        let mut s = Side::new(Frontend::Gui);
        s.act(ActionId::ShowJoinQr);
        assert!(s.any("No broker is running."), "{:?}", s.texts());
        assert!(s.app.live_join_qr().is_none());
        s.act(ActionId::StartBroker);
        let link = s.link();
        let code = s.app.live_broker_join_code().unwrap();
        s.act(ActionId::ShowJoinQr);
        let items = s.items();
        assert_eq!(
            items,
            [
                "Anyone who sees this code can join.",
                "QR code for joining 127.0.0.1",
                link.as_str(),
                "Copy the join link",
                "Save the QR code",
                "Close",
            ]
        );
        assert_eq!(s.app.list_model().unwrap().title, "Join QR code");
        assert!(
            s.any("Anyone who sees this code can join."),
            "the warning is said first: {:?}",
            s.texts()
        );
        let view = s.app.live_join_qr().unwrap().clone();
        assert_eq!(view.image_name, "QR code for joining 127.0.0.1");
        assert_eq!(read_back(&view.modules), std::slice::from_ref(&link));
        assert!(!format!("{view:?}").contains("twjoin"), "Debug hides it");
        // Copy the join link keeps the view open, on its row.
        s.choose(3);
        assert_eq!(s.app.take_clipboard().as_deref(), Some(link.as_str()));
        assert!(s.any("Join link copied."));
        assert_eq!(s.app.list_model().unwrap().selected, 3);
        assert!(s.app.live_join_qr_open());
        // Close, and the code is gone with the list.
        s.choose(5);
        assert!(s.app.list_model().is_none());
        assert!(s.app.live_join_qr().is_none());
        s.never_said(&code);
        s.app.shutdown();
    }

    #[test]
    fn save_writes_a_png_or_an_svg_where_asked_and_says_where() {
        let mut s = Side::new(Frontend::Terminal);
        s.act(ActionId::StartBroker);
        let link = s.link();
        let code = s.app.live_broker_join_code().unwrap();
        s.act(ActionId::ShowJoinQr);
        // The terminal reader: no image row, and Draw before Close.
        assert_eq!(
            s.items()[1..],
            [
                link.as_str(),
                "Copy the join link",
                "Save the QR code",
                "Draw the QR code in characters",
                "Close",
            ]
        );
        s.choose(3);
        assert_eq!(
            s.app.prompt_model().unwrap().purpose,
            PromptPurpose::SaveJoinQr
        );
        let png = s.tmp.path().join("join-qr");
        s.app
            .dispatch(Command::Answer(png.to_string_lossy().into_owned()));
        let png = png.with_extension("png");
        assert!(s.any("QR code saved: "), "{:?}", s.texts());
        assert!(s.any("join-qr.png"), "{:?}", s.texts());
        assert_eq!(
            textweaver_qr::decode_file(&png).unwrap(),
            std::slice::from_ref(&link)
        );
        // An SVG, named for joining the host, never with the link.
        s.act(ActionId::ShowJoinQr);
        s.choose(3);
        let svg = s.tmp.path().join("join-qr.svg");
        s.app
            .dispatch(Command::Answer(svg.to_string_lossy().into_owned()));
        let text = std::fs::read_to_string(&svg).unwrap();
        assert!(text.contains("QR code for joining 127.0.0.1"), "{text}");
        assert!(!text.contains("twjoin") && !text.contains(&code));
        // Any other kind of file is refused in words.
        s.act(ActionId::ShowJoinQr);
        s.choose(3);
        let gif = s.tmp.path().join("join-qr.gif");
        s.app
            .dispatch(Command::Answer(gif.to_string_lossy().into_owned()));
        assert!(
            s.any("QR code not saved: the file name must end in .png or .svg."),
            "{:?}",
            s.texts()
        );
        assert!(!gif.exists());
        s.never_said(&code);
        s.app.shutdown();
    }

    #[test]
    fn draw_shows_the_code_in_block_characters_after_a_warning() {
        let mut s = Side::new(Frontend::Terminal);
        s.act(ActionId::StartBroker);
        s.act(ActionId::ShowJoinQr);
        let view = s.app.live_join_qr().unwrap().clone();
        s.choose(4);
        let items = s.items();
        assert!(
            items[0].starts_with("Block characters follow, for a camera;"),
            "{items:?}"
        );
        assert_eq!(items.len(), 1 + view.modules.side().div_ceil(2));
        for line in &items[1..] {
            assert_eq!(line.chars().count(), view.modules.side());
            assert!(
                line.chars()
                    .all(|c| matches!(c, ' ' | '\u{2580}' | '\u{2584}' | '\u{2588}')),
                "{line}"
            );
        }
        // Only the warning is said.
        assert!(!s.texts().iter().any(|t| t.contains('\u{2588}')));
        s.app.dispatch(Command::ListKey(ListKey::Escape));
        assert!(s.app.list_model().is_none());
        s.app.shutdown();
    }

    #[test]
    fn join_from_a_qr_image_reads_the_clipboard_image_then_a_file() {
        let mut host = Side::new(Frontend::Gui);
        host.act(ActionId::StartBroker);
        let link = host.link();
        let code = host.app.live_broker_join_code().unwrap();
        let png = textweaver_qr::encode(&link).unwrap().to_png(4).unwrap();
        host.app.shutdown();

        let mut r = Side::new(Frontend::Gui);
        // An image on the clipboard: read at once.
        r.clip.set_image(Some(ClipboardImage::Encoded(png.clone())));
        r.act(ActionId::JoinFromQrImage);
        assert!(
            r.any("Join from the QR code on the clipboard."),
            "{:?}",
            r.texts()
        );
        assert!(
            r.any("Filled from the join link: host, port and code. Type the topic."),
            "{:?}",
            r.texts()
        );
        assert!(r.app.prompt_model().unwrap().label.starts_with("Topic"));
        r.key(PromptKey::Escape);

        // An image with other text: said, then the file prompt.
        let other = textweaver_qr::encode("textweaver")
            .unwrap()
            .to_png(4)
            .unwrap();
        r.clip.set_image(Some(ClipboardImage::Encoded(other)));
        r.act(ActionId::JoinFromQrImage);
        assert!(
            r.any(
                "No join link in the clipboard image: The QR code holds something else. Choose an image file."
            ),
            "{:?}",
            r.texts()
        );
        assert_eq!(
            r.app.prompt_model().unwrap().purpose,
            PromptPurpose::JoinQrImage
        );
        // A file with the code.
        let file = r.tmp.path().join("phone-photo.png");
        std::fs::write(&file, &png).unwrap();
        r.app
            .dispatch(Command::Answer(file.to_string_lossy().into_owned()));
        assert!(
            r.any("Join from the QR code in phone-photo.png."),
            "{:?}",
            r.texts()
        );
        assert_eq!(
            r.app.prompt_model().unwrap().purpose,
            PromptPurpose::LiveSetup
        );
        r.key(PromptKey::Escape);

        // No image on the clipboard: the file prompt at once; a file that
        // is not an image says why.
        r.clip.set_image(None);
        r.act(ActionId::JoinFromQrImage);
        assert_eq!(
            r.app.prompt_model().unwrap().purpose,
            PromptPurpose::JoinQrImage
        );
        let blank = r.tmp.path().join("notes.png");
        std::fs::write(&blank, "not an image").unwrap();
        r.app
            .dispatch(Command::Answer(blank.to_string_lossy().into_owned()));
        assert!(
            r.any("No join link: The image could not be read"),
            "{:?}",
            r.texts()
        );
        r.never_said(&code);
        r.app.shutdown();
    }

    #[test]
    fn the_join_code_field_has_a_context_menu_copy_paste_show_and_hide() {
        let mut r = Side::new(Frontend::Gui);
        r.act(ActionId::SetUpLiveSource);
        r.key(PromptKey::Tab);
        r.key(PromptKey::Tab);
        for _ in 0..4 {
            r.key(PromptKey::Down);
        }
        r.key(PromptKey::Tab);
        assert!(r.app.prompt_model().unwrap().is_masked());
        for c in "abcdefghjkmnpqrs".chars() {
            r.key(PromptKey::Char(c));
        }
        r.key(PromptKey::ContextMenu);
        assert_eq!(r.items(), ["Copy", "Paste", "Show the join code"]);
        assert_eq!(r.app.list_model().unwrap().title, "Join code field");
        // Copy: the code to the clipboard, never said; back to the field.
        r.choose(0);
        assert_eq!(r.app.take_clipboard().as_deref(), Some("abcdefghjkmnpqrs"));
        assert!(r.any("Join code copied."));
        let model = r.app.prompt_model().unwrap();
        assert_eq!(model.text(), "abcdefghjkmnpqrs");
        assert!(model.is_masked());
        // Show: the field opens shown as typed; then Hide is offered.
        r.key(PromptKey::ContextMenu);
        r.choose(2);
        assert!(r.any("Showing the join code"));
        assert!(r.app.prompt_model().unwrap().is_shown());
        r.key(PromptKey::ContextMenu);
        assert_eq!(r.items()[2], "Hide the join code");
        // Escape goes back to the field, its text kept.
        r.app.dispatch(Command::ListKey(ListKey::Escape));
        assert_eq!(r.app.prompt_model().unwrap().text(), "abcdefghjkmnpqrs");
        // Paste takes the code out of what the clipboard holds.
        r.key(PromptKey::SetText(String::new()));
        r.clip
            .set(ClipboardContents::text("Join code: wxyz wxyz wxyz wxyz."));
        r.key(PromptKey::ContextMenu);
        r.choose(1);
        assert_eq!(r.app.prompt_model().unwrap().text(), "WXYZ-WXYZ-WXYZ-WXYZ");
        assert!(r.any("Join code pasted."));
        for t in r.texts() {
            assert!(!t.contains("WXYZ") && !t.contains("abcd"), "said: {t}");
        }
        // On another field, the key does nothing.
        r.key(PromptKey::Tab);
        r.key(PromptKey::ContextMenu);
        assert!(r.app.list_model().is_none());
        assert!(r.app.prompt_model().is_some());
    }

    #[test]
    fn the_context_menu_offers_the_qr_code_and_stop_sharing_while_sharing() {
        let mut s = Side::new(Frontend::Gui);
        let actions = |s: &Side| -> Vec<ActionId> {
            s.app
                .context_menu()
                .iter()
                .filter_map(|i| match i.kind {
                    crate::menu::MenuItemKind::Action(a) => Some(a),
                    _ => None,
                })
                .collect()
        };
        assert!(!actions(&s).contains(&ActionId::ShowJoinQr));
        assert!(!actions(&s).contains(&ActionId::StopSharing));
        s.act(ActionId::StartBroker);
        assert!(actions(&s).contains(&ActionId::ShowJoinQr));
        let path = s.tmp.path().join("agenda.md");
        std::fs::write(&path, "# Agenda\n").unwrap();
        s.app.dispatch(Command::Open(path));
        s.act(ActionId::ShareDocumentLive);
        let a = actions(&s);
        assert!(a.contains(&ActionId::CopyJoinLink), "{a:?}");
        assert!(a.contains(&ActionId::ShowJoinQr), "{a:?}");
        assert!(a.contains(&ActionId::StopSharing), "{a:?}");
        s.act(ActionId::StopSharing);
        assert!(!actions(&s).contains(&ActionId::StopSharing));
        s.app.shutdown();
    }
}
