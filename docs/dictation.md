# Dictation: turning speech into text

`tw dictate` turns recorded speech into text. You give it an audio file, such as a recorded lecture, a voice memo, or a note you spoke into your phone, and it prints what was said. It can also follow spoken commands such as "new line" and "period", so you can dictate a letter or a paragraph with its punctuation. It is for students who find speaking easier than typing, and for anyone who wants a written copy of a recording. The recognition is done by Whisper, a speech recognition program you install once. Everything happens on your own computer. With the in-process Whisper model installed, `tw dictate` also records from the microphone; see [Whisper inside textweaver](#whisper-inside-textweaver).

This guide is written to be read with a screen reader. Each section starts with the command, then explains it.

## Before you start

You need two things, and sometimes a third:

- A Whisper program, or the Whisper model textweaver runs itself. textweaver does not include a Whisper program and never downloads one. It offers to download its own Whisper model when you first dictate, and downloads it only when you say yes; see [Whisper inside textweaver](#whisper-inside-textweaver).
- A Whisper model. The model is the part that knows the language. Two of the three Whisper programs download their models by themselves. whisper.cpp needs you to download a model file.
- ffmpeg, a program that converts audio. whisper.cpp needs it for any file that is not a WAV file. OpenAI Whisper needs it for every file.

### The Whisper programs textweaver can use

textweaver works with three Whisper programs. When you do not choose one, it uses the first one it finds, in this order:

1. The program named by the `TEXTWEAVER_WHISPER` environment variable, if that variable is set. See [Choose which Whisper program runs](#choose-which-whisper-program-runs).
2. whisper.cpp. textweaver looks for a program called `whisper-cli`, and then `whisper-cpp` (the name older builds used).
3. faster-whisper, through its command line program `whisper-ctranslate2`.
4. OpenAI Whisper, through its command line program `whisper`.

textweaver looks for these programs in the folders on your `PATH`. On Windows it tries the usual program extensions, such as `.exe`, first.

whisper.cpp comes first because it is one program with no Python, and it starts fastest. faster-whisper comes next. OpenAI Whisper is the slowest to start: on the development machine, its first run took about two minutes while Python loaded, and later runs took a few seconds.

A whisper.cpp program with another name can be used too. Name it with `--program` or `TEXTWEAVER_WHISPER`, and say it is whisper.cpp with `--engine cpp` or `TEXTWEAVER_WHISPER_ENGINE`. A program called `main`, the name of very old whisper.cpp builds, is recognized as whisper.cpp by itself.

### Install a Whisper program

Install one of these. You only need one.

whisper.cpp: download a release from the whisper.cpp project on GitHub (github.com/ggml-org/whisper.cpp, under Releases). The Windows download is a zip file. Unzip it, and put the folder that holds `whisper-cli.exe` on your `PATH`, or point `TEXTWEAVER_WHISPER` at the program. On a Mac with Homebrew, for example:

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

A model is the trained speech recognizer. Larger models are more accurate and slower. The sizes, smallest first:

- `tiny`: the fastest and the least accurate.
- `base`: the default. textweaver uses it when you do not choose.
- `small`.
- `medium`.
- `large-v3`: the most accurate and the slowest.
- `large-v3-turbo`: nearly as accurate as `large-v3`, and much faster.

faster-whisper and OpenAI Whisper download a model the first time you use it, and keep it for next time. That first download needs an internet connection. It is done by the Whisper program, not by textweaver, and it sends none of your audio.

whisper.cpp does not download models. You download the model file yourself. Its name is `ggml-` followed by the model size and `.bin`: `ggml-base.bin` for `base`, `ggml-large-v3-turbo.bin` for `large-v3-turbo`. The files are on the whisper.cpp project's model page on Hugging Face. whisper.cpp's source code also comes with a download script; in its folder, for example:

```bash
./models/download-ggml-model.sh base
```

### Where textweaver looks for whisper.cpp model files

textweaver looks for the model file in these folders, in this order, and uses the first one it finds:

1. The folder named by the `TEXTWEAVER_WHISPER_MODELS` environment variable, if it is set.
2. The `whisper` folder in textweaver's data folder:
   - Windows: `%APPDATA%\leavesofgrass\textweaver\data\whisper`
   - macOS: `~/Library/Application Support/org.leavesofgrass.textweaver/whisper`
   - Linux: `~/.local/share/textweaver/whisper` (or `textweaver/whisper` in `$XDG_DATA_HOME` when that is set)
   - When `TEXTWEAVER_HOME` is set: `data/whisper` inside that folder.
3. A `models` folder next to the whisper.cpp program.
4. A `models` folder one level up from the program's folder. This is where whisper.cpp's own download script puts models.

The simplest choice is to make the `whisper` folder in textweaver's data folder and put the model file there. To use a model file somewhere else, give it with `--model-file`; see [Use a whisper.cpp model file from anywhere](#use-a-whispercpp-model-file-from-anywhere).

### ffmpeg

whisper.cpp reads WAV files only. When you give it another kind of file, such as MP3 or M4A, textweaver converts it first with ffmpeg, to a 16 kHz, mono, 16-bit WAV file in a temporary folder. ffmpeg must be on your `PATH` for this. A file whose name ends in `.wav` is passed to whisper.cpp as it is, without converting.

OpenAI Whisper uses ffmpeg to read every audio file, WAV files included. Its own code says it needs the ffmpeg program on the `PATH`.

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

Three environment variables change how textweaver finds Whisper. You do not need any of them when your Whisper program is on your `PATH` and its models are in one of the usual folders.

- `TEXTWEAVER_WHISPER`: the full path of a Whisper program to use. It is tried before anything on the `PATH`. textweaver uses it only if the file exists and it can tell which Whisper program it is. Otherwise it is skipped without a message, so check with `tw dictate --list`.
- `TEXTWEAVER_WHISPER_ENGINE`: which kind of program `TEXTWEAVER_WHISPER` is, when its file name does not tell: `cpp`, `faster`, or `openai`. The longer names `whisper.cpp`, `faster-whisper`, and `openai-whisper` work too. It applies only to the program in `TEXTWEAVER_WHISPER`.
- `TEXTWEAVER_WHISPER_MODELS`: a folder of whisper.cpp model files. It is the first folder searched. The other programs ignore it.

To set one for the current PowerShell window, for example:

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
tw dictate --list
```

This lists every Whisper program textweaver found, in the order it would use them, and then the model sizes. It records nothing and transcribes nothing. On a Windows computer with only OpenAI Whisper installed, you see two lines like these (the folder depends on where Python put the program):

```text
OpenAI Whisper: C:\Users\you\AppData\Local\Programs\Python\Python313\Scripts\whisper.exe
Models: tiny, base, small, medium, large-v3, large-v3-turbo
```

Each program is named as "whisper.cpp", "faster-whisper", or "OpenAI Whisper", followed by where it is. The first one listed is the one `tw dictate` uses.

When nothing is found, you see:

```text
No Whisper program found. Install whisper.cpp (whisper-cli), faster-whisper (whisper-ctranslate2), or OpenAI Whisper (whisper).
Models: tiny, base, small, medium, large-v3, large-v3-turbo
```

For scripts, add `--json`:

```text
tw dictate --list --json
```

This prints a list with one entry per program. Each entry has `engine`, which is `cpp`, `faster`, or `openai`, and `program`, the path. When nothing is found, the list is empty: `[]`.

`--list` checks programs only. It does not check whether whisper.cpp's model file is present; a transcription does that before it starts.

## Transcribe an audio file

```text
tw dictate --file lecture.mp3
```

This transcribes `lecture.mp3` with the first Whisper program found and the `base` model. Any audio format your Whisper program can read works. With whisper.cpp, files other than WAV need ffmpeg (see [ffmpeg](#ffmpeg)).

While it works, you hear or see "Transcribing, this may take a while". Then each piece of speech is printed as Whisper finishes it. A piece is usually a sentence or a few sentences; the text does not appear word by word. When it is done, you hear or see how many words it found, for example "Transcribed 19 words", and then the whole transcript, as one paragraph.

The progress messages go to the error stream and the transcript goes to standard output. In a terminal you get both. To keep only the transcript, write it to a file with `--out`, or redirect standard output.

A long recording takes a while. A bigger model takes longer. On the development machine, OpenAI Whisper with the `tiny` model took about 15 seconds for 12 seconds of speech.

## Save the transcript to a file

```text
tw dictate --file lecture.mp3 --out lecture.txt
```

This writes the transcript to `lecture.txt` instead of printing it. The progress messages and the word count are still read out. If `lecture.txt` already exists, it is replaced without asking.

With `--json`, the file holds the JSON result instead of plain text.

## Choose a model

```text
tw dictate --file lecture.mp3 --model small
```

`--model` chooses the model size: `tiny`, `base`, `small`, `medium`, `large-v3`, or `large-v3-turbo`. Upper and lower case are both fine. Without it, the model is `base`. See [Models](#models) for how they differ.

Try `tiny` for a quick first look at a long recording. Try `small` or larger when names or technical words come out wrong.

With whisper.cpp, the model file for that size must be in one of the model folders, or you get a message naming the file and the folders searched.

## Choose the language

```text
tw dictate --file vorlesung.mp3 --language de
```

`--language` tells Whisper which language is spoken, as a short code: `en` for English, `de` for German, `fr` for French, `es` for Spanish, and so on. Without it, Whisper listens to the start of the recording and guesses. Giving the language is faster and avoids a wrong guess, especially for short recordings.

## Add times to each line

```text
tw dictate --file lecture.mp3 --timestamps
```

With `--timestamps`, each piece of speech starts on its own line with the time it starts in the recording, in minutes and seconds in square brackets. For example:

```text
[00:00] Today we look at how cells divide.
[00:08] There are two kinds of division.
```

From one hour on, the time has hours too, for example `[01:02:03]`. This is the same layout Star used. It helps you find a place in a long recording.

Do not use `--timestamps` together with `--commands` for now. The spoken commands join all the lines into one, so the times end up in the middle of the text.

## Use spoken commands

```text
tw dictate --file letter.wav --commands
```

With `--commands`, the words you say for punctuation and line breaks become the marks themselves. For example, if you say "Dear Sam comma new line thanks for the book period", the transcript is:

```text
Dear Sam,
Thanks for the book.
```

Spoken commands are off unless you ask for them, because the same words are often meant as words ("the period of history").

These are all the spoken commands. Case and any punctuation Whisper puts on the command word do not matter.

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

Whisper adds punctuation by itself, and it often writes a command word as a little sentence of its own, such as "Period." textweaver tidies this. The punctuation Whisper puts on a command word is dropped. Punctuation Whisper puts just before a punctuation or line command is replaced by the command, so "world, period" gives "world." and not "world,.".

To type a command word as a word, say "literal" before it. "literal period" writes the word "period", and "the literal period of history" gives "the period of history". "literal" protects only the one word right after it. For a two-word command, that is usually enough: "literal new line" writes "new line", because "line" alone is not a command. One exception is "semi colon": "literal semi colon" still turns "colon" into a colon, so say "literal semicolon" as one word instead. To write the word "literal" itself, say "literal literal".

The commands are applied to the finished transcript. The pieces printed while Whisper works show the words as Whisper heard them, with the command words still in. The word count ("Transcribed 19 words") also counts the words as Whisper heard them, command words included.

## Choose which Whisper program runs

```text
tw dictate --file lecture.mp3 --engine faster
```

`--engine` chooses the kind of Whisper program: `cpp` for whisper.cpp, `faster` for faster-whisper, or `openai` for OpenAI Whisper. textweaver uses the first program of that kind that it finds, in the order described in [The Whisper programs textweaver can use](#the-whisper-programs-textweaver-can-use). Without `--engine`, it uses the first program found of any kind. The longer names `whisper.cpp`, `faster-whisper`, `whisper-ctranslate2`, `openai-whisper`, and `whisper` work too.

To run one particular program, give its path with `--program`:

```text
tw dictate --file lecture.mp3 --program "C:\Tools\whisper\whisper-cli.exe"
```

textweaver then runs that program and does not search your `PATH`. It works out the kind of program from its file name: a name containing "ctranslate2" or "faster" is faster-whisper, a name containing "whisper-cli" or "whisper-cpp", or the name "main", is whisper.cpp, and any other name containing "whisper" is OpenAI Whisper. When the name does not tell, add `--engine`:

```text
tw dictate --file lecture.mp3 --program "C:\Tools\stt\recognizer.exe" --engine cpp
```

## Use a whisper.cpp model file from anywhere

```text
tw dictate --file lecture.wav --model-file "D:\models\ggml-base.en.bin"
```

`--model-file` gives whisper.cpp's model file directly, so it does not need to be in one of the model folders. This is also how you use a model file with a name textweaver does not look for by itself, such as the English-only models (`ggml-base.en.bin`) or smaller compressed ones.

When you give `--model-file`, whisper.cpp uses that file, whatever `--model` says. `--model-file` is used only with whisper.cpp. faster-whisper and OpenAI Whisper ignore it and use `--model`.

## Get the result as JSON

```text
tw dictate --file lecture.mp3 --json
```

With `--json`, `tw dictate` prints one JSON object and no progress messages. This is for scripts and other programs. The object has:

- `engine`: `cpp`, `faster`, or `openai`.
- `program`: the path of the Whisper program that ran.
- `model`: the model size, such as `base`.
- `text`: the transcript, with `--timestamps` and `--commands` applied if you gave them.
- `transcript`: the pieces exactly as Whisper produced them. It holds `segments`, a list in which each piece has `start_ms` and `end_ms` (its start and end in milliseconds from the start of the recording) and `text`.

Add `--out` to write the JSON to a file instead.

## What you hear and see

`tw` itself does not speak. It prints short messages, which your screen reader reads out in the terminal. These are the messages, exactly as the program writes them:

- "Transcribing, this may take a while": Whisper has started.
- Each piece of the transcript, as Whisper finishes it.
- "Transcribed 19 words" (with the real number), or "Transcribed 1 word": Whisper is done. The transcript follows.
- "Dictation produced no text": Whisper finished but recognized no words.

When something goes wrong, you hear one message starting with "Error:", and `tw dictate` ends with exit status 1, so scripts can tell. The messages are listed in [If something goes wrong](#if-something-goes-wrong).

With `--json`, none of the progress messages are printed; only the JSON is.

## Your audio stays on your computer

textweaver sends no audio anywhere. The dictation part of textweaver has no network code at all. It runs the Whisper program on your computer and reads the text it writes. Downloading the Whisper model, when you agree to it, is done by textweaver's optional components, which fetch the model files only.

While it works, textweaver makes a temporary folder for the session, in your system's temporary folder, with a name starting `textweaver-dictation-`. Whisper's output, and the converted WAV file when whisper.cpp needs one, go there. The folder is deleted when the transcription ends, whether it worked or not. Your own audio file is never changed.

The one time the internet is used is when faster-whisper or OpenAI Whisper downloads a model you have not used before. That download is done by the Whisper program, it fetches the model only, and it sends none of your audio.

## Whisper inside textweaver

textweaver can run Whisper itself, with no Whisper program, on RTen, a model runtime written in Rust ([ADR-0023](adr/0023-in-process-neural-speech.md)). It needs three files from Hugging Face's `onnx-community/whisper-base.en`, in one folder, `<data>/whisper/rten/base.en` (79.3 MB in all):

- `encoder_model_int8.onnx` (23 MB);
- `decoder_model_merged_int8.onnx` (54 MB);
- `tokenizer.json` (2.4 MB).

The license is MIT, from OpenAI. The onnx-community copies declare no license of their own, so textweaver says "MIT, unconfirmed".

### Getting the model

textweaver offers to download the model; it never downloads it on its own.

- **In the reader and the window:** the first time you dictate without the model, you hear "Dictation needs the Whisper model, 79.3 MB, license MIT, unconfirmed. Download it now? y or n". Press `y` and the download starts on its own, with its progress said every 10 percent; Escape stops it, and the next download goes on from where it stopped. When it finishes, you hear "Ready", and dictation starts. Press `n` and nothing is downloaded; you are not asked again until you start textweaver again, and Dictate says "No model, so no dictation for now."
- **From the Tools menu:** Download the dictation model asks the same question at any time.
- **From the command line:** `tw dictate download` says what it downloads and asks; `tw dictate download --yes` downloads without asking. When `tw dictate` finds no model, it offers the same download (`--yes` answers for it), then goes on dictating.
- **From Manage optional components** (Tools menu, or `tw components`): the model is listed with its size and license, with Download, Verify, Remove, and Install from a file.

Each file is checked against its published size and SHA-256 before it is kept, and nothing is used that does not match. The model is an optional component; see [Optional components](components.md) for the mirror and for installing from a downloaded zip or folder on a computer without internet.

**Choosing a model.** The Dictation model setting chooses which Whisper model dictation uses: base.en (the default) or small.en (251 MB, slower, more accurate English). The question and Download the dictation model offer the one chosen.

**Placing the model by hand.** You can still download the three files yourself and put them in `<data>/whisper/rten/base.en` (or a `onnx` folder inside it). textweaver adopts a folder that already holds the right files instead of downloading them again. `--model-dir DIR` on the command line, or the Dictation model folder setting, names another folder (`tw dictate --model-dir D:\Models\whisper-tiny.en`); a folder you name is used as it is, without the download offer.

When the folder holds the model, `tw dictate` uses it without being asked, and `--engine rten` asks for it.

**When the model cannot be used,** textweaver says why in words: "Dictation model lacks decoder_model_merged_int8.onnx." (a file is missing), "Dictation model damaged: tokenizer.json." (a file has the wrong size; the download is offered again), "No model folder: D:\Models\whisper" (the folder you named does not exist), or "No model; this build cannot download." (a reader built without downloads; place the files by hand).

### Dictating from the microphone

```text
tw dictate
```

Without `--file`, `tw dictate` records from your default microphone. You hear "Recording. Press Enter to stop." Speak, then press Enter. It says "Transcribing", then prints the text. Pauses are skipped: a voice detector (earshot) finds where you spoke, so long silences cost nothing and are not turned into invented words.

Without the in-process model, `tw dictate` offers to download it (or, when nobody can answer, says where it goes and that `tw dictate download` gets it), and you can still record with any program you like and transcribe the file with `tw dictate --file`.

### Seeing the words while you talk

```text
tw dictate --live
```

With `--live`, textweaver transcribes while you speak. Each phrase is finished at your pause, not when you press Enter, and in a long sentence the first words appear while you are still talking. Each group of words is printed on its own line as soon as it is certain, and is never changed afterwards, so your screen reader reads each word once. The finished text is still printed at the end, with spoken commands applied if you asked for them.

What to expect:

- **Short phrases,** under about three seconds, arrive at their pause, as quickly as without `--live`.
- **A long sentence** shows its first words about four seconds in, then more every second or two, a few seconds behind your voice. The last words come at the pause.
- **On a busy computer,** when Whisper takes longer than a second and a half, textweaver waits for your pause instead, so nothing arrives later than it would without `--live`.
- **A phrase with no words recognized** says "No words recognized in that phrase", so nothing vanishes silently.

`tw dictate --live --file note.wav` plays a recording in at speaking pace (silently) to show how it would go live. `--live` needs the in-process model.

### How long it takes

`--timings` prints where the time went: loading the model (once), the spectrogram, the encoder, the decoder, and the time from pressing Enter to the text.

```text
tw dictate --file note.wav --timings
```

## Dictating in the reader

In edit mode, the **Dictate** command types what you say at the cursor. It is in the Edit menu and the command palette (**F2**, then "dictate"); its key is **Ctrl+Shift+F9**, or whatever the menu shows beside it. Press it again to stop.

1. Open a document and turn on edit mode (**Ctrl+E**). If edit mode is off when you ask to dictate, textweaver asks "Turn on edit mode and dictate? y or n" first.
2. Choose Dictate. You hear "Dictating. Speak, then press" and the key "to stop." Reading aloud stops, so the microphone does not hear it.
3. Speak. The status line shows what textweaver is sure of as you talk, starting "Dictating:", with the newest words last and no more than 40 characters, so a 40-cell Braille display shows the latest words without scrolling. Words are never changed once shown.
4. Pause. At each pause (about half a second of silence) the phrase is typed at the cursor, with spoken commands applied ("new line", "period", "comma", "open quote"; see [Use spoken commands](#use-spoken-commands)). Each phrase is one step for Undo.
5. Choose Dictate again to stop. The last phrase is typed, and you hear "Dictation done."

**Your voice and textweaver's voice.** By default textweaver does not speak the words while the microphone is open: they appear on the status line as they come and are said once, at each pause. To hear them as they come, for example with headphones, turn on Speak while dictating in the settings (the Dictation section):

```toml
[dictation]
speak_while_recording = true
```

The words are said as ordinary messages, so [interface announcements](reading.md) set to off keeps them to the status line and the Braille display. Problems, such as a missing microphone, are always said.

**The model.** Dictation in the reader uses Whisper inside textweaver, from the same folder as `tw dictate` ([Whisper inside textweaver](#whisper-inside-textweaver)), or the folder in `model_dir`:

```toml
[dictation]
model_dir = "D:\\Models\\whisper-base.en"
```

The first dictation after starting textweaver loads the model, which takes a few seconds; later ones start at once.

**Nothing is lost.** Turning edit mode off, opening another document, starting a new one, or quitting while you dictate first finishes the dictation: recording stops and the last phrase is typed. If Whisper needs more than ten seconds for that, dictation ends and you hear "Dictation stopped before its last words were typed."

What you may hear:

- "No words recognized in that phrase.": textweaver heard speech but could not make out words. Try again, a little closer to the microphone.
- "Dictation failed:" and the reason, such as "No audio was recorded. Check your microphone."
- "Dictation needs the Whisper model in" and a folder: the model files are not there.

## Dictating into the reader with other software

```toml
[keyboard]
character_keys = false
```

You can dictate into textweaver with speech recognition software that types for you, such as Windows Voice Access, Dragon, or macOS Dictation. That software types letters as if you pressed the keys. In reading mode, many single letters and punctuation keys are shortcuts, so dictated text could set off commands.

To stop that, turn single-key shortcuts off. Press **F9**, in the window or in the terminal version. You hear "Single-key shortcuts off." Press **F9** again to turn them back on, and you hear "Single-key shortcuts on." You can also run `toggle character keys` from the command palette (**F2**), or put the setting above in `settings.toml`. textweaver remembers the choice.

While single-key shortcuts are off, letters, punctuation, and Space never trigger commands in any mode. Shortcuts with Ctrl or Alt, the arrow keys, the function keys, and the command palette still work. Quitting always asks first, so a stray keystroke cannot close your document.

To write text by dictation, switch to edit mode first; see [the editing guide](editing.md).

## If something goes wrong

- "No Whisper program found." from `tw dictate --list`, or "Speech recognition needs Whisper, which was not found." from a transcription: textweaver found no Whisper program. Install one (see [Install a Whisper program](#install-a-whisper-program)), then open a new terminal so it sees the new `PATH`. Or set `TEXTWEAVER_WHISPER` to the program, or give it with `--program`. You also get the second message when you ask with `--engine` for a kind of program that is not installed, even if another kind is.
- `TEXTWEAVER_WHISPER` seems to be ignored: textweaver skips it without a message when the file does not exist, or when it cannot tell which kind of program it is. Check the path, set `TEXTWEAVER_WHISPER_ENGINE`, and run `tw dictate --list` to see what is found.
- "The Whisper model "base" was not found. Download ggml-base.bin into one of:" followed by a list of folders: whisper.cpp needs its model file. Download the file it names into one of the folders it lists, or give the file with `--model-file`. This is checked before anything is transcribed.
- "The Whisper model "base" was not found. Download" followed by a file path and "into one of: the folder given": the file you gave with `--model-file` does not exist. Check the path.
- "Name the program's engine with --engine cpp, faster, or openai": the file name you gave with `--program` does not say which kind of Whisper program it is. Add `--engine`.
- "Unknown model size" or "Unknown engine": a typing mistake in `--model` or `--engine`. The message lists the names you can use.
- A file name followed by "file not found": the audio file given with `--file` does not exist. Check the name, and put it in quotation marks if it has spaces. This message is followed by "Caused by: file not found", which says the same thing again.
- "whisper.cpp reads WAV files only, and ffmpeg was not found to convert this one. Install ffmpeg, or convert the file to WAV first.": install ffmpeg (see [ffmpeg](#ffmpeg)), or convert the file yourself.
- "The audio file could not be converted:" followed by ffmpeg's reason: ffmpeg could not read the file. It may be damaged, or not an audio file.
- "Could not start" followed by a program and a reason: the Whisper program or ffmpeg could not be run. Check the path given with `--program` or `TEXTWEAVER_WHISPER`.
- "Whisper failed:" followed by a line from Whisper: the Whisper program stopped with an error. The line after the colon is Whisper's own last error line, or "it stopped without saying why". With OpenAI Whisper, a message about ffmpeg most likely means ffmpeg is missing; install it (see [ffmpeg](#ffmpeg)).
- "Whisper's transcript could not be read:": Whisper finished but its result could not be read. Try again, or try another Whisper program.
- "Dictation produced no text": Whisper heard no speech. Check that the file has speech in it and is not silent, and try `--language` if the speech is not in the language Whisper guessed.
- whisper.cpp gives an error about the WAV file: textweaver passes WAV files to whisper.cpp unchanged, and whisper.cpp expects 16 kHz audio. Convert the file to 16 kHz mono WAV with ffmpeg, for example:

  ```text
  ffmpeg -i lecture.wav -ar 16000 -ac 1 -c:a pcm_s16le lecture-16k.wav
  ```

- The words are often wrong: try a larger `--model`, give the `--language`, and use a recording with less background noise.
- It is very slow: try a smaller `--model`, such as `tiny` or `base`. whisper.cpp and faster-whisper need no PyTorch, so they may also start faster than OpenAI Whisper.
- Dictated text set off commands in the reader: press **F9** to turn single-key shortcuts off (see [Dictating into the reader with other software](#dictating-into-the-reader-with-other-software)).

## See also

- [ADR-0013: Dictation through a Whisper program](adr/0013-dictation.md): the design decision behind this feature.
- [Editing guide](editing.md): edit mode, for writing and changing text with speech feedback.
- [Keyboard reference](keyboard.md): every shortcut, including F9 and single-key shortcuts.
- [Documentation index](README.md)
