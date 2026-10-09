# The textweaver window

textweaver has two readers: the terminal reader, `textweaver`, and a window. They share everything that matters: the documents, the keys, the settings, the notes, and the voices. This page covers what is different about the window.

The window is written entirely in Rust (Xilem's Masonry widgets, Vello drawing, Parley text, and AccessKit for screen readers). It reads and it edits; see [Editing](#editing).

In a release package the program is `textweaver-gui`; see [the window package](install.md#the-gui). If you build the window yourself, `cargo build` makes `textweaver-xilem` instead; use that name where this guide says `textweaver-gui`.

## Starting it

```sh
textweaver-gui path/to/document.md
```

With no document, it opens empty and says which key opens one (Ctrl+O), at every announcement level but off.

The first time textweaver runs (no settings yet), the window says a short welcome with the five keys that get you reading (open, play and pause, stop, the command palette, and help), and on Windows the keys that show the hidden menu bar (Alt or F10), before the hint that no document is open. Then come at most three steps, one at a time, each skipped with Escape: the language list, only when the system's language is not one of the six textweaver speaks (when it is, textweaver uses it without asking); if a screen reader is running, textweaver reads documents aloud and leaves its messages to the screen reader, and says so in one sentence with the key that changes it (it never asks); and the optional components, with nothing chosen and nothing downloaded unless you choose. The terminal reader does the same. Ask again about first-run choices (Tools) brings the last two back at the next start. If an earlier run left unsaved work, it offers it back. The speech engine starts in the background, so the window is ready at once; Restart Speech (Shift+F8) starts it again, and it restarts by itself once if it stops.

Useful options:

- `--read`: start reading once the document is open.
- `--theme NAME`: use this theme instead of the saved one, for this run.
- `--voice ID` and `--backend ID`: the voice or speech engine for this run (see `tw voices` and `tw backends`).
- `--no-speech`: run silently.
- `--announce live` or `--announce uia`: how messages reach your screen reader, for this run (see [Announcements](#announcements)).
- `--select-spoken`: while reading, select the spoken word instead of only moving the cursor to it (see [The spoken word](#the-spoken-word)).
- `--home FOLDER`: keep settings and reading positions in this folder, as `TEXTWEAVER_HOME` does.
- `--list-menus`: show the menus as a list inside the window (F10), as on Linux, instead of the system's menu bar.
- `--graphics API`: draw with one graphics API only: `vulkan`, `dx12` (Windows), `metal` (macOS), or `gl`; `auto`, the default, lets the graphics library use every one it finds. On the development machine `vulkan` used about 26 MB less memory, but this depends on your graphics driver. To keep a choice, put `graphics = "vulkan"` in the `[gui]` section of `settings.toml`.
- `--log` or `--log-file PATH`: write what the window announces and does, for a bug report. Every 200 frames it also writes a "frame times" line with the median, the 95th percentile, and the worst time to draw a frame, so a change to drawing can be measured in a real window.

`textweaver-gui --help` lists every option.

On Windows the window opens with no console window beside it. Started from a terminal, `--help`, `--version`, and errors still appear in that terminal. PowerShell does not wait for a windowed program, so its output may come after the next prompt; `textweaver-gui --help | Out-Host` waits for it. Started from a shortcut or File Explorer, a startup error is shown in a message box, and `--log-file PATH` keeps it in a file too.

## What is in the window

From top to bottom:

1. **The menu bar** (Windows and macOS): File, Edit, View, Reading, Speech, Tools, and Help; see [Menus](#menus).
2. **The header,** a banner with five buttons: Open, Font, Edit (or Finish editing), Settings, and Commands. The document's title is the window's title.
3. **The Contents or Notes panel,** only when you show one (Ctrl+1 or Ctrl+2), to the left of the document; see [The Contents and Notes panels](#the-contents-and-notes-panels).
4. **The document,** one control your screen reader reads as a document (on macOS, a read-only text area, which VoiceOver reads with its text commands). The cursor keys are your system's own (see [Cursor keys](#cursor-keys)), with Shift to select. Ctrl+C (Command+C on macOS) copies the selection, and says what it copied. Every other key goes to textweaver's keymap, so the browse keys of NVDA and JAWS work here too: `h` for the next heading, `t` for the next table, `k` for the next link, and so on.
5. **The RSVP strip,** only while RSVP is on (Alt+Shift+R). It shows one word at a time under the document, so it never covers the text or the cursor.
6. **The toolbar,** named "Reading": Play or Pause, Stop, Previous sentence, Next sentence, Slower, and Faster.
7. **The status bar:** the last message, then what the terminal's title line shows: the reading state, "line 3 of 40, 7%", the accessibility mode, the rate, and the speech engine.

Every button has a key, shown on screen with its name, for example "Open… (Ctrl+O)". Your screen reader reads it as the button's shortcut key: NVDA and JAWS say it after the name when their setting for reporting shortcut keys is on (in NVDA, Object Presentation, "Report object shortcut keys"). The name itself is only the label, "Open", so it stays short. The key comes from the keymap, so a key you change in `keymap.toml` shows here too, and F1 and the command palette list every key. While single-key shortcuts are on, a button shows its single key ("Play (Space)"); press F9 to turn them off, and the buttons show their chords instead ("Play (Ctrl+Shift+Space)").

In a narrow window the buttons wrap onto more rows rather than leave the window. Below 800 pixels wide (at 100 percent; a 1366 by 768 laptop at 200 percent is 683 wide), the header and the toolbar fold into one flat bar above the document, the buttons hide their keys on screen (your screen reader still says them), and the panel goes above the document below 600 pixels. Header and Toolbar in the View menu hide either bar; their commands keep their keys and menu items, and the settings `gui.header` and `gui.toolbar` remember the choice on this computer.

Tab and Shift+Tab move between the document and the buttons, in the order they are on screen. F6 and Shift+F6 move between the window's regions, as in other Windows programs: the header, the panel (when shown), the document, and the toolbar, landing on the first control of each. Dialogs (settings, lists, the command palette) open inside the window and take the focus; Escape closes them and puts you back in the document.

## Menus

The window has the same menus as the terminal reader, built from the same list of commands, so both always offer the same things under the same names: File, Edit, View, Reading, Speech, Tools, and Help. Every command is in a menu, with its key beside it. The keys come from the keymap, so a key you change in `keymap.toml` shows in the menus too.

- **Windows:** a standard menu bar, which NVDA and JAWS read as any program's. Alt, or F10, enters it; Alt with a menu's underlined letter opens that menu (Alt+F for File); Alt+Space still opens the window's system menu. In a menu, each item is read with its key, for example "Open, Ctrl+O", and a setting you can turn on or off is read as checked or not checked. Escape leaves the menus and puts you back where you were.
- **macOS:** the menu bar at the top of the screen, with each command's key as its keyboard shortcut. VoiceOver reaches it with Control+Option+M.
- **Linux:** F10 shows the menus as a list inside the window, as the terminal does: "Menus, 1 of 7, File". Enter or Right opens a menu, a letter moves to the item with that letter, Enter runs a command, Left or Backspace goes back up, and Escape closes.

**Hiding the menu bar (Windows).** The menu bar is hidden by default and takes no room until you want it; the first-run welcome says so ("Alt or F10 the menus"). To keep it shown, turn off "Hide the menu bar" in Settings, under Window (`auto_hide_menu = false` in `[gui]`); a value you saved earlier is kept. Alt, F10, or Alt with a menu's letter shows it and enters it as before, so NVDA and JAWS still say "menu bar" or the menu's name; it hides again, silently, when the menu closes. Every Alt key the keymap uses still works. The bar also appears for a moment when you press Alt for one of those keys. While a dialog is open, close it before using the menus. This setting has no effect on Linux, where the menus are already the F10 list and take no room, or on macOS, whose menu bar is at the top of the screen.

Choosing a command in a menu runs it as its key would, and it joins the recent commands the command palette lists first (F2 with nothing typed). A few commands that only mean something in a terminal are left out of the window's menus (see [What only the terminal reader does](#what-only-the-terminal-reader-does)). File, Browse files opens textweaver's file browser in the window's list dialog (see [Reading](reading.md#from-the-file-browser-file-browse-files)); its keys work there as in the terminal, and the Say Status key previews the focused row. Batch conversion, audio export, and dictation are in the menus in the default build.

## Opening a document

Open (Ctrl+O) shows your system's own file chooser: on Windows the standard Open dialog, which NVDA and JAWS know. It lists the documents textweaver reads; choose "All files" in the file type list to see everything. It starts in the folder of the document you have open. When you choose a file, the dialog closes, textweaver says "Opened" and the title, and the focus is back in the document. Escape cancels.

To type a path instead, press Ctrl+Shift+G (Open Path): a one-line prompt where Tab completes the path, Up and Down recall earlier ones, and F4 opens textweaver's own file browser, whose choice fills the prompt for Enter to confirm. If the system's file chooser cannot open (on Linux it needs the XDG desktop portal), textweaver says so and shows this prompt instead.

The other commands that ask for a file use the system's file chooser too, each titled for what it does:

- **Save as** opens the system's Save dialog in the document's folder, offering the document's name. The system asks before replacing a file.
- **Insert image** (while editing) lists images (PNG, JPEG, GIF, SVG, WebP and BMP), starting in the document's folder.
- **Import references** lists BibTeX, RIS and CSL-JSON files.
- **Import profiles** and **Export profiles** (in the Profiles list) use TOML and JSON; export offers `textweaver-profiles.toml`.

Commands that ask for a folder use the system's folder chooser, titled with what the folder is for: audio export's "Another folder", the folders of a batch conversion, and the sync folder (Set up sync). It starts in the document's folder.

When a chooser closes, the focus returns to the document, or to the next step of the command (the batch's formats, the sync computer's name). Escape cancels and says "Cancelled." If a chooser cannot open, textweaver says so and asks another way: a file's path in the one-line prompt (with F4 for the file browser), or a folder in the file browser's list.

## Text size and font

- **Ctrl+Plus** (Ctrl+=, or the plus key on the number pad): larger text.
- **Ctrl+Minus**: smaller text.
- **Ctrl+0**: back to the standard size, 14 points.
- **Ctrl+D**, or the Font button: the font list. The fonts that come with textweaver are first, marked "built in": Atkinson Hyperlegible Next, Atkinson Hyperlegible Mono, and OpenDyslexic. Then your installed fonts. It is a list like textweaver's others: a letter moves to the next font starting with it, F1 says the list's name and size again, and Enter uses the font at once.

Each change is said, for example "Text size 18 points." or "Font: OpenDyslexic.", and saved in `[reading_aids.font]` (see [Settings](settings.md)). The size steps one point at a time around the usual sizes and more quickly above 16 points, from 8 up to 72 points. The Settings dialog changes the same settings, under "Reading aids".

## Cursor keys

The document moves its cursor with your system's keys, and Shift with any of them selects:

- **Windows and Linux:** Left and Right by character, Up and Down by line, Ctrl+Left and Ctrl+Right by word, Ctrl+Up and Ctrl+Down by paragraph, Home and End to the start and end of the line, Ctrl+Home and Ctrl+End to the start and end of the document, Page Up and Page Down by screen.
- **macOS:** Left and Right by character, Up and Down by line, Option+Left and Option+Right by word, Option+Up and Option+Down by paragraph, Command+Left and Command+Right to the start and end of the line, Command+Up and Command+Down to the start and end of the document (Home and End too), Page Up and Page Down by screen. No cursor key uses Control, so VoiceOver's keys (Control+Option) are never taken.

In browse mode, Home and End go to the ends of the line, as in any document window; in the terminal they go to the ends of the document, which Ctrl+Home and Ctrl+End do here. In Speech Cursor mode (Alt+Shift+S, or Reading, then Speech Cursor, in the menus; Tab moves the focus here, as in any window, and is the terminal's key), Up and Down read the next and previous line and Page Up and Page Down move by paragraph, as in the terminal.

## Keys

The window uses the same keymap as the terminal reader, with a few chords the terminal cannot send. The [keyboard reference](keyboard.md) lists every key, with a column for the window. The ones you will use most:

- **Space** (browse) or **Ctrl+Shift+Space**: play or pause.
- **Escape**: stop.
- **Alt+Down** and **Alt+Up**: next and previous sentence.
- **Ctrl+O**: open a document with the system's file chooser. **Ctrl+Shift+G**: type its path instead.
- **Ctrl+Plus**, **Ctrl+Minus**, **Ctrl+0**: text size. **Ctrl+D**: the font list.
- **F11** and **Shift+F11**: faster and slower (or **+** and **-** in browse). In the window, Ctrl+= and Ctrl+- size the text instead of the rate.
- **Ctrl+Shift+V**: the voice manager (see [Voices](#voices)).
- **Ctrl+,**: settings.
- **F2**: the command palette, every command by its short name, with its key at the right edge ("Find next, F3"). Type to filter; Up and Down say each match; Tab or Ctrl+L moves to the list of matches, where your screen reader reads each with its place; F1 says what the selected command does, which is also each row's description; Enter runs one. With nothing typed, the commands you ran last from the palette or the menus come first, marked "recent".
- **The keyboard shortcuts list** (? in browse, or the Help menu) has the same rows. Type to filter it by name, key, or menu ("12 of 226 commands match."); Page Down and Page Up move by group; F1 on a row says what it does.
- **F1**: help. In a list, F1 repeats the list's introduction.
- **F3** and **Shift+F3**: the next and previous match of the last search (Ctrl+F), as in other Windows programs. The list of every key is on **?** (browse) and in the Help menu.
- **Alt+Shift+S**: Speech Cursor mode on or off. Tab moves the focus, as in any window.
- **Alt+End**: say the last message and the status. In a list, it repeats the list's introduction too.
- **Alt+'**: say the last message again.
- **Alt+O**: the outline. Type to filter the headings, Enter jumps to one.
- **Ctrl+Shift+N**: the notes list. Space on a note opens its links (see [Links between notes and tracked changes](#links-between-notes-and-tracked-changes)).
- **Ctrl+Shift+J**: the list of tracked changes and comments in a Word, OpenDocument, or RTF document.
- **Ctrl+1** and **Ctrl+2**: the Contents and Notes panels beside the document (see [The Contents and Notes panels](#the-contents-and-notes-panels)).
- **F6** and **Shift+F6**: the next and previous region: the header, the panel, the document, and the toolbar.
- **Ctrl+T** and **Ctrl+Shift+T**: next and previous table. **Ctrl+Alt+arrows** move by cell in a table.
- **k** and **Shift+K** (browse): next and previous link. **Alt+Shift+F** follows a link.
- **Alt+Shift+A**: the window's mode, one of two: "textweaver reads aloud" (documents in textweaver's voice, its messages for your screen reader) or "my screen reader reads" (textweaver is silent, and your screen reader reads the text). The **Speak textweaver's messages** setting (Settings, Window), off by default, has textweaver say its messages, typing and cursor moves too, for reading by ear without a screen reader; `--self-voicing` turns it on for one run. The View menu and the Accessibility mode setting show the same two names. In `settings.toml` the mode keeps the terminal reader's three values: "textweaver reads aloud" is saved as `"hybrid"`, and "my screen reader reads" as `"screen-reader"`.
- **F5**: the next color theme.
- **F9**: single-key shortcuts off or on.

## The Contents and Notes panels

A panel beside the document keeps the document's headings, or its notes, in view while you read. It is the same list the outline (Alt+O) and the notes list (Ctrl+Shift+N) show, with the same names for each row, so the panel and the list never disagree.

- **Ctrl+1** shows the Contents panel, and **Ctrl+2** the Notes panel, and moves the focus to it. Your screen reader says the list and the row with its place, for example "Contents, Methods, level 2, 3 of 12", and textweaver says "Contents open, 12 items." Pressed while you are in the panel, the same key closes it and puts you back in the document ("Contents closed."). Pressed in the document while the panel is shown, it moves the focus to the panel.
- **Up, Down, Home, End, Page Up, Page Down,** and a letter move in the list, as in any list.
- **Enter** moves the document to that heading or note, says where it is as the outline does, and keeps you in the panel, so you can try the next one. **Shift+Enter** moves the document there and puts you back in the document. **Escape** puts you back in the document without moving it.
- **Space,** in the Notes panel, opens the selected note's links as a list, exactly as Space does in the notes list. Closing that list returns you to the panel. When the Notes panel opens with notes in it, textweaver mentions this key once, after the count; the Contents panel has no such key.
- **F6** and **Shift+F6** move between the header, the panel, the document, and the toolbar.
- The row where the cursor is has a bar beside it, and your screen reader hears ", current" after its name. While you are in the document, the panel's selected row follows the cursor, so going to the panel starts where you are.
- The panel is a navigation landmark named "Contents" or "Notes".

The panel never takes the focus on its own: when the window opens with a panel, or you choose one in the settings, the focus stays where it was. The window remembers the panel you showed last (Settings, Window, "Panel beside the document"; `sidebar` in `[gui]`, one of `off`, `contents`, or `notes`). A document without headings shows "No headings." in the Contents panel; a PDF without headings lists its pages, as the outline does. In edit mode, the Contents follow the headings you type once they are parsed again, as the outline does.

The panel costs nothing while it is closed. While it is open, its rows are built again only when the document, its headings, or its notes change, not as the reading highlight moves.

## Links between notes and tracked changes

Two lists bring the structure around a document's text into the window: the links between your notes, which together form a small knowledge graph, and the tracked changes and comments that other authors left in a document. Neither is drawn as a picture. Each is an ordinary list dialog, the same list the terminal reader shows, and each row begins with what it means, so the first forty cells of a Braille line, or the first words your screen reader speaks, carry the substance of the row. Where a row is longer than the dialog is wide, the drawn row ends with an ellipsis, but your screen reader still reads the whole row. The terminal reader's guides describe both lists fully: [Links between notes](notes.md#links-between-notes) and [Tracked changes and comments](reading.md#tracked-changes-and-comments-ctrlshiftj-or-alta).

**A note's links.** Press Space on a note, in the notes list (Ctrl+Shift+N) or in the Notes panel (Ctrl+2), to open its links. Each link is a row that names its type before its target, for example "supports: Chapter 3 note". Two rows follow the links: "What links here", which lists the notes that link to this one, from this document and from every library document with notes, and "Add a link". In this list, Enter follows a link, opening the other document if the target lives there; F2 changes a link's type or target; and Delete removes a link after asking "Remove this link?". Adding a link asks first for one of the ten types, then for the target note. Typing in these lists filters them by type, and the dialog's title shows the filter so far; Backspace takes back one letter.

**Tracked changes and comments.** Ctrl+Shift+J (Command+Shift+J on macOS) lists every insertion, deletion, and move, and every comment thread, in document order, for example "Inserted: 'renal', by Ada Example, Tuesday, March 3, 2026". A date is said in full when the document records one, and "date not recorded" otherwise. Enter goes to the place in the text. On a change, A accepts it and R rejects it; Shift+A and Shift+R accept or reject every change by the same author. On a comment, F2 asks for a reply, Space marks the thread resolved (or open again), and Delete deletes the thread and its replies after a question. N adds a comment where a note would attach: to the selection, or to the text at the cursor. These letters act only with the case you type: a capital comes from Shift alone, so Caps Lock never turns a single acceptance into an acceptance of everything by one author.

Both questions, removing a link and deleting a comment, appear as the window's usual question dialog (see [Questions](#questions)): the focus starts on **No**, the first button says **Remove** or **Delete**, and Y and N answer as in the terminal. Answering no keeps the link or the comment and shows the list again.

## Editing

Ctrl+E, or the Edit button, turns edit mode on, as in the terminal reader: you edit the document's source (a Markdown file's Markdown; for other formats, the Markdown made from them, which Save stores as a new `.md` file). Ctrl+E again finishes, asking to save if there are changes.

In edit mode the document is a multi-line edit, so NVDA and JAWS switch to focus mode by themselves.

List items keep their indent by depth while you edit, as in reading, and bulleted items keep their bullet shape (a disc, a ring, then a square), so the nesting shows. A numbered item shows the number you typed. These are drawn only: the screen reader reads the source text, with its own dashes and numbers.

- **Typing** goes in at the cursor, and over the selection if there is one. Enter starts a new line (and continues a list). Backspace and Delete delete. Input methods and dictation work too.
- **Your screen reader echoes** what you type, and reads the cursor and the selection as they move. With **Speak textweaver's messages** on, textweaver says them itself, as the terminal does: typing as the typing echo setting says (Shift+F9 cycles it), the character, word, or line the cursor moves to, and what a Shift key added to the selection or took from it.
- **Copy, cut, and paste:** Ctrl+C copies the selection and Ctrl+X cuts it, each saying what it took; Ctrl+V pastes what is on your system's clipboard at the cursor (on macOS, Command with each).
- **Undo** is Ctrl+Z, **redo** Ctrl+Y or Ctrl+Shift+Z, and each says what it undid. The editing keys are the terminal's: Ctrl+B bold, Ctrl+I italic, Ctrl+K a link, Ctrl+Alt+1 a heading, and the rest in the [keyboard reference](keyboard.md). Ctrl+S saves.
- **Tab** types a tab, or in a table moves to the next cell (Shift+Tab to the previous one), as in the terminal. **Ctrl+Tab** moves the focus out of the document, to the buttons.
- **Markdown lint:** Ctrl+F8 moves to the next lint problem (a skipped heading level, a mixed list marker, a bare web address) and says it; Ctrl+Shift+F8 goes back. Ctrl+F7 moves to the next grammar problem and Ctrl+Shift+F7 to the previous one, as in the terminal ([Editing](editing.md#grammar)).
- **Spell check:** Alt+M moves to the next misspelled word and selects it, so your screen reader says it and textweaver spells it. Type to replace it, or press Alt+J for suggestions. Alt+Shift+M goes back. Misspelled words are also marked on screen with a dotted underline, shortly after you stop typing (in documents up to a million characters).
- **Citations while writing:** Alt+C opens the citation picker. Type part of an author or title to filter, Enter inserts it, and textweaver asks for a page or other locator. Alt+Shift+D adds a reference by DOI or ISBN.
- **Export and preview:** the command palette (F2) has Export as a web page, PDF, Word, EPUB, and braille (BRF), each written next to the document, and Preview in the browser, which reloads when you save.
- Ctrl+Tab and Ctrl+Shift+Tab move between the document and the buttons, so you are never trapped in the edit; Ctrl+E is always the way out of edit mode.

## The spoken word

While textweaver reads, the spoken word has its own background color, and the cursor sits at its start, so your screen reader and Braille display follow the reading. This is the default, chosen after the first screen reader session. `--select-spoken` selects the word instead, for anyone who prefers it.

The document window: a very long document is shown a few hundred pages at a time, around where you are. When reading reaches the edge, the window moves on by itself. The text that stays keeps its place, so your screen reader does not lose it.

## Questions

When textweaver asks a yes-or-no question (a voice to download, after its size and license; a voice to remove; a file changed on disk), the window shows it as a small dialog: the question is the dialog's name, without the "y or n" the buttons already show, so your screen reader says it, and the focus is on **Yes**; a question that deletes, removes or replaces something names its verb on the button ("Delete") and starts on **No**, so Enter is the safe answer. Press **Y** or **N**, as in the terminal, or Tab to **No** and press Enter. Escape answers no. Any other letter asks the question again.

A question that deletes, removes, or replaces something (a note, a highlight, a link between notes, a comment, a profile, a voice, a downloaded component, a file that already exists) starts on **No** instead, so pressing Enter keeps things as they are. Its first button says what it does, **Delete**, **Remove**, or **Replace**, rather than "Yes". The keys are the same: **Y** goes ahead, **N** and Escape keep things.

## Closing the window

Closing the window with its close button or Alt+F4 quits as Quit (Ctrl+Q) does, without asking "Quit textweaver?". If you have unsaved edits, it first asks the same question Ctrl+Q asks: save, discard, or cancel. Cancel keeps the window open with your edits. An open dialog closes first, as Escape would, so the question is the one in front.

## Voices

**Ctrl+Shift+V** opens the voice manager, a dialog named "Choose a voice" with every voice of every engine on this computer: Eloquence, SAPI 5 (with the OneCore voices), DECtalk, eSpeak NG, Piper, and Apple's voices on macOS. The voices are the terminal's, with the same names, filters, and favorites. The focus starts in the list, named "Voices", on the voice in use.

From top to bottom, Tab moves through:

- **Language** and **Engine**, two buttons that filter the list. Each press shows only the next language or engine, then all of them again, and says what is shown, for example "3 voices: English, all engines." The button's name says the filter: "Language: English".
- **The list of voices.** Each says its name, language, engine, and tags; "favorite" for a favorite and "current" for the voice in use. Favorites come first. A Piper voice you can download says its size and license. A favorite from another computer that no engine here has is listed last as "not on this computer". The other engines' voices are listed in the background when the manager first opens; they join the list as they arrive, and you hear how many came.
- **Use voice** (Enter in the list): uses the voice and speaks a sample. On a voice of another engine, textweaver switches engine. On a voice you can download, textweaver reads its license and size, then asks before downloading.
- **Preview** (Alt+End in the list, the Say Status key): speaks a sample in the voice without choosing it, starting with the voice's name. A voice of another engine is previewed by starting that engine for the sample and closing it again. A voice you have not downloaded, or one not on this computer, says why it cannot be heard.
- **Favorite** (Space in the list): makes the voice a favorite, or stops it being one.
- **Remove** (Delete in the list): removes a downloaded Piper voice, after a yes. On a voice that cannot be removed, the button is unavailable: it says "Remove, unavailable", your screen reader says "unavailable", and its description gives the reason, "Only downloaded Piper voices can be removed." It stays in the Tab order, so you can land on it and hear why.
- **Fetch the Piper voice list from the internet**, when it can be fetched: asks first, then downloads the list of Piper voices, about 250 KB.
- **Close** (Escape).

Every button has its key as its shortcut and a short description, which NVDA and JAWS read as for any button. Enter or Space presses the focused button. The Help key and the Say Status key (Alt+End) work with the focus on a button too, as they do in the list: Alt+End previews the voice focused in the list.

## Language

The window's own labels (the buttons, the settings dialog, the hints) follow the interface language, `[interface] language` in `settings.toml`, as textweaver's messages do. Change it in Settings, under "Interface", and the window relabels itself at once.

## Announcements

textweaver's messages ("Paused.", "Reading at 300 words per minute.") reach your screen reader in one of two ways:

- **A live region** (the default). NVDA and JAWS both speak it.
- **UI Automation notifications** (Windows only). Choose it in Settings, under "Window", or with `--announce uia` for one run.

The setting is `announce` in the `[gui]` section of `settings.toml`; see [Settings](settings.md#gui).

Messages said while the window starts ("Opened", the title, "Reading at") wait until your screen reader has asked for the window's contents, then are said once. Before, they could be lost when the window was quicker than the screen reader.

When the window takes the focus (Alt+Tab, a click), your screen reader says the window's title, which is the document's title and "textweaver", then the document. With **Speak textweaver's messages** on, textweaver says the document's title and its name in its own voice.

How much textweaver says about its own interface is yours to choose: `[accessibility] interface_announcements`, or Ctrl+F9 to step through off, minimal, normal, and full. The window's messages follow it as the terminal's do. Errors and the answers to what you asked (a count, the font you chose) are always said; a dialog closing, the hint that no document is open, and the first run's welcome are said from normal up. In the screen reader and hybrid modes it starts at minimal, since your screen reader already says what opens and closes.

## Reading aids

The window draws the same [reading aids](reading-aids.md) as the terminal, with the same keys:

- **Text spacing:** line height, paragraph spacing, and letter and word spacing, in `[reading_aids.spacing]`. The window uses the exact values; the terminal rounds them to whole rows and spaces.
- **The reading ruler** (Alt+Shift+U): off, the current line, or the ruler. The reading line gets a band with a bar at its start, the lines around it a paler band, and with `mask_outside` the rest is dimmed. It follows the cursor, and the spoken word while reading.
- **Bionic reading** (Alt+Shift+B): the start of each word in bold.
- **Difficult words** (Alt+Shift+J): underlined with a thick line, never marked by color alone.
- **Syllables** (Alt+Shift+Z): long words drawn split into syllables with a middle dot, "read·a·bil·i·ty", as in the terminal. The dot is only drawn: the words keep their letters, so your screen reader and Braille display read "readability", and the cursor and the spoken word stay where they were. The separator and when words are split are in `[reading_aids.syllable_options]`.
- **RSVP** (Alt+Shift+R, then Alt+Shift+P to play): one word at a time in its own strip under the document. The word before and after sit to its left and right. The marked letter is bold and underlined as well as colored. RSVP's nine places move the word left, center, or right in the strip.

**Notes, highlights, bookmarks, and search matches** are drawn too, each with a shape as well as a color, so no color carries it alone: your highlights have a solid line under them, text with a note a dashed line, a bookmark a bar before it, a search match a box around it, and the match at the cursor a heavier box. While you listen, the spoken sentence is underlined and the spoken word is bold, on top of their bands, as in the terminal. The bold is drawn in place, so the line never shifts as the word moves. Marks inside the sentence stay visible while it is read. The spoken word's and sentence's colors follow `[highlight] color` and `sentence_color`, as in the terminal.

**Exploring a formula** (Alt+Shift+X) works as in the terminal: Right and Left move to the next and previous part, Down goes into a part and Up out of it, Home and End go to the first and last, Space or Enter says it again, and Escape leaves. Any other key leaves the formula and does what it usually does.

All of these change only how text looks. Your screen reader reads the same text either way. The RSVP word is hidden from screen readers, so it is never spoken by itself. Beside it is a quiet status ("RSVP paused, word 120 of 900") that you can find with your screen reader's review or object navigation.

## Colors and high contrast

The window starts in the theme your settings choose, and follows your system's light or dark setting as the terminal reader does (`display.follow_os_theme`, on unless you picked a theme). The system's setting is read while the rest of the window starts, and not at all with `--theme` or a theme you picked. F5 moves to the next theme.

With Windows High Contrast on (Contrast themes in Windows 11), the window draws with your contrast theme's own colors: its page and text, its highlight for the spoken word, the selection, and the focus ring, and its link color. Turning it on or off applies at once. Marks that have a tint of their own in textweaver's themes (the reading ruler's band, notes, bookmarks, search matches) keep their shapes instead: the ruler's bar, the lines and boxes. The spoken sentence has no band then; its underline, in your contrast theme's text color, marks it. The focus ring stays visible: when the highlight color is too close to the page, the ring is drawn in the text color. `--theme`, or `follow_os_theme = false` in `[display]`, keeps textweaver's own colors.

On macOS and Linux, the system's increased-contrast setting chooses textweaver's high-contrast theme when the window starts.

The title bar and the menus follow the theme too: a dark theme such as Galaxy gets a dark title bar, and on Windows a dark menu bar and dark drop-down menus, whatever Windows' own light or dark setting; a light theme gets light ones. The theme's page color decides. Changing the theme (F5, or Settings) changes them at once. With Windows High Contrast on, Windows draws them in your contrast theme's colors instead. Dark menus need Windows 10 version 1809 or later; on older versions they stay light.

### The Colors dialog

View, then Colors (or File, Settings, Colors) opens every color textweaver lets you choose in one list: the spoken word's and sentence's highlights first, then the reading ruler, difficult words, syllable marks, misspellings, lint marks, search matches, the selection, the focus outline, links, headings, the status bar, notes, and bookmarks.

- **Left and Right** choose from named colors, said in words ("dark blue", "gold"). Blue and orange come first: a pair that stays distinct for red-green color blindness.
- **Enter** types a color name or a `#rrggbb` value.
- **Delete** puts the theme's own color back for that part. The **Reset all colors** button puts it back for every part.

Each row says its color and how well it stands out where it is drawn, as a ratio and a word, for example "blue, contrast 4.8 to 1, good". Below 3 to 1 the change is still made, and you are told it will be hard to see. A small sample of the color is drawn beside the value, but the words always say it. Changes apply at once, and every mark keeps its underline, weight, or symbol whatever its color, so color never carries meaning alone.

## Settings

Settings (Ctrl+,) opens a dialog: the sections on the left, the chosen section's settings on the right. Every change takes effect and is saved at once. A switch shows its value in words beside it ("on", "off"). With a mouse, the arrows beside a number or a choice step it the way they point: the left one back, the right one forward. It is built from the same list as the terminal's settings screen, so every setting is in both; [Settings](settings.md) describes each one.

### The Reading settings dialog

View, then Reading settings (or Tools, Reading settings) opens the settings you change most while reading, in one list with the same rows as the Settings dialog: the rate; the font, its size and weight; line height, paragraph spacing, word spacing and letter spacing; the line length; the theme; what the highlight covers and its word and sentence colors; the reading ruler and its mask; bionic reading; and syllables. Below the list:

- **Voices** closes the dialog and opens the voice manager.
- **WCAG spacing** sets the four spacings to the values WCAG names (line height 1.5, paragraph spacing 2, letter spacing 0.12, word spacing 0.16), and **Generous spacing** to wider ones (2, 2.5, 0.15, 0.3). It says which, once, and the rows show the new values.
- **Close** (Escape) closes it.

In the terminal, the same command shows the settings screen with only these settings, as View, Colors does for the colors. The line length is the window's own, so the terminal lists the other sixteen.

**Export settings** and **Import settings** are under File, then Settings (Alt+Shift+E and Alt+Shift+I). Export opens your system's Save dialog, offering `textweaver-settings.toml`; a name ending in `.json` writes JSON instead. It writes every setting and your key changes. Import opens the system's Open dialog for a TOML or JSON file, checks it, and then asks before changing anything, naming the first changes: "Import 12 changed settings from home.toml: Rate, Theme, Link color, and 9 more? y or n". Yes applies them at once and says what changed; no leaves everything as it was. If the system's file chooser cannot open, a prompt asks for the file's path instead.

## Size, place, and when something goes wrong

- **The window remembers its size and place** on this computer. It opens where it closed, and whether it was maximized. A place on a screen that is no longer connected is not used, so the window never opens where you cannot see it. This is kept for each computer and is not synced.
- **Text follows the system's text size.** Interface text grows with the Windows "Text size" setting or the GNOME text scaling factor.
- **The caret blinks as the system's does,** or not at all when the system says so.
- **The window draws opaque on every graphics API.** It asks for an opaque drawing surface, never one that blends with what is behind the window. Up to alpha.9, with `auto` on Windows (which picks Vulkan on an NVIDIA card) the menu bar was see-through, because the driver offered a blended surface and the window took it; `--graphics dx12` hid the problem, since Direct3D 12 offers only opaque surfaces for a window. `--log` names the surface on the "graphics adapter" line ("surface opaque").
- **The icon is the loom,** textweaver's logo, in the title bar, the taskbar, and on the programs in Explorer and their shortcuts (`textweaver-gui.exe`, `textweaver.exe` and `tw.exe` carry it). In Windows High Contrast the window uses the one-color loom, white on black. The Linux menu entry uses the same drawing.
- **The window asks for the integrated graphics adapter** when the computer has one, because it draws text as fast and saves the battery. To use the fast adapter instead, set the environment variable `WGPU_POWER_PREF` to `high`.
- **A log file.** The window writes warnings and errors to `textweaver.log` in the state folder, as the terminal reader does. [Troubleshooting](troubleshooting.md#the-log-file) says where it is.
- **After a failure,** the window saves your unsaved edits as a recovery copy, and saves your place and settings. At the next start it offers the work back (see [Recovering unsaved work](editing.md#recovering-unsaved-work)). Signing out, shutting down, or restarting does the same. On Linux and macOS, so does a termination signal: closing the terminal the window was started from, Ctrl+C in that terminal, or the system ending the program (SIGHUP, SIGINT, or SIGTERM). The window saves, then closes.
- **If graphics cannot start,** the window says so in words. Started from a shortcut, it shows a message box. See [The window does not open, or it is blank](troubleshooting.md#the-window-does-not-open-or-it-is-blank).

## What only the terminal reader does

Every command works in the window as in the terminal reader, from the same keys, the menus, and the command palette (F2), with the same lists, questions, and messages. A few commands only mean something in a terminal, and the window's menus and command palette leave them out. If a key for one is pressed in the window (`j` or Shift+J, say), the window says "This command works in the terminal reader." and does nothing else:

- `scroll_down` and `scroll_up`: the terminal scrolls its screen by lines. The window scrolls with the mouse wheel and keeps the cursor in view.
- `toggle_line_numbers`: line numbers are the terminal's margin, on F6 there. In the window, the status bar says the line, and Say Position (Shift+W) says it too; F6 moves between the window's regions instead.

The command-line tools (`tw vault`, `tw convert`, `tw library`, and the rest) are the same for both readers.

## For testers

- `--background` starts the window without taking the focus, off screen, with no taskbar button, for automated checks. On Windows the window is marked as one that accessibility tools must not activate (`WS_EX_NOACTIVATE`). UI Automation still activates it on the first control pressed through it; the window then hands the foreground straight back to the window that had it, and `--log` says so. Expect a moment's flicker of focus, not a lost one.
- A debug build writes Masonry's full trace log only when `MASONRY_DENSE_LOG_DIR` names a folder for it (for example `target\masonry-logs`); otherwise it writes none. It never goes to the system's temporary folder.
- `--backend paced` reads silently, timing words like a real engine.
- `crates/textweaver-xilem/tools/uia-report.ps1` reports what UI Automation sees (Windows), including the menu bar's seven menus and access keys and every menu item's text and key as the window's menu holds them; `-WindowEdge` reads past the document window's edge; `-Menus` also opens the first menu to read its items through UI Automation (opening a menu may bring the window to the front, so use it on a test machine). `tools/atspi-check.sh` does the same with AT-SPI on Linux.
- `--log` lists the menu items the window built, one per line, with each key after a tab.
- `--review-screenshots FOLDER` draws the review screenshots without a window: four themes at 100% and 200%, the dialogs, the reading aids, the reading ruler on its own, edit mode, the window with no document, the Contents and Notes panels, and the window at 960 by 540, 683 by 384 at 200%, and 420 by 320. Give it a folder outside `docs/`, such as `target/review-screenshots`; the full set is for review and is not kept in the repository. A curated set of eight, each picture with a description, is in [the window in pictures](window-in-pictures.md). It, `--screenshot`, and `--measure-frames` need the `screenshot` feature (in the default build); a build without it ends with an error that names the option, and no window opens.
- `--log` also says how the title bar and menus are drawn: "frame: dark", "frame: light", or "frame: system" (high contrast), and how the drop-down menus are drawn ("drop-down menus: dark", "light", or "light, because dark drop-down menus are unavailable" on a Windows that lacks a call dark menus need), at startup on the "menus: native" line and again on each change; and "menu bar shown" and "menu bar hidden" while `auto_hide_menu` is on.

### Checking the title bar and menus by hand

Colors cannot be heard, so the color steps need a sighted helper, a screenshot read by a color tool, or the `--log` lines above. Use a throwaway state folder (`--home %TEMP%\tw-frame`) and run the steps once with NVDA and once with JAWS.

1. **Dark theme, Windows light.** In Windows Settings, Personalization, Colors, choose Light. Start `textweaver-gui --log --home %TEMP%\tw-frame`. Expected: the log says "frame: dark"; the title bar, the menu bar, and an opened File menu are dark, with light text.
2. **Dark theme, Windows dark.** Choose Dark in Windows and start again. Expected: the same, dark.
3. **Light theme, Windows dark.** Press F5 until a light theme is chosen, such as Galaxy Light. Expected: "frame: light" in the log at once, and a light title bar, menu bar, and menus, though Windows is dark.
4. **Light theme, Windows light.** Expected: light, as in step 3.
5. **High contrast.** Turn on a Contrast theme (Left Alt+Left Shift+Print Screen). Expected: "frame: system", and the title bar and menus in the contrast theme's colors, whatever textweaver's theme. Turn it off: the frame follows the theme again.
6. **Menus still read.** In each step, Alt, then Down: NVDA and JAWS say "menu bar", then "File" and the first item with its key, as before. Escape twice leaves.
7. **Hiding the menu bar.** In Settings, under Window, turn on "Hide the menu bar". Expected: the bar disappears and nothing is said beyond the setting's own message. Then:
   - Alt alone: "menu bar" is said, as before, and the bar is shown. Escape: the bar hides again, silently.
   - F10: the same.
   - Alt+F, Alt+E, Alt+V, and each other menu's letter: that menu opens and is read.
   - Alt+Space: the window's system menu, as before.
   - A key the keymap gives Alt with another key (see the [keyboard reference](keyboard.md)) still does its command; the bar may appear for a moment and hides again.
   - Choose a command from a menu: it runs, and the bar hides.
8. **Turn it off.** Expected: the bar is back at once, and stays.

Please note which steps did not behave as expected, with the screen reader and its version, and the Windows version.

## See also

- [Keyboard reference](keyboard.md)
- [Reading aids](reading-aids.md)
- [Using textweaver with a screen reader](screen-readers.md)
- [ADR-0027: Xilem GUI](adr/0027-xilem-gui.md), [ADR-0028: the Xilem GUI after the first listening session](adr/0028-xilem-gui-after-the-session.md), [ADR-0033: the window after further accessibility testing, and edit mode](adr/0033-gui-session-2-and-edit-mode.md), and [ADR-0046: native menus in the window](adr/0046-native-menus-in-the-gui.md)
- [ADR-0043: menus and the palette from one model](adr/0043-menus-and-the-palette-from-one-model.md)
