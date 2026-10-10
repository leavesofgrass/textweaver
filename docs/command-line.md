# The command line

`tw` is textweaver's terminal program. On its own it opens the terminal reader; with a command it does one job without the reader, such as `tw text`, `tw speak`, `tw convert` and `tw cite`. This page is for anyone who uses `tw` in a terminal or a script. It gives the rules every command follows, so that what you learn in one command works in the next. Each command's own options are in its guide and in `tw COMMAND --help`.

## One program, two names

| You type | What happens |
|---|---|
| `tw` | The terminal reader opens with no document. |
| `tw essay.md` | The terminal reader opens `essay.md`. `tw open essay.md`, the older spelling, does the same. |
| `tw convert essay.md --to html` | The command runs without the reader and ends. So do the other commands. |
| `tw --help` | Every command, and the reader's options. |

`textweaver` is a second name for `tw`, kept so that older shortcuts and scripts still work: `textweaver essay.md` and `tw essay.md` are the same, and `textweaver --help` prints the same help as `tw --help`. On Linux and macOS `textweaver` is a link to `tw`; on Windows it is a small launcher, `textweaver.exe`, that starts `tw.exe` beside it. The app itself is `textweaver-gui`, a separate program in the same download.

The reader's options go before the file: `tw --no-speech essay.md`, `tw --mode hybrid`, `tw --theme light essay.md`, `tw --backend espeak`, `tw --home DIR` and `tw --log`. They do not go with a command. A document whose name is also a command's name, such as a file called `convert`, opens with `tw open convert` or `tw ./convert`.

The reader needs a terminal. Started with standard output going to a file or a pipe, `tw` says so in one error line and ends with exit status 1, instead of drawing the reader into the file.

## The rules in one table

| Rule | What it means |
|---|---|
| Inputs | The file, folder or text a command works on comes first, without a flag: `tw text essay.md`. |
| Actions | An action is a word after the command, not an option: `tw library add FOLDER`, `tw stats clear`, `tw dictate list`. |
| Output | `--out PATH` writes to a file or folder; `-o` is its short form. |
| Format | `--to FORMAT` names what is written: `tw convert --to pdf`, `tw text --to markdown`, `tw cite export --to ris`, `tw marks essay.md --to bibtex`. |
| JSON | `--json` prints the facts as JSON on every command that prints facts. JSON keys are English and never translated. |
| Data folder | `--home DIR` uses another data folder for one run, on every command that reads or writes textweaver's files. |
| Language | `--lang` names a language: `tw dictate --lang de`, `tw ocr read scan.png --lang fra`. |
| Questions | A command asks a yes or no question only when it runs in a terminal. `-y` or `--yes` answers yes for you. |
| Exit status | 0 done; 1 failed or nothing found; 2 the command was typed wrong. |
| Errors | One line on standard error: "Error: what failed: why. What to do." |
| Results | Results go to standard output; progress and questions go to standard error. |

## Actions

A command that does more than one thing takes the action as a word after its name, and each action has its own `--help`:

```bash
tw library add Readings
tw library search mitochondria
tw stats clear
tw dictate list --json
tw changes accept draft.docx --out final.md
```

`tw library` alone still lists the library, `tw stats` alone still prints the statistics, and `tw changes FILE` alone still lists the changes. The older option spellings still work for now, and will be removed after beta 1: `tw library --add`, `--remove`, `--search` and `--continue`, `tw stats --clear`, `tw dictate --list`, and `tw changes --accept-all` and `--reject-all`.

## Output and format

`--out` writes a file or, for `tw convert`, a folder. `-o` is the short form, and `--output` still works.

```bash
tw speak "Hello" --out hello.wav
tw cite export --to ris -o references.ris
```

`--to` names the format. Older spellings still work for now, and will be removed after beta 1: `--format` (in `tw text` and `tw settings export`), `--export` (in `tw marks`), `--output`, and `--language` (in `tw dictate`). `tw cite format` keeps `--as` for its markup, because it chooses plain text, Markdown or HTML for the same references.

## JSON

Every command that prints facts takes `--json`, either on the command or on the subcommand that prints them: `tw info --json`, `tw text --json` (the same as `--to json`), `tw cite list --json` (and `check`, `styles`, `add`, `import`, `remove`, and `export` with `--out`), `tw settings path --json`, `tw components list --json`, `tw ocr status --json`. The keys are English in every language.

## The data folder

textweaver keeps your settings, places, notes and libraries in its data folders. `tw settings path` names them all, with the log file. `--home DIR` puts all of them under `DIR` for one run; the `TEXTWEAVER_HOME` environment variable does the same for every run. `--home` works on `tw` and `tw open` (the reader), `speak`, `voices`, `backends`, `convert`, `export-audio`, `library`, `vault`, `dictate`, `marks`, `notes`, `migrate-star`, `cite`, `settings`, `define`, `stats`, `summarize`, `changes`, `sync`, `serve` and `components`, and on `tw ocr status` and `tw ocr download`, where the OCR models are kept.

## Questions

A command that removes or downloads something asks first: `tw cite remove`, `tw stats clear`, `tw settings reset`, `tw settings profile delete`, `tw components download` and `remove`, `tw ocr download` and `tw dictate download`. The question goes to standard error and ends with "y or n"; type `y` or `yes`, in any case, and press Enter. Anything else keeps things as they are.

A question needs a terminal. When standard input is a pipe or a file, as in a script, `tw` never waits for an answer: it stops with exit status 1 and says "Add --yes to go ahead without a question." Add `-y` or `--yes` when you mean it.

## Exit status

| Status | Meaning |
|---|---|
| 0 | Done. |
| 1 | Something failed, or nothing was found: `tw search` with no match, `tw define` with no definition, `tw lint` with problems, `tw cite check` with keys missing from the library, `tw convert` with a file that failed, a question with no terminal, and the reader with no terminal. |
| 2 | The command was typed wrong: an unknown option, or a value that is missing or not allowed. The message names it. |

A script can test the status: `tw cite check essay.md || echo "Some keys are missing."`.

## Error lines

An error is one line on standard error, starting with "Error:", then what failed, why, and what to do when there is something to do:

```text
Error: Cannot open notes.md: there is no file named notes.md here. Check the name.
```

## Reading standard input

`tw speak -` reads the text to speak from standard input, so a pipe can feed it. To hear what is on the clipboard:

- PowerShell: `Get-Clipboard | tw speak -`
- macOS: `pbpaste | tw speak -`
- Linux: `wl-paste | tw speak -`, or `xclip -o -selection clipboard | tw speak -`

## A closed pipe

Results are written so that `tw search essay.md word | head` ends quietly once `head` has its lines.

## See also

- [Quick start](quickstart.md)
- [Settings](settings.md), for `tw settings path` and `--home`
- [Converting documents](converting.md), [Citations](citations.md) and [Dictation](dictation.md), for those commands' own options
- [Troubleshooting](troubleshooting.md)
