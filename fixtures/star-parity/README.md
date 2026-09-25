# Star parity fixtures

These JSON files record what **Star** (the Python reader that textweaver
reimplements) produces for the three textweaver fixtures. Use them as
reference output when checking that textweaver matches Star, or when you
deliberately choose to differ from it.

| File | Source fixture |
|---|---|
| `sample.txt.json` | `fixtures/sample.txt` |
| `sample.md.json` | `fixtures/sample.md` |
| `sample.html.json` | `fixtures/sample.html` |

## How they were generated

- **Star version:** 0.1.31 (`D:\star`, `pyproject.toml` `version = "0.1.31"`)
- **Star commit:** `e4b4ef83355ebe4d9307c4319f6334bd1ce0936f` (the `release: 0.1.31` commit; the working tree was clean)
- **Generated:** Friday, September 25, 2026 (2026-09-25)
- **Python:** 3.11.15, standard library only. No optional Star dependencies were installed.
- **Pandoc on PATH:** pandoc 3.9.0.2. This matters for HTML; see below.
- **Script:** `tools/star_parity_export.py`. To regenerate, run `python tools\star_parity_export.py`.

The script imports Star read-only from `D:\star` and does not write bytecode
there. It redirects `star.settings.SETTINGS_FILE` to a file that does not
exist, so every setting takes Star's built-in default (`star/settings.py`
`DEFAULTS`) and the user's `settings.json` is ignored. It also sets
`document_cache=False`, so Star never reads or writes its document cache.
Neither change affects the output.

Default settings that affect the output:

| Setting | Default |
|---|---|
| `tts_skip_code` | `True` |
| `table_reading_mode` | `"structured"` |
| `footnote_mode` | `"inline"` |
| `prefer_pandoc` | `True` |
| `expand_abbreviations` | `True` |
| `normalize_numbers` | `True` |
| `normalize_math` | `True` |
| `use_pronunciations` | `True`, with an empty lexicon |

## Fields

All offsets count **Unicode code points** (Python `str` indices), and every
end offset is exclusive. The `*_utf8` arrays give the same positions as
**UTF-8 byte offsets**, for Rust.

| Field | Meaning | Star function used |
|---|---|---|
| `title`, `format`, `markdown` | Document title, detected format, and Star's intermediate Markdown | `star.documents.load_document` (`star/documents/dispatch.py`) |
| `plain_text` | Star's canonical TTS text (`Document.plain_text`) | `load_document` → `star.ttstext._strip_markdown_for_tts` |
| `word_tokens` | `[start, end, text]` for each word | `star.documents.model._WORD_TOKEN_RE` = `\b\w[\w'-]*`, the tokenizer behind `_build_word_map` |
| `sentence_starts` | Character offsets where sentences start: `[0]` plus the end of every separator match | `star._runtime._SENTENCE_SPLIT_RE`, following `DocumentMixin._build_sentence_map` in `star/tui/mixin_document.py` |
| `sentence_start_words` | The sentence map Star actually navigates with: word indices with duplicates removed | Star's own `DocumentMixin._build_sentence_map`, run unchanged on a stub |
| `sentence_start_word_chars` | `tts_offset` of each entry in `sentence_start_words` | derived |
| `paragraph_starts` | `[0]` plus the end of every `\n{2,}` run in `plain_text` | derived; see note 1 |
| `normalized` | For each sentence: `{start, input, normalized}` | `star.ttstext._preprocess_tts_text` with default settings |
| `normalized_full_text` | `_preprocess_tts_text` applied to the whole `plain_text`. The TUI does this to its slice when it plays from word 0. | same |
| `tui_view` | The document laid out the way the TUI draws it at wrap width 78 (an 80-column terminal): display lines, paragraph-start lines and words, heading lines, table lines, and the word map | `star.render.render_markdown`, `star.documents.model._build_word_map`, `star.tui.theming._HEADING_ROLES` / `_TABLE_ROLES` |
| `variants` | HTML only; see below | |

Notes:

1. **Paragraph starts are derived, not taken from a Star function.** Star has
   no plain-text paragraph splitter. `_strip_markdown_for_tts` separates
   paragraphs with `"\n\n"`, and the third alternative of
   `_SENTENCE_SPLIT_RE` treats `\n{2,}` as a boundary, so `paragraph_starts`
   follows that same rule. Star's actual paragraph navigation works on display
   lines instead: in the TUI, blank lines separate them (`tui_view`); in the
   GUI, empty Qt text blocks do.
2. **The `normalized` field works one sentence at a time. Star does not.**
   The TUI normalizes the whole slice from the start word to the end of the
   document (`star/tui/mixin_playback.py:102`), and the GUI does not
   normalize continuous playback at all (`star/gui/mixin_playback.py:90-108`).
   Speech Cursor mode normalizes one display line at a time. Any regex that
   crosses a sentence boundary can therefore give a different result in Star.
   `normalized_full_text` is included so you can compare the whole-slice
   result.
3. **The per-sentence text is `plain_text[start:next_start].strip()`,** with
   empty pieces skipped.

## HTML: which route is "Star's"

With the default `prefer_pandoc=True` and a Pandoc binary installed, Star
converts HTML **with Pandoc** (`star/documents/dispatch.py:170-177`,
`star/documents/pandoc.py:48-67`). The top-level fields of
`sample.html.json` come from that route, using Pandoc 3.9.0.2. Other Pandoc
versions may produce different Markdown.

`variants` records two alternatives:

- **`native_prefer_pandoc_false`**: Star's own loader
  (`HTMLHandler` → `_load_html` → `_HTML2MD`). **The result is empty.**
  `_HTML2MD` lists the void elements `meta`, `link` and `base` in `_SKIP`
  (`star/documents/html.py:8-26`) and increments a skip counter on their
  start tag (`:53-55`). Void elements never get an end tag, so after
  `<meta charset="utf-8">` the counter stays above zero and everything that
  follows is skipped. This is what Star does on a machine without Pandoc, or
  when `prefer_pandoc` is off.
- **`diagnostic_html2md_void_fix_NOT_STAR_BEHAVIOR`**: **This is not Star
  output.** It is `_HTML2MD` with `meta`, `link` and `base` ignored, run
  through the rest of Star's pipeline unchanged. It shows the canonical text
  the native converter is meant to produce, including its other quirks: the
  title appears twice, the "Oranges" bullet is lost, `<caption>` is dropped,
  and the image runs into the block quote.

## What could not be exported

- **GUI (Qt) paragraph, heading and table navigation data.** These depend on
  `QTextDocument` blocks, and PyQt6 is not installed in this Python. Only the
  TUI line model is exported.
- **The EPUB/DAISY chapter list.** The fixtures contain no EPUB. Star also
  never fills in chapter word indices (they are always `0`).
- **A true per-sentence equivalent of how Star speaks.** Star never
  normalizes one sentence at a time (note 2), so `normalized` is an
  approximation of speak-time output.
