# ADR-0050: File names and formats frozen at the final alpha

- Status: accepted; the JSON-RPC methods and the conversion report are implemented, the other shapes are reserved, not implemented
- Date: 2026-10-03 (Saturday, October 3, 2026)
- Builds on: [ADR-0015](0015-json-rpc.md) (JSON-RPC and its version rule), [ADR-0045](0045-a-file-browser-on-the-list-model.md) (archive members as `course.zip!week1/notes.md`), and [ADR-0049](0049-sync-beyond-the-place.md) (sync ids, and no paths in the sync folder)

## Context

The final alpha freezes textweaver's public file names and formats. After it, a file an office or a student wrote must keep working, so a name or a shape can only grow, never change.

Several beta 1 features add a file that people will write by hand, ship to students, or read from other programs:

- a description for each figure, written once by an office;
- a study pack of documents with its pronunciations and glossary;
- a machine-wide policy file, and an exam profile that turns commands off;
- a reading queue of documents to read one after another;
- a reading plan for one document, with a due date;
- the JSON-RPC methods other programs call.

None of these features is built yet, apart from the JSON-RPC methods. Their names and shapes are settled here, before the freeze, so beta 1 adds them without breaking anything. Fields added to the document state are safe either way: `DocState` keeps keys it does not know (`crates/textweaver-store/src/doc_state.rs`, its `extra` map), so an older textweaver keeps a newer field instead of dropping it.

## Decision

### Rules for every file here

- **Plain names.** Every fixed file name uses only lowercase letters, digits, `-` and `.`. A name textweaver makes from a document's name keeps that name and adds a fixed suffix, as the conversion report does today (`essay.pdf.report.md`).
- **A format number.** Each TOML or JSON file carries a whole-number `format`, starting at 1. Adding a field keeps the number, and readers ignore fields they do not know. Renaming or removing a field, or changing what it means, raises the number. A file with a newer number than this textweaver knows is read for what it can use and never rewritten, as sync does with a newer `format.json`.
- **UTF-8, read aloud well.** Files are UTF-8 text a screen reader can read line by line. Errors name the file and the line, in words.
- **Paths inside a file** are relative to that file, written with `/`, and never contain `..` or start at a drive or root. A path that breaks this is skipped and reported.

### The figure description sidecar

Status: reserved, not implemented.

- **Name:** the document's full file name plus `.figures.md`, beside it: `lecture.pdf` gives `lecture.pdf.figures.md`. Inside a zip, the sidecar is the member with that name beside the document's member.
- **Shape:** Markdown. A level 1 heading is optional and ignored. Each figure is one level 2 heading naming where it is, followed by its description as ordinary paragraphs, up to the next level 2 heading. An empty description means "no description yet".
- **Where a figure is,** in one of three heading forms, matched without regard to case:
  - `## Page 12, figure 2`: the second picture on page 12, counted in reading order. For paged documents, such as PDF.
  - `## Figure 4`: the fourth picture in the document, counted in reading order. For documents without pages.
  - `## Id fig-membrane`: the picture whose element id is `fig-membrane`, in EPUB, HTML, and other formats with ids. Preferred when there is one, since it survives edits around the figure.
- The heading words (`Page`, `figure`, `Figure`, `Id`) are part of the format, in English whatever the interface language, like the keys of a TOML file.
- A heading in none of these forms, or naming a figure the document does not have, is skipped and reported once when the document opens. Nothing else in the file is lost.
- textweaver reads the sidecar when the document opens; narration says "graphic," then the description. `tw convert` writes the description as the picture's alternative text. A description in the sidecar wins over one in the document. `tw convert` never takes a sidecar or a report as an input document, as it already skips reports today (`crates/textweaver-convert/src/plan.rs`).

### The study pack manifest

Status: reserved, not implemented. The conversion report it carries exists today.

A study pack is a folder, or a plain zip file, with `pack.toml` at its top. Everything else sits beside it:

- the documents, in any folders below;
- `pack.toml`, the manifest;
- `pronunciations.toml` and `glossary.toml`, both optional;
- the figure sidecars, beside their documents;
- `profile.toml`, an optional exported profile;
- `conversion-report.md` or `conversion-report.json`, optional, as `tw convert` writes them today.

`pack.toml`:

```toml
format = 1
title = "Pharmacology 3"
language = "en-US"

[[documents]]
path = "week1/intro.pdf"
title = "Week 1: Introduction"

[[documents]]
path = "week2/receptors.epub"
```

- `title` is required. `language` is an optional language tag.
- `documents` lists the documents in reading order. `path` is required; `title` is optional and wins over the document's own title in the pack's list. A document in the folder but not listed is not part of the pack's order.
- `pronunciations.toml` has the shape of the medical lexicon overlay today: `format = 1` and a `[pronunciations]` table of `term = "spoken form"`, with the same matching rules (`crates/textweaver-speech/src/normalize/medical-en.toml`).
- `glossary.toml` is `format = 1` and a `[glossary]` table of `term = "definition"`, or `term = ["first sense", "second sense"]` for several senses. It is read after the reader's own glossary, so the reader's senses come first.
- `profile.toml` is a profile export in TOML as `Profiles::export` writes it today (`textweaver_profiles = 1`, then `[profiles.<name>...]`). Opening a pack offers the profile; it is never applied unasked.
- The pack's pronunciations and glossary apply only to the pack's documents, and never change the reader's own files.

### The managed policy file and the profile lock list

Status: reserved, not implemented.

A lock on a computer the student administers is a policy aid, not security, and textweaver never calls it security. Only a file the student cannot change, on a managed computer, is enforceable.

- **Name and place:** `policy.toml`, read once at start from the machine-wide folder only: `%ProgramData%\textweaver\policy.toml` on Windows, `/Library/Application Support/textweaver/policy.toml` on macOS, and `/etc/textweaver/policy.toml` on Linux. It is never read from the user's settings folder, and textweaver never writes it.
- **Shape:**

```toml
format = 1
locked_actions = ["define_word", "summarize", "dictate", "export_docx"]
downloads = false

[components]
mirror = "https://components.example.edu/textweaver"

[settings]
"speech.rate" = 220
```

- `locked_actions` lists action names as the keymap writes them (`crates/textweaver-keymap/src/action.rs`). A locked action is refused wherever actions run, including JSON-RPC `action`, with a sentence in words; it never touches speech, the screen reader, or Braille.
- `downloads = false` turns off every download of optional components; `[components] mirror` sets the mirror as `[components] mirror` does in the settings.
- `[settings]` holds settings by their dotted names. Each one is set to that value and locked: the settings screen shows it as set by the policy and does not change it.
- **The profile lock list:** a profile in `profiles.toml` may carry `locked_actions`, a list of action names, next to its settings tables. Switching to that profile locks those actions until another profile is chosen; opening and leaving it are written to the log. An exam profile is such a profile. A policy's locks always apply on top of a profile's.

### The reading queue file

Status: reserved, not implemented.

- **Name and place:** `queue.json` in the data folder, beside `library.json`. It stays on this computer; queue entries are paths, and the sync folder never holds paths (ADR-0049).
- **Shape:**

```json
{"format": 1, "items": [{"path": "D:/Courses/pharm/week1.pdf", "added": 1790000000}]}
```

- `items` is in reading order. `path` is the full path, or an archive member as `course.zip!week1/notes.md`. `added` is when it was queued, in Unix seconds (UTC).
- A setting decides what happens at a document's end: `[reading] after_end = "stop"`, `"ask"`, or `"next"`, with `"stop"` the default. textweaver says what comes next before it opens it, and never starts a document unasked.

### The plan record

Status: reserved, not implemented.

A reading plan lives in the document's own state, not in a file of its own, using the state's room for new keys:

```json
"rate": 180,
"plan": {"due": "2026-11-12", "daily_minutes": 22, "set": 1790000000}
```

- `rate` is this document's own speech rate in words per minute; without it, the settings' rate applies.
- `plan.due` is a calendar date in ISO form, in the reader's own time zone, with no time of day. `plan.daily_minutes` is optional, the minutes a day the reader wants to read. `plan.set` is when the plan was made, in Unix seconds (UTC).
- Both keys are optional. Whether they travel with sync is left to ADR-0049's rules for document state.

### The JSON-RPC methods

Status: implemented; frozen as they are.

The 29 method names in `crates/textweaver-app/src/rpc.rs` (`METHODS`), the six notification names (`NOTIFICATIONS`), the error codes `-32001` to `-32004`, and protocol version 1 are frozen, with the shapes [JSON-RPC](../json-rpc.md) documents. That includes the read-only methods added in Wave 9: `outline`, `notes`, `highlights`, and `info`. ADR-0015's rule stands: adding a method, notification, parameter, or result field keeps version 1; renaming, removing, or changing a meaning raises it.

Two names are reserved for the features above, not implemented:

- `figures`, a read-only method in the shape of `notes`: `{figures}`, each `{index, page, id, description}`, where `page`, `id`, and `description` may be null.
- `-32005`, "locked": an `action` refused by a policy or a profile lock. The message is the sentence the reader hears.

## Consequences

- Beta 1 builds these features without a format change, and an office can start writing figure sidecars and pack manifests to this shape as soon as each feature lands.
- Every shape is small. Anything not settled here (more manifest fields, more policy keys, the plan's later steps) is added under the "add only" rule.
- The figure anchors are the open risk. Counting pictures in reading order depends on each loader's reading order, which can still change before the freeze; the `Id` form is preferred where a format has ids for that reason. If counting proves unstable, a fourth heading form may be added, and the three here keep working.
- The exam lock is honest by design: the docs must say that without a managed computer and the machine-wide file, it is a policy aid.

## See also

- [JSON-RPC](../json-rpc.md)
- [Settings](../settings.md)
- [ADR-0015: JSON-RPC](0015-json-rpc.md)
- [ADR-0049: Sync beyond the place](0049-sync-beyond-the-place.md)
- [The ADR index](README.md)
