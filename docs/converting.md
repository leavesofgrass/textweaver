# Converting documents

`tw convert` turns documents into other formats: one file, a whole folder of files, or every file that arrives in a folder you are watching. It uses every processor core, and it skips files that are already converted, so running it again on a large folder takes a moment.

This guide is written to be read with a screen reader. Each section starts with the command, then explains it.

## Convert one file

```
tw convert notes.md --to html
```

This writes `notes.html` next to `notes.md`. When it finishes, you hear one sentence, for example: "Converted 1 file to HTML in 12 milliseconds. No failures."

The formats you can ask for with `--to`:

- `md`: Markdown (the default).
- `html`: a web page you can open in any browser. Headings, lists, tables, and math are real HTML structure, so screen readers can move by heading, table, and list.
- `txt`: plain text, the same text textweaver reads aloud.
- `epub`, `docx`, `brf`, and `pdf`: EPUB books, Word documents, braille files, and tagged PDF. These arrive with the native writers; until then `tw convert` says so before it starts.

## Convert a folder

```
tw convert "Biology notes" --to html --out "Biology site"
```

Every document in the folder and in all of its subfolders is converted. The output folder mirrors the input: `Biology notes/week 1/cells.md` becomes `Biology site/week 1/cells.html`. Hidden files and folders, whose names start with a dot, are left alone.

Without `--out`, each output is written next to its source. In that case files that already have the output's extension are treated as earlier outputs, not as sources, so converting a folder to HTML twice does not convert the HTML files it made the first time.

You can name several files and folders in one command.

## Only what changed is converted again

When you run the same command again, a file is skipped when its output is newer than the source. Edit one note, run the command, and only that note is converted. The summary tells you how many files were skipped.

To convert everything anyway, for example after changing the template, add `--force`:

```
tw convert "Biology notes" --to html --out "Biology site" --force
```

Outputs are written to a temporary file first and then renamed, so stopping a conversion part way never leaves a half-written file behind.

## Watch a folder

```
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
- `pandoc`: Pandoc's Markdown. Definition lists, fenced divs (`::: warning`), spans with classes (`[text]{.smallcaps}`), heading ids (`# Title {#intro}`), citations (`[@doe99, p. 33]`, shown as links to `#ref-doe99`), `H~2~O` and `2^10^`, and a title block (`% Title`, `% Author`, `% Date`).
- `commonmark`: plain CommonMark, with no extensions.

In every flavor except `commonmark`, a YAML block at the top of the file (between two lines of three dashes) sets the page's title, language, author, date, and description.

## Math

Math written in LaTeX between dollar signs, `$x^2$` inside a sentence or `$$ … $$` on its own lines, becomes MathML. Screen readers that read math (NVDA with MathCAT, JAWS, VoiceOver) can then speak it and let you explore it term by term. The LaTeX source travels with it, so copying still gives you the LaTeX. A formula that cannot be read is shown as its LaTeX source. Add `--no-math` to keep all math as LaTeX.

## Templates

HTML output is a complete web page made from a template. Three are built in; choose one with `--template`:

- `default`: an accessible page. It declares its language, starts with a "Skip to content" link, puts the document in a main landmark, adds a table of contents when there are two or more headings, and uses a readable style that follows your system's dark mode, high contrast, and reduced-motion settings.
- `print`: for printing or saving as PDF from a browser.
- `fragment`: only the converted HTML, for pasting into another page.

Use `--no-toc` to leave out the table of contents.

You can write your own templates. They are HTML files with MiniJinja placeholders such as `{{ title }}` and `{{ content }}`. Put them in a folder and pass `--templates FOLDER --template NAME`, where the name is the file name without its extension, or give a file directly with `--template my-page.html`. Your template can start with `{% extends "default" %}` to reuse the built-in page. The placeholders are listed in the rustdoc of `textweaver_render::template`.

## Other options

- `--engine comrak` uses the comrak Markdown parser instead of the default, pulldown-cmark. comrak implements every detail of GitHub's specification; pulldown-cmark is faster. Both give the same page structure.
- `--jobs 4` limits the work to four processor cores. By default every core is used.
- `--sanitize` removes scripts and other unsafe HTML, for Markdown you did not write yourself.
- `--smart` turns straight quotes into curly quotes and double hyphens into dashes.
- `--no-pandoc` never runs Pandoc (see below).
- `--verbose` reads out every file, not only the failures and the summary.
- `--json` prints the full result, every file with its status and timing, as JSON for scripts.

## Formats textweaver reads

textweaver reads Markdown, HTML, and plain text itself, and EPUB, Word, and PDF as those readers arrive. For other formats, such as OpenDocument text, RTF, reStructuredText, Org, and LaTeX, it asks Pandoc when Pandoc is installed. Pandoc is never used for a format textweaver reads itself.

## When something fails

A file that cannot be converted never stops the others. After the summary, each failure is read out with its reason, for example "Failed: old.rtf: no native reader for .rtf files, and Pandoc is not installed". When any file fails, `tw convert` ends with exit status 1, so scripts can tell.

Two cases are refused before converting:

- An output that would replace its own source, such as Markdown to Markdown in the same folder. Choose an output folder with `--out`.
- Two sources that would write the same output, such as `notes.md` and `notes.txt` both becoming `notes.html`. Rename one of them.
