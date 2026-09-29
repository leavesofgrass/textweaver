# ADR-0025: Define word offline, and the interface's message catalog

- Status: accepted
- Date: 2026-09-26
- Status update (2026-09-28): the subset was enough for Spanish, French, German, Portuguese, and Arabic, so `fluent-bundle` was not adopted. The whole interface now uses the catalog; see [ADR-0030](0030-interface-translations.md).

## Context

Star defined words through nltk's WordNet and CMUdict corpora, which a user had to download with Python tools, and layered a JSON glossary over them (`star/dictionary.py`). It translated its Qt chrome with a small `tr()` over JSON catalogs keyed by the English text (`star/i18n.py`). Both were planned in pure Rust (the pure-Rust research, kept outside the repository, "Define word"), offline, with no Python, and interface translations were planned to start with a message catalog, English complete, a pseudo-locale, and a right-to-left check.

## Decision

### Define word

**A new crate, `textweaver-lexicon`,** looks a word up in the user's glossary, then in Open English WordNet 2025 (CC BY 4.0), with pronunciations from CMUdict (BSD-style), all from one data file shipped beside the program.

- **Data file:** `lexicon-en.twlex` (9,988,663 bytes), built once by `tools/build_lexicon.py` from the two downloads, checked by SHA-256, and reproducible. Headwords (223,607: WordNet's lemmas, the exception lists' inflected forms, and CMUdict's words) are keys of an `fst` 0.4 map. Each value holds the headword record's number and flags for the parts of speech it has senses in, so morphy's candidate forms are tested without unpacking anything. Headword records and synset records are packed into zstd frames of about 48 KiB, read with the pure-Rust `ruzstd` 0.9. Records are bounds-checked, and the map and metadata carry a checksum, because `fst` trusts its input. The format is described in `crates/textweaver-lexicon/src/data.rs`.
- **Compression:** `ruzstd` implements only its fastest level, which packs this text about 1.6 to 1. The build plugs a hash-chain match finder with lazy matching into `ruzstd`'s encoder through its `Matcher` trait, for standard zstd frames about a fifth smaller. It also steers around two `ruzstd` 0.9 encoder bugs: match lengths over 65,538 are written wrongly, and a block whose sequences all have no literals panics. Both are worth reporting upstream.
- **Locality:** synsets are numbered word by word, the words with the most senses first, so `run`'s 57 senses sit in one or two blocks. A lookup unpacks one to three blocks: about a millisecond the first time, microseconds after that (16 blocks are kept per store).
- **Morphy:** a port of WordNet's and NLTK's: the exception lists, then the suffix rules, keeping forms WordNet has in that part of speech. `running` finds `running` (noun) and `run` (verb).
- **Pronunciation:** CMUdict's ARPAbet, stored one byte per phone, respelled for reading aloud with the stressed syllable in capitals (`RUN-ing`, `buh-NAN-uh`), since ARPAbet read by a speech engine is noise.
- **Glossary:** Star's JSON format, or a text file of `term: definition` lines, looked up first, through the base forms too. Its senses come before WordNet's rather than replacing them.
- **In the reader:** Ctrl+Shift+D (GUI) or Alt+E (terminal; Windows Terminal takes Alt+Shift+D) lists the senses for the selection or the word at the cursor; Enter copies a sense. `tw define` prints Markdown or JSON.

### Message catalog

**A small catalog in the same crate (`textweaver_lexicon::i18n`), reading a subset of Project Fluent's syntax,** instead of the `fluent-bundle` crates:

- no new dependencies (`fluent-bundle` brings `intl-memoizer`, `unic-langid`, `intl_pluralrules`, and `self_cell`);
- a `Catalog` is a value passed around in an `Arc`, not global state, so tests render several languages at once;
- messages have ids (`define-not-found`), not English keys, so a changed English sentence does not orphan its translations;
- the files are valid Fluent, so a later wave can switch to `fluent-bundle` without rewriting them.

The subset: messages, terms, comments, multiline text, variables, message and term references, string literals, and one-line select variants on a number (exact values and CLDR plural categories for English, the Romance languages, German, Arabic, Hebrew, and Persian). Values put into messages are isolated with U+2068 and U+2069 in right-to-left languages.

- `en` is complete: a test scans the workspace's sources for message ids and fails on any missing from `locales/en.ftl`, and on any message no code uses.
- `en-XA` is the accented pseudo-locale: every message comes out accented, a third longer, and in `⟦ ⟧`, so an unbracketed string in the interface was never looked up. The app's tests render define word, profiles, and statistics in it.
- `ar-XB` is the right-to-left pseudo-locale: text in right-to-left overrides, values isolated. `bidi_problems` checks that a rendered string closes every direction mark it opens.
- `[interface] language` chooses the language; `<language>.ftl` in the settings folder's `locales` adds one, falling back to English message by message.

Only the study features' messages are in the catalog so far. Moving the rest of the interface's strings in is future work.

## Consequences

- Define word works with no network and no Python, from a 10 MB file, which packages carry in `lexicon/`. WordNet's notice and CMUdict's licence go with it (`THIRD-PARTY-NOTICES.md`, `licenses/lexicon/`).
- The data file is in git. Rebuilding it changes 10 MB of history; that should happen only for a new WordNet or CMUdict release.
- A glossary edited while textweaver runs is read again on the next lookup.

## Fallbacks

- If the data file is missing, define word searches the glossary alone and says the dictionary is not installed.
- If `ruzstd` stays at its fastest level, the build keeps the custom match finder; if a real encoder level lands, the match finder can go.
- If a language needs more than the Fluent subset (attributes, functions, number formatting), switch to `fluent-bundle`; the catalogs stay as they are.

## See also

- `third_party/lexicon/README.md`: the sources, sums, and licences.
- [Reading guide: define a word](../reading.md#define-a-word-ctrlshiftd-or-alte)
