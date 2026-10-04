# ADR-0030: Interface translations

- Status: accepted
- Date: 2026-09-28

## Context

ADR-0025 gave textweaver a message catalog in a subset of Project Fluent, with English complete for the study features only. star, the program textweaver replaces, had Spanish, French, German, Portuguese, and Arabic catalogs for its menus (flat JSON keyed by the English text, no plurals). This work called for the whole interface in those five languages, right-to-left display, a first-run language choice, a voice per language, a live language change, and a pseudo-locale check in the development checks (the orchestration plan, kept outside the repository). The lessons from star were to never go silent, to apply a change at once, and to give every new setting its four places.

## Decision

### The catalog stays the Fluent subset

No language needed Fluent attributes, functions, or number formatting, so `fluent-bundle` is not adopted (ADR-0025's fallback is not needed). Two things were enough:

- **Selects on text values** for words that agree with a noun: a message that names a unit or a kind of structure gets the noun (`$what`, "heading") and a key that never changes (`$unit`, `heading`, `list-item`), so French or German can choose by the key. The translation tests allow `$unit` in those messages though English does not use it.
- **CLDR plural categories** already in `textweaver_lexicon::i18n::plural`, including Arabic's six.

### Every message goes through the catalog

About 1,540 messages in `crates/textweaver-lexicon/locales/en.ftl`, from the app and the terminal reader: announcements, errors, questions, prompts, list titles and items, the help list, every command's one-line help (`action-*`, English checked equal to the keymap's own text, so the generated keyboard reference does not change), the help categories, the settings screen's labels, help, choices, units, and sections (`setting-*`, `choice-*`, checked equal to the schema's English), and the names textweaver's own voice gives keys ("Control", "period", checked equal to `KeyChord::spoken`). English output did not change: the workspace's tests still compare it.

- Messages are whole sentences with values; code no longer builds sentences from fragments.
- Keys named in messages are values filled from the keymap (`named_key_in`), written on the status line and spoken in the interface's language by textweaver's voice. Written chords (`Ctrl+S`) are not translated. A test scans en.ftl for chords.
- Public functions other crates use keep their English signatures, with `_in` siblings that take a catalog (`named_key_in`, `chords_text_in`, `open_failure_message_in`, `Setting::describe_in`, `Setting::parse_in`). The GUI still draws its settings dialog's labels in English; its messages come through the app and are translated.
- Left in English on purpose: log lines, text printed by other crates that has no catalog (format loaders' errors, the settings store's summaries, operating-system errors), command-line help from clap, JSON-RPC errors for programs, `tw` output other than `tw settings language` and the study commands, and the words the user types into the command palette (command ids).

### Five built-in translations

Spanish (international, formal), French (with a no-break space before `: ; ? !`), German, Portuguese (Brazilian, matching the CLDR default the plural rules use), and Arabic (Modern Standard, with six plural forms), each complete. star's catalogs were converted by script where a string matched textweaver's English (64 strings each for the Romance languages and German, 35 for Arabic) and given to the translators as hints; the rest were translated. They are built into the program (`include_str!`); a `<tag>.ftl` in the settings folder's `locales` goes over a built-in one message by message, then English.

Tests in `textweaver-lexicon` check that each built-in file parses, has only English's ids, uses only the values the code gives, chooses plural variants its language can produce, is complete, and, for Arabic, closes every direction mark and isolates every value. A message added to en.ftl therefore needs its five translations in the same change.

### Live change, and a voice for the language

The catalog is a value in the app. Changing `[interface] language` swaps it at once; the confirmation is said in the new language, followed by the title line. The voice then follows (`textweaver_engines::voice_for_language`): the voice `[speech] voices_by_language` names, else the current voice if it speaks the language, else the engine's preferred voice for it (`prefer_voice`, "eloquence" by default), else its first. **When the engine has no voice for the language, the current voice stays and textweaver says so; it never goes silent.** If the voice list is still loading after a short wait, that is said too.

### First run and the command line

The terminal reader's first run shows the language list, the system's language first (`LC_ALL`, `LC_MESSAGES`, `LANG`, or the Windows user locale); the hybrid-mode question moves to the next start. `tw settings language` lists the languages or sets one.

### Right-to-left display

Text stays in logical order in the document, in speech, and in what screen readers get. The terminal reader can reorder lines with right-to-left letters for display with `unicode-bidi` (UAX #9), keeping each character's style and moving the cursor with its character, behind `[interface] rtl`:

- `auto` reorders only where it helps: not in terminals that reorder themselves (VTE, Konsole, mlterm, macOS Terminal), not on Windows (Windows Terminal and the console have no right-to-left support; documented as unsupported there), and not in hybrid or screen reader mode, because a screen reader reads the terminal's cells and would get reordered text backwards;
- `on` and `off` force it.

Direction controls are used by the algorithm and not drawn. The prompt's typing line is not reordered, so its caret follows the logical text. The GUI leaves bidi to Parley.

### The pseudo-locale check

`crates/textweaver-app/tests/pseudo_locale.rs` runs every action once in `en-XA` over a document written in Greek and fails on plain English outside the brackets; in `ar-XB` it fails on a direction mark left open. `scripts/dev-check.sh` and `.ps1` run it as the `pseudo` step.

## Consequences

- Every new user-facing string needs a catalog id and five translations. The checks say which are missing.
- The binary carries about 800 KB of catalogs (six languages).
- Translations were made for this project and have not been reviewed by native speakers; a listening session checks Spanish and French by ear.
- Numbers keep English thousands separators ("3,412") in every language.

## See also

- [ADR-0025: Define word offline, and the interface's message catalog](0025-lexicon-and-message-catalog.md)
- [Using textweaver with a screen reader: Language](../screen-readers.md#language)
- [Settings: `[interface]`](../settings.md#interface)
