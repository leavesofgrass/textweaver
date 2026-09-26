//! The Fonts dialog (View, Fonts): a family list (bundled families first,
//! then the installed ones), a size, a Bold check box, and a live preview.
//!
//! Every control is a native Win32 control named by the label just before
//! it (ADR-0014), in a tab order that follows the reading order: family,
//! size, bold, preview, OK, Cancel. Access keys: Alt+F family, Alt+S size,
//! Alt+B bold, Alt+P preview. Enter is OK, Escape is Cancel.
//!
//! Normally a modal dialog ([`run`]). Automated checks (`--background`)
//! get the same controls in a separate window that is minimized before it
//! is first shown ([`open_background`]), the arrangement that keeps the main
//! window from ever being activated: a modal dialog would take the
//! foreground from the person at the machine.

use std::cell::RefCell;
use std::rc::Rc;

use wxdragon::prelude::*;

use crate::fonts::{Applied, Choice};

/// The preview sentence: every letter, and the shapes hyperlegible fonts
/// set apart (capital I, lower-case l, the digit 1; O and 0).
pub const SAMPLE: &str = "The quick brown fox jumps over the lazy dog. \
Il1 O0 rn m. 0123456789.";

/// What the reader chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picked {
    /// The family, as the toolkit names it.
    pub family: String,
    /// Size in points.
    pub size: i32,
    /// Bold.
    pub bold: bool,
}

/// A font for the reading control or the preview.
pub fn make_font(face: Option<&str>, size: i32, bold: bool) -> Option<Font> {
    let mut b = Font::builder()
        .with_point_size(size.clamp(6, 144))
        .with_weight(if bold {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        });
    if let Some(face) = face {
        b = b.with_face_name(face);
    }
    b.build()
}

/// The chooser's controls, on `panel`, laid out and wired to the preview.
struct Controls {
    ok: Button,
    cancel: Button,
    read: Rc<dyn Fn() -> Option<Picked>>,
}

fn controls(panel: &Panel, choices: &[Choice], current: &Applied, log: bool) -> Controls {
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    let family_label = StaticText::builder(panel)
        .with_label("&Font family:")
        .build();
    let labels: Vec<String> = choices.iter().map(|c| c.label.clone()).collect();
    let list = ListBox::builder(panel)
        .with_choices(labels)
        .with_size(Size::new(380, 220))
        .build();
    let size_label = StaticText::builder(panel)
        .with_label("&Size in points:")
        .build();
    let spin = SpinCtrl::builder(panel)
        .with_range(6, 144)
        .with_initial_value(current.size.clamp(6, 144))
        .build();
    let bold = CheckBox::builder(panel)
        .with_label("&Bold")
        .with_value(current.bold)
        .build();
    let preview_label = StaticText::builder(panel).with_label("&Preview:").build();
    let preview = TextCtrl::builder(panel)
        .with_value(SAMPLE)
        .with_style(
            TextCtrlStyle::MultiLine
                | TextCtrlStyle::ReadOnly
                | TextCtrlStyle::Rich2
                | TextCtrlStyle::WordWrap,
        )
        .with_size(Size::new(460, 120))
        .build();
    let ok = Button::builder(panel)
        .with_id(ID_OK)
        .with_label("OK")
        .build();
    let cancel = Button::builder(panel)
        .with_id(ID_CANCEL)
        .with_label("Cancel")
        .build();
    ok.set_default();

    let pad = SizerFlag::Left | SizerFlag::Right | SizerFlag::Top;
    sizer.add(&family_label, 0, pad, 10);
    sizer.add(&list, 1, SizerFlag::Expand | SizerFlag::All, 10);
    let row = BoxSizer::builder(Orientation::Horizontal).build();
    row.add(
        &size_label,
        0,
        SizerFlag::AlignCenterVertical | SizerFlag::Right,
        8,
    );
    row.add(&spin, 0, SizerFlag::Right, 24);
    row.add(&bold, 0, SizerFlag::AlignCenterVertical, 0);
    sizer.add_sizer(&row, 0, SizerFlag::Left | SizerFlag::Right, 10);
    sizer.add(&preview_label, 0, pad, 10);
    sizer.add(&preview, 0, SizerFlag::Expand | SizerFlag::All, 10);
    let buttons = BoxSizer::builder(Orientation::Horizontal).build();
    buttons.add(&ok, 0, SizerFlag::Right, 8);
    buttons.add(&cancel, 0, SizerFlag::Right, 0);
    sizer.add_sizer(&buttons, 0, SizerFlag::AlignRight | SizerFlag::All, 10);
    panel.set_sizer_and_fit(sizer, true);

    let initial = current
        .face
        .as_deref()
        .and_then(|f| {
            choices
                .iter()
                .position(|c| c.family.eq_ignore_ascii_case(f))
        })
        .unwrap_or(0);
    if !choices.is_empty() {
        list.set_selection(initial as u32, true);
        list.ensure_visible(initial as i32);
    }

    let choices: Vec<Choice> = choices.to_vec();
    let read: Rc<dyn Fn() -> Option<Picked>> = Rc::new(move || {
        let i = list.get_selection()? as usize;
        Some(Picked {
            family: choices.get(i)?.family.clone(),
            size: spin.value(),
            bold: bold.is_checked(),
        })
    });
    let update = {
        let read = Rc::clone(&read);
        move || {
            if let Some(p) = read()
                && let Some(font) = make_font(Some(&p.family), p.size, p.bold)
            {
                preview.set_font(&font);
                preview.refresh(true, None);
                if log {
                    crate::log::line(&format!(
                        "font preview: {} {} {}",
                        p.family,
                        p.size,
                        if p.bold { "bold" } else { "regular" }
                    ));
                }
            }
        }
    };
    update();
    let u = update.clone();
    list.on_selection_changed(move |_| u());
    let u = update.clone();
    spin.on_value_changed(move |_| u());
    let u = update;
    bold.on_toggled(move |_| u());
    list.set_focus();
    Controls { ok, cancel, read }
}

/// Shows the dialog modally and returns the choice, or `None` when
/// cancelled.
pub fn run(parent: &Frame, choices: &[Choice], current: &Applied, log: bool) -> Option<Picked> {
    let dialog = Dialog::builder(parent, "Fonts")
        .with_style(DialogStyle::DefaultDialogStyle)
        .build();
    let panel = Panel::builder(&dialog).build();
    let c = controls(&panel, choices, current, log);
    let outer = BoxSizer::builder(Orientation::Vertical).build();
    outer.add(&panel, 1, SizerFlag::Expand, 0);
    dialog.set_sizer_and_fit(outer, true);
    dialog.centre();
    let answer = (dialog.show_modal() == ID_OK).then(|| (c.read)()).flatten();
    dialog.destroy();
    answer
}

/// Receives the background chooser's answer, once.
type Done = Box<dyn FnOnce(Option<Picked>)>;

/// For automated checks: the same controls in a window of their own,
/// minimized before it is first shown so it is never activated, with no
/// taskbar button. `done` gets the choice (or `None` for Cancel or closing
/// the window) once.
pub fn open_background(
    choices: &[Choice],
    current: &Applied,
    log: bool,
    done: impl FnOnce(Option<Picked>) + 'static,
) {
    let frame = Frame::builder()
        .with_title("Fonts")
        .with_style(FrameStyle::Default | FrameStyle::NoTaskbar)
        .build();
    let panel = Panel::builder(&frame).build();
    let c = controls(&panel, choices, current, log);
    let done: Rc<RefCell<Option<Done>>> = Rc::new(RefCell::new(Some(Box::new(done))));
    let finish = {
        let done = Rc::clone(&done);
        move |picked: Option<Picked>| {
            if let Some(f) = done.borrow_mut().take() {
                f(picked);
            }
        }
    };
    let (read, f) = (Rc::clone(&c.read), finish.clone());
    c.ok.on_click(move |_| {
        f(read());
        frame.close(true);
    });
    let f = finish.clone();
    c.cancel.on_click(move |_| {
        f(None);
        frame.close(true);
    });
    frame.on_close(move |ev| {
        finish(None);
        ev.skip(true);
    });
    frame.iconize(true);
    frame.show(true);
}
