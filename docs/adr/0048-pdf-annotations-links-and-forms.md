# ADR-0048: PDF annotations, links and forms

- Status: accepted; the owner's check 6 is still queued.
- Date: 2026-09-30 (Wednesday, September 30, 2026)

## Context

The PDF loader ([ADR-0010](0010-pdf-loader.md)) reads what is drawn on a page: text, tables, lists, and the structure tags of a tagged PDF. Three things students meet in PDFs are not drawn on the page but kept beside it, as annotations placed by rectangle:

- **Comments.** A lecturer marks up an essay in a PDF viewer: sticky notes, highlights with a comment typed in, struck-out sentences, replies, and a "Completed" review state. The Word and OpenDocument loaders read comments as notes ([ADR-0031](0031-native-rtf-odt-and-word-revisions.md)); a PDF's comments were silently dropped.
- **Links.** A PDF's links are annotations too. A reader heard "see page 3" with no way to follow it, and a web address only when it was also printed.
- **Form fields.** A filled form keeps its answers in the form's field dictionaries, not in the page's text, so a reader heard every question and no answer.

Two gaps from scanned pages remained from [ADR-0026](0026-ocr-and-student-formats.md): a page scanned sideways or upside down read as noise, and a scanned table read as a run of words, because the table finder wanted drawn text's exact positions and bold headers. And captions ("Figure 3.", "Table 2:") were read as ordinary paragraphs, or as headings when set large or bold.

## Decision

### Links

A `/Link` annotation becomes a `Link` marker over the words under its rectangle, found in the canonical text (below, "Finding the words"). Its reference is:

- a web or mail address (`http`, `https`, `mailto`, `ftp`; `www.` gets `https://`), as it is, so following it says the address and offers to open it, as with any other document;
- for a place in this document, the heading that starts at the destination, as `#methods`, the same form Markdown uses, when that heading's name is unique; else the page, as `#page=12` by its printed page label, the form PDF viewers accept;
- for another file, the file's name, without folders that climb out (`..`) or a drive.

Launch, JavaScript, and other actions that run something or change the viewer are dropped, as are `javascript:`, `data:`, and `file:` addresses. The app follows `#page=` anchors through one hook in `links.rs` (`pdf::page_anchor`), said with "Page" (`links-page-label`) where a heading would say "Heading".

### Comments as notes

Sticky notes, typed comments, highlights, underlines, squiggles, strike-outs, and drawn marks become `DocumentComment`s, recorded as the Word loader records them, so the reader turns them into notes tagged `comment` with no change to the app: "Note: Comment by Ada Example: Cite a source for this. Reply by Bo Example: Added a citation. Resolved."

- A mark on text (a highlight's quadrilaterals, a box or circle around words) is anchored on the words it covers. A note beside the text (a sticky note in the margin) is anchored at the start of the nearest line; failing both, at its page's start.
- Replies (`/IRT`) go under the thread's first comment, in order; a grouped annotation (`/RT /Group`) is part of its parent, not a reply. The latest review state decides "Resolved" (Completed or Accepted).
- A highlight, underline, or strike-out with nothing typed and no replies is kept with a word for what it does ("Highlighted", "Struck out"), because a lecturer's marking is itself a comment. Other empty annotations are dropped.
- Hidden annotations (flag 2) are not read; a review state, hidden by convention, still counts. Dates are written as ISO 8601; rich text (`/RC`) is read as plain text.

### Form fields

Each AcroForm field becomes one line where the field is on the page, its label first, then its value, in words:

- "Name: Ada Example", "Student ID, required: empty", "I agree to the terms: checked", "Payment: Credit card", "Course: Biology 101", "Signature: not signed". A password field says "hidden".
- The label is, in order of trust: the field's accessible name (`/TU`, what a screen reader says in a PDF viewer); the words printed just before the field on its line (just after a check box), or on a short line just above it; last, the field's internal name with its underscores as spaces.
- Printed words used as the label are taken out of the page's text, and so are underscores or dots drawn as a line to write on, so nothing is heard twice.
- A radio group is one field: the group's name, then the chosen option, named by the words beside its button. Each option's own words stay in the text.
- Push buttons ("Submit", "Print") do nothing in a reader and are left out. A form made only in XFA, which is not PDF form data, is named in a warning rather than silently skipped.

The line is a paragraph, never a heading, and is never taken for a running header.

### Finding the words

Annotations and fields are placed by rectangle, the reader needs characters. The glyphs under a rectangle are collected on the laid-out page, in its own space (so a scan that was turned upright still lines up), and their text is looked for in that page's part of the canonical text, then in the pages around it. Both sides are compared in lowercase without spaces or hyphens, so line joins, removed hyphens, and list labels do not stop a match. The search is linear (Knuth, Morris, and Pratt) and has a budget per document; past it, annotations fall back to their page's start. A link stays inside one line and leaves out a sentence's full stop.

### Rotated scans

Before a scanned page is recognized, `textweaver_ocr::orient` measures its ink, with no engine and no model: the row and column profiles say which way the lines run, and the ink above and below each line's middle band, with the left margin, says which way is up. The page is turned upright, or, when only the axis is clear, read both ways and the reading with more real words kept. The PDF loader turns the page's coordinates with it, and cached pages remember the turn.

### Tables on scans, and captions

On a recognized page, rows and columns are matched more loosely than drawn text (word boxes wander by a few points, and a tilted scan moves a row's baseline), rows may be further apart, and a first row of words over rows of numbers is the header, since a scan has no bold. The result is the same `Table` marker drawn tables get, so rows and columns are navigable in the same way.

A caption is found by pattern: a caption word (Figure, Fig., Table, Chart, Diagram, Map, Plate, and a few more), written as a word starts, a number (`3`, `2.1`, `A.4`, `S1`, `IV`), then punctuation ("Figure 3.", "Table 2:", "Fig. 4 -") or a capitalized title on a short paragraph ("Table 2 Results by year"). "Figure 3 shows the cycle" is a sentence, not a caption. A tagged PDF's `Caption` counts too.

- A table caption becomes its table's label, the table's name when moving by table.
- A figure caption reads as a graphic, as the LaTeX loader reads one, because an untagged picture has no other sign; beside a tagged picture, which is already the graphic, it stays a paragraph.
- A caption set large or bold is no longer taken for a heading.

### Limits

Hostile files are bounded: at most 20,000 annotations and 10,000 fields; field trees 32 levels deep, each object once; name trees 32 deep and 10,000 nodes; reply chains 64 long; destinations 8 names deep; quadrilaterals and arrays 4,096 numbers; labels and values 300 characters; comments cut as Word comments are. Past the annotation limit a warning says only the first twenty thousand are read. The fuzz target `pdf_annots` builds one-page PDFs around fuzzed annotation arrays, forms, and name trees.

## Consequences

- A PDF with comments reads like a Word document with comments: each is a note at its place, and the notes list has them all.
- A filled form is heard as questions with answers, in page order, label first.
- Links inside a PDF can be followed; outside addresses are said and offered, never opened unasked.
- Sideways and upside-down scans read in order. Scanned tables are navigable by row and column when their rows and columns line up; a table whose columns wander more than a word's height, or a page that is not upright after turning, still reads as paragraphs.
- Captions are found by pattern in English only. Other languages' caption words ("Abbildung", "Tableau") are not yet listed.
- The field words ("checked", "empty", "not signed") and markup words ("Highlighted") are English, set by the loader like the other loader words, not taken from the message catalogs.
- A figure caption now reads as a graphic, which changed one snapshot (the two-column page's caption).

## See also

- [ADR-0010: PDF loader](0010-pdf-loader.md)
- [ADR-0026: OCR, and formats for students](0026-ocr-and-student-formats.md)
- [ADR-0031: Native RTF, ODT, and Word revisions](0031-native-rtf-odt-and-word-revisions.md), for comments as notes
- [Converting guide: PDF files](../converting.md#pdf-files)
