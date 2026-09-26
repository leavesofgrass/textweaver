# Reading aids

textweaver has aids that make text easier to see and follow. They change how text looks, never what it says. Speech, search, bookmarks, and notes always use the real text.

The keys for each aid are in Help and in `docs/keyboard.md`. Every aid can be turned on and off, and textweaver says when it changes.

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

A note on safety: fast RSVP changes the screen many times a second. textweaver changes only the word, never the whole box. If flicker bothers you, slow down or use a smaller word size.

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

## Fonts

In the GUI and in HTML views, you can choose the font, its size, and how bold it is.

The choices are:

- **System font**: the font your computer uses for menus.
- **Sans serif**: a plain font without small strokes on the letters. This is the default.
- **Serif**: a font with small strokes on the letters.
- **Monospace**: every letter the same width.
- **Reading fonts**, made to be easy to read:
  - **OpenDyslexic**: for readers with dyslexia. The letters are heavier at the bottom and hard to mix up. Home page: https://opendyslexic.org/
  - **Atkinson Hyperlegible**: from the Braille Institute, for readers with low vision. Letters that look alike are made different. Home page: https://www.brailleinstitute.org/freefont/
  - **Lexend**: wide letter spacing, made to reduce visual stress. Home page: https://www.lexend.com/
- **Any other font** installed on your computer.

All three reading fonts are free, under the SIL Open Font License. textweaver does not include them. If you choose one that is not installed, textweaver:

1. uses another reading font you have, or a plain font, so you can keep reading;
2. tells you which font it is using;
3. offers to download the one you chose. It asks first, and says how big the download is. The files come from each font's own project on GitHub, and are kept in textweaver's cache folder. Nothing is installed on your system.

You can also install a reading font yourself, from its home page. textweaver finds it the next time it starts.

Font size is in points, from 6 to 144. The default is 14. Below 12, textweaver suggests a larger size.

## Reading ruler and current line

- **Current line** marks the line you are reading.
- **Reading ruler** marks that line and a band of lines around it. By default, one line above and one below.
- You can mark just the screen row, or the whole line when it wraps onto several rows.
- **Mask** (off by default) dims everything outside the band. Dim text can be hard to see, so try it before you rely on it.

In the terminal, the current line is underlined with a bar in the left margin. Lines in the band get a thinner bar. Nothing is shown by colour alone.

## Difficult words

textweaver can mark rare words, so you can look them up before you read. A word is rare when it is uncommon in everyday English.

This needs a word list, and textweaver does not include one yet. The lists we looked at have licences that need a decision first. You can load your own list: one word per line, most common first, or a word, a tab, and its Zipf frequency (the format `wordfreq` exports).

Short words (under four letters), names, web addresses, and code are never marked.

## Reading level

textweaver can estimate how hard a document, or a part of it, is to read. It gives:

- a **grade level** (Flesch-Kincaid): about the school year a reader needs;
- a **reading ease** score from 0 to 100 (Flesch): higher is easier;
- how many words and sentences it counted.

For example: "Grade 8.2, middle school. Reading ease 64 out of 100. 1,234 words in 56 sentences."

These are estimates. Other tools may give a slightly different grade.

## Syllables

textweaver can show long words split into syllables, like `read·a·bil·i·ty`. This helps you sound out a word. Only the screen changes. Speech reads the word normally.

The split is worked out from English spelling rules, not a dictionary, so a few words split in odd places.
