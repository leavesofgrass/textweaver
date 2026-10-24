# Speech engines and voices

This guide describes how textweaver speaks. It covers the speech engines, the way one is chosen, voice selection, and the controls for rate, pitch, and volume. It also explains how text is prepared before an engine receives it, how closely each engine's word highlight follows the audio, and every speech setting and environment variable. Read it when you want a different voice or a faster rate, or when speech is not behaving as expected.

## The engines

Each speech engine has a short id, used by `--backend` and by the `backend` setting.

- `eci`: ETI-Eloquence, through its engine library (ECI). Windows and Linux. It runs in an engine host (see [Engine hosts](#engine-hosts-how-the-engines-run)), and a 32-bit Eloquence runs in the 32-bit host, so it works with 64-bit textweaver. textweaver does not include Eloquence; the [Eloquence guide](eloquence.md) explains how to obtain it. Eloquence reads numbers, dates, and abbreviations itself.
- `sapi`: Windows SAPI5 voices, including the OneCore voices (Microsoft David, Zira, Mark). Windows only. Voices run in an engine host: a copy of textweaver started in its hidden `--engine-host` mode for 64-bit voices, and the 32-bit host, `textweaver-host-x86.exe`, for older 32-bit-only voices.
- `dectalk`: DECtalk, from a copy you installed. See the [DECtalk guide](dectalk.md).
- `piper`: Piper neural voices, run inside textweaver on RTen, a model runtime written in Rust. Every platform. It becomes available once you download a voice in the voice manager (Alt+V), and it highlights words from the voice's own timing. See [Piper voices](#piper-voices) below.
- `nsspeech`: Apple's system voices through the classic engine. macOS only. It responds fastest.
- `avspeech`: Apple's system voices through AVSpeechSynthesizer. macOS only. It highlights words most precisely. On macOS 13 and later the system voices include Eloquence (Reed, Shelley, and others).
- `espeak`: eSpeak NG, from a copy you installed. It runs in one of two ways. On Linux and macOS it runs inside textweaver, in builds made with it (the Linux AppImage and tarball, and the Linux install script); install the `espeak-ng` package to use it. On Windows it runs in an engine host: install eSpeak NG with its Windows installer, 64-bit or 32-bit, and textweaver starts the host that matches that copy. Without eSpeak NG, the backend is not available. `TEXTWEAVER_ESPEAK_LIBRARY` names the library file to load instead, and a copy in textweaver's components folder, from your own components source, is loaded first (see [Optional components](components.md#your-own-components-source)). The setting `[speech.espeak] helper` chooses between the two ways; see [below](#speechespeak-espeak-ng).
- `speechd`: Speech Dispatcher, the speech service Orca uses on Linux. In builds made with it; the Linux install script includes it.
- `omnivox`: an Omnivox speech server, a separate program found on your `PATH`. It supports rate and tones, but not pitch, volume, or voice selection.
- `cw`: CW, Morse code sent as a tone, character by character, exactly as written. Every platform. It is used only when you choose it, never as a fallback. See [CW: reading in Morse code](cw.md).
- `null`: silent. It is used when no other engine works, and by `--no-speech`.

A further engine, `recording`, is silent and exists for automated tests; it is used only when named.

## How textweaver chooses an engine

The setting `[speech] backend` names the engine. Its default is `"auto"`.

With `"auto"`, textweaver uses the available engine with the highest priority. The order, from highest to lowest, is:

1. `eci`, 1000;
2. `sapi`, 500;
3. `dectalk`, 300;
4. `piper`, 200;
5. `nsspeech`, 80, and `avspeech`, 70;
6. `espeak`, 50;
7. `speechd`, 45;
8. `omnivox`, 40;
9. `null`, last.

An installed, licensed Eloquence is therefore always the first choice. On macOS, `[speech.apple] backend` can put `avspeech` above `nsspeech`, or the other way round.

When you name an engine, with `--backend` or in the settings, textweaver uses it if it is available. If it is not, textweaver does not fall silent; it makes the automatic choice instead and tells you. The reader says "Speech engine", the id, "is not available; using", and the engine it chose. If the chosen engine then fails to start, the reader says "Speech could not start", the reason, then "running silently."

A voice saved for one engine is never an error with another. If speech falls back to Windows SAPI 5 and later returns to Eloquence or DECtalk, the saved SAPI voice is set aside quietly; the engine keeps its own voice, and your rate, pitch, and volume still apply. To make a voice permanent for an engine, choose it in the voice manager.

`--no-speech` always means silent.

### Who speaks: the accessibility mode

When a screen reader is running, you may not want textweaver to speak everything. `[accessibility] mode` determines the division of labor:

- `"self-voicing"` (the default): textweaver speaks everything.
- `"hybrid"`: textweaver reads documents aloud; your screen reader speaks messages, typing, and cursor moves from the status line.
- `"screen-reader"`: textweaver never speaks. `--no-speech` starts in this mode.

**Alt+Shift+A** cycles the mode and saves it, and `--mode` sets it for one run. `tw serve` always speaks for itself, unless you start it with `--no-speech`. [Using textweaver with a screen reader](screen-readers.md) explains each mode.

## List the engines: tw backends

```bash
tw backends
```

It prints one line per engine: its id and name, whether it is available here, its priority, and what it supports. The line also states where the word highlight's timing comes from. "Word timing from the engine" means the engine reports where each word falls (Piper's comes from the voice model itself); "Word timing estimated" means textweaver paces the highlight. The engine the automatic choice picks says "Chosen automatically". For example:

```text
eci: ETI-Eloquence. Available. Priority 1000. Supports word highlighting, pause, pitch, volume, audio files, tones, and reads numbers and abbreviations itself. Word timing from the engine. Chosen automatically.
```

For use in a script, print JSON, which includes the automatic choice as `auto`:

```bash
tw backends --json
```

`tw backends --devices` lists the sound devices instead, for `[speech] output_device`; see [Choose the sound device](#choose-the-sound-device). It takes `--json` too.

## List the voices: tw voices

```bash
tw voices
```

It lists the voices of the engine named in your settings (`[speech] backend`), or of the automatic choice. The output begins with a count, such as "Windows SAPI5 voices has 6 voices.", followed by one line per voice giving its id, name, languages, gender, and tags. To list another engine's voices, name it with `--backend`:

```bash
tw voices --backend sapi
```

Add `--json` for JSON output. Listing voices starts the engine, which can take a few seconds.

## Hear a voice: tw speak

```bash
tw speak "The quick brown fox."
```

`tw speak` speaks the text and then reports, for example, "Spoke 1 sentence with Windows SAPI5 voices." It accepts these arguments:

- `TEXT`: the text to speak. Leave it out when you use `--file`. A `-` reads standard input, so a pipe can supply the text. To hear the clipboard, use: `Get-Clipboard | tw speak -` in PowerShell, `pbpaste | tw speak -` on macOS, and `wl-paste | tw speak -` (or `xclip -o -selection clipboard | tw speak -`) on Linux.
- `--file FILE`: speak a document instead. Any file the reader opens works.
- `--backend ID`: the engine. The default is the automatic choice. An engine that is not available falls back to the automatic choice, with a note.
- `--voice VOICE`: the voice, by id or by plain name, such as `Zira` or `Reed`.
- `--rate WPM`: the rate in words per minute.
- `--pitch SEMITONES`: the pitch offset in semitones, such as `-2`.
- `--out FILE`: write the audio to a file instead of playing it. The engine must be able to write audio files.
- `--json`: print the prepared text, its offset maps, and every status the engine reported, as JSON.

`tw speak`, `tw voices`, and `tw backends` read your `settings.toml`, as the reader does. `tw speak` applies your engine, voice, rate, pitch, and volume, together with your normalization, punctuation, table, footnote, and math settings; options given on the command line take precedence. Your saved voice belongs to your saved engine: when `--backend` names a different engine, that engine's default voice speaks unless you also give `--voice`. `--home FOLDER` reads the settings from another folder, like `TEXTWEAVER_HOME`.

To write a whole document to an audio file with subtitles, use `tw export-audio`. See [the audio export guide](audio-export.md).

## Choose a voice in the reader: Alt+V

Press **Alt+V** (or **Ctrl+Alt+V**, except in a macOS terminal, where VoiceOver keeps it) to open the voice manager. The graphical version uses **Ctrl+Alt+V** (**Cmd+Option+V** on macOS) and presents the filters and actions as buttons beside the list (see [the app guide](gui.md#voices)). On opening, you hear "Voice manager", how many voices are shown and which language and engine, then "Enter uses a voice and speaks a sample, or downloads one; Alt+End previews a voice; Space marks a favorite; Delete removes a downloaded voice; Escape closes." Alt+End is the Say Status key; if you changed it in `keymap.toml`, the voice manager names your key.

The voice chosen here reads documents. textweaver's own words (messages, lists, help, and settings) follow a separate choice, the interface language, available in six languages: English, Spanish, French, German, Brazilian Portuguese, and Arabic. `[speech.voices_by_language]` picks which voice speaks each interface language, so switching the interface to Spanish can also switch to a Spanish voice. See [`[interface] language`](settings.md#interface) in the settings guide for how to choose it and how textweaver falls back when no voice exists for it.

The list holds:

- **Language: all languages.** Press Enter to restrict the list to one language, then the next, and so on until it returns to all. You hear, for example, "3 voices: English, all engines."
- **Engine: all engines.** This row filters by engine and works the same way.
- **The voices.** Each says its name, language, engine, and tags; a favorite says "favorite", and the one in use says "current". Favorites come first. The list holds the running engine's voices and the Piper voices you have downloaded. After you fetch the Piper voice list, it also includes the Piper voices available for download, each with its size and, where its license requires, "non-commercial".
- **Fetch the Piper voice list from the internet.** Press Enter, then y, to download the catalog of Piper voices (about 250 KB).

Press **Enter** on a voice. You hear "Voice", its name, then a sample: "The quick brown fox jumps over the lazy dog." The choice is saved in `[speech] voice`. Choosing a voice that belongs to another engine switches engines: you hear "Switching engine", and speech restarts with that voice.

**Each voice keeps its own rate and pitch**, as screen readers do: change the rate on one voice, switch to another, and return, and the first voice has the rate you gave it. The values are saved in `[speech.voice_params]`.

Press **Space** on a voice to add it to, or remove it from, your favorites. You hear, for example, "Microsoft Zira added to favorites." Favorites are listed first, in the order you added them, and are saved in `[speech] favorite_voices`.

Each engine lists its voices once, when it starts, and keeps the list, so the manager opens at once. Windows voices are enumerated in the background (a helper program reads each registry); if you press **Alt+V** before that finishes, you hear "The voices are still loading. The list opens when they are ready.", and it opens by itself when they arrive.

When a voice changes what the engine can do, textweaver says so. For example: "This voice does not report words, so the word highlight is estimated." or "Pitch cannot be changed with this voice."

### Piper voices

Piper voices are neural voices, each a file of 20 to 140 MB, with a natural sound. textweaver runs them itself and needs no other program. Nothing is downloaded until you agree.

1. Press **Alt+V**, go to "Fetch the Piper voice list from the internet", press Enter, and answer y.
2. Press Alt+V again. Use the language row to show your language.
3. Press Enter on a voice to download. textweaver reads the voice's details and says, for example: "Download Joe, English (United States), medium quality, 63 MB? License: CC0. Free to use for anything. y or n". Answer y. Progress is announced at each quarter, followed by "Joe is installed."
4. Press Enter on the new voice to use it.

**Licenses differ from voice to voice.** Some voices are free for any use (`joe` is CC0; `kristin`, `norman`, and `cori` are public domain), some require attribution (`libritts_r` is CC BY), and some are limited to personal, non-commercial use (`lessac`, `ryan`, `hfc_female`). textweaver states the license before every download and ships no voice itself.

Every file is verified against the hash Hugging Face publishes for it; a file that does not match is discarded and nothing is installed. Voices are kept in `<data>/piper/voices/`. To remove one, press Delete on it in the voice manager and answer y.

To convert words to phonemes, Piper uses eSpeak NG: the installed library when there is one, otherwise a Rust implementation built into textweaver (English only). Piper voices in other languages require eSpeak NG to be installed.

### Voice names

Wherever textweaver accepts a voice, you may give its id or a plain name; `Zira`, `Reed`, or `David` is enough. textweaver matches, in order: the exact id, a voice with exactly that name, then a voice whose name and id contain every word you gave. When several voices match, a US English one is preferred.

When no voice is chosen, `[speech] prefer_voice` selects one: the first voice whose name or id contains that text, again preferring US English. Its default is `"eloquence"`, so an Eloquence voice is used when the engine offers one. Set it to `""` for no preference.

## Rate, pitch, and volume

### Rate

The rate is in words per minute, from 50 to 900. The default is 265.

- **+** or **=**: faster by 20.
- **-**: slower by 20.

The graphical version uses the same keys while reading, including the number pad's plus and minus; in it, **Ctrl+=** and **Ctrl+-** change the text size instead, and in edit mode, where these keys type, the toolbar's Slower and Faster buttons change the rate (on macOS, **Cmd+Option+]** and **Cmd+Option+[** also work in edit mode). Each change is announced, for example "285 words per minute.", and at the limits you hear "Fastest rate." or "Slowest rate." Each engine converts words per minute to its own scale, so a given rate sounds approximately the same on every engine.

### Speed presets: F8

**F8** steps through four preset rates, from fastest to slowest:

- skim, 350;
- normal, 265;
- study, 200;
- slow, 150.

You hear the preset name and then its rate, for example "Study, rate 200." Change the presets, or add your own, in `[speech.speed_presets]`:

```toml
[speech.speed_presets]
skim = 400
normal = 265
study = 200
slow = 150
```

Setting this table replaces all four presets, so list every one you want to keep.

### Pitch

Pitch is an offset from the voice's own pitch, in semitones, from 12 below to 12 above. The default is 0.

- **Alt+=** or **)**: higher by one semitone.
- **Alt+-** or **(**: lower by one semitone.

You hear "Pitch plus 2.", "Pitch minus 1.", or "Normal pitch.", and at the limits "Highest pitch." or "Lowest pitch."

### Volume

Volume is a percentage from 0 to 100. The default is 100.

- **F7** or **0**: louder by 10.
- **Shift+F7** or **9**: quieter by 10.

You hear, for example, "Volume 90 percent.", and at the limits "Full volume." or "Volume off."

Every change is saved immediately.

## The speech settings

This section lists the `[speech]` settings in `settings.toml` with their defaults. `tw settings export` prints every setting and its current value, and the [settings guide](settings.md) describes the file itself.

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
skip_code = false
latency_offset_ms = 120
pause_heading_ms = 400
pause_paragraph_ms = 300
pause_list_item_ms = 150
markup_pauses = true
verbosity = "normal"
```

- `backend`: the engine id, or `"auto"`.
- `rate`: words per minute, 50 to 900.
- `volume`: 0 to 100.
- `pitch`: semitones, -12 to 12.
- `voice`: the voice, by id or name. It has no default; it is left out until you choose one.
- `prefer_voice`: part of a voice name to prefer when no voice is chosen; `""` means no preference.
- `favorite_voices`: your favorite voices, by id or name, listed first in the voice manager (**Alt+V**).
- `punctuation`: how much punctuation is spoken. `"none"` speaks no marks and only shapes the intonation; `"some"` speaks marks that carry meaning in prose, such as `@`, `#`, and `/`; `"all"` speaks every mark. **Alt+Shift+N** cycles the level while textweaver runs.
- `split_caps`: speak the parts of mixed-capital words separately: "camelCase" as "camel Case", "XMLHttpRequest" as "XML Http Request".
- `caps`: how a capital letter is marked when a single character is spoken: `"pitch"`, `"tone"`, `"say_cap"`, or `"none"`. See [Writing and editing](editing.md).
- `auto_play`: start reading as soon as a document opens.
- `skip_code`: omit code blocks from reading. Off by default, so code blocks are read.
- `speed_presets`: the presets for **F8**, as above.
- `latency_offset_ms`: the delay, in milliseconds, between an engine's reported word time and the highlight. See [How exactly words are highlighted](#how-exactly-words-are-highlighted).
- `pause_heading_ms`, `pause_paragraph_ms`, `pause_list_item_ms`: the length of the pause after a heading, a paragraph, and a list item, in milliseconds, from 0 to 3000. A value of 0 turns that pause off. See [Pauses at headings, paragraphs and list items](#pauses-at-headings-paragraphs-and-list-items).
- `markup_pauses`, default `true`: treat pause markup in a document, such as `<break time="1s"/>`, as a pause. See [Pauses written in the text](#pauses-written-in-the-text).
- `output_device`: the id of the sound device speech plays on. It is unset by default, so speech uses the system's default device. See [Choose the sound device](#choose-the-sound-device).
- `verbosity`: how much textweaver says about its own actions: `"low"`, `"normal"`, or `"high"`. **Alt+Shift+V** cycles it while textweaver runs. See [Reading and moving around](reading.md).

Both cycling keys announce the new level and save it. A value outside its range is set to the nearest allowed value, and textweaver says which setting it changed.

### [speech.eci]: Eloquence

```toml
[speech.eci]
dictionaries = true
code_factory = false
```

- `dictionaries`: controls the community IBMTTS pronunciation dictionaries that come with textweaver. `true` (the default) loads them, `false` disables them, and a folder path loads your own copy instead.
- `library`: the Eloquence engine library to load. It has no default; textweaver searches the usual places.
- `code_factory`: also use Code Factory's Eloquence for Windows. It is off by default, because an installed copy is not necessarily licensed for use with other programs; enable it only if you bought it.

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

- `library`: the DECtalk library to use. It has no default; textweaver searches the usual places. `TEXTWEAVER_DECTALK_LIBRARY` overrides it for one run. See the [DECtalk guide](dectalk.md).

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

### [speech.cw]: Morse code

The character speed, the effective (Farnsworth) speed, the tone, its level, prosigns for structure, and the characters on the status line. See [CW: reading in Morse code](cw.md#the-settings).

### [speech.espeak]: eSpeak NG

```toml
[speech.espeak]
helper = "auto"
```

- `helper`: where eSpeak NG runs. `"auto"`, the default, runs it in its helper program on Windows when the helper and an installed eSpeak NG are both found, and inside textweaver otherwise; on Linux and macOS it runs inside textweaver, and turns to the helper only if the library cannot be loaded there. `"always"` uses the helper program and nothing else; `"never"` runs eSpeak NG inside textweaver and nothing else. In Settings this is "eSpeak NG helper program". A change takes effect when speech restarts.

The two ways differ in what happens around the engine, not in what you hear. In the helper, a crash in eSpeak NG ends only the helper: the sentence reports an error, and the next one starts a fresh helper while the reader carries on. textweaver also plays the helper's audio itself, so eSpeak NG then uses the sound device you chose, pauses at once, and is highlighted word by word as each word is heard (see [How exactly words are highlighted](#how-exactly-words-are-highlighted)). And the helper matches the library rather than textweaver, so a 32-bit eSpeak NG works with 64-bit textweaver, through the 32-bit host. Inside textweaver, eSpeak NG plays its own sound on the system's default device, and starts about a fifth of a second sooner when speech first starts. Both ways read the same words at the same moments: they drive the same library with the same settings.

`[speech.voice_params]` keeps each voice's own rate and pitch, written back whenever you change voices; you do not need to edit it by hand. All four sections, `[speech.dectalk]`, `[speech.espeak]`, `[speech.piper]`, and `[speech.voice_params]`, are typed settings, so `tw settings export` lists them and `tw settings import` checks their values like any other setting.

## How text is prepared for speech

Before text reaches the engine, textweaver prepares it: it expands abbreviations, converts numbers to words, applies your pronunciations, and renders tables and footnotes as you have configured. The highlight still lands on the correct word, because textweaver keeps a map from every spoken word back to the document.

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

Some content must be read exactly as written, because a wrong interpretation changes the meaning. textweaver treats four kinds of it specially:

- **Identifiers are read digit by digit.** A number after CPT, PMID, NCT, ZIP, DOI, ISBN, or phone (also telephone, tel, and fax) is an identifier, not an amount. "PMID 31769816" is "PMID three one seven six nine eight one six", not "thirty-one million ...". A US phone number in its usual forms, such as "503-555-0123" or "(503) 555-0123", is read the same way without a label. Dots inside an identifier are read as "dot". The highlight moves digit by digit.
- **Error-prone medical abbreviations are spelled, never expanded.** These are the abbreviations on The Joint Commission's "Do Not Use" list and the ISMP list of error-prone abbreviations, because they are misread and have caused harm: U, IU, QD, QOD (and q.d., q.o.d.), MS, MSO4, MgSO4, cc, SC, SQ, HS, TIW, AD, AS, AU, OD, OS, OU, and drug-name abbreviations such as TPA and HCTZ. "QD" is read "Q D" and "MgSO4" is "M G S O 4". Neither the built-in abbreviations nor your own `abbrev_expansions` can replace them. "µg" is read "micrograms", because the symbol itself is the hazard. "AS" and "AD" in text set in capitals ("SUCH AS") stay words.
- **Symbols outside math are named.** Greek letters ("TNF-α" is "TNF alpha", "ΔG" is "delta G"), the micro sign ("µm" is "micro m"), the minus sign "−", "⇌" ("in equilibrium with"), arrows, and powers of ten ("11.5 × 10^9" is "eleven point five times ten to the ninth", "10⁻³" is "ten to the negative third"). Greek letters are named at every punctuation level, because they are words.

- **Units, ranges, and times are said in full.** A unit after a number is read as a word: "5 mg" is "5 milligrams", "1 mg" is "1 milligram", and mcg, ng, g, kg, mL, dL, L, mmol, mEq, mm, cm, °C, and °F the same way. A slash after the unit is "per": "2 mg/kg/day" is "2 milligrams per kilogram per day", "5 mmol/L" is "5 millimoles per liter", and "10^9/L" is "ten to the ninth per liter". "120/80 mmHg" is "120 over 80 millimeters of mercury". An en dash between numbers is a range: "6–8 weeks" is "6 to 8 weeks". A 24-hour time gains no AM or PM: "08:05" is "eight oh five", "15:30" is "fifteen thirty", and "08:00" is "oh eight hundred". A decimal comma before a unit or a percent sign, such as "2,5%", is read as a decimal point: "two point five percent".

With Eloquence, which reads numbers and abbreviations itself, the identifier rule and the error-prone list still apply, so Eloquence cannot guess either.

### The medical lexicon

textweaver comes with a medical pronunciation list for drug names, clinical terms, and eponyms that engines get wrong, such as "warfarin", "levetiracetam", "dyspnea", "Guillain-Barré", and "Sjögren". It is off by default; turn it on in a health sciences profile:

```toml
[normalization.medical_lexicon]
enabled = true
```

- `enabled` (default `false`): apply it.
- `overlay`: your own file of medical pronunciations. When it is not set, textweaver reads `medical-lexicon.toml` in the settings folder, if there is one.

Each word is read as a respelling, with the stressed syllable in capitals: "warfarin" is read "WAR-fuh-rin", and "hydroxyzine" and "hydralazine", a look-alike pair, are "hye-DROK-sih-zeen" and "hye-DRAL-uh-zeen". A word in any capitalization matches. It also reads safe dosing abbreviations: "PRN" is "as needed", "PO" is "by mouth", "BID" is "twice a day", and "q6h" is "every 6 hours". The error-prone abbreviations above are never expanded, by this list or by yours.

Tall Man drug names, such as "hydrOXYzine" and "DOPamine", are printed with capitals to tell look-alikes apart. textweaver keeps them whole: split caps never cuts one into pieces, with the lexicon on or off, and with it on they are read as their respelling or their plain name.

Your overlay file uses the same form as `[normalization.pronunciations]`: one `term = "spoken form"` per line. Its entries win over the built-in ones. A term in lower case matches any capitalization; a term with a capital letter matches only as written. For example:

```toml
warfarin = "WAR-far-in"
metformin = "met FOR min"
PRN = "when needed"
```

Your own `[normalization.pronunciations]` apply first, so they always win. The respellings are a starting point; if one does not sound right with your voice, put your own in the overlay file.

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

CW sends the text exactly as written and skips all of this: it has no numbers or abbreviations to read in words, and no structure spoken as words (headings and paragraphs are sent as prosigns). See [CW](cw.md#what-is-sent).

## Pauses at headings, paragraphs and list items

When textweaver reads a document, it leaves a short silence after each heading, paragraph, and list item, so the boundaries between them are audible. At high rates this cue is often the first to be lost. No words are added.

By default the pauses are 400 milliseconds after a heading, 300 after a paragraph, and 150 after a list item. Each is set in `[speech]` (`pause_heading_ms`, `pause_paragraph_ms`, `pause_list_item_ms`, from 0 to 3000), or in Settings as "Pause after headings", "Pause after paragraphs" and "Pause after list items". 0 turns a pause off. The lengths are for the default rate, 265 words per minute. Faster, they shorten in proportion: at 530 words per minute they are half as long, and never shorter than a quarter.

How the pause is made depends on who plays the sound:

- **Eloquence, Windows voices (SAPI), DECtalk, Piper, and eSpeak NG in its helper program.** textweaver plays these engines' audio itself, and plays the pause as silence between two sentences. The next sentence is prepared while the first is spoken, as always, so the pause adds nothing to the wait.
- **Apple voices, Speech Dispatcher, and eSpeak NG inside textweaver.** These engines play their own sound. textweaver waits for the sentence to end, waits out the pause, then hands the engine the next sentence.

Either way:

- The highlight stays on the last word of the heading or paragraph during the pause and moves to the next word as soon as it is heard.
- **Stop** ends a pause at once, and **Pause** holds it: nothing is heard until you resume.
- Skipping to the next sentence or heading starts it straight away, without the rest of the pause.
- A pause comes only between two parts of what is being read. Saying one sentence or one line has none, and neither does the end of a reading.

`tw speak --file` pauses the same way.

## Pauses written in the text

A document can request a pause itself, using SSML-style markup in its text:

- `<break time="500ms"/>` or `<break time="1s"/>`: a pause of that length, up to 10 seconds.
- `<break strength="medium"/>`: a pause by strength. `x-weak` is 250 milliseconds, `weak` 500, `medium` 750, `strong` 1000 and `x-strong` 1250. `none` is no pause. A bare `<break/>` is `medium`.

textweaver reads each break as a pause of that length where it stands, with every engine, the same way as the pauses above. Written pauses keep their length at every rate, and the structural pause settings do not change them. A break the text cannot be read from, such as `time="soon"`, stays as written.

The markup is never spoken and never shown: the reading view and the Braille display show the text without it, and the highlight stays on the right words. Breaks are read in plain text, Markdown and HTML files. In Edit mode you see and edit the markup as written, and saving keeps it.

For a document that quotes SSML as an example, disable this: `markup_pauses = false` in `[speech]`, or "Pauses written as markup" in Settings. Plain text then shows and reads the markup as text; Markdown and HTML leave the tag out, as other unknown tags. Reopen the document after changing it.

## How exactly words are highlighted

How closely the highlight follows the voice depends on what the engine reports to textweaver.

- **Eloquence, DECtalk, and eSpeak NG in its helper program.** textweaver plays their audio itself and learns the moment each word is heard. The highlight is exact.
- **SAPI, eSpeak NG inside textweaver, and avspeech.** Each word comes with its time in the audio. textweaver shows it at that time plus `[speech] latency_offset_ms`, 120 ms by default, to allow for the sound card's delay. If the highlight runs ahead of what you hear, raise the offset; if it lags, lower it.
- **nsspeech and Speech Dispatcher.** The engine reports each word as it speaks it, and textweaver shows it at once.
- **CW (Morse code).** Exact, and not reported by an engine: textweaver works out where every word starts in the tones it plays before it plays them, and shows each word at that time on the audio clock (plus `[speech] latency_offset_ms`, as for SAPI), so the highlight stops when the audio pauses.
- **Omnivox, and any voice that reports no words.** textweaver estimates: it moves the highlight one word at a time at the speaking rate. `[highlight] speed` (0.5 to 1.5) speeds the estimate up or slows it down. Word-level highlighting with Omnivox is not promised.

Some SAPI voices, such as Code Factory's, report no word times. textweaver tells you when you choose one: "This voice does not report words, so the word highlight is estimated."

## Engine hosts: how the engines run

Eloquence, SAPI voices, DECtalk, and (on Windows) eSpeak NG run in separate processes called engine hosts, so a crash in an engine cannot take textweaver down and each word's timing is exact. There is no separate program per engine:

- For an engine library of the same architecture as textweaver (64-bit on a 64-bit computer), textweaver starts a copy of itself in a hidden host mode, `--engine-host` followed by the engine's name. A second `textweaver.exe` therefore appears in Task Manager while that engine speaks, and it ends when textweaver does.
- On Windows, a 32-bit engine library (Code Factory's Eloquence, most DECtalk libraries, older 32-bit-only SAPI voices, a 32-bit eSpeak NG) runs in the one 32-bit host, `textweaver-host-x86.exe`. It must stay in the same folder as `textweaver.exe` and `tw.exe`. Without it, 32-bit voices are not listed and a 32-bit library is refused in words.

The dictionaries folder, `ibmtts-dictionaries`, must stay beside the programs too. Environment variables can point to a host elsewhere; see below.

## Choose the sound device

Speech plays on the system's default sound device unless you select another. To select one:

1. Run `tw backends --devices`. It prints one line per connected device, its name first and its id last, such as "Headset, chosen by your settings. Id: wasapi:..." The default comes first.
2. Copy the id into `[speech] output_device` in your settings, or set "Output device" in the settings screen.

```toml
[speech]
output_device = "the id from tw backends --devices"
```

The id is stable across restarts and reconnections. A change in the settings screen moves speech to the new device at once.

When the chosen device is not connected, speech plays on the default, and `tw backends --devices` ends with a line that says so. When the device in use goes away, such as a headset unplugged or switched off, textweaver reopens the sound output immediately, on the chosen device if it has returned and otherwise on the current default, and reading continues from where it was.

The setting applies to the engines whose audio textweaver plays itself: Eloquence, the Windows SAPI and OneCore voices, DECtalk, Piper, and eSpeak NG in its helper program. eSpeak NG inside textweaver, Speech Dispatcher, and the Apple voices play on the system's default device. The setting belongs to this computer and never syncs.

## When the engine stops or goes silent

If an engine crashes, or plays nothing for 12 seconds while reading, textweaver restarts it and reads on from the last word you heard. You hear "Speech restarted:", the reason, then "Reading on from the last word." For example: "Speech restarted: no speech for 12 seconds. Reading on from the last word."

If it goes silent again right away, reading stops with "Speech error: no speech for 12 seconds, even after restarting the voice, so reading stopped. Check the audio device". After three failed sentences in a row, reading stops too, instead of failing through the whole document.

A host that is starting (after a crash, or the first time you use a 32-bit voice) does not block anything: Stop and Pause work immediately, and the sentence is spoken once the engine is ready. The audio device is also opened in the background.

If speech itself stops working (an internal error in textweaver's speech thread), textweaver goes silent at once and restarts speech once on its own, with your current settings: "Speech stopped working", the reason, then "Restarting speech.", and a moment later "Speech restarted." After that, and at any time, **Shift+F8** (Restart speech) starts speech again with your current settings. The command palette (F2) has it as "Restart speech".

## Environment variables

These variables change how engines are located for a single run, and they take precedence over the settings.

- `TEXTWEAVER_ECI_LIBRARY`: the Eloquence engine library to load, instead of searching.
- `TEXTWEAVER_ECI_HOST`: a host program to run Eloquence in, instead of textweaver itself or `textweaver-host-x86.exe`. It may be a separate host from an earlier release, such as `textweaver-eci-host.exe`, or the 32-bit host.
- `TEXTWEAVER_ECI_DICTIONARIES`: `off`, or a folder of pronunciation dictionaries to use instead of the included ones. The community lexicon looks here too.
- `TEXTWEAVER_ECI_CODE_FACTORY`: set to `1` to use Code Factory's Eloquence for Windows, and to list Code Factory's SAPI voices. Only if you bought it.
- `TEXTWEAVER_SAPI_HOST`: a host program for 64-bit SAPI voices, instead of textweaver itself.
- `TEXTWEAVER_SAPI_HOST_X86`: a host program for 32-bit SAPI voices, instead of the `textweaver-host-x86.exe` beside textweaver.
- `TEXTWEAVER_DECTALK_LIBRARY`: the DECtalk library to load.
- `TEXTWEAVER_DECTALK_HOST`: a host program to run DECtalk in, instead of textweaver itself or `textweaver-host-x86.exe`.
- `TEXTWEAVER_ESPEAK_LIBRARY`: the eSpeak NG library to load, after the components folder's copy.
- `TEXTWEAVER_ESPEAK_HOST`: a host program to run eSpeak NG in, instead of textweaver itself or `textweaver-host-x86.exe`.
- `TEXTWEAVER_ESPEAK_OUTPUT`: set to `virtual` to have eSpeak NG make the audio and throw it away, keeping real timing, in either way of running it. For machines with no sound device.
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
- [CW: reading in Morse code](cw.md): sending a document as Morse code, keying it, and practicing.
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
