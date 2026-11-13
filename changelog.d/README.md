# Changelog fragments

Each branch adds one file here, `changelog.d/<branch-name-with-dashes>.md`, instead of editing `CHANGELOG.md`, so parallel branches never collide on it.

A fragment is one area heading line, then plain bullets, written as they should read in the changelog:

    ### Security and privacy
    - **The join code is never logged.** It is replaced by "the join code" in every log line.

Use an area a user would recognize. The areas in use are Reading and speech, Live documents, The app, The terminal reader, Documents and formats, Security and privacy, Documentation, and For contributors. A fragment whose area already has a heading under `[Unreleased]` joins that heading; any other area gets a new one.

Write a link in a fragment as it will read in `CHANGELOG.md`, relative to the repository root, such as `[Messages](docs/dev/messages.md)`. The link checker (`tools/check_links.py`) resolves the links of a fragment from the repository root, not from this folder, so the link works both here and after the fold. This file's own links resolve from this folder.

At merge, `cargo xtask changelog` folds every fragment into `[Unreleased]` and removes it with `git rm`; `cargo xtask changelog --check` shows where each would go and changes nothing. This file is never folded.
