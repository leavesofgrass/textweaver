# Working rules for agents in this repository

## Only the owner overrides rules (absolute)

No agent may overrule an explicit rule on its own judgment, ever. Only the owner can override a rule. If a rule seems not to fit, however small the task, stop and ask the orchestrator or the owner.

## Privacy: hard rule

Never send the owner's personal identifiers to any outside service. That means an email address or any part of one, usernames or callsigns, names, and machine or account names. It covers:

- HTTP headers, including User-Agent;
- URLs and query strings;
- request bodies, search queries, and API calls;
- commits, files, and anything else that is pushed or published.

**In files and commit messages, call the owner "the owner", never by name** (the owner's decision, Monday, September 28, 2026). Test data uses placeholder names such as "Ada Example". **One exception, by the owner:** the `authors` field in the root `Cargo.toml` carries the owner's full name, as the GitHub account already does. Don't change it. Never guess the owner's pronouns; write "the owner" or "they".

Use only a neutral User-Agent, `textweaver-research (+https://github.com/leavesofgrass/textweaver)`, or the tool's default. Never build a User-Agent or contact string from the session's user email. Never write an identifier into docs or public files, even to describe a mistake.

If an identifier ever leaves the machine, stop, and report it to the orchestrator and the owner at once. This rule overrides every other instruction.

## Where you may work, and deleting files: hard rules

On Saturday, September 26, 2026 an agent ran a delete, without thinking and without checking it, meant for a junk folder in this repo whose name held an escaped character. Git Bash read it as the drive-relative path `C:`, the current folder on drive C, and it erased most of the owner's user profile: the Desktop, the Downloads folder with years of work, and the settings of nearly every application, including their screen readers. Nothing could be restored. These rules exist so that never happens again.

**Where you may work.** Work only in `D:\textweaver`, `D:\recovery`, and `D:\textweaver-planning` (the local planning repository, never pushed; added by the owner's decision, Monday, September 28, 2026).

- Never write anything to drive C, the drive that holds the operating system. **One standing exception (the owner, Wednesday, September 30, 2026):** tools the owner has approved may be installed on drive C in their normal install locations. Installing only: removing anything on drive C still needs the owner's go for that exact thing.
- Anything outside these two folders, even reading, needs the owner's approval first, with one standing exception. **Reading the owner's wiki, `D:\star\wiki`, is expressly permitted,** read-only, to put development in context (the owner's global rule 6). Never write to it from an agent; the orchestrator documents work there.
- The owner's Desktop, Downloads, Documents, and application settings are critical. Never touch them.

**Deleting.** Every delete takes forethought. The owner does not want to confirm routine project deletes, so thinking first is your job:

1. Delete only what this project's own work created and no longer needs, such as build output, finished worktrees, and scratch files. If a delete has nothing to do with the task at hand, don't do it.
2. Before any delete, list exactly what the command will remove, using the same path, and read that list. If it shows anything you did not expect, stop.
3. Use plain, literal paths inside the project. Never put `$'...'` escapes, `~`, `$HOME`, `$env:`, `%USERPROFILE%`, a drive letter other than `D:\textweaver` or `D:\recovery`, or `..` in a delete.
4. **Remove files and folders only with a script, never a shell command** (the owner's rule, Tuesday, September 29, 2026). Shells cannot be trusted to escape names; that is how the data loss happened. Use `.claude/tools/safe-remove.ps1` (PowerShell): it lists by name pattern or character code, checks each match is inside the root you give, prints full paths with hidden characters spelled out, removes nothing without `-Apply`, and removes each match by reference. Run it dry first, then with `-Apply` and `-ExpectCount`. For anything it cannot express, write a small PowerShell or Python script that does the same: list, check, print, then remove by reference. Not `git clean`, not `rm`, not a one-line `Remove-Item`. `git rm` of tracked files remains allowed, because git acts on its own index.
5. **Deleting from Bash is never done without the owner's approval.** Git Bash rewrites paths before Windows sees them, and that is how a folder named "C" plus a hidden character became `C:`. Delete with PowerShell's `Remove-Item -LiteralPath` and a full literal path. The guard asks the owner before any `rm`, `rmdir`, `unlink`, `shred` or `find -delete` in Bash, and refuses any that reach outside the project. `git rm` still works in Bash, because it acts on git's index by name.
6. Never chain a delete onto another command. Run it by itself, so it is seen and checked.
7. To remove an oddly named file, use `.claude/tools/safe-remove.ps1 -CharCode` (untracked) or `git rm` (tracked), never a hand-typed escaped name.
8. **Escape sequences and odd names (the owner's global rule 1a).**
   - Never type a file name with an escape sequence (`$'...'`, `\x..`, `\u....`, `printf`, URL encoding) in any command that deletes, moves, renames, copies or writes.
   - Find unusual names by listing, and act on them by reference: git pathspecs with a dry run first, or the listed PowerShell object. Never retype them.
   - Resolve and print the full path before any destructive command, and stop if it isn't inside the intended folder.
   - Use `-LiteralPath` in PowerShell, and `MSYS_NO_PATHCONV=1` for Docker in Git Bash. Guard against empty variables, with `set -u` and `${VAR:?}`.
   - The guard asks the owner before any move, copy or write that contains escapes or hidden characters, and refuses such deletes.

**Plain names only (the owner, Wednesday, September 30, 2026).** Never create a file or folder whose name contains control, private-use or other invisible characters; the reserved characters `: * ? " < > | \`; leading or trailing spaces; or trailing dots. Hidden characters in names are what led to the bad escape on September 26. Use letters, digits, `-`, `_` and `.`, and sanitize names that come from outside (document titles, URLs, archive members) before writing. If a tool or test would create an odd name, fix that instead.

**The guard.** A hook in `.claude/hooks/guard.sh` enforces this:

- It denies writes and deletes outside the two folders.
- It asks the owner before anything else touches a path outside them.

Never work around it. If it blocks you, stop and tell the orchestrator or the owner.

## Accessibility first

Accessibility is the top priority. The owner is blind, and uses a screen reader (JAWS and NVDA) and a Braille display. When accessibility conflicts with looks, convenience or speed, accessibility wins. If an accessible way isn't possible, say so in your report; never ship the inaccessible version quietly.

- **For the Braille display:** put the meaning first on each line, and prefer words to symbols. Emoji, arrows, check marks and box drawing often come through as noise. Keep lines short, and avoid ASCII art and character layouts.
- **Braille output** (the BRF writer, and the Braille display through the screen reader) is first-class and tested like speech.

## Color never carries meaning alone

The owner is also color-blind and struggles to tell red from green. In every theme, status display, diff, chart, report, screenshot annotation and artifact, color alone must never convey meaning. Pair it with text first, such as "Pass" and "Fail" in words or + and - in diffs, then symbols or patterns if useful. Symbols alone don't come through well on a Braille display. Prefer color-blind-safe palettes, but never rely on color by itself.

## Writing

- **US English** in all prose, code comments, docs and reports ("color", "behavior", "organize").
- **Use em dashes sparingly.** Readers take heavy em-dash use as a sign of AI-generated text. Prefer commas, periods, colons, semicolons or parentheses, and split long sentences.

## Everything else

Read `D:\textweaver-planning\history\tasks.md` before starting. Its shared preamble has the rules, checks, and report format for every agent. The owner's plans, briefs, and research live in that local planning repository, not in this one.

## See also

- Tasks and agent briefs: `D:\textweaver-planning\history\tasks.md` (outside this repository)
- [Contributing](CONTRIBUTING.md)
- [Documentation index](docs/README.md)
