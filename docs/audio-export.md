# Exporting audio and subtitles

`tw export-audio` reads a document aloud into an audio file, so you can listen to it later on a phone, a music player, or a book player. It can write a WAV file, an MP3 file, or an M4B audiobook with one chapter for each heading. It can also write subtitles: a caption file that shows each sentence, or each word, at the moment it is spoken. This is for anyone who wants to take a reading with them, for example a student who wants an audiobook of this week's chapters, or a teacher who wants captions that follow the spoken text.

This guide is written to be read with a screen reader. Each task section starts with the command, then explains it.

## Before you start

You need two things: a voice that can write audio files, and, for MP3 and M4B only, the free program ffmpeg.

### Check that you have a voice that can write files

```bash
tw backends
```

This lists the speech engines textweaver knows about. An engine that can write files has "audio files" in its list of what it supports. For example:

```text
sapi: Windows SAPI5 voices. Available. Priority 500. Supports word highlighting, pause, pitch, volume, audio files, and tones.
```

These engines can write files: ETI-Eloquence (`eci`), Windows SAPI5 voices (`sapi`), DECtalk (`dectalk`), eSpeak NG (`espeak`), and the two Apple engines on macOS (`nsspeech` and `avspeech`). Omnivox (`omnivox`) and Speech Dispatcher (`speechd`) can only speak aloud, so they cannot export. The silent engine (`null`) cannot export either.

### WAV always works

WAV files are written by textweaver itself. You do not need anything else. WAV files are large, but every player can open them.

### MP3 and M4B need ffmpeg

To write `.mp3` or `.m4b`, textweaver first writes a WAV and then asks ffmpeg to convert it. textweaver never downloads or bundles ffmpeg, so you install it yourself once.

textweaver looks for ffmpeg in two places, in this order:

1. The `TEXTWEAVER_FFMPEG` environment variable, if it is set. It must be the full path to the ffmpeg program.
2. Otherwise, a program named `ffmpeg` in one of the folders on your `PATH`, which is where installers put it.

### Install ffmpeg on Windows

You can use the Windows package manager, winget. For example:

```powershell
winget install Gyan.FFmpeg
```

When it finishes, close the terminal window and open a new one, so the new `PATH` is read. Then check that ffmpeg is found:

```bash
ffmpeg -version
```

You hear a line that starts with "ffmpeg version". If you hear that the command is not recognized, see [If something goes wrong](#if-something-goes-wrong).

### Install ffmpeg on macOS

With Homebrew installed, run:

```bash
brew install ffmpeg
```

### Install ffmpeg on Linux

Use your distribution's package manager. The package is usually called `ffmpeg`. For example, on Debian or Ubuntu:

```bash
sudo apt install ffmpeg
```

### Tell textweaver where ffmpeg is

You only need this when ffmpeg is installed in a folder that is not on your `PATH`. In PowerShell, for the current window:

```powershell
$env:TEXTWEAVER_FFMPEG = "C:\Tools\ffmpeg\bin\ffmpeg.exe"
```

In a Linux or macOS shell:

```bash
export TEXTWEAVER_FFMPEG="$HOME/tools/ffmpeg"
```

When `TEXTWEAVER_FFMPEG` is set, textweaver uses only that path and does not search `PATH`. If the path is wrong, textweaver says ffmpeg was not found, even when ffmpeg is on your `PATH`.

## Export a document to a WAV file

```bash
tw export-audio reading.md --out reading.wav
```

This reads `reading.md` aloud, silently, into `reading.wav`. Nothing plays through your speakers. You can export any document textweaver can open, not only Markdown.

The document is read exactly as textweaver would read it to you: with your rate, pitch, volume, and normalization settings, and with the same announcements, such as "heading level 1". Each sentence is written one after another into one file.

The output format comes from the file name you give with `--out`:

- `.wav`: a WAV file. Always works.
- `.mp3`: an MP3 file. Needs ffmpeg. It is encoded with the LAME encoder at variable bit rate, quality 2.
- `.m4b`: an M4B audiobook (AAC audio at 64 kilobits per second). Needs ffmpeg.

Capital letters in the extension are fine, so `.MP3` works too. Any other extension is refused before anything is read.

If the output file already exists, it is replaced without asking.

### What you hear while it works

Progress messages come in whole tens of percent, one short sentence each, so your screen reader is not flooded. For example: "10 percent done." A message is given only when a new ten is reached after a sentence, so with a short document some tens are skipped.

When the export is done, you hear one sentence, and a second one when subtitles were written. This is a real result from a short document with two headings:

```text
Wrote reading.wav: 9 seconds, 7 sentences, 2 chapters, with Recording (test double). Subtitles in reading.srt.
```

The sentence says:

- The file written.
- The length of the audio, in words, such as "1 hour, 2 minutes, 5 seconds".
- How many pieces were read. Headings count, so a document with two headings and five sentences gives "7 sentences".
- How many chapters were found.
- The engine used. "Recording (test double)" is textweaver's test engine; with a real voice you hear its engine name, such as "Windows SAPI5 voices", "ETI-Eloquence", "DECtalk", or "eSpeak NG".

Progress messages go to the error output and the result sentence to the standard output, so you can redirect them separately.

To stop an export, press Control C. A partly written file may be left behind. Delete it and start again.

## Make an audiobook with chapters

```bash
tw export-audio "Chapter 3.docx" --out "Chapter 3.m4b"
```

This writes an M4B audiobook: one file with chapters. Audiobook players can open it, remember your place, and jump from chapter to chapter.

The M4B file also carries:

- The document's title, as the title and the album.
- The document's author, when the document names one, as the artist.
- The genre "Audiobook".
- One chapter for each heading, with the heading as its name.

While ffmpeg works, textweaver keeps the full WAV and a small metadata file in a hidden folder next to the output, whose name starts with `.textweaver-export-`. The folder is removed when the export finishes. You need enough free disk space for the full WAV, which is much larger than the finished M4B.

### How chapters are chosen

- Every heading, at every level from 1 to 6, starts a chapter. The chapter's name is the heading's text (up to 200 characters).
- Every section break also starts a chapter: in an EPUB, each file of the book; in a Word document, each section. When a section starts with a heading, the two make one chapter, named by the heading. A section without a heading is named by its label, or "Chapter 1", "Chapter 2", and so on when it has none.
- Text before the first heading becomes its own first chapter, named after the document's title, or "Audiobook" when the document has no title.
- A document with no headings at all is one chapter.
- A heading with no audio of its own, for example two headings read in one piece, is left out.
- Each chapter starts at the moment its heading is reached in the audio, measured, not estimated.

There is no option yet to choose which heading levels make chapters.

### Chapters in MP3 and WAV files

```bash
tw export-audio "Chapter 3.docx" --out "Chapter 3.mp3"
```

MP3 files get the same title and chapters as M4B files, as ID3 chapter tags. Players that read ID3 chapters show them. Many simple players ignore them and play the file straight through.

WAV files have no chapters inside them. The result sentence still counts them, and the JSON report lists each chapter's name, start, and end (see [Get a report as JSON](#get-a-report-as-json)).

## Add subtitles

```bash
tw export-audio reading.md --out reading.wav --subtitles reading.srt
```

This writes the audio and a subtitle file next to it. The format comes from the extension you give with `--subtitles`:

- `.srt`: SubRip, the most widely supported format.
- `.vtt`: WebVTT, the format web pages use. The file starts with the line `WEBVTT`.

Any other extension is refused with a message, before any audio is made.

Subtitles work with every audio format, so you can combine `--subtitles` with `.wav`, `.mp3`, or `.m4b`. Give the subtitle file the same name as the audio file, with its own extension, and most players load it by themselves.

This is the start of the real SRT file from the example above:

```text
1
00:00:00,750 --> 00:00:01,000
Photosynthesis

2
00:00:01,000 --> 00:00:02,250
Plants make food from light.

3
00:00:02,250 --> 00:00:03,750
They need water and carbon dioxide.
```

How captions are made:

- Each sentence gets its own caption. Its start and end are measured from the audio, so captions stay in step with the voice even at the end of a long book.
- A long sentence is split into several caption lines. A line holds at most 12 words and at most 90 characters.
- Captions show the document's own text, not the words the voice says. A price written "$5" and read "five dollars" is captioned "$5".
- Things that are spoken but are not in the document have no caption. In the example, the voice says "heading level 1" before "Photosynthesis", so that caption starts a little after the audio does.
- A caption never ends before it starts; a caption with no length is given 50 milliseconds.

## One subtitle per word

```bash
tw export-audio reading.md --out reading.wav --subtitles reading.vtt --word-level
```

`--word-level` gives each word its own cue, instead of whole caption lines. This suits tools that highlight one word at a time, like karaoke. This is the start of the real WebVTT file from the example document:

```text
WEBVTT

00:00:00.750 --> 00:00:01.000
Photosynthesis

00:00:01.000 --> 00:00:01.250
Plants

00:00:01.250 --> 00:00:01.500
make
```

Punctuation stays with its word, so the last word of a sentence reads "light." rather than "light".

### Which engines time each word exactly

Some engines report the exact moment each word sounds in the file. With them, word cues are exact. They are:

- ETI-Eloquence through textweaver's own Eloquence support (`eci`).
- Windows SAPI5 voices (`sapi`), except Code Factory's Eloquence voices through SAPI, which report no word times.
- DECtalk (`dectalk`).
- eSpeak NG (`espeak`).

The Apple engines (`nsspeech` and `avspeech`) and the SAPI Eloquence voices can write files but do not report word times. With them, word cues are estimated. Each sentence's start and end are still measured. The time between them is shared among its words by length, so longer words get more time. Sentence captions from these engines are just as accurate as from any other engine, because they depend only on the measured sentence times.

## Choose the engine, voice, rate, and pitch

```bash
tw export-audio reading.md --out reading.mp3 --backend sapi --voice David --rate 200 --pitch -2
```

Each option is optional:

- `--backend`: the engine, by its id from `tw backends`, such as `eci`, `sapi`, `dectalk`, or `espeak`.
- `--voice`: a voice, by its id or its name. Words from the name are enough: "David" finds "Microsoft David Desktop". `tw voices --backend sapi` lists the voices of an engine.
- `--rate`: the speaking rate in words per minute, from 50 to 900. Values outside that range are brought inside it.
- `--pitch`: a pitch change in semitones, from -12 to 12. A negative number lowers the voice.

Without `--backend`, textweaver chooses in this order:

1. The engine in your `[speech] backend` setting, if it is installed and can write files.
2. Otherwise, the best installed engine that can write files: ETI-Eloquence first, then Windows SAPI5 voices, DECtalk, the Apple engines, and eSpeak NG. Engines that can only speak aloud are passed over.

If the engine you name with `--backend` is not installed, textweaver chooses automatically instead. When that works, you hear, for example, "Backend eci is not available; using sapi." after the result.

Without `--voice`, textweaver uses your `[speech] voice` setting, but only when the chosen engine is the one in your `[speech] backend` setting, because a voice belongs to its engine. Otherwise the engine's own default voice is used.

### Settings the export reads

`tw export-audio` reads your settings, so the audio sounds the way textweaver reads to you. These settings are used:

- `[speech] backend` and `[speech] voice`, as described above.
- `[speech] rate` and `[speech] pitch`. The `--rate` and `--pitch` options override them for this export.
- `[speech] volume`. There is no option for it; change the setting.
- `[speech] punctuation` and `[speech] split_caps`: how much punctuation is spoken, and whether capitals inside words are spoken separately.
- `[speech] skip_code`: whether code blocks are left out.
- `[speech] verbosity`: how much structure is announced. At `low`, headings are not announced as "heading level 1".
- `[normalization]`: numbers, abbreviations, your pronunciations, math, and the community lexicon, the same as when reading aloud. Engines that read numbers and abbreviations themselves, such as ETI-Eloquence, are left to do so.
- `[normalization] table_mode`: tables read with row and column context (`structured`), as cell text only (`flat`), or left out (`skip`).
- `[normalization] footnote_mode`: footnotes read where they are referenced (`inline`), at the end of the section (`deferred`), or left out (`skip`).
- The three `[export]` settings, described next.

See [the settings guide](settings.md) for how to look at, export, and import your settings.

## Write subtitles every time

```bash
tw settings import export-settings.toml
```

The `[export]` settings let you have subtitles without typing `--subtitles` each time. Put them in a small TOML file, such as `export-settings.toml`, and import it. Import changes only the settings the file names. Add `--dry-run` first to hear what would change.

```toml
[export]
subtitles_with_audio = true
subtitle_format = "vtt"
subtitle_word_level = false
```

The three settings:

- `subtitles_with_audio`: `true` or `false`. The default is `false`. When `true`, every export also writes subtitles next to the audio, with the same name and the subtitle format's extension. Exporting `book.mp3` then also writes `book.srt`.
- `subtitle_format`: `"srt"` or `"vtt"`. The default is `"srt"`. It is used only for the file `subtitles_with_audio` names. A file you name with `--subtitles` always uses its own extension.
- `subtitle_word_level`: `true` or `false`. The default is `false`. When `true`, subtitles always have one cue per word, as if you had added `--word-level`.

A file named with `--subtitles` always wins over `subtitles_with_audio`. `--word-level` turns word cues on even when `subtitle_word_level` is `false`; there is no option to turn them off for one export when the setting is `true`.

These settings are used by `tw export-audio`. The reader does not have an audio export command yet, so in the reader there is nothing to start and nothing is announced. Use `tw export-audio` from a terminal.

## Get a report as JSON

```bash
tw export-audio reading.md --out reading.wav --json
```

`--json` prints a full report instead of the result sentence, for scripts and other programs. Progress messages are left out, as with `--quiet`. The report has two parts.

`backend` says which engine was used:

- `backend`: the engine's `id`, `name`, `priority`, whether it is `opt_in` (used only when named) and `available`, and its capabilities (`caps`).
- `requested`: the engine you named, or `null`.
- `fell_back`: `true` when the engine you named was not available and another was chosen.

`export` says what was written:

- `out`: the audio file.
- `format`: `wav`, `mp3`, or `m4b`.
- `subtitles`: the subtitle file, or `null`.
- `ffmpeg`: the ffmpeg program used, or `null` for WAV.
- `timeline`: everything about the audio.

`timeline` holds:

- `duration_ms`: the length of the audio in milliseconds.
- `title` and `author`: from the document, or `null`.
- `chapters`: each chapter's `title`, `start_ms`, `end_ms`, and `source_start` (where it starts in the document, in characters).
- `sentences`: each piece read, with `start_ms` and `end_ms`; `source`, its place in the document; `text`, its caption (the document's text); `spoken`, what the voice actually said (for a heading, for example "heading level 1, Photosynthesis"); and `words`.
- `words`: for engines that report word times, each word's `start_ms`, `end_ms`, `source`, `caption` (its place in the caption text), and `text`. The list is empty for engines that do not report word times.

## Quiet output

```bash
tw export-audio reading.md --out reading.m4b --quiet
```

`--quiet` leaves out the progress messages. You still hear the result sentence at the end, and any error.

## Use settings from another folder

```powershell
tw export-audio reading.md --out reading.wav --home "E:\textweaver"
```

`--home` reads settings from that folder instead of your usual settings, in the same way as the `TEXTWEAVER_HOME` environment variable. The settings file is `config\settings.toml` inside that folder. Use it for a portable copy of textweaver on a USB stick, or to try settings without changing your own.

## If something goes wrong

Every problem is reported as one sentence starting with "Error:". These are the messages you may hear, and what to do.

- "Error: writing MP3 needs ffmpeg, which was not found; install ffmpeg, set TEXTWEAVER_FFMPEG to its path, or export to .wav". For an M4B file it says "writing M4B". ffmpeg is not installed, or textweaver cannot find it. Install it as described in [Before you start](#before-you-start). If you just installed it, open a new terminal window. If `TEXTWEAVER_FFMPEG` is set, check that it is the full path to the program, including `ffmpeg.exe` on Windows, or remove the variable. Or export to `.wav`, which needs nothing.
- "Error: cannot write notes.ogg: use a .wav, .mp3, or .m4b file name". Only WAV, MP3, and M4B can be written. OGG is not available yet.
- "Error: Cannot write subtitles to notes.txt: use a .srt or .vtt file name." Give the subtitle file a `.srt` or `.vtt` extension.
- "Error: no installed voice can write audio files; install espeak-ng, or choose one with --backend". textweaver found no engine that can write files. Run `tw backends` and look for "audio files". On Windows, the SAPI5 voices usually can. On Linux, install eSpeak NG; see [the speech guide](speech.md).
- "Error: Silent (no audio) cannot write audio files; choose another voice with --backend". The engine chosen cannot write files. The name at the start is the engine's, for example "Omnivox speech server". This also happens when you name an engine with `--backend` that is not installed and the automatic choice falls on an engine that cannot write files. Run `tw backends`, and name an engine that is available and lists "audio files".
- "Error: cannot open reading.md", followed by the reason. The document was not found or could not be read. Check the name and folder. Put quotation marks around a name with spaces.
- "Error: ffmpeg failed:" followed by ffmpeg's last message. ffmpeg itself could not convert the audio. Check that the output folder exists and that you can write to it, and that there is enough free disk space. Exporting to `.wav` shows whether the problem is in ffmpeg.
- "Error: The voice failed on sentence 12:" followed by the engine's message. The engine stopped on that sentence. Try again, try another voice, or look at the sentence for unusual characters.
- "Error: There is nothing to read aloud." The document has no text to read, for example a scanned PDF with only pictures of pages. Try `tw text` on the document to see the text textweaver finds.
- "Error: The audio is too long for one WAV file; export a smaller part." A WAV file can hold at most 4 gigabytes, many hours of speech. Split the document into parts, for example one file per chapter, and export each one.
- A message about the settings file, printed before the export starts. The export still runs. "Settings file was unreadable and has been reset to defaults" means the whole file was damaged: textweaver used the default settings, and the message says where it saved a backup copy. "Some settings were invalid and use their defaults" names the settings that were wrong; the rest of your settings were used. See [the settings guide](settings.md).

Other problems:

- The chapters are missing in your player. WAV files have no chapters, and many simple players ignore MP3 chapters. Export to `.m4b` and use an audiobook player.
- The captions are a little behind the voice at the start of a heading. That is the spoken announcement, such as "heading level 1", which has no caption. Set `[speech] verbosity` to `low` if you do not want headings announced in the audio.
- Word cues do not match the words exactly. Your engine does not report word times, so they are estimated. See [Which engines time each word exactly](#which-engines-time-each-word-exactly).
- The voice is not the one you use in the reader. Your `[speech] voice` is used only with the engine in `[speech] backend`. Name the voice with `--voice`.

## See also

- [The speech guide](speech.md): choosing a speech engine and voice, and setting rate, pitch, and volume.
- [The conversion guide](converting.md): turning documents into HTML, EPUB, Word, braille, and PDF.
- [The settings guide](settings.md): where settings live, and how to export, import, and reset them.
- [ADR-0011: Audio export](adr/0011-audio-export.md): the design decision behind this feature.
- [Documentation index](README.md)
