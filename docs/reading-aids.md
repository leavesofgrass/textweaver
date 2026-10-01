# Reading aids

textweaver has aids that make text easier to see and follow. They change how text looks, never what it says. Speech, search, bookmarks, and notes always use the real text.

Every aid can be turned on and off, and textweaver says when it changes. The keys, in both the terminal and the GUI, are:

- **Alt+Shift+R**: show or hide RSVP, starting from the cursor.
- **Alt+Shift+P**: start or pause RSVP.
- **Alt+Shift+Up** and **Alt+Shift+Down**, or **Alt+Shift+PageUp** and **Alt+Shift+PageDown**: RSVP faster or slower. Windows Terminal resizes its panes with the first two; the Page keys always reach textweaver.
- **Alt+Shift+O**: move the RSVP word to the next place on the screen.
- **Alt+Shift+B**: bionic reading on or off.
- **Alt+Shift+U**: the reading ruler: off, current line, or ruler.
- **Alt+Shift+G**: say the reading level of the document, or of the selection.
- **Alt+Shift+Z**: syllables shown or hidden.
- **Alt+Shift+J**: difficult words marked or not.

In the window (`textweaver-xilem`), RSVP has its own strip under the document, so it never covers the text, and the words before and after sit to its left and right; see [The textweaver window](gui.md#reading-aids).

The [keyboard reference](keyboard.md) lists every key. Each aid's settings are in the `[reading_aids]` sections of `settings.toml`; [Settings](settings.md#reading_aids) lists them.

To try RSVP, bionic reading, and the ruler in a web browser first, open the [reading aids demo page](site/reading-aids.html) in `docs/site/`.

## RSVP: one word at a time

RSVP shows one word at a time in the same place on the screen. Your eyes stay still, so you do not have to track a line of text.

- One letter of each word is marked, a little left of centre. Look at that letter. It is where the eye reads a word fastest, and it stays in the same column from word to word.
- The word before and the word after are shown above and below, in normal type. You can hide either one.
- RSVP can run on its own timer, or follow speech.
  - On its own, it shows 300 words a minute. You can go faster or slower, from 50 to 1,500.
  - It pauses a little longer after a comma, longer after a sentence, and longer again after a paragraph. Long words stay a little longer too.
  - Following speech, it shows the word being spoken.
- You can pause and resume. You can move by word, sentence, or paragraph. Going back a sentence goes to the start of this sentence if you are more than three words in. Otherwise it goes to the sentence before.
- The word can sit in one of nine places: top, middle, or bottom, and left, centre, or right. The default is top centre. If the word would cover the line you are on, it moves out of the way.
- A word too long for the box ends with `…`, so you know part of it is hidden.
- The status line says where you are: for example, "Word 12 of 300, 3 percent. Sentence 2 of 20. 300 words per minute. Paused."

A note on safety: fast RSVP changes the screen many times a second. textweaver changes only the word, never the whole box: the box never goes blank or moves between words. WCAG 2.3.1 allows at most three flashes a second over an area bigger than about a quarter of what the eye takes in at once. A test checks the fastest rate, 1,500 words a minute, against that limit: the terminal box (with a terminal font up to about 24 points) and the window's RSVP panel stay well under it, because only the letters change. A frontend that ever draws much larger words has a cap ready that slows those changes to three flashes a second ([ADR-0037](adr/0037-extractive-summaries.md)). If flicker bothers you, slow down or use a smaller word size.

## Bionic reading

Bionic reading makes the first part of each word bold, so the eye has a place to land. By default, the first 40 percent of each word is bold. You can choose from 10 to 90 percent.

Numbers, web addresses, email addresses, and code are left alone.

## Text spacing

You can set four kinds of space. Each is a multiple of the font size.

- **Line height**: the space from one line to the next. 1.5 is a good start.
- **Paragraph spacing**: extra space after each paragraph.
- **Letter spacing**: extra space between letters.
- **Word spacing**: extra space between words.

There are three presets:

- **Default**: line height 1.5, a small gap between paragraphs.
- **WCAG**: the values in the WCAG text spacing guideline (line height 1.5, paragraph spacing 2, letter spacing 0.12, word spacing 0.16).
- **Generous**: more room than WCAG.

If a setting is below the WCAG value, textweaver tells you. That is information, not an error. Use what reads best for you.

In the terminal, textweaver cannot change letter spacing or line height exactly. It uses blank lines and extra spaces instead.

Set the spacing in `settings.toml`. For example, the WCAG values:

```toml
[reading_aids.spacing]
line_height = 1.5
paragraph_spacing = 2.0
letter_spacing = 0.12
word_spacing = 0.16
```

## Fonts

In the GUI and in HTML views, you can choose the font, its size, and how bold it is.

The terminal version always uses the terminal's own font. A terminal program cannot change the font, so to read in a different font there, change it in your terminal's settings. Everything else on this page works in the terminal.

### The Fonts dialog in the GUI

Choose View, then Fonts (Alt, V, F). The dialog has, in this order:

1. **Font family** (Alt+F): a list. The fonts that come with textweaver are first, marked "built in". After them come the fonts installed on your computer, in alphabetical order.
2. **Size in points** (Alt+S): from 6 to 144.
3. **Bold** (Alt+B): a check box.
4. **Preview** (Alt+P): a sample sentence in the font you chose. It changes as you move through the list. The sentence has the letters that are easy to mix up, such as capital I, small l, and the digit 1.
5. **OK** and **Cancel**. Enter is OK. Escape is Cancel.

When you choose OK, the document text changes to the new font, textweaver remembers it, and you hear, for example, "Font: OpenDyslexic, 16 points, bold." When you cancel, you hear "Font unchanged."

The font is saved in `settings.toml` under `[reading_aids.font]`:

```toml
[reading_aids.font]
family = "OpenDyslexic"
size_pt = 16.0
weight = 700
```

### Fonts that come with textweaver

Three fonts are built in. They work with no download and nothing installed on your computer:

- **Atkinson Hyperlegible Next**: from the Braille Institute, for readers with low vision. Letters that look alike are made different. It is also the font of PDF files textweaver makes.
- **Atkinson Hyperlegible Mono**: the same design with every letter the same width, for code.
- **OpenDyslexic**: for readers with dyslexia. The letters are heavier at the bottom and hard to mix up.

All three are free, under the SIL Open Font License. Their licenses are in `third_party/fonts/`.

### All the choices

The choices in settings (and in HTML views) are:

- **System font**: the font your computer uses for menus.
- **Sans serif**: a plain font without small strokes on the letters. This is the default.
- **Serif**: a font with small strokes on the letters.
- **Monospace**: every letter the same width.
- **Reading fonts**, made to be easy to read:
  - **OpenDyslexic**: for readers with dyslexia. The letters are heavier at the bottom and hard to mix up. Home page: https://opendyslexic.org/
  - **Atkinson Hyperlegible**: from the Braille Institute, for readers with low vision. Letters that look alike are made different. Home page: https://www.brailleinstitute.org/freefont/
  - **Lexend**: wide letter spacing, made to reduce visual stress. Home page: https://www.lexend.com/
- **Any other font** installed on your computer.

All three reading fonts are free, under the SIL Open Font License. OpenDyslexic comes with textweaver. For Atkinson Hyperlegible, textweaver uses the newer Atkinson Hyperlegible Next that comes with it, unless you have the original installed. Lexend does not come with textweaver: textweaver downloads it the first time you choose it, after asking.

### Lexend on first choice

When you choose Lexend, in Settings or in the window's font list (Ctrl+D), and it is not installed, textweaver asks first:

"Download the Lexend font, 206 KB, SIL Open Font License? y or n"

- **y** downloads it. textweaver says "Downloading Lexend.", then "Lexend downloaded and ready." The window uses it at once.
- **n** says "Not downloaded. Another font is used." Lexend stays your choice, and textweaver uses another reading font, or a plain font, so you can keep reading. Choosing Lexend again asks again.
- Any other key asks the question again.

The window's font list says which it is: "Lexend (download, 206 KB)" before, "Lexend (downloaded)" after. If you have Lexend installed yourself, textweaver uses that and asks nothing.

What textweaver downloads, and how it checks it:

- Two files, Lexend Regular and Lexend Bold, from the Lexend project's own repository at a fixed version. Nothing else is fetched, and nothing about you is sent.
- Each file is checked by its size and its SHA-256 fingerprint before it is kept. If one does not match, nothing is kept and textweaver says so: "Lexend not downloaded:" and the reason.
- The files are kept in textweaver's data folder, in `fonts/lexend`, with the license, `OFL.txt`. Lexend's license also comes with textweaver, in `third_party/fonts/lexend/`.

Once downloaded, Lexend works everywhere textweaver uses a font: the window, PDF files (`tw convert --font lexend`), and EPUB books, which carry the font and its license inside.

When there is no data folder (a session that keeps no files), textweaver says "No data folder to keep Lexend in." The lean reader, built without the `publish` feature, has no downloads and says "Font downloads are not in this build."; install Lexend yourself from its home page there, and textweaver finds it the next time it starts.

(The `[reading_aids.font] fetch_missing` setting, which never did anything, was removed in 0.1.0-alpha.5. Asking first is how downloads are turned off: say n.)

Font size is in points, from 6 to 144. The default is 14. Below 12, textweaver suggests a larger size.

## Reading ruler and current line

- **Current line** marks the line you are reading.
- **Reading ruler** marks that line and a band of lines around it. By default, one line above and one below.
- You can mark just the screen row, or the whole line when it wraps onto several rows.
- **Mask** (off by default) dims everything outside the band. Dim text can be hard to see, so try it before you rely on it.

In the terminal, the current line is underlined with a bar in the left margin. Lines in the band get a thinner bar. Nothing is shown by color alone.

## Difficult words

textweaver can mark rare words, so you can look them up before you read. A word is rare when it is uncommon in everyday English.

Press **Alt+Shift+J** (or run `difficult words toggle` from the palette) to mark them. You hear "Difficult words underlined." In the terminal, each difficult word is underlined, never shown by color alone; the window underlines them with a thick line, the same rule. When verbosity is high (**Alt+Shift+V**), moving onto one with the Right or Left arrow adds "difficult word" after it: "mitochondria, difficult word". The choice is saved as `difficult_words = true` under `[reading_aids]`. See also [The textweaver window](gui.md#reading-aids).

To hear what a difficult word means as you move onto it, turn on `difficult_definitions` under `[reading_aids]` (off by default; it is also in the settings screen as "Difficult word definitions"). With difficult words marked and verbosity high, you then hear the first definition from the define-word dictionary, your glossary first: "mitochondria, difficult word: an organelle containing enzymes responsible for producing energy". Only the first part of the definition is said, at most about 100 characters. The dictionary opens quietly the first time; until it has, you hear "difficult word" alone. Without the dictionary file, nothing changes.

textweaver has a word list built in, so this works with no download. The list comes from SCOWL (Spell Checker Oriented Word Lists), which sorts English words into sizes by the smallest dictionary they appear in:

- **35**: a small dictionary. Common words.
- **40** and **50**: a medium dictionary.
- **60**: the size spell checkers use.
- **65** and **70**: a large dictionary.
- **80**: valid words that are unusual, the kind used in word games.

By default, a word is difficult when it first appears in a size above 50. A word that is not in the list at all counts as rarer than 80, so it is marked too. You can choose the size: 35 marks the most words, 80 marks only the rarest.

SCOWL was made for spell checking, not for reading level. It says how many dictionaries include a word, not how often people use it. So it is a rough guide: "ubiquitous" is in the small dictionary, and "serendipity" first appears at 50.

You can also load your own list: one word per line, most common first, or a word, a tab, and its Zipf frequency (the format `wordfreq` exports).

Short words (under four letters), names, web addresses, and code are never marked.

## Reading level

textweaver can estimate how hard a document, or a part of it, is to read. It gives:

- a **grade level** (Flesch-Kincaid): about the school year a reader needs;
- a **reading ease** score from 0 to 100 (Flesch): higher is easier;
- how many words and sentences it counted.

For example: "Grade 8.2, middle school. Reading ease 64 out of 100. 1,234 words in 56 sentences."

These are estimates. Other tools may give a slightly different grade.

## Syllables

textweaver can show long words split into syllables, like `read·a·bil·i·ty`. Press **Alt+Shift+Z** (or run `syllables toggle` from the palette); you hear "Syllables shown." This helps you sound out a word. Only the screen changes. Speech, search, bookmarks, and positions use the word as it is.

The separator is drawn between the letters, so the reading highlight still covers exactly the word being spoken, separators and all, and the cursor stays on the right letter. The choice is saved as `syllables = true` under `[reading_aids]`; `[reading_aids.syllable_options]` sets the separator (a middle dot by default) and which words are split. Both the terminal reader and the GUI draw the split; only the letters shown change, so your screen reader and Braille display still read the word whole. See also [The textweaver window](gui.md#reading-aids).

The split is worked out from English spelling rules, not a dictionary, so a few words split in odd places.

## See also

- [Reading aids demo](site/reading-aids.html): try RSVP, bionic reading, and the ruler in a browser.
- [Themes](themes.md): the colors the aids use, all checked for contrast.
- [Settings](settings.md#reading_aids): every `[reading_aids]` setting.
- [Reading and moving around](reading.md): reading aloud and moving through a document.
- [ADR-0022: Reading aids](adr/0022-reading-aids.md): the design, and the Star faults each aid fixes.
- [Documentation index](README.md)
