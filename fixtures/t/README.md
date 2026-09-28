# Fixtures for the automated screen-reader checks

`reading.md` is the document the checks in [ADR-0039](../../docs/adr/0039-automated-screen-reader-checks.md) open: three sentences, two more headings, and a short list, so a script can read three sentences, move by heading, and open the outline. Change it and the accessibility tree changes too, so the next tree dump in the GUI workflow reports the difference against main.

`expected-phrases.json` lists what each step of a screen-reader session should say, for `tools/a11y/nvda-session.mjs` and `tools/a11y/voiceover-session.mjs`. The patterns are loose on purpose, since NVDA and VoiceOver word things differently. A "must" entry fails the run when no spoken phrase matches it; a "should" entry is reported as a warning.
