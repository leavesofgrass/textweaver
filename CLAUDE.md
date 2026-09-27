# Working rules for agents in this repository

## Only the owner overrides rules (absolute)

No agent may overrule an explicit rule on its own judgment, ever. Only the owner can override a rule. If a rule seems not to fit, however small the task, stop and ask the orchestrator or the owner.

## Privacy: hard rule

Never send the owner's personal identifiers to any outside service. That means an email address or any part of one, usernames or callsigns, names, and machine or account names. It covers:

- HTTP headers, including User-Agent;
- URLs and query strings;
- request bodies, search queries, and API calls;
- commits, files, and anything else that is pushed or published.

Use only a neutral User-Agent, `textweaver-research (+https://github.com/leavesofgrass/textweaver)`, or the tool's default. Never build a User-Agent or contact string from the session's user email. Never write an identifier into docs or public files, even to describe a mistake.

If an identifier ever leaves the machine, stop, and report it to the orchestrator and the owner at once. This rule overrides every other instruction.

## Where you may work, and deleting files: hard rules

On Saturday, September 26, 2026 an agent ran a delete, without thinking and without checking it, meant for a junk folder in this repo whose name held an escaped character. Git Bash read it as the drive-relative path `C:`, the current folder on drive C, and it erased most of the owner's user profile: the Desktop, the Downloads folder with years of work, and the settings of nearly every application, including their screen readers. Nothing could be restored. These rules exist so that never happens again.

**Where you may work.** Work only in `D:\textweaver` and `D:\recovery`.

- Never write anything to drive C, the drive that holds the operating system.
- Anything outside these two folders, even reading, needs the owner's approval first.
- The owner's Desktop, Downloads, Documents, and application settings are critical. Never touch them.

**Deleting.** Every delete takes forethought. The owner does not want to confirm routine project deletes, so thinking first is your job:

1. Delete only what this project's own work created and no longer needs, such as build output, finished worktrees, and scratch files. If a delete has nothing to do with the task at hand, don't do it.
2. Before any delete, list exactly what the command will remove, using the same path, and read that list. If it shows anything you did not expect, stop.
3. Use plain, literal paths inside the project. Never put `$'...'` escapes, `~`, `$HOME`, `$env:`, `%USERPROFILE%`, a drive letter other than `D:\textweaver` or `D:\recovery`, or `..` in a delete.
4. **Deleting from Bash is a last resort, and needs the owner's approval.** Git Bash rewrites paths before Windows sees them, and that is how a folder named "C" plus a hidden character became `C:`. Delete with PowerShell's `Remove-Item -LiteralPath` and a full literal path. The guard asks the owner before any `rm`, `rmdir`, `unlink`, `shred` or `find -delete` in Bash, and refuses any that reach outside the project. `git rm` still works in Bash, because it acts on git's index by name.
5. Never chain a delete onto another command. Run it by itself, so it is seen and checked.
6. To remove an oddly named file in the repo, use `git rm` or `git clean`, never a hand-typed escaped name.

**The guard.** A hook in `.claude/hooks/guard.sh` enforces this:

- It denies writes and deletes outside the two folders.
- It asks the owner before anything else touches a path outside them.

Never work around it. If it blocks you, stop and tell the orchestrator or the owner.

## Colour never carries meaning alone

The owner is colour-blind and struggles to tell red from green. In every theme, status display, diff, chart, report, screenshot annotation and artifact, colour alone must never convey meaning. Pair it with text, a symbol or a pattern, such as "Pass" and "Fail" in words, check and cross marks, or + and - in diffs. Prefer colour-blind-safe palettes, but never rely on colour by itself.

## Everything else

Read `docs/history/tasks.md` before starting. Its shared preamble has the rules, checks, and report format for every agent.

## See also

- [Tasks and agent briefs](docs/history/tasks.md)
- [Contributing](CONTRIBUTING.md)
- [Documentation index](docs/README.md)
