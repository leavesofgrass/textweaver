# ADR-0037: Offline intelligence, part 1: extractive summaries without a model

- Status: accepted
- Date: 2026-09-28
- Builds on: [ADR-0022](0022-reading-aids.md) (the reading aids: difficult words and RSVP), [ADR-0025](0025-lexicon-and-message-catalog.md) (define word), [ADR-0002](0002-text-model.md) (canonical positions)

## Context

Star had a summary command: its `summarize.py` ran sumy's LexRank with an English stemmer and stop words, and printed the top sentences. textweaver had none. Research found no maintained LexRank crate for Rust and judged the method small enough to write in-house (kept outside the repository, "Summaries"). The standing decision is that no machine-learning model is downloaded without approval, so the first part of "offline intelligence" had to work with no model at all.

The same round of work carried two small items that touch the reading aids:

- Difficult words (ADR-0022) were marked and named ("difficult word"), but a reader who wanted to know what the word meant had to stop and use Define Word.
- ADR-0022 left a follow-up: check RSVP at high rates against WCAG 2.3.1, Three Flashes or Below Threshold.

## Decision

### Summaries: a new crate, `textweaver-summary`

`summarize(&Document, k)` returns the `k` most central sentences, in document order, each with its canonical range and its text on one line. It depends only on core and text.

- **Sentences** are the text crate's sentence units, so a summary sentence begins and ends where navigation and speech say it does, and Enter in the list lands exactly on it. Headings, tables, code blocks, image text, and footnote bodies are left out, and so are sentences of fewer than four words.
- **Vectors:** TF-IDF over lowercase words, with a short list of English function words left out and no stemming. The stop list is textweaver's own, 225 words. SCOWL was the plan's first idea, but SCOWL ranks dictionary inclusion, and its smallest size holds everyday content words ("house", "water") a summary is about. Inverse document frequency weighs down whatever common words remain, which also helps in other languages.
- **Graph:** continuous LexRank (Erkan and Radev, 2004): every two sentences are linked, weighted by their cosine similarity, without a threshold. The cosine matrix is never built. With the unit vectors as the rows of a sparse matrix N, the matrix is N N^T, so each step of the power iteration multiplies by N^T and then N, in time linear in the number of words. A first version built the thresholded graph (cosine at least 0.1) from an inverted index; on the benchmark corpus, whose vocabulary is small, nearly every pair was linked, and 2,000 sentences took 500 to 1,000 ms. The factored form ranks 10,000 sentences in a few milliseconds.
- **Power iteration:** damping 0.85, a sentence with no links spreads its score evenly, and the loop stops when the scores move less than 1e-10 in total or after 200 rounds.
- **Ties** go to the earlier sentence, so the same document gives the same summary on every run.

### Limits: sampling long texts

A text longer than 600,000 characters is read in 64 windows spread evenly through it, whole paragraphs at a time, until each window's share is used; a paragraph that begins before its window or is longer than the share is cut, and only whole sentences inside a piece become candidates. At most 50,000 candidate sentences are ranked (more are sampled evenly; the budget makes that rare). `Summary::sampled()` says when either happened; the reader's list introduction says "from samples of this long text", and `tw summarize` says how many characters were read, on standard error.

Measured with `cargo run -p textweaver-summary --release --example summary_bench` on the development machine while other agents were building, 40 runs each after one first run (the first also fills the document's own caches, which the reader has filled already):

- generated plain text, 100 KB: median 10 ms, worst 25 ms;
- generated plain text, 1 MB (read in samples): median 68 ms, worst 91 ms;
- generated plain text, 10 MB: median 70 ms, worst 110 ms, first run 85 ms;
- the benchmark corpus `md-10mb.md` (`cargo xtask bench`): median 85 ms, worst 123 ms, first run 150 ms.

So a summary of 10 MB stays under the 200 ms the plan asked for, in every run measured. The reader summarizes on the input thread, which is acceptable at these times.

### Where it shows

- **`tw summarize FILE [--sentences N] [--json]`** prints the sentences one per line, meaning first, with nothing before them. `--json` adds each sentence's range, line, and score.
- **Summarize** (`summarize`, in the command palette; no key by default) summarizes the selection, else the chapter at the cursor when the document has more than one (section breaks, else level-1 headings, as Next Chapter uses), else the whole document. The list is a `ListModel` list: "Chapter summary, 5 sentences. Enter goes to the sentence and says it."; arrows say "2 of 5" and the sentence; Enter moves the cursor to the sentence and says it.
- **`[summary] sentences`**: how many sentences, 1 to 50, 5 by default.

### Difficult-word definitions

`[reading_aids] difficult_definitions` (off by default). With difficult words marked and the verbosity high, a word move onto a difficult word says "difficult word: " and the first sense of the word from the define-word dictionary (the glossary first, then WordNet), cut to its first clause and at most 100 characters, at a space, with no symbol at the cut. `textweaver-aids` holds the hook (`definitions::difficult_definition`, over any `Definitions` source); the app passes its dictionary. The dictionary file opens quietly on a helper thread on the first difficult word; until it is open, the word move says "difficult word" as before.

### RSVP and WCAG 2.3.1

`textweaver-aids::rsvp::flash` models what can flash. WCAG counts a flash only when a pair of opposing luminance changes covers at least a quarter of a 10-degree field, given as 341 by 256 pixels on a 1024 by 768 screen (21,824 square pixels). The frontends never blank, move, or restyle the panel between words; only glyphs change. The model counts, for each word change, the glyph cells that differ (the word row compared from the fixed pivot, and every grapheme of a changed context word), times the glyph box, times 0.35 of the cell's pixels (room for bold text). A change counts toward a flash when that area reaches the threshold.

The data tests play a worst case (the shortest word "a" and "incomprehensibilities" in turn, then prose) at 1,500 words per minute with every pause at zero, so each word shows for 40 ms:

- the terminal box, with cells up to 16 by 32 pixels (a 24-point terminal font): no change counts, so no flashes;
- the window's panel (a 40-pixel word, 18-pixel context words): no flashes;
- a 200-point word would count at every change, 12 flashes a second; `capped_duration` keeps such a word up for at least 167 ms, and the same test then finds at most three flashes in any second.

So no frontend needs the cap today, and none applies it. A frontend that draws words large enough to count (a future GUI setting for the word size, or a terminal zoomed past 16 by 32 pixel cells) uses `FlashModel::counts` and `capped_duration`.

### Embeddings: left off

Sentence embeddings (all-MiniLM-L6-v2 is about 23 MB; potion-base-8M about 30 MB) would find sentences that say the same thing in different words, which TF-IDF misses. They are left off: each needs a model download, which waits for approval; they would add a tokenizer and inference on RTen to a feature that works well enough without them; and the extractive summary has to stay instant on any machine. If a model is approved later, it slots in as another way to make the sentence vectors, behind the same `summarize` call.

## Consequences

- One new crate with no outside dependency; the lean reader gains a few kilobytes.
- The summary is only as good as sentence segmentation and word overlap: it picks representative sentences, not a rewritten abstract, and a document of short or list-like sentences may have few candidates ("Nothing to summarize" when none is four words long).
- The stop list is English. Other languages still get a summary, because inverse document frequency weighs down their common words, but it is less sharp.
- Difficult-word definitions depend on the dictionary file being installed; without it the word move says "difficult word" as before, and nothing is announced about the missing file.
- The RSVP flash model is an estimate, not a photometric measurement of pixels: glyph coverage is a fixed upper bound, and the terminal's cell size is not known to the reader. A check with a real display is still worth doing at the highest rate.

## See also

- [Reading and moving around](../reading.md#summaries): the Summarize command and `tw summarize`.
- [Reading aids](../reading-aids.md): difficult words and RSVP.
- [Settings](../settings.md): `[summary]` and `[reading_aids]`.
- [ADR-0022: Reading aids](0022-reading-aids.md)
- [Documentation index](../README.md)
