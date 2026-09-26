# IBMTTS community pronunciation dictionaries

A pinned copy of [eigencrow/IBMTTSDictionaries](https://github.com/eigencrow/IBMTTSDictionaries): a large, community-maintained pronunciation dictionary for ETI-Eloquence (IBMTTS), maintained by amirsol81, x0, and thunderdrop with contributions from many people. The exact release is in `PINNED.txt`.

- **License:** CC0 1.0 Universal (public domain dedication), in `LICENSE.md`. The files are the community's own work; they contain no part of the Eloquence engine or its data.
- **Contents:** US English (`ENUmain.dic`, `ENURoot.dic`, `ENUabbr.dic`) and German (`DEUmain.dic`, `DEURoot.dic`, `DEUabbr.dic`), in ECI's user-dictionary format: main (word to word or phonemes), root, and abbreviation dictionaries.
- **Encoding:** Windows-1252, as ECI expects. `.gitattributes` marks the files binary so they are stored byte for byte.
- **Used by:** the `eci` backend (`crates/textweaver-eci`, ADR-0007), which loads them into the engine with ECI's dictionary calls unless the user turns them off or points to their own copy. The main dictionary includes hyphenated words, so textweaver passes hyphens through to ECI unchanged.

Update to a newer monthly release with:

```bash
python tools/update_ibmtts_dictionaries.py
```

Then review the diff and commit.
