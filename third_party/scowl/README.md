# SCOWL word levels

textweaver's built-in list for marking difficult words, derived from SCOWL (Spell Checker Oriented Word Lists) version 2 by Kevin Atkinson and contributors.

- Project: https://github.com/en-wl/wordlist (SCOWL and Friends, http://wordlist.aspell.net/).
- Licence: MIT-like; see `Copyright`, copied verbatim from the release. Its notice must accompany the list and any copy of it. Because the list uses the Australian spelling code `D` and the region `AU`, the "=== AU" notice (Benjamin Titze) applies as well as the main notice. The list stops at size 80, so the UKACD notice (for lists larger than 80) does not apply; the whole file is kept anyway.
- Release: tag `rel-2026.02.25`, the newest release on 2026-09-25 (commit `7e99edab8e32f9f9ea2b15f249ca8d4d67237410`).
- Source archive: https://github.com/en-wl/wordlist/archive/refs/tags/rel-2026.02.25.tar.gz, 2,414,067 bytes, SHA-256 `74e7cc3e9e03e609c1c74bb7e8862fcd988cdd64768dcbee4611581b7e633852`.
- Downloaded: 2026-09-25.

## Files

- `scowl-levels.zip`: 692,007 bytes, SHA-256 `0d3f5fad5cda04591681b266ae7ef6c44e21109398bcd1765a68154f8630c655`. One entry, `scowl-levels.txt` (2,289,606 bytes, SHA-256 `4cd4a8164c96070be0496e4c310b2bb435c4b7cdf5ed2ef569119d21fa59b29d`), deflate-compressed.
- `Copyright`: SCOWL's copyright and permission notices, unchanged.

## Format

`scowl-levels.txt` is UTF-8 with LF line ends. It has one section per SCOWL size, in rising order. A section starts with a line `@SIZE` and lists every word whose smallest size is that size, one per line, lowercase, sorted in Unicode code point order (the same as UTF-8 byte order, so a program can binary-search it).

Sizes and word counts (225,038 words in all):

- 35 (small; SCOWLv2's smallest size, which takes in SCOWLv1's 10, 20, and 35): 41,093
- 40: 5,520
- 50 (medium): 28,299
- 60 (the default spell-checker size): 19,582
- 65: 1,275
- 70 (large): 52,748
- 80 ("a valid word in current usage"): 76,521

## How it was derived

Exactly what `tools/scowl_levels.py` does; running it again produces the same bytes.

1. Download the release archive above and check its SHA-256.
2. Build SCOWL's SQLite database with SCOWL's own tool, `python combine.py create-db scowl.db` (what SCOWL's Makefile runs). On Windows, set `PYTHONUTF8=1`, since the data files are UTF-8.
3. From SCOWL's `scowl_v0` view (the documented entry point for word lists), take `word` and its smallest `size` where: `size` is 80 or less; `variant_level` is 6 (`V`, acceptable variant) or less; `spelling` is any of `A` (American), `B` and `Z` (British), `C` (Canadian), `D` (Australian), or `_` (not region-specific); `region` is any; and `base_pos` is not `abbr` (abbreviations).
4. Keep only words made of letters, with apostrophes only inside a word (`don't`, `o'clock`). Drop possessives (words ending in `'s`); textweaver strips a possessive before looking a word up.
5. Lowercase every word. For a word with accents, also add its accent-free spelling (`naïve` gives `naive`). Each resulting word gets the smallest size of any entry it came from.
6. Write the sections as above, and store the text in a zip (deflate, level 9, a fixed 1980-01-01 timestamp) so the output is reproducible.

## How textweaver uses it

`textweaver-aids` embeds the zip (cargo feature `scowl`, on by default), unpacks it on first use (the `zip` crate, already in the workspace), and builds one sorted index over all sections. A word is difficult when its SCOWL size is above the threshold (50 by default, configurable from 35 to 95). A word not in the list counts as rarer than size 80. See `docs/reading-aids.md`.

SCOWL measures which dictionaries include a word, not how often people use it, so it is a rough guide: `ubiquitous` and `ornithology` are in size 35, and `serendipity` first appears at 50.
