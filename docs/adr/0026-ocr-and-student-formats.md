# ADR-0026: OCR, and formats for students

- Status: accepted
- Date: 2026-09-26 (Saturday, September 26, 2026)

## Context

Students with print disabilities get course material in many shapes: scanned chapters (PDFs with no text layer), photos of handouts, DAISY books from Bookshare, slides, spreadsheets, zip files from a learning platform, and web pages. Before this work, textweaver read a scanned PDF as one sentence saying it could not ("OCR is out of scope", ADR-0010), and refused the rest.

The spirit of this work (in the project briefs, kept outside the repository) is pure Rust and in process first, with a fallback for each bold choice. The research for this work is kept outside the repository.

## Decisions

### OCR: ocrs in process, Tesseract as the fallback

A new crate, `textweaver-ocr`, holds the engines. `textweaver-formats` uses it behind its `ocr` feature, which is on by default.

- **ocrs 0.13.1 on RTen 0.26** reads English in process. It is pure Rust. `rten`, its sibling crates, and `ocrs` build at `opt-level = 3` even in debug builds (the root `Cargo.toml`), because unoptimized they are forty times slower.
- **The models are not bundled.** `tw ocr download` fetches them after saying what they are and asking (`--yes` answers for you): 12.2 MB, CC BY-SA 4.0. Each file is checked by SHA-256 when downloaded and again when loaded. They go to `models/ocr/ocrs/` in the data folder. They are credited in `THIRD-PARTY-NOTICES.md`.
- **Which models.** The files ocrs's own command-line tool uses (`ocrs-models.s3-accelerate.amazonaws.com`, dated September 13, 2026) are used. The January 2024 files on Hugging Face (`robertknight/ocrs`) read far worse with ocrs 0.13: 80% of the words on a clean test page came out wrong, against 8.5% with the newer files. Their address has no version, so if the files change upstream, the SHA-256 check refuses them and the pins in `models.rs` must be updated.
- **Tesseract, as a separate program,** reads other languages, and English when the ocrs models are missing. It is found on `PATH`, in its installers' folders (Program Files on Windows, Homebrew on macOS), or at `TEXTWEAVER_TESSERACT`. It is stopped after two minutes on one page, or when the user cancels.
- **Routing (`textweaver_ocr::plan`).** The language comes from the `ocr_lang` setting, else the PDF's `/Lang`, else English. It is given as Tesseract codes (`fra+eng`) or language tags (`fr-CA`).
  - English goes to ocrs, or to Tesseract when the models are missing.
  - Other languages go to Tesseract. Without Tesseract, a Latin-script language goes to the PaddleOCR Latin model when it is downloaded. Failing that, it goes to ocrs, with a warning that accented letters will be misread.
  - `ocr_engine` (`auto`, `ocrs`, `tesseract`, `paddle`) forces a choice.
  - When nothing can run, the document says what is missing and how to get it, for example: "The English text recognition models (12.2 MB, CC BY-SA 4.0) are not downloaded. To download them, run tw ocr download, or answer yes when textweaver offers them."
- **Which pages.** Only pages with no text and at least one image are recognized; empty pages are not. A PDF with a partial text layer keeps it, and only the other pages are recognized.
- **The page image.**
  - A scanned page is usually one image and nothing else. That image is taken as it is, without rendering the page: a hayro interpreter "device" watches what the page draws, and hayro decodes the image, whatever its filter (JPEG, JPEG 2000, JBIG2, CCITT fax, Flate) and color space. This takes 5 ms against 80 to 150 ms to render a page at 200 dots per inch.
  - A page with more than one image, with drawings, or with a turned image is rendered with hayro 0.7.1 instead.
  - This departs from the research, which suggested lopdf for the extraction. lopdf does not decode CCITT or JBIG2, the filters office scanners use most, while hayro's interpreter decodes them all.
- **Layout.**
  - The recognized words become glyphs on the page, placed where they were printed. The PDF layout engine (ADR-0010) then rebuilds lines, paragraphs, headings (by size), columns, and reading order, and removes running heads and page numbers, as for any PDF.
  - Lines within a third of the page's usual line height share that height. Otherwise a line without descenders would look like smaller text.
  - Pictures (PNG, JPEG) are read the same way, as one page.
- **Speed and resolution.**
  - Images larger than 2,400 pixels on their longer side are scaled down for the in-process engines, and those over 3,400 for Tesseract.
  - Images smaller than 1,500 pixels are enlarged twice for the in-process engines. On a 100 dots per inch page, this cut ocrs's word errors from 27% to 15%.
- **Cache.** Recognized pages are kept in `ocr/` in the cache folder, keyed by the page image, the engine, and the languages. Opening the book again is instant.
- **Progress and cancel.** `LoadOptions::progress` (a `Progress` handle) carries per-page reports, such as "Recognizing text on page 3 (3 of 40).", and a cancel flag. A cancel stops before the next page, and the document says which pages were not read. The handle is not part of the options' identity, so it does not change the document cache key.
- **Warnings.** The document says it was recognized and by which engine: "This PDF has no text layer, so its text was recognized with ocrs (OCR). Recognized text can contain mistakes." Failed and cancelled pages are listed.

### PaddleOCR PP-OCRv5 Latin through RTen: works, experimental

PaddlePaddle's official ONNX export of `latin_PP-OCRv5_mobile_rec` (8.0 MB, Apache-2.0) loads and runs in RTen 0.26 with no missing operators.

- ocrs finds the lines, and the Paddle model reads each one. The line is cut out as ocrs cuts it for its own recognizer, scaled to 48 pixels high, and decoded with greedy CTC over the model's 836-character alphabet, read from its configuration file.
- It reads accents: 16.9% word errors on the French test page, against 40.3% for ocrs.
- It still drops a letter at some line ends, because ocrs's line boxes are tight. Tesseract is better on every test page. So Paddle is offered only as `ocr_engine = "paddle"`, or automatically when Tesseract is missing, and its download is separate: `tw ocr download paddle-latin`.

### Measurements

Release build on the development machine (Windows 11), on the pages in `fixtures/w3d` (made by `make_scans.py`, with a slight tilt, paper noise, and blur). Error rates are word and character edit distances over the truth (lower is better). They were measured with `cargo run --release -p textweaver-ocr --example ocr_eval`.

- **`scan-en.pdf`**, two pages at 200 dots per inch: a grayscale JPEG page, and a 1-bit CCITT G4 page.
  - ocrs: 9.6% words, 1.7% characters, 1.3 s for both pages. The errors are commas read as full stops, and a few `?` for `i`.
  - Tesseract 5.5: no errors, 1.6 s.
  - Paddle: 8.5% words, 1.9% characters, 2.5 s.
- **`scan-fr.png`**, French with accents:
  - Tesseract (`fra`): 2.6% words, 0.2% characters, 0.7 s.
  - Paddle: 16.9% words, 3.7% characters, 0.9 s.
  - ocrs: 40.3% words, 10.6% characters, 0.7 s.
- **`scan-small.png`**, English at 100 dots per inch:
  - Tesseract: 2.1% words, 0.3 s.
  - ocrs, with the page enlarged twice: 29.8% words, 0.4 s.
  - Paddle, also enlarged: 21.3% words.
- Times are from an unloaded machine. With a Docker build running at the same time, the same pages took about twice as long for ocrs and Paddle (2.6 s for `scan-en.pdf`), and Tesseract 1.7 s.
- **A clean rendering** of page 1 (no noise): ocrs 8.5%, Paddle 5.4%, Tesseract 0%.
- **Loading the ocrs models** takes 0.02 to 0.2 s. The first page is slower (up to 3 s), while RTen prepares its plans.

ocrs is fast and good enough to listen to on clean English scans, and it needs nothing installed. Tesseract is still more accurate, and it is the only choice for other languages. That is why it stays the fallback, and why the router sends every non-English page to it.

### DAISY 3 and DTBook

- `DaisyLoader` opens a DAISY 3 package (`.opf`), a DTBook file (`.xml` whose root is `dtbook`; other XML reads as text), and, through the archive loader, a Bookshare zip.
- The DTBook files are read in spine order: the order the SMIL files point into them.
- The NCX's entries become `SectionBreak` markers through their SMIL targets. Without an NCX, the DTBook's `level1`–`level6` become the sections.
- DTBook is turned into HTML and read by the HTML loader's rules, so headings, lists, tables, images, and notes behave as in EPUB and web pages. `pagenum` is not read aloud. It starts a `PageBreak` marker labeled with the printed page number, the same as PDF page labels.

### Archives

- Zip is always read. Tar, tar.gz, and 7z are read with the `archives` feature, on by default, using `tar` 0.4.46, `flate2`, and `sevenz-rust2` 0.23.0 with LZMA and LZMA2.
- Opening an archive lists its readable files as links. A DAISY book or an EPUB inside opens as the book.
- **`book.zip!chapter.pdf`** names a member. `Source::read` reads it, so every loader opens members, and notes and positions are keyed by that whole path. Nested archives work too: `outer.zip!inner.tar!notes.md`. A real file whose name contains `!` still opens as itself.
- **Limits:**
  - 256 MB per member.
  - 2 GB decompressed while searching a tar.gz or solid 7z.
  - 7z dictionaries over 256 MB are refused.
  - Listings stop at 10,000 entries.
  - Archives nest at most four deep.

### Web addresses

- `https://...` and `http://...` are fetched with `ureq`: at most 64 MB, within 60 seconds.
- HTML is decoded by the server's charset, which wins over the page's own.
- Other files, such as a PDF, are saved in the cache's `web/` folder and opened from there.
- The document's path stays the address.
- This is the `url` feature, **off by default**, because it brings an HTTP client, and `cargo xtask deps` keeps the reader free of `ureq`. `tw` turns it on, so `tw open https://...` and `tw text https://...` work. The same applies to model downloads: the `download` feature of `textweaver-ocr`.

### PowerPoint

- Slides are read in presentation order.
- Each slide is a level-1 `SectionBreak` ("Slide 3: Title") and a `PageBreak` labeled with its number.
- The title is a level-1 heading ("Slide 3" when there is none).
- Body placeholders and bulleted paragraphs become lists, nested by outline level, and numbered ones are numbered. Tables are tables. Pictures are read by their alternative text, and pictures without any are left out.
- Speaker notes follow the slide under a level-2 heading, "Speaker notes".
- Slide numbers, dates, and footers are left out.

### Spreadsheets

- CSV and TSV are read with quoted fields and a sniffed separator (comma, semicolon, or tab).
- ODS is read with the crate's own reader.
- XLSX, XLSM, and XLSB are read with `calamine` 0.36.1 (MIT), cell by cell.
- Each sheet is a heading, a section, and a table with its first row as the header. Dates read as `2026-09-26`.
- **Why not calamine's usual reader?** `calamine`'s `worksheet_range` allocates the whole rectangle a sheet claims. A hostile file with cells at A1 and XFD1048576 would ask for 17 billion cells. ODS has the same problem through repeated rows and columns. So both are read cell by cell, with limits of 200,000 cells and 1,000 columns, and the document says when it was cut.
- Old `.xls` files are not read, because calamine's reader for them builds the whole rectangle.

## Consequences

- Scanned PDFs and pictures read aloud with no setup beyond one 12.2 MB download, in process and pure Rust, for English.
- Other languages need Tesseract installed.
- **Binary size.** The reader and `tw` link RTen, ocrs, and hayro. The reader does not link an HTTP client: downloads and web addresses are in `tw`.
- **Progress and cancel in the reader.** PDFs, pictures, archives, and web addresses always open on the app's helper thread (ADR-0024's background open), whatever their size. Its "Still opening" message names the page being recognized ("Still opening scan.pdf: recognizing text on page 3 (3 of 40)."), and Escape cancels the recognition before its next page. The result cache makes the second opening instant. `tw ocr read FILE` shows progress on standard error, and Ctrl+C stops it.
- **Settings.** `[reading] ocr`, `ocr_lang`, and `ocr_engine` are store settings with settings-schema entries, so the settings screen offers them.
- **Fuzz targets:** `daisy`, `pptx`, `sheet`, `archive`, `image`, and `web` (a fetched response read by `web::read_response`, with no request and nothing saved). The nightly workflow's matrix does not list them yet; that file is outside this work.
- **Hostile-input tests:** `crates/textweaver-formats/tests/hostile_w3d.rs`.
- **Not done:**
  - Rotated scans. Tesseract's orientation detection and ocrs's upright assumption are not wired.
  - Table structure from OCR.
  - Handwriting.
  - `.xls`.
  - Password-protected archives.
  - Following links inside web pages to other pages.

## See also

- [ADR-0010: PDF loader](0010-pdf-loader.md)
- [ADR-0024: App core for the GUI](0024-app-core-for-the-gui.md): the background open this uses.
- [Converting documents](../converting.md#formats-textweaver-reads)
