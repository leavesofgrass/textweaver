# ADR-0019: Citations

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): `tw cite` is wired into the CLI as described. Two parts of this design are not connected yet. The editor has no citation picker: the `insert` API is ready, but no app code calls it. And the Pandoc-flavor renderer does not use this crate: `tw convert --flavor pandoc` shows each citation's keys as a link to a `#ref-` anchor, formats nothing, and adds no bibliography. The offline message quoted under Lookup also ends with the error's details. hayagriva's `csl-json` feature is still not enabled.
- Status update (Saturday, September 26, 2026, Agent P2b): the reader now calls the `insert` API. In edit mode Alt+C opens a filtered picker (`picker_entries`, `filter_picker`), asks for a locator (`parse_locator`), inserts `insertion_text` or extends the citation at the caret (`add_to_citation`), and speaks `announce_inserted`. Alt+Shift+D looks a DOI or ISBN up on a background thread with the lookup cache. The palette has insert bibliography, check citations (`commands::check`), and import references (`commands::import`). Word moves onto a citation, and Alt+Shift+K, say `describe_citation`. Continuous reading still reads the citation as written. See `docs/citations.md`.
- Status update (Saturday, September 26, 2026, Phase 1, Agent P1d): `tw convert` formats citations with this crate and adds a References section, so the first update's note on the Pandoc-flavor renderer no longer holds. `tw cite` also has `remove`, `styles`, and `check`.

## Context

Jon writes academic work by keyboard and speech. Citations have to be added, found, inserted, checked, and formatted without looking at anything: every step must be announceable, and every formatted string must read well aloud.

Star (`star/citations.py`, `star/gui/mixin_citations.py`, `star/publish.py`) kept a list of dicts in `settings.json` with eight flat fields (`id`, `type`, `title`, `author` as one "A and B" string, `year`, `journal`, `doi`, `url`, `publisher`). It parsed BibTeX with regular expressions, read and wrote RIS and CSL-JSON, looked up DOIs through the Crossref API and ISBNs through Open Library's Books API, inserted Pandoc `[@key]` markers, and formatted only at publish time, by handing a CSL-JSON export and one of three bundled `.csl` files to Pandoc's citeproc.

## Decision

**Crate.** `textweaver-cite` holds everything; it depends only on `core` (for `CharRange`). The CLI's `tw cite` is a thin wrapper over `textweaver_cite::commands`, which is tested with a temporary library and recorded HTTP.

**Storage: CSL-JSON.** A `Reference` is one CSL-JSON item: the common variables are typed fields; every other variable is kept verbatim (`extra`), so a library written by Zotero, Pandoc, or doi.org loads and saves unchanged. Deserialization is forgiving the way real CSL-JSON is loose (numbers as strings, strings as numbers, date parts as strings, one-element title arrays, `[[null]]` dates). A library is a JSON array saved atomically (temporary file, then rename).

- The **user library** is `references.json` in textweaver's data folder (`Paths::data_dir`).
- A **folder library** is `references.json` beside the documents, the name Pandoc users already pass to `--bibliography`, so a project folder is self-contained. Keys resolve folder first, then user (`Layered`).

**Keys.** Generated keys are the first creator's family name folded to lowercase ASCII plus the year (`doe2020`, `muller2019`, `vangogh1888`), with `a`, `b`, ... for collisions, as Better BibTeX does. They read aloud naturally and are valid Pandoc keys.

**Merging.** Adding or importing updates an existing reference only when it is the same work: same DOI (prefix- and case-insensitive), same ISBN (as ISBN-13), or same key and same title. A different work whose key is taken gets a new key, and the announcement says so ("Key doe2020 was taken, so the new reference is doe2020a"). Star replaced whatever entry had the same key.

**Formats.** BibTeX and BibLaTeX are read with the `biblatex` crate (string macros, `crossref`, month macros, and LaTeX accents resolved) and written by this crate in either dialect, with LaTeX specials escaped and literal `--` kept literal (`-{}-`). RIS is read and written by this crate with real RIS types (`JOUR`, `BOOK`, `CHAP`, `CPAPER`, ...), CRLF line endings as the specification asks, `SP`/`EP` page ranges, `DA` full dates, editors, `SN` split into ISBN or ISSN by length, keywords, abstracts, and wrapped continuation lines. A RIS name without a comma is an organization. Files are recognized by extension, then by content (Star's heuristic).

**Lookup.** Blocking HTTP with `ureq` (rustls), a 15-second timeout, a textweaver User-Agent, and no user data sent. ADR-0001 allows async outside speech, but one request at a time is all a user action needs.

- DOI: doi.org content negotiation for CSL-JSON, which serves Crossref, DataCite, and mEDRA DOIs (Star's Crossref-only API failed DataCite DOIs: datasets, many theses). Crossref labels items with its own types (`journal-article`), which are mapped to CSL types. Markup is stripped from titles (it would be spelled out aloud), bulky Crossref metadata is dropped, a URL that only repeats the DOI is dropped, `5-5` page ranges become `5`.
- ISBN: Open Library edition records (`/isbn/<isbn>.json`, trying the other ISBN form on a miss), then author records, or the work's authors when the edition lists none. Star's endpoint, the Books API (`/api/books?bibkeys=`), answered 404 for every ISBN when this was written. The catalog page is kept as `openlibrary`, not as the book's URL, because styles print URLs.
- Answers are cached as CSL-JSON under the cache folder (`citations/`, at most 500 files, oldest removed), so a repeated lookup works offline.
- Every failure says what happened and that nothing was added: "Could not reach doi.org. You may be offline; nothing was added. Try again when you are connected." Tests use `RecordedClient` (recorded responses in `fixtures/p/recorded`); one `#[ignore]`d test hits the network.

**Formatting: hayagriva.** Styles are the CSL styles bundled with hayagriva (about eighty) or any `.csl` file; dependent styles resolve their parent through the archive. Short names: `apa`, `mla`, `chicago` (author-date), `chicago-notes`, `harvard`, `ieee`, `vancouver`, `ama`, `nature`. hayagriva formats its own `Entry` type; its CSL-JSON input is behind the `csl-json` cargo feature, which the workspace does not enable, so references are converted into hayagriva's model (a journal article is an `article` with a `periodical` parent, a chapter a `chapter` with a `book` parent). Fields hayagriva rejects (a malformed URL or language tag) are dropped one at a time with a log line instead of failing the reference.

- `Formatter::document` formats all of a document's citations together, so numbering, "2020a" disambiguation, and "ibid." are right, and returns one string per citation plus the bibliography of cited works and the missing keys.
- Pandoc prefixes, suffixes, and locators are honored (`[see @a, pp. 33-35; also @b]`), `-@key` drops the author, and in-text `@key [p. 3]` gives "Doe (2020, p. 3)". In note styles, in-text and author-less citations are ordinary notes, as in Pandoc.
- Outputs are plain text, Markdown, and HTML.

**Accessibility of output.** Nothing relies on visual formatting alone. Superscript citation numbers (AMA, Nature) are bracketed in every output (`[1]`, `<sup>[1]</sup>`), so they are not heard as part of the word before them ("reported1"). Plain text carries no markup, entities, or terminal escapes. Missing keys are written into the text ("missing reference smith2020") rather than as a bare `?`. Reference labels for pickers and lists are sentences: "Doe and Roe, 2020. On X. Key doe2020." (Star's `[Doe2020] Doe  (2020)  On X` had brackets and double spaces that screen readers spell out or swallow). Three or more creators are "Doe and others".

**Pandoc citation syntax.** `pandoc::find_citations` finds bracketed and in-text citations with Pandoc's rules (every `;` part holds exactly one key; `@` must follow a space or opening punctuation, so `jon@example.com` is never a citation; `[@x](url)` is a link; a bare number is a page; `{…}` braces delimit keys and locators) and returns byte and character ranges (ADR-0002). `write_citation` writes them back; the two round-trip.

**API for the app and editor** (`insert`):

1. `picker_entries(folder, user)` gives one readable line per reference, folder first; `filter_picker` filters by typed words.
2. `parse_locator("pp. 3-5")` turns a prompt answer into a locator ("12" is a page).
3. `insertion_text(&keys, locator, narrative)` gives `[@doe2020, p. 12]` or `@doe2020 [p. 12]`; when the caret is inside a citation (`pandoc::citation_at`), `add_to_citation` gives the replacement text for its range.
4. `announce_inserted` gives "Inserted citation of Doe and Roe, 2020, page 12."; `describe_citation` reads the citation under the caret: "Citation: see Doe and Roe, 2020, On X, pages 3 to 5; missing reference nobody."

**API for Agent L's Pandoc-flavor renderer.** Find citations with `pandoc::find_citations(markdown)`, load the folder and user libraries, call `Formatter::new(&CitationStyle::resolve(style)?, OutputFormat::Html).document(&cites, &Layered { layers: &[&folder, &user] })`, replace each citation's `range` with `citations[i]` (inside a footnote when `note_style`), and append `bibliography` under a "References" heading. Skip code spans and code blocks before searching.

**CLI.** `tw cite add DOI|ISBN`, `import FILE`, `export --to bibtex|biblatex|ris|csl-json [KEY...] [-o FILE]`, `format [KEY...] --style STYLE [--as plain|markdown|html] [--citation|--bibliography]`, `list [--json]`, `remove KEY`, `styles`, `check FILE`; `--folder DIR` or `--library FILE` choose the library.

## Star compared

Carried over: CSL-JSON as the interchange format; import from `.bib`, `.ris`, `.json`/`.csl` by extension with the content sniffing fallback; ISBN-10 and ISBN-13 checksum validation (Star's test vectors pass); DOI prefixes (`https://doi.org/`, `doi:`) stripped; Pandoc `[@key]` insertion; author-plus-year keys; lossy UTF-8 reading.

Fixed:

- BibTeX parsing: macros, `crossref`, accents, month macros, and quoted values with braces (Star's regex dropped or mangled them); `booktitle` is no longer read as a journal.
- RIS export wrote the CSL type as the RIS type (`TY  - ARTICLE`), which no reference manager accepts; RIS import ignored `DA`, `SP`/`EP`, editors, ISBN, ISSN, abstracts, keywords, and continuation lines.
- CSL-JSON export wrote years as strings in `date-parts` and dropped everything but eight fields; import dropped editors, pages, volume, and issue.
- Authors were one "A and B" string, so organizations and particles ("van Gogh") broke; names are now CSL name objects.
- Importing a different work with a taken key overwrote the existing entry.
- DOI lookup used the Crossref API only; ISBN lookup used an endpoint that no longer answers.
- Network errors were raw exception text; they are now sentences that say nothing was added.
- Formatting happened only through Pandoc at publish time; textweaver formats in-process in any view, and the library is not stored in `settings.json`.

Kept on purpose: none of Star's quirks.

## Consequences

- One Rust crate formats citations without Pandoc; publishing through Pandoc can still use the same `references.json`.
- The conversion to hayagriva's model is where style output can differ from citeproc-js. Known differences:
  - hayagriva 0.10.1's NLM styles (`nlm-citation-sequence`, which hayagriva also calls `vancouver`) drop authors of books and articles; textweaver's `vancouver` therefore uses `elsevier-vancouver`.
  - hayagriva's own mapping of conference papers (an article in proceedings) makes Chicago and MLA print the proceedings title twice as a "special issue"; conference papers are formatted as chapters of the proceedings volume instead, which is how most styles print them.
  - Chicago author-date prints a web page's site name after the author; IEEE prints a stray period after a series title. Both come from the styles as hayagriva applies them.
- Enabling hayagriva's `csl-json` feature (and citationberg's `json`) would let CSL-JSON items be formatted directly, without the conversion; requested at integration.
- BibTeX export drops translators (BibTeX has no field for them; BibLaTeX export keeps them).

## See also

- [Citations](../citations.md): the user guide to `tw cite`.
- [Converting documents](../converting.md#markdown-flavors): Pandoc citations in HTML output.
- [Documentation index](../README.md)
