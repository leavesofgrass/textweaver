# Reading aids

textweaver offers aids that change how text is displayed, never what it says. Speech, search, bookmarks, and notes always operate on the underlying text. The evidence for each aid is mixed and readers differ, so treat each one as an option to evaluate, not as a recommendation.

Every aid can be switched on and off, and textweaver announces each change. The keys, in both the terminal reader and the app, are:

- **Alt+Shift+R**: show or hide RSVP, starting from the cursor.
- **Alt+Shift+P**: start or pause RSVP.
- **Alt+Shift+Up** and **Alt+Shift+Down**, or **Alt+Shift+PageUp** and **Alt+Shift+PageDown**: RSVP faster or slower. Windows Terminal resizes its panes with the first two; the Page keys always reach textweaver.
- **Alt+Shift+O**: move the RSVP word to the next place on the screen.
- **Alt+Shift+B**: bionic reading on or off.
- **Alt+Shift+U**: the reading ruler: off, current line, or ruler.
- **Alt+Shift+G**: say the reading level of the document, or of the selection.
- **Alt+Shift+Z**: syllables shown or hidden.
- **Alt+Shift+J**: difficult words marked or not.

In the app (`textweaver`), RSVP has its own strip under the document, so it never covers the text, and the words before and after sit to its left and right; see [The textweaver app](gui.md#reading-aids).

The [keyboard reference](keyboard.md) lists every key. Each aid's settings are in the `[reading_aids]` sections of `settings.toml`; [Settings](settings.md#reading_aids) lists them.

To try RSVP, bionic reading, and the ruler in a web browser first, open the [reading aids demo page](site/reading-aids.html) in `docs/site/`.

## RSVP: one word at a time

RSVP (rapid serial visual presentation) shows one word at a time in a fixed place on the screen, which removes the eye movements that tracking a line of text requires. Studies of RSVP find that it raises reading speed while lowering understanding of what is implied rather than stated, more so as the rate climbs, and that it helps readers with low vision only when their central vision is intact. It suits skimming and short passages; reduce the rate when you need to follow an argument.

- One letter of each word, slightly left of center, is marked as the fixation point. It is the position from which the eye recognizes a word fastest, and it stays in the same column from word to word.
- The word before and the word after are shown above and below, in normal type. You can hide either one.
- RSVP can run on its own timer, or follow speech.
  - On its own, it shows 300 words a minute. You can go faster or slower, from 50 to 1,500.
  - It pauses briefly after a comma, longer after a sentence, and longer again after a paragraph. Long words are displayed slightly longer.
  - Following speech, it shows the word being spoken.
- You can pause and resume. You can move by word, sentence, or paragraph. Going back a sentence goes to the start of this sentence if you are more than three words in. Otherwise it goes to the sentence before.
- The word can sit in one of nine places: top, middle, or bottom, and left, center, or right. The default is top center. If the word would cover the line you are on, it moves out of the way.
- A word too long for the box ends with `…`, so you know part of it is hidden.
- The status line says where you are: for example, "Word 12 of 300, 3 percent. Sentence 2 of 20. 300 words per minute. Paused."

Photosensitivity: fast RSVP changes the screen many times a second. textweaver changes only the word, never the whole box: the box never goes blank or moves between words. WCAG 2.3.1 (Three Flashes or Below Threshold) allows at most three flashes a second over an area larger than a defined fraction of the visual field. A test checks the fastest rate, 1,500 words a minute, against that limit: the terminal box (with a terminal font up to about 24 points) and the app's RSVP panel stay well under it, because only the letters change. A frontend that ever draws much larger words has a cap ready that slows those changes to three flashes a second ([ADR-0037](adr/0037-extractive-summaries.md)). If flicker is a problem for you, lower the rate or use a smaller word size.

## Bionic reading

Bionic reading renders the first part of each word in bold, so that the eye can anchor on a word's opening letters. Four independent studies found no gain in reading speed or understanding from it, though some readers prefer how it looks. By default, the first 40 percent of each word is bold. You can choose from 10 to 90 percent.

Numbers, web addresses, email addresses, and code are left alone.

## Text spacing

Four kinds of spacing can be set. Each is expressed as a multiple of the font size.

- **Line height**: the distance from one line to the next. WCAG 1.4.12 asks for at least 1.5.
- **Paragraph spacing**: extra space after each paragraph.
- **Letter spacing**: extra space between letters.
- **Word spacing**: extra space between words.

The WCAG text spacing guideline's values are line height 1.5, paragraph spacing 2, letter spacing 0.12, and word spacing 0.16; the example below sets them. textweaver does not warn about values below them; use whatever reads best for you. It says one thing when a change leaves letters spaced wider than words, "Raise word spacing with letter spacing.", because words then run into one another; WCAG's own values keep word spacing above letter spacing.

A terminal cannot adjust letter spacing or line height continuously, so textweaver approximates them there with blank lines and extra spaces.

Set the spacing in `settings.toml`. For example, the WCAG values:

```toml
[reading_aids.spacing]
line_height = 1.5
paragraph_spacing = 2.0
letter_spacing = 0.12
word_spacing = 0.16
```

## Fonts

In the app you can choose the font and its size; how bold it is is set in `settings.toml`.

The terminal version always uses the terminal's own font. A terminal program cannot change the font, so to read in a different font there, change it in your terminal's settings. Everything else on this page works in the terminal.

### The font list and text size in the app

- **Ctrl+D**, or the Font button, opens the font list. The fonts that come with textweaver are first, marked "built in", then Lexend ("download, 206 KB" until it is downloaded), then the fonts installed on your computer. It is a list like textweaver's others: a letter moves to the next font starting with it, and Enter uses the font at once. You hear, for example, "Font: OpenDyslexic." Escape keeps the font you had.
- **Ctrl+Plus** and **Ctrl+Minus** make the text larger and smaller, from 8 to 72 points, and **Ctrl+0** goes back to 14 points. You hear, for example, "Text size 18 points."
- The Settings dialog (Ctrl+,) has the same font and size under "Reading aids".

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
- **OpenDyslexic**: a font some readers prefer, with letters heavier at the bottom. Studies found no gain in reading speed or accuracy from it.

All three are free, under the SIL Open Font License. Their licenses are in `third_party/fonts/`.

### All the choices

The choices in settings (and in HTML views) are:

- **System font**: the font your computer uses for menus.
- **Sans serif**: a typeface without serifs (the small finishing strokes on letters). This is the default.
- **Serif**: a typeface with serifs.
- **Monospace**: a typeface in which every character has the same width.
- **Reading fonts**, designed for legibility:
  - **OpenDyslexic**: a font some readers prefer, with letters heavier at the bottom. Studies found no gain in reading speed or accuracy from it. Home page: https://opendyslexic.org/
  - **Atkinson Hyperlegible**: from the Braille Institute, for readers with low vision. Letters that look alike are made different. Home page: https://www.brailleinstitute.org/freefont/
  - **Lexend**: a font with wide letter spacing. The study behind it is small and has not been repeated. Home page: https://www.lexend.com/
- **Any other font** installed on your computer.

All three reading fonts are free, under the SIL Open Font License. OpenDyslexic comes with textweaver. For Atkinson Hyperlegible, textweaver uses the newer Atkinson Hyperlegible Next that comes with it, unless you have the original installed. Lexend does not come with textweaver: textweaver downloads it the first time you choose it, after asking.

### Lexend on first choice

When you choose Lexend, in Settings or in the app's font list (Ctrl+D), and it is not installed, textweaver asks first:

"Download the Lexend font, 206 KB, SIL Open Font License? y or n"

- **y** downloads it. textweaver says "Downloading Lexend.", then "Lexend downloaded and ready." The app uses it at once.
- **n** says "Not downloaded. Another font is used." Lexend stays your choice, and textweaver uses another reading font, or a plain font, so you can keep reading. Choosing Lexend again asks again.
- Any other key asks the question again.

The app's font list says which it is: "Lexend (download, 206 KB)" before, "Lexend (downloaded)" after. If you have Lexend installed yourself, textweaver uses that and asks nothing.

What textweaver downloads, and how it checks it:

- Two files, Lexend Regular and Lexend Bold, from the Lexend project's own repository at a fixed version. Nothing else is fetched, and nothing about you is sent.
- Each file is checked by its size and its SHA-256 fingerprint before it is kept. If one does not match, nothing is kept and textweaver says so: "Lexend not downloaded:" and the reason.
- The files are kept in textweaver's data folder, in `fonts/lexend`, with the license, `OFL.txt`. Lexend's license also comes with textweaver, in `third_party/fonts/lexend/`.

Once downloaded, Lexend works everywhere textweaver uses a font: the app, PDF files (`tw convert --font lexend`), and EPUB books, which carry the font and its license inside. Export to PDF or EPUB in the reader uses your reading font too, Lexend included; a font that is not on this computer leaves the export's usual font, and an installed font that textweaver does not bundle goes into PDF files only.

When there is no data folder (a session that keeps no files), textweaver says "No data folder to keep Lexend in." The lean reader, built without the `publish` feature, has no downloads and says "Font downloads are not in this version."; install Lexend yourself from its home page there, and textweaver finds it the next time it starts.

(The `[reading_aids.font] fetch_missing` setting, which never did anything, was removed in 0.1.0-alpha.5. Asking first is how downloads are turned off: say n.)

Font size is in points. The default is 14. `settings.toml` takes 6 to 144; the Settings dialog and the size keys go up to 72.

## Reading ruler and current line

- **Current line** marks the line you are reading.
- **Reading ruler** marks that line and a band of lines around it. By default, one line above and one below.
- You can mark just the screen row, or the whole line when it wraps onto several rows.
- **Mask** (off by default) dims everything outside the band. Dim text can be hard to see, so try it before you rely on it. In the terminal it looks the same in every terminal with full color or 256 colors; with 16 colors it uses the terminal's own dim, which some terminals draw weakly or not at all.

In the terminal, the current line is underlined with a bar in the left margin. Lines in the band get a thinner bar. In the app, the current line has a band with a bar at its start, and lines in the band a paler band with a thinner bar. Nothing is shown by color alone.

## Difficult words

textweaver can mark rare words, so that you can look them up before or while reading. A word counts as rare when it is uncommon in everyday English.

Press **Alt+Shift+J** (or run `difficult words toggle` from the palette) to mark them. You hear "Difficult words underlined." In the terminal, each difficult word is underlined, never shown by color alone; the app underlines them with a thick line, the same rule. When verbosity is high (**Alt+Shift+V**), moving onto one with the Right or Left arrow adds "difficult word" after it: "mitochondria, difficult word". The choice is saved as `difficult_words = true` under `[reading_aids]`. See also [The textweaver app](gui.md#reading-aids).

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

textweaver can estimate the reading difficulty of a document, or of a selection. It reports:

- a **grade level** (Flesch-Kincaid): the approximate US school grade at which the text is readable;
- a **reading ease** score from 0 to 100 (Flesch): higher is easier;
- how many words and sentences it counted.

For example: "Grade 8.2, middle school. Reading ease 64 out of 100. 1,234 words in 56 sentences."

Both formulas rest on sentence length and syllable counts alone, so they are rough estimates. Other tools may report a slightly different grade.

## Syllables

textweaver can display long words divided into syllables, for example `read·a·bil·i·ty`. Press **Alt+Shift+Z** (or run `syllables toggle` from the palette); you hear "Syllables shown." It shows where a long word breaks into syllables; no study of adult readers has tested it. Only the screen changes. Speech, search, bookmarks, and positions use the word as it is.

The separator is drawn between the letters, so the reading highlight still covers exactly the word being spoken, separators and all, and the cursor stays on the right letter. The choice is saved as `syllables = true` under `[reading_aids]`; `[reading_aids.syllable_options]` sets the separator (a middle dot by default) and which words are split. Both the terminal reader and the app draw the split; only the letters shown change, so your screen reader and Braille display still read the word whole. See also [The textweaver app](gui.md#reading-aids).

The split is worked out from English spelling rules, not a dictionary, so a few words split in odd places.

## See also

- [Reading aids demo](site/reading-aids.html): try RSVP, bionic reading, and the ruler in a browser.
- [Themes](themes.md): the colors the aids use, all checked for contrast.
- [Settings](settings.md#reading_aids): every `[reading_aids]` setting.
- [Reading and moving around](reading.md): reading aloud and moving through a document.
- [ADR-0022: Reading aids](adr/0022-reading-aids.md): the design, and the star faults each aid fixes.
- [Documentation index](README.md)
