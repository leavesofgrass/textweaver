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

Highlights are listed the same way, with their name. A new document that was never saved has no folder yet; its study sheet goes to the folder textweaver was started in.

To group the highlights by name instead, type `export study sheet by name` (or choose **Export study sheet by name** in **File**, **Export as**). That sheet has a section for each name in your palette, with its highlights in document order and the heading each falls under, then a section of your notes. It is saved as `essay-study-sheet-by-name.md`.

### Make a pocket review

A pocket review is your own marks as something to carry: the study sheet read aloud into an audio file for a walk or a bus ride, and a braille copy for a notetaker. It takes two commands after the study sheet:

1. Export the study sheet, as above. For `essay.md` this writes `essay-study-sheet.md`.
2. Read it into audio from a terminal:

   ```bash
   tw export-audio essay-study-sheet.md --out essay-review.mp3
   ```

   MP3, FLAC, Opus, and Ogg Vorbis need no other program; `essay-review.m4b`, an audiobook with a chapter for each heading of the essay, needs ffmpeg. In the reader, open the study sheet and use Export audio instead. See [audio export](audio-export.md#make-a-pocket-review).
3. For a braille copy, convert it:

   ```bash
   tw convert essay-study-sheet.md --to brf
   ```

The review holds only what you marked, in the order of the document, under its headings. It is a way to reach your marks without the whole text; hearing them again is still rereading, so test yourself as well (see [Study with textweaver](#study-with-textweaver)).

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

### Export the knowledge graph

The links of a single note answer a local question: what this idea supports, and what cites it. A literature review or an exam plan raises a global one, about the shape of everything you have connected across the semester. Exporting the knowledge graph writes every link in your library to one file, so that you can read the whole structure as a list or hand it to a tool that analyzes or draws networks.

In the reader, press **F2** for the command palette and type `export knowledge graph`, or choose **File**, **Export As**, **Export knowledge graph**. You hear how many links there are, then a list of formats. Press **Enter** on one, and textweaver writes `knowledge-graph` with that format's extension next to the open document (or, with no document open, in the folder textweaver was started in). You hear the file's name, "Open it? y or n.", and the folder. Press **y** to open it with the program your computer uses for that kind of file. If no note has a link yet, you hear "No links between notes to export." and nothing is written.

From the command line:

```bash
tw notes graph
tw notes graph --to json --out graph.json
tw notes graph --out graph.graphml
```

`tw notes graph` prints the Markdown list. `--to` chooses the format, `--json` is short for `--to json`, and `--out FILE` (or `-o FILE`) writes a file instead of printing; without `--to`, the extension of the file chooses the format, so `graph.graphml` is written as GraphML. `--home DIR` reads the notes kept under another data folder. The command only reads your notes; it never changes them.

The graph holds every note that links to another note or is linked from one, across the open document and every library document that has notes. A note with no links in either direction is left out, because it has no place in a network. A link whose target note no longer exists, because the note was deleted or its document left the library, is kept and points to a node labeled "Note not found", so an export never hides a broken link. A link type textweaver does not know, written by a newer version, is exported under its stored name.

The formats, in the order the list offers them:

- **Markdown list** (`md`). A heading for each note, with its document's title, followed by one line per link, the type first: "supports: Chapter 3 note, in Biology". The links the note makes come first, then the links made to it: "cites this, from: Week 4 note, in Pharmacology 2". This is the text equivalent of every other format, and the one to read with a screen reader or a Braille display; each line carries its meaning in the first words.
- **JSON** (`json`). An object with `nodes` and `edges`, the shape star used. Each node has an `id` (the note's own id), `doc` (the document's path), `title` and `label` (the note's text, shortened), and `missing: true` when the note was not found. Each edge has `src` and `dst` (node ids), `rel_type` as stored (`SUPPORTS`), `spoken` as it reads aloud (`supports`), and the link's comment as `note` when it has one. Gephi and Cytoscape import this shape with their JSON importers, and it suits scripts.
- **DOT** (`dot`), the language of Graphviz. Each note is a box labeled with its text; each link is an arrow labeled with its type.
- **GraphML** (`graphml`), the XML graph format that Gephi, Cytoscape and yEd open directly. Nodes carry the label, document path and title; edges carry the stored type, the spoken type and the comment.
- **Mermaid** (`mermaid`, written as `.mmd`), a flowchart that Markdown editors and code hosts that support Mermaid draw from text. It carries an accessible title and a description that names the Markdown list as its text equivalent.
- **PlantUML** (`plantuml`, written as `.puml`), a diagram for PlantUML and the editors that render it.
- **CSV edge list** (`csv`). A header row, `source,type,target`, then one row per link: the linking note, the type as it reads aloud, and the linked note, each note with its document's title. Spreadsheets open it, and Gephi can import it as an edge table.

Each format escapes note text by its own rules, so a quotation mark, an ampersand or a bracket in a note never breaks the file: quotes are escaped in DOT, characters are written as XML entities in GraphML, Mermaid uses its entity codes and generated node names, and the CSV file quotes fields as the CSV standard asks. A CSV field that a spreadsheet would run as a formula, such as a note beginning with an equals sign, starts with an apostrophe instead, so opening an export never runs anything. The words of the Markdown list are in English, as `tw notes links` prints them.

The drawn formats are for tools that lay out a network as a picture. textweaver itself never draws the graph; the lists in the reader and the Markdown list are how it presents links, and they hold the same information as any picture made from the other files. Concept extraction, which star used to suggest links from the words of a document, is not part of textweaver.

## Highlights

A highlight marks a passage, as a highlighter pen does on paper.

### Highlight: y

1. Select the text with **Shift** and the arrow keys, or leave nothing selected to highlight the sentence at the cursor.
2. Press **y**.

You hear "Highlighted," the name, and the start of the passage: "Highlighted, important: The cell membrane". At high verbosity you also hear the percentage. **y** highlights with the first name in your palette.

Press **y** again on a highlighted passage, with nothing selected, to remove the highlight. You hear "Highlight removed:" and the passage.

### Highlight with a name: Alt+1 to Alt+5

Each highlight has a name from your highlight palette, such as "important" or "ask the professor". The name is what you hear, and what you sort and collect by later.

- **Alt+1** to **Alt+5** highlight with the first five names, in reading mode, in the terminal reader and in the window. On a passage that already has that name, the same key removes the highlight; on a passage with another name, it changes the name, and you hear "Highlight changed to define:" and the passage.
- **Highlight with a name** lists every name in the palette, up to eight, with how many highlights have it, its shape, and its color: "important, 3 highlights, underline, yellow". Enter highlights with the name you choose. It is in the **Bookmarks and notes** menu, under **Highlights**, and in the command palette as `highlight_as`.

The keys are in the keymap as `highlight_name_1` to `highlight_name_5`, so you can change them (see [Keyboard](keyboard.md)). star used Ctrl+Shift+1 to 5 for its colors. textweaver cannot use those: Shift with a digit types a different symbol on each keyboard layout, and terminals do not send Ctrl with a digit.

### Highlight colors

The highlight palette is `[[highlight.palette]]` in `settings.toml`. It starts with star's five colors, each with a study name and a shape of its own:

| Name | Color | Shape |
|---|---|---|
| important | yellow | underline |
| define | green | double underline |
| question | cyan | brackets |
| example | pink | dotted underline |
| review | orange | bold |

Each entry has a `name` (any words you like), a `color` (a color name such as `skyblue` or `orange`, or `#rrggbb`), and a `shape`: `underline`, `double_underline`, `bold`, `dotted`, `brackets`, or `symbol`. You can have up to eight entries; textweaver keeps the first eight and tells you if there are more. To change a name, a color, or a shape, edit the file; Settings lists the palette as **Highlight names**. For example:

```toml
[[highlight.palette]]
name = "ask the professor"
color = "skyblue"
shape = "symbol"
```

Color never tells the names apart on its own. Every highlight is said by its name, and each name has its own shape:

- In the terminal reader, the highlight's color is the band behind the text, with black or white text, whichever reads better on it, and the shape is a set of text attributes of its own: underline; underline and bold; bold; underline and italic; italic; italic and bold. With colors off, the attributes still tell the names apart.
- In a BRF file, each of the first five names has its own transcriber-defined typeform, and a transcriber's note at the start says which typeform is which name. This happens when you export the document you highlighted (**File**, **Export as**, **BRF**). UEB has five transcriber-defined typeforms, so highlights with the sixth to eighth names are not marked in braille, and the export report says so.
- In the window, highlights keep the window's highlight mark for now; drawing each name's own shape there is still to come.

Highlights made before the palette, in textweaver or in star, have only a color. Each takes the name of the first palette entry with its color, so star's yellow highlights are "important". If you rename an entry, its highlights follow the same rule: they take the name of the entry with their color. A highlight whose color is in no entry is said by its color and drawn with a shape no entry uses, when one is free.

### List highlights: Shift+Y

Press **Shift+Y**. You hear "Highlights", the count, then "Enter goes to one, Delete removes it, F2 changes its name, Space shows only its name. C makes a card." **C** makes a study card from the highlight ([Study with cards](#study-with-cards)). Each item says the name, the passage, and the line: "important: The cell membrane, line 12".

In the list:

- **Enter** goes to the highlight. You hear "Highlight, important" and the passage.
- **F2** lists the palette's names; choose one with **Enter** to give the highlight that name.
- **Space** shows only the highlights with that item's name: "Highlights named important, 3 items. Space shows every highlight." **Space** again shows them all.
- **Delete** asks "Remove this highlight? y or n" and removes it on **y**.
- **Escape** closes the list.

In the command palette this command is `list_highlights`.

### Collect the highlights of one name

**Collect highlights** lists the palette's names. Choose one and press **Enter**: textweaver writes that name's highlights as a Markdown list next to the document, in document order, each with its line. `essay.md` and the name "ask the professor" give `essay-highlights-ask-the-professor.md`. You hear how many highlights went in, the file's name, "Open it? y or n.", and the folder. It is in the **Highlights** menu and in the command palette as `collect_highlights`.

### Highlighting is not studying

Highlighting on its own is a weak way to study. A review of ten common study techniques rated it low in utility (Dunlosky and colleagues, 2013; see [What the research says](#what-the-research-says-and-what-it-does-not)). Use your names to sort what you mark, then turn the marks into questions: the [self-test](#test-yourself-from-the-study-sheet) asks you about each highlight and shows the passage only when you ask.
## Delete a note or highlight at the cursor: Delete

When reading, move to a note or a highlight and press **Delete**. textweaver asks "Delete this note or highlight? y or n". Press **y** to delete it, or **n**, **a**, or **Escape** to keep it. A note under the cursor is deleted before a highlight. With nothing there you hear "No note or highlight here."

Delete in the notes and highlights lists asks the same way. Only the bookmarks list deletes at once, without asking.

## Study with textweaver

textweaver has three tools for studying what you read: a self-test made from your notes and highlights, study cards made from the same marks, kept with your grades and brought back at growing intervals, and recall prompts that stop reading at the end of each section. All are optional, and none changes your document.

### What the research says, and what it does not

A review of ten common study techniques rated practice testing (retrieving material from memory) and spacing study over time as high in utility, and rereading, highlighting, and summarizing as low (Dunlosky and colleagues, 2013). Highlighting on its own is therefore a weak way to study; the self-test turns your highlights into questions instead. In two experiments with prose passages, students who were tested on a passage remembered more of it two days and one week later than students who reread it, although rereading did better after five minutes and left students more confident (Roediger and Karpicke, 2006).

A quantitative synthesis of 839 assessments in 317 experiments on verbal recall found that spacing study sessions apart improves retention over studying in one session, and that the gap that works best grows with how long the material must be kept: a longer wait before the test calls for a longer gap between sessions (Cepeda and colleagues, 2006). This is the reason study cards come back after one day, then six, then at intervals that grow with each successful recall.

In a study of adaptive retrieval practice with 118 participants, the slower answers of participants with dyslexia came from typing them, not from memory, and answering aloud removed the gap (Wilschut, Sense, and van Rijn, 2024). This is why the self-test lets you answer aloud through dictation.

In an experiment with a 21-minute video lecture in four parts, undergraduates who answered short tests between the parts reported mind wandering on 19 percent of probes, against 39 percent for those who restudied the material between parts (Szpunar, Khan, and Schacter, 2013). Recall prompts bring a similar pause for recall to reading aloud.

These studies were done mostly with readers without disabilities, and with tests that experimenters wrote. textweaver's self-test, cards, schedule, and recall prompts have not themselves been studied. Treat them as ways to practice recalling, not as a promised gain.

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

Study cards turn the passages you marked into questions you answer from memory, and keep a record of how each answer went. The self-test asks every note and highlight afresh each time; cards are kept, and each one comes back when it is due, after an interval that lengthens as you keep recalling it. You can also reverse cards, remove the ones you no longer need, and see the last grade you gave each one.

#### Make cards

Press **F2** for the command palette and type `make cards`, or choose **Make cards** in **Study cards**, a submenu of the **Bookmarks and notes** menu. textweaver makes cards from every note and highlight in the document, and says, for example, "Cards made: 5 new, 12 in all."

- **A highlight** becomes a fill-in-the-blank card. The question is the sentence the highlight is in, with the highlighted words replaced by the word "blank", which is how it is read aloud and shown: "The kidneys blank the blood." The answer is the highlighted words. A highlight that covers its whole sentence leaves nothing to fill in, so it asks "What did you highlight in Renal clearance?" instead.
- **A note on a passage** becomes a question card. The note is the question and the passage is the answer, so a note written as a question ("What does the loop of Henle do?") makes the best card. A note with no text, or one on an empty passage, makes no card.
- **A heading** becomes a recall card when a note or highlight is in its section: "What does “Renal clearance” say?" The answer is the section's first sentence.

To make a card from one note or highlight, open the notes list (**Shift+A**; the window also has **Ctrl+Shift+N**) or the highlights list (**Shift+Y**), move to it, and press **C**. You hear "Card made:" and its question.

Making cards again is safe. A card is tied to the note, highlight, or heading it came from, so making cards again after you edit a note updates that card's question and answer and keeps its grades; it never makes a second copy. A card stays when you delete its note or highlight; remove it from the Cards list (below) if you no longer want it.

#### Study the cards

Type `study cards` in the palette, or choose **Study cards** in the same submenu. You first hear how many cards are due, then how the session works: "Due today: 7 cards, 5 new. Study cards, 12 cards. Enter shows each answer, 1 to 4 grade it. Space to answer aloud." and then the first question. The session asks the cards that are due first, those longest overdue at the head of the list, and then the new cards that have never been graded, in document order. Cards that are not due yet are left out. When nothing is due and every card has been graded, you hear when the next one falls due, "Nothing due today; the next card is due in 3 days. Studying every card ahead.", and the session asks every card. In the list:

- Answer silently or aloud, then press **Enter** to hear "Answer:" and the answer.
- Grade how well you recalled it, in your own judgment, with a number key or by name in the palette:
  - **1**, Again (`grade_again`): you did not recall it.
  - **2**, Hard (`grade_hard`): you recalled it with effort.
  - **3**, Good (`grade_good`): you recalled it.
  - **4**, Easy (`grade_easy`): you recalled it at once.
- After a grade you hear the grade, when the card will come back, and the next card: "Good, next in 6 days. Card 4 of 12. Question: ...". After the last card you hear, for example, "Again, next tomorrow. Done: all 12 cards graded." and the list closes.
- **Space** answers aloud, as in the self-test ([Answer aloud](#answer-aloud)): your words are read back before you reveal the answer, and textweaver never judges them. The grade is always yours.
- **R** reverses a question card: the passage is asked and the note becomes the answer. Press **R** again to put it back. The card stays reversed the next time you study it. Fill-in-the-blank and recall cards cannot be reversed.
- **Up** and **Down** move between cards without grading; **Escape** closes the list. The session waits: a grade from the palette or the submenu grades the card you were on and opens the list again on the next card ("Good, next in 6 days. Study cards, card 4 of 12."), and **Study cards** starts a new session with the cards due then.

#### When cards come back

Each grade is stored with the time you gave it, and textweaver works out from the grades alone when a card is next due. It uses SM-2, the scheduling method that SuperMemo published in 1990 and on which most flashcard programs have built since:

- A card you recall (Hard, Good, or Easy) comes back after one day the first time, after six days the second time, and after that at the last interval multiplied by the card's ease. The ease starts at 2.5, so a card you keep grading Good comes back after 1, 6, 15, and 38 days.
- **Easy** raises the card's ease by 0.1, so its intervals grow faster; **Hard** lowers it by 0.14, so they grow more slowly; **Good** leaves it unchanged. The ease never falls below 1.3.
- **Again** starts the card over: it comes back the next day, and then after one and six days again, as if new. Its ease is unchanged.

A card counts as due from half a day before its interval ends, so a card graded one evening is due the next morning. textweaver never shows a score: you hear counts ("Due today: 7 cards, 5 new.") and when a card comes back ("next in 3 days"), never a percentage or a mark. Because the schedule is computed from the grades rather than stored, a card graded on two computers is scheduled the same way on both once their grades have synced.

#### The Cards list

Type `list cards` in the palette, or choose **Cards** in the same submenu, to hear first how many cards are due and how many are new, then every card with its kind, its question, its last grade, and when it is next due: "Fill in the blank: The kidneys blank the blood., last graded Good, next in 6 days". A card that is due says "due today", and one never graded says "not graded yet". **Enter** goes to the card's source in the document, where its note or highlight is now. **Delete** asks "Remove this card and its grades?" and removes it on **y**.

#### Where cards are kept

Cards are kept on this computer beside your notes, one file per document (`cards/` in textweaver's data folder), and written in the background. With [sync](sync.md) set up, cards travel with your notes: a card made or graded on one computer arrives on your others, and a card taken out on one is taken out on the others. Every grade survives: when you grade the same card on two computers while they are apart, both grades are kept, in the order you gave them, and the schedule is worked out from all of them. A card's question, answer, and direction follow the newest edit, as a note does. Cards sync when the **Notes** group is on; turning that group off stops cards too.

#### Count due cards from the command line: tw study due

`tw study due` counts the cards due today in every document in your library that has cards, the total first: "Due today: 7 cards, 5 new, in 2 documents." followed by one line per document, such as "Due today: 3 cards, 1 new. Pharmacology". A document with nothing due says when its next card falls due: "Nothing due today, next in 3 days. Anatomy". Give a file to count only its cards: `tw study due chapter4.md`. `--json` prints the same counts as JSON for a script, and `--home DIR` reads the cards under another data folder. The command only reads; it never changes a card.

### Recall prompts at section ends

Turn on **Recall prompts** in Settings (`[reading] recall_prompts`, off by default). When continuous reading stops at the end of a section, it asks you to recall it, naming the section it just read: "Say what you remember from Renal clearance. Ctrl+Space to go on." The key you hear is your read key. Say or think what you remember, then press the key, and reading goes on with the next section.

Where reading stops is set by **Stop at section end** ([Reading and moving around](reading.md#stop-at-the-end-of-a-section)). If that is **never**, recall prompts stop at the next heading of any level; set it to **next chapter** for longer sections. Recall prompts do not record what you say.

### A study routine

One way to combine these:

1. Read a section aloud with recall prompts on. At each prompt, say what you remember before going on.
2. While reading, add a note (**a**) where a passage answers a question you expect, written as that question, and highlight (**y**) what you want to recall.
3. Later, open the self-test, or make cards and study them. Answer each question before you reveal it, aloud or silently, and add a note where you missed something.
4. Study the due cards each day rather than rereading the chapter. `tw study due` tells you, without opening a document, how many are waiting.

### Sources

- Cepeda, N. J., Pashler, H., Vul, E., Wixted, J. T., and Rohrer, D. (2006). *Psychological Bulletin*, 132(3). [doi:10.1037/0033-2909.132.3.354](https://doi.org/10.1037/0033-2909.132.3.354)
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

To list only the highlights with one name, as Space does in the highlights list:

```bash
tw marks essay.md --name important
```

Each highlight says its name, and the JSON gives its `name` and `shape` beside its `color`.

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

With sync set up (Tools, Sync, Set up sync), your bookmarks, notes, highlights, study cards with their grades, and reading places travel to your other computers through a folder you choose, such as one kept in step by Syncthing or a USB stick. A document is recognized by its contents, so it syncs wherever it is on each computer. [Syncing between computers](sync.md) explains it, including what you hear when the same note was edited on two computers: the newest edit wins, and the older text is kept in this computer's backup of replaced notes.

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
