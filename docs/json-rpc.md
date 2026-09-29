# Driving textweaver from another program (JSON-RPC)

`tw serve --stdio` runs textweaver as a server that another program controls. The other program, called the client, starts `tw serve` as a child process, writes requests to its standard input, and reads answers from its standard output. The client can open a document, move through it by sentence, heading, line, or any other textweaver unit, have it read aloud with textweaver's voices, run any keyboard action by name, and follow along as each word is spoken. This guide is for people writing an editor plugin, a web page, or any other tool that wants textweaver's reading without rebuilding it. You do not need to know much about JSON-RPC to follow it.

This guide is written to be read with a screen reader. Each message is shown in its own code block, with a sentence before it saying what it is.

## Before you start

- You need `tw` on your computer. Check with `tw --version`. See [the install guide](install.md) if you do not have it.
- The protocol is JSON-RPC 2.0. Every message is one JSON object. You do not need a JSON-RPC library: any language that can start a process, write a line, and read a line is enough.
- Positions are character offsets counted from the start of the document, starting at 0. They count Unicode characters (code points), the way Python counts string indexes. JavaScript and VS Code count UTF-16 units, so a client written in those must convert.
- Every example in this guide was run against the real `tw` program with `--no-speech`, so nothing was spoken aloud.

## Start the server

This starts the server and speaks, like the terminal reader:

```bash
tw serve --stdio
```

Nothing is printed for a person to read. The server waits for JSON messages on standard input and writes only JSON messages on standard output. A person does not normally type at it. A program starts it.

`--stdio` is required. Without it, `tw serve` stops at once with an error and exit code 1:

```bash
tw serve
```

You get "Error: `tw serve` needs a transport; use --stdio". There is no network or socket mode. A client that wants a long-running server keeps the child process open.

### Stay silent with --no-speech

```bash
tw serve --stdio --no-speech
```

Nothing is spoken. The client still receives every announcement and every playback change as a message, so it can show them or send them to a screen reader. Use this when the client already has a voice, for example a screen reader user's editor. See [Speech: who does the talking](#speech-who-does-the-talking).

### Choose a speech engine with --backend

```bash
tw serve --stdio --backend sapi
```

The value is a backend id, such as `eci` (ETI-Eloquence), `sapi`, `dectalk`, or `null`. See the list your computer has:

```bash
tw backends
```

Without `--backend`, the server uses the `[speech] backend` setting, which is normally `auto`. If the backend you name is not available, the server uses the automatic choice and tells the client with the announcement "Speech backend sapi is not available; using …", naming the one it chose. `--backend` does nothing together with `--no-speech`.

### Keep settings in another folder with --home

```bash
tw serve --stdio --home "C:\textweaver-test"
```

Settings, reading positions, bookmarks, and notes are read from and saved under this folder instead of your usual ones. This is useful while you build a client, so your tests never change your real reading positions. Setting the `TEXTWEAVER_HOME` environment variable to a folder does the same.

### Open a document at start with --open

```bash
tw serve --stdio --open notes.md
```

The document is open before the first request arrives, and the client hears "Opened" and the title as an announcement. If it cannot be opened, the server keeps running with no document and sends an announcement that starts "Could not open". You can always open a document later with the `open` method.

## How messages travel

### One message per line

The simplest way is one JSON object per line. Write the object, then a newline. The server answers the same way: one JSON object per line. Blank lines are ignored.

### Content-Length framing, like the Language Server Protocol

A client may instead put a header before each message, the way language servers do:

```text
Content-Length: 42

{"jsonrpc":"2.0","id":2,"method":"status"}
```

The header line ends with a carriage return and a line feed, and a blank line (another carriage return and line feed) comes before the message. The length is the number of bytes of the message in UTF-8. The header name may be in any case. Put `Content-Length` first: header lines after it, such as `Content-Type`, are skipped, but a line before it is read as a message of its own and fails.

The server works out the framing for each message it reads: a line that starts with `Content-Length:` means a framed message, and anything else is a message on its own line. It answers in the framing of the last message it received, and sends its own notifications in that framing too. Framed answers carry only a `Content-Length` header. Before the first message, the server uses one message per line.

Pick one framing and keep to it. If you switch in the middle, a framed answer is not followed by a newline, so the next one-per-line answer can end up on the same line.

### What every message looks like

Every message is a JSON-RPC 2.0 object:

- `jsonrpc` must be the string `"2.0"`. A message without it is refused.
- `method` is the name of the method, such as `open`.
- `params` is an object with the method's parameters. Leave it out when a method has none.
- `id` is a number or string you choose. The answer carries the same `id`, so you can match them. A message without an `id` is a notification: the server carries it out but sends no answer.

An answer has either `result` (success) or `error` (failure), never both. The server answers requests in the order they arrive.

A message can also be a JSON array of requests, called a batch. The server handles each one in order, but it sends the answers one by one, as separate messages, not as one array.

Messages from the server that have `method` and no `id` are notifications. They can arrive at any time: before the answer to the request that caused them, and also between requests, while speech is going on. Your client must be ready to read them whenever it reads.

## Speech: who does the talking

Without `--no-speech`, the server speaks, exactly like the terminal reader. It uses the same settings, the same engine choice, and the same voices. It reads the document aloud and also speaks its own announcements, such as "Opened Cell biology." The client never receives audio.

Every announcement is also sent to the client as an `announcement` notification, with its text and how urgent it is. So there are two ways to work:

- Let textweaver talk. The client shows the announcements, or ignores them.
- Start the server with `--no-speech`. The client gets the announcements and passes them to its own voice or to the user's screen reader. Use this for screen reader users, so they do not hear two voices at once.

The server does not follow the reader's accessibility mode (`[accessibility] mode`, see [Using textweaver with a screen reader](screen-readers.md)). It always speaks for itself, unless you start it with `--no-speech`, so a client that chose one way of working gets it whatever mode the terminal reader was left in.

With `--no-speech`, reading does not take any time. When you ask the server to read, you get "reading", one `position` notification, and "stopped" almost at once.

## Methods

The server has 24 methods. `initialize` lists them all, so a client can check. Parameters marked optional may be left out.

### initialize: say hello and learn the version

Parameters: none.

The result is an object with:

- `server`: always `"textweaver"`.
- `version`: the textweaver version, such as `"0.1.0-alpha.3"`.
- `protocol`: the protocol version, a number. It is `1`.
- `methods`: the names of every method.
- `notifications`: the names of every notification.

You do not have to call `initialize` first. Every method works without it.

### open: open a document

Parameters:

- `path` (string, required): the file to open. A relative path is taken from the folder the server was started in.

The result is an object with:

- `document`: facts about the document (see below).
- `position`: where reading starts (the same shape as the `position` method's result). If you read this document before, this is the saved position, and the announcement says "Resumed at" and a percentage.

The `document` object has:

- `title`: the document's title. For the Markdown example below it is the first heading. For a plain text file it is the file name.
- `path`: the full path, or null.
- `format`: the format, such as `"markdown"`.
- `length`: the number of characters in its text.
- `lines`: the number of lines.
- `editing`: true while edit mode is on.
- `dirty`: true when there are unsaved edits.

If edit mode is on when you call `open`, textweaver leaves edit mode first. The result then has `document` and `effects` instead of `document` and `position`, so call `position` afterwards.

Errors: `-32602` when `path` is missing, and `-32002` when the file does not exist or cannot be read.

### status: everything at once

Parameters: none. It works with or without a document.

The result is an object with:

- `mode`: the reader's mode, such as `"Browse"`, `"Edit"`, `"Speech Cursor"`, `"Find"`, `"Go to"`, or `"Command"`.
- `playback`: `"reading"`, `"paused"`, or `"stopped"`.
- `document`: the same object `open` returns, or null when no document is open.
- `position`: the same object `position` returns, or null.
- `rate`: the reading rate in words per minute.
- `backend`: the speech backend id, or `"silent"` with `--no-speech`.
- `status`: the last status message, the same text the terminal reader shows on its status line. It is empty at first.
- `pending`: the yes-or-no question waiting for an answer, or null. See [action](#action-run-any-command-by-name).

### position: where the cursor is

Parameters: none. It needs an open document.

The result is an object with:

- `char`: the character offset of the reading position.
- `line`: the line number, starting at 1.
- `column`: the column, starting at 1.
- `percent`: how far through the document, from 0 to 100, rounded down.
- `word`: the word at the position, as `{start, end, text}`, or null when the position is not inside a word (on a space, for example).

### navigate: move by a unit or go to a place

It needs an open document. Give one of these parameters:

- `action` (string): the id of a movement action, such as `next_sentence`, `previous_paragraph`, `next_heading`, `skip_next_heading`, or `next_bookmark`. Actions from four groups of the [keyboard reference](keyboard.md) are allowed: [Navigation](keyboard.md#navigation), [Speech Cursor](keyboard.md#speech-cursor), [Search](keyboard.md#search), and [Bookmarks and notes](keyboard.md#bookmarks-and-notes).
- `goto` (string): a place to go to. It accepts a line number (`"12"` or `"line 12"`), a percentage (`"50%"` or `"50 percent"`), a character offset (`"char 120"`), `"start"` (also `"top"` or `"beginning"`), or `"end"` (also `"bottom"`). Headings (`"heading 2"`) are not accepted here.

When both are given, `goto` is used.

The result is the new position, the same shape as the `position` method's result. The move is also announced, usually by saying the text you arrived at.

Errors: `-32602` "Not a navigation action" for an action from another group, and "Not a go-to target" for a `goto` value it cannot understand.

Some actions in these groups open a prompt or a list, for example `find` and `list_bookmarks`. With `navigate`, the client only hears the announcement. Use the `action` method for those instead, so the prompt or list comes back in the result.

### read: read aloud

It needs an open document. Parameters:

- `what` (string, optional): what to read. The default is `cursor`.
  - `cursor`: read on from the cursor until you stop it.
  - `document`: read from the start of the document.
  - `paragraph`: read on from the start of the current paragraph (the `replay_paragraph` action).
  - `sentence`, `line`, `word`, `character`: read just that one unit at the cursor, without moving.
  - `selection`: read the selected text. With nothing selected you hear "No selection."
- `from` (number, optional): a character offset to move to before reading.

The result is `{playback}`, the playback state right after the request: `"reading"`, `"paused"`, or `"stopped"`. For a short unit it may already be `"stopped"`. Then `playback` and `position` notifications follow as the reading goes on.

Errors: `-32602` "Cannot read" and the word you gave, for anything not in the list above.

### pause, resume, and stop

Parameters: none. The result of each is `{playback}`.

- `pause` pauses reading. It does nothing unless reading is going on.
- `resume` carries on after a pause. It does nothing unless reading is paused.
- `stop` stops reading.

`pause` and `resume` need an open document. `stop` works without one.

### search: find text

It needs an open document. Parameters:

- `pattern` (string, required): the text to find.
- `regex` (true or false, optional): treat the pattern as a regular expression. The default is false.

Matching follows the same rules as Find in the reader. In testing, a lowercase pattern also matched a capitalized word.

The result is an object with:

- `matches`: every match, each `{start, end, line}` (character offsets and a line number starting at 1).
- `current`: the index in `matches` of the match the cursor moved to, or null.

The cursor moves to the first match at or after it, wrapping round to the top when needed. You hear, for example, "Match 1 of 2: The nucleus".

### text: get the document's text

It needs an open document. Parameters:

- `start` (number, optional): the first character offset. The default is 0.
- `end` (number, optional): the offset just after the last character. The default is the end of the document.

Offsets past the end are treated as the end, and an `end` before `start` gives empty text. The result is `{text, start, end}`. The text is the same canonical text that `tw text` prints and that textweaver reads aloud.

### action: run any command by name

This is how a client does anything a key can do. Parameters:

- `id` (string, required): any action id from the [keyboard reference](keyboard.md), such as `add_bookmark`, `go_to`, `toggle_edit_mode`, `save`, or `quit`. It can also be one of the notes commands: `add_note`, `list_notes`, `next_note`, `previous_note`, `toggle_highlight`, `list_highlights`, `rename_bookmark`, or `delete_bookmark`.
- `confirm` (true or false, optional): the answer to give if the action asks a yes-or-no question first, as `quit` and `delete_note` do. `true` answers yes and `false` answers no.

The result is an object with:

- `status`: the status message after the action, the same text as its announcement.
- `effects`: what the action asked the client to show. Each effect is `{type, params}`:
  - `prompt`, with `params` `{label, purpose}`: a question that needs typed text, such as a Find or Go to box. `label` is what to show or say. `purpose` says what the answer is for, in lowercase with underscores: `find`, `go_to`, `open`, `command_palette`, `save_as`, `table_size`, `image_path`, `replace_find`, `replace_with`, `note_text`, `edit_note`, `rename_bookmark`, `export_settings`, `import_settings`, `citation_locator` (the page for a citation being inserted), `reference_identifier` (a DOI or ISBN to add), `import_references` (a file of references), or `template_title` (the title of a new document from a template). Answer it with `answer`.
  - `list`, with `params` `{title, items}`: a list to choose from, such as bookmarks or notes. `items` is a list of strings. Pick one with `choose`.
  - `quit`, with empty `params`: the app quit. The server stops after sending this answer.
- `pending`: when the action asked a yes-or-no question and you did not give `confirm`, this is `{action, question}`, for example `{"action": "quit", "question": "Quit textweaver? y or n"}`. Otherwise it is null. Answer later with another `action` call that has `confirm`, or with `cancel` for no.

The notes commands return `status` and `effects` but no `pending`.

The reader writes files in the background (saves, bookmarks, notes, positions). The server waits for those writes before it answers, so when an `action` such as `save` or `add_bookmark` returns, the file is on disk.

Errors: `-32602` "No action" and the id, for an id that does not exist, and "confirm must be true or false".

### answer, choose, and cancel: reply to a prompt or list

- `answer` takes `text` (string, required): the text typed into the open prompt. For a Go to prompt, for example, `"5"` goes to line 5.
- `choose` takes `index` (number, required): the item of the open list to pick, starting at 0.
- `cancel` takes no parameters. It closes the open prompt or list, or answers no to a waiting yes-or-no question. You hear "Cancelled."

Each returns `{status, effects}`, the same as `action` without `pending`.

### list_state and list_key: move through a list as the reader does

Since Wave 3 the list shown and its focused item are kept by textweaver itself, the same for the terminal reader, the GUI, and a client. These methods let a client move through a list with the reader's own keys and hear the same "item, 2 of 5" announcements.

- `list_state` takes no parameters. It returns the list shown as `{title, items, selected, filter}`, or null when no list is shown. `selected` counts from 0; `filter` is the text typed so far in a list that filters as you type (the outline, the citation picker, the settings), else null.
- `list_key` takes `key` (string, required): `up`, `down`, `page_up`, `page_down`, `home`, `end`, `left`, `right`, `enter`, `escape`, `backspace`, `delete`, `rename`, or one character. A character filters a list that filters, chooses by a list's own letter (`s`, `d`, `c` in Save, Discard, Cancel), or moves to the next item starting with it; a space marks an item (a favourite voice). `left` and `right` change a value in the settings list. It returns `{status, effects, list}`, where `list` is the list after the key, as `list_state` gives it.

### prompt_state and prompt_key: type into a prompt

- `prompt_state` takes no parameters. It returns the prompt open as `{label, purpose, text, caret}`, or null. `caret` counts characters from the start of `text`.
- `prompt_key` takes either `key` (string): one character, or `backspace`, `delete`, `left`, `right`, `home`, `end`, `up` and `down` (earlier answers to the same prompt, or command palette matches), `tab` (completes a command name or a file path), `kill_to_start`, `kill_to_end`, `delete_word_back`, `enter`, or `escape`; or `text` (string): the whole text of the prompt at once. It returns `{status, effects, prompt}`.

`answer` still works: it answers the open prompt with the text you give.

### settings_schema, get_setting, and set_setting: change settings

- `settings_schema` takes no parameters. It returns every setting in `settings.toml`, in order, as an array of objects with `path` (such as `"speech.rate"`), `section`, `label`, `help`, `kind`, `default`, and `internal` (true for the few that textweaver keeps for itself). By `kind`:
  - `"toggle"`: on or off.
  - `"number"`: with `min`, `max`, `step`, and `unit` (such as `"words per minute"`).
  - `"choice"`: with `choices`, a list of `{value, label}`, and `open` (true when other values may be typed, such as a theme of your own).
  - `"text"`: with `optional` (true when it may be unset, with null).
  - `"list"`: a list of texts, such as library folders.
  - `"table"`: names and values, such as pronunciations; edit these in `settings.toml`.
- `get_setting` takes `path` (string, required). It returns `{path, value, spoken}`: the value as JSON, and as it reads aloud ("265 words per minute").
- `set_setting` takes `path` (string, required) and `value` (any JSON; null puts the default back). The value is checked against the setting's type; a number outside its range is set to the nearest value in range. The change takes effect at once (the voice, the theme, the keys) and is saved before the answer. It returns `{path, value, spoken}`.

Errors: `-32602` "There is no setting" and the path, or a sentence saying why the value does not fit.

### shutdown and exit: finish

- `shutdown` takes no parameters and returns null. It saves the reading position and settings. After it, every request except `exit` fails with error `-32003`.
- `exit` takes no parameters. Send it as a notification (with no `id`). The server stops and closes its output. If you did not call `shutdown` first, `exit` saves anyway. If you send `exit` with an `id`, you get a null result first.

Closing the server's standard input also stops it, and saves the position too. So a client that is killed or crashes does not lose the reader's place.

The server does not poll on a timer. The speech engine wakes it as each word is heard, so `position` notifications go out at once; otherwise it looks for work about four times a second.

## Notifications from the server

These arrive without an `id`:

- `announcement`, with `{text, priority}`: something the app said, the same thing a screen reader user of the terminal reader would hear. `priority` is `"polite"` or `"assertive"`. Assertive ones are urgent, such as errors, and should interrupt.
- `playback`, with `{state}`: reading started (`"reading"`), paused (`"paused"`), or ended (`"stopped"`).
- `position`, with `{start, end, line}`: the word being spoken right now, as character offsets and a line number starting at 1. Use it to highlight or scroll. One arrives for each word the speech engine reports.

`initialize` also lists `prompt`, `list`, and `quit` as notifications. In this version the server does not send them on their own. They arrive only inside the `effects` of the result of `action`, `answer`, `choose`, and `cancel`, as described under [action](#action-run-any-command-by-name).

## Errors

An error answer looks like this:

```json
{"error":{"code":-32001,"message":"No document is open; call open first"},"id":3,"jsonrpc":"2.0"}
```

The codes JSON-RPC defines:

- `-32700`, parse error: the message is not JSON. The message starts "Parse error:" and the `id` is null.
- `-32600`, invalid request: the message is not a JSON-RPC 2.0 request, for example `jsonrpc` is missing. The message is "Not a JSON-RPC 2.0 request". An empty batch (`[]`) gives "Empty batch".
- `-32601`, method not found: "No method" and the name you sent.
- `-32602`, invalid parameters: a parameter is missing or has the wrong type or value. The message says which, for example "path is required and must be a string" or "index must be a number".

The server's own codes:

- `-32001`, no document: the method needs an open document. "No document is open; call open first".
- `-32002`, open failed: the document could not be opened. The message names the file and the reason.
- `-32003`, shut down: a request arrived after `shutdown`. "The server is shutting down; send exit".

## Versions

`initialize` reports `protocol`, which is `1`. The design record, [ADR-0015](adr/0015-json-rpc.md), sets the rule: adding methods, notifications, parameters, or result fields keeps version 1, so a client must ignore fields it does not know. Renaming or removing anything, or changing what something means, raises the version. Check `protocol` when you connect, and refuse to go on if it is a version you do not know.

The server sends the fields of each object in alphabetical order. Do not depend on the order.

## A complete example session

This is a real session. The client started the server with this command:

```bash
tw serve --stdio --no-speech
```

The document was a small Markdown file, `notes.md`:

```markdown
# Cell biology

Cells are the smallest units of life. Every living thing is made of cells.

## The nucleus

The nucleus holds the cell's DNA. It controls growth and division.
```

Each message went on one line. The answers are shown here with line breaks added so they are easier to read. The real folder of the file is replaced with `C:\Users\you\Documents`.

### Say hello

The client asks for the server's version.

```json
{"jsonrpc": "2.0", "id": 1, "method": "initialize"}
```

The server answers with its name, version, protocol, and the lists of methods and notifications.

```json
{
  "id": 1,
  "jsonrpc": "2.0",
  "result": {
    "methods": ["initialize", "open", "status", "position", "navigate", "read", "pause", "resume", "stop", "search", "text", "action", "answer", "choose", "cancel", "list_state", "list_key", "prompt_state", "prompt_key", "settings_schema", "get_setting", "set_setting", "shutdown", "exit"],
    "notifications": ["position", "playback", "announcement", "prompt", "list", "quit"],
    "protocol": 1,
    "server": "textweaver",
    "version": "0.1.0-alpha.3"
  }
}
```

### Open the document

The client opens `notes.md`. Backslashes in a Windows path are doubled inside JSON.

```json
{"jsonrpc": "2.0", "id": 2, "method": "open", "params": {"path": "C:\\Users\\you\\Documents\\notes.md"}}
```

First comes the announcement, as a notification.

```json
{"jsonrpc": "2.0", "method": "announcement", "params": {"priority": "polite", "text": "Opened Cell biology."}}
```

Then the answer, with the document and the starting position, the word "Cell" at the very start.

```json
{
  "id": 2,
  "jsonrpc": "2.0",
  "result": {
    "document": {
      "dirty": false,
      "editing": false,
      "format": "markdown",
      "length": 169,
      "lines": 7,
      "path": "C:\\Users\\you\\Documents\\notes.md",
      "title": "Cell biology"
    },
    "position": {
      "char": 0,
      "column": 1,
      "line": 1,
      "percent": 0,
      "word": {"end": 4, "start": 0, "text": "Cell"}
    }
  }
}
```

### Move to the next sentence

The client moves by one sentence.

```json
{"jsonrpc": "2.0", "id": 3, "method": "navigate", "params": {"action": "next_sentence"}}
```

The server announces the sentence it arrived at.

```json
{"jsonrpc": "2.0", "method": "announcement", "params": {"priority": "polite", "text": "Cells are the smallest units of life."}}
```

The answer is the new position: line 3, character 14, the word "Cells".

```json
{
  "id": 3,
  "jsonrpc": "2.0",
  "result": {
    "char": 14,
    "column": 1,
    "line": 3,
    "percent": 8,
    "word": {"end": 19, "start": 14, "text": "Cells"}
  }
}
```

### Read that sentence aloud

The client asks for the sentence at the cursor to be read.

```json
{"jsonrpc": "2.0", "id": 4, "method": "read", "params": {"what": "sentence"}}
```

Reading starts.

```json
{"jsonrpc": "2.0", "method": "playback", "params": {"state": "reading"}}
```

The answer says reading is going on.

```json
{"id": 4, "jsonrpc": "2.0", "result": {"playback": "reading"}}
```

A position notification says which word is being spoken. With a real voice there is one for every word. With `--no-speech` there is only this one.

```json
{"jsonrpc": "2.0", "method": "position", "params": {"end": 19, "line": 3, "start": 14}}
```

Reading ends.

```json
{"jsonrpc": "2.0", "method": "playback", "params": {"state": "stopped"}}
```

### Ask where the cursor is

The client asks for the position.

```json
{"jsonrpc": "2.0", "id": 5, "method": "position"}
```

Reading one sentence did not move the cursor.

```json
{
  "id": 5,
  "jsonrpc": "2.0",
  "result": {
    "char": 14,
    "column": 1,
    "line": 3,
    "percent": 8,
    "word": {"end": 19, "start": 14, "text": "Cells"}
  }
}
```

### Run an action: add a bookmark

The client runs the `add_bookmark` action, the same as pressing **m** in browse mode in the terminal reader.

```json
{"jsonrpc": "2.0", "id": 6, "method": "action", "params": {"id": "add_bookmark"}}
```

The announcement.

```json
{"jsonrpc": "2.0", "method": "announcement", "params": {"priority": "polite", "text": "Bookmark mark1 set at 8 percent."}}
```

The answer: no effects and no question waiting.

```json
{"id": 6, "jsonrpc": "2.0", "result": {"effects": [], "pending": null, "status": "Bookmark mark1 set at 8 percent."}}
```

### Run an action that asks for text: go to

The client runs `go_to`, which opens a prompt.

```json
{"jsonrpc": "2.0", "id": 7, "method": "action", "params": {"id": "go_to"}}
```

The prompt's label is announced.

```json
{"jsonrpc": "2.0", "method": "announcement", "params": {"priority": "polite", "text": "Go to line, percent, start, or end"}}
```

The answer carries the prompt as an effect.

```json
{
  "id": 7,
  "jsonrpc": "2.0",
  "result": {
    "effects": [
      {"params": {"label": "Go to line, percent, start, or end", "purpose": "go_to"}, "type": "prompt"}
    ],
    "pending": null,
    "status": "Go to line, percent, start, or end"
  }
}
```

The client answers the prompt with line 5.

```json
{"jsonrpc": "2.0", "id": 8, "method": "answer", "params": {"text": "5"}}
```

The server announces where it went.

```json
{"jsonrpc": "2.0", "method": "announcement", "params": {"priority": "polite", "text": "53 percent, line 5: The nucleus"}}
```

The answer.

```json
{"id": 8, "jsonrpc": "2.0", "result": {"effects": [], "status": "53 percent, line 5: The nucleus"}}
```

### Finish

The client asks the server to save and get ready to stop.

```json
{"jsonrpc": "2.0", "id": 9, "method": "shutdown"}
```

The answer is null.

```json
{"id": 9, "jsonrpc": "2.0", "result": null}
```

The client sends `exit` as a notification. There is no answer. The server closes its output and ends with exit code 0.

```json
{"jsonrpc": "2.0", "method": "exit"}
```

The next time this document is opened, the announcement is "Opened Cell biology. Resumed at" and the saved percentage.

## A minimal client in Python

This client uses only Python's standard library. It starts the server without speech, opens the file you name, moves to the next sentence, prints every announcement, and finishes cleanly. It reads one line at a time and skips notifications until the answer to its request arrives.

```python
import json
import subprocess
import sys

# Start the server as a child process. Use "tw" if it is on your PATH.
server = subprocess.Popen(
    ["tw", "serve", "--stdio", "--no-speech"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    text=True,
    encoding="utf-8",
)
next_id = 0


def call(method, params=None):
    """Send one request and return its result, printing announcements."""
    global next_id
    next_id += 1
    request = {"jsonrpc": "2.0", "id": next_id, "method": method}
    if params is not None:
        request["params"] = params
    server.stdin.write(json.dumps(request) + "\n")
    server.stdin.flush()
    while True:
        line = server.stdout.readline()
        if not line:
            raise RuntimeError("the server stopped")
        message = json.loads(line)
        if message.get("method") == "announcement":
            print("Announcement:", message["params"]["text"])
        if message.get("id") == next_id:
            if "error" in message:
                raise RuntimeError(message["error"]["message"])
            return message["result"]


info = call("initialize")
print("Connected to", info["server"], info["version"], "protocol", info["protocol"])
opened = call("open", {"path": sys.argv[1]})
print("Opened", opened["document"]["title"], "with", opened["document"]["lines"], "lines")
position = call("navigate", {"action": "next_sentence"})
print("Now at line", position["line"], "column", position["column"])
call("shutdown")
server.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "exit"}) + "\n")
server.stdin.flush()
server.wait()
```

Save it as `client.py` and run it with a document:

```bash
python client.py notes.md
```

With the example document, opened for the first time, it prints:

```text
Connected to textweaver 0.1.0-alpha.3 protocol 1
Announcement: Opened Cell biology.
Opened Cell biology with 7 lines
Announcement: Cells are the smallest units of life.
Now at line 3 column 1
```

This client only reads while it waits for an answer. A real client, such as an editor plugin, should read the server's output all the time, on its own thread or with an event loop, because `position` and `playback` notifications keep arriving while speech goes on.

## If something goes wrong

- **`tw serve` stops at once with "needs a transport".** Add `--stdio`.
- **Nothing comes back.** Check that each message ends with a newline, or has a correct `Content-Length` header, and that your client flushes its output after writing. Check that the message has `"jsonrpc": "2.0"`. A message without an `id` is a notification and never gets an answer.
- **Error -32001, "No document is open".** Call `open` first, or start the server with `--open`.
- **Error -32002 when opening.** The path is wrong or the file cannot be read. A relative path is taken from the folder the server was started in, so send a full path when you can.
- **Error -32602, "Not a navigation action".** `navigate` takes only movement, Speech Cursor, search, and bookmark actions. Use `action` for everything else.
- **Error -32003.** You already called `shutdown`. Send `exit` and start a new server.
- **The user hears two voices.** Both textweaver and the user's screen reader are speaking. Start the server with `--no-speech` and pass the announcements to the screen reader. See [the screen reader guide](screen-readers.md).
- **A prompt or list opened, but the client was not told.** Actions run through `navigate` or `read` do not return their effects. Run them with `action`, then use `answer`, `choose`, or `cancel`. `cancel` always closes whatever is open.
- **Highlighting is off by a few characters in JavaScript or VS Code.** Positions count Unicode characters, and those tools count UTF-16 units. Convert before highlighting.
- **No speech, and an announcement says the backend is not available.** Run `tw backends` to see which engines work on this computer, and see [the troubleshooting guide](troubleshooting.md).
- **You need more detail.** The server writes nothing but protocol messages to standard output, and nothing to standard error. Problems are written to the log file, `textweaver.log`, in the state folder (with `--home`, that is the `data\state` folder inside it). Set the `TEXTWEAVER_LOG` environment variable to `debug` before starting the server to log more.

## See also

- [ADR-0015: JSON-RPC server](adr/0015-json-rpc.md): the design decision behind `tw serve`.
- [Architecture](dev/architecture.md): how the server shares the app core with the terminal reader and the GUI.
- [Keyboard reference](keyboard.md): every action id you can pass to `action` and `navigate`.
- [Using textweaver with a screen reader](screen-readers.md): `--no-speech` and working with JAWS, NVDA, VoiceOver, and Orca.
- [Troubleshooting](troubleshooting.md): the log file and common problems.
- [Documentation index](README.md)
