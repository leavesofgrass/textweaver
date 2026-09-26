# ADR-0016: Rendering and bulk conversion

- Status: accepted
- Date: 2026-09-25

## Context

Jon asked for conversion and bulk conversion that are lightning fast, native Rust, and memory safe, even at the cost of custom parsers, with Pandoc only as a fallback. His choices: Markdown flavors GFM, Obsidian, and Pandoc Markdown; LaTeX math as MathML; MiniJinja templates; outputs Markdown, HTML, text, EPUB, DOCX, BRF, and PDF; folders converted by mirroring the tree and skipping outputs newer than their source.

Star converted one file at a time on one thread, preferred Pandoc when installed (inheriting its table and escaping problems, and decoding its output with the Windows ANSI code page), wrote flat output folders with `name (2).md` collisions, and had no HTML renderer of its own (`docs/star-parity.md` Part 1 §1.5, `star/convert.py`, `star/watch.py`).

## Decision

### Rendering (`textweaver-render`)

- **Two engines, one pipeline.** `Engine::PulldownCmark` (default) streams events from the source; `Engine::Comrak` parses a full AST (complete GFM: autolink literals, every table and task-list rule). The comrak AST is walked into the same `pulldown_cmark::Event` stream, so everything after parsing is shared and both engines produce the same HTML for the same input. The choice changes parsing fidelity and speed, never the shape of the output.
- **The shared pass** (`pipeline.rs`), on the event vector of one document:
  - adjacent text events merged, then flavor extensions applied to text runs only (never to code, raw HTML, or link text): GFM autolink literals for the pulldown engine; Obsidian wikilinks, embeds, tags, highlights, and block ids; Pandoc citations, bracketed spans, and intraword sub- and superscript;
  - math events (`$…$`, `$$…$$`, parsed natively by both engines) rendered to MathML by `pulldown-latex`, with the LaTeX as an `application/x-tex` annotation; a formula that fails, or panics inside the renderer, falls back to its source in `<code class="math-error">`;
  - heading ids (GitHub-style slugs, unique per document; explicit Pandoc `{#id}` honored), collected into a table of contents;
  - footnotes gathered into one `<section role="doc-endnotes">` with a "Footnotes" heading, numbered by first reference, with `role="doc-noteref"` references and back links labelled "Back to reference n";
  - pulldown-cmark's HTML writer, then optional `ammonia` sanitization that keeps MathML, ARIA, ids, and classes.
- **Block extensions neither engine has** (callouts and alerts, Pandoc fenced divs) are rewritten at the source level into raw HTML blocks around ordinary Markdown, which both engines pass through while still parsing the inside. Callouts become `<div class="callout" role="note">` with a title, or `<details>` when foldable. Code fences are never touched.
- **Front matter** is read by a small, safe YAML subset reader (maps, sequences, scalars, block scalars; no anchors or tags) into JSON values; malformed front matter stays text and never fails a document. Pandoc title blocks are read in the Pandoc flavor.
- **Templates** are MiniJinja with HTML auto-escaping everywhere. Built-ins: `default` (language, skip link, `main` landmark, table of contents in a labelled `nav`, a stylesheet honoring `prefers-color-scheme`, `prefers-contrast`, and `prefers-reduced-motion`), `print`, and `fragment`. User templates come from a folder or a path and may extend the built-ins.

### Conversion (`textweaver-convert`)

- **Plan, then run.** Inputs are expanded into jobs: files map to `<out>/<stem>.<ext>`; folders are walked recursively, hidden entries and the output folder skipped, and mirrored under `--out`. Without `--out`, outputs land beside their sources and files that already have the output extension are treated as outputs. A job whose output is its own source, or an output claimed by two sources, is rejected with a reason before anything runs.
- **Skip unchanged.** A job is skipped when its output's modification time is at or after its source's, unless `--force`. These checks run on the calling thread before the pool starts (parallel metadata calls contend on NTFS; see measurements).
- **Parallel.** Remaining jobs run on a `rayon` pool (`--jobs`, default every core). Each worker loads, converts, and writes one document at a time; results are small records in plan order.
- **Per file.** Markdown sources are rendered from their text (HTML), copied (Markdown), or loaded by `MarkdownLoader` (text, writers). Other sources load through the `textweaver-formats` registry; HTML output then goes through `formats::to_markdown` and the renderer, keeping the source's title, language, and author. Formats with no native loader go to Pandoc when it is installed, with its output decoded as UTF-8. Every failure is caught per file (including panics), so one bad file never stops a batch.
- **Atomic writes.** Outputs are written to a hidden temporary file beside the target and renamed, so an interrupted run never leaves a partial output whose fresh timestamp would make the next run skip it.
- **Writers seam.** EPUB, DOCX, BRF, and PDF go through `DocumentWriter` (`write(&Document, &WriteOptions, &mut dyn Write)`), the shape agreed for Agent M's `textweaver_writers::Writer`. Until writers are registered, those formats are refused before a batch starts with a sentence that says so.
- **Hot folder.** `watch()` keeps Star's `watch_*` semantics: convert what is present at start, then new files once their size holds still for `stable` (2 s) and they open; move sources to `processed/` (or keep them) and failures to `failed/`, never overwriting; log each attempt with a UTC timestamp to `<out>/textweaver-watch.log`. Events come from `notify` with a periodic rescan. Deliberate difference: an output with the same name is replaced atomically, where Star wrote `name (2).md`.

### Output for listening

`tw convert` prints one summary sentence ("Converted 12 files to HTML in 0.4 seconds, 30 files per second. No failures."), one line per failure, per-file lines only with `--verbose`, and JSON with `--json`. The exit status is 1 when any file failed.

## Measurements

CommonMark 0.31.2: both engines pass all 652 spec examples through the full pipeline (`tests/commonmark.rs`), with the same normalization pulldown-cmark's own suite uses.

Benchmark (`cargo run --release -p textweaver-convert --example bench_convert`): a generated corpus of 1,000 Markdown files, 12.1 MB (front matter, headings, emphasis, links, tables, code, task lists, footnotes, inline and display math), converted to HTML with the default template and `--force` on a 12-thread machine. Other agents' builds kept the host at 97 to 100 percent CPU during every measurement, so all numbers are a floor and vary between runs.

| Environment | Files per second, median (best) | Unchanged rerun | Peak memory |
|---|---|---|---|
| Linux container, pulldown, 12 threads | 1,938 to 3,177 (best 3,439) | 1,000 skipped in 13 to 20 ms | 13.4 MB resident |
| Linux container, comrak, 12 threads | 2,188 (2,976) | | 21.4 MB |
| Linux container, pulldown, 1 thread | 787 (944) | | 8.3 MB |
| Windows 11, NTFS, pulldown, 12 threads | 452 (679) | 1,000 skipped in 0.21 s | 15.4 MB working set |

Memory stays proportional to the documents in flight, not to the batch: in the container 11.7 MB for 100 files, 13.4 MB for 1,000, 19.0 MB for 4,000; on Windows 8.8, 15.4, and 19.2 MB. The growth is the per-file result records. (Linux: `VmHWM`; Windows: peak working set sampled every 5 ms from outside the process.)

Profile (`--profile`, one thread, per file, Linux container): reading 0.08 ms; rendering with the GFM pass 0.78 ms, of which math 0.07 ms and heading ids 0.02 ms, against 0.51 ms for parsing and writing alone; template 0.11 ms; atomic write 0.73 ms (plain write 0.38 ms). On Windows the file system dominates: reading 0.13 ms, rendering 0.32 ms, template 0.04 ms, but a plain write about 1 ms and the atomic write (create, write, rename) about 1.8 to 2.4 ms, largely antivirus scanning on file close. Parallel workers hide most of it.

Hot-path changes from profiling: text runs are merged in one linear pass and only when a flavor extension needs them; up-to-date checks run serially before the pool starts (on NTFS, 12 threads issuing metadata calls at once were 3.5 times slower than one thread); planning compares paths lexically instead of canonicalizing each one (canonicalizing cost more than converting on Windows).

## Alternatives considered

- **Two separate renderers** (pulldown-cmark's writer and comrak's formatter, with extensions implemented twice): rejected; output would differ between engines and every accessibility fix would be made twice.
- **comrak only:** complete GFM, but it allocates a full AST per document; pulldown-cmark streams and is faster, so it stays the default.
- **Pandoc for everything:** rejected by Jon's direction (speed, native code) and by Star's experience (tables, escaping, encoding).
- **KaTeX or MathJax output:** needs JavaScript or fonts at reading time; MathML is native in browsers and read by screen readers.
- **Direct (non-atomic) writes:** about half the write cost per file on both systems, but a crash would leave a partial output newer than its source, which the next run would skip. Correctness wins.
- **Parallel up-to-date checks:** measured 3.5 times slower than serial on NTFS with 12 threads.

## Consequences

- One event pipeline to maintain; new flavor features land once and work with both engines.
- Inline extensions work on merged text runs, so a construct split by emphasis (for example `[[a *b* c]]`) is left as written; block ids are recognized on paragraphs, not on tight list items.
- Embeds inline a note's text only when the embed stands alone in its paragraph and the note is found under the input folder; otherwise they are links.
- The `tw convert` command depends on `textweaver-convert` and `textweaver-render`; the CLI's manifest gains both.
- EPUB, DOCX, BRF, and PDF output become available when Agent M's writers are registered in `Writers::builtin()`.
