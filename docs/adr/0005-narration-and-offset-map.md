# ADR-0005: Narration and the OffsetMap

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented as decided. Math is now the first transform in the pipeline (`textweaver_math::speak_text`, ADR-0018), before Markdown residue, the lexicons, abbreviations, and numbers. Audio export maps word timings through the same offset maps, so subtitles show the document's text (ADR-0011), and the syllable display in `textweaver-aids` uses an `OffsetMap` too (ADR-0022).

## Context

Star builds the spoken text and the displayed text separately and aligns them with Python's `difflib` to find which displayed word is being spoken. Its normalization is a chain of regex substitutions that cannot report where anything came from. The inventory (the Star parity reference, kept outside the repository, speech section) found the consequences: the TUI expands abbreviations and numbers but maps engine word positions against the unexpanded text, so the highlight and the saved pause position point at the wrong word after the first expansion; the GUI avoids that by never normalizing during playback at all.

## Decision

Spoken text and its mapping back to the document are **built together**, never aligned after the fact.

**`OffsetMap`** (in core) is a sorted list of spans:

```rust
Span { spoken: Range<u32>,   // bytes in the spoken text
       source: CharRange,    // chars in the document (or in the transform's input)
       kind: Literal | Expanded | Inserted | Elided }
```

- `Literal`: identical text, char for char. A position inside maps to the exact source char.
- `Expanded`: spoken text replaces a whole source token ("Dr." → "Doctor", "$5" → "five dollars"). Any position inside highlights the whole token. An expansion never splits a source token.
- `Inserted`: spoken text with no source ("heading level 2", "row 3, column 2"). It maps to no highlight; its `source` is an empty anchor where reading resumes if paused inside it.
- `Elided`: source that is not spoken (Markdown `**`, skipped code). Its `spoken` range is empty.

**Invariants** (`OffsetMap::check_invariants`, property-tested): spans tile the spoken text in order; every spoken boundary is a char boundary; source ranges never go backwards or overlap; each kind has its shape; literal spans have as many chars as their source range.

**Queries:** `to_source(spoken, bytes)` for highlighting an engine's word event; `to_spoken(spoken, pos)` to start speech at the cursor; `resume_source(spoken, byte)` for pause and resume; `source_extent()`.

**Composition.** Every normalization `Transform` returns `(String, OffsetMap)` mapping its output to its input. `OffsetMap::compose(first, mid, second)` chains them, so after the whole pipeline the map still points into the document. Composition keeps literal spans exact, merges adjacent literals, and fuses pieces of one source token into a single expanded span.

**Narration.** `text::narrate::plan(&Document, range, &NarrationPolicy)` produces sentence-sized `Utterance`s, each with an identity map onto the document, plus `Inserted` spans for structure (heading levels, table coordinates in structured table mode, list positions) according to the policy and verbosity. The speech service then runs the normalization pipeline **per utterance, never across chunks**, so a transform can never pull text from the next sentence (Star's "3:45 today" → "three forty-five AMtoday" class of bug becomes a unit-testable transform error, not an alignment failure).

## Consequences

- The highlight is exact after any normalization, and pause/resume positions are exact.
- Every transform must build its output with `SpokenBuilder` (or produce an equivalent map); a transform that returns a string without a map is not accepted.
- Normalization test vectors from Star's `tests/test_ttstext.py` are ported as expected strings, and each also asserts the map's invariants.

## See also

- [Speech pipeline, step by step](../site/speech-pipeline.html): a worked example of an offset map, `$x^2$` read as "x squared".
- [Math](../math.md): how math is spoken with exact highlighting.
- [Speech engines and voices](../speech.md): normalization settings and engines that normalize natively.
- [Architecture](../dev/architecture.md): the crate map, the threads, and the path from a file to a spoken, highlighted word.
- [Documentation index](../README.md)
