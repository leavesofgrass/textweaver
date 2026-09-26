# Define word: Open English WordNet and CMUdict

textweaver's offline dictionary for define word (`textweaver-lexicon`), derived from two sources. Downloading both, from their official GitHub locations only, was approved on Saturday, September 26, 2026, and they were downloaded that day.

## Sources

### Open English WordNet 2025

- Project: https://github.com/globalwordnet/english-wordnet, the Open English WordNet team. It is based on Princeton WordNet 3.1.
- Release: `2025-edition` (published 2025-12-31), the WNDB (WordNet database) files.
- Download: https://github.com/globalwordnet/english-wordnet/releases/download/2025-edition/english-wordnet-2025.zip, 9,618,697 bytes, SHA-256 `73355e48f8117a24ca9ebc23ed75b35434e6cd21cc9dd3984e80aff5a5f63636` (the same sum GitHub lists for the asset).
- Licence: Creative Commons Attribution 4.0 International (CC BY 4.0), https://creativecommons.org/licenses/by/4.0/, with Princeton's WordNet notice. The notice at the head of every data file must go with every copy of the database, including changed copies; it is in `WORDNET-LICENSE` here, word for word, and in `licenses/lexicon/` in each package.
- Size: 107,519 synsets and 127,311 headwords, with 4,411 inflected forms in the exception lists.

### CMUdict (the CMU Pronouncing Dictionary)

- Project: https://github.com/cmusphinx/cmudict, Carnegie Mellon University.
- Commit: `74790861f652b15e4ac49015a90074ad62a27690` (2025-10-24), the newest on `master` when downloaded.
- Download: https://raw.githubusercontent.com/cmusphinx/cmudict/74790861f652b15e4ac49015a90074ad62a27690/cmudict.dict, 3,618,488 bytes, SHA-256 `81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22`.
- Licence: a BSD-style licence (two clauses), copyright 1993-2015 Carnegie Mellon University. The text is `CMUDICT-LICENSE` here, from the `LICENSE` file at the same commit (1,754 bytes, SHA-256 `bd4ce8e44170a5f9f481310ca85c51de3c4f851a65e679b40e603b143bd3542a`).
- Size: 126,052 words with 135,166 pronunciations in ARPAbet.

## Files

- `lexicon-en.twlex`: the derived data file, 9,988,663 bytes, SHA-256 `5c808a487b66b4e48319abfb9a598f62019caa4e9fbba269bd6b24af0bc2ed6e`. It holds 223,607 headwords: WordNet's, the exception lists' inflected forms, and CMUdict's words. Its sections:
  - metadata (the sources above, as JSON): 674 bytes;
  - the headword map (`fst`): 1,817,580 bytes;
  - headword records (pronunciations, senses by part of speech, base forms): 2,129,034 bytes, 3,349,541 unpacked, in 69 blocks;
  - synset records (words, definition, examples, opposites, "kind of"): 6,041,295 bytes, 12,123,865 unpacked, in 247 blocks.
- `WORDNET-LICENSE`: the WordNet notice, as described above.
- `CMUDICT-LICENSE`: CMUdict's licence.

## How it was derived

Exactly what `tools/build_lexicon.py` does; running it again produces the same bytes (`--check` confirms it).

1. Download both files above and check their SHA-256 sums.
2. Parse WordNet's `data.*`, `index.*`, and `*.exc` files. Keep, for each synset: its words (underscores as spaces, adjective markers such as `(a)` removed), its definition, its examples (the quoted parts of the gloss), the first word of each opposite (`!` pointers) and of each hypernym and instance hypernym (`@`, `@i`, shown as "a kind of"). Keep each headword's synsets per part of speech in WordNet's sense order, and the exception lists. Everything else (other pointers, verb frames, lexicographer files, sense keys) is left out.
3. Parse CMUdict: each word's pronunciations, in order; comments are dropped.
4. Write the data file (the format is described in `crates/textweaver-lexicon/src/data.rs`). ARPAbet is stored one byte per phone. Records are packed into zstd frames of about 48 KiB each.

**Modification notice (CC BY 4.0, section 3(a)(1)(B)):** the WordNet data was changed as described in step 2: selected fields were extracted, reformatted, and compressed; no definitions or examples were reworded.

## How textweaver uses it

Define word (Ctrl+Shift+D in the GUI, Alt+Shift+D in the terminal, and `tw define`) looks a word up in the user's glossary first, then here. Inflected words reach their base forms through morphy, a port of WordNet's and NLTK's: `running` finds `run`, `geese` finds `goose`. Pronunciations are respelled for reading aloud, with the stressed syllable in capitals (`RUN-ing`).

The file is looked for at `[lexicon] data_file` when that is set, then at `TEXTWEAVER_LEXICON`, next to the program (`lexicon/lexicon-en.twlex`), in `../share/textweaver/lexicon/` (Linux) or `../Resources/lexicon/` (macOS), in the user's data folder, and in the source tree. Opening it reads it into memory (10 MB, about 40 ms); a lookup unpacks one to three blocks and takes about a millisecond the first time, a few microseconds after that.
