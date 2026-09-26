//! Spreadsheets as tables: CSV and TSV, OpenDocument (`.ods`), and
//! (feature `spreadsheets`) Excel workbooks (`.xlsx`, `.xlsm`, `.xlsb`)
//! through `calamine`.
//!
//! - Each sheet of a workbook is a level-1 heading (its name) and a
//!   `SectionBreak`, then a table labeled with the sheet's name. A CSV or
//!   TSV file is one table.
//! - The first row is read as the header row. Rows with nothing in them are
//!   left out; columns run from the first to the last one used.
//! - Numbers read as they are stored (`3`, `2.5`); dates as `2026-09-26`
//!   (with the time when there is one); formulas by their last calculated
//!   value.
//! - CSV: quoted fields (with `""` for a quote and line breaks inside),
//!   and the separator found from the first line (comma, semicolon, or tab).
//!
//! Limits, so a hostile file cannot claim a million by sixteen thousand
//! cells and exhaust memory: cells are read one at a time (never as the
//! whole rectangle a sheet claims), at most [`MAX_CELLS`] of them and
//! [`MAX_COLUMNS`] columns wide; the document says when it was cut short.
//! Old binary Excel files (`.xls`) are not read, because their reader
//! builds the whole rectangle in memory.

use std::collections::BTreeMap;

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, HEADER_ROW_LABEL, Marker};

use crate::builder::Builder;
use crate::{LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// Most cells read from one file.
pub const MAX_CELLS: usize = 200_000;

/// Most columns read from one sheet.
pub const MAX_COLUMNS: u32 = 1_000;

/// Most sheets read from one workbook.
pub const MAX_SHEETS: usize = 500;

/// One sheet: its name and its cells by (row, column).
#[derive(Debug, Default)]
struct Sheet {
    name: String,
    cells: BTreeMap<(u32, u32), String>,
}

/// Counts cells across sheets and notes when a limit cut something.
#[derive(Debug, Default)]
struct Budget {
    cells: usize,
    cut: bool,
}

impl Budget {
    /// Adds a cell (empty values are skipped); false once the file's limit
    /// is reached.
    fn add(&mut self, sheet: &mut Sheet, row: u32, col: u32, value: String) -> bool {
        if self.cells >= MAX_CELLS {
            self.cut = true;
            return false;
        }
        if col >= MAX_COLUMNS {
            self.cut = true;
            return true;
        }
        let value = value.trim().to_owned();
        if !value.is_empty() {
            sheet.cells.insert((row, col), value);
            self.cells += 1;
        }
        true
    }
}

/// Loads spreadsheets.
#[derive(Clone, Copy, Debug, Default)]
pub struct SheetLoader;

impl Loader for SheetLoader {
    fn id(&self) -> &'static str {
        "sheet"
    }

    fn extensions(&self) -> &'static [&'static str] {
        if cfg!(feature = "spreadsheets") {
            &["csv", "tsv", "tab", "ods", "xlsx", "xlsm", "xlsb"]
        } else {
            &["csv", "tsv", "tab", "ods"]
        }
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, _options: &LoadOptions) -> Result<Document, LoadError> {
        let hint = source.hint().unwrap_or_default();
        let mut meta = meta_for(source, self.id());
        let mut budget = Budget::default();
        let (sheets, workbook) = match hint.as_str() {
            "ods" => (ods(&source.read()?, &mut budget)?, true),
            #[cfg(feature = "spreadsheets")]
            "xlsx" | "xlsm" | "xlsb" => (excel(source.read()?, &hint, &mut budget)?, true),
            _ => {
                let decoded = crate::decode_source(source, None)?;
                crate::note_encoding(&mut meta, &decoded);
                let sep = match hint.as_str() {
                    "tsv" | "tab" => '\t',
                    _ => sniff_separator(&decoded.text),
                };
                (vec![csv(&decoded.text, sep, &mut budget)], false)
            }
        };
        if budget.cut {
            crate::add_warning(
                &mut meta,
                "This spreadsheet is very large, so only part of it was read.",
            );
        }
        meta.title = meta.title.or_else(|| title_from_path(source));
        meta.properties
            .insert("sheets".into(), sheets.len().to_string());
        let (text, markers) = write(&sheets, workbook);
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

fn marker(kind: MarkerKind) -> Marker {
    Marker::new(kind, CharRange::empty(0))
}

/// The sheets as canonical text: headings and sections for workbooks.
fn write(sheets: &[Sheet], workbook: bool) -> (String, Vec<Marker>) {
    let mut b = Builder::new();
    for sheet in sheets {
        b.paragraph_break();
        let section = workbook.then(|| {
            b.open(
                marker(MarkerKind::SectionBreak)
                    .with_level(1)
                    .with_label(sheet.name.clone()),
            )
        });
        if workbook {
            let h = b.open(marker(MarkerKind::Heading).with_level(1));
            b.text(&sheet.name);
            b.close(h);
        }
        if sheet.cells.is_empty() {
            if workbook {
                b.paragraph_break();
                b.text("This sheet is empty.");
            }
        } else {
            table(&mut b, sheet);
        }
        if let Some(s) = section {
            b.close(s);
        }
    }
    b.finish()
}

fn table(b: &mut Builder, sheet: &Sheet) {
    let first_col = sheet.cells.keys().map(|(_, c)| *c).min().unwrap_or(0);
    let last_col = sheet.cells.keys().map(|(_, c)| *c).max().unwrap_or(0);
    b.paragraph_break();
    let mut tm = marker(MarkerKind::Table);
    if !sheet.name.is_empty() {
        tm = tm.with_label(sheet.name.clone());
    }
    let t = b.open(tm);
    let mut rows: Vec<u32> = sheet.cells.keys().map(|(r, _)| *r).collect();
    rows.dedup();
    for (i, row) in rows.iter().enumerate() {
        b.line_break();
        let mut rm = marker(MarkerKind::TableRow);
        if i == 0 {
            rm = rm.with_label(HEADER_ROW_LABEL);
        }
        let r = b.open(rm);
        for col in first_col..=last_col {
            if col > first_col {
                b.separator(crate::CELL_SEPARATOR);
            }
            let cell = b.open_here(marker(MarkerKind::TableCell));
            if let Some(v) = sheet.cells.get(&(*row, col)) {
                b.text(v);
            }
            b.close(cell);
        }
        b.close(r);
    }
    b.close(t);
    b.paragraph_break();
}

/// The separator a CSV file uses: whichever of comma, semicolon, and tab
/// appears most on its first line (outside quotes).
fn sniff_separator(text: &str) -> char {
    let first = text.lines().next().unwrap_or("");
    let mut quoted = false;
    let mut counts = [0usize; 3];
    for c in first.chars() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => counts[0] += 1,
            ';' if !quoted => counts[1] += 1,
            '\t' if !quoted => counts[2] += 1,
            _ => {}
        }
    }
    let seps = [',', ';', '\t'];
    let best = (0..3)
        .max_by_key(|&i| (counts[i], usize::from(i == 0)))
        .unwrap_or(0);
    seps[best]
}

/// Parses CSV (RFC 4180, with `sep` between fields).
fn csv(text: &str, sep: char, budget: &mut Budget) -> Sheet {
    let mut sheet = Sheet::default();
    let (mut row, mut col) = (0u32, 0u32);
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let mut push = |sheet: &mut Sheet, row: u32, col: u32, field: &mut String| -> bool {
        budget.add(sheet, row, col, std::mem::take(field))
    };
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.trim().is_empty() => {
                field.clear();
                quoted = true;
            }
            '\n' => {
                if !push(&mut sheet, row, col, &mut field) {
                    return sheet;
                }
                row = row.saturating_add(1);
                col = 0;
            }
            c if c == sep => {
                if !push(&mut sheet, row, col, &mut field) {
                    return sheet;
                }
                col = col.saturating_add(1);
            }
            c => field.push(c),
        }
    }
    push(&mut sheet, row, col, &mut field);
    sheet
}

/// A day count since 1970-01-01 as a civil date (Howard Hinnant's
/// algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// An Excel date serial (1900 system) as `2026-09-26` or
/// `2026-09-26 14:30`; a time alone as `14:30`.
fn excel_date(serial: f64) -> String {
    if !serial.is_finite() || !(0.0..3_000_000.0).contains(&serial) {
        return format_number(serial);
    }
    let days = serial.floor();
    let minutes = ((serial - days) * 1440.0).round() as i64;
    let (h, m) = (minutes / 60 % 24, minutes % 60);
    if days < 1.0 {
        return format!("{h:02}:{m:02}");
    }
    // Serial 25,569 is 1970-01-01; serials before 61 count Excel's
    // nonexistent 1900-02-29.
    let mut d = days as i64 - 25_569;
    if days < 61.0 {
        d += 1;
    }
    let (y, mo, da) = civil(d);
    if minutes == 0 {
        format!("{y:04}-{mo:02}-{da:02}")
    } else {
        format!("{y:04}-{mo:02}-{da:02} {h:02}:{m:02}")
    }
}

/// A number as a person would write it: no trailing zeros, no binary
/// rounding noise (`0.30000000000000004` reads `0.3`).
fn format_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{v:.0}");
    }
    let s = format!("{v:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_owned() }
}

#[cfg(feature = "spreadsheets")]
fn cell_text(v: &calamine::DataRef<'_>) -> String {
    use calamine::DataRef;
    match v {
        DataRef::Int(i) => i.to_string(),
        DataRef::Float(f) => format_number(*f),
        DataRef::String(s) => s.clone(),
        DataRef::SharedString(s) => (*s).to_owned(),
        DataRef::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_owned(),
        DataRef::DateTime(d) if d.is_datetime() => excel_date(d.as_f64()),
        DataRef::DateTime(d) => format_number(d.as_f64()),
        DataRef::DateTimeIso(s) | DataRef::DurationIso(s) => s.clone(),
        DataRef::Error(e) => e.to_string(),
        DataRef::Empty => String::new(),
    }
}

/// Excel workbooks, read cell by cell.
#[cfg(feature = "spreadsheets")]
fn excel(bytes: Vec<u8>, hint: &str, budget: &mut Budget) -> Result<Vec<Sheet>, LoadError> {
    use calamine::{Reader, Xlsb, Xlsx};
    use std::io::Cursor;
    let bad = |e: &dyn std::fmt::Display| LoadError::Parse(format!("not a readable workbook: {e}"));
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut sheets = Vec::new();
        if hint == "xlsb" {
            let mut wb: Xlsb<_> =
                calamine::open_workbook_from_rs(Cursor::new(bytes)).map_err(|e| bad(&e))?;
            for name in wb.sheet_names().into_iter().take(MAX_SHEETS) {
                let mut sheet = Sheet {
                    name: name.clone(),
                    ..Sheet::default()
                };
                let mut cells = wb.worksheet_cells_reader(&name).map_err(|e| bad(&e))?;
                while let Some(cell) = cells.next_cell().map_err(|e| bad(&e))? {
                    let (r, c) = cell.get_position();
                    if !budget.add(&mut sheet, r, c, cell_text(cell.get_value())) {
                        break;
                    }
                }
                sheets.push(sheet);
            }
        } else {
            let mut wb: Xlsx<_> =
                calamine::open_workbook_from_rs(Cursor::new(bytes)).map_err(|e| bad(&e))?;
            for name in wb.sheet_names().into_iter().take(MAX_SHEETS) {
                let mut sheet = Sheet {
                    name: name.clone(),
                    ..Sheet::default()
                };
                let mut cells = wb.worksheet_cells_reader(&name).map_err(|e| bad(&e))?;
                while let Some(cell) = cells.next_cell().map_err(|e| bad(&e))? {
                    let (r, c) = cell.get_position();
                    if !budget.add(&mut sheet, r, c, cell_text(cell.get_value())) {
                        break;
                    }
                }
                sheets.push(sheet);
            }
        }
        Ok(sheets)
    }));
    run.unwrap_or_else(|_| Err(LoadError::Parse("not a readable workbook".into())))
}

/// OpenDocument spreadsheets, read from `content.xml` with repeated rows
/// and cells expanded only as far as they hold something.
fn ods(bytes: &[u8], budget: &mut Budget) -> Result<Vec<Sheet>, LoadError> {
    let mut pkg = crate::package::Package::open(bytes.to_vec(), "OpenDocument spreadsheet")?;
    let content = pkg
        .read_text("content.xml")?
        .ok_or_else(|| LoadError::Parse("not a spreadsheet: no content.xml".into()))?;
    let xml = crate::package::parse_xml(&content)?;
    let mut sheets = Vec::new();
    for table in xml
        .descendants()
        .filter(|n| {
            n.tag_name().name() == "table"
                && n.tag_name()
                    .namespace()
                    .is_some_and(|ns| ns.contains("table"))
        })
        .take(MAX_SHEETS)
    {
        let mut sheet = Sheet {
            name: table
                .attributes()
                .find(|a| a.name() == "name")
                .map(|a| a.value().to_owned())
                .unwrap_or_default(),
            ..Sheet::default()
        };
        let mut row = 0u32;
        'rows: for tr in table
            .descendants()
            .filter(|n| n.tag_name().name() == "table-row")
        {
            let repeat = |n: roxmltree::Node<'_, '_>, attr: &str| {
                n.attributes()
                    .find(|a| a.name() == attr)
                    .and_then(|a| a.value().parse::<u32>().ok())
                    .unwrap_or(1)
                    .max(1)
            };
            let rows = repeat(tr, "number-rows-repeated");
            let mut cells: Vec<(u32, u32, String)> = Vec::new();
            let mut col = 0u32;
            for tc in tr
                .children()
                .filter(|n| matches!(n.tag_name().name(), "table-cell" | "covered-table-cell"))
            {
                let cols = repeat(tc, "number-columns-repeated");
                let text: Vec<String> = tc
                    .children()
                    .filter(|n| n.tag_name().name() == "p")
                    .map(|p| {
                        p.descendants()
                            .filter(|n| n.is_text())
                            .filter_map(|n| n.text())
                            .collect()
                    })
                    .collect();
                let text = text.join(" ");
                if !text.trim().is_empty() {
                    // A value repeated across columns is written into each,
                    // up to the column limit.
                    let span = cols.min(MAX_COLUMNS.saturating_sub(col));
                    cells.push((col, span, text));
                }
                col = col.saturating_add(cols);
            }
            if cells.is_empty() {
                row = row.saturating_add(rows);
                continue;
            }
            // Repeated rows with content are expanded up to the budget.
            for _ in 0..rows {
                for (c, span, text) in &cells {
                    for k in 0..*span {
                        if !budget.add(&mut sheet, row, c + k, text.clone()) {
                            break 'rows;
                        }
                    }
                }
                row = row.saturating_add(1);
            }
        }
        sheets.push(sheet);
    }
    Ok(sheets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(data: &[u8], hint: &str) -> Document {
        SheetLoader
            .load(
                &Source::Bytes {
                    data: data.to_vec(),
                    hint: hint.into(),
                },
                &LoadOptions::default(),
            )
            .unwrap()
    }

    #[test]
    fn csv_quotes_separators_and_empty_rows() {
        let d = load(
            b"Name,Note,Score\n\"Smith, Ann\",\"said \"\"hi\"\"\nthen left\",9\n\n,,\nBo,,7\n",
            "csv",
        );
        assert_eq!(
            d.text().to_string(),
            "Name | Note | Score\nSmith, Ann | said \"hi\" then left | 9\nBo |  | 7"
        );
        let rows: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::TableRow, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(rows[0].as_deref(), Some(HEADER_ROW_LABEL));
        assert_eq!(rows.len(), 3);
        let semi = load("a;b\n1;2".as_bytes(), "csv");
        assert_eq!(semi.text().to_string(), "a | b\n1 | 2");
        let tsv = load(b"a\tb,c\n1\t2", "tsv");
        assert_eq!(tsv.text().to_string(), "a | b,c\n1 | 2");
    }

    #[test]
    fn numbers_and_dates_read_well() {
        assert_eq!(format_number(3.0), "3");
        assert_eq!(format_number(0.1 + 0.2), "0.3");
        assert_eq!(format_number(-2.5), "-2.5");
        assert_eq!(excel_date(46291.0), "2026-09-26");
        assert_eq!(excel_date(46291.604_166_666_7), "2026-09-26 14:30");
        assert_eq!(excel_date(0.5), "12:00");
        assert_eq!(excel_date(1.0), "1900-01-01");
        assert_eq!(civil(0), (1970, 1, 1));
    }

    #[test]
    fn huge_csv_is_cut_with_a_warning() {
        let mut data = String::new();
        let wide = vec!["x"; (MAX_COLUMNS + 5) as usize].join(",");
        data.push_str(&wide);
        let d = load(data.as_bytes(), "csv");
        assert_eq!(
            d.marker_index().iter(MarkerKind::TableCell, None).count(),
            MAX_COLUMNS as usize
        );
        assert_eq!(crate::warnings(&d.meta).len(), 1);
    }

    fn ods_file(content: &str) -> Vec<u8> {
        use std::io::Write;
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        z.start_file("content.xml", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(content.as_bytes()).unwrap();
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn ods_sheets_and_hostile_repeats() {
        let content = r#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"><office:body><office:spreadsheet>
<table:table table:name="Marks"><table:table-row><table:table-cell><text:p>Name</text:p></table:table-cell><table:table-cell><text:p>Mark</text:p></table:table-cell></table:table-row>
<table:table-row><table:table-cell><text:p>Ann</text:p></table:table-cell><table:table-cell><text:p>9</text:p></table:table-cell><table:table-cell table:number-columns-repeated="16000"/></table:table-row>
<table:table-row table:number-rows-repeated="1048000"><table:table-cell table:number-columns-repeated="16384"/></table:table-row></table:table>
<table:table table:name="Bomb"><table:table-row table:number-rows-repeated="1048576"><table:table-cell table:number-columns-repeated="16384"><text:p>x</text:p></table:table-cell></table:table-row></table:table>
</office:spreadsheet></office:body></office:document-content>"#;
        let d = load(&ods_file(content), "ods");
        let text = d.text().to_string();
        assert!(
            text.starts_with("Marks\n\nName | Mark\nAnn | 9\n\nBomb"),
            "{}",
            &text[..80]
        );
        assert_eq!(crate::warnings(&d.meta).len(), 1);
        assert!(d.marker_index().iter(MarkerKind::TableCell, None).count() <= MAX_CELLS + 4);
    }
}
