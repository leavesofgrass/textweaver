### Security and privacy

- rusqlite 0.37.0 to 0.41.1, for RUSTSEC-2026-0459: the SQLite that rusqlite builds in (now 3.54.1, through libsqlite3-sys 0.39.1) could return stale bytes from memory when it opened a database together with a crafted write-ahead log whose page size differed. textweaver uses SQLite only to read and write Anki packages, entirely in memory, so the exposure was small; Anki import and export work as before. The 0.40 line still bundled an affected SQLite, so 0.41 is the lowest fixed release.
