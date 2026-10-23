# CW: reading in Morse code

textweaver can read a document in Morse code instead of speech. CW (continuous wave, the radio amateur's name for Morse code sent as a tone) is a speech engine like Eloquence or eSpeak NG: choose it, press Play, and the document is sent as a tone, character by character. The highlight and RSVP follow each word as it is sent, the rate keys change the speed, and audio export writes the Morse code to a file. This guide describes how CW reads, its settings, how it shows the characters on the status line for a Braille display, and the `tw cw monitor` command for recording what a Morse key adapter sends.

## Choose CW

CW is the engine `cw`. Choose it in Settings (Speech engine: CW (Morse code)), or in `settings.toml`:

```toml
[speech]
backend = "cw"
```

From the command line, `tw speak --backend cw "CQ CQ"` sends a phrase, and `tw export-audio --backend cw FILE --out FILE.wav` writes a document as Morse code.

CW is used only when you choose it. If another engine fails, textweaver falls back to the next engine that can speak, as it always does, but never to CW: a document is never sent in Morse code unless you asked for it. With CW chosen, everything textweaver says aloud is sent in Morse code, its own messages included. Self-voicing readers who want messages in speech can use a screen reader for them (see [Who speaks](speech.md#who-speaks-the-accessibility-mode)).

## What is sent

The text is sent exactly as written. The speech normalizer is skipped: "10" is sent as the figures 1 and 0, "Dr." as D, R and a full stop, and no punctuation is spelled out in words. Letters of either case, the figures, and the punctuation of the International Morse code (Recommendation ITU-R M.1677-1) are sent: `. , : ? ' - / ( ) " = + @`, the accented E (`é`), and the multiplication sign as X. Five signs in wide use that the recommendation does not define are sent too: `! & ; _ $`.

A character Morse code does not have, such as `#`, `*`, or a letter with an accent other than é, is skipped. The word around it is still sent, and when the reading ends the status line says how many characters were skipped ("3 characters not sent in Morse code.").

### Prosigns and document structure

With the setting CW prosigns on (the default), the document's structure is sent as prosigns, the procedure signals sent as one character without the usual gap between their letters:

- **BT** (`-...-`) after each heading and each paragraph, so a heading has BT before it (the break after the paragraph above) and after it.
- **AR** (`.-.-.`, end of message) when the reading reaches the end of the document.

Structure is never spelled out in words: "heading level 2" is not sent. A prosign written in the text between angle brackets is sent as a prosign: `<AR>`, `<AS>`, `<BT>`, `<CT>`, `<HH>`, `<KN>`, `<SK>`, `<SN>`, in either case. With CW prosigns off, no structure is sent and angle brackets are skipped like any character Morse code does not have.

## Speed and timing

Speeds are in words per minute on the PARIS standard: a dit is one unit and a dah three, with one unit between the elements of a character, three between characters, and seven between words. At 20 words per minute a unit lasts 60 milliseconds.

Two speeds are set, each from 5 to 50 words per minute:

- **Character speed** (default 20): how fast each character is sent.
- **Effective speed** (default 10): the overall speed. When it is below the character speed, the characters keep their speed and only the gaps between characters and words are stretched, so the text goes at the effective speed. This is Farnsworth spacing, computed with the ARRL's formula. It lets a learner hear each character at full speed, as it will sound on the air, with time to recognize it. At or above the character speed, the spacing is standard.

While CW is the engine, the rate keys (+ and - while reading, as for speech) change the character speed by one word per minute, and the effective speed with it in proportion: at 20 and 10, one step up gives 21 and 11. The reading goes on from the last word at the new speed. "Character speed 21, effective 11 words per minute." is said (in Morse code, since CW is speaking). Repeat sentence slower does not slow CW down.

## The tone

The tone is a sine wave, 600 Hz by default (CW tone, 200 to 1500 Hz). Each element rises and falls over 5 milliseconds on a raised-cosine curve, so it starts and stops without a click. The pitch keys move the tone by semitones from the CW tone, and the volume keys change its loudness as they change speech. The CW tone level (default 50 percent) sets how loud the tone is before the speech volume; a lower level is easier on the ears over a long session.

## Following along: highlight, RSVP, and the status line

The highlight follows the word being sent. Its timing is not estimated: textweaver computes where each word starts in the audio it plays, so the highlight moves when the word's first element sounds, and it stops when the audio pauses. RSVP (Alt+Shift+P) shows the word being sent. Dots and dashes are never shown.

For a reader who follows on a Braille display, the status line shows the characters as they are sent, newest last, after "CW:", in at most 40 cells: "CW: CQ CQ DE". A screen reader shows the status line on the Braille display, so each character appears as it starts to sound. textweaver itself announces nothing extra for them; the line is shown, not said. In the terminal reader, a screen reader that speaks changes on the screen may also speak the line as it changes; if it does, and you want the characters on the Braille display only, check your screen reader's settings for reading screen changes. To keep the status line for messages, turn off CW characters on the status line.

## The settings

```toml
[speech.cw]
wpm = 20
effective_wpm = 10
tone_hz = 600
volume = 50
prosigns = true
status_line = true
practice_threshold = 90
practice_group = 5
practice_groups = 10
practice_delay_ms = 1000
```

- `wpm`: the character speed, 5 to 50 words per minute.
- `effective_wpm`: the effective (Farnsworth) speed, 5 to 50; at or above `wpm`, standard spacing.
- `tone_hz`: the tone, 200 to 1500 hertz.
- `volume`: the tone level, 0 to 100 percent of full scale, before the speech volume.
- `prosigns`: send structure as prosigns (BT, AR) and read prosigns written as `<BT>`.
- `status_line`: show the characters being sent on the status line.
- `practice_threshold`, `practice_group`, `practice_groups`, `practice_delay_ms`: Morse code practice (see [Practice and training](#practice-and-training)).

A change applies from the next sentence; while CW is reading, the reading goes on from the last word with the new value. These settings are the same on every computer you sync.

## Practice and training

textweaver teaches Morse code by ear. It sends a group of characters, you type what you heard, and it tells you in words how you did. Nothing is shown before you answer: there are no dots and dashes on screen, and neither the status line nor RSVP shows a practice group while it is sent.

The commands are in the **Morse code practice** submenu of the **Tools** menu, and in the command palette (F2):

- **Koch practice** (`cw_practice_koch`): groups of the characters of your Koch lesson.
- **Random character practice** (`cw_practice_characters`): groups drawn evenly from every character the Koch course teaches.
- **Random word practice** (`cw_practice_words`): common English words and the abbreviations of a contact (CQ, DE, RST, 73, QTH and others), at least as many letters a group as the group size.
- **Practice progress** (`cw_practice_progress`): says your Koch lesson, its newest character, and your latest three rounds: "Koch lesson 3 of 40, newest R. Latest rounds: 92, 84, 78 percent."

### A round

A round is ten groups of five characters by default. Each group opens a prompt, "Group 1 of 10", and the group is sent. Type what you heard and press Enter. Case and spacing do not matter, and you may type while the group is still being sent, as you would copy on the air.

textweaver then says the score, count first, and the next group's number: "5 of 5, all correct. Group 2 of 10." When something was missed, it says what, and then what was sent: "4 of 5. Missed: K as R. Sent: K M R K M." A miss is a character copied as another ("K as R"), one not copied ("M not copied"), or one typed that was not sent ("E extra"). Your answer is lined up with the group by the fewest changes, so one missed character does not count every later one wrong. Punctuation is named in words: "period", "comma", "equals" (the BT sign), "slash", and "question mark".

- **Enter with nothing typed** sends the group again: "Again, group 2 of 10."
- **Escape** stops the round: "Practice stopped." An unfinished round is not kept.

After the last group, textweaver says the round's score: "Round: 46 of 50, 92 percent."

### The Koch method

The Koch method sends characters at full character speed from the start, so you learn each one by its sound rather than by counting dots and dashes, and adds one character at a time. Lesson 1 is K and M. When a Koch round scores 90 percent or better, the next character is added and said: "Lesson 2: added U." Below that, textweaver says what is needed: "90 percent adds the next character." Lesson 40 has all 41 characters, and a round passed there says "Every character learned."

The order is the one Learn CW Online (LCWO) teaches, taken from its source code:

K M U R E S N A P T L W I . J Z = F O Y , V G 5 / Q 9 2 H 3 8 B ? 4 7 C 1 D 6 0 X

The newest character of a lesson comes up about one time in three, so it is heard often while it is new.

Practice uses the CW settings. The usual Koch setup is the default: characters at 20 words per minute, with Farnsworth spacing for an effective 10. As you improve, raise the effective speed toward the character speed.

### Practice settings

- **CW practice pass mark** (`practice_threshold`, default 90, 50 to 100 percent): the round score that adds the next Koch character.
- **CW practice group size** (`practice_group`, default 5, 2 to 10 characters).
- **CW practice round length** (`practice_groups`, default 10, 1 to 50 groups).
- **CW practice start delay** (`practice_delay_ms`, default 1000, 0 to 5000 milliseconds): the quiet before each group, described below.

### Which engine sends the groups

With CW as your speech engine and textweaver speaking its own messages, everything is in Morse code: the prompt's message is sent, and the group follows it. With a screen reader speaking the messages, the group waits for the message as described next.

With another engine, which is the usual way to practice, textweaver's messages are spoken and the groups go to a CW engine started for practice alone. So that speech and Morse code do not overlap, each group waits for the message before it to be spoken, at your speech rate, and then for the start delay. With a screen reader speaking instead, the wait follows textweaver's speech rate setting, not the screen reader's; if a group starts before your screen reader finishes, raise the start delay.

### Progress

Your Koch lesson and your latest 20 rounds are kept in `cw-practice.json` in textweaver's data folder, so each person on a computer has their own. Practice progress is not synced between computers; the practice settings are, like the other CW settings.

## Record what a key adapter sends: tw cw monitor

`tw cw monitor` prints what a Morse key adapter sends, in words, with the time each message or key arrived, until you press Escape. It only prints: nothing is keyed, sent, or played. It is the first step toward keying Morse code into textweaver, and it records an adapter's messages once so they can be checked.

```text
tw cw monitor
```

Each line puts the meaning first, for a 40-cell Braille display:

```text
Note 20 on (dit in the usual mapping), channel 1, velocity 127, at 2.315 seconds, from TinyMIDI
Note 20 off (dit in the usual mapping), channel 1, velocity 0, at 2.377 seconds, from TinyMIDI
Key [ (character 005B) down, at 4.102 seconds
Key [ (character 005B) up, at 4.160 seconds
```

`tw cw ports` lists the MIDI inputs the monitor can listen to. The monitor's options:

- `--port TEXT`: listen only to MIDI inputs whose name contains the text, such as `--port tinymidi`.
- `--no-keys`: MIDI only. `--no-midi`: keys only.
- `--seconds N`: stop after N seconds instead of waiting for Escape.

Notes 20 and 21 are named "dit" and "dah in the usual mapping", the TinyMIDI's and CTR2 MIDI's; 30 and 31 the CTR2's straight key and PTT in its extended mode; 0, 1, and 2 the Vail adapter's straight key, dit, and dah. Controllers (such as the CTR2's knobs, controllers 100 to 107) are printed with their number and value.

### N6ARA TinyMIDI

The TinyMIDI is a Bluetooth LE device (its USB-C port is for power only). A switch chooses its mode.

- **MIDI mode** (dit note 20, dah note 21): pair the TinyMIDI with the computer, then run `tw cw ports` to check that it is listed, and `tw cw monitor --port tinymidi`. On Windows, textweaver reads MIDI through the Windows Runtime MIDI interface, the one that lists Bluetooth LE MIDI devices. After switching modes, remove the device from the computer's Bluetooth list and pair it again, since the computer remembers the mode it saw first.
- **Keyboard mode** (dit `[`, dah `]`): pair it as a keyboard, run `tw cw monitor --no-midi`, and key. The Windows console reports presses and releases; on Linux and macOS, only terminals with the kitty keyboard protocol (kitty, foot, WezTerm, Alacritty) report releases, and the monitor says at the start which kind it has.

### CTR2 MIDI

The CTR2 MIDI connects by USB (a USB-C data cable) in MIDI mode; its maker says Bluetooth MIDI is not supported on Windows. It always uses MIDI channel 1: the left paddle (tip) is note 20 and the right (ring) note 21, or in extended paddle mode 30 (straight key) and 31 (PTT). With Extended BTN mode on, the paddles move to notes 96 and 97 (or 98 and 99 in extended mode). Its buttons send a note when released, and its knobs controllers 100 to 107. Turn its keyer off so it sends the paddle contacts as they are; textweaver will do the keying. Connect it, then run `tw cw monitor --port ctr2`.

### Keyboard adapters

The VBand and Vail adapters are keyboards: dit is Left Control and dah Right Control. A terminal reports Control pressed on its own only in some cases (the Windows console, and terminals with the kitty keyboard protocol); `tw cw monitor --no-midi` shows what yours reports.

## See also

- [Speech engines and voices](speech.md)
- [Reading aids: RSVP](reading-aids.md)
- [Settings](settings.md)
- [The command line](command-line.md)
