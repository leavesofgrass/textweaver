# Streaming dictation: the Wave 5 spike

Agent W5d, measured Monday, September 28 and Tuesday, September 29, 2026. The question for W6d: can textweaver show and announce words while the speaker is still talking, with the Whisper it already runs in-process (`base.en`, int8, on RTen; ADR-0013, ADR-0023)? This page has the measurements, a recommended design, and what the announcements would sound like. ADR-0042 is reserved for W6d's decision.

## The short answer

- **Feasible, but only worth it for long utterances.** With LocalAgreement-2 inside earshot utterances, Whisper `base.en` on RTen showed the first words of a 21-second sentence about 2.5 seconds after the speaker started, where today nothing appears until the sentence is over (about 24 seconds). For dictation with a pause every sentence or two, streaming gave little: at most 12 of 44 words were agreed on before their pause, and the rest came at the pause, as fast as today when runs were quick and up to 1.1 seconds *later* when they were slow.
- **Latency on this machine, under load:** a Whisper run took 0.75 to 2.7 seconds (median per session), so on the long sentence a word was committed about 4 to 7 seconds after it was spoken (median), 7 to 10 seconds at worst. When runs took under a second, words on the short fixtures were committed about 2 seconds behind the voice, as the fake recognizer predicts.
- **Memory:** no new model and no new dependency. The loaded model is about 300 MB (working set and private bytes), peaking near 395 MB during a run; streaming adds only the utterance's audio (64 KB a second).
- **Accuracy:** the streamed text matched the batch text in 17 of 18 runs after one fix to the probe. Committing early has a cost, though: in one of the six sessions on the long sentence, a word committed early ("until 9") was later heard differently by the final run ("until 9am"). The committed word stays; the final run's version is lost.
- **The bottleneck is Whisper's 30-second encoder.** It costs the same for one second of speech as for thirty (0.55 to 0.8 seconds median at the lighter load), and this export refuses shorter input. Every partial run pays it.
- **For W6d, build first:** a streaming speech finder (earshot fed as audio arrives), then the agreement loop from the probe behind a gate (partials only once an utterance is past about 3 seconds, and only while runs stay fast), then the announcement and Braille path. The encoder's cost is the next problem, not the first.

## How it was measured

- **The probe:** `crates/textweaver-dictation/examples/stream_probe.rs`, built with `--features rten --release`. It uses the crate's public API only (`RtenWhisper`, `audio`, `vad`); the library code is unchanged.
- **The model:** onnx-community's `whisper-base.en` int8 pair and `tokenizer.json`, the files the dictation guide lists. Nothing was downloaded: the copy used is the one Agent W3f fetched in Wave 3, in `D:\textweaver\target-models\whisper\base.en`, given to the probe with `--model` (the same as `tw dictate --model-dir`). The default location, `<data>/whisper/rten/base.en`, is in the application data folder on drive C, which agents may not read.
- **The speech:** three texts in `fixtures/d/` (`stream-note`, dictation with a pause after each sentence; `stream-long`, one 21-second sentence; `stream-short`, three short commands), spoken by textweaver's own audio export with Windows SAPI5 at rate 160 into WAV files, with word-level SRT cues. No microphone was opened and nothing was played.
- **Reference word times:** the export's word cues are spread over each sentence's audio, pauses included, so the probe stretches each utterance's cues onto the speech earshot found. Word ends are therefore estimates, good to a few hundred milliseconds.
- **The streaming simulation:** time is virtual. Audio arrives in real time; inside each earshot utterance a run starts when the previous run has finished and at least one step (700 ms) of new audio is there; each run takes the wall time it really took. At the pause (600 ms of silence) one last run over the utterance commits everything. The batch comparison is today's behavior: each utterance transcribed once, after its pause.
- **Moonshine was not measured.** The Wave 5 launch decision was Whisper only, with no model downloaded; W6d asks the owner again.
- **The machine:** the owner's Ryzen 5 5600G (6 cores, 12 threads), 64 GB, with three or four other agents building at the same time. Every number here is under that load; the spread is reported, and a quiet machine will be faster.

## Measurements

### Whisper per run, by length of audio

`stream_probe bench --runs 40`: the first 1, 2, 4 and 8 seconds of `stream-long`, one warm-up run, then 40 runs each, median and worst. Two sessions: the first with the processor at 100 percent (three other agents building), the second at 84 to 99 percent. "Processor time" is the process's time on all threads per run, a figure less sensitive to load: it is the work, not the wait.

Second session (lighter load):

- 1 second: encoder 0.82 s median, 1.75 s worst; decoder 0.33 s, 0.79 s; whole run 1.19 s, 2.06 s; processor time 2.07 s.
- 2 seconds: encoder 0.55 s, 0.94 s; decoder 0.30 s, 0.54 s; whole run 0.88 s, 1.36 s; processor time 2.22 s.
- 4 seconds: encoder 0.57 s, 0.78 s; decoder 0.12 s, 0.16 s; whole run 0.69 s, 0.93 s; processor time 1.93 s. The text was empty in every run (see below).
- 8 seconds: encoder 0.59 s, 0.82 s; decoder 0.88 s, 1.43 s; whole run 1.49 s, 2.26 s; processor time 3.13 s.

First session (full load):

- 1 second: encoder 2.48 s median, 6.42 s worst; decoder 0.65 s, 1.43 s; whole run 3.24 s, 7.86 s.
- 2 seconds: encoder 3.25 s, 13.04 s; decoder 0.88 s, 3.81 s; whole run 4.35 s, 14.15 s.
- 4 seconds: encoder 1.41 s, 7.95 s; decoder 0.20 s, 1.42 s; whole run 1.61 s, 8.34 s (empty text).
- 8 seconds: encoder 3.35 s, 6.13 s; decoder 2.90 s, 5.56 s; whole run 6.22 s, 10.54 s.

What the numbers say:

- **The encoder is a fixed cost.** It does not grow with the audio, because every input is padded to 30 seconds. The spectrogram is 8 ms and does not matter.
- **The decoder grows with the words:** about 0.3 seconds for a few words, 0.9 seconds for 25, at the lighter load.
- **Load matters more than length.** The same run took three to four times as long with the processor full. A partial-results feature must expect runs of 3 seconds or more on a busy machine, and back off.
- **Whisper can return nothing for audio cut mid-word.** The first 4.0 seconds of `stream-long` end inside "on"; all 80 runs over two sessions returned empty text, deterministically. In the streaming runs one or two partial runs per long session came back empty. The loop must treat an empty run as no information, not as a disagreement.
- **Hallucinated endings:** one second of "When the library" came back as "When the Library of". Agreement between two runs is what keeps such a tail from being committed.

### The encoder does not take less than 30 seconds

`stream_probe encoder-lengths`: the int8 encoder refuses 5 and 10 seconds of input ("incompatible input shapes ... [1, 250, 512] ... [1500, 512]" at `/Add_2`, the positional embedding), and takes 30 seconds (1.65 s, the first run, under load). whisper.cpp's `audio_ctx` option works around this by cutting the positional embedding; doing the same here means patching the ONNX graph, as `onnx_patch.rs` already patches weights. Whisper was trained on 30-second windows, and shortened contexts are reported to lose accuracy, so this is an experiment for W6d to measure, not a plan.

### Memory

From the process itself (PowerShell's `Get-Process` on Windows):

- Before loading: 4 MB working set.
- After loading the model: 294 MB working set, 290 MB private bytes, 368 MB peak.
- During runs of up to 21 seconds of audio: peak working set 392 to 394 MB, private bytes 290 to 305 MB. It did not grow over 40 runs or over a streaming session.

Streaming adds no model and no copy of the model; it keeps one utterance of 16 kHz audio (64 KB a second) and two hypotheses.

### LocalAgreement-2 over the fixtures

`stream_probe stream --repeat 2` over the three fixtures, in three sessions (one with trimming at sentence ends; trimming never applied, because no fixture has a committed sentence end before the utterance's pause), so six measurements per file. Step 700 ms; a partial run still going at the pause is cancelled once its encoder is done. The median run took 0.75 to 2.7 seconds depending on the session and the file; the third session was the lightest.

`stream-note` (six utterances of 1.5 to 3 seconds):

- Words committed by agreement: 0 to 12 of 44; the rest at the pause. The 12 came in the lightest session, with runs of 0.85 seconds.
- Word end to commit, all words: median 2.2 to 4.0 seconds, worst 3.6 to 6.8 seconds.
- Pause to the last word: median 1.0 to 2.4 seconds streaming, against 1.0 to 1.4 seconds for today's batch.
- Text identical to the batch text in all six; word error rate 0 against the reference.

`stream-long` (one utterance of 21 seconds):

- Words committed by agreement: 37 to 52 of 70.
- First words committed ("When the library"): 2.3 to 3.0 seconds after the speech started. Batch shows its first word about 24 seconds after the speech started.
- Word end to commit, words committed by agreement: median 4.0 to 6.6 seconds, worst 7.1 to 10.3 seconds. All words: median 4.8 to 6.9 seconds.
- Pause to the last word: 2.5 to 4.2 seconds streaming, 2.6 to 4.4 seconds batch: the same.
- Rewrites avoided: 1 to 17 words that a display of every hypothesis would have shown and then changed.
- Text identical to the batch text in five of six; word error rate 1.4 percent against the reference ("nine" became "9am"). In the sixth, "until 9" was committed early and the final run said "until 9am"; the streamed text kept "9".
- Commits slow down as the buffer grows: the decoder re-reads the whole sentence each run, so the last 10 seconds were committed in two or three large bursts.

`stream-short` (three commands of 1.1 to 2 seconds): nothing committed by agreement in any session; pause to the last word 0.7 to 1.5 seconds, batch 0.8 to 2.5 seconds (load noise).

Before a fix to the probe, one long session committed "until 9 in the evening and to" early; the final run said "until 9am in the evening and to add more lamps", and the loop, matching the committed "to" against the wrong "to", dropped ten words. The probe now continues after the prefix of the final hypothesis closest to the committed words, and that session's text then matched the batch. W6d's loop needs the same care, and a test for it (the probe has one).

### The fake recognizer

`stream_probe fake` runs the same loop with a recognizer that returns the reference words heard so far and cuts the word being spoken in half, at a fixed cost (0.7 seconds a run plus 25 ms a word). It commits every word, never the cut tail, with a median of 1.8 to 2.1 seconds from word end to commit on the note and command fixtures, and 4.0 seconds on the long sentence (its cost grows with the words, as the decoder's does). Even at this speed, only 10 of the note's 44 words and 2 of the commands' 11 were agreed on before the pause; on short utterances the pause does the committing. That is the shape the real loop has on a quiet machine, and it runs without a model, so W6d can use the same approach in its tests.

## A recommended design for W6d

Feasible: yes, on the Whisper textweaver already has, for long utterances. What to build, in order:

1. **A streaming speech finder.** `vad::utterances` scores complete audio; live dictation needs earshot fed 16 ms frames as the microphone delivers them, with an utterance that opens after 200 ms of speech and closes after 600 ms of silence (the same `VadConfig`). Everything else hangs on the open and close events. It is small, testable with the fixtures, and useful to batch dictation too (it can start transcribing an utterance at its pause instead of at Enter).
2. **The agreement loop, on the worker thread `RtenDictation` already has.** Move `stream_probe`'s `la` module into the crate: runs over the open utterance whenever the last run has finished and 700 ms of new audio has arrived; commit the words two consecutive runs agree on after the committed ones; skip an empty run; at the pause, cancel a run still going (the `AtomicBool` that `transcribe` already checks between decoder steps) and run once over the whole utterance, committing the rest after the prefix closest to what is committed. Words are committed, never withdrawn.
3. **A gate, so short dictation is not made slower.** Partial runs only once an utterance is past about 3 seconds (below that little is agreed on before the pause, and a cancelled run can delay the final one), and only while runs stay under about 1.5 seconds; after a slow run, wait for the pause. With the gate, the worst case is today's behavior.
4. **A new event, and the announcement path.** A `DictationEvent` for committed words (the `Partial` event today carries a finished segment); the app announces each burst once through `textweaver_a11y::route` at `Priority::Polite`, and shows the utterance on the status line. The final text of each utterance is still the `Final` path, so spoken commands (`apply_spoken_commands`) apply to finished text only, as ADR-0013 decided.
5. **Then the speed.** The encoder's 30-second window is the floor under every run. In order of cost: pin RTen's thread count and measure it under load; try a graph patch that shortens the encoder's window (whisper.cpp's `audio_ctx`) and measure its accuracy on the fixtures; ask the owner again about Moonshine, whose encoder takes any length (a download of about 60 MB, unverified). Trimming the buffer at a committed sentence end, as Whisper-Streaming does, helps only when a speaker runs sentences together without a 600 ms pause; the fixtures never did, so it is not measured here.

Not recommended: showing or speaking the unstable tail. It changed up to 17 words per file in these runs, and a screen reader would speak each change.

What W6d should measure that this spike could not: the loop on a quiet machine (the owner's, with nothing else building), with a real microphone and a real voice, and the interplay with textweaver's own speech (below).

## What announcing committed words would sound like

The probe prints what would be announced, one line per commit, timed from the start of the audio. With real Whisper on `stream-long` (21 seconds of speech; second session, round 2):

- at 2.8 seconds: "When the library"
- at 5.8 seconds: "opened its new"
- at 6.9 seconds: "reading room on the"
- at 8.1 seconds: "third floor, the students found long wooden tables,"
- at 10.2 seconds: "quiet corners with"
- at 13.2 seconds: "soft chairs,"
- at 17.8 seconds: "and a row of computers along the east wall,"
- at 20.6 seconds: "and within a week the room was full every afternoon, so the"
- at 23.8 seconds, after the pause: the last 22 words at once.

On `stream-note`, every announcement was a whole sentence at its pause ("The tram was late again this morning."), exactly what batch dictation per utterance would say, a little later.

What that means for a listener:

- **Words come in bursts of two to twelve, several seconds behind the voice,** never the half-heard word at the end. A burst is spoken once and never corrected, so there is no "Mon... Monday" stutter.
- **The end of each utterance comes at the pause, in one burst.** Agreement needs a second run that has heard past a word, and the last words of an utterance have nothing after them, so the final run commits them after the 600 ms pause. On a long utterance that last burst can be 20 words.
- **Speech and the microphone.** textweaver's own voice must not be heard by the microphone. Star paused the reading voice before recording; with live announcements the voice speaks *during* recording. W6d needs one of: spoken announcements only with headphones, the Braille display only while recording (the committed words shown on the status line, no speech), or speech held until the pause. The spike cannot test this without a microphone; it belongs in a listening session.
- **Braille.** Committed words are stable text, so the status line can show them growing without redrawing earlier cells: the newest words last, the line starting with the utterance's first committed word. On a 40-cell display, the line should show the last 40 cells of the utterance, meaning first ("Dictating: notes for Monday.").
- **In self-voicing mode,** a burst is an announcement routed through `textweaver_a11y::route` at `Priority::Polite`, so it never interrupts the owner's own reading of the screen, and a new burst queues behind the last.

## See also

- [ADR-0013: Dictation through a Whisper program](../adr/0013-dictation.md)
- [ADR-0023: Piper voices and Whisper dictation in-process on RTen](../adr/0023-in-process-neural-speech.md)
- [Voice typing guide](../dictation.md)
- [Wave 5, recalibrated](wave5-recalibrated.md), section 3.1
