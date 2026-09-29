//! `braille-check GRADE IN.md OUT.brf OUT.txt`: writes `IN.md` as BRF in
//! UEB grade 1 (`1`, textweaver's native translator) or grade 2 (`2`,
//! through liblouis), without braille page numbers, and the document's
//! text to `OUT.txt`, for `compare.py`. Each warning of the writer is
//! printed as a line starting "Warning:". Exits 1 when loading or writing
//! fails, and 2 on a usage error.

use std::process::ExitCode;

use textweaver_writers::{BrailleGrade, BrailleOptions, Format, WriteOptions, write_to_vec};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [grade, input, brf, text] = args.as_slice() else {
        eprintln!("usage: braille-check 1|2 IN.md OUT.brf OUT.txt");
        return ExitCode::from(2);
    };
    let grade = match grade.as_str() {
        "1" => BrailleGrade::One,
        "2" => BrailleGrade::Two,
        other => {
            eprintln!("Fail: grade {other} is not 1 or 2");
            return ExitCode::from(2);
        }
    };
    match run(grade, input, brf, text) {
        Ok(warnings) => {
            for w in warnings {
                println!("Warning: {w}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Fail: {input}: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(grade: BrailleGrade, input: &str, brf: &str, text: &str) -> Result<Vec<String>, String> {
    let doc = textweaver_formats::load_path(input).map_err(|e| e.to_string())?;
    let options = WriteOptions {
        braille: BrailleOptions {
            grade,
            // Page numbers are layout, which the writer's own tests check;
            // this compares the translation.
            page_numbers: false,
            ..BrailleOptions::default()
        },
        ..WriteOptions::default()
    };
    let (bytes, report) = write_to_vec(&doc, Format::Brf, &options).map_err(|e| e.to_string())?;
    std::fs::write(brf, bytes).map_err(|e| format!("cannot write {brf}: {e}"))?;
    std::fs::write(text, doc.text().to_string())
        .map_err(|e| format!("cannot write {text}: {e}"))?;
    Ok(report.warnings)
}
