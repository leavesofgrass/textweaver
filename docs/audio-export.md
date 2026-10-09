# Exporting audio and subtitles

Export audio, in the reader's File menu, and `tw export-audio`, from a terminal, read a document aloud into an audio file, so you can listen to it later on a phone, a music player, or a book player. It can write a WAV file, a FLAC file, an MP3 file, an Opus file, an Ogg Vorbis file, or an M4B audiobook, each with one chapter for each heading, and an MP4 video that shows the text with the spoken word marked. It can also write subtitles: a caption file that shows each sentence, or each word, at the moment it is spoken. This is for anyone who wants to take a reading with them, for example a student who wants an audiobook of this week's chapters, or a teacher who wants captions that follow the spoken text.

This guide is written to be read with a screen reader. Each task section starts with the command, then explains it. [Export audio from the reader](#export-audio-from-the-reader) comes first; the rest of the guide is about `tw export-audio`, whose settings the reader shares.

## Export audio from the reader

In the reader, open the File menu (F10 in the terminal) and choose Export audio, or find "Export audio" in the command palette. A document must be open. Export audio is in the terminal reader, the window, and `tw` in every release; only a lean build of the reader, made with `--no-default-features`, leaves it out ([Building](dev/building.md)).

1. **The format.** You hear, for example, "Export essay as audio: choose a format, 6 choices." FLAC comes first: lossless and about half the size of WAV. MP3 follows: small, and every player opens it. Then Opus: the smallest, made for speech. Then Ogg Vorbis: small and open, and most players open it. Then WAV. M4B and "Video with captions: MP4, needs ffmpeg" are listed only when ffmpeg is installed; when it is not, you hear "M4B, MP4 need ffmpeg, which was not found." (see [Make a video with captions](#make-a-video-with-captions)). Last comes "Read-along page: text and audio, one file", which writes `essay.html` (see [Make a read-along page](#make-a-read-along-page)). Press Enter on a format.
2. **Where.** "Where should the audio go?" The first choice puts the file beside the document, with the document's name, such as `essay.flac`. The second opens the file browser to choose another folder: press Ctrl+Enter on the folder, or Enter on its "Choose this folder" row (see [Choosing a folder](reading.md)).
3. **The question.** "Export essay.flac with Microsoft David at 200 words per minute, into D:\Notes? y or n". It names the voice and the speed the export uses: your current voice and rate. Press y to start, or n to cancel.

The export runs in the background, so you can keep reading. You hear "Exporting audio, 30 percent." in tens, at most every ten seconds; the interface announcements setting can quiet these. Nothing plays through your speakers.

To stop, press Escape with no list or question open. You hear "Stop the export? No file is kept. y or n". Press y, and the export stops after the sentence being read; no file is left behind.

At the end you hear, for example, "Wrote essay.flac: 42 minutes and 5 seconds, 12 chapters. Open it? y or n" Press y to open the file in your default player. If another list or question is open then, you hear the result without the question.

Which engine it uses: your reading engine, when it can write audio files; otherwise the one in your `[speech] backend` setting, when it can; otherwise the best installed engine that can. Engines that can only speak aloud, such as Omnivox, are never used. The export reads the document as it was last opened, so in edit mode save and leave edit mode first to export your changes.

## Before you start

You need two things: a voice that can write audio files, and, for M4B audiobooks and MP4 videos only, the free program ffmpeg. WAV, FLAC, MP3, Opus, and Ogg Vorbis need nothing else.

### Check that you have a voice that can write files

```bash
tw backends
```

This lists the speech engines textweaver knows about. An engine that can write files has "audio files" in its list of what it supports. For example:

```text
sapi: Windows SAPI5 voices. Available. Priority 500. Supports word highlighting, pause, pitch, volume, audio files, and tones. Word timing from the engine.
```

These engines can write files: ETI-Eloquence (`eci`), Windows SAPI5 voices (`sapi`), DECtalk (`dectalk`), eSpeak NG (`espeak`), and the two Apple engines on macOS (`nsspeech` and `avspeech`). Omnivox (`omnivox`) and Speech Dispatcher (`speechd`) can only speak aloud, so they cannot export. The silent engine (`null`) cannot export either.

### WAV, FLAC, MP3, Opus, and Ogg Vorbis always work

WAV, FLAC, MP3, Opus, and Ogg Vorbis files are written by textweaver itself. You do not need anything else.

- WAV files are large, but every player can open them.
- FLAC files are about half the size of WAV, with exactly the same sound (FLAC is lossless). Most music and audiobook players on phones and computers can open them.
- MP3 files are the smallest, and every player can open them. textweaver encodes them with the LAME encoder, built in, at variable bit rate, quality 5, which suits a speaking voice. LAME is free software under the GNU LGPL; `THIRD-PARTY-NOTICES.md` says what that means for you.
- Opus files are the smallest of all: about 14 MB for an hour of speech, a quarter of the MP3. textweaver encodes them with libopus, the reference Opus encoder, built in: one channel at 32 kilobits per second, as an Ogg Opus file (`.opus`). Most phones, browsers, and VLC play them; some older car stereos and music players do not. libopus is free software under a BSD license.
- Ogg Vorbis files (`.ogg`) are small, and use an open format that most players, browsers, and VLC open, including some that do not play Opus. textweaver encodes them with libvorbis, built in, at quality 3 (the usual default of the `oggenc` tool), keeping the voice's own sample rate and channels. libvorbis and libogg are free software under a BSD license.

### M4B needs ffmpeg

To write `.m4b` or `.mp4`, textweaver first writes a WAV and then asks ffmpeg to convert it. textweaver never bundles ffmpeg, so you install it yourself once, or get it from your own components source (see [Optional components](components.md#your-own-components-source)).

textweaver looks for ffmpeg in three places, in this order:

1. textweaver's components folder, `components` in the data folder, where a copy from a components source goes.
2. The `TEXTWEAVER_FFMPEG` environment variable, if it is set. It must be the full path to the ffmpeg program.
3. Otherwise, a program named `ffmpeg` in one of the folders on your `PATH`, which is where installers put it.

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

When `TEXTWEAVER_FFMPEG` is set (and there is no ffmpeg in the components folder), textweaver uses only that path and does not search `PATH`. If the path is wrong, textweaver says ffmpeg was not found, even when ffmpeg is on your `PATH`.

## Export a document to a WAV file

```bash
tw export-audio reading.md --out reading.wav
```

This reads `reading.md` aloud, silently, into `reading.wav`. Nothing plays through your speakers. You can export any document textweaver can open, not only Markdown.

The document is read exactly as textweaver would read it to you: with your rate, pitch, volume, and normalization settings, and with the same announcements, such as "heading level 1". Each sentence is written one after another into one file.

The output format comes from the file name you give with `--out`:

- `.wav`: a WAV file. Always works.
- `.flac`: a FLAC file. Always works; textweaver writes it itself.
- `.mp3`: an MP3 file. Always works; textweaver encodes it itself with the LAME encoder, at variable bit rate, quality 5.
- `.opus`: an Ogg Opus file. Always works; textweaver encodes it itself with libopus, in one channel at 32 kilobits per second, at 48 kHz (the voice's audio is converted to that rate).
- `.ogg`: an Ogg Vorbis file. Always works; textweaver encodes it itself with libvorbis, at quality 3, at the voice's own sample rate.
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

While ffmpeg works, and while a FLAC, MP3, Opus, or Ogg Vorbis file is encoded, textweaver keeps the full WAV (and, for ffmpeg, a small metadata file) in a hidden folder next to the output, whose name starts with `.textweaver-export-`. The folder is removed when the export finishes. You need enough free disk space for the full WAV, which is much larger than the finished M4B.

### How chapters are chosen

- Every heading, at every level from 1 to 6, starts a chapter. The chapter's name is the heading's text (up to 200 characters).
- Every section break also starts a chapter: in an EPUB, each file of the book; in a Word document, each section. When a section starts with a heading, the two make one chapter, named by the heading. A section without a heading is named by its label, or "Chapter 1", "Chapter 2", and so on when it has none.
- Text before the first heading becomes its own first chapter, named after the document's title, or "Audiobook" when the document has no title.
- A document with no headings at all is one chapter.
- A heading with no audio of its own, for example two headings read in one piece, is left out.
- Each chapter starts at the moment its heading is reached in the audio, measured, not estimated.

There is no option yet to choose which heading levels make chapters.

### Chapters in FLAC, MP3, Opus, Ogg Vorbis, and WAV files

```bash
tw export-audio "Chapter 3.docx" --out "Chapter 3.flac"
```

Every format carries the same title, author, and chapters as the M4B, each in its own kind of tag:

- FLAC files: as Vorbis comments. The title is `TITLE` and `ALBUM`, the author is `ARTIST`, and each chapter is a pair, such as `CHAPTER001=00:01:30.250` for its start and `CHAPTER001NAME=Light` for its name. Audiobook players that read FLAC chapters show them.
- Opus and Ogg Vorbis files: as the same Vorbis comments as FLAC, in the file's comment header.
- MP3 files: as ID3 chapter tags (CHAP and CTOC), with the title, artist, album, and genre.
- WAV files: as ID3 chapter tags too, in an extra part of the file that players without ID3 support skip.

Players that read these chapters show them. Many simple players ignore them and play the file straight through. The JSON report lists each chapter's name, start, and end in every case (see [Get a report as JSON](#get-a-report-as-json)).

## Make a read-along page

```
tw export-audio essay.md --out essay.html
```

An `.html` file name writes a read-along page instead of an audio file: one web page that holds the document as real text, its audio inside as MP3, and a mark on the sentence and the word being read as it plays. Open it in any web browser; it needs no internet connection and no other file, so you can send it, post it to a course site, or open it on a phone. In the reader, choose "Read-along page: text and audio, one file", the last format in Export audio.

The text stays text, so a screen reader, a Braille display, zoom, reflow, and your own fonts all work on it. The page uses your reading theme (`[display] theme`); with the default theme it follows the system's light or dark setting.

- **The audio control.** The browser's own player, at the top of the page.
- **The buttons.** Play (Pause while playing), Back a sentence, Forward a sentence, and Follow along. Tab reaches each one; Enter or Space presses it.
- **Follow along.** On by default: the page scrolls to keep the sentence being read in view. Press it to turn scrolling off; a screen reader says "pressed" or "not pressed". With reduced motion set in the system, the page jumps instead of scrolling smoothly.
- **Speed.** A list from 0.75 to 2 times.
- **Contents.** For a document with chapters, a "Play section" button for each, such as "Play section: Photosynthesis".
- **Click a sentence** to start reading there.
- **What you see.** The sentence being read is underlined and tinted; the word being read is bold, underlined, and outlined in the theme's spoken-word colors. In a forced-colors (high contrast) mode the word uses the system highlight colors. Color alone never carries the mark.
- **With a screen reader.** The page never moves your focus and never speaks on its own, so your screen reader reads where you are while the audio plays.
- **Without scripts.** The text and the audio control still work; only the buttons and the marks are missing.

The page is about a third larger than the MP3 would be on its own. The subtitle and chapters settings work as for audio files.

## Make a video with captions

```
tw export-audio essay.md --out essay.mp4
```

An `.mp4` file name writes a karaoke video: the document read aloud, with the sentence being read on screen and the word being read in bold and underlined. Students asked for it, to play a reading on a class screen, post it to a course site, or watch it on a phone. In the reader, choose "Video with captions: MP4, needs ffmpeg" in Export audio. It needs ffmpeg, like M4B (see [M4B needs ffmpeg](#m4b-needs-ffmpeg)): without ffmpeg the format is not listed, and you hear "M4B, MP4 need ffmpeg, which was not found."

- **What you see.** A 1280 by 720 picture in your reading theme's page and text colors (`[display] theme`), in the bundled Atkinson Hyperlegible Next font. The whole sentence is shown, wrapped to fit; a very long sentence shows the lines around the word being read. The spoken word is bold and underlined, so the mark is a shape, never a color.
- **The captions.** The video carries the WebVTT captions as a soft subtitle track, which players can turn on and off and screen readers in some players can read. They are the same caption lines as `--subtitles essay.vtt`.
- **The chapters.** One chapter for each heading, as in an M4B, so players with a chapter list can jump.
- **The sound.** AAC at 96 kilobits per second.
- **The picture.** H.264, with ffmpeg's libx264 encoder when it has one, otherwise the system's own H.264 encoder that ffmpeg offers (Media Foundation on Windows, VideoToolbox on macOS), otherwise OpenH264, and as a last resort MPEG-4. The picture changes only when the spoken word changes, ten times a second at most, so the file stays small: a little more than the M4B would be.
- **How long it takes.** textweaver reads the whole document first, as for any audio file, then draws the frames and has ffmpeg encode them. The encode takes a few minutes for an hour of speech, with no progress messages of its own. A stop during the encode is honored: ffmpeg is stopped and no file is left behind.

`--subtitles` and `--chapters` still write their own files beside the video.

## Add subtitles

```bash
tw export-audio reading.md --out reading.wav --subtitles reading.srt
```

This writes the audio and a subtitle file next to it. The format comes from the extension you give with `--subtitles`:

- `.srt`: SubRip, the most widely supported format.
- `.vtt`: WebVTT, the format web pages use. The file starts with the line `WEBVTT`.
- `.ass`: Advanced SubStation Alpha, with karaoke by outline (see [Karaoke subtitles](#karaoke-subtitles)).

Any other extension is refused with a message, before any audio is made.

Subtitles work with every audio format, so you can combine `--subtitles` with `.wav`, `.flac`, `.mp3`, or `.m4b`. Give the subtitle file the same name as the audio file, with its own extension, and most players load it by themselves.

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

`--word-level` gives each word its own cue, instead of whole caption lines. This suits tools that highlight one word at a time. This is the start of the real WebVTT file from the example document, without its `NOTE` block (see [The note in each subtitle file](#the-note-in-each-subtitle-file)):

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
- Apple's AVSpeechSynthesizer (`avspeech`) on macOS 14 and later, which reports where each word starts in the audio it writes.

The classic Apple engine (`nsspeech`) and the SAPI Eloquence voices can write files but do not report word times. With them, word cues are estimated. Each sentence's start and end are still measured. The time between them is shared among its words by length, so longer words get more time. Sentence captions from these engines are just as accurate as from any other engine, because they depend only on the measured sentence times.

## Karaoke subtitles

Karaoke subtitles show the whole caption line and mark the word being read. The mark is always a shape: an underline, bold and underline, or an outline. It never depends on color alone. There are three ways to get them.

**Underline as spoken** (`--karaoke tags`, WebVTT only):

```bash
tw export-audio essay.md --out essay.mp3 --subtitles essay.vtt --karaoke tags
```

Each word after the first in a line gets a timestamp tag, and a `STYLE` block underlines the words already spoken. Players that do not understand the tags show the plain line. SRT has no tags, so an SRT file stays plain. An illustration, for a document titled Photosynthesis with no heading, read at an even 250 milliseconds a word:

```text
WEBVTT

STYLE
::cue(:past) {
  text-decoration: underline;
}

NOTE
Photosynthesis. Read by Recording (test double), 240 words a
minute. Made by textweaver.

00:00:00.000 --> 00:00:01.250
Plants <00:00:00.250>make <00:00:00.500>food <00:00:00.750>from <00:00:01.000>light.
```

**One cue per word** (`--karaoke lines`, WebVTT or SRT): every word gets its own cue showing the whole line, with that word in bold and underline. This works in the most players, YouTube included. A player that reads or brailles each cue repeats the line once per word, so it is never the default.

```text
00:00:00.000 --> 00:00:00.250
<b><u>Plants</u></b> make food from light.

00:00:00.250 --> 00:00:00.500
Plants <b><u>make</u></b> food from light.
```

**ASS karaoke** (`--subtitles essay.ass`): an Advanced SubStation Alpha file for players built on libass, such as mpv and VLC. Unread words are white with no outline; each word gains a 3 pixel black outline and turns yellow as it is reached. The `\ko` tag carries each word's time in hundredths of a second.

`--word-level` wins over `--karaoke`: one word per cue, with no line around it.

### The note in each subtitle file

Every WebVTT file textweaver writes now starts with a `NOTE` block that players do not show: the title, the voice and engine, the rate in words a minute, and "Made by textweaver." An ASS file has the same line as a comment and the title as its `Title`. The language belongs to the player's track settings, not the file, so it is not written.

## Write a chapters file

```bash
tw export-audio essay.md --out essay.mp3 --subtitles essay.vtt --chapters essay.chapters.vtt
```

This also writes a WebVTT chapters file: one cue per chapter, with the chapter's title as its text. Web players use it for a chapter menu. The chapters are the same ones written into an M4B. A document with no headings has one chapter named after its title. When it has no title either, the chapter is named in the interface language ("Audiobook", "Hörbuch", "Audiolibro" and so on), and untitled chapters are numbered the same way ("Chapter 2", "Kapitel 2").

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
subtitle_karaoke = "off"
subtitle_chapters = false
```

The settings:

- `subtitles_with_audio`: `true` or `false`. The default is `false`. When `true`, every export also writes subtitles next to the audio, with the same name and the subtitle format's extension. Exporting `book.mp3` then also writes `book.srt`.
- `subtitle_format`: `"srt"`, `"vtt"`, or `"ass"`. The default is `"srt"`. It is used only for the file `subtitles_with_audio` names. A file you name with `--subtitles` always uses its own extension.
- `subtitle_word_level`: `true` or `false`. The default is `false`. When `true`, subtitles always have one cue per word, as if you had added `--word-level`.
- `subtitle_karaoke`: `"off"`, `"tags"`, or `"lines"`. The default is `"off"`. It is the karaoke style when you do not give `--karaoke`.
- `subtitle_chapters`: `true` or `false`. The default is `false`. When `true`, every export also writes a chapters file beside the subtitles (or beside the audio when there are none), named like it with `.chapters.vtt`: `book.vtt` gives `book.chapters.vtt`.

A file named with `--subtitles` always wins over `subtitles_with_audio`. `--word-level` turns word cues on even when `subtitle_word_level` is `false`; there is no option to turn them off for one export when the setting is `true`.

These settings are used by `tw export-audio` and by Export audio in the reader, which writes subtitles beside the audio when `subtitles_with_audio` is `true`, with the karaoke style of `subtitle_karaoke`, a chapters file when `subtitle_chapters` is `true`, and a note naming the voice and the rate.

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
- `format`: `wav`, `flac`, `mp3`, or `m4b`.
- `subtitles`: the subtitle file, or `null`.
- `ffmpeg`: the ffmpeg program used for M4B, or `null`.
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

- "Error: writing M4B needs ffmpeg, which was not found; install ffmpeg, set TEXTWEAVER_FFMPEG to its path, or export to .flac, .mp3, .opus, .ogg, or .wav". This is said before anything is read aloud. ffmpeg is not installed, or textweaver cannot find it. Install it as described in [Before you start](#before-you-start). If you just installed it, open a new terminal window. If `TEXTWEAVER_FFMPEG` is set, check that it is the full path to the program, including `ffmpeg.exe` on Windows, or remove the variable. Or export to `.flac`, `.mp3`, `.opus`, `.ogg`, or `.wav`, which need nothing.
- "Error: cannot write notes.aac: use a .wav, .flac, .mp3, .opus, .ogg, or .m4b file name". Only WAV, FLAC, MP3, Opus, Ogg Vorbis, and M4B can be written. For a small file of speech, use `.opus` or `.ogg`.
- "Error: The voice failed on sentence 1: engine error: the voice could not be used:" followed by the reason. The voice you chose could not be loaded, so nothing was written with another voice by mistake. Run `tw voices --backend sapi` and choose a voice from the list.
- "Error: Cannot write subtitles to notes.txt: use a .srt, .vtt, or .ass file name." Give the subtitle file a `.srt`, `.vtt`, or `.ass` extension.
- "Error: no installed voice can write audio files; install espeak-ng, or choose one with --backend". textweaver found no engine that can write files. Run `tw backends` and look for "audio files". On Windows, the SAPI5 voices usually can. On Linux, install eSpeak NG; see [the speech guide](speech.md).
- "Error: Silent (no audio) cannot write audio files; choose another voice with --backend". The engine chosen cannot write files. The name at the start is the engine's, for example "Omnivox speech server". This also happens when you name an engine with `--backend` that is not installed and the automatic choice falls on an engine that cannot write files. Run `tw backends`, and name an engine that is available and lists "audio files".
- "Error: cannot open reading.md", followed by the reason. The document was not found or could not be read. Check the name and folder. Put quotation marks around a name with spaces.
- "Error: ffmpeg failed:" followed by ffmpeg's last message. ffmpeg itself could not convert the audio. Check that the output folder exists and that you can write to it, and that there is enough free disk space. Exporting to `.wav` shows whether the problem is in ffmpeg.
- "Error: The voice failed on sentence 12:" followed by the engine's message. The engine stopped on that sentence. Try again, try another voice, or look at the sentence for unusual characters.
- "Error: There is nothing to read aloud." The document has no text to read, for example a scanned PDF with only pictures of pages. Try `tw text` on the document to see the text textweaver finds.
- "Error: The audio is too long for one WAV file; export a smaller part." A WAV file can hold at most 4 gigabytes, many hours of speech. Split the document into parts, for example one file per chapter, and export each one.
- A message about the settings file, printed before the export starts. The export still runs. "Settings file was unreadable and has been reset to defaults" means the whole file was damaged: textweaver used the default settings, and the message says where it saved a backup copy. "Some settings were invalid and use their defaults" names the settings that were wrong; the rest of your settings were used. See [the settings guide](settings.md).

Other problems:

- The chapters are missing in your player. Many simple players ignore the chapters in FLAC, MP3, Opus, Ogg Vorbis, and WAV files. Export to `.m4b` and use an audiobook player.
- The captions are a little behind the voice at the start of a heading. That is the spoken announcement, such as "heading level 1", which has no caption. Set `[speech] verbosity` to `low` if you do not want headings announced in the audio.
- Word cues do not match the words exactly. Your engine does not report word times, so they are estimated. See [Which engines time each word exactly](#which-engines-time-each-word-exactly).
- The voice is not the one you use in the reader. Your `[speech] voice` is used only with the engine in `[speech] backend`. Name the voice with `--voice`.

## See also

- [The speech guide](speech.md): choosing a speech engine and voice, and setting rate, pitch, and volume.
- [The conversion guide](converting.md): turning documents into HTML, EPUB, Word, braille, and PDF.
- [The settings guide](settings.md): where settings live, and how to export, import, and reset them.
- [ADR-0011: Audio export](adr/0011-audio-export.md): the design decision behind this feature.
- [Documentation index](README.md)
