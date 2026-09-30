# ADR-0042: Streaming dictation and dictation in the reader

- Status: accepted; the voice while recording waits for a listening session with a microphone.
- Date: 2026-09-29 (Tuesday, September 29, 2026)

## Context

Dictation ran Whisper in-process on RTen ([ADR-0023](0023-in-process-neural-speech.md)), but only after recording stopped: `tw dictate` recorded until Enter, then transcribed. Nothing appeared while the speaker talked, and the reader had no dictation at all. Whisper does not stream: it transcribes a stretch of audio at a time, and its encoder always works on a 30-second window, so a run over one second of speech costs about as much as a run over thirty.

A measured spike (the streaming dictation spike, Wave 5) found that re-running Whisper over the utterance as it grows, and committing only the words two runs in a row agree on (LocalAgreement-2, as Whisper-Streaming does), shows the first words of a 21-second sentence about 2.5 seconds after the speaker starts, where batch transcription shows nothing for about 24 seconds. For dictation with a pause every sentence or two it gained little, and when runs were slow it made the text up to a second later than batch. It also found that Whisper returns nothing for audio cut mid-word, that a run's last word is often half heard, and that matching committed words against a changed hypothesis needs care (a committed "to" matched against the wrong "to" dropped ten words).

Three further constraints come from the reader: every announcement goes through the app's routing, so the interface announcement level applies ([ADR-0043](0043-menus-and-the-palette-from-one-model.md)); textweaver's own voice must not be heard by the microphone; and work in progress must never be lost silently when a document closes or the program quits.

## Decision

### Speech found as it arrives

`vad::SpeechFinder` feeds earshot 256-sample (16 ms) frames as the audio arrives, in pieces of any size, and reports each utterance as it opens (after 200 ms of speech) and closes (after 600 ms of silence), with the same `VadConfig` as before. The rules alone are `vad::SpanTracker`, which gives exactly the utterances `speech_spans` gives over the same scores, padding and joined neighbors included. A test checks this on random score patterns, and another checks that the finder and `vad::utterances` agree on the speech fixtures.

### The agreement loop

`stream` holds the rest, written against a feed and a recognizer so the tests drive it with a fake recognizer in virtual and real time:

- A listener thread takes the live audio (`LiveAudio`, which the microphone capture fills at 16 kHz as it records), finds utterances, and passes audio and speech events to the model's worker thread in order.
- The worker runs Whisper over the open utterance whenever the last run has finished and 700 ms of new audio has arrived, and commits the words that run and the last one agree on after the words already committed (`Agreement`). An empty run is no information. A run's words are compared lower case, without punctuation.
- When the listener finds the pause it cancels a partial run still going (Whisper stops between decoder steps; the encoder runs to its end), and the utterance's last run commits the rest, after the prefix of its hypothesis closest to the committed words by edit distance. Committed words are never withdrawn, so the utterance's text in the final transcript is exactly its committed words. The final run can disagree with an early commit ("until 9" against "until 9am"); the committed words stay.
- Committed words arrive as `DictationEvent::Committed`. A phrase heard with no words recognized arrives as `DictationEvent::NoWords` and is said, never left as silence. `Final` is unchanged, and spoken commands apply to finished text only, as [ADR-0013](0013-dictation.md) decided.

### The gate

Partial runs happen only when they can help:

- only once the utterance has about 3 seconds of speech, measured to the end of the latest speech heard, so the silence before a pause never counts;
- not once the speaker has been quiet for 300 ms, half the pause that ends an utterance;
- only while runs stay under about 1.5 seconds; after a slower run, the utterance waits for its pause.

So short dictation (phrases under three seconds) never runs a partial at all and is never later than transcribing each phrase at its pause; a test checks this over five short phrases at four recognizer speeds. `StreamConfig::pauses_only()` is that behavior on its own, the fallback when partial runs cost too much. Even with no partial runs, live dictation is faster than recording everything and transcribing after Enter, because each phrase is transcribed at its pause while recording goes on.

### Dictation in the reader

The Dictate command (`Ctrl+Shift+F9`, the Edit menu, the palette) gets its handler from the app's dictation module (the `dictation` feature, on by default). In edit mode it starts and stops the microphone; outside it, it asks whether to turn edit mode on first. Reading aloud stops, so the microphone does not hear it.

- **Typing.** At each pause the phrase is typed at the caret, with spoken commands applied, through the editor's replace path (the one a native text control uses, which says nothing itself) as one undo step. A space is added when a word would run into the one before.
- **The status line.** Committed words appear as they come, meaning first and within 40 cells, the newest last: "Dictating: notes for Monday." The status line starts again with each phrase.
- **Speech.** The words are said at `Polite` with importance `Result` through `App::announce_as`, so the interface announcement level applies; errors are never silenced. By default (`[dictation] speak_while_recording = false`) they are held while the microphone is open and said once at the pause, while the status line, and the Braille display that follows it, grows as they come. The status line written without speech is the one new sink in the routing test's list.
- **Nothing lost.** Leaving edit mode, opening or starting another document, and quitting first finish the dictation: recording stops, the last phrase is transcribed and typed, and only then does the command go on. The wait is at most ten seconds, as quitting waits for the writer; if Whisper takes longer, the session ends and "Dictation stopped before its last words were typed" is said. Quitting then shuts the recognizer down through `Dictation::shutdown`, which waits at most ten seconds for its worker thread.
- **The model** is the in-process one `tw dictate` uses, or the folder in `[dictation] model_dir`. It loads on the first dictation and stays loaded.

`tw dictate --live` prints each burst of committed words on its own line as it comes; with `--file`, the recording is played in at speaking pace, never aloud.

### Speed

Two ways to make runs cheaper were measured on the owner's machine (Ryzen 5 5600G, 6 cores and 12 threads), with other agents building, so the processor stood at 100 percent throughout.

- **A shorter encoder window.** `onnx_patch::shorten_encoder` cuts the encoder's positional embedding from 1500 rows to fewer, as whisper.cpp's `audio_ctx` does, and `RtenWhisper::load_with_window` loads such an encoder. The encoder's cost follows the window: over 1 to 4 seconds of speech it took 0.14 to 0.19 seconds (median) at a 10-second window, against 0.83 to 1.44 seconds at 30 seconds, and whole runs 0.27 to 0.45 seconds against 0.96 to 1.84. Accuracy does not hold. At 10 seconds Whisper repeated a sentence of the note fixture (15.9 percent word errors against 0) and looped on the 21-second sentence once it spanned two windows (651 percent); at 20 seconds the short fixtures were right, but the long sentence, again split, had 52.9 percent. The shortened encoder is not used. The loader stays for measurement (`stream_probe --window`).
- **RTen's thread count.** RTen uses one thread per physical core by default (6 here), and `RTEN_NUM_THREADS` pins another count for the process. Eight runs each over 1, 2, 4 and 8 seconds of speech, at 30 seconds of window: with 2 threads whole runs took 1.4 to 2.1 seconds (median; worst 3.4); with 4, 4.8 to 7.6 (worst 16.1); with 6, 3.6 to 7.0 (worst 14.0); with 12, 1.8 to 4.7 (worst 10.0). The processor time per run was about the same in all four (1.6 to 3.0 seconds): on a machine already busy, more threads wait for cores rather than work. This is one session with the load changing under it, so the default stays; the count to pin is measured again on a quiet machine, where more threads should help. If 2 threads holds up there too, the dictation worker gets its own pool (RTen's `RunOptions`), leaving the process default to the voices.

Not pursued: SimulStreaming's AlignAtt policy, about five times cheaper than LocalAgreement at the same quality, needs the decoder's cross-attention weights, which the ONNX export does not return; it is the known better policy if an export with them appears. Moonshine, whose encoder's cost follows the audio, waits for the owner's approval of the model.

## Checks

- The finder against `speech_spans` on 2,000 random cases and against `vad::utterances` on the fixtures.
- The loop with a fake recognizer in virtual time: words committed before the pause and never the half-heard word, an empty run ignored, a run at the pause cancelled, utterances kept apart, silence not kept, the gate's three rules, and short dictation never later than batch. The same loop in real time, with the listener thread and audio played at speaking pace. The timing tests passed 40 times of 40 under load.
- The reader with a scripted recognizer: typing with spoken commands as one undo step, the 40-cell status line, held and spoken words, the question outside edit mode, stopping, finishing before edit mode ends and at quit, a recognizer that never finishes, a phrase with no words, and a failure said at the interface level off.
- Whisper itself (ignored test, run by hand): live text identical to batch text on all three fixtures.

## Consequences

- On a quiet machine a long sentence shows its first words a few seconds in; short phrases arrive at their pause, as before. On a busy machine every run exceeds 1.5 seconds and live dictation becomes transcription per phrase at each pause, which is still sooner than transcription after Enter.
- The encoder's fixed 30-second cost is the floor under every run. Moving it needs a model whose encoder follows the audio, not a patch to this one.
- The reader, the GUI, and `tw` now link RTen, earshot, and the microphone (rodio and cpal) through the app's `dictation` feature; `--no-default-features` builds of the reader leave them out.
- Whether the voice should speak while the microphone is open is the owner's to decide in the dictation listening session; the setting exists either way.

## See also

- [ADR-0013: Dictation through a Whisper program](0013-dictation.md)
- [ADR-0023: Piper voices and Whisper dictation in-process on RTen](0023-in-process-neural-speech.md)
- [ADR-0043: Menus and the palette from one model](0043-menus-and-the-palette-from-one-model.md)
- [The voice typing guide](../dictation.md)
