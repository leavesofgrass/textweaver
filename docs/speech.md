# Speech engines and voices

This guide covers how textweaver speaks: the speech engines it can use, how it chooses one, how to pick a voice, and how to set the rate, pitch, and volume. It also covers how text is prepared for speech, how exactly each engine's words are highlighted, and every speech setting and environment variable. It is for anyone who wants a different voice, a faster rate, or a speech problem fixed.

## The engines

textweaver calls a speech engine a backend. Each has a short id, used by `--backend` and in settings.

- `eci`: ETI-Eloquence, through its engine library (ECI). Windows and Linux. It runs in a small helper program, the ECI host, so a 32-bit Eloquence works with 64-bit textweaver. textweaver does not include Eloquence; the [Eloquence guide](eloquence.md) explains how to get it. Eloquence reads numbers, dates, and abbreviations itself.
- `sapi`: Windows SAPI5 voices, including the OneCore voices (Microsoft David, Zira, Mark). Windows only. Each voice runs in a helper program: the 64-bit host for 64-bit voices, and the 32-bit host for older 32-bit-only voices.
- `dectalk`: DECtalk, from a copy you installed. See the [DECtalk guide](dectalk.md).
- `piper`: Piper neural voices, run inside textweaver on RTen, a speech model runtime written in Rust. Every platform. It is available once you download a voice in the voice manager (Alt+V). Words are highlighted from the voice's own timing. See [Piper voices](#piper-voices) below.
- `nsspeech`: Apple's system voices through the classic engine. macOS only. It answers fastest.
- `avspeech`: Apple's system voices through AVSpeechSynthesizer. macOS only. It highlights words most exactly. On macOS 13 and later the system voices include Eloquence (Reed, Shelley, and others).
- `espeak`: eSpeak NG, inside textweaver itself. In builds made with it: the Linux AppImage and tarball, and the Linux install script. textweaver loads libespeak-ng when it starts, so install the `espeak-ng` package to use it; without it, the backend is simply not available. `TEXTWEAVER_ESPEAK_LIBRARY` names the library file to load instead.
- `speechd`: Speech Dispatcher, the speech service Orca uses on Linux. In builds made with it; the Linux install script includes it.
- `omnivox`: an Omnivox speech server, a separate program found on your `PATH`. It supports rate and tones, but not pitch, volume, or voices.
- `null`: silent. Used when nothing else works, and by `--no-speech`.

There is also `recording`, a silent engine for automated tests. It is used only when named.

## How textweaver chooses an engine

The setting `[speech] backend` names the engine. Its default is `"auto"`.

With `"auto"`, textweaver uses the available engine with the highest priority. From highest to lowest:

1. `eci`, 1000;
2. `sapi`, 500;
3. `dectalk`, 300;
4. `piper`, 200;
5. `nsspeech`, 80, and `avspeech`, 70;
6. `espeak`, 50;
7. `speechd`, 45;
8. `omnivox`, 40;
9. `null`, last.

So an installed, licensed Eloquence is always the first choice. On macOS, `[speech.apple] backend` can put `avspeech` above `nsspeech`, or the other way round.

When you name an engine, with `--backend` or in the settings, textweaver uses it if it is available. If it is not, textweaver does not fall silent: it makes the automatic choice instead and tells you. The reader says "Speech engine", the id, "is not available; using", and the engine it chose. If the chosen engine then fails to start, the reader says "Speech could not start", the reason, then "running silently."

`--no-speech` always means silent.

### Who speaks: the accessibility mode

With a screen reader running, you may not want textweaver to speak everything. `[accessibility] mode` decides:

- `"self-voicing"` (the default): textweaver speaks everything.
- `"hybrid"`: textweaver reads documents aloud; your screen reader speaks messages, typing, and caret moves from the status line.
- `"screen-reader"`: textweaver never speaks. `--no-speech` starts in this mode.

**Alt+Shift+A** cycles the mode and saves it, and `--mode` sets it for one run. `tw serve` always speaks for itself, unless you start it with `--no-speech`. [Using textweaver with a screen reader](screen-readers.md) explains each mode.

## List the engines: tw backends

```bash
tw backends
```

It prints one line per engine: its id and name, whether it is available here, its priority, what it supports, and where the word highlight's timing comes from: "Word timing from the engine" when the engine says where each word is (Piper's comes from the voice model itself), or "Word timing estimated" when textweaver paces it. The engine the automatic choice picks says "Chosen automatically". For example:

```text
eci: ETI-Eloquence. Available. Priority 1000. Supports word highlighting, pause, pitch, volume, audio files, tones, and reads numbers and abbreviations itself. Word timing from the engine. Chosen automatically.
```

For a program or script, print JSON; it includes the automatic choice as `auto`:

```bash
tw backends --json
```

`tw backends --devices` lists the sound devices instead, for `[speech] output_device`; see [Choose the sound device](#choose-the-sound-device). It takes `--json` too.

## List the voices: tw voices

```bash
tw voices
```

It lists the voices of the engine in your settings (`[speech] backend`), or of the automatic choice: "Windows SAPI5 voices has 6 voices." then one line per voice, with the voice id, name, languages, gender, and tags. Name an engine with `--backend`:

```bash
tw voices --backend sapi
```

Add `--json` for JSON. Listing voices starts the engine, which can take a few seconds.

## Hear a voice: tw speak

```bash
tw speak "The quick brown fox."
```

`tw speak` speaks the text and says afterwards, for example, "Spoke 1 sentence with Windows SAPI5 voices." Its options:

- `TEXT`: the text to speak. Leave it out when you use `--file`.
- `--file FILE`: speak a document instead. Any file the reader opens works.
- `--backend ID`: the engine. The default is the automatic choice. An engine that is not available falls back to the automatic choice, with a note.
- `--voice VOICE`: the voice, by id or by plain name, such as `Zira` or `Reed`.
- `--rate WPM`: the rate in words per minute.
- `--pitch SEMITONES`: the pitch offset in semitones, such as `-2`.
- `--out FILE`: write the audio to a file instead of playing it. The engine must be able to write audio files.
- `--json`: print the prepared text, its offset maps, and every status the engine reported, as JSON.

`tw speak`, `tw voices`, and `tw backends` read your `settings.toml`, as the reader does. `tw speak` uses your engine, voice, rate, pitch, and volume, and your normalization, punctuation, table, footnote, and math settings; the options you give win over them. Your voice is used only with your engine: with `--backend` naming another engine, that engine's default voice speaks unless you give `--voice`. `--home FOLDER` reads the settings from another folder, like `TEXTWEAVER_HOME`.

To write a whole document to an audio file with subtitles, use `tw export-audio`. See [the audio export guide](audio-export.md).

## Choose a voice in the reader: Alt+V

Press **Alt+V**. The GUI uses **Ctrl+Shift+V**, and shows the filters and the actions as buttons beside the list (see [the GUI guide](gui.md#voices)). This opens the voice manager. You hear "Voice manager", how many voices are shown and which language and engine, then "Enter uses a voice and speaks a sample, or downloads one; Alt+End previews a voice; Space marks a favorite; Delete removes a downloaded voice; Escape closes." Alt+End is the Say Status key; if you changed it in `keymap.toml`, the voice manager names your key.

This is the voice for reading documents. textweaver's own words (messages, lists, help, and settings) have a separate choice, the interface language, in six languages: English, Spanish, French, German, Brazilian Portuguese, and Arabic. `[speech.voices_by_language]` picks which voice speaks each interface language, so switching the interface to Spanish can switch to a Spanish voice automatically. See [`[interface] language`](settings.md#interface) in the settings guide for how to choose it and how textweaver falls back when no voice exists for it.

The list holds:

- **Language: all languages.** Press Enter to show only the next language, then the next, and back to all. You hear, for example, "3 voices: English, all engines."
- **Engine: all engines.** Press Enter to show only one engine's voices, in the same way.
- **The voices.** Each says its name, language, engine, and tags; a favorite says "favorite", and the one in use says "current". Favorites come first. The list holds the running engine's voices and the Piper voices you have downloaded. Once you have fetched the Piper voice list, it also holds the Piper voices you can download, each with its size and "non-commercial" when its license says so.
- **Fetch the Piper voice list from the internet.** Press Enter, then y, to download the list of Piper voices (about 250 KB).

Press **Enter** on a voice. You hear "Voice", its name, then a sample: "The quick brown fox jumps over the lazy dog." The choice is saved in `[speech] voice`. A voice of another engine switches engine: you hear "Switching engine", and speech restarts with that voice.

**Each voice keeps its own rate and pitch**, as screen readers do. Change the rate on one voice, choose another, and come back: the first voice has its rate again. They are saved in `[speech.voice_params]`.

Press **Space** on a voice to make it a favorite, or to stop it being one. You hear, for example, "Microsoft Zira added to favorites." Favorites are listed first, in the order you added them, and are saved in `[speech] favorite_voices`.

An engine that offers no voices adds none to the list; the count you hear when the list opens says how many there are.

Each engine lists its voices once, when it starts, and keeps the list, so the list opens at once. The Windows voices are listed in the background (a helper program reads each registry); if you press **Alt+V** before that is done, you hear "The voices are still loading. The list opens when they are ready.", and it opens by itself when they arrive.

When a voice changes what the engine can do, textweaver tells you. For example: "This voice does not report words, so the word highlight is estimated." or "Pitch cannot be changed with this voice."

### Piper voices

Piper voices are neural voices: natural-sounding, and each a file of 20 to 140 MB. textweaver runs them itself, with no other program. Nothing is downloaded until you say yes.

1. Press **Alt+V**, go to "Fetch the Piper voice list from the internet", press Enter, and answer y.
2. Press Alt+V again. Use the language row to show your language.
3. Press Enter on a voice to download. textweaver reads the voice's details and says, for example: "Download Joe, English (United States), medium quality, 63 MB? License: CC0. Free to use for anything. y or n". Answer y. You hear progress at each quarter, then "Joe is installed."
4. Press Enter on the new voice to use it.

**Licenses differ from voice to voice.** Some are free for anything (`joe` is CC0; `kristin`, `norman`, and `cori` are public domain), some ask for credit (`libritts_r` is CC BY), and some are for personal, non-commercial use only (`lessac`, `ryan`, `hfc_female`). textweaver says the license before every download and does not ship any voice itself.

Every file is checked against the hash Hugging Face publishes for it; a file that does not match is deleted and nothing is installed. Voices are kept in `<data>/piper/voices/`. To remove one, press Delete on it in the voice manager and answer y.

To read words into phonemes, Piper uses eSpeak NG: the installed library when there is one, otherwise a copy written in Rust that is built into textweaver (English only). Piper voices in other languages need eSpeak NG installed.

### Voice names

Wherever textweaver takes a voice, you can give its id or a plain name. `Zira`, `Reed`, or `David` is enough. textweaver looks for, in order: the exact id, a voice with exactly that name, then a voice whose name and id contain every word you gave. When several voices match, a US English one is preferred.

With no voice chosen, `[speech] prefer_voice` picks one: the first voice whose name or id contains it, again preferring US English. Its default is `"eloquence"`, so an Eloquence voice is used when the engine has one. Set it to `""` for no preference.

## Rate, pitch, and volume

### Rate

The rate is in words per minute, from 50 to 900. The default is 265.

- **+** or **=**: faster by 20.
- **-**: slower by 20.

The GUI also has **F11** and **Shift+F11**; there, **Ctrl+=** and **Ctrl+-** change the text size. You hear the new rate, for example "285 words per minute." At the limits you hear "Fastest rate." or "Slowest rate." Each engine turns words per minute into its own scale, so the same rate sounds about the same on every engine.

### Speed presets: F8

Press **F8** to go through four preset rates, from fastest to slowest:

- skim, 350;
- normal, 265;
- study, 200;
- slow, 150.

You hear the preset first, then its rate, for example "Study, rate 200." You can change the presets, or add your own, in `[speech.speed_presets]`:

```toml
[speech.speed_presets]
skim = 400
normal = 265
study = 200
slow = 150
```

Setting this table replaces all four presets, so list every preset you want.

### Pitch

Pitch is a change from the voice's own pitch, in semitones, from 12 below to 12 above. The default is 0.

- **Alt+=** or **)**: higher by one semitone.
- **Alt+-** or **(**: lower by one semitone.

You hear "Pitch plus 2.", "Pitch minus 1.", or "Normal pitch." At the limits you hear "Highest pitch." or "Lowest pitch."

### Volume

Volume is a percentage from 0 to 100. The default is 100.

- **F7** or **0**: louder by 10.
- **Shift+F7** or **9**: quieter by 10.

You hear, for example, "Volume 90 percent." At the limits you hear "Full volume." or "Volume off."

Every change is saved at once.

## The speech settings

These are the `[speech]` settings in `settings.toml`, with their defaults. To see every setting and its value, run `tw settings export`. The [settings guide](settings.md) explains the file.

```toml
[speech]
backend = "auto"
rate = 265
volume = 100
pitch = 0
prefer_voice = "eloquence"
favorite_voices = []
punctuation = "some"
split_caps = false
caps = "pitch"
auto_play = false
skip_code = true
latency_offset_ms = 120
verbosity = "normal"
```

- `backend`: the engine id, or `"auto"`.
- `rate`: words per minute, 50 to 900.
- `volume`: 0 to 100.
- `pitch`: semitones, -12 to 12.
- `voice`: the voice, by id or name. It has no default; it is left out until you choose one.
- `prefer_voice`: part of a voice name to prefer when no voice is chosen. `""` means no preference.
- `favorite_voices`: your favorite voices, by id or name, listed first by Choose voice (**Alt+V**).
- `punctuation`: how much punctuation is spoken. `"none"` speaks none; it only shapes the voice. `"some"` speaks marks that carry meaning in prose, such as `@`, `#`, and `/`. `"all"` speaks every mark. **Alt+Shift+N** cycles it while textweaver runs.
- `split_caps`: speak the parts of words written in mixed capitals separately: "camelCase" as "camel Case", "XMLHttpRequest" as "XML Http Request".
- `caps`: how a capital letter is marked when a single character is spoken: `"pitch"`, `"tone"`, `"say_cap"`, or `"none"`. See [Writing and editing](editing.md).
- `auto_play`: start reading as soon as a document opens.
- `skip_code`: do not read code blocks aloud.
- `speed_presets`: the presets for **F8**, as above.
- `latency_offset_ms`: how many milliseconds to delay the highlight behind an engine's reported word time. See [How exactly words are highlighted](#how-exactly-words-are-highlighted).
- `output_device`: the sound device speech plays on, by its id. Not set by default: speech plays on the system's default device. See [Choose the sound device](#choose-the-sound-device).
- `verbosity`: how much textweaver says about what it does: `"low"`, `"normal"`, or `"high"`. **Alt+Shift+V** cycles it while textweaver runs. See [Reading and moving around](reading.md).

Both cycling keys say the new level and save it.

A value out of range is set to the nearest allowed value, and textweaver says which setting it changed.

### [speech.eci]: Eloquence

```toml
[speech.eci]
dictionaries = true
code_factory = false
```

- `dictionaries`: load the community IBMTTS pronunciation dictionaries that come with textweaver. `true` (the default) loads them, `false` turns them off, and a folder path loads your own copy instead.
- `library`: the Eloquence engine library to load. It has no default; textweaver searches the usual places.
- `code_factory`: also use Code Factory's Eloquence for Windows. Off by default, because a copy on a computer is not necessarily licensed for use with other programs. Turn it on only if you bought it.

### [speech.sapi]: Windows voices

```toml
[speech.sapi]
onecore = true
```

- `onecore`: also list the OneCore voices (Microsoft Mark, David, Zira). On by default.

### [speech.apple]: macOS

```toml
[speech.apple]
backend = "auto"
```

- `backend`: which Apple engine to try first: `"auto"`, `"nsspeech"`, or `"avspeech"`. The other stays available below it.

### [speech.dectalk]: DECtalk

```toml
[speech.dectalk]
library = "C:\\Path\\To\\DECtalk.dll"
```

- `library`: the DECtalk library to use. It has no default; textweaver searches the usual places. `TEXTWEAVER_DECTALK_LIBRARY` overrides it for one run. The [DECtalk guide](dectalk.md) explains it.

### [speech.piper]: Piper voices

```toml
[speech.piper]
voices = "D:\\Voices\\piper"
voice = "en_US-joe-medium"
phonemizer = "auto"
```

- `voices`: the folder of Piper voices. Default: `<data>/piper/voices`. `TEXTWEAVER_PIPER_VOICES` overrides it for one run.
- `voice`: the voice Piper starts with when `[speech] voice` does not name one of its voices. Default: the first English voice.
- `phonemizer`: `auto` (the installed eSpeak NG, else the built-in one), `library`, or `rust`.

`[speech.voice_params]` keeps each voice's own rate and pitch, written back whenever you change voices; you do not need to edit it by hand. All three sections, `[speech.dectalk]`, `[speech.piper]`, and `[speech.voice_params]`, are typed settings, so `tw settings export` lists them and `tw settings import` checks their values like any other setting.

## How text is prepared for speech

Before text reaches the engine, textweaver prepares it: it expands abbreviations, turns numbers into words, applies your pronunciations, and reads tables and footnotes the way you choose. The highlight still lands on the right word, because textweaver keeps a map from every spoken word back to the document.

These are the `[normalization]` settings, with their defaults:

```toml
[normalization]
abbreviations = true
numbers = true
use_pronunciations = true
table_mode = "structured"
footnote_mode = "inline"
math = true
math_verbosity = "normal"
```

- `abbreviations`: expand abbreviations, such as "Dr." to "Doctor".
- `abbrev_expansions`: your own abbreviations, as a table. Each line is `"abbreviation" = "expansion"`.
- `numbers`: say numbers, dates, times, and money as words.
- `use_pronunciations`: apply your pronunciation list.
- `pronunciations`: your pronunciation list, as a table. Each line is `word = "how to say it"`.
- `table_mode`: how tables are read. `"structured"` says row and column positions, `"flat"` reads only the cell text, and `"skip"` leaves tables out.
- `footnote_mode`: where footnotes are read. `"inline"` reads each where it is referenced, `"deferred"` at the end of the section, and `"skip"` not at all. It takes effect when a document is opened.
- `math`, `math_verbosity`, and `asciimath_delimiter`: spoken math. See [the math guide](math.md).

For example:

```toml
[normalization.pronunciations]
Nguyen = "win"
SQL = "sequel"

[normalization.abbrev_expansions]
"approx." = "approximately"
```

### Clinical and scientific text

Some things on the page must be read exactly as written, because a wrong guess changes the meaning. textweaver handles three of them:

- **Identifiers are read digit by digit.** A number after CPT, PMID, NCT, ZIP, DOI, ISBN, or phone (also telephone, tel, and fax) is an identifier, not an amount. "PMID 31769816" is "PMID three one seven six nine eight one six", not "thirty-one million ...". A US phone number in its usual forms, such as "503-555-0123" or "(503) 555-0123", is read the same way without a label. Dots inside an identifier are read as "dot". The highlight moves digit by digit.
- **Error-prone medical abbreviations are spelled, never expanded.** These are the abbreviations on The Joint Commission's "Do Not Use" list and the ISMP list of error-prone abbreviations, because they are misread and have caused harm: U, IU, QD, QOD (and q.d., q.o.d.), MS, MSO4, MgSO4, cc, SC, SQ, HS, TIW, AD, AS, AU, OD, OS, OU, and drug-name abbreviations such as TPA and HCTZ. "QD" is read "Q D" and "MgSO4" is "M G S O 4". Neither the built-in abbreviations nor your own `abbrev_expansions` can replace them. "µg" is read "micrograms", because the symbol itself is the hazard. "AS" and "AD" in text set in capitals ("SUCH AS") stay words.
- **Symbols outside math are named.** Greek letters ("TNF-α" is "TNF alpha", "ΔG" is "delta G"), the micro sign ("µm" is "micro m"), the minus sign "−", "⇌" ("in equilibrium with"), arrows, and powers of ten ("11.5 × 10^9" is "eleven point five times ten to the ninth", "10⁻³" is "ten to the negative third"). Greek letters are named at every punctuation level, because they are words.

With Eloquence, which reads numbers and abbreviations itself, the identifier rule and the error-prone list still apply, so Eloquence cannot guess either.

### The community lexicon

textweaver comes with the community IBMTTS pronunciation dictionaries. Eloquence loads them itself (see `[speech.eci] dictionaries`). For other engines you can apply them as a pronunciation list:

```toml
[normalization.community_lexicon]
enabled = true
language = "ENU"
```

- `enabled` (default `false`): apply them.
- `dir`: the folder with the dictionary files. When it is not set, textweaver looks in the `ibmtts-dictionaries` folder next to the programs, and in `TEXTWEAVER_ECI_DICTIONARIES`.
- `language` (default `"ENU"`): `"ENU"` for US English, or `"DEU"` for German.

### Engines that prepare text themselves

Eloquence reads numbers, dates, times, and abbreviations itself, and does it well. With Eloquence, and with Apple's Eloquence voices, textweaver leaves those to the engine. It still removes Markdown marks and applies your pronunciations, split caps, and punctuation level, and it still reads identifiers digit by digit and spells error-prone medical abbreviations (see [Clinical and scientific text](#clinical-and-scientific-text)). `tw backends` shows this as "reads numbers and abbreviations itself".

## How exactly words are highlighted

How closely the highlight follows the voice depends on what the engine tells textweaver.

- **Eloquence and DECtalk.** textweaver plays their audio itself and learns the moment each word is heard. The highlight is exact.
- **SAPI, eSpeak NG, and avspeech.** Each word comes with its time in the audio. textweaver shows it at that time plus `[speech] latency_offset_ms`, 120 ms by default, to allow for the sound card's delay. If the highlight runs ahead of what you hear, raise the offset; if it lags, lower it.
- **nsspeech and Speech Dispatcher.** The engine reports each word as it speaks it, and textweaver shows it at once.
- **Omnivox, and any voice that reports no words.** textweaver estimates: it moves the highlight one word at a time at the speaking rate. `[highlight] speed` (0.5 to 1.5) speeds the estimate up or slows it down. Word-level highlighting with Omnivox is not promised.

Some SAPI voices, such as Code Factory's, report no word times. textweaver tells you when you choose one: "This voice does not report words, so the word highlight is estimated."

## Keep the helper programs next to textweaver

Eloquence, SAPI voices, and DECtalk run in small helper programs, called engine hosts. They must stay in the same folder as `textweaver` and `tw`, with the dictionaries folder `ibmtts-dictionaries`:

- `textweaver-eci-host`, and on Windows `textweaver-eci-host-x86.exe`;
- on Windows, `textweaver-sapi-host.exe` and `textweaver-sapi-host-x86.exe`;
- `textweaver-dectalk-host`, and on Windows `textweaver-dectalk-host-x86.exe`.

If you copy only `textweaver` and `tw` somewhere else, those engines show as not available. The SAPI engine is available only when its 64-bit host is found. Environment variables can point to hosts elsewhere; see below.

## Choose the sound device

Speech plays on the system's default sound device unless you choose another. To choose one:

1. Run `tw backends --devices`. It prints one line per connected device, its name first and its id last, such as "Headset, chosen by your settings. Id: wasapi:..." The default comes first.
2. Copy the id into `[speech] output_device` in your settings, or set "Output device" in the settings screen.

```toml
[speech]
output_device = "the id from tw backends --devices"
```

The id stays the same when you restart or plug the device in again. A change in the settings screen moves speech to the new device at once.

When the chosen device is not connected, speech plays on the default, and `tw backends --devices` ends with a line that says so. When the device in use goes away, such as a headset unplugged or switched off, textweaver opens the sound output again at once, on the chosen device if it is back, else on the current default, and the reading goes on from where it was.

The setting applies to the engines whose audio textweaver plays itself: Eloquence, the Windows SAPI and OneCore voices, DECtalk, and Piper. eSpeak NG, Speech Dispatcher, and the Apple voices play on the system's default device. The setting belongs to this computer and never syncs.

## When the engine stops or goes silent

If an engine crashes, or plays nothing for 12 seconds while reading, textweaver restarts it and reads on from the last word you heard. You hear "Speech restarted:", the reason, then "Reading on from the last word." For example: "Speech restarted: no speech for 12 seconds. Reading on from the last word."

If it goes silent again right away, reading stops with "Speech error: no speech for 12 seconds, even after restarting the voice, so reading stopped. Check the audio device". After three failed sentences in a row, reading stops too, instead of failing through the whole document.

A helper program that is starting again (after a crash, or the first time you use a 32-bit voice) does not hold anything up: Stop and Pause work at once, and the sentence is spoken when the engine is ready. The audio device is opened in the background too.

If speech itself stops working (an internal error in textweaver's speech thread), textweaver goes silent at once and restarts speech by itself, once, with your current settings: "Speech stopped working", the reason, then "Restarting speech.", and a moment later "Speech restarted." After that, and at any time, **Shift+F8** (Restart speech) starts speech again with your current settings. The command palette (F2) has it as "Restart speech".

## Environment variables

These variables change how the engines are found for one run. They win over the settings.

- `TEXTWEAVER_ECI_LIBRARY`: the Eloquence engine library to load, instead of searching.
- `TEXTWEAVER_ECI_HOST`: the ECI host program to use, instead of the one next to textweaver.
- `TEXTWEAVER_ECI_DICTIONARIES`: `off`, or a folder of pronunciation dictionaries to use instead of the included ones. The community lexicon looks here too.
- `TEXTWEAVER_ECI_CODE_FACTORY`: set to `1` to use Code Factory's Eloquence for Windows, and to list Code Factory's SAPI voices. Only if you bought it.
- `TEXTWEAVER_SAPI_HOST`: the 64-bit SAPI host program.
- `TEXTWEAVER_SAPI_HOST_X86`: the 32-bit SAPI host program.
- `TEXTWEAVER_DECTALK_LIBRARY`: the DECtalk library to load.
- `TEXTWEAVER_DECTALK_HOST`: the DECtalk host program.
- `TEXTWEAVER_ESPEAK_OUTPUT`: set to `virtual` to have eSpeak NG make the audio and throw it away, keeping real timing. For machines with no sound device.
- `TEXTWEAVER_OMNIVOX`: the Omnivox server program, instead of `omnivox` on your `PATH`.
- `TEXTWEAVER_OMNIVOX_ARGS`: extra arguments for the Omnivox server, separated by spaces.

Speech Dispatcher also reads `SPEECHD_ADDRESS`, as its own programs do.

These are read only by the automated tests, to run them against real engines or test helpers: `TEXTWEAVER_ECI`, `TEXTWEAVER_SAPI`, `TEXTWEAVER_DECTALK`, `TEXTWEAVER_APPLE`, `TEXTWEAVER_SPEECHD`, `TEXTWEAVER_ECI_TEST_HOST`, `TEXTWEAVER_DECTALK_TEST_HOST`, and `TEXTWEAVER_DECTALK_TEST_LIBRARY`.

Other variables textweaver reads: `TEXTWEAVER_HOME` (where its files are; see [the library guide](library.md)), `TEXTWEAVER_LOG` (the log level; see [Troubleshooting](troubleshooting.md)), and `TEXTWEAVER_COLOR` (the color level; see [Themes](themes.md)).

On Windows, set a variable for good with `setx`, then open a new terminal:

```powershell
setx TEXTWEAVER_ECI_CODE_FACTORY 1
```

## If something goes wrong

- **No speech, or the wrong engine.** Run `tw backends` to see what is available. See [Troubleshooting](troubleshooting.md).
- **Eloquence is not found.** Run `tw eloquence`, and see the [Eloquence guide](eloquence.md).
- **A 32-bit SAPI voice is missing.** The 32-bit host is not next to textweaver. See [Troubleshooting](troubleshooting.md).
- **The highlight runs ahead or behind.** Change `[speech] latency_offset_ms`.

## See also

- [Getting ETI-Eloquence](eloquence.md): Eloquence on Windows, macOS, and Linux.
- [DECtalk](dectalk.md): using an installed DECtalk.
- [Settings](settings.md): `[interface] language`, the six built-in interface languages, and `[speech.voices_by_language]`.
- [Troubleshooting](troubleshooting.md): no speech, wrong voice, and other problems.
- [ADR-0003: Speech threading and event timing](adr/0003-speech-threading-and-event-timing.md): how the highlight follows speech.
- [ADR-0004: Rate, pitch, and volume](adr/0004-rate-pitch-volume.md): the engine-independent voice settings.
- [ADR-0007: Eloquence through the ECI host](adr/0007-eloquence-via-eci-host.md): how Eloquence is run.
- [ADR-0008: Apple speech](adr/0008-apple-speech.md): the two macOS engines.
- [ADR-0009: SAPI5 voices](adr/0009-sapi5-voices.md): Windows voices and their hosts.
- [ADR-0012: Engine hosts](adr/0012-engine-host.md): the helper programs.
- [ADR-0021: DECtalk](adr/0021-dectalk.md): the DECtalk engine.
- [Documentation index](README.md)
