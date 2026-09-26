# Citations and references

`tw cite` keeps a list of the books and articles you cite, called a reference library. You can add a reference by typing its DOI or ISBN, import references from other programs, format them in a citation style such as APA or MLA, and check that every citation in your essay points to a reference you have. It is for students and writers who work by keyboard and speech. Every message is a full sentence, so it reads well with a screen reader or with textweaver's own voice. Citations in your writing use Pandoc's Markdown syntax, such as `[@doe2020, p. 12]`, so the same files work with Pandoc and other academic tools.

## Before you start

- You need `tw`, which comes with textweaver. See [the install guide](install.md).
- Adding a reference by DOI or ISBN needs an internet connection the first time. Everything else works offline.
- You do not need Pandoc, Zotero, or any other program.

## What a reference library is

A reference library is one file called `references.json`. It holds your references in CSL-JSON, the format Zotero, Pandoc, and doi.org use. You can open it in any text editor, and Pandoc can read it directly.

textweaver uses two kinds of library:

- **Your personal library.** This is `references.json` in textweaver's data folder. Every `tw cite` command uses it unless you say otherwise.
- **A folder library.** This is `references.json` inside a folder of documents, such as a folder for one essay or one course. It keeps a project self-contained: copy the folder, and its references go with it. Choose it with `--folder FOLDER`.

When `tw cite format` or `tw cite check` looks up a key, it looks in the folder library first, then in your personal library. So a folder library can hold just the references for one project, and still use the rest from your personal library. The other commands (`add`, `import`, `export`, `list`, and `remove`) work on the folder library alone.

You can also use any library file with `--library FILE`. Like a folder library, it is searched first, and your personal library second, when formatting and checking. You cannot give `--folder` and `--library` together.

The personal library is in these places:

- Windows: `%APPDATA%\leavesofgrass\textweaver\data\references.json`
- macOS: `~/Library/Application Support/org.leavesofgrass.textweaver/references.json`
- Linux: `~/.local/share/textweaver/references.json`
- When the `TEXTWEAVER_HOME` environment variable is set: `data\references.json` inside that folder.

The file is created the first time you add or import a reference.

## Find your library

```bash
tw cite list
```

This lists every reference and says where the library file is. When the library is empty, you hear, for example:

```text
The reference library at C:\Users\you\AppData\Roaming\leavesofgrass\textweaver\data\references.json is empty. Add references with tw cite add or tw cite import.
```

To see a folder library instead, add `--folder` and the folder:

```bash
tw cite list --folder "Biology essay"
```

The `--folder` and `--library` options can go before or after the command name. `tw cite --folder "Biology essay" list` works too.

## Add a reference by DOI

```bash
tw cite add 10.1038/nature12373
```

A DOI is the identifier printed on most journal articles, such as `10.1038/nature12373`. You can type it in any of these forms:

- `10.1038/nature12373`
- `doi:10.1038/nature12373`
- `https://doi.org/10.1038/nature12373`
- `https://dx.doi.org/10.1038/nature12373`

textweaver asks doi.org for the reference and adds it. This works for DOIs from Crossref, DataCite, and mEDRA, so datasets and theses work as well as journal articles. When it is added, you hear one sentence. Here is a real one:

```text
Added reference kucsko2013. Kucsko and others, 2013. Nanometre-scale thermometry in a living cell. Key kucsko2013.
```

The first part says the reference was added and gives its citation key (here `kucsko2013`). The rest is the reference itself: authors, year, title, and key. Three or more authors are read as "and others".

If the same work is already in the library, it is updated instead of added twice:

```text
Updated reference kucsko2013, which was already in the library. Kucsko and others, 2013. Nanometre-scale thermometry in a living cell. Key kucsko2013.
```

### What is sent over the network

Only the DOI is sent. textweaver makes one secure (HTTPS) request to `https://doi.org/` followed by the DOI, and asks for the answer as CSL-JSON. The request says it comes from textweaver. No file, name, or other information about you is sent.

### Wait longer for a slow connection

```bash
tw cite add 10.1038/nature12373 --timeout 30
```

textweaver waits 15 seconds for an answer by default. `--timeout` sets the number of seconds.

## Add a book by ISBN

```bash
tw cite add 978-0-306-40615-7
```

An ISBN is the number on the back of a book. It has 10 or 13 digits. You can type it with or without hyphens and spaces, and with or without `ISBN` in front, for example `ISBN 0-306-40615-2`.

textweaver checks the last digit, which is a check digit, before it asks anyone. A mistyped digit is caught at once, without using the network.

Books are looked up in Open Library (`openlibrary.org`). textweaver asks for the edition record of the ISBN. If there is none, it tries the other form of the same ISBN (10 or 13 digits). Then it asks for the authors' records to get their names, up to ten authors. Only the ISBN and Open Library's own record numbers are sent. A real result:

```text
Added reference clark1981. Clark, 1981. Error-correction coding for digital communications. Key clark1981.
```

## Lookups, the cache, and working offline

Every answer from doi.org or Open Library is saved in a cache folder. When you add the same DOI or ISBN again, the answer comes from the cache, with no network at all. So a reference you have looked up once can be added again offline, for example to a folder library.

The cache is the `citations` folder inside textweaver's cache folder:

- Windows: `%LOCALAPPDATA%\leavesofgrass\textweaver\cache\citations`
- macOS: `~/Library/Caches/org.leavesofgrass.textweaver/citations`
- Linux: `~/.cache/textweaver/citations`
- With `TEXTWEAVER_HOME` set: `cache\citations` inside that folder.

The cache keeps the newest 500 answers and removes older ones. It is safe to delete the folder at any time.

When a lookup fails, nothing is added to the library, and the message says so. These are the messages, with a DOI or ISBN in place of the examples:

- Not connected: "Could not reach doi.org. You may be offline; nothing was added. Try again when you are connected." Some technical details follow, after the word "Details".
- Too slow: "doi.org did not answer within 15 seconds. Nothing was added; try again later."
- Unknown to the service: "doi.org has no record for DOI 10.9999/example. Nothing was added."
- The service had a problem: "Open Library answered with an error, status 503. Nothing was added; try again later."
- An answer textweaver could not read: "Open Library sent an answer textweaver could not read:" then the reason, then "Nothing was added."
- Not a DOI: "10.abc is not a DOI. A DOI starts with 10, then a dot, and contains a slash, for example 10.1038/nature12373."
- A wrong ISBN: "978-0-306-40615-8 is not a valid ISBN. An ISBN has 10 or 13 digits, and the last one is a check digit; check for a mistyped digit."
- Neither: "hello is neither a DOI nor an ISBN. Give a DOI such as 10.1038/nature12373, or a 10 or 13 digit ISBN."

The service is named in each message: doi.org for DOIs, Open Library for ISBNs.

## Import references from another program

```bash
tw cite import references.bib
```

This adds every reference in a file to your library. textweaver reads these formats:

- BibTeX and BibLaTeX: files ending in `.bib` or `.bibtex`. LaTeX accents such as `M{\"u}ller` become real letters.
- RIS: files ending in `.ris`. Zotero, Mendeley, EndNote, and most library catalogs can export RIS.
- CSL-JSON: files ending in `.json`. This is what Zotero calls "CSL JSON".

If the file has another ending, textweaver looks at what is inside to tell the format. If it still cannot tell, you hear: "Could not tell what kind of reference file notes.txt is. Use a .bib, .ris, or .json file."

When the import is done, you hear how many references were new and how many were updated:

```text
Imported 2 references: 2 new, 0 updated.
```

Importing the same file again does not make copies. A reference counts as the same work when it has the same DOI, the same ISBN, or the same key and the same title. The same work is updated with the new file's details.

A different work whose key is already taken gets a new key, and the message tells you:

```text
Imported 1 reference: 1 new, 0 updated. Key doe2020 was taken, so the new reference is doe2020a.
```

Keys in the file are kept. References without a key get one made for them, as described in [Citation keys](#citation-keys).

To import into a folder library, add `--folder`:

```bash
tw cite import references.bib --folder "Biology essay"
```

## Export references

```bash
tw cite export --to bibtex
```

This prints the whole library as BibTeX. `--to` takes one of these formats:

- `bibtex` (also `bib`)
- `biblatex`
- `ris`
- `csl-json` (also `json`)

To write a file instead of printing, add `-o` (or `--output`) and a file name:

```bash
tw cite export --to ris -o references.ris
```

You hear, for example: "Exported the references as RIS to references.ris."

To export only some references, give their keys:

```bash
tw cite export --to biblatex doe2020 muller2019
```

A key that is not in the library stops the export with "The citation key nobody is not in the reference library."

## List your references

```bash
tw cite list
```

This says how many references there are and where the library is. Then it gives one line per reference, sorted by author, then year, then title:

```text
2 references in C:\Users\you\AppData\Roaming\leavesofgrass\textweaver\data\references.json:
Doe and Roe, 2020. Reading Machines. Key doe2020.
Müller, 2019. Listening to Tables. Key muller2019.
```

Each line is a sentence: authors, year, title, and key. A reference with no date says "no date".

To get the library as CSL-JSON instead, for another program:

```bash
tw cite list --json
```

## Remove a reference

```bash
tw cite remove doe2020a
```

This removes the reference with that key and says which one it was:

```text
Removed reference doe2020a: Doe, 2020. A Different Book. Key doe2020a.
```

## Format references with a citation style

```bash
tw cite format --style mla
```

This formats every reference in the library in a citation style. For each reference you get the bibliography entry, then the in-text citation on a line starting with "In text:". Here is a real result for one key:

```bash
tw cite format doe2020 --style mla
```

```text
Doe, Jane, and Rick Roe. Reading Machines. Accessible Press, 2020.
In text: (Doe and Roe)
```

Give keys to format only those references. With no keys, every reference is formatted. The default style is APA.

In a note style, such as `chicago-notes`, the second line starts with "Note:" and holds the footnote:

```text
Doe, Jane, and Rick Roe. Reading Machines. Accessible Press, 2020.
Note: Jane Doe and Rick Roe, Reading Machines (Accessible Press, 2020).
```

Each reference is formatted here as if it were cited alone. So in a numbered style such as IEEE or Vancouver, every in-text citation is `[1]`. To get a numbered list, use `--bibliography`, described below.

### Choose a style

These short names are built in:

- `apa`: APA Style 7th edition (the default)
- `mla`: MLA Handbook 9th edition
- `chicago`: Chicago Manual of Style 18th edition, author-date
- `chicago-notes`: Chicago Manual of Style 18th edition, notes and bibliography
- `harvard`: Cite Them Right 12th edition, the Harvard style
- `ieee`: IEEE Reference Guide
- `vancouver`: Elsevier's NLM Vancouver style, numbered
- `ama`: AMA Manual of Style 11th edition
- `nature`: Nature

About 80 styles are built in altogether. To hear all of them, one per line, with their full titles:

```bash
tw cite styles
```

Each line starts with the name to give to `--style`, for example `american-chemical-society: ACS Guide 2022 revision`.

You can also give the path of a `.csl` style file, such as one from the Zotero style repository:

```bash
tw cite format --style "my-university.csl"
```

An unknown name gives: "There is no built-in citation style called nope. Try apa, mla, chicago, ieee, or vancouver, or give the path to a .csl file."

### Choose plain text, Markdown, or HTML

```bash
tw cite format --bibliography --as markdown
```

`--as` chooses the markup:

- `plain` (the default): plain text with no markup at all. Italics are simply left out. This is best for listening.
- `markdown`: italics as `*Reading Machines*` and links in angle brackets, ready to paste into a Markdown document.
- `html`: HTML, with `<i>` for italics and `&amp;` for the ampersand, ready for a web page.

In Markdown and HTML, entries are separated by a blank line.

### Only the in-text citation, or only the bibliography

```bash
tw cite format doe2020 muller2019 --citation
```

`--citation` prints one in-text citation for all the keys together. In APA, for two real references:

```text
(Clark, 1981; Kucsko et al., 2013)
```

In Vancouver, the same two keys give `[1,2]`.

```bash
tw cite format --style vancouver --bibliography
```

`--bibliography` prints only the bibliography entries. In a numbered style they are numbered in order:

```text
[1] Clark GC. Error-correction coding for digital communications. New York: Plenum Press; 1981.
[2] Kucsko G, Maurer PC, Yao NY, Kubo M, Noh HJ, Lo PK, et al. Nanometre-scale thermometry in a living cell. Nature 2013;500:54–8. https://doi.org/10.1038/nature12373.
```

You cannot give `--citation` and `--bibliography` together.

Numbers such as `[1]` are always in brackets, even in styles that print them raised. So a screen reader never runs a number into the word before it.

## Citation keys

A citation key is the short name you type to cite a work, such as `doe2020`. textweaver makes keys this way:

- The first author's family name, in lowercase, followed by the year: `doe2020`.
- Accented Latin letters become plain letters, and spaces and punctuation are dropped. Müller becomes `muller2019`. Van Gogh becomes `vangogh1888`.
- With no author or editor, the first word of the title that is longer than three letters is used, skipping words such as "the", "and", and "with". "The Accessible Classroom" from 2021 becomes `accessible2021`.
- With neither, the key starts with `ref`.

When a key is already used by a different work, a letter is added: `doe2020a`, then `doe2020b`, and so on to `z`. After that, a number is added: `doe2020-2`.

Keys read aloud naturally, for example "doe twenty twenty". Keys that come from an imported file are kept as they are.

## Write citations in Markdown

textweaver understands the citation syntax of Pandoc's Markdown. Type the key after an `@` sign. These are the forms:

- `[@doe2020]`: a plain citation, such as "(Doe & Roe, 2020)" in APA.
- `[@doe2020, p. 12]`: a citation with a page.
- `[see @doe2020, pp. 33-35, emphasis added]`: text before the key ("see"), a page range, and text after it.
- `[@doe2020; @muller2019]`: two works in one citation. Separate them with a semicolon. Each part holds exactly one key.
- `@doe2020 [p. 3]`: a citation that is part of your sentence, as in "Doe and Roe (2020, p. 3) show that...". The `@` must come at the start of a line, or after a space or opening punctuation.
- `[-@doe2020]`: a minus sign before the `@` leaves out the author, for when you have already named them: "Doe and Roe agree (2020)".
- `@{odd key}`: braces around a key that has unusual characters.

The part after the key can name a page or another division. These are the most useful labels:

- `p.` and `pp.`: page and pages. A bare number is a page, so `[@doe2020, 12]` means page 12.
- `chap.`: chapter
- `sec.` or `§`: section
- `para.` or `¶`: paragraph
- `fig.`: figure
- `vol.`: volume
- `l.` and `ll.`: line and lines
- `n.`: note

Full words work too, such as `page`, `chapter`, and `section`. Braces keep an unusual value together: `[@doe2020, p. {iv, 34-37}]`.

These are not citations:

- An email address such as `jon@example.com`.
- An escaped bracket: `\[@doe2020]`.
- A link: `[@doe2020](https://example.com)`.

Punctuation right after a key is not part of it, so "as @doe2020." ends the key before the period.

## Check a document's citations

```bash
tw cite check essay.md
```

This finds every citation in a Markdown or text document and checks each key against your library. If you use `--folder`, it checks the folder library first, then your personal library. You hear one sentence, for example:

```text
5 citations found. One key is not in the library: smith1999.
```

When every key is there, it says "Every key is in the library." When there are none at all, it says "The document has no citations."

Add the missing references with `tw cite add` or `tw cite import`, or fix the spelling of the key in your document.

## See citations in converted documents

```bash
tw convert essay.md --to html --style apa
```

`tw convert` formats your citations in a citation style and adds a References section with every work you cited. It does this for web pages, Word, PDF, EPUB, braille, and plain text; Markdown output keeps your citations as you wrote them. You do not need Pandoc. This is a real conversion to plain text:

```text
The area of a circle is $\pi r^2$ (see Doe, 2020, p. 12). As Müller (2019, p. 40) argues, reading aloud helps (2020).

References

Doe, J. (2020). Reading by ear. Example Press.

Müller, A. (2019). Speech and study. Journal of Listening, 4, 33–50.
```

- A bracketed citation, `[see @doe2020, p. 12]`, becomes "(see Doe, 2020, p. 12)", with its prefix and page.
- A citation in your sentence, `@muller2019 [p. 40]`, becomes "Müller (2019, p. 40)".
- `[-@doe2020]` leaves the author out: "(2020)".
- In a web page each citation is a link to its entry in the References section.
- `--style` chooses the style: `apa` (the default), `mla`, `chicago`, `chicago-notes`, `harvard`, `ieee`, `vancouver`, `ama`, `nature`, any name `tw cite styles` lists, or a `.csl` file. With a note style, such as `chicago-notes`, each citation becomes a footnote.

Keys are looked up in this order, and the first library that has the key wins:

1. The file you name with `--bibliography FILE`, or with a `bibliography:` line in the document's front matter. It can be CSL-JSON, BibTeX, BibLaTeX, or RIS, so a `.bib` file from another program works as it is.
2. `references.json` in the document's folder (a folder library).
3. Your own library, the one `tw cite add` and `tw cite import` fill.

A key that is in none of them is written as "missing reference" and the key, and the conversion warns you: "The citation key smith1999 is not in any library, so it reads as missing reference smith1999." Citations inside code or math are never touched. With the default `gfm` flavor, `@name` counts as a citation only when `name` is a key in one of the libraries, so a mention of a person or an email address stays as it is; use `--flavor pandoc` to have every citation formatted, as Pandoc does. `--no-citations` turns all of this off. See [the converting guide](converting.md#citations) for the other options.

Pandoc can also format citations from the same library, because `references.json` is CSL-JSON: `pandoc essay.md --citeproc --bibliography references.json -o essay.html`. That is Pandoc's own feature.

## Citations while reading and writing in textweaver

The reader uses the same libraries as `tw cite`: `references.json` in the document's folder first, then your own library.

### Insert a citation: Alt+C

In edit mode, press **Alt+C**. textweaver says how many references there are, for example "Insert citation, 12 references. Type to filter, Enter chooses, Escape cancels." A list appears; each item reads like "Doe and Roe, 2020. On reading. Key doe2020."

1. Type part of an author, a year, a title word, or the key. The list keeps the references that have every word you type, and says how many match. Backspace removes a letter.
2. Press **Enter** on the reference you want.
3. textweaver asks "Page or other locator, for example 12 or chapter 2; Enter for none". Type a page (`12`), pages (`3-5` or `pp. 3-5`), or another division (`chapter 2`, `section 4.1`), or press **Enter** for none. If textweaver cannot read what you typed, it says so and asks again.

The citation goes in at the caret, for example `[@doe2020, p. 12]`, with a space before it when it follows a word. You hear what went in: "Inserted citation of Doe and Roe, 2020, page 12." With the caret inside a citation already, the new reference joins it: `[@doe2020, p. 12; @roe2021]`. Each insertion is one undo step.

### Add a reference by DOI or ISBN: Alt+Shift+D

Press **Alt+Shift+D** and type a DOI (`10.1038/nature12373`, or its doi.org address) or an ISBN. You hear "Looking up" and the identifier; you can go on reading or writing. When the answer comes, you hear, for example, "Added reference kucsko2013. Kucsko, Maurer, and Yao, 2013. Nanometre-scale thermometry in a living cell." The reference goes into the folder's `references.json` when the document's folder has one, else into your own library. Lookups use the same cache as `tw cite add`, so an identifier looked up before works offline.

### Commands from the palette

Press **F2** and type part of the name:

- `insert bibliography`: in edit mode, inserts at the caret the formatted entries of every work the document cites, in APA style, or in the style the front matter names with `csl: mla`. You hear how many entries went in and any keys that are not in a library. Put the caret under your References heading first.
- `check citations`: the same check as `tw cite check`, on the text you are reading or writing, for example "3 citations found. Every key is in the library."
- `import references`: asks for a BibTeX, BibLaTeX, RIS, or CSL-JSON file and imports it into your own library, as `tw cite import` does.

### Hear a citation in words

When a word move (the Left and Right arrows in reading) lands on a citation, or you ask for the link address there (**Alt+Shift+K**), textweaver says it in words, for example "Citation: Doe and Roe, 2020, On reading, page 12." A key that no library has is named: "missing reference smith1999".

Continuous reading still reads a citation as it is written: `[@doe2020, p. 12]` is spoken by your speech engine, which decides how it says the bracket and the key. To hear citations formatted while you write, use `listen rendered` from the palette; see [the editing guide](editing.md#listen-to-the-rendered-text).

A note can name a source with a citation key. This comes from the `cite` field in a note's front matter when you import notes from an Obsidian vault. See [the vault guide](vault.md).

## If something goes wrong

- **"You may be offline; nothing was added."** Check your internet connection and try again. A DOI or ISBN you looked up before still works offline, from the cache.
- **"did not answer within 15 seconds."** The service is slow. Try again later, or wait longer with `--timeout 30`.
- **"has no record for".** Check the DOI or ISBN for a typing mistake. Some books are not in Open Library. Then add the reference another way: export it from your library catalog as RIS or BibTeX and use `tw cite import`.
- **"is not a valid ISBN".** A digit is mistyped. The last digit is a check digit, so textweaver can tell. Read the number again from the book.
- **"The citation key doe2020 is not in the reference library."** Run `tw cite list` to find the right key. If you use a folder library, remember `--folder`.
- **"the argument '--folder <FOLDER>' cannot be used with '--library <FILE>'".** Give only one of the two.
- **"Could not read the BibTeX data on line 5:" and a reason.** The file has a mistake on or near that line. Open it and fix it, or export it again from the other program. Nothing was imported. One known case: a BibTeX file with more than one entry that has no key, such as `@book{,`, fails with "duplicate key". Give each entry a key, or import it as RIS or CSL-JSON.
- **"Could not read the CSL-JSON data on line 2:" and a reason, for your own library.** The `references.json` file was damaged, perhaps by a hand edit. Fix the JSON in a text editor, or move the file away and import your references again.
- **A wrong reference was added.** Remove it with `tw cite remove KEY` and add the right one.
- **`tw cite check` reports keys from examples.** It also counts citations inside code, such as `` `[@example]` `` or a code block. Those are not real citations, so ignore them.
- **A `.csl` file is not found.** You hear "Could not read" with the file name. Check the path, and put it in quotes if it has spaces.

## See also

- [The converting guide](converting.md): `tw convert` and its Markdown flavors, including `pandoc`.
- [The vault guide](vault.md): Obsidian notes, including notes that name a source.
- [The editing guide](editing.md): writing Markdown in textweaver with speech feedback.
- [ADR-0019: Citations](adr/0019-citations.md): the design decision behind this feature.
- [Documentation index](README.md)
