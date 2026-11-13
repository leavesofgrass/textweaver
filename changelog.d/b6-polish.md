### For contributors
- **`tw cite add PMID 12345678` works without quotes.** The identifier may be several words, joined with a space, so `PMID 12345678`, `PMID:12345678` and `"PMID 12345678"` all look up the same article; a DOI or ISBN works as before ([Add an article by PubMed ID](docs/citations.md#add-an-article-by-pubmed-id)).
- The English messages spell "canceled" the US way: the task status "Canceled." and the help of Cycle task status.
- The code left over from the removed first-run question about hybrid mode is gone, with its two messages in every language; the help of `accessibility.hybrid_offered` says what the setting records now.
- `cargo xtask release` runs every step of a full `scripts/dev-check`, the one list of steps, in its own target folder; the dry run lists the steps ([Releasing](docs/dev/releasing.md)).
- Comments brought up to date: the steps in `dev-check.ps1`'s header, `tw changes`' hidden old flags, and the vendored MathCAT in the root `Cargo.toml`. The `tw backends` example in the speech guide names Eloquence as `tw` prints it.
