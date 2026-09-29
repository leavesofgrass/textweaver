# Converting documents

`tw convert` turns documents into other formats: one file, a whole folder of files, or every file that arrives in a folder you are watching. It uses every processor core, and it skips files that are already converted, so running it again on a large folder takes a moment.

This guide is written to be read with a screen reader. Each section starts with the command, then explains it.

## Convert one file

```bash
tw convert notes.md --to html
```

This writes `notes.html` next to `notes.md`. When it finishes, you hear one sentence, for example: "Converted 1 file to HTML in 32 milliseconds, 31.6 files per second. No failures."

The formats you can ask for with `--to`:

- `md`: Markdown (the default).
- `html`: a web page you can open in any browser. Headings, lists, tables, and math are real HTML structure, so screen readers can move by heading, table, and list.
- `txt`: plain text, the same text textweaver reads aloud.
- `epub`: an EPUB 3 book with a table of contents, real headings, and accessibility metadata.
- `docx`: a Word document with Word's own heading styles, numbered lists, and tables whose header row repeats.
- `brf`: a braille file for a braille display or embosser, in uncontracted (grade 1) Unified English Braille, 40 cells by 25 lines. Contracted (grade 2) braille needs a textweaver built with the `liblouis` feature and liblouis installed; without them, the file is grade 1 and the summary says so. Math is in Nemeth braille, or UEB mathematics with `--math-code ueb`, in a build with MathCAT (see [Math in braille files](math.md#math-in-braille-files)).
- `pdf`: a tagged PDF that screen readers can move through by heading, list, and table.

Some outputs come with warnings, for example "The image cat.png was not found, so its description was written instead." Each warning is read out with its file name, and the summary says how many files had warnings.

PDF output uses fonts that come with textweaver, so it works on every computer with nothing installed. [PDF and EPUB layout](#pdf-and-epub-layout) explains how to choose another font.

## Convert a folder

```bash
tw convert "Biology notes" --to html --out "Biology site"
```

Every document in the folder and in all of its subfolders is converted. The output folder mirrors the input: `Biology notes/week 1/cells.md` becomes `Biology site/week 1/cells.html`. Hidden files and folders, whose names start with a dot, are left alone.

Without `--out`, each output is written next to its source. In that case files that already have the output's extension are treated as earlier outputs, not as sources, so converting a folder to HTML twice does not convert the HTML files it made the first time.

You can name several files and folders in one command.

## Only what changed is converted again

When you run the same command again, a file is skipped when its output is newer than the source. Edit one note, run the command, and only that note is converted. The summary tells you how many files were skipped.

To convert everything anyway, for example after changing the template, add `--force`:

```bash
tw convert "Biology notes" --to html --out "Biology site" --force
```

Outputs are written to a temporary file first and then renamed, so stopping a conversion part way never leaves a half-written file behind.

## Watch a folder

```bash
tw convert Inbox --to txt --watch
```

This is a hot folder. Files already in `Inbox` are converted first, then each new file as it arrives. Converted files go to `Inbox/converted` (or to the folder you give with `--out`). Each event is spoken as one line, for example "Converted chapter 3.docx." Press Control C to stop.

It follows Star's hot-folder rules:

- A file is converted only after its size has stayed the same for 2 seconds, so a file that is still being copied is never read half-way. Change the wait with `--stable-seconds 5`.
- After a successful conversion the source moves to `Inbox/processed`. Add `--keep-sources` to leave it where it is.
- A file that fails moves to `Inbox/failed`, so it is not tried again and again. A name that is already taken in either folder gets the date and time added; nothing is overwritten.
- Files textweaver cannot read, such as pictures, stay where they are, and you hear "Ignored" once.
- Everything that happens is also written, with the time, to `textweaver-watch.log` in the output folder.

Only the folder itself is watched, not its subfolders.

## Markdown flavors

Markdown comes in dialects. Choose yours with `--flavor`:

- `gfm`, the default: GitHub Flavored Markdown. Tables, task lists, strikethrough, web addresses that become links by themselves, footnotes, and alerts such as `> [!NOTE]`.
- `obsidian`: everything in GFM, plus Obsidian's own syntax. `[[Note]]` links to `Note.html`, `[[Note#Heading]]` links to that heading, and `[[Note|text]]` shows your own link text. `![[picture.png|description]]` shows a picture with that description as its alternative text. `![[Note]]` is a link to the note, or, with `--embeds inline`, the note's text itself. Callouts such as `> [!tip] Remember` become labelled notes; a callout with a minus sign after the type becomes a section you can expand and collapse. `#tags`, `==highlights==`, and block references (`^id`) work too.
- `pandoc`: Pandoc's Markdown. Definition lists, fenced divs (`::: warning`), spans with classes (`[text]{.smallcaps}`), heading ids (`# Title {#intro}`), citations such as `[@doe99, p. 33]` (see [Citations](#citations) below), `H~2~O` and `2^10^`, and a title block (`% Title`, `% Author`, `% Date`).
- `commonmark`: plain CommonMark, with no extensions.

In every flavor except `commonmark`, a YAML block at the top of the file (between two lines of three dashes) sets the page's title, language, author, date, and description.

## Math

Math written in LaTeX between dollar signs, `$x^2$` inside a sentence or `$$ … $$` on its own lines, becomes MathML. Screen readers that read math (NVDA with MathCAT, JAWS, VoiceOver) can then speak it and let you explore it term by term. The LaTeX source travels with it, as the formula's text alternative and as an annotation, so copying still gives you the LaTeX. A formula that cannot be read is shown as its LaTeX source. Add `--no-math` to keep all math as LaTeX. Prices such as "$5 and $10" are not taken for math. The Markdown engine decides what is math, though, so a few forms, such as "$5-$10" with no spaces, can still become a formula; write `\$` for a dollar sign that must stay one.

ASCIIMath works too. A code block that starts with three backticks and the word `asciimath` (or `am`) becomes one formula. Course material written for MathJax often puts ASCIIMath between single backticks, like `` `x^2/2` ``; add `--asciimath` to read those as math instead of code.

The other outputs never print the dollar signs and LaTeX commands either:

- EPUB: MathML, as in a web page. Reading apps draw it, and screen readers read it and let you explore it.
- Word: Word's own equations. Word draws them, and Narrator, NVDA, and JAWS can read them.
- PDF: the formula in print form, such as πr² or (a + b)/2, marked as a formula whose description is how it is read aloud, for example "pi r squared".
- Braille: math braille, Nemeth by default or UEB mathematics with `--math-code ueb`, in a build with MathCAT; otherwise the formula as it is read aloud, "pi r squared". See [Math in braille files](math.md#math-in-braille-files).

[Math](math.md) explains how math is read aloud and how to write it.

## Citations

```bash
tw convert essay.md --to docx
```

Citations written the way Pandoc writes them, such as `[@doe2020, p. 12]` or `@doe2020 says`, are formatted in a citation style, and a References section listing the works you cited is added at the end. This works for web pages, Word, PDF, EPUB, braille, and plain text. Markdown output keeps your citations as you wrote them. You do not need Pandoc.

A real example, converted to plain text in the default style, APA:

```text
The area of a circle is $\pi r^2$ (see Doe, 2020, p. 12). As Müller (2019, p. 40) argues, reading aloud helps (2020).

References

Doe, J. (2020). Reading by ear. Example Press.

Müller, A. (2019). Speech and study. Journal of Listening, 4, 33–50.
```

The source was `[see @doe2020, p. 12]`, `@muller2019 [p. 40]`, and `[-@doe2020]`. (Plain text keeps math as LaTeX; the other outputs typeset it.)

- `--style NAME` chooses the style: `apa` (the default), `mla`, `chicago`, `chicago-notes`, `harvard`, `ieee`, `vancouver`, `ama`, `nature`, any other name `tw cite styles` lists, or a `.csl` file. A note style such as `chicago-notes` puts each citation in a footnote.
- `--bibliography FILE` names a file of references to look in first. It can be CSL-JSON, BibTeX, BibLaTeX, or RIS. A `bibliography:` line in the document's front matter does the same, with the path relative to the document.
- After that, keys are looked up in `references.json` in the document's folder, then in your own library, the one `tw cite` keeps.
- `--no-citations` leaves citations exactly as written.

In a web page, each citation is a link to its entry in the References section.

A citation key that is in none of the libraries is written as "missing reference" and the key, and you hear a warning such as "The citation key smith1999 is not in any library, so it reads as missing reference smith1999." Citations inside code and math are never touched. With the `gfm` and `obsidian` flavors, where `@name` is more often a mention of a person than a citation, `@name` is formatted only when `name` is a key in one of the libraries, and a bracketed citation only when at least one of its keys is; with `--flavor pandoc` every citation is formatted, as Pandoc does. [Citations](citations.md) explains how to build the library.

## Templates

HTML output is a complete web page made from a template. Three are built in; choose one with `--template`:

- `default`: an accessible page. It declares its language, starts with a "Skip to content" link, puts the document in a main landmark, adds a table of contents when there are two or more headings, and uses a readable style that follows your system's dark mode, high contrast, and reduced-motion settings.
- `print`: for printing or saving as PDF from a browser.
- `fragment`: only the converted HTML, for pasting into another page.

Use `--no-toc` to leave out the table of contents.

You can write your own templates. They are HTML files with MiniJinja placeholders such as `{{ title }}` and `{{ content }}`. Put them in a folder and pass `--templates FOLDER --template NAME`, where the name is the file name without its extension, or give a file directly with `--template my-page.html`. Your template can start with `{% extends "default" %}` to reuse the built-in page. The placeholders are listed in the rustdoc of `textweaver_render::template`.

## Publishing templates

```bash
tw convert essay.md --to docx --template apa
```

A publishing template gives an EPUB book, a Word document, or a PDF the look a kind of document needs. Choose one with `--template`:

- `apa`: an APA 7 student paper. A title page, double spacing, paragraphs with a first-line indent, APA's heading levels, the page number at the top right of every page, and references with hanging indents.
- `ama`: an AMA 11 manuscript. A title page with the word count, double spacing, and page numbers.
- `large-print`: 18-point sans-serif text, generous spacing, and bold rather than italic for emphasis.
- `dyslexia-friendly`: sans-serif text a little larger than usual, wider spacing between letters and lines, bold rather than italic or underline, and a cream page color in Word and EPUB.
- `high-contrast`: in EPUB, white text on black with yellow, underlined links; in Word and PDF, heavy black text on white, because a page color does not print.
- `manuscript`: standard manuscript format. A title page with a rounded word count, double spacing, a running head with your surname, the short title, and the page number, and a centered number sign for each scene break.

A template changes how the document looks, never how it is read. Headings stay real headings at their levels, lists stay lists, tables keep their header rows, and images keep their descriptions, so a screen reader moves through a templated paper exactly as through any other. The layout options in the next section still apply on top of a template: `--template apa --line-spacing 1.5` keeps the APA look with one-and-a-half spacing.

For HTML, `--template` still names a page template (see [Templates](#templates)); a publishing template name there uses the default page and says so. Braille keeps its own layout.

### The title page

The APA, AMA, and manuscript templates start with a title page made from the document's front matter:

```yaml
---
title: Listening as a Study Skill
author: Ada Example
affiliation: Department of Education, Example State University
course: EDU 501, Learning and Assistive Technology
instructor: Dr. Grace Placeholder
date: October 5, 2026
---
```

- APA uses the title, author, affiliation, course, instructor, and date, in that order, each on its own line.
- AMA uses the title, author, and affiliation, then `corresponding:` (the corresponding author) if you give one, and the word count of the running text. The count leaves out headings, tables, footnotes, and the abstract and reference sections.
- Manuscript uses the title, "by" and the author, and the word count to the nearest hundred.

`institution` or `university` can stand for `affiliation`, `professor` for `instructor`, and `due-date` for `date`. A field you leave out is left off the page. textweaver never guesses the date; `--date "October 5, 2026"` gives one on the command line.

When a paper's only level 1 heading is its title, as in a Markdown file that starts with `# Title`, APA's first heading look goes to the level 2 headings, so your sections look like APA level 1 headings while a screen reader still hears the title as level 1 and each section as level 2.

### Footnotes in Word

Footnotes in a Word document are real Word footnotes, whatever the template. Word numbers them and prints them at the foot of the page; JAWS and NVDA announce the reference as a footnote and can read its text, and Word's footnote pane lists them all. A footnote referenced twice becomes one footnote, with a cross-reference to its number at the second place. A footnote nothing refers to stays where it is in the text. textweaver reads its own Word documents back with the footnotes in place.

### The EPUB cover

Every template adds a cover to an EPUB book: an image with the title and author in the template's colors. Its description is "Cover: " followed by the title and author, the book lists it as the cover in its landmarks, and it is the first thing in the reading order. The book's accessibility metadata says it has an image with a text description.

### Print page numbers in PDF

When a document has print page numbers, as a DAISY book, an EPUB with a page list, or a scanned PDF does, each page of the PDF is labelled with the print page its first line belongs to, the way a printed book numbers its pages. A PDF reader's "go to page" then takes the print page number and opens the page where that print page is under way, and "Page 3 of 20, print page 42" appears at the foot of the page. A title page and table of contents before the first print page are numbered i, ii, and so on.

## PDF and EPUB layout

```bash
tw convert "Chapter 3.md" --to pdf --large-print --title-page --contents
```

PDF output uses fonts that come with textweaver, so a PDF looks the same on every computer and never fails for lack of a font. The text font is chosen in this order: `--pdf-font` (a font file), then `--font` (a name or a file), then the `TEXTWEAVER_PDF_FONT` environment variable (a font file), then the bundled Atkinson Hyperlegible Next. The text is in Atkinson Hyperlegible Next and code is in Atkinson Hyperlegible Mono, both from the Braille Institute and made so that letters that look alike are easy to tell apart.

These options change how a PDF looks. None of them changes what a screen reader hears: every PDF is tagged, has a title and a language, and passes the PDF/UA check textweaver runs while writing.

- `--font NAME`: the text font. The fonts that come with textweaver are "Atkinson Hyperlegible Next", "Atkinson Hyperlegible Mono", and "OpenDyslexic". You can also name any font installed on your computer, such as "Verdana", or give a font file. If the name is not found, textweaver says so and converts nothing, so a typing mistake is never silently replaced.
- `--code-font NAME`: the font for code, in the same way.
- `--font-size 14`: the text size in points. The default is 12.
- `--large-print`: text of 18 points or more, more space between lines and paragraphs, headings that are not much larger than the text, and three-quarter-inch margins so more words fit on a line. The other options still apply on top, for example `--large-print --font-size 20`.
- `--page-size a4`: `letter` (the default), `a4`, `a5`, `legal`, or your own size, such as `6x9in` or `148x210mm`.
- `--margin 20mm`: the margin on every side. You can write it in `in`, `mm`, `cm`, or `pt`. The default is one inch.
- `--line-spacing 1.8`: the space from one line to the next, as a multiple of the text size. The default is 1.5.
- `--no-page-numbers`: leave out "Page 3 of 12" at the bottom of each page. Page numbers are marked so screen readers skip them.
- `--title-page`: start with a page holding the title and the author. Add `--date "September 2026"` to print a date as well; textweaver never guesses today's date.
- `--contents`: a table of contents after the title page. Each entry is a link to its heading and shows its page number. `--contents-depth 2` lists only headings of levels 1 and 2 (the default is 3).
- `--lang fr`: the document language, for the document and its screen reader voice, when the source does not say or says it wrongly.

Links in a PDF work: web and email links open, and links to a heading in the same document, such as `[see the summary](#summary)` in Markdown, jump to that heading. Footnote numbers jump to their footnote. textweaver tells you about a link to a heading that does not exist, and about any image that has no description, since screen readers skip such images.

Struck-through text, such as `~~old plan~~` in Markdown, is drawn with a line through it. Screen readers still read the words.

For EPUB, `--font` and `--code-font` put one of textweaver's own fonts into the book, with its licence, and make it the book's font. Reading apps may still let you choose another. Fonts installed on your computer cannot be put into a book, because their licences may not allow it.

## Other options

- `--engine comrak` uses the comrak Markdown parser instead of the default, pulldown-cmark. comrak implements every detail of GitHub's specification; pulldown-cmark is faster. Both give the same page structure.
- `--jobs 4` limits the work to four processor cores. By default every core is used.
- `--sanitize` removes scripts and other unsafe HTML, for Markdown you did not write yourself.
- `--smart` turns straight quotes into curly quotes and double hyphens into dashes.
- `--no-pandoc` never runs Pandoc (see below).
- `--pandoc-timeout 300` gives Pandoc up to 300 seconds for each file instead of 120 (see below).
- `--pdf-font FILE` chooses a font file for the text of PDF output. `--font` does the same by name.
- `--verbose` reads out every file, not only the failures and the summary.
- `--json` prints the full result, every file with its status and timing, as JSON for scripts.

## Formats textweaver reads

textweaver reads these formats itself:

- Markdown, HTML, plain text, EPUB, Word (DOCX), and PDF.
- RTF (Rich Text Format): headings, lists, tables, footnotes, links, pictures' descriptions, and text in older code pages such as Cyrillic and Japanese.
- OpenDocument text (ODT, OTT, and flat FODT), as LibreOffice and Google Docs save it: headings, numbered and bulleted lists, tables, footnotes, links, and pictures' descriptions.
- Scanned PDFs and pictures (PNG, JPEG), by recognizing their text. See [Scanned pages](#scanned-pages-ocr).
- DAISY 3 books and DTBook files, including Bookshare zips.
- PowerPoint (PPTX): slides in order, each with its speaker notes.
- Spreadsheets: CSV, TSV, OpenDocument (ODS), and Excel (XLSX, XLSM, XLSB), as tables. Old binary Excel files (XLS) are not read.
- Archives (ZIP, TAR, TAR.GZ, and 7Z): opening one lists the files inside that textweaver can read, each a link. To open a file inside an archive directly, write its name after a `!`, as in `tw text course.zip!week1/notes.md`.
- Web pages: `tw open https://example.org/page` and `tw text https://...` fetch the page and read it. A PDF or other file at the address is saved in the cache and opened from there.

For other formats, such as reStructuredText, Org, and LaTeX, `tw convert` asks Pandoc when Pandoc is installed. Pandoc is never used for a format textweaver reads itself, and RTF and OpenDocument text no longer need it. `--no-pandoc` turns it off.

Equations in a Word document are read as math. textweaver turns them into LaTeX between dollar signs, as in Markdown, so they are spoken as formulas.

### Comments and tracked changes

Comments in Word and OpenDocument files, with their replies and whether they are resolved, come with the document. When you open it in the reader, each comment becomes a note on the text it is about, tagged `comment`, so reading tells you when you reach it ("Note: Comment by Ada Example: say how salty. Reply by Bo Example: added. Resolved."), and the notes list has them all. A comment you edit or keep stays as you left it when the document opens again.

Tracked changes in Word, OpenDocument, and RTF files are read as the final text by default: insertions as ordinary text, deletions left out. The reading setting `revisions` changes that:

- `auto`, the default, says each change where it is when announcements are set to high verbosity, and reads the final text otherwise.
- `marked` always says them: "The quiz is on (deleted by Bo Example: Tuesday) (inserted by Ada Example: Thursday)."
- `final` never does.

Set it in `settings.toml` under `[reading]`, for example `revisions = "marked"`. A document already open keeps the way it was read until you open it again. `tw convert` and `tw text` read the final text.

Pandoc runs in its sandbox, so a document cannot make it read other files on your computer (a LaTeX `\input`, for example); this needs Pandoc 2.19 or later. A file Pandoc takes more than two minutes on is stopped and counted as failed, with the reason "pandoc took longer than 2 minutes and was stopped", and the other files go on. Change the limit with `--pandoc-timeout SECONDS` or the `TEXTWEAVER_PANDOC_TIMEOUT` environment variable. To use a Pandoc that is not on your `PATH`, set `TEXTWEAVER_PANDOC` to its full path.

A damaged or deliberately odd file cannot stop a batch either. Content nested thousands of levels deep, in a web page, EPUB, Word, OpenDocument, or RTF document, is read as plain text below 256 levels, with the warning "Some content was nested too deeply to keep its structure, so it is read as plain text." List and page numbers that claim impossible values are capped. A file that is really a picture, a program, or another binary file is refused after its first 8 kilobytes, however large it is. Word, OpenDocument, EPUB, and PowerPoint files are zip packages; one with more than 50,000 files inside, with files that overlap, or with a file that claims to unpack to more than 1,000 times its size is refused, and no package is unpacked past 1 gigabyte.

The reader, `textweaver`, does not use Pandoc. It opens RTF and OpenDocument text itself. For a format only Pandoc reads, such as reStructuredText, convert it to Markdown first, then open the Markdown:

```bash
tw convert essay.rst --to md
```

```bash
textweaver essay.md
```

PDF files are read with column-aware reading order: running headers and page numbers are left out, and headings, lists, and tables are recovered. [ADR-0010](adr/0010-pdf-loader.md) explains how the PDF reader works.

### Scanned pages (OCR)

A scanned PDF has pictures of pages instead of text. textweaver recognizes the text in them (optical character recognition, OCR), and then reads the pages like any other PDF, with headings, paragraphs, and page numbers. Pictures (PNG and JPEG) are read the same way. Recognized text can contain mistakes, so textweaver says when a document was recognized.

- **English** is read inside textweaver, by the ocrs engine. Its models are a one-time download of 12.2 MB, under the CC BY-SA 4.0 licence. Run `tw ocr download`; it says what it will download and asks first. `tw ocr status` says what is ready.
- **Other languages** are read by Tesseract, a free program you install yourself, with the data for your language. Set the language with `ocr_lang` in the `[reading]` section of the settings, for example `ocr_lang = "fra"` for French, or `"deu+eng"` for German and English. When a PDF names its own language, that is used.
- `ocr_engine` chooses the engine: `auto` (the default), `ocrs`, `tesseract`, or `paddle` (an experimental in-process engine for accented Latin-script languages; download it with `tw ocr download paddle-latin`). `ocr = false` turns recognition off.
- `tw ocr read scan.pdf` recognizes a file and prints its text, with progress. Press Control C to stop it.
- Recognized pages are remembered, so a book opens instantly the second time.
- In the reader, a scanned book takes about a second a page to open the first time. Every three seconds you hear which page it is on, such as "Still opening scan.pdf: recognizing text on page 3 (3 of 40)." Escape stops it.

[ADR-0026](adr/0026-ocr-and-student-formats.md) explains the choices and gives measurements.

## Export from inside the reader

You can also convert the document you have open without leaving textweaver. Press **F2** for the command palette and type part of one of these names:

- `export html`: a web page.
- `export pdf`: a tagged PDF.
- `export docx`: a Word document.
- `export epub`: an EPUB book.
- `export brf`: braille.

The file goes next to the document, with the same name: exporting `essay.md` to PDF writes `essay.pdf` in the same folder, replacing an older export. In edit mode the text you are editing is exported, saved or not. Citations are formatted and a References section added, as with `tw convert`: from the bibliography your front matter names, the folder's `references.json`, and your own library.

You hear "Exporting to PDF." and can go on reading or writing while it works. If it takes more than two seconds you hear "Still exporting to PDF, 2 seconds.", and then again every ten seconds, never more often. When it is done you hear where the file went and a question, for example "Exported to PDF: essay.pdf in C:\Users\jon\Essays. Open it? y or n." Press **y** to open it with your computer's program for that kind of file, or **n** to leave it. A warning, such as an image that was not found, is read out before the question.

A new document that was never saved has no folder yet; its export goes to the folder textweaver was started in, like Save As suggests.

### Preview in the browser

Type `preview in browser` in the palette. textweaver writes the document as a web page, with math as MathML so screen readers can read it, and opens it in your default web browser. The page is kept in the `preview` folder of textweaver's cache folder, and images and links in it still point beside your document.

While you edit, each save (**Ctrl+S**) writes the preview again and you hear "Preview updated. Press F5 in the browser." The browser does not reload by itself, so it never moves your screen reader's place. To have it reload by itself, turn on automatic reloading; see [the editing guide](editing.md#preview-in-the-browser).

## When something fails

A file that cannot be converted never stops the others. After the summary, each failure is read out with its reason, for example "Failed: old.rst: no native reader for .rst files, and Pandoc is not installed". When any file fails, `tw convert` ends with exit status 1, so scripts can tell.

Two cases are refused before converting:

- An output that would replace its own source, such as Markdown to Markdown in the same folder. Choose an output folder with `--out`.
- Two sources that would write the same output, such as `notes.md` and `notes.txt` both becoming `notes.html`. Rename one of them.

## See also

- [Math](math.md): how LaTeX and ASCIIMath are read aloud and turned into MathML.
- [Citations](citations.md): the reference library behind Pandoc citations.
- [Writing and editing](editing.md): listening to the rendered text while you write.
- [Themes](themes.md): the colours HTML output uses.
- [Audio export](audio-export.md): turning a document into an audiobook instead.
- [scripts/README.md](../scripts/README.md#convert-foldersh-and-convert-folderps1): the convert-folder helper scripts.
- [ADR-0016: Rendering and bulk conversion](adr/0016-rendering-and-conversion.md): how conversion works and how fast it is.
- [ADR-0017: Native writers](adr/0017-writers.md): EPUB, Word, braille, and PDF output, and their accessibility checks.
- [ADR-0010: PDF loader](adr/0010-pdf-loader.md): how PDFs are read.
- [Documentation index](README.md)
