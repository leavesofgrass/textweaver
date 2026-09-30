//! Makes the scanned-page pictures in `fixtures/c5` (Agent W6c5), which
//! `fixtures/c5/make_fixtures.py` then wraps in PDFs:
//!
//! ```text
//! tw convert fixtures/c5/table-source.md --to pdf --out OUT
//! cargo run -p textweaver-ocr --example make_c5_scans -- OUT/table-source.pdf
//! python fixtures/c5/make_fixtures.py
//! ```
//!
//! - `sideways-scan.png`: page 1 of `fixtures/w3d/scan-en.pdf` (a tilted,
//!   noisy scan) turned a quarter counter-clockwise, as a book laid across
//!   the scanner comes out. It reads bottom to top.
//! - `table-scan.png`: `table-source.md` (a table with its caption) as
//!   `tw convert` writes it to PDF, with its fonts embedded, rendered at 200
//!   dots per inch and roughened with a little noise so it is not a perfect
//!   rendering. (A page in an unembedded standard font would render blank:
//!   hayro is built without its bundled fonts.)
//!
//! The text is written for these fixtures; nothing is copied.

use std::path::{Path, PathBuf};

use textweaver_ocr::GrayImage;
use textweaver_ocr::pdf::PdfPages;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Paper noise: a few specks, so the page is not a perfect rendering.
fn roughen(img: &mut GrayImage) {
    let mut seed = 20_260_929u32;
    for v in &mut img.data {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let r = (seed >> 16) % 1000;
        if r == 0 {
            *v = 60;
        } else if r < 6 {
            *v = v.saturating_sub(18);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = root().join("fixtures/c5");
    std::fs::create_dir_all(&out)?;

    let scan = std::fs::read(root().join("fixtures/w3d/scan-en.pdf"))?;
    let (page, _) = PdfPages::open(scan)?.page_image(0)?;
    let sideways = page.rotate(3);
    std::fs::write(out.join("sideways-scan.png"), sideways.to_png()?)?;
    println!(
        "sideways-scan.png: {} by {} pixels",
        sideways.width, sideways.height
    );

    let source = std::env::args()
        .nth(1)
        .ok_or("give the table's PDF, made by tw convert from table-source.md")?;
    let (mut table, _) = PdfPages::open(std::fs::read(source)?)?.page_image(0)?;
    roughen(&mut table);
    std::fs::write(out.join("table-scan.png"), table.to_png()?)?;
    println!("table-scan.png: {} by {} pixels", table.width, table.height);
    Ok(())
}
