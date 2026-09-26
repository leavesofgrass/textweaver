//! Speech Cursor (line) mode: each move reads exactly one line of canonical
//! text, "blank" for an empty line; moves clamp and never wrap; Enter leaves
//! and reads on (Star's TUI and GUI behavior).
//!
//! Lines are canonical-text lines, not wrapped display lines: textweaver's
//! canonical text keeps list items, table rows, and plain-text lines on their
//! own lines (ADR-0002), so a long paragraph is one line. Other navigation
//! commands used in this mode move the Speech Cursor to the line of their
//! target and read that line.

use textweaver_speech::Earcon;

use crate::app::{App, Mode};
use crate::text_util;

impl App {
    pub(crate) fn speech_cursor_toggle(&mut self) {
        if self.mode == Mode::SpeechCursor {
            self.stop_speech();
            self.leave_speech_cursor();
            self.note("Speech Cursor off.");
        } else {
            self.enter_speech_cursor();
        }
    }

    fn enter_speech_cursor(&mut self) {
        let Some(pos) = self.reading_position() else {
            return;
        };
        self.stop_speech();
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let line = text_util::line_of(&s.doc, pos);
        s.speech_cursor_line = Some(line);
        s.cursor = text_util::line_range(&s.doc, line).start;
        self.mode = Mode::SpeechCursor;
        self.speech.earcon(Earcon::ModeOn);
        self.scroll_to_line(line);
        self.speech_cursor_read();
        self.show(&format!(
            "Speech Cursor on, line {}. Up and Down read lines, Enter reads on, Tab or Escape leaves.",
            line + 1
        ));
    }

    /// Leaves the mode, putting the cursor on the first word at or after the
    /// Speech Cursor line (Star's rule).
    pub(crate) fn leave_speech_cursor(&mut self) {
        if self.mode != Mode::SpeechCursor {
            return;
        }
        self.mode = Mode::Browse;
        self.speech.earcon(Earcon::ModeOff);
        if let Some(s) = self.session.as_mut()
            && let Some(line) = s.speech_cursor_line.take()
        {
            let start = text_util::line_range(&s.doc, line).start;
            s.cursor = text_util::first_word_at_or_after(&s.doc, start).max(start);
        }
        self.scroll_to_cursor();
    }

    /// Reads the Speech Cursor line.
    pub(crate) fn speech_cursor_read(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let Some(line) = s.speech_cursor_line else {
            return;
        };
        let range = text_util::line_range(&s.doc, line);
        self.read_line_range(range);
    }

    pub(crate) fn speech_cursor_move(&mut self, delta: isize) {
        if self.mode != Mode::SpeechCursor {
            self.enter_speech_cursor();
            return;
        }
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let last = text_util::line_count(&s.doc) - 1;
        let line = s.speech_cursor_line.unwrap_or(0);
        let next = line.saturating_add_signed(delta).min(last);
        if next == line {
            self.speech.earcon(Earcon::Boundary);
            self.tell(if delta < 0 {
                "Top of document."
            } else {
                "End of document."
            });
            return;
        }
        s.speech_cursor_line = Some(next);
        s.cursor = text_util::line_range(&s.doc, next).start;
        self.scroll_to_line(next);
        self.speech_cursor_read();
    }

    pub(crate) fn speech_cursor_reread(&mut self) {
        if self.mode != Mode::SpeechCursor {
            self.read_current_line();
        } else {
            self.speech_cursor_read();
        }
    }

    pub(crate) fn speech_cursor_exit_and_read(&mut self) {
        if self.mode == Mode::SpeechCursor {
            self.stop_speech();
            self.leave_speech_cursor();
        }
        self.read_from_cursor();
    }
}
