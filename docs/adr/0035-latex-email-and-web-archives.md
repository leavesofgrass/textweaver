# ADR-0035: Native LaTeX subset, and email and web archives

- Status: accepted
- Date: 2026-09-28 (Monday, September 28, 2026; Agent W5c3, Wave 5)

## Context

Students get course notes as LaTeX, instructors' announcements as saved emails, and reading as web pages saved from a browser. Until now:

- LaTeX opened only through Pandoc, and only in `tw convert`. The reader never runs a subprocess to open a file, so a student had to convert first, and Pandoc is a large separate install.
- Email (`.eml`) and web archives (`.mhtml`, `.mht`) did not open at all.
- MathML in a web page was read as a run of letters ("x2+1"); only EPUB 3 chapters read it as math (ADR-0029).

The research for Wave 4 (`docs/research/wave4.md`) and the Wave 5 plan (`docs/research/wave5-plan.md`, section 2.3) chose: our own LaTeX tokenizer, with mitex's grammar as a design reference only (no release since June 2024), our own math parser for the formulas (ADR-0018), and no tectonic (C libraries) or unicodeit (LPPL); mail-parser for MIME.

## Decision

### LaTeX: our own subset

`crates/textweaver-formats/src/latex.rs`, for `.tex`, `.latex`, and `.ltx`, at the native loaders' priority, so Pandoc is never chosen for them.

**How it reads.** A lexer cuts the source into tokens. It reads math (`$...$`, `$$...$$`, `\(...\)`, `\[...\]`, and the math environments), code (`\verb`, `verbatim`, `lstlisting`, `minted`), and `\url` raw, so their contents are never taken for commands. An iterative parser then reads the tokens with an explicit stack of groups, environments, and arguments. A command's argument is put back in front of the stream between two markers of the parser's own, so nothing is recursive and no input can overflow the thread's stack.

The parser runs twice. The first pass learns what the reader needs before it reaches it: the number each `\label` names, each float's caption, and which tables have a header row. The second writes the canonical text.

**What is read:**

- `\part` to `\subparagraph` as headings. Sections are numbered as LaTeX numbers them ("2.1 Methods"), and starred ones are not; `\appendix` switches to letters. With chapters, a chapter is level 1 and a section level 2; without, a section is level 1.
- The title block (`\title`, `\author`, `\date`) at `\maketitle`, and in the document's metadata. The language from babel's or polyglossia's main language.
- `abstract`, and `thebibliography` as a numbered list under "References".
- `itemize`, `enumerate` (labeled `1.`, `(a)`, `i.`, `A.` by depth, or `\item[label]`), and `description` (the term in bold).
- `tabular` and its relatives (`tabular*`, `tabularx`, `longtable`, `array`) as tables. A first row followed by a rule (`\hline`, `\midrule`) is the header row. `\multicolumn` and `\multirow` read their text. A table inside a table's cell is running text, as in HTML.
- `table` and `figure` floats: the caption is a line of its own, "Table 1: Cell counts"; a table's marker is labeled with it, and a figure's caption is its picture's description, under an `Image` marker that names the file.
- Theorem environments declared with `\newtheorem` ("Theorem 1 (Euclid)."), and `proof` ("Proof.").
- `quote`, `quotation`, and `verse` as quotes.
- Math, as the Markdown loader writes it: LaTeX with its delimiters under a `Math` marker, level 1 for display math on its own line. `align`, `gather`, `multline`, `flalign`, `alignat`, and `eqnarray` become the `align` and `gather` environments the math parser knows. `\label`, `\tag`, `\nonumber`, and `\notag` are taken out; numbered environments are numbered per line, and the numbers follow the formula, "(1), (2)".
- `\ref`, `\eqref`, `\autoref`, `\cref`, and `\pageref` read the number (`\autoref` with its kind, "Section 2"). A reference to a section is a link to its heading, which following a link already knows. A key with no `\label` reads as the key.
- `\cite` and its natbib and biblatex relatives become Pandoc citations: `[see @doe2020; @roe99, p. 12]`, and `@doe2020` for `\citet` and `\textcite`. Reading says them as it says citations in Markdown (`[reading] citations`), and `tw convert` formats them.
- `\footnote`, placed as `LoadOptions::footnotes` says, as in the other loaders.
- `\emph`, `\textbf`, `\underline`, `\sout`, and `\texttt` as markers, and the switches (`\bf`, `\itshape`, `\ttfamily`, ...) to the end of their group. `\url` and `\href` as links. Accents (`\'e`), special characters (`\%`), TeX's dashes and quotes, and the usual named symbols.
- Macros: `\newcommand`, `\renewcommand`, `\providecommand`, and `\def` **without arguments** are expanded, in text and, written back as LaTeX, in math (`\R` for `\mathbb{R}`); `\DeclareMathOperator` in math. Macros with arguments are not expanded; they count as unknown commands.
- `\input`, `\include`, `\subfile`, and `\import` read `.tex` files **from the document's folder or below it**. The path must be relative with no `..`, and the resolved file, symbolic links followed, must still be inside the folder. A file from bytes, or inside an archive, has no folder, so nothing is included. Each file skipped is named in the warnings with the reason.

**What is skipped, and how it is announced.** The preamble's text is never read. Layout commands (spacing, page breaks, fonts, colors, counters, package options) are left out silently. Drawings (`tikzpicture`, `picture`) and `comment` are left out whole. Any other command is left out and its arguments read as text, so no words are lost. The document's warnings name the unknown commands and environments ("Some LaTeX commands are not supported, so only their text is read: \hl."). With the new `LoadOptions::name_skipped_commands`, each is also said where it was, "(command hl)". The reader should set it at high verbosity and leave it off otherwise, as it chooses `revisions`; that is one line in the app, requested of the orchestrator.

**Limits**, each a constant in `latex.rs`:

- 16 MB for the document or one included file, 64 MB of included files in all;
- tokens read and made in one pass: 8 for each byte of source (the document and the files it includes) plus 50,000, at most 2,000,000; parser steps: 32 for each byte plus 100,000, at most 8,000,000 (every token taken counts, so arguments put back cost too). Scaling with the source keeps a small file that expands without end cheap: the first fuzzing run, with a fixed budget, spent about half a second on each such input;
- 10,000 macro expansions;
- 256 levels of groups, environments, and arguments (`MAX_NESTING`); deeper ones are read without structure, with the nesting warning;
- 8 levels of `\input`, 64 included files;
- 64 KB for one formula (a longer one is read as text).

Past a limit the rest is left out and the document says so. A file over 16 MB, or one that is not text, is refused.

### Email and web archives: mail-parser

`crates/textweaver-formats/src/eml.rs`, on mail-parser 0.11.9 (Stalwart Labs, Apache-2.0 or MIT, `forbid(unsafe_code)`, fuzzed upstream) with its `encoding_rs` feature, so legacy charsets decode through the encoding_rs the crate already uses. mail-parser parses MIME, decodes transfer encodings and encoded words, and finds a message's text, HTML, and attachment parts; it keeps its own stack for multipart nesting and parses attached messages at most three deep. Its one new dependency is hashify, a small procedural macro crate (Apache-2.0 or MIT).

**Email** (`.eml`): the subject is a level-1 heading; then one line each for From, To, Cc, and Date, with names before addresses, "Ada Example (ada@example.org)", and the date written out with its weekday computed from the date and its time zone in words ("Monday, September 28, 2026, 10:15, UTC minus 7"). The body is each `text/plain` part as paragraphs, lines kept, with quoted lines (`> ...`) under a `Quote` marker without their marks; a message with no plain text is read through the HTML loader instead. Attachments are listed by name and size under an "Attachments" heading and never opened; an attached message is listed by its subject.

**Web archives** (`.mhtml`, `.mht`, RFC 2557): the root part is the one the `multipart/related` part's `start` parameter names, else the first HTML part. It is read through the HTML loader, which gained a resolver hook for this: `cid:` addresses resolve to their part, and relative addresses to absolute ones against the root's `Content-Location`, so a link leads to the page on the web it named, not to a missing local file, and a picture with no alternative text of its own is described by its part's `Content-Description` (never one marked decorative with `alt=""`).

**Limits:** 128 MB for the file; at most 10,000 parts, nested at most 32 deep (walked without recursion), else the file is refused with a sentence; 64 MB of body text, past which the rest is left out and the document says so.

### MathML in web pages

The HTML loader now reads `<math>` as math in every page, as W4c1's EPUB path does: LaTeX with its delimiters under a `Math` marker, from the page's TeX annotation or the presentation MathML (`mathml.rs`). `epub:switch` stays EPUB's. Web pages fetched by address and web archives get it too.

While here: a picture's alternative text no longer runs into the words beside it ("a cell A nucleus", not "a cellA nucleus").

### Messages

One open-failure message per format, in all six languages: `opening-damaged-latex`, `opening-damaged-email`, and `opening-damaged-mhtml`, said after "Could not open NAME:".

### Fuzzing

Two cargo-fuzz targets, `latex` (with skipped commands named, and with code skipped and notes inline) and `eml` (every input as an email and as a web archive), seeded from `fixtures/c3`, in the nightly matrix. Hostile-input tests in `crates/textweaver-formats/tests/c3.rs` cover macro bombs, unclosed groups, arguments, and math, deep nesting, and broken MIME.

## Consequences

- LaTeX, email, and web archives open in the reader and in `tw` with nothing installed. Pandoc is still registered for `tex` in `tw convert`, at its lower priority, so it is never chosen.
- `CANONICAL_VERSION` is 6: HTML with MathML reads differently, so cached documents are loaded again once.
- `LoadOptions` has a new field, `name_skipped_commands`; it is part of the cache key.
- Not read: macros with arguments, `\newenvironment` bodies, BibTeX files named by `\bibliography` (citations stay Pandoc citations, which the libraries resolve), `\pageref` page numbers (it reads the label's number), equation numbers per section in books without chapters, and TeX primitives beyond `\def` and `\let`. A LaTeX document that relies on them reads its text, with the commands named.
- A deliberately unbalanced document (thousands of unclosed arguments) costs time up to its step allowance: under a second for a 50 KB file in a debug build (the fourteen hostile inputs in `tests/c3.rs` take 2.8 seconds together), far less in release.
- An email's attachments are listed, not opened; opening one needs the attachment saved first.

## Alternatives considered

- **Pandoc for LaTeX in the reader:** rejected again (ADR-0031): the reader never runs a subprocess to open a file.
- **mitex-parser:** a good rowan grammar, but no release since June 2024, and it parses math into its own tree where we have ours. Kept as a design reference.
- **tectonic:** a real TeX engine, but C libraries and a large download; unicodeit is LPPL.
- **A recursive-descent LaTeX parser:** simpler to write, but a hostile file could overflow the stack; the explicit stack costs little.
- **Expanding macros with arguments:** real LaTeX, but the expansion rules (`#1`, delimited parameters, `\expandafter`) are a TeX engine's job, and the risk to a reader is high. Reading the arguments as text keeps the words.
- **Writing our own MIME parser:** mail-parser is safe Rust, fuzzed, and decodes the charsets and encodings an email really uses.

## See also

- [Converting documents](../converting.md): the formats textweaver reads.
- [ADR-0018: Math](0018-math.md): the math parser formulas go to.
- [ADR-0029: MathCAT speech](0029-mathcat-speech.md): MathML in EPUB.
- [ADR-0031: Native RTF, ODT, and Word revisions](0031-native-rtf-odt-and-word-revisions.md): the limits and patterns this follows.
- [Documentation index](../README.md)
