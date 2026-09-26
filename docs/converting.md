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
- `brf`: a braille file for a braille display or embosser, in uncontracted (grade 1) Unified English Braille, 40 cells by 25 lines. Contracted (grade 2) braille needs a textweaver built with the `liblouis` feature and liblouis installed; without them, the file is grade 1 and the summary says so.
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
- `pandoc`: Pandoc's Markdown. Definition lists, fenced divs (`::: warning`), spans with classes (`[text]{.smallcaps}`), heading ids (`# Title {#intro}`), citations (`[@doe99, p. 33]`, shown as links to `#ref-doe99`), `H~2~O` and `2^10^`, and a title block (`% Title`, `% Author`, `% Date`). [Citations](citations.md) explains how to keep the references those citations point to.
- `commonmark`: plain CommonMark, with no extensions.

In every flavor except `commonmark`, a YAML block at the top of the file (between two lines of three dashes) sets the page's title, language, author, date, and description.

## Math

Math written in LaTeX between dollar signs, `$x^2$` inside a sentence or `$$ … $$` on its own lines, becomes MathML. Screen readers that read math (NVDA with MathCAT, JAWS, VoiceOver) can then speak it and let you explore it term by term. The LaTeX source travels with it, as the formula's text alternative and as an annotation, so copying still gives you the LaTeX. A formula that cannot be read is shown as its LaTeX source. Add `--no-math` to keep all math as LaTeX. Prices such as "$5 and $10" are not taken for math. The Markdown engine decides what is math, though, so a few forms, such as "$5-$10" with no spaces, can still become a formula; write `\$` for a dollar sign that must stay one.

ASCIIMath works too. A code block that starts with three backticks and the word `asciimath` (or `am`) becomes one formula. Course material written for MathJax often puts ASCIIMath between single backticks, like `` `x^2/2` ``; add `--asciimath` to read those as math instead of code.

EPUB, Word, braille, and PDF output keep math as its LaTeX source for now. [Math](math.md) explains how math is read aloud and how to write it.

## Templates

HTML output is a complete web page made from a template. Three are built in; choose one with `--template`:

- `default`: an accessible page. It declares its language, starts with a "Skip to content" link, puts the document in a main landmark, adds a table of contents when there are two or more headings, and uses a readable style that follows your system's dark mode, high contrast, and reduced-motion settings.
- `print`: for printing or saving as PDF from a browser.
- `fragment`: only the converted HTML, for pasting into another page.

Use `--no-toc` to leave out the table of contents.

You can write your own templates. They are HTML files with MiniJinja placeholders such as `{{ title }}` and `{{ content }}`. Put them in a folder and pass `--templates FOLDER --template NAME`, where the name is the file name without its extension, or give a file directly with `--template my-page.html`. Your template can start with `{% extends "default" %}` to reuse the built-in page. The placeholders are listed in the rustdoc of `textweaver_render::template`.

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

For EPUB, `--font` and `--code-font` put one of textweaver's own fonts into the book, with its licence, and make it the book's font. Reading apps may still let you choose another. Fonts installed on your computer cannot be put into a book, because their licences may not allow it.

## Other options

- `--engine comrak` uses the comrak Markdown parser instead of the default, pulldown-cmark. comrak implements every detail of GitHub's specification; pulldown-cmark is faster. Both give the same page structure.
- `--jobs 4` limits the work to four processor cores. By default every core is used.
- `--sanitize` removes scripts and other unsafe HTML, for Markdown you did not write yourself.
- `--smart` turns straight quotes into curly quotes and double hyphens into dashes.
- `--no-pandoc` never runs Pandoc (see below).
- `--pdf-font FILE` chooses a font file for the text of PDF output. `--font` does the same by name.
- `--verbose` reads out every file, not only the failures and the summary.
- `--json` prints the full result, every file with its status and timing, as JSON for scripts.

## Formats textweaver reads

textweaver reads Markdown, HTML, plain text, EPUB, Word (DOCX), and PDF itself. For other formats, such as OpenDocument text, RTF, reStructuredText, Org, and LaTeX, `tw convert` asks Pandoc when Pandoc is installed. Pandoc is never used for a format textweaver reads itself. `--no-pandoc` turns it off.

The reader, `textweaver`, does not use Pandoc. To read an OpenDocument or RTF file aloud, convert it to Markdown first, then open the Markdown:

```bash
tw convert essay.odt --to md
```

```bash
textweaver essay.md
```

PDF files are read with column-aware reading order: running headers and page numbers are left out, and headings, lists, and tables are recovered. A scanned PDF with no text layer cannot be read; textweaver has no OCR yet. [ADR-0010](adr/0010-pdf-loader.md) explains how the PDF reader works.

## When something fails

A file that cannot be converted never stops the others. After the summary, each failure is read out with its reason, for example "Failed: old.rtf: no native reader for .rtf files, and Pandoc is not installed". When any file fails, `tw convert` ends with exit status 1, so scripts can tell.

Two cases are refused before converting:

- An output that would replace its own source, such as Markdown to Markdown in the same folder. Choose an output folder with `--out`.
- Two sources that would write the same output, such as `notes.md` and `notes.txt` both becoming `notes.html`. Rename one of them.

## See also

- [Math](math.md): how LaTeX and ASCIIMath are read aloud and turned into MathML.
- [Citations](citations.md): the reference library behind Pandoc citations.
- [Themes](themes.md): the colours HTML output uses.
- [Audio export](audio-export.md): turning a document into an audiobook instead.
- [scripts/README.md](../scripts/README.md#convert-foldersh-and-convert-folderps1): the convert-folder helper scripts.
- [ADR-0016: Rendering and bulk conversion](adr/0016-rendering-and-conversion.md): how conversion works and how fast it is.
- [ADR-0017: Native writers](adr/0017-writers.md): EPUB, Word, braille, and PDF output, and their accessibility checks.
- [ADR-0010: PDF loader](adr/0010-pdf-loader.md): how PDFs are read.
- [Documentation index](README.md)
