//! Tables in reading (and editing, on the source's table markers): move
//! by row and by cell with Ctrl+Alt and the arrows, as screen readers do,
//! hearing the column header with each cell, and the row and column
//! position in "say position".

use textweaver_a11y::Verbosity;
use textweaver_core::{CharPos, CharRange, Direction, MarkerKind};
use textweaver_lexicon::args;
use textweaver_speech::Earcon;
use textweaver_text::{Document, Marker};

use crate::app::App;

/// Which way a table move goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TableStep {
    /// Down or up a row, in the same column.
    Row,
    /// To the next or previous cell in the row.
    Column,
}

/// A table's rows, each with its cells, from the document's markers.
pub(crate) struct TableGrid {
    /// Row ranges, in order.
    pub(crate) rows: Vec<CharRange>,
    /// Whether each row is the header row.
    pub(crate) header: Vec<bool>,
    /// Each row's cell ranges, in order.
    pub(crate) cells: Vec<Vec<CharRange>>,
}

impl TableGrid {
    /// The grid of the table holding `pos`, if any.
    pub(crate) fn at(doc: &Document, pos: CharPos) -> Option<TableGrid> {
        let index = doc.marker_index();
        let table = index.enclosing(MarkerKind::Table, pos)?.range;
        let inside = |m: &&Marker| table.contains_range(m.range);
        let rows: Vec<&Marker> = index
            .iter(MarkerKind::TableRow, None)
            .filter(inside)
            .collect();
        if rows.is_empty() {
            return None;
        }
        let all_cells: Vec<CharRange> = index
            .iter(MarkerKind::TableCell, None)
            .filter(inside)
            .map(|m| m.range)
            .collect();
        let cells = rows
            .iter()
            .map(|r| {
                all_cells
                    .iter()
                    .copied()
                    .filter(|c| r.range.contains_range(*c))
                    .collect()
            })
            .collect();
        Some(TableGrid {
            rows: rows.iter().map(|r| r.range).collect(),
            header: rows.iter().map(|r| r.is_header_row()).collect(),
            cells,
        })
    }

    /// The row and column holding `pos` (the nearest row and cell when it
    /// is between them, as on a source table's divider line).
    pub(crate) fn locate(&self, pos: CharPos) -> (usize, usize) {
        let row = self.rows.iter().rposition(|r| r.start <= pos).unwrap_or(0);
        let col = self.cells[row]
            .iter()
            .rposition(|c| c.start <= pos)
            .unwrap_or(0);
        (row, col)
    }

    /// The header text of column `col`, if the table has a header row.
    pub(crate) fn header_of(&self, doc: &Document, col: usize) -> Option<String> {
        let h = self.header.iter().position(|&h| h)?;
        let text = doc.slice(*self.cells[h].get(col)?);
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_owned())
    }

    /// The number of rows.
    pub(crate) fn row_count(&self) -> usize {
        self.rows.len()
    }
}

impl App {
    /// Ctrl+Alt+arrows: the next or previous row (same column) or cell.
    pub(crate) fn table_move(&mut self, step: TableStep, dir: Direction) {
        self.refresh_structure(false);
        let Some(pos) = self.reading_position() else {
            return;
        };
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let doc = &s.doc;
        let Some(grid) = TableGrid::at(doc, pos) else {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg("common-not-in-table");
            self.tell(&msg);
            return;
        };
        let (row, col) = grid.locate(pos);
        let target = match (step, dir) {
            (TableStep::Row, Direction::Forward) => (row + 1 < grid.rows.len()).then(|| row + 1),
            (TableStep::Row, Direction::Backward) => row.checked_sub(1),
            (TableStep::Column, _) => Some(row),
        };
        let Some(trow) = target else {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg_args(
                "tables-edge-of-table",
                &args!["dir" => crate::words::dir_key(dir)],
            );
            self.tell(&msg);
            return;
        };
        let cells = &grid.cells[trow];
        let tcol = match step {
            TableStep::Row => Some(col.min(cells.len().saturating_sub(1))),
            TableStep::Column => match dir {
                Direction::Forward => (col + 1 < cells.len()).then(|| col + 1),
                Direction::Backward => col.checked_sub(1),
            },
        };
        let Some(tcol) = tcol.filter(|&c| c < cells.len()) else {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg_args(
                "tables-edge-of-row",
                &args!["dir" => crate::words::dir_key(dir)],
            );
            self.tell(&msg);
            return;
        };
        let cell = cells[tcol];
        let content = doc.slice(cell);
        let content = content.trim();
        let blank = self.msg("nav-blank");
        let content = if content.is_empty() {
            blank.as_str()
        } else {
            content
        };
        let header = if grid.header[trow] {
            None
        } else {
            grid.header_of(doc, tcol)
        };
        let column = match header {
            Some(h) => format!("{h}: {content}"),
            None => content.to_owned(),
        };
        let mut msg = match step {
            TableStep::Row if grid.header[trow] => {
                self.msg_args("tables-header-row", &args!["cell" => column])
            }
            TableStep::Row => {
                self.msg_args("tables-row", &args!["row" => trow + 1, "cell" => column])
            }
            TableStep::Column => column,
        };
        if self.settings.speech.verbosity >= Verbosity::High {
            msg = self.msg_args(
                "tables-with-position",
                &args![
                    "message" => msg,
                    "row" => trow + 1,
                    "rows" => grid.row_count(),
                    "col" => tcol + 1,
                    "cols" => cells.len()
                ],
            );
        }
        self.caret_to(cell.start);
        self.speak_content(textweaver_a11y::Channel::Caret, &msg);
    }

    /// "Table, row 2 of 5, column 3 of 4." for "say position", when the
    /// cursor is in a table.
    pub(crate) fn table_position(&self, pos: CharPos) -> Option<String> {
        let s = self.session.as_ref()?;
        let grid = TableGrid::at(&s.doc, pos)?;
        let (row, col) = grid.locate(pos);
        let cols = grid.cells[row].len();
        Some(self.msg_args(
            "tables-position",
            &args![
                "row" => row + 1,
                "rows" => grid.row_count(),
                "col" => col + 1,
                "cols" => cols
            ],
        ))
    }
}
