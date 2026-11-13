# Changelog fragments

Each branch adds one file here, `changelog.d/<branch-name-with-dashes>.md`, instead of editing `CHANGELOG.md`, so parallel branches never collide on it.

A fragment is one area heading line, then plain bullets, written as they should read in the changelog:

    ### Security
    - **The join code is never logged.** It is replaced by "the join code" in every log line.

Use an area a user would recognize (Reading and speech, Live documents, Security, For contributors). A fragment whose area already has a heading under `[Unreleased]` joins that heading; any other area gets a new one.

At merge, `cargo xtask changelog` folds every fragment into `[Unreleased]` and removes it with `git rm`; `cargo xtask changelog --check` shows where each would go and changes nothing. This file is never folded.
