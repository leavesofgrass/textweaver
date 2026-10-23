# CW: reading in Morse code

textweaver can read a document in Morse code instead of speech. CW (continuous wave, the radio amateur's name for Morse code sent as a tone) is a speech engine like Eloquence or eSpeak NG: choose it, press Play, and the document is sent as a tone, character by character. The highlight and RSVP follow each word as it is sent, the rate keys change the speed, and audio export writes the Morse code to a file. This guide describes how CW reads, its settings, how it shows the characters on the status line for a Braille display, how to key Morse code into a document with a key or paddle, and the `tw cw monitor` command for recording what a Morse key adapter sends.

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
```

- `wpm`: the character speed, 5 to 50 words per minute.
- `effective_wpm`: the effective (Farnsworth) speed, 5 to 50; at or above `wpm`, standard spacing.
- `tone_hz`: the tone, 200 to 1500 hertz.
- `volume`: the tone level, 0 to 100 percent of full scale, before the speech volume.
- `prosigns`: send structure as prosigns (BT, AR) and read prosigns written as `<BT>`.
- `status_line`: show the characters being sent on the status line.

A change applies from the next sentence; while CW is reading, the reading goes on from the last word with the new value. These settings are the same on every computer you sync.

## Key Morse code into a document

With a key or paddle and a key adapter, Morse code you key is typed into the document at the cursor. textweaver does the keying itself: the adapter only reports when each contact closes and opens, and textweaver times the dits and dahs, decodes the characters, and plays a sidetone while the key is down. Keying works the same whichever speech engine is chosen.

### Turn keying on and off

Keying is a mode, so ordinary typing is never taken for Morse code by surprise. Enter edit mode (Ctrl+E), then press **Alt+Shift+F11** (CW keying, also in the Edit menu and the command palette). textweaver says "CW keying on." and names each MIDI adapter as it starts listening to it. Press Alt+Shift+F11 again to turn keying off. Leaving edit mode turns keying off too, and says so. In reading mode the key says that keying types in edit mode.

### What is typed

- **Letters, figures, and punctuation** of the Morse code are typed as you key them, letters in lower case (Morse code has no case). The accented E (`..-..`) is typed as `é`.
- **A space** is typed when the silence after a character is as long as a word gap, about five units. You press nothing between words; you pause.
- **Prosigns** that have no character of their own are typed in angle brackets: AR as `<AR>`, AS as `<AS>`, CT as `<CT>`, SK as `<SK>`, and SN as `<SN>`. This is the form CW reads back as the prosign. AR shares its code with `+`, and AS with `&`, so those two characters are typed from the keyboard.
- **BT** (`-...-`) starts a new paragraph, as textweaver sends a paragraph break as BT. It shares its code with `=`.
- **KN** (`-.--.`) is the open bracket `(`, which shares its code; the close bracket `)` is `-.--.-`.
- **HH**, the error sign (eight dits or more), erases the word you were keying; between words, it erases the word before and its space. Only text that keying typed, and only while it is still just before the cursor, is erased. "Error sign: tm erased." is said.
- **A code Morse code does not have** types nothing, and "Not a Morse character." is said.

### Following along: the status line, RSVP, and the sidetone

The status line shows what you key, newest last, after "CW:", in at most 40 cells: "CW: cq de". A screen reader shows the status line on a Braille display, so each character appears as it is decoded. It is shown and not spoken, since the sidetone is what you hear. While keying, Alt+Shift+R (RSVP) shows the word being keyed, large, one word at a time; press it again to close it. Outside keying, RSVP is not offered in edit mode.

The sidetone is the CW tone (CW tone and CW tone level, in the settings), with the same soft edges as reading in Morse code, heard while the key is down. It plays through the speech output device of an engine that plays its own audio (Eloquence, eSpeak NG, SAPI and OneCore voices, DECtalk, Piper, or CW). There is none with the silent engine or when a screen reader speaks for textweaver; use your adapter's or your radio's own sidetone then. Turn it off with **CW sidetone** (`[speech.cw] sidetone = false`). Its pitch and level are read when keying turns on.

### The keyer and your speed

A paddle has two contacts, dit and dah. textweaver's keyer turns them into elements at the character speed (CW character speed, 20 words per minute by default), with one unit of silence after each element:

- **Iambic B** (the default): squeezing both paddles sends dits and dahs in turn, starting with the paddle closed first. If both paddles were closed during an element, one more opposite element follows it when you let go: squeeze for A and let go during the dah, and you key di-dah-dit, an R.
- **Iambic A**: the same, but releasing the squeeze ends with the element sounding: the same squeeze keys A.
- Both modes have **dot and dash memory**: a paddle you tap while an element sounds, or in the silence after it, is sent next even if you have let it go.
- **Straight key**: no keyer. Either contact keys the tone directly, for a straight key or a bug plugged into a paddle adapter. A straight key on its own MIDI note (the CTR2 MIDI's extended paddle mode) is always straight, whatever the keyer.

Choose with **CW keyer** (`[speech.cw] keyer`). **Swap CW paddles** swaps dit and dah from every adapter, for a paddle used with the other hand.

The decoder adapts to your speed. It starts from the character speed, then learns the length of your dits and dahs from the last two dozen elements you keyed, so a straight key decodes at your own speed and rhythm. It is tested from 5 to 30 words per minute with every element and gap off by up to a fifth. A character ends after a silence of about two units and a word after about five, at the speed it has learned. If you change speed a great deal, the first character at the new speed may be decoded wrong; the next ones follow.

### Adapters

textweaver uses the documented messages of each adapter. `tw cw monitor` (below) shows what yours sends, and the note numbers are settings.

#### N6ARA TinyMIDI

The TinyMIDI is a Bluetooth LE device; its USB-C port is for power only. A switch on it chooses its mode. After switching modes, remove it from the computer's Bluetooth devices and pair it again, since the computer remembers the mode it saw first. The red light blinks twice at power-on in keyboard mode, and not at all in MIDI mode.

- **MIDI mode** (switch to the right): pair it with the computer, then turn keying on. Dit is note 20 and dah note 21, the defaults. It works in the terminal reader and the textweaver app. On Windows, textweaver reads MIDI through the Windows Runtime MIDI interface, which lists Bluetooth LE MIDI devices; `tw cw ports` should list it. Its touch paddle reaches 30 words per minute, and its maker recommends iambic A above 20 (iambic B may cause errors there).
- **Keyboard mode** (switch to the left): pair it as a Bluetooth keyboard. It types `[` for dit and `]` for dah; while keying is on, the textweaver app takes those keys for keying. The terminal reader cannot key from it.

#### CTR2 MIDI

Use the CTR2 MIDI in MIDI mode, connected by a USB-C data cable (a charge-only cable does not work). Its maker says Bluetooth MIDI is not supported on Windows, and Bluetooth can add up to 15 milliseconds, enough to upset iambic timing. It always uses MIDI channel 1.

- Turn its own keyer off, so it sends the paddle contacts as they are and textweaver does the keying.
- In **normal paddle mode** (green light off), the left paddle (tip) is note 20 and the right (ring) note 21, the defaults. In **extended paddle mode** (green light on; a long press of the knob in its Home knob mode), the jack is a straight key on note 30, the default straight key note, and PTT on 31, which keying ignores.
- Leave **Extended BTN** (option F in its terminal menu) off. With it on, the paddles move to notes 96 and 97 (98 and 99 in extended paddle mode), and some buttons send 20 and 21: if you use it, set CW dit MIDI note and CW dah MIDI note to 96 and 97, and the straight key note to 98.
- Optionally set **CW MIDI channel** to 1, so nothing on another channel is taken for a paddle.

#### VBand and Vail keyboard adapters

These adapters are keyboards: dit is Left Control and dah Right Control. While keying is on, the textweaver app takes those two keys for keying, so they do nothing else, not even as Control in shortcuts, until keying is off. The terminal reader cannot key from them, since a terminal does not report Control pressed and released on its own. The Vail adapter's own MIDI mode uses notes 0, 1, and 2 (straight key, dit, dah): set the three notes to match if you use it.

### Settings for keying

- **CW keyer** (`keyer`): iambic B (the default), iambic A, or straight key.
- **Swap CW paddles** (`swap_paddles`), off.
- **CW sidetone** (`sidetone`), on.
- **CW dit MIDI note** (`dit_note`, 20), **CW dah MIDI note** (`dah_note`, 21), and **CW straight key MIDI note** (`straight_note`, 30), each 0 to 127.
- **CW MIDI channel** (`midi_channel`): 0 for any channel (the default), or 1 to 16.

A new speed, keyer, note, channel, or swap applies at once while keying. The notes and the channel belong to the adapter on each computer, so they are not synced. See [Settings](settings.md#speechcw).

## Record what a key adapter sends: tw cw monitor

`tw cw monitor` prints what a Morse key adapter sends, in words, with the time each message or key arrived, until you press Escape. It only prints: nothing is keyed, sent, or played. Use it to check what an adapter sends before keying with it, or to find the note numbers of an adapter whose notes differ from the defaults.

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

The CTR2 MIDI connects by USB (a USB-C data cable) in MIDI mode; its maker says Bluetooth MIDI is not supported on Windows. It always uses MIDI channel 1: the left paddle (tip) is note 20 and the right (ring) note 21, or in extended paddle mode 30 (straight key) and 31 (PTT). With Extended BTN mode on, the paddles move to notes 96 and 97 (or 98 and 99 in extended mode). Its buttons send a note when released, and its knobs controllers 100 to 107. Turn its keyer off so it sends the paddle contacts as they are; textweaver does the keying. Connect it, then run `tw cw monitor --port ctr2`.

### Keyboard adapters

The VBand and Vail adapters are keyboards: dit is Left Control and dah Right Control. A terminal reports Control pressed on its own only in some cases (the Windows console, and terminals with the kitty keyboard protocol); `tw cw monitor --no-midi` shows what yours reports.

## See also

- [Speech engines and voices](speech.md)
- [Reading aids: RSVP](reading-aids.md)
- [Settings](settings.md)
- [The command line](command-line.md)
