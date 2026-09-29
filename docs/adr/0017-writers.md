# ADR-0017: Native writers (EPUB 3, DOCX, BRF, tagged PDF)

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented with the amendment above. Of the two Consequences lines about fonts, the second is current: PDF output does not need an installed font. MathML in EPUB and DOCX, real Word footnotes, and native grade 2 braille are still to do.
- Status update (Saturday, September 26, 2026, Phase 1, Agent P1d): math is typeset by every writer: MathML in EPUB, Word equations in DOCX, print form with its spoken description in PDF, and the spoken form in braille. Of the first update's list, real Word footnotes and native grade 2 braille are still to do.
- Status update (Monday, September 28, 2026, Wave 5, Agent W5g): real Word footnotes and PDF page labels from print page breaks are done, with the publishing templates; see [ADR-0041](0041-publishing-templates.md).

## Context

The owner asked for conversion that is fast, native Rust, and memory safe, with EPUB, DOCX, braille (BRF), and PDF outputs beyond Markdown, HTML, and text. Star exported PDF and DOCX through Pandoc or Qt and BRF through its own grade 1 table (`star/braille.py`) or liblouis. Star's grade 1 table has wrong UEB symbols (both parentheses as dots 2-3-5-6, the slash as dots 3-4, straight double quotes always as the opening quote) and drops accented letters it cannot fold. Students with print disabilities use these files with screen readers, braille displays, and embossers, so structure and accessibility metadata matter as much as the text.

The workspace offers `zip`, `krilla` (PDF with tagging and PDF/UA validation), and `roxmltree`; no XML writer, font, or braille crate.

## Decision

`textweaver-writers` writes every format from a `Document` (ADR-0002) in pure Rust:

- **One block tree** (`model::blocks`) turns markers into headings, paragraphs, lists (ordered from item labels), tables (header rows from `HEADER_ROW_LABEL`, cells from `TableCell` markers or the `" | "` separator), code, quotes, figures (a paragraph that is one image), footnote bodies, and section and page breaks, with inline spans for bold, italic, underline, code, links, images, and footnote references. Overlapping inline markers are split so every writer emits well-nested markup. Plain text becomes paragraphs at blank lines with its single line breaks kept.
- **`Writer` trait**, the contract for Agent L's converter:

  ```rust
  pub trait Writer: Send + Sync {
      fn format(&self) -> Format;
      fn write(&self, doc: &Document, options: &WriteOptions, out: &mut dyn Write)
          -> Result<WriteReport, WriteError>;
  }
  pub fn writer_for(format: Format) -> Box<dyn Writer>;
  pub fn write_to_vec(doc: &Document, format: Format, options: &WriteOptions)
      -> Result<(Vec<u8>, WriteReport), WriteError>;
  ```

  `WriteOptions` (serde, all fields defaulted) carries title, language, and author overrides, a timestamp for reproducible output, the folder images resolve against, and per-format sections. `WriteReport::warnings` are sentences that read well aloud (an image not found, characters with no braille symbol). Containers are assembled in memory and written once, so `out` needs no `Seek`.
- **XML by hand** (`xml.rs`: escaping, dropping characters XML 1.0 forbids, NCName ids), no new dependency.
- **EPUB 3**: one XHTML content document per chapter (split at section breaks, else before level 1 headings), `nav.xhtml` (table of contents from the heading outline, landmarks, page list from print page breaks), an NCX for EPUB 2 reading systems, and images found on disk. Semantics: `h1`–`h6`, `ol start`/`ul`, tables with `caption`, `thead`, `th scope="col"`, `figure`/`img alt`, `pre`/`code`, `blockquote`, footnotes as `aside epub:type="footnote" role="doc-footnote"` with `noteref` links, `epub:type="pagebreak"` spans, `lang` and `xml:lang` everywhere. Package metadata: `schema:accessMode`, `accessModeSufficient`, `accessibilityFeature` (structural navigation, table of contents, reading order, alternative text, page navigation, print page numbers as they apply), `accessibilityHazard none`, and an `accessibilitySummary`. No WCAG conformance claim: that needs a human evaluation. The identifier is a stable `urn:uuid` (RFC 9562 version 8) hashed from the title and text. Links to files outside the book are written as text, since they cannot resolve inside the package.
- **DOCX**: built-in `heading 1`–`6` styles with outline levels; real numbering (one `w:num` per list so numbered lists restart; nesting as list levels); `Table Grid` tables with a repeating, marked header row (`w:tblHeader`) and the caption as a `Caption` paragraph and `w:tblCaption`; images embedded inline with `wp:docPr/@descr`; external links as hyperlinks; footnote references as internal hyperlinks to a bookmark on the footnote text (kept in place rather than moved into Word footnotes, so the text matches the document); document language in the default run properties and core properties; `compatibilityMode` 15 so Word does not open it in compatibility mode.
- **BRF**: UEB grade 1 translated natively (`ueb.rs`, tables in code): capital, capitals word, and capitals terminator indicators; numeric mode with decimal point, digit-group comma, and fraction line; the grade 1 indicator before a–j after a number; UEB punctuation and signs with directional quotes chosen by position; accented letters as modifier plus letter; Greek letters. Layout follows BANA Braille Formats in simplified form: 40 cells by 25 lines (configurable), centered, cell 5, and cell 7 headings kept with the next line, paragraphs in cell 3, list items with runovers, linear tables, the print page change indicator, braille page numbers on the last line, the line continuation indicator for divided words, CR LF lines and form feeds. Grade 2 goes through liblouis's `lou_translate` (display table `en-us-brf.dis`, table `en-ueb-g2.ctb` by default) behind the `liblouis` cargo feature; without it, or without liblouis installed, the writer falls back to grade 1 and says so in the report.
- **Tagged PDF**: krilla with its PDF/UA-1 validator on by default. One-column layout (Letter by default, 12-point text, 1.5 line spacing, one-inch margins) with every piece of text tagged in reading order: `H1`–`H6` with titles, `P`, `L` (list numbering) with `LI`/`Lbl`/`LBody`, `Table`/`Caption`/`TR`/`TH` (column scope)/`TD`, `Figure` with alt text, `BlockQuote`, `Code`, `Note`, and `Link` with its tagged link annotation. Table rules, repeated table headers, decorative images, and "Page N of M" footers are artifacts. The document has a title (displayed in the window title), a language on the catalog and the structure root, an outline from the headings, and embedded subset fonts. Text is measured with a small OpenType reader (`head`, `hhea`, `hmtx`, `cmap` formats 4 and 12, the `OS/2` embedding permission); fonts come from the `font` option, `TEXTWEAVER_PDF_FONT`, or the first installed family of Atkinson Hyperlegible, Verdana, Segoe UI, Arial, DejaVu Sans, Liberation Sans, and Noto Sans, with a monospaced font for code and per-character fallback fonts. Characters no font can show become `?` and are reported, because a `.notdef` glyph fails PDF/UA.

## Accessibility checks

| Format | Automated check | Result on 2026-09-25 |
|---|---|---|
| EPUB | OCF layout (mimetype first, stored, no extra field); every XML part parses; manifest and spine consistent; every internal link and fragment resolves; required tags present; accessibility metadata; round trip of every chapter through the HTML loader keeps every word in order (footnote bodies aside: the loader skips `aside`) | pass |
| EPUB | epubcheck (`epubcheck` on the PATH, or Java with `EPUBCHECK_JAR`) | skipped: neither Java nor epubcheck is installed on Windows or in the container |
| DOCX | every part parses; content types cover every part; every relationship target exists; every style used is defined; heading styles and outline levels; list numbering and nesting; header rows; image descriptions; every word read back in order | pass |
| DOCX | Microsoft Word opens the file through COM (ignored test, `TEXTWEAVER_WORD=1`): headings at outline levels 1–3, 2 lists, 1 table whose first row repeats as a header, the picture's alternative text | pass on Windows with Word 16 |
| BRF | UEB grade 1 vectors (letters, capitals, numbers, punctuation, accents, Greek); page geometry, CR LF, form feeds, page numbers; layout of headings, lists, tables, code, print page indicators | pass |
| BRF | liblouis grade 2 | skipped: liblouis is not installed on Windows or in the container |
| PDF | krilla's PDF/UA-1 validation while writing (title, language, tagging, alt text, heading titles, outline, character mappings, embeddable fonts); structure tree, attributes, outline, metadata, and annotations present in the file; long tables tag every row once; `pdftotext -raw` reads every word in order | pass (Windows with Verdana; container with DejaVu Sans; `pdftotext` only on Windows) |

What krilla's validator does not check, and textweaver does not yet do: a veraPDF or PAC run (neither is installed), `Lang` on spans in another language, `ActualText` for hyphenation (textweaver does not hyphenate), and table header association beyond column scope.

## Amendment: bundled fonts and PDF options (2026-09-25, Agent W)

The owner asked for bundled fonts and quick wins for PDF export.

- **Fonts.** The new `textweaver-fonts` crate embeds Atkinson Hyperlegible Next and Mono and OpenDyslexic (SIL OFL 1.1, `third_party/fonts/`, 1.35 MB) behind the cargo feature `bundled-fonts`, on by default here. PDF text defaults to Atkinson Hyperlegible Next and code to Atkinson Hyperlegible Mono, so PDFs look the same everywhere and never fail for lack of a font. `PdfOptions::font_family` and `code_font_family` choose a bundled or installed family by name (installed fonts are found by scanning the font folders for `name`-table family names), or a font file; a name that is neither is an error, reported once before a batch through `pdf::check_fonts`. Installed fonts still serve as per-character fallbacks for other scripts.
- **Layout.** `PageSize::parse` (letter, a4, a5, legal, `6x9in`) and `parse_length` (`1in`, `20mm`); margin and line spacing as before; `large_print` (18 points or more, 1.5 or more line spacing, more paragraph space, heading sizes 1.5 to 1 times the text, code at full size), and `PdfOptions::large_print()` as a preset (with 1.6 spacing and three-quarter-inch margins).
- **Structure.** Page numbers stay footer artifacts and can be turned off. `title_page` adds a title, author, and date page (the date only when given or in the document's front matter: the writer does not know the reader's time zone, and a wrong date is worse than none). `toc` adds a "Contents" heading and a `TOC` of `TOCI` entries, each a `Link` with a link annotation to its heading's destination and the heading's page number, to `toc_depth` levels.
- **Links.** Links to `#heading` anchors (slugs as Markdown renderers make them, or the heading text) and footnote references now jump to their target (XYZ destinations); a footnote body's own label is not a link. Anchors that point nowhere become text and are reported.
- **Reports.** Images without a description are artifacts, as before, and are now reported so the author can add alt text.
- **EPUB.** `EpubOptions::font` and `code_font` embed a bundled family (four files and its `OFL.txt`, `font/ttf` or `font/otf` in the manifest) with `@font-face` rules. Installed fonts are never embedded: their licences may not allow it.
- `tw convert` gains `--font`, `--code-font`, `--font-size`, `--page-size`, `--margin`, `--line-spacing`, `--large-print`, `--no-page-numbers`, `--title-page`, `--date`, `--contents`, `--contents-depth`, and `--lang` (`crates/textweaver-cli/src/cmd/convert_layout.rs`).

Every PDF in the new tests passes krilla's PDF/UA-1 validator.

## Consequences

- Agent L's converter calls `writer_for(format).write(...)` or `write_to_vec`; `WriteOptions` deserializes from the converter's settings with defaults for anything missing.
- The converter uses `Writer` itself (Agent V removed its separate `DocumentWriter`), and calls `pdf::check_fonts(&WriteOptions)` once before a PDF batch so a missing font is one message, not one failure per file.
- PDF output needs a font on the system. Bundling Atkinson Hyperlegible (SIL Open Font License) in the repository would make PDF output identical everywhere; that is a request to the orchestrator.
- PDF output no longer needs a font on the system (see the amendment). Built without `bundled-fonts`, it falls back to the installed families listed above.
- The HTML loader skips `aside` (Star's rule for web pages), so a future EPUB loader should read `aside epub:type="footnote"` as footnote bodies to round-trip textweaver's own EPUBs.
- Grade 2 braille depends on liblouis until a native contraction table is written and tested against liblouis's UEB test corpus.
- Not yet: BANA table formats beyond linear rows, typeform (bold, italic) braille indicators, the capitals passage indicator, SVG images in DOCX and PDF, MathML, real Word footnotes, and page labels in PDF from print page breaks.

## See also

- [Converting documents](../converting.md): choosing formats, fonts, and PDF layout.
- [ADR-0016: Rendering and bulk conversion](0016-rendering-and-conversion.md): the converter that calls these writers.
- [Documentation index](../README.md)
