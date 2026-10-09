# Bookmarks, notes, and highlights

This guide covers the three ways to mark what you read: bookmarks (named places), notes (your own text attached to a passage), and highlights (passages marked in color). It is for students and anyone who studies with textweaver. It also covers where they are kept, how to list them from the command line, and how to take them to Obsidian.

Keys are the terminal defaults. Most are single browse keys, which work while reading. Where the window uses a different key, this guide says so. With single-key shortcuts turned off (**F9**), run these commands from the command palette (**F2**) by the names given here.

## Bookmarks

A bookmark is a named place in a document.

### Add a bookmark: m

Press **m**. The bookmark goes on the word being read, or on the word at the cursor. It is named `mark1`, `mark2`, and so on, using the first free name. You hear "Bookmark mark1 set at 42 percent." If a bookmark is already on that word, you hear "Bookmark", its name, "is already here." The window also has **Ctrl+M**.

### Move between bookmarks: b and Shift+B

- **b**: the next bookmark.
- **Shift+B**: the previous bookmark.

You hear "Bookmark", the name, and a preview of the text. After the last one, textweaver wraps to the first and says "Wrapped." With no bookmarks you hear "No bookmarks."

### List bookmarks: Shift+M

Press **Shift+M**. You hear "Bookmarks", the count, then "Enter goes to one, Delete deletes it, F2 renames it." Each item says the name, the line, the percentage, and the start of the text.

In the list:

- **Enter** goes to the bookmark.
- **F2** renames it. The prompt says "New bookmark name, Enter keeps it". Type the new name and press **Enter**. You hear "Bookmark mark1 renamed to", then the new name. A name another bookmark already uses is refused.
- **Delete** deletes it at once, without asking. You hear "Bookmark", the name, "deleted." The list stays open on the next bookmark.
- **Escape** closes the list.

The command palette also has `rename_bookmark` and `delete_bookmark`. Each acts on the bookmark at the cursor, or opens the list when there is none there.

## Notes

A note is your own text attached to a passage: the selection, or the sentence at the cursor.

### Add a note: a

1. To attach the note to more or less than one sentence, select the text first with **Shift** and the arrow keys. Otherwise the note goes on the sentence being read, or the sentence at the cursor.
2. Press **a**, or **Alt+N**, which also works in edit mode and with single-key shortcuts off. The prompt says "Note".
3. Type the note and press **Enter**.

You hear "Note added on:", then the start of the passage.

Words starting with `#` in a note become tags. For example, "Check this for the #exam" is tagged `exam`. Tags are stored in lower case, without the `#`. When a note has tags you hear "Note added with tags exam on:" and the passage.

The note also keeps a copy of the passage it was attached to, up to 120 characters. That copy is its anchor. It keeps the note meaningful if the text later changes.

### Move between notes: e and Shift+E

- **e** or **F12**: the next note.
- **Shift+E** or **Shift+F12**: the previous note.

In the classic keys (`preset = "classic"` under `[keyboard]`), **Alt+Down** and **Alt+Up** step through notes as before. By default they move by sentence.

You hear "Note 2 of 5:", the note, then "On:" and the passage. After the last note, textweaver wraps to the first and says "Wrapped." With no notes you hear "No notes."

These keys step through notes only, not highlights.

### Hear that a note is here

While textweaver reads, it signals each note's passage once, as the reading reaches it: a short rising two-tone sound, when the voice can play tones. At normal verbosity and above, the status line also shows the note, for example "Note: Check this for the exam", without interrupting the reading.

When a word move (Left or Right) takes you into a note's passage, you hear the word and then "Has a note:" and the start of the note, at normal verbosity and above.

### Export a study sheet

Press **F2** for the command palette and type `export study sheet`. textweaver writes your notes and highlights as a Markdown file next to the document, named after it: `essay.md` gives `essay-study-sheet.md`. You hear how many notes and highlights went in, the file's name, "Open it? y or n.", and the folder.

The study sheet is grouped by the headings of the document, in order, so it follows the structure of what you read. Each passage is quoted, with your note under it:

```markdown
# Study sheet: Essay

Exported from textweaver on 2026-09-26.

## Methods

- > We measured things carefully.

  Check the method (tags: exam)
```

Highlights are listed the same way, with their color. A new document that was never saved has no folder yet; its study sheet goes to the folder textweaver was started in.

### List notes: Shift+A

Press **Shift+A**. The window also has **Ctrl+Shift+N**. You hear "Notes", the count, then "Enter goes to a note, Delete deletes it, F2 edits it, Space opens its links. C makes a card." Each item says the note, the line, and the passage, then its links when it has some.

In the list:

- **Enter** goes to the note.
- **F2** edits the note. You hear "Editing note:" and its text. The prompt says "Edit note, Enter keeps it". Type the new text and press **Enter**; tags are read again from the new text. You hear "Note updated." Enter on an empty prompt leaves the note as it was.
- **Delete** asks "Delete this note? y or n". Press **y** to delete it; you hear "Note deleted:" and the start of the note. Press **n**, **a**, or **Escape** to keep it; you hear "Kept." and the list comes back.
- **Space** opens the note's links (see [Links between notes](#links-between-notes)).
- **C** makes a study card from the note (see [Study with cards](#study-with-cards)). Because **C** makes a card, it does not jump to a note starting with C.
- **Escape** closes the list.

This list shows notes only. Highlights have their own list.

### Links between notes

A note can link to other notes, in this document or in another document in your library, as star's knowledge graph did. Each link has one of ten types: conflicts with, supports, is an example of, cites, contradicts, defines, extends, see also, precedes, and follows. textweaver shows links as lists, never as a picture, so every link reads well aloud and on a Braille display.

In the notes list, a note that has links ends with how many, for example "Links: 2 out, 1 in." "Out" counts the links this note makes; "in" counts the notes that link to it.

Press **Space** on a note in the notes list to open its links. From the document, the command palette's `note_links` opens the links of the note at the cursor. You hear "Links of", the note, the counts, and the keys. The list has:

- one row per link, the type first: "supports: Chapter 3 note". A note in another document adds its title: "cites: Renal clearance, in Pharmacology 2";
- "What links here", with how many notes link to this one;
- "Add a link".

In the links list:

- **Enter** on a link goes to the note it points to. A note in another document opens that document and goes to the note.
- **F2** on a link changes it: choose the type again, then the note.
- **Delete** on a link asks "Remove this link? y or n". Press **y** to remove it.
- **Typing** filters the links by type: type "sup" to keep the "supports" links. **Backspace** removes a letter, and **Escape** closes the list.

**Enter** on "What links here" lists the notes that link to this one, the type first: "supports this, from: Week 4 note". Enter goes to that note. Typing filters by type here too.

To add a link, press **Enter** on "Add a link":

1. Choose the type from the ten. Typing filters the list.
2. Choose the note to link to. The list has this document's other notes, then "A note in another document", which lists the library's documents that have notes, and then that document's notes.

You hear "Linked:" with the type and the note, and the links list comes back. Adding the same link twice says "Already linked".

Links are saved with the note, so they sync to your other computers with it, and `tw vault export` writes them as Dataview fields. "What links here" looks at this document and every library document that has notes; textweaver reads those when you first open a links list after opening a document.

### List links from the command line: tw notes links

```bash
tw notes links essay.md
```

`tw notes links` prints each note of the document that has links, then one line per link: its links out ("supports: Chapter 3 note"), then what links to it ("cites this, from: Week 4 note, in Pharmacology 2"). Add `--type supports` to list one type, and `--json` for a program or a script. It only reads; it never changes anything.

## Highlights

A highlight marks a passage, as a highlighter pen does on paper.

### Highlight: y

1. Select the text with **Shift** and the arrow keys, or leave nothing selected to highlight the sentence at the cursor.
2. Press **y**.

You hear "Highlighted:" and the start of the passage. At high verbosity you also hear the percentage.

Press **y** again on a highlighted passage, with nothing selected, to remove the highlight. You hear "Highlight removed:" and the passage.

### Highlight colors

Highlights made in textweaver are yellow. There is no key to choose another color yet.

Highlights imported from star, or from a synced folder, can have other colors. textweaver knows five by name: yellow, green, cyan, pink, and orange. Other colors are kept and shown by their code. On screen a highlight is marked by the theme's highlight style, which never relies on color alone.

### List highlights: Shift+Y

Press **Shift+Y**. You hear "Highlights", the count, then "Enter goes to one, Delete removes it. C makes a card." **C** makes a study card from the highlight ([Study with cards](#study-with-cards)). Each item says the passage, the line, and the color name.

In the list, **Enter** goes to the highlight, **Delete** asks "Remove this highlight? y or n" and removes it on **y**, and **Escape** closes the list. In the command palette this command is `list_highlights`.

## Delete a note or highlight at the cursor: Delete

When reading, move to a note or a highlight and press **Delete**. textweaver asks "Delete this note or highlight? y or n". Press **y** to delete it, or **n**, **a**, or **Escape** to keep it. A note under the cursor is deleted before a highlight. With nothing there you hear "No note or highlight here."

Delete in the notes and highlights lists asks the same way. Only the bookmarks list deletes at once, without asking.

## Study with textweaver

textweaver has three tools for studying what you read: a self-test made from your notes and highlights, study cards made from the same marks and kept with your grades, and recall prompts that stop reading at the end of each section. All are optional, and none changes your document.

### What the research says, and what it does not

A review of ten common study techniques rated practice testing (retrieving material from memory) and spacing study over time as high in utility, and rereading, highlighting, and summarizing as low (Dunlosky and colleagues, 2013). Highlighting on its own is therefore a weak way to study; the self-test turns your highlights into questions instead. In two experiments with prose passages, students who were tested on a passage remembered more of it two days and one week later than students who reread it, although rereading did better after five minutes and left students more confident (Roediger and Karpicke, 2006).

In a study of adaptive retrieval practice with 118 participants, the slower answers of participants with dyslexia came from typing them, not from memory, and answering aloud removed the gap (Wilschut, Sense, and van Rijn, 2024). This is why the self-test lets you answer aloud through dictation.

In an experiment with a 21-minute video lecture in four parts, undergraduates who answered short tests between the parts reported mind wandering on 19 percent of probes, against 39 percent for those who restudied the material between parts (Szpunar, Khan, and Schacter, 2013). Recall prompts bring a similar pause for recall to reading aloud.

These studies were done mostly with readers without disabilities, and with tests that experimenters wrote. textweaver's self-test and recall prompts have not themselves been studied. Treat them as ways to practice recalling, not as a promised gain.

### Test yourself from the study sheet

Press **F2** for the command palette and type `self test`, or choose **Self-test** in the **Bookmarks and notes** menu. textweaver makes a list of prompts from the same notes and highlights as the [study sheet](#export-a-study-sheet), in document order, and says, for example, "Self-test, 12 prompts. Enter shows each answer. Space to answer aloud."

- A note becomes a prompt in your own words, with the section it is in: "Check the method (in Methods)". Its answer is the passage the note is on.
- A highlight becomes "What did you highlight in Methods?" Its answer is the highlighted passage.
- Before the first heading, the section is the document's title.

Each prompt is said and its answer is hidden. In the list:

- **Up** and **Down** move between prompts, which are said with their position, such as "2 of 12".
- Answer silently or aloud, then press **Enter**. You hear "Answer:" and the passage. The prompt now shows its answer, and the list stays on it. **Enter** again says the answer again.
- **Space** answers aloud (see below).
- **Escape** closes the list. Opening the self-test again starts with every answer hidden.

textweaver never scores your answer. You compare it with the passage yourself. A note or highlight on an empty passage is left out, and with no notes or highlights you hear "No notes or highlights to test."

### Answer aloud

On a prompt, press **Space** and say your answer. You hear "Answer aloud now. Space to stop." Press **Space** again when you are done. When the last words are transcribed, you hear "You said:" and your words, then "Enter shows the answer." Press **Enter** to hear the answer and compare. Pressing **Enter** while still recording first finishes the recording and reads it back.

Answering aloud uses the same Whisper model as [dictation](dictation.md), in any mode, and types nothing into the document. The first time, textweaver offers to download the model if it is missing; open the self-test again once it is in place. If no words are heard, you hear "No answer heard. Space to try again." In a version without dictation, you hear that answering aloud needs it.

### Study with cards

Study cards turn the passages you marked into questions you answer from memory, and keep a record of how each answer went. The self-test asks every note and highlight afresh each time; cards are kept, so you can come back to them, reverse them, remove the ones you no longer need, and see the last grade you gave each one.

#### Make cards

Press **F2** for the command palette and type `make cards`, or choose **Make cards** in **Study cards**, a submenu of the **Bookmarks and notes** menu. textweaver makes cards from every note and highlight in the document, and says, for example, "Cards made: 5 new, 12 in all."

- **A highlight** becomes a fill-in-the-blank card. The question is the sentence the highlight is in, with the highlighted words replaced by the word "blank", which is how it is read aloud and shown: "The kidneys blank the blood." The answer is the highlighted words. A highlight that covers its whole sentence leaves nothing to fill in, so it asks "What did you highlight in Renal clearance?" instead.
- **A note on a passage** becomes a question card. The note is the question and the passage is the answer, so a note written as a question ("What does the loop of Henle do?") makes the best card. A note with no text, or one on an empty passage, makes no card.
- **A heading** becomes a recall card when a note or highlight is in its section: "What does “Renal clearance” say?" The answer is the section's first sentence.

To make a card from one note or highlight, open the notes list (**Shift+A**; the window also has **Ctrl+Shift+N**) or the highlights list (**Shift+Y**), move to it, and press **C**. You hear "Card made:" and its question.

Making cards again is safe. A card is tied to the note, highlight, or heading it came from, so making cards again after you edit a note updates that card's question and answer and keeps its grades; it never makes a second copy. A card stays when you delete its note or highlight; remove it from the Cards list (below) if you no longer want it.

#### Study the cards

Type `study cards` in the palette, or choose **Study cards** in the same submenu. You hear "Study cards, 12 cards. Enter shows each answer, 1 to 4 grade it. Space to answer aloud." and then the first question. In the list:

- Answer silently or aloud, then press **Enter** to hear "Answer:" and the answer.
- Grade how well you recalled it, in your own judgment, with a number key or by name in the palette:
  - **1**, Again (`grade_again`): you did not recall it.
  - **2**, Hard (`grade_hard`): you recalled it with effort.
  - **3**, Good (`grade_good`): you recalled it.
  - **4**, Easy (`grade_easy`): you recalled it at once.
- After a grade you hear the grade and the next card: "Good. Card 4 of 12. Question: ...". After the last card you hear "Done: all 12 cards graded." and the list closes.
- **Space** answers aloud, as in the self-test ([Answer aloud](#answer-aloud)): your words are read back before you reveal the answer, and textweaver never judges them. The grade is always yours.
- **R** reverses a question card: the passage is asked and the note becomes the answer. Press **R** again to put it back. The card stays reversed the next time you study it. Fill-in-the-blank and recall cards cannot be reversed.
- **Up** and **Down** move between cards without grading; **Escape** closes the list. The session waits: a grade from the palette or the submenu grades the card you were on and opens the list again on the next card ("Good. Study cards, card 4 of 12."), and **Study cards** starts again from the first card.

Each grade is stored with the time you gave it. This version does not yet use the grades to decide when a card should come back; every session asks every card, in document order.

#### The Cards list

Type `list cards` in the palette, or choose **Cards** in the same submenu, to hear every card with its kind, its question, and its last grade: "Fill in the blank: The kidneys blank the blood., last graded Good". **Enter** goes to the card's source in the document, where its note or highlight is now. **Delete** asks "Remove this card and its grades?" and removes it on **y**.

#### Where cards are kept

Cards are kept on this computer beside your notes, one file per document (`cards/` in textweaver's data folder), and written in the background. They do not sync between computers yet. Because a card is tied to its source, making cards on another computer from the same synced notes gives the same cards, without their grades.

### Recall prompts at section ends

Turn on **Recall prompts** in Settings (`[reading] recall_prompts`, off by default). When continuous reading stops at the end of a section, it asks you to recall it, naming the section it just read: "Say what you remember from Renal clearance. Ctrl+Space to go on." The key you hear is your read key. Say or think what you remember, then press the key, and reading goes on with the next section.

Where reading stops is set by **Stop at section end** ([Reading and moving around](reading.md#stop-at-the-end-of-a-section)). If that is **never**, recall prompts stop at the next heading of any level; set it to **next chapter** for longer sections. Recall prompts do not record what you say.

### A study routine

One way to combine these:

1. Read a section aloud with recall prompts on. At each prompt, say what you remember before going on.
2. While reading, add a note (**a**) where a passage answers a question you expect, written as that question, and highlight (**y**) what you want to recall.
3. Later, open the self-test, or make cards and study them. Answer each question before you reveal it, aloud or silently, and add a note where you missed something.
4. Study the cards again on another day rather than rereading the chapter.

### Sources

- Dunlosky, J., Rawson, K. A., Marsh, E. J., Nathan, M. J., and Willingham, D. T. (2013). *Psychological Science in the Public Interest*, 14(1). [doi:10.1177/1529100612453266](https://doi.org/10.1177/1529100612453266)
- Roediger, H. L., and Karpicke, J. D. (2006). *Psychological Science*, 17(3). [doi:10.1111/j.1467-9280.2006.01693.x](https://doi.org/10.1111/j.1467-9280.2006.01693.x)
- Szpunar, K. K., Khan, N. Y., and Schacter, D. L. (2013). *Proceedings of the National Academy of Sciences*, 110(16). [doi:10.1073/pnas.1221764110](https://doi.org/10.1073/pnas.1221764110)
- Wilschut, T., Sense, F., and van Rijn, H. (2024). *Topics in Cognitive Science*, 17(1). [doi:10.1111/tops.12769](https://doi.org/10.1111/tops.12769)

## How marks move when you edit

Bookmarks, notes, and highlights are tied to the text, not to a line number. When you edit the document (see [Writing and editing](editing.md)), they move with the text around them:

- Text typed before a mark pushes it along.
- A note whose whole passage is deleted is kept. It stays at the place of the deletion, and its anchor still holds the old passage.
- A highlight whose whole passage is deleted is removed.
- A bookmark whose word is deleted moves to the place of the deletion.

### When the file changes in another program

Your document may change while textweaver is closed: you edit it in Obsidian, `git pull` brings a new version, or another program rewrites it. textweaver keeps the text each mark was on: about 40 characters for your reading position and each bookmark, and the passage for each note and highlight. When you open the changed document, it finds each mark again:

1. its text near where it was;
2. its text anywhere else in the document, the nearest place winning, for a paragraph that moved;
3. the most similar text nearby, for a small edit inside the passage itself;
4. failing all three, the same share of the way through the document. Such a mark is marked as not found.

Marks still on their text are left alone. textweaver says once what moved and what it could not find, for example: "The file changed; 3 bookmarks were moved to match, 1 bookmark could not be found and is marked."

## Where they are stored

Everything about one document is kept in one file in the state folder: the reading position, the history of jumps, the bookmarks, the notes, and the highlights. The state folder is `state` inside the data folder. The [library guide](library.md) says where the data folder is on each system.

The file is named after the document's file name plus a code made from its full path, for example `essay.md-9f3c01a2b4d5e6f7.json`. So two files with the same name in different folders keep separate marks. Moving or renaming a document starts it with no marks.

Bookmarks, notes, and highlights are saved as soon as you change them. The reading position is saved when you quit, when you open another document, and every 30 seconds while it changes. The saving happens in the background, so a slow disk never holds up a key; quitting waits for it, and says "Still saving" if it takes more than a moment.

While you are in edit mode, marks are saved when you leave edit mode.

If a state file cannot be read, for example after a sync conflict or a hand edit, textweaver does not overwrite it. It renames it to `<name>.corrupt-<time>.bak` and starts that document with no marks, so you can recover them from the copy. See [Troubleshooting](troubleshooting.md).

## List marks from the command line: tw marks

```bash
tw marks essay.md
```

`tw marks` prints a document's saved reading position, its bookmarks, its notes, and its highlights, each with its percentage, character position, line, and saved time, plus the text of that line. If the document is in a library folder, it also prints the position in the folder's older progress file. It only reads; it never changes anything.

For a program or a script, print JSON instead:

```bash
tw marks essay.md --json
```

The JSON also has the document's state key and its history of jumps.

## Export notes as references: tw marks --to

Your notes and highlights can go into a reference manager such as Zotero, or into a BibTeX file, as star's notes export did. Each note and each highlight becomes one record:

- the title and author are the document's;
- the passage you noted or highlighted is the record's abstract;
- your note is the record's note, followed by "Cites doe2020." when the note has a citation key; a highlight's note is its color, for example "Highlighted, yellow.";
- your tags are its keywords;
- its date is the day you made the note.

Choose the format after `--to`: `bibtex`, `biblatex`, `ris`, or `json` (CSL-JSON, which Zotero and Pandoc read):

```bash
tw marks essay.md --to ris --out essay-notes.ris
```

Without `--out`, the records are printed. Keys are made from the file name: `essay-note-1`, `essay-note-2`, `essay-highlight-1`. In CSL-JSON each record also says where it is, for example "34 percent".

## Export to an Obsidian vault

`tw vault export` writes a document's notes and highlights into an Obsidian vault as Markdown notes. Name each document with `--document`:

```bash
tw vault export C:\Users\me\Vault --document essay.md
```

[The vault guide](vault.md) explains the options, the notes it writes, and importing from a vault.

## Sync between computers

With sync set up (Tools, Sync, Set up sync), your bookmarks, notes, highlights, and reading places travel to your other computers through a folder you choose, such as one kept in step by Syncthing or a USB stick. A document is recognized by its contents, so it syncs wherever it is on each computer. [Syncing between computers](sync.md) explains it, including what you hear when the same note was edited on two computers: the newest edit wins, and the older text is kept in this computer's backup of replaced notes.

Without sync, a document in a library folder still carries its reading place to other computers through a small file in that folder, `.textweaver/progress.json`, as older versions did; the notes, highlights, and bookmarks stay on the computer where you made them. With sync on, that file is only read. [The library guide](library.md#the-older-place-sync-through-a-library-folder) explains it.

## If something goes wrong

- **"Nothing here to attach a note to."** The cursor is on an empty line. Move to text, or select some.
- **A mark is missing after reopening.** The document may have moved or been renamed; marks follow the full path. Check with `tw marks` on the old path.
- **A note is in the wrong place after an edit outside textweaver.** textweaver looks for the note's passage again when the file changed (see [When the file changes in another program](#when-the-file-changes-in-another-program)). If the passage was rewritten or deleted, the note could not be found: it is marked, and put at the same share of the way through the document. The note's anchor still shows the passage it was made on.
- **The keys do nothing.** Single-key shortcuts may be off. Press **F9**, or use the command palette names: `add_bookmark`, `list_bookmarks`, `next_bookmark`, `previous_bookmark`, `add_note`, `list_notes`, `next_note`, `previous_note`, `highlight_selection`, `list_highlights`, `note_links`, `delete_note`, and `self_test`.

## See also

- [Reading and moving around](reading.md): selecting text, and the history of jumps.
- [Writing and editing](editing.md): edit mode, where marks move with your edits.
- [The library](library.md): library folders, "Continue reading", and where the state folder is.
- [Syncing between computers](sync.md): notes, highlights, bookmarks, and places on your other computers.
- [Obsidian vaults](vault.md): exporting notes and highlights to Obsidian.
- [Keyboard reference](keyboard.md): every key in both frontends.
- [ADR-0002: Text model](adr/0002-text-model.md): how positions and marks move with edits.
- [Documentation index](README.md)
