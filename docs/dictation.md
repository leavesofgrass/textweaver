# Dictation: turning speech into text

`tw dictate` converts speech to text. Given an audio file, such as a recorded lecture, a voice memo, or a note spoken into a phone, it prints what was said. It can also interpret spoken commands such as "new line" and "period", so that a letter or paragraph can be dictated with its punctuation. The feature serves people who find speaking easier than typing, and anyone who wants a written record of a recording. Recognition is performed by Whisper, a speech recognition system that you install once, and everything happens on your own computer. With the in-process Whisper model installed, `tw dictate` can also record from the microphone; see [Whisper inside textweaver](#whisper-inside-textweaver).

Each section below begins with the command and then explains it.

## Before you start

Dictation depends on up to three things:

- A Whisper program, or the Whisper model that textweaver runs itself. textweaver neither includes nor downloads a Whisper program. It offers to download its own Whisper model the first time you dictate, and does so only with your agreement; see [Whisper inside textweaver](#whisper-inside-textweaver).
- A Whisper model, the trained component that knows the language. Two of the three Whisper programs download their models themselves; whisper.cpp requires you to download a model file.
- ffmpeg, a program that converts audio. whisper.cpp needs it for any file that is not a WAV file, and OpenAI Whisper needs it for every file.

### The Whisper programs textweaver can use

textweaver works with three Whisper programs. When you do not choose one, it uses the first it finds, in this order:

1. The program named by the `TEXTWEAVER_WHISPER` environment variable, if that variable is set. See [Choose which Whisper program runs](#choose-which-whisper-program-runs).
2. whisper.cpp. textweaver looks for a program called `whisper-cli`, and then `whisper-cpp` (the name older builds used).
3. faster-whisper, through its command line program `whisper-ctranslate2`.
4. OpenAI Whisper, through its command line program `whisper`.

textweaver searches the folders on your `PATH` for these programs; on Windows it tries the usual program extensions, such as `.exe`, first.

whisper.cpp comes first because it is a single program without a Python dependency and starts fastest. faster-whisper comes next. OpenAI Whisper is the slowest to start: on the development machine its first run took about two minutes while Python loaded, and later runs took a few seconds.

A whisper.cpp program under another name also works: name it with `--program` or `TEXTWEAVER_WHISPER`, and declare its kind with `--engine cpp` or `TEXTWEAVER_WHISPER_ENGINE`. A program called `main`, the name used by very old whisper.cpp builds, is recognized as whisper.cpp automatically.

### Install a Whisper program

Install any one of these.

whisper.cpp: download a release from the whisper.cpp project on GitHub (github.com/ggml-org/whisper.cpp, under Releases). The Windows download is a zip file; unzip it, and either put the folder that holds `whisper-cli.exe` on your `PATH` or point `TEXTWEAVER_WHISPER` at the program. On a Mac with Homebrew, for example:

```bash
brew install whisper-cpp
```

faster-whisper: install its command line with Python's `pip`, for example:

```text
pip install whisper-ctranslate2
```

OpenAI Whisper: install it with `pip`, for example:

```text
pip install openai-whisper
```

After installing, check that textweaver finds it. See [Check which Whisper program textweaver finds](#check-which-whisper-program-textweaver-finds).

### Models

A model is the trained speech recognizer. Larger models are more accurate but slower. The sizes, smallest first, are:

- `tiny`: the fastest and the least accurate.
- `base`: the default. textweaver uses it when you do not choose.
- `small`.
- `medium`.
- `large-v3`: the most accurate and the slowest.
- `large-v3-turbo`: nearly as accurate as `large-v3`, and much faster.

faster-whisper and OpenAI Whisper download a model the first time you use it and keep it for later runs. That first download requires an internet connection; it is performed by the Whisper program, not by textweaver, and it sends none of your audio.

whisper.cpp does not download models, so you obtain the model file yourself. Its name is `ggml-` followed by the model size and `.bin`: `ggml-base.bin` for `base`, `ggml-large-v3-turbo.bin` for `large-v3-turbo`. The files are on the whisper.cpp project's model page on Hugging Face, and the whisper.cpp source also includes a download script, which can be run from its folder, for example:

```bash
./models/download-ggml-model.sh base
```

### Where textweaver looks for whisper.cpp model files

textweaver searches for the model file in these folders, in this order, and uses the first match:

1. The folder named by the `TEXTWEAVER_WHISPER_MODELS` environment variable, if it is set.
2. The `whisper` folder in textweaver's data folder:
   - Windows: `%APPDATA%\leavesofgrass\textweaver\data\whisper`
   - macOS: `~/Library/Application Support/org.leavesofgrass.textweaver/whisper`
   - Linux: `~/.local/share/textweaver/whisper` (or `textweaver/whisper` in `$XDG_DATA_HOME` when that is set)
   - When `TEXTWEAVER_HOME` is set: `data/whisper` inside that folder.
3. A `models` folder next to the whisper.cpp program.
4. A `models` folder one level up from the program's folder. This is where whisper.cpp's own download script puts models.

The simplest arrangement is to create the `whisper` folder in textweaver's data folder and place the model file there. To use a model file stored elsewhere, name it with `--model-file`; see [Use a whisper.cpp model file from anywhere](#use-a-whispercpp-model-file-from-anywhere).

### ffmpeg

whisper.cpp reads only WAV files. When given another format, such as MP3 or M4A, textweaver first converts it with ffmpeg to a 16 kHz, mono, 16-bit WAV file in a temporary folder, which requires ffmpeg on your `PATH`. A file whose name ends in `.wav` is passed to whisper.cpp unchanged.

OpenAI Whisper uses ffmpeg to read every audio file, WAV files included, and its own code requires the ffmpeg program on the `PATH`.

faster-whisper reads audio with its own library, so it does not need ffmpeg.

To install ffmpeg on Windows, for example:

```powershell
winget install --id Gyan.FFmpeg
```

On a Mac with Homebrew, for example:

```bash
brew install ffmpeg
```

On Debian or Ubuntu Linux, for example:

```bash
sudo apt install ffmpeg
```

### The environment variables

Three environment variables change how textweaver finds Whisper. None is needed when your Whisper program is on your `PATH` and its models are in one of the usual folders.

- `TEXTWEAVER_WHISPER`: the full path of a Whisper program to use. It is tried before anything on the `PATH`, but only if the file exists and textweaver can tell which Whisper program it is; otherwise it is skipped without a message, so verify with `tw dictate list`.
- `TEXTWEAVER_WHISPER_ENGINE`: the kind of program `TEXTWEAVER_WHISPER` names, when its file name does not reveal it: `cpp`, `faster`, or `openai`. The longer names `whisper.cpp`, `faster-whisper`, and `openai-whisper` work too. It applies only to the program in `TEXTWEAVER_WHISPER`.
- `TEXTWEAVER_WHISPER_MODELS`: a folder of whisper.cpp model files. It is the first folder searched. The other programs ignore it.

To set one for the current PowerShell session, for example:

```powershell
$env:TEXTWEAVER_WHISPER = "C:\Tools\whisper\whisper-cli.exe"
```

In a macOS or Linux terminal, for example:

```bash
export TEXTWEAVER_WHISPER=/opt/whisper/whisper-cli
```

When the file name does not say which program it is, set the engine as well, for example:

```powershell
$env:TEXTWEAVER_WHISPER_ENGINE = "cpp"
```

## Check which Whisper program textweaver finds

```text
tw dictate list
```

This lists every Whisper program textweaver found, in the order it would use them, followed by the model sizes. It records and transcribes nothing. On a Windows computer with only OpenAI Whisper installed, the output is two lines like these (the folder depends on where Python put the program):

```text
OpenAI Whisper: C:\Users\you\AppData\Local\Programs\Python\Python313\Scripts\whisper.exe
Models: tiny, base, small, medium, large-v3, large-v3-turbo
```

Each program is named as "whisper.cpp", "faster-whisper", or "OpenAI Whisper", followed by its location. The first one listed is the one `tw dictate` uses.

When nothing is found, you see:

```text
No Whisper program found. Install whisper.cpp (whisper-cli), faster-whisper (whisper-ctranslate2), or OpenAI Whisper (whisper).
Models: tiny, base, small, medium, large-v3, large-v3-turbo
```

For scripts, add `--json`:

```text
tw dictate list --json
```

This prints a list with one entry per program, each holding `engine` (`cpp`, `faster`, or `openai`) and `program` (the path). When nothing is found the list is empty: `[]`.

The listing checks programs only. It does not check whether whisper.cpp's model file is present; a transcription does that before it starts.

## Transcribe an audio file

```text
tw dictate --file lecture.mp3
```

This transcribes `lecture.mp3` with the first Whisper program found and the `base` model. Any audio format your Whisper program can read is accepted; with whisper.cpp, files other than WAV need ffmpeg (see [ffmpeg](#ffmpeg)).

While it works, the message "Transcribing, this may take a while" appears. Each piece of speech is then printed as Whisper finishes it. A piece is usually one sentence or a few, so the text does not arrive word by word. When transcription is complete, the word count is reported, for example "Transcribed 19 words", followed by the whole transcript as one paragraph.

Progress messages go to the error stream and the transcript to standard output, so a terminal shows both. To keep only the transcript, write it to a file with `--out` or redirect standard output.

Long recordings and larger models take longer. On the development machine, OpenAI Whisper with the `tiny` model took about 15 seconds for 12 seconds of speech.

## Save the transcript to a file

```text
tw dictate --file lecture.mp3 --out lecture.txt
```

This writes the transcript to `lecture.txt` instead of printing it; the progress messages and the word count are still shown. If `lecture.txt` already exists, it is replaced without asking.

With `--json`, the file holds the JSON result instead of plain text.

## Choose a model

```text
tw dictate --file lecture.mp3 --model small
```

`--model` selects the model size: `tiny`, `base`, `small`, `medium`, `large-v3`, or `large-v3-turbo`, in either case. Without it, the model is `base`. See [Models](#models) for how they differ.

`tiny` gives a quick first look at a long recording, while `small` or larger models help when names or technical vocabulary come out wrong.

With whisper.cpp, the model file for that size must be in one of the model folders; otherwise a message names the file and the folders searched.

## Choose the language

```text
tw dictate --file vorlesung.mp3 --lang de
```

`--lang` tells Whisper which language is spoken, as a short code: `en` for English, `de` for German, `fr` for French, `es` for Spanish, and so on. Without it, Whisper infers the language from the start of the recording. Stating the language is faster and avoids a wrong inference, especially for short recordings.

## Add times to each line

```text
tw dictate --file lecture.mp3 --timestamps
```

With `--timestamps`, each piece of speech begins on its own line with its start time in the recording, in minutes and seconds within square brackets. For example:

```text
[00:00] Today we look at how cells divide.
[00:08] There are two kinds of division.
```

From one hour on, the time includes hours, for example `[01:02:03]`. This is the layout star used, and it helps locate a place in a long recording.

Do not combine `--timestamps` with `--commands` for now: the spoken commands join all the lines into one, so the times end up in the middle of the text.

## Make captions of a recording

```bash
tw dictate --file lecture.wav --captions lecture.vtt
```

With `--captions`, textweaver also writes the transcript as a caption file, in the format given by the extension: `.srt`, `.vtt`, or `.ass`. Each piece of speech Whisper finds becomes a caption, and a long one is split into lines of at most 12 words and 90 characters, as in [audio export](audio-export.md). On completion you hear the length of the recording and the number of captions written, followed by "Machine captions: check before sharing."

The same warning is written into the file, in its `NOTE` block (WebVTT) or its comment line (ASS). Whisper makes mistakes and published captions should be accurate, so review and correct them before sharing.

Add `--karaoke lines` for one caption per word with that word in bold and underline, or `--karaoke tags` for WebVTT timestamp tags that underline words as they are spoken (see [Karaoke subtitles](audio-export.md#karaoke-subtitles)). Whisper supplies times for each piece of speech, not for each word, so word times are distributed by word length, and the note says they are estimated.

## Use spoken commands

```text
tw dictate --file letter.wav --commands
```

With `--commands`, the words you speak for punctuation and line breaks become the marks themselves. If you say "Dear Sam comma new line thanks for the book period", for example, the transcript is:

```text
Dear Sam,
Thanks for the book.
```

Spoken commands are off unless requested, because the same words are often meant literally ("the period of history").

The complete set of spoken commands follows. Case, and any punctuation Whisper attaches to the command word, do not matter.

- "new paragraph" or "next paragraph": starts a new paragraph (two line breaks, which leave a blank line). The next word starts with a capital letter.
- "new line", "next line", or "newline": starts a new line (one line break). The next word starts with a capital letter.
- "period", "full stop", or "fullstop": a period (`.`). The next word starts with a capital letter.
- "question mark": a question mark (`?`). The next word starts with a capital letter.
- "exclamation mark" or "exclamation point": an exclamation mark (`!`). The next word starts with a capital letter.
- "comma": a comma (`,`).
- "semicolon" or "semi colon": a semicolon (`;`).
- "colon": a colon (`:`).
- "ellipsis": three periods (`...`).
- "open quote" or "begin quote": an opening double quotation mark (`"`), joined to the word after it.
- "close quote", "end quote", or "unquote": a closing double quotation mark (`"`), joined to the word before it.
- "open parenthesis", "open paren", or "left paren": an opening parenthesis (`(`), joined to the word after it.
- "close parenthesis", "close paren", or "right paren": a closing parenthesis (`)`), joined to the word before it.
- "hyphen": a hyphen (`-`) with no spaces, joining the words on either side. "state hyphen of hyphen the hyphen art" gives "state-of-the-art".
- "dash": a long dash (an em dash, `—`) with a space on each side.

Whisper adds punctuation of its own, and it often renders a command word as a miniature sentence, such as "Period." textweaver tidies this: punctuation Whisper attaches to a command word is dropped, and punctuation Whisper places just before a punctuation or line command is replaced by the command, so "world, period" yields "world." and not "world,.".

To write a command word as an ordinary word, say "literal" before it: "literal period" writes the word "period", and "the literal period of history" gives "the period of history". "literal" protects only the single word that follows it, which is usually enough for a two-word command, since "literal new line" writes "new line" ("line" alone is not a command). The exception is "semi colon": "literal semi colon" still turns "colon" into a colon, so say "literal semicolon" as one word instead. To write the word "literal" itself, say "literal literal".

Commands are applied to the finished transcript. The pieces printed while Whisper works show the words as Whisper heard them, command words included, and the word count ("Transcribed 19 words") likewise counts the words as heard.

## Choose which Whisper program runs

```text
tw dictate --file lecture.mp3 --engine faster
```

`--engine` selects the kind of Whisper program: `cpp` for whisper.cpp, `faster` for faster-whisper, or `openai` for OpenAI Whisper. textweaver uses the first program of that kind that it finds, in the order described in [The Whisper programs textweaver can use](#the-whisper-programs-textweaver-can-use); without `--engine` it uses the first program found of any kind. The longer names `whisper.cpp`, `faster-whisper`, `whisper-ctranslate2`, `openai-whisper`, and `whisper` work too.

To run one particular program, give its path with `--program`:

```text
tw dictate --file lecture.mp3 --program "C:\Tools\whisper\whisper-cli.exe"
```

textweaver then runs that program without searching your `PATH`, and infers the kind of program from its file name: a name containing "ctranslate2" or "faster" is faster-whisper; a name containing "whisper-cli" or "whisper-cpp", or the name "main", is whisper.cpp; and any other name containing "whisper" is OpenAI Whisper. When the name is not informative, add `--engine`:

```text
tw dictate --file lecture.mp3 --program "C:\Tools\stt\recognizer.exe" --engine cpp
```

## Use a whisper.cpp model file from anywhere

```text
tw dictate --file lecture.wav --model-file "D:\models\ggml-base.en.bin"
```

`--model-file` names whisper.cpp's model file directly, so the file need not be in one of the model folders. It is also the way to use a model file whose name textweaver does not search for, such as the English-only models (`ggml-base.en.bin`) or smaller compressed ones.

When `--model-file` is given, whisper.cpp uses that file regardless of `--model`. The option applies only to whisper.cpp; faster-whisper and OpenAI Whisper ignore it and use `--model`.

## Get the result as JSON

```text
tw dictate --file lecture.mp3 --json
```

With `--json`, `tw dictate` prints a single JSON object and no progress messages, for use by scripts and other programs. The object contains:

- `engine`: `cpp`, `faster`, or `openai`.
- `program`: the path of the Whisper program that ran.
- `model`: the model size, such as `base`.
- `text`: the transcript, with `--timestamps` and `--commands` applied if you gave them.
- `transcript`: the pieces exactly as Whisper produced them. It holds `segments`, a list in which each piece has `start_ms` and `end_ms` (its start and end in milliseconds from the start of the recording) and `text`.

Add `--out` to write the JSON to a file instead.

## What you hear and see

`tw` itself does not speak; it prints short messages, which your screen reader reads in the terminal. The messages, exactly as the program writes them, are:

- "Transcribing, this may take a while": Whisper has started.
- Each piece of the transcript, as Whisper finishes it.
- "Transcribed 19 words" (with the real number), or "Transcribed 1 word": Whisper is done. The transcript follows.
- "Dictation produced no text": Whisper finished but recognized no words.

When something goes wrong, a single message beginning "Error:" appears and `tw dictate` exits with status 1, so that scripts can detect the failure. The messages are listed in [If something goes wrong](#if-something-goes-wrong).

With `--json`, none of the progress messages are printed; only the JSON is.

## Your audio stays on your computer

textweaver sends no audio anywhere, and the dictation code contains no networking. It runs the Whisper program on your computer and reads the text it writes. When you agree to download the Whisper model, textweaver's optional components perform the download and fetch the model files only.

While it works, textweaver creates a temporary folder for the session inside your system's temporary folder, with a name starting `textweaver-dictation-`. Whisper's output, and the converted WAV file when whisper.cpp needs one, are written there. The folder is deleted when the transcription ends, whether or not it succeeded, and your own audio file is never modified.

The only other use of the internet occurs when faster-whisper or OpenAI Whisper downloads a model you have not used before. The Whisper program performs that download, it fetches the model only, and it sends none of your audio.

## Whisper inside textweaver

textweaver can run Whisper itself, with no Whisper program, on RTen, a model runtime written in Rust ([ADR-0023](adr/0023-in-process-neural-speech.md)). It needs three files from Hugging Face's `onnx-community/whisper-base.en`, in one folder, `<data>/whisper/rten/base.en` (79.3 MB in all):

- `encoder_model_int8.onnx` (23 MB);
- `decoder_model_merged_int8.onnx` (54 MB);
- `tokenizer.json` (2.4 MB).

The model is released by OpenAI under the MIT license. The onnx-community copies declare no license of their own, so textweaver reports "MIT, unconfirmed".

### Getting the model

textweaver offers to download the model and never does so unprompted.

- **In the reader and the app:** the first time you dictate without the model, you hear "Dictation needs the Whisper model, 79.3 MB, license MIT, unconfirmed. Download it now? y or n". Press `y` and the download starts, with progress announced every 10 percent; Escape stops it, and the next download resumes where it stopped. When the download finishes you hear "Ready" and dictation starts. Press `n` and nothing is downloaded; you are not asked again until textweaver next starts, and Dictate says "No model, so no dictation for now."
- **From the Tools menu:** Download the dictation model asks the same question at any time.
- **From the command line:** `tw dictate download` says what it downloads and asks; `tw dictate download --yes` downloads without asking. When `tw dictate` finds no model, it offers the same download (`--yes` answers for it), then goes on dictating.
- **From Manage optional components** (Tools menu, or `tw components`): the model is listed with its size and license, with Download, Verify, Remove, and Install from a file.

Each file is verified against its published size and SHA-256 before it is kept, and nothing that fails the check is used. The model is an optional component; see [Optional components](components.md) for the mirror and for installing from a downloaded zip or folder on a computer without internet.

**Choosing a model.** The Dictation model setting selects the Whisper model that dictation uses: base.en (the default) or small.en (251 MB, slower, more accurate English). The download question and Download the dictation model offer whichever is selected.

**Placing the model by hand.** You may instead download the three files yourself and place them in `<data>/whisper/rten/base.en` (or an `onnx` folder inside it); textweaver adopts a folder that already holds the right files rather than downloading them again. `--model-dir DIR` on the command line, or the Dictation model folder setting, names a different folder (`tw dictate --model-dir D:\Models\whisper-tiny.en`), which is used as it is, without the download offer.

When the folder holds the model, `tw dictate` uses it automatically, and `--engine rten` requests it explicitly.

**When the model cannot be used,** textweaver explains why in words: "Dictation model lacks decoder_model_merged_int8.onnx." (a file is missing), "Dictation model damaged: tokenizer.json." (a file has the wrong size; the download is offered again), "No model folder: D:\Models\whisper" (the folder you named does not exist), or "No model; this version cannot download." (a reader built without downloads; place the files by hand).

### Dictating from the microphone

```text
tw dictate
```

Without `--file`, `tw dictate` records from your default microphone. You hear "Recording. Press Enter to stop."; speak, then press Enter. It announces "Transcribing" and prints the text. Pauses are skipped: a voice detector (earshot) finds where you spoke, so long silences cost nothing and are not turned into invented words.

Without the in-process model, `tw dictate` offers to download it (or, when nobody can answer, says where the model goes and that `tw dictate download` fetches it). You can also record with any program you like and transcribe the file with `tw dictate --file`.

### Seeing the words while you talk

```text
tw dictate --live
```

With `--live`, textweaver transcribes while you speak. Each phrase is completed at your pause rather than when you press Enter, and in a long sentence the first words appear while you are still talking. Each group of words is printed on its own line as soon as it is certain and is never changed afterward, so your screen reader reads each word once. The finished text is still printed at the end, with spoken commands applied if you requested them.

What to expect:

- **Short phrases,** under about three seconds, arrive at their pause, as quickly as without `--live`.
- **A long sentence** shows its first words about four seconds in, then more every second or two, a few seconds behind your voice. The last words come at the pause.
- **On a busy computer,** when Whisper takes longer than a second and a half, textweaver waits for your pause instead, so nothing arrives later than it would without `--live`.
- **A phrase with no words recognized** says "No words recognized in that phrase", so nothing vanishes silently.

`tw dictate --live --file note.wav` feeds a recording in at speaking pace, silently, to show how a live session would behave. `--live` requires the in-process model.

### How long it takes

`--timings` reports where the time went: loading the model (once), the spectrogram, the encoder, the decoder, and the interval from pressing Enter to the text.

```text
tw dictate --file note.wav --timings
```

## Dictating in the reader

In edit mode, the **Dictate** command types what you say at the cursor. It is in the Edit menu and the command palette (**F2**, then "dictate"), and its key is **Ctrl+Shift+F9**, or whatever the menu shows beside it. Press the key again to stop.

1. Open a document and turn on edit mode (**Ctrl+E**). If edit mode is off when you ask to dictate, textweaver first asks "Turn on edit mode and dictate? y or n".
2. Choose Dictate. You hear "Dictating. Speak, then press", the key, and "to stop." Reading aloud stops, so that the microphone does not pick it up.
3. Speak. The status line shows what textweaver is sure of as you talk, beginning "Dictating:", with the newest words last and no more than 40 characters, so a 40-cell Braille display shows the latest words without scrolling. Words are never changed once shown.
4. Pause. At each pause (about half a second of silence) the phrase is typed at the cursor, with spoken commands applied ("new line", "period", "comma", "open quote"; see [Use spoken commands](#use-spoken-commands)). Each phrase is one step for Undo.
5. Choose Dictate again to stop. The last phrase is typed and you hear "Dictation done."

**Pausing.** To answer a colleague, cough, or think aloud without having those words typed, pause the microphone with **Pause dictation**: its key is **Alt+Shift+F9**, and it is in the Edit menu and the command palette (F2, then "pause dictation"). You hear "Dictation paused." The phrase you were speaking is finished and typed as usual, so no words are lost; from then on, whatever the microphone hears is discarded. The session stays open, with the model loaded and your text in place. Press the same key again to resume, and you hear "Dictation resumed." Dictate still stops the session, paused or not. While dictation is paused, the terminal reader's status line begins "Dictation paused." whatever else it shows, and in the textweaver app the button reads "Resume dictation". Pausing when no dictation is running says "Not dictating."

**The buttons in the app.** While a dictation is under way, the textweaver app adds two buttons to the toolbar, before Slower and Faster: **Pause dictation** (named **Resume dictation** while paused) and **Stop dictation**. They are ordinary buttons, reached with Tab in the order they are drawn, and they leave the toolbar when the dictation ends; if focus was on one of them, it moves to the document. With the toolbar hidden from the View menu, they appear on the header instead. In focus mode they fade and return with the rest of the toolbar, and like every other control they stay in the Tab order and the accessibility tree while hidden. See [What is in the app](gui.md#what-is-in-the-app).

**Your voice and textweaver's voice.** By default textweaver does not speak the words while the microphone is open; they appear on the status line as they arrive and are spoken once, at each pause. To hear them as they arrive, for example with headphones, turn on Speak while dictating in the settings (the Dictation section):

```toml
[dictation]
speak_while_recording = true
```

The words are spoken as ordinary messages, so setting [interface announcements](reading.md) to off confines them to the status line and the Braille display. Problems, such as a missing microphone, are always spoken.

**The model.** Dictation in the reader uses Whisper inside textweaver, from the same folder as `tw dictate` ([Whisper inside textweaver](#whisper-inside-textweaver)), or the folder in `model_dir`:

```toml
[dictation]
model_dir = "D:\\Models\\whisper-base.en"
```

The first dictation after starting textweaver loads the model, which takes a few seconds; later ones start immediately.

**Nothing is lost.** Turning edit mode off, opening another document, starting a new one, or quitting during dictation first finishes the dictation: recording stops and the last phrase is typed. If Whisper needs more than ten seconds for that, dictation ends and you hear "Dictation stopped before its last words were typed."

What you may hear:

- "No words recognized in that phrase.": textweaver heard speech but could not make out words. Try again, a little closer to the microphone.
- "Dictation failed:" and the reason, such as "No audio was recorded. Check your microphone."
- "Dictation needs the Whisper model in" and a folder: the model files are not there.

## Dictating into the reader with other software

```toml
[keyboard]
character_keys = false
```

You can dictate into textweaver with speech recognition software that types on your behalf, such as Windows Voice Access, Dragon, or macOS Dictation. Such software enters letters as if you had pressed the keys, and in reading mode many single letters and punctuation keys are shortcuts, so dictated text could trigger commands.

To prevent this, turn single-key shortcuts off by pressing **F9**, in the app or the terminal version. You hear "Single-key shortcuts off." Press **F9** again to restore them, and you hear "Single-key shortcuts on." You can also run `toggle character keys` from the command palette (**F2**) or put the setting above in `settings.toml`; textweaver remembers the choice.

While single-key shortcuts are off, letters, punctuation, and Space never trigger commands in any mode. Shortcuts with Ctrl or Alt, the arrow keys, the function keys, and the command palette still work, and quitting always asks first, so a stray keystroke cannot close your document.

To write text by dictation, switch to edit mode first; see [the editing guide](editing.md).

## If something goes wrong

- "No Whisper program found." from `tw dictate list`, or "Speech recognition needs Whisper, which was not found." from a transcription: textweaver found no Whisper program. Install one (see [Install a Whisper program](#install-a-whisper-program)) and open a new terminal so that it sees the updated `PATH`, or set `TEXTWEAVER_WHISPER` to the program, or give it with `--program`. The second message also appears when `--engine` requests a kind of program that is not installed, even if another kind is.
- `TEXTWEAVER_WHISPER` seems to be ignored: textweaver skips it without a message when the file does not exist or when it cannot tell which kind of program it is. Check the path, set `TEXTWEAVER_WHISPER_ENGINE`, and run `tw dictate list` to see what is found.
- "The Whisper model "base" was not found. Download ggml-base.bin into one of:" followed by a list of folders: whisper.cpp needs its model file. Download the file it names into one of the folders listed, or give the file with `--model-file`. This is checked before anything is transcribed.
- "The Whisper model "base" was not found. Download" followed by a file path and "into one of: the folder given": the file given with `--model-file` does not exist. Check the path.
- "Name the program's engine with --engine cpp, faster, or openai": the file name given with `--program` does not reveal which kind of Whisper program it is. Add `--engine`.
- "Unknown model size" or "Unknown engine": a typing mistake in `--model` or `--engine`. The message lists the valid names.
- A file name followed by "file not found": the audio file given with `--file` does not exist. Check the name, and quote it if it contains spaces. The message is followed by "Caused by: file not found", which repeats the same information.
- "whisper.cpp reads WAV files only, and ffmpeg was not found to convert this one. Install ffmpeg, or convert the file to WAV first.": install ffmpeg (see [ffmpeg](#ffmpeg)), or convert the file yourself.
- "The audio file could not be converted:" followed by ffmpeg's reason: ffmpeg could not read the file, which may be damaged or may not be audio.
- "Could not start" followed by a program and a reason: the Whisper program or ffmpeg could not be run. Check the path given with `--program` or `TEXTWEAVER_WHISPER`.
- "Whisper failed:" followed by a line from Whisper: the Whisper program stopped with an error. The text after the colon is Whisper's own last error line, or "it stopped without saying why". With OpenAI Whisper, a message about ffmpeg most likely means that ffmpeg is missing; install it (see [ffmpeg](#ffmpeg)).
- "Whisper's transcript could not be read:": Whisper finished but its result could not be read. Try again, or try another Whisper program.
- "Dictation produced no text": Whisper heard no speech. Check that the file contains speech and is not silent, and try `--lang` if the speech is not in the language Whisper inferred.
- whisper.cpp gives an error about the WAV file: textweaver passes WAV files to whisper.cpp unchanged, and whisper.cpp expects 16 kHz audio. Convert the file to 16 kHz mono WAV with ffmpeg, for example:

  ```text
  ffmpeg -i lecture.wav -ar 16000 -ac 1 -c:a pcm_s16le lecture-16k.wav
  ```

- The words are often wrong: try a larger `--model`, state the `--lang`, and use a recording with less background noise.
- It is very slow: try a smaller `--model`, such as `tiny` or `base`. whisper.cpp and faster-whisper need no PyTorch, so they may also start faster than OpenAI Whisper.
- Dictated text set off commands in the reader: press **F9** to turn single-key shortcuts off (see [Dictating into the reader with other software](#dictating-into-the-reader-with-other-software)).

## See also

- [ADR-0013: Dictation through a Whisper program](adr/0013-dictation.md): the design decision behind this feature.
- [Editing guide](editing.md): edit mode, for writing and changing text with speech feedback.
- [Keyboard reference](keyboard.md): every shortcut, including F9 and single-key shortcuts.
- [Documentation index](README.md)
