# ADR-0041: Publishing templates, real Word footnotes, and PDF page labels

- Status: accepted (Wave 5, Agent W5g, pulled forward from W6g). The owner's check with Word and JAWS is queued in session 3.
- Date: 2026-09-28
- Builds on: [ADR-0017](0017-writers.md) (the native writers), [ADR-0016](0016-rendering-and-conversion.md) (the converter and `tw convert`), and [ADR-0019](0019-citations.md) (the References section a template styles)

## Context

The owner put the APA and AMA publishing templates, with real Word footnotes, on the feature-complete list. Star shipped them in its publishing arc (0.1.30): four stylesheets for EPUB and HTML (large print, dyslexia-friendly, high contrast, academic manuscript) and Word reference documents (large print, dyslexia-friendly, APA student paper, AMA manuscript), all through Pandoc, with the Word reference documents generated at install time rather than kept as binary files. Two of Star's lessons carry over: Pandoc styled Word output by style name, so a template that did not define a style silently had no effect; and Star's DOCX still lacked page numbers and an APA title page.

textweaver writes EPUB, DOCX, and PDF natively (ADR-0017). ADR-0017 left three items open that this ADR closes: real Word footnotes, page labels in PDF from print page breaks, and templates beyond large-print PDF.

## Decision

### One `Template` for three formats

`textweaver_writers::Template` has six values: `apa` (APA 7 student paper), `ama` (AMA 11 manuscript), `large-print`, `dyslexia-friendly`, `high-contrast`, and `manuscript` (standard manuscript format). `WriteOptions::template` carries it to every writer. `Template::apply` (or `WriteOptions::with_template`) also sets the template's PDF layout and turns on the EPUB cover, as defaults: `tw convert` applies the template first and the layout options on top, so `--template apa --line-spacing 1.5` keeps 1.5.

A template changes the look only. Every writer keeps the structure it already had (Word heading styles with outline levels, EPUB `h1` to `h6`, tagged PDF headings, lists, table header rows, alternative text), so a screen reader navigates a templated document exactly as an untemplated one. Templates are tested for this: each one's Word output is read back by textweaver's DOCX loader with every heading present.

### Word

The looks are generated in code (`template.rs`, `DocxLook`), as Star generated its reference documents, so there is no binary template in the repository and every style a document uses is defined in its `styles.xml` (a test checks both the document and footnote parts).

- **APA student paper:** Times New Roman 12, double spacing, no space between paragraphs, a half-inch first-line indent (a `Body Text` style, so table cells, lists, and footnotes are not indented), the APA 7 heading levels (centered bold; flush left bold; flush left bold italic; indented bold; indented bold italic), the page number at the top right (a `PAGE` field in `header1.xml`), and a `Bibliography` style with a half-inch hanging indent for the paragraphs under a "References" heading. The title page is a `Title` paragraph and centered lines from the front matter (author, affiliation, course, instructor, date), and the text starts with `pageBreakBefore` rather than an empty page-break paragraph.
- **Heading levels in a paper:** when the only level 1 heading is the first block and says the title (a Markdown paper that starts `# Title`), the template's first heading look goes to level 2 and so on. The sections look like APA level 1 headings, while screen readers still hear the title as heading level 1 and each section as level 2. The outline levels never move.
- **AMA manuscript:** the paper settings with flush-left headings (bold, then bold italic, then italic), page numbers, and a title page with the corresponding author (`corresponding:`) and the word count of the running text (paragraphs, list items, and quotes; not headings, tables, code, figures, footnotes, the abstract, or the reference list).
- **Manuscript:** the paper settings, a running head ("Surname / Short title / page"), the title page with "by" the author and the word count to the nearest hundred, and rules written as a centered "#" (scene breaks).
- **Large print** (after ACB and APH): Verdana 18, 1.5 spacing, headings 18 to 24 points, black text, and italic and plain underline shown as bold.
- **Dyslexia-friendly** (after the British Dyslexia Association's style guide): Verdana 14, character spacing expanded by 0.6 point, 1.5 spacing, bold for emphasis, dark gray text, and a cream page color (`w:background` with `displayBackgroundShape`; Word does not print it).
- **High contrast:** Arial 14, black text, larger bold headings, and dark blue underlined links. A page color is deliberately not used: it does not print, and white text would then vanish.

### Real Word footnotes

Footnotes become real Word footnotes by default (`DocxOptions::word_footnotes`, on): `word/footnotes.xml` with the separator and continuation separator Word expects, `footnotePr` in the settings, the `FootnoteText` and `FootnoteReference` styles, and a `w:footnoteReference` at the reference, numbered by Word in order of first reference. Word shows them at the foot of the page, lists them in its footnote pane, and JAWS and NVDA announce the reference as a footnote. Details:

- A footnote referenced twice is one footnote. The first reference carries a hidden bookmark (`_RefNoteN`), and the second is a `NOTEREF _RefNoteN \f \h` field, so it shows the same number in footnote style and links to it.
- A footnote body nothing references stays in the text where it is, as before.
- A heading whose section holds only footnotes that became Word footnotes (the "Footnotes" heading the Markdown loader adds) is left out, since it would head an empty section.
- Links and images inside a footnote get their relationships from `word/_rels/footnotes.xml.rels`, since a footnote part cannot use the document's; each image file is stored once.
- References inside a footnote body or a link are not nested footnotes; they stay text.
- Turning the option off restores ADR-0017's behavior: every footnote in place, with its reference linked to a bookmark on it.

textweaver's DOCX loader already reads `footnotes.xml`, so its own Word files round-trip with every word and a footnote marker (tested).

### EPUB

- **Stylesheets:** each template's stylesheet (`crates/textweaver-writers/templates/*.css`) follows the book's own rules in `style.css`, so its rules win and reading systems can still override them. APA, AMA, and manuscript use the manuscript stylesheet. The high contrast one is white on black (21 to 1) with yellow, underlined links (19.6 to 1); the dyslexia-friendly one names OpenDyslexic first, which applies when the book embeds it (`--font OpenDyslexic`) and otherwise falls back to Atkinson Hyperlegible Next or Verdana.
- **Cover** (`EpubOptions::cover`, on with any template): `cover.svg`, made from `templates/cover.svg` with the title and author wrapped onto lines in the template's colors (every pair above 7 to 1), is the manifest's `cover-image`; `cover.xhtml` shows it as `<img role="doc-cover">` inside `<section epub:type="cover">`, with the alternative text "Cover: Title, by Author." It is first in the spine and in the landmarks. The SVG has the same text in its `title`, labelled with `aria-labelledby`. The package metadata then declares the visual access mode, textual as sufficient, and alternative text. The cover image is text, not a picture of text, so no information is lost for someone who cannot see it.

### PDF

- **Templates** set the layout: APA, AMA, and manuscript 12 points, double spacing, one-inch margins, and a title page with the template's fields (the same `TitlePage` Word uses); large print the large-print preset; dyslexia-friendly 14 points at 1.6 spacing; high contrast 14 points.
- **Page labels from print page breaks:** a `PageBreak` block waits for the next line of text, and the PDF page that line lands on is where that print page starts. Each PDF page is labelled with the print page its first line belongs to (the last one started above that line), as a printed book's running page number would be, so a viewer's "go to page 42" lands on the page where print page 42 is under way at the top. A first try labelled each page with the first print page starting on it; with print pages about one PDF page long, a print page starting at the foot of a page then had no label at all, which the test caught. The rule chosen loses a label only for a print page that starts and ends inside one PDF page, and one PDF page can carry only one label either way. A label that is a plain number is written as an arabic range with that start value, so viewers can count; any other label ("xii", "A-3") is written as a literal prefix. Pages before the first print page (a title page, the contents) are numbered i, ii, iii. A document with no print page breaks gets no labels, as before. The footer, an artifact, reads "Page 3 of 20, print page 42". textweaver's PDF loader reads the labels back (tested with six print pages over more PDF pages, and a title page labelled i).

### `tw convert --template`

The existing `--template` option takes the six names (and aliases such as `apa-student-paper`, `dyslexia`, `contrast`). For HTML it still names a page template; a publishing template name there uses the default page and prints one sentence saying so, as it does for braille and plain text. The reader's own export keeps its settings for now; a template setting there is left for the GUI's parity wave.

## Checks

- The writers' tests: `tests/templates.rs` (package consistency for every template, styles defined, heading outline levels, the APA and AMA title pages, the heading shift, body and bibliography styles, headers, footnotes with a repeated reference and an orphan, footnotes kept in place when asked, round trips through the DOCX and PDF loaders, the EPUB cover and stylesheets, page labels, and every template's PDF passing krilla's PDF/UA-1 validation), unit tests of the templates, the cover SVG, wrapping, and page labels, and the existing DOCX tests updated for real footnotes.
- The second-tool workflow converts `fixtures/g2/apa-paper.md` with each template to EPUB and PDF, for epubcheck and veraPDF.

## Consequences

- Word files from textweaver now use real footnotes. A document whose Word text must match its source character for character sets `word_footnotes` off.
- Templates are data in code and CSS files; a new template is one enum value, one `DocxLook`, one stylesheet, and its PDF layout.
- Not done: a user's own Word template (`--reference-doc` in Star), page numbers in untemplated Word files, APA professional papers (running head), the HTML page picking up a publishing stylesheet (the HTML templates belong to `textweaver-render`), and a template setting in the reader.

## See also

- [Converting documents: publishing templates](../converting.md#publishing-templates)
- [Citations](../citations.md): the References section the APA and AMA templates style.
- [Documentation index](../README.md)
