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

  Tables are laid out in one of three ways, chosen with `--table-format` or `[braille] table_format`. `linear` (the default) puts each row on one line with semicolons between entries. `listed` starts each row in cell 5 with the first column's heading and entry, and puts each other entry on its own line after its column heading and a colon. `stairstep` puts each entry two cells to the right of the one before, with the column headings in a transcriber's note. Stairstep takes four columns at most; a wider table is listed instead, with a warning. Listed and stairstep follow BANA's Braille Formats (2016), 11.16 and 11.18. A transcriber's note explains the layout, an empty entry is three guide dots, and a row stays on one braille page when it fits.

  Bold, italic, and underlined text carry the UEB typeform indicators: a word indicator before one or two emphasized words, and a passage indicator and terminator around three or more. Three or more words in capitals get the capitals passage indicator once, instead of a word indicator before each. A passage that goes on over several paragraphs or list items is opened again at the start of each and ended once, after the last; each heading stands alone. Contracted (grade 2) braille carries the same typeform indicators, placed around liblouis's contractions; its capitals are liblouis's own, one paragraph at a time.
- `pdf`: a tagged PDF that screen readers can move through by heading, list, and table.
- `adoc`, `typ`, `tex`, `wiki`, and `org`: AsciiDoc, Typst, LaTeX, MediaWiki, and Org mode, written by carta. See [AsciiDoc, Typst, LaTeX, MediaWiki, and Org](#asciidoc-typst-latex-mediawiki-and-org).

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

## Convert a folder from the reader

In the reader, File, Batch convert (also in the command palette as "batch convert") converts a folder without leaving textweaver. It asks three short questions, each with its usual answer first:

1. **The folder.** The file browser opens on your places, with the open document's folder first. Choose a folder with the browser's choose key, or with Enter on the "Choose this folder" row inside it.
2. **The format.** Markdown, PDF, HTML, plain text, EPUB, Word, or braille. HTML then asks "Theme for the HTML page?", as [export](#export-from-inside-the-reader) does.
3. **Where the files go.** In a `converted` folder inside the one you chose (the usual answer), beside each file, or in another folder you then choose in the browser.

Then textweaver says how many files it found and asks, for example, "Convert 48 files to PDF into D:\Notes\converted? y or n". Press `y` to start, `n` or Escape to cancel.

The conversion runs in the background, so you can keep reading. It says when it starts, then how far it has got in tens of percent, for example "40 percent converted, 20 of 48 files", never more often than every ten seconds. Progress follows the interface announcements setting; the final answer and any failure are always said.

To stop, press Escape while nothing else is open. textweaver asks "Stop converting? Files already done are kept. y or n". A file being written when you answer yes is finished whole, no other file is started, and no file is left half-written.

At the end you hear the counts, for example "Converted 42 files to PDF; 4 up to date; 2 failed." When any file failed, the failures are shown as a list, each with the file's name first and then the reason ("report.docx: parse error: not a valid DOCX (zip) file"). Enter on one opens that file. A [conversion report](#the-conversion-report) is saved as `conversion-report.md` in the output folder (in the folder you chose when the files go beside their sources), as `tw convert` saves it, and the end of the run always says where it is, for example "Report saved in D:\Notes\converted\conversion-report.md." When some files have images without descriptions or other items that could not be made accessible, you also hear how many, for example "3 files have items not made accessible; see the report."

A file whose output is newer than the file itself is skipped, as with `tw convert`, and counted as up to date.

## Watch a folder

```bash
tw convert Inbox --to txt --watch
```

This is a hot folder. Files already in `Inbox` are converted first, then each new file as it arrives. Converted files go to `Inbox/converted` (or to the folder you give with `--out`). Each event is spoken as one line, for example "Converted chapter 3.docx." Press Control C to stop.

It follows star's hot-folder rules:

- A file is converted only after its size has stayed the same for 2 seconds, so a file that is still being copied is never read half-way. Change the wait with `--stable-seconds 5`.
- After a successful conversion the source moves to `Inbox/processed`. Add `--keep-sources` to leave it where it is.
- A file that fails moves to `Inbox/failed`, so it is not tried again and again. A name that is already taken in either folder gets the date and time added; nothing is overwritten.
- Files textweaver cannot read, such as pictures, stay where they are, and you hear "Ignored" once.
- Everything that happens is also written, with the time, to `textweaver-watch.log` in the output folder.

Only the folder itself is watched, not its subfolders.

## Markdown flavors

Markdown comes in dialects. Choose yours with `--flavor`:

- `gfm`, the default: GitHub Flavored Markdown. Tables, task lists, strikethrough, web addresses that become links by themselves, footnotes, and alerts such as `> [!NOTE]`.
- `obsidian`: everything in GFM, plus Obsidian's own syntax. `[[Note]]` links to `Note.html`, `[[Note#Heading]]` links to that heading, and `[[Note|text]]` shows your own link text. `![[picture.png|description]]` shows a picture with that description as its alternative text. `![[Note]]` is a link to the note, or, with `--embeds inline`, the note's text itself. Callouts such as `> [!tip] Remember` become labeled notes; a callout with a minus sign after the type becomes a section you can expand and collapse. `#tags`, `==highlights==`, and block references (`^id`) work too.
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

The page's colors come from a [theme](themes.md). Use `--theme NAME` to choose one, for example `--theme sepia` or `--theme galaxy-light` for printing and sharing. Without it, `tw convert` uses the theme in your settings and never asks. With Galaxy, the usual theme, the page also follows the reader's system to Galaxy Light or High Contrast.

You can write your own templates. They are HTML files with MiniJinja placeholders such as `{{ title }}` and `{{ content }}`. Put them in a folder and pass `--templates FOLDER --template NAME`, where the name is the file name without its extension, or give a file directly with `--template my-page.html`. Your template can start with `{% extends "default" %}` to reuse the built-in page. The placeholders are listed in the rustdoc of `textweaver_render::template`.

### How the page is set

The page takes its typography from your reading settings at the moment you convert, so that a document read in the browser looks like the same document read in textweaver. The font, its size and weight come from `[reading_aids.font]`; line height, paragraph spacing, and letter and word spacing come from `[reading_aids.spacing]`; and the length of a line comes from `[display] measure`, the number of characters a line may hold (0 lets the text fill the window). Change any of these in Settings and the next page you convert follows. The size is written relative to the browser's own default size, so the reader's browser zoom and minimum font size still apply. Themes carry colors only, which is why an older theme file keeps working unchanged. A user template that prints `{{ stylesheet }}` gets the same settings.

The rest of the page follows a few rules, each chosen so that nothing depends on color alone:

- **Headings** use one scale, each level 1.2 times the size of the level below it, with the space around a heading measured in the heading's own size. Levels 5 and 6 share the body size; level 6 is set in italics.
- **Code blocks** that name their language (a fence that opens with three backticks and `rust`, for example) are highlighted with the same syntax definitions and token kinds the terminal reader uses. Each kind takes a color from the theme's text roles, and the two that matter most also change their type: keywords are bold and comments are italic, so a black-and-white printout or a contrast theme keeps them apart. Strings keep their quotation marks and numbers their digits. A block in an unknown language, or without one, stays plain. Code blocks sit on the page's own background inside a border, because several token colors fall short of the 4.5 to 1 contrast floor on the darker code background.
- **Tables** set their header row in bold above a double rule, and every cell has a border, so rows are told apart by their borders; the tint on every other row is an extra. A table wider than the window scrolls sideways inside its own box, which takes keyboard focus and is named "Table 1", "Table 2", and so on, so it can be scrolled from the keyboard.
- **Images** that stand alone in a paragraph become figures. The image's title, written after its address as in `![A crow on a fence](crow.png "A crow keeps watch")`, becomes the caption under it, and the alternative text stays the description that a screen reader reads. The title is not repeated on the image, so the caption is read once.
- **Footnotes** are gathered at the end under the heading "Footnotes", and each one ends with a link back to the place it was cited, named "Back to reference 1" and so on.
- **Callouts** begin with their type word, so a reader hears what kind of note it is before its title: `> [!tip] Remember` becomes "Tip: Remember". A title that already begins with the word, such as "Tip of the day", is kept as written.
- **Math** is written as MathML, with the formula's source as its text alternative (what a screen reader falls back to) and as an annotation that can be copied. A formula that cannot be read is shown as its source, marked as an error in bold.

The CommonMark flavor is the exception: it renders exactly what the specification describes, with plain code blocks, images inside paragraphs, and tables without the scrolling box.

**Printing.** The `print` template, and the default page when printed from a browser, print black on white with no tinted backgrounds, at your reading font size in points. The address of each web link is printed after it, headings are kept with the text that follows them, and code blocks, quotations, callouts, figures, and table rows are not split across pages. A table's header row repeats at the top of each printed page.

**Contrast themes and motion.** Under a Windows contrast theme (the browser's forced colors), the system's colors take over, and every border and focus outline stays drawn, so code blocks, callouts, and tables keep their edges and code keeps its bold and italics. Nothing on the page animates; the one movement, smooth scrolling to a heading, is turned off when your system asks for reduced motion.

The same templates serve `tw convert`, Export from inside the reader, the batch conversion, and the browser preview. textweaver's own tests convert a sample page and check it with `tools/check_site_a11y.py --page`, which looks for the language, the title, one first-level heading, no skipped heading levels, the skip link, the main landmark, labels, image descriptions, table headers, and anything loaded from another host.

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

When a document has print page numbers, as a DAISY book, a scanned PDF, or an EPUB or web page that marks them (with a page list, or with page-break markers) does, each page of the PDF is labeled with the print page its first line belongs to, the way a printed book numbers its pages. A PDF reader's "go to page" then takes the print page number and opens the page where that print page is under way, and "Page 3 of 20, print page 42" appears at the foot of the page. A title page and table of contents before the first print page are numbered i, ii, and so on.

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

For EPUB, `--font` and `--code-font` put one of textweaver's own fonts into the book, with its license, and make it the book's font. Reading apps may still let you choose another. Fonts installed on your computer cannot be put into a book, because their licenses may not allow it.

## AsciiDoc, Typst, LaTeX, MediaWiki, and Org

```bash
tw convert thesis.md --to tex
tw convert "Lab notes" --to org --out "Lab notes in Org"
```

textweaver does not write these five formats itself; carta, the converter built into textweaver, writes them. The output is a whole document: a LaTeX file with its preamble, ready for `pdflatex` or `lualatex`; a Typst file with its page setup, ready for `typst compile`; and AsciiDoc and Org files that begin with the title. MediaWiki markup has no document header, so it is the page's text alone. The extensions are `.adoc`, `.typ`, `.tex`, `.wiki`, and `.org`, and `--to` also takes the full names `asciidoc`, `typst`, `latex`, and `mediawiki`. Batch convert in the reader offers the same five formats after the others.

carta writes from Markdown. A Markdown file goes to carta as it is, so its front matter title, footnotes, math, tables, and code blocks become the target format's own. Any other document is first turned into Markdown, the same Markdown `--to md` writes, and that Markdown goes to carta. Whatever Markdown cannot express is lost on the way: the footnotes of an Org file, for example, arrive as a numbered list under a "Footnotes" heading rather than as notes, and its title is the first heading rather than the document's title. To keep the most, convert to Markdown first, check it, and convert the Markdown.

The publishing templates, `--template`, and the PDF and EPUB layout options do not apply to these formats; each uses its usual layout. Citations in Markdown are formatted, as for every output but Markdown.

On the sample documents, carta's output, read back by Pandoc, gave the same text as Pandoc's own writers for LaTeX, MediaWiki, and Org. For AsciiDoc and Typst, carta kept a footnote, a link's text, and a formula that Pandoc's own round trip lost. In MediaWiki, quotation marks inside a code block are written as `&quot;`, which MediaWiki shows as quotation marks.

## Other options

- `--engine comrak` uses the comrak Markdown parser instead of the default, pulldown-cmark. comrak implements every detail of GitHub's specification; pulldown-cmark is faster. Both give the same page structure.
- `--jobs 4` limits the work to four processor cores. By default every core is used.
- `--sanitize` removes scripts and other unsafe HTML, for Markdown you did not write yourself.
- `--smart` turns straight quotes into curly quotes and double hyphens into dashes.
- `--from dokuwiki` reads every file in the batch as DokuWiki, whatever its extension; `--from jira` reads Jira markup the same way. Any extension a reader takes also works, such as `--from org` for Org files saved as `.txt`. See [Org, reStructuredText, and wiki markup](#org-restructuredtext-and-wiki-markup).
- `--no-pandoc` never runs Pandoc (see below).
- `--pandoc-timeout 300` gives Pandoc up to 300 seconds for each file instead of 120 (see below).
- `--pdf-font FILE` chooses a font file for the text of PDF output. `--font` does the same by name.
- `--verbose` reads out every file, not only the failures and the summary.
- `--json` prints the full result, every file with its status and timing, as JSON for scripts.
- `--report-format json` writes the [conversion report](#the-conversion-report) as JSON instead of Markdown.
- `--no-report` does not save a conversion report.

## Formats textweaver reads

textweaver reads these formats itself:

- Markdown, HTML, plain text, EPUB, Word (DOCX), and PDF. A PDF's links, comments, and filled-in form fields are read too. See [PDF files](#pdf-files).
- RTF (Rich Text Format): headings, lists, tables, footnotes, links, pictures' descriptions, and text in older code pages such as Cyrillic and Japanese.
- OpenDocument text (ODT, OTT, and flat FODT), as LibreOffice and Google Docs save it: headings, numbered and bulleted lists, tables, footnotes, links, and pictures' descriptions.
- LaTeX (TEX, LATEX, and LTX): sections, lists, tables, math, references, citations, and footnotes, and the files it includes from its own folder. See [LaTeX](#latex).
- Email (EML) and web pages saved from a browser as one file (MHTML and MHT). See [Email and web archives](#email-and-web-archives).
- Scanned PDFs and pictures (PNG, JPEG), by recognizing their text. See [Scanned pages](#scanned-pages-ocr).
- DAISY 3 books and DTBook files, including Bookshare zips.
- DAISY 2.02 books: open `ncc.html` in the book's folder, or the zip. Each heading in `ncc.html` starts a section, and the print page numbers are kept for "go to page" but not read aloud. The text files are read in the order the book's SMIL files give. A book with no text, only headings and audio, reads its headings and says so. Reading a book that has a recorded narration plays the recording; see [Talking books](reading.md#talking-books-the-recorded-narration). Conversion writes the text only.
- Braille files (BRF and BRL), such as the braille books of the NLS BARD service, read back to print through liblouis. See [Braille files](reading.md#braille-files-brf).
- PowerPoint (PPTX): slides in order, each with its speaker notes.
- Spreadsheets: CSV, TSV, OpenDocument (ODS), and Excel (XLSX, XLSM, XLSB), as tables. Old binary Excel files (XLS) are not read.
- Archives (ZIP, TAR, TAR.GZ, and 7Z): opening one lists the files inside that textweaver can read, each a link. To open a file inside an archive directly, write its name after a `!`, as in `tw text course.zip!week1/notes.md`.
- Web pages: `tw open https://example.org/page` and `tw text https://...` fetch the page and read it. A PDF or other file at the address is saved in the cache and opened from there.
- Obsidian notes, with their callouts, embedded notes, tags, highlights, and block links. See [Obsidian notes](#obsidian-notes).
- JSON and JSON Lines, as headings and lists you can move through by key, and Jupyter notebooks (IPYNB), cell by cell. See [JSON and notebooks](#json-and-notebooks).
- SVG drawings, by their title, description, labeled parts, and text. See [Drawings and formulas](#drawings-and-formulas).
- MathML formulas (MML), presentation or content MathML, as one formula.
- Org mode (ORG), reStructuredText (RST and REST), MediaWiki (WIKI and MEDIAWIKI), DokuWiki, and Jira markup, read by carta, a converter written in Rust that runs inside textweaver. See [Org, reStructuredText, and wiki markup](#org-restructuredtext-and-wiki-markup).

For other formats, such as Textile, DocBook, and FictionBook, `tw convert` asks Pandoc when Pandoc is installed. Pandoc is never used for a format textweaver reads itself, and RTF, OpenDocument text, LaTeX, notebooks, Org, and reStructuredText no longer need it. `--no-pandoc` turns it off. Text sent to Pandoc in an older encoding, such as a Windows-1252 file, is converted to UTF-8 first, because Pandoc reads only UTF-8.

JSON files and SVG drawings open when you name them, but a folder of them is not treated as a folder of documents: the library, `tw convert` on a folder, and a watched folder leave them out, as they leave out pictures and archives.

Equations in a Word document are read as math. textweaver turns them into LaTeX between dollar signs, as in Markdown, so they are spoken as formulas. Math written in MathML, in a web page, a saved web page, or an EPUB book, is read as math the same way.

### LaTeX

```bash
tw text "Chapter 3.tex"
```

textweaver reads LaTeX itself, the way course notes and papers use it, without Pandoc or a TeX installation:

- Sections are headings, numbered as LaTeX numbers them, such as "2.1 Methods", so you can move by heading. The title, author, and date come from `\maketitle`, and the abstract is a section of its own.
- Numbered, bulleted, and description lists; tables, with the first row as the header when a rule follows it; and captions, read as "Table 1: Cell counts". A figure's caption describes its picture.
- Math between dollar signs, `\[ \]`, and in the `equation`, `align`, and `gather` environments is read as math, like math in Markdown, and numbered equations say their numbers.
- `\ref` reads the number of what it names, and a reference to a section is a link to its heading. `\cite{doe2020}` is a citation, read and formatted as in Markdown (see [Citations](#citations)).
- Footnotes, emphasis, links, code listings, accents, and theorems declared with `\newtheorem`.
- Your own `\newcommand` and `\def` shortcuts are expanded, in text and in math, with up to nine arguments, such as `\newcommand{\vect}[1]{\mathbf{#1}}`. Your own environments made with `\newenvironment` work too.
- A table cell made with `\multicolumn` or `\multirow` says what it spans after its text, such as "Totals (spans 3 columns)".
- A picture is described by its figure's caption, or by the `alt` key of `\includegraphics[alt={A cell dividing}]{cell.png}`, or else named by its file.
- `\bibliography{refs}`, or biblatex's `\addbibresource{refs.bib}` with `\printbibliography`, reads `refs.bib` from the document's folder and lists the works you cited under "References", formatted in the style `\bibliographystyle` names (numeric styles such as `plain` and `ieeetr` as IEEE, the others as APA). `\nocite{*}` lists every entry. The reader built without its publishing features does not read the bibliography, and says so in the document's warnings.
- `\input` and `\include` read other `.tex` files from the document's folder and its subfolders, never from anywhere else. A file that is outside the folder, missing, or too large is left out, and textweaver tells you which.

What textweaver does not know is never lost: a command it does not know is left out and its text is read. The document's warnings list those commands, for example "Some LaTeX commands are not supported, so only their text is read: \hl." The preamble, layout commands such as spacing and page breaks, and drawings made with TikZ are left out.

A file over 16 megabytes is refused. A document that includes more than 8 levels of files, or is so long or so tangled that it passes textweaver's limits, is read up to that point, and the warning says the rest was left out.

### Org, reStructuredText, and wiki markup

```bash
tw text notes.org
tw convert "Course wiki" --from dokuwiki --to md --out "Course notes"
```

Org mode files, reStructuredText, and MediaWiki pages open in the reader and convert with `tw convert` like any other document, with nothing else installed. They are read by carta, a young converter written in Rust and modeled on Pandoc, which runs inside textweaver. carta turns the file into a web page in memory, and textweaver's own HTML reader takes it from there, so headings, lists, tables, links, and footnotes behave exactly as they do in a web page, and a heading move, a search, or a braille line works the same way. An Org file's `#+TITLE` becomes the document's title and its first heading. Math is read as math. On the sample documents, carta and Pandoc gave the same text and the same structure, and carta took a few milliseconds where Pandoc took about a third of a second.

DokuWiki pages and Jira markup have no file extension of their own; a DokuWiki page is saved as a `.txt` file. Name the format with `--from`, as in the example above, and every file given to that command is read as DokuWiki. Name a folder that holds only pages in that format, because `--from` applies to every file in the batch. A file whose name ends in `.dokuwiki` or `.jira` is recognized without `--from`.

When Pandoc is installed as well, carta reads these formats and Pandoc is not asked. Typst and LaTeX are not read through carta: carta's Typst reader would follow a document's instructions to read other files anywhere on the computer, with no way to turn that off, and textweaver's own LaTeX reader keeps tables and the title that carta loses.

In reStructuredText, the `.. include::` directive is never followed, because carta would read the named file from anywhere on the computer. Each include is left out, and the document's warnings say "An include directive was left out, so the file it names is not read." An include written inside a code example is left out too.

A few details are known to differ from Pandoc. In Org, a heading's `:ID:` property does not become a link target. In reStructuredText, the `contents` directive writes no table of contents. In MediaWiki, a displayed formula is read as an inline one. In DokuWiki, a footnote ends at the first `))`.

The lean reader, built without its default features, leaves carta out; there these files are read as plain text.

### Obsidian notes

```bash
tw text "Vault/Physics/Waves.md"
```

A note from an Obsidian vault reads the way Obsidian shows it:

- A callout says its type first, in words: `> [!warning] Hot surface` reads "Warning: Hot surface", then its text. Any type works, including your own (`> [!recipe]` reads "Recipe:"). A foldable callout says whether it starts collapsed or expanded, once: "Tip, collapsed: A folded tip". Its text is always read.
- An embedded note, `![[Other note]]`, is read in place, between "Embedded from Other note" and "End of embed". `![[Other note#Heading]]` reads only that section, and `![[Other note#^block]]` only that block. Embedded notes are found in the note's own folder and its subfolders, never anywhere else. An embed inside an embed is read too, but not deeper than that; a note that embeds itself, or one that embeds it, is not read again, and the text says so.
- An embedded picture, `![[diagram.png|300]]`, is a graphic named by its file.
- Tags read as words: `#physics/waves` is "tag physics slash waves". A number such as `#12` stays as it is.
- `==highlighted text==` is marked as a highlight, and `%%comments%%` are not read.
- Block ids such as `^key-point` are not read, but a link to `Note#^key-point` still finds the block.

`tw convert` with the `obsidian` flavor writes the same callouts to HTML, with the same types, titles, and fold states, because the reader and the converter share one set of callout rules.

### JSON and notebooks

```bash
tw text profile.json
```

A JSON file reads as a document you can move through by key with `h`: every top-level key is a heading, "name: Ada Example" for a plain value, or "address, object, 3 keys" before the values inside. Objects and arrays deeper down are headings one level lower, such as "courses, array, 2 entries" and then "item 1 of 2, object, 3 keys", down to heading level 6, and plain values inside them are list items, such as "city: Portland". Brackets, braces, commas, and quotes are never read. A file that is not valid JSON is read as plain text, and its warning says where the JSON broke, for example "at line 3, column 5".

A JSON Lines file (JSONL or NDJSON) reads each line as a heading, such as "line 3, object, 4 keys", with its values below it.

A Jupyter notebook (IPYNB) reads cell by cell: text cells as Markdown, with their headings, lists, and math; code cells as code after a line that names the language, such as "Python code"; and each code cell's results after an "Output" line: printed text as a quote, errors by name and message, and pictures as graphics, such as "Output picture, PNG". The notebook's title is its first heading.

A JSON or notebook file over 64 megabytes, or nested more than 256 levels deep, is refused or read as plain text.

### Drawings and formulas

```bash
tw text chart.svg
```

An SVG drawing is read the way screen readers read one on a web page: its title first, as a heading, then its description, then each part that has a title of its own, such as the bars of a chart, as a list, and then the text it shows, one line each. A drawing marked as one picture (`role="img"`) reads only its title and description. A drawing with no title, description, or text reads "Drawing with no description". The same reading applies to a drawing inside a web page, except that one with nothing to read stays silent, as a decorative picture does.

A MathML file (MML) is one formula, read as math is anywhere else. Both kinds of MathML work: presentation MathML, which describes how a formula looks, and content MathML, which describes what it means (`<apply><plus/>...`). Content MathML becomes the same formula, with parentheses where its structure needs them; an operator textweaver does not know is read by its name.

### Email and web archives

```bash
tw text "Lab notes.eml"
```

An email is read in this order: the subject as a heading; then From, To, Cc, and the date, one line each, with the weekday, such as "Date: Monday, September 28, 2026, 10:15, UTC minus 7"; then the message. Quoted lines from an earlier message, the ones that start with `>`, are read as a quote. A message sent only as HTML is read like a web page. Attachments are listed at the end under "Attachments", each with its size, such as "notes.pdf, 240 KB"; textweaver does not open them.

A web page saved as one file (MHTML, or MHT) is read like the page itself: headings, lists, tables, math, and pictures' descriptions. Its links lead to the pages they named on the web, and a picture without a description of its own is described by the one the browser saved with it, when there is one.

A file over 128 megabytes, or a message with more than 10,000 parts or parts nested more than 32 deep, is refused.

### Comments and tracked changes

Comments in Word, OpenDocument, and PDF files, with their replies and whether they are resolved, come with the document. When you open it in the reader, each comment becomes a note on the text it is about, tagged `comment`, so reading tells you when you reach it ("Note: Comment by Ada Example: say how salty. Reply by Bo Example: added. Resolved."), and the notes list has them all. A comment you edit or keep stays as you left it when the document opens again. In a PDF, a highlight or a struck-out sentence with nothing typed in it is a note too ("Note: Comment by Ada Example: Highlighted"), since the marking is the comment.

Tracked changes in Word, OpenDocument, and RTF files are read as the final text by default: insertions as ordinary text, deletions left out. The reading setting `revisions` changes that:

- `auto`, the default, says each change where it is when announcements are set to high verbosity, and reads the final text otherwise.
- `marked` always says them: "The quiz is on (deleted by Bo Example: Tuesday) (inserted by Ada Example: Thursday)."
- `final` never does.

Set it in `settings.toml` under `[reading]`, for example `revisions = "marked"`. A document already open keeps the way it was read until you open it again. `tw convert` and `tw text` read the final text.

Pandoc runs in its sandbox, so a document cannot make it read other files on your computer (a LaTeX `\input`, for example); this needs Pandoc 2.19 or later. A file Pandoc takes more than two minutes on is stopped and counted as failed, with the reason "pandoc took longer than 2 minutes and was stopped", and the other files go on. Change the limit with `--pandoc-timeout SECONDS` or the `TEXTWEAVER_PANDOC_TIMEOUT` environment variable. To use a Pandoc that is not on your `PATH`, set `TEXTWEAVER_PANDOC` to its full path.

A damaged or deliberately odd file cannot stop a batch either. Content nested thousands of levels deep, in a web page, EPUB, Word, OpenDocument, or RTF document, is read as plain text below 256 levels, with the warning "Some content was nested too deeply to keep its structure, so it is read as plain text." List and page numbers that claim impossible values are capped. A file that is really a picture, a program, or another binary file is refused after its first 8 kilobytes, however large it is. Word, OpenDocument, EPUB, and PowerPoint files are zip packages; one with more than 50,000 files inside, with files that overlap, or with a file that claims to unpack to more than 1,000 times its size is refused, and no package is unpacked past 1 gigabyte.

The reader, `textweaver`, does not use Pandoc. It opens RTF, OpenDocument text, Org, and reStructuredText itself. For a format only Pandoc reads, such as Textile, convert it to Markdown first, then open the Markdown:

```bash
tw convert essay.textile --to md
```

```bash
textweaver essay.md
```

### PDF files

PDF files are read with column-aware reading order: running headers and page numbers are left out, and headings, lists, and tables are recovered. [ADR-0010](adr/0010-pdf-loader.md) explains how the PDF reader works. What a PDF keeps beside its pages is read as well:

- **Links** are links. A web or email address is said, and offered to open, when you follow it. A link to another part of the PDF goes to the heading there, or else to the page, which is said as "Page" and its printed number, then its first line. Links that would run a program or a script are left out.
- **Comments** from a PDF viewer, such as sticky notes, highlights, and strike-outs, are notes on the text they mark, as in Word. See [Comments and tracked changes](#comments-and-tracked-changes).
- **Form fields** are read where they are on the page, the label first and then what was filled in: "Name: Ada Example", "Student ID, required: empty", "I agree to the terms: checked", "Payment: Credit card", "Signature: not signed". The printed label and the line to write on are not read a second time. Buttons such as Submit are left out. A form made in XFA, an older Adobe format, is not read, and a warning says so.
- **Captions** that start like "Figure 3." or "Table 2:" are found by that pattern, in English, Spanish, French, German, Portuguese, and Arabic ("Tabla 2:", "Abbildung 3.", "الشكل ٣:"). A table's caption becomes the table's name, said when you move to the table. A figure's caption is read as a graphic's description. A caption set large or bold is not taken for a heading.

[ADR-0048](adr/0048-pdf-annotations-links-and-forms.md) explains the choices and the limits.

### Scanned pages (OCR)

A scanned PDF has pictures of pages instead of text. textweaver recognizes the text in them (optical character recognition, OCR), and then reads the pages like any other PDF, with headings, paragraphs, and page numbers. Pictures (PNG and JPEG) are read the same way. Recognized text can contain mistakes, so textweaver says when a document was recognized.

- **English** is read inside textweaver, by the ocrs engine. Its models are a one-time download of 12.2 MB, under the CC BY-SA 4.0 license. Run `tw ocr download`; it says what it will download and asks first. `tw ocr status` says what is ready.
- **Other languages** are read by Tesseract, a free program you install yourself, with the data for your language. Set the language with `ocr_lang` in the `[reading]` section of the settings, for example `ocr_lang = "fra"` for French, or `"deu+eng"` for German and English. When a PDF names its own language, that is used.
- `ocr_engine` chooses the engine: `auto` (the default), `ocrs`, `tesseract`, or `paddle` (an experimental in-process engine for accented Latin-script languages; download it with `tw ocr download paddle-latin`). `ocr = false` turns recognition off.
- `tw ocr read scan.pdf` recognizes a file and prints its text, with progress. Press Control C to stop it.
- A page scanned sideways or upside down is turned upright before it is read, so its lines come in order. textweaver tells which way is up from the page itself; nothing is downloaded for it.
- A scanned table is read as a table, with rows and columns you can move through, when its rows and columns line up. A first row of words over rows of numbers is its header row.
- Recognized pages are remembered, so a book opens instantly the second time.
- In the reader, a scanned book takes about a second a page to open the first time. Every three seconds you hear which page it is on, such as "Still opening scan.pdf: recognizing text on page 3 (3 of 40)." Escape stops it.

[ADR-0026](adr/0026-ocr-and-student-formats.md) explains the choices and gives measurements.

## Export from inside the reader

You can also convert the document you have open without leaving textweaver. Press **F2** for the command palette and type part of one of these names:

- `export html`: a web page.
- `export pdf`: a tagged PDF.
- `export docx`: a Word document.
- `export epub`: an EPUB book.
- `export brf`: braille, with tables laid out by `[braille] table_format`.

The file goes next to the document, with the same name: exporting `essay.md` to PDF writes `essay.pdf` in the same folder, replacing an older export. In edit mode the text you are editing is exported, saved or not. Citations are formatted and a References section added, as with `tw convert`: from the bibliography your front matter names, the folder's `references.json`, and your own library.

You hear "Exporting to PDF." and can go on reading or writing while it works. If it takes more than two seconds you hear "Still exporting to PDF, 2 seconds.", and then again every ten seconds, never more often. When it is done you hear the file's name and a question, then the format and the folder, for example "Exported essay.pdf. Open it? y or n. Format PDF, in C:\Users\ada\Essays." Press **y** to open it with your computer's program for that kind of file, or **n** to leave it. A warning, such as an image that was not found, is read out before the question.

Exporting to HTML first asks "Theme for the HTML page?" with a list of themes. Your reading theme is first and selected, so Enter keeps it; choose another, such as Galaxy Light for printing and sharing, with the arrow keys. Escape cancels the export. textweaver remembers your answer until you quit and offers it first next time.

A new document that was never saved has no folder yet; its export goes to the folder textweaver was started in, like Save As suggests.

### Preview in the browser

Type `preview in browser` in the palette. textweaver writes the document as a web page, with math as MathML so screen readers can read it, and opens it in your default web browser. It asks for the theme first, as an HTML export does. The page is kept in the `preview` folder of textweaver's cache folder, and images and links in it still point beside your document.

The first preview of a session says in one sentence what will happen next, such as "Preview opens in your browser. Press F5 there after each save." While you edit, each save (**Ctrl+S**) writes the preview again and you hear "Preview updated. Press F5 in the browser." By default the browser does not reload by itself, so it never moves your screen reader's place without your asking. The setting Browser preview follows (View menu, the palette, or `[preview] follow` in Settings) lets the page reload by itself after each save, or also when you pause in your typing; it is served for this only from your own computer. [The editing guide](editing.md#preview-in-the-browser) explains the choices.

## When something fails

A file that cannot be converted never stops the others. After the summary, each failure is read out with its reason, for example "Failed: old.textile: no native reader for .textile files, and Pandoc is not installed". When any file fails, `tw convert` ends with exit status 1, so scripts can tell.

Failures are also listed in the [conversion report](#the-conversion-report), each with the file's name first, for example "report.docx: parse error: not a valid DOCX (zip) file".

Two cases are refused before converting:

- An output that would replace its own source, such as Markdown to Markdown in the same folder. Choose an output folder with `--out`.
- Two sources that would write the same output, such as `notes.md` and `notes.txt` both becoming `notes.html`. Rename one of them.

## The conversion report

Every conversion leaves a report that says where each output came from and what in it could not be made accessible. An accommodations office can keep it in a student's file, or send it to a publisher as the list of what to fix.

**Where it goes.**

- A folder, or files converted with `--out`: one report for the run, `conversion-report.md` in the output folder (or in the folder you converted, when there is no `--out`), with a section for each file.
- A file named on its own without `--out`: a report beside its output, named after it. Converting `essay.md` to PDF writes `essay.pdf.report.md`.
- Export from inside the reader: a report beside the exported file, the same way.
- Batch convert from the reader: `conversion-report.md`, as for a folder.

Each run replaces the report it writes, so it always describes the last run. A report is never converted as a document. `tw convert` ends by saying where the report is, for example "Report saved as converted/conversion-report.md." `--report-format json` writes `conversion-report.json` (or `essay.pdf.report.json`) instead, for programs and records systems, and `--no-report` writes none. Watching a folder keeps its own log instead; see [Watch a folder](#watch-a-folder).

**What it says.** The report starts with the summary sentence, then:

- **About this report:** the version of textweaver that made it, the date and time it was written (from the computer's clock, in UTC, with the weekday), the output format, and the counts.
- **Could not be made accessible:** the files with such items, and how many each has.
- **Failed:** each failure with its reason.
- **Files:** a section for each file, with what happened to it, the output's name, the source's SHA-256 (a fingerprint that shows the file has not changed since), and the source's size. Then the list of what could not be made accessible, and the writers' other notes, such as an image that could not be embedded or characters braille cannot show.

**What it looks for.** Each item says what it is first, then where it is: the heading it is under, the print page when the source has pages (a PDF, a DAISY book, a scan), and the line: in the source for a Markdown file, where you would fix it, and in the converted text for other formats.

- **Image without a description:** an image with no alt text, or only its file name as its description. Screen readers skip it or read the file name. (In HTML sources, an image with no alt text at all is left out when the page is read, so it is not listed yet.)
- **Table whose columns do not line up:** rows with different numbers of cells. This is common in tables recognized by OCR from a scan; check the table against the original.
- **Table without a header row:** its cells cannot be read with their column headings.
- **Math that did not parse:** the formula is shown as written, with why it did not parse. It is written as its source, so it may read poorly.

For example:

```markdown
### lab.html

- Result: converted to lab.epub
- Source SHA-256: 3f0a…
- Source size: 512 bytes

Could not be made accessible, 3 items:

1. Image without a description: chart.png. Under the heading "Results", line 3.
2. Table whose columns do not line up: 2 rows, with 2, 1 cells. Under the heading "Data", line 7.
3. Math that did not parse: `$\left( a$`, \left without \right. Under the heading "Model", line 11.
```

**Privacy.** The report names files by their name, or their path inside the folder that was converted, never by a full path. It holds no user name or computer name, and an image is named by its file name only. Nothing is sent anywhere. `--json` on the command line prints the same facts (each file's `sha256` and `issues`) with full paths, for scripts on your own computer.

## See also

- [Math](math.md): how LaTeX and ASCIIMath are read aloud and turned into MathML.
- [Citations](citations.md): the reference library behind Pandoc citations.
- [Writing and editing](editing.md): listening to the rendered text while you write.
- [Themes](themes.md): the colors HTML output uses.
- [Audio export](audio-export.md): turning a document into an audiobook instead.
- [scripts/README.md](../scripts/README.md#convert-foldersh-and-convert-folderps1): the convert-folder helper scripts.
- [ADR-0016: Rendering and bulk conversion](adr/0016-rendering-and-conversion.md): how conversion works and how fast it is.
- [ADR-0017: Native writers](adr/0017-writers.md): EPUB, Word, braille, and PDF output, and their accessibility checks.
- [ADR-0010: PDF loader](adr/0010-pdf-loader.md): how PDFs are read.
- [ADR-0048: PDF annotations, links and forms](adr/0048-pdf-annotations-links-and-forms.md): PDF comments as notes, links, form fields, captions, and rotated and tabular scans.
- [Documentation index](README.md)
